#![forbid(unsafe_code)]

//! Offline verifier for construction robot sealed-trace v0.1.
//!
//! It distinguishes internal digest consistency, externally supplied root
//! anchoring, and capture completeness. It never infers sender authentication,
//! broker completeness, safety authorization, or construction acceptance.

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const EVENT_DOMAIN: &[u8] = b"construction-robot-trace-event-v0.1\0";
const ROOT_DOMAIN: &[u8] = b"construction-robot-trace-root-v0.1\0";
const ANNOTATION_DOMAIN: &[u8] = b"construction-robot-trace-annotation-v0.1\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    pub integrity: &'static str,
    pub completeness: &'static str,
    pub external_anchor: &'static str,
    pub trace_id: String,
    pub event_count: usize,
    pub verified_payload_bytes: u64,
    pub final_trace_digest: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationError {
    Json(String),
    DuplicateKey(String),
    Shape(String),
    DigestMismatch(String),
    AnchorMismatch { expected: String, actual: String },
    UnsupportedNumber,
    InvalidBase64(String),
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message) => write!(f, "invalid JSON: {message}"),
            Self::DuplicateKey(key) => write!(f, "duplicate JSON object key: {key}"),
            Self::Shape(message) => write!(f, "invalid trace shape: {message}"),
            Self::DigestMismatch(message) => write!(f, "digest verification failed: {message}"),
            Self::AnchorMismatch { expected, actual } => {
                write!(
                    f,
                    "external root anchor mismatch: expected {expected}, got {actual}"
                )
            }
            Self::UnsupportedNumber => write!(
                f,
                "trace JCS profile accepts only safe JSON integers, not floats or unsafe integers"
            ),
            Self::InvalidBase64(message) => write!(f, "invalid base64 payload: {message}"),
        }
    }
}

impl std::error::Error for VerificationError {}

