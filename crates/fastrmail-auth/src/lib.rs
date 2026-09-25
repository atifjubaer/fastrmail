//! FastrMail Auth — DKIM, SPF, DMARC verification, DKIM signing, and Argon2 password hashing.

use std::net::IpAddr;

use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use mail_auth::{
    common::{
        crypto::{RsaKey, Sha256},
        headers::HeaderWriter,
    },
    dkim::generate::DkimKeyPair,
    AuthenticatedMessage, Resolver,
};
use mail_builder::encoders::base64::base64_encode;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ─── Verification Models ─────────────────────────────────────

/// Result of DKIM verification on an incoming email.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DkimResult {
    pub pass: bool,
    pub domain: String,
    pub selector: String,
}

/// Result of SPF verification on an incoming connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpfResult {
    pub pass: bool,
    pub domain: String,
}

/// Result of DMARC evaluation on an incoming email.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DmarcResult {
    pub policy: String,
    pub pass: bool,
}

/// Generated DKIM key pair container.
#[derive(Debug, Clone)]
pub struct DkimGeneratedKey {
    pub private_key_pem: String,
    pub public_key_base64: String,
}

// ─── DKIM Verifier ───────────────────────────────────────────

pub struct DkimVerifier;

impl DkimVerifier {
    /// Verify DKIM signature(s) on the provided raw RFC 5322 email bytes.
    pub async fn verify_dkim(raw_email: &[u8]) -> Result<DkimResult> {
        let authenticated_message = match AuthenticatedMessage::parse(raw_email) {
            Some(msg) => msg,
            None => {
                return Ok(DkimResult {
                    pass: false,
                    domain: String::new(),
                    selector: String::new(),
                });
            }
        };

        if authenticated_message.dkim_headers.is_empty() {
            return Ok(DkimResult {
                pass: false,
                domain: String::new(),
                selector: String::new(),
            });
        }

        // Extract domain and selector from the first parsed DKIM header
        let (domain, selector) = authenticated_message
            .dkim_headers
            .first()
            .and_then(|h| h.header.as_ref().ok())
            .map(|sig| (sig.d.clone(), sig.s.clone()))
            .unwrap_or_default();

        let resolver = match Resolver::new_system_conf().or_else(|_| Resolver::new_cloudflare()) {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to initialize DNS resolver for DKIM: {e}");
                return Ok(DkimResult {
                    pass: false,
                    domain,
                    selector,
                });
            }
        };

        let outputs = resolver.verify_dkim(&authenticated_message).await;
        let pass = outputs
            .iter()
            .any(|output| matches!(output.result(), mail_auth::DkimResult::Pass));

        debug!(
            "DKIM verification: domain={}, selector={}, pass={}",
            domain, selector, pass
        );

        Ok(DkimResult {
            pass,
            domain,
            selector,
        })
    }
}

// ─── SPF Verifier ────────────────────────────────────────────

pub struct SpfVerifier;

impl SpfVerifier {
    /// Verify SPF for a given client IP address, HELO/EHLO domain, and envelope sender address.
    pub async fn verify_spf(ip: IpAddr, helo: &str, sender: &str) -> Result<SpfResult> {
        let sender_domain = if let Some((_, domain)) = sender.split_once('@') {
            domain.trim().to_string()
        } else if !helo.is_empty() {
            helo.trim().to_string()
        } else {
            return Ok(SpfResult {
                pass: false,
                domain: String::new(),
            });
        };

        let resolver = match Resolver::new_system_conf().or_else(|_| Resolver::new_cloudflare()) {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to initialize DNS resolver for SPF: {e}");
                return Ok(SpfResult {
                    pass: false,
                    domain: sender_domain,
                });
            }
        };

        let output = resolver
            .verify_spf_sender(ip, helo, "localhost", sender)
            .await;

        let pass = matches!(output.result(), mail_auth::SpfResult::Pass);

        debug!(
            "SPF verification: ip={}, sender={}, domain={}, pass={}",
            ip, sender, sender_domain, pass
        );

        Ok(SpfResult {
            pass,
            domain: sender_domain,
        })
    }
}

// ─── DMARC Evaluator ─────────────────────────────────────────

pub struct DmarcEvaluator;

