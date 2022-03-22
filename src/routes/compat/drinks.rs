use crate::db;
use crate::ldap::client::LdapClient;
use crate::ldap::user::LdapUserChangeSet;
use crate::machine;
use crate::oidc::auth::OIDCAuth;
use crate::{DrinkResponse, Item, Machine, Slot};
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use itertools::Itertools;
use log::{debug, error, info, warn};
use serde_json::json;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

// GET /drinks
pub async fn get_drinks(
    OIDCAuth(_user): OIDCAuth,
    Extension(pool): Extension<Arc<Pool<Postgres>>>,
) -> impl IntoResponse {
    let machines = db::machines::get_active_machines(&pool).await.unwrap();
    let futures: FuturesOrdered<_> = machines
        .iter()
        .map(|m| machine::get_status(&m.name))
        .collect();
    let machine_states: Vec<Result<machine::MachineResponse, reqwest::Error>> =
        futures.collect().await;
    let slots = db::slots::get_slots_with_items(&pool).await.unwrap();
    let resp =
        DrinkResponse {
            machines: machines
                .iter()
                .map(|machine| Machine {
                    id: machine.id,
                    name: machine.name.clone(),
                    display_name: machine.display_name.clone(),
                    is_online: machine_states.iter().any(
                        |machine_response| match machine_response {
                            Ok(r) => r.name == machine.name,
                            _ => false,
                        },
                    ),
                    slots: slots
                        .iter()
                        .filter(|slot| slot.machine == machine.id)
                        .map(|slot| Slot {
                            active: slot.active,
                            count: slot.count,
                            empty: match slot.count {
                                Some(0) => true,
                                Some(_) => false,
                                _ => match machine_states.iter().find(|machine_response| {
                                    match machine_response {
                                        Ok(response) => response.name == machine.name,
                                        _ => false,
                                    }
                                }) {
                                    Some(response) => {
                                        match response.as_ref().unwrap().slots.iter().find(
                                            |slot_response| slot_response.number == slot.number,
                                        ) {
                                            Some(slot_response) => !slot_response.stocked,
                                            None => true,
                                        }
                                    }
                                    None => true,
                                },
                            },
                            item: Item {
                                name: slot.name.clone(),
                                id: slot.id,
                                price: slot.price,
                            },
                            machine: machine.id,
                            number: slot.number,
                        })
                        .collect(),
                })
                .collect(),
            message: format!(
                "Successfully retrieved machine contents for {}",
                machines.iter().map(|machine| &machine.name).join(", ")
            ),
        };
    Json(resp)
}

