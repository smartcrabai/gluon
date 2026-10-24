//! `WebAuthn` ceremonies. Verification state never leaves the server-side store.
use super::store::{StoredChallenge, StoredPasskey};
use super::{Auth, AuthContext, AuthError, AuthFlow, AuthUser, UserId, random_id};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use webauthn_rs::prelude::*;

fn invalid() -> AuthError {
    AuthError {
        code: "invalid_passkey",
    }
}

fn internal() -> AuthError {
    AuthError {
        code: "storage_error",
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct SavedKey {
    format: u32,
    credential: Passkey,
    user_handle: Uuid,
}
fn decode_key(value: &Value) -> Result<SavedKey, AuthError> {
    let key = SavedKey::deserialize(value).map_err(|_| internal())?;
    if key.format != 1 {
        return Err(internal());
    }
    Ok(key)
}

#[derive(Serialize, Deserialize)]
struct Ceremony {
    format: u32,
    state: Value,
    handle: Option<Uuid>,
    versions: HashMap<String, u64>,
}

impl Auth {
    fn webauthn(&self) -> Result<Webauthn, AuthError> {
        let cfg = self.config.passkey.as_ref().ok_or(AuthError {
            code: "method_disabled",
        })?;
        let origin = Url::parse(&cfg.origin).map_err(|_| AuthError {
            code: "configuration",
        })?;
        WebauthnBuilder::new(&cfg.rp_id, &origin)
            .and_then(WebauthnBuilder::build)
            .map_err(|_| AuthError {
                code: "configuration",
            })
    }

    async fn save_passkey_flow(
        &self,
        context: &AuthContext,
        kind: &str,
        user: Option<String>,
        options: Value,
        ceremony: Ceremony,
    ) -> Result<AuthFlow, AuthError> {
        let flow_id = random_id();
        let data = serde_json::to_value(ceremony).map_err(|_| internal())?;
        let now = self.config.clock.now();
        let expires = now + self.config.flow_ttl;
        self.store.prepare_capacity(now).await?;
        self.store
            .transact(&mut |store| {
                store.challenges.insert(
                    flow_id.clone(),
                    StoredChallenge {
                        kind: kind.to_owned(),
                        binding: context.browser_binding.clone(),
                        user: user.clone(),
                        expires,
                        data: data.clone(),
                    },
                );
                Ok(())
            })
            .await?;
        Ok(AuthFlow { flow_id, options })
    }

    // Commit consumption before cryptographic validation, including failed signatures.
    async fn take_passkey_flow(
        &self,
        context: &AuthContext,
        flow_id: &str,
        kind: &str,
    ) -> Result<(Option<String>, Ceremony), AuthError> {
        let mut taken = None;
        self.store
            .transact(&mut |store| {
                taken = store.challenges.remove(flow_id);
                Ok(())
            })
            .await?;
        let flow = taken.ok_or_else(invalid)?;
        if flow.kind != kind
            || flow.binding != context.browser_binding
            || flow.expires <= self.config.clock.now()
        {
            return Err(invalid());
        }
        let ceremony: Ceremony = serde_json::from_value(flow.data).map_err(|_| internal())?;
        if ceremony.format != 1 {
            return Err(internal());
        }
        Ok((flow.user, ceremony))
    }

    /// Start enrollment for the recently authenticated session owner.
    ///
    /// # Errors
    /// Returns an error when passkeys are disabled, recent authentication is
    /// missing, configuration is invalid, or server-side state cannot be stored.
    pub async fn start_passkey_registration(
        &self,
        context: &AuthContext,
    ) -> Result<AuthFlow, AuthError> {
        let webauthn = self.webauthn()?;
        let user = self.require_recent_user(context).await?;
        let mut keys = Vec::new();
        self.store
            .transact(&mut |store| {
                keys.clear();
                for stored in store.passkeys.values().filter(|key| key.user == user.id.0) {
                    keys.push(decode_key(&stored.data)?);
                }
                Ok(())
            })
            .await?;
        let handle = keys
            .first()
            .map_or_else(Uuid::new_v4, |key| key.user_handle);
        let excluded = keys
            .iter()
            .map(|key| key.credential.cred_id().clone())
            .collect();
        let (options, state) = webauthn
            .start_passkey_registration(handle, &user.id.0, &user.id.0, Some(excluded))
            .map_err(|_| invalid())?;
        self.save_passkey_flow(
            context,
            "passkey_registration",
            Some(user.id.0),
            serde_json::to_value(options).map_err(|_| internal())?,
            Ceremony {
                format: 1,
                state: serde_json::to_value(state).map_err(|_| internal())?,
                handle: Some(handle),
                versions: HashMap::new(),
            },
        )
        .await
    }

    /// Verify an enrollment response and store its globally unique credential.
    ///
    /// # Errors
    /// Returns an error for expired, replayed, misbound, or invalid ceremonies,
    /// missing recent authentication, duplicate credentials, or storage failure.
    pub async fn finish_passkey_registration(
        &self,
        context: &AuthContext,
        flow_id: &str,
        response: Value,
    ) -> Result<(), AuthError> {
        let webauthn = self.webauthn()?;
        let (owner, ceremony) = self
            .take_passkey_flow(context, flow_id, "passkey_registration")
            .await?;
        let user = self.require_recent_user(context).await?;
        if owner.as_deref() != Some(user.id.0.as_str()) {
            return Err(invalid());
        }
        let response: RegisterPublicKeyCredential =
            serde_json::from_value(response).map_err(|_| invalid())?;
        let state: PasskeyRegistration =
            serde_json::from_value(ceremony.state).map_err(|_| internal())?;
        let credential = webauthn
            .finish_passkey_registration(&response, &state)
            .map_err(|_| invalid())?;
        let id = URL_SAFE_NO_PAD.encode(credential.cred_id().as_ref());
        let key = SavedKey {
            format: 1,
            credential,
            user_handle: ceremony.handle.ok_or_else(internal)?,
        };
        let data = serde_json::to_value(key).map_err(|_| internal())?;
        self.store
            .transact(&mut |store| {
                if !store.users.get(&user.id.0).is_some_and(|user| user.enabled)
                    || store.passkeys.contains_key(&id)
                {
                    return Err(invalid());
                }
                store.passkeys.insert(
                    id.clone(),
                    StoredPasskey {
                        user: user.id.0.clone(),
                        data: data.clone(),
                        version: 1,
                    },
                );
                Ok(())
            })
            .await
    }

    /// Start an identified-user or conditional discoverable authentication.
    ///
    /// # Errors
    /// Returns an error for disabled or misconfigured passkeys, an unknown or
    /// disabled identified user, unavailable credentials, or storage failure.
    pub async fn start_passkey_authentication(
        &self,
        context: &AuthContext,
        user: Option<&UserId>,
    ) -> Result<AuthFlow, AuthError> {
        let webauthn = self.webauthn()?;
        let mut versions = HashMap::new();
        let (options, state) = if let Some(user) = user {
            let mut keys = Vec::new();
            self.store
                .transact(&mut |store| {
                    keys.clear();
                    versions.clear();
                    if !store.users.get(&user.0).is_some_and(|user| user.enabled) {
                        return Err(invalid());
                    }
                    for (id, stored) in store.passkeys.iter().filter(|(_, key)| key.user == user.0)
                    {
                        keys.push(decode_key(&stored.data)?.credential);
                        versions.insert(id.clone(), stored.version);
                    }
                    Ok(())
                })
                .await?;
            if keys.is_empty() {
                return Err(invalid());
            }
            let (options, state) = webauthn
                .start_passkey_authentication(&keys)
                .map_err(|_| invalid())?;
            (
                options,
                serde_json::to_value(state).map_err(|_| internal())?,
            )
        } else {
            let (options, state) = webauthn
                .start_discoverable_authentication()
                .map_err(|_| invalid())?;
            (
                options,
                serde_json::to_value(state).map_err(|_| internal())?,
            )
        };
        self.save_passkey_flow(
            context,
            "passkey_authentication",
            user.map(|user| user.0.clone()),
            serde_json::to_value(options).map_err(|_| internal())?,
            Ceremony {
                format: 1,
                state,
                handle: None,
                versions,
            },
        )
        .await
    }

    /// Verify a signed assertion and establish a fresh authenticated session.
    ///
    /// # Errors
    /// Returns an error for invalid, expired, replayed, or stale ceremonies,
    /// disabled users, failed signature or user verification, or persistence errors.
    pub async fn finish_passkey_authentication(
        &self,
        context: &AuthContext,
        flow_id: &str,
        response: Value,
    ) -> Result<AuthUser, AuthError> {
        let webauthn = self.webauthn()?;
        let (owner, ceremony) = self
            .take_passkey_flow(context, flow_id, "passkey_authentication")
            .await?;
        let response: PublicKeyCredential =
            serde_json::from_value(response).map_err(|_| invalid())?;
        let id = URL_SAFE_NO_PAD.encode(response.get_credential_id());
        let mut authenticated = None;
        self.store
            .transact(&mut |store| {
                let stored = store.passkeys.get_mut(&id).ok_or_else(invalid)?;
                let mut key = decode_key(&stored.data)?;
                let result = if let Some(owner) = &owner {
                    if &stored.user != owner || ceremony.versions.get(&id) != Some(&stored.version)
                    {
                        return Err(invalid());
                    }
                    let state: PasskeyAuthentication =
                        PasskeyAuthentication::deserialize(&ceremony.state).map_err(|_| internal())?;
                    webauthn
                        .finish_passkey_authentication(&response, &state)
                        .map_err(|_| invalid())?
                } else {
                    let (handle, identified_id) = webauthn
                        .identify_discoverable_authentication(&response)
                        .map_err(|_| invalid())?;
                    if handle != key.user_handle
                        || identified_id != key.credential.cred_id().as_ref()
                    {
                        return Err(invalid());
                    }
                    let state: DiscoverableAuthentication =
                        DiscoverableAuthentication::deserialize(&ceremony.state).map_err(|_| internal())?;
                    webauthn
                        .finish_discoverable_authentication(
                            &response,
                            state,
                            &[DiscoverableKey::from(&key.credential)],
                        )
                        .map_err(|_| invalid())?
                };
                if !result.user_verified() {
                    return Err(invalid());
                }
                // Discoverable validation uses the current key. For identified flows,
                // the version fence rejects stale state before any counter/backup update.
                key.credential
                    .update_credential(&result)
                    .ok_or_else(invalid)?;
                let user = store
                    .users
                    .get(&stored.user)
                    .filter(|user| user.enabled)
                    .ok_or_else(invalid)?
                    .user
                    .clone();
                let version = stored.version.checked_add(1).ok_or_else(internal)?;
                stored.data = serde_json::to_value(&key).map_err(|_| internal())?;
                stored.version = version;
                authenticated = Some(user);
                Ok(())
            })
            .await?;
        self.establish_session(context, authenticated.ok_or_else(internal)?)
            .await
    }

    /// List only public credential identifiers belonging to the session owner.
    ///
    /// # Errors
    /// Returns an error for an unauthenticated session or a storage failure.
    pub async fn passkeys(&self, context: &AuthContext) -> Result<Vec<Value>, AuthError> {
        let user = self.require_user(context).await?;
        let mut keys = Vec::new();
        self.store
            .transact(&mut |store| {
                keys = store
                    .passkeys
                    .iter()
                    .filter(|(_, key)| key.user == user.id.0)
                    .map(|(id, _)| json!({"id": id}))
                    .collect();
                Ok(())
            })
            .await?;
        keys.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
        Ok(keys)
    }

    /// Delete an owned credential without removing the last authentication method.
    ///
    /// # Errors
    /// Returns an error for missing recent authentication, an unknown or foreign
    /// credential, removal of the final authentication method, or storage failure.
    pub async fn delete_passkey(
        &self,
        context: &AuthContext,
        credential_id: &str,
    ) -> Result<(), AuthError> {
        self.webauthn()?;
        let user = self.require_recent_user(context).await?;
        self.store
            .transact(&mut |store| {
                let key = store.passkeys.get(credential_id).ok_or_else(invalid)?;
                if key.user != user.id.0 {
                    return Err(invalid());
                }
                let has_other = store.identities.iter().any(|(identity, owner)| {
                    owner == &user.id.0 && self.identity_available(identity)
                }) || store
                    .passkeys
                    .iter()
                    .any(|(id, key)| id != credential_id && key.user == user.id.0);
                if !has_other {
                    return Err(AuthError {
                        code: "last_authentication_method",
                    });
                }
                store.passkeys.remove(credential_id);
                Ok(())
            })
            .await
    }
}

#[cfg(test)]
mod deletion_tests {
    use super::*;
    use crate::auth::{
        AuthBuilder, EmailSender, MemoryAuthStore, OAuthExchange, OAuthIdentity, OAuthProvider,
        OtpEmail, PasskeyConfig, StoredUser,
    };
    use async_trait::async_trait;
    use std::sync::Arc;
    use tower_sessions::{MemoryStore, Session};

    struct UnusedSender;
    #[async_trait]
    impl EmailSender for UnusedSender {
        async fn send(&self, _email: OtpEmail) -> Result<(), AuthError> {
            Err(invalid())
        }
    }
    struct ConfiguredProvider;
    #[async_trait]
    impl OAuthProvider for ConfiguredProvider {
        fn name(&self) -> &'static str {
            "configured"
        }
        fn callback_url(&self) -> Option<&'static str> {
            Some("https://app.example.test/auth/oauth/configured/callback")
        }
        fn authorization_url(&self, _state: &str, _pkce: &str, _nonce: &str) -> String {
            String::new()
        }
        async fn exchange(&self, _request: OAuthExchange) -> Result<OAuthIdentity, AuthError> {
            Err(invalid())
        }
    }
    #[derive(Clone, Copy)]
    enum EnabledMethod {
        None,
        Email,
        OAuth,
    }

    // Seed only records read by deletion policy. Real cryptographic enrollment
    // and assertions are independently covered by the integration suite.
    async fn check_deletion(
        identity: &str,
        method: EnabledMethod,
        allowed: bool,
    ) -> Result<(), AuthError> {
        let store = Arc::new(MemoryAuthStore::new(32));
        let builder = AuthBuilder::new().store(store);
        let builder = if matches!(method, EnabledMethod::Email) {
            builder.email_otp(Arc::new(UnusedSender), vec![7; 32])
        } else {
            builder
        };
        let mut config = builder.build()?;
        config.passkey = Some(PasskeyConfig {
            rp_id: "app.example.test".into(),
            origin: "https://app.example.test".into(),
        });
        if matches!(method, EnabledMethod::OAuth) {
            config.oauth.push(Arc::new(ConfiguredProvider));
        }
        let auth = Auth::new(config, None).await?;
        let user = AuthUser {
            id: UserId("owner".into()),
        };
        auth.store
            .transact(&mut |data| {
                data.users.insert(
                    user.id.0.clone(),
                    StoredUser {
                        user: user.clone(),
                        enabled: true,
                        created_at: auth.config.clock.now(),
                    },
                );
                data.identities.insert(identity.into(), user.id.0.clone());
                data.passkeys.insert(
                    "credential".into(),
                    StoredPasskey {
                        user: user.id.0.clone(),
                        data: Value::Null,
                        version: 1,
                    },
                );
                Ok(())
            })
            .await?;
        let context = AuthContext {
            session: Session::new(None, Arc::new(MemoryStore::default()), None),
            source: [192, 0, 2, 1].into(),
            browser_binding: "deletion-regression".into(),
        };
        auth.establish_session(&context, user).await?;
        let result = auth.delete_passkey(&context, "credential").await;
        if allowed {
            result?;
        } else {
            assert!(matches!(
                result,
                Err(AuthError {
                    code: "last_authentication_method"
                })
            ));
        }
        assert_eq!(auth.passkeys(&context).await?.len(), usize::from(!allowed));
        Ok(())
    }
    #[tokio::test]
    async fn historical_email_does_not_allow_last_passkey_deletion() -> Result<(), AuthError> {
        check_deletion("email-otp:owner@example.test", EnabledMethod::None, false).await
    }
    #[tokio::test]
    async fn enabled_email_allows_passkey_deletion() -> Result<(), AuthError> {
        check_deletion("email-otp:owner@example.test", EnabledMethod::Email, true).await
    }
    #[tokio::test]
    async fn historical_oauth_does_not_allow_last_passkey_deletion() -> Result<(), AuthError> {
        check_deletion(
            r#"["oauth","configured","https://issuer.example.test","subject"]"#,
            EnabledMethod::None,
            false,
        )
        .await
    }
    #[tokio::test]
    async fn different_configured_provider_does_not_enable_historical_identity()
    -> Result<(), AuthError> {
        check_deletion(
            r#"["oauth","removed","https://issuer.example.test","subject"]"#,
            EnabledMethod::OAuth,
            false,
        )
        .await
    }
    #[tokio::test]
    async fn enabled_matching_provider_allows_passkey_deletion() -> Result<(), AuthError> {
        check_deletion(
            r#"["oauth","configured","https://issuer.example.test","subject"]"#,
            EnabledMethod::OAuth,
            true,
        )
        .await
    }
    #[tokio::test]
    async fn malformed_identity_cannot_use_configured_provider_fallback() -> Result<(), AuthError> {
        check_deletion("oauth:configured:historical", EnabledMethod::OAuth, false).await
    }
}
