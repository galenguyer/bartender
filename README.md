# bartender
Bartender is the next iteration of drink server. Currently, it is compatible with routes offered by [mizu](https://github.com/ComputerScienceHouse/mizu). These compatible routes are provided by the `routes::compat` module, which should be expected to be deprecated in a future release, once new routes are established.

## Environment Variables
Bartender is configured using environment variables. [dotenvy](https://lib.rs/crates/dotenvy) is used to load environment variables from a `.env` file, if present. Below are the required environment variables.
```
DATABASE_URL
MACHINE_SECRET
LDAP_BIND_DN
LDAP_BIND_PW
```
