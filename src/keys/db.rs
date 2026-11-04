use crate::keys::fs::read_key_file;
use anyhow::{Context, Result, bail};
use notify::event::{CreateKind, ModifyKind, RemoveKind};
use notify::{EventKind, RecommendedWatcher, Watcher};
use sequoia_openpgp::serialize::SerializeInto;
use sequoia_openpgp::{Cert, Fingerprint};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::hash::Hash;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::{RwLock, RwLockWriteGuard};
use tokio::{fs, task};
use tracing::{debug, error, info};

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct CertKey {
    pub hashed_username: String,
    pub domain: String,
}

#[derive(Clone, Debug)]
pub struct CertEntry {
    pub username: String,
    pub cert: Cert,
    pub path: OsString,
}

type Cache = HashMap<CertKey, Vec<CertEntry>>;

pub struct KeyDb {
    _watcher: RecommendedWatcher,
    keys: Arc<RwLock<Cache>>,
}

impl KeyDb {
    pub async fn new(key_path: &Path, split_keys: bool) -> Result<Self> {
        if !key_path.exists() || !key_path.is_dir() {
            bail!("Key path not found");
        }

        let cache = Arc::new(RwLock::new(HashMap::new()));

        let (tx, mut rx) = unbounded_channel();

        let inner_cache = cache.clone();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = tx.send(event);
        })?;

        watcher.watch(key_path, notify::RecursiveMode::Recursive)?;

        task::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    Ok(event) => {
                        debug!("event: {:?}", event);
                        if let Err(e) =
                            Self::handle_file_event(&inner_cache, event, split_keys).await
                        {
                            error!("Error while handling file event: {:?}", e);
                        }
                    }
                    Err(error) => error!("watch error: {:?}", error),
                }
            }
        });

        let mut db = Self {
            _watcher: watcher,
            keys: cache,
        };

        db.populate(key_path, split_keys).await?;

        Ok(db)
    }

    async fn populate(&mut self, key_path: &Path, split_keys: bool) -> Result<()> {
        info!("Populating keys db, key splitting enabled: {split_keys}");

        let mut read_dir = fs::read_dir(key_path).await?;

        let mut lock = self.keys.write().await;

        while let Some(file) = read_dir.next_entry().await? {
            if let Err(e) = Self::cache_file(&mut lock, &file.path(), split_keys) {
                error!("error caching file: {:?}", e);
            }
        }

        info!(
            "Populated db with {} keys",
            lock.values().map(Vec::len).sum::<usize>()
        );

        Ok(())
    }

    async fn handle_file_event(
        cache: &RwLock<Cache>,
        event: notify::Event,
        split_keys: bool,
    ) -> Result<()> {
        match event.kind {
            EventKind::Create(CreateKind::File) => {
                for path in event.paths {
                    Self::cache_file(&mut cache.write().await, &path, split_keys)?;
                }
            }
            EventKind::Modify(ModifyKind::Data(_)) => {
                for path in event.paths {
                    Self::cache_file(&mut cache.write().await, &path, split_keys)?;
                }
            }
            EventKind::Modify(ModifyKind::Name(_)) => {
                let mut lock = cache.write().await;

                for path in event.paths {
                    if let Ok(true) = fs::try_exists(&path).await {
                        Self::cache_file(&mut lock, &path, split_keys)?;
                    } else {
                        Self::remove_file_from_cache(&mut lock, &path)?;
                    }
                }
            }
            EventKind::Remove(RemoveKind::File) => {
                for path in event.paths {
                    Self::remove_file_from_cache(&mut cache.write().await, &path)?;
                }
            }
            _ => { /* ignore */ }
        }

        Ok(())
    }

    fn cache_file(
        cache: &mut RwLockWriteGuard<Cache>,
        path: &Path,
        split_keys: bool,
    ) -> Result<()> {
        // first, we remove all files that might be in here still because of this path
        Self::remove_file_from_cache(cache, path)?;

        let entries = read_key_file(path, split_keys).context("Reading file")?;
        if entries.is_empty() {
            info!("Ignoring file {}, no entries found", path.to_string_lossy());
            return Ok(());
        }
        entries.into_iter().for_each(|(entry, content)| {
            info!(
                "Adding key '{}@{}' from file {} to db",
                content.username,
                entry.domain,
                path.to_string_lossy()
            );
            cache.entry(entry).or_default().push(content);
        });
        Ok(())
    }

    fn remove_file_from_cache(cache: &mut RwLockWriteGuard<Cache>, path: &Path) -> Result<()> {
        // we remove all items that were inserted into the map because of this file.
        cache.retain(|_, entries| {
            entries.retain(|entry| entry.path != path.as_os_str());
            !entries.is_empty()
        });
        Ok(())
    }

    pub async fn get(
        &self,
        hash: &str,
        domain: &str,
        username: Option<&String>,
    ) -> Result<Option<Vec<u8>>> {
        let values = self
            .keys
            .read()
            .await
            .get(&CertKey {
                hashed_username: hash.to_string(),
                domain: domain.to_string(),
            })
            .cloned();

        let Some(values) = values else {
            return Ok(None);
        };

        let values = if let Some(requested) = username {
            let filtered = values
                .into_iter()
                .filter(|entry| &entry.username == requested)
                .collect::<Vec<_>>();

            if filtered.is_empty() {
                info!(
                    "hash matched for domain '{domain}', but requested local part '{requested}' did not match. Ignoring."
                );
                return Ok(None);
            }

            filtered
        } else {
            values
        };

        Ok(Some(Self::serialize_entries(values)?))
    }

    fn serialize_entries(entries: Vec<CertEntry>) -> Result<Vec<u8>> {
        let mut certs: BTreeMap<Fingerprint, Cert> = BTreeMap::new();

        for entry in entries {
            let fingerprint = entry.cert.fingerprint();
            match certs.entry(fingerprint) {
                Entry::Occupied(e) => {
                    let (fingerprint, cert) = e.remove_entry();
                    certs.insert(fingerprint, cert.merge_public(entry.cert)?);
                }
                Entry::Vacant(e) => {
                    e.insert(entry.cert);
                }
            }
        }

        let mut output = Vec::new();

        for (_, cert) in certs {
            output.extend(cert.to_vec()?);
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::KeyDb;
    use crate::keys::hash::mail_to_key_entry;
    use anyhow::Result;
    use sequoia_openpgp as openpgp;
    use sequoia_openpgp::cert::{CertBuilder, CertParser};
    use sequoia_openpgp::parse::Parse;
    use sequoia_openpgp::serialize::SerializeInto;
    use std::path::Path;
    use tempfile::TempDir;

    fn write_public_cert(path: &Path, email: &str) -> Result<String> {
        let (cert, _) = CertBuilder::general_purpose(Some(email)).generate()?;
        let cert = cert.strip_secret_key_material();
        let fingerprint = cert.fingerprint().to_string();
        std::fs::write(path, cert.armored().to_vec()?)?;
        Ok(fingerprint)
    }

    fn parse_fingerprints(content: &[u8]) -> Result<Vec<String>> {
        let mut fingerprints = CertParser::from_bytes(content)?
            .collect::<openpgp::Result<Vec<_>>>()?
            .into_iter()
            .map(|cert| cert.fingerprint().to_string())
            .collect::<Vec<_>>();
        fingerprints.sort();
        Ok(fingerprints)
    }

    #[tokio::test]
    async fn merges_same_address_entries_from_multiple_files() -> Result<()> {
        let tempdir = TempDir::new()?;
        let email = "alice@example.org";
        let path_one = tempdir.path().join("alice-1.asc");
        let path_two = tempdir.path().join("alice-2.asc");

        let fingerprint_one = write_public_cert(&path_one, email)?;
        let fingerprint_two = write_public_cert(&path_two, email)?;

        let db = KeyDb::new(tempdir.path(), false).await?;
        let (username, key) = mail_to_key_entry(email)?.unwrap();

        let content = db
            .get(&key.hashed_username, &key.domain, Some(&username))
            .await?
            .expect("key data should exist");

        let mut expected = vec![fingerprint_one, fingerprint_two];
        expected.sort();
        assert_eq!(parse_fingerprints(&content)?, expected);

        Ok(())
    }

    #[tokio::test]
    async fn serves_cert_once_with_multiple_user_ids_for_same_address() -> Result<()> {
        let tempdir = TempDir::new()?;
        let email = "alice@example.org";

        let (cert, _) = CertBuilder::general_purpose(Some(format!("Alice <{email}>")))
            .add_userid(format!("Alice (work) <{email}>"))
            .generate()?;
        let cert = cert.strip_secret_key_material();
        let fingerprint = cert.fingerprint().to_string();
        std::fs::write(tempdir.path().join("alice.asc"), cert.armored().to_vec()?)?;
        // the same cert in a second file must not be served twice either
        std::fs::write(
            tempdir.path().join("alice-copy.asc"),
            cert.armored().to_vec()?,
        )?;

        let (username, key) = mail_to_key_entry(email)?.unwrap();

        for split_keys in [false, true] {
            let db = KeyDb::new(tempdir.path(), split_keys).await?;

            let content = db
                .get(&key.hashed_username, &key.domain, Some(&username))
                .await?
                .expect("key data should exist");

            assert_eq!(parse_fingerprints(&content)?, vec![fingerprint.clone()]);
        }

        Ok(())
    }
}
