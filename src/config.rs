use anyhow::{anyhow, Result};
use clap::Parser;
use std::path::Path;

#[derive(Parser, Debug)]
pub struct Config {
    /// The path where the GPG keys are stored
    pub keys_path: String,
    #[clap(long, env, default_value = "0.0.0.0")]
    /// Address to bind the HTTP server to.
    /// Defaults to 0.0.0.0 to listen on all interfaces.
    pub address: String,
    #[clap(long, env, default_value = "8080")]
    /// Port to bind the HTTP server to.
    /// Defaults to 8080.
    pub port: String,
    /// The path to the policy directory. If not set, an empty policy is served.
    #[clap(long, short, env)]
    pub policy: Option<String>,
    #[clap(long, env)]
    /// Split certificate into individual user IDs.
    /// If set, only the requested user ID and corresponding key will be returned from the certificate.
    /// Otherwise, the response will include all user IDs and keys found in the file.
    pub split_keys: bool,
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
            if !policy_path.exists() || !policy_path.is_dir() {
                return Err(anyhow!("Policy directory '{}' is not a directory.", policy));
            }
        }

        Ok(())
    }
}
