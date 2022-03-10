use axum::extract::{Extension, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum::{routing::get, Router};
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use itertools::Itertools;
use serde::Serialize;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use bartender::db;
use bartender::ldap::client as ldap_client;
use bartender::ldap::user::*;
use bartender::machine;
use bartender::oidc::{auth::OIDCAuth, client as oidc_client};

#[derive(Debug, Serialize)]
struct DrinkResponse {
    pub machines: Box<[Machine]>,
    pub message: String,
}
#[derive(Debug, Serialize)]
struct Machine {
    pub display_name: String,
    pub id: i32,
    pub is_online: bool,
    pub name: String,
    pub slots: Box<[Slot]>,
}
#[derive(Debug, Serialize)]
struct Slot {
    pub active: bool,
    pub count: Option<i32>,
    pub empty: bool,
    pub item: Item,
    pub machine: i32,
    pub number: i32,
}
#[derive(Debug, Serialize)]
struct Item {
    pub id: i32,
    pub name: String,
    pub price: i32,
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    // Set logging levels if not already set
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "bartender=debug,tower_http=debug,tokio=debug");
    }

    // Initialize tracing with previously set logging levels
    tracing_subscriber::fmt::init();

    let pg_pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&env::var("DATABASE_URL").unwrap())
            .await?,
    );
    let oidc_client = oidc_client::OIDCClient::new();

    let ldap_client = ldap_client::LdapClient::new(
        &env::var("LDAP_BIND_DN").unwrap(),
        &env::var("LDAP_BIND_PW").unwrap(),
    )
    .await;

    let app = Router::new()
        .route("/", get(root))
        .route("/auth_test", get(auth_test))
        .route("/ldap_test", get(ldap_test))
        .route("/search_users", get(users_search))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(Extension(ldap_client))
                .layer(Extension(pg_pool))
                .layer(Extension(oidc_client)),
        );

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    println!("starting on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn root(Extension(pool): Extension<Arc<Pool<Postgres>>>) -> impl IntoResponse {
    let machines = db::get_active_machines(&pool).await.unwrap();
    let futures: FuturesOrdered<_> = machines
        .iter()
        .map(|m| machine::get_status(&m.name))
        .collect();
    let machine_states: Vec<Result<machine::MachineResponse, reqwest::Error>> =
        futures.collect().await;
    let slots = db::get_slots_with_items(&pool).await.unwrap();
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

async fn auth_test(OIDCAuth(user): OIDCAuth) -> impl IntoResponse {
    format!("{:#?}", user)
}

async fn ldap_test(
    Extension(mut ldap): Extension<ldap_client::LdapClient>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let uid = params.get("uid").map(|id| id.to_owned());
    if uid.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"no uid given"})),
        );
    }

    let user = ldap.get_user(&uid.clone().unwrap()).await;
    let credits = params.get("credits").map(|num| num.parse::<i64>());

    match (user, credits) {
        (None, _) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": "user not found"})),
        ),
        (Some(u), None) => (StatusCode::OK, Json(json!(u))),
        (Some(u), Some(c)) => {
            let change_set = LdapUserChangeSet {
                dn: u.clone().dn,
                drinkBalance: Some(c.unwrap()),
                ibutton: None,
            };
            ldap.update_user(&change_set).await;
            let user = ldap.get_user(&uid.unwrap()).await.unwrap();
            (StatusCode::OK, Json(json!(user)))
        }
    }
}

async fn users_search(
    Extension(mut ldap): Extension<ldap_client::LdapClient>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let query = params.get("query").map(|id| id.to_owned());
    if query.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"no query given"})),
        );
    }

    let users = ldap.search_users(&query.clone().unwrap()).await;
    (StatusCode::OK, Json(json!(users)))
}
