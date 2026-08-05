use crate::{
    redundancy::{replication::Replication, scheme::RedundancyScheme},
    AntArchiveError,
};

pub mod replication;
pub mod scheme;

pub fn from_id(redundancy_strategy: &str) -> Result<Box<dyn RedundancyScheme>, AntArchiveError> {
    let redundancy: Box<dyn RedundancyScheme> = match redundancy_strategy {
        "replication" => Box::new(Replication::new(3)),
        other => {
            return Err(AntArchiveError::InternalServerError(
                "ANT-ERR-136",
                Some(anyhow::anyhow!("Invalid redundancy strategy: {other}")),
            ))
        }
    };

    Ok(redundancy)
}
