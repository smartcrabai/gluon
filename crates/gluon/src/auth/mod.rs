//! Opt-in passwordless authentication shared by Rust callers and HTTP routes.
mod extractors;
mod http;
mod jwt;
mod mail;
pub mod oauth;
mod passkey;
mod store;
use crate::Session;
use async_trait::async_trait;
pub use extractors::CurrentUser;
pub use mail::{SmtpConfig, SmtpEmailSender};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{net::IpAddr, sync::Arc};
pub use store::{
    AuthStore, MemoryAuthStore, PostgresAuthStore, StoreData, StoredChallenge, StoredLogin,
    StoredPasskey, StoredRate, StoredUser,
};
use time::{Duration, OffsetDateTime};

pub trait Clock: Send + Sync {
    fn now(&self) -> OffsetDateTime;
}
struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserId(pub String);
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: UserId,
}
#[derive(Clone, Debug)]
pub struct AuthError {
    pub code: &'static str,
}
impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code)
    }
}
impl std::error::Error for AuthError {}
impl axum::response::IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.code {
            "unauthorized" => ::http::StatusCode::UNAUTHORIZED,
            "rate_limited" => ::http::StatusCode::TOO_MANY_REQUESTS,
            "configuration" | "storage_error" | "storage_capacity" | "schema_mismatch"
            | "internal_error" => ::http::StatusCode::INTERNAL_SERVER_ERROR,
            "user_disabled" | "reauthentication_required" => ::http::StatusCode::FORBIDDEN,
            _ => ::http::StatusCode::BAD_REQUEST,
        };
        (
            status,
            [(::http::header::CACHE_CONTROL, "no-store")],
            axum::Json(serde_json::json!({"code":self.code})),
        )
            .into_response()
    }
}
pub(crate) fn error(code: &'static str) -> AuthError {
    AuthError { code }
}
pub(crate) fn random_id() -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>())
}
#[derive(Clone)]
pub struct AuthContext {
    pub session: Session,
    pub source: IpAddr,
    pub browser_binding: String,
}
#[derive(Clone, Debug)]
pub struct OtpEmail {
    pub recipient: String,
    pub challenge_id: String,
    pub code: String,
}
#[async_trait]
pub trait EmailSender: Send + Sync {
    async fn send(&self, email: OtpEmail) -> Result<(), AuthError>;
}
#[derive(Clone)]
pub struct EmailOtpConfig {
    pub sender: Arc<dyn EmailSender>,
    pub secret: Vec<u8>,
    pub ttl: Duration,
    pub max_attempts: u32,
    pub resend_interval: Duration,
    pub source_limit: u32,
    pub destination_limit: u32,
    pub rate_window: Duration,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OtpChallenge {
    pub challenge_id: String,
}
#[derive(Clone)]
pub struct PasskeyConfig {
    pub rp_id: String,
    pub origin: String,
}
#[derive(Clone)]
pub struct JwtKey {
    pub kid: String,
    pub private_key_pem: Vec<u8>,
}
#[derive(Clone)]
pub struct JwtConfig {
    pub issuer: String,
    pub audience: String,
    pub keys: Vec<JwtKey>,
    pub active_kid: String,
    pub ttl: Duration,
    pub clock_skew: Duration,
}
impl JwtConfig {
    /// Explicit signing material with the default five-minute external-token lifetime.
    /// Adjust `ttl` and `clock_skew` before constructing `Auth` when needed.
    #[must_use]
    pub fn new(issuer: String, audience: String, keys: Vec<JwtKey>, active_kid: String) -> Self {
        Self {
            issuer,
            audience,
            keys,
            active_kid,
            ttl: Duration::minutes(5),
            clock_skew: Duration::ZERO,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthFlow {
    pub flow_id: String,
    pub options: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OAuthStart {
    pub flow_id: String,
    pub authorization_url: String,
}
#[derive(Clone, Debug)]
pub struct OAuthIdentity {
    pub issuer: String,
    pub subject: String,
    pub email: Option<String>,
}
#[derive(Clone, Debug)]
pub struct OAuthExchange {
    pub code: String,
    pub pkce_verifier: String,
    pub nonce: String,
    pub callback: String,
}
#[async_trait]
pub trait OAuthProvider: Send + Sync {
    fn name(&self) -> &str;
    /// The server-configured callback, when owned by the provider adapter.
    fn callback_url(&self) -> Option<&str> {
        None
    }
    fn authorization_url(&self, state: &str, pkce_challenge: &str, nonce: &str) -> String;
    async fn exchange(&self, request: OAuthExchange) -> Result<OAuthIdentity, AuthError>;
}
#[derive(Clone)]
pub struct AuthConfig {
    pub store: Option<Arc<dyn AuthStore>>,
    pub clock: Arc<dyn Clock>,
    pub email_otp: Option<EmailOtpConfig>,
    pub passkey: Option<PasskeyConfig>,
    pub oauth: Vec<Arc<dyn OAuthProvider>>,
    pub jwt: Option<JwtConfig>,
    pub mount_path: String,
    pub http_enabled: bool,
    pub migrate_on_start: bool,
    pub flow_ttl: Duration,
    pub idle_timeout: Duration,
    pub absolute_timeout: Duration,
    pub allowed_origins: Vec<String>,
}
/// A verified identity presented to the application's registration policy.
pub struct VerifiedIdentity {
    pub method: String,
    pub issuer: String,
    pub subject: String,
}
/// Runs after protocol verification and before first identity creation.
/// Return an existing auth user ID to attach a business-owned subject, `None`
/// to allocate a new auth user, or an error to prohibit registration.
pub trait RegistrationPolicy: Send + Sync {
    /// # Errors
    /// Reject registration by returning a stable application policy error code.
    fn register(&self, identity: &VerifiedIdentity) -> Result<Option<UserId>, AuthError>;
}
pub struct Auth {
    pub(crate) config: AuthConfig,
    pub(crate) store: Arc<dyn AuthStore>,
    registration_policy: Option<Arc<dyn RegistrationPolicy>>,
}
pub struct AuthBuilder(AuthConfig);
impl Default for AuthBuilder {
    fn default() -> Self {
        Self::new()
    }
}
impl AuthBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self(AuthConfig {
            store: None,
            clock: Arc::new(SystemClock),
            email_otp: None,
            passkey: None,
            oauth: vec![],
            jwt: None,
            mount_path: "/auth".into(),
            http_enabled: true,
            migrate_on_start: false,
            flow_ttl: Duration::minutes(5),
            idle_timeout: Duration::minutes(30),
            absolute_timeout: Duration::hours(8),
            allowed_origins: vec![],
        })
    }
    #[must_use]
    pub fn store(mut self, store: Arc<dyn AuthStore>) -> Self {
        self.0.store = Some(store);
        self
    }
    #[must_use]
    pub fn clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.0.clock = clock;
        self
    }
    #[must_use]
    pub fn email_otp(mut self, sender: Arc<dyn EmailSender>, secret: Vec<u8>) -> Self {
        self.0.email_otp = Some(EmailOtpConfig {
            sender,
            secret,
            ttl: Duration::minutes(5),
            max_attempts: 5,
            resend_interval: Duration::seconds(60),
            source_limit: 100,
            destination_limit: 10,
            rate_window: Duration::minutes(10),
        });
        self
    }
    /// # Errors
    /// Returns `configuration` for invalid authentication settings.
    pub fn build(self) -> Result<AuthConfig, AuthError> {
        validate_config(&self.0)?;
        Ok(self.0)
    }
}
fn oauth_callback(config: &AuthConfig, provider: &dyn OAuthProvider) -> Result<String, AuthError> {
    let callback = if let Some(callback) = provider.callback_url() {
        callback.to_owned()
    } else {
        let origin = config
            .allowed_origins
            .first()
            .ok_or_else(|| error("configuration"))?;
        format!(
            "{}{}/oauth/{}/callback",
            origin.trim_end_matches('/'),
            config.mount_path,
            provider.name()
        )
    };
    let url = reqwest::Url::parse(&callback).map_err(|_| error("configuration"))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(error("configuration"));
    }
    Ok(callback)
}

fn validate_config(config: &AuthConfig) -> Result<(), AuthError> {
    let p = &config.mount_path;
    if !p.starts_with('/')
        || p == "/"
        || p.ends_with('/')
        || p.contains(['?', '#', '{', '}', '\\'])
        || p.contains("//")
        || p.split('/')
            .any(|s| s == "." || s == ".." || s.starts_with([':', '*']))
        || p.chars().any(char::is_whitespace)
        || config.flow_ttl <= Duration::ZERO
        || config.idle_timeout <= Duration::ZERO
        || config.absolute_timeout <= Duration::ZERO
    {
        return Err(error("configuration"));
    }
    for origin in &config.allowed_origins {
        let url = reqwest::Url::parse(origin).map_err(|_| error("configuration"))?;
        if url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
            || !(url.scheme() == "https"
                || (url.scheme() == "http"
                    && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))))
        {
            return Err(error("configuration"));
        }
    }
    if let Some(otp) = &config.email_otp
        && (otp.secret.len() < 32
            || otp.ttl <= Duration::ZERO
            || otp.max_attempts == 0
            || otp.source_limit == 0
            || otp.destination_limit == 0
            || otp.rate_window <= Duration::ZERO
            || otp.resend_interval < Duration::ZERO)
    {
        return Err(error("configuration"));
    }
    if let Some(passkey) = &config.passkey {
        let url = reqwest::Url::parse(&passkey.origin).map_err(|_| error("configuration"))?;
        webauthn_rs::prelude::WebauthnBuilder::new(&passkey.rp_id, &url)
            .map_err(|_| error("configuration"))?
            .build()
            .map_err(|_| error("configuration"))?;
    }
    if let Some(jwt) = &config.jwt {
        jwt::validate_jwt(jwt)?;
    }
    let mut providers = std::collections::HashSet::new();
    for provider in &config.oauth {
        oauth_callback(config, provider.as_ref())?;
        if provider.name().is_empty()
            || !provider
                .name()
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || !providers.insert(provider.name())
        {
            return Err(error("configuration"));
        }
    }
    Ok(())
}
impl Auth {
    /// Sets the application policy for first-time verified identities. Existing identities are unaffected.
    #[must_use]
    pub fn with_registration_policy(mut self, policy: Arc<dyn RegistrationPolicy>) -> Self {
        self.registration_policy = Some(policy);
        self
    }
    /// Constructs the common authentication service without implicit memory fallback.
    /// # Errors
    /// Returns an error for invalid settings, missing storage, or unavailable schema.
    pub async fn new(
        mut config: AuthConfig,
        pool: Option<sqlx::PgPool>,
    ) -> Result<Self, AuthError> {
        validate_config(&config)?;
        for origin in &mut config.allowed_origins {
            *origin = reqwest::Url::parse(origin)
                .map_err(|_| error("configuration"))?
                .origin()
                .ascii_serialization();
        }
        let store = if let Some(store) = &config.store {
            store.clone()
        } else {
            let store = Arc::new(PostgresAuthStore::new(
                pool.ok_or_else(|| error("configuration"))?,
            ));
            if config.migrate_on_start {
                store.migrate().await?;
            } else {
                store.check_schema().await?;
            }
            store
        };
        Ok(Self {
            config,
            store,
            registration_policy: None,
        })
    }
    pub fn router(self: &Arc<Self>) -> axum::Router<Arc<crate::Container>> {
        if !self.config.http_enabled {
            return axum::Router::new();
        }
        http::router(self.clone())
    }
    pub fn public_router(self: &Arc<Self>) -> axum::Router<Arc<crate::Container>> {
        if !self.config.http_enabled {
            return axum::Router::new();
        }
        http::public_router(self.clone())
    }
}
mod service;
