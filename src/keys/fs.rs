use crate::keys::db::{CertEntry, CertKey};
use crate::keys::hash;
use anyhow::{Context, Result, bail};
use openpgp::armor::{Kind, Reader, ReaderMode};
use sequoia_openpgp as openpgp;
use sequoia_openpgp::Cert;
use sequoia_openpgp::parse::Parse;
use sequoia_openpgp::policy::StandardPolicy;
use std::io::BufReader;
use std::path::Path;
use tracing::warn;

pub fn read_key_file(path: &Path, split_keys: bool) -> Result<Vec<(CertKey, CertEntry)>> {
    let Some(cert) = read_cert(path)? else {
        return Ok(vec![]);
    };

    let p = StandardPolicy::new();
    let cert = cert.with_policy(&p, None).context("invalid certificate")?;

    let mut certs = Vec::new();

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

    Ok(certs)
}

fn read_cert(path: &Path) -> Result<Option<Cert>> {
    if !path.exists() || !path.is_file() {
        bail!("File {} not found or not a file", path.to_string_lossy());
    }

    let content = std::fs::read(path)?;

    // Validate the public key, tolerate common formatting errors such as erroneous
    // whitespace, but fail on private keys
    let reader = BufReader::new(Reader::from_bytes(
        &content,
        ReaderMode::Tolerant(Some(Kind::PublicKey)),
    ));
    if let Ok(cert) = Cert::from_reader(reader) {
        Ok(Some(cert))
    } else {
        Ok(None)
    }
}
