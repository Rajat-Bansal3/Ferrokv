use bytes::Bytes;

use crate::{ProtoError, RespValue};

pub struct Parser {
    buf: Bytes,
    pos: usize,
}

impl Parser {
    pub fn new(buf: Bytes) -> Self {
        Parser { buf, pos: 0 }
    }
    pub fn parse(&mut self) -> Result<Option<RespValue>, ProtoError> {
        if self.pos >= self.buf.len() {
            return Err(ProtoError::Incomplete);
        }
        match self.buf[self.pos] {
            b'+' => {
                self.pos += 1;
                Ok(Some(RespValue::SimpleString(self.read_line()?)))
            }
            b'-' => {
                self.pos += 1;
                Ok(Some(RespValue::SimpleError(self.read_line()?)))
            }
            b':' => {
                self.pos += 1;
                Ok(Some(RespValue::Integer(self.parse_int()?)))
            }
            b'$' => {
                self.pos += 1;
                Ok(Some(RespValue::BlobString(self.parse_strings()?)))
            }
            b'_' => {
                self.pos += 1;
                match self.buf[self.pos..].windows(2).position(|w| w == b"\r\n") {
                    None => Err(ProtoError::Incomplete),
                    Some(end) => {
                        if end != 0 {
                            return Err(ProtoError::InvalidLength);
                        }
                        self.pos += end + 2;
                        Ok(Some(RespValue::Null))
                    }
                }
            }
            b'#' => {
                self.pos += 1;
                Ok(Some(RespValue::Boolean(self.parse_bool()?)))
            }
            b',' => {
                self.pos += 1;
                Ok(Some(RespValue::Double(self.parse_float()?)))
            }
            b'(' => {
                self.pos += 1;
                Ok(Some(RespValue::BigNumber(self.parse_big_int()?)))
            }
            b'!' => {
                self.pos += 1;
                Ok(Some(RespValue::BlobError(self.parse_strings()?)))
            }
            b'=' => {
                self.pos += 1;

                let res = self.parse_verbatium_strings()?;

                Ok(Some(RespValue::VerbatimString {
                    encoding: res.0,
                    data: res.1,
                }))
            }
            b'*' => {
                self.pos += 1;
                Ok(Some(RespValue::Array(self.parse_arrays()?)))
            }
            // b'%' => {}                              // Maps (RESP3)
            // b'~' => {}                              // Sets (RESP3)
            // b'>' => {}                              // Pushes (RESP3)
            // b'|' => {}                              // Attributes (RESP3)
            _ => Err(ProtoError::InvalidTypeByte(self.buf[self.pos])),
        }
    }
    fn read_line(&mut self) -> Result<Bytes, ProtoError> {
        match self.buf[self.pos..].windows(2).position(|w| w == b"\r\n") {
            None => Err(ProtoError::Incomplete),
            Some(end) => {
                let line = self.buf.slice(self.pos..self.pos + end);
                self.pos += end + 2;
                Ok(line)
            }
        }
    }
    fn parse_int(&mut self) -> Result<i64, ProtoError> {
        let line = self.read_line()?;
        let (sign, digit) = match line.first() {
            Some(b'+') => (1i64, &line[1..]),
            Some(b'-') => (-1i64, &line[1..]),
            _ => (1i64, &line[..]),
        };
        let s = std::str::from_utf8(digit).map_err(|_| ProtoError::InvalidInteger)?;

        let n: i64 = s.parse().map_err(|_| ProtoError::InvalidInteger)?;

        Ok(sign * n)
    }
    fn parse_big_int(&mut self) -> Result<i128, ProtoError> {
        let line = self.read_line()?;
        let (sign, digit) = match line.first() {
            Some(b'+') => (1i128, &line[1..]),
            Some(b'-') => (-1i128, &line[1..]),
            _ => (1i128, &line[..]),
        };
        let s = std::str::from_utf8(digit).map_err(|_| ProtoError::InvalidInteger)?;

        let n: i128 = s.parse().map_err(|_| ProtoError::InvalidInteger)?;

        Ok(sign * n)
    }
    fn parse_float(&mut self) -> Result<f64, ProtoError> {
        let line = self.read_line()?;
        match line.as_ref() {
            b"inf" | b"+inf" => return Ok(f64::INFINITY),
            b"-inf" => return Ok(f64::NEG_INFINITY),
            b"nan" => return Ok(f64::NAN),
            _ => {}
        }
        let (sign, digits) = match line.first() {
            Some(b'+') => (1f64, &line[1..]),
            Some(b'-') => (-1f64, &line[1..]),
            _ => (1f64, &line[..]),
        };
        let s = std::str::from_utf8(digits).map_err(|_| ProtoError::InvalidFloat)?;

        let n: f64 = s.parse().map_err(|_| ProtoError::InvalidFloat)?;

        Ok(sign * n)
    }
    fn parse_bool(&mut self) -> Result<bool, ProtoError> {
        let line = self.read_line()?;
        if line.len() != 1 {
            return Err(ProtoError::InvalidLength);
        }
        match line.first() {
            Some(b't') => Ok(true),
            Some(b'f') => Ok(false),
            _ => Err(ProtoError::InvalidBoolean),
        }
    }
    fn parse_strings(&mut self) -> Result<Bytes, ProtoError> {
        let line = self.read_line()?;
        let s = std::str::from_utf8(&line).map_err(|_| ProtoError::InvalidInteger)?;
        let len_string: i64 = s.parse().map_err(|_| ProtoError::InvalidInteger)?;

        if len_string == 1 {
            return Ok(Bytes::new());
        }
        if len_string < 0 {
            return Err(ProtoError::InvalidInteger);
        }
        let len_string = len_string as usize;
        let start = self.pos;
        let end = start + len_string;

        if end + 2 > self.buf.len() {
            return Err(ProtoError::Incomplete);
        }

        let line = self.buf.slice(start..end);
        self.pos += len_string + 2;
        Ok(line)
    }
    fn parse_verbatium_strings(&mut self) -> Result<(Bytes, Bytes), ProtoError> {
        let line = self.read_line()?;
        let s = std::str::from_utf8(&line).map_err(|_| ProtoError::InvalidInteger)?;
        let len_string: i64 = s.parse().map_err(|_| ProtoError::InvalidInteger)?;

        if len_string < 4 {
            return Err(ProtoError::InvalidInteger);
        }
        let len_string = len_string as usize;
        let mut start = self.pos;
        let end = start + len_string;

        if end + 2 > self.buf.len() {
            return Err(ProtoError::Incomplete);
        }
        let encoding = self.buf.slice(start..start + 3);

        if self.buf[start + 3] != b':' {
            return Err(ProtoError::InvalidLength);
        }

        start += 4;

        let line = self.buf.slice(start..end);
        self.pos = end + 2;
        Ok((encoding, line))
    }
    fn parse_arrays(&mut self) -> Result<Vec<RespValue>, ProtoError> {
        let line = self.read_line()?;
        let s = std::str::from_utf8(&line).map_err(|_| ProtoError::InvalidInteger)?;
        let len_vec: i64 = s.parse().map_err(|_| ProtoError::InvalidInteger)?;

        if len_vec == -1 {
            return Ok(Vec::new());
        }
        if len_vec < 0 {
            return Err(ProtoError::InvalidInteger);
        }
        let mut vec = Vec::with_capacity(len_vec as usize);
        for _ in 0..len_vec {
            match self.parse()? {
                Some(val) => vec.push(val),
                None => return Err(ProtoError::Incomplete),
            }
        }
        Ok(vec)
    }
}

