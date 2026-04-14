use std::time::Duration;

use bytes::Bytes;
use proto::{ProtoError, RespValue};

pub enum Command {
    Get {
        key: Bytes,
    },
    MGet {
        keys: Vec<Bytes>,
    },
    GetSet {
        key: Bytes,
        value: Bytes,
    },
    GetRange {
        key: Bytes,
        start: i64,
        end: i64,
    },
    Strlen {
        key: Bytes,
    },
    Exists {
        keys: Vec<Bytes>,
    },
    Ttl {
        key: Bytes,
    },
    // Pttl {
    //     key: Bytes,
    // },
    Type {
        key: Bytes,
    },
    Keys {
        pattern: Option<Bytes>,
    },
    RandomKey,
    // Scan {
    //     cursor: u64,
    //     count: Option<u64>,
    //     pattern: Option<Bytes>,
    // },
    Len,

    Set {
        key: Bytes,
        value: Bytes,
        ttl: Option<Duration>,
    },
    SetNx {
        key: Bytes,
        value: Bytes,
    },
    SetEx {
        key: Bytes,
        value: Bytes,
        ttl: Duration,
    },
    // PSetEx {
    //     key: Bytes,
    //     value: Bytes,
    //     ttl: Duration,
    // },
    MSet {
        pairs: Vec<(Bytes, Bytes)>,
    },
    Incr {
        key: Bytes,
    },
    IncrBy {
        key: Bytes,
        delta: i64,
    },
    Decr {
        key: Bytes,
    },
    DecrBy {
        key: Bytes,
        delta: i64,
    },
    Append {
        key: Bytes,
        value: Bytes,
    },
    Rename {
        key: Bytes,
        new_key: Bytes,
    },
    Del {
        keys: Vec<Bytes>,
    },
    Persist {
        key: Bytes,
    },
    Flush,

    Ping {
        message: Option<Bytes>,
    },
    Stats,
    Config,
    // Hello,
    Unknown(Bytes),
}
impl Command {
    pub fn from_resp(value: RespValue) -> Result<Command, ProtoError> {
        match value {
            RespValue::Array(parts) if !parts.is_empty() => {
                let cmd_name = match &parts[0] {
                    RespValue::BlobString(b) => b.to_ascii_uppercase(),
                    _ => return Err(ProtoError::InvalidCommand),
                };

                match cmd_name.as_slice() {
                    b"GET" => Command::parse_get(parts),
                    b"MGET" => Command::parse_mget(parts),
                    b"GETRANGE" => Command::parse_getrange(parts),
                    b"STRLEN" => Command::parse_strlen(parts),
                    b"TYPE" => Command::parse_type(parts),
                    b"EXISTS" => Command::parse_exists(parts),
                    b"TTL" => Command::parse_ttl(parts),
                    b"KEYS" => Command::parse_keys(parts),
                    b"LEN" => Ok(Command::Len),

                    b"SET" => Command::parse_set(parts),
                    b"GETSET" => Command::parse_getset(parts),
                    b"SETNX" => Command::parse_setnx(parts),
                    b"SETEX" => Command::parse_setex(parts),
                    b"MSET" => Command::parse_mset(parts),
                    b"INCR" => Command::parse_incr(parts),
                    b"INCRBY" => Command::parse_incrby(parts),
                    b"DECR" => Command::parse_decr(parts),
                    b"DECRBY" => Command::parse_decrby(parts),
                    b"APPEND" => Command::parse_append(parts),
                    b"RENAME" => Command::parse_rename(parts),

                    b"DEL" => Command::parse_del(parts),
                    b"PERSIST" => Command::parse_persist(parts),
                    b"FLUSH" => Ok(Command::Flush),

                    b"PING" => Command::parse_ping(parts),
                    b"STATS" => Ok(Command::Stats),
                    b"RANDOMKEY" => Ok(Command::RandomKey),
                    b"CONFIG GET" => Ok(Command::Config),
                    _ => Ok(Command::Unknown(Bytes::from(cmd_name))),
                }
            }
            _ => Err(ProtoError::InvalidCommand),
        }
    }

