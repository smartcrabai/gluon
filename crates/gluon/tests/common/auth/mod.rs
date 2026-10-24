#![allow(dead_code)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use async_trait::async_trait;
use gluon::auth::{
    Auth, AuthConfig, AuthContext, AuthError, AuthUser, Clock, EmailOtpConfig, EmailSender,
    MemoryAuthStore, OtpEmail,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use time::{Duration, OffsetDateTime};
use tower_sessions::{MemoryStore, Session};

pub struct TestClock(Mutex<OffsetDateTime>);
impl TestClock {
    pub fn new() -> Self {
        Self(Mutex::new(
            OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap(),
        ))
    }
    pub fn advance(&self, duration: Duration) {
        *self.0.lock().unwrap() += duration;
    }
}
impl Clock for TestClock {
    fn now(&self) -> OffsetDateTime {
        *self.0.lock().unwrap()
    }
}

#[derive(Default)]
pub struct Mailbox {
    pub messages: Mutex<Vec<OtpEmail>>,
    pub fail: AtomicBool,
}
impl Mailbox {
    pub fn last(&self) -> OtpEmail {
        self.messages
            .lock()
            .unwrap()
            .last()
            .expect("OTP delivered to test sender")
            .clone()
    }
}
#[async_trait]
impl EmailSender for Mailbox {
    async fn send(&self, email: OtpEmail) -> Result<(), AuthError> {
        self.messages.lock().unwrap().push(email);
        if self.fail.load(Ordering::SeqCst) {
            Err(AuthError {
                code: "delivery_failed",
            })
        } else {
            Ok(())
        }
    }
}

pub fn config(clock: Arc<TestClock>, sender: Arc<Mailbox>) -> AuthConfig {
    AuthConfig {
        store: Some(Arc::new(MemoryAuthStore::new(1024))),
        clock,
        email_otp: Some(EmailOtpConfig {
            sender,
            secret: vec![0x71; 32],
            ttl: Duration::minutes(5),
            max_attempts: 5,
            resend_interval: Duration::seconds(60),
            source_limit: 100,
            destination_limit: 10,
            rate_window: Duration::minutes(10),
        }),
        passkey: None,
        oauth: vec![],
        jwt: None,
        mount_path: "/auth".into(),
        http_enabled: true,
        migrate_on_start: false,
        flow_ttl: Duration::minutes(5),
        idle_timeout: Duration::minutes(30),
        absolute_timeout: Duration::hours(8),
        allowed_origins: vec!["https://app.example.test".into()],
    }
}

pub fn context() -> AuthContext {
    AuthContext {
        session: Session::new(None, Arc::new(MemoryStore::default()), None),
        source: "192.0.2.10".parse().unwrap(),
        browser_binding: "test-browser-a".into(),
    }
}

pub struct Fixture {
    pub auth: Arc<Auth>,
    pub config: AuthConfig,
    pub clock: Arc<TestClock>,
    pub mail: Arc<Mailbox>,
}
impl Fixture {
    pub async fn new() -> Self {
        let clock = Arc::new(TestClock::new());
        let mail = Arc::new(Mailbox::default());
        Self::with_config(config(clock.clone(), mail.clone()), clock, mail).await
    }
    pub async fn with_config(
        config: AuthConfig,
        clock: Arc<TestClock>,
        mail: Arc<Mailbox>,
    ) -> Self {
        let auth = Arc::new(Auth::new(config.clone(), None).await.unwrap());
        Self {
            auth,
            config,
            clock,
            mail,
        }
    }
    pub async fn login(&self, context: &AuthContext, email: &str) -> AuthUser {
        let challenge = self.auth.request_email_otp(context, email).await.unwrap();
        self.auth
            .verify_email_otp(context, &challenge.challenge_id, &self.mail.last().code)
            .await
            .unwrap()
    }
}

pub fn assert_code<T: std::fmt::Debug>(result: Result<T, AuthError>, code: &str) {
    assert_eq!(result.unwrap_err().code, code);
}