#[cfg(test)]
mod test {
    use super::{Parser, ProtoError};
    use crate::RespValue;
    use bytes::Bytes;

    #[test]
    fn test_simple_string() {
        let mut parser = Parser::new(Bytes::from_static(b"+OK\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::SimpleString(val)) => assert_eq!(val.as_ref(), b"OK"),
            _ => panic!("expected SimpleString"),
        }
    }

    #[test]
    fn test_simple_err() {
        let mut parser = Parser::new(Bytes::from_static(b"-Error message\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::SimpleError(val)) => assert_eq!(val.as_ref(), b"Error message"),
            _ => panic!("expected Error message"),
        }
    }

    #[test]
    fn test_integer() {
        let mut parser = Parser::new(Bytes::from_static(b":27\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::Integer(val)) => assert_eq!(val, 27),
            _ => panic!("expected Integer"),
        }
    }

    #[test]
    fn test_integer_negative() {
        let mut parser = Parser::new(Bytes::from_static(b":-27\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::Integer(val)) => assert_eq!(val, -27),
            _ => panic!("expected negative Integer"),
        }
    }

    #[test]
    fn test_integer_explicit_positive() {
        let mut parser = Parser::new(Bytes::from_static(b":+27\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::Integer(val)) => assert_eq!(val, 27),
            _ => panic!("expected Integer"),
        }
    }

    #[test]
    fn test_array() {
        let mut parser = Parser::new(Bytes::from_static(
            b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n",
        ));
        match parser.parse().unwrap() {
            Some(RespValue::Array(items)) => {
                assert_eq!(items.len(), 3);
                assert!(matches!(&items[0], RespValue::BlobString(v) if v.as_ref() == b"SET"));
                assert!(matches!(&items[1], RespValue::BlobString(v) if v.as_ref() == b"foo"));
                assert!(matches!(&items[2], RespValue::BlobString(v) if v.as_ref() == b"bar"));
            }
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn test_array_null() {
        let mut parser = Parser::new(Bytes::from_static(b"*-1\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::Array(items)) => assert!(items.is_empty()),
            _ => panic!("expected empty Array"),
        }
    }

    #[test]
    fn test_array_empty() {
        let mut parser = Parser::new(Bytes::from_static(b"*0\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::Array(items)) => assert!(items.is_empty()),
            _ => panic!("expected empty Array"),
        }
    }

    #[test]
    fn test_array_nested() {
        let mut parser = Parser::new(Bytes::from_static(b"*2\r\n*2\r\n:1\r\n:2\r\n*1\r\n+OK\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::Array(items)) => {
                assert_eq!(items.len(), 2);
                assert!(matches!(&items[0], RespValue::Array(inner) if inner.len() == 2));
                assert!(matches!(&items[1], RespValue::Array(inner) if inner.len() == 1));
            }
            _ => panic!("expected nested Array"),
        }
    }

    #[test]
    fn test_null() {
        let mut parser = Parser::new(Bytes::from_static(b"_\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::Null) => {}
            _ => panic!("expected Null"),
        }
    }

    #[test]
    fn test_null_invalid() {
        let mut parser = Parser::new(Bytes::from_static(b"_garbage\r\n"));
        assert!(matches!(parser.parse(), Err(ProtoError::InvalidLength)));
    }

    #[test]
    fn test_big_number() {
        let mut parser = Parser::new(Bytes::from_static(b"(1234567890123456789\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::BigNumber(val)) => assert_eq!(val, 1234567890123456789i128),
            _ => panic!("expected BigNumber"),
        }
    }

    #[test]
    fn test_big_number_negative() {
        let mut parser = Parser::new(Bytes::from_static(b"(-1234567890123456789\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::BigNumber(val)) => assert_eq!(val, -1234567890123456789i128),
            _ => panic!("expected negative BigNumber"),
        }
    }

    #[test]
    fn test_double() {
        let mut parser = Parser::new(Bytes::from_static(b",3.14\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::Double(val)) => assert!((val - 3.14f64).abs() < f64::EPSILON),
            _ => panic!("expected Double"),
        }
    }

    #[test]
    fn test_double_negative() {
        let mut parser = Parser::new(Bytes::from_static(b",-3.14\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::Double(val)) => assert!((val - (-3.14f64)).abs() < f64::EPSILON),
            _ => panic!("expected negative Double"),
        }
    }

    #[test]
    fn test_double_inf() {
        let mut parser = Parser::new(Bytes::from_static(b",inf\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::Double(val)) => assert!(val.is_infinite() && val.is_sign_positive()),
            _ => panic!("expected inf Double"),
        }
    }

    #[test]
    fn test_double_neg_inf() {
        let mut parser = Parser::new(Bytes::from_static(b",-inf\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::Double(val)) => assert!(val.is_infinite() && val.is_sign_negative()),
            _ => panic!("expected -inf Double"),
        }
    }

    #[test]
    fn test_double_nan() {
        let mut parser = Parser::new(Bytes::from_static(b",nan\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::Double(val)) => assert!(val.is_nan()),
            _ => panic!("expected NaN Double"),
        }
    }

    #[test]
    fn test_bulk_error() {
        let mut parser = Parser::new(Bytes::from_static(b"!21\r\nSYNTAX invalid syntax\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::BlobError(val)) => assert_eq!(val.as_ref(), b"SYNTAX invalid syntax"),
            _ => panic!("expected BlobError"),
        }
    }

    #[test]
    fn test_bulk_error_incomplete() {
        let mut parser = Parser::new(Bytes::from_static(b"!21\r\nSYNTAX"));
        assert!(matches!(parser.parse(), Err(ProtoError::Incomplete)));
    }

    #[test]
    fn test_verbatim_string() {
        let mut parser = Parser::new(Bytes::from_static(b"=15\r\ntxt:hello world\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::VerbatimString { encoding, data }) => {
                assert_eq!(encoding.as_ref(), b"txt");
                assert_eq!(data.as_ref(), b"hello world");
            }
            _ => panic!("expected VerbatimString"),
        }
    }

    #[test]
    fn test_verbatim_string_markdown() {
        let mut parser = Parser::new(Bytes::from_static(b"=13\r\nmkd:# heading\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::VerbatimString { encoding, data }) => {
                assert_eq!(encoding.as_ref(), b"mkd");
                assert_eq!(data.as_ref(), b"# heading");
            }
            _ => panic!("expected VerbatimString mkd"),
        }
    }

    #[test]
    fn test_verbatim_string_invalid_no_colon() {
        let mut parser = Parser::new(Bytes::from_static(b"=10\r\ntxthelloXXX\r\n"));
        assert!(matches!(parser.parse(), Err(ProtoError::InvalidLength)));
    }

    #[test]
    fn test_bulk_string() {
        let mut parser = Parser::new(Bytes::from_static(b"$5\r\nhello\r\n"));
        match parser.parse().unwrap() {
            Some(RespValue::BlobString(val)) => assert_eq!(val.as_ref(), b"hello"),
            _ => panic!("expected hello"),
        }
    }
}
