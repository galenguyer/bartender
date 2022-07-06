use crate::db;
use crate::db::models::Drop;
use crate::ldap::client::LdapClient;
use crate::ldap::user::LdapUserChangeSet;
use crate::machine;
use crate::oidc::auth::OIDCAuth;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use itertools::Itertools;
use serde::Deserialize;
use serde_json::json;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

#[derive(Deserialize)]
pub struct SmsMessage {
    message: String,
}

// GET /api/v2/users/credits
// TODO: Holy moley, logging lmao
pub async fn handle(
    OIDCAuth(user): OIDCAuth,
    Extension(mut ldap): Extension<LdapClient>,
    Extension(pool): Extension<Arc<Pool<Postgres>>>,
    Json(payload): Json<SmsMessage>,
) -> impl IntoResponse {
    log::info!(
        "Recieved SMS from {} with content \"{}\"",
        user.preferred_username,
        payload.message
    );
    let parts: Vec<String> = payload
        .message
        .split_whitespace()
        .into_iter()
        .map(|s| s.to_owned())
        .collect();

    let command = parts.get(0);
    if command.is_none() {
        return (
            StatusCode::OK,
            Json(json!({
                "message": "Please specify a command"
            })),
        );
    }
    let command = command.unwrap().to_ascii_lowercase();
    match command.as_str() {
        "credits" => {
            log::info!("Getting credits for {}", user.preferred_username);

            // unwrap is safe because we got the username from OIDC
            let user = ldap.get_user(&user.preferred_username).await.unwrap();
            (
                StatusCode::OK,
                Json(json!({
                    "message": format!("You have {} drink credits!", user.drinkBalance.unwrap_or(0))
                })),
            )
        }
        "machines" => {
            log::info!("Listing machines for {}", user.preferred_username);
            let machines = db::machines::get_active_machines(&pool).await.unwrap();
            let futures: FuturesOrdered<_> = machines
                .iter()
                .map(|m| machine::get_status(&m.name))
                .collect();
            let machine_states: Vec<Result<machine::MachineResponse, reqwest::Error>> =
                futures.collect().await;

            let resp = machines
                .iter()
                .map(|machine| {
                    let online =
                        machine_states
                            .iter()
                            .any(|machine_response| match machine_response {
                                Ok(r) => r.name == machine.name,
                                _ => false,
                            });
                    format!(
                        "{}{}",
                        machine.name,
                        match online {
                            true => "",
                            false => " (offline)",
                        }
                    )
                })
                .join("\n");

            (StatusCode::OK, Json(json!({ "message": resp })))
        }
        "show" => {
            let machine_name = parts.get(1);
            if machine_name.is_none() {
                return (
                    StatusCode::OK,
                    Json(json!({ "message": "Make sure you provide a machine name!" })),
                );
            }
            let machine_name = machine_name.unwrap().to_owned();

            let machine = db::machines::get_machine(&pool, &machine_name).await;
            if machine.is_err() {
                return (
                    StatusCode::OK,
                    Json(json!({ "message": "Unknown machine" })),
                );
            }
            let machine = machine.unwrap();

            let machine_state = machine::get_status(&machine.name).await;
            if let Err(e) = machine_state {
                log::error!(
                    "Error getting machine {} state for {}: {}",
                    &machine.name,
                    user.preferred_username,
                    e
                );
                return (
                    StatusCode::OK,
                    Json(json!({
                        "message": format!("{} is offline", machine.display_name)
                    })),
                );
            }
            let machine_state = machine_state.unwrap();

            let slots = db::slots::get_slots_with_items(&pool, Some(machine.id)).await;
            if let Err(e) = slots {
                log::error!(
                    "Error getting slots in machine {} for {}: {}",
                    &machine.name,
                    user.preferred_username,
                    e
                );
                return (
                    StatusCode::OK,
                    Json(json!({ "message": "Unknown error getting slots" })),
                );
            }
            let slots = slots.unwrap();

            let resp = slots
                .iter()
                .filter(|slot| {
                    slot.active
                        && match slot.count {
                            Some(0) => false,
                            Some(_) => true,
                            _ => {
                                match machine_state
                                    .slots
                                    .iter()
                                    .find(|state_slot| state_slot.number == slot.number)
                                {
                                    Some(state_slot) => state_slot.stocked,
                                    None => false,
                                }
                            }
                        }
                })
                .map(|slot| format!("{} - {} ({}cr)", slot.number, slot.name, slot.price))
                .join("\n");

            (StatusCode::OK, Json(json!({ "message": resp })))
        }
        "drop" => {
            let machine_name = parts.get(1);
            if machine_name.is_none() {
                log::warn!(
                    "Rejecting request from {} to drop a drink, did not provide a machine name",
                    user.preferred_username,
                );
                return (
                    StatusCode::OK,
                    Json(json!({ "message": "Make sure you provide a machine name!" })),
                );
            }
            let machine_name = machine_name.unwrap().to_owned();

            let machine = db::machines::get_machine(&pool, &machine_name).await;
            if let Err(e) = machine {
                log::error!(
                    "Error getting machine {} for {}: {}",
                    &machine_name,
                    user.preferred_username,
                    e
                );
                return (
                    StatusCode::OK,
                    Json(json!({ "message": "Unknown machine" })),
                );
            }
            let machine = machine.unwrap();

            let slot_num = parts.get(2);
            if slot_num.is_none() {
                log::warn!(
                    "Rejecting request from {} to drop a drink, did not provide a slot number",
                    user.preferred_username,
                );
                return (
                    StatusCode::OK,
                    Json(json!({ "message": "Make sure you provide a slot number!" })),
                );
            }
            let slot_num = slot_num.unwrap().to_owned().parse::<i32>();
            if let Err(e) = slot_num {
                log::error!(
                    "Error parsing slot number {} for {}: {}",
                    parts.get(2).unwrap(),
                    user.preferred_username,
                    e
                );
                return (
                    StatusCode::OK,
                    Json(json!({
                        "message": format!("Couldn't parse slot number {}", parts.get(2).unwrap())
                    })),
                );
            }
            let slot_num = slot_num.unwrap();

            let slot = db::slots::get_slot_with_item(&pool, machine.id, slot_num).await;
            if let Err(e) = slot {
                log::error!(
                    "Error getting slot {} in machine {} for {}: {}",
                    slot_num,
                    machine.name,
                    user.preferred_username,
                    e
                );
                return (
                    StatusCode::OK,
                    Json(json!({
                        "message": format!("{} doesn't have that slot", machine.display_name)
                    })),
                );
            }
            let slot = slot.unwrap();

            let machine_state = machine::get_status(&machine.name).await;
            if let Err(e) = machine_state {
                log::error!(
                    "Error getting machine {} state for {}: {}",
                    machine.name,
                    user.preferred_username,
                    e
                );
                return (
                    StatusCode::OK,
                    Json(json!({
                        "message": format!("{} is offline", machine.display_name)
                    })),
                );
            }
            let machine_state = machine_state.unwrap();
            let slot_state = machine_state
                .slots
                .iter()
                .find(|slot| slot.number == slot_num);

            let slot_empty = match slot.count {
                Some(0) => true,
                Some(_) => false,
                _ => match slot_state {
                    Some(slot_state) => !slot_state.stocked,
                    None => true,
                },
            };
            if slot_empty {
                log::warn!(
                    "Rejecting request from {} to drop from machine {} slot {}, slot is empty",
                    user.preferred_username,
                    machine.name,
                    slot.number,
                );

                return (
                    StatusCode::OK,
                    Json(json!({
                        "message": format!("{} slot {} is empty", machine.display_name, slot.number)
                    })),
                );
            }

            let user = ldap.get_user(&user.preferred_username).await.unwrap();
            if user.drinkBalance.unwrap_or(0) < slot.price.into() {
                log::warn!(
                    "Rejecting request from {} to drop a drink, insufficient drink credits",
                    &user.uid,
                );
                return (
                    StatusCode::OK,
                    Json(json!({ "message": "You don't have enough drink credits!" })),
                );
            }

            let drop_response = machine::drop(&machine.name, slot.number).await;
            if let Err(drop_error) = drop_response {
                if drop_error.is_connect() {
                    log::error!(
                        "Error dropping drink for {}, could not connect to machine {}",
                        user.uid,
                        machine.name
                    );
                    return (
                        StatusCode::OK,
                        Json(json!({
                            "message":
                                format!("Could not contact {} for drop!", machine.display_name)
                        })),
                    );
                } else if drop_error.is_timeout() {
                    log::error!(
                        "Error dropping drink for {}, machine {} timed out",
                        user.uid,
                        machine.name
                    );
                    return (
                        StatusCode::OK,
                        Json(json!({
                            "error": format!("Connection to {} timed out!", machine.display_name)
                        })),
                    );
                }

                log::error!(
                    "Error dropping drink for {}, an unknown error occured occured dropping a drink from machine {} slot {}",
                    user.uid,
                    machine.name,
                    slot.number
                );
                return (
                    StatusCode::OK,
                    Json(json!({ "message": "An unknown error occured while trying to drop :(" })),
                );
            }

            let drop_response = drop_response.unwrap();

            if drop_response.error_for_status_ref().is_err() {
                let drop_content = drop_response.json::<serde_json::Value>().await.unwrap();
                log::error!(
                    "Error dropping drink for {}, an error occured occured dropping a drink from machine {} slot {}: {}",
                    user.uid,
                    machine.name,
                    slot.number,
                    drop_content["error"].as_str().unwrap()
                );
                return (
                    StatusCode::OK,
                    Json(json!({
                        "message": "Could not access slot for drop",
                    })),
                );
            }

            let new_balance = user.drinkBalance.unwrap() - slot.price as i64;

            let change_set = LdapUserChangeSet {
                dn: user.clone().dn,
                drinkBalance: Some(new_balance),
                ibutton: None,
            };
            ldap.update_user(&change_set).await;

            if machine.name == "snack" {
                if db::slots::update_slot_count(
                    &pool,
                    machine.id,
                    slot.number,
                    slot.count.unwrap_or(1) - 1,
                )
                .await
                .is_err()
                {
                    log::error!(
                        "Error updating db after drop for {}, could not change machine {} slot {} count {}",
                        user.uid,
                        machine.name,
                        slot.number,
                        slot.count.unwrap_or(1) - 1
                    );
                }
                #[allow(clippy::collapsible_if)]
                if slot.count.unwrap_or(1) == 1 {
                    if db::slots::update_slot_active(&pool, machine.id, slot.number, false)
                        .await
                        .is_err()
                    {
                        log::error!(
                            "Error updating db after drop for {}, could not change machine {} slot {} active {}",
                            user.uid,
                            machine.name,
                            slot.number,
                                false
                        );
                    }
                }
            }

            let drop = Drop {
                id: 0,                 // placeholder,
                timestamp: Utc::now(), // placeholder
                username: user.uid.clone(),
                machine: machine.id,
                slot: slot.number,
                item: slot.id,
                item_name: slot.name.clone(),
                item_price: slot.price,
            };
            if let Err(e) = db::drops::log_drop(&pool, &drop).await {
                log::warn!("Error logging drop: {e}");
            }

            log::info!("Successfully dropped {} for {}", slot.name, &user.uid);
            (
                StatusCode::OK,
                Json(json!({
                    "message": format!("Dropped you a {} from {}! You have {} credits remaining", slot.name, machine.display_name, new_balance)
                })),
            )
        },
        "commands" | "help" => (
            StatusCode::OK,
            Json(json!({ "message": "Valid commands:\ncredits\nmachines\nshow [machine]\ndrop [machine] [slot]" })),
        ),
        unknown => (
            StatusCode::OK,
            Json(json!({ "message": format!("Unknown command {}", unknown) })),
        ),
    }
}
