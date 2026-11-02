//! Preregistered paired inference for matched scientific benchmark campaigns.
//!
//! The implementation is intentionally conservative:
//! - final claims accept SealedHoldout manifests only;
//! - directional effects use a two-sided exact paired sign test;
//! - mean paired loss uses an outward-rounded Hoeffding interval;
//! - Holm step-down controls family-wise error over the full preregistered
//!   mechanism-family set, including families with insufficient sample size.
//!
//! The sign test concerns directional preference among discordant pairs; it
//! is not a test of the mean. The Hoeffding interval concerns the mean and
//! assumes independent paired task instances sampled from the declared frame.

use super::counterexample_ledger::EvidenceSplit;
use super::matched_benchmark::{
    BenchmarkScope, BenchmarkTaskValidity, MatchedBenchmarkManifest,
    MatchedBenchmarkTrial, validate_and_summarize_matched_campaign,
};
use crate::Rational;
use std::collections::{BTreeMap, BTreeSet};

const MAX_ID_BYTES: usize = 256;
const MAX_FAMILIES: usize = 128;
const MAX_EXACT_DISCORDANT_PAIRS: usize = 120;
const SCALE: u128 = 1_000_000;
const SQRT_RADIUS_NUMERATOR: u128 = 2_000_000_000_000; // 2 * 10^12

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreregisteredPairedAnalysisPlan {
    pub analysis_id: String,
    pub analysis_version: String,
    pub campaign_id: String,
    pub scope: BenchmarkScope,
    pub group_id: String,
    pub agent_panel_id: String,
    pub task_manifest_digest: String,
    pub model_configuration_digest: String,
    pub prompt_configuration_digest: String,
    pub harness_digest: String,
    pub toolset_digest: String,
    pub reference_arm_id: String,
    pub candidate_arm_id: String,
    pub pre_registration_receipt_digest: String,
    pub family_wise_alpha: Rational,
    pub maximum_terminal_loss_units: u64,
    pub minimum_pairs_per_family: usize,
    /// Complete family set fixed before any SealedHoldout result is opened.
    pub mechanism_families: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactSignedMeanEffect {
    /// Mean paired loss difference is numerator / denominator.
    /// Negative values favor the candidate; positive values favor the reference.
    pub numerator: i128,
    pub denominator: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoeffdingMeanLossInterval {
    /// Bounds are in millionths of one terminal-loss unit.
    pub lower_micro_units: i128,
    pub upper_micro_units: i128,
    pub scale: u128,
    pub confidence_level_percent: u8,
    pub method: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyInferentialResult {
    pub mechanism_family_id: String,
    pub valid_pairs: usize,
    pub invalid_tasks_excluded: usize,
    pub candidate_wins: usize,
    pub reference_wins: usize,
    pub ties: usize,
    pub discordant_pairs: usize,
    pub exact_two_sided_sign_p_value: Rational,
    pub mean_paired_loss_difference: Option<ExactSignedMeanEffect>,
    pub mean_loss_95_percent_interval: Option<HoeffdingMeanLossInterval>,
    pub meets_minimum_pair_count: bool,
    pub holm_rank: usize,
    pub holm_threshold: Rational,
    pub rejected_after_holm: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreregisteredPairedInferenceReceipt {
    pub analysis_id: String,
    pub analysis_version: String,
    pub pre_registration_receipt_digest: String,
    pub campaign_id: String,
    pub scope: BenchmarkScope,
    pub split: EvidenceSplit,
    pub group_id: String,
    pub reference_arm_id: String,
    pub candidate_arm_id: String,
    pub family_wise_alpha: Rational,
    pub family_count: usize,
    pub total_valid_pairs: usize,
    pub total_invalid_tasks_excluded: usize,
    pub family_results: Vec<FamilyInferentialResult>,
    pub sealed_holdout_only: bool,
    pub all_preregistered_families_retained: bool,
    pub holm_family_wise_error_control_applied: bool,
    pub pointwise_intervals_not_simultaneous: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PairedInferenceError {
    InvalidPlan(String),
    ManifestMismatch(String),
    NonSealedEvidence(String),
    UnregisteredFamily(String),
    LossOutsidePreregisteredRange(String),
    UnsupportedExactSampleSize(String),
    ArithmeticOverflow(String),
    InvalidMatchedCampaign(String),
}

impl std::fmt::Display for PairedInferenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlan(reason)
            | Self::ManifestMismatch(reason)
            | Self::NonSealedEvidence(reason)
            | Self::UnregisteredFamily(reason)
            | Self::LossOutsidePreregisteredRange(reason)
            | Self::UnsupportedExactSampleSize(reason)
            | Self::ArithmeticOverflow(reason)
            | Self::InvalidMatchedCampaign(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for PairedInferenceError {}

#[derive(Clone, Debug, Default)]
struct FamilyCounts {
    valid_pairs: usize,
    invalid_tasks_excluded: usize,
    candidate_wins: usize,
    reference_wins: usize,
    ties: usize,
    loss_difference_sum: i128,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn validate_plan(plan: &PreregisteredPairedAnalysisPlan) -> Result<(), PairedInferenceError> {
    for (value, label) in [
        (&plan.analysis_id, "analysis ID"),
        (&plan.analysis_version, "analysis version"),
        (&plan.campaign_id, "campaign ID"),
        (&plan.group_id, "sealed group ID"),
        (&plan.agent_panel_id, "agent panel ID"),
        (&plan.task_manifest_digest, "task manifest digest"),
        (&plan.model_configuration_digest, "model configuration digest"),
        (&plan.prompt_configuration_digest, "prompt configuration digest"),
        (&plan.harness_digest, "harness digest"),
        (&plan.toolset_digest, "toolset digest"),
        (&plan.reference_arm_id, "reference arm ID"),
        (&plan.candidate_arm_id, "candidate arm ID"),
        (&plan.pre_registration_receipt_digest, "pre-registration receipt digest"),
    ] {
        if !valid_id(value) {
            return Err(PairedInferenceError::InvalidPlan(format!(
                "{label} is empty or overlong"
            )));
        }
    }
    if plan.reference_arm_id == plan.candidate_arm_id {
        return Err(PairedInferenceError::InvalidPlan(
            "reference and candidate arm IDs must differ".into(),
        ));
    }
    if plan.family_wise_alpha.is_zero() || !plan.family_wise_alpha.is_probability() {
        return Err(PairedInferenceError::InvalidPlan(
            "family-wise alpha must be in (0,1]".into(),
        ));
    }
    if plan.maximum_terminal_loss_units == 0 {
        return Err(PairedInferenceError::InvalidPlan(
            "maximum terminal-loss range must be positive".into(),
        ));
    }
    if plan.minimum_pairs_per_family == 0 {
        return Err(PairedInferenceError::InvalidPlan(
            "minimum pairs per family must be positive".into(),
        ));
    }
    if plan.mechanism_families.is_empty() || plan.mechanism_families.len() > MAX_FAMILIES {
        return Err(PairedInferenceError::InvalidPlan(format!(
            "preregistered family set must contain 1..={MAX_FAMILIES} families"
        )));
    }
    let mut seen = BTreeSet::new();
    for family in &plan.mechanism_families {
        if !valid_id(family) || !seen.insert(family.clone()) {
            return Err(PairedInferenceError::InvalidPlan(
                "preregistered family set contains an invalid or duplicate family ID".into(),
            ));
        }
    }
    Ok(())
}

fn validate_manifest_binding(
    plan: &PreregisteredPairedAnalysisPlan,
    manifest: &MatchedBenchmarkManifest,
) -> Result<(), PairedInferenceError> {
    if manifest.split != EvidenceSplit::SealedHoldout {
        return Err(PairedInferenceError::NonSealedEvidence(
            "final paired inference accepts SealedHoldout campaigns only".into(),
        ));
    }
    if plan.campaign_id != manifest.campaign_id
        || plan.scope != manifest.scope
        || plan.group_id != manifest.group_id
        || plan.agent_panel_id != manifest.agent_panel_id
        || plan.task_manifest_digest != manifest.task_manifest_digest
        || plan.model_configuration_digest != manifest.model_configuration_digest
        || plan.prompt_configuration_digest != manifest.prompt_configuration_digest
        || plan.harness_digest != manifest.harness_digest
        || plan.toolset_digest != manifest.toolset_digest
        || plan.reference_arm_id != manifest.reference_arm_id
        || plan.candidate_arm_id != manifest.candidate_arm_id
        || plan.minimum_pairs_per_family != manifest.minimum_pairs_per_family
    {
        return Err(PairedInferenceError::ManifestMismatch(
            "analysis plan does not match the pre-registered sealed campaign manifest".into(),
        ));
    }
    Ok(())
}

fn ceil_div_u128(numerator: u128, denominator: u128) -> u128 {
    numerator / denominator + u128::from(!numerator.is_multiple_of(denominator))
}

fn integer_sqrt_ceil(value: u128) -> Result<u128, PairedInferenceError> {
    let mut low = 0u128;
    let mut high = 2_000_001u128;
    while low < high {
        let middle = low + (high - low) / 2;
        let square = middle.checked_mul(middle).ok_or_else(|| {
            PairedInferenceError::ArithmeticOverflow("integer square overflow".into())
        })?;
        if square >= value {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    Ok(low)
}

fn binomial_coefficient(n: usize, k: usize) -> Result<u128, PairedInferenceError> {
    let k = k.min(n.saturating_sub(k));
    let mut value = 1u128;
    for index in 1..=k {
        let factor = u128::try_from(n - index + 1).map_err(|_| {
            PairedInferenceError::ArithmeticOverflow("binomial factor exceeds u128".into())
        })?;
        value = value.checked_mul(factor).ok_or_else(|| {
            PairedInferenceError::ArithmeticOverflow("binomial coefficient overflow".into())
        })? / u128::try_from(index).map_err(|_| {
            PairedInferenceError::ArithmeticOverflow("binomial divisor exceeds u128".into())
        })?;
    }
    Ok(value)
}

/// Two-sided non-randomized exact sign-test p-value, conditional on discordant
/// pairs. Ties are excluded. Exact u128 enumeration supports up to 120
/// discordant pairs; larger cases fail closed rather than approximate.
pub fn exact_two_sided_sign_test_p_value(
    candidate_wins: usize,
    reference_wins: usize,
) -> Result<Rational, PairedInferenceError> {
    let discordant = candidate_wins
        .checked_add(reference_wins)
        .ok_or_else(|| PairedInferenceError::ArithmeticOverflow(
            "discordant pair count overflow".into(),
        ))?;
    if discordant > MAX_EXACT_DISCORDANT_PAIRS {
        return Err(PairedInferenceError::UnsupportedExactSampleSize(format!(
            "{discordant} discordant pairs exceeds the exact u128 cap of {MAX_EXACT_DISCORDANT_PAIRS}"
        )));
    }
    if discordant == 0 {
        return Ok(Rational::one());
    }

    let tail_count = candidate_wins.min(reference_wins);
    let mut tail_sum = 0u128;
    for k in 0..=tail_count {
        tail_sum = tail_sum
            .checked_add(binomial_coefficient(discordant, k)?)
            .ok_or_else(|| PairedInferenceError::ArithmeticOverflow(
                "exact sign-test tail sum overflow".into(),
            ))?;
    }
    let doubled_tail = tail_sum.checked_mul(2).ok_or_else(|| {
        PairedInferenceError::ArithmeticOverflow("exact sign-test tail doubling overflow".into())
    })?;
    let denominator = 1u128.checked_shl(discordant as u32).ok_or_else(|| {
        PairedInferenceError::UnsupportedExactSampleSize(
            "exact sign-test denominator exceeds u128".into(),
        )
    })?;
    Rational::new(doubled_tail.min(denominator), denominator)
        .map_err(PairedInferenceError::ArithmeticOverflow)
}

fn floor_div_signed(numerator: i128, denominator: i128) -> i128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder < 0 { quotient - 1 } else { quotient }
}

fn ceil_div_signed(numerator: i128, denominator: i128) -> i128 {
    -floor_div_signed(-numerator, denominator)
}

/// Pointwise 95% Hoeffding interval for the mean paired loss difference.
///
/// Both arm losses lie in [0,L], so paired differences lie in [-L,L].
/// Hoeffding's radius 2L*sqrt(2/n) is conservative because ln(40) < 4.
/// Bounds are rounded outward to micro-loss units using integer arithmetic.
fn hoeffding_mean_interval(
    difference_sum: i128,
    pair_count: usize,
    maximum_loss: u64,
) -> Result<HoeffdingMeanLossInterval, PairedInferenceError> {
    if pair_count == 0 {
        return Err(PairedInferenceError::InvalidMatchedCampaign(
            "mean interval requires at least one valid pair".into(),
        ));
    }
    let n = u128::try_from(pair_count).map_err(|_| {
        PairedInferenceError::ArithmeticOverflow("pair count exceeds u128".into())
    })?;
    let n_signed = i128::try_from(pair_count).map_err(|_| {
        PairedInferenceError::ArithmeticOverflow("pair count exceeds i128".into())
    })?;
    let scaled_sum = difference_sum
        .checked_mul(i128::try_from(SCALE).map_err(|_| {
            PairedInferenceError::ArithmeticOverflow("micro scale exceeds i128".into())
        })?)
        .ok_or_else(|| PairedInferenceError::ArithmeticOverflow(
            "scaled mean numerator overflow".into(),
        ))?;
    let centre_lower = floor_div_signed(scaled_sum, n_signed);
    let centre_upper = ceil_div_signed(scaled_sum, n_signed);
    let radius_square = ceil_div_u128(SQRT_RADIUS_NUMERATOR, n);
    let radius_ppm = integer_sqrt_ceil(radius_square)?;
    let radius = i128::from(maximum_loss)
        .checked_mul(2)
        .and_then(|value| value.checked_mul(i128::try_from(radius_ppm).ok()?))
        .ok_or_else(|| PairedInferenceError::ArithmeticOverflow(
            "Hoeffding radius overflow".into(),
        ))?;
    let lower = centre_lower.checked_sub(radius).ok_or_else(|| {
        PairedInferenceError::ArithmeticOverflow("Hoeffding lower bound overflow".into())
    })?;
    let upper = centre_upper.checked_add(radius).ok_or_else(|| {
        PairedInferenceError::ArithmeticOverflow("Hoeffding upper bound overflow".into())
    })?;
    Ok(HoeffdingMeanLossInterval {
        lower_micro_units: lower,
        upper_micro_units: upper,
        scale: SCALE,
        confidence_level_percent: 95,
        method: "pointwise conservative Hoeffding; radius 2L*sqrt(2/n); outward integer micro-units".into(),
    })
}

fn apply_holm(
    results: &mut [FamilyInferentialResult],
    alpha: Rational,
) -> Result<(), PairedInferenceError> {
    let mut order = (0..results.len()).collect::<Vec<_>>();
    order.sort_by(|left, right| {
        results[*left]
            .exact_two_sided_sign_p_value
            .cmp(&results[*right].exact_two_sided_sign_p_value)
            .then_with(|| {
                results[*left]
                    .mechanism_family_id
                    .cmp(&results[*right].mechanism_family_id)
            })
    });

    let family_count = u64::try_from(results.len()).map_err(|_| {
        PairedInferenceError::ArithmeticOverflow("family count exceeds u64".into())
    })?;
    let mut still_rejecting = true;
    for (rank_index, family_index) in order.into_iter().enumerate() {
        let rank = rank_index + 1;
        let remaining = family_count - u64::try_from(rank_index).map_err(|_| {
            PairedInferenceError::ArithmeticOverflow("Holm rank exceeds u64".into())
        })?;
        let threshold = alpha
            .checked_div(Rational::from_u64(remaining))
            .map_err(PairedInferenceError::ArithmeticOverflow)?;
        let family = &mut results[family_index];
        family.holm_rank = rank;
        family.holm_threshold = threshold;
        let passes = family.exact_two_sided_sign_p_value <= threshold
            && family.meets_minimum_pair_count;
        family.rejected_after_holm = still_rejecting && passes;
        if !passes {
            still_rejecting = false;
        }
    }
    Ok(())
}

/// Produce a preregistered inferential receipt for one SealedHoldout campaign.
///
/// The plan fixes all campaign-identifying fields, arm identities, alpha,
/// loss range, minimum sample size and the complete mechanism-family set
/// before sealed outcomes are evaluated. Every registered family is retained
/// in the receipt even if no valid paired task remains for it.
pub fn analyze_sealed_matched_campaign(
    plan: &PreregisteredPairedAnalysisPlan,
    manifest: &MatchedBenchmarkManifest,
    trials: &[MatchedBenchmarkTrial],
) -> Result<PreregisteredPairedInferenceReceipt, PairedInferenceError> {
    validate_plan(plan)?;
    validate_manifest_binding(plan, manifest)?;

    let matched_receipt = validate_and_summarize_matched_campaign(manifest, trials)
        .map_err(|error| PairedInferenceError::InvalidMatchedCampaign(error.to_string()))?;
    let registered_families = plan
        .mechanism_families
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut counts = registered_families
        .iter()
        .cloned()
        .map(|family| (family, FamilyCounts::default()))
        .collect::<BTreeMap<_, _>>();

    for trial in trials {
        if !registered_families.contains(&trial.mechanism_family_id) {
            return Err(PairedInferenceError::UnregisteredFamily(format!(
                "observed mechanism family {} was not preregistered",
                trial.mechanism_family_id
            )));
        }
        if trial.task_validity == BenchmarkTaskValidity::Invalid {
            continue;
        }
        let reference = trial.reference.as_ref().ok_or_else(|| {
            PairedInferenceError::InvalidMatchedCampaign(
                "valid trial is missing reference arm outcome".into(),
            )
        })?;
        let candidate = trial.candidate.as_ref().ok_or_else(|| {
            PairedInferenceError::InvalidMatchedCampaign(
                "valid trial is missing candidate arm outcome".into(),
            )
        })?;
        if reference.terminal_loss_units > plan.maximum_terminal_loss_units
            || candidate.terminal_loss_units > plan.maximum_terminal_loss_units
        {
            return Err(PairedInferenceError::LossOutsidePreregisteredRange(format!(
                "task {} contains terminal loss above the preregistered maximum {}",
                trial.task_id, plan.maximum_terminal_loss_units
            )));
        }
        let difference = i128::from(candidate.terminal_loss_units)
            - i128::from(reference.terminal_loss_units);
        let family = counts
            .get_mut(&trial.mechanism_family_id)
            .ok_or_else(|| PairedInferenceError::UnregisteredFamily(
                "registered family disappeared during analysis".into(),
            ))?;
        family.valid_pairs = family.valid_pairs.checked_add(1).ok_or_else(|| {
            PairedInferenceError::ArithmeticOverflow("valid pair count overflow".into())
        })?;
        family.loss_difference_sum = family.loss_difference_sum.checked_add(difference).ok_or_else(|| {
            PairedInferenceError::ArithmeticOverflow("paired loss-difference sum overflow".into())
        })?;
        match difference.cmp(&0) {
            std::cmp::Ordering::Less => {
                family.candidate_wins = family.candidate_wins.checked_add(1).ok_or_else(|| {
                    PairedInferenceError::ArithmeticOverflow("candidate win count overflow".into())
                })?;
            }
            std::cmp::Ordering::Equal => {
                family.ties = family.ties.checked_add(1).ok_or_else(|| {
                    PairedInferenceError::ArithmeticOverflow("paired tie count overflow".into())
                })?;
            }
            std::cmp::Ordering::Greater => {
                family.reference_wins = family.reference_wins.checked_add(1).ok_or_else(|| {
                    PairedInferenceError::ArithmeticOverflow("reference win count overflow".into())
                })?;
            }
        }
    }

    for summary in &matched_receipt.family_summaries {
        if !registered_families.contains(&summary.mechanism_family_id) {
            return Err(PairedInferenceError::UnregisteredFamily(format!(
                "matched campaign contains unregistered family {}",
                summary.mechanism_family_id
            )));
        }
    }

    let mut family_results = Vec::with_capacity(plan.mechanism_families.len());
    for family_id in &plan.mechanism_families {
        let family = counts.get(family_id).cloned().unwrap_or_default();
        let discordant_pairs = family
            .candidate_wins
            .checked_add(family.reference_wins)
            .ok_or_else(|| PairedInferenceError::ArithmeticOverflow(
                "discordant count overflow".into(),
            ))?;
        let p_value = exact_two_sided_sign_test_p_value(
            family.candidate_wins,
            family.reference_wins,
        )?;
        let mean_effect = (family.valid_pairs > 0).then_some(ExactSignedMeanEffect {
            numerator: family.loss_difference_sum,
            denominator: family.valid_pairs as u128,
        });
        let mean_interval = if family.valid_pairs > 0 {
            Some(hoeffding_mean_interval(
                family.loss_difference_sum,
                family.valid_pairs,
                plan.maximum_terminal_loss_units,
            )?)
        } else {
            None
        };
        let invalid_tasks_excluded = matched_receipt
            .family_summaries
            .iter()
            .find(|summary| summary.mechanism_family_id == *family_id)
            .map(|summary| summary.invalid_tasks_excluded)
            .unwrap_or(0);
        family_results.push(FamilyInferentialResult {
            mechanism_family_id: family_id.clone(),
            valid_pairs: family.valid_pairs,
            invalid_tasks_excluded,
            candidate_wins: family.candidate_wins,
            reference_wins: family.reference_wins,
            ties: family.ties,
            discordant_pairs,
            exact_two_sided_sign_p_value: p_value,
            mean_paired_loss_difference: mean_effect,
            mean_loss_95_percent_interval: mean_interval,
            meets_minimum_pair_count: family.valid_pairs >= plan.minimum_pairs_per_family,
            holm_rank: 0,
            holm_threshold: Rational::zero(),
            rejected_after_holm: false,
        });
    }

    apply_holm(&mut family_results, plan.family_wise_alpha)?;
    let total_valid_pairs = family_results.iter().try_fold(0usize, |sum, family| {
        sum.checked_add(family.valid_pairs).ok_or_else(|| {
            PairedInferenceError::ArithmeticOverflow("total valid-pair count overflow".into())
        })
    })?;
    let total_invalid_tasks_excluded = family_results.iter().try_fold(0usize, |sum, family| {
        sum.checked_add(family.invalid_tasks_excluded).ok_or_else(|| {
            PairedInferenceError::ArithmeticOverflow("total invalid-task count overflow".into())
        })
    })?;

    Ok(PreregisteredPairedInferenceReceipt {
        analysis_id: plan.analysis_id.clone(),
        analysis_version: plan.analysis_version.clone(),
        pre_registration_receipt_digest: plan.pre_registration_receipt_digest.clone(),
        campaign_id: plan.campaign_id.clone(),
        scope: plan.scope,
        split: manifest.split,
        group_id: plan.group_id.clone(),
        reference_arm_id: plan.reference_arm_id.clone(),
        candidate_arm_id: plan.candidate_arm_id.clone(),
        family_wise_alpha: plan.family_wise_alpha,
        family_count: family_results.len(),
        total_valid_pairs,
        total_invalid_tasks_excluded,
        family_results,
        sealed_holdout_only: true,
        all_preregistered_families_retained: true,
        holm_family_wise_error_control_applied: true,
        pointwise_intervals_not_simultaneous: true,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        PreregisteredPairedAnalysisPlan, analyze_sealed_matched_campaign,
        exact_two_sided_sign_test_p_value, hoeffding_mean_interval,
    };
    use super::super::counterexample_ledger::EvidenceSplit;
    use super::super::matched_benchmark::{
        ArmOutcome, BenchmarkScope, BenchmarkTaskValidity, MatchedBenchmarkManifest,
        PairedBenchmarkTrial, ResourceBudget, ResourceUsage,
    };
    use crate::Rational;

    fn q(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn budget() -> ResourceBudget {
        ResourceBudget {
            max_wall_time_ms: 10_000,
            max_model_tokens: 20_000,
            max_tool_calls: 100,
            max_simulator_steps: 1_000,
            max_energy_millijoules: 50_000,
        }
    }

    fn manifest(split: EvidenceSplit) -> MatchedBenchmarkManifest {
        MatchedBenchmarkManifest {
            campaign_id: "campaign-sealed-1".into(),
            protocol_version: "v1".into(),
            scope: BenchmarkScope::PlannerOnly,
            split,
            group_id: "sealed-group-1".into(),
            agent_panel_id: "panel-fixed-v1".into(),
            task_manifest_digest: "task-manifest-sha256".into(),
            model_configuration_digest: "model-config-sha256".into(),
            prompt_configuration_digest: "prompt-config-sha256".into(),
            harness_digest: "harness-sha256".into(),
            toolset_digest: "toolset-sha256".into(),
            reference_arm_id: "baseline-eig".into(),
            candidate_arm_id: "symtropy-decision-aware".into(),
            resource_budget: budget(),
            minimum_pairs_per_family: 4,
        }
    }

    fn plan() -> PreregisteredPairedAnalysisPlan {
        PreregisteredPairedAnalysisPlan {
            analysis_id: "analysis-1".into(),
            analysis_version: "v1".into(),
            campaign_id: "campaign-sealed-1".into(),
            scope: BenchmarkScope::PlannerOnly,
            group_id: "sealed-group-1".into(),
            agent_panel_id: "panel-fixed-v1".into(),
            task_manifest_digest: "task-manifest-sha256".into(),
            model_configuration_digest: "model-config-sha256".into(),
            prompt_configuration_digest: "prompt-config-sha256".into(),
            harness_digest: "harness-sha256".into(),
            toolset_digest: "toolset-sha256".into(),
            reference_arm_id: "baseline-eig".into(),
            candidate_arm_id: "symtropy-decision-aware".into(),
            pre_registration_receipt_digest: "prereg-digest".into(),
            family_wise_alpha: q(5, 100),
            maximum_terminal_loss_units: 100,
            minimum_pairs_per_family: 4,
            mechanism_families: vec![
                "sensor-drift".into(),
                "omitted-physics".into(),
                "actuator-fault".into(),
            ],
        }
    }

    fn arm(id: &str, loss: u64, digest: &str) -> ArmOutcome {
        ArmOutcome {
            arm_id: id.into(),
            system_version: format!("{id}-v1"),
            task_completed: true,
            safe_success: true,
            hazard_observed: false,
            abstained: false,
            terminal_loss_units: loss,
            usage: ResourceUsage {
                wall_time_ms: 1_000,
                model_tokens: 2_000,
                tool_calls: 10,
                simulator_steps: 40,
                energy_millijoules: 3_000,
            },
            evidence_artifact_digest: digest.into(),
        }
    }

    fn trial(family: &str, pair: usize, reference_loss: u64, candidate_loss: u64) -> PairedBenchmarkTrial {
        let id = format!("{family}-{pair}");
        PairedBenchmarkTrial {
            pair_id: format!("pair-{id}"),
            task_id: format!("task-{id}"),
            mechanism_family_id: family.into(),
            paired_seed: pair as u64,
            task_validity: BenchmarkTaskValidity::Valid,
            task_oracle_receipt_digest: format!("oracle-{id}"),
            task_manifest_digest: "task-manifest-sha256".into(),
            model_configuration_digest: "model-config-sha256".into(),
            prompt_configuration_digest: "prompt-config-sha256".into(),
            harness_digest: "harness-sha256".into(),
            toolset_digest: "toolset-sha256".into(),
            scope: BenchmarkScope::PlannerOnly,
            split: EvidenceSplit::SealedHoldout,
            group_id: "sealed-group-1".into(),
            agent_panel_id: "panel-fixed-v1".into(),
            resource_budget: budget(),
            reference: Some(arm("baseline-eig", reference_loss, &format!("ref-{id}"))),
            candidate: Some(arm(
                "symtropy-decision-aware",
                candidate_loss,
                &format!("cand-{id}"),
            )),
        }
    }

    fn run(trials: &[PairedBenchmarkTrial]) -> super::PreregisteredPairedInferenceReceipt {
        analyze_sealed_matched_campaign(&plan(), &manifest(EvidenceSplit::SealedHoldout), trials)
            .unwrap()
    }

    #[test]
    fn exact_sign_test_matches_known_all_win_and_tie_fixtures() {
        assert_eq!(
            exact_two_sided_sign_test_p_value(8, 0).unwrap(),
            q(1, 128)
        );
        assert_eq!(
            exact_two_sided_sign_test_p_value(4, 2).unwrap(),
            q(11, 16)
        );
        assert_eq!(exact_two_sided_sign_test_p_value(0, 0).unwrap(), Rational::one());
        assert_eq!(
            exact_two_sided_sign_test_p_value(0, 8).unwrap(),
            q(1, 128)
        );
    }

    #[test]
    fn sign_test_exhaustive_small_domains_is_symmetric_and_bounded() {
        for wins in 0..=12 {
            for losses in 0..=12 {
                let forward = exact_two_sided_sign_test_p_value(wins, losses).unwrap();
                let reverse = exact_two_sided_sign_test_p_value(losses, wins).unwrap();
                assert_eq!(forward, reverse);
                assert!(forward.is_probability());
            }
        }
        assert!(exact_two_sided_sign_test_p_value(121, 0).is_err());
        assert!(exact_two_sided_sign_test_p_value(60, 60).is_ok());
    }

    #[test]
    fn sealed_analysis_preserves_effects_ties_and_underpowered_families() {
        let mut trials = Vec::new();
        for pair in 0..8 {
            trials.push(trial("sensor-drift", pair, 10, 8));
        }
        trials.push(trial("omitted-physics", 0, 10, 8));
        trials.push(trial("omitted-physics", 1, 10, 8));
        trials.push(trial("actuator-fault", 0, 10, 10));
        let result = run(&trials);
        assert_eq!(result.total_valid_pairs, 11);
        assert_eq!(result.family_count, 3);
        assert!(result.all_preregistered_families_retained);
        let sensor = result.family_results.iter().find(|f| f.mechanism_family_id == "sensor-drift").unwrap();
        assert_eq!(sensor.candidate_wins, 8);
        assert_eq!(sensor.exact_two_sided_sign_p_value, q(1, 128));
        assert!(sensor.rejected_after_holm);
        let physics = result.family_results.iter().find(|f| f.mechanism_family_id == "omitted-physics").unwrap();
        assert!(!physics.meets_minimum_pair_count);
        assert!(!physics.rejected_after_holm);
        let actuator = result.family_results.iter().find(|f| f.mechanism_family_id == "actuator-fault").unwrap();
        assert_eq!(actuator.ties, 1);
        assert_eq!(actuator.exact_two_sided_sign_p_value, Rational::one());
    }

    #[test]
    fn rejects_development_calibration_unregistered_family_and_loss_range_escape() {
        let one = vec![trial("sensor-drift", 0, 10, 8)];
        for split in [EvidenceSplit::Development, EvidenceSplit::Calibration] {
            assert!(analyze_sealed_matched_campaign(&plan(), &manifest(split), &one).is_err());
        }

        let extra = vec![trial("not-preregistered", 0, 10, 8)];
        assert!(run(&extra).is_err());

        let mut too_large = trial("sensor-drift", 0, 10, 101);
        too_large.candidate.as_mut().unwrap().terminal_loss_units = 101;
        assert!(run(&[too_large]).is_err());
    }

    #[test]
    fn mean_loss_interval_is_outward_and_covers_the_exact_sample_mean() {
        let interval = hoeffding_mean_interval(-10, 5, 100).unwrap();
        let mean_scaled = -2_000_000i128;
        assert!(interval.lower_micro_units <= mean_scaled);
        assert!(interval.upper_micro_units >= mean_scaled);
        assert!(interval.lower_micro_units <= interval.upper_micro_units);
        assert!(hoeffding_mean_interval(0, 0, 100).is_err());
    }

    #[test]
    fn rejects_manifest_or_preregistration_mismatch() {
        let mut trials = vec![trial("sensor-drift", 0, 10, 8)];
        let mut changed_manifest = manifest(EvidenceSplit::SealedHoldout);
        changed_manifest.harness_digest = "other-harness".into();
        assert!(analyze_sealed_matched_campaign(&plan(), &changed_manifest, &trials).is_err());

        changed_manifest = manifest(EvidenceSplit::SealedHoldout);
        changed_manifest.group_id = "other-group".into();
        assert!(analyze_sealed_matched_campaign(&plan(), &changed_manifest, &trials).is_err());

        trials[0].split = EvidenceSplit::Development;
        assert!(analyze_sealed_matched_campaign(&plan(), &manifest(EvidenceSplit::SealedHoldout), &trials).is_err());
    }
}
