//! Schema and semantic validation for the frozen synthetic spatial-resilience corpus.
//!
//! This crate does not simulate building physics. The temporal oracle is deliberately
//! small and deterministic: it qualifies event/recovery semantics only.

use jsonschema::validator_for;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const EMBEDDED_MANIFEST: &str = include_str!(
    "../../../docs/research/spatial-design/SPATIAL_RESILIENCE_TRAJECTORY_FIXTURES_V0_1.json"
);
pub const EMBEDDED_SCHEMA: &str = include_str!(
    "../../../docs/research/spatial-design/SPATIAL_RESILIENCE_TRAJECTORY_FIXTURES_V0_1.schema.json"
);

const EXPECTED_CASE_COUNT: usize = 15;
const EXPECTED_INVARIANTS: &[&str] = &[
    "paired_trace_identical",
    "hard_floor_before_pareto",
    "unknown_not_pass",
    "scope_revisions_bind_evidence",
    "explicit_units_required",
    "stable_recovery_window_required",
    "missing_part_blocks_recovery",
    "no_future_information",
    "no_scalar_compensation",
    "uncertainty_tolerance_enforced",
    "resident_priority_preserved",
    "service_access_is_constraint",
    "duplicate_delivery_idempotent",
    "unrelated_change_preserves_current_result",
    "dissent_not_collapsed",
    "solver_claim_ceiling_preserved",
];

const EXPECTED_DIMENSIONS: &[&str] = &[
    "paired_rank_reversal",
    "missing_solver",
    "stale_weather",
    "unit_mismatch",
    "recurring_fault",
    "repair_delay",
    "trace_mismatch",
    "future_information",
    "hard_floor_compensation",
    "uncertainty_tolerance",
    "resident_task_change",
    "repair_access",
    "duplicate_event",
    "scoped_invalidation",
    "resident_dissent",
];

