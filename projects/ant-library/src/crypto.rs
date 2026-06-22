use argon2::{password_hash::SaltString, Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use base64ct::{Base64, Encoding};
use sha2::Digest;
use tracing::{debug, info};

pub fn make_token_hash(token: &str) -> String {
    let hash = sha2::Sha256::digest(token.as_bytes());
    return Base64::encode_string(&hash);
}

pub async fn make_password_hash(password: &str) -> Result<String, anyhow::Error> {
    let password = password.to_owned();
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut rsa::rand_core::OsRng);
        let argon2 = Argon2::default();

        info!("Hashing password");
        let phc: String = match argon2.hash_password(password.as_bytes(), &salt) {
            Ok(phc) => {
                info!("Password hashed successfully");
                phc.to_string()
            }
            Err(e) => {
                debug!("Hashing password failed: {}", e);
                return Err(anyhow::Error::msg(e.to_string()));
            }
        };

        // Sanity check: verify the hash we just produced is valid.
        info!("Running sanity password check");
        if !verify_password_hash_sync(&password, &phc)? {
            debug!("password self-verification failed");
            return Err(anyhow::Error::msg("sanity test self-verification failed!"));
        }

        Ok(phc)
    })
    .await?
}

pub async fn verify_password_hash(
    password_attempt: &str,
    db_password: &str,
) -> Result<bool, anyhow::Error> {
    let password_attempt = password_attempt.to_owned();
    let db_password = db_password.to_owned();
    tokio::task::spawn_blocking(move || verify_password_hash_sync(&password_attempt, &db_password))
        .await?
}

fn verify_password_hash_sync(
    password_attempt: &str,
    db_password: &str,
) -> Result<bool, anyhow::Error> {
    let argon2 = Argon2::default();

    debug!("Parsing stored password as PHC formatted string...");
    let phc = match PasswordHash::new(db_password) {
        Ok(phc) => phc,
        Err(e) => {
            debug!("Stored password was not PHC formatted string: {}", e);
            return Err(anyhow::Error::msg(e.to_string()));
        }
    };

    debug!("Verifying hash...");
    match argon2.verify_password(password_attempt.as_bytes(), &phc) {
        Err(e) => {
            debug!("hash verification failed: {}", e);
            Ok(false)
        }
        Ok(()) => Ok(true),
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto::{make_password_hash, verify_password_hash};

    #[tokio::test]
    async fn password_hashing_works() {
        let hash = make_password_hash("super-secret-ant-password")
            .await
            .unwrap();
        assert!(hash.contains("argon2"))
    }

    #[tokio::test]
    async fn roundtrip() {
        let hash = make_password_hash("super-secret-ant-password")
            .await
            .unwrap();
        assert!(verify_password_hash("super-secret-ant-password", &hash)
            .await
            .unwrap());
    }
}
