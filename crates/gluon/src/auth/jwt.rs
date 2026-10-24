//! Session-gated Ed25519 JWT issuance and public verification keys.
use super::{Auth, AuthContext, AuthError, JwtConfig, JwtKey, TokenClaims, random_id};
use ed25519_dalek::{SigningKey, pkcs8::DecodePrivateKey};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
    jwk::{Jwk, PublicKeyUse},
};
use serde_json::{Value, json};
use std::collections::HashSet;

fn invalid_config() -> AuthError {
    AuthError {
        code: "configuration",
    }
}
fn invalid_token() -> AuthError {
    AuthError {
        code: "invalid_token",
    }
}

fn public_key(key: &JwtKey) -> Result<Value, AuthError> {
    let pem = std::str::from_utf8(&key.private_key_pem).map_err(|_| invalid_config())?;
    let signing = SigningKey::from_pkcs8_pem(pem).map_err(|_| invalid_config())?;
    let decoding = DecodingKey::from_ed_der(signing.verifying_key().as_bytes());
    let mut jwk =
        Jwk::from_decoding_key(&decoding, Some(Algorithm::EdDSA)).map_err(|_| invalid_config())?;
    jwk.common.key_id = Some(key.kid.clone());
    jwk.common.public_key_use = Some(PublicKeyUse::Signature);
    serde_json::to_value(jwk).map_err(|_| invalid_config())
}

/// Validate all signing material before serving any authentication requests.
pub(crate) fn validate_jwt(config: &JwtConfig) -> Result<(), AuthError> {
    if config.issuer.trim().is_empty()
        || config.audience.trim().is_empty()
        || config.active_kid.is_empty()
        || config.ttl.whole_seconds() <= 0
        || config.clock_skew.is_negative()
        || config.keys.is_empty()
    {
        return Err(invalid_config());
    }
    let mut ids = HashSet::new();
    for key in &config.keys {
        if key.kid.trim().is_empty() || !ids.insert(key.kid.as_str()) {
            return Err(invalid_config());
        }
        public_key(key)?;
        EncodingKey::from_ed_pem(&key.private_key_pem).map_err(|_| invalid_config())?;
    }
    if !ids.contains(config.active_kid.as_str()) {
        return Err(invalid_config());
    }
    Ok(())
}

impl Auth {
    /// Mint a short-lived external API token for the current cookie session.
    ///
    /// # Errors
    /// Returns an error for an invalid session, disabled JWTs, or invalid signing configuration.
    pub async fn token(&self, context: &AuthContext) -> Result<String, AuthError> {
        let user = self.require_user(context).await?;
        let config = self.config.jwt.as_ref().ok_or(AuthError {
            code: "method_disabled",
        })?;
        let key = config
            .keys
            .iter()
            .find(|key| key.kid == config.active_kid)
            .ok_or_else(invalid_config)?;
        let now = self.config.clock.now().unix_timestamp();
        let claims = TokenClaims {
            iss: config.issuer.clone(),
            aud: config.audience.clone(),
            sub: user.id.0,
            iat: now,
            exp: now
                .checked_add(config.ttl.whole_seconds())
                .ok_or_else(invalid_config)?,
            jti: random_id(),
        };
        let mut header = Header::new(Algorithm::EdDSA);
        header.kid = Some(key.kid.clone());
        let encoding =
            EncodingKey::from_ed_pem(&key.private_key_pem).map_err(|_| invalid_config())?;
        encode(&header, &claims, &encoding).map_err(|_| invalid_config())
    }

    /// Verify independently of cookie sessions. Logout does not revoke issued JWTs.
    ///
    /// # Errors
    /// Returns an error for invalid signatures, claims, key identifiers, or disabled JWTs.
    pub fn verify_token(&self, token: &str) -> Result<TokenClaims, AuthError> {
        let config = self.config.jwt.as_ref().ok_or(AuthError {
            code: "method_disabled",
        })?;
        let header = decode_header(token).map_err(|_| invalid_token())?;
        if header.alg != Algorithm::EdDSA {
            return Err(invalid_token());
        }
        let kid = header.kid.as_deref().ok_or_else(invalid_token)?;
        let key = config
            .keys
            .iter()
            .find(|key| key.kid == kid)
            .ok_or_else(invalid_token)?;
        let jwk = serde_json::from_value(public_key(key)?).map_err(|_| invalid_config())?;
        let decoding = DecodingKey::from_jwk(&jwk).map_err(|_| invalid_config())?;
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_required_spec_claims(&["iss", "aud", "sub", "iat", "exp", "jti"]);
        validation.set_issuer(&[&config.issuer]);
        validation.set_audience(&[&config.audience]);
        // jsonwebtoken uses wall time. Expiration below uses the configured clock.
        validation.validate_exp = false;
        validation.validate_nbf = false;
        let claims = decode::<TokenClaims>(token, &decoding, &validation)
            .map_err(|_| invalid_token())?
            .claims;
        let now = self.config.clock.now().unix_timestamp();
        let skew = config.clock_skew.whole_seconds();
        if claims.sub.is_empty()
            || claims.jti.is_empty()
            || claims.exp <= claims.iat
            || now >= claims.exp.saturating_add(skew)
            || claims.iat > now.saturating_add(skew)
        {
            return Err(invalid_token());
        }
        Ok(claims)
    }

    /// Public-only key set. Retain old configured keys for the rotation overlap.
    #[must_use]
    pub fn jwks(&self) -> Value {
        let keys: Vec<Value> = self
            .config
            .jwt
            .as_ref()
            .map(|config| {
                config
                    .keys
                    .iter()
                    .filter_map(|key| public_key(key).ok())
                    .collect()
            })
            .unwrap_or_default();
        json!({"keys":keys})
    }
}
