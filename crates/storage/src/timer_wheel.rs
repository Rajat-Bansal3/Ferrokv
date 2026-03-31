use std::{sync::Mutex, time::Instant};

use ahash::{HashSet, HashSetExt};
use bytes::Bytes;

pub struct Inner {
    pub buckets: Vec<HashSet<Bytes>>,
    pub cur_idx: usize,
}
pub struct Timer {
    pub inner: Mutex<Inner>,
    pub resolution: u64,
    pub total_buckets: usize,
    pub start_time: Instant,
}

impl Timer {
    pub fn new(resolution: u64, total_buckets: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                buckets: vec![HashSet::new(); total_buckets],
                cur_idx: 0,
            }),
            resolution,
            total_buckets,
            start_time: Instant::now(),
        }
    }
    fn bucket_for(&self, expires_at: Instant) -> usize {
        let elapsed = expires_at
            .checked_duration_since(self.start_time)
            .unwrap_or_default()
            .as_millis();
        ((elapsed / self.resolution as u128) % self.total_buckets as u128) as usize
    }
    pub fn insert(&self, key: Bytes, expires_at: Instant) {
        let slot = self.bucket_for(expires_at);
        self.inner.lock().unwrap().buckets[slot].insert(key);
    }
    pub fn remove(&self, key: &Bytes, expires_at: Instant) {
        let slot = self.bucket_for(expires_at);
        self.inner.lock().unwrap().buckets[slot].remove(key);
    }
    pub fn advance(&self) -> Vec<Bytes> {
        let mut inner = self.inner.lock().unwrap();
        let idx = inner.cur_idx;
        let keys: Vec<Bytes> = inner.buckets[idx].drain().collect();
        inner.cur_idx = (inner.cur_idx + 1) % self.total_buckets;
        keys
    }
    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .unwrap()
            .buckets
            .iter()
            .map(|set| set.len())
            .sum()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
