use crate::db;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use itertools::Itertools;
use serde_json::json;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

pub async fn get_items(Extension(pool): Extension<Arc<Pool<Postgres>>>) -> impl IntoResponse {
    match db::items::get_items(&pool).await {
        Ok(items) => (
            StatusCode::OK,
            Json(json!({
                "message": format!("Retrieved {} items", items.len()),
                "items": items
            })),
        ),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "message": format!("{:?}", e),
                "errorCode": 404
            })),
        ),
    }
}

pub async fn post_items(
    Json(body): Json<serde_json::Value>,
    Extension(pool): Extension<Arc<Pool<Postgres>>>,
) -> impl IntoResponse {
    let name = body["name"].as_str();
    let price = body["price"].as_i64();

    let mut unprovided: Vec<String> = Vec::new();

    if body.get("name").is_none() || name.is_none() {
        unprovided.push(String::from("name"));
    }
    if body.get("price").is_none() || price.is_none() {
        unprovided.push(String::from("price"));
    }
    if !unprovided.is_empty() {
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

    let name = name.unwrap();
    let price = price.unwrap();

    if price < 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message": "You cannot create a worthless item"
            })),
        );
    }

    match db::items::create_item(&pool, name, price as i32).await {
        Ok(_) => (
            StatusCode::CREATED,
            Json(json!({
                "message":
                    format!(
                        "Item '{}' added succesfully at a price of {} credits",
                        name, price
                    )
            })),
        ),
        // UHHHHHH, Ram?
        // https://github.com/ComputerScienceHouse/mizu/blob/master/mizu/items.py#L88-L92
        Err(_) => (
            StatusCode::CREATED,
            Json(json!({
                "message":
                    format!(
                        "Item '{}' added succesfully at a price of {} credits",
                        name, price
                    )
            })),
        ),
    }
}

pub async fn put_items(
    Json(body): Json<serde_json::Value>,
    Extension(pool): Extension<Arc<Pool<Postgres>>>,
) -> impl IntoResponse {
    let id = body["id"].as_i64();
    if id.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message": "An Item ID must be provided to update"
            })),
        );
    }
    let id = id.unwrap();

    let price = body["price"].as_i64();
    let name = body["name"].as_str();

    if price.is_none() && name.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message": "The name, price, or both values of an item must be provided to update"
            })),
        );
    }

    let old_item = db::items::get_item(&pool, id as i32).await;
    if old_item.is_err() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message": "Item ID value provided was invalid, ensure that the ID being provided is attached to an item that is present in the system."
            })),
        );
    }
    let old_item = old_item.unwrap();

    if let Some(price) = price {
        if price < 0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "message": "You cannot create a worthless item"
                })),
            );
        }

        let _ = db::items::update_item_price(&pool, old_item.id, price as i32).await;
    }
    if let Some(name) = name {
        if name.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "message": "An item cannot have an empty name"
                })),
            );
        }

        let _ = db::items::update_item_name(&pool, old_item.id, name).await;
    }

    let item = db::items::get_item(&pool, id as i32).await.unwrap();

    (
        StatusCode::OK,
        Json(json!({
            "message": format!("Item ID {} was '{}' for {} credits, now '{}' for {} credits", item.id, old_item.name, old_item.id, item.name, item.price),
            "item": item,
        })),
    )
}

pub async fn delete_items(
    Json(body): Json<serde_json::Value>,
    Extension(pool): Extension<Arc<Pool<Postgres>>>,
) -> impl IntoResponse {
    let id = body["id"].as_i64();
    if id.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message": "An Item ID must be provided for deletion"
            })),
        );
    }
    let id = id.unwrap() as i32;

    let item = db::items::get_item(&pool, id as i32).await;
    if item.is_err() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message": "Item ID value provided was invalid, ensure that the ID being provided is attached to an item that is present in the system."
            })),
        );
    }
    let item = item.unwrap();

    let _ = db::items::delete_item(&pool, item.id).await;

    (
        StatusCode::OK,
        Json(json!({
            "message":
                format!(
                    "Item '{}' with ID {} successfully deleted",
                    item.name, item.id
                )
        })),
    )
}
