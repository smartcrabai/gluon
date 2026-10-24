//! OAuth provider adapters and a bounded, nonredirecting HTTP transport.
use super::{AuthError, Clock, OAuthExchange, OAuthIdentity, OAuthProvider};
use async_trait::async_trait;
use openidconnect::{
    ClientId, IssuerUrl, Nonce, SignatureVerificationError,
    core::{CoreIdToken, CoreIdTokenVerifier, CoreJsonWebKeySet, CoreProviderMetadata},
};
use serde::Deserialize;
use std::{sync::Arc, time::Duration};

fn failure() -> AuthError {
    AuthError {
        code: "oauth_provider_error",
    }
}
#[async_trait]
pub trait OAuthHttpClient: Send + Sync {
    async fn execute(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, AuthError>;
}

pub struct ReqwestOAuthClient {
    client: Result<reqwest::Client, reqwest::Error>,
    timeout: Duration,
    max_response_bytes: usize,
}
impl ReqwestOAuthClient {
    #[must_use]
    pub fn new(timeout: Duration, max_response_bytes: usize) -> Self {
        Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(timeout)
                .connect_timeout(timeout)
                .build(),
            timeout,
            max_response_bytes,
        }
    }
}
#[async_trait]
impl OAuthHttpClient for ReqwestOAuthClient {
    async fn execute(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<http::Response<Vec<u8>>, AuthError> {
        // The outer deadline covers reading the entire body as well as connecting.
        tokio::time::timeout(self.timeout, async {
            let client = self.client.as_ref().map_err(|_| failure())?;
            let (parts, body) = request.into_parts();
            let mut response = client
                .request(parts.method, parts.uri.to_string())
                .headers(parts.headers)
                .body(body)
                .send()
                .await
                .map_err(|_| failure())?;
            if response
                .content_length()
                .is_some_and(|n| n > self.max_response_bytes as u64)
            {
                return Err(failure());
            }
            let mut result = http::Response::builder()
                .status(response.status())
                .version(response.version());
            *result.headers_mut().ok_or_else(failure)? = response.headers().clone();
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| failure())? {
                if chunk.len() > self.max_response_bytes.saturating_sub(bytes.len()) {
                    return Err(failure());
                }
                bytes.extend_from_slice(&chunk);
            }
            result.body(bytes).map_err(|_| failure())
        })
        .await
        .map_err(|_| failure())?
    }
}
#[derive(Clone)]
pub struct OAuthClientConfig {
    pub client_id: String,
    pub client_secret: String,
    pub callback_url: String,
    pub clock: Arc<dyn Clock>,
}
fn secure_url(value: &str) -> Result<reqwest::Url, AuthError> {
    let url = reqwest::Url::parse(value).map_err(|_| failure())?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(failure());
    }
    Ok(url)
}
fn validate_config(config: &OAuthClientConfig) -> Result<(), AuthError> {
    secure_url(&config.callback_url)?;
    if config.client_id.is_empty() || config.client_secret.is_empty() {
        return Err(failure());
    }
    Ok(())
}
async fn json_request<T: serde::de::DeserializeOwned>(
    client: &dyn OAuthHttpClient,
    request: http::Request<Vec<u8>>,
) -> Result<T, AuthError> {
    secure_url(&request.uri().to_string())?;
    let response = client.execute(request).await?;
    if !response.status().is_success() {
        return Err(failure());
    }
    serde_json::from_slice(response.body()).map_err(|_| failure())
}
fn authorization(
    endpoint: &str,
    config: &OAuthClientConfig,
    state: &str,
    challenge: &str,
    nonce: Option<&str>,
) -> String {
    use oauth2::{AuthUrl, CsrfToken, RedirectUrl, Scope, basic::BasicClient};
    let Ok(auth_url) = AuthUrl::new(endpoint.into()) else {
        return String::new();
    };
    let Ok(redirect_url) = RedirectUrl::new(config.callback_url.clone()) else {
        return String::new();
    };
    let client = BasicClient::new(ClientId::new(config.client_id.clone()))
        .set_auth_uri(auth_url)
        .set_redirect_uri(redirect_url);
    // The public API receives an already-computed S256 challenge, not its secret verifier.
    let mut request = client
        .authorize_url(|| CsrfToken::new(state.into()))
        .add_extra_param("code_challenge", challenge)
        .add_extra_param("code_challenge_method", "S256");
    if let Some(nonce) = nonce {
        request = request
            .add_scope(Scope::new("openid".into()))
            .add_scope(Scope::new("email".into()))
            .add_extra_param("nonce", nonce);
    }
    request.url().0.into()
}
#[derive(Deserialize)]
struct Tokens {
    access_token: String,
    id_token: Option<String>,
}
#[derive(Clone, Debug, serde::Serialize, Deserialize)]
struct TokenExtras {
    id_token: Option<String>,
}
impl oauth2::ExtraTokenFields for TokenExtras {}

