use std::time::Duration;

use bytes::Bytes;

use crate::{StorageError, StoreStats};
pub type StorageResult<T> = Result<T, StorageError>;
#[derive(Clone)]
pub enum StoreValue {
    Integer(i64),
    Bytes(Bytes),
}

pub trait Store: Send + Sync {
    fn get(&self, key: &Bytes) -> Option<StoreValue>;
    fn set(&self, key: Bytes, value: StoreValue, ttl: Option<Duration>) -> StorageResult<()>;
    fn del(&self, key: &Bytes) -> bool;
    fn exists(&self, key: &Bytes) -> bool;
    fn ttl(&self, key: &Bytes) -> Option<Duration>;
    fn persist(&self, key: &Bytes) -> bool;
    fn flush(&self);
    fn len(&self) -> usize;
    fn keys(&self) -> Vec<Bytes>;
    fn stats(&self) -> StoreStats;
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
}
