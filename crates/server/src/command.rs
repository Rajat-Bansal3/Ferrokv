use std::time::Duration;

use bytes::Bytes;
use proto::{ProtoError, RespValue};

pub enum Command {
    Get {
        key: Bytes,
    },
    Exists {
        keys: Vec<Bytes>,
    },
    Ttl {
        key: Bytes,
    },
    Keys {
        pattern: Option<Bytes>,
    },
    Len,

    Set {
        key: Bytes,
        value: Bytes,
        ttl: Option<Duration>,
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
                    b"EXISTS" => Command::parse_exists(parts),
                    b"TTL" => Command::parse_ttl(parts),
                    b"KEYS" => Command::parse_keys(parts),
                    b"LEN" => Ok(Command::Len),

                    b"SET" => Command::parse_set(parts),
                    b"DEL" => Command::parse_del(parts),
                    b"PERSIST" => Command::parse_persist(parts),
                    b"FLUSH" => Ok(Command::Flush),

                    b"PING" => Command::parse_ping(parts),
                    b"STATS" => Ok(Command::Stats),

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
}