const DISPOSITIONS: &[&str] = &[
    "accept_with_scope",
    "exclude_candidate",
    "blocked_unknown",
    "invalidate_result",
    "reject_input",
    "reject_claim",
    "replay_identically",
    "preserve_dissent",
    "reopen_decision",
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ValidationFailure {
    /// Stable code for automation and regression tests.
    pub code: String,
    /// JSON instance path or a repository-local semantic path.
    pub path: String,
    /// Human-readable detail; do not use this text as a machine identifier.
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationSummary {
    pub benchmark_id: Option<String>,
    pub version: Option<String>,
    pub manifest_sha256: String,
    pub schema_sha256: String,
    pub cases_seen: usize,
    pub invariant_registry_count: usize,
    pub passed: bool,
    pub failures: Vec<ValidationFailure>,
}

impl ValidationSummary {
    fn new(manifest_bytes: &[u8], schema_bytes: &[u8]) -> Self {
        Self {
            benchmark_id: None,
            version: None,
            manifest_sha256: sha256_hex(manifest_bytes),
            schema_sha256: sha256_hex(schema_bytes),
            cases_seen: 0,
            invariant_registry_count: 0,
            passed: false,
            failures: Vec::new(),
        }
    }

    fn fail(&mut self, code: &str, path: impl Into<String>, detail: impl Into<String>) {
        self.failures.push(ValidationFailure {
            code: code.to_owned(),
            path: path.into(),
            detail: detail.into(),
        });
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Validate a manifest against its supplied Draft 2020-12 schema, then run
/// semantic checks that JSON Schema alone does not express for this corpus.
pub fn validate_manifest_bytes(manifest_bytes: &[u8], schema_bytes: &[u8]) -> ValidationSummary {
    let mut summary = ValidationSummary::new(manifest_bytes, schema_bytes);

    let schema_value: Value = match serde_json::from_slice(schema_bytes) {
        Ok(value) => value,
        Err(error) => {
            summary.fail("SCHEMA_JSON_INVALID", "$", error.to_string());
            return summary;
        }
    };

    let manifest_value: Value = match serde_json::from_slice(manifest_bytes) {
        Ok(value) => value,
        Err(error) => {
            summary.fail("MANIFEST_JSON_INVALID", "$", error.to_string());
            return summary;
        }
    };

    if let Some(object) = manifest_value.as_object() {
        summary.benchmark_id = object
            .get("benchmark_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        summary.version = object
            .get("version")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        summary.cases_seen = object
            .get("cases")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        summary.invariant_registry_count = object
            .get("registered_invariant_ids")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
    }

    let validator = match validator_for(&schema_value) {
        Ok(validator) => validator,
        Err(error) => {
            summary.fail("SCHEMA_COMPILE_FAILED", "$", error.to_string());
            return summary;
        }
    };

    for error in validator.iter_errors(&manifest_value) {
        summary.fail(
            "SCHEMA_VALIDATION_FAILED",
            error.instance_path().to_string(),
            error.to_string(),
        );
    }

    semantic_checks(&manifest_value, &mut summary);
    summary.failures.sort_by(|left, right| {
        (&left.path, &left.code, &left.detail).cmp(&(&right.path, &right.code, &right.detail))
    });
    summary.passed = summary.failures.is_empty();
    summary
}

fn semantic_checks(manifest: &Value, summary: &mut ValidationSummary) {
    let Some(root) = manifest.as_object() else {
        summary.fail("MANIFEST_ROOT_NOT_OBJECT", "$", "Expected a JSON object.");
        return;
    };

    let expected_schema_uri = "https://json-schema.org/draft/2020-12/schema";
    if root.get("$schema").and_then(Value::as_str) != Some(expected_schema_uri) {
        summary.fail(
            "SCHEMA_DRAFT_MISMATCH",
            "$.$schema",
            "Manifest must identify JSON Schema Draft 2020-12.",
        );
    }
    if root.get("benchmark_id").and_then(Value::as_str) != Some("SPATIAL-RESILIENCE-TRAJECTORY") {
        summary.fail(
            "BENCHMARK_ID_MISMATCH",
            "$.benchmark_id",
            "Unexpected benchmark identifier.",
        );
    }
    if root.get("version").and_then(Value::as_str) != Some("0.1.0") {
        summary.fail(
            "BENCHMARK_VERSION_MISMATCH",
            "$.version",
            "This verifier freezes version 0.1.0; introduce versioned logic for future contracts.",
        );
    }
    if root.get("required_case_count").and_then(Value::as_u64) != Some(EXPECTED_CASE_COUNT as u64) {
        summary.fail(
            "REQUIRED_CASE_COUNT_MISMATCH",
            "$.required_case_count",
            format!("Expected exactly {EXPECTED_CASE_COUNT} cases."),
        );
    }

    let mut registry = BTreeSet::new();
    if let Some(values) = root
        .get("registered_invariant_ids")
        .and_then(Value::as_array)
    {
        for (index, value) in values.iter().enumerate() {
            if let Some(id) = value.as_str() {
                if !registry.insert(id.to_owned()) {
                    summary.fail(
                        "INVARIANT_REGISTRY_DUPLICATE",
                        format!("$.registered_invariant_ids[{index}]"),
                        format!("Invariant ID {id:?} is registered more than once."),
                    );
                }
            }
        }
        let expected: BTreeSet<String> = EXPECTED_INVARIANTS
            .iter()
            .map(|item| (*item).to_owned())
            .collect();
        if registry != expected {
            summary.fail(
                "INVARIANT_REGISTRY_DRIFT",
                "$.registered_invariant_ids",
                "Invariant registry differs from this verifier's reviewed v0.1 vocabulary.",
            );
        }
    }

    let Some(cases) = root.get("cases").and_then(Value::as_array) else {
        // The JSON Schema error already explains the missing or mistyped array.
        return;
    };

    if cases.len() != EXPECTED_CASE_COUNT {
        summary.fail(
            "CASE_COUNT_MISMATCH",
            "$.cases",
            format!(
                "Expected exactly {EXPECTED_CASE_COUNT} cases, got {}.",
                cases.len()
            ),
        );
    }

    let mut case_ids = BTreeSet::new();
    let mut dimensions = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let path = format!("$.cases[{index}]");
        let Some(case_object) = case.as_object() else {
            continue;
        };

        if let Some(id) = case_object.get("id").and_then(Value::as_str) {
            if !case_ids.insert(id.to_owned()) {
                summary.fail(
                    "CASE_ID_DUPLICATE",
                    format!("{path}.id"),
                    format!("Case ID {id:?} is duplicated."),
                );
            }

            let expected_id = format!("SPR-T-{:03}", index + 1);
            if id != expected_id {
                summary.fail(
                    "CASE_ID_SEQUENCE",
                    format!("{path}.id"),
                    format!("Expected {expected_id:?} at this index, got {id:?}."),
                );
            }
        }

        if let Some(dimension) = case_object.get("dimension").and_then(Value::as_str) {
            if !dimensions.insert(dimension.to_owned()) {
                summary.fail(
                    "CASE_DIMENSION_DUPLICATE",
                    format!("{path}.dimension"),
                    format!("Dimension {dimension:?} is already covered by an earlier case."),
                );
            }
            if !EXPECTED_DIMENSIONS.contains(&dimension) {
                summary.fail(
                    "CASE_DIMENSION_UNKNOWN",
                    format!("{path}.dimension"),
                    format!("Unknown dimension {dimension:?} for benchmark v0.1."),
                );
            }
        }

        if let Some(required) = case_object
            .get("required_invariants")
            .and_then(Value::as_array)
        {
            for (invariant_index, invariant) in required.iter().enumerate() {
                if let Some(invariant_id) = invariant.as_str() {
                    if !registry.contains(invariant_id) {
                        summary.fail(
                            "CASE_INVARIANT_UNREGISTERED",
                            format!("{path}.required_invariants[{invariant_index}]"),
                            format!("Invariant {invariant_id:?} is not in the manifest registry."),
                        );
                    }
                }
            }
        }

        if let Some(negative_control) = case_object
            .get("negative_control")
            .and_then(Value::as_object)
        {
            match negative_control
                .get("expected_disposition")
                .and_then(Value::as_str)
            {
                Some("reject_input" | "reject_claim") => {}
                Some(other) => summary.fail(
                    "NEGATIVE_CONTROL_NOT_FAIL_CLOSED",
                    format!("{path}.negative_control.expected_disposition"),
                    format!("Negative control must reject input or claim, got {other:?}."),
                ),
                None => {}
            }
        }

        if let Some(disposition) = case_object
            .get("expected_disposition")
            .and_then(Value::as_str)
        {
            if !DISPOSITIONS.contains(&disposition) {
                summary.fail(
                    "DISPOSITION_UNKNOWN",
                    format!("{path}.expected_disposition"),
                    format!("Unknown disposition {disposition:?}."),
                );
            }
        }
    }

    let expected_dimensions: BTreeSet<String> = EXPECTED_DIMENSIONS
        .iter()
        .map(|item| (*item).to_owned())
        .collect();
    if dimensions != expected_dimensions {
        summary.fail(
            "DIMENSION_COVERAGE_DRIFT",
            "$.cases",
            "Case dimension set differs from the reviewed v0.1 corpus.",
        );
    }
}

/// Service availability transition used only for a hand-checkable synthetic timeline.
/// Duplicate events with an identical ID, generation and payload are ignored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceEvent {
    pub id: String,
    pub generation: u64,
    pub tick: u32,
    /// Unique within a tick; callers must declare order for simultaneous events.
    pub sequence: u32,
    pub kind: ServiceEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceEventKind {
    Restored,
    Lost,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OracleError {
    EmptyEventId,
    EventPastHorizon,
    ZeroStabilityWindow,
    ConflictingDuplicate { id: String, generation: u64 },
    AmbiguousEventOrder { tick: u32, sequence: u32 },
}

impl std::fmt::Display for OracleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyEventId => write!(formatter, "event ID must not be empty"),
            Self::EventPastHorizon => write!(formatter, "event occurs after the declared horizon"),
            Self::ZeroStabilityWindow => {
                write!(formatter, "stability window must be greater than zero")
            }
            Self::ConflictingDuplicate { id, generation } => {
                write!(
                    formatter,
                    "conflicting duplicate event {id:?}, generation {generation}"
                )
            }
            Self::AmbiguousEventOrder { tick, sequence } => {
                write!(
                    formatter,
                    "event order is ambiguous at tick {tick}, sequence {sequence}"
                )
            }
        }
    }
}

impl std::error::Error for OracleError {}

/// Return the first tick at which service has been continuously available for
/// stability_window ticks. A loss exactly at the candidate recovery tick
/// invalidates that candidate; event ordering at each tick is explicit.
/// Returns None if stable recovery does not occur before the horizon.
pub fn stable_recovery_tick(
    events: &[ServiceEvent],
    initially_available: bool,
    stability_window: u32,
    horizon: u32,
) -> Result<Option<u32>, OracleError> {
    if stability_window == 0 {
        return Err(OracleError::ZeroStabilityWindow);
    }

    let mut unique_events: BTreeMap<(String, u64), ServiceEvent> = BTreeMap::new();
    for event in events {
        if event.id.trim().is_empty() {
            return Err(OracleError::EmptyEventId);
        }
        if event.tick > horizon {
            return Err(OracleError::EventPastHorizon);
        }
        let key = (event.id.clone(), event.generation);
        match unique_events.get(&key) {
            Some(existing) if existing == event => {
                // Idempotent redelivery: ignore the exact same receipt.
            }
            Some(_) => {
                return Err(OracleError::ConflictingDuplicate {
                    id: event.id.clone(),
                    generation: event.generation,
                });
            }
            None => {
                unique_events.insert(key, event.clone());
            }
        }
    }

    let mut ordered_events: Vec<ServiceEvent> = unique_events.into_values().collect();
    ordered_events.sort_by(|left, right| {
        (left.tick, left.sequence, &left.id, left.generation).cmp(&(
            right.tick,
            right.sequence,
            &right.id,
            right.generation,
        ))
    });

    let mut used_ordering_slots = BTreeSet::new();
    for event in &ordered_events {
        if !used_ordering_slots.insert((event.tick, event.sequence)) {
            return Err(OracleError::AmbiguousEventOrder {
                tick: event.tick,
                sequence: event.sequence,
            });
        }
    }

    let mut available_since = initially_available.then_some(0_u32);
    for event in &ordered_events {
        if let Some(start_tick) = available_since {
            if let Some(candidate_tick) = start_tick.checked_add(stability_window) {
                // If the window elapsed strictly before the next event, it is stable.
                // A loss exactly at candidate_tick is processed first and resets it.
                if candidate_tick < event.tick {
                    return Ok(Some(candidate_tick));
                }
            }
        }

        match event.kind {
            ServiceEventKind::Restored => {
                if available_since.is_none() {
                    available_since = Some(event.tick);
                }
            }
            ServiceEventKind::Lost => {
                available_since = None;
            }
        }
    }

    if let Some(start_tick) = available_since {
        if let Some(candidate_tick) = start_tick.checked_add(stability_window) {
            if candidate_tick <= horizon {
                return Ok(Some(candidate_tick));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, tick: u32, sequence: u32, kind: ServiceEventKind) -> ServiceEvent {
        ServiceEvent {
            id: id.to_owned(),
            generation: 1,
            tick,
            sequence,
            kind,
        }
    }

    fn has_code(summary: &ValidationSummary, code: &str) -> bool {
        summary.failures.iter().any(|failure| failure.code == code)
    }

    #[test]
    fn embedded_fixture_manifest_passes_schema_and_semantics() {
        let summary =
            validate_manifest_bytes(EMBEDDED_MANIFEST.as_bytes(), EMBEDDED_SCHEMA.as_bytes());
        assert!(
            summary.passed,
            "unexpected failures: {:?}",
            summary.failures
        );
        assert_eq!(summary.cases_seen, EXPECTED_CASE_COUNT);
        assert_eq!(summary.invariant_registry_count, EXPECTED_INVARIANTS.len());
        assert_eq!(summary.manifest_sha256.len(), 64);
        assert_eq!(summary.schema_sha256.len(), 64);
    }

    #[test]
    fn duplicate_case_id_is_rejected_even_when_objects_differ() {
        let mut manifest: Value = serde_json::from_str(EMBEDDED_MANIFEST).expect("valid manifest");
        let first_id = manifest["cases"][0]["id"]
            .as_str()
            .expect("case ID")
            .to_owned();
        manifest["cases"][1]["id"] = Value::String(first_id);
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutation");
        let summary = validate_manifest_bytes(&bytes, EMBEDDED_SCHEMA.as_bytes());
        assert!(has_code(&summary, "CASE_ID_DUPLICATE"));
    }

    #[test]
    fn duplicated_dimension_is_rejected() {
        let mut manifest: Value = serde_json::from_str(EMBEDDED_MANIFEST).expect("valid manifest");
        let first_dimension = manifest["cases"][0]["dimension"].clone();
        manifest["cases"][1]["dimension"] = first_dimension;
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutation");
        let summary = validate_manifest_bytes(&bytes, EMBEDDED_SCHEMA.as_bytes());
        assert!(has_code(&summary, "CASE_DIMENSION_DUPLICATE"));
    }

    #[test]
    fn negative_control_cannot_claim_success() {
        let mut manifest: Value = serde_json::from_str(EMBEDDED_MANIFEST).expect("valid manifest");
        manifest["cases"][0]["negative_control"]["expected_disposition"] =
            Value::String("accept_with_scope".to_owned());
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutation");
        let summary = validate_manifest_bytes(&bytes, EMBEDDED_SCHEMA.as_bytes());
        assert!(has_code(&summary, "NEGATIVE_CONTROL_NOT_FAIL_CLOSED"));
    }

    #[test]
    fn wrong_invariant_registry_is_rejected() {
        let mut manifest: Value = serde_json::from_str(EMBEDDED_MANIFEST).expect("valid manifest");
        manifest["registered_invariant_ids"][0] = Value::String("made_up_invariant".to_owned());
        let bytes = serde_json::to_vec(&manifest).expect("serialize mutation");
        let summary = validate_manifest_bytes(&bytes, EMBEDDED_SCHEMA.as_bytes());
        assert!(
            has_code(&summary, "SCHEMA_VALIDATION_FAILED")
                || has_code(&summary, "INVARIANT_REGISTRY_DRIFT")
        );
    }

    #[test]
    fn stable_recovery_restarts_after_recurrence() {
        let timeline = [
            event("restore-a", 2, 1, ServiceEventKind::Restored),
            event("loss-a", 4, 1, ServiceEventKind::Lost),
            event("restore-b", 6, 1, ServiceEventKind::Restored),
        ];
        assert_eq!(stable_recovery_tick(&timeline, false, 2, 10), Ok(Some(8)));
    }

    #[test]
    fn loss_at_stability_boundary_prevents_false_recovery() {
        let timeline = [
            event("restore-a", 2, 1, ServiceEventKind::Restored),
            event("loss-a", 4, 1, ServiceEventKind::Lost),
        ];
        assert_eq!(stable_recovery_tick(&timeline, false, 2, 10), Ok(None));
    }

    #[test]
    fn identical_duplicate_event_is_idempotent() {
        let restored = event("restore-a", 2, 1, ServiceEventKind::Restored);
        let timeline = [
            restored.clone(),
            restored,
            event("loss-a", 7, 1, ServiceEventKind::Lost),
        ];
        assert_eq!(stable_recovery_tick(&timeline, false, 2, 10), Ok(Some(4)));
    }

    #[test]
    fn conflicting_duplicate_receipt_is_rejected() {
        let timeline = [
            event("same-id", 2, 1, ServiceEventKind::Restored),
            event("same-id", 3, 1, ServiceEventKind::Restored),
        ];
        assert!(matches!(
            stable_recovery_tick(&timeline, false, 2, 10),
            Err(OracleError::ConflictingDuplicate { .. })
        ));
    }

    #[test]
    fn equal_tick_sequence_collision_is_rejected() {
        let timeline = [
            event("restore-a", 2, 1, ServiceEventKind::Restored),
            event("loss-a", 2, 1, ServiceEventKind::Lost),
        ];
        assert!(matches!(
            stable_recovery_tick(&timeline, false, 2, 10),
            Err(OracleError::AmbiguousEventOrder { .. })
        ));
    }

    #[test]
    fn recovery_beyond_horizon_is_unknown_not_success() {
        let timeline = [event("restore-a", 8, 1, ServiceEventKind::Restored)];
        assert_eq!(stable_recovery_tick(&timeline, false, 4, 10), Ok(None));
    }

    #[test]
    fn zero_stability_window_is_rejected() {
        assert_eq!(
            stable_recovery_tick(&[], false, 0, 10),
            Err(OracleError::ZeroStabilityWindow)
        );
    }

    #[test]
    fn stability_window_overflow_is_not_reported_as_recovery() {
        let timeline = [event(
            "restore-near-max",
            u32::MAX,
            1,
            ServiceEventKind::Restored,
        )];
        assert_eq!(
            stable_recovery_tick(&timeline, false, 1, u32::MAX),
            Ok(None)
        );
    }

    #[test]
    fn events_after_horizon_are_rejected() {
        let timeline = [event("restore-a", 11, 1, ServiceEventKind::Restored)];
        assert_eq!(
            stable_recovery_tick(&timeline, false, 2, 10),
            Err(OracleError::EventPastHorizon)
        );
    }

    #[test]
    fn malformed_json_returns_stable_failure_code() {
        let summary = validate_manifest_bytes(b"{bad", EMBEDDED_SCHEMA.as_bytes());
        assert!(has_code(&summary, "MANIFEST_JSON_INVALID"));
    }
}

pub const EMBEDDED_ORACLE_CORPUS: &str = include_str!(
    "../../../docs/research/spatial-design/SPATIAL_RESILIENCE_TIMELINE_ORACLE_V0_1.json"
);
pub const EMBEDDED_ORACLE_SCHEMA: &str = include_str!(
    "../../../docs/research/spatial-design/SPATIAL_RESILIENCE_TIMELINE_ORACLE_V0_1.schema.json"
);

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TimelineFixtureFailure {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimelineFixtureSummary {
    pub benchmark_id: Option<String>,
    pub version: Option<String>,
    pub corpus_sha256: String,
    pub schema_sha256: String,
    pub cases_seen: usize,
    pub cases_passed: usize,
    pub passed: bool,
    pub failures: Vec<TimelineFixtureFailure>,
}

impl TimelineFixtureSummary {
    fn new(corpus_bytes: &[u8], schema_bytes: &[u8]) -> Self {
        Self {
            benchmark_id: None,
            version: None,
            corpus_sha256: sha256_hex(corpus_bytes),
            schema_sha256: sha256_hex(schema_bytes),
            cases_seen: 0,
            cases_passed: 0,
            passed: false,
            failures: Vec::new(),
        }
    }

    fn fail(&mut self, code: &str, path: impl Into<String>, detail: impl Into<String>) {
        self.failures.push(TimelineFixtureFailure {
            code: code.to_owned(),
            path: path.into(),
            detail: detail.into(),
        });
    }
}

fn oracle_error_code(error: &OracleError) -> &'static str {
    match error {
        OracleError::EmptyEventId => "empty_event_id",
        OracleError::EventPastHorizon => "event_past_horizon",
        OracleError::ZeroStabilityWindow => "zero_stability_window",
        OracleError::ConflictingDuplicate { .. } => "conflicting_duplicate",
        OracleError::AmbiguousEventOrder { .. } => "ambiguous_event_order",
    }
}

fn parse_timeline_event(value: &Value, path: &str) -> Result<ServiceEvent, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{path}: event must be an object"))?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{path}.id: expected string"))?;
    let generation = object
        .get("generation")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{path}.generation: expected unsigned integer"))?;
    let tick = object
        .get("tick")
        .and_then(Value::as_u64)
        .and_then(|number| u32::try_from(number).ok())
        .ok_or_else(|| format!("{path}.tick: expected 32-bit unsigned integer"))?;
    let sequence = object
        .get("sequence")
        .and_then(Value::as_u64)
        .and_then(|number| u32::try_from(number).ok())
        .ok_or_else(|| format!("{path}.sequence: expected 32-bit unsigned integer"))?;
    let kind = match object.get("kind").and_then(Value::as_str) {
        Some("restored") => ServiceEventKind::Restored,
        Some("lost") => ServiceEventKind::Lost,
        _ => return Err(format!("{path}.kind: expected restored or lost")),
    };

    Ok(ServiceEvent {
        id: id.to_owned(),
        generation,
        tick,
        sequence,
        kind,
    })
}

/// Schema-validate and execute a frozen synthetic event corpus against the
/// independent finite-state recovery oracle.
pub fn validate_timeline_fixtures_bytes(
    corpus_bytes: &[u8],
    schema_bytes: &[u8],
) -> TimelineFixtureSummary {
    let mut summary = TimelineFixtureSummary::new(corpus_bytes, schema_bytes);
    let schema_value: Value = match serde_json::from_slice(schema_bytes) {
        Ok(value) => value,
        Err(error) => {
            summary.fail("TIMELINE_SCHEMA_JSON_INVALID", "$", error.to_string());
            return summary;
        }
    };
    let corpus_value: Value = match serde_json::from_slice(corpus_bytes) {
        Ok(value) => value,
        Err(error) => {
            summary.fail("TIMELINE_CORPUS_JSON_INVALID", "$", error.to_string());
            return summary;
        }
    };

    let validator = match validator_for(&schema_value) {
        Ok(validator) => validator,
        Err(error) => {
            summary.fail("TIMELINE_SCHEMA_COMPILE_FAILED", "$", error.to_string());
            return summary;
        }
    };

    let schema_errors: Vec<_> = validator.iter_errors(&corpus_value).collect();
    if !schema_errors.is_empty() {
        for error in schema_errors {
            summary.fail(
                "TIMELINE_SCHEMA_VALIDATION_FAILED",
                error.instance_path().to_string(),
                error.to_string(),
            );
        }
        return summary;
    }

    let Some(root) = corpus_value.as_object() else {
        summary.fail(
            "TIMELINE_CORPUS_ROOT_INVALID",
            "$",
            "Expected a JSON object.",
        );
        return summary;
    };

    summary.benchmark_id = root
        .get("benchmark_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    summary.version = root
        .get("version")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);

    let Some(cases) = root.get("cases").and_then(Value::as_array) else {
        summary.fail(
            "TIMELINE_CASES_MISSING",
            "$.cases",
            "Expected an array of cases.",
        );
        return summary;
    };
    summary.cases_seen = cases.len();
    let mut case_ids = BTreeSet::new();

    for (index, case) in cases.iter().enumerate() {
        let path = format!("$.cases[{index}]");
        let Some(object) = case.as_object() else {
            summary.fail("TIMELINE_CASE_INVALID", &path, "Expected a JSON object.");
            continue;
        };
        let id = object.get("id").and_then(Value::as_str).unwrap_or("");
        if !case_ids.insert(id.to_owned()) {
            summary.fail(
                "TIMELINE_CASE_ID_DUPLICATE",
                format!("{path}.id"),
                format!("Timeline case ID {id:?} is duplicated."),
            );
            continue;
        }
        let expected_id = format!("SPR-O-{:03}", index + 1);
        if id != expected_id {
            summary.fail(
                "TIMELINE_CASE_ID_SEQUENCE",
                format!("{path}.id"),
                format!("Expected {expected_id:?}, got {id:?}."),
            );
            continue;
        }

        let initial_service = object
            .get("initial_service")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let stability_window = object
            .get("stability_window")
            .and_then(Value::as_u64)
            .and_then(|number| u32::try_from(number).ok());
        let horizon = object
            .get("horizon")
            .and_then(Value::as_u64)
            .and_then(|number| u32::try_from(number).ok());
        let events_value = object.get("events").and_then(Value::as_array);
        let (Some(stability_window), Some(horizon), Some(events_value)) =
            (stability_window, horizon, events_value)
        else {
            summary.fail(
                "TIMELINE_CASE_FIELD_INVALID",
                &path,
                "Window, horizon or event list could not be parsed.",
            );
            continue;
        };

        let mut events = Vec::with_capacity(events_value.len());
        let mut event_parse_failed = false;
        for (event_index, event_value) in events_value.iter().enumerate() {
            match parse_timeline_event(event_value, &format!("{path}.events[{event_index}]")) {
                Ok(event) => events.push(event),
                Err(detail) => {
                    summary.fail(
                        "TIMELINE_EVENT_FIELD_INVALID",
                        format!("{path}.events[{event_index}]"),
                        detail,
                    );
                    event_parse_failed = true;
                }
            }
        }
        if event_parse_failed {
            continue;
        }

        let expected_tick = match object.get("expected_stable_recovery_tick") {
            Some(Value::Null) => None,
            Some(value) => value.as_u64().and_then(|number| u32::try_from(number).ok()),
            None => {
                summary.fail(
                    "TIMELINE_EXPECTATION_MISSING",
                    &path,
                    "Expected recovery tick field is required.",
                );
                continue;
            }
        };
        let expected_error = match object.get("expected_error") {
            Some(Value::Null) => None,
            Some(value) => value.as_str().map(ToOwned::to_owned),
            None => {
                summary.fail(
                    "TIMELINE_EXPECTATION_MISSING",
                    &path,
                    "Expected error field is required.",
                );
                continue;
            }
        };

        let actual = stable_recovery_tick(&events, initial_service, stability_window, horizon);
        let matches = match (&actual, expected_error.as_deref()) {
            (Ok(actual_tick), None) => *actual_tick == expected_tick,
            (Err(error), Some(expected_code)) => {
                expected_tick.is_none() && oracle_error_code(error) == expected_code
            }
            _ => false,
        };
        if matches {
            summary.cases_passed += 1;
        } else {
            summary.fail(
                "TIMELINE_EXPECTATION_MISMATCH",
                &path,
                format!(
                    "Expected tick {expected_tick:?}, error {expected_error:?}; got {actual:?}."
                ),
            );
        }
    }

    summary.failures.sort_by(|left, right| {
        (&left.path, &left.code, &left.detail).cmp(&(&right.path, &right.code, &right.detail))
    });
    summary.passed = summary.failures.is_empty() && summary.cases_passed == summary.cases_seen;
    summary
}

#[cfg(test)]
mod timeline_fixture_tests {
    use super::*;

    fn has_code(summary: &TimelineFixtureSummary, code: &str) -> bool {
        summary.failures.iter().any(|failure| failure.code == code)
    }

    #[test]
    fn embedded_timeline_corpus_matches_independent_oracle() {
        let summary = validate_timeline_fixtures_bytes(
            EMBEDDED_ORACLE_CORPUS.as_bytes(),
            EMBEDDED_ORACLE_SCHEMA.as_bytes(),
        );
        assert!(
            summary.passed,
            "unexpected failures: {:?}",
            summary.failures
        );
        assert_eq!(summary.cases_seen, 8);
        assert_eq!(summary.cases_passed, 8);
        assert_eq!(summary.corpus_sha256.len(), 64);
        assert_eq!(summary.schema_sha256.len(), 64);
    }

    #[test]
    fn duplicate_timeline_case_id_is_rejected() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_ORACLE_CORPUS).expect("valid timeline corpus");
        let first_id = corpus["cases"][0]["id"]
            .as_str()
            .expect("case ID")
            .to_owned();
        corpus["cases"][1]["id"] = Value::String(first_id);
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary = validate_timeline_fixtures_bytes(&bytes, EMBEDDED_ORACLE_SCHEMA.as_bytes());
        assert!(has_code(&summary, "TIMELINE_CASE_ID_DUPLICATE"));
    }

    #[test]
    fn mutated_expected_recovery_tick_is_rejected() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_ORACLE_CORPUS).expect("valid timeline corpus");
        corpus["cases"][0]["expected_stable_recovery_tick"] = Value::from(7);
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary = validate_timeline_fixtures_bytes(&bytes, EMBEDDED_ORACLE_SCHEMA.as_bytes());
        assert!(has_code(&summary, "TIMELINE_EXPECTATION_MISMATCH"));
    }

    #[test]
    fn timeline_corpus_schema_mismatch_is_rejected_before_execution() {
        let malformed_schema = br#"{"type":"object","required":["absent"]}"#;
        let summary =
            validate_timeline_fixtures_bytes(EMBEDDED_ORACLE_CORPUS.as_bytes(), malformed_schema);
        assert!(has_code(&summary, "TIMELINE_SCHEMA_VALIDATION_FAILED"));
    }
}

pub mod outcomes;
pub use outcomes::{
    CandidateDisposition, ComparisonResult, EMBEDDED_OUTCOME_CORPUS, EMBEDDED_OUTCOME_SCHEMA,
    OutcomeFailure, OutcomeSummary, validate_outcome_fixtures_bytes,
};

pub mod decision_impact;
pub use decision_impact::{
    ActionImpact, DecisionCaseResult, DecisionImpactFailure, DecisionImpactSummary, DecisionSet,
    EMBEDDED_DECISION_IMPACT_CORPUS, EMBEDDED_DECISION_IMPACT_SCHEMA, OutcomeImpact,
    validate_decision_impact_bytes,
};


pub mod adaptive_policy;
pub use adaptive_policy::{
    validate_adaptive_policy_bytes, AdaptivePolicyFailure, AdaptivePolicySummary,
    EMBEDDED_ADAPTIVE_POLICY_CORPUS, EMBEDDED_ADAPTIVE_POLICY_SCHEMA,
};
