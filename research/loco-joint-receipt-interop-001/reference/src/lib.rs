//! Strict JSON/JCS integrity verification for LOCO-JOINT-RECEIPT-INTEROP-001 v0.1.
//!
//! This module verifies duplicate-key-free JSON, the v0.1 no-float/safe-integer
//! profile, RFC 8785 canonical payload bytes, the SHA-256 known-answer digest,
//! and the no-actuation-authority invariant. Full Draft 2020-12 schema
//! validation remains a separate gate in the parent interop workflow.

#![forbid(unsafe_code)]

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt;

const MAX_SAFE_JSON_INTEGER: u64 = 9_007_199_254_740_991;
const PROTOCOL_ID: &str = "LOCO-JOINT-RECEIPT-INTEROP-001";
const SCHEMA_VERSION: &str = "0.1.0";
const CANONICALIZATION: &str = "RFC8785-JCS-UTF8";
const DIGEST_SCOPE: &str = "top_level_without_integrity";

/// Strict JSON value for the v0.1 profile.
///
/// Object members are retained as pairs so canonical output can sort by UTF-16
/// code units exactly as required by JCS. Duplicate names are rejected while
/// parsing, before any object representation could overwrite them.
#[derive(Debug, Clone, PartialEq, Eq)]
enum JsonValue {
    Null,
    Bool(bool),
    Integer(i64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

struct StrictJsonVisitor;

impl<'de> Visitor<'de> for StrictJsonVisitor {
    type Value = JsonValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("strict v0.1 JSON with unique keys, safe integers and no floats")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(JsonValue::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(JsonValue::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(JsonValue::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value.unsigned_abs() > MAX_SAFE_JSON_INTEGER {
            return Err(E::custom("JSON integer exceeds the IEEE-754 safe-integer range"));
        }
        Ok(JsonValue::Integer(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value > MAX_SAFE_JSON_INTEGER {
            return Err(E::custom("JSON integer exceeds the IEEE-754 safe-integer range"));
        }
        Ok(JsonValue::Integer(value as i64))
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Err(E::custom(
            "floating-point JSON numbers are forbidden by protocol v0.1",
        ))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(JsonValue::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(JsonValue::String(value))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<JsonValue>()? {
            values.push(value);
        }
        Ok(JsonValue::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut seen = BTreeSet::new();
        let mut fields = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            if !seen.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object key: {key}"
                )));
            }
            let value = map.next_value::<JsonValue>()?;
            fields.push((key, value));
        }
        Ok(JsonValue::Object(fields))
    }
}

impl<'de> Deserialize<'de> for JsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictJsonVisitor)
    }
}

/// Validated protocol envelope summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedReceipt {
    /// Protocol identity.
    pub protocol_id: String,
    /// Exact wire schema version.
    pub schema_version: String,
    /// Verified SHA-256 over the top-level object without the integrity member.
    pub payload_sha256: String,
    /// The protocol can never authorize actuation.
    pub actuation_authorized: bool,
}

/// Structural or integrity failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WireError {
    /// JSON syntax, duplicate-key, float or safe-integer parsing failure.
    InvalidJson(String),
    /// Root must be a JSON object.
    RootMustBeObject,
    /// Required top-level field is missing.
    MissingField(&'static str),
    /// A required field has the wrong JSON type or value.
    InvalidField(&'static str),
    /// Integrity object is malformed.
    InvalidIntegrityField(&'static str),
    /// Claimed and recomputed payload digests differ.
    DigestMismatch {
        expected: String,
        observed: String,
    },
}

impl fmt::Display for WireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for WireError {}

fn parse_strict_json(input: &[u8]) -> Result<JsonValue, WireError> {
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let value = JsonValue::deserialize(&mut deserializer)
        .map_err(|error| WireError::InvalidJson(error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| WireError::InvalidJson(error.to_string()))?;
    Ok(value)
}

fn get_member<'a>(object: &'a [(String, JsonValue)], name: &str) -> Option<&'a JsonValue> {
    object
        .iter()
        .find_map(|(key, value)| (key == name).then_some(value))
}

fn require_string<'a>(
    object: &'a [(String, JsonValue)],
    name: &'static str,
) -> Result<&'a str, WireError> {
    match get_member(object, name) {
        Some(JsonValue::String(value)) => Ok(value),
        Some(_) => Err(WireError::InvalidField(name)),
        None => Err(WireError::MissingField(name)),
    }
}

