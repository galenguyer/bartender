use super::{user::OIDCUser, OIDCError};
use std::time::Duration;

#[derive(Clone)]
pub struct OIDCClient {
    http_client: reqwest::Client,
}

impl OIDCClient {
    #[must_use]
    pub fn new() -> Self {
        OIDCClient {
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn validate_token(&self, token: &str) -> Result<OIDCUser, OIDCError> {
        let formatted_token = if token.starts_with("Bearer") {
            token.to_string()
        } else {
            format!("Bearer {token}")
        };

        let res = self
            .http_client
            .get("https://sso.csh.rit.edu/auth/realms/csh/protocol/openid-connect/userinfo")
            .header("Authorization", &formatted_token)
            .timeout(Duration::from_secs(5))
            .send()
            .await;

        match res {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<OIDCUser>().await {
                        Ok(user) => Ok(user),
                        Err(e) => Err(OIDCError::ReqwestError(e)),
                    }
                } else if response.status().is_client_error() {
                    Err(OIDCError::Unauthorized)
                } else {
                    Err(OIDCError::Unknown)
                }
            }
            Err(e) => Err(OIDCError::ReqwestError(e)),
        }
    }
}

impl Default for OIDCClient {
    fn default() -> Self {
        Self::new()
    }
}
