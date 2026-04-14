use std::sync::Arc;

use bytes::Bytes;
use command::Command;
use persist::persist::PersistHandle;
use proto::RespValue;
use storage::{StorageError, Store, StoreValue};

pub fn dispatch(cmd: Command, store: &Arc<dyn Store>, persist: &PersistHandle) -> RespValue {
    let store = store.clone();
    match cmd {
        Command::Get { key } => match store.get(&key) {
            Ok(Some(value)) => RespValue::BlobString(value.to_bytes()),
            Ok(None) => RespValue::Null,
            Err(e) => storage_err(e),
        },
        Command::MGet { keys } => {
            let mut res = Vec::new();
            for key in keys {
                match store.get(&key) {
                    Ok(Some(value)) => res.push(RespValue::BlobString(value.to_bytes())),
                    Ok(None) => res.push(RespValue::Null),
                    Err(e) => return storage_err(e),
                };
            }
            RespValue::Array(res)
        }
        Command::GetRange { key, start, end } => match store.get(&key) {
            Ok(Some(value)) => {
                let bytes = value.to_bytes();
                let len = bytes.len() as i64;
                let s = if start < 0 { len + start } else { start }.max(0);
                let e = if end < 0 { len + end } else { end }.min(len - 1);
                if s > e || s >= len {
                    return RespValue::BlobString(bytes::Bytes::new());
                }
                let final_slice = bytes.slice((s as usize)..(e as usize + 1));
                RespValue::BlobString(final_slice)
            }
            Ok(None) => RespValue::Null,
            Err(e) => storage_err(e),
        },
        Command::Strlen { key } => match store.get(&key) {
            Ok(Some(value)) => RespValue::Integer(value.to_bytes().len() as i64),
            Ok(None) => RespValue::Integer(0),
            Err(e) => storage_err(e),
        },
        Command::Type { key } => match store.get(&key) {
            Ok(Some(_)) => RespValue::SimpleString(Bytes::from_static(b"string")),
            Ok(None) => RespValue::SimpleString(Bytes::from_static(b"none")),
            Err(e) => storage_err(e),
        },
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
        //TODO: filter by pattern using a global filter
        Command::Keys { pattern: _ } => match store.keys() {
            Ok(keys) => RespValue::Array(
                keys.into_iter()
                    .map(|key| RespValue::BlobString(key))
                    .collect(),
            ),
            Err(e) => storage_err(e),
        },
        Command::Len => RespValue::Integer(store.len() as i64),
        Command::RandomKey => match store.random_key() {
            Ok(Some(key)) => RespValue::BlobString(key),
            Ok(None) => RespValue::Null,
            Err(e) => storage_err(e),
        },
        Command::Set { key, value, ttl } => {
            match validate_kv(&key, &value) {
                Ok(_) => {}
                Err(e) => return storage_err(e),
            }
            match store.set(key.clone(), StoreValue::from_bytes(value.clone()), ttl) {
                Ok(_) => {
                    persist.log_command(&Command::Set { key, value, ttl }).ok();
                    RespValue::SimpleString(Bytes::from_static(b"OK"))
                }
                Err(e) => storage_err(e),
            }
        }
        Command::SetEx { key, value, ttl } => {
            match validate_kv(&key, &value) {
                Ok(_) => {}
                Err(e) => return storage_err(e),
            }
            match store.set(
                key.clone(),
                StoreValue::from_bytes(value.clone()),
                Some(ttl),
            ) {
                Ok(_) => {
                    persist
                        .log_command(&Command::SetEx { key, value, ttl })
                        .ok();
                    RespValue::SimpleString(Bytes::from_static(b"OK"))
                }
                Err(e) => storage_err(e),
            }
        }
        // Command::SetNx { key, value } => {
        //     match validate_kv(&key, &value) {
        //         Ok(_) => {}
        //         Err(e) => return storage_err(e),
        //     };
        //     match store.set_not_exists(key, StoreValue::from_bytes(value)) {
        //         Ok(_) => RespValue::SimpleString(Bytes::from_static(b"OK")),
        //         Err(e) => storage_err(e),
        //     }
        // }
        Command::GetSet { key, value } => {
            match validate_kv(&key, &value) {
                Ok(_) => {}
                Err(e) => return storage_err(e),
            }
            match store.get_set(key.clone(), StoreValue::from_bytes(value.clone())) {
                Ok(prev) => {
                    persist.log_command(&Command::GetSet { key, value }).ok();
                    match prev {
                        Some(val) => RespValue::BlobString(val.to_bytes()),
                        None => RespValue::Null,
                    }
                }
                Err(e) => storage_err(e),
            }
        }
        Command::MSet { pairs } => {
            for (key, value) in pairs.clone() {
                match validate_kv(&key, &value) {
                    Ok(_) => {}
                    Err(e) => return storage_err(e),
                }
                if let Err(e) = store.set(key, StoreValue::from_bytes(value), None) {
                    return storage_err(e);
                }
            }
            persist.log_command(&Command::MSet { pairs }).ok();
            RespValue::SimpleString(Bytes::from_static(b"OK"))
        }
        Command::Incr { key } => match store.incr(&key, 1) {
            Ok(val) => {
                persist.log_command(&Command::Incr { key }).ok();
                RespValue::Integer(val)
            }
            Err(e) => storage_err(e),
        },
        Command::Decr { key } => match store.decr(&key, 1) {
            Ok(val) => {
                persist.log_command(&Command::Decr { key }).ok();
                RespValue::Integer(val)
            }
            Err(e) => storage_err(e),
        },
        Command::IncrBy { key, delta } => match store.incr(&key, delta) {
            Ok(val) => {
                persist.log_command(&Command::IncrBy { key, delta }).ok();
                RespValue::Integer(val)
            }
            Err(e) => storage_err(e),
        },
        Command::DecrBy { key, delta } => match store.decr(&key, delta) {
            Ok(val) => {
                persist.log_command(&Command::DecrBy { key, delta }).ok();
                RespValue::Integer(val)
            }
            Err(e) => storage_err(e),
        },
        Command::Append { key, value } => match store.append(&key, value.clone()) {
            Ok(len) => {
                persist.log_command(&Command::Append { key, value }).ok();
                RespValue::Integer(len as i64)
            }
            Err(e) => storage_err(e),
        },
        // Command::Rename { key, new_key } => match store.rename(key, new_key) {
        //     Ok(_) => RespValue::Integer(1),
        //     Err(e) => storage_err(e),
        // },
        Command::Del { keys } => {
            let count = keys
                .iter()
                .filter_map(|k| store.del(k).ok())
                .filter(|&deleted| deleted)
                .count();
            if count > 0 {
                persist.log_command(&Command::Del { keys }).ok();
            }
            RespValue::Integer(count as i64)
        }
        Command::Persist { key } => match store.persist(&key) {
            Ok(true) => {
                persist.log_command(&Command::Persist { key }).ok();
                RespValue::Integer(1)
            }
            Ok(false) => RespValue::Integer(0),
            Err(e) => storage_err(e),
        },
        Command::Flush => {
            store.flush();
            persist.log_command(&Command::Flush).ok();
            RespValue::SimpleString(Bytes::from_static(b"OK"))
        }
        Command::Ping { message } => match message {
            Some(msg) => RespValue::BlobString(msg),
            None => RespValue::SimpleString(Bytes::from_static(b"PONG")),
        },
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
        _ => todo!("building store traits for these"),
    }
}

fn storage_err(e: StorageError) -> RespValue {
    RespValue::SimpleError(Bytes::from(format!("ERR {}", e)))
}

fn validate_kv(key: &Bytes, value: &Bytes) -> Result<(), StorageError> {
    const MAX_SIZE: usize = 512 * 1024 * 1024;
    if key.is_empty() {
        return Err(StorageError::InvalidKeyLen);
    }
    if key.contains(&0) {
        return Err(StorageError::KeyContainsNull);
    }
    if key.len() > MAX_SIZE {
        return Err(StorageError::InvalidKeyLen);
    }
    if value.len() > MAX_SIZE {
        return Err(StorageError::InvalidValueLen);
    }
    Ok(())
}
