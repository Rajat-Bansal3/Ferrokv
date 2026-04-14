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
    #[error("key size should be greater than 1 and less than 512 mb")]
    InvalidKeyLen,
    #[error("value size should be greater than 1 and less than 512 mb")]
    InvalidValueLen,
    #[error("key contains a null symbol")]
    KeyContainsNull,
    #[error("not and integer")]
    NotInteger,
    #[error("integer overflow")]
    IntegerOverflow,
    #[error("key expired")]
    KeyExpired,
}
