use bytes::{Bytes, BytesMut};
use command::Command;
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use crate::{error::AOFError, serialise::serialize_command};

pub type AofResponse<T> = anyhow::Result<T, AOFError>;

pub struct AofWriter {
    pub file: BufWriter<File>,
    pub path: PathBuf,
    pub bytes_written: usize,
}

impl AofWriter {
    pub fn open(path: &Path) -> AofResponse<Self> {
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(path)
            .map_err(|_| AOFError::ErrOpeningFile)?;
        let buff_file = BufWriter::with_capacity(65536, file);
        Ok(Self {
            bytes_written: 0,
            file: buff_file,
            path: path.to_path_buf(),
        })
    }

    pub fn append(&mut self, command: &Bytes) -> AofResponse<()> {
        // let mut buf = BytesMut::new();
        // serialize_command(command, &mut buf);
        self.bytes_written += self
            .file
            .write(&command)
            .map_err(|_| AOFError::ErrorWrittingToFile)?;
        Ok(())
    }
    pub fn flush(&mut self) -> AofResponse<()> {
        self.bytes_written = 0;
        self.file.flush().map_err(|_| AOFError::ErrorFlushingBuffer)
    }
    pub fn fsync(&mut self) -> AofResponse<()> {
        self.file
            .get_ref()
            .sync_data()
            .map_err(|_| AOFError::ErrorFlushingBuffer)
    }
}
