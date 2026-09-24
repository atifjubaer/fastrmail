//! FastrMail Auth — DKIM, SPF, DMARC verification, DKIM signing, and Argon2 password hashing.

use std::net::IpAddr;

use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
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
        let pass = outputs.iter().any(|output| {
            matches!(output.result(), mail_auth::DkimResult::Pass)
        });

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
}
