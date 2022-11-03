use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use log::warn;

#[derive(Debug, Deserialize)]
pub struct MachineResponse {
    #[serde(skip)]
    pub name: String,
    pub slots: Box<[SlotResponse]>,
    pub temp: f32,
}

#[derive(Debug, Deserialize)]
pub struct SlotResponse {
    pub number: i32,
    pub stocked: bool,
}

fn report_error(name: &str, err: reqwest::Error) -> reqwest::Error {
    warn!("Couldn't get status for machine {}! Assuming it's offline. {:?}", name, err);
    err
}

pub async fn get_status(name: &str) -> Result<MachineResponse, reqwest::Error> {
    let client = reqwest::Client::new();

    let mut res = client
        .get(format!("https://{}.csh.rit.edu/slots", name))
        .header("X-Auth-Token", env::var("MACHINE_SECRET").unwrap())
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|err| report_error(name, err))?
        .json::<MachineResponse>()
        .await
        .map_err(|err| report_error(name, err))?;

    res.name = name.to_string();
    Ok(res)
}

pub async fn drop(name: &str, slot: i32) -> Result<reqwest::Response, reqwest::Error> {
    let client = reqwest::Client::new();
    // We pass the right slot, but bubbler does a -1... TODO: not be out of sync
    let body = HashMap::from([("slot", slot+1)]);

    client
        .post(format!("https://{}.csh.rit.edu/drop", name))
        .header("X-Auth-Token", env::var("MACHINE_SECRET").unwrap())
        .timeout(Duration::from_secs(15))
        .json(&body)
        .send()
        .await
}