/// Verify a trace. An expected root must arrive through an independent trusted
/// channel; copying the digest from the trace does not create an anchor.
pub fn verify_trace_bytes(
    input: &[u8],
    expected_root_anchor: Option<&str>,
) -> Result<VerificationReport, VerificationError> {
    if input.len() > 256 * 1024 * 1024 {
        return shape("trace exceeds the 256 MiB verifier limit");
    }

    let root = parse_unique_json(input)?;
    let top = as_object(&root, "trace")?;
    require_exact_keys(
        top,
        &["manifest", "events", "annotations", "final_trace_digest"],
        &[],
        "trace",
    )?;

    let manifest = field(top, "manifest")?;
    let manifest_obj = as_object(manifest, "manifest")?;
    require_exact_keys(
        manifest_obj,
        &["schema_version", "trace_id", "capture", "seal"],
        &[],
        "manifest",
    )?;
    expect_string(
        field(manifest_obj, "schema_version")?,
        "manifest.schema_version",
        "0.1.0",
    )?;
    let trace_id =
        non_empty_string(field(manifest_obj, "trace_id")?, "manifest.trace_id")?.to_owned();

    let capture_obj = as_object(field(manifest_obj, "capture")?, "manifest.capture")?;
    require_exact_keys(
        capture_obj,
        &[
            "tool",
            "tool_version",
            "configuration_digest",
            "started_at",
            "ended_at",
            "protocol_hints",
            "capture_completeness",
            "recorder_drop_count",
            "broker_drop_count",
            "broker_drop_evidence",
            "filters_applied",
            "filter_profile_digest",
            "redaction_applied",
            "parent_trace_digest",
            "transformation_digest",
            "first_sequence",
            "last_sequence",
            "event_count",
            "capture_boundary_evidence_ref",
        ],
        &[],
        "manifest.capture",
    )?;

    for key in ["tool", "tool_version"] {
        non_empty_string(field(capture_obj, key)?, key)?;
    }
    for key in ["started_at", "ended_at"] {
        let timestamp = non_empty_string(field(capture_obj, key)?, key)?;
        if !is_rfc3339(timestamp) {
            return shape(&format!("{key} must be an RFC3339 timestamp"));
        }
    }
    require_digest(
        field(capture_obj, "configuration_digest")?,
        "configuration_digest",
    )?;
    for hint in as_array(field(capture_obj, "protocol_hints")?, "protocol_hints")? {
        non_empty_string(hint, "protocol_hints[]")?;
    }

    let completeness_value = non_empty_string(
        field(capture_obj, "capture_completeness")?,
        "capture_completeness",
    )?;
    let recorder_drop_count = non_negative_integer(
        field(capture_obj, "recorder_drop_count")?,
        "recorder_drop_count",
    )?;
    let broker_drop_count = nullable_non_negative_integer(
        field(capture_obj, "broker_drop_count")?,
        "broker_drop_count",
    )?;
    let broker_drop_evidence = non_empty_string(
        field(capture_obj, "broker_drop_evidence")?,
        "broker_drop_evidence",
    )?;
    match broker_drop_evidence {
        "measured_zero" if broker_drop_count != Some(0) => {
            return shape("measured_zero broker evidence requires count 0");
        }
        "measured_nonzero" if !matches!(broker_drop_count, Some(value) if value > 0) => {
            return shape("measured_nonzero broker evidence requires count greater than 0");
        }
        "not_available" if broker_drop_count.is_some() => {
            return shape("not_available broker evidence requires a null count");
        }
        "measured_zero" | "measured_nonzero" | "not_available" => {}
        _ => return shape("unknown broker_drop_evidence value"),
    }

    let filters_applied = as_bool(field(capture_obj, "filters_applied")?, "filters_applied")?;
    let filter_digest = field(capture_obj, "filter_profile_digest")?;
    if filters_applied || !filter_digest.is_null() {
        require_digest(filter_digest, "filter_profile_digest")?;
    }

    let redaction_applied = as_bool(
        field(capture_obj, "redaction_applied")?,
        "redaction_applied",
    )?;
    let parent_digest = field(capture_obj, "parent_trace_digest")?;
    let transformation_digest = field(capture_obj, "transformation_digest")?;
    if redaction_applied {
        require_digest(parent_digest, "parent_trace_digest")?;
        require_digest(transformation_digest, "transformation_digest")?;
    } else {
        if !parent_digest.is_null() {
            require_digest(parent_digest, "parent_trace_digest")?;
        }
        if !transformation_digest.is_null() {
            require_digest(transformation_digest, "transformation_digest")?;
        }
    }

    let first_sequence =
        non_negative_integer(field(capture_obj, "first_sequence")?, "first_sequence")?;
    let last_sequence =
        non_negative_integer(field(capture_obj, "last_sequence")?, "last_sequence")?;
    let declared_event_count =
        non_negative_integer(field(capture_obj, "event_count")?, "event_count")?;
    let boundary_ref = field(capture_obj, "capture_boundary_evidence_ref")?;
    let boundary_ref_present = match boundary_ref {
        Value::String(value) => !value.trim().is_empty(),
        Value::Null => false,
        _ => return shape("capture_boundary_evidence_ref must be string or null"),
    };

    let seal_obj = as_object(field(manifest_obj, "seal")?, "manifest.seal")?;
    require_exact_keys(
        seal_obj,
        &[
            "algorithm",
            "event_hash_profile",
            "trace_hash_profile",
            "canonicalization",
            "external_anchor",
            "external_anchor_ref",
        ],
        &[],
        "manifest.seal",
    )?;
    expect_string(field(seal_obj, "algorithm")?, "seal.algorithm", "sha-256")?;
    expect_string(
        field(seal_obj, "event_hash_profile")?,
        "seal.event_hash_profile",
        "construction-robot-trace-event-v0.1",
    )?;
    expect_string(
        field(seal_obj, "trace_hash_profile")?,
        "seal.trace_hash_profile",
        "construction-robot-trace-root-v0.1",
    )?;
    expect_string(
        field(seal_obj, "canonicalization")?,
        "seal.canonicalization",
        "RFC8785",
    )?;
    let anchor_claim =
        non_empty_string(field(seal_obj, "external_anchor")?, "seal.external_anchor")?;
    if !matches!(
        anchor_claim,
        "none" | "detached_signature" | "external_receipt"
    ) {
        return shape("unknown external_anchor claim");
    }
    match field(seal_obj, "external_anchor_ref")? {
        Value::String(value) if !value.trim().is_empty() => {}
        Value::Null => {}
        _ => return shape("external_anchor_ref must be non-empty string or null"),
    }

    let events = as_array(field(top, "events")?, "events")?;
    if events.is_empty() {
        return shape("events must not be empty");
    }
    if declared_event_count != events.len() as u64 {
        return shape("event_count does not match events array length");
    }

    let mut previous_digest: Option<String> = None;
    let mut previous_sequence: Option<u64> = None;
    let mut payload_bytes_total = 0_u64;
    let mut event_digests = Vec::with_capacity(events.len());

    for (index, event) in events.iter().enumerate() {
        let event_obj = as_object(event, "events[]")?;
        require_exact_keys(
            event_obj,
            &[
                "capture_sequence",
                "direction",
                "topic",
                "qos",
                "retain",
                "dup",
                "connection_epoch",
                "participant",
                "timestamps",
                "payload",
                "previous_event_digest",
                "event_digest",
            ],
            &["packet_id"],
            "events[]",
        )?;

        let sequence =
            non_negative_integer(field(event_obj, "capture_sequence")?, "capture_sequence")?;
        if let Some(previous) = previous_sequence {
            if sequence <= previous {
                return shape("capture_sequence must be strictly increasing");
            }
            if sequence != previous + 1 && recorder_drop_count == 0 {
                return shape("sequence gap requires a nonzero recorder_drop_count");
            }
        }
        if index == 0 {
            if sequence != first_sequence {
                return shape("first event sequence differs from manifest");
            }
            if !field(event_obj, "previous_event_digest")?.is_null() {
                return shape("first event previous_event_digest must be null");
            }
        } else {
            let declared_previous = digest_string(
                field(event_obj, "previous_event_digest")?,
                "previous_event_digest",
            )?;
            if Some(declared_previous) != previous_digest.as_deref() {
                return Err(VerificationError::DigestMismatch(format!(
                    "event {sequence} previous digest differs from prior event"
                )));
            }
        }

        let direction = non_empty_string(field(event_obj, "direction")?, "direction")?;
        if !matches!(
            direction,
            "broker_to_client" | "client_to_broker" | "unknown"
        ) {
            return shape("unknown event direction");
        }
        non_empty_string(field(event_obj, "topic")?, "topic")?;
        if non_negative_integer(field(event_obj, "qos")?, "qos")? > 2 {
            return shape("qos must be 0, 1, or 2");
        }
        as_bool(field(event_obj, "retain")?, "retain")?;
        as_bool(field(event_obj, "dup")?, "dup")?;
        if let Some(packet_id) = event_obj.get("packet_id")
            && !packet_id.is_null()
        {
            let id = non_negative_integer(packet_id, "packet_id")?;
            if !(1..=65_535).contains(&id) {
                return shape("packet_id must be between 1 and 65535");
            }
        }
        non_empty_string(field(event_obj, "connection_epoch")?, "connection_epoch")?;

        let participant = as_object(field(event_obj, "participant")?, "participant")?;
        require_exact_keys(participant, &["role", "identity_claim"], &[], "participant")?;
        let role = non_empty_string(field(participant, "role")?, "participant.role")?;
        if !matches!(
            role,
            "robot" | "fleet_manager" | "peripheral" | "broker" | "recorder" | "unknown"
        ) {
            return shape("unknown participant role");
        }
        if !matches!(
            field(participant, "identity_claim")?,
            Value::String(_) | Value::Null
        ) {
            return shape("identity_claim must be string or null");
        }

        let timestamps = as_array(field(event_obj, "timestamps")?, "timestamps")?;
        if timestamps.is_empty() {
            return shape("timestamps must not be empty");
        }
        let mut has_recorder_timestamp = false;
        for timestamp in timestamps {
            let timestamp_obj = as_object(timestamp, "timestamps[]")?;
            require_exact_keys(
                timestamp_obj,
                &["source", "timestamp", "clock_id", "uncertainty_ms"],
                &[],
                "timestamp",
            )?;
            let source = non_empty_string(field(timestamp_obj, "source")?, "timestamp.source")?;
            if !matches!(
                source,
                "recorder_received" | "broker_ingress" | "broker_egress" | "robot_reported"
            ) {
                return shape("unknown timestamp source");
            }
            has_recorder_timestamp |= source == "recorder_received";
            let time_value =
                non_empty_string(field(timestamp_obj, "timestamp")?, "timestamp.timestamp")?;
            if !is_rfc3339(time_value) {
                return shape("event timestamp must be RFC3339");
            }
            non_empty_string(field(timestamp_obj, "clock_id")?, "timestamp.clock_id")?;
            non_negative_integer(
                field(timestamp_obj, "uncertainty_ms")?,
                "timestamp.uncertainty_ms",
            )?;
        }
        if !has_recorder_timestamp {
            return shape("each event requires a recorder_received timestamp");
        }

        let payload = as_object(field(event_obj, "payload")?, "payload")?;
        require_exact_keys(payload, &["encoding", "data", "sha256"], &[], "payload")?;
        expect_string(field(payload, "encoding")?, "payload.encoding", "base64")?;
        let decoded = decode_base64(as_string(field(payload, "data")?, "payload.data")?)?;
        if sha256_hex(&decoded) != digest_string(field(payload, "sha256")?, "payload.sha256")? {
            return Err(VerificationError::DigestMismatch(format!(
                "payload SHA-256 mismatch at capture sequence {sequence}"
            )));
        }
        payload_bytes_total = payload_bytes_total.saturating_add(decoded.len() as u64);

        let declared_event_digest =
            digest_string(field(event_obj, "event_digest")?, "event_digest")?;
        let mut unsigned_event = event.clone();
        as_object_mut(&mut unsigned_event, "event")?.remove("event_digest");
        let actual_event_digest = hash_domain(EVENT_DOMAIN, &unsigned_event)?;
        if actual_event_digest != declared_event_digest {
            return Err(VerificationError::DigestMismatch(format!(
                "event digest mismatch at capture sequence {sequence}"
            )));
        }

        previous_digest = Some(actual_event_digest.clone());
        previous_sequence = Some(sequence);
        event_digests.push(Value::String(actual_event_digest));
    }

    if previous_sequence != Some(last_sequence) {
        return shape("last event sequence differs from manifest");
    }

    let annotations = as_array(field(top, "annotations")?, "annotations")?;
    let known_event_digests: BTreeSet<&str> =
        event_digests.iter().filter_map(Value::as_str).collect();
    for annotation in annotations {
        let annotation_obj = as_object(annotation, "annotations[]")?;
        require_exact_keys(
            annotation_obj,
            &[
                "source_event_digest",
                "parser_name",
                "parser_version",
                "rule_set_digest",
                "interpretation_digest",
                "interpretation",
            ],
            &[],
            "annotation",
        )?;
        let source = digest_string(
            field(annotation_obj, "source_event_digest")?,
            "source_event_digest",
        )?;
        if !known_event_digests.contains(source) {
            return shape("annotation references an event digest not present in this trace");
        }
        non_empty_string(field(annotation_obj, "parser_name")?, "parser_name")?;
        non_empty_string(field(annotation_obj, "parser_version")?, "parser_version")?;
        require_digest(field(annotation_obj, "rule_set_digest")?, "rule_set_digest")?;
        let interpretation = field(annotation_obj, "interpretation")?;
        if !interpretation.is_object() {
            return shape("annotation interpretation must be an object");
        }
        let expected = digest_string(
            field(annotation_obj, "interpretation_digest")?,
            "interpretation_digest",
        )?;
        if hash_domain(ANNOTATION_DOMAIN, interpretation)? != expected {
            return Err(VerificationError::DigestMismatch(
                "annotation interpretation digest mismatch".to_owned(),
            ));
        }
    }

    let actual_root = hash_root(manifest, &event_digests, annotations)?;
    let declared_root = digest_string(field(top, "final_trace_digest")?, "final_trace_digest")?;
    if actual_root != declared_root {
        return Err(VerificationError::DigestMismatch(
            "final trace root digest mismatch".to_owned(),
        ));
    }

    if let Some(expected) = expected_root_anchor {
        validate_digest_text(expected, "expected_root_anchor")?;
        if expected != actual_root {
            return Err(VerificationError::AnchorMismatch {
                expected: expected.to_owned(),
                actual: actual_root,
            });
        }
    }

    let completeness = match completeness_value {
        "unknown" => "unknown",
        "known_incomplete" => "known_incomplete",
        "complete_with_boundary_evidence" => {
            if !boundary_ref_present {
                return shape("complete capture requires capture_boundary_evidence_ref");
            }
            if recorder_drop_count != 0
                || broker_drop_evidence != "measured_zero"
                || broker_drop_count != Some(0)
            {
                return shape(
                    "complete capture requires zero recorder and broker drops with measured evidence",
                );
            }
            for pair in events.windows(2) {
                let left = non_negative_integer(
                    field(as_object(&pair[0], "event")?, "capture_sequence")?,
                    "sequence",
                )?;
                let right = non_negative_integer(
                    field(as_object(&pair[1], "event")?, "capture_sequence")?,
                    "sequence",
                )?;
                if right != left + 1 {
                    return shape("complete capture cannot contain sequence gaps");
                }
            }
            "complete_for_declared_capture_scope"
        }
        _ => return shape("unknown capture_completeness value"),
    };

    let external_anchor = match (expected_root_anchor.is_some(), anchor_claim) {
        (true, _) => "externally_supplied_root_digest_matches",
        (false, "none") => "unanchored_internal_consistency_only",
        (false, _) => "anchor_claim_present_but_not_verified",
    };

    Ok(VerificationReport {
        integrity: "valid",
        completeness,
        external_anchor,
        trace_id,
        event_count: events.len(),
        verified_payload_bytes: payload_bytes_total,
        final_trace_digest: actual_root,
    })
}

