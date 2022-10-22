use anyhow::{Context, Result};
use config::Config;

mod config;
mod router;
mod unifi_dream_router;

fn main() -> Result<()> {
    let config = config::get_config().context("Failed to read settings")?;
    let router = unifi_dream_router::UnifiDreamRouter::new(&config.router)
        .context("Failed to create router interface")?;
    show_who_is_home(&router, &config)?;
    Ok(())
}

fn show_who_is_home(router: &dyn router::Router, config: &Config) -> Result<()> {
    let clients: Vec<_> = router
        .connected_clients()
        .context("Failed to get list of connected client")?;

    for person_home in config
        .persons
        .iter()
        .filter(|p| p.devices.iter().any(|d| clients.contains(d)))
    {
        println!("{} is home", person_home.name);
    }

    Ok(())
}
