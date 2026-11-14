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
use std::collections::{BTreeMap, BTreeSet};
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
            return Err(E::custom(
                "JSON integer exceeds the IEEE-754 safe-integer range",
            ));
        }
        Ok(JsonValue::Integer(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value > MAX_SAFE_JSON_INTEGER {
            return Err(E::custom(
                "JSON integer exceeds the IEEE-754 safe-integer range",
            ));
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
    DigestMismatch { expected: String, observed: String },
    /// A UQ sidecar does not bind to its exact parent receipt or scope.
    UqSidecarBindingMismatch(&'static str),
    /// A UQ sidecar has an invalid measurement/uncertainty/derived-result join.
    UqSidecarLineageMismatch(&'static str),
    /// A required channel or currently valid evidence is missing/incomplete.
    UqSidecarIncompleteEvidence(&'static str),
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

fn integrity_digest(integrity: &[(String, JsonValue)]) -> Result<&str, WireError> {
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
        return Err(WireError::DigestMismatch { expected, observed });
    }

    Ok(ValidatedReceipt {
        protocol_id: protocol_id.to_owned(),
        schema_version: schema_version.to_owned(),
        payload_sha256: observed,
        actuation_authorized: false,
    })
}

const UQ_SIDECAR_PROTOCOL_ID: &str = "LOCO-JOINT-UQ-SIDECAR-001";

/// Verified cross-packet measurement-uncertainty sidecar summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedUqSidecar {
    /// Sidecar protocol identity.
    pub protocol_id: String,
    /// UQ sidecar schema version.
    pub schema_version: String,
    /// Canonical sidecar payload digest.
    pub payload_sha256: String,
    /// Exact parent causal-receipt payload digest.
    pub parent_receipt_payload_sha256: String,
    /// Audit tick used to assess calibration and uncertainty receipt freshness.
    pub evaluation_tick: u64,
    /// Number of predeclared measurement channel requirements.
    pub measurement_requirement_count: usize,
    /// Number of channels marked Required.
    pub required_measurement_count: usize,
    /// Number of channels explicitly marked NotApplicable.
    pub not_applicable_requirement_count: usize,
    /// Number of instrument-backed or derived measurements.
    pub measurement_count: usize,
    /// Number of uncertainty budget / coverage receipts.
    pub uncertainty_receipt_count: usize,
    /// Number of derived-result lineage receipts.
    pub derived_result_count: usize,
    /// Always false; evidence packets do not authorize actuation.
    pub actuation_authorized: bool,
    /// Always false; evidence packets do not certify conformity.
    pub conformity_certified: bool,
}

fn object_field<'a>(
    object: &'a [(String, JsonValue)],
    field: &'static str,
) -> Result<&'a [(String, JsonValue)], WireError> {
    match get_member(object, field) {
        Some(JsonValue::Object(value)) => Ok(value),
        Some(_) => Err(WireError::InvalidField(field)),
        None => Err(WireError::MissingField(field)),
    }
}

fn array_field<'a>(
    object: &'a [(String, JsonValue)],
    field: &'static str,
) -> Result<&'a [JsonValue], WireError> {
    match get_member(object, field) {
        Some(JsonValue::Array(value)) => Ok(value),
        Some(_) => Err(WireError::InvalidField(field)),
        None => Err(WireError::MissingField(field)),
    }
}

fn optional_string<'a>(
    object: &'a [(String, JsonValue)],
    field: &'static str,
) -> Result<Option<&'a str>, WireError> {
    match get_member(object, field) {
        Some(JsonValue::String(value)) => Ok(Some(value)),
        Some(JsonValue::Null) => Ok(None),
        Some(_) => Err(WireError::InvalidField(field)),
        None => Err(WireError::MissingField(field)),
    }
}

fn decimal_u64_field(
    object: &[(String, JsonValue)],
    field: &'static str,
) -> Result<u64, WireError> {
    let value = require_string(object, field)?;
    value.parse::<u64>().map_err(|_| WireError::InvalidField(field))
}

