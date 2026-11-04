use crate::keys::db::{CertEntry, CertKey};
use crate::keys::hash;
use anyhow::{Context, Result, bail};
use openpgp::cert::CertParser;
use sequoia_openpgp as openpgp;
use sequoia_openpgp::Cert;
use sequoia_openpgp::parse::Parse;
use sequoia_openpgp::policy::StandardPolicy;
use std::path::Path;
use tracing::warn;

pub fn read_key_file(path: &Path, split_keys: bool) -> Result<Vec<(CertKey, CertEntry)>> {
    let p = StandardPolicy::new();

    let mut certs = Vec::new();

    for cert in read_certs(path)? {
        if cert.is_tsk() {
            bail!(
                "file {} contains secret key material",
                path.to_string_lossy()
            );
        }

        let cert = cert.with_policy(&p, None).context("invalid certificate")?;

        for userid in cert.userids() {
            let Some(email) = userid
                .userid()
                .email()
                .context("user id does not have a valid email")?
            else {
                warn!(
                    "user id {} does not have an email, skipping",
                    userid.userid()
                );
                continue;
            };

            let Some((username, cert_key)) = hash::mail_to_key_entry(email)? else {
                bail!("could not hash {email}");
            };

            let mut cert = userid.cert().clone().strip_secret_key_material();
            if split_keys {
                cert = cert.retain_userids(|uid| uid.userid() == userid.userid());
            }

            let cert_entry = CertEntry {
                username,
                cert,
                path: path.as_os_str().into(),
            };

            certs.push((cert_key, cert_entry));
        }
    }

    Ok(certs)
}

fn read_certs(path: &Path) -> Result<Vec<Cert>> {
    if !path.exists() || !path.is_file() {
        bail!("File {} not found or not a file", path.to_string_lossy());
    }

    let content = std::fs::read(path)?;

    CertParser::from_bytes(&content)?
        .collect::<openpgp::Result<Vec<_>>>()
        .context("failed to parse certificate data")
}

#[cfg(test)]
mod tests {
    use super::read_key_file;
    use crate::keys::hash::mail_to_key_entry;
    use anyhow::Result;
    use sequoia_openpgp::cert::CertBuilder;
    use sequoia_openpgp::serialize::SerializeInto;
    use tempfile::TempDir;

    fn write_concatenated_certs(tempdir: &TempDir, file_name: &str, email: &str) -> Result<()> {
        let mut content = Vec::new();

        for _ in 0..2 {
            let (cert, _) = CertBuilder::general_purpose(Some(email)).generate()?;
            content.extend(cert.strip_secret_key_material().armored().to_vec()?);
        }

        std::fs::write(tempdir.path().join(file_name), content)?;
        Ok(())
    }

    #[test]
    fn reads_multiple_concatenated_public_certs() -> Result<()> {
        let tempdir = TempDir::new()?;
        let path = tempdir.path().join("alice@example.org.asc");

        write_concatenated_certs(&tempdir, "alice@example.org.asc", "alice@example.org")?;

        let entries = read_key_file(&path, false)?;
        assert_eq!(entries.len(), 2);

        let (_, expected_key) = mail_to_key_entry("alice@example.org")?.unwrap();
        assert!(entries.iter().all(|(key, _)| key == &expected_key));

        let mut fingerprints = entries
            .iter()
            .map(|(_, entry)| entry.cert.fingerprint().to_string())
            .collect::<Vec<_>>();
        fingerprints.sort();
        fingerprints.dedup();
        assert_eq!(fingerprints.len(), 2);

        Ok(())
    }
}
