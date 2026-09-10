use sha2::Sha256;
use sha2::Digest;
use std::fmt;

pub(crate) fn put_u32(digest: &mut Sha256, value: u32) {
    digest.update(value.to_le_bytes());
}

pub(crate) fn put_u64(digest: &mut Sha256, value: u64) {
    digest.update(value.to_le_bytes());
}

pub(crate) fn put_text(digest: &mut Sha256, value: &str) {
    put_u64(digest, value.len() as u64);
    digest.update(value.as_bytes());
}

pub(crate) fn fmt_hex(bytes: &[u8; 32], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in bytes {
        write!(f, "{byte:02x}")?;
    }
    Ok(())
}
