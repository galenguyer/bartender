use crate::ldap::client::LdapClient;

use super::client::OIDCClient;
use super::user;
use axum::async_trait;
use axum::extract::FromRequest;
use axum::http::StatusCode;
use axum::BoxError;
use serde_json::json;
use serde::Deserialize;
use std::env;

pub struct OIDCAuth(pub user::OIDCUser);

#[derive(Deserialize, Debug)]
struct MinimalUserInfo {
    preferred_username: String,
}

#[async_trait]
impl<B> FromRequest<B> for OIDCAuth
where
    B: axum::body::HttpBody + Send,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    type Rejection = (axum::http::StatusCode, axum::Json<serde_json::Value>);

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        // Grab the "Authorization" header from the request
        let auth_header = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .map(|h| h.to_str().unwrap());

        if let Some(header) = auth_header {
            // Get the OIDClient from the request global state
            let oidc_client: &OIDCClient = &*req.extensions().get().unwrap();

            match oidc_client.validate_token(header).await {
                Ok(user) => {
                    return Ok(Self(user));
                }
                Err(_) => {
                    return Err((
                        StatusCode::UNAUTHORIZED,
                        axum::Json(json!({"error": "token invalid or expired"})),
                    ))
                }
            }
        }

        // If there's no "Authorization" header, get the "X-Auth-Token" header
        let secret_header = req
            .headers()
            .get("X-Auth-Token")
            .map(|value| value.to_str().unwrap());
        if let Some(secret) = secret_header {
            if secret == env::var("MACHINE_SECRET").unwrap() {
                // If X-User-Info is set
                let uid_header = req
                    .headers()
                    .get("X-User-Info")
                    .map(|v| v.to_str().unwrap().to_owned())
                    .and_then(|value| serde_json::from_str::<MinimalUserInfo>(&value).ok());
                if let Some(user) = uid_header {
                    let ldap = &mut *req.extensions_mut().get_mut::<LdapClient>().unwrap();
                    match ldap.get_user(&user.preferred_username).await {
                        Some(user) => {
                            return Ok(Self(user::OIDCUser {
                                name: Some(user.cn),
                                preferred_username: user.uid,
                                groups: user.groups.try_into().unwrap(),
                                drink_balance: user.drinkBalance,
                            }))
                        }
                        None => {
                            return Err((
                                StatusCode::UNAUTHORIZED,
                                axum::Json(json!({"error": "user not found"})),
                            ));
                        }
                    }
                }

                // Else if X-User-Phone is set
                let phone_header = req
                    .headers()
                    .get("X-User-Phone")
                    .map(|v| v.to_str().unwrap().to_owned());
                if let Some(mut phone) = phone_header {
                    log::debug!("Got SMS from {}", phone);

                    // Remove +1 from numbers
                    if phone.starts_with('+') {
                        if !phone.starts_with("+1") {
                            return Err((
                                StatusCode::UNAUTHORIZED,
                                axum::Json(
                                    json!({"error": "Only US phone numbers are supported for now"}),
                                ),
                            ));
                        } else {
                            phone = phone.replace("+1", "");
                        }
                    }

                    let ldap = &mut *req.extensions_mut().get_mut::<LdapClient>().unwrap();
                    match ldap.get_user_by_phone(&phone).await {
                        Some(user) => {
                            log::info!("Got user {} from phone number {}", user.uid, phone);
                            return Ok(Self(user::OIDCUser {
                                name: Some(user.cn),
                                preferred_username: user.uid,
                                groups: user.groups.try_into().unwrap(),
                                drink_balance: user.drinkBalance,
                            }));
                        }
                        None => {
                            return Err((
                                StatusCode::UNAUTHORIZED,
                                axum::Json(json!({
                                    "error": "invalid user",
                                    "message": format!("Make sure your phone number is in the format {} on profiles", &phone)
                                })),
                            ));
                        }
                    }
                }
                // If no other identifying information is provided
                else {
                    return Ok(Self(user::OIDCUser {
                        name: Some(String::from("Drink Machine")),
                        preferred_username: String::from("drink_machine"),
                        groups: Box::new([String::from("drink")]),
                        drink_balance: Some(0),
                    }));
                }
            } else {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    axum::Json(json!({"error": "invalid machine secret"})),
                ));
            }
        }

        return Err((
            StatusCode::UNAUTHORIZED,
            axum::Json(json!({"error": "missing auth header"})),
        ));
    }
}
