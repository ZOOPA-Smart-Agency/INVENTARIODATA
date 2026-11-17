//! Talking to the inventory server over HTTPS.
//!
//! Certificates are checked against the computer's own trusted certificates (which include any company
//! certificate authority) and the public web ones. A server with its own certificate (an in-house install with
//! TLS_MODE=internal) is only accepted after a person confirms a SHA-256 fingerprint, and that choice is remembered
//! for that server address only:
//! - if the server publishes the certificate authority that signs its certificates (/ca.crt), and that authority
//!   really signed the certificate presented, the person confirms the authority's fingerprint. The server's
//!   certificates are renewed every few hours, the authority lasts years.
//! - otherwise the person confirms the fingerprint of the certificate itself.
//! The certificate is checked before anything, the password included, is sent.

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::client::WebPkiServerVerifier;
use rustls::crypto::{ring, CryptoProvider};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use std::io::Read;
use rustls::{DigitallySignedStruct, RootCertStore, SignatureScheme};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Why a request did not work. `Server` carries the inventory's own message key ("login_err", "err_dup"...).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Problem {
    /// The certificate is not trusted. `offer` is what a person can confirm to trust it.
    Untrusted { offer: Trust },
    /// The server answered with an error.
    Server { status: u16, code: String },
    /// No answer: wrong address, no network, server down.
    Network { detail: String },
    /// The address cannot be used.
    BadAddress { detail: String },
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Problem::Untrusted { offer } => write!(f, "the server's certificate is not trusted (SHA-256 {})", offer.fingerprint),
            Problem::Server { status, code } => write!(f, "the server refused: {code} (HTTP {status})"),
            Problem::Network { detail } => write!(f, "could not reach the server: {detail}"),
            Problem::BadAddress { detail } => write!(f, "{detail}"),
        }
    }
}

/// A server's own certificate a person chose to trust: its certificate authority (`ca`, DER in hex) or, when the
/// server does not publish one, the certificate itself (`ca` empty). `fingerprint` is SHA-256 of whichever it is.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Trust {
    pub fingerprint: String,
    pub ca: String,
}

impl Trust {
    pub fn matches(&self, fingerprint: &str) -> bool {
        !self.fingerprint.is_empty() && compact(&self.fingerprint) == compact(fingerprint)
    }

    /// The certificate authority, if `ca` holds one whose fingerprint is `fingerprint`.
    fn authority(&self) -> Option<CertificateDer<'static>> {
        let der = from_hex(&self.ca)?;
        (compact(&fingerprint(&der)) == compact(&self.fingerprint)).then(|| CertificateDer::from(der))
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn from_hex(text: &str) -> Option<Vec<u8>> {
    if text.is_empty() || text.len() % 2 != 0 {
        return None;
    }
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok()).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub nombre: String,
    pub rol: String,
}

/// "inventario.empresa.es" -> "https://inventario.empresa.es". Plain http is only allowed for this computer
/// (localhost), for testing.
pub fn normalize_url(input: &str) -> Result<String, Problem> {
    let invalid = || Problem::BadAddress { detail: "that address is not valid".into() };
    let input = input.trim();
    if input.is_empty() {
        return Err(Problem::BadAddress { detail: "empty address".into() });
    }
    let (scheme, rest) = match input.split_once("://") {
        Some((scheme, rest)) => (scheme.to_lowercase(), rest),
        None => ("https".to_string(), input),
    };
    let rest = rest.split(['?', '#']).next().unwrap_or("").trim_end_matches('/');
    let host = rest.split('/').next().unwrap_or("");
    let host_name = if host.starts_with('[') { host.split(']').next().unwrap_or("").trim_start_matches('[') } else { host.rsplit_once(':').map_or(host, |(h, _)| h) };
    if host_name.is_empty() || host_name.contains(char::is_whitespace) || host.contains('@') {
        return Err(invalid());
    }
    match scheme.as_str() {
        "https" => {}
        "http" if ["localhost", "127.0.0.1", "::1"].contains(&host_name) => {}
        _ => return Err(Problem::BadAddress { detail: "the address must start with https://".into() }),
    }
    Ok(format!("{scheme}://{rest}"))
}

