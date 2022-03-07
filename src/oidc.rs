use std::error::Error;

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

pub mod auth;
pub mod client;
pub mod user;
