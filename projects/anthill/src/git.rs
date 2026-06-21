use std::path::PathBuf;

use chrono::{Datelike, Timelike};
use git2::{Commit, Repository};

#[derive(Clone)]
pub struct GitState {
    pub root: PathBuf,
    pub head_sha: String,
    pub head_number: i32,
    pub head_datetime: String,
}

impl GitState {
    pub fn new() -> Result<Self, anyhow::Error> {
        let repo = git2::Repository::discover(".")?;
        let commit = Self::get_head_commit_data(&repo)?;

        Ok(Self {
            root: repo.workdir().unwrap().to_path_buf(),
            head_sha: commit.id().to_string().chars().take(8).collect(),
            head_number: Self::commit_count(&repo)?,
            head_datetime: Self::format_datetime(commit.time()),
        })
    }

    pub fn version(&self) -> String {
        format!(
            "{}-{}-{}",
            self.head_number, self.head_datetime, self.head_sha
        )
    }

    fn commit_count(repo: &Repository) -> Result<i32, anyhow::Error> {
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        Ok(revwalk.count() as i32)
    }

    fn get_head_commit_data<'a>(repo: &'a Repository) -> Result<Commit<'a>, anyhow::Error> {
        let obj = repo.head()?.resolve()?.peel(git2::ObjectType::Commit)?;
        let commit = obj
            .into_commit()
            .map_err(|_| git2::Error::from_str("Not a commit"))?;

        Ok(commit)
    }

    fn format_datetime(t: git2::Time) -> String {
        let d = chrono::DateTime::from_timestamp_secs(t.seconds()).expect("malformed datetime");
        format!(
            "{}-{}-{}-{}-{}",
            d.year(),
            d.month(),
            d.day(),
            d.hour(),
            d.minute()
        )
    }
}