impl DmarcEvaluator {
    /// Evaluate DMARC alignment and policy based on DKIM, SPF, and the Header From domain.
    pub async fn evaluate(
        dkim_result: &DkimResult,
        spf_result: &SpfResult,
        from_domain: &str,
    ) -> Result<DmarcResult> {
        let from_domain = from_domain.trim().to_lowercase();

        // Check DKIM alignment (relaxed: matches domain or subdomain)
        let dkim_aligned = dkim_result.pass
            && !dkim_result.domain.is_empty()
            && (dkim_result.domain.to_lowercase() == from_domain
                || from_domain.ends_with(&format!(".{}", dkim_result.domain.to_lowercase())));

        // Check SPF alignment (relaxed)
        let spf_aligned = spf_result.pass
            && !spf_result.domain.is_empty()
            && (spf_result.domain.to_lowercase() == from_domain
                || from_domain.ends_with(&format!(".{}", spf_result.domain.to_lowercase())));

        let pass = dkim_aligned || spf_aligned;

        // Query DMARC policy from DNS (_dmarc.<from_domain>)
        let mut policy = "none".to_string();
        if let Ok(resolver) = Resolver::new_system_conf().or_else(|_| Resolver::new_cloudflare()) {
            let dmarc_record_name = format!("_dmarc.{}", from_domain);
            if let Ok(txt_bytes) = resolver.txt_raw_lookup(&dmarc_record_name).await {
                let txt = String::from_utf8_lossy(&txt_bytes);
                for part in txt.split(';') {
                    let part = part.trim();
                    if let Some(p) = part.strip_prefix("p=") {
                        let parsed_p = p.trim().to_lowercase();
                        if parsed_p == "reject" || parsed_p == "quarantine" || parsed_p == "none" {
                            policy = parsed_p;
                            break;
                        }
                    }
                }
            }
        }

        debug!(
            "DMARC evaluation: from_domain={}, dkim_pass={}, spf_pass={}, policy={}, overall_pass={}",
            from_domain, dkim_result.pass, spf_result.pass, policy, pass
        );

        Ok(DmarcResult { policy, pass })
    }
}

// ─── DKIM Signer ─────────────────────────────────────────────

pub struct DkimSigner;

impl DkimSigner {
    /// Sign an RFC 5322 email with DKIM using the given RSA private key PEM,
    /// returning the email bytes with the generated DKIM-Signature prepended.
    pub fn sign(
        raw_email: &[u8],
        domain: &str,
        selector: &str,
        private_key_pem: &str,
    ) -> Result<Vec<u8>> {
        // Load the RSA private key (try PKCS1 then PKCS8)
        let rsa_key = RsaKey::<Sha256>::from_rsa_pem(private_key_pem)
            .or_else(|_| RsaKey::<Sha256>::from_pkcs8_pem(private_key_pem))
            .map_err(|e| anyhow::anyhow!("Failed to parse RSA private key PEM: {e:?}"))?;

        let signer = mail_auth::dkim::DkimSigner::from_key(rsa_key)
            .domain(domain)
            .selector(selector)
            .headers(["From", "To", "Subject", "Date", "Message-ID"])
            .sign(raw_email)
            .map_err(|e| anyhow::anyhow!("Failed to generate DKIM signature: {e:?}"))?;

        let header = signer.to_header();
        let mut signed = Vec::with_capacity(header.len() + raw_email.len());
        signed.extend_from_slice(header.as_bytes());
        signed.extend_from_slice(raw_email);

        info!("Signed email for domain={}, selector={}", domain, selector);
        Ok(signed)
    }

    /// Generate a fresh 2048-bit RSA key pair for DKIM signing.
    pub fn generate_key_pair() -> Result<DkimGeneratedKey> {
        let key_pair = DkimKeyPair::generate_rsa(2048)
            .map_err(|e| anyhow::anyhow!("Failed to generate RSA key: {e:?}"))?;

        let raw_der = key_pair.private_key();
        let b64 = base64_encode(raw_der).unwrap_or_default();
        let b64_str = String::from_utf8_lossy(&b64);

        // Format into standard PKCS#1 PEM lines of 64 chars
        let mut pem = String::from("-----BEGIN RSA PRIVATE KEY-----\n");
        for chunk in b64_str.as_bytes().chunks(64) {
            pem.push_str(std::str::from_utf8(chunk)?);
            pem.push('\n');
        }
        pem.push_str("-----END RSA PRIVATE KEY-----\n");

        let public_key_base64 = key_pair.encoded_public_key();

        Ok(DkimGeneratedKey {
            private_key_pem: pem,
            public_key_base64,
        })
    }
}

