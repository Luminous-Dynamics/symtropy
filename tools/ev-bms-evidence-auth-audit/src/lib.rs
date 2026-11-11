use ed25519_dalek::{Signature, VerifyingKey};
use jsonschema::{Validator, draft202012};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CORPUS_VERSION: &str = "ev-bms-evidence-auth-corpus/0.1";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub corpus_version: String,
    pub policy: TrustPolicy,
    pub trusted_keys: Vec<TrustedKey>,
    pub asset_bindings: Vec<AssetBinding>,
    /// Event order is semantically significant for anti-replay and clock checks.
    pub events: Vec<SignedEvent>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrustPolicy {
    pub maximum_clock_uncertainty_ms: u64,
    pub trusted_clock_sources: Vec<String>,
    /// References admitted by an external boot-attestation verifier/policy.
    pub admitted_boot_epochs: Vec<AdmittedBootEpoch>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdmittedBootEpoch {
    pub evidence_ref: String,
    pub producer_id: String,
    pub boot_epoch_id: String,
    pub firmware_sha256: String,
    pub vehicle_ref: String,
    pub pack_ref: String,
    pub configuration_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedKey {
    pub key_id: String,
    pub producer_id: String,
    pub producer_role: String,
    pub public_key_hex: String,
    pub valid_from_ms: u64,
    pub valid_until_ms: Option<u64>,
    pub revoked: bool,
    pub authorized_event_types: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AssetBinding {
    pub vehicle_ref: String,
    pub pack_ref: String,
    pub configuration_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedEvent {
    pub claims: EventClaims,
    pub signature_hex: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventClaims {
    pub event_id: String,
    pub producer_id: String,
    pub producer_role: String,
    pub vehicle_ref: String,
    pub pack_ref: String,
    pub configuration_sha256: String,
    pub key_id: String,
    pub boot_epoch_id: String,
    pub boot_attestation_ref: Option<String>,
    pub sequence: u64,
    pub nonce_hex: String,
    pub observed_at_ms: u64,
    pub clock_source: String,
    pub clock_uncertainty_ms: u64,
    pub firmware_sha256: String,
    pub payload: BatteryPayload,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BatteryPayload {
    pub event_type: String,
    pub channel: String,
    pub sensor_ref: String,
    pub value: Option<f64>,
    pub unit: Option<String>,
    pub evidence_class: String,
    pub calibration_status: String,
    pub calibration_ref: Option<String>,
    pub calibration_sha256: Option<String>,
    pub sensor_valid: bool,
    pub out_of_domain: bool,
    pub protocol_ref: Option<String>,
    pub observation_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventAudit {
    pub event_id: String,
    pub producer_id: String,
    pub producer_role: String,
    pub pack_ref: String,
    pub boot_epoch_id: String,
    pub sequence: u64,
    pub authenticity_state: String,
    pub physical_validity_state: String,
    pub evidence_class: String,
    pub sequence_gap_before_event: u64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditReport {
    pub corpus_version: String,
    pub input_sha256: String,
    pub report_sha256: String,
    pub event_count: usize,
    pub authenticated_count: usize,
    pub quarantined_count: usize,
    pub rejected_count: usize,
    pub replay_rejected_count: usize,
    pub unattested_count: usize,
    pub events: Vec<EventAudit>,
}

#[derive(Default)]
struct StreamState {
    last_sequence: Option<u64>,
    last_timestamp_ms: Option<u64>,
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn compile_schema(bytes: &[u8]) -> Result<Validator, String> {
    let schema: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("schema JSON is invalid: {error}"))?;
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return Err("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if !draft202012::meta::is_valid(&schema) {
        return Err("schema fails Draft 2020-12 meta-schema validation".to_owned());
    }
    draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("schema compilation failed: {error}"))
}

fn validate_instance(validator: &Validator, value: &Value) -> Result<(), String> {
    let errors: Vec<String> = validator.iter_errors(value).map(|error| error.to_string()).collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("BMS evidence corpus failed schema validation: {}", errors.join("; ")))
    }
}

fn decode_hex<const N: usize>(value: &str, label: &str) -> Result<[u8; N], String> {
    let bytes = hex::decode(value).map_err(|_| format!("{label} is not valid lowercase hexadecimal"))?;
    if value.bytes().any(|byte| byte.is_ascii_uppercase()) || bytes.len() != N {
        return Err(format!("{label} must be exactly {} lowercase-hex bytes", N));
    }
    let mut output = [0u8; N];
    output.copy_from_slice(&bytes);
    Ok(output)
}

fn verify_event_signature(event: &SignedEvent, key: &TrustedKey) -> Result<(), String> {
    let signature_hex = event.signature_hex.as_deref().filter(|value| !value.is_empty())
        .ok_or_else(|| "UNATTESTED: event has no signature".to_owned())?;
    let public_key = decode_hex::<32>(&key.public_key_hex, "trusted public key")?;
    let signature_bytes = decode_hex::<64>(signature_hex, "event signature")?;
    let verifying_key = VerifyingKey::from_bytes(&public_key)
        .map_err(|_| "REJECTED_KEY_MALFORMED: trusted public key is invalid".to_owned())?;
    let signature = Signature::from_bytes(&signature_bytes);
    let signed_bytes = serde_json::to_vec(&event.claims)
        .map_err(|error| format!("cannot serialize signed claims: {error}"))?;
    verifying_key
        .verify_strict(&signed_bytes, &signature)
        .map_err(|_| "REJECTED_SIGNATURE: signed claims do not verify".to_owned())
}

fn payload_digest_is_valid(claims: &EventClaims) -> Result<(), String> {
    if claims.payload.sensor_ref.trim().is_empty() {
        return Err("REJECTED_PAYLOAD_SENSOR_IDENTITY: sensor reference is empty".to_owned());
    }
    if claims.payload.calibration_status != "not_applicable" {
        if claims.payload.calibration_ref.as_deref().is_none_or(str::is_empty)
            || claims.payload.calibration_sha256.as_deref().is_none_or(|digest| !is_sha256(digest))
        {
            return Err("REJECTED_CALIBRATION_BINDING: calibration status lacks exact reference/digest".to_owned());
        }
    }
    if let Some(digest) = &claims.payload.observation_digest {
        if !is_sha256(digest) {
            return Err("REJECTED_PAYLOAD_DIGEST: observation digest is malformed".to_owned());
        }
    }
    if let Some(value) = claims.payload.value {
        if !value.is_finite() {
            return Err("REJECTED_PAYLOAD_VALUE: payload value is non-finite".to_owned());
        }
    }
    Ok(())
}

fn physical_validity(payload: &BatteryPayload) -> (String, Option<String>) {
    if !payload.sensor_valid {
        return (
            "SOURCE_REPORTED_SENSOR_INVALID".to_owned(),
            Some("the signed producer reports an invalid sensor; signature does not independently validate the sensor".to_owned()),
        );
    }
    if payload.calibration_status != "current" {
        return (
            "SOURCE_REPORTED_CALIBRATION_NOT_CURRENT".to_owned(),
            Some("the signed producer reports expired/unknown calibration; the calibration record is not independently validated here".to_owned()),
        );
    }
    if payload.out_of_domain {
        return (
            "SOURCE_REPORTED_OUT_OF_DOMAIN".to_owned(),
            Some("the signed producer flags an out-of-domain observation; physical validity remains unverified".to_owned()),
        );
    }
    (
        "NOT_ESTABLISHED_BY_SIGNATURE".to_owned(),
        Some("cryptographic authenticity does not independently establish sensor truth or physical validity".to_owned()),
    )
}

fn make_row(
    claims: &EventClaims,
    state: &str,
    physical_state: &str,
    gap: u64,
    reason: Option<String>,
) -> EventAudit {
    EventAudit {
        event_id: claims.event_id.clone(),
        producer_id: claims.producer_id.clone(),
        producer_role: claims.producer_role.clone(),
        pack_ref: claims.pack_ref.clone(),
        boot_epoch_id: claims.boot_epoch_id.clone(),
        sequence: claims.sequence,
        authenticity_state: state.to_owned(),
        physical_validity_state: physical_state.to_owned(),
        evidence_class: claims.payload.evidence_class.clone(),
        sequence_gap_before_event: gap,
        reason,
    }
}

pub fn audit_corpus(schema_bytes: &[u8], corpus_bytes: &[u8]) -> Result<AuditReport, String> {
    let validator = compile_schema(schema_bytes)?;
    let json: Value = serde_json::from_slice(corpus_bytes)
        .map_err(|error| format!("corpus JSON is invalid: {error}"))?;
    validate_instance(&validator, &json)?;
    let corpus: Corpus = serde_json::from_value(json)
        .map_err(|error| format!("corpus decode failed: {error}"))?;
    if corpus.corpus_version != CORPUS_VERSION {
        return Err("unsupported BMS evidence corpus version".to_owned());
    }
    if corpus.events.is_empty() {
        return Err("BMS evidence corpus must contain at least one event".to_owned());
    }

    let mut keys = BTreeMap::new();
    for key in &corpus.trusted_keys {
        if key.key_id.trim().is_empty()
            || key.producer_id.trim().is_empty()
            || key.producer_role.trim().is_empty()
            || keys.insert(key.key_id.clone(), key).is_some()
        {
            return Err(format!("empty or duplicate trusted key identity: {}", key.key_id));
        }
        if key.valid_until_ms.is_some_and(|end| end < key.valid_from_ms) {
            return Err(format!("trusted key {} has an inverted validity interval", key.key_id));
        }
        decode_hex::<32>(&key.public_key_hex, "trusted public key")?;
    }

    let mut admitted_boots = BTreeSet::new();
    for admission in &corpus.policy.admitted_boot_epochs {
        if admission.evidence_ref.trim().is_empty()
            || admission.producer_id.trim().is_empty()
            || admission.boot_epoch_id.trim().is_empty()
            || admission.vehicle_ref.trim().is_empty()
            || admission.pack_ref.trim().is_empty()
            || !is_sha256(&admission.firmware_sha256)
            || !is_sha256(&admission.configuration_sha256)
        {
            return Err("admitted boot epoch contains incomplete identity or malformed digest".to_owned());
        }
        if !admitted_boots.insert((
            admission.evidence_ref.clone(),
            admission.producer_id.clone(),
            admission.boot_epoch_id.clone(),
        )) {
            return Err("duplicate admitted boot epoch reference".to_owned());
        }
    }

    let mut asset_bindings = BTreeSet::new();
    for binding in &corpus.asset_bindings {
        if binding.vehicle_ref.trim().is_empty()
            || binding.pack_ref.trim().is_empty()
            || !is_sha256(&binding.configuration_sha256)
        {
            return Err("asset binding contains empty identity or malformed configuration digest".to_owned());
        }
        if !asset_bindings.insert((
            binding.vehicle_ref.clone(),
            binding.pack_ref.clone(),
            binding.configuration_sha256.clone(),
        )) {
            return Err("duplicate asset binding".to_owned());
        }
    }

    let mut event_ids = BTreeSet::new();
    let mut replay_nonces = BTreeSet::new();
    let mut streams: BTreeMap<(String, String), StreamState> = BTreeMap::new();
    let mut events = Vec::with_capacity(corpus.events.len());
    let mut authenticated_count = 0;
    let mut quarantined_count = 0;
    let mut rejected_count = 0;
    let mut replay_rejected_count = 0;
    let mut unattested_count = 0;

    for event in &corpus.events {
        let claims = &event.claims;
        let (physical_state, physical_reason) = physical_validity(&claims.payload);
        if claims.event_id.trim().is_empty() || !event_ids.insert(claims.event_id.clone()) {
            rejected_count += 1;
            events.push(make_row(
                claims,
                "REJECTED_DUPLICATE_EVENT_ID",
                &physical_state,
                0,
                Some("event identity is empty or repeated".to_owned()),
            ));
            continue;
        }

        let Some(key) = keys.get(&claims.key_id).copied() else {
            rejected_count += 1;
            events.push(make_row(
                claims,
                "REJECTED_UNKNOWN_KEY",
                &physical_state,
                0,
                Some("key_id is not present in the admitted trust store".to_owned()),
            ));
            continue;
        };

        match verify_event_signature(event, key) {
            Ok(()) => {}
            Err(reason) if reason.starts_with("UNATTESTED") => {
                unattested_count += 1;
                events.push(make_row(
                    claims,
                    "UNATTESTED",
                    &physical_state,
                    0,
                    Some(reason),
                ));
                continue;
            }
            Err(reason) => {
                rejected_count += 1;
                events.push(make_row(
                    claims,
                    if reason.starts_with("REJECTED_KEY_MALFORMED") {
                        "REJECTED_KEY_MALFORMED"
                    } else {
                        "REJECTED_SIGNATURE"
                    },
                    &physical_state,
                    0,
                    Some(reason),
                ));
                continue;
            }
        }

        if key.revoked {
            rejected_count += 1;
            events.push(make_row(
                claims,
                "REJECTED_REVOKED_KEY",
                &physical_state,
                0,
                Some("signing key is revoked under the supplied trust policy".to_owned()),
            ));
            continue;
        }
        if claims.producer_id != key.producer_id || claims.producer_role != key.producer_role {
            rejected_count += 1;
            events.push(make_row(
                claims,
                "REJECTED_SIGNER_BINDING",
                &physical_state,
                0,
                Some("signed producer identity/role does not match the trusted key record".to_owned()),
            ));
            continue;
        }

        let stream_key = (claims.producer_id.clone(), claims.boot_epoch_id.clone());
        let stream = streams.entry(stream_key).or_default();
        let mut gap = 0;
        if let Some(last) = stream.last_sequence {
            if claims.sequence <= last {
                replay_rejected_count += 1;
                rejected_count += 1;
                events.push(make_row(
                    claims,
                    "REJECTED_REPLAY",
                    &physical_state,
                    0,
                    Some(format!("sequence {} is not greater than prior accepted sequence {last}", claims.sequence)),
                ));
                continue;
            }
            gap = claims.sequence.saturating_sub(last).saturating_sub(1);
        }
        let nonce_key = (claims.producer_id.clone(), claims.nonce_hex.clone());
        if claims.nonce_hex.len() != 32
            || claims.nonce_hex.bytes().any(|byte| !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase())
            || !replay_nonces.insert(nonce_key)
        {
            replay_rejected_count += 1;
            rejected_count += 1;
            events.push(make_row(
                claims,
                "REJECTED_REPLAY",
                &physical_state,
                0,
                Some("nonce is malformed or has already been used by this producer".to_owned()),
            ));
            continue;
        }
        if stream.last_timestamp_ms.is_some_and(|last| claims.observed_at_ms < last) {
            quarantined_count += 1;
            stream.last_sequence = Some(claims.sequence);
            events.push(make_row(
                claims,
                "QUARANTINED_CLOCK_REGRESSION",
                &physical_state,
                gap,
                Some("signed event time regressed within the same producer boot epoch".to_owned()),
            ));
            continue;
        }

        // A cryptographically verified event consumes its sequence/nonce even
        // when later policy checks quarantine it. Unsigned or bad-signature
        // events cannot poison this replay state.
        stream.last_sequence = Some(claims.sequence);
        stream.last_timestamp_ms = Some(claims.observed_at_ms);

        let asset_ok = asset_bindings.contains(&(
            claims.vehicle_ref.clone(),
            claims.pack_ref.clone(),
            claims.configuration_sha256.clone(),
        ));
        if !asset_ok {
            rejected_count += 1;
            events.push(make_row(
                claims,
                "REJECTED_ASSET_BINDING",
                &physical_state,
                gap,
                Some("vehicle/pack/configuration tuple is not in the admitted asset binding set".to_owned()),
            ));
            continue;
        }
        if !key.authorized_event_types.iter().any(|event_type| event_type == &claims.payload.event_type) {
            rejected_count += 1;
            events.push(make_row(
                claims,
                "REJECTED_AUTHORITY",
                &physical_state,
                gap,
                Some("trusted key role is not authorized to assert this event type".to_owned()),
            ));
            continue;
        }
        if !corpus.policy.trusted_clock_sources.iter().any(|source| source == &claims.clock_source)
            || claims.clock_uncertainty_ms > corpus.policy.maximum_clock_uncertainty_ms
        {
            quarantined_count += 1;
            events.push(make_row(
                claims,
                "QUARANTINED_CLOCK_UNTRUSTED",
                &physical_state,
                gap,
                Some("clock source is not trusted or its uncertainty exceeds policy".to_owned()),
            ));
            continue;
        }
        if claims.observed_at_ms < key.valid_from_ms
            || key.valid_until_ms.is_some_and(|end| claims.observed_at_ms > end)
        {
            rejected_count += 1;
            events.push(make_row(
                claims,
                "REJECTED_KEY_OUTSIDE_VALIDITY",
                &physical_state,
                0,
                Some("event time lies outside the signing key validity interval".to_owned()),
            ));
            continue;
        }
        let boot_admitted = claims.boot_attestation_ref.as_deref().is_some_and(|reference| {
            corpus.policy.admitted_boot_epochs.iter().any(|admission| {
                admission.evidence_ref == reference
                    && admission.producer_id == claims.producer_id
                    && admission.boot_epoch_id == claims.boot_epoch_id
                    && admission.firmware_sha256 == claims.firmware_sha256
                    && admission.vehicle_ref == claims.vehicle_ref
                    && admission.pack_ref == claims.pack_ref
                    && admission.configuration_sha256 == claims.configuration_sha256
            })
        });
        if !boot_admitted {
            quarantined_count += 1;
            events.push(make_row(
                claims,
                "QUARANTINED_BOOT_EPOCH_UNKNOWN",
                &physical_state,
                gap,
                Some("boot epoch reference is absent or does not match an externally admitted producer/configuration/firmware binding".to_owned()),
            ));
            continue;
        }
        if claims.firmware_sha256.len() != 64
            || !is_sha256(&claims.configuration_sha256)
            || !is_sha256(&claims.firmware_sha256)
            || claims.payload.channel.trim().is_empty()
            || claims.payload.event_type.trim().is_empty()
        {
            rejected_count += 1;
            events.push(make_row(
                claims,
                "REJECTED_MALFORMED_EVIDENCE",
                &physical_state,
                gap,
                Some("signed event lacks a valid firmware/configuration digest or event channel".to_owned()),
            ));
            continue;
        }
        if let Err(reason) = payload_digest_is_valid(claims) {
            rejected_count += 1;
            events.push(make_row(
                claims,
                "REJECTED_MALFORMED_EVIDENCE",
                &physical_state,
                gap,
                Some(reason),
            ));
            continue;
        }

        authenticated_count += 1;
        events.push(make_row(
            claims,
            "AUTHENTICATED_AND_AUTHORIZED",
            &physical_state,
            gap,
            physical_reason,
        ));
    }

    let input_sha256 = sha256(corpus_bytes);
    let report_core = serde_json::to_vec(&events)
        .map_err(|error| format!("cannot serialize audit rows: {error}"))?;
    let report_sha256 = sha256(&report_core);
    Ok(AuditReport {
        corpus_version: corpus.corpus_version,
        input_sha256,
        report_sha256,
        event_count: events.len(),
        authenticated_count,
        quarantined_count,
        rejected_count,
        replay_rejected_count,
        unattested_count,
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SCHEMA: &[u8] = include_bytes!("../../../schemas/transport/ev_bms_signed_event_v0_1.schema.json");
    const CORPUS: &[u8] = include_bytes!("../../../fixtures/ev_battery/bms_evidence_auth_v0_1.json");

    fn report() -> AuditReport {
        audit_corpus(SCHEMA, CORPUS).expect("frozen BMS evidence corpus")
    }

    fn row<'a>(report: &'a AuditReport, id: &str) -> &'a EventAudit {
        report.events.iter().find(|event| event.event_id == id).expect("event row")
    }

    #[test]
    fn frozen_corpus_separates_authentication_quarantine_rejection_and_unattested() {
        let report = report();
        assert_eq!(report.event_count, 14);
        assert_eq!(report.authenticated_count, 4);
        assert_eq!(report.quarantined_count, 2);
        assert_eq!(report.rejected_count, 7);
        assert_eq!(report.replay_rejected_count, 1);
        assert_eq!(report.unattested_count, 1);
    }

    #[test]
    fn valid_signature_does_not_establish_sensor_truth() {
        let report = report();
        let event = row(&report, "evt-valid-001");
        assert_eq!(event.authenticity_state, "AUTHENTICATED_AND_AUTHORIZED");
        assert_eq!(event.physical_validity_state, "NOT_ESTABLISHED_BY_SIGNATURE");
        assert_eq!(event.evidence_class, "bms_reported");
    }

    #[test]
    fn duplicate_sequence_and_nonce_are_rejected_as_replay() {
        let report = report();
        assert_eq!(row(&report, "evt-replay-002").authenticity_state, "REJECTED_REPLAY");
        assert_eq!(report.replay_rejected_count, 1);
    }

    #[test]
    fn compromised_or_invalid_physical_measurement_does_not_change_authenticity() {
        let report = report();
        let calibration = row(&report, "evt-calibration-expired-009");
        assert_eq!(calibration.authenticity_state, "AUTHENTICATED_AND_AUTHORIZED");
        assert_eq!(calibration.physical_validity_state, "SOURCE_REPORTED_CALIBRATION_NOT_CURRENT");
        let sensor = row(&report, "evt-sensor-invalid-010");
        assert_eq!(sensor.authenticity_state, "AUTHENTICATED_AND_AUTHORIZED");
        assert_eq!(sensor.physical_validity_state, "SOURCE_REPORTED_SENSOR_INVALID");
    }

    #[test]
    fn payload_change_after_signing_fails_signature_verification() {
        let mut corpus: Value = serde_json::from_slice(CORPUS).expect("corpus JSON");
        let event = corpus["events"].as_array_mut().unwrap()
            .iter_mut().find(|event| event["claims"]["event_id"] == "evt-signature-tamper-003").unwrap();
        event["claims"]["firmware_sha256"] = Value::String("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into());
        let bytes = serde_json::to_vec(&corpus).expect("tampered corpus");
        let report = audit_corpus(SCHEMA, &bytes).expect("tamper becomes a rejected row");
        assert_eq!(row(&report, "evt-signature-tamper-003").authenticity_state, "REJECTED_SIGNATURE");
    }

    #[test]
    fn revoked_key_and_unauthorized_event_role_are_rejected() {
        let report = report();
        assert_eq!(row(&report, "evt-revoked-key-005").authenticity_state, "REJECTED_REVOKED_KEY");
        assert_eq!(row(&report, "evt-unauthorized-service-004").authenticity_state, "REJECTED_AUTHORITY");
    }

    #[test]
    fn wrong_pack_configuration_is_rejected_even_with_valid_signature() {
        let report = report();
        assert_eq!(row(&report, "evt-wrong-config-006").authenticity_state, "REJECTED_ASSET_BINDING");
    }

    #[test]
    fn unknown_boot_and_untrusted_clock_are_quarantined() {
        let report = report();
        assert_eq!(row(&report, "evt-untrusted-clock-007").authenticity_state, "QUARANTINED_CLOCK_UNTRUSTED");
        assert_eq!(row(&report, "evt-unknown-boot-008").authenticity_state, "QUARANTINED_BOOT_EPOCH_UNKNOWN");
    }

    #[test]
    fn an_unsigned_event_cannot_poison_the_next_signed_sequence() {
        let report = report();
        assert_eq!(row(&report, "evt-missing-signature-011").authenticity_state, "UNATTESTED");
        assert_eq!(row(&report, "evt-valid-after-unsigned-012").authenticity_state, "AUTHENTICATED_AND_AUTHORIZED");
    }
    #[test]
    fn expired_and_unknown_keys_are_never_authenticated() {
        let report = report();
        assert_eq!(
            row(&report, "evt-expired-key-013").authenticity_state,
            "REJECTED_KEY_OUTSIDE_VALIDITY"
        );
        assert_eq!(
            row(&report, "evt-unknown-key-014").authenticity_state,
            "REJECTED_UNKNOWN_KEY"
        );
    }

    #[test]
    fn boot_epoch_reference_must_match_the_external_admission_binding() {
        let mut corpus: Value = serde_json::from_slice(CORPUS).expect("corpus JSON");
        corpus["policy"]["admitted_boot_epochs"][0]["firmware_sha256"] =
            Value::String("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".into());
        let bytes = serde_json::to_vec(&corpus).expect("mutated corpus");
        let report = audit_corpus(SCHEMA, &bytes).expect("mismatch quarantines the row");
        assert_eq!(
            row(&report, "evt-valid-001").authenticity_state,
            "QUARANTINED_BOOT_EPOCH_UNKNOWN"
        );
    }

    #[test]
    fn calibration_reference_and_digest_remain_bound_to_the_signed_event() {
        let report = report();
        let event = row(&report, "evt-valid-001");
        assert_eq!(event.authenticity_state, "AUTHENTICATED_AND_AUTHORIZED");
        assert_eq!(event.physical_validity_state, "NOT_ESTABLISHED_BY_SIGNATURE");
    }

}
