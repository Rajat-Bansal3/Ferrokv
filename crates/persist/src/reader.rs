use std::{fs::OpenOptions, io::Read, path::Path, sync::Arc};

use server::command::Command;
use storage::{Store, StoreValue};

use crate::{error::AOFError, writer::AofResponse};

pub fn replay(path: &Path, store: &Arc<dyn Store>) -> AofResponse<u64> {
    let mut file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|_| AOFError::ErrOpeningFile)?;

    let mut count = 0u64;
    let mut raw = Vec::new();
    file.read_to_end(&mut raw)
        .map_err(|_| AOFError::ErrOpeningFile)?;

    let mut parser = proto::Parser::new(raw.as_slice());
    loop {
        match parser.parse() {
            Ok(Some(res)) => match Command::from_resp(res) {
                Ok(cmd) => {
                    match cmd {
                        Command::Set { key, value, ttl } => {
                            store.set(key, StoreValue::from_bytes(value), ttl).ok();
                        }
                        Command::SetEx { key, value, ttl } => {
                            store
                                .set(key, StoreValue::from_bytes(value), Some(ttl))
                                .ok();
                        }
                        // Command::SetNx { key, value } => {
                        //     store.set_nx(key, StoreValue::from_bytes(value)).ok();
                        // }
                        Command::MSet { pairs } => {
                            for (k, v) in pairs {
                                store.set(k, StoreValue::from_bytes(v), None).ok();
                            }
                        }
                        Command::GetSet { key, value } => {
                            store.get_set(key, StoreValue::from_bytes(value)).ok();
                        }
                        Command::Incr { key } => {
                            store.incr(&key, 1).ok();
                        }
                        Command::IncrBy { key, delta } => {
                            store.incr(&key, delta).ok();
                        }
                        Command::Decr { key } => {
                            store.decr(&key, 1).ok();
                        }
                        Command::DecrBy { key, delta } => {
                            store.decr(&key, -delta).ok();
                        }
                        Command::Append { key, value } => {
                            store.append(&key, value).ok();
                        }
                        Command::Persist { key } => {
                            store.persist(&key).ok();
                        }
                        Command::Del { keys } => {
                            for k in keys {
                                store.del(&k).ok();
                            }
                        }
                        Command::Flush => {
                            store.flush();
                        }
                        _ => {}
                    }
                    count += 1;
                }
                Err(e) => {
                    eprintln!("AOF skip corrupted command: {}", e);
                }
            },
            Ok(None) => break,
            Err(e) => {
                eprintln!("AOF parse error, stopping replay: {}", e);
                break;
            }
        }
    }

    Ok(count)
}
