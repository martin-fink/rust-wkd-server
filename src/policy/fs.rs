use std::{fs, io};
use tracing::error;

pub fn get_policy(policy_dir: &str, domain: &str) -> Result<Option<String>, io::Error> {
    if !fs::exists(policy_dir)? {
        error!("policy dir {} does not exist", policy_dir);
        return Ok(None);
    }

    // first, we check if the domain policy exists.
    let path = format!("{policy_dir}/{domain}");
    let policy = try_read_policy(&path)?;
    if policy.is_some() {
        return Ok(policy);
    }

    // otherwise, we try to serve the default policy.
    let path = format!("{policy_dir}/default");
    let policy = try_read_policy(&path)?;

    Ok(policy)
}

fn try_read_policy(domain_policy: &str) -> Result<Option<String>, io::Error> {
    let result = if fs::exists(domain_policy)? {
        Some(fs::read_to_string(domain_policy)?)
    } else {
        None
    };

    Ok(result)
}
