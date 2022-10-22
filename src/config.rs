use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub router: String,
    pub persons: Vec<Person>,
}

#[derive(Deserialize, Debug)]
pub struct Person {
    pub name: String,
    pub devices: Vec<String>,
}

pub fn get_config() -> Result<Config> {
    let config = std::fs::read_to_string("config.dhall").context("Unable to read config.dhall")?;
    let config = serde_dhall::from_str(&config)
        .parse()
        .context("Failed to parse config.dhall")?;
    Ok(config)
}
