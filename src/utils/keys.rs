use anyhow::{anyhow, Context, Result};
use log::trace;
use once_cell::sync::Lazy;
use openpgp::armor::{Kind, Reader, ReaderMode};
use regex::Regex;
use sequoia_openpgp as openpgp;
use sha1::{Digest, Sha1};
use std::io::Read;
use std::path::Path;
use tokio::fs;

/// Match any text that has one @ sign and split at the @ sign
/// Trailing optional .asc is not included in the domain. See some examples below.
///
/// Will be included:
/// ```
/// user@example.com
/// user2@example.com.asc
/// ```
///
/// Will not be included:
/// ```
/// ktujkt7nrz91b17es7prizffedzxrsna
/// my-public-key.asc
/// ```
static FILE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^([^@]+)@([^@]+?)(?:\.asc)?$").unwrap());

pub async fn get_key_for_hash(path: &str, hash: &str, domain: &str) -> Result<Option<Vec<u8>>> {
    let path = Path::new(path);
    if !path.exists() || !path.is_dir() {
        return Err(anyhow!("File not found"));
    }

    let mut read_dir = fs::read_dir(path).await?;
    while let Some(file) = read_dir.next_entry().await? {
        let filename = file.file_name();
        let Some(filename) = filename.to_str() else {
            return Err(anyhow!("Filename is not valid utf-8: {:?}", &file.file_name()));
        };
        let Some(captures) = FILE_REGEX.captures(filename) else {
            trace!("Ignoring '{filename}'.");
            continue;
        };

        // Unwrap is ok here, as we know the regex
        let username = captures.get(1).unwrap().as_str();
        let host = captures.get(2).unwrap().as_str();
        trace!("Trying file {filename} (username = {username}, domain = {host})");

        if host != domain {
            continue;
        }

        let hashed_name = hash_file_name(username);
        trace!("Username hash: {hashed_name}");

        if hashed_name == hash {
            trace!("Found match, trying to read public key.");

            // We found the file, read its contents
            let content = fs::read_to_string(file.path()).await?;

            // Validate the public key, tolerate common formatting errors such as erroneous
            // whitespace, but fail on private keys
            let mut reader = Reader::from_bytes(
                content.as_bytes(),
                ReaderMode::Tolerant(Some(Kind::PublicKey)),
            );
            let mut buf = Vec::new();
            reader
                .read_to_end(&mut buf)
                .context("File is not a valid public key")?;
            return Ok(Some(buf));
        }
    }

    Ok(None)
}

fn hash_file_name(name: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(name.as_bytes());
    let result = hasher.finalize();
    zbase32::encode_full_bytes(&result[..])
}
