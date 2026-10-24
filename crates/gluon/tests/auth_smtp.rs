//! Plan §5.2: the standard SMTP adapter must not downgrade to plaintext.
#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use gluon::auth::{EmailSender, OtpEmail, SmtpConfig, SmtpEmailSender};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct ServerTask(tokio::task::JoinHandle<Vec<u8>>);
impl Drop for ServerTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test]
async fn smtp_without_tls_cannot_receive_credentials_or_otp_message() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let mut server = ServerTask(tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        if socket
            .write_all(b"220 fixture.example.test ESMTP\r\n")
            .await
            .is_err()
        {
            return vec![];
        }
        let mut observed = vec![];
        let mut pending = vec![];
        let mut chunk = [0u8; 4096];
        'connection: loop {
            let count = socket.read(&mut chunk).await.unwrap_or(0);
            if count == 0 {
                break;
            }
            observed.extend_from_slice(&chunk[..count]);
            pending.extend_from_slice(&chunk[..count]);
            // A TLS ClientHello is also safe: the plaintext server cannot
            // complete that handshake. Do not constrain implicit vs STARTTLS.
            if pending.first() == Some(&0x16) {
                break;
            }
            while let Some(end) = pending.windows(2).position(|bytes| bytes == b"\r\n") {
                let line: Vec<_> = pending.drain(..end + 2).collect();
                let command = String::from_utf8_lossy(&line).to_ascii_uppercase();
                if command.starts_with("AUTH ") || command == "DATA\r\n" {
                    break 'connection;
                }
                if command.starts_with("EHLO ") || command.starts_with("HELO ") {
                    // Omit STARTTLS: a TLS-required sender must stop here.
                    if socket
                        .write_all(b"250-fixture.example.test\r\n250 AUTH PLAIN LOGIN\r\n")
                        .await
                        .is_err()
                    {
                        break 'connection;
                    }
                } else {
                    break 'connection;
                }
            }
        }
        observed
    }));
    let sender = SmtpEmailSender::new(SmtpConfig {
        server: "127.0.0.1".into(),
        port,
        username: "smtp-test-user".into(),
        password: "smtp-test-secret".into(),
        from: "auth@example.test".into(),
    })
    .unwrap();
    let sent = tokio::time::timeout(
        Duration::from_secs(3),
        sender.send(OtpEmail {
            recipient: "recipient@example.test".into(),
            challenge_id: "smtp-challenge".into(),
            code: "00012345".into(),
        }),
    )
    .await
    .expect("SMTP negotiation must finish or fail within fixture deadline");
    assert!(
        sent.is_err(),
        "plaintext SMTP must not report successful OTP delivery"
    );
    let captured = tokio::time::timeout(Duration::from_secs(3), &mut server.0)
        .await
        .expect("SMTP fixture must terminate when client rejects missing TLS")
        .unwrap();
    let captured = String::from_utf8_lossy(&captured);
    for secret in ["smtp-test-user", "smtp-test-secret", "00012345"] {
        assert!(
            !captured.contains(secret),
            "secret sent over a plaintext connection"
        );
    }
    let upper = captured.to_ascii_uppercase();
    assert!(
        !upper.contains("AUTH "),
        "SMTP credentials must not be submitted before TLS"
    );
    assert!(
        !upper.contains("DATA\r\n"),
        "OTP message must not be submitted before TLS"
    );
}