fn parse_unique_json(input: &[u8]) -> Result<Value, VerificationError> {
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    let parsed = StrictValue::deserialize(&mut deserializer).map_err(|error| {
        let message = error.to_string();
        if message.contains("duplicate object key") {
            VerificationError::DuplicateKey(message)
        } else {
            VerificationError::Json(message)
        }
    })?;
    deserializer
        .end()
        .map_err(|error| VerificationError::Json(error.to_string()))?;
    Ok(parsed.0)
}

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictVisitor)
    }
}

struct StrictVisitor;

impl<'de> Visitor<'de> for StrictVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(Number::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Number(Number::from(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let number = Number::from_f64(value).ok_or_else(|| E::custom("non-finite JSON number"))?;
        Ok(StrictValue(Value::Number(number)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(StrictValue(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(StrictValue(Value::Null))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        StrictValue::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element::<StrictValue>()? {
            values.push(value.0);
        }
        Ok(StrictValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut object = Map::new();
        while let Some((key, value)) = access.next_entry::<String, StrictValue>()? {
            if object.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate object key {key:?}")));
            }
            object.insert(key, value.0);
        }
        Ok(StrictValue(Value::Object(object)))
    }
}

/// RFC 8785 canonicalization for the v0.1 integer-only I-JSON profile.
/// Floating-point values and integers outside the IEEE-754 safe range are rejected.
fn canonical_json(value: &Value) -> Result<Vec<u8>, VerificationError> {
    let mut output = Vec::new();
    write_canonical(value, &mut output)?;
    Ok(output)
}

fn write_canonical(value: &Value, output: &mut Vec<u8>) -> Result<(), VerificationError> {
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(false) => output.extend_from_slice(b"false"),
        Value::Bool(true) => output.extend_from_slice(b"true"),
        Value::Number(number) => {
            if let Some(signed) = number.as_i64() {
                if signed.unsigned_abs() > MAX_SAFE_INTEGER {
                    return Err(VerificationError::UnsupportedNumber);
                }
                output.extend_from_slice(signed.to_string().as_bytes());
            } else if let Some(unsigned) = number.as_u64() {
                if unsigned > MAX_SAFE_INTEGER {
                    return Err(VerificationError::UnsupportedNumber);
                }
                output.extend_from_slice(unsigned.to_string().as_bytes());
            } else {
                return Err(VerificationError::UnsupportedNumber);
            }
        }
        Value::String(value) => {
            let serialized = serde_json::to_string(value)
                .map_err(|error| VerificationError::Json(error.to_string()))?;
            output.extend_from_slice(serialized.as_bytes());
        }
        Value::Array(values) => {
            output.push(b'[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                write_canonical(value, output)?;
            }
            output.push(b']');
        }
        Value::Object(values) => {
            output.push(b'{');
            let mut keys: Vec<&String> = values.keys().collect();
            keys.sort_by(|left, right| left.encode_utf16().cmp(right.encode_utf16()));
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                output.extend_from_slice(
                    serde_json::to_string(key)
                        .map_err(|error| VerificationError::Json(error.to_string()))?
                        .as_bytes(),
                );
                output.push(b':');
                let child = values.get(*key).ok_or_else(|| {
                    VerificationError::Shape(
                        "object key missing during canonicalization".to_owned(),
                    )
                })?;
                write_canonical(child, output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}

fn hash_domain(domain: &[u8], value: &Value) -> Result<String, VerificationError> {
    let canonical = canonical_json(value)?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(canonical);
    Ok(hex_lower(&hasher.finalize()))
}

fn hash_root(
    manifest: &Value,
    event_digests: &[Value],
    annotations: &[Value],
) -> Result<String, VerificationError> {
    let root = serde_json::json!({
        "manifest": manifest,
        "ordered_event_digests": event_digests,
        "annotations": annotations
    });
    hash_domain(ROOT_DOMAIN, &root)
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_lower(&Sha256::digest(bytes))
}

fn hex_lower(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

fn decode_base64(input: &str) -> Result<Vec<u8>, VerificationError> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    if !input.len().is_multiple_of(4) {
        return Err(invalid_base64("encoded length must be divisible by four"));
    }
    let bytes = input.as_bytes();
    let mut output = Vec::with_capacity((bytes.len() / 4) * 3);
    for (index, chunk) in bytes.chunks_exact(4).enumerate() {
        let final_chunk = index + 1 == bytes.len() / 4;
        let a = base64_value(chunk[0]).ok_or_else(|| invalid_base64("invalid first sextet"))?;
        let b = base64_value(chunk[1]).ok_or_else(|| invalid_base64("invalid second sextet"))?;
        let pad_c = chunk[2] == b'=';
        let pad_d = chunk[3] == b'=';
        if pad_c && !pad_d {
            return Err(invalid_base64(
                "third-character padding requires fourth-character padding",
            ));
        }
        if (pad_c || pad_d) && !final_chunk {
            return Err(invalid_base64(
                "padding is permitted only in the final quartet",
            ));
        }
        output.push((a << 2) | (b >> 4));
        if pad_c {
            if b & 0x0f != 0 {
                return Err(invalid_base64("nonzero unused bits before == padding"));
            }
            continue;
        }
        let c = base64_value(chunk[2]).ok_or_else(|| invalid_base64("invalid third sextet"))?;
        output.push((b << 4) | (c >> 2));
        if pad_d {
            if c & 0x03 != 0 {
                return Err(invalid_base64("nonzero unused bits before = padding"));
            }
            continue;
        }
        let d = base64_value(chunk[3]).ok_or_else(|| invalid_base64("invalid fourth sextet"))?;
        output.push((c << 6) | d);
    }
    Ok(output)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn invalid_base64(message: &str) -> VerificationError {
    VerificationError::InvalidBase64(message.to_owned())
}

fn validate_digest_text(value: &str, name: &str) -> Result<(), VerificationError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return shape(&format!(
            "{name} must be 64 lowercase hexadecimal characters"
        ));
    }
    Ok(())
}

fn digest_string<'a>(value: &'a Value, name: &str) -> Result<&'a str, VerificationError> {
    let value = as_string(value, name)?;
    validate_digest_text(value, name)?;
    Ok(value)
}

fn require_digest(value: &Value, name: &str) -> Result<(), VerificationError> {
    digest_string(value, name).map(|_| ())
}

fn as_object<'a>(
    value: &'a Value,
    name: &str,
) -> Result<&'a Map<String, Value>, VerificationError> {
    value
        .as_object()
        .ok_or_else(|| VerificationError::Shape(format!("{name} must be an object")))
}

fn as_object_mut<'a>(
    value: &'a mut Value,
    name: &str,
) -> Result<&'a mut Map<String, Value>, VerificationError> {
    value
        .as_object_mut()
        .ok_or_else(|| VerificationError::Shape(format!("{name} must be an object")))
}