// ─── Password Hashing ────────────────────────────────────────

/// Hash a plaintext password using Argon2id with a secure random salt.
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Argon2 hash error: {e}"))?
        .to_string();
    Ok(hash)
}

/// Verify a plaintext password against an Argon2id password hash.
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("Invalid password hash: {e}"))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

// ─── DNSBL Verifier (Spam Guard) ─────────────────────────────

/// Real-time DNS Blocklist (DNSBL) verifier for inbound IP reputation checking.
pub struct DnsblVerifier {
    zones: Vec<String>,
    resolver: TokioAsyncResolver,
    mock_blocked: bool,
}

impl Default for DnsblVerifier {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsblVerifier {
    /// Create a new DNSBL verifier with default blocklist zones.
    pub fn new() -> Self {
        Self::with_zones(vec![
            "zen.spamhaus.org".to_string(),
            "b.barracudacentral.org".to_string(),
        ])
    }

    /// Create a new DNSBL verifier with a custom list of zones.
    pub fn with_zones(zones: Vec<String>) -> Self {
        let resolver =
            TokioAsyncResolver::tokio(ResolverConfig::cloudflare(), ResolverOpts::default());
        Self {
            zones,
            resolver,
            mock_blocked: false,
        }
    }

    /// Create a mock DNSBL verifier that always simulates a blocked IP (for testing).
    pub fn mock_blocked() -> Self {
        let resolver =
            TokioAsyncResolver::tokio(ResolverConfig::cloudflare(), ResolverOpts::default());
        Self {
            zones: Vec::new(),
            resolver,
            mock_blocked: true,
        }
    }

    /// Check if the given IP address is listed on any of the configured DNSBL zones.
    ///
    /// Reverses the IP address octets (e.g. `1.2.3.4` -> `4.3.2.1.<zone>`)
    /// and performs an `A` record query. If any `127.0.0.x` response is returned,
    /// the IP is considered blocked (returns `Ok(true)`).
    /// If NXDOMAIN or clean, returns `Ok(false)`.
    pub async fn check_ip(&self, ip: IpAddr) -> Result<bool> {
        if self.mock_blocked {
            info!("DNSBL mock triggered: blocking IP {ip}");
            return Ok(true);
        }
        let reversed = match ip {
            IpAddr::V4(v4) => {
                let o = v4.octets();
                format!("{}.{}.{}.{}", o[3], o[2], o[1], o[0])
            }
            IpAddr::V6(v6) => {
                let mut nibbles = Vec::with_capacity(32);
                for seg in v6.segments().iter() {
                    for nibble in format!("{:04x}", seg).chars() {
                        nibbles.push(nibble);
                    }
                }
                nibbles.reverse();
                nibbles
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(".")
            }
        };

        for zone in &self.zones {
            let lookup_host = format!("{reversed}.{zone}.");
            debug!("Querying DNSBL: {lookup_host}");
            match self.resolver.ipv4_lookup(&lookup_host).await {
                Ok(lookup) => {
                    for record in lookup.iter() {
                        let octets = record.octets();
                        // Per RFC 5782, DNSBL return codes for listed IP addresses are in 127.0.0.x (e.g. 127.0.0.2 - 127.0.0.127).
                        // IPs like 127.255.255.x are query refusal / rate-limit notices by providers and not spam listings.
                        if octets[0] == 127 && octets[1] == 0 && octets[2] == 0 && octets[3] >= 2 {
                            info!("IP {ip} is listed on DNSBL {zone}: {record}");
                            return Ok(true);
                        }
                    }
                }
                Err(e) => {
                    debug!("DNSBL lookup for {lookup_host} clean or unlisted: {e}");
                }
            }
        }

        Ok(false)
    }
}

// ─── LDAP Authentication Gateway ──────────────────────────────

/// LDAP / Active Directory authentication gateway for enterprise single sign-on.
#[derive(Debug, Clone)]
pub struct LdapAuthGateway {
    pub ldap_url: String,
    pub bind_dn_template: String,
    mock_mode: bool,
    mock_users: std::collections::HashMap<String, String>,
}

impl Default for LdapAuthGateway {
    fn default() -> Self {
        let ldap_url = std::env::var("FASTRMAIL_LDAP_URL").unwrap_or_default();
        let bind_dn_template = std::env::var("FASTRMAIL_LDAP_BIND_DN_TEMPLATE")
            .unwrap_or_else(|_| "uid={},ou=users,dc=fastrmail,dc=internal".to_string());
        Self {
            ldap_url,
            bind_dn_template,
            mock_mode: false,
            mock_users: std::collections::HashMap::new(),
        }
    }
}

impl LdapAuthGateway {
    /// Create a new LDAP gateway pointing to the given LDAP server URL.
    pub fn new(ldap_url: &str, bind_dn_template: &str) -> Self {
        Self {
            ldap_url: ldap_url.to_string(),
            bind_dn_template: bind_dn_template.to_string(),
            mock_mode: false,
            mock_users: std::collections::HashMap::new(),
        }
    }

