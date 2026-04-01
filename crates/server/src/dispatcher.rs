use std::sync::Arc;

use bytes::Bytes;
use proto::RespValue;
use storage::{StorageError, Store, StoreValue};

use crate::command::Command;

pub fn dispatch(cmd: Command, store: &Arc<dyn Store>) -> RespValue {
    let store = store.clone();
    match cmd {
        Command::Ping { message } => match message {
            Some(msg) => RespValue::BlobString(msg),
            None => RespValue::SimpleString(Bytes::from_static(b"PONG")),
        },

        Command::Get { key } => match store.get(&key) {
            Ok(Some(value)) => RespValue::BlobString(value.to_bytes()),
            Ok(None) => RespValue::Null,
            Err(e) => storage_err(e),
        },

        Command::Set { key, value, ttl } => {
            match store.set(key, StoreValue::from_bytes(value), ttl) {
                Ok(_) => RespValue::SimpleString(Bytes::from_static(b"OK")),
                Err(e) => storage_err(e),
            }
        }

        Command::Del { keys } => {
            let count = keys
                .iter()
                .filter_map(|k| store.del(k).ok())
                .filter(|&deleted| deleted)
                .count();
            RespValue::Integer(count as i64)
        }

        Command::Exists { keys } => {
            let count = keys
                .iter()
                .filter_map(|k| store.exists(k).ok())
                .filter(|&exists| exists)
                .count();
            RespValue::Integer(count as i64)
        }

        Command::Ttl { key } => match store.ttl(&key) {
            Ok(Some(ttl)) => RespValue::Integer(ttl.as_secs() as i64),
            Ok(None) => match store.exists(&key) {
                Ok(true) => RespValue::Integer(-1),
                _ => RespValue::Integer(-2),
            },
            Err(e) => storage_err(e),
        },

        Command::Persist { key } => match store.persist(&key) {
            Ok(true) => RespValue::Integer(1),
            Ok(false) => RespValue::Integer(0),
            Err(e) => storage_err(e),
        },
        //TODO: filter by pattern using a global filter
        Command::Keys { pattern } => match store.keys() {
            Ok(keys) => RespValue::Array(
                keys.into_iter()
                    .map(|key| RespValue::BlobString(key))
                    .collect(),
            ),
            Err(e) => storage_err(e),
        },
        Command::Len => RespValue::Integer(store.len() as i64),
        Command::Flush => {
            store.flush();
            RespValue::SimpleString(Bytes::from_static(b"OK"))
        }
        Command::Stats => {
            let snap = store.stats();
            let fields: Vec<RespValue> = vec![
                RespValue::SimpleString(Bytes::from_static(b"hits")),
                RespValue::Integer(snap.hits as i64),
                RespValue::SimpleString(Bytes::from_static(b"misses")),
                RespValue::Integer(snap.misses as i64),
                RespValue::SimpleString(Bytes::from_static(b"total_keys")),
                RespValue::Integer(snap.total_keys as i64),
                RespValue::SimpleString(Bytes::from_static(b"used_memory")),
                RespValue::Integer(snap.used_memory as i64),
                RespValue::SimpleString(Bytes::from_static(b"expired_keys")),
                RespValue::Integer(snap.expired_keys as i64),
                RespValue::SimpleString(Bytes::from_static(b"evicted_keys")),
                RespValue::Integer(snap.evicted_keys as i64),
                RespValue::SimpleString(Bytes::from_static(b"hit_ratio")),
                RespValue::Double(snap.hit_ratio),
            ];
            RespValue::Array(fields)
        }
        Command::Unknown(b) => RespValue::SimpleError(Bytes::from(format!(
            "Invalid Command {}",
            String::from_utf8_lossy(&b)
        ))),
    }
}
fn storage_err(e: StorageError) -> RespValue {
    RespValue::SimpleError(Bytes::from(format!("ERR {}", e)))
}
