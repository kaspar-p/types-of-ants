pub mod blob;
pub mod v1;

pub use blob::BlobHandle;
pub use v1::V1Codec;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::err::AntArchiveStorageError;

#[derive(Debug)]
pub enum CodecError {
    NotFound(String),
    Internal(anyhow::Error),
}

impl From<CodecError> for AntArchiveStorageError {
    fn from(e: CodecError) -> Self {
        match e {
            CodecError::NotFound(s) => AntArchiveStorageError::NotFound(s),
            CodecError::Internal(e) => AntArchiveStorageError::InternalServerError(Some(e)),
        }
    }
}

#[async_trait]
pub trait BlobCodec: AsyncRead + AsyncWrite + Unpin + Send {
    fn size(&self) -> u64;
    async fn seek(&mut self, offset: u64) -> Result<(), CodecError>;
    async fn sync(&mut self) -> Result<(), CodecError>;
}