fn as_array<'a>(value: &'a Value, name: &str) -> Result<&'a Vec<Value>, VerificationError> {
    value
        .as_array()
        .ok_or_else(|| VerificationError::Shape(format!("{name} must be an array")))
}

fn as_string<'a>(value: &'a Value, name: &str) -> Result<&'a str, VerificationError> {
    value
        .as_str()
        .ok_or_else(|| VerificationError::Shape(format!("{name} must be a string")))
}

fn non_empty_string<'a>(value: &'a Value, name: &str) -> Result<&'a str, VerificationError> {
    let text = as_string(value, name)?;
    if text.trim().is_empty() {
        return shape(&format!("{name} must not be empty"));
    }
    Ok(text)
}

fn non_negative_integer(value: &Value, name: &str) -> Result<u64, VerificationError> {
    let number = value.as_u64().ok_or_else(|| {
        VerificationError::Shape(format!("{name} must be a non-negative integer"))
    })?;
    if number > MAX_SAFE_INTEGER {
        return Err(VerificationError::UnsupportedNumber);
    }
    Ok(number)
}

fn nullable_non_negative_integer(
    value: &Value,
    name: &str,
) -> Result<Option<u64>, VerificationError> {
    if value.is_null() {
        Ok(None)
    } else {
        non_negative_integer(value, name).map(Some)
    }
}

