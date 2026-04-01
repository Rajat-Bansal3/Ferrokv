use std::time::Duration;

use bytes::Bytes;

use crate::{StorageError, StoreStatsSnapshot};
pub type StorageResult<T> = Result<T, StorageError>;
#[derive(Clone)]
pub enum StoreValue {
    Integer(i64),
    Bytes(Bytes),
}

pub trait Store: Send + Sync {
    fn get(&self, key: &Bytes) -> StorageResult<Option<StoreValue>>;
    fn set(&self, key: Bytes, value: StoreValue, ttl: Option<Duration>) -> StorageResult<()>;
    fn del(&self, key: &Bytes) -> StorageResult<bool>;
    fn exists(&self, key: &Bytes) -> StorageResult<bool>;
    fn ttl(&self, key: &Bytes) -> StorageResult<Option<Duration>>;
    fn persist(&self, key: &Bytes) -> StorageResult<bool>;
    fn flush(&self);
    fn len(&self) -> usize;
    fn keys(&self) -> StorageResult<Vec<Bytes>>;
    fn stats(&self) -> StoreStatsSnapshot;
}

impl StoreValue {
    pub fn from_bytes(raw: Bytes) -> StoreValue {
        match std::str::from_utf8(&raw)
            .ok()
            .and_then(|s| s.parse::<i64>().ok())
        {
            Some(int) => StoreValue::Integer(int),
            None => StoreValue::Bytes(raw),
        }
    }
    pub fn to_bytes(self) -> Bytes {
        match self {
            StoreValue::Integer(n) => Bytes::from(n.to_string()),
            StoreValue::Bytes(bytes) => bytes,
        }
    }
    pub fn get_size(&self) -> usize {
        match self {
            Self::Bytes(b) => b.len(),
            Self::Integer(_) => size_of::<i64>(),
        }
    }
}
