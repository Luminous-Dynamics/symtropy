// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Narrow C2SP checkpoint/cosignature wire adapter.
//!
//! This module maps the service-neutral Symtropy VDS fields onto the current
//! C2SP note and Ed25519 timestamped-cosignature shapes. It intentionally does
//! not reuse the internal Symtropy checkpoint signature as a C2SP note signature:
//! the two signatures cover different canonical messages.
//!
//! The adapter does not implement HTTP, policy files, or SCITT/COSE receipts.

use ring::signature::{ED25519, UnparsedPublicKey};
use sha2::{Digest, Sha256};

use super::transparency::TransparencyCheckpointV1;

const COSIGNATURE_HEADER: &str = "cosignature/v1";
const WITNESS_SIGNATURE_TYPE: u8 = 0x04;
const MAX_COSIGNATURE_TIMESTAMP: u64 = i64::MAX as u64;

/// A canonical C2SP transparency checkpoint note body without signature lines.
///
/// C2SP notes have an origin, decimal tree size, base64 RFC 6962 Merkle root,
/// and optional non-empty extension lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C2spCheckpointNoteBodyV1 {
    origin: String,
    tree_size: u64,
    root_hash_base64: String,
    extensions: Vec<String>,
    serialized: String,
}

impl C2spCheckpointNoteBodyV1 {
    pub fn from_checkpoint(
        checkpoint: &TransparencyCheckpointV1,
        origin: impl Into<String>,
    ) -> Result<Self, C2spWireError> {
        Self::new(
            origin,
            checkpoint.vds_tree_size(),
            &checkpoint.vds_root_hash(),
            Vec::new(),
        )
    }

    pub fn new(
        origin: impl Into<String>,
        tree_size: u64,
        root_hash_hex: &str,
        extensions: Vec<String>,
    ) -> Result<Self, C2spWireError> {
        let origin = origin.into();
        if origin.is_empty() || origin.bytes().any(|byte| byte == b'\n' || byte == b'\r') {
            return Err(C2spWireError::Invalid(
                "C2SP checkpoint origin must be one non-empty line".to_string(),
            ));
        }
        let root = decode_hex_32(root_hash_hex)?;
        if tree_size == 0 {
            let empty_root: [u8; 32] = Sha256::digest(b"").into();
            if root != empty_root {
                return Err(C2spWireError::Invalid(
                    "C2SP empty checkpoint must use the empty-tree root".to_string(),
                ));
            }
        }
        if extensions.iter().any(|extension| {
            extension.is_empty()
                || extension
                    .bytes()
                    .any(|byte| byte == b'\n' || byte == b'\r')
        }) {
            return Err(C2spWireError::Invalid(
                "C2SP checkpoint extensions must be non-empty single lines".to_string(),
            ));
        }

        let mut serialized = format!(
            "{origin}\n{tree_size}\n{}\n",
            base64_encode(&root)
        );
        for extension in &extensions {
            serialized.push_str(extension);
            serialized.push('\n');
        }

        Ok(Self {
            origin,
            tree_size,
            root_hash_base64: base64_encode(&root),
            extensions,
            serialized,
        })
    }

    #[must_use]
    pub fn origin(&self) -> &str {
        &self.origin
    }

    #[must_use]
    pub const fn tree_size(&self) -> u64 {
        self.tree_size
    }

    #[must_use]
    pub fn root_hash_base64(&self) -> &str {
        &self.root_hash_base64
    }

    #[must_use]
    pub fn root_hash_hex(&self) -> Result<String, C2spWireError> {
        Ok(encode_hex(&decode_base64_exact::<32>(&self.root_hash_base64)?))
    }

    #[must_use]
    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }

    /// Return the exact note body covered by C2SP cosignatures, including its
    /// terminating newline but excluding all signature lines.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.serialized
    }

    pub fn validate_basic(&self) -> Result<(), C2spWireError> {
        let rebuilt = Self::new(
            self.origin.clone(),
            self.tree_size,
            &encode_hex(
                &decode_base64_exact::<32>(&self.root_hash_base64)
                    .map_err(|_| {
                        C2spWireError::Invalid(
                            "C2SP checkpoint root must be canonical base64 for 32 bytes"
                                .to_string(),
                        )
                    })?,
            ),
            self.extensions.clone(),
        )?;
        if rebuilt.serialized != self.serialized
            || rebuilt.root_hash_base64 != self.root_hash_base64
        {
            return Err(C2spWireError::Invalid(
                "C2SP checkpoint note is not canonically serialized".to_string(),
            ));
        }
        Ok(())
    }
}

