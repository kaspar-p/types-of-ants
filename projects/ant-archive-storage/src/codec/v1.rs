use anyhow::Context;
use async_trait::async_trait;
use std::io::{ErrorKind, SeekFrom};
use std::path::Path;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt, ReadBuf};

use crate::codec::{BlobCodec, CodecError};

const VERSION_BYTE: u8 = 1;

pub struct V1Codec {
    file: tokio::fs::File,
    size: u64,
}

impl V1Codec {
    /// Open a V1 blob for reading. `file` must be positioned immediately after
    /// the version byte. `physical_body_size` = file_size - 1.
    pub fn new(file: tokio::fs::File, physical_body_size: u64) -> Self {
        V1Codec {
            file,
            size: physical_body_size,
        }
    }

    /// Create a new V1 blob for writing. Writes the version byte immediately;
    /// subsequent content goes through AsyncWrite.
    pub async fn create(dest: &Path) -> Result<Self, CodecError> {
        let mut file = tokio::fs::File::create(dest)
            .await
            .context("Failed to create blob file")
            .map_err(CodecError::Internal)?;

        file.write_all(&[VERSION_BYTE])
            .await
            .context("Failed to write encoding byte")
            .map_err(CodecError::Internal)?;

        Ok(V1Codec { file, size: 0 })
    }

    pub async fn verify(path: &Path) -> Result<(), CodecError> {
        let mut file = tokio::fs::File::open(path)
            .await
            .map_err(|e| match e.kind() {
                ErrorKind::NotFound => CodecError::NotFound(path.to_string_lossy().into_owned()),
                _ => CodecError::Internal(e.into()),
            })?;

        let mut buf = [0u8; 1];
        file.read_exact(&mut buf)
            .await
            .context("Failed to read encoding byte")
            .map_err(CodecError::Internal)?;

        if buf[0] != VERSION_BYTE {
            return Err(CodecError::Internal(anyhow::anyhow!(
                "Unknown encoding version {}",
                buf[0]
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl BlobCodec for V1Codec {
    fn size(&self) -> u64 {
        self.size
    }

    async fn seek(&mut self, offset: u64) -> Result<(), CodecError> {
        self.file
            .seek(SeekFrom::Start(1 + offset))
            .await
            .context("Failed to seek blob")
            .map_err(CodecError::Internal)?;
        Ok(())
    }

    async fn sync(&mut self) -> Result<(), CodecError> {
        self.file
            .flush()
            .await
            .context("Failed to flush blob")
            .map_err(CodecError::Internal)?;
        self.file
            .sync_all()
            .await
            .context("Failed to fsync blob")
            .map_err(CodecError::Internal)?;
        Ok(())
    }
}

impl AsyncRead for V1Codec {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().file).poll_read(cx, buf)
    }
}

impl AsyncWrite for V1Codec {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.get_mut().file).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().file).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().file).poll_shutdown(cx)
    }
}
