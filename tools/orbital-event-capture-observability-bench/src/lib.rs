use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const CORPUS_SCHEMA: &str = "luminous.orbital-event-capture.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-capture-reconciliation-only";
const MAX_EVENTS: usize = 100_000;
const MAX_RATE: f64 = 1.0e9;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

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
pub struct DeclaredEvidenceBindings {
    pub qualified_scope: Scope,
    pub test_scope: Scope,
    pub test_window_seconds: f64,
    pub minimum_event_rate_per_second: f64,
    pub maximum_event_rate_per_second: f64,
    pub reference_event_count: usize,
    pub derived_reference_event_rate_per_second: Option<f64>,
    pub reference: ReferenceEvidence,
    pub counters: CaptureCounters,
    pub calibration: CalibrationEvidence,
    pub clock: ClockEvidence,
}

fn declared_evidence_bindings(case: &CaptureCase) -> DeclaredEvidenceBindings {
    let derived_reference_event_rate_per_second = if case.test_window_seconds.is_finite()
        && case.test_window_seconds > 0.0
    {
        Some(case.reference_events.len() as f64 / case.test_window_seconds)
    } else {
        None
    };
    DeclaredEvidenceBindings {
        qualified_scope: case.qualified_scope.clone(),
        test_scope: case.test_scope.clone(),
        test_window_seconds: case.test_window_seconds,
        minimum_event_rate_per_second: case.minimum_event_rate_per_second,
        maximum_event_rate_per_second: case.maximum_event_rate_per_second,
        reference_event_count: case.reference_events.len(),
        derived_reference_event_rate_per_second,
        reference: case.reference.clone(),
        counters: case.counters.clone(),
        calibration: case.calibration.clone(),
        clock: case.clock.clone(),
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub declared_evidence_bindings: DeclaredEvidenceBindings,
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
fn log_add_exp(left: f64, right: f64) -> f64 {
    if left == f64::NEG_INFINITY {
        return right;
    }
    if right == f64::NEG_INFINITY {
        return left;
    }
    let maximum = left.max(right);
    maximum + ((left - maximum).exp() + (right - maximum).exp()).ln()
}

fn log_binomial_cdf_at_most(misses: usize, trials: usize, probability: f64) -> f64 {
    if probability <= 0.0 {
        return 0.0;
    }
    if probability >= 1.0 {
        return if misses == trials {
            0.0
        } else {
            f64::NEG_INFINITY
        };
    }

    let log_q = probability.ln();
    let log_not_q = (-probability).ln_1p();
    let mut log_mass = trials as f64 * log_not_q;
    let mut log_sum = log_mass;
    for j in 1..=misses {
        log_mass += ((trials - j + 1) as f64).ln() - (j as f64).ln() + log_q - log_not_q;
        log_sum = log_add_exp(log_sum, log_mass);
    }
    log_sum.min(0.0)
}

/// One-sided exact 95% Clopper-Pearson upper confidence limit for the
/// probability of missing an independently referenced event.
pub fn miss_probability_upper_bound_95(misses: usize, trials: usize) -> Result<f64, String> {
    if trials == 0 || trials > MAX_EVENTS || misses > trials {
        return Err("misses/trials must satisfy 0 <= misses <= trials <= v0.1 maximum".to_owned());
    }
    if misses == trials {
        return Ok(1.0);
    }

    let log_alpha = 0.05_f64.ln();
    let mut lower = 0.0;
    let mut upper = 1.0;
    for _ in 0..120 {
        let midpoint = lower + (upper - lower) / 2.0;
        if log_binomial_cdf_at_most(misses, trials, midpoint) > log_alpha {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
    }
    Ok(upper)
}

/// Zero-miss closed form retained as an explicit convenience function.
pub fn zero_miss_upper_bound_95(trials: usize) -> Result<f64, String> {
    if trials == 0 || trials > MAX_EVENTS {
        return Err("trial count must be between 1 and the v0.1 maximum".to_owned());
    }
    // Numerically stable form of 1 - 0.05^(1/N).
    Ok(-((0.05_f64.ln() / trials as f64).exp_m1()))
}

fn validate_reference_events(events: &[ReferenceEvent]) -> Result<(), String> {
    if events.is_empty() || events.len() > MAX_EVENTS {
        return Err("reference event count is empty or exceeds the v0.1 limit".to_owned());
    }
    let mut ids = HashSet::with_capacity(events.len());
    let mut previous_sequence: Option<u64> = None;
    let mut previous_time: Option<u64> = None;
    for event in events {
        if event.event_id.trim().is_empty()
            || event.sequence > MAX_SAFE_INTEGER
            || event.time_ticks > MAX_SAFE_INTEGER
            || !valid_hex(&event.payload_sha256, 64)
            || !ids.insert(event.event_id.as_str())
        {
            return Err("reference event identity or payload digest is invalid".to_owned());
        }
        if let Some(sequence) = previous_sequence
            && sequence.checked_add(1) != Some(event.sequence)
        {
            return Err("independent reference sequence is not contiguous".to_owned());
        }
        if let Some(time) = previous_time
            && event.time_ticks <= time
        {
            return Err("independent reference times are not strictly increasing".to_owned());
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
        return blocked(
            Disposition::Rejected,
            "malformed_scope_rate_or_evidence_digest",
        );
    }

    let reference_event_rate = case.reference_events.len() as f64 / case.test_window_seconds;
    if !reference_event_rate.is_finite()
        || reference_event_rate > MAX_RATE
        || case.test_scope != case.qualified_scope
        || reference_event_rate < case.minimum_event_rate_per_second
        || reference_event_rate > case.maximum_event_rate_per_second
    {
        return blocked(
            Disposition::OutOfEnvelope,
            "test_configuration_or_event_rate_outside_declared_envelope",
        );
    }

    if !case.reference.available
        || case.reference.system_id.trim().is_empty()
        || case.reference.failure_domains.is_empty()
        || case.reference.acquisition_failure_domains.is_empty()
        || case
            .reference
            .failure_domains
            .iter()
            .any(|domain| domain.trim().is_empty())
        || case
            .reference
            .acquisition_failure_domains
            .iter()
            .any(|domain| domain.trim().is_empty())
        || shared_failure_domain(
            &case.reference.failure_domains,
            &case.reference.acquisition_failure_domains,
        )
    {
        return blocked(
            Disposition::Indeterminate,
            "independent_reference_missing_or_common_cause_unresolved",
        );
    }

    if !case.calibration.valid_at_test_time || !case.calibration.scope_matches {
        return blocked(
            Disposition::Indeterminate,
            "calibration_missing_invalid_or_out_of_scope",
        );
    }

    if !case.clock.synchronized
        || !case.clock.monotonic
        || case.clock.allowed_max_error_ticks > MAX_SAFE_INTEGER
        || case.clock.measured_max_error_ticks.is_none_or(|error| {
            error > case.clock.allowed_max_error_ticks || error > MAX_SAFE_INTEGER
        })
    {
        return blocked(
            Disposition::Indeterminate,
            "clock_integrity_or_error_bound_not_established",
        );
    }
    let measured_clock_error = case.clock.measured_max_error_ticks.unwrap_or(0);

    let counter_values = [
        case.counters.overflow_count,
        case.counters.dropped_event_count,
        case.counters.dropped_packet_count,
        case.counters.reset_count,
        case.counters.sequence_gap_count,
    ];
    if counter_values
        .iter()
        .any(|value| value.is_none_or(|count| count > MAX_SAFE_INTEGER))
    {
        return blocked(
            Disposition::Indeterminate,
            "required_capture_loss_counter_missing",
        );
    }
    let loss_counters_nonzero = counter_values.iter().any(|value| value.unwrap_or(0) > 0);

    if validate_reference_events(&case.reference_events).is_err() {
        return blocked(
            Disposition::Rejected,
            "independent_reference_event_stream_invalid",
        );
    }
    if case.observed_events.len() > MAX_EVENTS {
        return blocked(
            Disposition::Rejected,
            "observed_event_count_exceeds_v0_1_limit",
        );
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
            || observed.time_ticks > MAX_SAFE_INTEGER
            || !valid_hex(&observed.payload_sha256, 64)
            || !record_ids.insert(observed.record_id.as_str())
        {
            let decision = blocked(
                Disposition::Disputed,
                "duplicate_record_identity_or_malformed_observation",
            );
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
        if captured_by_reference
            .get(reference.event_id.as_str())
            .copied()
            .unwrap_or(0)
            == 0
        {
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

    let upper_miss_probability_95 = if !loss_counters_nonzero
        && duplicate_count == 0
        && unexpected_count == 0
        && !disputed_payload_or_time
    {
        miss_probability_upper_bound_95(missed_count, case.reference_events.len()).ok()
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
            declared_evidence_bindings: declared_evidence_bindings(case),
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
        assert!(
            (miss_probability_upper_bound_95(1, 5).unwrap() - 0.6574083180011387).abs() < 1.0e-12
        );
        assert!(
            (miss_probability_upper_bound_95(2, 5).unwrap() - 0.8107446225622293).abs() < 1.0e-12
        );
        assert!(
            (miss_probability_upper_bound_95(1, 10).unwrap() - 0.3941633024365048).abs() < 1.0e-12
        );
        assert_eq!(miss_probability_upper_bound_95(5, 5).unwrap(), 1.0);
        assert!(miss_probability_upper_bound_95(6, 5).is_err());
    }

    #[test]
    fn clopper_pearson_upper_bound_inverts_the_exact_binomial_tail() {
        let log_alpha = 0.05_f64.ln();
        for trials in 1..=40 {
            for misses in 0..trials {
                let upper = miss_probability_upper_bound_95(misses, trials)
                    .expect("valid miss/trial counts");
                let log_tail = log_binomial_cdf_at_most(misses, trials, upper);
                assert!(
                    (log_tail - log_alpha).abs() < 1.0e-10,
                    "tail inversion failed for {misses}/{trials}: {log_tail}"
                );
                assert!((0.0..=1.0).contains(&upper));
            }
            assert_eq!(
                miss_probability_upper_bound_95(trials, trials).unwrap(),
                1.0
            );
        }
    }

    #[test]
    fn miss_bound_is_monotone_in_misses_and_sample_size() {
        let zero_of_five = miss_probability_upper_bound_95(0, 5).unwrap();
        let one_of_five = miss_probability_upper_bound_95(1, 5).unwrap();
        let two_of_five = miss_probability_upper_bound_95(2, 5).unwrap();
        let zero_of_ten = miss_probability_upper_bound_95(0, 10).unwrap();
        assert!(zero_of_five < one_of_five);
        assert!(one_of_five < two_of_five);
        assert!(zero_of_ten < zero_of_five);
        assert!((zero_of_five - zero_miss_upper_bound_95(5).unwrap()).abs() < 1.0e-12);
    }

    #[test]
    fn clopper_pearson_upper_bound_has_at_least_nominal_grid_coverage() {
        let probabilities = [0.01_f64, 0.05, 0.2, 0.5, 0.9];
        for trials in 1..=20 {
            for true_miss_probability in probabilities {
                let mut mass = (1.0 - true_miss_probability).powi(trials as i32);
                let mut coverage = 0.0;
                for misses in 0..=trials {
                    let upper = miss_probability_upper_bound_95(misses, trials)
                        .expect("valid miss/trial counts");
                    if true_miss_probability <= upper + 1.0e-12 {
                        coverage += mass;
                    }
                    if misses < trials {
                        mass *= (trials - misses) as f64 / (misses + 1) as f64
                            * true_miss_probability
                            / (1.0 - true_miss_probability);
                    }
                }
                assert!(
                    coverage >= 0.95 - 1.0e-10,
                    "undercoverage at n={trials}, q={true_miss_probability}: {coverage}"
                );
            }
        }
    }

    #[test]
    fn receipt_preserves_declared_observation_and_metrology_bindings() {
        let corpus: Corpus = serde_json::from_str(include_str!(
            "../fixtures/v0.1/capture-cases.json"
        ))
        .expect("frozen capture corpus parses");
        let report = evaluate(&corpus).expect("frozen capture corpus evaluates");
        let first = report.cases.first().expect("first case exists");
        let bindings = &first.declared_evidence_bindings;

        assert_eq!(
            bindings.qualified_scope.sensor_id,
            "sensor-qualification-coupon-01"
        );
        assert_eq!(bindings.test_scope, bindings.qualified_scope);
        assert_eq!(bindings.reference_event_count, 5);
        assert_eq!(
            bindings.derived_reference_event_rate_per_second,
            Some(1.0)
        );
        assert_eq!(
            bindings.reference.system_id,
            "independent-pulse-reference-01"
        );
        assert_eq!(bindings.counters.overflow_count, Some(0));
        assert!(bindings.calibration.valid_at_test_time);
        assert!(bindings.calibration.scope_matches);
        assert!(bindings.clock.synchronized);
        assert!(bindings.clock.monotonic);
        assert_eq!(bindings.clock.measured_max_error_ticks, Some(1));
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
