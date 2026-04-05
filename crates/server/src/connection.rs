use proto::{Parser, ProtoError, RespValue};
use std::{fmt::Write, sync::Arc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{ConnectionResult, command::Command, dispatcher::dispatch, error::ServerError};
use bytes::{Buf, Bytes, BytesMut};
use storage::Store;
use tokio::io::{ReadHalf, WriteHalf};

pub struct Connection {
    reader: ReadHalf<TcpStream>,
    writer: WriteHalf<TcpStream>,
    store: Arc<dyn Store>,
    read_buf: BytesMut,
    write_buf: BytesMut,
    id: u64,
}
impl Connection {
    pub fn new(stream: TcpStream, store: Arc<dyn Store>, id: u64) -> Self {
        let (reader, writer) = tokio::io::split(stream);
        Self {
            id,
            read_buf: BytesMut::with_capacity(4096),
            write_buf: BytesMut::with_capacity(4096),
            reader,
            store,
            writer,
        }
    }
    pub async fn run(&mut self) -> ConnectionResult<()> {
        loop {
            self.read_buf.reserve(4096);
            let len = self
                .reader
                .read_buf(&mut self.read_buf)
                .await
                .map_err(|_| ProtoError::Incomplete)?;

            if len == 0 {
                return Ok(());
            }
            loop {
                let mut parser = Parser::new(&self.read_buf);
                match parser.parse() {
                    Ok(Some(value)) => {
                        let consumed = parser.pos;
                        drop(parser);
                        self.read_buf.advance(consumed);
                        let res = match Command::from_resp(value) {
                            Ok(cmd) => dispatch(cmd, &self.store),
                            Err(_) => RespValue::SimpleError(bytes::Bytes::from_static(
                                b"ERR bad command",
                            )),
                        };
                        proto::serializer(&res, &mut self.write_buf);
                    }
                    Ok(None) => {
                        break;
                    }
                    Err(e) => {
                        let err_resp = RespValue::SimpleError(bytes::Bytes::from_static(
                            b"ERR protocol error",
                        ));
                        proto::serializer(&err_resp, &mut self.write_buf);
                        self.flush().await?;
                        return Err(e);
                    }
                }
            }
            self.flush().await?
        }
    }
    async fn flush(&mut self) -> ConnectionResult<()> {
        self.writer
            .write_all(&self.write_buf)
            .await
            .map_err(|_| ProtoError::Incomplete)?;
        self.write_buf.clear();
        Ok(())
    }
}
