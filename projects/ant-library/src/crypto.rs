use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use base64ct::{Base64, Encoding};
use sha2::Digest;
use tracing::{debug, info};

pub fn make_token_hash(token: &str) -> String {
    let hash = sha2::Sha256::digest(token.as_bytes());
    return Base64::encode_string(&hash);
}

pub fn make_password_hash(password: &str) -> Result<String, anyhow::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    // Step 1: Hash the password using the salt
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

    // Step 2: Sanity check verify works
    info!("Running sanity password check");
    if !verify_password_hash(password, phc.as_str())? {
        debug!("password self-verification failed");
        return Err(anyhow::Error::msg("sanity test self-verification failed!"));
    }

    return Ok(phc);
}

pub fn verify_password_hash(
    password_attempt: &str,
    db_password: &str,
) -> Result<bool, anyhow::Error> {
    // Step: Verify attempt with stored PHC string
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
            return Ok(false);
        }
        Ok(()) => {
            return Ok(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto::{make_password_hash, verify_password_hash};

    #[test]
    fn password_hashing_works() {
        let hash = make_password_hash("super-secret-ant-password").unwrap();
        println!("{}", hash);

        assert!(hash.contains("argon2"))
    }

    #[test]
    fn roundtrip() {
        let hash = make_password_hash("super-secret-ant-password").unwrap();
        println!("{}", hash);

        assert!(verify_password_hash("super-secret-ant-password", &hash).unwrap());
    }
}
