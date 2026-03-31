use std::sync::atomic::AtomicU64;

use crate::{StorageError, StorageResult};

pub struct Memory {
    pub used: AtomicU64,
    pub max: u64,
}

impl Memory {
    pub fn new(max_memory: u64) -> Self {
        Memory {
            used: AtomicU64::new(0),
            max: max_memory,
        }
    }
    pub fn alloc(&self, used: u64) -> StorageResult<()> {
        if self.max == 0 {
            return Ok(());
        }
        let current = self.used.load(std::sync::atomic::Ordering::Relaxed);
        if current + used > self.max {
            return Err(StorageError::OutOfMemory);
        }
        self.used
            .fetch_add(used, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
    pub fn free(&self, size: u64) {
        let curr = self.used.load(std::sync::atomic::Ordering::Relaxed);
        let new = curr.saturating_sub(size);
        self.used.store(new, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn usage_ratio(&self) -> StorageResult<f64> {
        if self.max == 0 {
            return Ok(0.0f64);
        }
        Ok(self.used.load(std::sync::atomic::Ordering::Relaxed) as f64 / self.max as f64)
    }
    pub fn is_over_limit(&self) -> StorageResult<bool> {
        Ok(self.usage_ratio()? >= 1.0f64)
    }
    pub fn memory_used(&self) -> u64 {
        self.used.load(std::sync::atomic::Ordering::Relaxed)
    }
}
