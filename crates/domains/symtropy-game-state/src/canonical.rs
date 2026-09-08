// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Serializer-independent canonical encoding primitives for causal identity.
//!
//! These helpers deliberately do not use `serde` bytes as identity input. Variable-length
//! values carry explicit lengths, options carry explicit presence tags, integers use a frozen
//! big-endian representation, and every digest is domain-separated by its caller.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

/// SHA-256 digest over a frozen canonical byte contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CanonicalDigest([u8; 32]);

impl CanonicalDigest {
    /// Constructs a digest from exact bytes.
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the exact digest bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns lowercase hexadecimal text for diagnostics and frozen vectors.
    pub fn to_hex(self) -> String {
        hex(&self.0)
    }
}

impl fmt::Display for CanonicalDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&hex(&self.0))
    }
}

/// Canonical digest supplied by the domain that owns an event payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PayloadDigest(CanonicalDigest);

impl PayloadDigest {
    /// Wraps an already-domain-separated canonical digest as a payload digest.
    pub const fn new(digest: CanonicalDigest) -> Self {
        Self(digest)
    }

    /// Returns the underlying canonical digest.
    pub const fn canonical(self) -> CanonicalDigest {
        self.0
    }
}

impl fmt::Display for PayloadDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Canonical v2 event identity digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EventDigestV2(CanonicalDigest);

impl EventDigestV2 {
    /// Wraps the canonical event digest.
    pub const fn new(digest: CanonicalDigest) -> Self {
        Self(digest)
    }

    /// Returns the underlying canonical digest.
    pub const fn canonical(self) -> CanonicalDigest {
        self.0
    }
}

impl fmt::Display for EventDigestV2 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Domain-owned payloads provide their own serializer-independent semantic digest.
///
/// The event layer binds both `PAYLOAD_SCHEMA` and the returned digest. It never hashes the
/// payload's `serde` representation as canonical identity.
pub trait CanonicalEventPayload {
    /// Stable portable schema identifier for the semantic payload digest contract.
    const PAYLOAD_SCHEMA: &'static str;

    /// Returns the domain-owned canonical semantic digest of this payload.
    fn canonical_payload_digest(&self) -> PayloadDigest;
}

/// Incremental writer for the frozen canonical byte grammar.
///
/// The writer starts every digest with `domain || 0x00`. Callers must choose a stable non-empty
/// ASCII domain separator. Variable-length byte/string values are encoded as `u64_be(len) ||
/// bytes`. Collections use the same explicit `u64_be(count)`. Options use one byte: `0` for
/// absent, `1` for present.
pub struct CanonicalWriter {
    hasher: Sha256,
}

impl CanonicalWriter {
    /// Starts a new domain-separated canonical digest.
    pub fn new(domain: &'static [u8]) -> Result<Self, CanonicalError> {
        if domain.is_empty() || !domain.is_ascii() || domain.contains(&0) {
            return Err(CanonicalError::InvalidDomainSeparator);
        }
        let mut hasher = Sha256::new();
        hasher.update(domain);
        hasher.update([0]);
        Ok(Self { hasher })
    }

    /// Writes one byte exactly.
    pub fn write_u8(&mut self, value: u8) {
        self.hasher.update([value]);
    }

    /// Writes an unsigned 32-bit integer in big-endian order.
    pub fn write_u32(&mut self, value: u32) {
        self.hasher.update(value.to_be_bytes());
    }

    /// Writes an unsigned 64-bit integer in big-endian order.
    pub fn write_u64(&mut self, value: u64) {
        self.hasher.update(value.to_be_bytes());
    }

    /// Writes an explicit portable collection count.
    pub fn write_count(&mut self, count: usize) -> Result<(), CanonicalError> {
        let count = u64::try_from(count).map_err(|_| CanonicalError::LengthOverflow)?;
        self.write_u64(count);
        Ok(())
    }

    /// Writes length-prefixed bytes.
    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), CanonicalError> {
        self.write_count(bytes.len())?;
        self.hasher.update(bytes);
        Ok(())
    }

    /// Writes a UTF-8 string as length-prefixed bytes.
    pub fn write_str(&mut self, value: &str) -> Result<(), CanonicalError> {
        self.write_bytes(value.as_bytes())
    }

    /// Writes an optional value with an explicit presence tag.
    pub fn write_option<T>(
        &mut self,
        value: Option<&T>,
        write_value: impl FnOnce(&mut Self, &T) -> Result<(), CanonicalError>,
    ) -> Result<(), CanonicalError> {
        match value {
            Some(value) => {
                self.write_u8(1);
                write_value(self, value)
            }
            None => {
                self.write_u8(0);
                Ok(())
            }
        }
    }

    /// Writes a fixed 32-byte canonical digest without another length prefix.
    pub fn write_digest(&mut self, digest: CanonicalDigest) {
        self.hasher.update(digest.as_bytes());
    }

    /// Finalizes this canonical digest.
    pub fn finish(self) -> CanonicalDigest {
        CanonicalDigest::from_bytes(self.hasher.finalize().into())
    }
}

/// Canonical encoding failures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonicalError {
    /// The digest domain separator was empty, non-ASCII, or contained NUL.
    InvalidDomainSeparator,
    /// A host collection length could not be represented by the frozen `u64` grammar.
    LengthOverflow,
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDomainSeparator => formatter
                .write_str("canonical domain separator must be non-empty ASCII without NUL"),
            Self::LengthOverflow => formatter.write_str("canonical length exceeds u64"),
        }
    }
}

impl Error for CanonicalError {}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_encoding_is_explicit_and_reproducible() {
        let mut first = CanonicalWriter::new(b"symtropy/test/v1").expect("valid domain");
        first.write_u8(7);
        first.write_u32(0x0102_0304);
        first.write_u64(9);
        first.write_str("gerald").expect("length fits");
        first
            .write_option(Some(&42_u64), |writer, value| {
                writer.write_u64(*value);
                Ok(())
            })
            .expect("option encodes");

        let mut second = CanonicalWriter::new(b"symtropy/test/v1").expect("valid domain");
        second.write_u8(7);
        second.write_u32(0x0102_0304);
        second.write_u64(9);
        second.write_str("gerald").expect("length fits");
        second
            .write_option(Some(&42_u64), |writer, value| {
                writer.write_u64(*value);
                Ok(())
            })
            .expect("option encodes");

        assert_eq!(first.finish(), second.finish());
    }

    #[test]
    fn domain_separation_changes_identity() {
        let mut a = CanonicalWriter::new(b"symtropy/a/v1").expect("valid domain");
        a.write_u64(5);
        let mut b = CanonicalWriter::new(b"symtropy/b/v1").expect("valid domain");
        b.write_u64(5);
        assert_ne!(a.finish(), b.finish());
    }
}
