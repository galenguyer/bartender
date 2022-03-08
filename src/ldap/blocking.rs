pub mod client {
    use super::super::user::LdapUser;
    use ldap3::{LdapConn, LdapConnSettings, Mod, SearchEntry};
    use rand::seq::SliceRandom;
    use std::collections::HashSet;
    use std::{collections::HashMap, fmt::Debug, str::FromStr};
    use trust_dns_resolver::Resolver;

    pub struct LdapClient {
        #[allow(dead_code)]
        servers: Vec<String>,
        ldap: LdapConn,
    }

    impl LdapClient {
        pub fn new(bind_dn: &str, bind_pw: &str) -> Self {
            let servers = get_ldap_servers();
            let mut ldap = LdapConn::with_settings(
                LdapConnSettings::new().set_no_tls_verify(true),
                servers.choose(&mut rand::thread_rng()).unwrap(),
            )
            .unwrap();
            ldap.with_timeout(std::time::Duration::from_secs(5));
            ldap.simple_bind(bind_dn, bind_pw).unwrap();
            LdapClient { servers, ldap }
        }

        pub fn get_user(mut self, uid: &str) -> Option<LdapUser> {
            self.ldap.with_timeout(std::time::Duration::from_secs(5));
            let (results, _result) = self
                .ldap
                .search(
                    "cn=users,cn=accounts,dc=csh,dc=rit,dc=edu",
                    ldap3::Scope::Subtree,
                    &format!("uid={uid}"),
                    vec!["*"],
                )
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

        pub fn update_drink_credits(
            mut self,
            user: &LdapUser,
        ) -> Result<ldap3::LdapResult, ldap3::LdapError> {
            self.ldap.with_timeout(std::time::Duration::from_secs(5));
            self.ldap.modify(
                &user.dn,
                vec![Mod::Replace(
                    "drinkBalance",
                    HashSet::from([format!("{}", user.drinkBalance).as_str()]),
                )],
            )
        }
    }

    fn get_ldap_servers() -> Vec<String> {
        let resolver = Resolver::default().unwrap();
        let response = resolver.srv_lookup("_ldap._tcp.csh.rit.edu").unwrap();

        response
            .iter()
            .map(|record| {
                format!(
                    "ldaps://{}",
                    record.target().to_string().trim_end_matches('.')
                )
            })
            .filter(|addr| LdapConn::new(addr).is_ok())
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
}
