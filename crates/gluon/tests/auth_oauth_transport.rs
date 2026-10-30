//! Plan section 5.4: drive the OAuth HTTP bridge against a real local server.
#![cfg(feature = "auth")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{Router, body::Body, response::IntoResponse, routing::get};
use gluon::auth::oauth::{OAuthHttpClient, ReqwestOAuthClient};
use http::{Request, StatusCode};
use openssl::{
    asn1::Asn1Time,
    bn::BigNum,
    ec::{EcGroup, EcKey},
    hash::MessageDigest,
    nid::Nid,
    pkey::{PKey, Private},
    ssl::{SslAcceptor, SslMethod},
    x509::{
        X509, X509NameBuilder, X509NameRef,
        extension::{
            AuthorityKeyIdentifier, BasicConstraints, ExtendedKeyUsage, KeyUsage,
            SubjectAlternativeName, SubjectKeyIdentifier,
        },
    },
};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

struct Server {
    base: String,
    task: tokio::task::JoinHandle<()>,
    redirected_requests: Arc<AtomicUsize>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Server {
    async fn start() -> Self {
        let redirected_requests = Arc::new(AtomicUsize::new(0));
        let counter = redirected_requests.clone();
        let app = Router::new()
            .route(
                "/redirect",
                get(|| async {
                    (
                        StatusCode::FOUND,
                        [("location", "/target")],
                        "redirect response",
                    )
                        .into_response()
                }),
            )
            .route(
                "/target",
                get(move || {
                    let counter = counter.clone();
                    async move {
                        counter.fetch_add(1, Ordering::SeqCst);
                        "must not be fetched"
                    }
                }),
            )
            .route("/exact", get(|| async { Body::from(vec![b'x'; 1024]) }))
            .route("/oversized", get(|| async { Body::from(vec![b'x'; 1025]) }))
            .route(
                "/never",
                get(|| async { std::future::pending::<String>().await }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            base,
            task,
            redirected_requests,
        }
    }
    fn request(&self, path: &str) -> Request<Vec<u8>> {
        Request::builder()
            .uri(format!("{}{path}", self.base))
            .body(vec![])
            .unwrap()
    }
}
fn private_key() -> PKey<Private> {
    let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
    PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap()
}

fn test_certificate(
    subject: &X509NameRef,
    issuer: Option<&X509>,
    key: &PKey<Private>,
    issuer_key: &PKey<Private>,
    serial: u32,
    is_ca: bool,
) -> X509 {
    let mut certificate = X509::builder().unwrap();
    certificate.set_version(2).unwrap();
    certificate
        .set_serial_number(&BigNum::from_u32(serial).unwrap().to_asn1_integer().unwrap())
        .unwrap();
    certificate.set_subject_name(subject).unwrap();
    certificate
        .set_issuer_name(issuer.map_or(subject, |certificate| certificate.subject_name()))
        .unwrap();
    certificate.set_pubkey(key).unwrap();
    certificate
        .set_not_before(&Asn1Time::days_from_now(0).unwrap())
        .unwrap();
    certificate
        .set_not_after(&Asn1Time::days_from_now(1).unwrap())
        .unwrap();

    let basic_constraints = if is_ca {
        BasicConstraints::new().critical().ca().build().unwrap()
    } else {
        BasicConstraints::new().critical().build().unwrap()
    };
    let key_usage = if is_ca {
        KeyUsage::new()
            .critical()
            .key_cert_sign()
            .crl_sign()
            .build()
            .unwrap()
    } else {
        KeyUsage::new()
            .critical()
            .digital_signature()
            .build()
            .unwrap()
    };
    certificate.append_extension(basic_constraints).unwrap();
    certificate.append_extension(key_usage).unwrap();
    let (subject_key_id, authority_key_id, subject_alt_name, extended_key_usage) = {
        let context = certificate.x509v3_context(issuer.map(AsRef::as_ref), None);
        let subject_key_id = SubjectKeyIdentifier::new().build(&context).unwrap();
        let authority_key_id = issuer.map(|_| {
            AuthorityKeyIdentifier::new()
                .keyid(true)
                .issuer(true)
                .build(&context)
                .unwrap()
        });
        let subject_alt_name = (!is_ca).then(|| {
            SubjectAlternativeName::new()
                .dns("localhost")
                .build(&context)
                .unwrap()
        });
        let extended_key_usage =
            (!is_ca).then(|| ExtendedKeyUsage::new().server_auth().build().unwrap());
        (
            subject_key_id,
            authority_key_id,
            subject_alt_name,
            extended_key_usage,
        )
    };
    certificate.append_extension(subject_key_id).unwrap();
    if let Some(authority_key_id) = authority_key_id {
        certificate.append_extension(authority_key_id).unwrap();
    }
    if let Some(subject_alt_name) = subject_alt_name {
        certificate.append_extension(subject_alt_name).unwrap();
    }
    if let Some(extended_key_usage) = extended_key_usage {
        certificate.append_extension(extended_key_usage).unwrap();
    }
    certificate
        .sign(issuer_key, MessageDigest::sha256())
        .unwrap();
    certificate.build()
}

struct TlsServer {
    addr: SocketAddr,
    ca_pem: Vec<u8>,
    stop: Arc<AtomicBool>,
    task: Option<JoinHandle<()>>,
}
impl TlsServer {
    fn start() -> Self {
        let ca_key = private_key();
        let mut ca_name = X509NameBuilder::new().unwrap();
        ca_name
            .append_entry_by_nid(Nid::COMMONNAME, "OAuth test CA")
            .unwrap();
        let ca_name = ca_name.build();
        let ca = test_certificate(&ca_name, None, &ca_key, &ca_key, 1, true);

        let server_key = private_key();
        let mut server_name = X509NameBuilder::new().unwrap();
        server_name
            .append_entry_by_nid(Nid::COMMONNAME, "localhost")
            .unwrap();
        let server_name = server_name.build();
        let server_certificate =
            test_certificate(&server_name, Some(&ca), &server_key, &ca_key, 2, false);
        let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls_server()).unwrap();
        acceptor.set_certificate(&server_certificate).unwrap();
        acceptor.set_private_key(&server_key).unwrap();
        acceptor.check_private_key().unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let acceptor = acceptor.build();
        let stop = Arc::new(AtomicBool::new(false));
        let server_stop = stop.clone();
        let task = std::thread::spawn(move || {
            while let Ok((socket, _)) = listener.accept() {
                if server_stop.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(mut stream) = acceptor.accept(socket) {
                    let mut request = [0; 4096];
                    if stream.read(&mut request).is_ok() {
                        let _ = stream.write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                        );
                    }
                }
            }
        });
        Self {
            addr,
            ca_pem: ca.to_pem().unwrap(),
            stop,
            task: Some(task),
        }
    }