fn as_bool(value: &Value, name: &str) -> Result<bool, VerificationError> {
    value
        .as_bool()
        .ok_or_else(|| VerificationError::Shape(format!("{name} must be a boolean")))
}

fn field<'a>(object: &'a Map<String, Value>, name: &str) -> Result<&'a Value, VerificationError> {
    object
        .get(name)
        .ok_or_else(|| VerificationError::Shape(format!("missing required field {name}")))
}

fn expect_string(value: &Value, name: &str, expected: &str) -> Result<(), VerificationError> {
    if as_string(value, name)? != expected {
        return shape(&format!("{name} must equal {expected}"));
    }
    Ok(())
}

fn require_exact_keys(
    object: &Map<String, Value>,
    required: &[&str],
    optional: &[&str],
    name: &str,
) -> Result<(), VerificationError> {
    for key in required {
        if !object.contains_key(*key) {
            return shape(&format!("{name} missing required field {key}"));
        }
    }
    for key in object.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return shape(&format!("{name} contains unsupported field {key}"));
        }
    }
    Ok(())
}

fn is_rfc3339(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return false;
    }
    for (start, end) in [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)] {
        if !bytes[start..end].iter().all(u8::is_ascii_digit) {
            return false;
        }
    }
    let number = |start: usize, end: usize| -> u32 { value[start..end].parse().unwrap_or(0) };
    let year = number(0, 4);
    let month = number(5, 7);
    let day = number(8, 10);
    let hour = number(11, 13);
    let minute = number(14, 16);
    let second = number(17, 19);
    if year == 0
        || !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return false;
    }
    let mut index = 19;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        let fractional_start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == fractional_start {
            return false;
        }
    }
    match bytes.get(index) {
        Some(b'Z') => index + 1 == bytes.len(),
        Some(b'+') | Some(b'-') => {
            if index + 6 != bytes.len() || bytes.get(index + 3) != Some(&b':') {
                return false;
            }
            bytes[index + 1..index + 3].iter().all(u8::is_ascii_digit)
                && bytes[index + 4..index + 6].iter().all(u8::is_ascii_digit)
                && number(index + 1, index + 3) <= 23
                && number(index + 4, index + 6) <= 59
        }
        _ => false,
    }
}

