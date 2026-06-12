use clap_complete::engine::CompletionCandidate;

pub fn complete_projects(_current: &std::ffi::OsStr) -> Vec<CompletionCandidate> {
    let Ok(repo) = git2::Repository::discover(".") else {
        return vec![];
    };
    let Some(workdir) = repo.workdir() else {
        return vec![];
    };
    let Ok(entries) = std::fs::read_dir(workdir.join("projects")) else {
        return vec![];
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .map(|s| CompletionCandidate::new(s))
        .collect()
}
