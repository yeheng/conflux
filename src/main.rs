mod config;
mod error;
mod raft;

use anyhow::Result;
use config::AppConfig;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    init_tracing()?;

    info!("Starting Conflux distributed configuration center");

    // Load configuration
    let config = AppConfig::load().await?;
    info!("Configuration loaded successfully");

    // TODO: Initialize and start the application
    info!("Conflux server starting on {}:{}", config.server.host, config.server.port);

    // Keep the application running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down Conflux server");

    Ok(())
}

fn init_tracing() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "conflux=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    Ok(())
}
