//! Prospective power qualification for paired virgin/recovered feedstock cohorts.
//!
//! Independent source-lot pairs are the top-level inferential units. Paired
//! build blocks are nested observations. Monte Carlo power is conditional on
//! the declared Gaussian random-effects model, pilot variance bounds, missing
//! at-random mechanism, endpoint and multiplicity policy. It is not a claim
//! about production capability or population transportability.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Maximize,
    Minimize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesignDecision {
    NotComparable,
    Indeterminate,
    Underpowered,
    TypeIUncontrolled,
    DesignQualified,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

impl Interval {
    pub const fn new(lower: f64, upper: f64) -> Self {
        Self { lower, upper }
    }

    pub fn is_valid_nonnegative(self) -> bool {
        self.lower.is_finite() && self.upper.is_finite() && self.lower >= 0.0 && self.lower <= self.upper
    }

    pub fn corners(self) -> [f64; 2] {
        [self.lower, self.upper]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VarianceBounds {
    /// Standard deviation of independent source-lot-pair treatment effects.
    pub between_lot_sd: Interval,
    /// Standard deviation of paired build-block differences nested in a lot pair.
    pub within_lot_build_sd: Interval,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesignCandidate {
    pub candidate_id: String,
    pub independent_lot_pairs: u32,
    pub paired_build_blocks_per_lot: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerSpec {
    pub campaign_id: String,
    pub endpoint_id: String,
    pub unit: String,
    pub direction: Direction,
    /// Positive margin in the direction where recovered feedstock is worse.
    pub noninferiority_margin: f64,
    /// Expected raw recovered-minus-virgin effect. Direction orients positive as better.
    pub target_effect: f64,
    /// Familywise one-sided alpha, divided by multiplicity_endpoints.
    pub familywise_alpha: f64,
    pub multiplicity_endpoints: u32,
    pub target_power: f64,
    pub maximum_type1_error_upper_bound: f64,
    pub minimum_power_lower_bound: f64,
    pub minimum_independent_lot_pairs: u32,
    pub minimum_build_pairs_per_lot: u32,
    pub build_pair_missing_probability: f64,
    pub variance_bounds: VarianceBounds,
    pub monte_carlo_replicates: u32,
    pub random_seed: u64,
    pub candidates: Vec<DesignCandidate>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProportionInterval {
    pub estimate: f64,
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VarianceScenarioResult {
    pub between_lot_sd: f64,
    pub within_lot_build_sd: f64,
    /// Type-I error conditional on having enough complete lot pairs to run the test.
    pub null_rejection_probability: ProportionInterval,
    /// Operational power: a trial without enough complete lot pairs counts as no success.
    pub alternative_power: ProportionInterval,
    pub null_analyzable_replicates: u32,
    pub alternative_analyzable_replicates: u32,
    pub null_analysis_coverage: ProportionInterval,
    pub alternative_analysis_coverage: ProportionInterval,
    pub mean_analyzable_lot_pairs: f64,
    pub probability_below_minimum_lot_pairs: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateEvaluation {
    pub candidate_id: String,
    pub independent_lot_pairs: u32,
    pub paired_build_blocks_per_lot: u32,
    pub total_planned_paired_build_blocks: u64,
    pub decision: DesignDecision,
    pub scenario_results: Vec<VarianceScenarioResult>,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PowerEvaluation {
    pub campaign_id: String,
    pub endpoint_id: String,
    pub alpha_per_endpoint: f64,
    pub decision: DesignDecision,
    pub recommended_candidate_id: Option<String>,
    pub candidates: Vec<CandidateEvaluation>,
    pub assumptions: Vec<String>,
    pub claim_ceiling: String,
}

pub fn evaluate_power_design(spec: &PowerSpec) -> PowerEvaluation {
    let global_blockers = validate_spec(spec);
    let alpha_per_endpoint = spec.familywise_alpha / f64::from(spec.multiplicity_endpoints);
    if !global_blockers.is_empty() {
        return PowerEvaluation {
            campaign_id: spec.campaign_id.clone(),
            endpoint_id: spec.endpoint_id.clone(),
            alpha_per_endpoint,
            decision: DesignDecision::NotComparable,
            recommended_candidate_id: None,
            candidates: Vec::new(),
            assumptions: model_assumptions(),
            claim_ceiling: "NotComparable: invalid or incomplete design specification".to_string(),
        };
    }

    let mut evaluations = spec
        .candidates
        .iter()
        .map(|candidate| evaluate_candidate(spec, candidate, alpha_per_endpoint))
        .collect::<Vec<_>>();

    evaluations.sort_by(|a, b| {
        a.total_planned_paired_build_blocks
            .cmp(&b.total_planned_paired_build_blocks)
            .then_with(|| b.independent_lot_pairs.cmp(&a.independent_lot_pairs))
            .then_with(|| a.candidate_id.cmp(&b.candidate_id))
    });

    let recommended_candidate_id = evaluations
        .iter()
        .find(|candidate| candidate.decision == DesignDecision::DesignQualified)
        .map(|candidate| candidate.candidate_id.clone());

    let decision = if recommended_candidate_id.is_some() {
        DesignDecision::DesignQualified
    } else if evaluations
        .iter()
        .any(|candidate| candidate.decision == DesignDecision::TypeIUncontrolled)
    {
        DesignDecision::TypeIUncontrolled
    } else if evaluations
        .iter()
        .any(|candidate| candidate.decision == DesignDecision::Indeterminate)
    {
        DesignDecision::Indeterminate
    } else {
        DesignDecision::Underpowered
    };

    PowerEvaluation {
        campaign_id: spec.campaign_id.clone(),
        endpoint_id: spec.endpoint_id.clone(),
        alpha_per_endpoint,
        decision,
        recommended_candidate_id,
        candidates: evaluations,
        assumptions: model_assumptions(),
        claim_ceiling: format!(
            "{decision:?}: simulated design-conditional power for one preregistered endpoint under stated nested Gaussian assumptions; no population, M5 or manufacturing capability claim"
        ),
    }
}

fn evaluate_candidate(
    spec: &PowerSpec,
    candidate: &DesignCandidate,
    alpha: f64,
) -> CandidateEvaluation {
    let mut blockers = Vec::new();
    if candidate.candidate_id.trim().is_empty() {
        blockers.push("MissingCandidateIdentity".to_string());
    }
    if candidate.independent_lot_pairs < spec.minimum_independent_lot_pairs {
        blockers.push("InsufficientIndependentLotPairs".to_string());
    }
    if candidate.paired_build_blocks_per_lot < spec.minimum_build_pairs_per_lot {
        blockers.push("InsufficientPairedBuildBlocksPerLot".to_string());
    }

    if !blockers.is_empty() {
        return CandidateEvaluation {
            candidate_id: candidate.candidate_id.clone(),
            independent_lot_pairs: candidate.independent_lot_pairs,
            paired_build_blocks_per_lot: candidate.paired_build_blocks_per_lot,
            total_planned_paired_build_blocks: u64::from(candidate.independent_lot_pairs)
                * u64::from(candidate.paired_build_blocks_per_lot),
            decision: DesignDecision::Underpowered,
            scenario_results: Vec::new(),
            blockers,
        };
    }

    let tau_values = spec.variance_bounds.between_lot_sd.corners();
    let sigma_values = spec.variance_bounds.within_lot_build_sd.corners();
    let mut scenario_results = Vec::new();
    let mut type_i_uncontrolled = false;
    let mut underpowered = false;
    let mut indeterminate = false;

    for (tau_index, tau) in tau_values.into_iter().enumerate() {
        for (sigma_index, sigma) in sigma_values.into_iter().enumerate() {
            let scenario_seed = mix_seed(
                spec.random_seed,
                u64::from(candidate.independent_lot_pairs),
                u64::from(candidate.paired_build_blocks_per_lot),
                (tau_index * 2 + sigma_index) as u64,
            );
            let result = simulate_scenario(spec, candidate, tau, sigma, alpha, scenario_seed);
            if result.null_rejection_probability.upper > spec.maximum_type1_error_upper_bound {
                type_i_uncontrolled = true;
            }
            if result.alternative_power.estimate < spec.target_power
                || result.alternative_power.lower < spec.minimum_power_lower_bound
            {
                underpowered = true;
            }
            if result.probability_below_minimum_lot_pairs > 0.05 {
                indeterminate = true;
            }
            scenario_results.push(result);
        }
    }

    let decision = if type_i_uncontrolled {
        DesignDecision::TypeIUncontrolled
    } else if indeterminate {
        DesignDecision::Indeterminate
    } else if underpowered {
        DesignDecision::Underpowered
    } else {
        DesignDecision::DesignQualified
    };

    CandidateEvaluation {
        candidate_id: candidate.candidate_id.clone(),
        independent_lot_pairs: candidate.independent_lot_pairs,
        paired_build_blocks_per_lot: candidate.paired_build_blocks_per_lot,
        total_planned_paired_build_blocks: u64::from(candidate.independent_lot_pairs)
            * u64::from(candidate.paired_build_blocks_per_lot),
        decision,
        scenario_results,
        blockers,
    }
}

fn simulate_scenario(
    spec: &PowerSpec,
    candidate: &DesignCandidate,
    between_sd: f64,
    within_sd: f64,
    alpha: f64,
    seed: u64,
) -> VarianceScenarioResult {
    let mut rng = XorShift64::new(seed);
    let mut null_rejections = 0_u32;
    let mut alternative_rejections = 0_u32;
    let mut null_analyzable_replicates = 0_u32;
    let mut alternative_analyzable_replicates = 0_u32;
    let mut analyzable_total = 0_u64;
    let mut below_minimum = 0_u32;

    let null_effect = -spec.noninferiority_margin;
    let alternative_effect = orient_effect(spec.target_effect, spec.direction);

    for _ in 0..spec.monte_carlo_replicates {
        let null_sample = simulate_lot_means(
            &mut rng,
            candidate,
            between_sd,
            within_sd,
            spec.build_pair_missing_probability,
            null_effect,
        );
        analyzable_total += u64::from(null_sample.len() as u32);
        if null_sample.len() < spec.minimum_independent_lot_pairs as usize {
            below_minimum += 1;
        } else {
            null_analyzable_replicates += 1;
            if one_sided_noninferiority_rejects(&null_sample, null_effect, alpha) {
                null_rejections += 1;
            }
        }

        let alternative_sample = simulate_lot_means(
            &mut rng,
            candidate,
            between_sd,
            within_sd,
            spec.build_pair_missing_probability,
            alternative_effect,
        );
        analyzable_total += u64::from(alternative_sample.len() as u32);
        if alternative_sample.len() < spec.minimum_independent_lot_pairs as usize {
            below_minimum += 1;
        } else {
            alternative_analyzable_replicates += 1;
            if one_sided_noninferiority_rejects(&alternative_sample, null_effect, alpha) {
                alternative_rejections += 1;
            }
        }
    }

    let total = spec.monte_carlo_replicates.saturating_mul(2).max(1);
    let null_ci = wilson_interval(null_rejections, null_analyzable_replicates);
    let power_ci = wilson_interval(alternative_rejections, spec.monte_carlo_replicates);
    let null_coverage = wilson_interval(null_analyzable_replicates, spec.monte_carlo_replicates);
    let alternative_coverage =
        wilson_interval(alternative_analyzable_replicates, spec.monte_carlo_replicates);
    let mean_analyzable = analyzable_total as f64 / f64::from(total);
    let below_probability = f64::from(below_minimum) / f64::from(total);

    VarianceScenarioResult {
        between_lot_sd: between_sd,
        within_lot_build_sd: within_sd,
        null_rejection_probability: null_ci,
        alternative_power: power_ci,
        null_analyzable_replicates,
        alternative_analyzable_replicates,
        null_analysis_coverage: null_coverage,
        alternative_analysis_coverage: alternative_coverage,
        mean_analyzable_lot_pairs: mean_analyzable,
        probability_below_minimum_lot_pairs: below_probability,
    }
}

fn simulate_lot_means(
    rng: &mut XorShift64,
    candidate: &DesignCandidate,
    between_sd: f64,
    within_sd: f64,
    missing_probability: f64,
    mean_effect: f64,
) -> Vec<f64> {
    let mut means = Vec::with_capacity(candidate.independent_lot_pairs as usize);
    for _ in 0..candidate.independent_lot_pairs {
        let lot_effect = mean_effect + between_sd * rng.standard_normal();
        let mut build_differences = Vec::new();
        let mut complete_lot_pair = true;
        for _ in 0..candidate.paired_build_blocks_per_lot {
            if rng.uniform() >= missing_probability {
                build_differences.push(lot_effect + within_sd * rng.standard_normal());
            } else {
                complete_lot_pair = false;
            }
        }
        // Complete-case lot-pair analysis preserves equal within-lot precision;
        // incomplete lot pairs remain missing in the planning denominator.
        if complete_lot_pair
            && build_differences.len() == candidate.paired_build_blocks_per_lot as usize
        {
            means.push(build_differences.iter().sum::<f64>() / build_differences.len() as f64);
        }
    }
    means
}

fn one_sample_t_statistic(values: &[f64], null_boundary: f64) -> Option<f64> {
    if values.len() < 2 || !null_boundary.is_finite() {
        return None;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|value| (value - mean).powi(2)).sum::<f64>() / (n - 1.0);
    if !variance.is_finite() {
        return None;
    }
    if variance <= f64::EPSILON {
        return Some(if mean > null_boundary {
            f64::INFINITY
        } else if mean < null_boundary {
            f64::NEG_INFINITY
        } else {
            0.0
        });
    }
    let se = (variance / n).sqrt();
    (se.is_finite() && se > 0.0).then_some((mean - null_boundary) / se)
}

fn one_sided_noninferiority_rejects(values: &[f64], null_boundary: f64, alpha: f64) -> bool {
    if values.len() < 2 {
        return false;
    }
    let Some(statistic) = one_sample_t_statistic(values, null_boundary) else {
        return false;
    };
    let critical = student_t_quantile(1.0 - alpha, (values.len() - 1) as f64);
    statistic > critical
}

fn orient_effect(raw_effect: f64, direction: Direction) -> f64 {
    match direction {
        Direction::Maximize => raw_effect,
        Direction::Minimize => -raw_effect,
    }
}

fn wilson_interval(successes: u32, trials: u32) -> ProportionInterval {
    if trials == 0 {
        return ProportionInterval {
            estimate: 0.0,
            lower: 0.0,
            upper: 1.0,
        };
    }
    let n = f64::from(trials);
    let p = f64::from(successes) / n;
    let z = 1.959_963_984_540_054;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let centre = (p + z2 / (2.0 * n)) / denominator;
    let half = z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / denominator;
    ProportionInterval {
        estimate: p,
        lower: (centre - half).max(0.0),
        upper: (centre + half).min(1.0),
    }
}

fn student_t_quantile(probability: f64, degrees_of_freedom: f64) -> f64 {
    let mut lower = -100.0;
    let mut upper = 100.0;
    for _ in 0..120 {
        let middle = (lower + upper) * 0.5;
        if student_t_cdf(middle, degrees_of_freedom) < probability {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    (lower + upper) * 0.5
}

fn student_t_cdf(value: f64, degrees_of_freedom: f64) -> f64 {
    if value == 0.0 {
        return 0.5;
    }
    let x = degrees_of_freedom / (degrees_of_freedom + value * value);
    let beta = regularized_incomplete_beta(x, degrees_of_freedom * 0.5, 0.5);
    if value > 0.0 {
        1.0 - 0.5 * beta
    } else {
        0.5 * beta
    }
}

fn regularized_incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let factor = (log_gamma(a + b) - log_gamma(a) - log_gamma(b)
        + a * x.ln()
        + b * (1.0 - x).ln())
    .exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        factor * beta_continued_fraction(a, b, x) / a
    } else {
        1.0 - factor * beta_continued_fraction(b, a, 1.0 - x) / b
    }
}

fn beta_continued_fraction(a: f64, b: f64, x: f64) -> f64 {
    const MAX_ITERATIONS: usize = 200;
    const EPSILON: f64 = 3.0e-14;
    const FLOOR: f64 = 1.0e-300;

    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < FLOOR {
        d = FLOOR;
    }
    d = 1.0 / d;
    let mut h = d;

    for m in 1..=MAX_ITERATIONS {
        let m_f = m as f64;
        let m2 = 2.0 * m_f;

        let mut aa = m_f * (b - m_f) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < FLOOR {
            d = FLOOR;
        }
        c = 1.0 + aa / c;
        if c.abs() < FLOOR {
            c = FLOOR;
        }
        d = 1.0 / d;
        h *= d * c;

        aa = -(a + m_f) * (qab + m_f) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < FLOOR {
            d = FLOOR;
        }
        c = 1.0 + aa / c;
        if c.abs() < FLOOR {
            c = FLOOR;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < EPSILON {
            break;
        }
    }
    h
}

fn log_gamma(z: f64) -> f64 {
    const COEFFICIENTS: [f64; 9] = [
        0.999_999_999_999_809_93,
        676.520_368_121_885_1,
        -1259.139_216_722_402_8,
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
    let mut x = COEFFICIENTS[0];
    for (index, coefficient) in COEFFICIENTS.iter().enumerate().skip(1) {
        x += coefficient / (shifted + index as f64);
    }
    let t = shifted + 7.5;
    0.5 * (2.0 * std::f64::consts::PI).ln()
        + (shifted + 0.5) * t.ln()
        - t
        + x.ln()
}

fn mix_seed(seed: u64, lots: u64, builds: u64, scenario: u64) -> u64 {
    let mut value = seed ^ lots.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= builds.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= scenario.wrapping_mul(0x94D0_49BB_1331_11EB);
    if value == 0 { 0xA076_1D64_78BD_642F } else { value }
}

struct XorShift64 {
    state: u64,
    spare_normal: Option<f64>,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xA076_1D64_78BD_642F } else { seed },
            spare_normal: None,
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn uniform(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) / 9_007_199_254_740_992.0
    }

    fn standard_normal(&mut self) -> f64 {
        if let Some(spare) = self.spare_normal.take() {
            return spare;
        }
        let u1 = self.uniform().max(f64::MIN_POSITIVE);
        let u2 = self.uniform();
        let radius = (-2.0 * u1.ln()).sqrt();
        let angle = 2.0 * std::f64::consts::PI * u2;
        self.spare_normal = Some(radius * angle.sin());
        radius * angle.cos()
    }
}

fn validate_spec(spec: &PowerSpec) -> Vec<String> {
    let mut blockers = Vec::new();
    if spec.campaign_id.trim().is_empty()
        || spec.endpoint_id.trim().is_empty()
        || spec.unit.trim().is_empty()
        || spec.candidates.is_empty()
        || !spec.noninferiority_margin.is_finite()
        || spec.noninferiority_margin < 0.0
        || !spec.target_effect.is_finite()
        || orient_effect(spec.target_effect, spec.direction) <= -spec.noninferiority_margin
        || !spec.familywise_alpha.is_finite()
        || spec.familywise_alpha <= 0.0
        || spec.familywise_alpha >= 0.5
        || spec.multiplicity_endpoints == 0
        || !spec.target_power.is_finite()
        || !(0.5..1.0).contains(&spec.target_power)
        || !spec.maximum_type1_error_upper_bound.is_finite()
        || !(0.0..1.0).contains(&spec.maximum_type1_error_upper_bound)
        || !spec.minimum_power_lower_bound.is_finite()
        || !(0.0..1.0).contains(&spec.minimum_power_lower_bound)
        || spec.minimum_independent_lot_pairs < 6
        || spec.minimum_build_pairs_per_lot < 3
        || !spec.build_pair_missing_probability.is_finite()
        || !(0.0..1.0).contains(&spec.build_pair_missing_probability)
        || !spec.variance_bounds.between_lot_sd.is_valid_nonnegative()
        || !spec.variance_bounds.within_lot_build_sd.is_valid_nonnegative()
        || spec.monte_carlo_replicates < 500
    {
        blockers.push("InvalidOrIncompletePowerSpecification".to_string());
    }
    let mut candidate_ids = BTreeSet::new();
    for candidate in &spec.candidates {
        if candidate.candidate_id.trim().is_empty()
            || !candidate_ids.insert(candidate.candidate_id.as_str())
        {
            blockers.push("DuplicateOrMissingCandidateId".to_string());
        }
    }
    blockers
}

fn model_assumptions() -> Vec<String> {
    vec![
        "Independent source-lot pairs are the top-level inferential units.".to_string(),
        "Build-pair observations are nested within source-lot pairs and do not create additional independent clusters.".to_string(),
        "Lot-pair effects and within-lot build errors are Gaussian with standard deviations varied over the preregistered pilot bounds.".to_string(),
        "Build-pair missingness is independent and at random at the declared probability; the primary inferential analysis uses complete lot pairs so each retained lot mean has equal nested precision.".to_string(),
        "The one-sided paired t test uses source-lot-pair means and n_lot_pairs - 1 degrees of freedom.".to_string(),
        "Familywise alpha is controlled with Bonferroni division across the preregistered endpoint count.".to_string(),
        "Monte Carlo Wilson intervals quantify simulation error, not uncertainty in the assumed variance model.".to_string(),
        "The design-qualified result is conditional on these assumptions and does not establish M5 process capability or population transportability.".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> PowerSpec {
        PowerSpec {
            campaign_id: "POWER-PAIRED-COHORT-001".to_string(),
            endpoint_id: "absolute_br_paired_difference".to_string(),
            unit: "T".to_string(),
            direction: Direction::Maximize,
            noninferiority_margin: 0.05,
            target_effect: 0.0,
            familywise_alpha: 0.05,
            multiplicity_endpoints: 1,
            target_power: 0.80,
            maximum_type1_error_upper_bound: 0.10,
            minimum_power_lower_bound: 0.70,
            minimum_independent_lot_pairs: 6,
            minimum_build_pairs_per_lot: 3,
            build_pair_missing_probability: 0.05,
            variance_bounds: VarianceBounds {
                between_lot_sd: Interval::new(0.01, 0.03),
                within_lot_build_sd: Interval::new(0.01, 0.04),
            },
            monte_carlo_replicates: 1000,
            random_seed: 0x1234_5678_9ABC_DEF0,
            candidates: vec![
                DesignCandidate {
                    candidate_id: "PILOT-3x3".to_string(),
                    independent_lot_pairs: 3,
                    paired_build_blocks_per_lot: 3,
                },
                DesignCandidate {
                    candidate_id: "CONFIRM-12x4".to_string(),
                    independent_lot_pairs: 12,
                    paired_build_blocks_per_lot: 4,
                },
            ],
        }
    }

    #[test]
    fn incomplete_three_lot_pilot_is_underpowered_for_inference() {
        let result = evaluate_power_design(&spec());
        let pilot = result
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "PILOT-3x3")
            .unwrap();
        assert_eq!(pilot.decision, DesignDecision::Underpowered);
        assert!(pilot.blockers.contains(&"InsufficientIndependentLotPairs".to_string()));
    }

    #[test]
    fn simulation_is_deterministic_for_fixed_seed() {
        let design = spec();
        let first = evaluate_power_design(&design);
        let second = evaluate_power_design(&design);
        assert_eq!(first, second);
    }

    #[test]
    fn each_candidate_evaluates_all_variance_interval_corners() {
        let result = evaluate_power_design(&spec());
        let qualified = result
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "CONFIRM-12x4")
            .unwrap();
        assert_eq!(qualified.scenario_results.len(), 4);
    }

    #[test]
    fn source_lot_pairs_are_the_reported_inferential_sample_size() {
        let result = evaluate_power_design(&spec());
        let candidate = result
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == "CONFIRM-12x4")
            .unwrap();
        assert_eq!(candidate.independent_lot_pairs, 12);
        assert_eq!(candidate.total_planned_paired_build_blocks, 48);
    }

    #[test]
    fn increasing_nested_builds_does_not_change_lot_pair_count() {
        let mut design = spec();
        design.candidates = vec![
            DesignCandidate {
                candidate_id: "12x3".to_string(),
                independent_lot_pairs: 12,
                paired_build_blocks_per_lot: 3,
            },
            DesignCandidate {
                candidate_id: "12x9".to_string(),
                independent_lot_pairs: 12,
                paired_build_blocks_per_lot: 9,
            },
        ];
        let result = evaluate_power_design(&design);
        assert!(result.candidates.iter().all(|candidate| candidate.independent_lot_pairs == 12));
    }

    #[test]
    fn more_independent_lot_pairs_are_preferred_when_build_budget_ties() {
        let mut design = spec();
        design.candidates = vec![
            DesignCandidate {
                candidate_id: "10x6".to_string(),
                independent_lot_pairs: 10,
                paired_build_blocks_per_lot: 6,
            },
            DesignCandidate {
                candidate_id: "12x5".to_string(),
                independent_lot_pairs: 12,
                paired_build_blocks_per_lot: 5,
            },
        ];
        let result = evaluate_power_design(&design);
        assert_eq!(result.candidates[0].candidate_id, "12x5");
    }

    #[test]
    fn bonferroni_multiplicity_adjustment_is_applied() {
        let mut design = spec();
        design.multiplicity_endpoints = 5;
        let result = evaluate_power_design(&design);
        assert!((result.alpha_per_endpoint - 0.01).abs() < 1e-12);
    }

    #[test]
    fn higher_missingness_reduces_analyzable_lot_count() {
        let mut low = spec();
        low.candidates = vec![DesignCandidate {
            candidate_id: "DESIGN".to_string(),
            independent_lot_pairs: 12,
            paired_build_blocks_per_lot: 3,
        }];
        low.build_pair_missing_probability = 0.0;
        let mut high = low.clone();
        high.build_pair_missing_probability = 0.90;
        let low_result = evaluate_power_design(&low);
        let high_result = evaluate_power_design(&high);
        let low_mean = low_result.candidates[0].scenario_results[0].mean_analyzable_lot_pairs;
        let high_mean = high_result.candidates[0].scenario_results[0].mean_analyzable_lot_pairs;
        assert!(high_mean < low_mean);
    }

    #[test]
    fn alpha_must_be_in_valid_one_sided_range() {
        let mut design = spec();
        design.familywise_alpha = 0.0;
        let result = evaluate_power_design(&design);
        assert_eq!(result.decision, DesignDecision::NotComparable);
    }

    #[test]
    fn variance_bounds_must_be_nonnegative_and_ordered() {
        let mut design = spec();
        design.variance_bounds.between_lot_sd = Interval::new(0.03, 0.01);
        let result = evaluate_power_design(&design);
        assert_eq!(result.decision, DesignDecision::NotComparable);
    }

    #[test]
    fn candidate_ids_must_be_unique() {
        let mut design = spec();
        design.candidates[1].candidate_id = design.candidates[0].candidate_id.clone();
        let result = evaluate_power_design(&design);
        assert_eq!(result.decision, DesignDecision::NotComparable);
    }

    #[test]
    fn too_few_monte_carlo_replicates_is_rejected() {
        let mut design = spec();
        design.monte_carlo_replicates = 100;
        let result = evaluate_power_design(&design);
        assert_eq!(result.decision, DesignDecision::NotComparable);
    }

    #[test]
    fn wilson_interval_contains_observed_proportion() {
        let result = wilson_interval(50, 100);
        assert!(result.lower <= result.estimate);
        assert!(result.estimate <= result.upper);
    }

    #[test]
    fn student_t_quantile_has_expected_one_sided_ordering() {
        let q = student_t_quantile(0.95, 10.0);
        assert!(q > 1.5 && q < 2.0);
    }

    #[test]
    fn student_t_cdf_is_symmetric() {
        let positive = student_t_cdf(1.7, 12.0);
        let negative = student_t_cdf(-1.7, 12.0);
        assert!((positive + negative - 1.0).abs() < 1e-10);
    }

    #[test]
    fn lot_level_student_t_uses_independent_lot_means() {
        let statistic = one_sample_t_statistic(&[0.1, 0.2, 0.0, 0.15, 0.05, 0.12], 0.0);
        assert!(statistic.is_some_and(|value| value > 0.0));
        let centered = one_sample_t_statistic(&[-0.05, -0.04, -0.06, -0.05, -0.04, -0.06], -0.05);
        assert!(centered.is_some_and(|value| value.abs() < 1e-10));
    }

    #[test]
    fn minimizing_endpoint_orients_negative_raw_effect_as_improvement() {
        assert_eq!(orient_effect(-0.2, Direction::Minimize), 0.2);
        assert_eq!(orient_effect(0.2, Direction::Maximize), 0.2);
    }

    #[test]
    fn type_i_test_is_centered_on_noninferiority_boundary_not_zero() {
        let boundary = -0.05;
        let sample = vec![-0.05, -0.04, -0.06, -0.05, -0.04, -0.06];
        let statistic = one_sample_t_statistic(&sample, boundary).unwrap();
        assert!(statistic.abs() < 1e-10);
        assert!(!one_sided_noninferiority_rejects(&sample, boundary, 0.05));
    }

    #[test]
    fn model_assumptions_are_reported_with_every_design() {
        let result = evaluate_power_design(&spec());
        assert!(result.assumptions.iter().any(|value| value.contains("top-level inferential units")));
        assert!(result.assumptions.iter().any(|value| value.contains("Gaussian")));
    }

    #[test]
    fn incomplete_nested_builds_drop_the_whole_lot_pair_from_primary_inference() {
        let mut design = spec();
        design.candidates = vec![DesignCandidate {
            candidate_id: "COMPLETE-CASE".to_string(),
            independent_lot_pairs: 12,
            paired_build_blocks_per_lot: 3,
        }];
        design.build_pair_missing_probability = 0.5;
        let result = evaluate_power_design(&design);
        let scenario = &result.candidates[0].scenario_results[0];
        assert!(scenario.mean_analyzable_lot_pairs < 12.0);
        assert!(scenario.probability_below_minimum_lot_pairs > 0.0);
    }

    #[test]
    fn type_one_error_is_conditioned_on_analysable_null_simulations() {
        let mut design = spec();
        design.candidates = vec![DesignCandidate {
            candidate_id: "DESIGN".to_string(),
            independent_lot_pairs: 12,
            paired_build_blocks_per_lot: 3,
        }];
        design.build_pair_missing_probability = 0.5;
        let result = evaluate_power_design(&design);
        let scenario = &result.candidates[0].scenario_results[0];
        assert!(scenario.null_analyzable_replicates <= design.monte_carlo_replicates);
        assert!(scenario.alternative_analyzable_replicates <= design.monte_carlo_replicates);
        assert!(scenario.null_analysis_coverage.estimate < 1.0);
        assert!(scenario.alternative_analysis_coverage.estimate < 1.0);
        assert!(scenario.null_rejection_probability.estimate <= 1.0);
        assert!(scenario.alternative_power.estimate <= scenario.alternative_coverage.estimate);
    }

    #[test]
    fn degenerate_zero_variance_samples_have_defined_boundary_behavior() {
        assert_eq!(one_sample_t_statistic(&[0.1, 0.1, 0.1], 0.0), Some(f64::INFINITY));
        assert_eq!(one_sample_t_statistic(&[-0.05, -0.05, -0.05], -0.05), Some(0.0));
    }

    #[test]
    fn seed_changes_the_simulated_estimates() {
        let mut first = spec();
        first.candidates = vec![DesignCandidate {
            candidate_id: "DESIGN".to_string(),
            independent_lot_pairs: 12,
            paired_build_blocks_per_lot: 4,
        }];
        let mut second = first.clone();
        second.random_seed = 0x0FED_CBA9_8765_4321;
        let a = evaluate_power_design(&first);
        let b = evaluate_power_design(&second);
        assert_ne!(
            a.candidates[0].scenario_results[0].alternative_power.estimate,
            b.candidates[0].scenario_results[0].alternative_power.estimate
        );
    }
}
