use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct LdapUser {
    pub dn: String,
    pub cn: String,
    pub uid: String,
    pub krbPrincipalName: String,
    pub mail: Vec<String>,
    pub mobile: Vec<String>,
    pub drinkBalance: i64,
    pub ibutton: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LdapUserChangeSet {
    pub dn: String,
    pub drinkBalance: Option<i64>,
    pub ibutton: Option<Vec<String>>,
}
