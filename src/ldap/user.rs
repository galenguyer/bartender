use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LdapUser {
    pub cn: String,
    pub uid: String,
    pub krbPrincipalName: String,
    pub mail: Vec<String>,
    pub mobile: Vec<String>,
    pub drinkBalance: i64,
    pub ibutton: Vec<String>,
}
