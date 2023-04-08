use anyhow::{anyhow, Result};
use clap::Parser;
use std::path::Path;

#[derive(Parser, Debug)]
pub struct Config {
    /// The path where the GPG keys are stored
    pub keys_path: String,
    #[clap(long, env, default_value = "0.0.0.0")]
    pub address: String,
    #[clap(long, env, default_value = "8080")]
    pub port: String,
    /// The path to the policy file. If not set, an empty policy is served.
    #[clap(long, short, env)]
    pub policy: Option<String>,
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        let keys_path = Path::new(&self.keys_path);
        if !keys_path.exists() {
            return Err(anyhow!("Keys path '{}' does not exist.", self.keys_path));
        }
        if !keys_path.is_dir() {
            return Err(anyhow!(
                "Keys path '{}' is not a directory.",
                self.keys_path
            ));
        }

        if let Some(policy) = &self.policy {
            let policy_path = Path::new(policy);
            if !policy_path.exists() || policy_path.is_dir() {
                return Err(anyhow!("Policy '{}' is not a file.", policy));
            }
        }

        Ok(())
    }
}
