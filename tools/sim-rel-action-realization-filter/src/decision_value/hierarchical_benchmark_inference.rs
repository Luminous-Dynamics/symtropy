//! Dependence-aware inference over the scientific benchmark hierarchy:
//! domain -> mechanism family -> concept -> recipe template -> task cluster
//! -> repeated run.
//!
//! Runs are collapsed by SIM-REL-046, task effects are inherited from the
//! qualified SIM-REL-048 cluster adapter, task effects are averaged equally
//! within recipe templates, recipe effects are averaged equally within
//! scientific concepts, and inference uses one concept effect per
//! domain × mechanism-family × endpoint stratum.
//!
//! This corrects known recipe/concept pseudo-replication. It does not prove
//! concepts are IID or representative of all scientific domains.

use super::adaptive_benchmark_lifecycle::{
    AdaptiveBenchmarkFreezeReceipt, AdaptiveBenchmarkPlan, AdaptiveBenchmarkSealedRelease,
    AdaptiveBenchmarkLifecycleReceipt, BenchmarkSemanticOverlapAuditor,
    CalibrationQualificationReceipt, DevelopmentAdaptationEvent,
    verify_adaptive_benchmark_release,
};
use super::counterexample_ledger::EvidenceSplit;
use super::matched_benchmark::{BenchmarkScope, MatchedBenchmarkManifest};
use super::repeated_run_reliability::{
    RepeatedRunReliabilityPlan, RepeatedRunReliabilityReceipt, TaskClusterReliability,
};
use super::scientific_challenge::ChallengeSplitRegistry;
use super::task_cluster_inference::{
    ClusterInferenceEndpoint, TaskClusterInferencePlan, TaskClusterInferenceReceipt,
    analyze_repeated_run_task_clusters,
};
use crate::Rational;
use std::collections::{BTreeMap, BTreeSet};

