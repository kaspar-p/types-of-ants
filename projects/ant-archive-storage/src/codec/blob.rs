use anyhow::Context;
use num_enum::TryFromPrimitive;
use std::io::ErrorKind;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf};

use crate::codec::{BlobCodec, CodecError, V1Codec};

#[derive(TryFromPrimitive)]
#[repr(u8)]
enum CodecVersion {
    V1 = 1,
}

pub struct BlobHandle {
    pub size: u64,
    inner: Box<dyn BlobCodec>,
}

impl BlobHandle {
    pub async fn open(mut file: tokio::fs::File) -> Result<Self, CodecError> {
        let physical = file
            .metadata()
            .await
            .context("Failed to stat blob")
            .map_err(CodecError::Internal)?
            .len();

        if physical == 0 {
            return Err(CodecError::Internal(anyhow::anyhow!(
                "Blob is empty (no version byte)"
            )));
        }

        let mut buf = [0u8; 1];
        file.read_exact(&mut buf)
            .await
            .context("Failed to read version byte")
            .map_err(CodecError::Internal)?;

        let version = buf[0];
        let physical_body = physical - 1;

        let codec_version = CodecVersion::try_from(version).map_err(|_| {
            CodecError::Internal(anyhow::anyhow!("Unknown encoding version {version}"))
        })?;

        let inner: Box<dyn BlobCodec> = match codec_version {
            CodecVersion::V1 => Box::new(V1Codec::new(file, physical_body)),
        };

        let size = inner.size();
        Ok(BlobHandle { size, inner })
    }

    pub async fn create(dest: &Path) -> Result<Self, CodecError> {
        let codec = V1Codec::create(dest).await?;
        Ok(BlobHandle {
            size: 0,
            inner: Box::new(codec),
        })
    }

    pub async fn write(dest: &Path, mut reader: impl AsyncRead + Unpin) -> Result<(), CodecError> {
        let mut handle = BlobHandle::create(dest).await?;
        tokio::io::copy(&mut reader, &mut handle)
            .await
            .context("Failed to write blob")
            .map_err(CodecError::Internal)?;
        handle.sync().await?;
        Ok(())
    }

    pub async fn sync(&mut self) -> Result<(), CodecError> {
        self.inner.sync().await
    }

    pub async fn seek(&mut self, offset: u64) -> Result<(), CodecError> {
        self.inner.seek(offset).await
    }

    pub async fn size(path: &Path) -> Result<u64, CodecError> {
        let file = tokio::fs::File::open(path)
            .await
            .map_err(|e| match e.kind() {
                ErrorKind::NotFound => CodecError::NotFound(path.to_string_lossy().into_owned()),
                _ => CodecError::Internal(e.into()),
            })?;
        Ok(BlobHandle::open(file).await?.size)
    }
}

impl AsyncRead for BlobHandle {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for BlobHandle {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.get_mut().inner).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_shutdown(cx)
    }
}
