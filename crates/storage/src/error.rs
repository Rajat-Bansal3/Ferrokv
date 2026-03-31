use thiserror::Error;
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("out of memory")]
    OutOfMemory,
    #[error("key not found")]
    KeyNotFound,
    #[error("wrong type")]
    WrongType,
    #[error("invalid expiry")]
    InvalidExpiry,
    #[error("shard lock poisoned")]
    ShardPoisoned,
    #[error("eviction failed")]
    EvictionFailed,
    #[error("not enough memory is used")]
    NotEnoughUsed,
}