struct HttpBridge<'a>(&'a dyn OAuthHttpClient);
impl<'c> oauth2::AsyncHttpClient<'c> for HttpBridge<'_> {
    type Error = std::io::Error;
    type Future = std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<oauth2::HttpResponse, Self::Error>> + Send + 'c,
        >,
    >;
    fn call(&'c self, request: oauth2::HttpRequest) -> Self::Future {
        Box::pin(secure_execute(self.0, request))
    }
}
async fn secure_execute(
    client: &dyn OAuthHttpClient,
    request: http::Request<Vec<u8>>,
) -> Result<http::Response<Vec<u8>>, std::io::Error> {
    secure_url(&request.uri().to_string())
        .map_err(|_| std::io::Error::other("invalid provider URL"))?;
    client
        .execute(request)
        .await
        .map_err(|_| std::io::Error::other("provider HTTP failure"))
}
async fn token_exchange(
    endpoint: &str,
    config: &OAuthClientConfig,
    client: &dyn OAuthHttpClient,
    request: &OAuthExchange,
    basic_auth: bool,
) -> Result<Tokens, AuthError> {
    use oauth2::{
        AuthType, AuthorizationCode, Client, ClientSecret, PkceCodeVerifier, RedirectUrl,
        StandardRevocableToken, StandardTokenResponse, TokenResponse, TokenUrl,
        basic::{
            BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
            BasicTokenType,
        },
    };
    type TokenClient = Client<
        BasicErrorResponse,
        StandardTokenResponse<TokenExtras, BasicTokenType>,
        BasicTokenIntrospectionResponse,
        StandardRevocableToken,
        BasicRevocationErrorResponse,
    >;
    if request.callback != config.callback_url
        || request.code.is_empty()
        || request.pkce_verifier.is_empty()
    {
        return Err(failure());
    }
    let token_client = TokenClient::new(ClientId::new(config.client_id.clone()))
        .set_client_secret(ClientSecret::new(config.client_secret.clone()))
        .set_token_uri(TokenUrl::new(endpoint.into()).map_err(|_| failure())?)
        .set_redirect_uri(RedirectUrl::new(config.callback_url.clone()).map_err(|_| failure())?)
        .set_auth_type(if basic_auth {
            AuthType::BasicAuth
        } else {
            AuthType::RequestBody
        });
    let bridge = HttpBridge(client);
    let response = token_client
        .exchange_code(AuthorizationCode::new(request.code.clone()))
        .set_pkce_verifier(PkceCodeVerifier::new(request.pkce_verifier.clone()))
        .request_async(&bridge)
        .await
        .map_err(|_| failure())?;
    if response.access_token().secret().is_empty()
        || response.token_type() != &BasicTokenType::Bearer
    {
        return Err(failure());
    }
    Ok(Tokens {
        access_token: response.access_token().secret().clone(),
        id_token: response.extra_fields().id_token.clone(),
    })
}

// Do not let provider cache headers extend the lifetime of trusted signing keys.
const SIGNING_KEYS_LIFETIME: time::Duration = time::Duration::minutes(5);

struct SigningKeys {
    jwks: CoreJsonWebKeySet,
    fetched_at: time::OffsetDateTime,
}