/// Parsed C2SP v1 timestamped Ed25519 witness cosignature.
///
/// The wire signature blob is:
///     4-byte key ID || 8-byte big-endian timestamp || 64-byte Ed25519 signature
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct C2spTimestampedEd25519CosignatureV1 {
    witness_name: String,
    key_id: [u8; 4],
    timestamp: u64,
    signature: [u8; 64],
}

impl C2spTimestampedEd25519CosignatureV1 {
    pub fn from_signature_line(line: &str) -> Result<Self, C2spWireError> {
        let remainder = line.strip_prefix("— ").ok_or_else(|| {
            C2spWireError::Invalid(
                "C2SP cosignature must start with an em dash and space".to_string(),
            )
        })?;
        let (witness_name, encoded) = remainder.split_once(' ').ok_or_else(|| {
            C2spWireError::Invalid(
                "C2SP cosignature must contain witness name and base64 payload".to_string(),
            )
        })?;
        if witness_name.is_empty()
            || encoded.is_empty()
            || witness_name
                .bytes()
                .any(|byte| byte == b'\n' || byte == b'\r')
        {
            return Err(C2spWireError::Invalid(
                "C2SP cosignature witness name must be one non-empty line".to_string(),
            ));
        }

        let blob = decode_base64_exact::<76>(encoded)?;
        let mut key_id = [0_u8; 4];
        key_id.copy_from_slice(&blob[..4]);

        let mut timestamp_bytes = [0_u8; 8];
        timestamp_bytes.copy_from_slice(&blob[4..12]);
        let timestamp = u64::from_be_bytes(timestamp_bytes);
        if timestamp > MAX_COSIGNATURE_TIMESTAMP {
            return Err(C2spWireError::Invalid(
                "C2SP cosignature timestamp exceeds signed 63-bit POSIX range".to_string(),
            ));
        }

        let mut signature = [0_u8; 64];
        signature.copy_from_slice(&blob[12..]);

        Ok(Self {
            witness_name: witness_name.to_string(),
            key_id,
            timestamp,
            signature,
        })
    }

    #[must_use]
    pub fn witness_name(&self) -> &str {
        &self.witness_name
    }

    #[must_use]
    pub const fn key_id(&self) -> [u8; 4] {
        self.key_id
    }

    #[must_use]
    pub const fn timestamp(&self) -> u64 {
        self.timestamp
    }

    #[must_use]
    pub fn signature(&self) -> &[u8; 64] {
        &self.signature
    }

    #[must_use]
    pub fn signed_message(&self, note_body: &C2spCheckpointNoteBodyV1) -> String {
        format!(
            "{COSIGNATURE_HEADER}\ntime {}\n{}",
            self.timestamp,
            note_body.as_str()
        )
    }

    /// Verify the C2SP v1 witness key-ID derivation and Ed25519 signature.
    pub fn verify(
        &self,
        note_body: &C2spCheckpointNoteBodyV1,
        witness_public_key: &[u8; 32],
    ) -> Result<(), C2spWireError> {
        note_body.validate_basic()?;
        let expected_key_id = witness_key_id(&self.witness_name, witness_public_key);
        if self.key_id != expected_key_id {
            return Err(C2spWireError::KeyIdMismatch);
        }
        let message = self.signed_message(note_body);
        UnparsedPublicKey::new(&ED25519, witness_public_key)
            .verify(message.as_bytes(), &self.signature)
            .map_err(|_| C2spWireError::SignatureInvalid)
    }

    #[must_use]
    pub fn signature_line(&self) -> String {
        let mut blob = Vec::with_capacity(76);
        blob.extend_from_slice(&self.key_id);
        blob.extend_from_slice(&self.timestamp.to_be_bytes());
        blob.extend_from_slice(&self.signature);
        format!("— {} {}", self.witness_name, base64_encode(&blob))
    }
}

/// Compute the C2SP timestamped-Ed25519 witness key ID.
#[must_use]
pub fn witness_key_id(witness_name: &str, witness_public_key: &[u8; 32]) -> [u8; 4] {
    let mut hasher = Sha256::new();
    hasher.update(witness_name.as_bytes());
    hasher.update(b"\n");
    hasher.update([WITNESS_SIGNATURE_TYPE]);
    hasher.update(witness_public_key);
    let digest = hasher.finalize();
    [digest[0], digest[1], digest[2], digest[3]]
}

fn parse_canonical_decimal(value: &str) -> Result<u64, C2spWireError> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(C2spWireError::Invalid(
            "C2SP tree size must be canonical ASCII decimal".to_string(),
        ));
    }
    value.parse::<u64>().map_err(|_| {
        C2spWireError::Invalid("C2SP tree size exceeds uint64 range".to_string())
    })
}

