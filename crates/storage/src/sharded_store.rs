use std::{
    hash::{Hash, Hasher},
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

use bytes::Bytes;
use config::StorageConfig;

use crate::{
    Entry, Eviction, Memory, Shard, StorageError, StorageResult, Store, StoreStats,
    StoreStatsSnapshot, StoreValue, Timer,
};

pub struct ShardedStore {
    pub shards: Vec<Shard>,
    pub shard_count: usize,
    pub memory: Memory,
    pub timer: Timer,
    pub evictor: Eviction,
    pub stats: Arc<StoreStats>,
    pub config: StorageConfig,
}

impl Store for ShardedStore {
    fn get(&self, key: &bytes::Bytes) -> StorageResult<Option<StoreValue>> {
        let shard = self.shard(key);

        match shard.get(key)? {
            Some(mut etry) => {
                etry.get_touch();
                Ok(Some(etry.value))
            }
            None => Ok(None),
        }
    }

    fn exists(&self, key: &bytes::Bytes) -> StorageResult<bool> {
        let shard = self.shard(key);
        Ok(shard.exists(key)?)
    }

    fn keys(&self) -> StorageResult<Vec<Bytes>> {
        let mut keys: Vec<Bytes> = Vec::new();
        for shard in self.shards.iter() {
            keys.extend(shard.keys()?);
        }
        Ok(keys)
    }

    fn len(&self) -> usize {
        let mut len: usize = 0;
        for shard in self.shards.iter() {
            len += shard.length.load(Ordering::Relaxed) as usize;
        }
        len
    }

    fn ttl(&self, key: &bytes::Bytes) -> StorageResult<Option<std::time::Duration>> {
        let shard = self.shard(key);
        match shard.get_raw(key) {
            Some(entry) => match entry.expired_at {
                Some(expiry) => {
                    let ttl = expiry.saturating_duration_since(Instant::now());
                    Ok(Some(ttl))
                }
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    fn set(
        &self,
        key: bytes::Bytes,
        value: crate::StoreValue,
        ttl: Option<std::time::Duration>,
    ) -> crate::StorageResult<()> {
        let size = value.get_size();
        let expires_at = ttl.map(|d| Instant::now() + d);
        self.check_memory_and_evict(size)?;
        self.memory.alloc(size as u64)?;
        let entry = Entry {
            access_count: 0,
            expired_at: expires_at,
            last_accessed: Instant::now(),
            size_bytes: size,
            value: value,
        };

        let is_new = self.shard(&key).set(&key, entry)?;
        if let Some(exp) = expires_at {
            self.timer.insert(key, exp);
        }
        if is_new {
            self.stats.total_keys.fetch_add(1, Ordering::Relaxed);
        }
        self.stats.total_commands.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn del(&self, key: &bytes::Bytes) -> StorageResult<bool> {
        let shard = self.shard(key);

        match shard.del(key)? {
            None => Ok(false),
            Some(entry) => {
                self.memory.free(entry.size_bytes as u64);
                if let Some(exp) = entry.expired_at {
                    self.timer.remove(key, exp);
                }
                self.stats.total_keys.fetch_sub(1, Ordering::Relaxed);
                Ok(true)
            }
        }
    }

    fn persist(&self, key: &bytes::Bytes) -> StorageResult<bool> {
        let shard = self.shard(key);
        let mut map = shard
            .data
            .write()
            .map_err(|_| StorageError::ShardPoisoned)?;
        match map.get_mut(key) {
            None => Ok(false),
            Some(entry) => {
                if let Some(exp) = entry.expired_at.take() {
                    self.timer.remove(key, exp);
                    self.stats.keys_with_ttl.fetch_sub(1, Ordering::Relaxed);
                }
                Ok(true)
            }
        }
    }

    fn flush(&self) {
        for shard in self.shards.iter() {
            shard.flush().unwrap();
        }
        self.stats.total_keys.store(0, Ordering::Relaxed);
        self.stats.keys_with_ttl.store(0, Ordering::Relaxed);
        self.memory.used.store(0, Ordering::Relaxed);
    }

    fn stats(&self) -> StoreStatsSnapshot {
        self.stats.snapshot()
    }
}

impl ShardedStore {
    pub fn new(config: StorageConfig) -> Arc<Self> {
        let shards = config.shard_count;
        let shard_capacity = 128;
        let store = Arc::new(ShardedStore {
            shards: (0..shards).map(|_| Shard::new(shard_capacity)).collect(),
            shard_count: shards,
            memory: Memory::new(config.maxmemory as u64),
            timer: Timer::new(100, 600),
            evictor: Eviction::new(config.eviction_policy, config.eviction_sample_size as u64),
            stats: Arc::new(StoreStats::new()),
            config: config,
        });
        store.spawn_ttl_loop();
        store
    }
    pub fn spawn_ttl_loop(self: &Arc<Self>) {
        let store = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(store.timer.resolution)).await;
                store.sweep_expired_keys().await;
            }
        });
    }
    async fn sweep_expired_keys(self: &Arc<ShardedStore>) {
        let keys = self.timer.advance();
        for key in keys {
            let shard = self.shard(&key);
            match shard.get_raw(&key) {
                Some(entry) if entry.is_expired() => {
                    shard.del(&key).ok();
                    self.memory.free(entry.size_bytes as u64);
                    self.stats.expired_keys.fetch_add(1, Ordering::Relaxed);
                    self.stats.total_keys.fetch_sub(1, Ordering::Relaxed);
                }
                Some(entry) => {
                    if let Some(expires_at) = entry.expired_at {
                        self.timer.insert(key, expires_at);
                    }
                }
                None => {}
            }
        }
    }
    fn shard_idx(&self, key: &Bytes) -> usize {
        let mut hasher = ahash::AHasher::default();
        key.hash(&mut hasher);
        hasher.finish() as usize & (self.shard_count - 1)
    }
    fn shard(&self, key: &Bytes) -> &Shard {
        &self.shards[self.shard_idx(key)]
    }
    fn check_memory_and_evict(&self, size: usize) -> StorageResult<()> {
        if self.memory.max == 0 {
            return Ok(());
        }

        let mut attempts = 0;
        while !self.memory.is_allowed(size as u64) {
            if attempts >= 10 {
                return Err(StorageError::EvictionFailed);
            }
            match self.evictor.evict(&self.shards)? {
                None => return Err(StorageError::OutOfMemory),
                Some(key) => {
                    let shard = self.shard(&key);
                    if let Some(entry) = shard.get_raw(&key) {
                        shard.del(&key).ok();
                        self.memory.free(entry.size_bytes as u64);
                        self.stats.evicted_keys.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
            attempts += 1;
        }
        Ok(())
    }
}
