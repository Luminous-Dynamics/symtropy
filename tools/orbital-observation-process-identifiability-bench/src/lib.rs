use serde::{Deserialize, Serialize};

pub mod pod;
use std::collections::HashSet;

const CORPUS_SCHEMA: &str = "luminous.orbital-observation-identifiability.corpus.v0.2";
const CLAIM_CEILING: &str = "synthetic-thinned-plp-identifiability-and-pod-bounds-only";
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const MAX_EVENTS: usize = 1_000_000;
const MAX_SIMULATION_DRAWS: usize = 20_000_000;
const PRNG_ID: &str = "splitmix64-top52-open-uniform-independent-streams-v1";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlpParameters {
    pub eta: f64,
    pub beta: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptureModel {
    pub p0: f64,
    pub gamma: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservedParameters {
    pub eta_observed: f64,
    pub beta_observed: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AnalyticCase {
    pub case_id: String,
    pub physical: PlpParameters,
    pub capture: CaptureModel,
    pub exposure_horizon: f64,
    pub decomposition_gammas: Vec<f64>,
    pub decomposition_p0: f64,
    pub expected_observed: ObservedParameters,
    pub expected_equivalent_physical_betas: Vec<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SensitivityCase {
    pub case_id: String,
    pub beta_observed: f64,
    pub gamma_min: f64,
    pub gamma_max: f64,
    pub expected_beta_min: f64,
    pub expected_beta_max: f64,
    pub expected_lower_open: bool,
    pub expected_identifiable: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationCase {
    pub case_id: String,
    pub physical_beta: f64,
    pub capture_p0: f64,
    pub capture_gamma: f64,
    pub physical_event_count: usize,
    pub exposure_horizon: f64,
    pub physical_seed: u64,
    pub capture_seed: u64,
    pub maximum_beta_error: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub analytic_cases: Vec<AnalyticCase>,
    pub sensitivity_cases: Vec<SensitivityCase>,
    pub simulation_cases: Vec<SimulationCase>,
    pub pod_trend_cases: Vec<pod::PodTrendCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EquivalentDecomposition {
    pub capture: CaptureModel,
    pub physical: PlpParameters,
    pub reproduced_observed: ObservedParameters,
    pub shape_residual: f64,
    pub scale_relative_residual: f64,
    pub observationally_equivalent: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalyticResult {
    pub case_id: String,
    pub observed: ObservedParameters,
    pub equivalent_decompositions: Vec<EquivalentDecomposition>,
    pub shape_identifiable_from_timestamps_alone: bool,
    pub scale_identifiable_from_timestamps_alone: bool,
    pub expected_result_matched: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BetaSensitivityInterval {
    pub beta_min: f64,
    pub beta_max: f64,
    pub lower_open: bool,
    pub identifiable: bool,
    pub gamma_min: f64,
    pub gamma_max: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SensitivityResult {
    pub case_id: String,
    pub interval: BetaSensitivityInterval,
    pub expected_result_matched: bool,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimulationResult {
    pub case_id: String,
    pub physical_beta: f64,
    pub capture_p0: f64,
    pub capture_gamma: f64,
    pub theoretical_observed_beta: f64,
    pub estimated_observed_beta: Option<f64>,
    pub physical_event_count: usize,
    pub captured_event_count: usize,
    pub empirical_capture_fraction: f64,
    pub theoretical_capture_fraction: f64,
    pub beta_absolute_error: Option<f64>,
    pub simulation_consistent_with_observed_plp: bool,
    pub physical_rng_seed: u64,
    pub capture_rng_seed: u64,
    pub prng_id: String,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub analytic_case_count: usize,
    pub equivalent_decomposition_count: usize,
    pub sensitivity_case_count: usize,
    pub analytic_expected_result_mismatches: usize,
    pub sensitivity_mismatches: usize,
    pub simulation_case_count: usize,
    pub pod_trend_case_count: usize,
    pub pod_trend_bounded_count: usize,
    pub pod_trend_unbounded_count: usize,
    pub pod_trend_incompatible_count: usize,
    pub pod_trend_indeterminate_count: usize,
    pub pod_trend_expected_result_mismatches: usize,
    pub simulation_consistent_count: usize,
    pub simulation_inconsistent_count: usize,
    pub shape_nonidentifiability_cases: usize,
    pub scale_nonidentifiability_cases: usize,
    pub physical_reliability_claims: usize,
    pub qualification_claims: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub mathematical_identity: String,
    pub physical_reliability_claim_granted: bool,
    pub qualification_granted: bool,
    pub authority_granted: bool,
    pub summary: Summary,
    pub analytic_cases: Vec<AnalyticResult>,
    pub sensitivity_cases: Vec<SensitivityResult>,
    pub simulation_cases: Vec<SimulationResult>,
    pub pod_trend_cases: Vec<pod::PodTrendResult>,
}

#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn uniform_open(&mut self) -> f64 {
        let numerator = (self.next_u64() >> 12) + 1;
        numerator as f64 / ((1_u64 << 52) + 1) as f64
    }
}

fn validate_physical_capture(
    physical: PlpParameters,
    capture: CaptureModel,
    exposure_horizon: f64,
) -> Result<(), String> {
    if !physical.eta.is_finite() || physical.eta <= 0.0
        || !physical.beta.is_finite() || physical.beta <= 0.0
        || !capture.p0.is_finite() || capture.p0 <= 0.0 || capture.p0 > 1.0
        || !capture.gamma.is_finite() || capture.gamma < 0.0
        || !exposure_horizon.is_finite() || exposure_horizon <= 0.0
    {
        return Err("physical PLP, capture model or horizon is outside the supported domain".to_owned());
    }
    if !(physical.beta + capture.gamma).is_finite() {
        return Err("observed PLP shape overflows the supported numeric domain".to_owned());
    }
    Ok(())
}

/// Independent thinning with p(t)=p0*(t/T)^gamma maps a physical PLP
/// (eta,beta) to an observed PLP with beta_obs=beta+gamma and
/// eta_obs=eta*p0*beta/(beta+gamma)*T^(-gamma).
pub fn observed_parameters(
    physical: PlpParameters,
    capture: CaptureModel,
    exposure_horizon: f64,
) -> Result<ObservedParameters, String> {
    validate_physical_capture(physical, capture, exposure_horizon)?;
    let beta_observed = physical.beta + capture.gamma;
    let eta_observed = physical.eta * capture.p0 * physical.beta / beta_observed
        * exposure_horizon.powf(-capture.gamma);
    if !eta_observed.is_finite() || eta_observed <= 0.0 {
        return Err("observed PLP scale is not finite and positive".to_owned());
    }
    Ok(ObservedParameters {
        eta_observed,
        beta_observed,
    })
}

/// Invert the observed scale/shape for a proposed capture trend gamma and
/// baseline detection p0. This produces a physical PLP that gives exactly
/// the same observed process under the independent-thinning assumptions.
pub fn equivalent_physical_decomposition(
    observed: ObservedParameters,
    capture: CaptureModel,
    exposure_horizon: f64,
) -> Result<PlpParameters, String> {
    if !observed.eta_observed.is_finite() || observed.eta_observed <= 0.0
        || !observed.beta_observed.is_finite() || observed.beta_observed <= 0.0
        || !capture.p0.is_finite() || capture.p0 <= 0.0 || capture.p0 > 1.0
        || !capture.gamma.is_finite() || capture.gamma < 0.0
        || capture.gamma >= observed.beta_observed
        || !exposure_horizon.is_finite() || exposure_horizon <= 0.0
    {
        return Err("observed parameters or proposed detection decomposition are invalid".to_owned());
    }
    let beta_physical = observed.beta_observed - capture.gamma;
    let eta_physical = observed.eta_observed * observed.beta_observed / beta_physical
        * exposure_horizon.powf(capture.gamma) / capture.p0;
    if !eta_physical.is_finite() || eta_physical <= 0.0 {
        return Err("equivalent physical PLP scale is not finite and positive".to_owned());
    }
    Ok(PlpParameters {
        eta: eta_physical,
        beta: beta_physical,
    })
}

pub fn physical_beta_sensitivity_interval(
    beta_observed: f64,
    gamma_min: f64,
    gamma_max: f64,
) -> Result<BetaSensitivityInterval, String> {
    if !beta_observed.is_finite() || beta_observed <= 0.0
        || !gamma_min.is_finite() || gamma_min < 0.0
        || !gamma_max.is_finite() || gamma_max < gamma_min
    {
        return Err("observed beta or detection-trend interval is invalid".to_owned());
    }
    if gamma_min >= beta_observed {
        return Err("detection-trend lower bound leaves no positive physical beta".to_owned());
    }
    let raw_lower = beta_observed - gamma_max;
    let lower_open = raw_lower <= 0.0;
    let beta_min = if lower_open { 0.0 } else { raw_lower };
    let beta_max = beta_observed - gamma_min;
    let width = beta_max - beta_min;
    let identifiable = !lower_open && width.abs() <= 1.0e-12;
    Ok(BetaSensitivityInterval {
        beta_min,
        beta_max,
        lower_open,
        identifiable,
        gamma_min,
        gamma_max,
    })
}

pub fn estimate_plp_shape(normalized_event_times: &[f64]) -> Result<f64, String> {
    if normalized_event_times.is_empty() {
        return Err("cannot estimate PLP shape from zero captured events".to_owned());
    }
    let mut sum = 0.0;
    let mut previous = 0.0;
    for time in normalized_event_times {
        if !time.is_finite() || *time <= 0.0 || *time >= 1.0 || *time <= previous {
            return Err("normalized event times must be unique, ordered and in (0,1)".to_owned());
        }
        sum += -time.ln();
        previous = *time;
    }
    let beta = normalized_event_times.len() as f64 / sum;
    if !beta.is_finite() || beta <= 0.0 {
        return Err("fitted observed PLP shape is invalid".to_owned());
    }
    Ok(beta)
}

/// Simulate a fixed number of physical PLP events, then thin each event using
/// a separate deterministic random stream for the observation process.
pub fn simulate_thinned_history(
    physical_beta: f64,
    capture: CaptureModel,
    physical_event_count: usize,
    physical_seed: u64,
    capture_seed: u64,
) -> Result<Vec<f64>, String> {
    if !physical_beta.is_finite() || physical_beta <= 0.0
        || !capture.p0.is_finite() || capture.p0 <= 0.0 || capture.p0 > 1.0
        || !capture.gamma.is_finite() || capture.gamma < 0.0
        || physical_event_count == 0 || physical_event_count > MAX_EVENTS
        || physical_seed > MAX_SAFE_INTEGER || capture_seed > MAX_SAFE_INTEGER
        || physical_seed == capture_seed
        || physical_event_count.saturating_mul(2) > MAX_SIMULATION_DRAWS
    {
        return Err("simulation configuration is outside the supported domain".to_owned());
    }

    let mut physical_rng = SplitMix64::new(physical_seed);
    let mut capture_rng = SplitMix64::new(capture_seed);
    let mut retained = Vec::new();
    for _ in 0..physical_event_count {
        let physical_uniform = physical_rng.uniform_open();
        let time = physical_uniform.powf(1.0 / physical_beta);
        let capture_probability = (capture.p0 * time.powf(capture.gamma)).min(1.0);
        if capture_rng.uniform_open() < capture_probability {
            retained.push(time);
        }
    }
    retained.sort_by(f64::total_cmp);
    Ok(retained)
}

fn evaluate_analytic_case(case: &AnalyticCase) -> Result<AnalyticResult, String> {
    if case.case_id.trim().is_empty()
        || !case.exposure_horizon.is_finite()
        || case.decomposition_gammas.is_empty()
    {
        return Err("analytic case identity, horizon or decomposition set is invalid".to_owned());
    }
    let observed = observed_parameters(case.physical, case.capture, case.exposure_horizon)?;
    let mut decompositions = Vec::with_capacity(case.decomposition_gammas.len());
    for gamma in &case.decomposition_gammas {
        let alternative_capture = CaptureModel {
            p0: case.decomposition_p0,
            gamma: *gamma,
        };
        let physical = equivalent_physical_decomposition(
            observed,
            alternative_capture,
            case.exposure_horizon,
        )?;
        let reproduced = observed_parameters(
            physical,
            alternative_capture,
            case.exposure_horizon,
        )?;
        let shape_residual = (reproduced.beta_observed - observed.beta_observed).abs();
        let scale_relative_residual =
            ((reproduced.eta_observed - observed.eta_observed) / observed.eta_observed).abs();
        let observationally_equivalent =
            shape_residual <= 1.0e-12 && scale_relative_residual <= 1.0e-12;
        decompositions.push(EquivalentDecomposition {
            capture: alternative_capture,
            physical,
            reproduced_observed: reproduced,
            shape_residual,
            scale_relative_residual,
            observationally_equivalent,
        });
    }
    let shape_identifiable = false;
    let equivalent_betas: Vec<f64> = decompositions
        .iter()
        .map(|item| item.physical.beta)
        .collect();
    let expected_result_matched = (observed.eta_observed - case.expected_observed.eta_observed).abs()
        <= 1.0e-10 * case.expected_observed.eta_observed.abs().max(1.0)
        && (observed.beta_observed - case.expected_observed.beta_observed).abs() <= 1.0e-12
        && equivalent_betas.len() == case.expected_equivalent_physical_betas.len()
        && equivalent_betas
            .iter()
            .zip(&case.expected_equivalent_physical_betas)
            .all(|(actual, expected)| (actual - expected).abs() <= 1.0e-10);
    let all_decompositions_equivalent =
        decompositions.iter().all(|item| item.observationally_equivalent);
    Ok(AnalyticResult {
        case_id: case.case_id.clone(),
        observed,
        equivalent_decompositions: decompositions,
        shape_identifiable_from_timestamps_alone: shape_identifiable,
        scale_identifiable_from_timestamps_alone: false,
        expected_result_matched: expected_result_matched && all_decompositions_equivalent,
        blockers: vec![
            "physical_shape_and_capture_trend_are_structurally_confounded_without_external_detection_evidence".to_owned(),
            "observed_scale_also_depends_on_baseline_detection_probability".to_owned(),
        ],
    })
}

fn evaluate_sensitivity_case(case: &SensitivityCase) -> Result<SensitivityResult, String> {
    let interval = physical_beta_sensitivity_interval(
        case.beta_observed,
        case.gamma_min,
        case.gamma_max,
    )?;
    let matched = (interval.beta_min - case.expected_beta_min).abs() <= 1.0e-12
        && (interval.beta_max - case.expected_beta_max).abs() <= 1.0e-12
        && interval.lower_open == case.expected_lower_open
        && interval.identifiable == case.expected_identifiable;
    Ok(SensitivityResult {
        case_id: case.case_id.clone(),
        interval,
        expected_result_matched: matched,
        blockers: if matched {
            Vec::new()
        } else {
            vec!["physical_beta_sensitivity_interval_mismatch".to_owned()]
        },
    })
}

fn evaluate_simulation_case(case: &SimulationCase) -> Result<SimulationResult, String> {
    if case.case_id.trim().is_empty()
        || !case.exposure_horizon.is_finite()
        || case.exposure_horizon <= 0.0
        || !case.maximum_beta_error.is_finite()
        || case.maximum_beta_error <= 0.0
    {
        return Err("simulation case identity, horizon or tolerance is invalid".to_owned());
    }
    let retained = simulate_thinned_history(
        case.physical_beta,
        CaptureModel {
            p0: case.capture_p0,
            gamma: case.capture_gamma,
        },
        case.physical_event_count,
        case.physical_seed,
        case.capture_seed,
    )?;
    let theoretical_beta = case.physical_beta + case.capture_gamma;
    let estimated_beta = if retained.len() >= 5 {
        Some(estimate_plp_shape(&retained)?)
    } else {
        None
    };
    let beta_absolute_error = estimated_beta.map(|estimate| (estimate - theoretical_beta).abs());
    let theoretical_capture_fraction =
        case.capture_p0 * case.physical_beta / theoretical_beta;
    let empirical_capture_fraction = retained.len() as f64 / case.physical_event_count as f64;
    let consistent = beta_absolute_error.is_some_and(|error| error <= case.maximum_beta_error);
    Ok(SimulationResult {
        case_id: case.case_id.clone(),
        physical_beta: case.physical_beta,
        capture_p0: case.capture_p0,
        capture_gamma: case.capture_gamma,
        theoretical_observed_beta: theoretical_beta,
        estimated_observed_beta: estimated_beta,
        physical_event_count: case.physical_event_count,
        captured_event_count: retained.len(),
        empirical_capture_fraction,
        theoretical_capture_fraction,
        beta_absolute_error,
        simulation_consistent_with_observed_plp: consistent,
        physical_rng_seed: case.physical_seed,
        capture_rng_seed: case.capture_seed,
        prng_id: PRNG_ID.to_owned(),
        blockers: if consistent {
            vec!["synthetic_thinning_only_not_physical_measurement_evidence".to_owned()]
        } else {
            vec![
                "captured_sample_too_small_or_observed_shape_outside_smoke_tolerance".to_owned(),
                "synthetic_thinning_only_not_physical_measurement_evidence".to_owned(),
            ]
        },
    })
}

pub fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.analytic_cases.is_empty()
        || corpus.sensitivity_cases.is_empty()
        || corpus.simulation_cases.is_empty()
        || corpus.pod_trend_cases.is_empty()
    {
        return Err("unsupported, empty or incorrectly classified identifiability corpus".to_owned());
    }
    let mut ids = HashSet::new();
    let mut analytic_cases = Vec::new();
    let mut sensitivity_cases = Vec::new();
    let mut simulation_cases = Vec::new();
    let mut pod_trend_results = Vec::new();
    let mut pod_trend_bounded_count = 0usize;
    let mut pod_trend_unbounded_count = 0usize;
    let mut pod_trend_incompatible_count = 0usize;
    let mut pod_trend_indeterminate_count = 0usize;
    let mut pod_trend_expected_result_mismatches = 0usize;
    let mut equivalent_decomposition_count = 0usize;
    let mut analytic_expected_result_mismatches = 0usize;
    let mut shape_nonidentifiability_cases = 0usize;
    let mut scale_nonidentifiability_cases = 0usize;
    let mut sensitivity_mismatches = 0usize;
    let mut simulation_consistent_count = 0usize;
    let mut simulation_inconsistent_count = 0usize;

    for case in &corpus.analytic_cases {
        if !ids.insert(case.case_id.as_str()) {
            return Err("identifiability case IDs must be globally unique".to_owned());
        }
        let result = evaluate_analytic_case(case)?;
        equivalent_decomposition_count += result.equivalent_decompositions
            .iter().filter(|item| item.observationally_equivalent).count();
        analytic_expected_result_mismatches += usize::from(!result.expected_result_matched);
        shape_nonidentifiability_cases += usize::from(!result.shape_identifiable_from_timestamps_alone);
        scale_nonidentifiability_cases += usize::from(!result.scale_identifiable_from_timestamps_alone);
        analytic_cases.push(result);
    }
    for case in &corpus.sensitivity_cases {
        if !ids.insert(case.case_id.as_str()) {
            return Err("identifiability case IDs must be globally unique".to_owned());
        }
        let result = evaluate_sensitivity_case(case)?;
        sensitivity_mismatches += usize::from(!result.expected_result_matched);
        sensitivity_cases.push(result);
    }
    for case in &corpus.simulation_cases {
        if !ids.insert(case.case_id.as_str()) {
            return Err("identifiability case IDs must be globally unique".to_owned());
        }
        let result = evaluate_simulation_case(case)?;
        if result.simulation_consistent_with_observed_plp {
            simulation_consistent_count += 1;
        } else {
            simulation_inconsistent_count += 1;
        }
        simulation_cases.push(result);
    }

    for case in &corpus.pod_trend_cases {
        if !ids.insert(case.case_id.as_str()) {
            return Err("identifiability case IDs must be globally unique".to_owned());
        }
        let result = pod::analyze_pod_trend(case)?;
        match result.status {
            pod::PodTrendStatus::Bounded => pod_trend_bounded_count += 1,
            pod::PodTrendStatus::Unbounded => pod_trend_unbounded_count += 1,
            pod::PodTrendStatus::ModelIncompatible => pod_trend_incompatible_count += 1,
            pod::PodTrendStatus::Indeterminate => pod_trend_indeterminate_count += 1,
        }
        pod_trend_expected_result_mismatches += usize::from(!result.expected_result_matched);
        pod_trend_results.push(result);
    }

    let result = if analytic_expected_result_mismatches == 0
        && sensitivity_mismatches == 0
        && simulation_inconsistent_count == 0
        && pod_trend_expected_result_mismatches == 0
        && equivalent_decomposition_count > 0
    {
        "pass"
    } else {
        "fail"
    };
    let summary = Summary {
        analytic_case_count: analytic_cases.len(),
        equivalent_decomposition_count,
        sensitivity_case_count: sensitivity_cases.len(),
        analytic_expected_result_mismatches,
        sensitivity_mismatches,
        simulation_case_count: simulation_cases.len(),
        pod_trend_case_count: pod_trend_results.len(),
        pod_trend_bounded_count,
        pod_trend_unbounded_count,
        pod_trend_incompatible_count,
        pod_trend_indeterminate_count,
        pod_trend_expected_result_mismatches,
        simulation_consistent_count,
        simulation_inconsistent_count,
        shape_nonidentifiability_cases,
        scale_nonidentifiability_cases,
        physical_reliability_claims: 0,
        qualification_claims: 0,
        authority_grants: 0,
    };
    Ok(Report {
        result: result.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        mathematical_identity:
            "beta_observed = beta_physical + gamma_detection".to_owned(),
        physical_reliability_claim_granted: false,
        qualification_granted: false,
        authority_granted: false,
        summary,
        analytic_cases,
        sensitivity_cases,
        simulation_cases,
        pod_trend_cases: pod_trend_results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observed_shape_is_physical_shape_plus_detection_trend() {
        let observed = observed_parameters(
            PlpParameters { eta: 2.0, beta: 0.8 },
            CaptureModel { p0: 0.9, gamma: 0.7 },
            100.0,
        )
        .expect("valid thinned PLP");
        assert!((observed.beta_observed - 1.5).abs() < 1.0e-12);
    }

    #[test]
    fn distinct_physical_and_detection_pairs_reproduce_same_observed_process() {
        let target = ObservedParameters { eta_observed: 0.25, beta_observed: 1.5 };
        let first_capture = CaptureModel { p0: 0.8, gamma: 0.0 };
        let second_capture = CaptureModel { p0: 0.8, gamma: 0.7 };
        let first = equivalent_physical_decomposition(target, first_capture, 100.0)
            .expect("first decomposition");
        let second = equivalent_physical_decomposition(target, second_capture, 100.0)
            .expect("second decomposition");
        let first_observed = observed_parameters(first, first_capture, 100.0).unwrap();
        let second_observed = observed_parameters(second, second_capture, 100.0).unwrap();
        assert!((first.beta - second.beta).abs() > 0.5);
        assert!((first_observed.beta_observed - second_observed.beta_observed).abs() < 1.0e-12);
        assert!((first_observed.eta_observed - second_observed.eta_observed).abs() < 1.0e-12);
    }

    #[test]
    fn external_gamma_interval_bounds_physical_beta() {
        let interval = physical_beta_sensitivity_interval(1.5, 0.2, 0.6).unwrap();
        assert!((interval.beta_min - 0.9).abs() < 1.0e-12);
        assert!((interval.beta_max - 1.3).abs() < 1.0e-12);
        assert!(!interval.identifiable);
        assert!(!interval.lower_open);
    }

    #[test]
    fn large_capture_trend_leaves_physical_beta_lower_bound_open() {
        let interval = physical_beta_sensitivity_interval(1.5, 0.2, 2.0).unwrap();
        assert_eq!(interval.beta_min, 0.0);
        assert!(interval.lower_open);
        assert!((interval.beta_max - 1.3).abs() < 1.0e-12);
    }

    #[test]
    fn frozen_identifiability_corpus_replays_without_claim_escalation() {
        let corpus: Corpus = serde_json::from_str(include_str!(
            "../fixtures/v0.2/identifiability-cases.json"
        ))
        .expect("identifiability corpus parses");
        let report = evaluate(&corpus).expect("identifiability corpus evaluates");
        assert_eq!(report.result, "pass");
        assert_eq!(report.summary.analytic_expected_result_mismatches, 0);
        assert_eq!(report.summary.sensitivity_mismatches, 0);
        assert_eq!(report.summary.simulation_inconsistent_count, 0);
        assert_eq!(report.summary.pod_trend_expected_result_mismatches, 0);
        assert!(report.summary.pod_trend_case_count >= 7);
        assert!(report.summary.shape_nonidentifiability_cases > 0);
        assert!(report.summary.scale_nonidentifiability_cases > 0);
        assert_eq!(report.summary.physical_reliability_claims, 0);
        assert_eq!(report.summary.qualification_claims, 0);
        assert_eq!(report.summary.authority_grants, 0);
        assert!(report.analytic_cases.iter().all(|case| {
            !case.shape_identifiable_from_timestamps_alone
                && !case.scale_identifiable_from_timestamps_alone
                && case.expected_result_matched
        }));
    }

    #[test]
    fn independent_thinning_simulation_recovers_observed_shape() {
        let retained = simulate_thinned_history(
            0.8,
            CaptureModel { p0: 0.9, gamma: 0.7 },
            200_000,
            123_456,
            654_321,
        )
        .expect("simulated thinned process");
        assert!(retained.len() > 5_000);
        let estimated = estimate_plp_shape(&retained).unwrap();
        assert!((estimated - 1.5).abs() < 0.04);
    }
}
