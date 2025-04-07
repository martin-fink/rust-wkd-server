use crate::keys::db::{CertEntry, CertKey};
use anyhow::{anyhow, bail, Context, Result};
use once_cell::sync::Lazy;
use openpgp::armor::{Kind, Reader, ReaderMode};
use regex::Regex;
use sequoia_openpgp as openpgp;
use sequoia_openpgp::parse::Parse;
use sequoia_openpgp::Cert;
use sha1::{Digest, Sha1};
use std::io::BufReader;
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

pub fn file_to_key_entry(path: &Path) -> Result<Option<(String, CertKey)>> {
    let filename = path
        .file_name()
        .ok_or_else(|| anyhow!("Path is empty"))?
        .to_str()
        .ok_or_else(|| anyhow!("Filename not valid utf-8"))?;
    let Some(captures) = FILE_REGEX.captures(filename) else {
        return Ok(None);
    };

    // Unwrap is ok here, as we know the regex
    let username = captures.get(1).unwrap().as_str();
    let hashed_username = hash_file_name(username);
    let host = captures.get(2).unwrap().as_str();

    let key = CertKey {
        hashed_username,
        domain: host.to_string(),
    };

    Ok(Some((username.into(), key)))
}

pub fn read_key_file(path: &Path) -> Result<Option<(CertKey, CertEntry)>> {
    if !path.exists() || !path.is_file() {
        bail!("File {} not found or not a file", path.to_string_lossy());
    }

    let Some((username, key_entry)) = file_to_key_entry(path)? else {
        return Ok(None);
    };
    let content = std::fs::read(path)?;

    // Validate the public key, tolerate common formatting errors such as erroneous
    // whitespace, but fail on private keys
    let reader = BufReader::new(Reader::from_bytes(
        &content,
        ReaderMode::Tolerant(Some(Kind::PublicKey)),
    ));
    let cert =
        Cert::from_reader(reader).context(format!("could not read certificate {:?}", path))?;

    Ok(Some((key_entry, CertEntry { username, cert })))
}

fn hash_file_name(name: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(name.as_bytes());
    let result = hasher.finalize();
    zbase32::encode_full_bytes(&result[..])
}

#[cfg(test)]
mod tests {
    use super::file_to_key_entry;
    use crate::keys::db::CertKey;
    use std::path::Path;

    #[test]
    fn file_to_entry() {
        assert_eq!(
            file_to_key_entry(Path::new("m@example.com"))
                .unwrap()
                .unwrap(),
            (
                "m".to_string(),
                CertKey {
                    hashed_username: "pcgudogicctdyjg4eiwtmbdr8mda3fze".to_string(),
                    domain: "example.com".to_string()
                }
            )
        );
        assert_eq!(
            file_to_key_entry(Path::new("hello.world@domain"))
                .unwrap()
                .unwrap(),
            (
                "hello.world".to_string(),
                CertKey {
                    hashed_username: "nsaw3ax9dxhjee85afxziy7i79oxx6rh".to_string(),
                    domain: "domain".to_string()
                }
            )
        );
        assert_eq!(
            file_to_key_entry(Path::new("hello.world@sub.domain-asdf.com"))
                .unwrap()
                .unwrap(),
            (
                "hello.world".to_string(),
                CertKey {
                    hashed_username: "nsaw3ax9dxhjee85afxziy7i79oxx6rh".to_string(),
                    domain: "sub.domain-asdf.com".to_string()
                }
            )
        );
    }

    #[test]
    fn file_path_absolute() {
        assert_eq!(
            file_to_key_entry(Path::new("/tmp/asd/.test/hello.world@domain"))
                .unwrap()
                .unwrap(),
            (
                "hello.world".to_string(),
                CertKey {
                    hashed_username: "nsaw3ax9dxhjee85afxziy7i79oxx6rh".to_string(),
                    domain: "domain".to_string()
                }
            )
        );
    }

    #[test]
    fn file_path_empty() {
        assert!(file_to_key_entry(Path::new("/")).is_err());
    }

    #[test]
    fn file_invalid_name() {
        assert!(file_to_key_entry(Path::new("hello.txt")).unwrap().is_none());
        assert!(file_to_key_entry(Path::new("hello@asd@@@@"))
            .unwrap()
            .is_none());
    }
}
