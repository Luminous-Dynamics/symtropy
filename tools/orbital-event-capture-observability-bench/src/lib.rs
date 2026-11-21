use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const CORPUS_SCHEMA: &str = "luminous.orbital-event-capture.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-capture-reconciliation-only";
const MAX_EVENTS: usize = 100_000;
const MAX_RATE: f64 = 1.0e9;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    ReconciledFixtureOnly,
    Incomplete,
    Indeterminate,
    OutOfEnvelope,
    Rejected,
    Disputed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    pub sensor_id: String,
    pub firmware_sha256: String,
    pub acquisition_config_sha256: String,
    pub event_class: String,
    pub operating_mode_id: String,
    pub environment_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceEvidence {
    pub available: bool,
    pub system_id: String,
    pub reference_artifact_sha256: String,
    pub failure_domains: Vec<String>,
    pub acquisition_failure_domains: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceEvent {
    pub event_id: String,
    pub sequence: u64,
    pub time_ticks: u64,
    pub payload_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservedEvent {
    pub record_id: String,
    pub reference_event_id: String,
    pub time_ticks: u64,
    pub payload_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CaptureCounters {
    pub overflow_count: Option<u64>,
    pub dropped_event_count: Option<u64>,
    pub dropped_packet_count: Option<u64>,
    pub reset_count: Option<u64>,
    pub sequence_gap_count: Option<u64>,
    pub counter_artifact_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CalibrationEvidence {
    pub valid_at_test_time: bool,
    pub scope_matches: bool,
    pub certificate_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClockEvidence {
    pub synchronized: bool,
    pub monotonic: bool,
    pub measured_max_error_ticks: Option<u64>,
    pub allowed_max_error_ticks: u64,
    pub synchronization_artifact_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: Disposition,
    pub injected_count: usize,
    pub captured_count: usize,
    pub missed_count: usize,
    pub duplicate_count: usize,
    pub unexpected_count: usize,
    pub upper_miss_probability_95: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptureCase {
    pub case_id: String,
    pub qualified_scope: Scope,
    pub test_scope: Scope,
    pub minimum_event_rate_per_second: f64,
    pub maximum_event_rate_per_second: f64,
    pub test_window_seconds: f64,
    pub reference: ReferenceEvidence,
    pub reference_events: Vec<ReferenceEvent>,
    pub observed_events: Vec<ObservedEvent>,
    pub counters: CaptureCounters,
    pub calibration: CalibrationEvidence,
    pub clock: ClockEvidence,
    pub expected: Expected,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<CaptureCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub injected_count: usize,
    pub captured_count: usize,
    pub missed_count: usize,
    pub duplicate_count: usize,
    pub unexpected_count: usize,
    pub upper_miss_probability_95: Option<f64>,
    pub loss_counters_nonzero: bool,
    pub fit_acceptance_granted: bool,
    pub reliability_claim_granted: bool,
    pub physical_completeness_claim_granted: bool,
    pub authority_granted: bool,
    pub expected_result_matched: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub reconciled_fixture_only: usize,
    pub incomplete: usize,
    pub indeterminate: usize,
    pub out_of_envelope: usize,
    pub rejected: usize,
    pub disputed: usize,
    pub expected_result_mismatches: usize,
    pub missing_events: usize,
    pub duplicate_events: usize,
    pub unexpected_events: usize,
    pub loss_counter_cases: usize,
    pub fit_acceptances: usize,
    pub reliability_claims: usize,
    pub physical_completeness_claims: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub fit_acceptance_granted: bool,
    pub reliability_claim_granted: bool,
    pub physical_completeness_claim_granted: bool,
    pub authority_granted: bool,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
}

#[derive(Debug, Clone)]
struct Decision {
    disposition: Disposition,
    injected_count: usize,
    captured_count: usize,
    missed_count: usize,
    duplicate_count: usize,
    unexpected_count: usize,
    upper_miss_probability_95: Option<f64>,
    loss_counters_nonzero: bool,
    blockers: Vec<String>,
}

fn blocked(disposition: Disposition, code: &str) -> Decision {
    Decision {
        disposition,
        injected_count: 0,
        captured_count: 0,
        missed_count: 0,
        duplicate_count: 0,
        unexpected_count: 0,
        upper_miss_probability_95: None,
        loss_counters_nonzero: false,
        blockers: vec![code.to_owned()],
    }
}

fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_scope(scope: &Scope) -> bool {
    !scope.sensor_id.trim().is_empty()
        && valid_hex(&scope.firmware_sha256, 64)
        && valid_hex(&scope.acquisition_config_sha256, 64)
        && !scope.event_class.trim().is_empty()
        && !scope.operating_mode_id.trim().is_empty()
        && !scope.environment_id.trim().is_empty()
}

fn shared_failure_domain(reference: &[String], acquisition: &[String]) -> bool {
    let reference_set: HashSet<&str> = reference.iter().map(String::as_str).collect();
    acquisition
        .iter()
        .any(|domain| reference_set.contains(domain.as_str()))
}

/// One-sided exact 95% upper bound for a binomial miss probability when zero
/// misses are observed in N independent, representative known-event trials.
/// This is a conditional sampling statement, not proof of representativeness.
pub fn zero_miss_upper_bound_95(trials: usize) -> Result<f64, String> {
    if trials == 0 || trials > MAX_EVENTS {
        return Err("trial count must be between 1 and the v0.1 maximum".to_owned());
    }
    // Numerically stable form of 1 - 0.05^(1/N).
    Ok(-((-0.05_f64.ln() / trials as f64).exp_m1()))
}

fn validate_reference_events(events: &[ReferenceEvent]) -> Result<(), String> {
    if events.is_empty() || events.len() > MAX_EVENTS {
        return Err("reference event count is empty or exceeds the v0.1 limit".to_owned());
    }
    let mut ids = HashSet::with_capacity(events.len());
    let mut previous_sequence = None;
    let mut previous_time = None;
    for event in events {
        if event.event_id.trim().is_empty()
            || !valid_hex(&event.payload_sha256, 64)
            || !ids.insert(event.event_id.as_str())
        {
            return Err("reference event identity or payload digest is invalid".to_owned());
        }
        if let Some(sequence) = previous_sequence {
            if sequence.checked_add(1) != Some(event.sequence) {
                return Err("independent reference sequence is not contiguous".to_owned());
            }
        }
        if let Some(time) = previous_time {
            if event.time_ticks <= time {
                return Err("independent reference times are not strictly increasing".to_owned());
            }
        }
        previous_sequence = Some(event.sequence);
        previous_time = Some(event.time_ticks);
    }
    Ok(())
}

fn evaluate_case(case: &CaptureCase) -> Decision {
    if case.case_id.trim().is_empty()
        || !valid_scope(&case.qualified_scope)
        || !valid_scope(&case.test_scope)
        || !valid_hex(&case.reference.reference_artifact_sha256, 64)
        || !valid_hex(&case.counters.counter_artifact_sha256, 64)
        || !valid_hex(&case.calibration.certificate_sha256, 64)
        || !valid_hex(&case.clock.synchronization_artifact_sha256, 64)
        || !case.minimum_event_rate_per_second.is_finite()
        || !case.maximum_event_rate_per_second.is_finite()
        || !case.test_window_seconds.is_finite()
        || case.test_window_seconds <= 0.0
        || case.minimum_event_rate_per_second < 0.0
        || case.maximum_event_rate_per_second < case.minimum_event_rate_per_second
        || case.maximum_event_rate_per_second > MAX_RATE
    {
        return blocked(Disposition::Rejected, "malformed_scope_rate_or_evidence_digest");
    }

    let reference_event_rate = case.reference_events.len() as f64 / case.test_window_seconds;
    if !reference_event_rate.is_finite()
        || reference_event_rate > MAX_RATE
        || case.test_scope != case.qualified_scope
        || reference_event_rate < case.minimum_event_rate_per_second
        || reference_event_rate > case.maximum_event_rate_per_second
    {
        return blocked(Disposition::OutOfEnvelope, "test_configuration_or_event_rate_outside_declared_envelope");
    }

    if !case.reference.available
        || case.reference.system_id.trim().is_empty()
        || case.reference.failure_domains.is_empty()
        || case.reference.acquisition_failure_domains.is_empty()
        || shared_failure_domain(
            &case.reference.failure_domains,
            &case.reference.acquisition_failure_domains,
        )
    {
        return blocked(Disposition::Indeterminate, "independent_reference_missing_or_common_cause_unresolved");
    }

    if !case.calibration.valid_at_test_time || !case.calibration.scope_matches {
        return blocked(Disposition::Indeterminate, "calibration_missing_invalid_or_out_of_scope");
    }

    if !case.clock.synchronized
        || !case.clock.monotonic
        || case
            .clock
            .measured_max_error_ticks
            .is_none_or(|error| error > case.clock.allowed_max_error_ticks)
    {
        return blocked(Disposition::Indeterminate, "clock_integrity_or_error_bound_not_established");
    }
    let measured_clock_error = case.clock.measured_max_error_ticks.unwrap_or(0);

    let counter_values = [
        case.counters.overflow_count,
        case.counters.dropped_event_count,
        case.counters.dropped_packet_count,
        case.counters.reset_count,
        case.counters.sequence_gap_count,
    ];
    if counter_values.iter().any(Option::is_none) {
        return blocked(Disposition::Indeterminate, "required_capture_loss_counter_missing");
    }
    let loss_counters_nonzero = counter_values.iter().any(|value| value.unwrap_or(0) > 0);

    if validate_reference_events(&case.reference_events).is_err() {
        return blocked(Disposition::Rejected, "independent_reference_event_stream_invalid");
    }
    if case.observed_events.len() > MAX_EVENTS {
        return blocked(Disposition::Rejected, "observed_event_count_exceeds_v0_1_limit");
    }

    let reference_by_id: HashMap<&str, &ReferenceEvent> = case
        .reference_events
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect();
    let mut record_ids = HashSet::with_capacity(case.observed_events.len());
    let mut captured_by_reference: HashMap<&str, usize> = HashMap::new();
    let mut captured_count = 0usize;
    let mut duplicate_count = 0usize;
    let mut unexpected_count = 0usize;
    let mut disputed_payload_or_time = false;

    for observed in &case.observed_events {
        if observed.record_id.trim().is_empty()
            || observed.reference_event_id.trim().is_empty()
            || !valid_hex(&observed.payload_sha256, 64)
            || !record_ids.insert(observed.record_id.as_str())
        {
            let mut decision = blocked(Disposition::Disputed, "duplicate_record_identity_or_malformed_observation");
            return decision;
        }
        let Some(reference) = reference_by_id.get(observed.reference_event_id.as_str()) else {
            unexpected_count += 1;
            continue;
        };
        let count = captured_by_reference
            .entry(observed.reference_event_id.as_str())
            .or_insert(0);
        *count += 1;
        if *count > 1 {
            duplicate_count += 1;
        }
        if observed.payload_sha256 != reference.payload_sha256
            || observed.time_ticks.abs_diff(reference.time_ticks) > measured_clock_error
        {
            disputed_payload_or_time = true;
        }
    }

    let mut missed_count = 0usize;
    for reference in &case.reference_events {
        if captured_by_reference.get(reference.event_id.as_str()).copied().unwrap_or(0) == 0 {
            missed_count += 1;
        } else {
            captured_count += 1;
        }
    }

    let disposition = if disputed_payload_or_time || duplicate_count > 0 || unexpected_count > 0 {
        Disposition::Disputed
    } else if missed_count > 0 || loss_counters_nonzero {
        Disposition::Incomplete
    } else {
        Disposition::ReconciledFixtureOnly
    };

    let upper_miss_probability_95 = if missed_count == 0 {
        zero_miss_upper_bound_95(case.reference_events.len()).ok()
    } else {
        None
    };

    let mut blockers = Vec::new();
    if missed_count > 0 {
        blockers.push("independently_injected_events_not_captured".to_owned());
    }
    if duplicate_count > 0 {
        blockers.push("reference_event_captured_more_than_once".to_owned());
    }
    if unexpected_count > 0 {
        blockers.push("observation_has_no_matching_independent_reference".to_owned());
    }
    if disputed_payload_or_time {
        blockers.push("payload_or_timestamp_disagrees_with_independent_reference".to_owned());
    }
    if loss_counters_nonzero {
        blockers.push("capture_loss_counter_nonzero".to_owned());
    }

    Decision {
        disposition,
        injected_count: case.reference_events.len(),
        captured_count,
        missed_count,
        duplicate_count,
        unexpected_count,
        upper_miss_probability_95,
        loss_counters_nonzero,
        blockers,
    }
}

pub fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
    {
        return Err("unsupported, empty or incorrectly classified capture corpus".to_owned());
    }

    let mut case_ids = HashSet::with_capacity(corpus.cases.len());
    let mut summary = Summary {
        case_count: corpus.cases.len(),
        reconciled_fixture_only: 0,
        incomplete: 0,
        indeterminate: 0,
        out_of_envelope: 0,
        rejected: 0,
        disputed: 0,
        expected_result_mismatches: 0,
        missing_events: 0,
        duplicate_events: 0,
        unexpected_events: 0,
        loss_counter_cases: 0,
        fit_acceptances: 0,
        reliability_claims: 0,
        physical_completeness_claims: 0,
        authority_grants: 0,
    };
    let mut results = Vec::with_capacity(corpus.cases.len());

    for case in &corpus.cases {
        if case.case_id.trim().is_empty() || !case_ids.insert(case.case_id.as_str()) {
            return Err("capture case IDs must be unique and non-empty".to_owned());
        }
        let decision = evaluate_case(case);
        match decision.disposition {
            Disposition::ReconciledFixtureOnly => summary.reconciled_fixture_only += 1,
            Disposition::Incomplete => summary.incomplete += 1,
            Disposition::Indeterminate => summary.indeterminate += 1,
            Disposition::OutOfEnvelope => summary.out_of_envelope += 1,
            Disposition::Rejected => summary.rejected += 1,
            Disposition::Disputed => summary.disputed += 1,
        }
        summary.missing_events += decision.missed_count;
        summary.duplicate_events += decision.duplicate_count;
        summary.unexpected_events += decision.unexpected_count;
        summary.loss_counter_cases += usize::from(decision.loss_counters_nonzero);

        let expected = &case.expected;
        let close = |left: Option<f64>, right: Option<f64>| match (left, right) {
            (Some(a), Some(b)) => (a - b).abs() <= 1.0e-12_f64.max(b.abs() * 1.0e-10),
            (None, None) => true,
            _ => false,
        };
        let expected_result_matched = decision.disposition == expected.disposition
            && decision.injected_count == expected.injected_count
            && decision.captured_count == expected.captured_count
            && decision.missed_count == expected.missed_count
            && decision.duplicate_count == expected.duplicate_count
            && decision.unexpected_count == expected.unexpected_count
            && close(
                decision.upper_miss_probability_95,
                expected.upper_miss_probability_95,
            );
        summary.expected_result_mismatches += usize::from(!expected_result_matched);

        results.push(CaseResult {
            case_id: case.case_id.clone(),
            disposition: decision.disposition,
            injected_count: decision.injected_count,
            captured_count: decision.captured_count,
            missed_count: decision.missed_count,
            duplicate_count: decision.duplicate_count,
            unexpected_count: decision.unexpected_count,
            upper_miss_probability_95: decision.upper_miss_probability_95,
            loss_counters_nonzero: decision.loss_counters_nonzero,
            fit_acceptance_granted: false,
            reliability_claim_granted: false,
            physical_completeness_claim_granted: false,
            authority_granted: false,
            expected_result_matched,
            blockers: decision.blockers,
        });
    }

    let result = if summary.expected_result_mismatches == 0
        && summary.fit_acceptances == 0
        && summary.reliability_claims == 0
        && summary.physical_completeness_claims == 0
        && summary.authority_grants == 0
    {
        "pass"
    } else {
        "fail"
    };

    Ok(Report {
        result: result.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        fit_acceptance_granted: false,
        reliability_claim_granted: false,
        physical_completeness_claim_granted: false,
        authority_granted: false,
        summary,
        cases: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_miss_bound_has_expected_exact_values() {
        assert!((zero_miss_upper_bound_95(1).unwrap() - 0.95).abs() < 1.0e-12);
        assert!(zero_miss_upper_bound_95(2995).unwrap() < 0.001);
        assert!(zero_miss_upper_bound_95(2994).unwrap() > 0.001);
        assert!(zero_miss_upper_bound_95(29956).unwrap() < 0.0001);
        assert!(zero_miss_upper_bound_95(0).is_err());
    }

    #[test]
    fn shared_failure_domain_is_not_independent_reference() {
        assert!(shared_failure_domain(
            &["power".to_owned(), "reference_clock".to_owned()],
            &["reference_clock".to_owned(), "daq".to_owned()]
        ));
        assert!(!shared_failure_domain(
            &["independent_pulse_generator".to_owned()],
            &["daq".to_owned(), "storage".to_owned()]
        ));
    }

    #[test]
    fn scope_digest_rejects_uppercase_or_wrong_length() {
        let valid = Scope {
            sensor_id: "sensor-a".to_owned(),
            firmware_sha256: "a".repeat(64),
            acquisition_config_sha256: "b".repeat(64),
            event_class: "impact".to_owned(),
            operating_mode_id: "nominal".to_owned(),
            environment_id: "ambient".to_owned(),
        };
        assert!(valid_scope(&valid));
        let mut invalid = valid;
        invalid.firmware_sha256 = "A".repeat(64);
        assert!(!valid_scope(&invalid));
    }

    #[test]
    fn reference_stream_rejects_gaps_and_duplicate_ids() {
        let event = |id: &str, sequence: u64, time: u64| ReferenceEvent {
            event_id: id.to_owned(),
            sequence,
            time_ticks: time,
            payload_sha256: "c".repeat(64),
        };
        assert!(validate_reference_events(&[event("e1", 1, 10), event("e2", 2, 20)]).is_ok());
        assert!(validate_reference_events(&[event("e1", 1, 10), event("e2", 3, 20)]).is_err());
        assert!(validate_reference_events(&[event("e1", 1, 10), event("e1", 2, 20)]).is_err());
    }
}
