use bytes::BytesMut;

use crate::RespValue;

pub fn serializer(response: &RespValue, buf: &mut BytesMut) {
    match response {
        RespValue::SimpleString(s) => {
            buf.extend_from_slice(b"+");
            buf.extend_from_slice(s);
            buf.extend_from_slice(b"\r\n");
        }
        RespValue::SimpleError(s) => {
            buf.extend_from_slice(b"-");
            buf.extend_from_slice(s);
            buf.extend_from_slice(b"\r\n");
        }
        RespValue::Integer(n) => {
            buf.extend_from_slice(b":");
            buf.extend_from_slice(n.to_string().as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
        RespValue::BlobString(s) => {
            buf.extend_from_slice(b"$");
            buf.extend_from_slice(s.len().to_string().as_bytes());
            buf.extend_from_slice(b"\r\n");
            buf.extend_from_slice(s);
            buf.extend_from_slice(b"\r\n");
        }
        RespValue::Null => buf.extend_from_slice(b"_\r\n"),
        RespValue::Boolean(b) => {
            buf.extend_from_slice(if *b { b"#t\r\n" } else { b"#f\r\n" });
        }
        RespValue::Array(items) => {
            buf.extend_from_slice(b"*");
            buf.extend_from_slice(items.len().to_string().as_bytes());
            buf.extend_from_slice(b"\r\n");
            for item in items {
                serializer(item, buf);
            }
        }
        RespValue::Double(n) => {
            buf.extend_from_slice(b".");
            buf.extend_from_slice(n.to_string().as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
        RespValue::BlobError(s) => {
            buf.extend_from_slice(b"!");
            buf.extend_from_slice(s.len().to_string().as_bytes());
            buf.extend_from_slice(b"\r\n");
            buf.extend_from_slice(s);
            buf.extend_from_slice(b"\r\n");
        }
        RespValue::BigNumber(n) => {
            buf.extend_from_slice(b"(");
            buf.extend_from_slice(n.to_string().as_bytes());
            buf.extend_from_slice(b"\r\n");
        }
        // RespValue::Map => {}
        // RespValue::Set => {}
        // RespValue::Push => {}
        _ => unimplemented!("pending parser"),
    }
}
