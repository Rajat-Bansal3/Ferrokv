use std::sync::atomic::{AtomicU64, AtomicUsize};

#[repr(align(64))]
pub struct StoreStats {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub expired_keys: AtomicU64,
    pub evicted_keys: AtomicU64,
    pub total_commands: AtomicU64,
    pub used_memory: AtomicUsize,
    pub total_keys: AtomicUsize,
    pub keys_with_ttl: AtomicUsize,
}

pub struct StoreStatsSnapshot {
    pub hits: u64,
    pub misses: u64,
    pub expired_keys: u64,
    pub evicted_keys: u64,
    pub total_commands: u64,
    pub used_memory: usize,
    pub total_keys: usize,
    pub keys_with_ttl: usize,
    pub hit_ratio: f64,
}

impl Default for StoreStats {
    fn default() -> Self {
        Self {
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            expired_keys: AtomicU64::new(0),
            evicted_keys: AtomicU64::new(0),
            total_commands: AtomicU64::new(0),
            used_memory: AtomicUsize::new(0),
            total_keys: AtomicUsize::new(0),
            keys_with_ttl: AtomicUsize::new(0),
        }
    }
}

impl StoreStats {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn snapshot(&self) -> StoreStatsSnapshot {
        let hits = self.hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.misses.load(std::sync::atomic::Ordering::Relaxed);
        let hit_ratio = if hits + misses == 0 {
            0.0
        } else {
            hits as f64 / (hits as f64 + misses as f64)
        };

        StoreStatsSnapshot {
            hits: self.hits.load(std::sync::atomic::Ordering::Relaxed),
            misses: self.misses.load(std::sync::atomic::Ordering::Relaxed),
            expired_keys: self.expired_keys.load(std::sync::atomic::Ordering::Relaxed),
            evicted_keys: self.evicted_keys.load(std::sync::atomic::Ordering::Relaxed),
            total_commands: self
                .total_commands
                .load(std::sync::atomic::Ordering::Relaxed),
            used_memory: self.used_memory.load(std::sync::atomic::Ordering::Relaxed),
            total_keys: self.total_keys.load(std::sync::atomic::Ordering::Relaxed),
            keys_with_ttl: self
                .keys_with_ttl
                .load(std::sync::atomic::Ordering::Relaxed),
            hit_ratio: hit_ratio,
        }
    }
}
