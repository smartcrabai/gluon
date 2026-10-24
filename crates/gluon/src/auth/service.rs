use super::store::{StoreData, StoredChallenge, StoredLogin, StoredRate, StoredUser};
use super::{
    Auth, AuthContext, AuthError, AuthUser, OAuthExchange, OAuthStart, OtpChallenge, OtpEmail,
    RegistrationPolicy, UserId, VerifiedIdentity, error, random_id,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, KeyInit, Mac};
use rand::RngExt as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::json;
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};
const SESSION_KEY: &str = "__gluon_auth";
#[derive(Serialize, Deserialize)]
struct Login {
    #[serde(default)]
    id: String,
    user: AuthUser,
    authenticated: OffsetDateTime,
    last_seen: OffsetDateTime,
}
#[derive(Serialize, Deserialize)]
struct OtpState {
    version: u32,
    email: String,
    tag: Vec<u8>,
    attempts: u32,
}
fn normalize_email(email: &str) -> Result<String, AuthError> {
    let email = email.trim();
    if email.len() > 254 || email.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(error("invalid_email"));
    }
    let (local, domain) = email
        .rsplit_once('@')
        .ok_or_else(|| error("invalid_email"))?;
    if local.is_empty() || local.contains('@') || domain.is_empty() || !domain.contains('.') {
        return Err(error("invalid_email"));
    }
    Ok(format!("{local}@{}", domain.to_ascii_lowercase()))
}
fn otp_mac(secret: &[u8], id: &str, email: &str, code: &str) -> Result<Hmac<Sha256>, AuthError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|_| error("configuration"))?;
    for component in [id, "email-otp-login", email, code] {
        mac.update(&(component.len() as u64).to_be_bytes());
        mac.update(component.as_bytes());
    }
    Ok(mac)
}
fn resolve_identity(
    data: &mut StoreData,
    key: &str,
    link: Option<&str>,
    policy: Option<&dyn RegistrationPolicy>,
    identity: &VerifiedIdentity,
    now: OffsetDateTime,
) -> Result<AuthUser, AuthError> {
    let id = if let Some(id) = data.identities.get(key) {
        if link.is_some_and(|owner| owner != id) {
            return Err(error("identity_conflict"));
        }
        id.clone()
    } else if let Some(owner) = link {
        if !data.users.get(owner).is_some_and(|user| user.enabled) {
            return Err(error("user_disabled"));
        }
        data.identities.insert(key.into(), owner.into());
        owner.into()
    } else {
        let selected = policy
            .map(|policy| policy.register(identity))
            .transpose()?
            .flatten();
        let id = selected.map_or_else(|| uuid::Uuid::new_v4().to_string(), |id| id.0);
        if id.is_empty() {
            return Err(error("invalid_identity"));
        }
        if data.users.get(&id).is_some_and(|user| !user.enabled) {
            return Err(error("user_disabled"));
        }
        if !data.users.contains_key(&id) {
            let user = AuthUser {
                id: UserId(id.clone()),
            };
            data.users.insert(
                id.clone(),
                StoredUser {
                    user,
                    enabled: true,
                    created_at: now,
                },
            );
        }
        data.identities.insert(key.into(), id.clone());
        id
    };
    let user = data.users.get(&id).ok_or_else(|| error("storage_error"))?;
    if !user.enabled {
        return Err(error("user_disabled"));
    }
    Ok(user.user.clone())
}
impl Auth {
    pub(crate) fn identity_available(&self, key: &str) -> bool {
        if key.starts_with("email-otp:") {
            return self.config.email_otp.is_some();
        }
        let Ok((method, provider, _, _)) =
            serde_json::from_str::<(String, String, String, String)>(key)
        else {
            return false;
        };
        method == "oauth"
            && self
                .config
                .oauth
                .iter()
                .any(|configured| configured.name() == provider)
    }
    pub(crate) async fn establish_session(
        &self,
        context: &AuthContext,
        user: AuthUser,
    ) -> Result<AuthUser, AuthError> {
        self.revoke_login(context).await?;
        context
            .session
            .flush()
            .await
            .map_err(|_| error("storage_error"))?;
        context
            .session
            .cycle_id()
            .await
            .map_err(|_| error("storage_error"))?;
        let now = self.config.clock.now();
        let id = random_id();
        self.store.prepare_capacity(now).await?;
        let persisted = async {
            context
                .session
                .insert(
                    SESSION_KEY,
                    Login {
                        id: id.clone(),
                        user: user.clone(),
                        authenticated: now,
                        last_seen: now,
                    },
                )
                .await
                .map_err(|_| error("storage_error"))?;
            context
                .session
                .save()
                .await
                .map_err(|_| error("storage_error"))
        }
        .await;
        if let Err(failure) = persisted {
            let _ = context.session.flush().await;
            return Err(failure);
        }
        // Only establishment can create authority, and only after cookie storage
        // succeeds. Restoration never inserts a missing login record.
        if let Err(failure) = self
            .store
            .transact(&mut |data| {
                if !data
                    .users
                    .get(&user.id.0)
                    .is_some_and(|stored| stored.enabled)
                {
                    return Err(error("user_disabled"));
                }
                data.logins.insert(
                    id.clone(),
                    StoredLogin {
                        user: user.id.0.clone(),
                        authenticated: now,
                        last_seen: now,
                        idle_expires: now + self.config.idle_timeout,
                        absolute_expires: now + self.config.absolute_timeout,
                    },
                );
                Ok(())
            })
            .await
        {
            let _ = context.session.flush().await;
            return Err(failure);
        }
        Ok(user)
    }
    /// Restores an enabled user from a live cookie session and extends only its idle deadline.
    /// # Errors
    /// Returns storage errors rather than treating unavailable state as anonymous.
    pub async fn current_user(&self, context: &AuthContext) -> Result<Option<AuthUser>, AuthError> {
        let Some(login) = context
            .session
            .get::<Login>(SESSION_KEY)
            .await
            .map_err(|_| error("storage_error"))?
        else {
            return Ok(None);
        };
        let now = self.config.clock.now();
        let mut live = false;
        self.store
            .transact(&mut |data| {
                let enabled = data.users.get(&login.user.id.0).is_some_and(|u| u.enabled);
                if let Some(record) = data.logins.get_mut(&login.id) {
                    live = enabled
                        && record.user == login.user.id.0
                        && record.authenticated == login.authenticated
                        && now < record.idle_expires
                        && now < record.absolute_expires
                        && now < record.authenticated + self.config.absolute_timeout
                        && now < record.last_seen + self.config.idle_timeout;
                    if live {
                        record.last_seen = record.last_seen.max(now);
                        record.idle_expires = record.last_seen + self.config.idle_timeout;
                    }
                }
                Ok(())
            })
            .await?;
        if !live {
            context
                .session
                .flush()
                .await
                .map_err(|_| error("storage_error"))?;
            return Ok(None);
        }
        Ok(Some(login.user))
    }
    pub(crate) async fn require_user(&self, context: &AuthContext) -> Result<AuthUser, AuthError> {
        self.current_user(context)
            .await?
            .ok_or_else(|| error("unauthorized"))
    }
    pub(crate) async fn require_recent_user(
        &self,
        context: &AuthContext,
    ) -> Result<AuthUser, AuthError> {
        let user = self.require_user(context).await?;
        let login = context
            .session
            .get::<Login>(SESSION_KEY)
            .await
            .map_err(|_| error("storage_error"))?
            .ok_or_else(|| error("unauthorized"))?;
        if self.config.clock.now() >= login.authenticated + Duration::minutes(5) {
            return Err(error("reauthentication_required"));
        }
        Ok(user)
    }
    /// Deletes the server-side session before reporting completion.
    /// # Errors
    /// Returns an error when server-side deletion fails.
    pub async fn logout(&self, context: &AuthContext) -> Result<(), AuthError> {
        self.revoke_login(context).await?;
        context
            .session
            .flush()
            .await
            .map_err(|_| error("storage_error"))
    }
    async fn revoke_login(&self, context: &AuthContext) -> Result<(), AuthError> {
        if let Some(login) = context
            .session
            .get::<Login>(SESSION_KEY)
            .await
            .map_err(|_| error("storage_error"))?
        {
            self.store
                .transact(&mut |data| {
                    data.logins.remove(&login.id);
                    Ok(())
                })
                .await?;
        }
        Ok(())
    }
    /// Issues and delivers an eight-digit, single-use email challenge.
    /// # Errors
    /// Rejects invalid addresses, rate limits, disabled methods, storage and delivery failures.
    pub async fn request_email_otp(
        &self,
        context: &AuthContext,
        email: &str,
    ) -> Result<OtpChallenge, AuthError> {
        let config = self
            .config
            .email_otp
            .as_ref()
            .ok_or_else(|| error("method_disabled"))?;
        let email = normalize_email(email)?;
        let id = random_id();
        let code = format!("{:08}", rand::rng().random_range(0..100_000_000u32));
        let tag = otp_mac(&config.secret, &id, &email, &code)?
            .finalize()
            .into_bytes()
            .to_vec();
        let now = self.config.clock.now();
        let limits = [
            (
                format!("otp-source:{}", context.source),
                config.source_limit,
                config.rate_window,
            ),
            (
                format!("otp-destination:{email}"),
                config.destination_limit,
                config.rate_window,
            ),
            (format!("otp-resend:{email}"), 1, config.resend_interval),
        ];
        self.store.prepare_capacity(now).await?;
        self.store
            .transact(&mut |data| {
                data.rates.retain(|_, rate| rate.expires > now);
                for (key, limit, _) in &limits {
                    if data.rates.get(key).is_some_and(|rate| rate.count >= *limit) {
                        return Err(error("rate_limited"));
                    }
                }
                for (key, _, window) in &limits {
                    let rate = data.rates.entry(key.clone()).or_insert(StoredRate {
                        count: 0,
                        expires: now + *window,
                    });
                    rate.count += 1;
                }
                data.challenges.retain(|_, flow| {
                    !(flow.kind == "otp"
                        && flow.data.get("email").and_then(Value::as_str) == Some(&email))
                });
                data.challenges.insert(
                    id.clone(),
                    StoredChallenge {
                        kind: "otp".into(),
                        binding: context.browser_binding.clone(),
                        user: None,
                        expires: now + config.ttl,
                        data: serde_json::to_value(OtpState {
                            version: 1,
                            email: email.clone(),
                            tag: tag.clone(),
                            attempts: 0,
                        })
                        .map_err(|_| error("internal_error"))?,
                    },
                );
                Ok(())
            })
            .await?;
        if let Err(failure) = config
            .sender
            .send(OtpEmail {
                recipient: email,
                challenge_id: id.clone(),
                code,
            })
            .await
        {
            self.store
                .transact(&mut |data| {
                    data.challenges.remove(&id);
                    Ok(())
                })
                .await?;
            return Err(failure);
        }
        Ok(OtpChallenge { challenge_id: id })
    }
    /// Verifies and atomically consumes a challenge, then persists a rotated login session.
    /// # Errors
    /// Rejects invalid, expired or exhausted challenges and persistence failures.
    pub async fn verify_email_otp(
        &self,
        context: &AuthContext,
        challenge_id: &str,
        code: &str,
    ) -> Result<AuthUser, AuthError> {
        let config = self
            .config
            .email_otp
            .as_ref()
            .ok_or_else(|| error("method_disabled"))?;
        let now = self.config.clock.now();
        let mut user = None;
        self.store
            .transact(&mut |data| {
                let challenge = data
                    .challenges
                    .get_mut(challenge_id)
                    .ok_or_else(|| error("invalid_otp"))?;
                if challenge.kind != "otp"
                    || challenge.expires <= now
                    || challenge.binding != context.browser_binding
                {
                    return Err(error("invalid_otp"));
                }
                let mut otp =
                    OtpState::deserialize(&challenge.data).map_err(|_| error("storage_error"))?;
                if otp.version != 1 || otp.attempts >= config.max_attempts {
                    return Err(error("invalid_otp"));
                }
                otp.attempts += 1;
                let valid = code.len() == 8
                    && code.bytes().all(|b| b.is_ascii_digit())
                    && otp_mac(&config.secret, challenge_id, &otp.email, code)?
                        .verify_slice(&otp.tag)
                        .is_ok();
                challenge.data = serde_json::to_value(&otp).map_err(|_| error("internal_error"))?;
                if !valid {
                    return Err(error("invalid_otp"));
                }
                data.challenges.remove(challenge_id);
                user = Some(resolve_identity(
                    data,
                    &format!("email-otp:{}", otp.email),
                    None,
                    self.registration_policy.as_deref(),
                    &VerifiedIdentity {
                        method: "email-otp".into(),
                        issuer: String::new(),
                        subject: otp.email,
                    },
                    now,
                )?);
                Ok(())
            })
            .await?;
        self.establish_session(context, user.ok_or_else(|| error("internal_error"))?)
            .await
    }
    /// Starts a browser-bound authorization code flow, optionally linking the current user.
    /// # Errors
    /// Rejects disabled providers, invalid configuration, unauthenticated linking and storage failures.
    pub async fn start_oauth(
        &self,
        context: &AuthContext,
        provider: &str,
        link: bool,
    ) -> Result<OAuthStart, AuthError> {
        let provider = self
            .config
            .oauth
            .iter()
            .find(|p| p.name() == provider)
            .ok_or_else(|| error("method_disabled"))?;
        let user = if link {
            Some(self.require_recent_user(context).await?.id.0)
        } else {
            None
        };
        let state = random_id();
        let nonce = random_id();
        let verifier = random_id();
        let pkce = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let callback = super::oauth_callback(&self.config, provider.as_ref())?;
        let authorization_url = provider.authorization_url(&state, &pkce, &nonce);
        let url = reqwest::Url::parse(&authorization_url).map_err(|_| error("configuration"))?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(error("configuration"));
        }
        let now = self.config.clock.now();
        self.store.prepare_capacity(now).await?;
        self.store
            .transact(&mut |data| {
                data.rates.retain(|_, rate| rate.expires > now);
                data.challenges.insert(state.clone(), StoredChallenge {
                kind: format!("oauth:{}", provider.name()),
                binding: context.browser_binding.clone(),
                user: user.clone(),
                expires: now + self.config.flow_ttl,
                data: json!({"version":1, "verifier":verifier, "nonce":nonce, "callback":callback}),
            });
                Ok(())
            })
            .await?;
        Ok(OAuthStart {
            flow_id: state,
            authorization_url,
        })
    }
    /// Consumes a saved flow before exchanging its authorization code with the trusted provider.
    /// # Errors
    /// Rejects invalid flows, provider failures, identity conflicts and session persistence errors.
    pub async fn finish_oauth(
        &self,
        context: &AuthContext,
        provider: &str,
        state: &str,
        code: &str,
    ) -> Result<AuthUser, AuthError> {
        let implementation = self
            .config
            .oauth
            .iter()
            .find(|p| p.name() == provider)
            .ok_or_else(|| error("method_disabled"))?;
        let mut saved = None;
        self.store
            .transact(&mut |data| {
                let flow = data
                    .challenges
                    .remove(state)
                    .ok_or_else(|| error("invalid_flow"))?;
                if flow.kind != format!("oauth:{provider}")
                    || flow.binding != context.browser_binding
                    || flow.expires <= self.config.clock.now()
                {
                    return Err(error("invalid_flow"));
                }
                saved = Some(flow);
                Ok(())
            })
            .await?;
        let flow = saved.ok_or_else(|| error("invalid_flow"))?;
        if flow.data.get("version").and_then(Value::as_u64) != Some(1) {
            return Err(error("storage_error"));
        }
        if let Some(owner) = &flow.user
            && self.require_recent_user(context).await?.id.0 != *owner
        {
            return Err(error("invalid_flow"));
        }
        if code.is_empty() {
            return Err(error("invalid_flow"));
        }
        let field = |name: &str| {
            flow.data
                .get(name)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| error("storage_error"))
        };
        let identity = implementation
            .exchange(OAuthExchange {
                code: code.into(),
                pkce_verifier: field("verifier")?,
                nonce: field("nonce")?,
                callback: field("callback")?,
            })
            .await?;
        if identity.issuer.is_empty() || identity.subject.is_empty() {
            return Err(error("invalid_identity"));
        }
        let key = serde_json::to_string(&("oauth", provider, &identity.issuer, &identity.subject))
            .map_err(|_| error("internal_error"))?;
        let mut user = None;
        self.store
            .transact(&mut |data| {
                user = Some(resolve_identity(
                    data,
                    &key,
                    flow.user.as_deref(),
                    self.registration_policy.as_deref(),
                    &VerifiedIdentity {
                        method: format!("oauth:{provider}"),
                        issuer: identity.issuer.clone(),
                        subject: identity.subject.clone(),
                    },
                    self.config.clock.now(),
                )?);
                Ok(())
            })
            .await?;
        let user = user.ok_or_else(|| error("internal_error"))?;
        if flow.user.is_some() {
            Ok(user)
        } else {
            self.establish_session(context, user).await
        }
    }
}
