// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Validated namespaces for canonical v2 deterministic identifiers.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

use crate::StableId;

const DERIVED_SUFFIX_LEN: usize = 1 + 32; // ':' plus 16 digest bytes rendered as hex.
const MAX_STABLE_ID_LEN: usize = 96;
const MAX_NAMESPACE_LEN: usize = MAX_STABLE_ID_LEN - DERIVED_SUFFIX_LEN;

/// Portable validated namespace used by canonical v2 deterministic identifiers.
///
/// The maximum length is intentionally smaller than a complete `StableId` so that the derived
/// `namespace:<32 hex chars>` value always satisfies the existing 96-byte StableId contract.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StableIdNamespace(String);

impl StableIdNamespace {
    /// Parses a namespace under the portable StableId character grammar.
    pub fn parse(value: impl Into<String>) -> Result<Self, NamespaceError> {
        let namespace = Self(value.into());
        namespace.validate()?;
        Ok(namespace)
    }

    /// Re-validates a namespace after deserialization.
    pub fn validate(&self) -> Result<(), NamespaceError> {
        let valid = !self.0.is_empty()
            && self.0.len() <= MAX_NAMESPACE_LEN
            && self.0.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':')
            });
        if valid {
            Ok(())
        } else {
            Err(NamespaceError::Invalid(self.0.clone()))
        }
    }

    /// Returns portable namespace text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StableIdNamespace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl StableId {
    /// Derives a canonical-v2 stable identifier from a validated namespace, seed, and ordinal.
    ///
    /// This path is additive and deliberately does not alter the historical v1 `derive` contract.
    /// Validation is repeated here so an invalid value created through deserialization cannot
    /// bypass the v2 portable-namespace boundary.
    pub fn derive_v2(
        namespace: &StableIdNamespace,
        seed: u64,
        ordinal: u64,
    ) -> Result<Self, NamespaceError> {
        namespace.validate()?;
        let namespace_len =
            u64::try_from(namespace.as_str().len()).map_err(|_| NamespaceError::Invalid(namespace.as_str().to_owned()))?;
        let mut hasher = Sha256::new();
        hasher.update(b"symtropy/stable-id/v2\0");
        hasher.update(namespace_len.to_be_bytes());
        hasher.update(namespace.as_str().as_bytes());
        hasher.update(seed.to_be_bytes());
        hasher.update(ordinal.to_be_bytes());
        let digest = hasher.finalize();
        let id = format!("{}:{}", namespace.as_str(), hex(&digest[..16]));
        Ok(Self(id))
    }
}

/// Namespace validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamespaceError {
    /// Namespace was empty, too long for a derived StableId, or used non-portable characters.
    Invalid(String),
}

impl fmt::Display for NamespaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(value) => write!(formatter, "invalid stable-id namespace: {value:?}"),
        }
    }
}

impl Error for NamespaceError {}

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
    fn v2_derived_ids_are_portable_and_reproducible() {
        let namespace = StableIdNamespace::parse("resident").expect("valid namespace");
        let first = StableId::derive_v2(&namespace, 41, 7).expect("validated derivation");
        let second = StableId::derive_v2(&namespace, 41, 7).expect("validated derivation");
        assert_eq!(first, second);
        assert!(StableId::parse(first.as_str()).is_ok());
    }

    #[test]
    fn namespace_cannot_make_an_oversized_stable_id() {
        let oversized = "a".repeat(MAX_NAMESPACE_LEN + 1);
        assert!(matches!(
            StableIdNamespace::parse(oversized),
            Err(NamespaceError::Invalid(_))
        ));
    }

    #[test]
    fn derive_revalidates_deserialized_namespace() {
        let invalid: StableIdNamespace = serde_json::from_str("\"contains space\"")
            .expect("transparent deserialization can construct raw representation");
        assert!(matches!(
            StableId::derive_v2(&invalid, 1, 0),
            Err(NamespaceError::Invalid(_))
        ));
    }
}