fn shape<T>(message: &str) -> Result<T, VerificationError> {
    Err(VerificationError::Shape(message.to_owned()))
}

#[cfg(test)]
mod tests {
    #[cfg(test)]
    fn encode_base64(bytes: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut output = String::new();
        for chunk in bytes.chunks(3) {
            let a = chunk[0];
            let b = *chunk.get(1).unwrap_or(&0);
            let c = *chunk.get(2).unwrap_or(&0);
            output.push(TABLE[(a >> 2) as usize] as char);
            output.push(TABLE[(((a & 0x03) << 4) | (b >> 4)) as usize] as char);
            output.push(if chunk.len() > 1 {
                TABLE[(((b & 0x0f) << 2) | (c >> 6)) as usize] as char
            } else {
                '='
            });
            output.push(if chunk.len() > 2 {
                TABLE[(c & 0x3f) as usize] as char
            } else {
                '='
            });
        }
        output
    }

    use super::*;

    const FIXTURE: &str = include_str!("../../fixtures/trace-synthetic-vda-001.json");
    const ROOT: &str = "6812602182c7eceeceadf06d82424a069351acbbba7c4e17eb36d4ccf69e73a1";

    fn fixture_value() -> Value {
        parse_unique_json(FIXTURE.as_bytes()).expect("fixture JSON")
    }

