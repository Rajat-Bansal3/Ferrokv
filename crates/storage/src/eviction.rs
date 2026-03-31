use std::time::Instant;

use bytes::Bytes;
use config::EvictionPolicy;
use rand::seq::{IndexedRandom, index::sample};

use crate::{Shard, StorageResult};

pub struct Eviction {
    pub policy: EvictionPolicy,
    pub sample_size: u64,
}
impl Eviction {
    pub fn new(policy: EvictionPolicy, sample_size: u64) -> Self {
        Eviction {
            policy,
            sample_size,
        }
    }
    pub fn evict(&self, shards: &[Shard]) -> StorageResult<Option<Bytes>> {
        match self.policy {
            EvictionPolicy::NoEviction => Ok(None),
            EvictionPolicy::AllKeysLfu => Ok(self.lfu(shards, false)),
            EvictionPolicy::VolatileLfu => Ok(self.lfu(shards, true)),
            EvictionPolicy::AllKeysLru => Ok(self.lru(shards, false)),
            EvictionPolicy::VolatileLru => Ok(self.lru(shards, true)),
            EvictionPolicy::AllKeysRandom => Ok(self.random(shards, false)),
            EvictionPolicy::VolatileRandom => Ok(self.random(shards, true)),
            EvictionPolicy::VolatileTtl => Ok(self.ttl(shards)),
        }
    }

    pub fn lru(&self, shards: &[Shard], is_volatile: bool) -> Option<Bytes> {
        let mut rng = rand::rng();
        let mut candidates: Vec<(Bytes, Instant)> = Vec::with_capacity(self.sample_size as usize);
        let shard_idxs = sample(
            &mut rng,
            shards.len(),
            self.sample_size.min(shards.len() as u64) as usize,
        );
        for idx in shard_idxs {
            let shard = &shards[idx];
            if let Ok(Some(key)) = shard.random_key() {
                if let Ok(Some(entry)) = shard.get(&key) {
                    if is_volatile && entry.expired_at.is_none() {
                        continue;
                    }
                    candidates.push((key, entry.last_accessed));
                }
            }
        }
        if candidates.is_empty() {
            return None;
        }
        candidates
            .into_iter()
            .min_by_key(|(_, access)| *access)
            .map(|(key, _)| key)
    }
    pub fn lfu(&self, shards: &[Shard], is_volatile: bool) -> Option<Bytes> {
        let mut rng = rand::rng();
        let mut candidates: Vec<(Bytes, u32)> = Vec::with_capacity(self.sample_size as usize);
        let shard_idxs = sample(
            &mut rng,
            shards.len(),
            self.sample_size.min(shards.len() as u64) as usize,
        );
        for idx in shard_idxs {
            let shard = &shards[idx];
            if let Ok(Some(key)) = shard.random_key() {
                if let Ok(Some(entry)) = shard.get(&key) {
                    if is_volatile && entry.expired_at.is_none() {
                        continue;
                    }
                    candidates.push((key, entry.access_count));
                }
            }
        }
        if candidates.is_empty() {
            return None;
        }
        candidates
            .into_iter()
            .min_by_key(|(_, access)| *access)
            .map(|(key, _)| key)
    }
    pub fn random(&self, shards: &[Shard], is_volatile: bool) -> Option<Bytes> {
        let mut rng = rand::rng();
        let mut candidates: Vec<Bytes> = Vec::with_capacity(self.sample_size as usize);
        let shard_idxs = sample(
            &mut rng,
            shards.len(),
            self.sample_size.min(shards.len() as u64) as usize,
        );
        for idx in shard_idxs {
            let shard = &shards[idx];
            if let Ok(Some(key)) = shard.random_key() {
                if let Ok(Some(entry)) = shard.get(&key) {
                    if is_volatile && entry.expired_at.is_none() {
                        continue;
                    }
                    candidates.push(key);
                }
            }
        }
        if candidates.is_empty() {
            return None;
        }
        candidates.choose(&mut rng).cloned()
    }
    pub fn ttl(&self, shards: &[Shard]) -> Option<Bytes> {
        let mut rng = rand::rng();
        let mut candidates: Vec<(Bytes, Instant)> = Vec::with_capacity(self.sample_size as usize);
        let shard_idxs = sample(
            &mut rng,
            shards.len(),
            self.sample_size.min(shards.len() as u64) as usize,
        );
        for idx in shard_idxs {
            let shard = &shards[idx];
            if let Ok(Some(key)) = shard.random_key() {
                if let Ok(Some(entry)) = shard.get(&key) {
                    if let Some(expired_at) = entry.expired_at {
                        candidates.push((key, expired_at));
                    }
                }
            }
        }
        if candidates.is_empty() {
            return None;
        }
        candidates
            .into_iter()
            .min_by_key(|(_, access)| *access)
            .map(|(key, _)| key)
    }
}
