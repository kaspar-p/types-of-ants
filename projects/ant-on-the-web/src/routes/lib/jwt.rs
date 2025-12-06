use std::sync::LazyLock;

use jsonwebtoken::{DecodingKey, EncodingKey, Header};
use serde::{de::DeserializeOwned, Serialize};
use tracing::debug;

use super::err::AntOnTheWebError;

static JWT_SECRET_KEYS: LazyLock<JwtSecretKeys> = LazyLock::new(|| {
    debug!("Initializing jwt secret...");
    let secret = ant_library::secret::load_secret("jwt").expect("jwt secret");

    debug!("jwt secret initialized...");
    JwtSecretKeys::new(secret.as_bytes())
});

struct JwtSecretKeys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl JwtSecretKeys {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

/// Create a new signed JWT containing some secret claim that the server can later verify.
pub fn encode_jwt<T: Serialize>(claims: &T) -> Result<String, anyhow::Error> {
    let jwt = jsonwebtoken::encode(&Header::default(), &claims, &JWT_SECRET_KEYS.encoding)?;
    Ok(jwt)
}

/// Decode a new JWT containing some claim that the server verified.
/// If the decoding fails, this process returns access denied.
pub fn decode_jwt<T: DeserializeOwned>(token: &str) -> Result<T, AntOnTheWebError> {
    let a = jsonwebtoken::decode(
        &token,
        &JWT_SECRET_KEYS.decoding,
        &jsonwebtoken::Validation::default(),
    )
    .map_err(|e| {
        AntOnTheWebError::AccessDenied(Some("tampered jwt: ".to_string() + &e.to_string()))
    })?;

    return Ok(a.claims);
}

#[cfg(test)]
mod test {
    use std::env::set_var;

    use chrono::{Duration, Utc};
    use serde::{Deserialize, Serialize};

    use super::{decode_jwt, encode_jwt};

    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
    struct Claims {
        pub sub: String,
        pub exp: i64,
    }

    #[test]
    fn inverses() {
        set_var("TYPESOFANTS_SECRET_DIR", "./tests/integration/test-secrets");

        let claims = Claims {
            sub: "sub".to_string(),
            exp: (Utc::now() + Duration::days(5)).timestamp(),
        };

        let jwt = encode_jwt(&claims).unwrap();
        let claims2: Claims = decode_jwt(&jwt).unwrap();

        assert_eq!(claims, claims2);
    }
}
