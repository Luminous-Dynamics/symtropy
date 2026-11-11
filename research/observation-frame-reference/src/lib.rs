//! Bevy-free reference algebra for the V0.1 observation-frame contract.
//!
//! This package validates a finite synthetic reference representation. It does
//! not authenticate observations, qualify sensors, or establish physical truth.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_INPUT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_ENTITIES: usize = 100_000;
pub const MAX_OBSERVABLES_PER_ENTITY: usize = 10_000;
pub const MAX_TOTAL_OBSERVABLES: usize = 1_000_000;
pub const MAX_EVENTS: usize = 1_000_000;

const FRAME_HASH_DOMAIN: &[u8] = b"symtropy-observation-frame-reference-v1\0";
const EVENT_HASH_DOMAIN: &[u8] = b"symtropy-observation-event-stream-v1\0";
const OBSERVABLE_SET_HASH_DOMAIN: &[u8] = b"symtropy-observable-set-profile-v1\0";
const CLAIM_CEILING: &str = concat!(
    "Finite structural validation and canonical identity of the supplied reference ",
    "representation only; no source authentication, sensor calibration, freshness, ",
    "numerical health, model credibility, physical truth, engineering acceptance, ",
    "safety, certification, or actuation authority.",
);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservationFrameInputV1 {
    pub schema_version: u32,
    pub identity: FrameIdentityV1,
    pub model_time: ModelTimeV1,
    pub entities: Vec<ObservationEntityV1>,
    /// Source order is semantic. This list is never sorted by the oracle.
    pub event_stream: Option<Vec<ObservationEventV1>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FrameIdentityV1 {
    pub scenario_id: String,
    pub scenario_revision_sha256: String,
    pub run_id: String,
    /// Full lowercase Git SHA-1 revision.
    pub source_revision: String,
    pub branch_id: String,
    pub replay_identity_sha256: String,
    pub initial_state_sha256: String,
    pub simulation_state_sha256: String,
    pub model_profile_sha256: String,
    pub observable_set_sha256: String,
    pub adapter_version: String,
    pub simulation_tick: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModelTimeV1 {
    pub clock_id: String,
    pub timebase_id: String,
    pub ticks: u64,
    /// Exact rational scale: one model tick equals this many nanoseconds.
    pub nanoseconds_per_tick: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservationEntityV1 {
    pub scenario_entity_id: String,
    /// True only when the entity is included in a claim-bearing observation scope.
    pub claim_bearing: bool,
    pub coordinate_frame_id: String,
    pub binding_receipt_sha256: Option<String>,
    pub runtime_subject: Option<RuntimeSubjectRefV1>,
    pub observables: Vec<ObservableV1>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeSubjectRefV1 {
    pub physical_authority_id: String,
    pub world_generation_id: String,
    pub net_id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservableV1 {
    pub observable_id: String,
    pub quantity_id: String,
    pub unit: String,
    pub value: Option<ObservationValueV1>,
    pub source_class: ObservationSourceClass,
    pub source_ref_sha256: Option<String>,
    pub source_time: Option<SourceTimeV1>,
    pub uncertainty: Option<UncertaintyIntervalV1>,
    pub unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ObservationValueV1 {
    Boolean(bool),
    UnsignedInteger(u64),
    Utf8Text(String),
    /// Exactly 16 lowercase hexadecimal digits representing raw IEEE-754 bits.
    Binary64Bits(String),
    /// Each member is exactly 16 lowercase hexadecimal digits.
    VectorBinary64Bits(Vec<String>),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ObservationSourceClass {
    PhysicalMeasurement,
    CalibratedMeasurement,
    Derived,
    Inferred,
    Synthetic,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceTimeV1 {
    pub clock_id: String,
    pub timestamp_ticks: u64,
    pub validity_window_ticks: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UncertaintyIntervalV1 {
    pub lower_binary64_bits: String,
    pub upper_binary64_bits: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservationEventV1 {
    pub sequence: u64,
    pub event_id: String,
    pub event_kind: String,
    pub scenario_entity_id: Option<String>,
    pub payload_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FrameFinding {
    EventOrderUnavailable,
    NonFiniteObservationValue(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservationFrameReceiptV1 {
    pub schema_version: u32,
    pub frame_sha256: String,
    pub event_order_sha256: Option<String>,
    pub entity_count: usize,
    pub observable_count: usize,
    pub event_count: usize,
    pub findings: Vec<FrameFinding>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    UnsupportedSchema(u32),
    EmptyField(&'static str),
    InvalidSha256(&'static str),
    InvalidSourceRevision,
    InvalidModelTime,
    TooManyEntities,
    TooManyObservables,
    TooManyEvents,
    DuplicateEntityId(String),
    DuplicateObservableId(String),
    DuplicateEventId(String),
    DuplicateRuntimeSubject(String),
    InvalidRuntimeSubject,
    ObservableSetDigestMismatch,
    ClaimBearingSubjectMissing(String),
    InvalidObservationDisposition(String),
    InvalidBinary64Bits(String),
    InvalidUncertainty(String),
    NonMonotonicEventSequence,
    UnknownEventSubject(String),
    Serialization(String),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for FrameError {}

/// Validate and canonicalize one reference frame, then return its identity receipt.
///
/// Entity and observable sets are sorted by stable semantic IDs. The optional
/// event stream is retained byte-semantically in caller-provided order.
pub fn compile_observation_frame(
    input: ObservationFrameInputV1,
) -> Result<ObservationFrameReceiptV1, FrameError> {
    validate_frame(&input)?;

    let entity_count = input.entities.len();
    let observable_count = input.entities.iter().map(|entity| entity.observables.len()).sum();
    let event_count = input.event_stream.as_ref().map_or(0, Vec::len);

    let mut canonical = input;
    canonical.entities.sort_by(|left, right| {
        left.scenario_entity_id.cmp(&right.scenario_entity_id)
    });
    for entity in &mut canonical.entities {
        entity.observables.sort_by(|left, right| {
            left.observable_id.cmp(&right.observable_id)
        });
    }

    let event_order_sha256 = canonical
        .event_stream
        .as_ref()
        .map(|events| hash_serialized(EVENT_HASH_DOMAIN, events))
        .transpose()?;
    let frame_sha256 = hash_serialized(FRAME_HASH_DOMAIN, &canonical)?;

    let mut findings = Vec::new();
    if event_order_sha256.is_none() {
        findings.push(FrameFinding::EventOrderUnavailable);
    }
    for entity in &canonical.entities {
        for observable in &entity.observables {
            if value_has_non_finite_bits(observable.value.as_ref())? {
                findings.push(FrameFinding::NonFiniteObservationValue(format!(
                    "{}:{}",
                    entity.scenario_entity_id, observable.observable_id
                )));
            }
        }
    }
    findings.sort();

    Ok(ObservationFrameReceiptV1 {
        schema_version: SCHEMA_VERSION,
        frame_sha256,
        event_order_sha256,
        entity_count,
        observable_count,
        event_count,
        findings,
        claim_ceiling: CLAIM_CEILING.to_string(),
    })
}

fn validate_frame(input: &ObservationFrameInputV1) -> Result<(), FrameError> {
    if input.schema_version != SCHEMA_VERSION {
        return Err(FrameError::UnsupportedSchema(input.schema_version));
    }

    let identity = &input.identity;
    require_text(&identity.scenario_id, "scenario_id")?;
    require_sha256(&identity.scenario_revision_sha256, "scenario_revision_sha256")?;
    require_text(&identity.run_id, "run_id")?;
    validate_source_revision(&identity.source_revision)?;
    require_text(&identity.branch_id, "branch_id")?;
    require_sha256(&identity.replay_identity_sha256, "replay_identity_sha256")?;
    require_sha256(&identity.initial_state_sha256, "initial_state_sha256")?;
    require_sha256(&identity.simulation_state_sha256, "simulation_state_sha256")?;
    require_sha256(&identity.model_profile_sha256, "model_profile_sha256")?;
    require_sha256(&identity.observable_set_sha256, "observable_set_sha256")?;
    require_text(&identity.adapter_version, "adapter_version")?;
    require_text(&input.model_time.clock_id, "model_time.clock_id")?;
    require_text(&input.model_time.timebase_id, "model_time.timebase_id")?;
    if input.model_time.nanoseconds_per_tick == 0 {
        return Err(FrameError::InvalidModelTime);
    }
    if input.entities.is_empty() || input.entities.len() > MAX_ENTITIES {
        return Err(FrameError::TooManyEntities);
    }

    let entity_ids: BTreeSet<&str> = input
        .entities
        .iter()
        .map(|entity| entity.scenario_entity_id.as_str())
        .collect();
    if entity_ids.len() != input.entities.len() {
        let mut seen = BTreeSet::new();
        let duplicate = input.entities.iter()
            .find(|entity| !seen.insert(entity.scenario_entity_id.as_str()))
            .map_or("<unknown>", |entity| entity.scenario_entity_id.as_str());
        return Err(FrameError::DuplicateEntityId(duplicate.to_string()));
    }

    let mut runtime_subjects = BTreeSet::new();
    let mut observable_total = 0usize;
    for entity in &input.entities {
        require_text(&entity.scenario_entity_id, "scenario_entity_id")?;
        require_text(&entity.coordinate_frame_id, "coordinate_frame_id")?;
        if let Some(binding) = entity.binding_receipt_sha256.as_deref() {
            require_sha256(binding, "binding_receipt_sha256")?;
        }
        if entity.claim_bearing
            && (entity.runtime_subject.is_none() || entity.binding_receipt_sha256.is_none())
        {
            return Err(FrameError::ClaimBearingSubjectMissing(
                entity.scenario_entity_id.clone(),
            ));
        }
        if let Some(subject) = &entity.runtime_subject {
            require_text(&subject.physical_authority_id, "physical_authority_id")?;
            require_text(&subject.world_generation_id, "world_generation_id")?;
            if entity.binding_receipt_sha256.is_none() {
                return Err(FrameError::InvalidRuntimeSubject);
            }
            let key = (
                subject.physical_authority_id.as_str(),
                subject.world_generation_id.as_str(),
                subject.net_id,
            );
            if !runtime_subjects.insert(key) {
                return Err(FrameError::DuplicateRuntimeSubject(format!(
                    "{}:{}:{}",
                    subject.physical_authority_id,
                    subject.world_generation_id,
                    subject.net_id
                )));
            }
        }

        if entity.observables.is_empty()
            || entity.observables.len() > MAX_OBSERVABLES_PER_ENTITY
        {
            return Err(FrameError::TooManyObservables);
        }
        observable_total = observable_total
            .checked_add(entity.observables.len())
            .ok_or(FrameError::TooManyObservables)?;
        if observable_total > MAX_TOTAL_OBSERVABLES {
            return Err(FrameError::TooManyObservables);
        }

        let mut observable_ids = BTreeSet::new();
        for observable in &entity.observables {
            require_text(&observable.observable_id, "observable_id")?;
            require_text(&observable.quantity_id, "quantity_id")?;
            require_text(&observable.unit, "unit")?;
            if !observable_ids.insert(observable.observable_id.as_str()) {
                return Err(FrameError::DuplicateObservableId(format!(
                    "{}:{}",
                    entity.scenario_entity_id, observable.observable_id
                )));
            }
            if let Some(source_ref) = observable.source_ref_sha256.as_deref() {
                require_sha256(source_ref, "source_ref_sha256")?;
            }
            if let Some(source_time) = &observable.source_time {
                require_text(&source_time.clock_id, "source_time.clock_id")?;
            }
            if let Some(uncertainty) = &observable.uncertainty {
                validate_uncertainty(uncertainty)?;
            }
            validate_observation_disposition(observable)?;
            validate_value_encoding(observable.value.as_ref())?;
        }
    }

    if let Some(events) = input.event_stream.as_ref() {
        if events.len() > MAX_EVENTS {
            return Err(FrameError::TooManyEvents);
        }
        let mut event_ids = BTreeSet::new();
        let mut previous_sequence = None;
        for event in events {
            require_text(&event.event_id, "event_id")?;
            require_text(&event.event_kind, "event_kind")?;
            require_sha256(&event.payload_sha256, "event.payload_sha256")?;
            if !event_ids.insert(event.event_id.as_str()) {
                return Err(FrameError::DuplicateEventId(event.event_id.clone()));
            }
            if previous_sequence.is_some_and(|previous| event.sequence <= previous) {
                return Err(FrameError::NonMonotonicEventSequence);
            }
            previous_sequence = Some(event.sequence);
            if let Some(subject) = event.scenario_entity_id.as_deref() {
                if !entity_ids.contains(subject) {
                    return Err(FrameError::UnknownEventSubject(subject.to_string()));
                }
            }
        }
    }

    let expected_observable_set_sha256 = compute_observable_set_sha256(&input.entities)?;
    if expected_observable_set_sha256 != identity.observable_set_sha256 {
        return Err(FrameError::ObservableSetDigestMismatch);
    }

    Ok(())
}

fn validate_observation_disposition(observable: &ObservableV1) -> Result<(), FrameError> {
    match observable.source_class {
        ObservationSourceClass::Unavailable => {
            if observable.value.is_some()
                || observable.unavailable_reason.as_deref().is_none_or(str::is_empty)
            {
                return Err(FrameError::InvalidObservationDisposition(
                    observable.observable_id.clone(),
                ));
            }
        }
        ObservationSourceClass::PhysicalMeasurement
        | ObservationSourceClass::CalibratedMeasurement => {
            if observable.value.is_none()
                || observable.unavailable_reason.is_some()
                || observable.source_ref_sha256.is_none()
            {
                return Err(FrameError::InvalidObservationDisposition(
                    observable.observable_id.clone(),
                ));
            }
        }
        ObservationSourceClass::Derived
        | ObservationSourceClass::Inferred
        | ObservationSourceClass::Synthetic => {
            if observable.value.is_none() || observable.unavailable_reason.is_some() {
                return Err(FrameError::InvalidObservationDisposition(
                    observable.observable_id.clone(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_value_encoding(value: Option<&ObservationValueV1>) -> Result<(), FrameError> {
    match value {
        Some(ObservationValueV1::Utf8Text(text)) if text.len() > 16_384 => {
            Err(FrameError::InvalidBinary64Bits("text value exceeds bound".to_string()))
        }
        Some(ObservationValueV1::Binary64Bits(bits)) => {
            parse_binary64_bits(bits).map(|_| ())
        }
        Some(ObservationValueV1::VectorBinary64Bits(values)) => {
            if values.is_empty() || values.len() > 4096 {
                return Err(FrameError::InvalidBinary64Bits(
                    "binary64 vector length is outside bounds".to_string(),
                ));
            }
            for bits in values {
                parse_binary64_bits(bits)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn value_has_non_finite_bits(value: Option<&ObservationValueV1>) -> Result<bool, FrameError> {
    let is_non_finite = |bits: &str| -> Result<bool, FrameError> {
        Ok(!f64::from_bits(parse_binary64_bits(bits)?).is_finite())
    };
    match value {
        Some(ObservationValueV1::Binary64Bits(bits)) => is_non_finite(bits),
        Some(ObservationValueV1::VectorBinary64Bits(values)) => {
            for bits in values {
                if is_non_finite(bits)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn validate_uncertainty(interval: &UncertaintyIntervalV1) -> Result<(), FrameError> {
    let lower = f64::from_bits(parse_binary64_bits(&interval.lower_binary64_bits)?);
    let upper = f64::from_bits(parse_binary64_bits(&interval.upper_binary64_bits)?);
    if !lower.is_finite() || !upper.is_finite() || lower > upper {
        return Err(FrameError::InvalidUncertainty(
            "interval bounds must be finite and ordered".to_string(),
        ));
    }
    Ok(())
}

fn parse_binary64_bits(value: &str) -> Result<u64, FrameError> {
    if value.len() != 16
        || !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(FrameError::InvalidBinary64Bits(value.to_string()));
    }
    u64::from_str_radix(value, 16)
        .map_err(|_| FrameError::InvalidBinary64Bits(value.to_string()))
}

fn validate_source_revision(value: &str) -> Result<(), FrameError> {
    if value.len() != 40
        || !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(FrameError::InvalidSourceRevision);
    }
    Ok(())
}

fn require_sha256(value: &str, field: &'static str) -> Result<(), FrameError> {
    if value.len() != 64
        || !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(FrameError::InvalidSha256(field));
    }
    Ok(())
}

fn require_text(value: &str, field: &'static str) -> Result<(), FrameError> {
    if value.trim().is_empty() || value.len() > 4096 {
        return Err(FrameError::EmptyField(field));
    }
    Ok(())
}

#[derive(Serialize)]
struct ObservableSetEntryRef<'a> {
    scenario_entity_id: &'a str,
    coordinate_frame_id: &'a str,
    observable_id: &'a str,
    quantity_id: &'a str,
    unit: &'a str,
}

/// Compute a canonical digest for declared observable membership.
///
/// The digest binds semantic channel membership and units, not observed values
/// or source classes. An unavailable channel remains a member of the set.
pub fn compute_observable_set_sha256(
    entities: &[ObservationEntityV1],
) -> Result<String, FrameError> {
    let mut entries = Vec::new();
    for entity in entities {
        for observable in &entity.observables {
            entries.push(ObservableSetEntryRef {
                scenario_entity_id: &entity.scenario_entity_id,
                coordinate_frame_id: &entity.coordinate_frame_id,
                observable_id: &observable.observable_id,
                quantity_id: &observable.quantity_id,
                unit: &observable.unit,
            });
        }
    }
    entries.sort_by(|left, right| {
        (
            left.scenario_entity_id,
            left.observable_id,
            left.quantity_id,
            left.unit,
            left.coordinate_frame_id,
        )
            .cmp(&(
                right.scenario_entity_id,
                right.observable_id,
                right.quantity_id,
                right.unit,
                right.coordinate_frame_id,
            ))
    });
    hash_serialized(OBSERVABLE_SET_HASH_DOMAIN, &entries)
}

fn hash_serialized<T: Serialize>(domain: &[u8], value: &T) -> Result<String, FrameError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| FrameError::Serialization(error.to_string()))?;
    let mut material = Vec::with_capacity(domain.len() + bytes.len());
    material.extend_from_slice(domain);
    material.extend_from_slice(&bytes);
    let digest = Sha256::digest(&material);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ObservationFrameInputV1 {
        serde_json::from_str(include_str!("../fixtures/machine-test-01.json"))
            .expect("reference fixture parses")
    }

    #[test]
    fn valid_reference_fixture_compiles_with_event_commitment() {
        let receipt = compile_observation_frame(fixture()).expect("valid fixture");
        assert_eq!(receipt.schema_version, SCHEMA_VERSION);
        assert_eq!(receipt.entity_count, 2);
        assert_eq!(receipt.observable_count, 4);
        assert_eq!(receipt.event_count, 2);
        assert!(receipt.event_order_sha256.is_some());
        assert!(receipt.findings.is_empty());
    }

    #[test]
    fn observable_set_digest_matches_fixed_known_answer() {
        let input = fixture();
        assert_eq!(
            compute_observable_set_sha256(&input.entities).expect("observable set"),
            "49f7e8d90e39e2913f695e64e59c4a6c5ee0ffd3137d0a5b7c5a892b46fa4ae4"
        );
    }

    #[test]
    fn observable_set_profile_is_independent_of_measurement_values() {
        let mut input = fixture();
        let original =
            compute_observable_set_sha256(&input.entities).expect("original profile");
        input.entities[0].observables[0].value = Some(
            ObservationValueV1::Binary64Bits("4000000000000000".to_string())
        );
        assert_eq!(
            original,
            compute_observable_set_sha256(&input.entities).expect("changed value profile")
        );
    }

    #[test]
    fn unavailable_channel_remains_a_member_of_the_observable_set() {
        let mut input = fixture();
        let original =
            compute_observable_set_sha256(&input.entities).expect("original profile");
        let observation = &mut input.entities[0].observables[0];
        observation.source_class = ObservationSourceClass::Unavailable;
        observation.value = None;
        observation.source_ref_sha256 = None;
        observation.uncertainty = None;
        observation.unavailable_reason = Some("sensor disconnected".to_string());
        assert_eq!(
            original,
            compute_observable_set_sha256(&input.entities).expect("unavailable channel profile")
        );
        assert!(compile_observation_frame(input).is_ok());
    }

    #[test]
    fn observable_set_digest_rejects_channel_membership_or_unit_mismatch() {
        let mut input = fixture();
        input.entities[0].observables[0].unit = "cm".to_string();
        assert_eq!(
            compile_observation_frame(input).unwrap_err(),
            FrameError::ObservableSetDigestMismatch
        );
    }

    #[test]
    fn observable_set_digest_is_permutation_invariant() {
        let mut input = fixture();
        let expected =
            compute_observable_set_sha256(&input.entities).expect("original profile");
        input.entities.reverse();
        for entity in &mut input.entities {
            entity.observables.reverse();
        }
        assert_eq!(
            expected,
            compute_observable_set_sha256(&input.entities).expect("permuted profile")
        );
    }

    #[test]
    fn frame_digest_matches_fixed_known_answer() {
        let receipt = compile_observation_frame(fixture()).expect("known-answer frame");
        assert_eq!(
            receipt.frame_sha256,
            "fc5f9e54c629e395ab764f683a73e2be8f99f4c3f9e456fe52f81ce4729863fd"
        );
    }

    #[test]
    fn entity_order_does_not_change_frame_identity() {
        let mut input = fixture();
        let expected = compile_observation_frame(input.clone()).expect("original").frame_sha256;
        input.entities.reverse();
        assert_eq!(
            expected,
            compile_observation_frame(input).expect("permuted").frame_sha256
        );
    }

    #[test]
    fn observable_order_does_not_change_frame_identity() {
        let mut input = fixture();
        let expected = compile_observation_frame(input.clone()).expect("original").frame_sha256;
        input.entities[0].observables.reverse();
        assert_eq!(
            expected,
            compile_observation_frame(input).expect("permuted").frame_sha256
        );
    }

    #[test]
    fn event_order_digest_matches_fixed_known_answer() {
        let receipt = compile_observation_frame(fixture()).expect("known-answer fixture");
        assert_eq!(
            receipt.event_order_sha256.as_deref(),
            Some("f161695abcfd5dbf2275c1e9a2ff7fbf221fb2f36cc44e49230c47cfd89cf2c0")
        );
    }

    #[test]
    fn source_event_order_changes_event_and_frame_identity() {
        let input = fixture();
        let original = compile_observation_frame(input.clone()).expect("original");
        let mut reordered = input;
        let events = reordered.event_stream.as_mut().expect("events");
        events.reverse();
        events[0].sequence = 1;
        events[1].sequence = 2;
        let changed = compile_observation_frame(reordered).expect("reordered events");
        assert_ne!(original.event_order_sha256, changed.event_order_sha256);
        assert_ne!(original.frame_sha256, changed.frame_sha256);
    }

    #[test]
    fn absent_event_stream_is_not_synthesized_from_observables() {
        let mut input = fixture();
        input.event_stream = None;
        let receipt = compile_observation_frame(input).expect("frame without event stream");
        assert_eq!(receipt.event_order_sha256, None);
        assert!(receipt.findings.contains(&FrameFinding::EventOrderUnavailable));
    }

    #[test]
    fn unavailable_is_distinct_from_an_explicit_numeric_zero() {
        let mut input = fixture();
        input.entities[0].observables[0].source_class = ObservationSourceClass::Unavailable;
        input.entities[0].observables[0].value = None;
        input.entities[0].observables[0].source_ref_sha256 = None;
        input.entities[0].observables[0].unavailable_reason =
            Some("sensor not present".to_string());
        let unavailable =
            compile_observation_frame(input.clone()).expect("unavailable observation");
        input.entities[0].observables[0].source_class = ObservationSourceClass::Synthetic;
        input.entities[0].observables[0].value = Some(ObservationValueV1::Binary64Bits(
            "0000000000000000".to_string(),
        ));
        input.entities[0].observables[0].unavailable_reason = None;
        let zero = compile_observation_frame(input).expect("zero value");
        assert_ne!(unavailable.frame_sha256, zero.frame_sha256);
    }

    #[test]
    fn unavailable_observation_cannot_carry_a_value() {
        let mut input = fixture();
        let observation = &mut input.entities[0].observables[0];
        observation.source_class = ObservationSourceClass::Unavailable;
        observation.unavailable_reason = Some("source omitted".to_string());
        assert!(matches!(
            compile_observation_frame(input),
            Err(FrameError::InvalidObservationDisposition(_))
        ));
    }

    #[test]
    fn unavailable_observation_requires_a_reason() {
        let mut input = fixture();
        let observation = &mut input.entities[0].observables[0];
        observation.source_class = ObservationSourceClass::Unavailable;
        observation.value = None;
        observation.source_ref_sha256 = None;
        observation.unavailable_reason = None;
        assert!(matches!(
            compile_observation_frame(input),
            Err(FrameError::InvalidObservationDisposition(_))
        ));
    }

    #[test]
    fn physical_measurements_require_a_source_reference() {
        let mut input = fixture();
        input.entities[0].observables[0].source_class =
            ObservationSourceClass::PhysicalMeasurement;
        input.entities[0].observables[0].source_ref_sha256 = None;
        assert!(matches!(
            compile_observation_frame(input),
            Err(FrameError::InvalidObservationDisposition(_))
        ));
    }

    #[test]
    fn synthetic_source_class_is_preserved_and_not_promoted() {
        let input = fixture();
        let before = serde_json::to_value(&input).expect("serialize input");
        let _receipt = compile_observation_frame(input.clone()).expect("compile");
        let after = serde_json::to_value(&input).expect("serialize input again");
        assert_eq!(before, after);
        assert!(matches!(
            input.entities[0].observables[0].source_class,
            ObservationSourceClass::Synthetic
        ));
    }

    #[test]
    fn presentation_profile_and_capture_fields_are_not_observation_fields() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/machine-test-01.json"))
                .expect("fixture JSON");
        value["render_profile_id"] = serde_json::json!("cinematic");
        value["capture_path"] = serde_json::json!("/tmp/capture.png");
        value["render_resolution"] = serde_json::json!([3840, 2160]);
        assert!(serde_json::from_value::<ObservationFrameInputV1>(value).is_err());
    }

    #[test]
    fn unknown_transient_body_handle_field_is_rejected() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/machine-test-01.json"))
                .expect("fixture JSON");
        value["entities"][0]["body_handle"] = serde_json::json!(42);
        assert!(serde_json::from_value::<ObservationFrameInputV1>(value).is_err());
    }

    #[test]
    fn malformed_source_digest_rejects() {
        let mut input = fixture();
        input.identity.model_profile_sha256 = "not-a-digest".to_string();
        assert_eq!(
            compile_observation_frame(input).unwrap_err(),
            FrameError::InvalidSha256("model_profile_sha256")
        );
    }

    #[test]
    fn empty_unit_rejects() {
        let mut input = fixture();
        input.entities[0].observables[0].unit = " ".to_string();
        assert_eq!(
            compile_observation_frame(input).unwrap_err(),
            FrameError::EmptyField("unit")
        );
    }

    #[test]
    fn unknown_event_subject_rejects() {
        let mut input = fixture();
        input.event_stream.as_mut().expect("events")[0].scenario_entity_id =
            Some("machine-test-01/not-in-this-frame".to_string());
        assert_eq!(
            compile_observation_frame(input).unwrap_err(),
            FrameError::UnknownEventSubject("machine-test-01/not-in-this-frame".to_string())
        );
    }

    #[test]
    fn zero_model_time_scale_rejects() {
        let mut input = fixture();
        input.model_time.nanoseconds_per_tick = 0;
        assert_eq!(
            compile_observation_frame(input).unwrap_err(),
            FrameError::InvalidModelTime
        );
    }

    #[test]
    fn duplicate_entity_ids_reject() {
        let mut input = fixture();
        input.entities[1].scenario_entity_id = input.entities[0].scenario_entity_id.clone();
        assert!(matches!(
            compile_observation_frame(input),
            Err(FrameError::DuplicateEntityId(_))
        ));
    }

    #[test]
    fn duplicate_observable_ids_reject() {
        let mut input = fixture();
        input.entities[0].observables[1].observable_id =
            input.entities[0].observables[0].observable_id.clone();
        assert!(matches!(
            compile_observation_frame(input),
            Err(FrameError::DuplicateObservableId(_))
        ));
    }

    #[test]
    fn duplicate_event_ids_reject() {
        let mut input = fixture();
        let events = input.event_stream.as_mut().expect("events");
        events[1].event_id = events[0].event_id.clone();
        assert!(matches!(
            compile_observation_frame(input),
            Err(FrameError::DuplicateEventId(_))
        ));
    }

    #[test]
    fn event_sequence_must_be_strictly_increasing() {
        let mut input = fixture();
        let events = input.event_stream.as_mut().expect("events");
        events[1].sequence = events[0].sequence;
        assert_eq!(
            compile_observation_frame(input).unwrap_err(),
            FrameError::NonMonotonicEventSequence
        );
    }

    #[test]
    fn malformed_source_revision_rejects() {
        let mut input = fixture();
        input.identity.source_revision = "not-a-commit".to_string();
        assert_eq!(
            compile_observation_frame(input).unwrap_err(),
            FrameError::InvalidSourceRevision
        );
    }

    #[test]
    fn unsupported_schema_rejects() {
        let mut input = fixture();
        input.schema_version += 1;
        assert_eq!(
            compile_observation_frame(input).unwrap_err(),
            FrameError::UnsupportedSchema(SCHEMA_VERSION + 1)
        );
    }

    #[test]
    fn inverted_uncertainty_interval_rejects() {
        let mut input = fixture();
        input.entities[0].observables[0].uncertainty = Some(UncertaintyIntervalV1 {
            lower_binary64_bits: "3ff0000000000000".to_string(),
            upper_binary64_bits: "0000000000000000".to_string(),
        });
        assert!(matches!(
            compile_observation_frame(input),
            Err(FrameError::InvalidUncertainty(_))
        ));
    }

    #[test]
    fn non_finite_observation_is_preserved_and_reported() {
        let mut input = fixture();
        input.entities[0].observables[0].value = Some(ObservationValueV1::Binary64Bits(
            "7ff8000000000001".to_string(),
        ));
        let receipt = compile_observation_frame(input).expect("raw NaN remains representable");
        assert!(receipt.findings.iter().any(|finding| matches!(
            finding,
            FrameFinding::NonFiniteObservationValue(_)
        )));
    }

    #[test]
    fn signed_zero_and_nan_payload_bits_remain_identity_distinct() {
        let mut positive_zero = fixture();
        positive_zero.entities[0].observables[0].value = Some(
            ObservationValueV1::Binary64Bits("0000000000000000".to_string())
        );
        let mut negative_zero = positive_zero.clone();
        negative_zero.entities[0].observables[0].value = Some(
            ObservationValueV1::Binary64Bits("8000000000000000".to_string())
        );
        let mut other_nan = negative_zero.clone();
        other_nan.entities[0].observables[0].value = Some(
            ObservationValueV1::Binary64Bits("7ff8000000000002".to_string())
        );
        let positive = compile_observation_frame(positive_zero).expect("positive zero");
        let negative = compile_observation_frame(negative_zero).expect("negative zero");
        let nan = compile_observation_frame(other_nan).expect("NaN payload");
        assert_ne!(positive.frame_sha256, negative.frame_sha256);
        assert_ne!(negative.frame_sha256, nan.frame_sha256);
    }

    #[test]
    fn claim_bearing_entity_requires_a_stable_subject_and_binding_receipt() {
        let mut input = fixture();
        input.entities[0].runtime_subject = None;
        assert!(matches!(
            compile_observation_frame(input),
            Err(FrameError::ClaimBearingSubjectMissing(_))
        ));
    }

    #[test]
    fn changed_observable_set_profile_changes_frame_identity() {
        let input = fixture();
        let original = compile_observation_frame(input.clone()).expect("original");
        let mut changed = input;
        changed.identity.observable_set_sha256 = "7".repeat(64);
        let next = compile_observation_frame(changed).expect("changed observable set");
        assert_ne!(original.frame_sha256, next.frame_sha256);
    }

    #[test]
    fn changed_run_identity_changes_frame_digest() {
        let input = fixture();
        let original = compile_observation_frame(input.clone()).expect("original");
        let mut changed = input;
        changed.identity.run_id = "machine-test-01-run-0002".to_string();
        let next = compile_observation_frame(changed).expect("changed run");
        assert_ne!(original.frame_sha256, next.frame_sha256);
    }

    #[test]
    fn duplicate_runtime_subject_in_one_generation_rejects() {
        let mut input = fixture();
        let duplicate = input.entities[0].runtime_subject.clone().expect("subject");
        input.entities[1].binding_receipt_sha256 = Some("c".repeat(64));
        input.entities[1].runtime_subject = Some(duplicate);
        assert!(matches!(
            compile_observation_frame(input),
            Err(FrameError::DuplicateRuntimeSubject(_))
        ));
    }

    #[test]
    fn same_numeric_net_id_in_a_new_generation_is_distinct() {
        let mut input = fixture();
        let original = compile_observation_frame(input.clone()).expect("original frame");
        let mut subject = input.entities[0].runtime_subject.clone().expect("subject");
        subject.world_generation_id = "world-generation-2".to_string();
        input.entities[1].binding_receipt_sha256 = Some("c".repeat(64));
        input.entities[1].runtime_subject = Some(subject);
        let next = compile_observation_frame(input).expect("new generation frame");
        assert_ne!(original.frame_sha256, next.frame_sha256);
    }

    #[test]
    fn source_clock_identity_survives_round_trip() {
        let input = fixture();
        let bytes = serde_json::to_vec(&input).expect("serialize");
        let restored: ObservationFrameInputV1 =
            serde_json::from_slice(&bytes).expect("deserialize");
        assert_eq!(input, restored);
    }
}
