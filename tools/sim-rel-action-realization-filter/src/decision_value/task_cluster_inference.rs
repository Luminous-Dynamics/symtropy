//! Preregistered inferential comparison over SIM-REL-046 independent task
//! clusters. Repeated runs are collapsed within each task before inference;
//! the inferential n is the number of distinct valid family/task clusters.

use super::counterexample_ledger::EvidenceSplit;
use super::matched_benchmark::{BenchmarkScope, MatchedBenchmarkManifest, ResourceBudget};
use super::repeated_run_reliability::{
    RepeatedRunReliabilityPlan, RepeatedRunReliabilityReceipt, TaskClusterReliability,
};
use crate::Rational;
use std::collections::{BTreeMap, BTreeSet};

const MAX_ID_BYTES: usize = 256;
const MAX_FAMILIES: usize = 128;
const MAX_ENDPOINTS: usize = 4;
const MAX_EXACT_CLUSTER_DISCORDANT_PAIRS: usize = 120;
const MICRO_SCALE: i128 = 1_000_000;
const SQRT_RADIUS_NUMERATOR: u128 = 2_000_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ClusterInferenceEndpoint {
    PassAtK,
    PassEveryK,
    MeanRunSafeSuccessRate,
    AnyHazard,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskClusterInferencePlan {
    pub analysis_id: String,
    pub analysis_version: String,
    pub source_reliability_analysis_id: String,
    pub pre_registration_receipt_digest: String,
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
    pub resource_budget: ResourceBudget,
    pub repetitions_per_task: usize,
    pub minimum_valid_task_clusters_per_family: usize,
    pub family_wise_alpha: Rational,
    /// Complete family set fixed before SealedHoldout cluster outcomes are opened.
    pub mechanism_families: Vec<String>,
    /// Complete endpoint set fixed before evaluation. Abstention remains descriptive.
    pub endpoints: Vec<ClusterInferenceEndpoint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedClusterMeanEffect {
    /// Exact mean paired cluster difference: numerator / denominator.
    /// Positive values favor the candidate for all registered endpoint directions.
    pub numerator: i128,
    pub denominator: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HoeffdingClusterMeanInterval {
    /// Bounds are millionths of one normalized endpoint unit.
    pub lower_micro_units: i128,
    pub upper_micro_units: i128,
    pub scale: u128,
    pub confidence_level_percent: u8,
    pub method: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyEndpointInference {
    pub mechanism_family_id: String,
    pub endpoint: ClusterInferenceEndpoint,
    pub valid_task_clusters: usize,
    pub invalid_task_clusters_excluded: usize,
    pub required_valid_task_clusters: usize,
    pub meets_minimum_cluster_count: bool,
    pub candidate_cluster_wins: usize,
    pub reference_cluster_wins: usize,
    pub tied_clusters: usize,
    pub discordant_clusters: usize,
    pub exact_two_sided_sign_p_value: Rational,
    pub mean_paired_cluster_effect: Option<SignedClusterMeanEffect>,
    pub mean_effect_95_percent_interval: Option<HoeffdingClusterMeanInterval>,
    pub holm_rank: usize,
    pub holm_threshold: Rational,
    pub rejected_after_holm: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyAbstentionDescriptive {
    pub mechanism_family_id: String,
    pub valid_task_clusters: usize,
    pub candidate_clusters_with_any_abstention: usize,
    pub reference_clusters_with_any_abstention: usize,
    pub candidate_abstention_runs: usize,
    pub reference_abstention_runs: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskClusterInferenceReceipt {
    pub analysis_id: String,
    pub analysis_version: String,
    pub source_reliability_analysis_id: String,
    pub pre_registration_receipt_digest: String,
    pub campaign_id: String,
    pub scope: BenchmarkScope,
    pub split: EvidenceSplit,
    pub group_id: String,
    pub reference_arm_id: String,
    pub candidate_arm_id: String,
    pub repetitions_per_task: usize,
    pub family_count: usize,
    pub endpoint_count: usize,
    pub family_endpoint_hypothesis_count: usize,
    pub total_task_clusters: usize,
    pub total_valid_task_clusters: usize,
    pub total_invalid_task_clusters_excluded: usize,
    pub inferential_unit: String,
    pub family_endpoint_results: Vec<FamilyEndpointInference>,
    /// Abstentions are reported descriptively; no universal direction of
    /// superiority is assigned because abstention can be appropriate.
    pub family_abstention_descriptive: Vec<FamilyAbstentionDescriptive>,
    pub repeated_runs_not_counted_as_independent_samples: bool,
    pub cluster_level_pairing_preserved: bool,
    pub abstention_is_descriptive_only: bool,
    pub pointwise_intervals_not_simultaneous: bool,
    pub holm_family_wise_error_control_applied: bool,
    pub sealed_holdout_only: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClusterInferenceError {
    InvalidPlan(String),
    ManifestMismatch(String),
    NonSealedEvidence(String),
    FamilyRegistry(String),
    EndpointRegistry(String),
    ClusterReceipt(String),
    DuplicateCluster(String),
    InvalidCluster(String),
    UnsupportedExactSampleSize(String),
    ArithmeticOverflow(String),
}

impl std::fmt::Display for ClusterInferenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlan(reason)
            | Self::ManifestMismatch(reason)
            | Self::NonSealedEvidence(reason)
            | Self::FamilyRegistry(reason)
            | Self::EndpointRegistry(reason)
            | Self::ClusterReceipt(reason)
            | Self::DuplicateCluster(reason)
            | Self::InvalidCluster(reason)
            | Self::UnsupportedExactSampleSize(reason)
            | Self::ArithmeticOverflow(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for ClusterInferenceError {}

#[derive(Clone, Debug, Default)]
struct EndpointCounts {
    valid_task_clusters: usize,
    invalid_task_clusters: usize,
    candidate_wins: usize,
    reference_wins: usize,
    ties: usize,
    effect_sum: i128,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn valid_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_plan(
    plan: &TaskClusterInferencePlan,
    source_plan: &RepeatedRunReliabilityPlan,
    manifest: &MatchedBenchmarkManifest,
    source_receipt: &RepeatedRunReliabilityReceipt,
) -> Result<(), ClusterInferenceError> {
    for (value, label) in [
        (&plan.analysis_id, "analysis ID"),
        (&plan.analysis_version, "analysis version"),
        (
            &plan.source_reliability_analysis_id,
            "source reliability analysis ID",
        ),
        (&plan.campaign_id, "campaign ID"),
        (&plan.group_id, "sealed evidence group"),
        (&plan.agent_panel_id, "agent panel ID"),
        (&plan.task_manifest_digest, "task manifest digest"),
        (
            &plan.model_configuration_digest,
            "model configuration digest",
        ),
        (
            &plan.prompt_configuration_digest,
            "prompt configuration digest",
        ),
        (&plan.harness_digest, "harness digest"),
        (&plan.toolset_digest, "toolset digest"),
        (&plan.reference_arm_id, "reference arm ID"),
        (&plan.candidate_arm_id, "candidate arm ID"),
    ] {
        if !valid_id(value) {
            return Err(ClusterInferenceError::InvalidPlan(format!(
                "{label} is empty or overlong"
            )));
        }
    }
    if !valid_digest(&plan.pre_registration_receipt_digest) {
        return Err(ClusterInferenceError::InvalidPlan(
            "cluster inference pre-registration receipt is not a canonical sha256 digest".into(),
        ));
    }
    if plan.reference_arm_id == plan.candidate_arm_id {
        return Err(ClusterInferenceError::InvalidPlan(
            "reference and candidate arm IDs must differ".into(),
        ));
    }
    if plan.family_wise_alpha.is_zero() || !plan.family_wise_alpha.is_probability() {
        return Err(ClusterInferenceError::InvalidPlan(
            "family-wise alpha must be in (0,1]".into(),
        ));
    }
    if plan.mechanism_families.is_empty() || plan.mechanism_families.len() > MAX_FAMILIES {
        return Err(ClusterInferenceError::InvalidPlan(format!(
            "family registry must contain 1..={MAX_FAMILIES} entries"
        )));
    }
    let mut families = BTreeSet::new();
    for family in &plan.mechanism_families {
        if !valid_id(family) || !families.insert(family.clone()) {
            return Err(ClusterInferenceError::FamilyRegistry(
                "preregistered family set has invalid or duplicate identities".into(),
            ));
        }
    }
    if plan.endpoints.is_empty() || plan.endpoints.len() > MAX_ENDPOINTS {
        return Err(ClusterInferenceError::InvalidPlan(format!(
            "endpoint set must contain 1..={MAX_ENDPOINTS} supported endpoints"
        )));
    }
    let mut endpoints = BTreeSet::new();
    for endpoint in &plan.endpoints {
        if !endpoints.insert(*endpoint) {
            return Err(ClusterInferenceError::EndpointRegistry(
                "preregistered endpoint set contains duplicates".into(),
            ));
        }
    }

    if plan.campaign_id != source_plan.campaign_id
        || plan.scope != source_plan.scope
        || plan.group_id != source_plan.group_id
        || plan.agent_panel_id != source_plan.agent_panel_id
        || plan.task_manifest_digest != source_plan.task_manifest_digest
        || plan.model_configuration_digest != source_plan.model_configuration_digest
        || plan.prompt_configuration_digest != source_plan.prompt_configuration_digest
        || plan.harness_digest != source_plan.harness_digest
        || plan.toolset_digest != source_plan.toolset_digest
        || plan.reference_arm_id != source_plan.reference_arm_id
        || plan.candidate_arm_id != source_plan.candidate_arm_id
        || plan.resource_budget != source_plan.resource_budget
        || plan.repetitions_per_task != source_plan.repetitions_per_task
        || plan.minimum_valid_task_clusters_per_family
            != source_plan.minimum_valid_task_clusters_per_family
        || plan.source_reliability_analysis_id != source_plan.analysis_id
        || plan.mechanism_families != source_plan.mechanism_families
    {
        return Err(ClusterInferenceError::ManifestMismatch(
            "cluster inference plan does not match the preregistered repeated-run reliability plan"
                .into(),
        ));
    }
    if plan.campaign_id != manifest.campaign_id
        || plan.scope != manifest.scope
        || manifest.split != EvidenceSplit::SealedHoldout
        || plan.group_id != manifest.group_id
        || plan.agent_panel_id != manifest.agent_panel_id
        || plan.task_manifest_digest != manifest.task_manifest_digest
        || plan.model_configuration_digest != manifest.model_configuration_digest
        || plan.prompt_configuration_digest != manifest.prompt_configuration_digest
        || plan.harness_digest != manifest.harness_digest
        || plan.toolset_digest != manifest.toolset_digest
        || plan.reference_arm_id != manifest.reference_arm_id
        || plan.candidate_arm_id != manifest.candidate_arm_id
        || plan.resource_budget != manifest.resource_budget
        || source_plan.split != EvidenceSplit::SealedHoldout
    {
        return Err(ClusterInferenceError::ManifestMismatch(
            "cluster inference plan does not match the sealed matched-benchmark manifest".into(),
        ));
    }
    if source_receipt.split != EvidenceSplit::SealedHoldout {
        return Err(ClusterInferenceError::NonSealedEvidence(
            "cluster inference accepts SealedHoldout repeated-run receipts only".into(),
        ));
    }
    if source_receipt.analysis_id != source_plan.analysis_id
        || source_receipt.analysis_version != source_plan.analysis_version
        || source_receipt.campaign_id != source_plan.campaign_id
        || source_receipt.scope != source_plan.scope
        || source_receipt.group_id != source_plan.group_id
        || source_receipt.reference_arm_id != source_plan.reference_arm_id
        || source_receipt.candidate_arm_id != source_plan.candidate_arm_id
        || source_receipt.repetitions_per_task != source_plan.repetitions_per_task
        || source_receipt.sealed_holdout_only == false
        || source_receipt.pass_at_k_is_not_pass_every_k == false
        || source_receipt.repeated_runs_not_counted_as_independent_task_samples == false
        || source_receipt.exact_manifest_and_resource_matching == false
    {
        return Err(ClusterInferenceError::ClusterReceipt(
            "source repeated-run receipt does not match its registered plan or assurance flags"
                .into(),
        ));
    }
    Ok(())
}

fn validate_arm_cluster(
    arm: &super::repeated_run_reliability::ArmTaskReliability,
    repetitions: usize,
) -> Result<(), ClusterInferenceError> {
    if arm.safe_success_count > repetitions
        || arm.completion_count > repetitions
        || arm.hazard_count > repetitions
        || arm.abstention_count > repetitions
        || arm.safe_success_count > arm.completion_count
        || arm.pass_at_k != (arm.safe_success_count > 0)
        || arm.pass_every_k != (arm.safe_success_count == repetitions)
        || arm.completed_at_least_once != (arm.completion_count > 0)
        || arm.completed_every_run != (arm.completion_count == repetitions)
        || arm.any_hazard != (arm.hazard_count > 0)
        || arm.any_abstention != (arm.abstention_count > 0)
    {
        return Err(ClusterInferenceError::InvalidCluster(
            "arm task-cluster counts and derived reliability flags are inconsistent".into(),
        ));
    }
    Ok(())
}

fn cluster_effect(
    endpoint: ClusterInferenceEndpoint,
    cluster: &TaskClusterReliability,
) -> Result<i128, ClusterInferenceError> {
    let reference = cluster.reference.as_ref().ok_or_else(|| {
        ClusterInferenceError::InvalidCluster(
            "valid task cluster is missing reference reliability".into(),
        )
    })?;
    let candidate = cluster.candidate.as_ref().ok_or_else(|| {
        ClusterInferenceError::InvalidCluster(
            "valid task cluster is missing candidate reliability".into(),
        )
    })?;
    let bool_value = |value: bool| if value { 1i128 } else { 0i128 };
    Ok(match endpoint {
        ClusterInferenceEndpoint::PassAtK => {
            bool_value(candidate.pass_at_k) - bool_value(reference.pass_at_k)
        }
        ClusterInferenceEndpoint::PassEveryK => {
            bool_value(candidate.pass_every_k) - bool_value(reference.pass_every_k)
        }
        ClusterInferenceEndpoint::MeanRunSafeSuccessRate => {
            i128::try_from(candidate.safe_success_count).map_err(|_| {
                ClusterInferenceError::ArithmeticOverflow(
                    "candidate cluster success count exceeds i128".into(),
                )
            })? - i128::try_from(reference.safe_success_count).map_err(|_| {
                ClusterInferenceError::ArithmeticOverflow(
                    "reference cluster success count exceeds i128".into(),
                )
            })?
        }
        ClusterInferenceEndpoint::AnyHazard => {
            // Lower hazard is favorable: no candidate hazard against a reference
            // hazard is a positive candidate effect.
            bool_value(reference.any_hazard) - bool_value(candidate.any_hazard)
        }
    })
}

fn floor_div_signed(numerator: i128, denominator: i128) -> i128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder < 0 {
        quotient - 1
    } else {
        quotient
    }
}

fn ceil_div_signed(numerator: i128, denominator: i128) -> i128 {
    -floor_div_signed(-numerator, denominator)
}

fn ceil_div_u128(numerator: u128, denominator: u128) -> u128 {
    numerator / denominator + u128::from(!numerator.is_multiple_of(denominator))
}

fn integer_sqrt_ceil(value: u128) -> Result<u128, ClusterInferenceError> {
    let mut low = 0u128;
    let mut high = 2_000_001u128;
    while low < high {
        let middle = low + (high - low) / 2;
        let square = middle.checked_mul(middle).ok_or_else(|| {
            ClusterInferenceError::ArithmeticOverflow("integer square overflow".into())
        })?;
        if square >= value {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    Ok(low)
}

fn hoeffding_interval(
    sum_normalized_numerator: i128,
    valid_task_clusters: usize,
    per_cluster_scale: usize,
) -> Result<HoeffdingClusterMeanInterval, ClusterInferenceError> {
    let cluster_count = i128::try_from(valid_task_clusters).map_err(|_| {
        ClusterInferenceError::ArithmeticOverflow("cluster count exceeds i128".into())
    })?;
    let scale = i128::try_from(per_cluster_scale).map_err(|_| {
        ClusterInferenceError::ArithmeticOverflow("per-cluster scale exceeds i128".into())
    })?;
    let denominator = cluster_count.checked_mul(scale).ok_or_else(|| {
        ClusterInferenceError::ArithmeticOverflow("mean-effect denominator overflow".into())
    })?;
    if denominator <= 0 {
        return Err(ClusterInferenceError::InvalidCluster(
            "mean interval requires at least one valid task cluster".into(),
        ));
    }
    let scaled_sum = sum_normalized_numerator
        .checked_mul(MICRO_SCALE)
        .ok_or_else(|| {
            ClusterInferenceError::ArithmeticOverflow(
                "mean-effect micro-unit scaling overflow".into(),
            )
        })?;
    let centre_lower = floor_div_signed(scaled_sum, denominator);
    let centre_upper = ceil_div_signed(scaled_sum, denominator);
    let n = u128::try_from(valid_task_clusters).map_err(|_| {
        ClusterInferenceError::ArithmeticOverflow("cluster count exceeds u128".into())
    })?;
    // For every endpoint, one paired cluster difference lies in [-1,1].
    // ln(40) < 4 gives the conservative 95% Hoeffding radius 2*sqrt(2/n).
    let radius_square = ceil_div_u128(SQRT_RADIUS_NUMERATOR, n);
    let radius_ppm = integer_sqrt_ceil(radius_square)?;
    let radius = i128::try_from(radius_ppm)
        .ok()
        .and_then(|value| value.checked_mul(2))
        .ok_or_else(|| {
            ClusterInferenceError::ArithmeticOverflow("Hoeffding interval radius overflow".into())
        })?;
    Ok(HoeffdingClusterMeanInterval {
        lower_micro_units: centre_lower.checked_sub(radius).ok_or_else(|| {
            ClusterInferenceError::ArithmeticOverflow("Hoeffding lower bound overflow".into())
        })?,
        upper_micro_units: centre_upper.checked_add(radius).ok_or_else(|| {
            ClusterInferenceError::ArithmeticOverflow("Hoeffding upper bound overflow".into())
        })?,
        scale: 1_000_000,
        confidence_level_percent: 95,
        method: "pointwise conservative Hoeffding over independent task clusters; bounded paired difference [-1,1]; outward micro-unit rounding".into(),
    })
}

fn binomial_coefficient(n: usize, k: usize) -> Result<u128, ClusterInferenceError> {
    let k = k.min(n.saturating_sub(k));
    let mut value = 1u128;
    for index in 1..=k {
        let factor = u128::try_from(n - index + 1).map_err(|_| {
            ClusterInferenceError::ArithmeticOverflow("binomial factor exceeds u128".into())
        })?;
        value = value.checked_mul(factor).ok_or_else(|| {
            ClusterInferenceError::ArithmeticOverflow("binomial coefficient overflow".into())
        })? / u128::try_from(index).map_err(|_| {
            ClusterInferenceError::ArithmeticOverflow("binomial divisor exceeds u128".into())
        })?;
    }
    Ok(value)
}

/// Exact two-sided paired sign test, conditional on discordant task clusters.
/// Ties are excluded from the sign denominator but retained in the effect
/// summary. The bounded u128 implementation fails closed above 120 signs.
fn exact_cluster_sign_test_p_value(
    candidate_wins: usize,
    reference_wins: usize,
) -> Result<Rational, ClusterInferenceError> {
    let discordant = candidate_wins.checked_add(reference_wins).ok_or_else(|| {
        ClusterInferenceError::ArithmeticOverflow("discordant task-cluster count overflow".into())
    })?;
    if discordant > MAX_EXACT_CLUSTER_DISCORDANT_PAIRS {
        return Err(ClusterInferenceError::UnsupportedExactSampleSize(format!(
            "{discordant} discordant clusters exceeds exact cap {MAX_EXACT_CLUSTER_DISCORDANT_PAIRS}"
        )));
    }
    if discordant == 0 {
        return Ok(Rational::one());
    }
    let tail_count = candidate_wins.min(reference_wins);
    let mut tail_sum = 0u128;
    for index in 0..=tail_count {
        tail_sum = tail_sum
            .checked_add(binomial_coefficient(discordant, index)?)
            .ok_or_else(|| {
                ClusterInferenceError::ArithmeticOverflow("exact sign-test tail overflow".into())
            })?;
    }
    let denominator = 1u128.checked_shl(discordant as u32).ok_or_else(|| {
        ClusterInferenceError::UnsupportedExactSampleSize(
            "exact sign-test denominator exceeds u128".into(),
        )
    })?;
    let doubled_tail = tail_sum.checked_mul(2).ok_or_else(|| {
        ClusterInferenceError::ArithmeticOverflow("exact sign-test tail doubling overflow".into())
    })?;
    Rational::new(doubled_tail.min(denominator), denominator)
        .map_err(ClusterInferenceError::ArithmeticOverflow)
}

fn exact_run_rate(successes: usize, total: usize) -> Option<super::repeated_run_reliability::ExactRunRate> {
    (total > 0).then_some(super::repeated_run_reliability::ExactRunRate {
        numerator: successes,
        denominator: total,
    })
}

fn checked_count_add(
    value: &mut usize,
    amount: usize,
    label: &str,
) -> Result<(), ClusterInferenceError> {
    *value = value.checked_add(amount).ok_or_else(|| {
        ClusterInferenceError::ArithmeticOverflow(format!("{label} aggregate overflow"))
    })?;
    Ok(())
}

/// Recompute every descriptive family aggregate from the retained task-cluster
/// details. The summary is not trusted merely because it came beside the detail
/// in the same upstream receipt.
fn validate_family_summary(
    summary: &super::repeated_run_reliability::FamilyRepeatedRunSummary,
    clusters: &[&TaskClusterReliability],
    minimum_clusters: usize,
    repetitions_per_task: usize,
) -> Result<FamilyAbstentionDescriptive, ClusterInferenceError> {
    let mut valid_clusters = 0usize;
    let mut invalid_clusters = 0usize;
    let mut candidate_pass_at_k = 0usize;
    let mut reference_pass_at_k = 0usize;
    let mut candidate_pass_every_k = 0usize;
    let mut reference_pass_every_k = 0usize;
    let mut candidate_safe_successes = 0usize;
    let mut reference_safe_successes = 0usize;
    let mut candidate_any_hazard = 0usize;
    let mut reference_any_hazard = 0usize;
    let mut candidate_any_abstention = 0usize;
    let mut reference_any_abstention = 0usize;
    let mut candidate_abstention_runs = 0usize;
    let mut reference_abstention_runs = 0usize;

    for cluster in clusters {
        if cluster.invalid_task_cluster {
            checked_count_add(&mut invalid_clusters, 1, "invalid task cluster")?;
            continue;
        }
        checked_count_add(&mut valid_clusters, 1, "valid task cluster")?;
        let candidate = cluster.candidate.as_ref().ok_or_else(|| {
            ClusterInferenceError::InvalidCluster("valid task cluster lacks candidate metrics".into())
        })?;
        let reference = cluster.reference.as_ref().ok_or_else(|| {
            ClusterInferenceError::InvalidCluster("valid task cluster lacks reference metrics".into())
        })?;
        checked_count_add(&mut candidate_pass_at_k, usize::from(candidate.pass_at_k), "candidate pass@k")?;
        checked_count_add(&mut reference_pass_at_k, usize::from(reference.pass_at_k), "reference pass@k")?;
        checked_count_add(&mut candidate_pass_every_k, usize::from(candidate.pass_every_k), "candidate pass^k")?;
        checked_count_add(&mut reference_pass_every_k, usize::from(reference.pass_every_k), "reference pass^k")?;
        checked_count_add(&mut candidate_safe_successes, candidate.safe_success_count, "candidate safe successes")?;
        checked_count_add(&mut reference_safe_successes, reference.safe_success_count, "reference safe successes")?;
        checked_count_add(&mut candidate_any_hazard, usize::from(candidate.any_hazard), "candidate any-hazard clusters")?;
        checked_count_add(&mut reference_any_hazard, usize::from(reference.any_hazard), "reference any-hazard clusters")?;
        checked_count_add(&mut candidate_any_abstention, usize::from(candidate.any_abstention), "candidate any-abstention clusters")?;
        checked_count_add(&mut reference_any_abstention, usize::from(reference.any_abstention), "reference any-abstention clusters")?;
        checked_count_add(&mut candidate_abstention_runs, candidate.abstention_count, "candidate abstention runs")?;
        checked_count_add(&mut reference_abstention_runs, reference.abstention_count, "reference abstention runs")?;
    }

    let total_valid_run_pairs = valid_clusters.checked_mul(repetitions_per_task).ok_or_else(|| {
        ClusterInferenceError::ArithmeticOverflow("valid run-pair aggregate overflow".into())
    })?;
    let meets_minimum = valid_clusters >= minimum_clusters;
    if summary.valid_task_clusters != valid_clusters
        || summary.invalid_task_clusters_excluded != invalid_clusters
        || summary.required_valid_task_clusters != minimum_clusters
        || summary.meets_minimum_valid_task_clusters != meets_minimum
        || summary.candidate_pass_at_k_tasks != candidate_pass_at_k
        || summary.reference_pass_at_k_tasks != reference_pass_at_k
        || summary.candidate_pass_at_k_rate != exact_run_rate(candidate_pass_at_k, valid_clusters)
        || summary.reference_pass_at_k_rate != exact_run_rate(reference_pass_at_k, valid_clusters)
        || summary.candidate_pass_every_k_tasks != candidate_pass_every_k
        || summary.reference_pass_every_k_tasks != reference_pass_every_k
        || summary.candidate_pass_every_k_rate != exact_run_rate(candidate_pass_every_k, valid_clusters)
        || summary.reference_pass_every_k_rate != exact_run_rate(reference_pass_every_k, valid_clusters)
        || summary.candidate_run_safe_successes != candidate_safe_successes
        || summary.reference_run_safe_successes != reference_safe_successes
        || summary.total_valid_run_pairs != total_valid_run_pairs
        || summary.candidate_run_safe_success_rate != exact_run_rate(candidate_safe_successes, total_valid_run_pairs)
        || summary.reference_run_safe_success_rate != exact_run_rate(reference_safe_successes, total_valid_run_pairs)
        || summary.candidate_any_hazard_tasks != candidate_any_hazard
        || summary.reference_any_hazard_tasks != reference_any_hazard
        || summary.candidate_any_abstention_tasks != candidate_any_abstention
        || summary.reference_any_abstention_tasks != reference_any_abstention
    {
        return Err(ClusterInferenceError::ClusterReceipt(format!(
            "family summary values do not reconstruct from task-cluster detail for {}",
            summary.mechanism_family_id
        )));
    }

    Ok(FamilyAbstentionDescriptive {
        mechanism_family_id: summary.mechanism_family_id.clone(),
        valid_task_clusters: valid_clusters,
        candidate_clusters_with_any_abstention: candidate_any_abstention,
        reference_clusters_with_any_abstention: reference_any_abstention,
        candidate_abstention_runs,
        reference_abstention_runs,
    })
}

fn increment(value: &mut usize, label: &str) -> Result<(), ClusterInferenceError> {
    *value = value.checked_add(1).ok_or_else(|| {
        ClusterInferenceError::ArithmeticOverflow(format!("{label} count overflow"))
    })?;
    Ok(())
}

fn apply_holm(
    results: &mut [FamilyEndpointInference],
    alpha: Rational,
) -> Result<(), ClusterInferenceError> {
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
            .then_with(|| results[*left].endpoint.cmp(&results[*right].endpoint))
    });
    let total = u64::try_from(results.len()).map_err(|_| {
        ClusterInferenceError::ArithmeticOverflow("hypothesis count exceeds u64".into())
    })?;
    let mut still_rejecting = true;
    for (index, result_index) in order.into_iter().enumerate() {
        let remaining = total
            - u64::try_from(index).map_err(|_| {
                ClusterInferenceError::ArithmeticOverflow("Holm rank exceeds u64".into())
            })?;
        let threshold = alpha
            .checked_div(Rational::from_u64(remaining))
            .map_err(ClusterInferenceError::ArithmeticOverflow)?;
        let result = &mut results[result_index];
        result.holm_rank = index + 1;
        result.holm_threshold = threshold;
        let passes =
            result.meets_minimum_cluster_count && result.exact_two_sided_sign_p_value <= threshold;
        result.rejected_after_holm = still_rejecting && passes;
        if !passes {
            still_rejecting = false;
        }
    }
    Ok(())
}

/// Infer over distinct SIM-REL-046 task clusters, never over individual runs.
///
/// For each registered family and endpoint, the paired sign test counts
/// candidate wins, reference wins and ties across distinct task instances.
/// Mean paired effects and pointwise Hoeffding intervals use one observation per
/// task cluster (for MeanRunSafeSuccessRate, the within-task k-run fraction).
/// Holm correction spans every preregistered family × endpoint hypothesis.
pub fn analyze_repeated_run_task_clusters(
    plan: &TaskClusterInferencePlan,
    source_plan: &RepeatedRunReliabilityPlan,
    manifest: &MatchedBenchmarkManifest,
    source_receipt: &RepeatedRunReliabilityReceipt,
) -> Result<TaskClusterInferenceReceipt, ClusterInferenceError> {
    validate_plan(plan, source_plan, manifest, source_receipt)?;

    let family_set = plan
        .mechanism_families
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut family_clusters = plan
        .mechanism_families
        .iter()
        .cloned()
        .map(|family| (family, Vec::<&TaskClusterReliability>::new()))
        .collect::<BTreeMap<_, _>>();
    let mut task_keys = BTreeSet::new();
    let mut total_valid_task_clusters = 0usize;
    let mut total_invalid_task_clusters = 0usize;

    for cluster in &source_receipt.task_clusters {
        if !family_set.contains(&cluster.mechanism_family_id) || !valid_id(&cluster.task_id) {
            return Err(ClusterInferenceError::FamilyRegistry(
                "source cluster contains an unregistered family or invalid task ID".into(),
            ));
        }
        if !task_keys.insert((cluster.mechanism_family_id.clone(), cluster.task_id.clone())) {
            return Err(ClusterInferenceError::DuplicateCluster(format!(
                "duplicate family/task cluster {} / {}",
                cluster.mechanism_family_id, cluster.task_id
            )));
        }
        if cluster.repetition_count != plan.repetitions_per_task {
            return Err(ClusterInferenceError::InvalidCluster(format!(
                "cluster {} in family {} contains {} repetitions; expected {}",
                cluster.task_id,
                cluster.mechanism_family_id,
                cluster.repetition_count,
                plan.repetitions_per_task
            )));
        }
        let family = family_clusters
            .get_mut(&cluster.mechanism_family_id)
            .ok_or_else(|| {
                ClusterInferenceError::FamilyRegistry(
                    "registered family disappeared during analysis".into(),
                )
            })?;
        if cluster.invalid_task_cluster {
            if cluster.reference.is_some() || cluster.candidate.is_some() {
                return Err(ClusterInferenceError::InvalidCluster(
                    "invalid task cluster must not contain arm reliability outcomes".into(),
                ));
            }
            increment(&mut total_invalid_task_clusters, "invalid task cluster")?;
        } else {
            let reference = cluster.reference.as_ref().ok_or_else(|| {
                ClusterInferenceError::InvalidCluster(
                    "valid task cluster lacks reference metrics".into(),
                )
            })?;
            let candidate = cluster.candidate.as_ref().ok_or_else(|| {
                ClusterInferenceError::InvalidCluster(
                    "valid task cluster lacks candidate metrics".into(),
                )
            })?;
            validate_arm_cluster(reference, plan.repetitions_per_task)?;
            validate_arm_cluster(candidate, plan.repetitions_per_task)?;
            increment(&mut total_valid_task_clusters, "valid task cluster")?;
        }
        family.push(cluster);
    }

    if source_receipt.task_cluster_count != source_receipt.task_clusters.len()
        || source_receipt.valid_task_cluster_count != total_valid_task_clusters
        || source_receipt.invalid_task_cluster_count != total_invalid_task_clusters
    {
        return Err(ClusterInferenceError::ClusterReceipt(
            "source repeated-run receipt totals do not match its task-cluster detail".into(),
        ));
    }

    let summaries = source_receipt
        .family_summaries
        .iter()
        .map(|summary| (summary.mechanism_family_id.clone(), summary))
        .collect::<BTreeMap<_, _>>();
    if summaries.len() != plan.mechanism_families.len()
        || plan
            .mechanism_families
            .iter()
            .any(|family| !summaries.contains_key(family))
    {
        return Err(ClusterInferenceError::FamilyRegistry(
            "source family summary set differs from the preregistered family set".into(),
        ));
    }

    let hypotheses = plan
        .mechanism_families
        .len()
        .checked_mul(plan.endpoints.len())
        .ok_or_else(|| {
            ClusterInferenceError::ArithmeticOverflow(
                "family × endpoint hypothesis count overflow".into(),
            )
        })?;
    let mut family_endpoint_results = Vec::with_capacity(hypotheses);
    let mut family_abstention_descriptive = Vec::with_capacity(plan.mechanism_families.len());

    for family_id in &plan.mechanism_families {
        let clusters = family_clusters.get(family_id).ok_or_else(|| {
            ClusterInferenceError::FamilyRegistry("registered family missing cluster vector".into())
        })?;
        let valid_clusters = clusters
            .iter()
            .filter(|cluster| !cluster.invalid_task_cluster)
            .count();
        let invalid_clusters = clusters.len().checked_sub(valid_clusters).ok_or_else(|| {
            ClusterInferenceError::ArithmeticOverflow("invalid cluster count underflow".into())
        })?;
        let summary = summaries.get(family_id).ok_or_else(|| {
            ClusterInferenceError::FamilyRegistry("registered family summary missing".into())
        })?;
        if summary.valid_task_clusters != valid_clusters
            || summary.invalid_task_clusters_excluded != invalid_clusters
        {
            return Err(ClusterInferenceError::ClusterReceipt(format!(
                "family task-cluster totals differ from retained detail for {family_id}"
            )));
        }
        let abstention_descriptive = validate_family_summary(
            summary,
            clusters,
            plan.minimum_valid_task_clusters_per_family,
            plan.repetitions_per_task,
        )?;
        family_abstention_descriptive.push(abstention_descriptive);

        for endpoint in &plan.endpoints {
            let mut counts = EndpointCounts {
                valid_task_clusters: valid_clusters,
                invalid_task_clusters: invalid_clusters,
                ..EndpointCounts::default()
            };
            for cluster in clusters
                .iter()
                .filter(|cluster| !cluster.invalid_task_cluster)
            {
                let difference = cluster_effect(*endpoint, cluster)?;
                counts.effect_sum = counts.effect_sum.checked_add(difference).ok_or_else(|| {
                    ClusterInferenceError::ArithmeticOverflow("cluster effect sum overflow".into())
                })?;
                match difference.cmp(&0) {
                    std::cmp::Ordering::Less => {
                        increment(&mut counts.reference_wins, "reference cluster win")?
                    }
                    std::cmp::Ordering::Equal => increment(&mut counts.ties, "tied cluster")?,
                    std::cmp::Ordering::Greater => {
                        increment(&mut counts.candidate_wins, "candidate cluster win")?
                    }
                }
            }
            let discordant = counts
                .candidate_wins
                .checked_add(counts.reference_wins)
                .ok_or_else(|| {
                    ClusterInferenceError::ArithmeticOverflow(
                        "discordant cluster count overflow".into(),
                    )
                })?;
            if discordant > MAX_EXACT_CLUSTER_DISCORDANT_PAIRS {
                return Err(ClusterInferenceError::UnsupportedExactSampleSize(format!(
                    "family {family_id} endpoint {endpoint:?} has {discordant} discordant clusters; exact paired sign cap is {}",
                    MAX_EXACT_CLUSTER_DISCORDANT_PAIRS
                )));
            }
            let p_value =
                exact_two_sided_sign_test_p_value(counts.candidate_wins, counts.reference_wins)
                    .map_err(|error| {
                        ClusterInferenceError::ArithmeticOverflow(error.to_string())
                    })?;

            let per_cluster_scale = if *endpoint == ClusterInferenceEndpoint::MeanRunSafeSuccessRate
            {
                plan.repetitions_per_task
            } else {
                1
            };
            let effect_denominator =
                valid_clusters
                    .checked_mul(per_cluster_scale)
                    .ok_or_else(|| {
                        ClusterInferenceError::ArithmeticOverflow(
                            "mean effect denominator overflow".into(),
                        )
                    })?;
            let mean_effect = (valid_clusters > 0).then_some(SignedClusterMeanEffect {
                numerator: counts.effect_sum,
                denominator: effect_denominator as u128,
            });
            let interval = if valid_clusters > 0 {
                Some(hoeffding_interval(
                    counts.effect_sum,
                    valid_clusters,
                    per_cluster_scale,
                )?)
            } else {
                None
            };
            family_endpoint_results.push(FamilyEndpointInference {
                mechanism_family_id: family_id.clone(),
                endpoint: *endpoint,
                valid_task_clusters: counts.valid_task_clusters,
                invalid_task_clusters_excluded: counts.invalid_task_clusters,
                required_valid_task_clusters: plan.minimum_valid_task_clusters_per_family,
                meets_minimum_cluster_count: valid_clusters
                    >= plan.minimum_valid_task_clusters_per_family,
                candidate_cluster_wins: counts.candidate_wins,
                reference_cluster_wins: counts.reference_wins,
                tied_clusters: counts.ties,
                discordant_clusters: discordant,
                exact_two_sided_sign_p_value: p_value,
                mean_paired_cluster_effect: mean_effect,
                mean_effect_95_percent_interval: interval,
                holm_rank: 0,
                holm_threshold: Rational::zero(),
                rejected_after_holm: false,
            });
        }
    }

    apply_holm(&mut family_endpoint_results, plan.family_wise_alpha)?;

    Ok(TaskClusterInferenceReceipt {
        analysis_id: plan.analysis_id.clone(),
        analysis_version: plan.analysis_version.clone(),
        source_reliability_analysis_id: plan.source_reliability_analysis_id.clone(),
        pre_registration_receipt_digest: plan.pre_registration_receipt_digest.clone(),
        campaign_id: plan.campaign_id.clone(),
        scope: plan.scope,
        split: manifest.split,
        group_id: plan.group_id.clone(),
        reference_arm_id: plan.reference_arm_id.clone(),
        candidate_arm_id: plan.candidate_arm_id.clone(),
        repetitions_per_task: plan.repetitions_per_task,
        family_count: plan.mechanism_families.len(),
        endpoint_count: plan.endpoints.len(),
        family_endpoint_hypothesis_count: family_endpoint_results.len(),
        total_task_clusters: source_receipt.task_cluster_count,
        total_valid_task_clusters,
        total_invalid_task_clusters_excluded: total_invalid_task_clusters,
        inferential_unit: "one distinct mechanism_family × task_id cluster".into(),
        family_endpoint_results,
        family_abstention_descriptive,
        repeated_runs_not_counted_as_independent_samples: true,
        cluster_level_pairing_preserved: true,
        abstention_is_descriptive_only: true,
        pointwise_intervals_not_simultaneous: true,
        holm_family_wise_error_control_applied: true,
        sealed_holdout_only: true,
    })
}

#[cfg(test)]
mod tests {
    use super::super::counterexample_ledger::EvidenceSplit;
    use super::super::matched_benchmark::{
        BenchmarkScope, MatchedBenchmarkManifest, ResourceBudget,
    };
    use super::super::repeated_run_reliability::{
        ArmTaskReliability, FamilyRepeatedRunSummary, RepeatedRunReliabilityPlan,
        RepeatedRunReliabilityReceipt, TaskClusterReliability,
    };
    use super::{
        ClusterInferenceEndpoint, TaskClusterInferencePlan, analyze_repeated_run_task_clusters,
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

    fn manifest() -> MatchedBenchmarkManifest {
        MatchedBenchmarkManifest {
            campaign_id: "cluster-campaign".into(),
            protocol_version: "v1".into(),
            scope: BenchmarkScope::FullScientificCampaign,
            split: EvidenceSplit::SealedHoldout,
            group_id: "cluster-sealed".into(),
            agent_panel_id: "panel-fixed".into(),
            task_manifest_digest: "task-manifest".into(),
            model_configuration_digest: "model-config".into(),
            prompt_configuration_digest: "prompt-config".into(),
            harness_digest: "harness".into(),
            toolset_digest: "toolset".into(),
            reference_arm_id: "reference".into(),
            candidate_arm_id: "candidate".into(),
            resource_budget: budget(),
            minimum_pairs_per_family: 1,
        }
    }

    fn source_plan(families: Vec<String>) -> RepeatedRunReliabilityPlan {
        RepeatedRunReliabilityPlan {
            analysis_id: "reliability-analysis".into(),
            analysis_version: "v1".into(),
            campaign_id: "cluster-campaign".into(),
            scope: BenchmarkScope::FullScientificCampaign,
            split: EvidenceSplit::SealedHoldout,
            group_id: "cluster-sealed".into(),
            agent_panel_id: "panel-fixed".into(),
            task_manifest_digest: "task-manifest".into(),
            model_configuration_digest: "model-config".into(),
            prompt_configuration_digest: "prompt-config".into(),
            harness_digest: "harness".into(),
            toolset_digest: "toolset".into(),
            reference_arm_id: "reference".into(),
            candidate_arm_id: "candidate".into(),
            resource_budget: budget(),
            repetitions_per_task: 3,
            minimum_valid_task_clusters_per_family: 2,
            mechanism_families: families,
        }
    }

    fn inference_plan(families: Vec<String>) -> TaskClusterInferencePlan {
        TaskClusterInferencePlan {
            analysis_id: "cluster-inference".into(),
            analysis_version: "v1".into(),
            source_reliability_analysis_id: "reliability-analysis".into(),
            pre_registration_receipt_digest: format!("sha256:{}", "a".repeat(64)),
            campaign_id: "cluster-campaign".into(),
            scope: BenchmarkScope::FullScientificCampaign,
            group_id: "cluster-sealed".into(),
            agent_panel_id: "panel-fixed".into(),
            task_manifest_digest: "task-manifest".into(),
            model_configuration_digest: "model-config".into(),
            prompt_configuration_digest: "prompt-config".into(),
            harness_digest: "harness".into(),
            toolset_digest: "toolset".into(),
            reference_arm_id: "reference".into(),
            candidate_arm_id: "candidate".into(),
            resource_budget: budget(),
            repetitions_per_task: 3,
            minimum_valid_task_clusters_per_family: 2,
            family_wise_alpha: q(5, 100),
            mechanism_families: families,
            endpoints: vec![
                ClusterInferenceEndpoint::PassAtK,
                ClusterInferenceEndpoint::PassEveryK,
                ClusterInferenceEndpoint::MeanRunSafeSuccessRate,
                ClusterInferenceEndpoint::AnyHazard,
            ],
        }
    }

    fn arm(
        safe_success_count: usize,
        completion_count: usize,
        hazard_count: usize,
        abstention_count: usize,
    ) -> ArmTaskReliability {
        ArmTaskReliability {
            safe_success_count,
            pass_at_k: safe_success_count > 0,
            pass_every_k: safe_success_count == 3,
            completion_count,
            completed_at_least_once: completion_count > 0,
            completed_every_run: completion_count == 3,
            hazard_count,
            any_hazard: hazard_count > 0,
            abstention_count,
            any_abstention: abstention_count > 0,
        }
    }

    fn cluster(
        family: &str,
        task: &str,
        candidate: ArmTaskReliability,
        reference: ArmTaskReliability,
    ) -> TaskClusterReliability {
        TaskClusterReliability {
            mechanism_family_id: family.into(),
            task_id: task.into(),
            repetition_count: 3,
            invalid_task_cluster: false,
            reference: Some(reference),
            candidate: Some(candidate),
        }
    }

    fn invalid_cluster(family: &str, task: &str) -> TaskClusterReliability {
        TaskClusterReliability {
            mechanism_family_id: family.into(),
            task_id: task.into(),
            repetition_count: 3,
            invalid_task_cluster: true,
            reference: None,
            candidate: None,
        }
    }

    fn clusters() -> Vec<TaskClusterReliability> {
        vec![
            cluster("sensor-drift", "task-1", arm(3, 3, 0, 0), arm(0, 3, 1, 0)),
            cluster("sensor-drift", "task-2", arm(0, 3, 1, 0), arm(3, 3, 0, 0)),
            cluster("sensor-drift", "task-3", arm(1, 3, 0, 0), arm(0, 3, 0, 1)),
            cluster("sensor-drift", "task-4", arm(0, 2, 0, 1), arm(0, 2, 0, 0)),
            cluster("actuator-fault", "task-5", arm(2, 3, 0, 0), arm(1, 3, 0, 0)),
            invalid_cluster("actuator-fault", "task-6"),
        ]
    }

    fn summary(family: &str, valid: usize, invalid: usize) -> FamilyRepeatedRunSummary {
        FamilyRepeatedRunSummary {
            mechanism_family_id: family.into(),
            valid_task_clusters: valid,
            invalid_task_clusters_excluded: invalid,
            required_valid_task_clusters: 2,
            meets_minimum_valid_task_clusters: valid >= 2,
            candidate_pass_at_k_tasks: 0,
            reference_pass_at_k_tasks: 0,
            candidate_pass_at_k_rate: None,
            reference_pass_at_k_rate: None,
            candidate_pass_every_k_tasks: 0,
            reference_pass_every_k_tasks: 0,
            candidate_pass_every_k_rate: None,
            reference_pass_every_k_rate: None,
            candidate_run_safe_successes: 0,
            reference_run_safe_successes: 0,
            total_valid_run_pairs: valid * 3,
            candidate_run_safe_success_rate: None,
            reference_run_safe_success_rate: None,
            candidate_any_hazard_tasks: 0,
            reference_any_hazard_tasks: 0,
            candidate_any_abstention_tasks: 0,
            reference_any_abstention_tasks: 0,
        }
    }

    fn receipt(clusters: Vec<TaskClusterReliability>) -> RepeatedRunReliabilityReceipt {
        let valid = clusters.iter().filter(|c| !c.invalid_task_cluster).count();
        let invalid = clusters.len() - valid;
        RepeatedRunReliabilityReceipt {
            analysis_id: "reliability-analysis".into(),
            analysis_version: "v1".into(),
            campaign_id: "cluster-campaign".into(),
            scope: BenchmarkScope::FullScientificCampaign,
            split: EvidenceSplit::SealedHoldout,
            group_id: "cluster-sealed".into(),
            reference_arm_id: "reference".into(),
            candidate_arm_id: "candidate".into(),
            repetitions_per_task: 3,
            independent_generalization_unit: "mechanism_family × distinct_task_instance".into(),
            task_cluster_count: clusters.len(),
            valid_task_cluster_count: valid,
            invalid_task_cluster_count: invalid,
            family_summaries: vec![
                summary("sensor-drift", 4, 0),
                summary("actuator-fault", 1, 1),
            ],
            task_clusters: clusters,
            pass_at_k_is_not_pass_every_k: true,
            repeated_runs_not_counted_as_independent_task_samples: true,
            exact_manifest_and_resource_matching: true,
            sealed_holdout_only: true,
        }
    }

    fn analyze(
        plan: &TaskClusterInferencePlan,
        source_plan: &RepeatedRunReliabilityPlan,
        receipt: &RepeatedRunReliabilityReceipt,
    ) -> Result<super::TaskClusterInferenceReceipt, super::ClusterInferenceError> {
        analyze_repeated_run_task_clusters(plan, source_plan, &manifest(), receipt)
    }

    #[test]
    fn uses_distinct_task_clusters_not_k_times_task_clusters_as_n() {
        let source = source_plan(vec!["sensor-drift".into(), "actuator-fault".into()]);
        let plan = inference_plan(source.mechanism_families.clone());
        let receipt = receipt(clusters());
        let result = analyze(&plan, &source, &receipt).unwrap();
        assert_eq!(result.total_task_clusters, 6);
        assert_eq!(result.total_valid_task_clusters, 5);
        assert_eq!(result.total_invalid_task_clusters_excluded, 1);
        assert_eq!(
            result.inferential_unit,
            "one distinct mechanism_family × task_id cluster"
        );
        assert_eq!(result.family_endpoint_hypothesis_count, 8);
        assert!(result.repeated_runs_not_counted_as_independent_samples);
        assert!(result.cluster_level_pairing_preserved);
    }

    #[test]
    fn pass_every_k_and_hazard_endpoints_use_preregistered_candidate_direction() {
        let source = source_plan(vec!["sensor-drift".into(), "actuator-fault".into()]);
        let plan = inference_plan(source.mechanism_families.clone());
        let receipt = receipt(clusters());
        let result = analyze(&plan, &source, &receipt).unwrap();
        let pass_every = result
            .family_endpoint_results
            .iter()
            .find(|r| {
                r.mechanism_family_id == "sensor-drift"
                    && r.endpoint == ClusterInferenceEndpoint::PassEveryK
            })
            .unwrap();
        assert_eq!(pass_every.candidate_cluster_wins, 1);
        assert_eq!(pass_every.reference_cluster_wins, 1);
        assert_eq!(pass_every.tied_clusters, 2);

        let hazard = result
            .family_endpoint_results
            .iter()
            .find(|r| {
                r.mechanism_family_id == "sensor-drift"
                    && r.endpoint == ClusterInferenceEndpoint::AnyHazard
            })
            .unwrap();
        assert_eq!(hazard.candidate_cluster_wins, 1);
        assert_eq!(hazard.reference_cluster_wins, 1);
    }

    #[test]
    fn mean_run_rate_effect_is_exact_and_interval_contains_cluster_mean() {
        let source = source_plan(vec!["sensor-drift".into(), "actuator-fault".into()]);
        let plan = inference_plan(source.mechanism_families.clone());
        let receipt = receipt(clusters());
        let result = analyze(&plan, &source, &receipt).unwrap();
        let run_rate = result
            .family_endpoint_results
            .iter()
            .find(|r| {
                r.mechanism_family_id == "sensor-drift"
                    && r.endpoint == ClusterInferenceEndpoint::MeanRunSafeSuccessRate
            })
            .unwrap();
        let mean = run_rate.mean_paired_cluster_effect.as_ref().unwrap();
        let interval = run_rate.mean_effect_95_percent_interval.as_ref().unwrap();
        assert_eq!(mean.denominator, 12);
        assert!(interval.lower_micro_units <= 0);
        assert!(interval.upper_micro_units >= 0);
        assert_eq!(interval.confidence_level_percent, 95);
    }

    #[test]
    fn abstention_is_not_given_a_universal_superiority_direction() {
        let source = source_plan(vec!["sensor-drift".into(), "actuator-fault".into()]);
        let plan = inference_plan(source.mechanism_families.clone());
        assert!(
            plan.endpoints
                .contains(&ClusterInferenceEndpoint::AnyHazard)
        );
        let result = analyze(&plan, &source, &receipt(clusters())).unwrap();
        assert!(result.abstention_is_descriptive_only);
        assert_eq!(result.endpoint_count, 4);
    }

    #[test]
    fn missing_or_underpowered_families_remain_in_holm_family() {
        let source = source_plan(vec!["sensor-drift".into(), "actuator-fault".into()]);
        let plan = inference_plan(source.mechanism_families.clone());
        let result = analyze(&plan, &source, &receipt(clusters())).unwrap();
        let underpowered = result
            .family_endpoint_results
            .iter()
            .filter(|r| r.mechanism_family_id == "actuator-fault")
            .collect::<Vec<_>>();
        assert_eq!(underpowered.len(), 4);
        assert!(underpowered.iter().all(|r| !r.meets_minimum_cluster_count));
        assert_eq!(result.family_endpoint_hypothesis_count, 8);
    }

    #[test]
    fn rejects_duplicate_clusters_k_mismatch_and_split_leakage() {
        let source = source_plan(vec!["sensor-drift".into(), "actuator-fault".into()]);
        let plan = inference_plan(source.mechanism_families.clone());
        let mut duplicate = clusters();
        duplicate.push(duplicate[0].clone());
        assert!(analyze(&plan, &source, &receipt(duplicate)).is_err());

        let mut wrong_k = clusters();
        wrong_k[0].repetition_count = 4;
        assert!(analyze(&plan, &source, &receipt(wrong_k)).is_err());

        let mut leaked = receipt(clusters());
        leaked.split = EvidenceSplit::Development;
        assert!(analyze(&plan, &source, &leaked).is_err());
    }

    #[test]
    fn rejects_task_cluster_or_family_summary_mutation() {
        let source = source_plan(vec!["sensor-drift".into(), "actuator-fault".into()]);
        let plan = inference_plan(source.mechanism_families.clone());
        let mut changed = receipt(clusters());
        changed.family_summaries[0].valid_task_clusters = 99;
        assert!(analyze(&plan, &source, &changed).is_err());

        let mut changed = receipt(clusters());
        changed.task_clusters[0]
            .candidate
            .as_mut()
            .unwrap()
            .safe_success_count = 4;
        assert!(analyze(&plan, &source, &changed).is_err());
    }

    #[test]
    fn rejects_endpoint_set_mutation_and_manifest_mismatch() {
        let source = source_plan(vec!["sensor-drift".into(), "actuator-fault".into()]);
        let mut plan = inference_plan(source.mechanism_families.clone());
        plan.endpoints.push(ClusterInferenceEndpoint::PassAtK);
        assert!(analyze(&plan, &source, &receipt(clusters())).is_err());

        plan = inference_plan(source.mechanism_families.clone());
        plan.harness_digest = "other-harness".into();
        assert!(analyze(&plan, &source, &receipt(clusters())).is_err());
    }
}