fn require_false(object: &[(String, JsonValue)], field: &'static str) -> Result<(), WireError> {
    match get_member(object, field) {
        Some(JsonValue::Bool(false)) => Ok(()),
        Some(_) => Err(WireError::InvalidField(field)),
        None => Err(WireError::MissingField(field)),
    }
}

fn canonical_value_sha256(value: &JsonValue) -> String {
    let mut canonical = String::new();
    write_jcs_value(value, &mut canonical);
    lowercase_hex(&Sha256::digest(canonical.as_bytes()))
}

fn require_equal_string_fields(
    left: &[(String, JsonValue)],
    left_field: &'static str,
    right: &[(String, JsonValue)],
    right_field: &'static str,
    binding: &'static str,
) -> Result<(), WireError> {
    if require_string(left, left_field)? != require_string(right, right_field)? {
        return Err(WireError::UqSidecarBindingMismatch(binding));
    }
    Ok(())
}

fn required_array_strings(
    object: &[(String, JsonValue)],
    field: &'static str,
) -> Result<BTreeSet<String>, WireError> {
    let values = array_field(object, field)?;
    let mut output = BTreeSet::new();
    for value in values {
        let JsonValue::String(value) = value else {
            return Err(WireError::InvalidField(field));
        };
        if !output.insert(value.clone()) {
            return Err(WireError::UqSidecarLineageMismatch(field));
        }
    }
    Ok(output)
}