pub struct OidcProvider {
    name: String,
    config: OAuthClientConfig,
    client: Arc<dyn OAuthHttpClient>,
    metadata: CoreProviderMetadata,
    signing_keys: tokio::sync::Mutex<SigningKeys>,
}
impl OidcProvider {
    /// Discovers metadata and signing keys from a server-configured trusted issuer.
    ///
    /// # Errors
    /// Returns an error for invalid configuration, discovery failures, or unsafe metadata.
    pub async fn discover(
        name: &str,
        issuer: &str,
        config: OAuthClientConfig,
        client: Arc<dyn OAuthHttpClient>,
    ) -> Result<Self, AuthError> {
        validate_config(&config)?;
        let issuer_url = secure_url(issuer)?;
        if name.is_empty() || issuer_url.query().is_some() {
            return Err(failure());
        }
        // Discovery verifies exact issuer equality. The bridge additionally rejects
        // non-TLS metadata/JWKS URLs before they can trigger network requests.
        let bridge = HttpBridge(client.as_ref());
        let metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new(issuer.into()).map_err(|_| failure())?,
            &bridge,
        )
        .await
        .map_err(|_| failure())?;
        secure_url(metadata.authorization_endpoint().as_str())?;
        if metadata
            .token_endpoint_auth_methods_supported()
            .is_some_and(|methods| {
                !methods.iter().any(|method| {
                    matches!(
                        method,
                        openidconnect::core::CoreClientAuthMethod::ClientSecretBasic
                            | openidconnect::core::CoreClientAuthMethod::ClientSecretPost
                    )
                })
            })
        {
            return Err(failure());
        }
        secure_url(metadata.token_endpoint().ok_or_else(failure)?.as_str())?;
        secure_url(metadata.jwks_uri().as_str())?;
        let signing_keys = tokio::sync::Mutex::new(SigningKeys {
            jwks: metadata.jwks().clone(),
            fetched_at: config.clock.now(),
        });
        Ok(Self {
            name: name.into(),
            config,
            client,
            metadata,
            signing_keys,
        })
    }

    fn verifier(&self, jwks: CoreJsonWebKeySet) -> CoreIdTokenVerifier<'static> {
        let now: std::time::SystemTime = self.config.clock.now().into();
        CoreIdTokenVerifier::new_public_client(
            ClientId::new(self.config.client_id.clone()),
            self.metadata.issuer().clone(),
            jwks,
        )
        .set_allowed_algs(
            self.metadata
                .id_token_signing_alg_values_supported()
                .iter()
                .cloned(),
        )
        .set_time_fn(move || now.into())
        .set_issue_time_verifier_fn(move |issued| {
            if std::time::SystemTime::from(issued) <= now {
                Ok(())
            } else {
                Err("future issue time".into())
            }
        })
    }

    async fn signing_verifier(
        &self,
        token: &CoreIdToken,
    ) -> Result<CoreIdTokenVerifier<'static>, AuthError> {
        // Serialize refreshes and share successful updates across exchanges on this
        // provider (including the Google adapter). No blocking lock crosses await.
        let mut cache = self.signing_keys.lock().await;
        let now = self.config.clock.now();
        let mut verifier = self.verifier(cache.jwks.clone());
        if now < cache.fetched_at
            || now - cache.fetched_at >= SIGNING_KEYS_LIFETIME
            || matches!(
                token.signing_key(&verifier),
                Err(SignatureVerificationError::NoMatchingKey)
            )
        {
            // Only discovery metadata chooses this URL, never a token header. The
            // existing bridge enforces HTTPS and the bounded transport on refresh.
            let bridge = HttpBridge(self.client.as_ref());
            let jwks = CoreJsonWebKeySet::fetch_async(self.metadata.jwks_uri(), &bridge)
                .await
                .map_err(|_| failure())?;
            verifier = self.verifier(jwks.clone());
            *cache = SigningKeys {
                jwks,
                fetched_at: self.config.clock.now(),
            };
        }
        // At most one fetch per received ID token. Missing keys or invalid
        // signatures after refresh fail the normal verification below, not retry.
        Ok(verifier)
    }
}
#[async_trait]
impl OAuthProvider for OidcProvider {
    fn callback_url(&self) -> Option<&str> {
        Some(&self.config.callback_url)
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn authorization_url(&self, state: &str, pkce_challenge: &str, nonce: &str) -> String {
        authorization(
            self.metadata.authorization_endpoint().as_str(),
            &self.config,
            state,
            pkce_challenge,
            Some(nonce),
        )
    }
    async fn exchange(&self, request: OAuthExchange) -> Result<OAuthIdentity, AuthError> {
        if request.nonce.is_empty() {
            return Err(failure());
        }
        let tokens = token_exchange(
            self.metadata.token_endpoint().ok_or_else(failure)?.as_str(),
            &self.config,
            self.client.as_ref(),
            &request,
            self.metadata
                .token_endpoint_auth_methods_supported()
                .is_none_or(|methods| {
                    methods.contains(&openidconnect::core::CoreClientAuthMethod::ClientSecretBasic)
                }),
        )
        .await?;
        let token: CoreIdToken = serde_json::from_value(serde_json::Value::String(
            tokens.id_token.ok_or_else(failure)?,
        ))
        .map_err(|_| failure())?;
        let verifier = self.signing_verifier(&token).await?;
        let claims = token
            .claims(&verifier, &Nonce::new(request.nonce))
            .map_err(|_| failure())?;
        if let Some(expected) = claims.access_token_hash() {
            let actual = openidconnect::AccessTokenHash::from_token(
                &openidconnect::AccessToken::new(tokens.access_token),
                token.signing_alg().map_err(|_| failure())?,
                token.signing_key(&verifier).map_err(|_| failure())?,
            )
            .map_err(|_| failure())?;
            if &actual != expected {
                return Err(failure());
            }
        }
        if claims.subject().as_str().is_empty() {
            return Err(failure());
        }
        Ok(OAuthIdentity {
            issuer: claims.issuer().as_str().into(),
            subject: claims.subject().as_str().into(),
            email: if claims.email_verified() == Some(true) {
                claims.email().map(|v| v.as_str().into())
            } else {
                None
            },
        })
    }
}

pub struct GoogleProvider(OidcProvider);
impl GoogleProvider {
    /// Builds the Google adapter using trusted discovery.
    ///
    /// # Errors
    /// Returns an error when configuration or discovery is invalid.
    pub async fn new(
        config: OAuthClientConfig,
        client: Arc<dyn OAuthHttpClient>,
    ) -> Result<Self, AuthError> {
        Ok(Self(
            OidcProvider::discover("google", "https://accounts.google.com", config, client).await?,
        ))
    }
}
#[async_trait]
impl OAuthProvider for GoogleProvider {
    fn callback_url(&self) -> Option<&str> {
        self.0.callback_url()
    }
    fn name(&self) -> &'static str {
        "google"
    }
    fn authorization_url(&self, state: &str, pkce_challenge: &str, nonce: &str) -> String {
        self.0.authorization_url(state, pkce_challenge, nonce)
    }
    async fn exchange(&self, request: OAuthExchange) -> Result<OAuthIdentity, AuthError> {
        self.0.exchange(request).await
    }
}