/// "AB:CD:..." -> "abcd..." for comparing fingerprints however they were written.
fn compact(fingerprint: &str) -> String {
    fingerprint.chars().filter(|c| c.is_ascii_hexdigit()).collect::<String>().to_lowercase()
}

pub fn fingerprint(cert: &[u8]) -> String {
    Sha256::digest(cert).iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":")
}

/// Accepts a certificate the computer trusts (plus a certificate authority a person trusted for this server), or the
/// certificate whose fingerprint a person confirmed. `any` accepts everything: only used to download /ca.crt.
#[derive(Debug)]
struct Verifier {
    standard: Arc<WebPkiServerVerifier>,
    provider: Arc<CryptoProvider>,
    pinned: Option<String>,
    any: bool,
    seen: Arc<Mutex<Option<String>>>,
    refused: Arc<Mutex<bool>>,
}

impl ServerCertVerifier for Verifier {
    fn verify_server_cert(&self, end_entity: &CertificateDer<'_>, intermediates: &[CertificateDer<'_>], server_name: &ServerName<'_>,
                          ocsp: &[u8], now: UnixTime) -> Result<ServerCertVerified, rustls::Error> {
        let fp = fingerprint(end_entity.as_ref());
        *self.seen.lock().unwrap() = Some(fp.clone());
        if self.any || self.pinned.as_deref().map(compact) == Some(compact(&fp)) {
            return Ok(ServerCertVerified::assertion());
        }
        let result = self.standard.verify_server_cert(end_entity, intermediates, server_name, ocsp, now);
        if result.is_err() {
            *self.refused.lock().unwrap() = true;
        }
        result
    }

    fn verify_tls12_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}

fn roots() -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    for cert in rustls_native_certs::load_native_certs().certs {
        let _ = roots.add(cert); // the computer's own list, including company certificate authorities
    }
    roots
}

/// A connection to one inventory server.
#[derive(Clone)]
pub struct Client {
    pub base: String,
    agent: ureq::Agent,
    seen: Arc<Mutex<Option<String>>>,
    refused: Arc<Mutex<bool>>,
    token: Option<String>,
}

impl Client {
    /// `trusted`: what a person chose to trust for this server, if anything.
    pub fn new(base: &str, trusted: Option<Trust>) -> Result<Client, Problem> {
        Client::build(base, trusted, false)
    }

