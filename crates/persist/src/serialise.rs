use bytes::{BufMut, BytesMut};
use command::Command;

pub fn serialize_command(cmd: &Command, buf: &mut BytesMut) {
    match cmd {
        Command::Set { key, value, ttl } => match ttl {
            Some(ttl) => {
                write_array(buf, 5);
                write_blob(buf, b"SET");
                write_blob(buf, key);
                write_blob(buf, value);
                write_blob(buf, b"EX");
                write_blob(buf, ttl.as_secs().to_string().as_bytes());
            }
            None => {
                write_array(buf, 3);
                write_blob(buf, b"SET");
                write_blob(buf, key);
                write_blob(buf, value);
            }
        },
        Command::SetEx { key, value, ttl } => {
            write_array(buf, 4);
            write_blob(buf, b"SETEX");
            write_blob(buf, key);
            write_blob(buf, ttl.as_secs().to_string().as_bytes());
            write_blob(buf, value);
        }
        Command::SetNx { key, value } => {
            write_array(buf, 3);
            write_blob(buf, b"SETNX");
            write_blob(buf, key);
            write_blob(buf, value);
        }
        Command::MSet { pairs } => {
            write_array(buf, 1 + pairs.len() * 2);
            write_blob(buf, b"MSET");
            for (k, v) in pairs {
                write_blob(buf, k);
                write_blob(buf, v);
            }
        }
        Command::GetSet { key, value } => {
            write_array(buf, 3);
            write_blob(buf, b"GETSET");
            write_blob(buf, key);
            write_blob(buf, value);
        }
        Command::Incr { key } => {
            write_array(buf, 2);
            write_blob(buf, b"INCR");
            write_blob(buf, key);
        }
        Command::Decr { key } => {
            write_array(buf, 2);
            write_blob(buf, b"DECR");
            write_blob(buf, key);
        }
        Command::IncrBy { key, delta } => {
            write_array(buf, 3);
            write_blob(buf, b"INCRBY");
            write_blob(buf, key);
            write_blob(buf, delta.to_string().as_bytes());
        }
        Command::DecrBy { key, delta } => {
            write_array(buf, 3);
            write_blob(buf, b"DECRBY");
            write_blob(buf, key);
            write_blob(buf, delta.to_string().as_bytes());
        }
        Command::Append { key, value } => {
            write_array(buf, 3);
            write_blob(buf, b"APPEND");
            write_blob(buf, key);
            write_blob(buf, value);
        }
        Command::Rename { key, new_key } => {
            write_array(buf, 3);
            write_blob(buf, b"RENAME");
            write_blob(buf, key);
            write_blob(buf, new_key);
        }
        Command::Del { keys } => {
            write_array(buf, 1 + keys.len());
            write_blob(buf, b"DEL");
            for k in keys {
                write_blob(buf, k);
            }
        }
        Command::Persist { key } => {
            write_array(buf, 2);
            write_blob(buf, b"PERSIST");
            write_blob(buf, key);
        }
        Command::Flush => {
            write_array(buf, 1);
            write_blob(buf, b"FLUSH");
        }
        _ => {}
    }
}

#[inline]
fn write_array(buf: &mut BytesMut, count: usize) {
    buf.put_u8(b'*');
    buf.put_slice(count.to_string().as_bytes());
    buf.put_slice(b"\r\n");
}

#[inline]
fn write_blob(buf: &mut BytesMut, data: &[u8]) {
    buf.put_u8(b'$');
    buf.put_slice(data.len().to_string().as_bytes());
    buf.put_slice(b"\r\n");
    buf.put_slice(data);
    buf.put_slice(b"\r\n");
}