pub struct GitHubProvider {
    config: OAuthClientConfig,
    client: Arc<dyn OAuthHttpClient>,
}
impl GitHubProvider {
    /// Builds the GitHub adapter.
    ///
    /// # Errors
    /// Returns an error when client credentials or callback configuration are invalid.
    pub fn new(
        config: OAuthClientConfig,
        client: Arc<dyn OAuthHttpClient>,
    ) -> Result<Self, AuthError> {
        validate_config(&config)?;
        Ok(Self { config, client })
    }
}
#[async_trait]
impl OAuthProvider for GitHubProvider {
    fn callback_url(&self) -> Option<&str> {
        Some(&self.config.callback_url)
    }
    fn name(&self) -> &'static str {
        "github"
    }
    fn authorization_url(&self, state: &str, pkce_challenge: &str, _nonce: &str) -> String {
        authorization(
            "https://github.com/login/oauth/authorize",
            &self.config,
            state,
            pkce_challenge,
            None,
        )
    }
    async fn exchange(&self, request: OAuthExchange) -> Result<OAuthIdentity, AuthError> {
        #[derive(Deserialize)]
        struct User {
            id: u64,
        }
        let tokens = token_exchange(
            "https://github.com/login/oauth/access_token",
            &self.config,
            self.client.as_ref(),
            &request,
            false,
        )
        .await?;
        let request = http::Request::builder()
            .uri("https://api.github.com/user")
            .header("authorization", format!("Bearer {}", tokens.access_token))
            .header("accept", "application/vnd.github+json")
            .header("user-agent", "gluon-auth")
            .header("x-github-api-version", "2022-11-28")
            .body(Vec::new())
            .map_err(|_| failure())?;
        let user: User = json_request(self.client.as_ref(), request).await?;
        if user.id == 0 {
            return Err(failure());
        }
        Ok(OAuthIdentity {
            issuer: "https://github.com".into(),
            subject: user.id.to_string(),
            email: None,
        })
    }
}
