use axum::extract::Extension;
use axum::http::Method;
use axum::{
    routing::{get, post, put},
    Router,
};
use dotenvy::dotenv;
use log::info;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::{self, CorsLayer, Origin};
use tower_http::trace::TraceLayer;

use bartender::ldap::client as ldap_client;
use bartender::oidc::client as oidc_client;
use bartender::routes;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // Load environment variables from .env file
    dotenv().ok();
    // Set logging levels if not already set
    if env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "bartender=debug,tower_http=info");
    }

    // Initialize tracing with previously set logging levels
    tracing_subscriber::fmt::init();

    // Connect to Postgres
    let pg_pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&env::var("DATABASE_URL").unwrap())
            .await?,
    );
    info!("Postgres pool initialized");

    // Create an OIDC client
    let oidc_client = oidc_client::OIDCClient::new();
    info!("OIDC client initialized");

    // Create an LDAP client
    let ldap_client = ldap_client::LdapClient::new(
        &env::var("LDAP_BIND_DN").unwrap(),
        &env::var("LDAP_BIND_PW").unwrap(),
    )
    .await;
    info!("LDAP client initialized");

    // Map routes to handlers
    let app = Router::new()
        .route("/", get(routes::compat::root::root))
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
        .nest(
            "/api",
            Router::new().nest(
                "/v2",
                Router::new().route("/sms", post(routes::v2::sms::handle)),
            ),
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

    // Bind and serve
    // TODO: Optionally read this from configuration
    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    info!("Listening on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