/// Verify a UQ sidecar against the exact v0.1 parent causal packet.
///
/// The strict Draft 2020-12 schema and detailed coverage/dependence semantics
/// remain separately checked by the sidecar preflight and the UQ auditor. This
/// Rust join independently verifies duplicate-key-free JCS/SHA-256 integrity,
/// exact parent payload identity, common physical scope, measurement-to-UQ
/// identities, and derived input/output receipt sets.
pub fn validate_uq_sidecar(
    sidecar_input: &[u8],
    parent_input: &[u8],
) -> Result<ValidatedUqSidecar, WireError> {
    let parent_report = validate_receipt(parent_input)?;
    let parent_value = parse_strict_json(parent_input)?;
    let JsonValue::Object(parent_fields) = &parent_value else {
        return Err(WireError::RootMustBeObject);
    };

    let sidecar_value = parse_strict_json(sidecar_input)?;
    let JsonValue::Object(sidecar_fields) = &sidecar_value else {
        return Err(WireError::RootMustBeObject);
    };

    let protocol_id = require_string(sidecar_fields, "protocol_id")?;
    if protocol_id != UQ_SIDECAR_PROTOCOL_ID {
        return Err(WireError::InvalidField("protocol_id"));
    }
    let schema_version = require_string(sidecar_fields, "schema_version")?;
    if schema_version != SCHEMA_VERSION {
        return Err(WireError::InvalidField("schema_version"));
    }
    if require_string(sidecar_fields, "profile_id")?.trim().is_empty()
        || require_string(sidecar_fields, "profile_version")? != SCHEMA_VERSION
    {
        return Err(WireError::InvalidField("profile_id/profile_version"));
    }
    let evaluation_tick = decimal_u64_field(sidecar_fields, "evaluation_tick")?;
    require_false(sidecar_fields, "actuation_authorized")?;
    require_false(sidecar_fields, "conformity_certified")?;

    let integrity = object_field(sidecar_fields, "integrity")?;
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
    let expected_digest = integrity_digest(integrity)?.to_owned();
    let sidecar_payload = canonical_payload_bytes(&sidecar_value)?;
    let observed_digest = lowercase_hex(&Sha256::digest(sidecar_payload));
    if observed_digest != expected_digest {
        return Err(WireError::DigestMismatch {
            expected: expected_digest,
            observed: observed_digest,
        });
    }

    let parent_binding = object_field(sidecar_fields, "parent_binding")?;
    if require_string(parent_binding, "protocol_id")? != PROTOCOL_ID
        || require_string(parent_binding, "schema_version")? != SCHEMA_VERSION
    {
        return Err(WireError::UqSidecarBindingMismatch(
            "parent protocol/version",
        ));
    }
    if require_string(parent_binding, "payload_sha256")? != parent_report.payload_sha256 {
        return Err(WireError::UqSidecarBindingMismatch("parent payload digest"));
    }

    let scope = object_field(sidecar_fields, "scope")?;
    let parent_scope = object_field(parent_fields, "scope")?;
    let parent_configuration = object_field(parent_fields, "configuration")?;
    let parent_events = object_field(parent_fields, "events")?;
    let parent_physical = object_field(parent_events, "physical_transition")?;

    for (sidecar_field, parent_field, label) in [
        ("mission_id", "mission_id", "mission identity"),
        ("robot_id", "robot_id", "robot identity"),
        ("actuator_id", "actuator_id", "actuator identity"),
        (
            "scenario_manifest_digest",
            "scenario_manifest_digest",
            "scenario manifest",
        ),
        ("monotonic_clock_id", "clock_id", "clock identity"),
        ("boot_epoch", "boot_epoch", "boot epoch"),
    ] {
        require_equal_string_fields(scope, sidecar_field, parent_scope, parent_field, label)?;
    }
    for (sidecar_field, parent_field, label) in [
        (
            "actuator_configuration_digest",
            "actuator_configuration_digest",
            "actuator configuration",
        ),
        (
            "controller_configuration_digest",
            "controller_configuration_digest",
            "controller configuration",
        ),
        (
            "physical_boundary_manifest_digest",
            "energy_boundary_manifest_digest",
            "physical boundary",
        ),
    ] {
        require_equal_string_fields(
            scope,
            sidecar_field,
            parent_configuration,
            parent_field,
            label,
        )?;
    }
    require_equal_string_fields(
        scope,
        "physical_state_predecessor_digest",
        parent_physical,
        "reserve_predecessor_digest",
        "physical-state predecessor",
    )?;
    require_equal_string_fields(
        scope,
        "physical_state_result_digest",
        parent_physical,
        "reserve_result_digest",
        "physical-state result",
    )?;

    let scope_value =
        get_member(sidecar_fields, "scope").ok_or(WireError::MissingField("scope"))?;
    let scope_digest = canonical_value_sha256(scope_value);

    let requirements = array_field(sidecar_fields, "measurement_requirements")?;
    let measurements = array_field(sidecar_fields, "measurements")?;
    let receipts = array_field(sidecar_fields, "uncertainty_receipts")?;
    let derived_results = array_field(sidecar_fields, "derived_results")?;
    if measurements.is_empty() || receipts.is_empty() {
        return Err(WireError::UqSidecarLineageMismatch(
            "empty measurements or uncertainty receipts",
        ));
    }

    let mut requirements_by_id: BTreeMap<
        String,
        (&[(String, JsonValue)], &'static str),
    > = BTreeMap::new();
    let mut required_thermal_nodes = BTreeSet::new();
    let mut required_measurement_count = 0_usize;
    let mut not_applicable_requirement_count = 0_usize;
    for requirement_value in requirements {
        let JsonValue::Object(requirement) = requirement_value else {
            return Err(WireError::InvalidField("measurement_requirements"));
        };
        let measurement_id = require_string(requirement, "measurement_id")?.to_owned();
        let applicability = object_field(requirement, "applicability")?;
        let applicability_kind = require_string(applicability, "kind")?;
        let kind = require_string(requirement, "kind")?;
        let node_id = optional_string(requirement, "node_id")?;
        if kind == "thermal_node" {
            let node = node_id.ok_or(WireError::InvalidField("measurement_requirements.node_id"))?;
            if applicability_kind == "required" {
                required_thermal_nodes.insert(node.to_owned());
            }
        } else if node_id.is_some() {
            return Err(WireError::InvalidField("measurement_requirements.node_id"));
        }
        let stored_kind = match applicability_kind {
            "required" => {
                required_measurement_count += 1;
                "required"
            }
            "not_applicable" => {
                not_applicable_requirement_count += 1;
                if require_string(applicability, "reason")?.trim().is_empty() {
                    return Err(WireError::InvalidField("measurement_requirements.reason"));
                }
                "not_applicable"
            }
            _ => return Err(WireError::InvalidField("measurement_requirements.applicability")),
        };
        if requirements_by_id
            .insert(measurement_id, (requirement, stored_kind))
            .is_some()
        {
            return Err(WireError::UqSidecarLineageMismatch(
                "duplicate measurement requirement identity",
            ));
        }
    }
    if requirements_by_id.is_empty() {
        return Err(WireError::UqSidecarIncompleteEvidence(
            "measurement requirements missing",
        ));
    }
    let parent_thermal = object_field(parent_fields, "thermal")?;
    let parent_required_thermal_nodes =
        required_array_strings(parent_thermal, "required_node_ids")?;
    let sidecar_required_thermal_nodes = required_thermal_nodes
        .iter()
        .map(|node| (*node).to_owned())
        .collect::<BTreeSet<_>>();
    if sidecar_required_thermal_nodes != parent_required_thermal_nodes {
        return Err(WireError::UqSidecarBindingMismatch(
            "required thermal node set",
        ));
    }

    let mut measurements_by_id: BTreeMap<String, &[(String, JsonValue)]> = BTreeMap::new();
    for measurement_value in measurements {
        let JsonValue::Object(measurement) = measurement_value else {
            return Err(WireError::InvalidField("measurements"));
        };
        let measurement_id = require_string(measurement, "measurement_id")?.to_owned();
        if measurements_by_id
            .insert(measurement_id, measurement)
            .is_some()
        {
            return Err(WireError::UqSidecarLineageMismatch(
                "duplicate measurement identity",
            ));
        }
    }

    let mut required_measurement_ids = BTreeSet::new();
    for (measurement_id, (requirement, applicability)) in &requirements_by_id {
        match *applicability {
            "required" => {
                required_measurement_ids.insert(measurement_id.clone());
                let measurement = measurements_by_id.get(measurement_id).ok_or(
                    WireError::UqSidecarIncompleteEvidence("required channel missing"),
                )?;
                require_equal_string_fields(
                    requirement,
                    "kind",
                    measurement,
                    "kind",
                    "required measurement kind",
                )?;
                if optional_string(requirement, "node_id")?
                    != optional_string(measurement, "node_id")?
                {
                    return Err(WireError::UqSidecarBindingMismatch(
                        "required measurement node identity",
                    ));
                }
            }
            "not_applicable" => {
                if measurements_by_id.contains_key(measurement_id) {
                    return Err(WireError::UqSidecarLineageMismatch(
                        "NotApplicable channel supplied",
                    ));
                }
            }
            _ => {
                return Err(WireError::InvalidField(
                    "measurement_requirements.applicability",
                ));
            }
        }
    }
    let observed_measurement_ids = measurements_by_id.keys().cloned().collect::<BTreeSet<_>>();
    if observed_measurement_ids != required_measurement_ids {
        return Err(WireError::UqSidecarLineageMismatch(
            "measurement set differs from Required channel set",
        ));
    }

    let mut receipts_by_id: BTreeMap<String, &[(String, JsonValue)]> = BTreeMap::new();
    for receipt_value in receipts {
        let JsonValue::Object(receipt) = receipt_value else {
            return Err(WireError::InvalidField("uncertainty_receipts"));
        };
        let receipt_id = require_string(receipt, "receipt_id")?.to_owned();
        if receipts_by_id.insert(receipt_id, receipt).is_some() {
            return Err(WireError::UqSidecarLineageMismatch(
                "duplicate uncertainty receipt identity",
            ));
        }
        if require_string(receipt, "scope_digest")? != scope_digest {
            return Err(WireError::UqSidecarBindingMismatch(
                "uncertainty receipt common scope digest",
            ));
        }
    }

    for (measurement_id, measurement) in &measurements_by_id {
        let receipt_id = require_string(measurement, "uncertainty_receipt_id")?;
        let receipt = receipts_by_id
            .get(receipt_id)
            .ok_or(WireError::UqSidecarLineageMismatch(
                "measurement uncertainty receipt missing",
            ))?;
        if require_string(receipt, "measurement_id")? != measurement_id {
            return Err(WireError::UqSidecarLineageMismatch(
                "measurement/uncertainty identity mismatch",
            ));
        }
        let start_tick = decimal_u64_field(measurement, "start_tick")?;
        let end_tick = decimal_u64_field(measurement, "end_tick")?;
        let calibration_from = decimal_u64_field(measurement, "calibration_valid_from_tick")?;
        let calibration_through = decimal_u64_field(measurement, "calibration_valid_through_tick")?;
        let receipt_from = decimal_u64_field(receipt, "valid_from_tick")?;
        let receipt_through = decimal_u64_field(receipt, "valid_through_tick")?;
        if start_tick > end_tick
            || calibration_from > calibration_through
            || receipt_from > receipt_through
        {
            return Err(WireError::UqSidecarLineageMismatch(
                "measurement/calibration/uncertainty interval inverted",
            ));
        }
        if calibration_from > start_tick || calibration_through < end_tick {
            return Err(WireError::UqSidecarBindingMismatch(
                "calibration does not cover measurement window",
            ));
        }
        if receipt_from > start_tick || receipt_through < end_tick {
            return Err(WireError::UqSidecarBindingMismatch(
                "uncertainty receipt does not cover measurement window",
            ));
        }
        if evaluation_tick < calibration_from || evaluation_tick > calibration_through {
            return Err(WireError::UqSidecarIncompleteEvidence(
                "calibration not valid at evaluation tick",
            ));
        }
        if evaluation_tick < receipt_from || evaluation_tick > receipt_through {
            return Err(WireError::UqSidecarIncompleteEvidence(
                "uncertainty receipt not valid at evaluation tick",
            ));
        }
        for field in [
            "measurement_artifact_digest",
            "instrument_configuration_digest",
            "calibration_receipt_digest",
            "unit_code",
        ] {
            require_equal_string_fields(
                measurement,
                field,
                receipt,
                field,
                "measurement/uncertainty receipt binding",
            )?;
        }
        match (
            get_member(measurement, "decimal_exponent"),
            get_member(receipt, "decimal_exponent"),
        ) {
            (Some(JsonValue::Integer(left)), Some(JsonValue::Integer(right))) if left == right => {}
            _ => {
                return Err(WireError::UqSidecarBindingMismatch(
                    "measurement/uncertainty decimal scale",
                ));
            }
        }
    }

    for derived_value in derived_results {
        let JsonValue::Object(derived) = derived_value else {
            return Err(WireError::InvalidField("derived_results"));
        };
        let result_id = require_string(derived, "result_id")?;
        let output_id = require_string(derived, "output_measurement_id")?;
        if result_id != output_id {
            return Err(WireError::UqSidecarLineageMismatch(
                "derived result/output measurement identity",
            ));
        }
        let output =
            measurements_by_id
                .get(output_id)
                .ok_or(WireError::UqSidecarLineageMismatch(
                    "derived output measurement missing",
                ))?;
        require_equal_string_fields(
            derived,
            "output_result_digest",
            output,
            "measurement_artifact_digest",
            "derived output artifact digest",
        )?;
        require_equal_string_fields(
            derived,
            "output_uncertainty_receipt_id",
            output,
            "uncertainty_receipt_id",
            "derived output uncertainty receipt",
        )?;
        let input_measurements = required_array_strings(derived, "input_measurement_ids")?;
        let supplied_input_receipts =
            required_array_strings(derived, "input_uncertainty_receipt_ids")?;
        let mut expected_input_receipts = BTreeSet::new();
        for input_id in input_measurements {
            let input =
                measurements_by_id
                    .get(&input_id)
                    .ok_or(WireError::UqSidecarLineageMismatch(
                        "derived input measurement missing",
                    ))?;
            expected_input_receipts
                .insert(require_string(input, "uncertainty_receipt_id")?.to_owned());
        }
        if supplied_input_receipts != expected_input_receipts {
            return Err(WireError::UqSidecarLineageMismatch(
                "derived input uncertainty receipt set",
            ));
        }
        if supplied_input_receipts
            .iter()
            .any(|receipt_id| !receipts_by_id.contains_key(receipt_id))
        {
            return Err(WireError::UqSidecarLineageMismatch(
                "derived input uncertainty receipt missing",
            ));
        }
        let output_receipt = require_string(derived, "output_uncertainty_receipt_id")?;
        if !receipts_by_id.contains_key(output_receipt) {
            return Err(WireError::UqSidecarLineageMismatch(
                "derived output uncertainty receipt missing",
            ));
        }
    }

    Ok(ValidatedUqSidecar {
        protocol_id: protocol_id.to_owned(),
        schema_version: schema_version.to_owned(),
        payload_sha256: observed_digest,
        parent_receipt_payload_sha256: parent_report.payload_sha256,
        evaluation_tick,
        measurement_requirement_count: requirements.len(),
        required_measurement_count,
        not_applicable_requirement_count,
        measurement_count: measurements.len(),
        uncertainty_receipt_count: receipts.len(),
        derived_result_count: derived_results.len(),
        actuation_authorized: false,
        conformity_certified: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOLDEN_PACKET: &str = include_str!("../../fixtures/joint_receipt_v0_1_complete.json");
    const GOLDEN_DIGEST: &str = "cb19e0327c2f47c895eae4e49a89c9f8f9c9918e080d52b201d98b1a794823b8";
    const GOLDEN_UQ_SIDECAR: &str =
        include_str!("../../fixtures/joint_uq_sidecar_v0_1_complete.json");
    const GOLDEN_UQ_DIGEST: &str =
        "dcb4672f36b37d18e69c8cb5f1f750dfcf0b506cd1bd0831e21eb2f6c9dc5dd3";

    fn resign_uq_sidecar(input: &str) -> String {
        let placeholder = "0000000000000000000000000000000000000000000000000000000000000000";
        let provisional = input.replace(GOLDEN_UQ_DIGEST, placeholder);
        let digest = payload_sha256(provisional.as_bytes()).unwrap();
        provisional.replace(placeholder, &digest)
    }

    #[test]
    fn synthetic_uq_sidecar_is_integrity_verified_and_parent_bound() {
        let report =
            validate_uq_sidecar(GOLDEN_UQ_SIDECAR.as_bytes(), GOLDEN_PACKET.as_bytes()).unwrap();
        assert_eq!(report.protocol_id, UQ_SIDECAR_PROTOCOL_ID);
        assert_eq!(report.schema_version, SCHEMA_VERSION);
        assert_eq!(report.payload_sha256, GOLDEN_UQ_DIGEST);
        assert_eq!(report.parent_receipt_payload_sha256, GOLDEN_DIGEST);
        assert_eq!(report.measurement_count, 3);
        assert_eq!(report.uncertainty_receipt_count, 3);
        assert_eq!(report.derived_result_count, 1);
        assert!(!report.actuation_authorized);
        assert!(!report.conformity_certified);
    }

    #[test]
    fn uq_sidecar_parent_payload_mismatch_is_rejected_after_resealing() {
        let changed = GOLDEN_UQ_SIDECAR.replace(
            GOLDEN_DIGEST,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        let changed = resign_uq_sidecar(&changed);
        assert!(matches!(
            validate_uq_sidecar(changed.as_bytes(), GOLDEN_PACKET.as_bytes()),
            Err(WireError::UqSidecarBindingMismatch("parent payload digest"))
        ));
    }

    #[test]
    fn uq_sidecar_actuator_scope_drift_is_rejected_after_resealing() {
        let changed = GOLDEN_UQ_SIDECAR.replace(
            "\"actuator_id\": \"hip-flexion-left\"",
            "\"actuator_id\": \"hip-flexion-other\"",
        );
        let changed = resign_uq_sidecar(&changed);
        assert!(matches!(
            validate_uq_sidecar(changed.as_bytes(), GOLDEN_PACKET.as_bytes()),
            Err(WireError::UqSidecarBindingMismatch("actuator identity"))
        ));
    }

    #[test]
    fn uq_sidecar_physical_state_predecessor_drift_is_rejected_after_resealing() {
        let changed = GOLDEN_UQ_SIDECAR.replace(
            "6af5552cd30ac3a4aa8a44ccb39d502d5294aff2e59777793a69e51b302feb32",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        let changed = resign_uq_sidecar(&changed);
        assert!(matches!(
            validate_uq_sidecar(changed.as_bytes(), GOLDEN_PACKET.as_bytes()),
            Err(WireError::UqSidecarBindingMismatch(
                "physical-state predecessor"
            ))
        ));
    }

    #[test]
    fn uq_sidecar_derived_input_uncertainty_omission_is_rejected_after_resealing() {
        let changed = GOLDEN_UQ_SIDECAR.replace(
            "\"input_uncertainty_receipt_ids\": [\n        \"uq-torque-001\",\n        \"uq-voltage-001\"\n      ]",
            "\"input_uncertainty_receipt_ids\": [\n        \"uq-torque-001\"\n      ]",
        );
        assert_ne!(changed, GOLDEN_UQ_SIDECAR);
        let changed = resign_uq_sidecar(&changed);
        assert!(matches!(
            validate_uq_sidecar(changed.as_bytes(), GOLDEN_PACKET.as_bytes()),
            Err(WireError::UqSidecarLineageMismatch(
                "derived input uncertainty receipt set"
            ))
        ));
    }

    #[test]
    fn uq_sidecar_measurement_artifact_must_match_its_uncertainty_receipt() {
        let changed = GOLDEN_UQ_SIDECAR.replacen(
            "98134ec5203d4c75386c1088adea2e4fe960c3ef935076c840c7a1749c953527",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            1,
        );
        let changed = resign_uq_sidecar(&changed);
        assert!(matches!(
            validate_uq_sidecar(changed.as_bytes(), GOLDEN_PACKET.as_bytes()),
            Err(WireError::UqSidecarBindingMismatch(
                "measurement/uncertainty receipt binding"
            ))
        ));
    }

    #[test]
    fn uq_sidecar_actuation_authority_true_is_never_accepted() {
        let changed = GOLDEN_UQ_SIDECAR.replace(
            "\"actuation_authorized\": false",
            "\"actuation_authorized\": true",
        );
        let changed = resign_uq_sidecar(&changed);
        assert!(matches!(
            validate_uq_sidecar(changed.as_bytes(), GOLDEN_PACKET.as_bytes()),
            Err(WireError::InvalidField("actuation_authorized"))
        ));
    }

    #[test]
    fn uq_sidecar_digest_tampering_is_rejected() {
        let changed = GOLDEN_UQ_SIDECAR.replace(
            GOLDEN_UQ_DIGEST,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        assert!(matches!(
            validate_uq_sidecar(changed.as_bytes(), GOLDEN_PACKET.as_bytes()),
            Err(WireError::DigestMismatch { .. })
        ));
    }

    #[test]
    fn uq_sidecar_duplicate_nested_json_key_is_rejected() {
        let changed = GOLDEN_UQ_SIDECAR.replacen(
            "\"measurement_id\": \"torque-001\"",
            "\"measurement_id\": \"torque-001\", \"measurement_id\": \"forged\"",
            1,
        );
        assert!(matches!(
            validate_uq_sidecar(changed.as_bytes(), GOLDEN_PACKET.as_bytes()),
            Err(WireError::InvalidJson(_))
        ));
    }

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
        let changed = GOLDEN_PACKET.replace(
            "\"schema_version\":\"0.1.0\"",
            "\"schema_version\":\"0.2.0\"",
        );
        assert!(matches!(
            validate_receipt(changed.as_bytes()),
            Err(WireError::InvalidField("schema_version")) | Err(WireError::DigestMismatch { .. })
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
