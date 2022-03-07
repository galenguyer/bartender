use serde::Deserialize;
use std::env;
use std::time::Duration;
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

pub async fn get_status(name: &str) -> Result<MachineResponse, reqwest::Error> {
    let client = reqwest::Client::new();

    let mut res = client
        .get(format!("https://{}.csh.rit.edu/slots", name))
        .header("X-Auth-Token", env::var("MACHINE_API_TOKEN").unwrap())
        .timeout(Duration::from_secs(5))
        .send()
        .await?
        .json::<MachineResponse>()
        .await?;

    res.name = name.to_string();
    Ok(res)
}
