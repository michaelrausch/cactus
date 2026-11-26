//! `- tool: cert`: connect to a host over TLS and check its certificate:
//! dates, name, trust, key and signature, and whether it speaks TLS 1.3.
//!
//! It makes its own handshake rather than sending a request, and accepts
//! any certificate so it can report on one that a browser would refuse.
//! TLS 1.0 and 1.1 can't be tried: rustls doesn't speak them.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{CertificateError, ClientConfig, DigitallySignedStruct, ProtocolVersion, RootCertStore, SignatureScheme};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::time::{Instant, timeout};
use tokio_rustls::TlsConnector;
use x509_parser::extensions::GeneralName;
use x509_parser::prelude::{FromDer, X509Certificate};
use x509_parser::public_key::PublicKey;

use super::findings::{Exports, export};
use super::{FailOn, Finding, Metric, Severity, Tool, ToolCx, ToolOutput, findings_verdict, is_zero, metrics_verdict};
use crate::collection::LenientString;
use crate::scenario::Assertion;
use crate::{Error, Result};

/// Days before expiry that count as expiring, when not set.
const DEFAULT_WARN_DAYS: u32 = 30;
/// Expiring this soon is high rather than medium.
const URGENT_DAYS: i64 = 7;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const SECONDS_PER_DAY: i64 = 86_400;
/// Signature algorithms that can be forged: MD2, MD5 and SHA-1 with RSA,
/// DSA and ECDSA.
const WEAK_SIGNATURES: &[(&str, &str)] = &[
    ("1.2.840.113549.1.1.2", "MD2 with RSA"),
    ("1.2.840.113549.1.1.4", "MD5 with RSA"),
    ("1.2.840.113549.1.1.5", "SHA-1 with RSA"),
    ("1.2.840.10040.4.3", "SHA-1 with DSA"),
    ("1.2.840.10045.4.1", "SHA-1 with ECDSA"),
];
const MIN_RSA_BITS: usize = 2048;
const MIN_EC_BITS: usize = 256;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Cert {
    /// `https://host[:port]`, or just `host[:port]`.
    #[serde(skip_serializing_if = "LenientString::is_empty")]
    pub url: LenientString,
    /// Report a certificate that expires within this many days; 30 when not set.
    #[serde(skip_serializing_if = "is_zero")]
    pub warn_days: u32,
    /// Limits on the metrics: `days_left gt 14`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checks: Vec<Assertion>,
    #[serde(skip_serializing_if = "FailOn::is_default")]
    pub fail_on: FailOn,
    /// Metrics or findings for later steps: `days: days_left`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub set: BTreeMap<String, LenientString>,
}

/// What `set` can name.
const EXPORTS: Exports = Exports { metrics: &["days_left", "handshake_ms"], findings: true };

/// The host to connect to.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Target {
    host: Host,
    port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Host {
    Name(String),
    Ip(IpAddr),
}

impl Target {
    fn parse(text: &str) -> Result<Self> {
        let text = text.trim();
        if text.is_empty() {
            return Err(Error::MissingUrl);
        }
        let full = if text.contains("://") { text.to_string() } else { format!("https://{text}") };
        let invalid = |reason: String| Error::InvalidUrl { url: text.into(), reason };
        let url = url::Url::parse(&full).map_err(|e| invalid(e.to_string()))?;
        if !matches!(url.scheme(), "https" | "wss") {
            return Err(invalid("not https".into()));
        }
        let host = match url.host() {
            Some(url::Host::Domain(name)) => Host::Name(name.to_ascii_lowercase()),
            Some(url::Host::Ipv4(ip)) => Host::Ip(ip.into()),
            Some(url::Host::Ipv6(ip)) => Host::Ip(ip.into()),
            None => return Err(invalid("no host".into())),
        };
        Ok(Self { host, port: url.port_or_known_default().unwrap_or(443) })
    }

    fn server_name(&self) -> Result<ServerName<'static>> {
        match &self.host {
            Host::Name(name) => ServerName::try_from(name.clone())
                .map_err(|e| Error::InvalidUrl { url: name.clone(), reason: e.to_string() }),
            Host::Ip(ip) => Ok(ServerName::IpAddress((*ip).into())),
        }
    }

    /// `host:port`, as findings name it.
    fn location(&self) -> String {
        match &self.host {
            Host::Name(name) => format!("{name}:{}", self.port),
            Host::Ip(IpAddr::V6(ip)) => format!("[{ip}]:{}", self.port),
            Host::Ip(ip) => format!("{ip}:{}", self.port),
        }
    }
}