fn decode_hex_32(value: &str) -> Result<[u8; 32], C2spWireError> {
    if value.len() != 64 {
        return Err(C2spWireError::Invalid(
            "expected 32-byte lowercase hexadecimal root".to_string(),
        ));
    }
    let mut bytes = [0_u8; 32];
    for index in 0..32 {
        let high = hex_nibble(value.as_bytes()[index * 2])?;
        let low = hex_nibble(value.as_bytes()[index * 2 + 1])?;
        bytes[index] = (high << 4) | low;
    }
    Ok(bytes)
}

fn hex_nibble(value: u8) -> Result<u8, C2spWireError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(C2spWireError::Invalid(
            "hexadecimal values must use lowercase ASCII".to_string(),
        )),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

const BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(bytes: &[u8]) -> String {
    let output_len = ((bytes.len() + 2) / 3) * 4;
    let mut output = String::with_capacity(output_len);
    let mut index = 0;
    while index < bytes.len() {
        let remaining = bytes.len() - index;
        let a = bytes[index];
        let b = if remaining > 1 { bytes[index + 1] } else { 0 };
        let c = if remaining > 2 { bytes[index + 2] } else { 0 };
        output.push(BASE64_TABLE[(a >> 2) as usize] as char);
        output.push(BASE64_TABLE[((a & 0x03) << 4 | (b >> 4)) as usize] as char);
        output.push(if remaining > 1 {
            BASE64_TABLE[((b & 0x0f) << 2 | (c >> 6)) as usize] as char
        } else {
            '='
        });
        output.push(if remaining > 2 {
            BASE64_TABLE[(c & 0x3f) as usize] as char
        } else {
            '='
        });
        index += 3;
    }
    output
}

fn decode_base64_exact<const N: usize>(encoded: &str) -> Result<[u8; N], C2spWireError> {
    if encoded.len() % 4 != 0 {
        return Err(C2spWireError::Invalid(
            "base64 payload length must be a multiple of four".to_string(),
        ));
    }
    let mut bytes = Vec::with_capacity(encoded.len() / 4 * 3);
    let chars = encoded.as_bytes();
    for chunk in chars.chunks_exact(4) {
        let a = base64_value(chunk[0])?;
        let b = base64_value(chunk[1])?;
        let c = if chunk[2] == b'=' { 0 } else { base64_value(chunk[2])? };
        let d = if chunk[3] == b'=' { 0 } else { base64_value(chunk[3])? };

        bytes.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            bytes.push((b << 4) | (c >> 2));
        }
        if chunk[3] != b'=' {
            bytes.push((c << 6) | d);
        }

        if chunk[2] == b'=' && chunk[3] != b'=' {
            return Err(C2spWireError::Invalid(
                "invalid base64 padding".to_string(),
            ));
        }
    }

    if bytes.len() != N {
        return Err(C2spWireError::Invalid(format!(
            "decoded base64 payload must contain exactly {N} bytes"
        )));
    }

    // Re-encode to reject non-canonical pad bits/alternate representations.
    if base64_encode(&bytes) != encoded {
        return Err(C2spWireError::Invalid(
            "base64 payload is not canonical".to_string(),
        ));
    }

    let mut output = [0_u8; N];
    output.copy_from_slice(&bytes);
    Ok(output)
}

