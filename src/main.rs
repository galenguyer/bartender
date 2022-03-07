use axum::extract::Extension;
use axum::response::IntoResponse;
use axum::Json;
use axum::{routing::get, Router};
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use itertools::Itertools;
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use bartender::db;
use bartender::machine;

struct State {
    pub pg_pool: Pool<Postgres>,
}

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
        env::set_var("RUST_LOG", "bartender=debug,tower_http=debug");
    }

    // Initialize tracing with previously set logging levels
    tracing_subscriber::fmt::init();

    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&env::var("DATABASE_URL").unwrap())
        .await?;

    let shared_state = Arc::new(State { pg_pool });

    let app = Router::new()
        .route("/", get(root))
        .layer(TraceLayer::new_for_http())
        .layer(Extension(shared_state));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    println!("starting on http://{}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn root(Extension(state): Extension<Arc<State>>) -> impl IntoResponse {
    let pool = &(*state).pg_pool;
    let machines = db::get_active_machines(pool).await.unwrap();
    let futures: FuturesOrdered<_> = machines
        .iter()
        .map(|m| machine::get_status(&m.name))
        .collect();
    let machine_states: Vec<Result<machine::MachineResponse, reqwest::Error>> =
        futures.collect().await;
    let slots = db::get_slots_with_items(pool).await.unwrap();
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
