use crate::keys::db::CertKey;
use once_cell::sync::Lazy;
use regex::Regex;
use sha1::{Digest, Sha1};

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

pub fn mail_to_key_entry(email: &str) -> anyhow::Result<Option<(String, CertKey)>> {
    let Some(captures) = FILE_REGEX.captures(email) else {
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

fn hash_file_name(name: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(name.as_bytes());
    let result = hasher.finalize();
    zbase32::encode_full_bytes(&result[..])
}

#[cfg(test)]
mod tests {
    use crate::keys::db::CertKey;
    use crate::keys::hash::mail_to_key_entry;

    #[test]
    fn file_to_entry() {
        assert_eq!(
            mail_to_key_entry("m@example.com").unwrap().unwrap(),
            (
                "m".to_string(),
                CertKey {
                    hashed_username: "pcgudogicctdyjg4eiwtmbdr8mda3fze".to_string(),
                    domain: "example.com".to_string()
                }
            )
        );
        assert_eq!(
            mail_to_key_entry("hello.world@domain").unwrap().unwrap(),
            (
                "hello.world".to_string(),
                CertKey {
                    hashed_username: "nsaw3ax9dxhjee85afxziy7i79oxx6rh".to_string(),
                    domain: "domain".to_string()
                }
            )
        );
        assert_eq!(
            mail_to_key_entry("hello.world@sub.domain-asdf.com")
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
            mail_to_key_entry("hello.world@domain").unwrap().unwrap(),
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
        assert!(mail_to_key_entry("/").unwrap().is_none());
    }

    #[test]
    fn file_invalid_name() {
        assert!(mail_to_key_entry("hello@asd@ts@@@").unwrap().is_none());
    }
}