// POST /drinks/drop
pub async fn drop(
    OIDCAuth(user): OIDCAuth,
    Json(payload): Json<serde_json::Value>,
    Extension(pool): Extension<Arc<Pool<Postgres>>>,
    Extension(mut ldap_client): Extension<LdapClient>,
) -> impl IntoResponse {
    let user_id = user.preferred_username;

    debug!("Validating drop request by {}", user_id);
    let mut unprovided: Vec<String> = Vec::new();

    if payload.get("machine").is_none() {
        unprovided.push(String::from("machine"));
    }
    if payload.get("slot").is_none() {
        unprovided.push(String::from("slot"));
    }
    if !unprovided.is_empty() {
        warn!(
            "Rejecting request from {} to drop a drink, missing parameters {}",
            user_id,
            unprovided.iter().join(", ")
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message":
                    format!(
                        "The following required parameters were not provided: {}",
                        unprovided.iter().join(", ")
                    )
            })),
        );
    }

    debug!("Fetching database info for drop request by {}", user_id);
    let machine = db::machines::get_machine(&pool, payload["machine"].as_str().unwrap()).await;
    if machine.is_err() {
        warn!(
            "Rejecting request from {} to drop a drink, {} is not a valid machine",
            user_id,
            payload["machine"].as_str().unwrap()
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message":
                    format!(
                        "The machine name '{}' is not a valid machine",
                        payload["machine"].as_str().unwrap()
                    )
            })),
        );
    }
    let machine = machine.unwrap();

    let slot =
        db::slots::get_slot_with_item(&pool, machine.id, payload["slot"].as_i64().unwrap() as i32)
            .await;
    if slot.is_err() {
        warn!(
            "Rejecting request from {} to drop a drink, machine {} does not have a slot with id {}",
            user_id,
            payload["machine"].as_str().unwrap(),
            payload["slot"].as_i64().unwrap()
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message":
                    format!(
                        "The machine '{}' does not have a slot with id '{}'",
                        payload["machine"].as_str().unwrap(),
                        payload["slot"].as_i64().unwrap()
                    )
            })),
        );
    }
    let slot = slot.unwrap();

    debug!(
        "Checking machine {} status for {}",
        payload["machine"].as_str().unwrap(),
        user_id
    );
    let machine_status = machine::get_status(&machine.name).await;
    if machine_status.is_err() {
        warn!(
            "Rejecting request from {} to drop a drink, machine {} is not online",
            user_id,
            payload["machine"].as_str().unwrap(),
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message":
                    format!(
                        "The machine '{}' is not online",
                        payload["machine"].as_str().unwrap(),
                    )
            })),
        );
    }
    let machine_status = machine_status.unwrap();

    if (machine.name == "snack" && slot.count.unwrap_or(0) < 1)
        || !(*machine_status.slots.get(slot.number as usize).unwrap()).stocked
    {
        warn!(
            "Rejecting request from {} to drop a drink, machine {} slot {} is empty",
            user_id,
            payload["machine"].as_str().unwrap(),
            payload["slot"].as_i64().unwrap()
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message":
                    "The requested slot is empty!"
            })),
        );
    }

    debug!("Checking drink credits for {}", user_id);
    let user = ldap_client.get_user(&user_id).await;
    if user.is_none() {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "message": format!("Could not find an account with the username '{}'", user_id)
            })),
        );
    }
    let user = user.unwrap();
    if user.drinkBalance.unwrap_or(0) < slot.price.into() {
        warn!(
            "Rejecting request from {} to drop a drink, insufficient drink balance for {} (has {}, needs {})",
            user_id,
            slot.name,
            user.drinkBalance.unwrap_or(0),
            slot.price,
        );
        return (
            StatusCode::PAYMENT_REQUIRED,
            Json(json!({
                "message":
                    format!(
                        "The user '{}' does not have a sufficient drinkBalance",
                        user_id
                    )
            })),
        );
    }

    debug!(
        "Sending drop request for machine {} slot {} by {}",
        payload["machine"].as_str().unwrap(),
        payload["slot"].as_i64().unwrap(),
        user_id
    );
    let drop_response = machine::drop(&machine.name, slot.number).await;

    if let Err(drop_error) = drop_response {
        if drop_error.is_connect() {
            error!(
                "Error dropping drink for {}, could not connect to machine {}",
                user_id,
                payload["machine"].as_str().unwrap(),
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    json!({"error": "Could not contact drink machine for drop!", "errorCode": 500}),
                ),
            );
        } else if drop_error.is_timeout() {
            error!(
                "Error dropping drink for {}, machine {} timed out",
                user_id,
                payload["machine"].as_str().unwrap(),
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    json!({"error": "Connection to the drink machine timed out!", "errorCode": 500}),
                ),
            );
        }
        error!(
            "Error dropping drink for {}, an unknown error occured occured dropping a drink from machine {} slot {}",
            user_id,
            payload["machine"].as_str().unwrap(),
            payload["slot"].as_i64().unwrap()
        );
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                json!({"error": "An unknown error occured while trying to drop a drink", "errorCode": 500}),
            ),
        );
    }

    let drop_response = drop_response.unwrap();

    if let Err(drop_error) = drop_response.error_for_status_ref() {
        let drop_content = drop_response.json::<serde_json::Value>().await.unwrap();
        error!(
            "Error dropping drink for {}, an error occured occured dropping a drink from machine {} slot {}: {}",
            user_id,
            payload["machine"].as_str().unwrap(),
            payload["slot"].as_i64().unwrap(),
            drop_content["error"].as_str().unwrap()
        );
        return (
            drop_error.status().unwrap(),
            Json(json!({
                "error": "Could not access slot for drop!",
                "message": drop_content["error"].as_str().unwrap(),
                "errorCode": drop_error.status().unwrap().as_u16()
            })),
        );
    }

    debug!("Updating drink balance for {}", user_id);
    let new_balance = user.drinkBalance.unwrap() - slot.price as i64;

    let change_set = LdapUserChangeSet {
        dn: user.clone().dn,
        drinkBalance: Some(new_balance),
        ibutton: None,
    };
    ldap_client.update_user(&change_set).await;

    if machine.name == "snack" {
        if db::slots::update_slot_count(&pool, machine.id, slot.number, slot.count.unwrap_or(1) - 1)
            .await
            .is_err()
        {
            error!(
                "Error updating db after drop for {}, could not change machine {} slot {} count {}",
                user_id,
                payload["machine"].as_str().unwrap(),
                payload["slot"].as_i64().unwrap(),
                slot.count.unwrap_or(1) - 1
            );
        }
        #[allow(clippy::collapsible_if)]
        if slot.count.unwrap_or(1) == 1 {
            if db::slots::update_slot_active(&pool, machine.id, slot.number, false)
                .await
                .is_err()
            {
                error!(
                    "Error updating db after drop for {}, could not change machine {} slot {} active {}",
                    user_id,
                    payload["machine"].as_str().unwrap(),
                    payload["slot"].as_i64().unwrap(),
                    false
                );
            }
        }
    }

    info!("Successfully dropped {} for {}", slot.name, user_id);
    (
        StatusCode::OK,
        Json(json!({"message": "Drop successful!", "drinkBalance": new_balance})),
    )
}
