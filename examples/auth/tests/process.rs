//! Real subprocess using the exact application assembly used by main, with test-only mail delivery.
use async_trait::async_trait;
use gluon::auth::{
    AuthBuilder, AuthError, EmailSender, JwtConfig, JwtKey, MemoryAuthStore, OtpEmail,
};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

struct Mailbox(PathBuf);
#[async_trait]
impl EmailSender for Mailbox {
    async fn send(&self, email: OtpEmail) -> Result<(), AuthError> {
        std::fs::write(
            &self.0,
            json!({"challenge_id": email.challenge_id, "code": email.code}).to_string(),
        )
        .map_err(|_| AuthError {
            code: "delivery_failed",
        })
    }
}

// This entrypoint exists only in the integration test executable, never in sample-auth.
#[test]
fn fixture_process() {
    let Ok(mailbox) = std::env::var("GLUON_AUTH_TEST_MAILBOX") else {
        return;
    };
    let mut config = AuthBuilder::new()
        .store(Arc::new(MemoryAuthStore::new(1024)))
        .email_otp(Arc::new(Mailbox(mailbox.into())), vec![0x71; 32])
        .build()
        .expect("fixture configuration");
    config.jwt = Some(JwtConfig {
        issuer: "https://auth.example.test".into(), audience: "example-api".into(),
        keys: vec![JwtKey { kid: "fixture".into(), private_key_pem: b"-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK2VwBCIEIEmyZSI2Yqo54etzjA/kMeF9rVI5ixzZ1YKPQEFh9XKt\n-----END PRIVATE KEY-----\n".to_vec() }],
        active_kid: "fixture".into(), ttl: time::Duration::minutes(5), clock_skew: time::Duration::seconds(30),
    });
    tokio::runtime::Runtime::new()
        .expect("runtime")
        .block_on(sample_auth::run(config))
        .expect("fixture server");
}

struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn csrf(client: &reqwest::Client, base: &str) -> String {
    let response = client
        .get(format!("{base}/auth/csrf"))
        .send()
        .await
        .expect("CSRF request");
    assert_eq!(response.status(), 200);
    response.json::<Value>().await.expect("CSRF JSON")["token"]
        .as_str()
        .expect("CSRF token")
        .into()
}

#[tokio::test]
async fn otp_session_jwt_logout_through_shared_example_process() {
    let dir = tempfile::tempdir().expect("private mailbox directory");
    let mailbox = dir.path().join("mail.json");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("reserve port");
    let bind = listener.local_addr().expect("port");
    drop(listener);
    let base = format!("http://{bind}");
    let child = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", "fixture_process", "--nocapture"])
        .env("GLUON_AUTH_TEST_MAILBOX", &mailbox)
        .env_remove("DATABASE_URL")
        .env("GLUON_BIND", bind.to_string())
        .env("GLUON_INSECURE_COOKIE", "1")
        .env("GLUON_TELEMETRY_DISABLED", "1")
        .env(
            "SECRET_KEY_BASE",
            "test-only-cookie-key-0000000000000000000000000000000000000000000000",
        )
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn fixture");
    let mut process = Process(child);
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client");
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        assert!(
            process.0.try_wait().expect("child status").is_none(),
            "fixture exited before readiness"
        );
        if client
            .get(format!("{base}/health"))
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
        {
            break;
        }
        assert!(Instant::now() < deadline, "fixture readiness timeout");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let old_csrf = csrf(&client, &base).await;
    let request = client
        .post(format!("{base}/auth/email-otp/request"))
        .header("x-csrf-token", &old_csrf)
        .json(&json!({"email": "process@example.test"}))
        .send()
        .await
        .expect("OTP request");
    assert!(request.status().is_success());
    let challenge = request.json::<Value>().await.expect("challenge");
    assert!(challenge.get("code").is_none());
    let delivered: Value =
        serde_json::from_slice(&std::fs::read(&mailbox).expect("delivered mailbox"))
            .expect("mail JSON");
    assert_eq!(challenge["challenge_id"], delivered["challenge_id"]);
    let verify = client
        .post(format!("{base}/auth/email-otp/verify"))
        .header("x-csrf-token", &old_csrf)
        .json(&delivered)
        .send()
        .await
        .expect("OTP verification");
    assert_eq!(verify.status(), 200);
    let user = verify.json::<Value>().await.expect("verified user")["user"].clone();
    let session = client
        .get(format!("{base}/auth/session"))
        .send()
        .await
        .expect("session");
    assert_eq!(session.status(), 200);
    assert_eq!(
        session.json::<Value>().await.expect("session JSON")["user"],
        user
    );
    let stale = client
        .post(format!("{base}/auth/token"))
        .header("x-csrf-token", &old_csrf)
        .send()
        .await
        .expect("stale CSRF");
    assert_eq!(stale.status(), 403);
    let fresh_csrf = csrf(&client, &base).await;
    assert_ne!(fresh_csrf, old_csrf);
    let issued = client
        .post(format!("{base}/auth/token"))
        .header("x-csrf-token", &fresh_csrf)
        .send()
        .await
        .expect("JWT issuance");
    assert_eq!(issued.status(), 200);
    let issued = issued.json::<Value>().await.expect("token JSON");
    let token = issued["token"].as_str().expect("JWT");
    assert_eq!(token.split('.').count(), 3);
    let jwks = client
        .get(format!("{base}/auth/jwks"))
        .send()
        .await
        .expect("JWKS");
    assert_eq!(jwks.status(), 200);
    assert!(!jwks.headers().contains_key("set-cookie"));
    let jwks = jwks
        .json::<jsonwebtoken::jwk::JwkSet>()
        .await
        .expect("public JWKS JSON");
    let key =
        jsonwebtoken::DecodingKey::from_jwk(jwks.find("fixture").expect("fixture public key"))
            .expect("public key");
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::EdDSA);
    validation.set_issuer(&["https://auth.example.test"]);
    validation.set_audience(&["example-api"]);
    let claims = jsonwebtoken::decode::<Value>(token, &key, &validation)
        .expect("valid issued JWT")
        .claims;
    assert_eq!(claims["sub"], user["id"]);
    assert_eq!(
        claims["exp"].as_i64().expect("exp") - claims["iat"].as_i64().expect("iat"),
        300
    );
    let logout = client
        .post(format!("{base}/auth/logout"))
        .header("x-csrf-token", &fresh_csrf)
        .send()
        .await
        .expect("logout");
    assert_eq!(logout.status(), 200);
    let anonymous_csrf = csrf(&client, &base).await;
    let denied = client
        .post(format!("{base}/auth/token"))
        .header("x-csrf-token", anonymous_csrf)
        .send()
        .await
        .expect("post-logout token");
    assert_eq!(denied.status(), 401);
    assert!(
        jsonwebtoken::decode::<Value>(token, &key, &validation).is_ok(),
        "logout must not imply offline JWT revocation"
    );
}
