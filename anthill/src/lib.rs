use anyhow::{Error, Result};
use build::{next_frontend, rust_bin};
use config::{BuildKind, ProjectConfig};
use std::env::current_dir;
use std::path::Path;
use std::{fs::File, io::BufReader, path::PathBuf};

mod build;
mod config;

fn read_config<P: AsRef<Path>>(root: P) -> Result<ProjectConfig> {
    let f = File::open(root.as_ref().join("anthill.json"))?;
    let reader = BufReader::new(f);
    let config: ProjectConfig = serde_json::from_reader(reader)?;
    return Ok(config);
}

/// Checks the current directory and all parent directories until it finds
/// a directory that contains an anthill.json file. This is assumes to be the root
/// of the project.
pub fn get_root() -> Result<PathBuf> {
    let mut dir = current_dir()?;
    let mut level = 0;
    let max_level = 4;

    while level < max_level {
        level += 1;
        for entry in dir.read_dir()? {
            let dirent = match entry {
                Err(e) => return Err(e.into()),
                Ok(dirent) => dirent,
            };

            if dirent.file_name() == "anthill.json" {
                return Ok(dirent.path().parent().unwrap().into());
            }
        }

        dir = PathBuf::from(dir.parent().unwrap());
    }

    return Err(Error::msg(
        "No anthill.json file found in the nearest 4 parent directories!",
    ));
}

// Assumes run from root of project, e.g. types-of-ants/ant-on-the-web
async fn build(root: &Path, config: ProjectConfig) -> () {
    for artifact in config.artifacts {
        match artifact.kind {
            BuildKind::NextFrontendExport => next_frontend::build().await,
            BuildKind::RustBin => rust_bin::build(root).await,
        }
    }
}

pub async fn build_artifacts<P: AsRef<Path>>(root: &P) -> Result<()> {
    let config = read_config(root)?;
    build(root.as_ref(), config).await;
    return Ok(());
}