    fn request(&self) -> Request<Vec<u8>> {
        Request::builder()
            .uri(format!("https://localhost:{}/", self.addr.port()))
            .body(vec![])
            .unwrap()
    }
}
impl Drop for TlsServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr);
        if let Some(task) = self.task.take() {
            let _ = task.join();
        }
    }
}

#[tokio::test]
async fn oauth_http_extra_root_certificate_is_opt_in() {
    let server = TlsServer::start();
    let timeout = Duration::from_secs(2);
    let unconfigured = ReqwestOAuthClient::new(timeout, 1024);
    assert!(unconfigured.execute(server.request()).await.is_err());

    let trusted =
        ReqwestOAuthClient::new_with_root_certificates(timeout, 1024, &server.ca_pem).unwrap();
    let response = trusted.execute(server.request()).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.body(), b"ok");
}

#[tokio::test]
async fn oauth_http_redirect_is_not_followed() {
    let server = Server::start().await;
    let client = ReqwestOAuthClient::new(Duration::from_secs(2), 1024);
    let result = client.execute(server.request("/redirect")).await;
    // The adapter may return the redirect for the protocol layer to reject, or
    // reject it itself. Following it and returning a successful 200 is forbidden.
    if let Ok(response) = result {
        assert!(response.status().is_redirection());
    }
    assert_eq!(server.redirected_requests.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn oauth_http_response_limit_allows_exact_boundary_but_rejects_one_byte_more() {
    let server = Server::start().await;
    let client = ReqwestOAuthClient::new(Duration::from_secs(2), 1024);
    let exact = client.execute(server.request("/exact")).await.unwrap();
    assert_eq!(exact.status(), StatusCode::OK);
    assert_eq!(exact.body().len(), 1024);
    assert!(client.execute(server.request("/oversized")).await.is_err());
}

#[tokio::test]
async fn oauth_http_timeout_bounds_a_nonresponding_provider() {
    let server = Server::start().await;
    let client = ReqwestOAuthClient::new(Duration::from_millis(50), 1024);
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        client.execute(server.request("/never")),
    )
    .await
    .expect("OAuth HTTP client must enforce its configured timeout");
    assert!(result.is_err());
}
