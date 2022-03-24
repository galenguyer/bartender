use chrono::prelude::*;
use serde::Serialize;

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct Machine {
    pub id: i32,
    pub name: String,
    pub display_name: String,
    pub active: bool,
}

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct Slot {
    pub machine: i32,
    pub number: i32,
    pub item: i32,
    pub active: bool,
    pub count: Option<i32>,
}

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct Item {
    pub id: i32,
    pub name: String,
    pub price: i32,
}

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct Drop {
    pub id: i32,
    pub timestamp: DateTime<Utc>,
    pub username: String,
    pub machine: i32,
    pub slot: i32,
    pub item: i32,
    pub item_name: String,
    pub item_price: i32,
}

#[derive(sqlx::FromRow, Debug, Serialize)]
pub struct SlotWithItem {
    pub machine: i32,
    pub number: i32,
    pub item: i32,
    pub active: bool,
    pub count: Option<i32>,
    pub id: i32,
    pub name: String,
    pub price: i32,
}
