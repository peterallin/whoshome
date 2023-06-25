use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::Config;
use router::{Client, Router};
use tracing::trace;
use tracing_subscriber::{prelude::*, util::SubscriberInitExt};

mod config;
mod router;
mod unifi_dream_router;

#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Block { client_name: String },
    Unblock { client_name: String },
    ShowWhosHome,
}

fn find_client(router: &dyn Router, client_name: &str) -> Result<Client> {
    let clients = router.known_clients()?;
    let client = clients
        .iter()
        .find(|c| c.name == client_name)
        .with_context(|| format!("Could not find client named {client_name}"))?;
    Ok(client.clone())
}

fn main() -> Result<()> {
    configure_tracing();
    let options = Cli::parse();
    let config = config::get_config().context("Failed to read settings")?;
    let router = unifi_dream_router::UnifiDreamRouter::new(&config.router)
        .context("Failed to create router interface")?;

    match options.command {
        Commands::Block { client_name } => {
            router.block_client(&find_client(&router, &client_name)?)?
        }
        Commands::Unblock { client_name } => {
            router.unblock_client(&find_client(&router, &client_name)?)?
        }
        Commands::ShowWhosHome => show_who_is_home(&router, &config)?,
    };

    Ok(())
}

fn show_who_is_home(router: &dyn router::Router, config: &Config) -> Result<()> {
    let clients: Vec<_> = router
        .online_clients()
        .context("Failed to get list of connected client")?;

    trace!("Online clients {clients:?}");

    for person_home in config.persons.iter().filter(|p| {
        p.devices
            .iter()
            .any(|d| clients.iter().any(|c| &c.name == d))
    }) {
        println!("{} is home", person_home.name);
    }

    Ok(())
}

pub fn configure_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("whoshome=trace"))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
