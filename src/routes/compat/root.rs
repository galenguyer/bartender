use axum::response::{IntoResponse, Redirect};
use std::env;

pub async fn root() -> impl IntoResponse {
    Redirect::temporary(
        env::var("WEBDRINK_URL")
            .unwrap_or_else(|_| String::from("https://webdrink.csh.rit.edu/"))
            .parse()
            .unwrap(),
    )
}