impl Tool for Cert {
    fn title(&self) -> String {
        format!("Certificate {}", self.url.trim())
    }

    fn templates(&mut self) -> Vec<&mut String> {
        let mut texts = vec![&mut self.url.0];
        texts.extend(self.checks.iter_mut().filter_map(|check| check.value.as_mut().map(|value| &mut value.0)));
        texts
    }

    async fn run(&self, cx: &mut ToolCx<'_>) -> Result<ToolOutput> {
        EXPORTS.check(&self.set)?;
        let target = Target::parse(&self.url)?;
        let handshake = handshake(&target).await?;
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
        let now = i64::try_from(now).unwrap_or(i64::MAX);
        let warn_days = if self.warn_days == 0 { DEFAULT_WARN_DAYS } else { self.warn_days };
        let mut seen = inspect(&handshake.leaf, &target, now, warn_days)?;
        if let Some(why) = handshake.untrusted {
            seen.findings.push(finding(Severity::High, "Untrusted certificate", "cert.untrusted", &target, why));
        }
        if handshake.protocol != Some(ProtocolVersion::TLSv1_3) {
            let detail = format!("Negotiated {}", protocol_name(handshake.protocol));
            seen.findings.push(finding(Severity::Info, "No TLS 1.3", "cert.no-tls13", &target, detail));
        }
        #[allow(clippy::cast_precision_loss)] // Milliseconds and days are far below 2^53.
        let metrics = vec![
            Metric::new("days_left", "Days left", seen.days_left as f64, "d"),
            Metric::new("handshake_ms", "Handshake", handshake.millis as f64, "ms"),
        ];
        let text = format!("{}\n{} · {}", seen.text, protocol_name(handshake.protocol), handshake.cipher);
        let output = ToolOutput { findings: Some(seen.findings), metrics, ..ToolOutput::text(text) };
        export(cx, &self.set, &output)?;
        Ok(output)
    }

    fn verdict(&self, output: &ToolOutput) -> Option<String> {
        findings_verdict(self.fail_on, output.findings.as_deref())
            .or_else(|| metrics_verdict(&self.checks, &output.metrics))
    }
}

/// What the handshake showed.
struct Handshake {
    leaf: CertificateDer<'static>,
    /// Why the chain isn't trusted, apart from dates and name, which
    /// [`inspect`] reports itself.
    untrusted: Option<String>,
    protocol: Option<ProtocolVersion>,
    cipher: String,
    millis: u128,
}

async fn handshake(target: &Target) -> Result<Handshake> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let roots = Arc::new(RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() });
    let webpki = WebPkiServerVerifier::builder_with_provider(roots, Arc::clone(&provider))
        .build()
        .map_err(|e| Error::Tool(format!("TLS: {e}")))?;
    let recorder = Arc::new(Recorder { webpki, outcome: Mutex::new(None) });
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| Error::Tool(format!("TLS: {e}")))?
        .dangerous()
        .with_custom_certificate_verifier(Arc::clone(&recorder) as Arc<dyn ServerCertVerifier>)
        .with_no_client_auth();
    let address = match &target.host {
        Host::Name(name) => format!("{name}:{}", target.port),
        Host::Ip(ip) => std::net::SocketAddr::new(*ip, target.port).to_string(),
    };
    let started = Instant::now();
    let connect = async {
        let tcp = TcpStream::connect(&address).await?;
        TlsConnector::from(Arc::new(config)).connect(target.server_name()?, tcp).await.map_err(Error::from)
    };
    let stream = timeout(CONNECT_TIMEOUT, connect)
        .await
        .map_err(|_| {
            Error::Tool(format!("No TLS answer from {} in {}s", target.location(), CONNECT_TIMEOUT.as_secs()))
        })?
        .map_err(|e| Error::Tool(format!("TLS handshake with {} failed: {e}", target.location())))?;
    let millis = started.elapsed().as_millis();
    let connection = stream.get_ref().1;
    let leaf = connection
        .peer_certificates()
        .and_then(|chain| chain.first())
        .ok_or_else(|| Error::Tool("The server sent no certificate".into()))?
        .clone()
        .into_owned();
    let untrusted = recorder.outcome.lock().unwrap().take().and_then(untrusted_reason);
    Ok(Handshake {
        leaf,
        untrusted,
        protocol: connection.protocol_version(),
        cipher: connection.negotiated_cipher_suite().map_or_default(|suite| format!("{:?}", suite.suite())),
        millis,
    })
}

