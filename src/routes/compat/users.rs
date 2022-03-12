use crate::ldap::client as ldap_client;
use crate::ldap::user::LdapUserChangeSet;
use axum::extract::{Extension, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use itertools::Itertools;
use serde_json::json;
use std::collections::HashMap;

pub async fn get_credits(
    Extension(mut ldap): Extension<ldap_client::LdapClient>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let uid = params.get("uid").map(|id| id.to_owned());
    let ibutton = params.get("ibutton").map(|id| id.to_owned());

    if uid.is_some() {
        let uid = uid.unwrap();
        let user = ldap.get_user(&uid).await;
        if user.is_none() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "message": format!("The requested uid '{}' does not belong to any user.", uid)
                })),
            );
        }
        let user = user.unwrap();
        return (
            StatusCode::OK,
            Json(json!({
                "message": format!("Retrieved user with uid {}", uid),
                "user": {
                    "uid": uid,
                    "cn": user.cn,
                    "drinkBalance": user.drinkBalance.unwrap_or(0)
                }
            })),
        );
    } else if ibutton.is_some() {
        let ibutton = ibutton.unwrap();
        let user = ldap.get_user_by_ibutton(&ibutton).await;
        if user.is_none() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"message": "The provided iButton value does not belong to any user."})),
            );
        }
        let user = user.unwrap();
        return (
            StatusCode::OK,
            Json(json!({
                "message": format!("Retrieved user with iButton {}", ibutton),
                "user": {
                    "uid": uid,
                    "cn": user.cn,
                    "drinkBalance": user.drinkBalance.unwrap_or(0)
                }
            })),
        );
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(
                json!({"message":"Please provide a valid CSH uid or ibutton value as a URI parameter."}),
            ),
        )
    }
}

pub async fn set_credits(
    Extension(mut ldap): Extension<ldap_client::LdapClient>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let uid = body["uid"].as_str();
    let new_balance = body["drinkBalance"].as_i64();

    let mut unprovided: Vec<String> = Vec::new();
    if uid.is_none() {
        unprovided.push(String::from("uid"))
    }
    if new_balance.is_none() {
        unprovided.push(String::from("drinkBalance"))
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

    let user = ldap.get_user(uid.unwrap()).await;

    if user.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "message":
                    format!(
                        "The requested uid '{}' does not belong to any user.",
                        uid.unwrap()
                    )
            })),
        );
    }
    let user = user.unwrap();

    let old_credits = user.drinkBalance.unwrap_or(0);
    let change_set = LdapUserChangeSet {
        dn: user.clone().dn,
        drinkBalance: Some(new_balance.unwrap()),
        ibutton: None,
    };
    ldap.update_user(&change_set).await;
    let user = ldap.get_user(uid.unwrap()).await.unwrap();
    let new_balance = user.drinkBalance.unwrap_or(0);

    (
        StatusCode::OK,
        Json(json!({
            "message":
                format!(
                    "Drink balance updated from {} credits to {} credits for user '{}'",
                    old_credits, new_balance, user.uid
                )
        })),
    )
}
