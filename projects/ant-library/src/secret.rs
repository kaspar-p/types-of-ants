use std::str::FromStr;

/// Load a secret from the secret directory wherever it's configured. It must be the case that
/// the TYPESOFANTS_SECRET_DIR is defined in the systemd unit file ass %d, see:
/// https://systemd.io/CREDENTIALS
///
/// Where the name of the file matches the name of the secret with the ".secret" suffix.
/// The file must be in plaintext, either mounted via ramfs in systemd or actually plaintext
/// during development.
///
/// ```rs
/// let db_name: String = load_secret("my_db_name")
/// ```
/// will read the file $TYPESOFANTS_SECRET_DIR/my_db_name.secret
pub fn load_secret(secret_name: &str) -> Result<String, anyhow::Error> {
    Ok(String::from_utf8(load_secret_binary(&secret_name)?)?
        .trim()
        .to_owned())
}

/// Load a secret from the secret directory wherever it's configured. It must be the case that
/// the TYPESOFANTS_SECRET_DIR is defined in the systemd unit file ass %d, see:
/// https://systemd.io/CREDENTIALS
///
/// Where the name of the file matches the name of the secret with the ".secret" suffix.
/// The file must be in plaintext, either mounted via ramfs in systemd or actually plaintext
/// during development.
///
/// ```rs
/// let secret_key: Vec<u8> = load_secret("my-secret_key")
/// ```
/// will read the file $TYPESOFANTS_SECRET_DIR/secret_key.secret
pub fn load_secret_binary(secret_name: &str) -> Result<Vec<u8>, anyhow::Error> {
    let secret_dir =
        dotenv::var("TYPESOFANTS_SECRET_DIR").expect("no TYPESOFANTS_SECRET_DIR defined");

    let path = std::path::PathBuf::from_str(&secret_dir)
        .unwrap()
        .join(secret_name.to_string() + ".secret");

    tracing::debug!("Reading secret: {}", path.to_str().unwrap().to_string());

    let secret_content = std::fs::read(&path)
        .map_err(|e| anyhow::Error::from(e).context(path.to_str().unwrap().to_string()))?;

    Ok(secret_content)
}