    /// Create a mock gateway for automated testing.
    pub fn new_mock() -> Self {
        let mut mock_users = std::collections::HashMap::new();
        mock_users.insert("ldapuser@example.com".to_string(), "ldap_secret_123".to_string());
        mock_users.insert("admin@corp.internal".to_string(), "corp_pass_2026".to_string());
        Self {
            ldap_url: "ldap://mock.internal:389".to_string(),
            bind_dn_template: "uid={},ou=users,dc=example,dc=com".to_string(),
            mock_mode: true,
            mock_users,
        }
    }

    /// Add a user to mock gateway for tests.
    pub fn add_mock_user(&mut self, username: &str, password: &str) {
        self.mock_users.insert(username.to_string(), password.to_string());
    }

    /// Check if LDAP is configured and active.
    pub fn is_enabled(&self) -> bool {
        self.mock_mode || !self.ldap_url.is_empty()
    }

    /// Authenticate a user against the LDAP / Active Directory directory.
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<bool> {
        if !self.is_enabled() {
            return Ok(false);
        }

        if self.mock_mode {
            if let Some(expected_pw) = self.mock_users.get(username) {
                return Ok(expected_pw == password);
            }
            return Ok(false);
        }

        let user_dn = if self.bind_dn_template.contains("{}") {
            self.bind_dn_template.replace("{}", username)
        } else {
            username.to_string()
        };

        info!("Attempting LDAP bind for DN: {user_dn}");
        let (conn, mut ldap) = ldap3::LdapConnAsync::new(&self.ldap_url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to LDAP server {}: {e}", self.ldap_url))?;

        ldap3::drive!(conn);

        let res = ldap
            .simple_bind(&user_dn, password)
            .await
            .map_err(|e| anyhow::anyhow!("LDAP simple bind failed: {e}"))?;

        if res.rc == 0 {
            info!("LDAP bind successful for user: {username}");
            Ok(true)
        } else {
            warn!("LDAP bind failed with result code {}: {}", res.rc, res.matched);
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_and_verify() {
        let password = "SuperSecretPassword123!";
        let hash = hash_password(password).expect("Hashing should succeed");
        assert!(hash.starts_with("$argon2id$"));

        // Verify correct password
        let is_valid = verify_password(password, &hash).expect("Verification should succeed");
        assert!(is_valid, "Correct password must verify successfully");

        // Verify wrong password
        let is_invalid =
            verify_password("WrongPassword!", &hash).expect("Verification should succeed");
        assert!(!is_invalid, "Incorrect password must fail verification");
    }

    #[test]
    fn test_dkim_key_generation() {
        let keys = DkimSigner::generate_key_pair().expect("DKIM key generation should succeed");
        assert!(keys.private_key_pem.contains("BEGIN RSA PRIVATE KEY"));
        assert!(!keys.public_key_base64.is_empty());

        // Verify the generated key PEM can be parsed by RsaKey
        let rsa = RsaKey::<Sha256>::from_rsa_pem(&keys.private_key_pem);
        assert!(rsa.is_ok(), "Generated PEM should be valid PKCS#1 RSA");
    }

    #[test]
    fn test_dkim_sign() {
        let keys = DkimSigner::generate_key_pair().expect("DKIM key generation should succeed");

        let raw_email = b"From: sender@example.com\r\n\
                          To: recipient@example.com\r\n\
                          Subject: Hello from FastrMail\r\n\
                          Date: Thu, 24 Sep 2026 00:00:00 +0000\r\n\
                          Message-ID: <test1234@example.com>\r\n\
                          \r\n\
                          This is a test message body.";

        let signed = DkimSigner::sign(raw_email, "example.com", "default", &keys.private_key_pem)
            .expect("Signing should succeed");

        let signed_str = String::from_utf8_lossy(&signed);
        assert!(
            signed_str.starts_with("DKIM-Signature:"),
            "Signed email must start with DKIM-Signature header"
        );
        assert!(signed_str.contains("d=example.com;"));
        assert!(signed_str.contains("s=default;"));
        assert!(signed_str.contains("This is a test message body."));
    }

    #[tokio::test]
    async fn test_dkim_verify_unsigned() {
        let raw_email = b"From: sender@example.com\r\n\
                          To: recipient@example.com\r\n\
                          Subject: Unsigned Email\r\n\
                          \r\n\
                          No DKIM signature here.";

        let result = DkimVerifier::verify_dkim(raw_email).await.unwrap();
        assert!(!result.pass, "Unsigned email should not pass DKIM");
        assert!(result.domain.is_empty());
    }

    #[tokio::test]
    async fn test_spf_verify_format() {
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let result = SpfVerifier::verify_spf(ip, "mail.example.com", "user@example.com")
            .await
            .unwrap();
        assert_eq!(result.domain, "example.com");
    }

    #[tokio::test]
    async fn test_dmarc_evaluator_aligned() {
        let dkim = DkimResult {
            pass: true,
            domain: "example.com".to_string(),
            selector: "default".to_string(),
        };
        let spf = SpfResult {
            pass: false,
            domain: "other.com".to_string(),
        };

        // When DKIM passes and aligns with From domain, DMARC passes
        let result = DmarcEvaluator::evaluate(&dkim, &spf, "example.com")
            .await
            .unwrap();
        assert!(result.pass, "Aligned DKIM must pass DMARC");

        // When From domain does not align, DMARC fails
        let unaligned = DmarcEvaluator::evaluate(&dkim, &spf, "different.org")
            .await
            .unwrap();
        assert!(!unaligned.pass, "Unaligned domain must fail DMARC");
    }

    #[tokio::test]
    async fn test_dmarc_evaluator_spf_aligned() {
        let dkim = DkimResult {
            pass: false,
            domain: String::new(),
            selector: String::new(),
        };
        let spf = SpfResult {
            pass: true,
            domain: "fastrmail.org".to_string(),
        };

        // When SPF passes and aligns, DMARC passes
        let result = DmarcEvaluator::evaluate(&dkim, &spf, "fastrmail.org")
            .await
            .unwrap();
        assert!(result.pass, "Aligned SPF must pass DMARC");
    }

    #[tokio::test]
    async fn test_dnsbl_safe_ip() {
        let verifier = DnsblVerifier::new();
        let safe_ip: IpAddr = "8.8.8.8".parse().unwrap();
        let is_blocked = verifier.check_ip(safe_ip).await.unwrap_or(false);
        assert!(
            !is_blocked,
            "8.8.8.8 must not be blocked on standard DNSBLs"
        );
    }

    #[tokio::test]
    async fn test_dnsbl_empty_zones() {
        let verifier = DnsblVerifier::with_zones(vec![]);
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        let is_blocked = verifier.check_ip(ip).await.unwrap();
        assert!(!is_blocked, "Empty zones must always return unblocked");
    }

    #[tokio::test]
    async fn test_ldap_auth_mock() {
        let mut gateway = LdapAuthGateway::new_mock();
        assert!(gateway.is_enabled());

        // Test existing mock users
        assert!(gateway
            .authenticate("ldapuser@example.com", "ldap_secret_123")
            .await
            .unwrap());
        assert!(!gateway
            .authenticate("ldapuser@example.com", "wrong_password")
            .await
            .unwrap());

        // Add fresh user
        gateway.add_mock_user("jane@company.org", "SecurePassword99!");
        assert!(gateway
            .authenticate("jane@company.org", "SecurePassword99!")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_ldap_auth_disabled() {
        let gateway = LdapAuthGateway::new("", "");
        assert!(!gateway.is_enabled());
        let res = gateway.authenticate("anyone", "pass").await.unwrap();
        assert!(!res);
    }
}