    fn parse_get(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 2 {
            return Err(ProtoError::WrongArity);
        }

        let mut iter = parts.into_iter();
        iter.next();
        match iter.next().unwrap() {
            RespValue::BlobString(key) => Ok(Command::Get { key }),
            _ => Err(ProtoError::InvalidCommand),
        }
    }
    fn parse_strlen(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 2 {
            return Err(ProtoError::WrongArity);
        }

        let mut iter = parts.into_iter();
        iter.next();
        match iter.next().unwrap() {
            RespValue::BlobString(key) => Ok(Command::Strlen { key }),
            _ => Err(ProtoError::InvalidCommand),
        }
    }
    fn parse_type(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 2 {
            return Err(ProtoError::WrongArity);
        }

        let mut iter = parts.into_iter();
        iter.next();
        match iter.next().unwrap() {
            RespValue::BlobString(key) => Ok(Command::Type { key }),
            _ => Err(ProtoError::InvalidCommand),
        }
    }
    fn parse_mget(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() < 2 {
            return Err(ProtoError::WrongArity);
        }

        let keys = parts
            .into_iter()
            .skip(1)
            .map(|arg| match arg {
                RespValue::BlobString(key) => Ok(key),
                _ => Err(ProtoError::InvalidCommand),
            })
            .collect::<Result<Vec<_>, ProtoError>>()?;

        Ok(Command::MGet { keys })
    }
    fn parse_getset(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() < 3 {
            return Err(ProtoError::WrongArity);
        }
        let mut iter = parts.into_iter();
        iter.next();
        let key = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        let value = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        Ok(Command::GetSet { key, value })
    }
    fn parse_exists(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() < 2 {
            return Err(ProtoError::WrongArity);
        }

        let mut iter = parts.into_iter();
        iter.next();
        let mut args: Vec<Bytes> = Vec::new();
        while let Some(arg) = iter.next() {
            match arg {
                RespValue::BlobString(key) => args.push(key),
                _ => return Err(ProtoError::InvalidCommand),
            };
        }
        Ok(Command::Exists { keys: args })
    }
    fn parse_ttl(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 2 {
            return Err(ProtoError::WrongArity);
        }

        let mut iter = parts.into_iter();
        iter.next();
        match iter.next().unwrap() {
            RespValue::BlobString(key) => Ok(Command::Ttl { key }),
            _ => Err(ProtoError::InvalidCommand),
        }
    }
    fn parse_keys(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 2 {
            return Err(ProtoError::WrongArity);
        }

        let mut iter = parts.into_iter();
        iter.next();
        match iter.next().unwrap() {
            RespValue::BlobString(pattern) => {
                if pattern.as_ref() == b"*" {
                    Ok(Command::Keys { pattern: None })
                } else {
                    Ok(Command::Keys {
                        pattern: Some(pattern),
                    })
                }
            }
            RespValue::Null => Ok(Command::Keys { pattern: None }),
            _ => Err(ProtoError::InvalidCommand),
        }
    }
    fn parse_set(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() < 3 {
            return Err(ProtoError::WrongArity);
        }
        let mut iter = parts.into_iter();
        iter.next();
        let key = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        let value = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        let mut ttl = None;
        while let Some(arg) = iter.next() {
            match arg {
                RespValue::BlobString(opt) => match opt.to_ascii_uppercase().as_slice() {
                    b"EX" => {
                        let secs = match iter.next() {
                            Some(RespValue::BlobString(n)) => std::str::from_utf8(&n)
                                .map_err(|_| ProtoError::InvalidCommand)?
                                .parse::<u64>()
                                .map_err(|_| ProtoError::InvalidCommand)?,
                            _ => return Err(ProtoError::WrongArity),
                        };
                        ttl = Some(Duration::from_secs(secs));
                    }
                    b"PX" => {
                        let milli = match iter.next() {
                            Some(RespValue::BlobString(n)) => std::str::from_utf8(&n)
                                .map_err(|_| ProtoError::InvalidCommand)?
                                .parse::<u64>()
                                .map_err(|_| ProtoError::InvalidCommand)?,
                            _ => return Err(ProtoError::WrongArity),
                        };
                        ttl = Some(Duration::from_millis(milli));
                    }
                    b"EXAT" => {
                        let unix_timestamp_sec = match iter.next() {
                            Some(RespValue::BlobString(n)) => std::str::from_utf8(&n)
                                .map_err(|_| ProtoError::InvalidCommand)?
                                .parse::<u64>()
                                .map_err(|_| ProtoError::InvalidCommand)?,
                            _ => return Err(ProtoError::WrongArity),
                        };
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        ttl = Some(Duration::from_secs(unix_timestamp_sec.saturating_sub(now)));
                    }
                    b"PXAT" => {
                        let unix_timestamp_millis = match iter.next() {
                            Some(RespValue::BlobString(n)) => std::str::from_utf8(&n)
                                .map_err(|_| ProtoError::InvalidCommand)?
                                .parse::<u64>()
                                .map_err(|_| ProtoError::InvalidCommand)?,
                            _ => return Err(ProtoError::WrongArity),
                        };
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64;
                        ttl = Some(Duration::from_millis(
                            unix_timestamp_millis.saturating_sub(now),
                        ));
                    }
                    _ => return Err(ProtoError::InvalidCommand),
                    // b"KEEPTTL" => {}
                },
                _ => return Err(ProtoError::InvalidCommand),
            }
        }
        Ok(Command::Set { key, value, ttl })
    }
    fn parse_setnx(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 3 {
            return Err(ProtoError::WrongArity);
        }
        let mut iter = parts.into_iter();
        iter.next();
        let key = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        let value = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        Ok(Command::SetNx { key, value })
    }
    fn parse_setex(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 4 {
            return Err(ProtoError::WrongArity);
        }
        let mut iter = parts.into_iter();
        iter.next();
        let key = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        let secs = match iter.next() {
            Some(RespValue::BlobString(n)) => std::str::from_utf8(&n)
                .map_err(|_| ProtoError::InvalidCommand)?
                .parse::<u64>()
                .map_err(|_| ProtoError::InvalidCommand)?,
            _ => return Err(ProtoError::WrongArity),
        };
        let ttl = Duration::from_secs(secs);
        let value = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        Ok(Command::SetEx { key, ttl, value })
    }
    fn parse_mset(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        let len = parts.len();
        if len < 3 || (len % 2) == 0 {
            return Err(ProtoError::WrongArity);
        }
        let mut iter = parts.into_iter().skip(1);
        let mut key_val = Vec::with_capacity((len - 1) / 2);
        while let (Some(k), Some(v)) = (iter.next(), iter.next()) {
            let key = match k {
                RespValue::BlobString(b) => b,
                _ => return Err(ProtoError::InvalidCommand),
            };
            let value = match v {
                RespValue::BlobString(b) => b,
                _ => return Err(ProtoError::InvalidCommand),
            };
            key_val.push((key, value));
        }

        Ok(Command::MSet { pairs: key_val })
    }
    fn parse_incr(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 2 {
            return Err(ProtoError::WrongArity);
        }

        let mut iter = parts.into_iter();
        iter.next();
        match iter.next().unwrap() {
            RespValue::BlobString(key) => Ok(Command::Incr { key }),
            _ => Err(ProtoError::InvalidCommand),
        }
    }
    fn parse_incrby(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 3 {
            return Err(ProtoError::WrongArity);
        }

        let mut iter = parts.into_iter();
        iter.next();
        let key = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        let delta = match iter.next().unwrap() {
            RespValue::BlobString(b) => std::str::from_utf8(&b)
                .map_err(|_| ProtoError::InvalidCommand)?
                .parse::<i64>()
                .map_err(|_| ProtoError::InvalidCommand)?,
            _ => return Err(ProtoError::InvalidCommand),
        };
        Ok(Command::IncrBy { key, delta })
    }
    fn parse_decr(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 2 {
            return Err(ProtoError::WrongArity);
        }

        let mut iter = parts.into_iter();
        iter.next();
        match iter.next().unwrap() {
            RespValue::BlobString(key) => Ok(Command::Decr { key }),
            _ => Err(ProtoError::InvalidCommand),
        }
    }
    fn parse_decrby(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 3 {
            return Err(ProtoError::WrongArity);
        }

        let mut iter = parts.into_iter();
        iter.next();
        let key = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        let delta = match iter.next().unwrap() {
            RespValue::BlobString(b) => std::str::from_utf8(&b)
                .map_err(|_| ProtoError::InvalidCommand)?
                .parse::<i64>()
                .map_err(|_| ProtoError::InvalidCommand)?,
            _ => return Err(ProtoError::InvalidCommand),
        };
        Ok(Command::DecrBy { key, delta })
    }
    fn parse_del(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() < 2 {
            return Err(ProtoError::WrongArity);
        }

        let mut iter = parts.into_iter();
        iter.next();
        let mut args: Vec<Bytes> = Vec::new();
        while let Some(arg) = iter.next() {
            match arg {
                RespValue::BlobString(key) => args.push(key),
                _ => return Err(ProtoError::InvalidCommand),
            };
        }
        Ok(Command::Del { keys: args })
    }
    fn parse_persist(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 2 {
            return Err(ProtoError::WrongArity);
        }

        let mut iter = parts.into_iter();
        iter.next();
        match iter.next().unwrap() {
            RespValue::BlobString(key) => Ok(Command::Persist { key }),
            _ => Err(ProtoError::InvalidCommand),
        }
    }
    fn parse_ping(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        let mut iter = parts.into_iter();
        iter.next();
        match iter.next() {
            Some(RespValue::BlobString(msg)) => Ok(Command::Ping { message: Some(msg) }),
            None => Ok(Command::Ping { message: None }),
            _ => Err(ProtoError::InvalidCommand),
        }
    }
    fn parse_append(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 3 {
            return Err(ProtoError::WrongArity);
        }
        let mut iter = parts.into_iter();
        iter.next();
        let key = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        let value = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        Ok(Command::Append { key, value })
    }
    fn parse_rename(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 3 {
            return Err(ProtoError::WrongArity);
        }
        let mut iter = parts.into_iter();
        iter.next();
        let key = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        let new_key = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        Ok(Command::Rename { key, new_key })
    }
    fn parse_getrange(parts: Vec<RespValue>) -> Result<Command, ProtoError> {
        if parts.len() != 4 {
            return Err(ProtoError::WrongArity);
        }
        let mut iter = parts.into_iter();
        iter.next();
        let key = match iter.next().unwrap() {
            RespValue::BlobString(b) => b,
            _ => return Err(ProtoError::InvalidCommand),
        };
        let start = match iter.next().unwrap() {
            RespValue::BlobString(b) => std::str::from_utf8(&b)
                .map_err(|_| ProtoError::InvalidCommand)?
                .parse::<i64>()
                .map_err(|_| ProtoError::InvalidCommand)?,
            _ => return Err(ProtoError::InvalidCommand),
        };
        let end = match iter.next().unwrap() {
            RespValue::BlobString(b) => std::str::from_utf8(&b)
                .map_err(|_| ProtoError::InvalidCommand)?
                .parse::<i64>()
                .map_err(|_| ProtoError::InvalidCommand)?,
            _ => return Err(ProtoError::InvalidCommand),
        };
        Ok(Command::GetRange { key, start, end })
    }
}
