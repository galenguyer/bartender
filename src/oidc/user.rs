use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OIDCUser {
    pub name: Option<String>,
    pub preferred_username: String,
    pub groups: Box<[String]>,
    pub drink_balance: Option<i32>,
}

impl OIDCUser {
    pub fn is_drink_admin(&self) -> bool {
        self.groups.iter().contains(&String::from("drink"))
    }
}
