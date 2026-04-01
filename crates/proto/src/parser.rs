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
