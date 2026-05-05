//! Checksum parsing and signature/hash verification for updater assets.

use std::{fs::File, io::Read, path::Path};

use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

use super::super::{CHECKSUMS_PUBLIC_KEY_BASE64, UpdateError};

const MAX_SIGNATURE_BYTES: usize = 8 * 1024;

/// Parse a checksums file and return the hash for the requested asset name.
pub(crate) fn parse_checksums_for_asset(
    checksums: &[u8],
    asset_name: &str,
) -> Result<String, UpdateError> {
    let text = std::str::from_utf8(checksums)
        .map_err(|err| UpdateError::Invalid(format!("Invalid checksums file: {err}")))?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((hash, filename)) = line.split_once("  ") else {
            continue;
        };
        if filename.trim() == asset_name {
            return Ok(hash.trim().to_string());
        }
    }
    Err(UpdateError::Invalid(format!(
        "Checksums file did not include {asset_name}"
    )))
}

/// Verify the checksums signature using the embedded public key.
pub(crate) fn verify_checksums_signature(
    checksums: &[u8],
    signature_bytes: &[u8],
) -> Result<(), UpdateError> {
    verify_checksums_signature_with_key(checksums, signature_bytes, CHECKSUMS_PUBLIC_KEY_BASE64)
}

/// Compare the on-disk zip checksum to the expected SHA-256 digest.
pub(crate) fn verify_zip_checksum(path: &Path, expected: &str) -> Result<(), UpdateError> {
    let actual = sha256_file(path)?;
    if actual != expected {
        return Err(UpdateError::ChecksumMismatch {
            filename: path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("archive.zip")
                .to_string(),
            expected: expected.to_string(),
            actual,
        });
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, UpdateError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn verify_checksums_signature_with_key(
    checksums: &[u8],
    signature_bytes: &[u8],
    public_key_base64: &str,
) -> Result<(), UpdateError> {
    let signature_text = std::str::from_utf8(signature_bytes)
        .map_err(|err| UpdateError::Invalid(format!("Invalid signature file: {err}")))?;
    let signature_text = signature_text.trim();
    if signature_text.is_empty() {
        return Err(UpdateError::Invalid("Empty signature file".into()));
    }
    if signature_text.len() > MAX_SIGNATURE_BYTES {
        return Err(UpdateError::Invalid("Signature file too large".into()));
    }
    let signature_raw = base64::engine::general_purpose::STANDARD
        .decode(signature_text)
        .map_err(|err| UpdateError::Invalid(format!("Invalid signature encoding: {err}")))?;
    let signature = Signature::from_slice(&signature_raw)
        .map_err(|err| UpdateError::Invalid(format!("Invalid signature length: {err}")))?;
    let public_key_raw = base64::engine::general_purpose::STANDARD
        .decode(public_key_base64)
        .map_err(|err| UpdateError::Invalid(format!("Invalid public key encoding: {err}")))?;
    let public_key = VerifyingKey::from_bytes(
        public_key_raw
            .as_slice()
            .try_into()
            .map_err(|_| UpdateError::Invalid("Invalid public key length".into()))?,
    )
    .map_err(|err| UpdateError::Invalid(format!("Invalid public key: {err}")))?;
    public_key
        .verify_strict(checksums, &signature)
        .map_err(|err| UpdateError::Invalid(format!("Checksums signature mismatch: {err}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn verify_checksums_signature_rejects_invalid_signature() {
        let checksums = b"deadbeef  sempal.zip\n";
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let signature = signing_key.sign(b"other");
        let signature_text = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
        let public_key_text = base64::engine::general_purpose::STANDARD
            .encode(signing_key.verifying_key().to_bytes());
        let err = verify_checksums_signature_with_key(
            checksums,
            signature_text.as_bytes(),
            &public_key_text,
        )
        .expect_err("tampered signature must fail");
        assert!(err.to_string().contains("Checksums signature mismatch"));
    }

    #[test]
    fn verify_checksums_signature_rejects_empty_signature() {
        let checksums = b"deadbeef  sempal.zip\n";
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let public_key_text = base64::engine::general_purpose::STANDARD
            .encode(signing_key.verifying_key().to_bytes());
        let err = verify_checksums_signature_with_key(checksums, b"  \n", &public_key_text)
            .expect_err("empty signature must fail");
        assert!(err.to_string().contains("Empty signature file"));
    }

    #[test]
    fn verify_checksums_signature_rejects_tampered_checksums() {
        let checksums = b"deadbeef  sempal.zip\n";
        let tampered = b"beefdead  sempal.zip\n";
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let signature = signing_key.sign(checksums);
        let signature_text = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
        let public_key_text = base64::engine::general_purpose::STANDARD
            .encode(signing_key.verifying_key().to_bytes());
        let err = verify_checksums_signature_with_key(
            tampered,
            signature_text.as_bytes(),
            &public_key_text,
        )
        .expect_err("tampered checksums must fail");
        assert!(err.to_string().contains("Checksums signature mismatch"));
    }

    #[test]
    fn verify_checksums_signature_accepts_valid_signature() {
        let checksums = b"deadbeef  sempal.zip\n";
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let signature = signing_key.sign(checksums);
        let signature_text = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
        let public_key_text = base64::engine::general_purpose::STANDARD
            .encode(signing_key.verifying_key().to_bytes());
        verify_checksums_signature_with_key(checksums, signature_text.as_bytes(), &public_key_text)
            .expect("signature should verify");
    }
}
