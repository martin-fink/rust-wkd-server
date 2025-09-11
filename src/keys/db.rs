use crate::keys::fs::read_key_file;
use anyhow::{Context, Result, bail};
use notify::event::{CreateKind, ModifyKind, RemoveKind};
use notify::{EventKind, RecommendedWatcher, Watcher};
use sequoia_openpgp::Cert;
use sequoia_openpgp::serialize::SerializeInto;
use std::collections::HashMap;
use std::ffi::OsString;
use std::hash::Hash;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::channel;
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

type Cache = HashMap<CertKey, CertEntry>;

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

        let (tx, rx) = channel();

        let inner_cache = cache.clone();
        let mut watcher = notify::recommended_watcher(tx)?;

        watcher.watch(key_path, notify::RecursiveMode::Recursive)?;

        task::spawn(async move {
            while let Ok(event) = rx.recv() {
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

        info!("Populated db with {} keys", lock.len());

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
            cache.insert(entry, content);
        });
        Ok(())
    }

    fn remove_file_from_cache(cache: &mut RwLockWriteGuard<Cache>, path: &Path) -> Result<()> {
        // we remove all items that were inserted into the map because of this file.
        cache.retain(|_, v| v.path != path.as_os_str());
        Ok(())
    }

    pub async fn get(
        &self,
        hash: &str,
        domain: &str,
        username: Option<&String>,
    ) -> Result<Option<Vec<u8>>> {
        let value = self
            .keys
            .read()
            .await
            .get(&CertKey {
                hashed_username: hash.to_string(),
                domain: domain.to_string(),
            })
            .cloned();

        match (username, value) {
            (Some(requested), Some(CertEntry { username, .. })) if requested != &username => {
                info!(
                    "hash matched for '{username}@{domain}', but requested local part '{requested}' did not match. Ignoring."
                );
                Ok(None)
            }
            (_, value) => Ok(value.map(|entry| entry.cert.to_vec().unwrap())),
        }
    }
}