/// Why webpki refused the chain, unless it's for a reason [`inspect`] reports.
fn untrusted_reason(error: rustls::Error) -> Option<String> {
    match error {
        rustls::Error::InvalidCertificate(
            CertificateError::Expired
            | CertificateError::ExpiredContext { .. }
            | CertificateError::NotValidYet
            | CertificateError::NotValidYetContext { .. }
            | CertificateError::NotValidForName
            | CertificateError::NotValidForNameContext { .. },
        ) => None,
        rustls::Error::InvalidCertificate(CertificateError::UnknownIssuer) => {
            Some("Not issued by a trusted authority, or the chain is incomplete".into())
        }
        other => Some(other.to_string()),
    }
}

fn is_date_error(error: Option<&rustls::Error>) -> bool {
    matches!(
        error,
        Some(rustls::Error::InvalidCertificate(
            CertificateError::Expired
                | CertificateError::ExpiredContext { .. }
                | CertificateError::NotValidYet
                | CertificateError::NotValidYetContext { .. }
        ))
    )
}

/// A moment inside the certificate's validity: its midpoint.
fn time_within_validity(der: &CertificateDer<'_>) -> Option<UnixTime> {
    let (_, cert) = X509Certificate::from_der(der).ok()?;
    let validity = cert.validity();
    let middle = validity.not_before.timestamp().midpoint(validity.not_after.timestamp());
    Some(UnixTime::since_unix_epoch(Duration::from_secs(u64::try_from(middle).ok()?)))
}

fn protocol_name(protocol: Option<ProtocolVersion>) -> String {
    match protocol {
        Some(ProtocolVersion::TLSv1_3) => "TLS 1.3".into(),
        Some(ProtocolVersion::TLSv1_2) => "TLS 1.2".into(),
        Some(other) => format!("{other:?}"),
        None => "unknown".into(),
    }
}

/// Checks the chain as usual, but keeps the outcome instead of failing the
/// handshake, so a bad certificate can still be looked at.
#[derive(Debug)]
struct Recorder {
    webpki: Arc<WebPkiServerVerifier>,
    outcome: Mutex<Option<rustls::Error>>,
}

impl ServerCertVerifier for Recorder {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        let verify = |now| self.webpki.verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now);
        let mut outcome = verify(now).err();
        // webpki checks dates before trust, so an out-of-date certificate
        // hides whether it's trusted: ask again as of a time it was valid.
        if is_date_error(outcome.as_ref())
            && let Some(valid_at) = time_within_validity(end_entity)
        {
            outcome = verify(valid_at).err();
        }
        *self.outcome.lock().unwrap() = outcome;
        Ok(ServerCertVerified::assertion())
    }

    // Handshake signatures are accepted even when they can't be checked:
    // ring refuses weak keys here, and [`inspect`] reports them instead.
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(self
            .webpki
            .verify_tls12_signature(message, cert, dss)
            .unwrap_or_else(|_| HandshakeSignatureValid::assertion()))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        Ok(self
            .webpki
            .verify_tls13_signature(message, cert, dss)
            .unwrap_or_else(|_| HandshakeSignatureValid::assertion()))
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.webpki.supported_verify_schemes()
    }
}

/// What the certificate itself says.
#[derive(Debug)]
struct Inspected {
    findings: Vec<Finding>,
    days_left: i64,
    /// Subject, issuer and dates.
    text: String,
}

