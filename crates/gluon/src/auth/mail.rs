//! Standard asynchronous SMTP delivery with mandatory, certificate-validated TLS.
use super::{AuthError, EmailSender, OtpEmail};
use async_trait::async_trait;
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, message::Mailbox,
    transport::smtp::authentication::Credentials,
};
use std::time::Duration;

/// SMTP submission settings. Port 465 uses implicit TLS, all others require STARTTLS.
pub struct SmtpConfig {
    pub server: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from: String,
}

/// A reusable Tokio SMTP transport. Never falls back to plaintext delivery.
pub struct SmtpEmailSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl SmtpEmailSender {
    /// Construct a certificate-validating, TLS-required SMTP submission transport.
    ///
    /// # Errors
    /// Returns an error for invalid server, credentials, sender address, or TLS settings.
    pub fn new(config: SmtpConfig) -> Result<Self, AuthError> {
        let invalid = || AuthError {
            code: "configuration",
        };
        if config.server.trim().is_empty()
            || config.port == 0
            || config.username.is_empty()
            || config.password.is_empty()
        {
            return Err(invalid());
        }
        let from = config.from.parse().map_err(|_| invalid())?;
        let builder = if config.port == 465 {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.server)
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.server)
        }
        .map_err(|_| invalid())?;
        let transport = builder
            .port(config.port)
            .credentials(Credentials::new(config.username, config.password))
            .timeout(Some(Duration::from_secs(10)))
            .build();
        Ok(Self { transport, from })
    }
}

#[async_trait]
impl EmailSender for SmtpEmailSender {
    async fn send(&self, email: OtpEmail) -> Result<(), AuthError> {
        let failure = || AuthError {
            code: "delivery_failed",
        };
        let recipient = email.recipient.parse().map_err(|_| failure())?;
        let message = Message::builder().from(self.from.clone()).to(recipient)
            .subject("Your sign-in code")
            .body(format!("Your sign-in code is: {}\n\nIf you did not request this code, ignore this email.\n", email.code))
            .map_err(|_| failure())?;
        self.transport.send(message).await.map_err(|_| failure())?;
        Ok(())
    }
}
