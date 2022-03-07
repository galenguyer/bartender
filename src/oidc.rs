use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{error::Error, time::Duration};

pub struct OIDCClient {
    http_client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OIDCUser {
    pub name: String,
    pub groups: Box<[String]>,
    pub drink_balance: i32,
}

#[derive(Debug)]
pub enum OIDCError {
    Unauthorized,
    ReqwestError(reqwest::Error),
    Unknown,
}

impl Error for OIDCError {}

impl std::fmt::Display for OIDCError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            OIDCError::Unauthorized => write!(f, "OIDC returned Unauthorized"),
            OIDCError::ReqwestError(re) => write!(f, "Reqwest Error: {re}"),
            &OIDCError::Unknown => write!(f, "Unknown OIDC Error"),
        }
    }
}

impl OIDCClient {
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

impl OIDCUser {
    pub fn is_drink_admin(&self) -> bool {
        self.groups.iter().contains(&String::from("drink"))
    }
}
