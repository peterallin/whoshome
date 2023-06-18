use anyhow::{Context, Result};
use clap::{Subcommand, Parser};
// use config::Config;
use router::Router;
use tracing_subscriber::{prelude::*, util::SubscriberInitExt};

mod config;
mod router;
mod unifi_dream_router;

#[derive(clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
enum Commands {
    Block {
        client_name: String,
    },
    Unblock {
        client_name: String,
    }
}

impl Cli {
    fn client_name(&self) -> &str {
        match &self.command {
            Commands::Block { client_name } => client_name,
            Commands::Unblock { client_name } => client_name,
        }
    }
}

fn main() -> Result<()> {
    configure_tracing();
    let options = Cli::parse();
    let config = config::get_config().context("Failed to read settings")?;
    let router = unifi_dream_router::UnifiDreamRouter::new(&config.router)
        .context("Failed to create router interface")?;
    // show_who_is_home(&mut router, &config)?;

    let clients = router.known_clients()?;
    print!("{:#?}", clients);
    let client = clients
        .iter()
        .find(|c| c.name == options.client_name())
        .unwrap();

    match options.command {
        Commands::Block { .. } => router.block_client(client),
        Commands::Unblock { .. } => router.unblock_client(client),
    }?;

    Ok(())
}

// This is not currently working because we right now show all known clients, not just the online ones
// fn show_who_is_home(router: &mut dyn router::Router, config: &Config) -> Result<()> {
//     let clients: Vec<_> = router
//         .known_clients()
//         .context("Failed to get list of connected client")?;

//     trace!("Online clients {clients:?}");

//     for person_home in config.persons.iter().filter(|p| {
//         p.devices
//             .iter()
//             .any(|d| clients.iter().any(|c| &c.name == d))
//     }) {
//         println!("{} is home", person_home.name);
//     }

//     Ok(())
// }

pub fn configure_tracing() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("whoshome=trace"))
        .with(tracing_subscriber::fmt::layer())
        .init();
}
