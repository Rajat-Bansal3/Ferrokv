use std::time::Instant;

use bytes::Bytes;
use rand::RngExt;

use crate::StoreValue;

#[derive(Clone)]
#[repr(C)]
pub struct Entry {
    pub value: StoreValue,
    pub expired_at: Option<Instant>,
    pub last_accessed: Instant,
    pub access_count: u32,
    pub size_bytes: usize,
}

impl Entry {
    pub fn is_expired(&self) -> bool {
        match self.expired_at {
            Some(exp) => exp <= Instant::now(),
            None => false,
        }
    }
    pub fn touch(&mut self) {
        self.last_accessed = Instant::now();
        if self.access_count == 0 {
            self.access_count = 1;
            return;
        }
        let mut rng = rand::rng();
        let threshold = self.access_count;
        if rng.random_range(0..threshold) == 0 {
            self.access_count += 1;
        }
    }
    pub fn memory_size(key: &Bytes, value: &StoreValue) -> usize {
        let value_size = match value {
            StoreValue::Bytes(b) => b.len(),
            StoreValue::Integer(_) => std::mem::size_of::<i64>(),
        };
        std::mem::size_of::<Entry>() + key.len() + value_size
    }
}