const MAX_ID_BYTES: usize = 256;
const MAX_DOMAINS: usize = 64;
const MAX_HIERARCHY_ROWS: usize = 100_000;
const MAX_EXACT_DISCORDANT_CONCEPTS: usize = 120;
const MICRO_SCALE: i128 = 1_000_000;
const SQRT_RADIUS_NUMERATOR: u128 = 2_000_000_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HierarchicalInferencePlan {
    pub analysis_id: String,
    pub analysis_version: String,
    pub source_task_cluster_analysis_id: String,
    pub source_reliability_analysis_id: String,
    pub pre_registration_receipt_digest: String,
    pub campaign_id: String,
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub sealed_release_id: String,
    pub sealed_release_digest: String,
    pub scope: BenchmarkScope,
    pub group_id: String,
    pub minimum_valid_concepts_per_domain_family: usize,
    pub family_wise_alpha: Rational,
    /// Complete domain set is fixed before the sealed release is opened.
    /// Domains with zero tasks remain in the hypothesis family.
    pub domains: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactHierarchicalEffect {
    /// Reduced exact rational; denominator is positive and non-zero.
    pub numerator: i128,
    pub denominator: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HierarchicalRecipeEffect {
    pub domain_id: String,
    pub mechanism_family_id: String,
    pub concept_digest: String,
    pub recipe_template_digest: String,
    pub task_cluster_count: usize,
    pub valid_task_cluster_count: usize,
    pub invalid_task_cluster_count: usize,
    /// Unweighted mean of valid task-cluster effects in this recipe.
    /// None means all task clusters were invalid.
    pub mean_task_effect: Option<ExactHierarchicalEffect>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HierarchicalConceptEffect {
    pub domain_id: String,
    pub mechanism_family_id: String,
    pub concept_digest: String,
    pub task_cluster_count: usize,
    pub valid_task_cluster_count: usize,
    pub invalid_task_cluster_count: usize,
    pub recipe_template_count: usize,
    pub valid_recipe_template_count: usize,
    pub invalid_recipe_template_count: usize,
    /// Unweighted mean of valid recipe-template effects. A concept with no
    /// valid recipe remains visible with None and does not enter inference.
    pub mean_recipe_effect: Option<ExactHierarchicalEffect>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HierarchicalMeanEffectInterval {
    pub lower_micro_units: i128,
    pub upper_micro_units: i128,
    pub scale: u128,
    pub confidence_level_percent: u8,
    pub method: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainFamilyEndpointInference {
    pub domain_id: String,
    pub mechanism_family_id: String,
    pub endpoint: ClusterInferenceEndpoint,
    pub total_concept_clusters: usize,
    pub valid_concept_clusters: usize,
    pub concepts_without_valid_recipes: usize,
    pub required_valid_concept_clusters: usize,
    pub meets_minimum_concept_count: bool,
    pub candidate_concept_wins: usize,
    pub reference_concept_wins: usize,
    pub tied_concepts: usize,
    pub discordant_concepts: usize,
    pub exact_two_sided_sign_p_value: Rational,
    pub mean_concept_effect: Option<ExactHierarchicalEffect>,
    pub mean_effect_95_percent_interval: Option<HierarchicalMeanEffectInterval>,
    pub holm_rank: usize,
    pub holm_threshold: Rational,
    pub rejected_after_holm: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HierarchicalInferenceReceipt {
    pub analysis_id: String,
    pub analysis_version: String,
    pub source_task_cluster_analysis_id: String,
    pub source_reliability_analysis_id: String,
    pub pre_registration_receipt_digest: String,
    pub campaign_id: String,
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub sealed_release_id: String,
    pub sealed_release_digest: String,
    pub scope: BenchmarkScope,
    pub split: EvidenceSplit,
    pub group_id: String,
    pub domain_count: usize,
    pub mechanism_family_count: usize,
    pub endpoint_count: usize,
    pub hypothesis_count: usize,
    pub task_cluster_count: usize,
    pub valid_task_cluster_count: usize,
    pub invalid_task_cluster_count: usize,
    pub recipe_template_count: usize,
    pub concept_cluster_count: usize,
    pub valid_concept_cluster_count: usize,
    pub inferential_unit: String,
    pub hierarchy: Vec<HierarchicalRecipeEffect>,
    pub concept_effects: Vec<HierarchicalConceptEffect>,
    pub domain_family_endpoint_results: Vec<DomainFamilyEndpointInference>,
    pub runs_collapsed_within_task: bool,
    pub tasks_equal_weighted_within_recipe: bool,
    pub recipes_equal_weighted_within_concept: bool,
    pub one_concept_one_effect_per_hypothesis: bool,
    pub domains_reported_as_fixed_strata: bool,
    pub pointwise_intervals_not_simultaneous: bool,
    pub abstention_not_given_unregistered_direction: bool,
    pub sealed_holdout_only: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HierarchicalInferenceError {
    InvalidPlan(String),
    InvalidIdentity(String),
    InvalidDigest(String),
    SourcePlanMismatch(String),
    SourceReceiptMismatch(String),
    ReleaseMismatch(String),
    NonSealedEvidence(String),
    DomainRegistry(String),
    FamilyRegistry(String),
    DuplicateTask(String),
    MissingTaskArtifact(String),
    OrphanTaskArtifact(String),
    TaskBindingMismatch(String),
    InvalidHierarchy(String),
    UnsupportedExactSampleSize(String),
    ArithmeticOverflow(String),
}

impl std::fmt::Display for HierarchicalInferenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlan(reason)
            | Self::InvalidIdentity(reason)
            | Self::InvalidDigest(reason)
            | Self::SourcePlanMismatch(reason)
            | Self::SourceReceiptMismatch(reason)
            | Self::ReleaseMismatch(reason)
            | Self::NonSealedEvidence(reason)
            | Self::DomainRegistry(reason)
            | Self::FamilyRegistry(reason)
            | Self::DuplicateTask(reason)
            | Self::MissingTaskArtifact(reason)
            | Self::OrphanTaskArtifact(reason)
            | Self::TaskBindingMismatch(reason)
            | Self::InvalidHierarchy(reason)
            | Self::UnsupportedExactSampleSize(reason)
            | Self::ArithmeticOverflow(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for HierarchicalInferenceError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TaskEffectRow {
    domain_id: String,
    mechanism_family_id: String,
    concept_digest: String,
    recipe_template_digest: String,
    task_id: String,
}

#[derive(Clone, Debug, Default)]
struct ConceptCounts {
    total_concepts: usize,
    valid_concepts: usize,
    concepts_without_valid_recipes: usize,
    candidate_wins: usize,
    reference_wins: usize,
    ties: usize,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn require_id(value: &str, label: &str) -> Result<(), HierarchicalInferenceError> {
    if !valid_id(value) {
        return Err(HierarchicalInferenceError::InvalidIdentity(format!(
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
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_digest(value: &str, label: &str) -> Result<(), HierarchicalInferenceError> {
    if !valid_digest(value) {
        return Err(HierarchicalInferenceError::InvalidDigest(format!(
            "{label} is not a canonical sha256 digest"
        )));
    }
    Ok(())
}

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

impl ExactHierarchicalEffect {
    fn new(numerator: i128, denominator: u128) -> Result<Self, HierarchicalInferenceError> {
        if denominator == 0 {
            return Err(HierarchicalInferenceError::InvalidHierarchy(
                "exact effect denominator must be positive".into(),
            ));
        }
        let divisor = gcd_u128(numerator.unsigned_abs(), denominator);
        Ok(Self {
            numerator: numerator / i128::try_from(divisor).map_err(|_| {
                HierarchicalInferenceError::ArithmeticOverflow(
                    "effect reduction divisor exceeds i128".into(),
                )
            })?,
            denominator: denominator / divisor,
        })
    }

    fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    fn checked_add(
        &self,
        other: &Self,
    ) -> Result<Self, HierarchicalInferenceError> {
        let common = gcd_u128(self.denominator, other.denominator);
        let left_factor = other.denominator / common;
        let right_factor = self.denominator / common;
        let left_factor_i128 = i128::try_from(left_factor).map_err(|_| {
            HierarchicalInferenceError::ArithmeticOverflow(
                "effect denominator multiplier exceeds i128".into(),
            )
        })?;
        let right_factor_i128 = i128::try_from(right_factor).map_err(|_| {
            HierarchicalInferenceError::ArithmeticOverflow(
                "effect denominator multiplier exceeds i128".into(),
            )
        })?;
        let left = self.numerator.checked_mul(left_factor_i128).ok_or_else(|| {
            HierarchicalInferenceError::ArithmeticOverflow("effect numerator overflow".into())
        })?;
        let right = other.numerator.checked_mul(right_factor_i128).ok_or_else(|| {
            HierarchicalInferenceError::ArithmeticOverflow("effect numerator overflow".into())
        })?;
        let numerator = left.checked_add(right).ok_or_else(|| {
            HierarchicalInferenceError::ArithmeticOverflow("effect sum overflow".into())
        })?;
        let denominator = self.denominator.checked_mul(left_factor).ok_or_else(|| {
            HierarchicalInferenceError::ArithmeticOverflow("effect denominator overflow".into())
        })?;
        Self::new(numerator, denominator)
    }

    fn checked_mean(
        values: &[Self],
    ) -> Result<Option<Self>, HierarchicalInferenceError> {
        if values.is_empty() {
            return Ok(None);
        }
        let mut sum = Self::zero();
        for value in values {
            sum = sum.checked_add(value)?;
        }
        let count = u128::try_from(values.len()).map_err(|_| {
            HierarchicalInferenceError::ArithmeticOverflow("effect count exceeds u128".into())
        })?;
        let denominator = sum.denominator.checked_mul(count).ok_or_else(|| {
            HierarchicalInferenceError::ArithmeticOverflow("effect mean denominator overflow".into())
        })?;
        Ok(Some(Self::new(sum.numerator, denominator)?))
    }

    fn sign(&self) -> std::cmp::Ordering {
        self.numerator.cmp(&0)
    }
}

fn bool_value(value: bool) -> i128 {
    if value { 1 } else { 0 }
}

fn task_effect(
    endpoint: ClusterInferenceEndpoint,
    cluster: &TaskClusterReliability,
    repetitions: usize,
) -> Result<ExactHierarchicalEffect, HierarchicalInferenceError> {
    let reference = cluster.reference.as_ref().ok_or_else(|| {
        HierarchicalInferenceError::InvalidHierarchy(
            "valid task cluster lacks reference-arm reliability".into(),
        )
    })?;
    let candidate = cluster.candidate.as_ref().ok_or_else(|| {
        HierarchicalInferenceError::InvalidHierarchy(
            "valid task cluster lacks candidate-arm reliability".into(),
        )
    })?;
    let numerator = match endpoint {
        ClusterInferenceEndpoint::PassAtK => {
            bool_value(candidate.pass_at_k) - bool_value(reference.pass_at_k)
        }
        ClusterInferenceEndpoint::PassEveryK => {
            bool_value(candidate.pass_every_k) - bool_value(reference.pass_every_k)
        }
        ClusterInferenceEndpoint::MeanRunSafeSuccessRate => {
            i128::try_from(candidate.safe_success_count).map_err(|_| {
                HierarchicalInferenceError::ArithmeticOverflow(
                    "candidate safe success count exceeds i128".into(),
                )
            })? - i128::try_from(reference.safe_success_count).map_err(|_| {
                HierarchicalInferenceError::ArithmeticOverflow(
                    "reference safe success count exceeds i128".into(),
                )
            })?
        }
        ClusterInferenceEndpoint::AnyHazard => {
            // Fewer any-hazard task clusters favors the candidate.
            bool_value(reference.any_hazard) - bool_value(candidate.any_hazard)
        }
    };
    let denominator = if endpoint == ClusterInferenceEndpoint::MeanRunSafeSuccessRate {
        u128::try_from(repetitions).map_err(|_| {
            HierarchicalInferenceError::ArithmeticOverflow(
                "repetition count exceeds u128".into(),
            )
        })?
    } else {
        1
    };
    ExactHierarchicalEffect::new(numerator, denominator)
}

fn floor_div_signed(numerator: i128, denominator: i128) -> i128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder < 0 { quotient - 1 } else { quotient }
}

fn ceil_div_signed(numerator: i128, denominator: i128) -> i128 {
    -floor_div_signed(-numerator, denominator)
}

fn ceil_div_u128(numerator: u128, denominator: u128) -> u128 {
    numerator / denominator + u128::from(!numerator.is_multiple_of(denominator))
}

fn integer_sqrt_ceil(value: u128) -> Result<u128, HierarchicalInferenceError> {
    let mut low = 0u128;
    let mut high = 2_000_001u128;
    while low < high {
        let middle = low + (high - low) / 2;
        let square = middle.checked_mul(middle).ok_or_else(|| {
            HierarchicalInferenceError::ArithmeticOverflow("square-root search overflow".into())
        })?;
        if square >= value {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    Ok(low)
}

fn mean_interval(
    mean: &ExactHierarchicalEffect,
    sample_count: usize,
) -> Result<HierarchicalMeanEffectInterval, HierarchicalInferenceError> {
    let denominator = i128::try_from(mean.denominator).map_err(|_| {
        HierarchicalInferenceError::ArithmeticOverflow(
            "mean effect denominator exceeds i128".into(),
        )
    })?;
    let scaled = mean.numerator.checked_mul(MICRO_SCALE).ok_or_else(|| {
        HierarchicalInferenceError::ArithmeticOverflow("mean effect micro-scaling overflow".into())
    })?;
    let centre_lower = floor_div_signed(scaled, denominator);
    let centre_upper = ceil_div_signed(scaled, denominator);
    let n = u128::try_from(sample_count).map_err(|_| {
        HierarchicalInferenceError::ArithmeticOverflow("concept count exceeds u128".into())
    })?;
    if n == 0 {
        return Err(HierarchicalInferenceError::InvalidHierarchy(
            "mean interval requires at least one valid concept".into(),
        ));
    }
    // Paired concept effects lie in [-1,1]. This conservative radius is
    // >= sqrt(2 ln(40) / n), with outward integer rounding.
    let radius_square = ceil_div_u128(SQRT_RADIUS_NUMERATOR, n);
    let radius = integer_sqrt_ceil(radius_square)?
        .checked_mul(2)
        .ok_or_else(|| {
            HierarchicalInferenceError::ArithmeticOverflow("Hoeffding radius overflow".into())
        })?;
    let radius = i128::try_from(radius).map_err(|_| {
        HierarchicalInferenceError::ArithmeticOverflow("Hoeffding radius exceeds i128".into())
    })?;
    Ok(HierarchicalMeanEffectInterval {
        lower_micro_units: centre_lower.checked_sub(radius).ok_or_else(|| {
            HierarchicalInferenceError::ArithmeticOverflow("Hoeffding lower bound overflow".into())
        })?,
        upper_micro_units: centre_upper.checked_add(radius).ok_or_else(|| {
            HierarchicalInferenceError::ArithmeticOverflow("Hoeffding upper bound overflow".into())
        })?,
        scale: 1_000_000,
        confidence_level_percent: 95,
        method: "pointwise conservative Hoeffding over distinct concept clusters; bounded paired concept effect [-1,1]; outward micro-unit rounding".into(),
    })
}

fn binomial_coefficient(
    n: usize,
    k: usize,
) -> Result<u128, HierarchicalInferenceError> {
    let k = k.min(n.saturating_sub(k));
    let mut value = 1u128;
    for index in 1..=k {
        let factor = u128::try_from(n - index + 1).map_err(|_| {
            HierarchicalInferenceError::ArithmeticOverflow("binomial factor exceeds u128".into())
        })?;
        value = value.checked_mul(factor).ok_or_else(|| {
            HierarchicalInferenceError::ArithmeticOverflow("binomial coefficient overflow".into())
        })? / u128::try_from(index).map_err(|_| {
            HierarchicalInferenceError::ArithmeticOverflow("binomial divisor exceeds u128".into())
        })?;
    }
    Ok(value)
}

fn exact_sign_p_value(
    candidate_wins: usize,
    reference_wins: usize,
) -> Result<Rational, HierarchicalInferenceError> {
    let discordant = candidate_wins.checked_add(reference_wins).ok_or_else(|| {
        HierarchicalInferenceError::ArithmeticOverflow("discordant concept count overflow".into())
    })?;
    if discordant > MAX_EXACT_DISCORDANT_CONCEPTS {
        return Err(HierarchicalInferenceError::UnsupportedExactSampleSize(format!(
            "{discordant} discordant concepts exceeds exact sign-test cap {MAX_EXACT_DISCORDANT_CONCEPTS}"
        )));
    }
    if discordant == 0 {
        return Ok(Rational::one());
    }
    let tail_count = candidate_wins.min(reference_wins);
    let mut tail = 0u128;
    for index in 0..=tail_count {
        tail = tail.checked_add(binomial_coefficient(discordant, index)?).ok_or_else(|| {
            HierarchicalInferenceError::ArithmeticOverflow("sign-test tail overflow".into())
        })?;
    }
    let denominator = 1u128.checked_shl(discordant as u32).ok_or_else(|| {
        HierarchicalInferenceError::UnsupportedExactSampleSize(
            "sign-test denominator exceeds u128".into(),
        )
    })?;
    let doubled = tail.checked_mul(2).ok_or_else(|| {
        HierarchicalInferenceError::ArithmeticOverflow("sign-test tail overflow".into())
    })?;
    Rational::new(doubled.min(denominator), denominator)
        .map_err(HierarchicalInferenceError::ArithmeticOverflow)
}

fn apply_holm(
    results: &mut [DomainFamilyEndpointInference],
    alpha: Rational,
) -> Result<(), HierarchicalInferenceError> {
    let mut order = (0..results.len()).collect::<Vec<_>>();
    order.sort_by(|left, right| {
        results[*left]
            .exact_two_sided_sign_p_value
            .cmp(&results[*right].exact_two_sided_sign_p_value)
            .then_with(|| results[*left].domain_id.cmp(&results[*right].domain_id))
            .then_with(|| results[*left].mechanism_family_id.cmp(&results[*right].mechanism_family_id))
            .then_with(|| results[*left].endpoint.cmp(&results[*right].endpoint))
    });
    let total = u64::try_from(results.len()).map_err(|_| {
        HierarchicalInferenceError::ArithmeticOverflow("hypothesis count exceeds u64".into())
    })?;
    let mut still_rejecting = true;
    for (rank, result_index) in order.into_iter().enumerate() {
        let remaining = total.checked_sub(u64::try_from(rank).map_err(|_| {
            HierarchicalInferenceError::ArithmeticOverflow("Holm rank exceeds u64".into())
        })?).ok_or_else(|| {
            HierarchicalInferenceError::ArithmeticOverflow("Holm remaining count underflow".into())
        })?;
        let threshold = alpha
            .checked_div(Rational::from_u64(remaining))
            .map_err(HierarchicalInferenceError::ArithmeticOverflow)?;
        let result = &mut results[result_index];
        result.holm_rank = rank + 1;
        result.holm_threshold = threshold;
        let passes = result.meets_minimum_concept_count
            && result.exact_two_sided_sign_p_value <= threshold;
        result.rejected_after_holm = still_rejecting && passes;
        if !passes {
            still_rejecting = false;
        }
    }
    Ok(())
}

fn validate_plan(
    plan: &HierarchicalInferencePlan,
    cluster_plan: &TaskClusterInferencePlan,
    reliability_plan: &RepeatedRunReliabilityPlan,
    adaptive_plan: &AdaptiveBenchmarkPlan,
    manifest: &MatchedBenchmarkManifest,
    release: &AdaptiveBenchmarkSealedRelease,
) -> Result<(), HierarchicalInferenceError> {
    for (value, label) in [
        (&plan.analysis_id, "hierarchical analysis ID"),
        (&plan.analysis_version, "hierarchical analysis version"),
        (&plan.source_task_cluster_analysis_id, "source task-cluster analysis ID"),
        (&plan.source_reliability_analysis_id, "source reliability analysis ID"),
        (&plan.campaign_id, "hierarchical campaign ID"),
        (&plan.benchmark_id, "benchmark ID"),
        (&plan.benchmark_version, "benchmark version"),
        (&plan.sealed_release_id, "sealed release ID"),
        (&plan.group_id, "sealed evidence group"),
    ] {
        require_id(value, label)?;
    }
    require_digest(&plan.pre_registration_receipt_digest, "hierarchical preregistration")?;
    require_digest(&plan.sealed_release_digest, "sealed release digest")?;
    if plan.family_wise_alpha.is_zero() || !plan.family_wise_alpha.is_probability()
        || plan.minimum_valid_concepts_per_domain_family == 0
    {
        return Err(HierarchicalInferenceError::InvalidPlan(
            "alpha must be in (0,1] and minimum valid concepts must be positive".into(),
        ));
    }
    if plan.domains.is_empty() || plan.domains.len() > MAX_DOMAINS {
        return Err(HierarchicalInferenceError::DomainRegistry(format!(
            "domain registry must contain 1..={MAX_DOMAINS} strata"
        )));
    }
    let mut domains = BTreeSet::new();
    for domain in &plan.domains {
        require_id(domain, "registered domain ID")?;
        if !domains.insert(domain.clone()) {
            return Err(HierarchicalInferenceError::DomainRegistry(
                "registered domain IDs must be unique".into(),
            ));
        }
    }
    let adaptive_domains = adaptive_plan.allowed_domains.iter().cloned().collect::<BTreeSet<_>>();
    if domains != adaptive_domains {
        return Err(HierarchicalInferenceError::DomainRegistry(
            "hierarchical domain registry must equal the frozen adaptive benchmark domain set, retaining empty strata".into(),
        ));
    }
    if plan.source_task_cluster_analysis_id != cluster_plan.analysis_id
        || plan.source_reliability_analysis_id != reliability_plan.analysis_id
        || plan.pre_registration_receipt_digest != cluster_plan.pre_registration_receipt_digest
        || plan.campaign_id != cluster_plan.campaign_id
        || plan.campaign_id != reliability_plan.campaign_id
        || plan.campaign_id != manifest.campaign_id
        || plan.scope != BenchmarkScope::FullScientificCampaign
        || plan.scope != cluster_plan.scope
        || plan.scope != reliability_plan.scope
        || plan.scope != manifest.scope
        || plan.group_id != cluster_plan.group_id
        || plan.group_id != reliability_plan.group_id
        || plan.group_id != manifest.group_id
        || manifest.split != EvidenceSplit::SealedHoldout
        || cluster_plan.split != EvidenceSplit::SealedHoldout
        || reliability_plan.split != EvidenceSplit::SealedHoldout
        || adaptive_plan.benchmark_id != plan.benchmark_id
        || adaptive_plan.benchmark_version != plan.benchmark_version
        || release.benchmark_id != plan.benchmark_id
        || release.benchmark_version != plan.benchmark_version
        || release.release_id != plan.sealed_release_id
        || release.release_digest != plan.sealed_release_digest
        || release.split != EvidenceSplit::SealedHoldout
        || release.family_group_id != plan.group_id
        || cluster_plan.endpoints.is_empty()
    {
        return Err(HierarchicalInferenceError::SourcePlanMismatch(
            "hierarchical plan, source plans, adaptive benchmark and sealed release do not share exact campaign/scope/group/identity bindings".into(),
        ));
    }
    Ok(())
}

fn build_task_rows(
    plan: &HierarchicalInferencePlan,
    cluster_plan: &TaskClusterInferencePlan,
    reliability_plan: &RepeatedRunReliabilityPlan,
    reliability_receipt: &RepeatedRunReliabilityReceipt,
    release: &AdaptiveBenchmarkSealedRelease,
) -> Result<Vec<TaskEffectRow>, HierarchicalInferenceError> {
    if reliability_receipt.split != EvidenceSplit::SealedHoldout
        || !reliability_receipt.sealed_holdout_only
        || reliability_receipt.task_cluster_count != reliability_receipt.task_clusters.len()
        || reliability_receipt.task_cluster_count != release.task_artifacts.len()
        || reliability_receipt.repetitions_per_task != reliability_plan.repetitions_per_task
    {
        return Err(HierarchicalInferenceError::NonSealedEvidence(
            "source reliability receipt must be sealed-only and map one-to-one to release task artifacts".into(),
        ));
    }
    if release.task_artifacts.len() > MAX_HIERARCHY_ROWS {
        return Err(HierarchicalInferenceError::InvalidPlan(
            "sealed task artifact count exceeds hierarchical inference cap".into(),
        ));
    }
    let domains = plan.domains.iter().cloned().collect::<BTreeSet<_>>();
    let families = cluster_plan.mechanism_families.iter().cloned().collect::<BTreeSet<_>>();
    let mut artifacts = BTreeMap::new();
    for artifact in &release.task_artifacts {
        require_id(&artifact.task_id, "sealed task ID")?;
        require_digest(&artifact.concept_digest, "task concept digest")?;
        require_digest(&artifact.recipe_template_digest, "task recipe template digest")?;
        require_id(&artifact.domain_id, "task domain ID")?;
        if artifact.split != EvidenceSplit::SealedHoldout
            || artifact.family_group_id != plan.group_id
            || !families.contains(&artifact.mechanism_family_id)
            || !domains.contains(&artifact.domain_id)
        {
            return Err(HierarchicalInferenceError::TaskBindingMismatch(format!(
                "task {} does not bind the registered sealed group/domain",
                artifact.task_id
            )));
        }
        if artifacts.insert(artifact.task_id.clone(), artifact).is_some() {
            return Err(HierarchicalInferenceError::DuplicateTask(format!(
                "sealed task ID {} occurs more than once in release",
                artifact.task_id
            )));
        }
    }

    let mut seen_cluster_task_ids = BTreeSet::new();
    let mut rows = Vec::with_capacity(reliability_receipt.task_clusters.len());
    for cluster in &reliability_receipt.task_clusters {
        if !families.contains(&cluster.mechanism_family_id) {
            return Err(HierarchicalInferenceError::FamilyRegistry(format!(
                "task cluster {} uses an unregistered mechanism family",
                cluster.task_id
            )));
        }
        if !seen_cluster_task_ids.insert(cluster.task_id.clone()) {
            return Err(HierarchicalInferenceError::DuplicateTask(format!(
                "task ID {} appears in more than one mechanism-family cluster; one release task must bind exactly one task cluster",
                cluster.task_id
            )));
        }
        let artifact = artifacts.get(&cluster.task_id).ok_or_else(|| {
            HierarchicalInferenceError::MissingTaskArtifact(format!(
                "task cluster {} has no matching sealed release artifact",
                cluster.task_id
            ))
        })?;
        if artifact.mechanism_family_id != cluster.mechanism_family_id {
            return Err(HierarchicalInferenceError::TaskBindingMismatch(format!(
                "task {} release mechanism-family {} differs from reliability cluster {}",
                cluster.task_id, artifact.mechanism_family_id, cluster.mechanism_family_id
            )));
        }
        if cluster.invalid_task_cluster && (cluster.reference.is_some() || cluster.candidate.is_some()) {
            return Err(HierarchicalInferenceError::InvalidHierarchy(
                "invalid task cluster must not contain arm outcomes".into(),
            ));
        }
        rows.push(TaskEffectRow {
            domain_id: artifact.domain_id.clone(),
            mechanism_family_id: cluster.mechanism_family_id.clone(),
            concept_digest: artifact.concept_digest.clone(),
            recipe_template_digest: artifact.recipe_template_digest.clone(),
            task_id: cluster.task_id.clone(),
        });
    }
    if seen_cluster_task_ids.len() != artifacts.len() {
        let orphan_count = artifacts.len().saturating_sub(seen_cluster_task_ids.len());
        return Err(HierarchicalInferenceError::OrphanTaskArtifact(format!(
            "{orphan_count} sealed task artifacts have no source task-cluster result"
        )));
    }
    Ok(rows)
}

fn aggregate_for_endpoint(
    rows: &[TaskEffectRow],
    domains: &[String],
    families: &[String],
    endpoint: ClusterInferenceEndpoint,
    reliability_receipt: &RepeatedRunReliabilityReceipt,
    minimum_concepts: usize,
) -> Result<(
    Vec<HierarchicalRecipeEffect>,
    Vec<HierarchicalConceptEffect>,
    Vec<DomainFamilyEndpointInference>,
), HierarchicalInferenceError> {
    let clusters_by_task = reliability_receipt.task_clusters.iter()
        .map(|cluster| (cluster.task_id.as_str(), cluster))
        .collect::<BTreeMap<_, _>>();
    let mut recipe_task_effects: BTreeMap<
        (String, String, String, String),
        Vec<(String, Option<ExactHierarchicalEffect>)>,
    > = BTreeMap::new();

    for row in rows {
        let cluster = clusters_by_task.get(row.task_id.as_str()).ok_or_else(|| {
            HierarchicalInferenceError::MissingTaskArtifact(
                "task cluster disappeared while aggregating endpoint".into(),
            )
        })?;
        let effect = if cluster.invalid_task_cluster {
            None
        } else {
            Some(task_effect(endpoint, cluster, reliability_receipt.repetitions_per_task)?)
        };
        let key = (
            row.domain_id.clone(),
            row.mechanism_family_id.clone(),
            row.concept_digest.clone(),
            row.recipe_template_digest.clone(),
        );
        recipe_task_effects.entry(key).or_default().push((row.task_id.clone(), effect));
    }

    let mut recipe_details = Vec::with_capacity(recipe_task_effects.len());
    let mut concept_recipes: BTreeMap<
        (String, String, String),
        Vec<HierarchicalRecipeEffect>,
    > = BTreeMap::new();

    for ((domain, family, concept, recipe), mut task_values) in recipe_task_effects {
        task_values.sort_by(|left, right| left.0.cmp(&right.0));
        let task_cluster_count = task_values.len();
        let effects = task_values.iter()
            .filter_map(|(_, effect)| effect.clone())
            .collect::<Vec<_>>();
        let valid_task_cluster_count = effects.len();
        let invalid_task_cluster_count = task_cluster_count.saturating_sub(valid_task_cluster_count);
        let detail = HierarchicalRecipeEffect {
            domain_id: domain.clone(),
            mechanism_family_id: family.clone(),
            concept_digest: concept.clone(),
            recipe_template_digest: recipe,
            task_cluster_count,
            valid_task_cluster_count,
            invalid_task_cluster_count,
            mean_task_effect: ExactHierarchicalEffect::checked_mean(&effects)?,
        };
        concept_recipes.entry((domain, family, concept)).or_default().push(detail.clone());
        recipe_details.push(detail);
    }

    let mut concept_details = Vec::with_capacity(concept_recipes.len());
    let mut concept_effects_by_stratum: BTreeMap<
        (String, String),
        Vec<ExactHierarchicalEffect>,
    > = BTreeMap::new();
    let mut concept_counts_by_stratum: BTreeMap<(String, String), ConceptCounts> = BTreeMap::new();

    for ((domain, family, concept), mut recipes) in concept_recipes {
        recipes.sort_by(|left, right| left.recipe_template_digest.cmp(&right.recipe_template_digest));
        let task_cluster_count = recipes.iter().map(|recipe| recipe.task_cluster_count).sum::<usize>();
        let valid_task_cluster_count = recipes.iter().map(|recipe| recipe.valid_task_cluster_count).sum::<usize>();
        let invalid_task_cluster_count = task_cluster_count.saturating_sub(valid_task_cluster_count);
        let valid_recipe_effects = recipes.iter()
            .filter_map(|recipe| recipe.mean_task_effect.clone())
            .collect::<Vec<_>>();
        let valid_recipe_template_count = valid_recipe_effects.len();
        let invalid_recipe_template_count = recipes.len().saturating_sub(valid_recipe_template_count);
        let mean_recipe_effect = ExactHierarchicalEffect::checked_mean(&valid_recipe_effects)?;
        let detail = HierarchicalConceptEffect {
            domain_id: domain.clone(),
            mechanism_family_id: family.clone(),
            concept_digest: concept.clone(),
            task_cluster_count,
            valid_task_cluster_count,
            invalid_task_cluster_count,
            recipe_template_count: recipes.len(),
            valid_recipe_template_count,
            invalid_recipe_template_count,
            mean_recipe_effect: mean_recipe_effect.clone(),
        };
        if let Some(effect) = mean_recipe_effect {
            concept_effects_by_stratum.entry((domain.clone(), family.clone())).or_default().push(effect);
        }
        let counts = concept_counts_by_stratum.entry((domain, family)).or_default();
        counts.total_concepts = counts.total_concepts.checked_add(1).ok_or_else(|| {
            HierarchicalInferenceError::ArithmeticOverflow("concept count overflow".into())
        })?;
        if mean_recipe_effect.is_some() {
            counts.valid_concepts = counts.valid_concepts.checked_add(1).ok_or_else(|| {
                HierarchicalInferenceError::ArithmeticOverflow("valid concept count overflow".into())
            })?;
        } else {
            counts.concepts_without_valid_recipes = counts.concepts_without_valid_recipes.checked_add(1).ok_or_else(|| {
                HierarchicalInferenceError::ArithmeticOverflow("invalid concept count overflow".into())
            })?;
        }
        concept_details.push(detail);
    }

    let mut summaries = Vec::with_capacity(domains.len() * families.len());
    for domain in domains {
        for family in families {
            let key = (domain.clone(), family.clone());
            let counts = concept_counts_by_stratum.get(&key).cloned().unwrap_or_default();
            let effects = concept_effects_by_stratum.get(&key).cloned().unwrap_or_default();
            let mut candidate_wins = 0usize;
            let mut reference_wins = 0usize;
            let mut tied = 0usize;
            for effect in &effects {
                match effect.sign() {
                    std::cmp::Ordering::Greater => candidate_wins = candidate_wins.checked_add(1).ok_or_else(|| {
                        HierarchicalInferenceError::ArithmeticOverflow("candidate concept win count overflow".into())
                    })?,
                    std::cmp::Ordering::Less => reference_wins = reference_wins.checked_add(1).ok_or_else(|| {
                        HierarchicalInferenceError::ArithmeticOverflow("reference concept win count overflow".into())
                    })?,
                    std::cmp::Ordering::Equal => tied = tied.checked_add(1).ok_or_else(|| {
                        HierarchicalInferenceError::ArithmeticOverflow("tied concept count overflow".into())
                    })?,
                }
            }
            let p_value = exact_sign_p_value(candidate_wins, reference_wins)?;
            let mean_effect = ExactHierarchicalEffect::checked_mean(&effects)?;
            let interval = mean_effect.as_ref()
                .filter(|_| !effects.is_empty())
                .map(|mean| mean_interval(mean, effects.len()))
                .transpose()?;
            summaries.push(DomainFamilyEndpointInference {
                domain_id: domain.clone(),
                mechanism_family_id: family.clone(),
                endpoint,
                total_concept_clusters: counts.total_concepts,
                valid_concept_clusters: counts.valid_concepts,
                concepts_without_valid_recipes: counts.concepts_without_valid_recipes,
                required_valid_concept_clusters: minimum_concepts,
                meets_minimum_concept_count: counts.valid_concepts >= minimum_concepts,
                candidate_concept_wins: candidate_wins,
                reference_concept_wins: reference_wins,
                tied_concepts: tied,
                discordant_concepts: candidate_wins.checked_add(reference_wins).ok_or_else(|| {
                    HierarchicalInferenceError::ArithmeticOverflow("discordant concept count overflow".into())
                })?,
                exact_two_sided_sign_p_value: p_value,
                mean_concept_effect: mean_effect,
                mean_effect_95_percent_interval: interval,
                holm_rank: 0,
                holm_threshold: Rational::zero(),
                rejected_after_holm: false,
            });
        }
    }

    // Multiplicity correction is applied by the caller across the complete
    // domain × family × endpoint grid, not separately within one endpoint.
    recipe_details.shrink_to_fit();
    concept_details.shrink_to_fit();
    Ok((recipe_details, concept_details, summaries))
}

/// Recompute both upstream evidence layers and then aggregate at the declared
/// scientific-concept level. Supplied source receipts must exactly equal the
/// independently reconstructed receipts; summary-only or stale receipts fail.
#[allow(clippy::too_many_arguments)]
pub fn analyze_hierarchical_concept_inference<A>(
    plan: &HierarchicalInferencePlan,
    cluster_plan: &TaskClusterInferencePlan,
    cluster_receipt: &TaskClusterInferenceReceipt,
    reliability_plan: &RepeatedRunReliabilityPlan,
    reliability_receipt: &RepeatedRunReliabilityReceipt,
    manifest: &MatchedBenchmarkManifest,
    adaptive_plan: &AdaptiveBenchmarkPlan,
    split_registry: &ChallengeSplitRegistry,
    development_events: &[DevelopmentAdaptationEvent],
    freeze: &AdaptiveBenchmarkFreezeReceipt,
    calibration: &CalibrationQualificationReceipt,
    release: &AdaptiveBenchmarkSealedRelease,
    release_receipt: &AdaptiveBenchmarkLifecycleReceipt,
    auditor: &A,
) -> Result<HierarchicalInferenceReceipt, HierarchicalInferenceError>
where
    A: BenchmarkSemanticOverlapAuditor,
{
    validate_plan(plan, cluster_plan, reliability_plan, adaptive_plan, manifest, release)?;
    if reliability_receipt.split != EvidenceSplit::SealedHoldout
        || cluster_receipt.split != EvidenceSplit::SealedHoldout
        || cluster_receipt.campaign_id != plan.campaign_id
        || cluster_receipt.group_id != plan.group_id
        || cluster_receipt.source_reliability_analysis_id != reliability_plan.analysis_id
        || !cluster_receipt.repeated_runs_not_counted_as_independent_samples
        || !cluster_receipt.cluster_level_pairing_preserved
    {
        return Err(HierarchicalInferenceError::NonSealedEvidence(
            "source cluster inference receipt is not the expected sealed paired analysis".into(),
        ));
    }

    let recomputed_cluster_receipt = analyze_repeated_run_task_clusters(
        cluster_plan,
        reliability_plan,
        manifest,
        reliability_receipt,
    )
    .map_err(|error| HierarchicalInferenceError::SourceReceiptMismatch(error.to_string()))?;
    if &recomputed_cluster_receipt != cluster_receipt {
        return Err(HierarchicalInferenceError::SourceReceiptMismatch(
            "supplied SIM-REL-048 receipt differs from exact reconstruction of SIM-REL-046 clusters".into(),
        ));
    }

    let recomputed_release_receipt = verify_adaptive_benchmark_release(
        adaptive_plan,
        split_registry,
        development_events,
        freeze,
        calibration,
        release,
        auditor,
    )
    .map_err(|error| HierarchicalInferenceError::ReleaseMismatch(error.to_string()))?;
    if &recomputed_release_receipt != release_receipt {
        return Err(HierarchicalInferenceError::ReleaseMismatch(
            "supplied adaptive benchmark receipt differs from exact sealed release verification".into(),
        ));
    }
    if release_receipt.release_digest != plan.sealed_release_digest
        || release_receipt.release_id != plan.sealed_release_id
        || release_receipt.benchmark_id != plan.benchmark_id
        || release_receipt.benchmark_version != plan.benchmark_version
        || !release_receipt.split_firewall_passed
        || !release_receipt.semantic_overlap_audit_passed
    {
        return Err(HierarchicalInferenceError::ReleaseMismatch(
            "hierarchical analysis must bind a passed split-firewalled release".into(),
        ));
    }

    let base_rows = build_task_rows(
        plan,
        cluster_plan,
        reliability_plan,
        reliability_receipt,
        release,
    )?;
    let total_task_cluster_count = reliability_receipt.task_cluster_count;
    let valid_task_cluster_count = reliability_receipt.valid_task_cluster_count;
    let invalid_task_cluster_count = reliability_receipt.invalid_task_cluster_count;

    let mut all_recipe_details = Vec::new();
    let mut all_concept_details = Vec::new();
    let mut all_summaries = Vec::new();
    for endpoint in &cluster_plan.endpoints {
        let (recipes, concepts, summaries) = aggregate_for_endpoint(
            &base_rows,
            &plan.domains,
            &cluster_plan.mechanism_families,
            *endpoint,
            reliability_receipt,
            plan.minimum_valid_concepts_per_domain_family,
        )?;
        all_recipe_details.extend(recipes);
        all_concept_details.extend(concepts);
        all_summaries.extend(summaries);
    }
    apply_holm(&mut all_summaries, plan.family_wise_alpha)?;
    let recipe_template_count = all_recipe_details.iter()
        .map(|detail| (
            detail.domain_id.clone(),
            detail.mechanism_family_id.clone(),
            detail.concept_digest.clone(),
            detail.recipe_template_digest.clone(),
        ))
        .collect::<BTreeSet<_>>()
        .len();
    let concept_cluster_count = all_concept_details.iter()
        .map(|detail| (
            detail.domain_id.clone(),
            detail.mechanism_family_id.clone(),
            detail.concept_digest.clone(),
        ))
        .collect::<BTreeSet<_>>()
        .len();
    let valid_concept_cluster_count = all_concept_details.iter()
        .filter(|detail| detail.mean_recipe_effect.is_some())
        .map(|detail| (detail.domain_id.clone(), detail.mechanism_family_id.clone(), detail.concept_digest.clone()))
        .collect::<BTreeSet<_>>()
        .len();
    let hypothesis_count = all_summaries.len();

    Ok(HierarchicalInferenceReceipt {
        analysis_id: plan.analysis_id.clone(),
        analysis_version: plan.analysis_version.clone(),
        source_task_cluster_analysis_id: plan.source_task_cluster_analysis_id.clone(),
        source_reliability_analysis_id: plan.source_reliability_analysis_id.clone(),
        pre_registration_receipt_digest: plan.pre_registration_receipt_digest.clone(),
        campaign_id: plan.campaign_id.clone(),
        benchmark_id: plan.benchmark_id.clone(),
        benchmark_version: plan.benchmark_version.clone(),
        sealed_release_id: plan.sealed_release_id.clone(),
        sealed_release_digest: plan.sealed_release_digest.clone(),
        scope: plan.scope,
        split: EvidenceSplit::SealedHoldout,
        group_id: plan.group_id.clone(),
        domain_count: plan.domains.len(),
        mechanism_family_count: cluster_plan.mechanism_families.len(),
        endpoint_count: cluster_plan.endpoints.len(),
        hypothesis_count,
        task_cluster_count: total_task_cluster_count,
        valid_task_cluster_count,
        invalid_task_cluster_count,
        recipe_template_count,
        concept_cluster_count,
        valid_concept_cluster_count,
        inferential_unit: "one distinct domain × mechanism_family × scientific_concept per endpoint hypothesis".into(),
        hierarchy: all_recipe_details,
        concept_effects: all_concept_details,
        domain_family_endpoint_results: all_summaries,
        runs_collapsed_within_task: true,
        tasks_equal_weighted_within_recipe: true,
        recipes_equal_weighted_within_concept: true,
        one_concept_one_effect_per_hypothesis: true,
        domains_reported_as_fixed_strata: true,
        pointwise_intervals_not_simultaneous: true,
        abstention_not_given_unregistered_direction: true,
        sealed_holdout_only: true,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        ExactHierarchicalEffect, HierarchicalInferenceError, TaskEffectRow,
        aggregate_for_endpoint, exact_sign_p_value, mean_interval,
    };
    use super::super::task_cluster_inference::ClusterInferenceEndpoint;
    use super::super::repeated_run_reliability::{
        ArmTaskReliability, TaskClusterReliability,
    };
    use super::super::counterexample_ledger::EvidenceSplit;

    fn ratio(numerator: i128, denominator: u128) -> ExactHierarchicalEffect {
        ExactHierarchicalEffect::new(numerator, denominator).unwrap()
    }

    fn arm(successes: usize, hazard: bool, abstention: bool) -> ArmTaskReliability {
        ArmTaskReliability {
            safe_success_count: successes,
            pass_at_k: successes > 0,
            pass_every_k: successes == 3,
            completion_count: 3,
            completed_at_least_once: true,
            completed_every_run: true,
            hazard_count: usize::from(hazard),
            any_hazard: hazard,
            abstention_count: usize::from(abstention),
            any_abstention: abstention,
        }
    }

    fn cluster(task: &str, candidate_successes: usize, reference_successes: usize) -> TaskClusterReliability {
        TaskClusterReliability {
            mechanism_family_id: "causal-mechanism".into(),
            task_id: task.into(),
            repetition_count: 3,
            invalid_task_cluster: false,
            reference: Some(arm(reference_successes, false, false)),
            candidate: Some(arm(candidate_successes, false, false)),
        }
    }

    fn invalid_cluster(task: &str) -> TaskClusterReliability {
        TaskClusterReliability {
            mechanism_family_id: "causal-mechanism".into(),
            task_id: task.into(),
            repetition_count: 3,
            invalid_task_cluster: true,
            reference: None,
            candidate: None,
        }
    }

    fn rows() -> (Vec<TaskEffectRow>, Vec<TaskClusterReliability>) {
        let clusters = vec![
            cluster("t1", 3, 0),
            cluster("t2", 0, 3),
            cluster("t3", 3, 0),
            cluster("t4", 1, 0),
            invalid_cluster("t5"),
        ];
        let rows = vec![
            TaskEffectRow {
                domain_id: "materials".into(),
                mechanism_family_id: "causal-mechanism".into(),
                concept_digest: "concept-a".into(),
                recipe_template_digest: "recipe-1".into(),
                task_id: "t1".into(),
            },
            TaskEffectRow {
                domain_id: "materials".into(),
                mechanism_family_id: "causal-mechanism".into(),
                concept_digest: "concept-a".into(),
                recipe_template_digest: "recipe-1".into(),
                task_id: "t2".into(),
            },
            TaskEffectRow {
                domain_id: "materials".into(),
                mechanism_family_id: "causal-mechanism".into(),
                concept_digest: "concept-a".into(),
                recipe_template_digest: "recipe-2".into(),
                task_id: "t3".into(),
            },
            TaskEffectRow {
                domain_id: "materials".into(),
                mechanism_family_id: "causal-mechanism".into(),
                concept_digest: "concept-b".into(),
                recipe_template_digest: "recipe-3".into(),
                task_id: "t4".into(),
            },
            TaskEffectRow {
                domain_id: "materials".into(),
                mechanism_family_id: "causal-mechanism".into(),
                concept_digest: "concept-b".into(),
                recipe_template_digest: "recipe-4".into(),
                task_id: "t5".into(),
            },
        ];
        (rows, clusters)
    }

    #[test]
    fn exact_effect_reduces_and_averages_without_floating_point() {
        assert_eq!(ratio(2, 4), ratio(1, 2));
        let mean = ExactHierarchicalEffect::checked_mean(&[
            ratio(1, 2),
            ratio(1, 4),
        ]).unwrap().unwrap();
        assert_eq!(mean, ratio(3, 8));
    }

    #[test]
    fn sign_test_is_exact_symmetric_and_bounded() {
        assert_eq!(exact_sign_p_value(0, 0).unwrap(), crate::Rational::one());
        assert_eq!(exact_sign_p_value(5, 0).unwrap(), crate::Rational::new(1, 16).unwrap());
        assert_eq!(exact_sign_p_value(0, 5).unwrap(), exact_sign_p_value(5, 0).unwrap());
        assert!(matches!(
            exact_sign_p_value(121, 0),
            Err(HierarchicalInferenceError::UnsupportedExactSampleSize(_))
        ));
    }

    #[test]
    fn concept_mean_interval_contains_exact_centre() {
        let effect = ratio(1, 3);
        let interval = mean_interval(&effect, 4).unwrap();
        assert!(interval.lower_micro_units <= 333_334);
        assert!(interval.upper_micro_units >= 333_333);
        assert_eq!(interval.confidence_level_percent, 95);
    }

    #[test]
    fn task_effect_is_directional_and_hazard_is_inverted() {
        let task = cluster("task-1", 2, 1);
        let mean = super::task_effect(ClusterInferenceEndpoint::MeanRunSafeSuccessRate, &task, 3).unwrap();
        assert_eq!(mean, ratio(1, 3));
        let hazard_task = TaskClusterReliability {
            mechanism_family_id: "causal-mechanism".into(),
            task_id: "hazard".into(),
            repetition_count: 3,
            invalid_task_cluster: false,
            reference: Some(arm(1, true, false)),
            candidate: Some(arm(1, false, false)),
        };
        assert_eq!(
            super::task_effect(ClusterInferenceEndpoint::AnyHazard, &hazard_task, 3).unwrap(),
            ratio(1, 1)
        );
    }

    #[test]
    fn recipe_and_concept_weighting_cannot_be_inflated_by_many_tasks() {
        let (mut task_rows, clusters) = rows();
        let mut receipt = super::super::repeated_run_reliability::RepeatedRunReliabilityReceipt {
            analysis_id: "reliability".into(),
            analysis_version: "v1".into(),
            campaign_id: "campaign".into(),
            scope: super::super::matched_benchmark::BenchmarkScope::FullScientificCampaign,
            split: EvidenceSplit::SealedHoldout,
            group_id: "sealed".into(),
            reference_arm_id: "reference".into(),
            candidate_arm_id: "candidate".into(),
            repetitions_per_task: 3,
            independent_generalization_unit: "task".into(),
            task_cluster_count: clusters.len(),
            valid_task_cluster_count: 4,
            invalid_task_cluster_count: 1,
            family_summaries: Vec::new(),
            task_clusters: clusters,
            pass_at_k_is_not_pass_every_k: true,
            repeated_runs_not_counted_as_independent_task_samples: true,
            exact_manifest_and_resource_matching: true,
            sealed_holdout_only: true,
        };
        // Add 100 extra tasks to recipe-1 with the same positive task effect.
        // Recipe 1's mean changes only as a recipe estimate; concept A still
        // averages recipe 1 and recipe 2 equally, not 102 tasks.
        for index in 0..100 {
            let task_id = format!("extra-{index}");
            receipt.task_clusters.push(cluster(&task_id, 3, 0));
            task_rows.push(TaskEffectRow {
                domain_id: "materials".into(),
                mechanism_family_id: "causal-mechanism".into(),
                concept_digest: "concept-a".into(),
                recipe_template_digest: "recipe-1".into(),
                task_id,
            });
        }
        let (recipes, concepts, summaries) = aggregate_for_endpoint(
            &task_rows,
            &["materials".into(), "biology".into()],
            &["causal-mechanism".into()],
            ClusterInferenceEndpoint::PassAtK,
            &receipt,
            1,
        ).unwrap();
        let concept_a = concepts.iter().find(|c| c.concept_digest == "concept-a" && c.domain_id == "materials").unwrap();
        let recipe_1 = recipes.iter().find(|r| r.recipe_template_digest == "recipe-1").unwrap();
        assert_eq!(recipe_1.valid_task_cluster_count, 102);
        assert_eq!(recipe_1.mean_task_effect, Some(ratio(50, 51)));
        assert_eq!(concept_a.recipe_template_count, 2);
        assert_eq!(concept_a.mean_recipe_effect, Some(ratio(101, 102)));
        assert_eq!(summaries.len(), 2);
        let empty_domain = summaries.iter().find(|s| s.domain_id == "biology").unwrap();
        assert_eq!(empty_domain.valid_concept_clusters, 0);
        assert!(!empty_domain.meets_minimum_concept_count);
    }
}