fn base64_value(value: u8) -> Result<u8, C2spWireError> {
    match value {
        b'A'..=b'Z' => Ok(value - b'A'),
        b'a'..=b'z' => Ok(value - b'a' + 26),
        b'0'..=b'9' => Ok(value - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(C2spWireError::Invalid(
            "invalid base64 character".to_string(),
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum C2spWireError {
    Invalid(String),
    KeyIdMismatch,
    SignatureInvalid,
}

impl std::fmt::Display for C2spWireError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "invalid C2SP wire value: {message}"),
            Self::KeyIdMismatch => write!(formatter, "C2SP witness key ID mismatch"),
            Self::SignatureInvalid => write!(formatter, "C2SP witness signature invalid"),
        }
    }
}

impl std::error::Error for C2spWireError {}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::{
        rand::SystemRandom,
        signature::{Ed25519KeyPair, KeyPair},
    };

    #[test]
    fn checkpoint_note_serialization_is_canonical() {
        let root = [0xabu8; 32];
        let note = C2spCheckpointNoteBodyV1::new(
            "example.com/log",
            104,
            &encode_hex(&root),
            vec!["extension-v1".to_string()],
        )
        .expect("note");

        assert_eq!(note.as_str().lines().count(), 4);
        assert!(note.as_str().ends_with('\n'));
        note.validate_basic().expect("canonical");
    }

    #[test]
    fn checkpoint_note_parse_round_trip_preserves_wire_bytes() {
        let note = C2spCheckpointNoteBodyV1::new(
            "example.com/log",
            104,
            &encode_hex(&[0xabu8; 32]),
            vec!["extension-v1".to_string()],
        )
        .expect("note");
        let parsed = C2spCheckpointNoteBodyV1::parse(note.as_str()).expect("parse");
        assert_eq!(parsed, note);
        assert_eq!(parsed.root_hash_hex().expect("root"), encode_hex(&[0xabu8; 32]));
    }

    #[test]
    fn checkpoint_note_rejects_noncanonical_tree_size_and_empty_root_mismatch() {
        let root = encode_hex(&[0x00u8; 32]);
        assert!(matches!(
            C2spCheckpointNoteBodyV1::parse(&format!("example.com/log\n01\n{}\n", base64_encode(&[0u8; 32]))),
            Err(C2spWireError::Invalid(_))
        ));
        assert!(matches!(
            C2spCheckpointNoteBodyV1::new("example.com/log", 0, &root, Vec::new()),
            Err(C2spWireError::Invalid(_))
        ));
    }

    #[test]
    fn checkpoint_note_rejects_noncanonical_extensions_and_root() {
        assert!(matches!(
            C2spCheckpointNoteBodyV1::new("example.com/log", 1, &"AA".repeat(32), Vec::new()),
            Err(C2spWireError::Invalid(_))
        ));
        assert!(matches!(
            C2spCheckpointNoteBodyV1::new(
                "example.com/log",
                1,
                &"00".repeat(31),
                Vec::new()
            ),
            Err(C2spWireError::Invalid(_))
        ));
        assert!(matches!(
            C2spCheckpointNoteBodyV1::new(
                "example.com/log",
                1,
                &"00".repeat(32),
                vec!["".to_string()],
            ),
            Err(C2spWireError::Invalid(_))
        ));
    }

    #[test]
    fn timestamped_cosignature_verifies_and_round_trips() {
        let key_bytes = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
            .expect("key")
            .to_vec();
        let key_pair = Ed25519KeyPair::from_pkcs8(&key_bytes).expect("pair");
        let public_key = key_pair.public_key();
        let public_key: [u8; 32] = public_key.as_ref().try_into().expect("public key");

        let note = C2spCheckpointNoteBodyV1::new(
            "example.com/log",
            104,
            &encode_hex(&[0xabu8; 32]),
            Vec::new(),
        )
        .expect("note");
        let timestamp = 1_769_000_000u64;
        let witness_name = "witness.example/w1";
        let key_id = witness_key_id(witness_name, &public_key);
        let unsigned = C2spTimestampedEd25519CosignatureV1 {
            witness_name: witness_name.to_string(),
            key_id,
            timestamp,
            signature: {
                let message = format!(
                    "{COSIGNATURE_HEADER}\ntime {timestamp}\n{}",
                    note.as_str()
                );
                key_pair.sign(message.as_bytes()).as_ref().try_into().expect("signature")
            },
        };

        let line = unsigned.signature_line();
        let parsed =
            C2spTimestampedEd25519CosignatureV1::from_signature_line(&line).expect("parse");

        parsed.verify(&note, &public_key).expect("verify");
        assert_eq!(parsed.signature_line(), line);
    }

    #[test]
    fn key_id_mismatch_and_bad_signature_fail_closed() {
        let key_bytes = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
            .expect("key")
            .to_vec();
        let key_pair = Ed25519KeyPair::from_pkcs8(&key_bytes).expect("pair");
        let public_key: [u8; 32] = key_pair
            .public_key()
            .as_ref()
            .try_into()
            .expect("public key");
        let note = C2spCheckpointNoteBodyV1::new(
            "example.com/log",
            1,
            &encode_hex(&[0x00u8; 32]),
            Vec::new(),
        )
        .expect("note");

        let mut signature = [0u8; 64];
        signature[0] = 1;
        let key_id = [0u8; 4];
        let fake = C2spTimestampedEd25519CosignatureV1 {
            witness_name: "witness.example/w1".to_string(),
            key_id,
            timestamp: 0,
            signature,
        };
        assert!(matches!(
            fake.verify(&note, &public_key),
            Err(C2spWireError::KeyIdMismatch)
        ));
    }
}
