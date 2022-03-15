use axum::extract::{Extension, Query};
use axum::http::Method;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use axum::{
    routing::{get, post, put},
    Router,
};
use log::info;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{self, CorsLayer, Origin};
use tower_http::trace::TraceLayer;

use bartender::ldap::client as ldap_client;
use bartender::oidc::{auth::OIDCAuth, client as oidc_client};
use bartender::routes;

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
    info!("Postgres pool initialized");

    let oidc_client = oidc_client::OIDCClient::new();
    info!("OIDC client initialized");

    let ldap_client = ldap_client::LdapClient::new(
        &env::var("LDAP_BIND_DN").unwrap(),
        &env::var("LDAP_BIND_PW").unwrap(),
    )
    .await;
    info!("LDAP client initialized");

    let app = Router::new()
        .route("/drinks", get(routes::compat::drinks::get_drinks))
        .route("/drinks/drop", post(routes::compat::drinks::drop))
        .route("/users", get(routes::compat::users::get_users))
        .route(
            "/users/credits",
            get(routes::compat::users::get_credits).put(routes::compat::users::set_credits),
        )
        .route("/slots", put(routes::compat::slots::update_slot_status))
        .route(
            "/items",
            get(routes::compat::items::get_items)
                .post(routes::compat::items::post_items)
                .put(routes::compat::items::put_items)
                .delete(routes::compat::items::delete_items),
        )
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(
                    CorsLayer::new()
                        .allow_origin(Origin::list(vec![
                            "http://localhost:3000".parse().unwrap(),
                            "https://webdrink.csh.rit.edu".parse().unwrap(),
                        ]))
                        .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE])
                        .allow_headers(cors::Any),
                )
                .layer(Extension(ldap_client))
                .layer(Extension(pg_pool))
                .layer(Extension(oidc_client)),
        );

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    info!("Listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

// For later references
// async fn auth_test(OIDCAuth(user): OIDCAuth) -> impl IntoResponse {
//     format!("{:#?}", user)
// }

// async fn users_search(
//     Extension(mut ldap): Extension<ldap_client::LdapClient>,
//     Query(params): Query<HashMap<String, String>>,
// ) -> impl IntoResponse {
//     let query = params.get("query").map(|id| id.to_owned());
//     if query.is_none() {
//         return (
//             StatusCode::BAD_REQUEST,
//             Json(json!({"error":"no query given"})),
//         );
//     }

//     let users = ldap.search_users(&query.clone().unwrap()).await;
//     (StatusCode::OK, Json(json!(users)))
// }
