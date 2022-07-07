use crate::db;
use crate::oidc::auth::OIDCAuth;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use itertools::Itertools;
use log::{debug, error, info, warn};
use serde_json::json;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

// PUT /slots
pub async fn update_slot_status(
    OIDCAuth(user): OIDCAuth,
    Json(body): Json<serde_json::Value>,
    Extension(pool): Extension<Arc<Pool<Postgres>>>,
) -> impl IntoResponse {
    if !user.has_group("drink") {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "User does not have the correct permissions",
                "errorCode": 401
            })),
        );
    }

    let user_id = user.preferred_username;

    let machine_name = body["machine"].as_str();
    let slot_id = body["slot"].as_i64();

    let mut unprovided: Vec<String> = Vec::new();
    if machine_name.is_none() {
        unprovided.push(String::from("machine"))
    }
    if slot_id.is_none() {
        unprovided.push(String::from("slot"))
    }

    if !unprovided.is_empty() {
        warn!(
            "Rejecting request from {} to update a slot, missing parameters {}",
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
    debug!("Validated required parameters");

    let active = body["active"].as_bool();
    let item_id = body["item_id"]
        .as_str()
        .and_then(|id| match id.parse::<i32>() {
            Ok(i) => Some(i),
            Err(_) => None,
        });

    if active.is_none() && item_id.is_none() {
        warn!(
            "Rejecting request from {} to update a, neither 'active' or 'item_id' specified",
            user_id,
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message":
                "Either the state or item within a slot must be provided for an update."
            })),
        );
    }

    let machine = db::machines::get_machine(&pool, machine_name.unwrap()).await;
    if machine.is_err() {
        warn!(
            "Rejecting request from {} to update a slot, machine '{}' does not exist",
            user_id,
            machine_name.unwrap()
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message":
                    format!(
                        "The machine '{}' is not a valid machine",
                        machine_name.unwrap()
                    )
            })),
        );
    }
    let machine = machine.unwrap();
    debug!("Validated existence of machine {}", machine.name);

    let slot = db::slots::get_slot(&pool, machine.id, slot_id.unwrap() as i32).await;
    if slot.is_err() {
        warn!(
            "Rejecting request from {} to update a slot, machine '{}' does not have a slot number '{}'",
            user_id,
            machine.name,
            slot_id.unwrap()
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message":
                    format!(
                        "The machine '{}' does not have a slot number {}",
                        machine.name,
                        slot_id.unwrap()
                    )
            })),
        );
    }
    let slot = slot.unwrap();
    debug!(
        "Validated existence of machine {}, slot {}",
        machine.name, slot.number
    );

    if let Some(active) = active {
        match db::slots::update_slot_active(&pool, machine.id, slot.number, active).await {
            Ok(_) => {
                debug!(
                    "Updated machine {} slot {} active: {} -> {}",
                    machine.name, slot.number, slot.active, active
                );
            }
            Err(e) => {
                error!("Failed to process request from {} to update machine {} slot {}: set active to {}", user_id, machine.name, slot.number, active);
                error!("Error: {:#?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Could not update slot",
                        "errorCode": 500,
                        "message": "Contact a drink admin"
                    })),
                );
            }
        }
    }

    if let Some(item_id) = item_id {
        let item = db::items::get_item(&pool, item_id as i32).await;
        match item {
            Ok(item) => {
                match db::slots::update_slot_item(&pool, machine.id, slot.number, item.id).await {
                    Ok(_) => {
                        debug!(
                            "Updated machine {} slot {} item: {} -> {}",
                            machine.name, slot.number, slot.item, item.id
                        );
                    }
                    Err(e) => {
                        error!("Failed to process request from {} to update machine {} slot {}: set item to {}", user_id, machine.name, slot.number, item_id);
                        error!("Error: {:#?}", e);
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({
                                "error": "Could not update slot",
                                "errorCode": 500,
                                "message": "Contact a drink admin"
                            })),
                        );
                    }
                }
            }
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "message": format!("No item with ID {} is present in the system", item_id)
                    })),
                );
            }
        }
    }

    if let Some(count) = body["count"].as_str().and_then(|s| Some(s.parse::<i64>().unwrap_or(-1))) {
        if count < 0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "message":
                    "The count value must be a positive integer"
                })),
            );
        }

        match db::slots::update_slot_count(&pool, machine.id, slot.number, count as i32).await {
            Ok(_) => {
                debug!(
                    "Updated machine {} slot {} count: {} -> {}",
                    machine.name,
                    slot.number,
                    slot.count.unwrap_or(-1),
                    count
                );
            }
            Err(e) => {
                error!("Failed to process request from {} to update machine {} slot {}: set count to {}", user_id, machine.name, slot.number, count);
                error!("Error: {:#?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": "Could not update slot",
                        "errorCode": 500,
                        "message": "Contact a drink admin"
                    })),
                );
            }
        }
    }

    let slot = db::slots::get_slot(&pool, machine.id, slot.number)
        .await
        .unwrap();
    info!(
        "Refreshed machine {} slot {} information for {}",
        machine.name, slot.number, user_id
    );

    (
        StatusCode::OK,
        Json(json!({
            "machine": machine.name,
            "number": slot.number,
            "active": slot.active,
            "item_id": slot.item,
            "count": slot.count
        })),
    )
}