    fn build(base: &str, trusted: Option<Trust>, any: bool) -> Result<Client, Problem> {
        let provider = Arc::new(ring::default_provider());
        let mut roots = roots();
        let mut pinned = None;
        if let Some(trust) = trusted.filter(|t| !t.fingerprint.is_empty()) {
            match trust.authority() {
                Some(ca) => { let _ = roots.add(ca); }
                None if trust.ca.is_empty() => pinned = Some(trust.fingerprint),
                None => {} // damaged: trust nothing extra
            }
        }
        let standard = WebPkiServerVerifier::builder_with_provider(Arc::new(roots), provider.clone()).build()
            .map_err(|e| Problem::Network { detail: e.to_string() })?;
        let seen = Arc::new(Mutex::new(None));
        let refused = Arc::new(Mutex::new(false));
        let verifier = Verifier { standard, provider: provider.clone(), pinned, any, seen: seen.clone(), refused: refused.clone() };
        let config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions().map_err(|e| Problem::Network { detail: e.to_string() })?
            .dangerous().with_custom_certificate_verifier(Arc::new(verifier))
            .with_no_client_auth();
        let agent = ureq::AgentBuilder::new()
            .tls_config(Arc::new(config))
            .timeout_connect(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .user_agent(&format!("inventario-registro/{}", crate::VERSION))
            .build();
        Ok(Client { base: base.to_string(), agent, seen, refused, token: None })
    }

    fn problem(&self, error: ureq::Error) -> Problem {
        match error {
            ureq::Error::Status(status, response) => {
                let code = response.into_json::<Value>().ok().and_then(|v| v["error"].as_str().map(String::from)).unwrap_or_else(|| "err_invalid".into());
                Problem::Server { status, code }
            }
            ureq::Error::Transport(t) => {
                if *self.refused.lock().unwrap() {
                    if let Some(fingerprint) = self.seen.lock().unwrap().clone() {
                        return Problem::Untrusted { offer: Trust { fingerprint, ca: String::new() } };
                    }
                }
                Problem::Network { detail: t.to_string() }
            }
        }
    }

    fn json(&self, response: ureq::Response) -> Result<Value, Problem> {
        let is_json = response.content_type().contains("json");
        if !is_json {
            // for example the web server's maintenance page while the inventory restarts
            return Err(Problem::Server { status: response.status(), code: "maintenance".into() });
        }
        response.into_json().map_err(|e| Problem::Network { detail: e.to_string() })
    }

    fn authorized(&self, request: ureq::Request) -> ureq::Request {
        match &self.token {
            Some(token) => request.set("Authorization", &format!("Bearer {token}")),
            None => request,
        }
    }

    /// Checks the server answers and its certificate is trusted, without sending anything private. When it is not
    /// trusted, the problem carries what a person can choose to trust (see the top of this file).
    pub fn check(&self) -> Result<(), Problem> {
        match self.health() {
            Err(Problem::Untrusted { offer }) => Err(Problem::Untrusted { offer: self.authority_offer().unwrap_or(offer) }),
            other => other,
        }
    }

    fn health(&self) -> Result<(), Problem> {
        let response = self.agent.get(&format!("{}/healthz", self.base)).call().map_err(|e| self.problem(e))?;
        let body = self.json(response)?;
        if body["status"] == "ok" { Ok(()) } else { Err(Problem::Server { status: 200, code: "not_inventory".into() }) }
    }

    /// The server's published certificate authority, if it has one and it really signed the certificate presented.
    fn authority_offer(&self) -> Option<Trust> {
        let open = Client::build(&self.base, None, true).ok()?;
        let response = open.agent.get(&format!("{}/ca.crt", self.base)).call().ok()?;
        let mut bytes = Vec::new();
        response.into_reader().take(64 * 1024).read_to_end(&mut bytes).ok()?;
        let der = match CertificateDer::from_pem_slice(&bytes) {
            Ok(cert) => cert.as_ref().to_vec(),
            Err(_) => bytes,
        };
        let offer = Trust { fingerprint: fingerprint(&der), ca: to_hex(&der) };
        Client::new(&self.base, Some(offer.clone())).ok()?.health().ok()?;
        Some(offer)
    }

    pub fn login(&mut self, email: &str, password: &str) -> Result<User, Problem> {
        let response = self.agent.post(&format!("{}/api/agent/login", self.base))
            .send_json(json!({ "email": email, "password": password })).map_err(|e| self.problem(e))?;
        let body = self.json(response)?;
        self.token = body["token"].as_str().map(String::from);
        serde_json::from_value(body["user"].clone()).map_err(|e| Problem::Network { detail: e.to_string() })
    }

    /// `hostname`: this computer's name, for the server to say whether it can be its asset tag (inventory 1.10.0).
    pub fn options(&self, serial: &str, hostname: &str) -> Result<Value, Problem> {
        let mut request = self.agent.get(&format!("{}/api/agent/options", self.base)).query("serial", serial);
        if !hostname.is_empty() {
            request = request.query("hostname", hostname);
        }
        let response = self.authorized(request).call().map_err(|e| self.problem(e))?;
        self.json(response)
    }

    pub fn report(&self, body: &Value) -> Result<Value, Problem> {
        let request = self.agent.post(&format!("{}/api/agent/report", self.base));
        let response = self.authorized(request).send_json(body.clone()).map_err(|e| self.problem(e))?;
        self.json(response)
    }

    /// The page of a device in the inventory, to open in the browser.
    pub fn device_url(&self, id: &str) -> String {
        format!("{}/#dispositivo-{id}", self.base)
    }
}
