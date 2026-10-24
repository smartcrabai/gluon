//! Screenless auth server. No OTPs or private keys are printed.
use std::{env, error::Error, io, sync::Arc};

use gluon::auth::{
    AuthBuilder, JwtConfig, JwtKey, MemoryAuthStore, PasskeyConfig, SmtpConfig, SmtpEmailSender,
};
use time::Duration;

fn required(name: &str) -> Result<String, Box<dyn Error>> {
    env::var(name).map_err(|_| io::Error::other(format!("missing {name}")).into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let smtp = SmtpEmailSender::new(SmtpConfig {
        server: required("AUTH_SMTP_SERVER")?,
        port: env::var("AUTH_SMTP_PORT")
            .unwrap_or_else(|_| "587".into())
            .parse()?,
        username: required("AUTH_SMTP_USERNAME")?,
        password: required("AUTH_SMTP_PASSWORD")?,
        from: required("AUTH_SMTP_FROM")?,
    })
    .map_err(|e| io::Error::other(e.code))?;
    let mut builder =
        AuthBuilder::new().email_otp(Arc::new(smtp), required("AUTH_OTP_SECRET")?.into_bytes());
    match env::var("AUTH_STORE").as_deref() {
        Ok("memory-development") => {
            if env::var("GLUON_INSECURE_COOKIE").as_deref() != Ok("1") {
                return Err(io::Error::other(
                    "memory-development requires GLUON_INSECURE_COOKIE=1",
                )
                .into());
            }
            builder = builder.store(Arc::new(MemoryAuthStore::new(10_000)));
        }
        Ok("postgres") | Err(env::VarError::NotPresent) => {
            required("DATABASE_URL")?;
        }
        _ => {
            return Err(
                io::Error::other("AUTH_STORE must be postgres or memory-development").into(),
            );
        }
    }
    let mut config = builder.build().map_err(|e| io::Error::other(e.code))?;
    config.migrate_on_start = env::var("AUTH_MIGRATE_ON_START").as_deref() == Ok("1");
    if let Ok(origin) = env::var("AUTH_ORIGIN") {
        config.allowed_origins = vec![origin.clone()];
        if let Ok(rp_id) = env::var("AUTH_RP_ID") {
            config.passkey = Some(PasskeyConfig { rp_id, origin });
        }
    } else if env::var_os("AUTH_RP_ID").is_some() {
        return Err(io::Error::other("AUTH_RP_ID requires AUTH_ORIGIN").into());
    }
    // JWT is optional, but supplying any JWT setting requires all settings.
    if [
        "AUTH_JWT_PRIVATE_KEY_FILE",
        "AUTH_JWT_KID",
        "AUTH_JWT_ISSUER",
        "AUTH_JWT_AUDIENCE",
    ]
    .iter()
    .any(|name| env::var_os(name).is_some())
    {
        let kid = required("AUTH_JWT_KID")?;
        let mut jwt = JwtConfig::new(
            required("AUTH_JWT_ISSUER")?,
            required("AUTH_JWT_AUDIENCE")?,
            vec![JwtKey {
                kid: kid.clone(),
                private_key_pem: std::fs::read(required("AUTH_JWT_PRIVATE_KEY_FILE")?)?,
            }],
            kid,
        );
        jwt.clock_skew = Duration::seconds(30);
        config.jwt = Some(jwt);
    }
    sample_auth::run(config).await?;
    Ok(())
}
