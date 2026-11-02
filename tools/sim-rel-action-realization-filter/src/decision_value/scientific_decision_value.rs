//! Decision value per resource for scientific-agent workflows.
//!
//! The unit of inference is the SIM-REL-050 scientific concept within a
//! registered domain × mechanism-family stratum. This module reports decision
//! loss reduction, verified outcome improvement, each resource-cost channel,
//! and safety as separate dimensions. It never manufactures a universal
//! composite score from incomparable units.
//!
//! Causal language is permitted only for a valid randomized paired or
//! preregistered crossover design. Observational designs remain association-only.

use super::hierarchical_benchmark_inference::HierarchicalInferenceReceipt;
use super::matched_benchmark::BenchmarkScope;
use super::task_cluster_inference::ClusterInferenceEndpoint;
use super::counterexample_ledger::EvidenceSplit;
use crate::Rational;
use std::collections::{BTreeMap, BTreeSet};

const MAX_ID_BYTES: usize = 256;
const MAX_RESOURCE_CHANNELS: usize = 64;
const MAX_CONCEPT_EVIDENCE: usize = 100_000;
const MAX_EXACT_DISCORDANT_CONCEPTS: usize = 120;
const MICRO_SCALE: i128 = 1_000_000;
const SQRT_RADIUS_NUMERATOR: u128 = 2_000_000_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionValuePlan {
    pub analysis_id: String,
    pub analysis_version: String,
    pub pre_registration_receipt_digest: String,
    pub decision_loss_table_digest: String,
    pub decision_loss_verifier_profile_digest: String,
    pub source_hierarchical_analysis_id: String,
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub sealed_release_id: String,
    pub sealed_release_digest: String,
    pub campaign_id: String,
    pub group_id: String,
    pub scope: BenchmarkScope,
    pub reference_arm_id: String,
    pub candidate_arm_id: String,
    /// Complete fixed domain/family registries include zero-concept strata.
    pub domains: Vec<String>,
    pub mechanism_families: Vec<String>,
    pub minimum_valid_concepts_per_stratum: usize,
    pub family_wise_alpha: Rational,
    pub causal_design: CausalIdentificationDesign,
    pub outcome_measure: Option<OutcomeMeasureDefinition>,
    /// Units are never combined. Every channel remains visible separately.
    pub resource_channels: Vec<ResourceChannelDefinition>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CausalIdentificationDesign {
    RandomizedPaired {
        design_receipt_digest: String,
        randomization_receipt_digest: String,
        allocation_concealment_receipt_digest: String,
        interference_assessment_digest: String,
        randomized_assignment_verified: bool,
        allocation_concealed: bool,
        interference_checked: bool,
    },
    CounterbalancedCrossover {
        design_receipt_digest: String,
        sequence_randomization_digest: String,
        washout_protocol_digest: String,
        carryover_test_receipt_digest: String,
        order_randomized: bool,
        washout_completed: bool,
        carryover_test_passed: bool,
    },
    Observational {
        design_receipt_digest: String,
        confounding_assessment_digest: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CausalAttributionStatus {
    CausalIdentifiedUnderDeclaredDesign,
    AssociationOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutcomeDirection {
    HigherIsBetter,
    LowerIsBetter,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutcomeMeasureDefinition {
    pub measure_id: String,
    pub unit: String,
    pub direction: OutcomeDirection,
    pub lower_bound: ExactValue,
    pub upper_bound: ExactValue,
    pub verifier_profile_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceChannelDefinition {
    pub channel_id: String,
    pub unit: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceMeasurement {
    pub channel_id: String,
    pub amount: ExactValue,
    pub complete: bool,
    pub meter_receipt_digest: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConceptArmResourceUse {
    pub measurements: Vec<ResourceMeasurement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedDecisionLoss {
    /// Loss must be normalized to [0,1] using the preregistered loss table.
    pub normalized_loss: ExactValue,
    pub decision_loss_evidence_digest: String,
    pub independent_loss_verification_receipt_digest: String,
    pub loss_verifier_profile_digest: String,
    pub oracle_loss_table_digest: String,
    pub selected_decision_digest: String,
    pub oracle_optimal_decision_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedOutcomeMeasurement {
    pub value: ExactValue,
    pub raw_observation_digest: String,
    pub independent_verification_receipt_digest: String,
    pub verifier_profile_digest: String,
    pub calibration_chain_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConceptEvidenceStatus {
    Verified,
    Inconclusive,
    Invalidated,
    NotMeasured,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConceptDecisionValueEvidence {
    pub domain_id: String,
    pub mechanism_family_id: String,
    pub concept_digest: String,
    pub evidence_status: ConceptEvidenceStatus,
    pub baseline_loss: Option<VerifiedDecisionLoss>,
    pub candidate_loss: Option<VerifiedDecisionLoss>,
    pub baseline_outcome: Option<VerifiedOutcomeMeasurement>,
    pub candidate_outcome: Option<VerifiedOutcomeMeasurement>,
    pub baseline_resource_use: ConceptArmResourceUse,
    pub candidate_resource_use: ConceptArmResourceUse,
    pub baseline_hazard: bool,
    pub candidate_hazard: bool,
    pub candidate_forbidden_transition: bool,
    pub candidate_false_safety_certifications: usize,
    pub safety_review_receipt_digest: String,
    pub concept_evidence_receipt_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactValue {
    pub numerator: i128,
    pub denominator: u128,
}

impl ExactValue {
    fn new(numerator: i128, denominator: u128) -> Result<Self, DecisionValueError> {
        if denominator == 0 {
            return Err(DecisionValueError::InvalidValue(
                "exact value denominator must be positive".into(),
            ));
        }
        let divisor = gcd_u128(numerator.unsigned_abs(), denominator);
        let divisor_i128 = i128::try_from(divisor).map_err(|_| {
            DecisionValueError::ArithmeticOverflow("value reduction divisor exceeds i128".into())
        })?;
        Ok(Self {
            numerator: numerator / divisor_i128,
            denominator: denominator / divisor,
        })
    }

    fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    fn one() -> Self {
        Self {
            numerator: 1,
            denominator: 1,
        }
    }

    fn checked_add(&self, other: &Self) -> Result<Self, DecisionValueError> {
        let common = gcd_u128(self.denominator, other.denominator);
        let left_factor = other.denominator / common;
        let right_factor = self.denominator / common;
        let left = self.numerator.checked_mul(i128::try_from(left_factor).map_err(|_| {
            DecisionValueError::ArithmeticOverflow("value addition factor exceeds i128".into())
        })?).ok_or_else(|| {
            DecisionValueError::ArithmeticOverflow("value addition numerator overflow".into())
        })?;
        let right = other.numerator.checked_mul(i128::try_from(right_factor).map_err(|_| {
            DecisionValueError::ArithmeticOverflow("value addition factor exceeds i128".into())
        })?).ok_or_else(|| {
            DecisionValueError::ArithmeticOverflow("value addition numerator overflow".into())
        })?;
        let numerator = left.checked_add(right).ok_or_else(|| {
            DecisionValueError::ArithmeticOverflow("value addition overflow".into())
        })?;
        let denominator = self.denominator.checked_mul(left_factor).ok_or_else(|| {
            DecisionValueError::ArithmeticOverflow("value addition denominator overflow".into())
        })?;
        Self::new(numerator, denominator)
    }

    fn checked_sub(&self, other: &Self) -> Result<Self, DecisionValueError> {
        let negative = other.numerator.checked_neg().ok_or_else(|| {
            DecisionValueError::ArithmeticOverflow("value negation overflow".into())
        })?;
        self.checked_add(&Self::new(negative, other.denominator)?)
    }

    fn checked_div(&self, other: &Self) -> Result<Self, DecisionValueError> {
        if other.numerator == 0 {
            return Err(DecisionValueError::InvalidValue("division by zero".into()));
        }
        let sign = if other.numerator < 0 { -1i128 } else { 1i128 };
        let numerator = self.numerator
            .checked_mul(i128::try_from(other.denominator).map_err(|_| {
                DecisionValueError::ArithmeticOverflow("value denominator exceeds i128".into())
            })?)
            .and_then(|value| value.checked_mul(sign))
            .ok_or_else(|| DecisionValueError::ArithmeticOverflow("value division overflow".into()))?;
        let denominator = self.denominator.checked_mul(other.numerator.unsigned_abs()).ok_or_else(|| {
            DecisionValueError::ArithmeticOverflow("value division denominator overflow".into())
        })?;
        Self::new(numerator, denominator)
    }

    fn mean(values: &[Self]) -> Result<Option<Self>, DecisionValueError> {
        if values.is_empty() {
            return Ok(None);
        }
        let mut sum = Self::zero();
        for value in values {
            sum = sum.checked_add(value)?;
        }
        let count = Self::new(i128::try_from(values.len()).map_err(|_| {
            DecisionValueError::ArithmeticOverflow("value count exceeds i128".into())
        })?, 1)?;
        Ok(Some(sum.checked_div(&count)?))
    }

    fn compare(&self, other: &Self) -> Result<std::cmp::Ordering, DecisionValueError> {
        let left = self.numerator.checked_mul(i128::try_from(other.denominator).map_err(|_| {
            DecisionValueError::ArithmeticOverflow("comparison denominator exceeds i128".into())
        })?).ok_or_else(|| DecisionValueError::ArithmeticOverflow("value comparison overflow".into()))?;
        let right = other.numerator.checked_mul(i128::try_from(self.denominator).map_err(|_| {
            DecisionValueError::ArithmeticOverflow("comparison denominator exceeds i128".into())
        })?).ok_or_else(|| DecisionValueError::ArithmeticOverflow("value comparison overflow".into()))?;
        Ok(left.cmp(&right))
    }

    fn is_non_negative(&self) -> bool {
        self.numerator >= 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DecisionValueEndpoint {
    DecisionLossReduction,
    VerifiedOutcomeImprovement,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceChannelSummary {
    pub domain_id: String,
    pub mechanism_family_id: String,
    pub channel_id: String,
    pub unit: String,
    pub complete_concept_count: usize,
    pub expected_valid_concept_count: usize,
    pub reference_total: Option<ExactValue>,
    pub candidate_total: Option<ExactValue>,
    /// Candidate minus reference. Positive means the agent-assisted workflow
    /// consumed more of this channel.
    pub mean_incremental_cost_per_concept: Option<ExactValue>,
    /// Incremental resource cost / normalized mean decision-loss reduction.
    /// None if the cost channel is incomplete or the loss-reduction mean <= 0.
    pub incremental_cost_per_decision_loss_unit: Option<ExactValue>,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainFamilyDecisionValueInference {
    pub domain_id: String,
    pub mechanism_family_id: String,
    pub endpoint: DecisionValueEndpoint,
    pub source_concept_count: usize,
    pub measured_concept_count: usize,
    pub missing_or_invalid_concept_count: usize,
    pub required_valid_concept_count: usize,
    pub meets_minimum_concept_count: bool,
    pub candidate_concept_wins: usize,
    pub reference_concept_wins: usize,
    pub tied_concepts: usize,
    pub discordant_concepts: usize,
    pub exact_two_sided_sign_p_value: Rational,
    pub mean_concept_effect: Option<ExactValue>,
    pub mean_effect_95_percent_interval: Option<DecisionValueInterval>,
    pub holm_rank: usize,
    pub holm_threshold: Rational,
    pub rejected_after_holm: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionValueInterval {
    pub lower_micro_units: i128,
    pub upper_micro_units: i128,
    pub scale: u128,
    pub confidence_level_percent: u8,
    pub method: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionValueReceipt {
    pub analysis_id: String,
    pub analysis_version: String,
    pub source_hierarchical_analysis_id: String,
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub sealed_release_id: String,
    pub sealed_release_digest: String,
    pub campaign_id: String,
    pub scope: BenchmarkScope,
    pub split: EvidenceSplit,
    pub group_id: String,
    pub causal_attribution_status: CausalAttributionStatus,
    pub domain_count: usize,
    pub mechanism_family_count: usize,
    pub endpoint_count: usize,
    pub hypothesis_count: usize,
    pub source_concept_count: usize,
    pub verified_concept_count: usize,
    pub inconclusive_concept_count: usize,
    pub invalidated_concept_count: usize,
    pub not_measured_concept_count: usize,
    pub baseline_hazard_concept_count: usize,
    pub candidate_hazard_concept_count: usize,
    pub candidate_forbidden_transition_count: usize,
    pub candidate_false_safety_certification_count: usize,
    pub safety_gate_passed: bool,
    pub evidence_verification_required: bool,
    pub cost_channels: Vec<ResourceChannelSummary>,
    pub domain_family_results: Vec<DomainFamilyDecisionValueInference>,
    pub decision_loss_and_outcome_reported_separately: bool,
    pub resource_channels_not_collapsed: bool,
    pub abstention_and_safety_not_traded_for_utility: bool,
    pub missing_cost_prevents_complete_cost_effectiveness: bool,
    pub no_causal_claim_from_observational_design: bool,
    pub concept_level_inference_preserved: bool,
    pub sealed_holdout_only: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisionValueError {
    InvalidPlan(String),
    InvalidIdentity(String),
    InvalidDigest(String),
    InvalidValue(String),
    SourceMismatch(String),
    NonSealedEvidence(String),
    CausalDesign(String),
    ConceptSetMismatch(String),
    DuplicateConcept(String),
    MissingOutcomeEvidence(String),
    ResourceAccounting(String),
    SafetyEvidence(String),
    UnsupportedSampleSize(String),
    ArithmeticOverflow(String),
}

impl std::fmt::Display for DecisionValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlan(reason)
            | Self::InvalidIdentity(reason)
            | Self::InvalidDigest(reason)
            | Self::InvalidValue(reason)
            | Self::SourceMismatch(reason)
            | Self::NonSealedEvidence(reason)
            | Self::CausalDesign(reason)
            | Self::ConceptSetMismatch(reason)
            | Self::DuplicateConcept(reason)
            | Self::MissingOutcomeEvidence(reason)
            | Self::ResourceAccounting(reason)
            | Self::SafetyEvidence(reason)
            | Self::UnsupportedSampleSize(reason)
            | Self::ArithmeticOverflow(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for DecisionValueError {}

fn gcd_u128(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn require_id(value: &str, label: &str) -> Result<(), DecisionValueError> {
    if !valid_id(value) {
        return Err(DecisionValueError::InvalidIdentity(format!(
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

fn require_digest(value: &str, label: &str) -> Result<(), DecisionValueError> {
    if !valid_digest(value) {
        return Err(DecisionValueError::InvalidDigest(format!(
            "{label} is not a canonical sha256 digest"
        )));
    }
    Ok(())
}

fn in_unit_interval(value: &ExactValue) -> Result<bool, DecisionValueError> {
    Ok(value.compare(&ExactValue::zero())? != std::cmp::Ordering::Less
        && value.compare(&ExactValue::one())? != std::cmp::Ordering::Greater)
}

fn validate_causal_design(
    design: &CausalIdentificationDesign,
) -> Result<CausalAttributionStatus, DecisionValueError> {
    match design {
        CausalIdentificationDesign::RandomizedPaired {
            design_receipt_digest,
            randomization_receipt_digest,
            allocation_concealment_receipt_digest,
            interference_assessment_digest,
            randomized_assignment_verified,
            allocation_concealed,
            interference_checked,
        } => {
            for (value, label) in [
                (design_receipt_digest, "randomized design receipt"),
                (randomization_receipt_digest, "randomization receipt"),
                (allocation_concealment_receipt_digest, "allocation concealment"),
                (interference_assessment_digest, "interference assessment"),
            ] {
                require_digest(value, label)?;
            }
            if *randomized_assignment_verified && *allocation_concealed && *interference_checked {
                Ok(CausalAttributionStatus::CausalIdentifiedUnderDeclaredDesign)
            } else {
                Ok(CausalAttributionStatus::AssociationOnly)
            }
        }
        CausalIdentificationDesign::CounterbalancedCrossover {
            design_receipt_digest,
            sequence_randomization_digest,
            washout_protocol_digest,
            carryover_test_receipt_digest,
            order_randomized,
            washout_completed,
            carryover_test_passed,
        } => {
            for (value, label) in [
                (design_receipt_digest, "crossover design receipt"),
                (sequence_randomization_digest, "crossover sequence randomization"),
                (washout_protocol_digest, "crossover washout protocol"),
                (carryover_test_receipt_digest, "crossover carryover test"),
            ] {
                require_digest(value, label)?;
            }
            if *order_randomized && *washout_completed && *carryover_test_passed {
                Ok(CausalAttributionStatus::CausalIdentifiedUnderDeclaredDesign)
            } else {
                Ok(CausalAttributionStatus::AssociationOnly)
            }
        }
        CausalIdentificationDesign::Observational {
            design_receipt_digest,
            confounding_assessment_digest,
        } => {
            require_digest(design_receipt_digest, "observational design receipt")?;
            require_digest(confounding_assessment_digest, "confounding assessment")?;
            Ok(CausalAttributionStatus::AssociationOnly)
        }
    }
}

fn validate_plan(plan: &DecisionValuePlan) -> Result<CausalAttributionStatus, DecisionValueError> {
    for (value, label) in [
        (&plan.analysis_id, "decision-value analysis ID"),
        (&plan.analysis_version, "decision-value analysis version"),
        (&plan.source_hierarchical_analysis_id, "source hierarchical analysis ID"),
        (&plan.benchmark_id, "benchmark ID"),
        (&plan.benchmark_version, "benchmark version"),
        (&plan.sealed_release_id, "sealed release ID"),
        (&plan.campaign_id, "campaign ID"),
        (&plan.group_id, "sealed group ID"),
        (&plan.reference_arm_id, "reference arm ID"),
        (&plan.candidate_arm_id, "candidate arm ID"),
    ] {
        require_id(value, label)?;
    }
    for (value, label) in [
        (&plan.pre_registration_receipt_digest, "decision-value preregistration"),
        (&plan.decision_loss_table_digest, "decision loss table"),
        (&plan.decision_loss_verifier_profile_digest, "decision loss verifier profile"),
        (&plan.sealed_release_digest, "sealed release"),
    ] {
        require_digest(value, label)?;
    }
    if plan.reference_arm_id == plan.candidate_arm_id
        || plan.domains.is_empty()
        || plan.mechanism_families.is_empty()
        || plan.minimum_valid_concepts_per_stratum == 0
        || plan.family_wise_alpha.is_zero()
        || !plan.family_wise_alpha.is_probability()
        || plan.resource_channels.is_empty()
        || plan.resource_channels.len() > MAX_RESOURCE_CHANNELS
    {
        return Err(DecisionValueError::InvalidPlan(
            "distinct arms, positive concept floor, alpha in (0,1] and bounded non-empty resource channels are required".into(),
        ));
    }
    let mut domain_ids = BTreeSet::new();
    for domain in &plan.domains {
        require_id(domain, "decision-value domain")?;
        if !domain_ids.insert(domain.clone()) {
            return Err(DecisionValueError::InvalidPlan("domain registry contains duplicates".into()));
        }
    }
    let mut family_ids = BTreeSet::new();
    for family in &plan.mechanism_families {
        require_id(family, "decision-value mechanism family")?;
        if !family_ids.insert(family.clone()) {
            return Err(DecisionValueError::InvalidPlan("mechanism-family registry contains duplicates".into()));
        }
    }
    let mut channel_ids = BTreeSet::new();
    for channel in &plan.resource_channels {
        require_id(&channel.channel_id, "resource channel ID")?;
        require_id(&channel.unit, "resource channel unit")?;
        if !channel_ids.insert(channel.channel_id.clone()) {
            return Err(DecisionValueError::InvalidPlan(
                "resource channel IDs must be unique".into(),
            ));
        }
    }
    if let Some(measure) = &plan.outcome_measure {
        require_id(&measure.measure_id, "outcome measure ID")?;
        require_id(&measure.unit, "outcome measure unit")?;
        require_digest(&measure.verifier_profile_digest, "outcome verifier profile")?;
        if measure.lower_bound.compare(&measure.upper_bound)? != std::cmp::Ordering::Less {
            return Err(DecisionValueError::InvalidPlan(
                "outcome measure lower bound must be below upper bound".into(),
            ));
        }
    }
    validate_causal_design(&plan.causal_design)
}

fn validate_source_hierarchy(
    plan: &DecisionValuePlan,
    hierarchy: &HierarchicalInferenceReceipt,
) -> Result<BTreeSet<(String, String, String)>, DecisionValueError> {
    if hierarchy.split != EvidenceSplit::SealedHoldout
        || !hierarchy.sealed_holdout_only
        || !hierarchy.runs_collapsed_within_task
        || !hierarchy.tasks_equal_weighted_within_recipe
        || !hierarchy.recipes_equal_weighted_within_concept
        || !hierarchy.one_concept_one_effect_per_hypothesis
    {
        return Err(DecisionValueError::NonSealedEvidence(
            "source hierarchy must be a qualified SealedHoldout concept-level analysis".into(),
        ));
    }
    if hierarchy.analysis_id != plan.source_hierarchical_analysis_id
        || hierarchy.benchmark_id != plan.benchmark_id
        || hierarchy.benchmark_version != plan.benchmark_version
        || hierarchy.sealed_release_id != plan.sealed_release_id
        || hierarchy.sealed_release_digest != plan.sealed_release_digest
        || hierarchy.campaign_id != plan.campaign_id
        || hierarchy.scope != plan.scope
        || hierarchy.group_id != plan.group_id
    {
        return Err(DecisionValueError::SourceMismatch(
            "decision-value plan does not match the exact SIM-REL-050 hierarchy and sealed release".into(),
        ));
    }
    let registered_domains = plan.domains.iter().cloned().collect::<BTreeSet<_>>();
    let registered_families = plan.mechanism_families.iter().cloned().collect::<BTreeSet<_>>();
    let hierarchy_domains = hierarchy.domain_family_endpoint_results.iter()
        .map(|row| row.domain_id.clone())
        .collect::<BTreeSet<_>>();
    let hierarchy_families = hierarchy.domain_family_endpoint_results.iter()
        .map(|row| row.mechanism_family_id.clone())
        .collect::<BTreeSet<_>>();
    let endpoint_registry = hierarchy.domain_family_endpoint_results.iter()
        .map(|row| (row.domain_id.clone(), row.mechanism_family_id.clone(), row.endpoint))
        .collect::<BTreeSet<_>>();
    let expected_grid_size = plan.domains.len()
        .checked_mul(plan.mechanism_families.len())
        .and_then(|value| value.checked_mul(hierarchy.endpoint_count))
        .ok_or_else(|| DecisionValueError::ArithmeticOverflow(
            "source hierarchy domain × family × endpoint grid overflow".into(),
        ))?;
    if hierarchy_domains != registered_domains
        || hierarchy_families != registered_families
        || endpoint_registry.len() != expected_grid_size
        || hierarchy.domain_family_endpoint_results.len() != expected_grid_size
    {
        return Err(DecisionValueError::SourceMismatch(
            "preregistered decision-value domain/family registry must match the complete source hierarchy grid, including empty strata".into(),
        ));
    }
    let mut concepts = BTreeSet::new();
    for concept in &hierarchy.concept_effects {
        require_id(&concept.domain_id, "source concept domain")?;
        require_id(&concept.mechanism_family_id, "source concept mechanism family")?;
        require_digest(&concept.concept_digest, "source concept digest")?;
        if !registered_domains.contains(&concept.domain_id)
            || !registered_families.contains(&concept.mechanism_family_id)
        {
            return Err(DecisionValueError::ConceptSetMismatch(
                "source concept is outside the preregistered domain/family registry".into(),
            ));
        }
        concepts.insert((
            concept.domain_id.clone(),
            concept.mechanism_family_id.clone(),
            concept.concept_digest.clone(),
        ));
    }
    if concepts.is_empty() {
        return Err(DecisionValueError::ConceptSetMismatch(
            "source hierarchy contains no concept units".into(),
        ));
    }
    Ok(concepts)
}

fn validate_resource_use(
    plan: &DecisionValuePlan,
    use_record: &ConceptArmResourceUse,
) -> Result<BTreeMap<String, &ResourceMeasurement>, DecisionValueError> {
    let allowed = plan.resource_channels.iter()
        .map(|channel| channel.channel_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut values = BTreeMap::new();
    for measurement in &use_record.measurements {
        require_id(&measurement.channel_id, "resource measurement channel")?;
        if !allowed.contains(measurement.channel_id.as_str()) {
            return Err(DecisionValueError::ResourceAccounting(
                "resource observation contains a channel not preregistered in the plan".into(),
            ));
        }
        if !measurement.amount.is_non_negative() {
            return Err(DecisionValueError::ResourceAccounting(
                "resource consumption cannot be negative".into(),
            ));
        }
        if measurement.complete {
            let digest = measurement.meter_receipt_digest.as_ref().ok_or_else(|| {
                DecisionValueError::ResourceAccounting(
                    "complete resource measurement requires a meter receipt".into(),
                )
            })?;
            require_digest(digest, "resource meter receipt")?;
        }
        if values.insert(measurement.channel_id.clone(), measurement).is_some() {
            return Err(DecisionValueError::ResourceAccounting(
                "resource channel appears more than once for an arm".into(),
            ));
        }
    }
    Ok(values)
}

fn validate_concept_evidence(
    plan: &DecisionValuePlan,
    evidence: &ConceptDecisionValueEvidence,
) -> Result<(bool, bool), DecisionValueError> {
    for (value, label) in [
        (&evidence.safety_review_receipt_digest, "safety review receipt"),
        (&evidence.concept_evidence_receipt_digest, "concept evidence receipt"),
    ] {
        require_digest(value, label)?;
    }
    validate_resource_use(plan, &evidence.baseline_resource_use)?;
    validate_resource_use(plan, &evidence.candidate_resource_use)?;

    match evidence.evidence_status {
        ConceptEvidenceStatus::Verified => {
            let baseline = evidence.baseline_loss.as_ref().ok_or_else(|| {
                DecisionValueError::MissingOutcomeEvidence("verified concept is missing baseline decision loss".into())
            })?;
            let candidate = evidence.candidate_loss.as_ref().ok_or_else(|| {
                DecisionValueError::MissingOutcomeEvidence("verified concept is missing candidate decision loss".into())
            })?;
            for loss in [baseline, candidate] {
                if loss.oracle_loss_table_digest != plan.decision_loss_table_digest
                    || loss.loss_verifier_profile_digest != plan.decision_loss_verifier_profile_digest
                {
                    return Err(DecisionValueError::MissingOutcomeEvidence(
                        "decision loss evidence uses a different loss table or verifier profile from the preregistered plan".into(),
                    ));
                }
                if !in_unit_interval(&loss.normalized_loss)? {
                    return Err(DecisionValueError::InvalidValue(
                        "normalized decision loss must be in [0,1]".into(),
                    ));
                }
                require_digest(&loss.decision_loss_evidence_digest, "decision loss evidence")?;
                require_digest(&loss.independent_loss_verification_receipt_digest, "independent loss verification")?;
                require_digest(&loss.loss_verifier_profile_digest, "loss verifier profile")?;
                require_digest(&loss.oracle_loss_table_digest, "oracle loss table")?;
                require_digest(&loss.selected_decision_digest, "selected terminal decision")?;
                require_digest(&loss.oracle_optimal_decision_digest, "oracle-optimal decision")?;
            }
            if baseline.oracle_optimal_decision_digest != candidate.oracle_optimal_decision_digest {
                return Err(DecisionValueError::MissingOutcomeEvidence(
                    "baseline and candidate decision-loss records do not share the same oracle-optimal decision reference".into(),
                ));
            }
            match (&plan.outcome_measure, &evidence.baseline_outcome, &evidence.candidate_outcome) {
                (Some(_), Some(baseline), Some(candidate)) => {
                    for measurement in [baseline, candidate] {
                        require_digest(&measurement.raw_observation_digest, "raw outcome observation")?;
                        require_digest(&measurement.independent_verification_receipt_digest, "independent outcome verification")?;
                        require_digest(&measurement.calibration_chain_digest, "measurement calibration chain")?;
                    }
                }
                (Some(_), _, _) => {
                    return Err(DecisionValueError::MissingOutcomeEvidence(
                        "verified outcome measure requires baseline and candidate independently verified outcomes".into(),
                    ));
                }
                (None, Some(_), _) | (None, _, Some(_)) => {
                    return Err(DecisionValueError::InvalidPlan(
                        "outcome observations supplied without a preregistered outcome measure".into(),
                    ));
                }
                (None, None, None) => {}
            }
            Ok((true, false))
        }
        ConceptEvidenceStatus::Inconclusive | ConceptEvidenceStatus::Invalidated | ConceptEvidenceStatus::NotMeasured => {
            if evidence.baseline_loss.is_some()
                || evidence.candidate_loss.is_some()
                || evidence.baseline_outcome.is_some()
                || evidence.candidate_outcome.is_some()
            {
                return Err(DecisionValueError::InvalidValue(
                    "non-verified concepts must not carry verified outcome values".into(),
                ));
            }
            Ok((false, true))
        }
    }
}

fn decision_loss_effect(
    evidence: &ConceptDecisionValueEvidence,
) -> Result<ExactValue, DecisionValueError> {
    let baseline = evidence.baseline_loss.as_ref().ok_or_else(|| {
        DecisionValueError::MissingOutcomeEvidence("baseline decision loss missing".into())
    })?;
    let candidate = evidence.candidate_loss.as_ref().ok_or_else(|| {
        DecisionValueError::MissingOutcomeEvidence("candidate decision loss missing".into())
    })?;
    baseline.normalized_loss.checked_sub(&candidate.normalized_loss)
}

fn outcome_effect(
    measure: &OutcomeMeasureDefinition,
    evidence: &ConceptDecisionValueEvidence,
) -> Result<ExactValue, DecisionValueError> {
    let baseline = evidence.baseline_outcome.as_ref().ok_or_else(|| {
        DecisionValueError::MissingOutcomeEvidence("baseline outcome missing".into())
    })?;
    let candidate = evidence.candidate_outcome.as_ref().ok_or_else(|| {
        DecisionValueError::MissingOutcomeEvidence("candidate outcome missing".into())
    })?;
    if baseline.value.compare(&measure.lower_bound)? == std::cmp::Ordering::Less
        || baseline.value.compare(&measure.upper_bound)? == std::cmp::Ordering::Greater
        || candidate.value.compare(&measure.lower_bound)? == std::cmp::Ordering::Less
        || candidate.value.compare(&measure.upper_bound)? == std::cmp::Ordering::Greater
    {
        return Err(DecisionValueError::InvalidValue(
            "verified outcome value is outside the preregistered measurement range".into(),
        ));
    }
    let raw = match measure.direction {
        OutcomeDirection::HigherIsBetter => candidate.value.checked_sub(&baseline.value)?,
        OutcomeDirection::LowerIsBetter => baseline.value.checked_sub(&candidate.value)?,
    };
    raw.checked_div(&measure.upper_bound.checked_sub(&measure.lower_bound)?)
}

fn cost_channel_summaries(
    plan: &DecisionValuePlan,
    domains: &[String],
    families: &[String],
    evidence: &[ConceptDecisionValueEvidence],
    decision_loss_effects: &BTreeMap<(String, String, String), ExactValue>,
) -> Result<Vec<ResourceChannelSummary>, DecisionValueError> {
    let mut summaries = Vec::with_capacity(domains.len() * families.len() * plan.resource_channels.len());
    for domain in domains {
        for family in families {
            let stratum = evidence.iter()
                .filter(|item| item.domain_id == *domain && item.mechanism_family_id == *family)
                .collect::<Vec<_>>();
            let valid_items = stratum.iter()
                .filter(|item| item.evidence_status == ConceptEvidenceStatus::Verified)
                .collect::<Vec<_>>();
            let valid_count = valid_items.len();
            let loss_values = valid_items.iter()
                .filter_map(|item| decision_loss_effects.get(&(
                    item.domain_id.clone(),
                    item.mechanism_family_id.clone(),
                    item.concept_digest.clone(),
                )).cloned())
                .collect::<Vec<_>>();
            let mean_loss_effect = ExactValue::mean(&loss_values)?;

            for channel in &plan.resource_channels {
                let mut reference_total = ExactValue::zero();
                let mut candidate_total = ExactValue::zero();
                let mut complete_concepts = 0usize;
                for item in &valid_items {
                    let reference = validate_resource_use(plan, &item.baseline_resource_use)?;
                    let candidate = validate_resource_use(plan, &item.candidate_resource_use)?;
                    match (reference.get(&channel.channel_id), candidate.get(&channel.channel_id)) {
                        (Some(reference), Some(candidate)) if reference.complete && candidate.complete => {
                            reference_total = reference_total.checked_add(&reference.amount)?;
                            candidate_total = candidate_total.checked_add(&candidate.amount)?;
                            complete_concepts = complete_concepts.checked_add(1).ok_or_else(|| {
                                DecisionValueError::ArithmeticOverflow("complete cost concept count overflow".into())
                            })?;
                        }
                        _ => {}
                    }
                }
                let complete = valid_count > 0 && complete_concepts == valid_count;
                let mean_increment = if complete {
                    Some(candidate_total.checked_sub(&reference_total)?.checked_div(
                        &ExactValue::new(i128::try_from(complete_concepts).map_err(|_| {
                            DecisionValueError::ArithmeticOverflow("cost concept count exceeds i128".into())
                        })?, 1)?
                    )?)
                } else {
                    None
                };
                let incremental_cost_per_loss_unit = if complete {
                    mean_loss_effect.as_ref()
                        .filter(|loss| loss.numerator > 0)
                        .map(|loss| {
                            candidate_total.checked_sub(&reference_total)?
                                .checked_div(&ExactValue::new(i128::try_from(complete_concepts).map_err(|_| {
                                    DecisionValueError::ArithmeticOverflow("cost concept count exceeds i128".into())
                                })?, 1)?)?
                                .checked_div(loss)
                        })
                        .transpose()?
                } else {
                    None
                };
                summaries.push(ResourceChannelSummary {
                    domain_id: domain.clone(),
                    mechanism_family_id: family.clone(),
                    channel_id: channel.channel_id.clone(),
                    unit: channel.unit.clone(),
                    complete_concept_count: complete_concepts,
                    expected_valid_concept_count: valid_count,
                    reference_total: complete.then_some(reference_total),
                    candidate_total: complete.then_some(candidate_total),
                    mean_incremental_cost_per_concept: mean_increment,
                    incremental_cost_per_decision_loss_unit: incremental_cost_per_loss_unit,
                    complete,
                });
            }
        }
    }
    Ok(summaries)
}
fn exact_sign_p_value(
    candidate_wins: usize,
    reference_wins: usize,
) -> Result<Rational, DecisionValueError> {
    let discordant = candidate_wins.checked_add(reference_wins).ok_or_else(|| {
        DecisionValueError::ArithmeticOverflow("decision-value discordant concept count overflow".into())
    })?;
    if discordant > MAX_EXACT_DISCORDANT_CONCEPTS {
        return Err(DecisionValueError::UnsupportedSampleSize(format!(
            "{discordant} discordant concepts exceeds exact sign-test cap {MAX_EXACT_DISCORDANT_CONCEPTS}"
        )));
    }
    if discordant == 0 {
        return Ok(Rational::one());
    }
    let k = candidate_wins.min(reference_wins).min(discordant - candidate_wins.min(reference_wins));
    let mut tail = 1u128;
    let mut coefficient = 1u128;
    for index in 1..=k {
        coefficient = coefficient
            .checked_mul(u128::try_from(discordant - index + 1).map_err(|_| {
                DecisionValueError::ArithmeticOverflow("sign coefficient factor exceeds u128".into())
            })?)
            .ok_or_else(|| DecisionValueError::ArithmeticOverflow("sign coefficient overflow".into()))?
            / u128::try_from(index).map_err(|_| {
                DecisionValueError::ArithmeticOverflow("sign coefficient divisor exceeds u128".into())
            })?;
        tail = tail.checked_add(coefficient).ok_or_else(|| {
            DecisionValueError::ArithmeticOverflow("sign tail overflow".into())
        })?;
    }
    let denominator = 1u128.checked_shl(discordant as u32).ok_or_else(|| {
        DecisionValueError::UnsupportedSampleSize("exact sign denominator exceeds u128".into())
    })?;
    let doubled = tail.checked_mul(2).ok_or_else(|| {
        DecisionValueError::ArithmeticOverflow("sign tail doubling overflow".into())
    })?;
    Rational::new(doubled.min(denominator), denominator)
        .map_err(DecisionValueError::ArithmeticOverflow)
}

fn mean_interval(
    mean: &ExactValue,
    count: usize,
) -> Result<DecisionValueInterval, DecisionValueError> {
    let denominator = i128::try_from(mean.denominator).map_err(|_| {
        DecisionValueError::ArithmeticOverflow("decision-value mean denominator exceeds i128".into())
    })?;
    let scaled = mean.numerator.checked_mul(MICRO_SCALE).ok_or_else(|| {
        DecisionValueError::ArithmeticOverflow("decision-value mean micro-scaling overflow".into())
    })?;
    let centre_lower = floor_div_signed(scaled, denominator);
    let centre_upper = ceil_div_signed(scaled, denominator);
    let n = u128::try_from(count).map_err(|_| {
        DecisionValueError::ArithmeticOverflow("decision-value concept count exceeds u128".into())
    })?;
    if n == 0 {
        return Err(DecisionValueError::InvalidValue(
            "decision-value interval requires at least one concept".into(),
        ));
    }
    let radius_square = ceil_div_u128(SQRT_RADIUS_NUMERATOR, n);
    let radius = integer_sqrt_ceil(radius_square)?
        .checked_mul(2)
        .ok_or_else(|| DecisionValueError::ArithmeticOverflow("decision-value Hoeffding radius overflow".into()))?;
    let radius = i128::try_from(radius).map_err(|_| {
        DecisionValueError::ArithmeticOverflow("decision-value Hoeffding radius exceeds i128".into())
    })?;
    Ok(DecisionValueInterval {
        lower_micro_units: centre_lower.checked_sub(radius).ok_or_else(|| {
            DecisionValueError::ArithmeticOverflow("decision-value interval lower bound overflow".into())
        })?,
        upper_micro_units: centre_upper.checked_add(radius).ok_or_else(|| {
            DecisionValueError::ArithmeticOverflow("decision-value interval upper bound overflow".into())
        })?,
        scale: 1_000_000,
        confidence_level_percent: 95,
        method: "pointwise conservative Hoeffding over distinct concept units; paired effects bounded [-1,1]; outward micro-unit rounding".into(),
    })
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

fn integer_sqrt_ceil(value: u128) -> Result<u128, DecisionValueError> {
    let mut low = 0u128;
    let mut high = value.min(u64::MAX as u128).saturating_add(1);
    while low < high {
        let middle = low + (high - low) / 2 + 1;
        if middle <= value / middle {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    if low.checked_mul(low) == Some(value) {
        Ok(low)
    } else {
        low.checked_add(1).ok_or_else(|| {
            DecisionValueError::ArithmeticOverflow("integer square-root ceiling overflow".into())
        })
    }
}

fn apply_holm(
    results: &mut [DomainFamilyDecisionValueInference],
    alpha: Rational,
) -> Result<(), DecisionValueError> {
    let mut order = (0..results.len()).collect::<Vec<_>>();
    order.sort_by(|left, right| {
        results[*left].exact_two_sided_sign_p_value
            .cmp(&results[*right].exact_two_sided_sign_p_value)
            .then_with(|| results[*left].domain_id.cmp(&results[*right].domain_id))
            .then_with(|| results[*left].mechanism_family_id.cmp(&results[*right].mechanism_family_id))
            .then_with(|| results[*left].endpoint.cmp(&results[*right].endpoint))
    });
    let total = u64::try_from(results.len()).map_err(|_| {
        DecisionValueError::ArithmeticOverflow("decision-value hypothesis count exceeds u64".into())
    })?;
    let mut still_rejecting = true;
    for (rank, result_index) in order.into_iter().enumerate() {
        let remaining = total - u64::try_from(rank).map_err(|_| {
            DecisionValueError::ArithmeticOverflow("Holm rank exceeds u64".into())
        })?;
        let threshold = alpha.checked_div(Rational::from_u64(remaining))
            .map_err(DecisionValueError::ArithmeticOverflow)?;
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

/// Analyze verified decision value at the concept level. Every concept in the
/// source hierarchy must appear exactly once; invalid/missing outcomes remain
/// in the intention-to-treat denominator and never become fabricated zeros.
pub fn analyze_scientific_decision_value(
    plan: &DecisionValuePlan,
    hierarchy: &HierarchicalInferenceReceipt,
    evidence: &[ConceptDecisionValueEvidence],
) -> Result<DecisionValueReceipt, DecisionValueError> {
    let causal_status = validate_plan(plan)?;
    let source_concepts = validate_source_hierarchy(plan, hierarchy)?;
    if evidence.len() > MAX_CONCEPT_EVIDENCE || evidence.len() != source_concepts.len() {
        return Err(DecisionValueError::ConceptSetMismatch(
            "decision-value evidence must cover every source concept exactly once within the configured cap".into(),
        ));
    }
    let mut by_concept = BTreeMap::new();
    let mut verified_count = 0usize;
    let mut inconclusive_count = 0usize;
    let mut invalidated_count = 0usize;
    let mut not_measured_count = 0usize;
    let mut baseline_hazard_count = 0usize;
    let mut candidate_hazard_count = 0usize;
    let mut forbidden_transition_count = 0usize;
    let mut false_safety_certification_count = 0usize;
    let mut loss_effects = BTreeMap::new();

    for item in evidence {
        require_id(&item.domain_id, "decision-value domain")?;
        require_id(&item.mechanism_family_id, "decision-value mechanism family")?;
        require_digest(&item.concept_digest, "decision-value concept")?;
        let key = (
            item.domain_id.clone(),
            item.mechanism_family_id.clone(),
            item.concept_digest.clone(),
        );
        if !source_concepts.contains(&key) {
            return Err(DecisionValueError::ConceptSetMismatch(
                "decision-value evidence includes a concept outside the SIM-REL-050 hierarchy".into(),
            ));
        }
        if by_concept.contains_key(&key) {
            return Err(DecisionValueError::DuplicateConcept(
                "decision-value concept evidence must be unique per domain × family × concept".into(),
            ));
        }
        let (valid, _missing) = validate_concept_evidence(plan, item)?;
        // Safety signals are retained for every concept, including those whose
        // efficacy/outcome evidence is inconclusive or invalidated.
        if item.baseline_hazard {
            baseline_hazard_count = baseline_hazard_count.checked_add(1).ok_or_else(|| {
                DecisionValueError::ArithmeticOverflow("baseline hazard concept count overflow".into())
            })?;
        }
        if item.candidate_hazard {
            candidate_hazard_count = candidate_hazard_count.checked_add(1).ok_or_else(|| {
                DecisionValueError::ArithmeticOverflow("candidate hazard concept count overflow".into())
            })?;
        }
        if item.candidate_forbidden_transition {
            forbidden_transition_count = forbidden_transition_count.checked_add(1).ok_or_else(|| {
                DecisionValueError::ArithmeticOverflow("forbidden transition count overflow".into())
            })?;
        }
        false_safety_certification_count = false_safety_certification_count
            .checked_add(item.candidate_false_safety_certifications)
            .ok_or_else(|| DecisionValueError::ArithmeticOverflow(
                "false safety certification count overflow".into()
            ))?;
        match item.evidence_status {
            ConceptEvidenceStatus::Verified => {
                verified_count = verified_count.checked_add(1).ok_or_else(|| {
                    DecisionValueError::ArithmeticOverflow("verified concept count overflow".into())
                })?;
                let effect = decision_loss_effect(item)?;
                loss_effects.insert(key.clone(), effect);
            }
            ConceptEvidenceStatus::Inconclusive => inconclusive_count += 1,
            ConceptEvidenceStatus::Invalidated => invalidated_count += 1,
            ConceptEvidenceStatus::NotMeasured => not_measured_count += 1,
        }
        let _ = valid;
        by_concept.insert(key, item);
    }
    if by_concept.keys().cloned().collect::<BTreeSet<_>>() != source_concepts {
        return Err(DecisionValueError::ConceptSetMismatch(
            "some source hierarchy concepts have no evidence-status row".into(),
        ));
    }

    let safety_gate_passed = candidate_hazard_count == 0
        && forbidden_transition_count == 0
        && false_safety_certification_count == 0;
    let mut all_results = Vec::new();
    for domain in &plan.domains {
        for family in &plan.mechanism_families {
            let stratum_keys = source_concepts.iter()
                .filter(|(d, f, _)| d == domain && f == family)
                .cloned()
                .collect::<Vec<_>>();
            for endpoint in std::iter::once(DecisionValueEndpoint::DecisionLossReduction)
                .chain(plan.outcome_measure.as_ref().map(|_| DecisionValueEndpoint::VerifiedOutcomeImprovement))
            {
                let mut effects = Vec::new();
                for key in &stratum_keys {
                    let item = by_concept.get(key).ok_or_else(|| {
                        DecisionValueError::ConceptSetMismatch("concept evidence row missing".into())
                    })?;
                    if item.evidence_status != ConceptEvidenceStatus::Verified {
                        continue;
                    }
                    effects.push(match endpoint {
                        DecisionValueEndpoint::DecisionLossReduction => decision_loss_effect(item)?,
                        DecisionValueEndpoint::VerifiedOutcomeImprovement => outcome_effect(
                            plan.outcome_measure.as_ref().ok_or_else(|| {
                                DecisionValueError::InvalidPlan("outcome endpoint without outcome measure".into())
                            })?,
                            item,
                        )?,
                    });
                }
                let mut candidate_wins = 0usize;
                let mut reference_wins = 0usize;
                let mut ties = 0usize;
                for effect in &effects {
                    match effect.numerator.cmp(&0) {
                        std::cmp::Ordering::Greater => candidate_wins += 1,
                        std::cmp::Ordering::Less => reference_wins += 1,
                        std::cmp::Ordering::Equal => ties += 1,
                    }
                }
                let p_value = exact_sign_p_value(candidate_wins, reference_wins)?;
                let mean = ExactValue::mean(&effects)?;
                let interval = mean.as_ref().map(|value| mean_interval(value, effects.len())).transpose()?;
                let missing = stratum_keys.len().saturating_sub(effects.len());
                all_results.push(DomainFamilyDecisionValueInference {
                    domain_id: domain.clone(),
                    mechanism_family_id: family.clone(),
                    endpoint,
                    source_concept_count: stratum_keys.len(),
                    measured_concept_count: effects.len(),
                    missing_or_invalid_concept_count: missing,
                    required_valid_concept_count: plan.minimum_valid_concepts_per_stratum,
                    meets_minimum_concept_count: effects.len() >= plan.minimum_valid_concepts_per_stratum,
                    candidate_concept_wins: candidate_wins,
                    reference_concept_wins: reference_wins,
                    tied_concepts: ties,
                    discordant_concepts: candidate_wins + reference_wins,
                    exact_two_sided_sign_p_value: p_value,
                    mean_concept_effect: mean,
                    mean_effect_95_percent_interval: interval,
                    holm_rank: 0,
                    holm_threshold: Rational::zero(),
                    rejected_after_holm: false,
                });
            }
        }
    }
    apply_holm(&mut all_results, plan.family_wise_alpha)?;

    let cost_channels = cost_channel_summaries(
        plan, &plan.domains, &plan.mechanism_families, evidence, &loss_effects
    )?;
    let cost_complete = cost_channels.iter().all(|channel| channel.complete);
    let source_count = source_concepts.len();

    Ok(DecisionValueReceipt {
        analysis_id: plan.analysis_id.clone(),
        analysis_version: plan.analysis_version.clone(),
        source_hierarchical_analysis_id: plan.source_hierarchical_analysis_id.clone(),
        benchmark_id: plan.benchmark_id.clone(),
        benchmark_version: plan.benchmark_version.clone(),
        sealed_release_id: plan.sealed_release_id.clone(),
        sealed_release_digest: plan.sealed_release_digest.clone(),
        campaign_id: plan.campaign_id.clone(),
        scope: plan.scope,
        split: EvidenceSplit::SealedHoldout,
        group_id: plan.group_id.clone(),
        causal_attribution_status: causal_status,
        domain_count: plan.domains.len(),
        mechanism_family_count: plan.mechanism_families.len(),
        endpoint_count: if plan.outcome_measure.is_some() { 2 } else { 1 },
        hypothesis_count: all_results.len(),
        source_concept_count: source_count,
        verified_concept_count: verified_count,
        inconclusive_concept_count: inconclusive_count,
        invalidated_concept_count: invalidated_count,
        not_measured_concept_count: not_measured_count,
        baseline_hazard_concept_count: baseline_hazard_count,
        candidate_hazard_concept_count: candidate_hazard_count,
        candidate_forbidden_transition_count: forbidden_transition_count,
        candidate_false_safety_certification_count: false_safety_certification_count,
        safety_gate_passed,
        evidence_verification_required: true,
        cost_channels,
        domain_family_results: all_results,
        decision_loss_and_outcome_reported_separately: true,
        resource_channels_not_collapsed: true,
        abstention_and_safety_not_traded_for_utility: true,
        missing_cost_prevents_complete_cost_effectiveness: !cost_complete,
        no_causal_claim_from_observational_design: causal_status == CausalAttributionStatus::AssociationOnly,
        concept_level_inference_preserved: true,
        sealed_holdout_only: true,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CausalAttributionStatus, CausalIdentificationDesign, ConceptArmResourceUse,
        ConceptDecisionValueEvidence, ConceptEvidenceStatus, DecisionValueError,
        DecisionValuePlan, ExactValue, OutcomeDirection, OutcomeMeasureDefinition,
        ResourceChannelDefinition, ResourceMeasurement, VerifiedDecisionLoss,
        VerifiedOutcomeMeasurement, analyze_scientific_decision_value,
    };
    use super::super::hierarchical_benchmark_inference::{
        DomainFamilyEndpointInference, ExactHierarchicalEffect,
        HierarchicalConceptEffect, HierarchicalInferenceReceipt,
    };
    use super::super::task_cluster_inference::ClusterInferenceEndpoint;
    use super::super::counterexample_ledger::EvidenceSplit;
    use super::super::matched_benchmark::BenchmarkScope;
    use crate::Rational;

    const D1: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const D2: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const D3: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const D4: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn q(n: u128, d: u128) -> ExactValue {
        ExactValue::new(i128::try_from(n).unwrap(), d).unwrap()
    }

    fn design() -> CausalIdentificationDesign {
        CausalIdentificationDesign::RandomizedPaired {
            design_receipt_digest: D1.into(),
            randomization_receipt_digest: D2.into(),
            allocation_concealment_receipt_digest: D3.into(),
            interference_assessment_digest: D4.into(),
            randomized_assignment_verified: true,
            allocation_concealed: true,
            interference_checked: true,
        }
    }

    fn plan() -> DecisionValuePlan {
        DecisionValuePlan {
            analysis_id: "decision-value".into(),
            analysis_version: "v1".into(),
            pre_registration_receipt_digest: D1.into(),
            decision_loss_table_digest: D2.into(),
            decision_loss_verifier_profile_digest: D3.into(),
            source_hierarchical_analysis_id: "hierarchy".into(),
            benchmark_id: "science-benchmark".into(),
            benchmark_version: "v1".into(),
            sealed_release_id: "release-1".into(),
            sealed_release_digest: D2.into(),
            campaign_id: "campaign".into(),
            group_id: "sealed".into(),
            scope: BenchmarkScope::FullScientificCampaign,
            reference_arm_id: "reference".into(),
            candidate_arm_id: "candidate".into(),
            domains: vec!["materials".into()],
            mechanism_families: vec!["causal".into()],
            minimum_valid_concepts_per_stratum: 2,
            family_wise_alpha: Rational::new(5, 100).unwrap(),
            causal_design: design(),
            outcome_measure: Some(OutcomeMeasureDefinition {
                measure_id: "yield".into(),
                unit: "normalized-yield".into(),
                direction: OutcomeDirection::HigherIsBetter,
                lower_bound: q(0, 1),
                upper_bound: q(1, 1),
                verifier_profile_digest: D3.into(),
            }),
            resource_channels: vec![
                ResourceChannelDefinition {
                    channel_id: "tokens".into(),
                    unit: "tokens".into(),
                },
                ResourceChannelDefinition {
                    channel_id: "energy".into(),
                    unit: "mJ".into(),
                },
            ],
        }
    }

    fn hierarchy() -> HierarchicalInferenceReceipt {
        let concept = |digest: &str| HierarchicalConceptEffect {
            domain_id: "materials".into(),
            endpoint: ClusterInferenceEndpoint::PassAtK,
            mechanism_family_id: "causal".into(),
            concept_digest: digest.into(),
            task_cluster_count: 2,
            valid_task_cluster_count: 2,
            invalid_task_cluster_count: 0,
            recipe_template_count: 1,
            valid_recipe_template_count: 1,
            invalid_recipe_template_count: 0,
            mean_recipe_effect: Some(ExactHierarchicalEffect {
                numerator: 1,
                denominator: 2,
            }),
        };
        HierarchicalInferenceReceipt {
            analysis_id: "hierarchy".into(),
            analysis_version: "v1".into(),
            source_task_cluster_analysis_id: "cluster".into(),
            source_reliability_analysis_id: "reliability".into(),
            pre_registration_receipt_digest: D1.into(),
            campaign_id: "campaign".into(),
            benchmark_id: "science-benchmark".into(),
            benchmark_version: "v1".into(),
            sealed_release_id: "release-1".into(),
            sealed_release_digest: D2.into(),
            scope: BenchmarkScope::FullScientificCampaign,
            split: EvidenceSplit::SealedHoldout,
            group_id: "sealed".into(),
            domain_count: 1,
            mechanism_family_count: 1,
            endpoint_count: 1,
            hypothesis_count: 1,
            task_cluster_count: 4,
            valid_task_cluster_count: 4,
            invalid_task_cluster_count: 0,
            recipe_template_count: 2,
            concept_cluster_count: 2,
            valid_concept_cluster_count: 2,
            inferential_unit: "concept".into(),
            hierarchy: Vec::new(),
            concept_effects: vec![concept(D3), concept(D4)],
            domain_family_endpoint_results: vec![DomainFamilyEndpointInference {
                domain_id: "materials".into(),
                mechanism_family_id: "causal".into(),
                endpoint: ClusterInferenceEndpoint::PassAtK,
                total_concept_clusters: 2,
                valid_concept_clusters: 2,
                concepts_without_valid_recipes: 0,
                required_valid_concept_clusters: 1,
                meets_minimum_concept_count: true,
                candidate_concept_wins: 1,
                reference_concept_wins: 1,
                tied_concepts: 0,
                discordant_concepts: 2,
                exact_two_sided_sign_p_value: Rational::one(),
                mean_concept_effect: Some(ExactHierarchicalEffect { numerator: 0, denominator: 1 }),
                mean_effect_95_percent_interval: None,
                holm_rank: 1,
                holm_threshold: Rational::new(5, 100).unwrap(),
                rejected_after_holm: false,
            }],
            runs_collapsed_within_task: true,
            tasks_equal_weighted_within_recipe: true,
            recipes_equal_weighted_within_concept: true,
            one_concept_one_effect_per_hypothesis: true,
            domains_reported_as_fixed_strata: true,
            pointwise_intervals_not_simultaneous: true,
            abstention_not_given_unregistered_direction: true,
            sealed_holdout_only: true,
        }
    }

    fn hierarchy_with_empty_domain() -> HierarchicalInferenceReceipt {
        let mut source = hierarchy();
        source.domain_count = 2;
        source.hypothesis_count = 2;
        source.domain_family_endpoint_results.push(DomainFamilyEndpointInference {
            domain_id: "biology".into(),
            mechanism_family_id: "causal".into(),
            endpoint: ClusterInferenceEndpoint::PassAtK,
            total_concept_clusters: 0,
            valid_concept_clusters: 0,
            concepts_without_valid_recipes: 0,
            required_valid_concept_clusters: 1,
            meets_minimum_concept_count: false,
            candidate_concept_wins: 0,
            reference_concept_wins: 0,
            tied_concepts: 0,
            discordant_concepts: 0,
            exact_two_sided_sign_p_value: Rational::one(),
            mean_concept_effect: None,
            mean_effect_95_percent_interval: None,
            holm_rank: 2,
            holm_threshold: Rational::new(1, 100).unwrap(),
            rejected_after_holm: false,
        });
        source
    }

    fn plan_with_empty_domain() -> DecisionValuePlan {
        let mut result = plan();
        result.domains.push("biology".into());
        result
    }

    fn loss(value: u128, suffix: &str) -> VerifiedDecisionLoss {
        VerifiedDecisionLoss {
            normalized_loss: q(value, 10),
            decision_loss_evidence_digest: D1.into(),
            independent_loss_verification_receipt_digest: D3.into(),
            loss_verifier_profile_digest: D3.into(),
            oracle_loss_table_digest: D2.into(),
            selected_decision_digest: format!("sha256:{}", suffix.repeat(64 / suffix.len())),
            oracle_optimal_decision_digest: D4.into(),
        }
    }

    fn outcome(value: u128) -> VerifiedOutcomeMeasurement {
        VerifiedOutcomeMeasurement {
            value: q(value, 10),
            raw_observation_digest: D1.into(),
            independent_verification_receipt_digest: D2.into(),
            verifier_profile_digest: D3.into(),
            calibration_chain_digest: D3.into(),
        }
    }

    fn cost(amount: u128, digest: &str) -> ResourceMeasurement {
        ResourceMeasurement {
            channel_id: "tokens".into(),
            amount: q(amount, 1),
            complete: true,
            meter_receipt_digest: Some(digest.into()),
        }
    }

    fn cost_use(tokens: u128, energy: u128) -> ConceptArmResourceUse {
        ConceptArmResourceUse {
            measurements: vec![
                cost(tokens, D1),
                ResourceMeasurement {
                    channel_id: "energy".into(),
                    amount: q(energy, 1),
                    complete: true,
                    meter_receipt_digest: Some(D2.into()),
                },
            ],
        }
    }

    fn evidence(concept: &str, baseline_loss: u128, candidate_loss: u128) -> ConceptDecisionValueEvidence {
        ConceptDecisionValueEvidence {
            domain_id: "materials".into(),
            mechanism_family_id: "causal".into(),
            concept_digest: concept.into(),
            evidence_status: ConceptEvidenceStatus::Verified,
            baseline_loss: Some(loss(baseline_loss, "e")),
            candidate_loss: Some(loss(candidate_loss, "f")),
            baseline_outcome: Some(outcome(4)),
            candidate_outcome: Some(outcome(6)),
            baseline_resource_use: cost_use(100, 10),
            candidate_resource_use: cost_use(120, 12),
            baseline_hazard: false,
            candidate_hazard: false,
            candidate_forbidden_transition: false,
            candidate_false_safety_certifications: 0,
            safety_review_receipt_digest: D4.into(),
            concept_evidence_receipt_digest: D1.into(),
        }
    }

    #[test]
    fn randomized_verified_decision_value_is_separate_from_outcome_and_cost() {
        let receipt = analyze_scientific_decision_value(
            &plan(),
            &hierarchy(),
            &[evidence(D3, 8, 4), evidence(D4, 7, 3)],
        ).unwrap();
        assert_eq!(receipt.causal_attribution_status, CausalAttributionStatus::CausalIdentifiedUnderDeclaredDesign);
        assert_eq!(receipt.endpoint_count, 2);
        assert_eq!(receipt.verified_concept_count, 2);
        assert!(receipt.safety_gate_passed);
        assert!(!receipt.missing_cost_prevents_complete_cost_effectiveness);
        assert_eq!(receipt.cost_channels.len(), 2);
        assert_eq!(receipt.baseline_hazard_concept_count, 0);
        assert_eq!(receipt.cost_channels[0].domain_id, "materials");
        assert_eq!(receipt.cost_channels[0].mechanism_family_id, "causal");
        let loss = receipt.domain_family_results.iter()
            .find(|row| row.endpoint == super::DecisionValueEndpoint::DecisionLossReduction)
            .unwrap();
        assert_eq!(loss.mean_concept_effect, Some(q(4, 10)));
        let outcome = receipt.domain_family_results.iter()
            .find(|row| row.endpoint == super::DecisionValueEndpoint::VerifiedOutcomeImprovement)
            .unwrap();
        assert_eq!(outcome.mean_concept_effect, Some(q(2, 10)));
        let tokens = receipt.cost_channels.iter().find(|row| row.channel_id == "tokens").unwrap();
        assert_eq!(tokens.mean_incremental_cost_per_concept, Some(q(20, 1)));
        assert_eq!(tokens.incremental_cost_per_decision_loss_unit, Some(q(50, 1)));
    }

    #[test]
    fn preregistered_empty_domain_remains_in_results_and_cost_grid() {
        let receipt = analyze_scientific_decision_value(
            &plan_with_empty_domain(),
            &hierarchy_with_empty_domain(),
            &[evidence(D3, 8, 4), evidence(D4, 7, 3)],
        ).unwrap();
        assert_eq!(receipt.domain_count, 2);
        let empty = receipt.domain_family_results.iter()
            .find(|row| row.domain_id == "biology"
                && row.endpoint == super::DecisionValueEndpoint::DecisionLossReduction)
            .unwrap();
        assert_eq!(empty.source_concept_count, 0);
        assert_eq!(empty.measured_concept_count, 0);
        assert_eq!(empty.exact_two_sided_sign_p_value, Rational::one());
        assert!(!empty.meets_minimum_concept_count);
        let empty_cost = receipt.cost_channels.iter()
            .find(|row| row.domain_id == "biology" && row.channel_id == "tokens")
            .unwrap();
        assert!(!empty_cost.complete);
    }

    #[test]
    fn safety_violations_block_positive_safety_gate_even_with_better_loss() {
        let mut candidate = evidence(D3, 8, 2);
        candidate.candidate_hazard = true;
        let receipt = analyze_scientific_decision_value(&plan(), &hierarchy(), &[candidate, evidence(D4, 8, 2)]).unwrap();
        assert!(!receipt.safety_gate_passed);
        assert_eq!(receipt.candidate_hazard_concept_count, 1);
    }

    #[test]
    fn observational_evidence_is_association_only_not_causal() {
        let mut plan = plan();
        plan.causal_design = CausalIdentificationDesign::Observational {
            design_receipt_digest: D1.into(),
            confounding_assessment_digest: D2.into(),
        };
        let receipt = analyze_scientific_decision_value(&plan, &hierarchy(), &[evidence(D3, 8, 4), evidence(D4, 7, 3)]).unwrap();
        assert_eq!(receipt.causal_attribution_status, CausalAttributionStatus::AssociationOnly);
        assert!(receipt.no_causal_claim_from_observational_design);
    }

    #[test]
    fn missing_resource_channel_prevents_complete_cost_effectiveness() {
        let mut item = evidence(D3, 8, 4);
        item.candidate_resource_use.measurements.retain(|m| m.channel_id != "energy");
        let receipt = analyze_scientific_decision_value(&plan(), &hierarchy(), &[item, evidence(D4, 7, 3)]).unwrap();
        assert!(receipt.missing_cost_prevents_complete_cost_effectiveness);
        let energy = receipt.cost_channels.iter().find(|row| row.channel_id == "energy").unwrap();
        assert!(!energy.complete);
        assert!(energy.candidate_total.is_none());
    }

    #[test]
    fn more_outputs_or_unverified_evidence_cannot_create_a_verified_effect() {
        let mut item = evidence(D3, 8, 4);
        item.evidence_status = ConceptEvidenceStatus::Inconclusive;
        item.baseline_loss = None;
        item.candidate_loss = None;
        item.baseline_outcome = None;
        item.candidate_outcome = None;
        let receipt = analyze_scientific_decision_value(&plan(), &hierarchy(), &[item, evidence(D4, 7, 3)]).unwrap();
        assert_eq!(receipt.verified_concept_count, 1);
        assert_eq!(receipt.inconclusive_concept_count, 1);
        let loss = receipt.domain_family_results.iter()
            .find(|row| row.endpoint == super::DecisionValueEndpoint::DecisionLossReduction)
            .unwrap();
        assert_eq!(loss.source_concept_count, 2);
        assert_eq!(loss.measured_concept_count, 1);
        assert_eq!(loss.missing_or_invalid_concept_count, 1);
    }

    #[test]
    fn duplicate_or_out_of_hierarchy_concepts_reject() {
        let duplicate = vec![evidence(D3, 8, 4), evidence(D3, 7, 3)];
        assert!(matches!(
            analyze_scientific_decision_value(&plan(), &hierarchy(), &duplicate),
            Err(DecisionValueError::DuplicateConcept(_))
        ));
        let outside = vec![evidence(D3, 8, 4), evidence("sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee", 7, 3)];
        assert!(matches!(
            analyze_scientific_decision_value(&plan(), &hierarchy(), &outside),
            Err(DecisionValueError::ConceptSetMismatch(_))
        ));
    }

    #[test]
    fn safety_violations_are_retained_even_when_efficiency_evidence_is_inconclusive() {
        let mut item = evidence(D3, 8, 4);
        item.evidence_status = ConceptEvidenceStatus::Inconclusive;
        item.baseline_loss = None;
        item.candidate_loss = None;
        item.baseline_outcome = None;
        item.candidate_outcome = None;
        item.candidate_hazard = true;
        item.candidate_false_safety_certifications = 1;
        let receipt = analyze_scientific_decision_value(
            &plan(),
            &hierarchy(),
            &[item, evidence(D4, 7, 3)],
        ).unwrap();
        assert!(!receipt.safety_gate_passed);
        assert_eq!(receipt.candidate_hazard_concept_count, 1);
        assert_eq!(receipt.candidate_false_safety_certification_count, 1);
    }

    #[test]
    fn decision_loss_evidence_must_use_preregistered_verifier_profile() {
        let mut item = evidence(D3, 8, 4);
        item.candidate_loss.as_mut().unwrap().loss_verifier_profile_digest = D4.into();
        assert!(matches!(
            analyze_scientific_decision_value(&plan(), &hierarchy(), &[item, evidence(D4, 7, 3)]),
            Err(DecisionValueError::MissingOutcomeEvidence(_))
        ));
    }

    #[test]
    fn unverified_resource_meter_receipt_rejects_complete_cost_claim() {
        let mut item = evidence(D3, 8, 4);
        item.candidate_resource_use.measurements[0].meter_receipt_digest = None;
        assert!(matches!(
            analyze_scientific_decision_value(&plan(), &hierarchy(), &[item, evidence(D4, 7, 3)]),
            Err(DecisionValueError::ResourceAccounting(_))
        ));
    }
}
