use persist::persist::PersistHandle;
use proto::{Parser, ProtoError, RespValue};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use command::Command;

use crate::{ConnectionResult, dispatcher::dispatch};
use bytes::{Buf, BytesMut};
use storage::Store;
use tokio::io::{ReadHalf, WriteHalf};

pub struct Connection {
    reader: ReadHalf<TcpStream>,
    writer: WriteHalf<TcpStream>,
    store: Arc<dyn Store>,
    read_buf: BytesMut,
    write_buf: BytesMut,
    id: u64,
    last_activity: Instant,
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
            last_activity: Instant::now(),
        }
    }
    pub async fn run(
        &mut self,
        max_ideal_time: u64,
        persist_handler: &PersistHandle,
    ) -> ConnectionResult<()> {
        loop {
            self.read_buf.reserve(4096);
            let read_result = tokio::time::timeout(
                Duration::from_secs(max_ideal_time),
                self.reader.read_buf(&mut self.read_buf),
            )
            .await;

            match read_result {
                Err(_) => {
                    tracing::info!(id = self.id, "connection timed out");
                    return Ok(());
                }
                Ok(Err(_)) => {
                    return Err(ProtoError::Incomplete);
                }
                Ok(Ok(0)) => {
                    return Ok(());
                }
                Ok(Ok(len)) => {
                    if len == 0 {
                        return Ok(());
                    }
                    loop {
                        let mut parser = Parser::new(&self.read_buf);
                        match parser.parse() {
                            Ok(Some(value)) => {
                                self.last_activity = Instant::now();
                                let consumed = parser.pos;
                                drop(parser);
                                self.read_buf.advance(consumed);
                                let res = match Command::from_resp(value) {
                                    Ok(cmd) => dispatch(cmd, &self.store, persist_handler),
                                    Err(_) => RespValue::SimpleError(bytes::Bytes::from_static(
                                        b"ERR bad command",
                                    )),
                                };
                                proto::serializer(&res, &mut self.write_buf);
                            }
                            Ok(None) => {
                                break;
                            }
                            Err(_) => {
                                let err_resp = RespValue::SimpleError(bytes::Bytes::from_static(
                                    b"ERR protocol error",
                                ));
                                proto::serializer(&err_resp, &mut self.write_buf);
                                self.flush().await?;
                                // Err(e);
                                break;
                            }
                        }
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
