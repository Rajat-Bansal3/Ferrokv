use std::{
    collections::HashMap,
    sync::{RwLock, atomic::AtomicU64},
};

use ahash::RandomState;
use bytes::Bytes;

use crate::{Entry, StorageError, StorageResult, StoreStats, StoreStatsSnapshot};

#[repr(align(64))]
pub struct Shard {
    pub data: RwLock<HashMap<Bytes, Entry, RandomState>>,
    pub length: AtomicU64,
    pub stats: StoreStats,
}

impl Default for Shard {
    fn default() -> Self {
        Self::new(128)
    }
}
impl Shard {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: RwLock::new(HashMap::with_capacity_and_hasher(
                capacity,
                RandomState::new(),
            )),
            stats: StoreStats::new(),
            length: AtomicU64::new(0),
        }
    }
    pub fn get(&self, key: &Bytes) -> StorageResult<Option<Entry>> {
        let map = self.data.read().map_err(|_| StorageError::ShardPoisoned)?;

        match map.get(key) {
            None => {
                self.stats
                    .misses
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(None)
            }
            Some(entry) => {
                if entry.is_expired() {
                    self.stats
                        .misses
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    self.stats
                        .expired_keys
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return Ok(None);
                }
                self.stats
                    .hits
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(Some(entry.clone()))
            }
        }
    }
    pub fn get_raw(&self, key: &Bytes) -> Option<Entry> {
        self.data.read().unwrap().get(key).cloned()
    }
    pub fn set(&self, key: &Bytes, entry: Entry) -> StorageResult<bool> {
        let mut map = self.data.write().map_err(|_| StorageError::ShardPoisoned)?;

        let has_ttl = entry.expired_at.is_some();
        let is_new_key = !map.contains_key(key);
        map.insert(key.clone(), entry);
        if is_new_key {
            self.length
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.stats
                .total_keys
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        if has_ttl {
            self.stats
                .keys_with_ttl
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        Ok(is_new_key)
    }
    pub fn del(&self, key: &Bytes) -> StorageResult<Option<Entry>> {
        let mut map = self.data.write().map_err(|_| StorageError::ShardPoisoned)?;

        match map.remove(key) {
            None => Ok(None),
            Some(entry) => {
                self.length
                    .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                self.stats
                    .total_keys
                    .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                if entry.expired_at.is_some() {
                    self.stats
                        .keys_with_ttl
                        .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                }
                Ok(Some(entry))
            }
        }
    }
    pub fn exists(&self, key: &Bytes) -> StorageResult<bool> {
        let map = self.data.read().map_err(|_| StorageError::ShardPoisoned)?;
        match map.get(key) {
            None => Ok(false),
            Some(entry) => Ok(!entry.is_expired()),
        }
    }
    pub fn flush(&self) -> StorageResult<()> {
        let mut map = self.data.write().map_err(|_| StorageError::ShardPoisoned)?;
        map.clear();
        self.length.store(0, std::sync::atomic::Ordering::Relaxed);
        self.stats
            .total_keys
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.stats
            .keys_with_ttl
            .store(0, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.length.load(std::sync::atomic::Ordering::Relaxed) as usize
    }
    pub fn keys(&self) -> StorageResult<Vec<Bytes>> {
        let shard_keys: Vec<Bytes> = self
            .data
            .read()
            .map_err(|_| StorageError::ShardPoisoned)?
            .keys()
            .cloned()
            .collect();
        Ok(shard_keys)
    }
    pub fn random_key(&self) -> StorageResult<Option<Bytes>> {
        let map = self.data.read().map_err(|_| StorageError::ShardPoisoned)?;
        if map.len() == 0 {
            return Ok(None);
        }
        let random = fastrand::usize(..map.len());
        Ok(map.keys().nth(random).cloned())
    }
    pub fn snapshot_stats(&self) -> StoreStatsSnapshot {
        self.stats.snapshot()
    }
}
