use crate::keys::db::KeyEntry;
use anyhow::{anyhow, bail, Context, Result};
use once_cell::sync::Lazy;
use openpgp::armor::{Kind, Reader, ReaderMode};
use regex::Regex;
use sequoia_openpgp as openpgp;
use sha1::{Digest, Sha1};
use std::io::Read;
use std::path::Path;

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

pub fn file_to_key_entry(path: &Path) -> Result<Option<KeyEntry>> {
    let filename = path
        .to_str()
        .ok_or_else(|| anyhow!("Filename not valid utf-8"))?;
    let Some(captures) = FILE_REGEX.captures(filename) else {
        return Ok(None);
    };

    // Unwrap is ok here, as we know the regex
    let username = captures.get(1).unwrap().as_str();
    let hashed_username = hash_file_name(username);
    let host = captures.get(2).unwrap().as_str();

    Ok(Some(KeyEntry {
        hashed_username,
        domain: host.to_string(),
    }))
}

pub fn read_key_file(path: &Path) -> Result<Option<(KeyEntry, Vec<u8>)>> {
    if !path.exists() || !path.is_file() {
        bail!("File {} not found or not a file", path.to_string_lossy());
    }

    let Some(key_entry) = file_to_key_entry(path)? else {
        return Ok(None);
    };
    let content = std::fs::read(path)?;

    // Validate the public key, tolerate common formatting errors such as erroneous
    // whitespace, but fail on private keys
    let mut reader = Reader::from_bytes(&content, ReaderMode::Tolerant(Some(Kind::PublicKey)));
    let mut buf = Vec::new();
    reader
        .read_to_end(&mut buf)
        .context("File is not a valid public key")?;

    Ok(Some((key_entry, buf)))
}

fn hash_file_name(name: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(name.as_bytes());
    let result = hasher.finalize();
    zbase32::encode_full_bytes(&result[..])
}
