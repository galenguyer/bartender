use ldap3::SearchEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;

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

impl LdapUser {
    pub fn from_entry(entry: &SearchEntry) -> Self {
        let user_attrs = &entry.attrs;
        LdapUser {
            dn: entry.dn.clone(),
            cn: get_one(user_attrs, "cn"),
            drinkBalance: get_one(user_attrs, "drinkBalance"),
            krbPrincipalName: get_one(user_attrs, "krbPrincipalName"),
            mail: get_vec(user_attrs, "mail"),
            mobile: get_vec(user_attrs, "mobile"),
            ibutton: get_vec(user_attrs, "ibutton"),
            uid: get_one(user_attrs, "uid"),
        }
    }
}

fn get_one<T>(entry: &HashMap<String, Vec<String>>, field: &str) -> T
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    // TODO: Handle null
    entry
        .get(field)
        .unwrap()
        .get(0)
        .unwrap()
        .parse::<T>()
        .unwrap()
}

fn get_vec<T>(entry: &HashMap<String, Vec<String>>, field: &str) -> Vec<T>
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    match entry.get(field) {
        Some(v) => v.iter().map(|f| f.parse::<T>().unwrap()).collect(),
        None => vec![],
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct LdapUserChangeSet {
    pub dn: String,
    pub drinkBalance: Option<i64>,
    pub ibutton: Option<Vec<String>>,
}
