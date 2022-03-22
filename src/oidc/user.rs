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
    #[must_use]
    pub fn has_group(&self, group_name: &str) -> bool {
        self.groups.iter().contains(&group_name.to_owned())
    }
}
