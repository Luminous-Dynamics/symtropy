//! Cluster-aware uncertainty for adversarial benchmark red-team campaigns.
//!
//! The independent unit is the attack-author × generator pair (or clean-control
//! author × generator pair) within a threat family. Repeated scenarios from the
//! same pair do not inflate inferential n. Clopper-Pearson intervals are
//! numerically evaluated through regularized incomplete-beta quantiles and
//! outward-rounded to integer parts per million.

use super::adversarial_benchmark_redteam::{
    AdversarialBenchmarkSource, AdversarialRedTeamPlan, AdversarialRedTeamReceipt,
    AdversarialScenario, AdversarialTrial, ScenarioKind, ThreatFamily,
    evaluate_adversarial_redteam_campaign,
};
use std::collections::{BTreeMap, BTreeSet};

const FAMILY_COUNT: usize = 12;
const METRICS_PER_FAMILY: usize = 3;
const INTERVAL_SCALE: u32 = 1_000_000;
const MAX_SCENARIOS: usize = 100_000;
const MAX_BETA_ITERATIONS: usize = 512;
const MAX_QUANTILE_ITERATIONS: usize = 128;
const BETA_EPSILON: f64 = 3.0e-14;
const BETA_FPMIN: f64 = 1.0e-300;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedTeamClusterConfidencePlan {
    pub analysis_id: String,
    pub analysis_version: String,
    pub source_campaign_id: String,
    pub source_redteam_plan_digest: String,
    pub source_attack_catalog_digest: String,
    pub source_detector_artifact_digest: String,
    pub source_sealed_release_digest: String,
    pub analysis_plan_digest: String,
    /// Bonferroni family-wise error allocation across all families and metrics.
    pub familywise_alpha_numerator: u64,
    pub familywise_alpha_denominator: u64,
    pub minimum_attack_clusters_per_family: usize,
    pub minimum_clean_control_clusters_per_family: usize,
    pub minimum_successful_exploits_per_attack_cluster: usize,
    pub minimum_detection_lower_bound_ppm: u32,
    pub minimum_containment_lower_bound_ppm: u32,
    pub maximum_false_positive_upper_bound_ppm: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClopperPearsonInterval {
    pub successes: usize,
    pub trials: usize,
    pub lower_bound_ppm: u32,
    pub upper_bound_ppm: u32,
    pub confidence_method: String,
    pub numerical_quantile_tolerance: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThreatFamilyClusterConfidence {
    pub family: ThreatFamily,
    pub total_attack_clusters: usize,
    pub eligible_attack_clusters: usize,
    pub attack_clusters_without_successful_exploit: usize,
    pub attack_clusters_with_incomplete_trial: usize,
    pub detection_passing_clusters: usize,
    pub containment_passing_clusters: usize,
    pub total_clean_control_clusters: usize,
    pub eligible_clean_control_clusters: usize,
    pub clean_control_clusters_with_incomplete_trial: usize,
    pub clean_control_passing_clusters: usize,
    pub minimum_attack_clusters: usize,
    pub minimum_clean_control_clusters: usize,
    pub cluster_floor_passed: bool,
    pub detection_cluster_success_interval: Option<ClopperPearsonInterval>,
    pub containment_cluster_success_interval: Option<ClopperPearsonInterval>,
    pub clean_control_cluster_success_interval: Option<ClopperPearsonInterval>,
    pub detection_probability_gate_passed: bool,
    pub containment_probability_gate_passed: bool,
    pub false_positive_probability_gate_passed: bool,
    pub family_confidence_gate_passed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedTeamClusterConfidenceReceipt {
    pub analysis_id: String,
    pub analysis_version: String,
    pub source_campaign_id: String,
    pub source_redteam_plan_digest: String,
    pub source_attack_catalog_digest: String,
    pub source_detector_artifact_digest: String,
    pub source_sealed_release_digest: String,
    pub analysis_plan_digest: String,
    pub family_count: usize,
    pub metric_count: usize,
    pub simultaneous_familywise_coverage_percent: u8,
    pub familywise_alpha_numerator: u64,
    pub familywise_alpha_denominator: u64,
    pub source_campaign_empirical_gates_passed: bool,
    pub attack_clusters_are_author_generator_pairs: bool,
    pub clean_control_clusters_are_author_generator_pairs: bool,
    pub repeated_scenarios_not_counted_as_independent_clusters: bool,
    pub incomplete_or_no_success_attack_clusters_remain_visible: bool,
    pub intervals_are_two_sided_clopper_pearson: bool,
    pub bonferroni_allocation_covers_full_family_metric_grid: bool,
    pub confidence_results: Vec<ThreatFamilyClusterConfidence>,
    pub all_cluster_floors_passed: bool,
    pub all_detection_probability_gates_passed: bool,
    pub all_containment_probability_gates_passed: bool,
    pub all_false_positive_probability_gates_passed: bool,
    pub simultaneous_confidence_gates_passed: bool,
    pub claim_boundary: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClusterConfidenceError {
    InvalidPlan(String),
    InvalidIdentity(String),
    InvalidDigest(String),
    SourceCampaignMismatch(String),
    SourceCampaignNotPassed(String),
    ScenarioTrialMismatch(String),
    NumericDomain(String),
    NumericConvergence(String),
    ArithmeticOverflow(String),
}

impl std::fmt::Display for ClusterConfidenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlan(reason)
            | Self::InvalidIdentity(reason)
            | Self::InvalidDigest(reason)
            | Self::SourceCampaignMismatch(reason)
            | Self::SourceCampaignNotPassed(reason)
            | Self::ScenarioTrialMismatch(reason)
            | Self::NumericDomain(reason)
            | Self::NumericConvergence(reason)
            | Self::ArithmeticOverflow(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for ClusterConfidenceError {}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct ClusterKey {
    family: ThreatFamily,
    is_attack: bool,
    author_identity: String,
    generator_digest: String,
}

#[derive(Clone, Debug, Default)]
struct ClusterAccumulator {
    scenario_count: usize,
    incomplete_count: usize,
    successful_exploit_count: usize,
    correctly_detected_exploit_count: usize,
    contained_exploit_count: usize,
    leaked_exploit_count: usize,
    clean_alert_count: usize,
}

#[derive(Clone, Debug, Default)]
struct FamilyClusterAccumulator {
    attack_cluster_count: usize,
    eligible_attack_cluster_count: usize,
    no_success_attack_cluster_count: usize,
    incomplete_attack_cluster_count: usize,
    detection_passing_cluster_count: usize,
    containment_passing_cluster_count: usize,
    clean_cluster_count: usize,
    eligible_clean_cluster_count: usize,
    incomplete_clean_cluster_count: usize,
    clean_passing_cluster_count: usize,
}

fn require_id(value: &str, label: &str) -> Result<(), ClusterConfidenceError> {
    if value.trim().is_empty() || value.len() > 256 {
        return Err(ClusterConfidenceError::InvalidIdentity(format!(
            "{label} is empty or overlong"
        )));
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_digest(value: &str, label: &str) -> Result<(), ClusterConfidenceError> {
    if !valid_digest(value) {
        return Err(ClusterConfidenceError::InvalidDigest(format!(
            "{label} is not a canonical sha256 digest"
        )));
    }
    Ok(())
}

fn validate_plan(plan: &RedTeamClusterConfidencePlan) -> Result<f64, ClusterConfidenceError> {
    for (value, label) in [
        (&plan.analysis_id, "cluster confidence analysis ID"),
        (&plan.analysis_version, "cluster confidence analysis version"),
        (&plan.source_campaign_id, "source red-team campaign ID"),
    ] {
        require_id(value, label)?;
    }
    for (value, label) in [
        (&plan.source_redteam_plan_digest, "source red-team plan"),
        (&plan.source_attack_catalog_digest, "source attack catalog"),
        (&plan.source_detector_artifact_digest, "source detector artifact"),
        (&plan.source_sealed_release_digest, "source sealed release"),
        (&plan.analysis_plan_digest, "cluster confidence plan"),
    ] {
        require_digest(value, label)?;
    }
    if plan.familywise_alpha_numerator == 0
        || plan.familywise_alpha_denominator <= plan.familywise_alpha_numerator
        || plan.minimum_attack_clusters_per_family == 0
        || plan.minimum_clean_control_clusters_per_family == 0
        || plan.minimum_successful_exploits_per_attack_cluster == 0
        || plan.minimum_detection_lower_bound_ppm > INTERVAL_SCALE
        || plan.minimum_containment_lower_bound_ppm > INTERVAL_SCALE
        || plan.maximum_false_positive_upper_bound_ppm > INTERVAL_SCALE
    {
        return Err(ClusterConfidenceError::InvalidPlan(
            "alpha must be in (0,1), cluster floors and per-cluster successful-exploit floor must be positive, and probability thresholds must be in [0,1]".into(),
        ));
    }
    Ok(plan.familywise_alpha_numerator as f64 / plan.familywise_alpha_denominator as f64)
}

fn log_gamma(z: f64) -> f64 {
    // Lanczos approximation, g=7, 9 coefficients.
    const COEFFICIENTS: [f64; 8] = [
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if z < 0.5 {
        return std::f64::consts::PI.ln()
            - (std::f64::consts::PI * z).sin().ln()
            - log_gamma(1.0 - z);
    }
    let shifted = z - 1.0;
    let mut x = 0.999_999_999_999_809_9;
    for (index, coefficient) in COEFFICIENTS.iter().enumerate() {
        x += coefficient / (shifted + index as f64 + 1.0);
    }
    let t = shifted + 7.5;
    0.918_938_533_204_672_7 + (shifted + 0.5) * t.ln() - t + x.ln()
}

fn beta_continued_fraction(a: f64, b: f64, x: f64) -> Result<f64, ClusterConfidenceError> {
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < BETA_FPMIN {
        d = BETA_FPMIN;
    }
    d = 1.0 / d;
    let mut h = d;
    for iteration in 1..=MAX_BETA_ITERATIONS {
        let m = iteration as f64;
        let m2 = 2.0 * m;
        let mut aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < BETA_FPMIN {
            d = BETA_FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < BETA_FPMIN {
            c = BETA_FPMIN;
        }
        d = 1.0 / d;
        h *= d * c;

        aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < BETA_FPMIN {
            d = BETA_FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < BETA_FPMIN {
            c = BETA_FPMIN;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() <= BETA_EPSILON {
            if h.is_finite() && h >= 0.0 {
                return Ok(h);
            }
            return Err(ClusterConfidenceError::NumericDomain(
                "beta continued fraction produced a non-finite or negative result".into(),
            ));
        }
    }
    Err(ClusterConfidenceError::NumericConvergence(
        "regularized incomplete beta continued fraction did not converge".into(),
    ))
}

fn regularized_incomplete_beta(
    x: f64,
    a: f64,
    b: f64,
) -> Result<f64, ClusterConfidenceError> {
    if !(0.0..=1.0).contains(&x) || a <= 0.0 || b <= 0.0 || !a.is_finite() || !b.is_finite() {
        return Err(ClusterConfidenceError::NumericDomain(
            "regularized incomplete beta requires x in [0,1] and finite positive shape parameters".into(),
        ));
    }
    if x == 0.0 {
        return Ok(0.0);
    }
    if x == 1.0 {
        return Ok(1.0);
    }
    let log_beta_factor = log_gamma(a + b) - log_gamma(a) - log_gamma(b)
        + a * x.ln()
        + b * (-x).ln_1p();
    let beta_factor = log_beta_factor.exp();
    let result = if x < (a + 1.0) / (a + b + 2.0) {
        beta_factor * beta_continued_fraction(a, b, x)? / a
    } else {
        1.0 - beta_factor * beta_continued_fraction(b, a, 1.0 - x)? / b
    };
    if !result.is_finite() {
        return Err(ClusterConfidenceError::NumericDomain(
            "regularized incomplete beta returned a non-finite value".into(),
        ));
    }
    Ok(result.clamp(0.0, 1.0))
}

fn beta_quantile(p: f64, a: f64, b: f64) -> Result<f64, ClusterConfidenceError> {
    if !(0.0..=1.0).contains(&p) || a <= 0.0 || b <= 0.0 {
        return Err(ClusterConfidenceError::NumericDomain(
            "beta quantile requires p in [0,1] and positive shape parameters".into(),
        ));
    }
    if p == 0.0 {
        return Ok(0.0);
    }
    if p == 1.0 {
        return Ok(1.0);
    }
    let mut lower = 0.0;
    let mut upper = 1.0;
    for _ in 0..MAX_QUANTILE_ITERATIONS {
        let midpoint = lower + (upper - lower) / 2.0;
        let cdf = regularized_incomplete_beta(midpoint, a, b)?;
        if cdf < p {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
        if upper - lower <= 2.0e-14 {
            break;
        }
    }
    let midpoint = lower + (upper - lower) / 2.0;
    if midpoint.is_finite() && (0.0..=1.0).contains(&midpoint) {
        Ok(midpoint)
    } else {
        Err(ClusterConfidenceError::NumericConvergence(
            "beta quantile search returned an invalid result".into(),
        ))
    }
}

/// Two-sided Clopper-Pearson exact binomial interval. The quantiles are
/// numerically evaluated and then rounded outward to integer ppm.
pub fn clopper_pearson_interval(
    successes: usize,
    trials: usize,
    interval_alpha: f64,
) -> Result<ClopperPearsonInterval, ClusterConfidenceError> {
    if trials == 0 || successes > trials
        || !(0.0..1.0).contains(&interval_alpha)
        || !interval_alpha.is_finite()
    {
        return Err(ClusterConfidenceError::NumericDomain(
            "Clopper-Pearson requires 0 <= successes <= trials, trials > 0 and alpha in (0,1)".into(),
        ));
    }
    let n = trials as f64;
    let k = successes as f64;
    let tail = interval_alpha / 2.0;
    let lower = if successes == 0 {
        0.0
    } else {
        beta_quantile(tail, k, n - k + 1.0)?
    };
    let upper = if successes == trials {
        1.0
    } else {
        beta_quantile(1.0 - tail, k + 1.0, n - k)?
    };
    let scale = f64::from(INTERVAL_SCALE);
    let lower_ppm = (lower * scale).floor().clamp(0.0, scale) as u32;
    let upper_ppm = (upper * scale).ceil().clamp(0.0, scale) as u32;
    if lower_ppm > upper_ppm {
        return Err(ClusterConfidenceError::NumericConvergence(
            "Clopper-Pearson lower bound exceeds upper bound after outward rounding".into(),
        ));
    }
    Ok(ClopperPearsonInterval {
        successes,
        trials,
        lower_bound_ppm: lower_ppm,
        upper_bound_ppm: upper_ppm,
        confidence_method: "two-sided Clopper-Pearson exact binomial interval; regularized incomplete-beta inversion".into(),
        numerical_quantile_tolerance: "bisection width <= 2e-14; bounds rounded outward to 1 ppm; numerical implementation requires independent reference-vector qualification".into(),
    })
}

fn confidence_interval(
    successes: usize,
    trials: usize,
    per_metric_alpha: f64,
) -> Result<ClopperPearsonInterval, ClusterConfidenceError> {
    clopper_pearson_interval(successes, trials, per_metric_alpha)
}

#[derive(Clone, Debug)]
struct ClusterMetricCounts {
    attack_clusters: usize,
    eligible_attack_clusters: usize,
    no_success_attack_clusters: usize,
    incomplete_attack_clusters: usize,
    detection_passing_clusters: usize,
    containment_passing_clusters: usize,
    clean_clusters: usize,
    eligible_clean_clusters: usize,
    incomplete_clean_clusters: usize,
    clean_passing_clusters: usize,
}

fn aggregate_clusters(
    plan: &RedTeamClusterConfidencePlan,
    scenarios: &[AdversarialScenario],
    trials: &[AdversarialTrial],
) -> Result<BTreeMap<ThreatFamily, ClusterMetricCounts>, ClusterConfidenceError> {
    if scenarios.len() > MAX_SCENARIOS || scenarios.len() != trials.len() {
        return Err(ClusterConfidenceError::ScenarioTrialMismatch(
            "scenario and trial counts must match within the configured cap".into(),
        ));
    }
    let mut trial_by_scenario = BTreeMap::new();
    for trial in trials {
        if trial_by_scenario.insert(trial.scenario_id.as_str(), trial).is_some() {
            return Err(ClusterConfidenceError::ScenarioTrialMismatch(
                "duplicate trial scenario ID encountered during cluster aggregation".into(),
            ));
        }
    }
    let mut clusters: BTreeMap<ClusterKey, ClusterAccumulator> = BTreeMap::new();
    for scenario in scenarios {
        let trial = trial_by_scenario.get(scenario.scenario_id.as_str()).ok_or_else(|| {
            ClusterConfidenceError::ScenarioTrialMismatch(
                "scenario has no matching trial during cluster aggregation".into(),
            )
        })?;
        let key = ClusterKey {
            family: scenario.family,
            is_attack: scenario.kind == ScenarioKind::Attack,
            author_identity: scenario.author_identity.clone(),
            generator_digest: scenario.generator_digest.clone(),
        };
        let cluster = clusters.entry(key).or_default();
        cluster.scenario_count = cluster.scenario_count.checked_add(1).ok_or_else(|| {
            ClusterConfidenceError::ArithmeticOverflow("cluster scenario count overflow".into())
        })?;
        if !trial.execution_complete {
            cluster.incomplete_count = cluster.incomplete_count.checked_add(1).ok_or_else(|| {
                ClusterConfidenceError::ArithmeticOverflow("cluster incomplete count overflow".into())
            })?;
        }
        if scenario.kind == ScenarioKind::Attack {
            if trial.exploit_succeeded {
                cluster.successful_exploit_count += 1;
                if trial.detector_alerted && trial.correctly_attributed_to_family {
                    cluster.correctly_detected_exploit_count += 1;
                }
                if trial.contained_before_exposure && !trial.leak_occurred {
                    cluster.contained_exploit_count += 1;
                }
                if trial.leak_occurred {
                    cluster.leaked_exploit_count += 1;
                }
            }
        } else if trial.detector_alerted {
            cluster.clean_alert_count += 1;
        }
    }
    if trial_by_scenario.len() != scenarios.len() {
        return Err(ClusterConfidenceError::ScenarioTrialMismatch(
            "trial and scenario ID sets differ".into(),
        ));
    }

    let mut results = ThreatFamily::ALL.into_iter()
        .map(|family| (family, ClusterMetricCounts {
            attack_clusters: 0,
            eligible_attack_clusters: 0,
            no_success_attack_clusters: 0,
            incomplete_attack_clusters: 0,
            detection_passing_clusters: 0,
            containment_passing_clusters: 0,
            clean_clusters: 0,
            eligible_clean_clusters: 0,
            incomplete_clean_clusters: 0,
            clean_passing_clusters: 0,
        }))
        .collect::<BTreeMap<_, _>>();

    for (key, cluster) in clusters {
        let result = results.get_mut(&key.family).ok_or_else(|| {
            ClusterConfidenceError::ScenarioTrialMismatch("cluster family is unregistered".into())
        })?;
        if key.is_attack {
            result.attack_clusters += 1;
            if cluster.incomplete_count > 0 {
                result.incomplete_attack_clusters += 1;
                continue;
            }
            if cluster.successful_exploit_count < plan.minimum_successful_exploits_per_attack_cluster {
                result.no_success_attack_clusters += 1;
                continue;
            }
            result.eligible_attack_clusters += 1;
            if cluster.correctly_detected_exploit_count == cluster.successful_exploit_count {
                result.detection_passing_clusters += 1;
            }
            if cluster.contained_exploit_count == cluster.successful_exploit_count
                && cluster.leaked_exploit_count == 0
            {
                result.containment_passing_clusters += 1;
            }
        } else {
            result.clean_clusters += 1;
            if cluster.incomplete_count > 0 {
                result.incomplete_clean_clusters += 1;
                continue;
            }
            result.eligible_clean_clusters += 1;
            if cluster.clean_alert_count == 0 {
                result.clean_passing_clusters += 1;
            }
        }
    }
    Ok(results)
}

pub fn analyze_redteam_cluster_confidence(
    plan: &RedTeamClusterConfidencePlan,
    source_plan: &AdversarialRedTeamPlan,
    source: &AdversarialBenchmarkSource,
    source_receipt: &AdversarialRedTeamReceipt,
    scenarios: &[AdversarialScenario],
    trials: &[AdversarialTrial],
) -> Result<RedTeamClusterConfidenceReceipt, ClusterConfidenceError> {
    let familywise_alpha = validate_plan(plan)?;
    let recomputed = evaluate_adversarial_redteam_campaign(source_plan, source, scenarios, trials)
        .map_err(|error| ClusterConfidenceError::SourceCampaignMismatch(error.to_string()))?;
    if &recomputed != source_receipt {
        return Err(ClusterConfidenceError::SourceCampaignMismatch(
            "supplied SIM-REL-053 receipt differs from exact reconstruction of source scenarios and trials".into(),
        ));
    }
    if plan.source_campaign_id != source_receipt.campaign_id
        || plan.source_redteam_plan_digest != source_receipt.plan_digest
        || plan.source_attack_catalog_digest != source_receipt.attack_catalog_digest
        || plan.source_detector_artifact_digest != source_receipt.detector_artifact_digest
        || plan.source_sealed_release_digest != source_receipt.sealed_release_digest
    {
        return Err(ClusterConfidenceError::SourceCampaignMismatch(
            "cluster confidence plan does not bind the exact SIM-REL-053 campaign identities".into(),
        ));
    }
    if !source_receipt.all_trials_complete
        || !source_receipt.all_scenarios_used_once
        || !source_receipt.independent_roles_separated
        || !source_receipt.resource_accounting_complete
    {
        return Err(ClusterConfidenceError::SourceCampaignNotPassed(
            "source campaign lacks complete trials, scenario bijection, role separation or resource accounting".into(),
        ));
    }

    let clustered = aggregate_clusters(plan, scenarios, trials)?;
    let per_metric_alpha = familywise_alpha / (FAMILY_COUNT as f64 * METRICS_PER_FAMILY as f64);
    let mut results = Vec::with_capacity(FAMILY_COUNT);
    for family in ThreatFamily::ALL {
        let counts = clustered.get(&family).ok_or_else(|| {
            ClusterConfidenceError::ScenarioTrialMismatch("required family missing from cluster aggregate".into())
        })?;
        let attack_floor = counts.attack_clusters >= plan.minimum_attack_clusters_per_family
            && counts.eligible_attack_clusters == counts.attack_clusters;
        let clean_floor = counts.clean_clusters >= plan.minimum_clean_control_clusters_per_family
            && counts.eligible_clean_clusters == counts.clean_clusters;
        let detection_interval = (counts.eligible_attack_clusters > 0)
            .then(|| confidence_interval(
                counts.detection_passing_clusters,
                counts.eligible_attack_clusters,
                per_metric_alpha,
            ))
            .transpose()?;
        let containment_interval = (counts.eligible_attack_clusters > 0)
            .then(|| confidence_interval(
                counts.containment_passing_clusters,
                counts.eligible_attack_clusters,
                per_metric_alpha,
            ))
            .transpose()?;
        let clean_interval = (counts.eligible_clean_clusters > 0)
            .then(|| confidence_interval(
                counts.clean_passing_clusters,
                counts.eligible_clean_clusters,
                per_metric_alpha,
            ))
            .transpose()?;
        let detection_passed = detection_interval.as_ref()
            .is_some_and(|interval| interval.lower_bound_ppm >= plan.minimum_detection_lower_bound_ppm);
        let containment_passed = containment_interval.as_ref()
            .is_some_and(|interval| interval.lower_bound_ppm >= plan.minimum_containment_lower_bound_ppm);
        let false_positive_passed = clean_interval.as_ref()
            .is_some_and(|interval| {
                INTERVAL_SCALE.saturating_sub(interval.lower_bound_ppm)
                    <= plan.maximum_false_positive_upper_bound_ppm
            });
        let floor_passed = attack_floor && clean_floor;
        let family_passed = floor_passed && detection_passed && containment_passed && false_positive_passed;
        results.push(ThreatFamilyClusterConfidence {
            family,
            total_attack_clusters: counts.attack_clusters,
            eligible_attack_clusters: counts.eligible_attack_clusters,
            attack_clusters_without_successful_exploit: counts.no_success_attack_clusters,
            attack_clusters_with_incomplete_trial: counts.incomplete_attack_clusters,
            detection_passing_clusters: counts.detection_passing_clusters,
            containment_passing_clusters: counts.containment_passing_clusters,
            total_clean_control_clusters: counts.clean_clusters,
            eligible_clean_control_clusters: counts.eligible_clean_clusters,
            clean_control_clusters_with_incomplete_trial: counts.incomplete_clean_clusters,
            clean_control_passing_clusters: counts.clean_passing_clusters,
            minimum_attack_clusters: plan.minimum_attack_clusters_per_family,
            minimum_clean_control_clusters: plan.minimum_clean_control_clusters_per_family,
            cluster_floor_passed: floor_passed,
            detection_cluster_success_interval: detection_interval,
            containment_cluster_success_interval: containment_interval,
            clean_control_cluster_success_interval: clean_interval,
            detection_probability_gate_passed: detection_passed,
            containment_probability_gate_passed: containment_passed,
            false_positive_probability_gate_passed: false_positive_passed,
            family_confidence_gate_passed: family_passed,
        });
    }
    let all_cluster_floors_passed = results.iter().all(|result| result.cluster_floor_passed);
    let all_detection_probability_gates_passed = results.iter().all(|result| result.detection_probability_gate_passed);
    let all_containment_probability_gates_passed = results.iter().all(|result| result.containment_probability_gate_passed);
    let all_false_positive_probability_gates_passed = results.iter().all(|result| result.false_positive_probability_gate_passed);
    let simultaneous_confidence_gates_passed = source_receipt.campaign_integrity_passed
        && all_cluster_floors_passed
        && all_detection_probability_gates_passed
        && all_containment_probability_gates_passed
        && all_false_positive_probability_gates_passed;

    Ok(RedTeamClusterConfidenceReceipt {
        analysis_id: plan.analysis_id.clone(),
        analysis_version: plan.analysis_version.clone(),
        source_campaign_id: source_receipt.campaign_id.clone(),
        source_redteam_plan_digest: source_receipt.plan_digest.clone(),
        source_attack_catalog_digest: source_receipt.attack_catalog_digest.clone(),
        source_detector_artifact_digest: source_receipt.detector_artifact_digest.clone(),
        source_sealed_release_digest: source_receipt.sealed_release_digest.clone(),
        analysis_plan_digest: plan.analysis_plan_digest.clone(),
        family_count: FAMILY_COUNT,
        metric_count: METRICS_PER_FAMILY,
        simultaneous_familywise_coverage_percent: u8::try_from(
            (100u64.checked_mul(
                plan.familywise_alpha_denominator - plan.familywise_alpha_numerator
            ).ok_or_else(|| ClusterConfidenceError::ArithmeticOverflow(
                "family-wise coverage percentage overflow".into()
            ))?) / plan.familywise_alpha_denominator
        ).map_err(|_| ClusterConfidenceError::ArithmeticOverflow(
            "family-wise coverage percentage exceeds u8".into()
        ))?,
        familywise_alpha_numerator: plan.familywise_alpha_numerator,
        familywise_alpha_denominator: plan.familywise_alpha_denominator,
        source_campaign_empirical_gates_passed: source_receipt.campaign_integrity_passed,
        attack_clusters_are_author_generator_pairs: true,
        clean_control_clusters_are_author_generator_pairs: true,
        repeated_scenarios_not_counted_as_independent_clusters: true,
        incomplete_or_no_success_attack_clusters_remain_visible: true,
        intervals_are_two_sided_clopper_pearson: true,
        bonferroni_allocation_covers_full_family_metric_grid: true,
        confidence_results: results,
        all_cluster_floors_passed,
        all_detection_probability_gates_passed,
        all_containment_probability_gates_passed,
        all_false_positive_probability_gates_passed,
        simultaneous_confidence_gates_passed,
        claim_boundary: "Simultaneous probability bounds apply to the declared author×generator cluster population and registered threat families only. Identity strings do not prove organizational independence; unknown attack families are not bounded. Intervals use numerical beta quantiles with outward ppm rounding and require independent reference-vector qualification.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        RedTeamClusterConfidencePlan, aggregate_clusters, beta_quantile,
        clopper_pearson_interval, regularized_incomplete_beta,
    };
    use super::super::adversarial_benchmark_redteam::{
        AdversarialBenchmarkSource, AdversarialRedTeamPlan, AdversarialScenario,
        AdversarialTrial, RedTeamResourceChannel, RedTeamResourceMeasurement,
        ScenarioKind, ThreatFamily, ThreatFamilyRequirement,
        evaluate_adversarial_redteam_campaign,
    };
    use super::super::counterexample_ledger::EvidenceSplit;
    use super::super::matched_benchmark::BenchmarkScope;

    const D1: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const D2: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const D3: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const D4: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const D5: &str = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    const D6: &str = "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    fn source_plan() -> AdversarialRedTeamPlan {
        AdversarialRedTeamPlan {
            campaign_id: "red-team".into(),
            campaign_version: "v1".into(),
            benchmark_id: "science".into(),
            benchmark_version: "v1".into(),
            sealed_release_id: "release".into(),
            sealed_release_digest: D1.into(),
            source_hierarchical_analysis_id: "hierarchy".into(),
            source_hierarchical_analysis_digest: D2.into(),
            source_pre_registration_digest: D3.into(),
            plan_digest: D4.into(),
            threat_model_digest: D5.into(),
            attack_catalog_digest: D6.into(),
            detector_artifact_digest: D1.into(),
            attack_harness_digest: D2.into(),
            independent_verifier_artifact_digest: D3.into(),
            attack_operator_identity: "red-team-operator".into(),
            detector_owner_identity: "detector-owner".into(),
            independent_verifier_identity: "verifier".into(),
            scope: BenchmarkScope::FullScientificCampaign,
            family_requirements: ThreatFamily::ALL.into_iter().map(|family| ThreatFamilyRequirement {
                family,
                critical: true,
                minimum_attack_scenarios: 2,
                minimum_clean_controls: 1,
                minimum_successful_exploits: 1,
                minimum_attack_authors: 2,
                minimum_generator_variants: 2,
                minimum_clean_control_authors: 1,
                minimum_correct_detection_percent: 100,
                maximum_false_positive_percent: 0,
                maximum_uncontained_exploit_percent: 0,
            }).collect(),
            resource_channels: vec![
                RedTeamResourceChannel { channel_id: "wall-ms".into(), unit: "ms".into() },
            ],
        }
    }

    fn source() -> AdversarialBenchmarkSource {
        AdversarialBenchmarkSource {
            benchmark_id: "science".into(),
            benchmark_version: "v1".into(),
            sealed_release_id: "release".into(),
            sealed_release_digest: D1.into(),
            hierarchical_analysis_id: "hierarchy".into(),
            hierarchical_analysis_digest: D2.into(),
            pre_registration_digest: D3.into(),
            split: EvidenceSplit::SealedHoldout,
            scope: BenchmarkScope::FullScientificCampaign,
            sealed_holdout_only: true,
        }
    }

    fn scenarios_and_trials() -> (Vec<AdversarialScenario>, Vec<AdversarialTrial>) {
        let mut scenarios = Vec::new();
        let mut trials = Vec::new();
        for (family_index, family) in ThreatFamily::ALL.into_iter().enumerate() {
            for variant in 0..2 {
                let scenario_id = format!("{family:?}-attack-{variant}");
                let scenario = AdversarialScenario {
                    scenario_id: scenario_id.clone(),
                    family,
                    kind: ScenarioKind::Attack,
                    scenario_digest: format!("sha256:{:064x}", family_index * 10 + variant + 1),
                    generator_digest: if variant == 0 { D5.into() } else { D6.into() },
                    author_identity: format!("author-{family_index}-{variant}"),
                    split: EvidenceSplit::SealedHoldout,
                    development_exposed: false,
                };
                trials.push(AdversarialTrial {
                    trial_id: format!("trial-{scenario_id}"),
                    scenario_id,
                    execution_complete: true,
                    execution_receipt_digest: D1.into(),
                    attack_operator_identity: "red-team-operator".into(),
                    detector_owner_identity: "detector-owner".into(),
                    independent_verifier_identity: "verifier".into(),
                    detector_artifact_digest: D1.into(),
                    attack_harness_digest: D2.into(),
                    independent_verifier_artifact_digest: D3.into(),
                    exploit_succeeded: true,
                    detector_alerted: true,
                    correctly_attributed_to_family: true,
                    contained_before_exposure: true,
                    leak_occurred: false,
                    resource_measurements: vec![RedTeamResourceMeasurement {
                        channel_id: "wall-ms".into(),
                        unit: "ms".into(),
                        amount: 10,
                        meter_receipt_digest: D4.into(),
                    }],
                });
                scenarios.push(scenario);
            }
            let clean_id = format!("{family:?}-clean");
            scenarios.push(AdversarialScenario {
                scenario_id: clean_id.clone(),
                family,
                kind: ScenarioKind::CleanNegativeControl,
                scenario_digest: format!("sha256:{:064x}", family_index * 10 + 3),
                generator_digest: D4.into(),
                author_identity: format!("control-author-{family_index}"),
                split: EvidenceSplit::SealedHoldout,
                development_exposed: false,
            });
            trials.push(AdversarialTrial {
                trial_id: format!("trial-{clean_id}"),
                scenario_id: clean_id,
                execution_complete: true,
                execution_receipt_digest: D1.into(),
                attack_operator_identity: "red-team-operator".into(),
                detector_owner_identity: "detector-owner".into(),
                independent_verifier_identity: "verifier".into(),
                detector_artifact_digest: D1.into(),
                attack_harness_digest: D2.into(),
                independent_verifier_artifact_digest: D3.into(),
                exploit_succeeded: false,
                detector_alerted: false,
                correctly_attributed_to_family: false,
                contained_before_exposure: false,
                leak_occurred: false,
                resource_measurements: vec![RedTeamResourceMeasurement {
                    channel_id: "wall-ms".into(),
                    unit: "ms".into(),
                    amount: 5,
                    meter_receipt_digest: D4.into(),
                }],
            });
        }
        (scenarios, trials)
    }

    fn confidence_plan() -> RedTeamClusterConfidencePlan {
        RedTeamClusterConfidencePlan {
            analysis_id: "cluster-confidence".into(),
            analysis_version: "v1".into(),
            source_campaign_id: "red-team".into(),
            source_redteam_plan_digest: D4.into(),
            source_attack_catalog_digest: D6.into(),
            source_detector_artifact_digest: D1.into(),
            source_sealed_release_digest: D1.into(),
            analysis_plan_digest: D5.into(),
            familywise_alpha_numerator: 5,
            familywise_alpha_denominator: 100,
            minimum_attack_clusters_per_family: 2,
            minimum_clean_control_clusters_per_family: 1,
            minimum_successful_exploits_per_attack_cluster: 1,
            minimum_detection_lower_bound_ppm: 950_000,
            minimum_containment_lower_bound_ppm: 950_000,
            maximum_false_positive_upper_bound_ppm: 10_000,
        }
    }

    #[test]
    fn regularized_beta_and_quantile_match_uniform_distribution() {
        assert!((regularized_incomplete_beta(0.5, 1.0, 1.0).unwrap() - 0.5).abs() < 1.0e-10);
        assert!((beta_quantile(0.25, 1.0, 1.0).unwrap() - 0.25).abs() < 1.0e-10);
    }

    #[test]
    fn clopper_pearson_edges_match_closed_form_bounds() {
        let all_success = clopper_pearson_interval(10, 10, 0.05).unwrap();
        let all_failure = clopper_pearson_interval(0, 10, 0.05).unwrap();
        assert_eq!(all_success.upper_bound_ppm, 1_000_000);
        assert_eq!(all_failure.lower_bound_ppm, 0);
        assert!(all_success.lower_bound_ppm > 690_000);
        assert!(all_success.lower_bound_ppm < 693_000);
        assert!(all_failure.upper_bound_ppm > 307_000);
        assert!(all_failure.upper_bound_ppm < 310_000);
    }

    #[test]
    fn cluster_grouping_does_not_count_scenario_variants_as_independent_units() {
        let (scenarios, trials) = scenarios_and_trials();
        let mut plan = confidence_plan();
        plan.minimum_successful_exploits_per_attack_cluster = 1;
        let counts = aggregate_clusters(&plan, &scenarios, &trials).unwrap();
        let family = counts.get(&ThreatFamily::SearchTimeMetadataLeakage).unwrap();
        assert_eq!(family.attack_clusters, 2);
        assert_eq!(family.eligible_attack_clusters, 2);
        assert_eq!(family.clean_clusters, 1);
        assert_eq!(family.eligible_clean_clusters, 1);
    }

    #[test]
    fn repeated_scenarios_in_one_author_generator_cluster_do_not_increase_n() {
        let (mut scenarios, mut trials) = scenarios_and_trials();
        let mut repeated = scenarios[0].clone();
        repeated.scenario_id = "same-cluster-extra-scenario".into();
        repeated.scenario_digest = "sha256:9999999999999999999999999999999999999999999999999999999999999999".into();
        scenarios.insert(2, repeated.clone());
        let mut extra = trials[0].clone();
        extra.trial_id = "same-cluster-extra-trial".into();
        extra.scenario_id = repeated.scenario_id.clone();
        trials.insert(2, extra);
        // This raw cluster utility only groups by author × generator; the
        // source verifier will still enforce one row per distinct scenario.
        let mut plan = confidence_plan();
        plan.minimum_successful_exploits_per_attack_cluster = 1;
        let counts = aggregate_clusters(&plan, &scenarios, &trials).unwrap();
        let family = counts.get(&ThreatFamily::SearchTimeMetadataLeakage).unwrap();
        assert_eq!(family.attack_clusters, 2);
        assert_eq!(family.eligible_attack_clusters, 2);
    }

    #[test]
    fn interval_widens_after_familywise_bonferroni_allocation() {
        let pointwise = clopper_pearson_interval(10, 10, 0.05).unwrap();
        let familywise = clopper_pearson_interval(10, 10, 0.05 / 36.0).unwrap();
        assert!(familywise.lower_bound_ppm < pointwise.lower_bound_ppm);
    }

    #[test]
    fn source_campaign_reconstruction_is_a_required_input_to_confidence_layer() {
        let (scenarios, trials) = scenarios_and_trials();
        let source_plan = source_plan();
        let source = source();
        let source_receipt = evaluate_adversarial_redteam_campaign(
            &source_plan, &source, &scenarios, &trials
        ).unwrap();
        assert!(source_receipt.campaign_integrity_passed);
        let plan = confidence_plan();
        let result = super::analyze_redteam_cluster_confidence(
            &plan, &source_plan, &source, &source_receipt, &scenarios, &trials
        ).unwrap();
        assert_eq!(result.family_count, 12);
        assert_eq!(result.metric_count, 3);
        assert!(!result.simultaneous_confidence_gates_passed);
        assert_eq!(result.confidence_results[0].eligible_attack_clusters, 2);
        assert_eq!(result.confidence_results[0].eligible_clean_control_clusters, 1);
        assert!(result.bonferroni_allocation_covers_full_family_metric_grid);
    }
}
