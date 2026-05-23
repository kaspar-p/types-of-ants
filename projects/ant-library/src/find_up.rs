pub fn find_up(filename: &str) -> std::path::PathBuf {
    let mut dir = std::env::current_dir().unwrap();

    loop {
        let candidate = dir.join(filename);
        if std::fs::exists(&candidate).unwrap() {
            return candidate;
        }

        dir = dir
            .parent()
            .expect(&format!("got to root without finding: {filename}"))
            .to_path_buf()
    }
}