/// Check the leaf certificate `der` for `target` at Unix time `now`.
fn inspect(der: &[u8], target: &Target, now: i64, warn_days: u32) -> Result<Inspected> {
    let (_, cert) =
        X509Certificate::from_der(der).map_err(|e| Error::Tool(format!("Couldn't read the certificate: {e}")))?;
    let mut findings = Vec::new();
    let mut add = |severity, title: &str, rule: &str, detail: String| {
        findings.push(finding(severity, title, rule, target, detail));
    };
    let validity = cert.validity();
    let (not_before, not_after) = (validity.not_before.timestamp(), validity.not_after.timestamp());
    let days_left = (not_after - now).div_euclid(SECONDS_PER_DAY);
    if now > not_after {
        add(Severity::Critical, "Certificate expired", "cert.expired", format!("Expired {}", validity.not_after));
    } else if now < not_before {
        add(Severity::Critical, "Not valid yet", "cert.not-yet-valid", format!("Valid from {}", validity.not_before));
    } else if days_left < i64::from(warn_days) {
        let severity = if days_left < URGENT_DAYS { Severity::High } else { Severity::Medium };
        let detail = format!("Expires in {days_left} days, {}", validity.not_after);
        add(severity, "Certificate expiring", "cert.expiring", detail);
    }

    let names = alt_names(&cert);
    if !names.iter().any(|name| name.matches(&target.host)) {
        let listed: Vec<String> = names.iter().map(ToString::to_string).collect();
        let listed = if listed.is_empty() { "no names".into() } else { listed.join(", ") };
        add(
            Severity::High,
            "Name doesn't match",
            "cert.hostname",
            format!("Not for {}; for {listed}", target.location()),
        );
    }

    match cert.public_key().parsed() {
        Ok(PublicKey::RSA(rsa)) if rsa_bits(rsa.modulus) < MIN_RSA_BITS => {
            add(Severity::High, "Weak key", "cert.weak-key", format!("{}-bit RSA", rsa_bits(rsa.modulus)));
        }
        Ok(PublicKey::EC(ec)) if ec.key_size() < MIN_EC_BITS => {
            add(Severity::High, "Weak key", "cert.weak-key", format!("{}-bit EC", ec.key_size()));
        }
        _ => {}
    }

    let algorithm = cert.signature_algorithm.algorithm.to_id_string();
    if let Some((_, name)) = WEAK_SIGNATURES.iter().find(|(oid, _)| *oid == algorithm) {
        add(Severity::High, "Weak signature", "cert.weak-signature", format!("Signed with {name}"));
    }

    let text = format!(
        "{}\nIssued by {}\nValid {} to {}",
        cert.subject(),
        cert.issuer(),
        validity.not_before,
        validity.not_after
    );
    Ok(Inspected { findings, days_left, text })
}

fn finding(severity: Severity, title: &str, rule: &str, target: &Target, detail: String) -> Finding {
    Finding {
        severity,
        title: title.into(),
        rule: rule.into(),
        location: target.location(),
        detail,
        ..Finding::default()
    }
}

/// The bits of an RSA modulus, without its leading zeros.
fn rsa_bits(modulus: &[u8]) -> usize {
    let modulus = modulus.iter().position(|&byte| byte != 0).map_or(&[][..], |start| &modulus[start..]);
    modulus.first().map_or(0, |&first| modulus.len() * 8 - first.leading_zeros() as usize)
}

/// A name the certificate is for.
#[derive(Debug)]
enum AltName {
    Dns(String),
    Ip(IpAddr),
}

impl AltName {
    fn matches(&self, host: &Host) -> bool {
        match (self, host) {
            (Self::Dns(pattern), Host::Name(name)) => dns_matches(pattern, name),
            (Self::Ip(ip), Host::Ip(host)) => ip == host,
            _ => false,
        }
    }
}

impl std::fmt::Display for AltName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dns(name) => f.write_str(name),
            Self::Ip(ip) => write!(f, "{ip}"),
        }
    }
}

