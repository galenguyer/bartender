use crate::db;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

pub async fn get_items(Extension(pool): Extension<Arc<Pool<Postgres>>>) -> impl IntoResponse {
    match db::get_items(&pool).await {
        Ok(items) => {
            (
                StatusCode::OK,
                Json(json!({
                    "message": format!("Retrieved {} items", items.len()),
                    "items": items
                })),
            )
        }
        Err(e) => {
            (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "message": format!("{:?}", e),
                    "errorCode": 404
                })),
            )
        }
    }
}
