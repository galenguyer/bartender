use serde::Serialize;

pub mod db;
pub mod ldap;
pub mod machine;
pub mod oidc;
pub mod routes;

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
