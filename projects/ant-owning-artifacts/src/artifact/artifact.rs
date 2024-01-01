use ant_metadata::{get_typesofants_home, Architecture, ArtifactSelection, Project};
use anyhow::Result;
use axum::body::StreamBody;
use std::path::PathBuf;
use thiserror::Error;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tracing::{debug, info};

pub struct ArtifactFile {
    pub path: PathBuf,
    pub filename: String,
    pub stream: StreamBody<ReaderStream<File>>,
}

#[derive(Debug)]
pub struct Artifact {
    project: Project,
    architecture: Architecture,
    selection: ArtifactSelection,
    filename: String,
    path: PathBuf,
}

#[derive(Debug, Error)]
pub enum ArtifactBuildError {
    FileLoad,
    Checkout,
    Build,
}

impl std::fmt::Display for ArtifactBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactBuildError::Build => f.write_str("Build")?,
            ArtifactBuildError::Checkout => f.write_str("Checkout")?,
            ArtifactBuildError::FileLoad => f.write_str("FileLoad")?,
        }
        return Ok(());
    }
}

impl Artifact {
    pub fn new(
        project: Project,
        architecture: Architecture,
        selection: ArtifactSelection,
    ) -> Artifact {
        let path = Artifact::artifact_path(project, architecture, selection);
        match selection {
            ArtifactSelection::Latest => (),
            ArtifactSelection::SpecificVersion(_) => todo!("SpecificVersion not yet supported!"),
        }
        return Artifact {
            project,
            architecture,
            selection,
            path: path.0,
            filename: path.1,
        };
    }

    fn exists(&self) -> bool {
        return self.path.is_file();
    }

    fn checkout(&self) -> Result<PathBuf> {
        let path = get_typesofants_home().join("repo");
        let repo = if path.exists() && path.is_dir() && path.join(".git").is_dir() {
            git2::Repository::open(path)?
        } else {
            git2::Repository::clone("https://github.com/kaspar-p/types-of-ants", path)?
        };

        let refname = "ref/heads/v1.0";
        let (object, reference) = repo.revparse_ext(refname).expect("Object not found");

        repo.checkout_tree(&object, None)?;
        match reference {
            // gref is an actual reference like branches or tags
            Some(branch_ref) => repo.set_head(branch_ref.name().expect("Branch name not UTF8")),
            // this is a commit, not a reference
            None => repo.set_head_detached(object.id()),
        }?;

        Ok(repo.path().join(self.project.as_str()))
    }

    async fn build(&self, project_path: &PathBuf) -> Result<()> {
        // 3. Run build tooling against it
        anthill::build_artifacts(project_path).await?;

        // 4. From known location, move to artifact path
        todo!("The artifact is somewhere, just don't know where!");
    }

    pub async fn get(self) -> Result<ArtifactFile, ArtifactBuildError> {
        if !self.exists() {
            info!("Building artifact!");
            let path = match self.checkout() {
                Err(e) => {
                    debug!("Error checking repository out: {e}");
                    return Err(ArtifactBuildError::Checkout);
                }
                Ok(path) => path,
            };

            match self.build(&path).await {
                Ok(_) => (),
                Err(e) => {
                    debug!("Error during build: {e}");
                    return Err(ArtifactBuildError::Build);
                }
            };
        }

        return match self.load().await {
            Ok(artifact) => Ok(artifact),
            Err(e) => {
                debug!("Error during loading: {e}");
                return Err(ArtifactBuildError::FileLoad);
            }
        };
    }

    async fn load(self) -> Result<ArtifactFile> {
        let file = tokio::fs::File::open(&self.path).await?;
        let stream = ReaderStream::new(file);
        let body = StreamBody::new(stream);
        return Ok(ArtifactFile {
            path: self.path,
            filename: self.filename,
            stream: body,
        });
    }

    fn artifact_path(
        project: Project,
        arch: Architecture,
        selection: ArtifactSelection,
    ) -> (PathBuf, String) {
        let mut path = get_typesofants_home();
        path.push("artifacts");
        path.push(project.as_str());
        path.push(arch.as_str());

        let filename = selection.as_str();
        path.push(filename.clone());
        path.set_extension("zip");

        (path, filename)
    }
}
