use anyhow::{bail, Context, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

const TOKEN_EXPIRY_HOURS: i64 = 8;
const TOKEN_PREFIX: &str = "admin_";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    sub: i64,
    username: String,
    exp: usize,
    iat: usize,
}

fn jwt_secret() -> Result<String> {
    let secret = std::env::var("ADMIN_JWT_SECRET")
        .context("ADMIN_JWT_SECRET must be set when authentication is enabled")?;
    if secret.len() < 32 {
        bail!("ADMIN_JWT_SECRET must contain at least 32 bytes");
    }
    Ok(secret)
}

pub fn validate_configuration() -> Result<()> {
    jwt_secret().map(|_| ())
}

/// Create a signed, expiring admin token tied to the authenticated database user.
pub fn generate_jwt_token(user_id: i64, username: &str) -> Result<String> {
    let now = Utc::now();
    let expires_at = now + Duration::hours(TOKEN_EXPIRY_HOURS);
    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        iat: now.timestamp() as usize,
        exp: expires_at.timestamp() as usize,
    };
    let jwt = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret()?.as_bytes()),
    )
    .context("failed to sign admin token")?;
    Ok(format!("{TOKEN_PREFIX}{jwt}"))
}

/// Validate signature and expiry, returning the database user identity in the token.
pub fn validate_jwt_token(token: &str) -> Result<(i64, String)> {
    let jwt = token
        .strip_prefix(TOKEN_PREFIX)
        .context("invalid admin token prefix")?;
    let token_data = decode::<Claims>(
        jwt,
        &DecodingKey::from_secret(jwt_secret()?.as_bytes()),
        &Validation::default(),
    )
    .context("invalid or expired admin token")?;
    Ok((token_data.claims.sub, token_data.claims.username))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configure_test_secret() {
        std::env::set_var(
            "ADMIN_JWT_SECRET",
            "test-only-secret-with-at-least-thirty-two-bytes",
        );
    }

    #[test]
    fn signed_token_round_trip() {
        configure_test_secret();
        let token = generate_jwt_token(42, "operator").unwrap();
        assert_eq!(validate_jwt_token(&token).unwrap(), (42, "operator".into()));
    }

    #[test]
    fn forged_prefix_token_is_rejected() {
        configure_test_secret();
        assert!(validate_jwt_token("admin_this-is-not-a-jwt").is_err());
    }
}