    fn reseal(value: &mut Value) {
        let trace = as_object_mut(value, "trace").expect("trace object");
        let manifest = trace.get("manifest").expect("manifest").clone();
        let annotations = trace
            .get("annotations")
            .expect("annotations")
            .as_array()
            .expect("array")
            .clone();
        let events = trace
            .get_mut("events")
            .expect("events")
            .as_array_mut()
            .expect("array");
        let mut previous: Option<String> = None;
        let mut digests = Vec::new();
        for event in events {
            let object = as_object_mut(event, "event").expect("event object");
            object.insert(
                "previous_event_digest".to_owned(),
                previous.clone().map(Value::String).unwrap_or(Value::Null),
            );
            object.remove("event_digest");
            let digest = hash_domain(EVENT_DOMAIN, event).expect("event digest");
            as_object_mut(event, "event")
                .expect("event object")
                .insert("event_digest".to_owned(), Value::String(digest.clone()));
            previous = Some(digest.clone());
            digests.push(Value::String(digest));
        }
        let root_digest = hash_root(&manifest, &digests, &annotations).expect("root digest");
        trace.insert("final_trace_digest".to_owned(), Value::String(root_digest));
    }

    #[test]
    fn verifies_fixture_as_integrity_valid_but_unanchored_and_completeness_unknown() {
        let report = verify_trace_bytes(FIXTURE.as_bytes(), None).expect("valid fixture");
        assert_eq!(report.integrity, "valid");
        assert_eq!(report.completeness, "unknown");
        assert_eq!(
            report.external_anchor,
            "unanchored_internal_consistency_only"
        );
        assert_eq!(report.event_count, 3);
        assert_eq!(report.verified_payload_bytes, 139);
        assert_eq!(report.final_trace_digest, ROOT);
    }

    #[test]
    fn externally_supplied_matching_root_is_distinguished_from_internal_digest() {
        let report =
            verify_trace_bytes(FIXTURE.as_bytes(), Some(ROOT)).expect("matching root anchor");
        assert_eq!(
            report.external_anchor,
            "externally_supplied_root_digest_matches"
        );
    }

