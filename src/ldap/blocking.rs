pub mod client {
    use super::super::user::LdapUser;
    use ldap3::{LdapConn, LdapConnSettings, Mod, SearchEntry};
    use rand::seq::SliceRandom;
    use std::collections::HashSet;
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
                Some(LdapUser::from_entry(&user))
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
                    HashSet::from([format!("{}", user.drinkBalance.unwrap()).as_str()]),
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
}
