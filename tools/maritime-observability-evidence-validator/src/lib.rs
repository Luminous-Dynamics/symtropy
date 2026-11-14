//! Shared evidence-integrity primitives for maritime benchmark executables.
use sha2::{Digest, Sha256};

/// Verify caller-supplied content identity against the bytes actually read.
/// This prevents a syntactically valid but unrelated digest from being copied
/// into an evidence receipt.
pub fn verify_sha256(bytes: &[u8], expected: &str) -> Result<(), String> {
    if expected.len() != 64
        || !expected
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("expected lowercase SHA-256 digest".to_owned());
    }
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected {
        return Err(format!(
            "input SHA-256 mismatch: expected {expected}, computed {actual}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_content_digest_is_accepted() {
        let bytes = b"maritime-evidence-v0.1";
        let digest = format!("{:x}", Sha256::digest(bytes));
        assert!(verify_sha256(bytes, &digest).is_ok());
    }

    #[test]
    fn changed_content_is_rejected() {
        let bytes = b"maritime-evidence-v0.1";
        let digest = format!("{:x}", Sha256::digest(bytes));
        assert!(verify_sha256(b"tampered-evidence-v0.1", &digest).is_err());
    }

    #[test]
    fn malformed_or_uppercase_digest_is_rejected() {
        assert!(verify_sha256(b"x", "bad").is_err());
        assert!(verify_sha256(b"x", &"A".repeat(64)).is_err());
    }
}
