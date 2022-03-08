use ldap3::{drive, Ldap, LdapConnAsync, SearchEntry};
use rand::prelude::SliceRandom;
use rand::SeedableRng;
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;
use trust_dns_resolver::{
    config::{ResolverConfig, ResolverOpts},
    AsyncResolver,
};

use super::user::LdapUser;

#[derive(Clone)]
pub struct LdapClient {
    ldap: Ldap,
}

impl LdapClient {
    pub async fn new(bind_dn: &str, bind_pw: &str) -> Self {
        let servers = get_ldap_servers().await;
        let (conn, mut ldap) = LdapConnAsync::new(
            servers
                .choose(&mut rand::rngs::StdRng::from_entropy())
                .unwrap(),
        )
        .await
        .unwrap();
        drive!(conn);

        ldap.simple_bind(bind_dn, bind_pw).await.unwrap();

        LdapClient { ldap }
    }

    pub async fn get_user(&mut self, uid: &str) -> Option<LdapUser> {
        self.ldap.with_timeout(std::time::Duration::from_secs(5));
        let (results, _result) = self
            .ldap
            .search(
                "cn=users,cn=accounts,dc=csh,dc=rit,dc=edu",
                ldap3::Scope::Subtree,
                &format!("uid={uid}"),
                vec!["*"],
            )
            .await
            .unwrap()
            .success()
            .unwrap();

        if results.len() == 1 {
            let user = SearchEntry::construct(results.get(0).unwrap().to_owned());
            let user_attrs = user.attrs;
            Some(LdapUser {
                dn: user.dn,
                cn: get_one(&user_attrs, "cn"),
                drinkBalance: get_one(&user_attrs, "drinkBalance"),
                krbPrincipalName: get_one(&user_attrs, "krbPrincipalName"),
                mail: get_vec(&user_attrs, "mail"),
                mobile: get_vec(&user_attrs, "mobile"),
                ibutton: get_vec(&user_attrs, "ibutton"),
                uid: get_one(&user_attrs, "uid"),
            })
        } else {
            None
        }
    }
}

async fn get_ldap_servers() -> Vec<String> {
    let resolver =
        AsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default()).unwrap();
    let response = resolver.srv_lookup("_ldap._tcp.csh.rit.edu").await.unwrap();

    // TODO: Make sure servers are working
    response
        .iter()
        .map(|record| {
            format!(
                "ldaps://{}",
                record.target().to_string().trim_end_matches('.')
            )
        })
        .collect()
}

fn get_one<T>(entry: &HashMap<String, Vec<String>>, field: &str) -> T
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
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
    entry
        .get(field)
        .unwrap()
        .iter()
        .map(|f| f.parse::<T>().unwrap())
        .collect()
}
