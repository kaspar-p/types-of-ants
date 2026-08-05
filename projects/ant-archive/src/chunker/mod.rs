use crate::{
    chunker::{chunker::Chunker, fixed_size::FixedSize, no_chunk::NoChunk},
    AntArchiveError, AntArchiveState,
};

pub mod chunker;
mod crypto;
pub mod fixed_size;
pub mod no_chunk;

pub fn from_id(
    state: &AntArchiveState,
    chunk_strategy: &str,
) -> Result<Box<dyn Chunker>, AntArchiveError> {
    let chunker: Box<dyn Chunker> = match chunk_strategy {
        "no_chunk" => Box::new(NoChunk {}),
        "fixed_size" => Box::new(FixedSize {
            size: state.chunk_size,
        }),
        other => {
            return Err(AntArchiveError::InternalServerError(
                "ANT-ERR-132",
                Some(anyhow::anyhow!(
                    "No valid chunk strategy for chunk strategy: {other}"
                )),
            ))
        }
    };

    Ok(chunker)
}
