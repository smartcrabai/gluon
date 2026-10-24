//! Public, test-only signing material. Never use these keys outside tests.
#![allow(dead_code)]
use gluon::auth::{JwtConfig, JwtKey};
use time::Duration;

pub const OLD_KEY: &[u8] = b"-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIEmyZSI2Yqo54etzjA/kMeF9rVI5ixzZ1YKPQEFh9XKt\n-----END PRIVATE KEY-----\n";
pub const NEW_KEY: &[u8] = b"-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIFjz/vZ21pr8S/9/ywd+4m6niDcJRuWHpLs9qJ9tAst3\n-----END PRIVATE KEY-----\n";

pub fn jwt_config() -> JwtConfig {
    JwtConfig {
        issuer: "https://auth.example.test".into(),
        audience: "external-api".into(),
        keys: vec![JwtKey {
            kid: "old-key".into(),
            private_key_pem: OLD_KEY.to_vec(),
        }],
        active_kid: "old-key".into(),
        ttl: Duration::minutes(5),
        clock_skew: Duration::ZERO,
    }
}
