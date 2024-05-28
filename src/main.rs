use crate::config::Config;
use anyhow::Context;
use clap::Parser;

mod config;
mod http;
mod keys;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    env_logger::init();

    let config = Config::parse();
    config.validate().context("Failed to validate config")?;

    http::serve(config).await?;

    Ok(())
}
