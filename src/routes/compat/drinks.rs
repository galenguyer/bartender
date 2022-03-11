use crate::db;
use crate::machine;
use crate::{DrinkResponse, Item, Machine, Slot};
use axum::extract::Extension;
use axum::response::IntoResponse;
use axum::Json;
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use itertools::Itertools;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

pub async fn get_drinks(Extension(pool): Extension<Arc<Pool<Postgres>>>) -> impl IntoResponse {
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