    #[test]
    fn wrong_external_root_is_rejected() {
        let wrong = "0".repeat(64);
        assert!(matches!(
            verify_trace_bytes(FIXTURE.as_bytes(), Some(&wrong)),
            Err(VerificationError::AnchorMismatch { .. })
        ));
    }

    #[test]
    fn payload_and_topic_mutations_are_rejected_without_resealing() {
        let mut value = fixture_value();
        value["events"][0]["payload"]["data"] = Value::String("YWJj".to_owned());
        assert!(verify_trace_bytes(&serde_json::to_vec(&value).expect("serialize"), None).is_err());

        let mut value = fixture_value();
        value["events"][0]["topic"] = Value::String("mutated/topic".to_owned());
        assert!(verify_trace_bytes(&serde_json::to_vec(&value).expect("serialize"), None).is_err());
    }

    #[test]
    fn recomputed_unanchored_rewrite_is_consistent_but_not_authenticated() {
        let mut value = fixture_value();
        let payload = b"rewritten-by-untrusted-party";
        value["events"][0]["payload"]["data"] = Value::String(encode_base64(payload));
        value["events"][0]["payload"]["sha256"] = Value::String(sha256_hex(payload));
        reseal(&mut value);
        let bytes = serde_json::to_vec(&value).expect("serialize");
        let report = verify_trace_bytes(&bytes, None).expect("internally consistent rewrite");
        assert_eq!(
            report.external_anchor,
            "unanchored_internal_consistency_only"
        );
        assert_ne!(report.final_trace_digest, ROOT);
        assert!(verify_trace_bytes(&bytes, Some(ROOT)).is_err());
    }

    #[test]
    fn duplicate_json_keys_are_rejected() {
        let bytes = br#"{"manifest":{},"manifest":{},"events":[],"annotations":[],"final_trace_digest":""}"#;
        assert!(matches!(
            parse_unique_json(bytes),
            Err(VerificationError::DuplicateKey(_))
        ));
    }

    #[test]
    fn completeness_and_redaction_claims_require_their_evidence() {
        let mut value = fixture_value();
        value["manifest"]["capture"]["capture_completeness"] =
            Value::String("complete_with_boundary_evidence".to_owned());
        reseal(&mut value);
        assert!(verify_trace_bytes(&serde_json::to_vec(&value).expect("serialize"), None).is_err());

        let mut value = fixture_value();
        value["manifest"]["capture"]["redaction_applied"] = Value::Bool(true);
        reseal(&mut value);
        assert!(verify_trace_bytes(&serde_json::to_vec(&value).expect("serialize"), None).is_err());
    }

    #[test]
    fn annotations_must_reference_a_real_event_and_valid_digest() {
        let mut value = fixture_value();
        let interpretation = serde_json::json!({"kind":"executor_action_finished_reported"});
        value["annotations"] = serde_json::json!([{
            "source_event_digest": "0".repeat(64),
            "parser_name": "test-parser",
            "parser_version": "0.1.0",
            "rule_set_digest": "a".repeat(64),
            "interpretation_digest": hash_domain(ANNOTATION_DOMAIN, &interpretation).expect("hash"),
            "interpretation": interpretation
        }]);
        reseal(&mut value);
        assert!(verify_trace_bytes(&serde_json::to_vec(&value).expect("serialize"), None).is_err());
    }

    #[test]
    fn jcs_property_order_uses_utf16_code_units() {
        let value = parse_unique_json(r#"{"":2,"😀":1}"#.as_bytes()).expect("unicode JSON");
        let canonical = canonical_json(&value).expect("canonicalize");
        assert_eq!(
            String::from_utf8(canonical).expect("utf8"),
            r#"{"😀":1,"":2}"#
        );
    }

    #[test]
    fn jcs_profile_rejects_floats_and_unsafe_integer_values() {
        let float = parse_unique_json(br#"{"v":1.5}"#).expect("parse number");
        assert_eq!(
            canonical_json(&float),
            Err(VerificationError::UnsupportedNumber)
        );
        let large = parse_unique_json(br#"{"v":9007199254740992}"#).expect("parse integer");
        assert_eq!(
            canonical_json(&large),
            Err(VerificationError::UnsupportedNumber)
        );
    }

    #[test]
    fn strict_base64_checks_padding_and_unused_bits() {
        assert_eq!(decode_base64("YWJj").expect("valid"), b"abc");
        assert_eq!(decode_base64("YWI=").expect("valid"), b"ab");
        assert_eq!(decode_base64("YQ==").expect("valid"), b"a");
        assert!(decode_base64("YQ=A").is_err());
        assert!(decode_base64("YR==").is_err());
    }
}