fn utf16_key_order(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

fn write_jcs_string(value: &str, output: &mut String) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{0009}' => output.push_str("\\t"),
            '\u{000a}' => output.push_str("\\n"),
            '\u{000c}' => output.push_str("\\f"),
            '\u{000d}' => output.push_str("\\r"),
            character if character <= '\u{001f}' => {
                use fmt::Write as _;
                let _ = write!(output, "\\u{:04x}", character as u32);
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

fn write_jcs_value(value: &JsonValue, output: &mut String) {
    match value {
        JsonValue::Null => output.push_str("null"),
        JsonValue::Bool(false) => output.push_str("false"),
        JsonValue::Bool(true) => output.push_str("true"),
        JsonValue::Integer(value) => output.push_str(&value.to_string()),
        JsonValue::String(value) => write_jcs_string(value, output),
        JsonValue::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_jcs_value(value, output);
            }
            output.push(']');
        }
        JsonValue::Object(fields) => {
            let mut sorted = fields.iter().collect::<Vec<_>>();
            sorted.sort_by(|(left, _), (right, _)| utf16_key_order(left, right));
            output.push('{');
            for (index, (key, value)) in sorted.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_jcs_string(key, output);
                output.push(':');
                write_jcs_value(value, output);
            }
            output.push('}');
        }
    }
}

/// Canonicalize strict v0.1 JSON according to the supported RFC 8785 profile.
///
/// This accepts objects, arrays, strings, booleans, null and safe JSON integers.
/// It rejects duplicate names, floats, non-safe integer numbers and trailing data.
pub fn canonicalize_json(input: &[u8]) -> Result<Vec<u8>, WireError> {
    let value = parse_strict_json(input)?;
    let mut output = String::new();
    write_jcs_value(&value, &mut output);
    Ok(output.into_bytes())
}

fn canonical_payload_bytes(value: &JsonValue) -> Result<Vec<u8>, WireError> {
    let JsonValue::Object(fields) = value else {
        return Err(WireError::RootMustBeObject);
    };
    if get_member(fields, "integrity").is_none() {
        return Err(WireError::MissingField("integrity"));
    }
    let payload = JsonValue::Object(
        fields
            .iter()
            .filter(|(key, _)| key != "integrity")
            .cloned()
            .collect(),
    );
    let mut output = String::new();
    write_jcs_value(&payload, &mut output);
    Ok(output.into_bytes())
}

/// Compute SHA-256 over the RFC 8785 canonical top-level object without integrity.
pub fn payload_sha256(input: &[u8]) -> Result<String, WireError> {
    let value = parse_strict_json(input)?;
    let payload = canonical_payload_bytes(&value)?;
    let digest = Sha256::digest(payload);
    Ok(lowercase_hex(&digest))
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn integrity_digest<'a>(integrity: &'a [(String, JsonValue)]) -> Result<&'a str, WireError> {
    let digest = require_string(integrity, "payload_sha256")
        .map_err(|_| WireError::InvalidIntegrityField("payload_sha256"))?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(WireError::InvalidIntegrityField("payload_sha256"));
    }
    Ok(digest)
}

