use sqlx::{Pool, Postgres};

pub mod db;
pub mod ldap;
pub mod machine;
pub mod oidc;

pub struct State {
    pub pg_pool: Pool<Postgres>,
    pub oidc_client: oidc::client::OIDCClient,
}
