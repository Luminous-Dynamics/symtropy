use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

const CORPUS_SCHEMA: &str = "luminous.orbital-interface-reliability.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-independent-mission-binomial-semantics-only";
const MAX_TRIALS: usize = 100_000;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrialOutcome {
    Success,
    Failure,
    Restricted,
    Aborted,
    Withdrawn,
    Invalid,
    RightCensored,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    ReliabilityDemonstrated,
    ReliabilityNotDemonstrated,
    Indeterminate,
    ModelNotSupported,
    Rejected,
    Disputed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Trial {
    pub trial_id: String,
    pub article_serial: String,
    pub sampling_cluster_id: String,
    pub population_id: String,
    pub lot_id: String,
    pub configuration_id: String,
    pub mission_profile_id: String,
    pub failure_definition_id: String,
    pub outcome: Option<TrialOutcome>,
    pub raw_artifact_sha256: String,
    pub evidence_complete: bool,
    pub mission_profile_verified: bool,
    pub intervention_during_mission: bool,
    pub exposure_cycles: u64,
    pub daq_sample_count: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: Disposition,
    pub unique_trials: usize,
    pub successes: usize,
    pub failures: usize,
    pub restricted_outcomes: usize,
    pub invalid_trials: usize,
    pub aborted_trials: usize,
    pub withdrawn_trials: usize,
    pub censored_trials: usize,
    pub duplicate_trials: usize,
    pub lower_confidence_bound: Option<f64>,
    pub meets_reliability_target: Option<bool>,
    pub naive_success_fraction: Option<f64>,
    pub naive_baseline_false_assurance: bool,
    pub minimum_zero_failure_trials: Option<usize>,
    pub naive_cycle_lower_bound: Option<f64>,
    pub naive_cycle_false_assurance: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReliabilityCase {
    pub case_id: String,
    pub preregistration_ref: Option<String>,
    pub population_id: String,
    pub lot_id: String,
    pub configuration_id: String,
    pub mission_profile_id: String,
    pub failure_definition_id: String,
    pub sampling_design: String,
    pub sampling_frame_ref: Option<String>,
    pub dependence_assessment_ref: Option<String>,
    pub reliability_target: f64,
    pub confidence: f64,
    pub reported_cycle_successes: usize,
    pub reported_cycle_failures: usize,
    pub trials: Vec<Trial>,
    pub expected: Expected,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<ReliabilityCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: Disposition,
    pub unique_trials: usize,
    pub successes: usize,
    pub failures: usize,
    pub restricted_outcomes: usize,
    pub invalid_trials: usize,
    pub aborted_trials: usize,
    pub withdrawn_trials: usize,
    pub censored_trials: usize,
    pub duplicate_trials: usize,
    pub lower_confidence_bound: Option<f64>,
    pub meets_reliability_target: Option<bool>,
    pub naive_success_fraction: Option<f64>,
    pub naive_baseline_false_assurance: bool,
    pub minimum_zero_failure_trials: Option<usize>,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub reliability_demonstrated: usize,
    pub reliability_not_demonstrated: usize,
    pub indeterminate: usize,
    pub model_not_supported: usize,
    pub rejected: usize,
    pub disputed: usize,
    pub unique_trials: usize,
    pub successes: usize,
    pub failures: usize,
    pub restricted_outcomes: usize,
    pub invalid_trials: usize,
    pub aborted_trials: usize,
    pub withdrawn_trials: usize,
    pub censored_trials: usize,
    pub duplicate_trials: usize,
    pub naive_false_assurance_cases: usize,
    pub naive_cycle_false_assurance_cases: usize,
    pub expected_result_mismatches: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
}

#[derive(Debug, Clone)]
struct Decision {
    disposition: Disposition,
    unique_trials: usize,
    successes: usize,
    failures: usize,
    restricted_outcomes: usize,
    invalid_trials: usize,
    aborted_trials: usize,
    withdrawn_trials: usize,
    censored_trials: usize,
    duplicate_trials: usize,
    lower_confidence_bound: Option<f64>,
    meets_reliability_target: Option<bool>,
    naive_success_fraction: Option<f64>,
    naive_baseline_false_assurance: bool,
    minimum_zero_failure_trials: Option<usize>,
    naive_cycle_lower_bound: Option<f64>,
    naive_cycle_false_assurance: bool,
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

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

fn log_binomial_cdf_at_most(n: usize, failures: usize, failure_probability: f64) -> f64 {
    if failures >= n {
        return 0.0;
    }
    if failure_probability <= 0.0 {
        return 0.0;
    }
    if failure_probability >= 1.0 {
        return f64::NEG_INFINITY;
    }

    let log_q = failure_probability.ln();
    let log_one_minus_q = (-failure_probability).ln_1p();
    let mut log_mass = (n as f64) * log_one_minus_q;
    let mut log_sum = log_mass;

    for k in 1..=failures {
        log_mass += ((n - k + 1) as f64).ln()
            - (k as f64).ln()
            + log_q
            - log_one_minus_q;
        log_sum = log_add_exp(log_sum, log_mass);
    }
    log_sum.min(0.0)
}

/// Exact one-sided Clopper-Pearson lower confidence bound for mission success probability.
pub fn mission_reliability_lower_bound(
    trials: usize,
    failures: usize,
    confidence: f64,
) -> Result<f64, String> {
    if trials == 0 || trials > MAX_TRIALS {
        return Err("trial count must be between 1 and 100000".to_owned());
    }
    if failures > trials {
        return Err("failure count cannot exceed trial count".to_owned());
    }
    if !confidence.is_finite() || !(0.0 < confidence && confidence < 1.0) {
        return Err("confidence must be finite and strictly between zero and one".to_owned());
    }
    if failures == trials {
        return Ok(0.0);
    }

    let alpha = 1.0 - confidence;
    let log_alpha = alpha.ln();
    let mut lower_failure_probability = 0.0;
    let mut upper_failure_probability = 1.0;

    for _ in 0..100 {
        let midpoint = lower_failure_probability
            + (upper_failure_probability - lower_failure_probability) / 2.0;
        let log_cdf = log_binomial_cdf_at_most(trials, failures, midpoint);
        if log_cdf > log_alpha {
            lower_failure_probability = midpoint;
        } else {
            upper_failure_probability = midpoint;
        }
    }

    Ok((1.0 - upper_failure_probability).clamp(0.0, 1.0))
}

/// Exact minimum sample size for a zero-failure one-sided binomial demonstration.
pub fn minimum_zero_failure_trials(
    reliability_target: f64,
    confidence: f64,
) -> Result<usize, String> {
    if !reliability_target.is_finite()
        || !(0.0 < reliability_target && reliability_target < 1.0)
    {
        return Err("reliability target must be finite and strictly between zero and one".to_owned());
    }
    if !confidence.is_finite() || !(0.0 < confidence && confidence < 1.0) {
        return Err("confidence must be finite and strictly between zero and one".to_owned());
    }

    let alpha = 1.0 - confidence;
    let raw = (alpha.ln() / reliability_target.ln()).ceil();
    if !raw.is_finite() || raw < 1.0 || raw > MAX_TRIALS as f64 {
        return Err("required sample size is outside the v0.1 limit".to_owned());
    }
    let mut n = raw as usize;
    while mission_reliability_lower_bound(n, 0, confidence)? < reliability_target {
        n += 1;
    }
    while n > 1
        && mission_reliability_lower_bound(n - 1, 0, confidence)? >= reliability_target
    {
        n -= 1;
    }
    Ok(n)
}

fn empty_decision(disposition: Disposition, duplicate_trials: usize) -> Decision {
    Decision {
        disposition,
        unique_trials: 0,
        successes: 0,
        failures: 0,
        restricted_outcomes: 0,
        invalid_trials: 0,
        aborted_trials: 0,
        withdrawn_trials: 0,
        censored_trials: 0,
        duplicate_trials,
        lower_confidence_bound: None,
        meets_reliability_target: None,
        naive_success_fraction: None,
        naive_baseline_false_assurance: false,
        minimum_zero_failure_trials: None,
        naive_cycle_lower_bound: None,
        naive_cycle_false_assurance: false,
    }
}

fn evaluate_case(case: &ReliabilityCase) -> Result<Decision, String> {
    if case.case_id.is_empty() || case.trials.is_empty() {
        return Err("case ID and trial records must be non-empty".to_owned());
    }
    if !case.reliability_target.is_finite()
        || !(0.0 < case.reliability_target && case.reliability_target < 1.0)
        || !case.confidence.is_finite()
        || !(0.0 < case.confidence && case.confidence < 1.0)
    {
        return Ok(empty_decision(Disposition::Rejected, 0));
    }
    if case
        .preregistration_ref
        .as_deref()
        .is_none_or(|reference| reference.trim().is_empty())
        || case
            .sampling_frame_ref
            .as_deref()
            .is_none_or(|reference| reference.trim().is_empty())
        || case
            .dependence_assessment_ref
            .as_deref()
            .is_none_or(|reference| reference.trim().is_empty())
    {
        return Ok(empty_decision(Disposition::Indeterminate, 0));
    }
    if case.sampling_design != "simple_random_single_lot_independent_articles" {
        return Ok(empty_decision(Disposition::ModelNotSupported, 0));
    }
    if case.trials.len() > MAX_TRIALS {
        return Err("trial record count exceeds v0.1 limit".to_owned());
    }

    let mut by_trial_id: HashMap<&str, &Trial> = HashMap::new();
    let mut by_article: HashMap<&str, &Trial> = HashMap::new();
    let mut by_cluster: HashMap<&str, &Trial> = HashMap::new();
    let mut unique = Vec::with_capacity(case.trials.len());
    let mut duplicate_trials = 0usize;

    for trial in &case.trials {
        if trial.trial_id.is_empty()
            || trial.article_serial.is_empty()
            || trial.sampling_cluster_id.is_empty()
        {
            return Ok(empty_decision(Disposition::Indeterminate, duplicate_trials));
        }
        if let Some(previous) = by_trial_id.get(trial.trial_id.as_str()) {
            if *previous == trial {
                duplicate_trials += 1;
                continue;
            }
            return Ok(empty_decision(Disposition::Disputed, duplicate_trials));
        }
        if let Some(previous) = by_article.get(trial.article_serial.as_str()) {
            let disposition = if previous.outcome != trial.outcome
                || previous.raw_artifact_sha256 != trial.raw_artifact_sha256
            {
                Disposition::Disputed
            } else {
                Disposition::Indeterminate
            };
            return Ok(empty_decision(disposition, duplicate_trials));
        }
        if by_cluster.contains_key(trial.sampling_cluster_id.as_str()) {
            return Ok(empty_decision(Disposition::Indeterminate, duplicate_trials));
        }

        by_trial_id.insert(&trial.trial_id, trial);
        by_article.insert(&trial.article_serial, trial);
        by_cluster.insert(&trial.sampling_cluster_id, trial);
        unique.push(trial.clone());
    }

    for trial in &unique {
        if trial.population_id != case.population_id
            || trial.lot_id != case.lot_id
            || trial.configuration_id != case.configuration_id
            || trial.mission_profile_id != case.mission_profile_id
            || trial.failure_definition_id != case.failure_definition_id
        {
            return Ok(empty_decision(Disposition::Rejected, duplicate_trials));
        }
    }

    let mut successes = 0usize;
    let mut failures = 0usize;
    let mut restricted_outcomes = 0usize;
    let mut invalid_trials = 0usize;
    let mut aborted_trials = 0usize;
    let mut withdrawn_trials = 0usize;
    let mut censored_trials = 0usize;

    for trial in &unique {
        if !valid_digest(&trial.raw_artifact_sha256)
            || !trial.evidence_complete
            || !trial.mission_profile_verified
            || trial.intervention_during_mission
        {
            invalid_trials += 1;
            continue;
        }
        match trial.outcome {
            Some(TrialOutcome::Success) => successes += 1,
            Some(TrialOutcome::Failure) => failures += 1,
            Some(TrialOutcome::Restricted) => {
                restricted_outcomes += 1;
                failures += 1;
            }
            Some(TrialOutcome::Aborted) => aborted_trials += 1,
            Some(TrialOutcome::Withdrawn) => withdrawn_trials += 1,
            Some(TrialOutcome::Invalid) => invalid_trials += 1,
            Some(TrialOutcome::RightCensored) => censored_trials += 1,
            None => invalid_trials += 1,
        }
    }

    if invalid_trials > 0
        || aborted_trials > 0
        || withdrawn_trials > 0
        || censored_trials > 0
    {
        return Ok(Decision {
            disposition: Disposition::Indeterminate,
            unique_trials: unique.len(),
            successes,
            failures,
            restricted_outcomes,
            invalid_trials,
            aborted_trials,
            withdrawn_trials,
            censored_trials,
            duplicate_trials,
            lower_confidence_bound: None,
            meets_reliability_target: None,
            naive_success_fraction: if successes + failures > 0 {
                Some(successes as f64 / (successes + failures) as f64)
            } else {
                None
            },
            naive_baseline_false_assurance: false,
            minimum_zero_failure_trials: minimum_zero_failure_trials(
                case.reliability_target,
                case.confidence,
            )
            .ok(),
        });
    }

    let n = successes + failures;
    if n == 0 {
        return Ok(empty_decision(Disposition::Indeterminate, duplicate_trials));
    }
    let lower_bound = mission_reliability_lower_bound(n, failures, case.confidence)?;
    let meets_target = lower_bound >= case.reliability_target;
    let naive_fraction = successes as f64 / n as f64;
    let naive_baseline_false_assurance =
        naive_fraction >= case.reliability_target && !meets_target;
    let disposition = if meets_target {
        Disposition::ReliabilityDemonstrated
    } else {
        Disposition::ReliabilityNotDemonstrated
    };
    let required_zero_failure = minimum_zero_failure_trials(
        case.reliability_target,
        case.confidence,
    )?;

    Ok(Decision {
        disposition,
        unique_trials: n,
        successes,
        failures,
        restricted_outcomes,
        invalid_trials,
        aborted_trials,
        withdrawn_trials,
        censored_trials,
        duplicate_trials,
        lower_confidence_bound: Some(lower_bound),
        meets_reliability_target: Some(meets_target),
        naive_success_fraction: Some(naive_fraction),
        naive_baseline_false_assurance,
        minimum_zero_failure_trials: Some(required_zero_failure),
    })
}

pub fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    if corpus.schema != CORPUS_SCHEMA {
        return Err("unsupported reliability corpus schema".to_owned());
    }
    if corpus.evidence_class != "synthetic" {
        return Err("v0.1 accepts synthetic reliability fixtures only".to_owned());
    }
    if corpus.claim_ceiling != CLAIM_CEILING {
        return Err("unsupported reliability claim ceiling".to_owned());
    }
    if corpus.cases.is_empty() {
        return Err("reliability corpus must contain cases".to_owned());
    }

    let mut case_ids = HashSet::with_capacity(corpus.cases.len());
    let mut summary = Summary {
        case_count: corpus.cases.len(),
        reliability_demonstrated: 0,
        reliability_not_demonstrated: 0,
        indeterminate: 0,
        model_not_supported: 0,
        rejected: 0,
        disputed: 0,
        unique_trials: 0,
        successes: 0,
        failures: 0,
        restricted_outcomes: 0,
        invalid_trials: 0,
        aborted_trials: 0,
        withdrawn_trials: 0,
        censored_trials: 0,
        duplicate_trials: 0,
        naive_false_assurance_cases: 0,
        naive_cycle_false_assurance_cases: 0,
        expected_result_mismatches: 0,
    };
    let mut results = Vec::with_capacity(corpus.cases.len());

    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("reliability case IDs must be unique".to_owned());
        }
        let mut decision = evaluate_case(case)?;
        if case.reported_cycle_successes + case.reported_cycle_failures > 0 {
            let cycle_trials = case.reported_cycle_successes + case.reported_cycle_failures;
            if cycle_trials <= MAX_TRIALS {
                let cycle_lower = mission_reliability_lower_bound(
                    cycle_trials,
                    case.reported_cycle_failures,
                    case.confidence,
                )?;
                decision.naive_cycle_lower_bound = Some(cycle_lower);
                decision.naive_cycle_false_assurance = cycle_lower >= case.reliability_target
                    && decision.disposition != Disposition::ReliabilityDemonstrated;
            }
        }
        match decision.disposition {
            Disposition::ReliabilityDemonstrated => summary.reliability_demonstrated += 1,
            Disposition::ReliabilityNotDemonstrated => {
                summary.reliability_not_demonstrated += 1
            }
            Disposition::Indeterminate => summary.indeterminate += 1,
            Disposition::ModelNotSupported => summary.model_not_supported += 1,
            Disposition::Rejected => summary.rejected += 1,
            Disposition::Disputed => summary.disputed += 1,
        }
        summary.unique_trials += decision.unique_trials;
        summary.successes += decision.successes;
        summary.failures += decision.failures;
        summary.restricted_outcomes += decision.restricted_outcomes;
        summary.invalid_trials += decision.invalid_trials;
        summary.aborted_trials += decision.aborted_trials;
        summary.withdrawn_trials += decision.withdrawn_trials;
        summary.censored_trials += decision.censored_trials;
        summary.duplicate_trials += decision.duplicate_trials;
        summary.naive_false_assurance_cases +=
            usize::from(decision.naive_baseline_false_assurance);
        summary.naive_cycle_false_assurance_cases +=
            usize::from(decision.naive_cycle_false_assurance);

        let expected = &case.expected;
        let bound_matches = match (
            decision.lower_confidence_bound,
            expected.lower_confidence_bound,
        ) {
            (Some(actual), Some(target)) => (actual - target).abs() <= 1e-10,
            (None, None) => true,
            _ => false,
        };
        let fraction_matches = match (
            decision.naive_success_fraction,
            expected.naive_success_fraction,
        ) {
            (Some(actual), Some(target)) => (actual - target).abs() <= 1e-12,
            (None, None) => true,
            _ => false,
        };
        let matched = decision.disposition == expected.disposition
            && decision.unique_trials == expected.unique_trials
            && decision.successes == expected.successes
            && decision.failures == expected.failures
            && decision.restricted_outcomes == expected.restricted_outcomes
            && decision.invalid_trials == expected.invalid_trials
            && decision.aborted_trials == expected.aborted_trials
            && decision.withdrawn_trials == expected.withdrawn_trials
            && decision.censored_trials == expected.censored_trials
            && decision.duplicate_trials == expected.duplicate_trials
            && bound_matches
            && decision.meets_reliability_target == expected.meets_reliability_target
            && fraction_matches
            && decision.naive_baseline_false_assurance == expected.naive_baseline_false_assurance
            && decision.minimum_zero_failure_trials == expected.minimum_zero_failure_trials
            && match (decision.naive_cycle_lower_bound, expected.naive_cycle_lower_bound) {
                (Some(actual), Some(target)) => (actual - target).abs() <= 1e-10,
                (None, None) => true,
                _ => false,
            }
            && decision.naive_cycle_false_assurance == expected.naive_cycle_false_assurance;
        if !matched {
            summary.expected_result_mismatches += 1;
        }

        results.push(CaseResult {
            case_id: case.case_id.clone(),
            disposition: decision.disposition,
            unique_trials: decision.unique_trials,
            successes: decision.successes,
            failures: decision.failures,
            restricted_outcomes: decision.restricted_outcomes,
            invalid_trials: decision.invalid_trials,
            aborted_trials: decision.aborted_trials,
            withdrawn_trials: decision.withdrawn_trials,
            censored_trials: decision.censored_trials,
            duplicate_trials: decision.duplicate_trials,
            lower_confidence_bound: decision.lower_confidence_bound,
            meets_reliability_target: decision.meets_reliability_target,
            naive_success_fraction: decision.naive_success_fraction,
            naive_baseline_false_assurance: decision.naive_baseline_false_assurance,
            minimum_zero_failure_trials: decision.minimum_zero_failure_trials,
            naive_cycle_lower_bound: decision.naive_cycle_lower_bound,
            naive_cycle_false_assurance: decision.naive_cycle_false_assurance,
            expected_result_matched: matched,
        });
    }

    Ok(Report {
        result: if summary.expected_result_mismatches == 0 {
            "pass".to_owned()
        } else {
            "fail".to_owned()
        },
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        authority_granted: false,
        summary,
        cases: results,
    })
}