/// Parse and verify the integrity envelope and no-actuation-authority invariant.
///
/// This is not a complete JSON Schema validator. The parent workflow continues
/// to validate the closed Draft 2020-12 schema and the entire mutation corpus.
pub fn validate_receipt(input: &[u8]) -> Result<ValidatedReceipt, WireError> {
    let value = parse_strict_json(input)?;
    let JsonValue::Object(fields) = &value else {
        return Err(WireError::RootMustBeObject);
    };

    let protocol_id = require_string(fields, "protocol_id")?;
    if protocol_id != PROTOCOL_ID {
        return Err(WireError::InvalidField("protocol_id"));
    }
    let schema_version = require_string(fields, "schema_version")?;
    if schema_version != SCHEMA_VERSION {
        return Err(WireError::InvalidField("schema_version"));
    }
    match get_member(fields, "actuation_authorized") {
        Some(JsonValue::Bool(false)) => {}
        Some(_) => return Err(WireError::InvalidField("actuation_authorized")),
        None => return Err(WireError::MissingField("actuation_authorized")),
    }

    let integrity = match get_member(fields, "integrity") {
        Some(JsonValue::Object(value)) => value,
        Some(_) => return Err(WireError::InvalidIntegrityField("integrity")),
        None => return Err(WireError::MissingField("integrity")),
    };
    if require_string(integrity, "canonicalization")
        .map_err(|_| WireError::InvalidIntegrityField("canonicalization"))?
        != CANONICALIZATION
    {
        return Err(WireError::InvalidIntegrityField("canonicalization"));
    }
    if require_string(integrity, "digest_scope")
        .map_err(|_| WireError::InvalidIntegrityField("digest_scope"))?
        != DIGEST_SCOPE
    {
        return Err(WireError::InvalidIntegrityField("digest_scope"));
    }
    let expected = integrity_digest(integrity)?.to_owned();
    let payload = canonical_payload_bytes(&value)?;
    let observed = lowercase_hex(&Sha256::digest(payload));
    if observed != expected {
        return Err(WireError::DigestMismatch {
            expected,
            observed,
        });
    }

    Ok(ValidatedReceipt {
        protocol_id: protocol_id.to_owned(),
        schema_version: schema_version.to_owned(),
        payload_sha256: observed,
        actuation_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOLDEN_PACKET: &str =
        include_str!("../../fixtures/joint_receipt_v0_1_complete.json");
    const GOLDEN_DIGEST: &str =
        "cb19e0327c2f47c895eae4e49a89c9f8f9c9918e080d52b201d98b1a794823b8";

    #[test]
    fn synthetic_golden_receipt_matches_rfc8785_sha256_known_answer() {
        let report = validate_receipt(GOLDEN_PACKET.as_bytes()).unwrap();
        assert_eq!(report.protocol_id, PROTOCOL_ID);
        assert_eq!(report.schema_version, SCHEMA_VERSION);
        assert_eq!(report.payload_sha256, GOLDEN_DIGEST);
        assert!(!report.actuation_authorized);
    }

    #[test]
    fn payload_digest_is_independent_of_integrity_member_contents() {
        let original = GOLDEN_PACKET;
        let changed_integrity = original.replace(
            GOLDEN_DIGEST,
            "0000000000000000000000000000000000000000000000000000000000000000",
        );
        assert_eq!(
            payload_sha256(original.as_bytes()).unwrap(),
            payload_sha256(changed_integrity.as_bytes()).unwrap()
        );
        assert!(matches!(
            validate_receipt(changed_integrity.as_bytes()),
            Err(WireError::DigestMismatch { .. })
        ));
    }

    #[test]
    fn payload_mutation_is_detected() {
        let changed = GOLDEN_PACKET.replace("robot-a1-synthetic-01", "robot-a1-synthetic-02");
        assert!(matches!(
            validate_receipt(changed.as_bytes()),
            Err(WireError::DigestMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_top_level_json_key_is_rejected() {
        let duplicated = GOLDEN_PACKET.replacen(
            "{\"schema_version\":",
            "{\"protocol_id\":\"spoofed\",\"schema_version\":",
            1,
        );
        assert!(matches!(
            validate_receipt(duplicated.as_bytes()),
            Err(WireError::InvalidJson(_))
        ));
    }

    #[test]
    fn duplicate_nested_json_key_is_rejected() {
        let duplicated = GOLDEN_PACKET.replacen(
            "\"mission_id\":\"jm-golden-001\"",
            "\"mission_id\":\"jm-golden-001\",\"mission_id\":\"attacker-value\"",
            1,
        );
        assert!(matches!(
            validate_receipt(duplicated.as_bytes()),
            Err(WireError::InvalidJson(_))
        ));
    }

    #[test]
    fn floating_point_values_are_rejected_by_v0_1_profile() {
        assert!(matches!(
            canonicalize_json(br#"{"sample_rate":2000.0}"#),
            Err(WireError::InvalidJson(_))
        ));
        assert!(matches!(
            canonicalize_json(br#"{"sample_rate":2e3}"#),
            Err(WireError::InvalidJson(_))
        ));
    }

    #[test]
    fn numbers_outside_json_safe_integer_range_are_rejected() {
        assert!(matches!(
            canonicalize_json(br#"{"unsafe":9007199254740992}"#),
            Err(WireError::InvalidJson(_))
        ));
        assert!(matches!(
            canonicalize_json(br#"{"unsafe":-9007199254740992}"#),
            Err(WireError::InvalidJson(_))
        ));
    }

    #[test]
    fn object_key_order_is_canonical_and_utf16_ordered() {
        let first = canonicalize_json("{\"z\":1,\"a\":2,\"𐀀\":3,\"\":4}".as_bytes()).unwrap();
        let second = canonicalize_json("{\"\":4,\"𐀀\":3,\"a\":2,\"z\":1}".as_bytes()).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            String::from_utf8(first).unwrap(),
            "{\"a\":2,\"z\":1,\"𐀀\":3,\"\":4}"
        );
    }

    #[test]
    fn jcs_string_escaping_uses_short_control_escapes() {
        let output = canonicalize_json("{\"value\":\"\\b\\t\\n\\f\\r\"}".as_bytes()).unwrap();
        assert_eq!(output, br#"{"value":"\b\t\n\f\r"}"#);
    }

    #[test]
    fn actuation_authority_true_is_never_accepted() {
        let changed = GOLDEN_PACKET.replace(
            "\"actuation_authorized\":false",
            "\"actuation_authorized\":true",
        );
        assert!(matches!(
            validate_receipt(changed.as_bytes()),
            Err(WireError::InvalidField("actuation_authorized"))
                | Err(WireError::DigestMismatch { .. })
        ));
    }

    #[test]
    fn protocol_version_mismatch_is_rejected() {
        let changed = GOLDEN_PACKET.replace("\"schema_version\":\"0.1.0\"", "\"schema_version\":\"0.2.0\"");
        assert!(matches!(
            validate_receipt(changed.as_bytes()),
            Err(WireError::InvalidField("schema_version"))
                | Err(WireError::DigestMismatch { .. })
        ));
    }

    #[test]
    fn malformed_or_uppercase_digest_is_rejected() {
        let changed = GOLDEN_PACKET.replace(
            GOLDEN_DIGEST,
            "CB19E0327C2F47C895EAE4E49A89C9F8F9C9918E080D52B201D98B1A794823B8",
        );
        assert!(matches!(
            validate_receipt(changed.as_bytes()),
            Err(WireError::InvalidIntegrityField("payload_sha256"))
        ));
    }
}
