use std::path::PathBuf;

use tracing::debug;

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

/// Canonicalize the name of a secret
pub fn secret_name(name: &str) -> String {
    if name.ends_with(".secret") {
        return name.to_string();
    } else {
        return name.to_string() + ".secret";
    }
}

/// Return the filepath to a secret, useful if you don't want to read the secret content but you still
/// need to refer to the secret. For example, ant-host-agent needs this to replicate secrets down to
/// the deployed services.
pub fn find_secret(secret: &str, secret_dir: Option<PathBuf>) -> PathBuf {
    let secret_dir = secret_dir.unwrap_or_else(|| {
        std::path::PathBuf::from(
            dotenv::var("TYPESOFANTS_SECRET_DIR").expect("no TYPESOFANTS_SECRET_DIR defined"),
        )
    });

    let path = secret_dir.join(secret_name(secret));

    path
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
pub fn load_secret_binary(secret: &str) -> Result<Vec<u8>, anyhow::Error> {
    let path = find_secret(secret, None);

    debug!("Reading secret: {}", path.display());

    let secret_content = std::fs::read(&path)
        .map_err(|e| anyhow::Error::from(e).context(path.to_str().unwrap().to_string()))?;

    Ok(secret_content)
}
