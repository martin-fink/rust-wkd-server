use crate::keys::fs::{file_to_key_entry, read_key_file};
use anyhow::{bail, Context, Result};
use notify::event::{CreateKind, ModifyKind, RemoveKind};
use notify::{EventKind, RecommendedWatcher, Watcher};
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockWriteGuard};
use tokio::{fs, task};
use tracing::{debug, error, info};

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct KeyEntry {
    pub hashed_username: String,
    pub domain: String,
}

type Cache = HashMap<KeyEntry, Vec<u8>>;

pub struct KeyDb {
    _watcher: RecommendedWatcher,
    keys: Arc<RwLock<Cache>>,
}

impl KeyDb {
    pub async fn new(key_path: &Path) -> Result<Self> {
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
                        if let Err(e) = Self::handle_file_event(&inner_cache, event).await {
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

        db.populate(key_path).await?;

        Ok(db)
    }

    async fn populate(&mut self, key_path: &Path) -> Result<()> {
        let mut read_dir = fs::read_dir(key_path).await?;

        let mut lock = self.keys.write().await;

        while let Some(file) = read_dir.next_entry().await? {
            if let Err(e) = Self::cache_file(&mut lock, &file.path()) {
                error!("error caching file: {:?}", e);
            }
        }

        info!("Populated db with {} keys", lock.len());

        Ok(())
    }

    async fn handle_file_event(cache: &RwLock<Cache>, event: notify::Event) -> Result<()> {
        match event.kind {
            EventKind::Create(CreateKind::File) => {
                for path in event.paths {
                    Self::cache_file(&mut cache.write().await, &path)?;
                }
            }
            EventKind::Modify(ModifyKind::Data(_)) => {
                for path in event.paths {
                    Self::cache_file(&mut cache.write().await, &path)?;
                }
            }
            EventKind::Modify(ModifyKind::Name(_)) => {
                let mut lock = cache.write().await;

                for path in event.paths {
                    if let Ok(true) = fs::try_exists(&path).await {
                        Self::cache_file(&mut lock, &path)?;
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

    fn cache_file(cache: &mut RwLockWriteGuard<Cache>, path: &Path) -> Result<()> {
        let Some((entry, content)) = read_key_file(path).context("Reading file")? else {
            info!("ignoring file {}", path.to_string_lossy());
            return Ok(());
        };
        cache.insert(entry, content);
        info!("Added key {} to db", path.to_string_lossy());
        Ok(())
    }

    fn remove_file_from_cache(cache: &mut RwLockWriteGuard<Cache>, path: &Path) -> Result<()> {
        let Some(entry) = &file_to_key_entry(path).context("Ignoring file")? else {
            info!("Ignoring file {}", path.to_string_lossy());
            return Ok(());
        };
        if cache.remove(entry).is_some() {
            info!("Removed key {} from db", path.to_string_lossy());
        }
        Ok(())
    }

    pub async fn get(&self, hash: &str, domain: &str) -> Result<Option<Vec<u8>>> {
        let value = self
            .keys
            .read()
            .await
            .get(&KeyEntry {
                hashed_username: hash.to_string(),
                domain: domain.to_string(),
            })
            .cloned();
        Ok(value)
    }
}