fn alt_names(cert: &X509Certificate<'_>) -> Vec<AltName> {
    let Ok(Some(names)) = cert.subject_alternative_name() else { return Vec::new() };
    names
        .value
        .general_names
        .iter()
        .filter_map(|name| match name {
            GeneralName::DNSName(dns) => Some(AltName::Dns(dns.to_ascii_lowercase())),
            GeneralName::IPAddress(bytes) => match bytes.len() {
                4 => <[u8; 4]>::try_from(*bytes).ok().map(|ip| AltName::Ip(ip.into())),
                16 => <[u8; 16]>::try_from(*bytes).ok().map(|ip| AltName::Ip(ip.into())),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// Whether `pattern` (`api.example.com`, `*.example.com`) covers `name`. A
/// wildcard covers exactly one label.
fn dns_matches(pattern: &str, name: &str) -> bool {
    let name = name.trim_end_matches('.');
    match pattern.strip_prefix("*.") {
        Some(parent) => name.split_once('.').is_some_and(|(label, rest)| !label.is_empty() && rest == parent),
        None => pattern == name,
    }
}

#[cfg(test)]
mod tests {
    use rcgen::{CertificateParams, KeyPair, date_time_ymd};

    use super::*;

    /// 2026-06-01, a fixed "now" for the dates below.
    const NOW: i64 = 1_780_272_000;

    fn certificate(names: &[&str], until: (i32, u8, u8)) -> Vec<u8> {
        let mut params = CertificateParams::new(names.iter().map(ToString::to_string).collect::<Vec<_>>()).unwrap();
        params.not_before = date_time_ymd(2026, 1, 1);
        params.not_after = date_time_ymd(until.0, until.1, until.2);
        params.self_signed(&KeyPair::generate().unwrap()).unwrap().der().to_vec()
    }

    fn rules(inspected: &Inspected) -> Vec<&str> {
        inspected.findings.iter().map(|finding| finding.rule.as_str()).collect()
    }

    fn target(text: &str) -> Target {
        Target::parse(text).unwrap()
    }

    #[test]
    fn a_current_certificate_for_the_host_is_clean() {
        let der = certificate(&["api.example.com"], (2027, 1, 1));
        let inspected = inspect(&der, &target("api.example.com"), NOW, 30).unwrap();
        assert_eq!(rules(&inspected), Vec::<&str>::new());
        assert_eq!(inspected.days_left, 214);
    }

    #[test]
    fn finds_expired_expiring_and_misnamed_certificates() {
        let expired = certificate(&["api.example.com"], (2026, 5, 1));
        let inspected = inspect(&expired, &target("https://api.example.com/x"), NOW, 30).unwrap();
        assert_eq!(rules(&inspected), ["cert.expired"]);
        assert_eq!(inspected.days_left, -31);

        let soon = certificate(&["*.example.com"], (2026, 6, 20));
        let inspected = inspect(&soon, &target("api.example.com:8443"), NOW, 30).unwrap();
        assert_eq!(rules(&inspected), ["cert.expiring"]);
        assert_eq!(inspected.findings[0].severity, Severity::Medium);
        assert_eq!(inspected.findings[0].location, "api.example.com:8443");
        let urgent = certificate(&["api.example.com"], (2026, 6, 4));
        assert_eq!(inspect(&urgent, &target("api.example.com"), NOW, 30).unwrap().findings[0].severity, Severity::High);

        let other = certificate(&["*.example.com"], (2027, 1, 1));
        let inspected = inspect(&other, &target("a.b.example.com"), NOW, 30).unwrap();
        assert_eq!(rules(&inspected), ["cert.hostname"]);
        assert_eq!(inspected.findings[0].detail, "Not for a.b.example.com:443; for *.example.com");
    }

    #[test]
    fn ip_addresses_match_ip_names() {
        let der = certificate(&["127.0.0.1"], (2027, 1, 1));
        assert_eq!(rules(&inspect(&der, &target("127.0.0.1"), NOW, 30).unwrap()), Vec::<&str>::new());
        assert_eq!(rules(&inspect(&der, &target("localhost"), NOW, 30).unwrap()), ["cert.hostname"]);
    }

    #[test]
    fn reads_targets() {
        assert_eq!(target("example.com").location(), "example.com:443");
        assert_eq!(target("https://Example.com:8443/path?q").location(), "example.com:8443");
        assert_eq!(target("[::1]:9443").location(), "[::1]:9443");
        assert!(Target::parse("http://example.com").is_err());
        assert!(matches!(Target::parse(" "), Err(Error::MissingUrl)));
    }

    #[test]
    fn counts_rsa_bits_without_leading_zeros() {
        assert_eq!(rsa_bits(&[0x00, 0x80, 0x00]), 16);
        assert_eq!(rsa_bits(&[0x01, 0xff]), 9);
        assert_eq!(rsa_bits(&[]), 0);
    }

    #[test]
    fn wildcards_cover_one_label() {
        assert!(dns_matches("*.example.com", "api.example.com"));
        assert!(!dns_matches("*.example.com", "example.com"));
        assert!(!dns_matches("*.example.com", "a.b.example.com"));
        assert!(dns_matches("example.com", "example.com."));
    }
}
