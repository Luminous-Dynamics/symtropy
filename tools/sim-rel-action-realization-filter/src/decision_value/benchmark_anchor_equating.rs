//! Protected-anchor common-panel linking/equating for dynamic benchmark forms.
//!
//! A fixed reference-agent panel takes both benchmark forms. Exact common
//! anchor tasks test item stability; full-form reference-panel score
//! distributions estimate a linear mean/sigma transformation. Different
//! blueprints may be linked for exploratory comparison, but are not treated as
//! interchangeable forms. Anchor reuse is budgeted to reduce contamination.

use super::matched_benchmark::ResourceBudget;
use std::collections::{BTreeMap, BTreeSet};

const MAX_ID_BYTES: usize = 256;
const MAX_ANCHORS: usize = 2_000;
const MAX_PANEL_AGENTS: usize = 2_000;
const MAX_RESPONSE_ROWS: usize = 2_000_000;
const MICRO_SCALE: i128 = 1_000_000;
const SQRT_RADIUS_NUMERATOR: u128 = 2_000_000_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactLinkRatio {
    pub numerator: i128,
    pub denominator: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkFormIdentity {
    pub form_id: String,
    pub benchmark_id: String,
    pub benchmark_version: String,
    pub release_id: String,
    pub release_digest: String,
    pub construct_family_digest: String,
    pub construct_blueprint_digest: String,
    pub score_semantics_digest: String,
    pub harness_digest: String,
    pub toolset_digest: String,
    pub reference_panel_digest: String,
    pub resource_budget: ResourceBudget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorTaskIdentity {
    pub anchor_id: String,
    pub task_public_digest: String,
    pub sealed_oracle_commitment_digest: String,
    pub oracle_verification_receipt_digest: String,
    pub environment_digest: String,
    pub domain_id: String,
    pub mechanism_family_id: String,
    pub concept_digest: String,
    pub recipe_template_digest: String,
    pub difficulty_band: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorBankItem {
    pub identity: AnchorTaskIdentity,
    pub current_reuse_count: usize,
    pub maximum_reuse_count: usize,
    /// Previous release IDs that exposed this anchor to a reference/candidate
    /// panel. The current link adds exactly two form exposures.
    pub previously_exposed_release_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorBankSnapshot {
    pub bank_id: String,
    pub bank_version: String,
    pub snapshot_digest: String,
    pub reuse_policy_digest: String,
    pub items: Vec<AnchorBankItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorStratumMinimum {
    pub domain_id: String,
    pub mechanism_family_id: String,
    pub minimum_anchor_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceAgentStratumMinimum {
    pub agent_stratum_id: String,
    pub minimum_agent_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnchorLinkMethod {
    LinearMeanSigmaCommonPerson,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorLinkingPlan {
    pub link_id: String,
    pub plan_version: String,
    pub benchmark_id: String,
    pub older_form_id: String,
    pub newer_form_id: String,
    pub anchor_bank_id: String,
    pub anchor_bank_version: String,
    pub anchor_bank_snapshot_digest: String,
    pub anchor_reuse_policy_digest: String,
    pub selected_anchor_ids: Vec<String>,
    pub required_anchor_strata: Vec<AnchorStratumMinimum>,
    pub required_difficulty_bands: Vec<String>,
    pub required_agent_strata: Vec<ReferenceAgentStratumMinimum>,
    pub reference_panel_digest: String,
    pub minimum_reference_agents: usize,
    pub repetitions_per_anchor_agent: usize,
    pub maximum_anchor_reuse_count: usize,
    pub maximum_per_anchor_response_disagreement: ExactLinkRatio,
    pub method: AnchorLinkMethod,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorResponseObservation {
    pub form_id: String,
    pub anchor_id: String,
    pub agent_id: String,
    pub agent_configuration_digest: String,
    pub agent_stratum_id: String,
    pub repetition_index: usize,
    pub seed_commitment_digest: String,
    pub observation_receipt_digest: String,
    pub valid: bool,
    pub safe_success: bool,
    pub any_hazard: bool,
    pub abstained: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceAgentFormScore {
    pub form_id: String,
    pub agent_id: String,
    pub agent_configuration_digest: String,
    pub agent_stratum_id: String,
    pub reference_panel_digest: String,
    pub safe_success_count: usize,
    pub total_scored_attempts: usize,
    pub score_evidence_digest: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnchorEquatingStatus {
    Equated,
    LinkedOnly,
    NotComparable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorItemDrift {
    pub anchor_id: String,
    pub paired_response_count: usize,
    pub valid_paired_response_count: usize,
    pub invalid_paired_response_count: usize,
    pub any_outcome_disagreement_count: usize,
    pub safe_success_disagreement_count: usize,
    pub hazard_disagreement_count: usize,
    pub abstention_disagreement_count: usize,
    pub any_outcome_disagreement_rate: Option<ExactLinkRatio>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeanSigmaScoreTransform {
    /// Convert a score y on the newer form to the older form's reference scale:
    /// old_mean + slope * (y - new_mean).
    pub older_reference_panel_mean: ExactLinkRatio,
    pub newer_reference_panel_mean: ExactLinkRatio,
    pub slope_lower_micro: i128,
    pub slope_upper_micro: i128,
    pub paired_mean_difference: ExactLinkRatio,
    pub paired_mean_difference_95_percent_interval: LinkMeanInterval,
    pub fixed_point_scale: u128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinkMeanInterval {
    pub lower_micro_units: i128,
    pub upper_micro_units: i128,
    pub confidence_level_percent: u8,
    pub method: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorEquatingReceipt {
    pub link_id: String,
    pub benchmark_id: String,
    pub older_form_id: String,
    pub newer_form_id: String,
    pub older_release_digest: String,
    pub newer_release_digest: String,
    pub anchor_bank_snapshot_digest: String,
    pub reference_panel_digest: String,
    pub method: AnchorLinkMethod,
    pub status: AnchorEquatingStatus,
    pub reason_code: String,
    pub selected_anchor_count: usize,
    pub required_anchor_count: usize,
    pub reference_agent_count: usize,
    pub paired_anchor_response_count: usize,
    pub valid_paired_anchor_response_count: usize,
    pub invalid_paired_anchor_response_count: usize,
    pub maximum_observed_anchor_disagreement: Option<ExactLinkRatio>,
    pub mean_observed_anchor_disagreement: Option<ExactLinkRatio>,
    pub anchor_item_drifts: Vec<AnchorItemDrift>,
    pub older_form_reference_scores: Vec<ReferenceAgentFormScore>,
    pub newer_form_reference_scores: Vec<ReferenceAgentFormScore>,
    pub score_transform: Option<MeanSigmaScoreTransform>,
    pub score_transfer_permitted: bool,
    pub anchor_reuse_count_after_link: usize,
    pub anchor_reuse_limit: usize,
    pub same_construct_blueprint: bool,
    pub same_score_semantics: bool,
    pub common_reference_panel: bool,
    pub anchor_identity_preserved: bool,
    pub anchor_reuse_budget_respected: bool,
    pub drift_gate_passed: bool,
    pub common_person_design_complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AnchorEquatingError {
    InvalidIdentity(String),
    InvalidDigest(String),
    InvalidPlan(String),
    InvalidRatio(String),
    AnchorBankMismatch(String),
    AnchorSelection(String),
    AnchorCoverage(String),
    AnchorExposure(String),
    DuplicateAnchor(String),
    AnchorResponse(String),
    ReferencePanel(String),
    FormScore(String),
    ArithmeticOverflow(String),
    UnsupportedSampleSize(String),
}

impl std::fmt::Display for AnchorEquatingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidIdentity(reason)
            | Self::InvalidDigest(reason)
            | Self::InvalidPlan(reason)
            | Self::InvalidRatio(reason)
            | Self::AnchorBankMismatch(reason)
            | Self::AnchorSelection(reason)
            | Self::AnchorCoverage(reason)
            | Self::AnchorExposure(reason)
            | Self::DuplicateAnchor(reason)
            | Self::AnchorResponse(reason)
            | Self::ReferencePanel(reason)
            | Self::FormScore(reason)
            | Self::ArithmeticOverflow(reason)
            | Self::UnsupportedSampleSize(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for AnchorEquatingError {}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn require_id(value: &str, label: &str) -> Result<(), AnchorEquatingError> {
    if !valid_id(value) {
        return Err(AnchorEquatingError::InvalidIdentity(format!(
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

fn require_digest(value: &str, label: &str) -> Result<(), AnchorEquatingError> {
    if !valid_digest(value) {
        return Err(AnchorEquatingError::InvalidDigest(format!(
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

impl ExactLinkRatio {
    fn new(numerator: i128, denominator: u128) -> Result<Self, AnchorEquatingError> {
        if denominator == 0 {
            return Err(AnchorEquatingError::InvalidRatio(
                "ratio denominator must be positive".into(),
            ));
        }
        let divisor = gcd_u128(numerator.unsigned_abs(), denominator);
        let divisor_i128 = i128::try_from(divisor).map_err(|_| {
            AnchorEquatingError::ArithmeticOverflow("ratio reduction divisor exceeds i128".into())
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

    fn checked_add(&self, other: &Self) -> Result<Self, AnchorEquatingError> {
        let common = gcd_u128(self.denominator, other.denominator);
        let left_factor = other.denominator / common;
        let right_factor = self.denominator / common;
        let left = self.numerator.checked_mul(i128::try_from(left_factor).map_err(|_| {
            AnchorEquatingError::ArithmeticOverflow("ratio factor exceeds i128".into())
        })?).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("ratio numerator overflow".into())
        })?;
        let right = other.numerator.checked_mul(i128::try_from(right_factor).map_err(|_| {
            AnchorEquatingError::ArithmeticOverflow("ratio factor exceeds i128".into())
        })?).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("ratio numerator overflow".into())
        })?;
        let numerator = left.checked_add(right).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("ratio addition overflow".into())
        })?;
        let denominator = self.denominator.checked_mul(left_factor).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("ratio denominator overflow".into())
        })?;
        Self::new(numerator, denominator)
    }

    fn checked_sub(&self, other: &Self) -> Result<Self, AnchorEquatingError> {
        let negated = other.numerator.checked_neg().ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("ratio negation overflow".into())
        })?;
        self.checked_add(&Self::new(negated, other.denominator)?)
    }

    fn checked_mul(&self, other: &Self) -> Result<Self, AnchorEquatingError> {
        let numerator = self.numerator.checked_mul(other.numerator).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("ratio multiplication overflow".into())
        })?;
        let denominator = self.denominator.checked_mul(other.denominator).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("ratio denominator overflow".into())
        })?;
        Self::new(numerator, denominator)
    }

    fn checked_div(&self, other: &Self) -> Result<Self, AnchorEquatingError> {
        if other.numerator == 0 {
            return Err(AnchorEquatingError::InvalidRatio(
                "division by zero ratio".into(),
            ));
        }
        let sign = if other.numerator < 0 { -1i128 } else { 1i128 };
        let numerator = self.numerator
            .checked_mul(i128::try_from(other.denominator).map_err(|_| {
                AnchorEquatingError::ArithmeticOverflow("ratio denominator exceeds i128".into())
            })?)
            .and_then(|value| value.checked_mul(sign))
            .ok_or_else(|| AnchorEquatingError::ArithmeticOverflow("ratio division overflow".into()))?;
        let denominator = self.denominator
            .checked_mul(other.numerator.unsigned_abs())
            .ok_or_else(|| AnchorEquatingError::ArithmeticOverflow("ratio denominator overflow".into()))?;
        Self::new(numerator, denominator)
    }

    fn checked_square(&self) -> Result<Self, AnchorEquatingError> {
        self.checked_mul(self)
    }

    fn mean(values: &[Self]) -> Result<Self, AnchorEquatingError> {
        if values.is_empty() {
            return Err(AnchorEquatingError::InvalidRatio(
                "mean requires at least one observation".into(),
            ));
        }
        let mut sum = Self::zero();
        for value in values {
            sum = sum.checked_add(value)?;
        }
        sum.checked_div(&Self::new(
            i128::try_from(values.len()).map_err(|_| {
                AnchorEquatingError::ArithmeticOverflow("ratio count exceeds i128".into())
            })?,
            1,
        )?)
    }

    fn variance(values: &[Self]) -> Result<Self, AnchorEquatingError> {
        let mean = Self::mean(values)?;
        let squares = values.iter()
            .map(Self::checked_square)
            .collect::<Result<Vec<_>, _>>()?;
        let mean_square = Self::mean(&squares)?;
        mean_square.checked_sub(&mean.checked_square()?)
    }

    fn checked_square_alias(&self) -> Result<Self, AnchorEquatingError> {
        self.checked_square()
    }

    fn compare(&self, other: &Self) -> Result<std::cmp::Ordering, AnchorEquatingError> {
        let left = self.numerator.checked_mul(i128::try_from(other.denominator).map_err(|_| {
            AnchorEquatingError::ArithmeticOverflow("ratio comparison denominator exceeds i128".into())
        })?).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("ratio comparison overflow".into())
        })?;
        let right = other.numerator.checked_mul(i128::try_from(self.denominator).map_err(|_| {
            AnchorEquatingError::ArithmeticOverflow("ratio comparison denominator exceeds i128".into())
        })?).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("ratio comparison overflow".into())
        })?;
        Ok(left.cmp(&right))
    }
}

impl ExactLinkRatio {
    fn checked_square(&self) -> Result<Self, AnchorEquatingError> {
        self.checked_mul(self)
    }
}

fn exact_mean_interval(
    mean: &ExactLinkRatio,
    sample_count: usize,
) -> Result<LinkMeanInterval, AnchorEquatingError> {
    let denominator = i128::try_from(mean.denominator).map_err(|_| {
        AnchorEquatingError::ArithmeticOverflow("mean denominator exceeds i128".into())
    })?;
    let scaled = mean.numerator.checked_mul(MICRO_SCALE).ok_or_else(|| {
        AnchorEquatingError::ArithmeticOverflow("mean micro-scaling overflow".into())
    })?;
    let centre_lower = floor_div_signed(scaled, denominator);
    let centre_upper = ceil_div_signed(scaled, denominator);
    let n = u128::try_from(sample_count).map_err(|_| {
        AnchorEquatingError::ArithmeticOverflow("sample count exceeds u128".into())
    })?;
    if n == 0 {
        return Err(AnchorEquatingError::InvalidRatio(
            "mean interval requires a non-empty sample".into(),
        ));
    }
    let radius_square = ceil_div_u128(SQRT_RADIUS_NUMERATOR, n);
    let radius = integer_sqrt_ceil(radius_square)?
        .checked_mul(2)
        .ok_or_else(|| AnchorEquatingError::ArithmeticOverflow("interval radius overflow".into()))?;
    let radius = i128::try_from(radius).map_err(|_| {
        AnchorEquatingError::ArithmeticOverflow("interval radius exceeds i128".into())
    })?;
    Ok(LinkMeanInterval {
        lower_micro_units: centre_lower.checked_sub(radius).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("mean interval lower bound overflow".into())
        })?,
        upper_micro_units: centre_upper.checked_add(radius).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("mean interval upper bound overflow".into())
        })?,
        confidence_level_percent: 95,
        method: "pointwise conservative Hoeffding interval over paired reference-agent scores; bounded score difference [-1,1]".into(),
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

fn integer_sqrt_floor(value: u128) -> Result<u128, AnchorEquatingError> {
    let mut low = 0u128;
    let mut high = 2_000_001u128;
    while low < high {
        let middle = low + (high - low) / 2;
        let square = middle.checked_mul(middle).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("integer square overflow".into())
        })?;
        if square <= value {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    Ok(low.saturating_sub(1))
}

fn integer_sqrt_ceil(value: u128) -> Result<u128, AnchorEquatingError> {
    let floor = integer_sqrt_floor(value)?;
    let square = floor.checked_mul(floor).ok_or_else(|| {
        AnchorEquatingError::ArithmeticOverflow("integer square overflow".into())
    })?;
    if square == value {
        Ok(floor)
    } else {
        floor.checked_add(1).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("integer square-root ceiling overflow".into())
        })
    }
}

fn validate_form(
    form: &BenchmarkFormIdentity,
    label: &str,
) -> Result<(), AnchorEquatingError> {
    for (value, field) in [
        (&form.form_id, "form ID"),
        (&form.benchmark_id, "benchmark ID"),
        (&form.benchmark_version, "benchmark version"),
        (&form.release_id, "release ID"),
    ] {
        require_id(value, &format!("{label} {field}"))?;
    }
    for (value, field) in [
        (&form.release_digest, "release"),
        (&form.construct_family_digest, "construct family"),
        (&form.construct_blueprint_digest, "construct blueprint"),
        (&form.score_semantics_digest, "score semantics"),
        (&form.harness_digest, "harness"),
        (&form.toolset_digest, "toolset"),
        (&form.reference_panel_digest, "reference panel"),
    ] {
        require_digest(value, &format!("{label} {field}"))?;
    }
    Ok(())
}

fn validate_plan(plan: &AnchorLinkingPlan) -> Result<(), AnchorEquatingError> {
    for (value, label) in [
        (&plan.link_id, "link ID"),
        (&plan.plan_version, "link plan version"),
        (&plan.benchmark_id, "benchmark ID"),
        (&plan.older_form_id, "older form ID"),
        (&plan.newer_form_id, "newer form ID"),
        (&plan.anchor_bank_id, "anchor bank ID"),
        (&plan.anchor_bank_version, "anchor bank version"),
    ] {
        require_id(value, label)?;
    }
    for (value, label) in [
        (&plan.anchor_bank_snapshot_digest, "anchor bank snapshot"),
        (&plan.anchor_reuse_policy_digest, "anchor reuse policy"),
        (&plan.reference_panel_digest, "reference panel"),
    ] {
        require_digest(value, label)?;
    }
    if plan.older_form_id == plan.newer_form_id
        || plan.selected_anchor_ids.is_empty()
        || plan.selected_anchor_ids.len() > MAX_ANCHORS
        || plan.minimum_reference_agents < 2
        || plan.minimum_reference_agents > MAX_PANEL_AGENTS
        || plan.repetitions_per_anchor_agent == 0
        || plan.maximum_anchor_reuse_count < 2
        || plan.required_anchor_strata.is_empty()
        || plan.required_agent_strata.is_empty()
    {
        return Err(AnchorEquatingError::InvalidPlan(
            "link plan requires distinct forms, bounded non-empty anchors/strata, at least two reference agents and positive repetition/reuse limits".into(),
        ));
    }
    if plan.maximum_per_anchor_response_disagreement.denominator == 0
        || plan.maximum_per_anchor_response_disagreement.numerator < 0
        || plan.maximum_per_anchor_response_disagreement.compare(&ExactLinkRatio::new(1, 1)?)?
            == std::cmp::Ordering::Greater
    {
        return Err(AnchorEquatingError::InvalidPlan(
            "maximum anchor disagreement must be in [0,1]".into(),
        ));
    }
    let mut ids = BTreeSet::new();
    for anchor_id in &plan.selected_anchor_ids {
        require_id(anchor_id, "selected anchor ID")?;
        if !ids.insert(anchor_id.clone()) {
            return Err(AnchorEquatingError::InvalidPlan(
                "selected anchor IDs must be unique".into(),
            ));
        }
    }
    let mut strata = BTreeSet::new();
    for requirement in &plan.required_anchor_strata {
        require_id(&requirement.domain_id, "anchor domain stratum")?;
        require_id(&requirement.mechanism_family_id, "anchor mechanism-family stratum")?;
        if requirement.minimum_anchor_count == 0
            || !strata.insert((requirement.domain_id.clone(), requirement.mechanism_family_id.clone()))
        {
            return Err(AnchorEquatingError::InvalidPlan(
                "anchor strata must be unique and require a positive anchor count".into(),
            ));
        }
    }
    let mut agent_strata = BTreeSet::new();
    for requirement in &plan.required_agent_strata {
        require_id(&requirement.agent_stratum_id, "reference agent stratum")?;
        if requirement.minimum_agent_count == 0 || !agent_strata.insert(requirement.agent_stratum_id.clone()) {
            return Err(AnchorEquatingError::InvalidPlan(
                "reference-agent strata must be unique and require a positive agent count".into(),
            ));
        }
    }
    if plan.required_difficulty_bands.is_empty() {
        return Err(AnchorEquatingError::InvalidPlan(
            "difficulty coverage requirements must be non-empty".into(),
        ));
    }
    let mut difficulty_bands = BTreeSet::new();
    for band in &plan.required_difficulty_bands {
        require_id(band, "difficulty band")?;
        if !difficulty_bands.insert(band.clone()) {
            return Err(AnchorEquatingError::InvalidPlan(
                "difficulty bands must be unique".into(),
            ));
        }
    }
    Ok(())
}

fn validate_anchor_identity(
    anchor: &AnchorTaskIdentity,
    label: &str,
) -> Result<(), AnchorEquatingError> {
    for (value, field) in [
        (&anchor.anchor_id, "anchor ID"),
        (&anchor.domain_id, "domain ID"),
        (&anchor.mechanism_family_id, "mechanism-family ID"),
        (&anchor.difficulty_band, "difficulty band"),
    ] {
        require_id(value, &format!("{label} {field}"))?;
    }
    for (value, field) in [
        (&anchor.task_public_digest, "public task"),
        (&anchor.sealed_oracle_commitment_digest, "oracle commitment"),
        (&anchor.oracle_verification_receipt_digest, "oracle verification receipt"),
        (&anchor.environment_digest, "anchor environment"),
        (&anchor.concept_digest, "concept"),
        (&anchor.recipe_template_digest, "recipe template"),
    ] {
        require_digest(value, &format!("{label} {field}"))?;
    }
    Ok(())
}

fn response_key(
    response: &AnchorResponseObservation,
) -> (String, String, String, usize) {
    (
        response.anchor_id.clone(),
        response.agent_id.clone(),
        response.agent_configuration_digest.clone(),
        response.repetition_index,
    )
}

fn score_map(
    form: &BenchmarkFormIdentity,
    scores: &[ReferenceAgentFormScore],
    plan: &AnchorLinkingPlan,
) -> Result<BTreeMap<String, ExactLinkRatio>, AnchorEquatingError> {
    let mut values = BTreeMap::new();
    let mut agent_strata = BTreeMap::<String, usize>::new();
    for score in scores {
        require_id(&score.agent_id, "reference agent ID")?;
        require_id(&score.agent_stratum_id, "reference agent stratum")?;
        require_digest(&score.agent_configuration_digest, "reference agent configuration")?;
        require_digest(&score.reference_panel_digest, "reference panel digest")?;
        require_digest(&score.score_evidence_digest, "reference score evidence")?;
        if score.form_id != form.form_id
            || score.reference_panel_digest != plan.reference_panel_digest
            || score.total_scored_attempts == 0
            || score.safe_success_count > score.total_scored_attempts
            || values.contains_key(&score.agent_id)
        {
            return Err(AnchorEquatingError::FormScore(
                "form score has an invalid form/panel identity, denominator, count or duplicate agent".into(),
            ));
        }
        values.insert(
            score.agent_id.clone(),
            ExactLinkRatio::new(
                i128::try_from(score.safe_success_count).map_err(|_| {
                    AnchorEquatingError::ArithmeticOverflow("safe-success count exceeds i128".into())
                })?,
                u128::try_from(score.total_scored_attempts).map_err(|_| {
                    AnchorEquatingError::ArithmeticOverflow("attempt count exceeds u128".into())
                })?,
            )?,
        );
        *agent_strata.entry(score.agent_stratum_id.clone()).or_default() += 1;
    }
    if values.len() < plan.minimum_reference_agents || values.len() > MAX_PANEL_AGENTS {
        return Err(AnchorEquatingError::ReferencePanel(
            "reference panel is below the minimum or above the configured cap".into(),
        ));
    }
    for requirement in &plan.required_agent_strata {
        if agent_strata.get(&requirement.agent_stratum_id).copied().unwrap_or(0)
            < requirement.minimum_agent_count
        {
            return Err(AnchorEquatingError::ReferencePanel(format!(
                "reference panel misses minimum agent coverage for stratum {}",
                requirement.agent_stratum_id
            )));
        }
    }
    Ok(values)
}

fn validate_anchor_responses(
    plan: &AnchorLinkingPlan,
    old_form: &BenchmarkFormIdentity,
    new_form: &BenchmarkFormIdentity,
    old_responses: &[AnchorResponseObservation],
    new_responses: &[AnchorResponseObservation],
    anchor_ids: &BTreeSet<String>,
    score_agent_ids: &BTreeSet<String>,
) -> Result<(Vec<AnchorItemDrift>, usize, usize, usize, ExactLinkRatio, ExactLinkRatio, bool), AnchorEquatingError> {
    if old_responses.len() > MAX_RESPONSE_ROWS || new_responses.len() > MAX_RESPONSE_ROWS {
        return Err(AnchorEquatingError::UnsupportedSampleSize(
            "anchor response rows exceed configured cap".into(),
        ));
    }
    let mut old_map = BTreeMap::new();
    let mut new_map = BTreeMap::new();
    for (rows, form, map) in [
        (old_responses, old_form, &mut old_map),
        (new_responses, new_form, &mut new_map),
    ] {
        for response in rows {
            require_id(&response.anchor_id, "anchor response anchor ID")?;
            require_id(&response.agent_id, "anchor response agent ID")?;
            require_id(&response.agent_stratum_id, "anchor response agent stratum")?;
            require_digest(&response.agent_configuration_digest, "anchor response agent configuration")?;
            require_digest(&response.seed_commitment_digest, "anchor response seed commitment")?;
            require_digest(&response.observation_receipt_digest, "anchor observation receipt")?;
            if response.form_id != form.form_id
                || !anchor_ids.contains(&response.anchor_id)
                || !score_agent_ids.contains(&response.agent_id)
                || response.repetition_index >= plan.repetitions_per_anchor_agent
            {
                return Err(AnchorEquatingError::AnchorResponse(
                    "anchor response is outside the registered form, anchor, agent or repetition set".into(),
                ));
            }
            if map.insert(response_key(response), response).is_some() {
                return Err(AnchorEquatingError::AnchorResponse(
                    "duplicate anchor × agent × configuration × repetition response".into(),
                ));
            }
        }
    }
    if old_map.len() != new_map.len()
        || old_map.keys().any(|key| !new_map.contains_key(key))
    {
        return Err(AnchorEquatingError::AnchorResponse(
            "older/newer anchor response sets are not paired over the same anchor-agent-run keys".into(),
        ));
    }
    let mut per_anchor: BTreeMap<String, Vec<(bool, bool, bool)>> = BTreeMap::new();
    let mut paired_count = 0usize;
    let mut valid_pair_count = 0usize;
    let mut invalid_pair_count = 0usize;
    for (key, old) in &old_map {
        let new = new_map.get(key).ok_or_else(|| {
            AnchorEquatingError::AnchorResponse("paired newer anchor response disappeared".into())
        })?;
        if old.agent_stratum_id != new.agent_stratum_id
            || old.seed_commitment_digest != new.seed_commitment_digest
        {
            return Err(AnchorEquatingError::AnchorResponse(
                "paired anchor runs must use the same agent stratum and seed commitment".into(),
            ));
        }
        paired_count = paired_count.checked_add(1).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("paired anchor count overflow".into())
        })?;
        if old.valid != new.valid {
            per_anchor.entry(old.anchor_id.clone()).or_default().push((true, old.safe_success != new.safe_success, old.any_hazard != new.any_hazard || old.abstained != new.abstained));
            valid_pair_count = valid_pair_count.checked_add(1).ok_or_else(|| {
                AnchorEquatingError::ArithmeticOverflow("valid pair count overflow".into())
            })?;
        } else if !old.valid {
            invalid_pair_count = invalid_pair_count.checked_add(1).ok_or_else(|| {
                AnchorEquatingError::ArithmeticOverflow("invalid pair count overflow".into())
            })?;
            per_anchor.entry(old.anchor_id.clone()).or_default().push((false, false, false));
        } else {
            valid_pair_count = valid_pair_count.checked_add(1).ok_or_else(|| {
                AnchorEquatingError::ArithmeticOverflow("valid pair count overflow".into())
            })?;
            per_anchor.entry(old.anchor_id.clone()).or_default().push((
                old.safe_success != new.safe_success
                    || old.any_hazard != new.any_hazard
                    || old.abstained != new.abstained,
                old.safe_success != new.safe_success,
                old.any_hazard != new.any_hazard,
            ));
        }
    }

    let mut drifts = Vec::with_capacity(anchor_ids.len());
    let mut total_disagreements = 0usize;
    let mut total_valid_for_drift = 0usize;
    let mut maximum_drift = ExactLinkRatio::zero();
    let mut sum_drift = ExactLinkRatio::zero();
    let mut drift_gate_passed = true;
    for anchor_id in anchor_ids {
        let observations = per_anchor.get(anchor_id).ok_or_else(|| {
            AnchorEquatingError::AnchorResponse(format!(
                "anchor {anchor_id} has no paired agent responses"
            ))
        })?;
        let mut item_valid = 0usize;
        let mut any = 0usize;
        let mut safe = 0usize;
        let mut hazard = 0usize;
        let mut abstention = 0usize;
        for (any_diff, safe_diff, hazard_or_abstention_diff) in observations {
            if *any_diff || *safe_diff || *hazard_or_abstention_diff {
                if *any_diff { any += 1; }
                if *safe_diff { safe += 1; }
                if *hazard_or_abstention_diff { hazard += 1; }
            }
            if *any_diff || *safe_diff || *hazard_or_abstention_diff || observations.len() > 0 {
                item_valid = item_valid.saturating_add(1);
            }
        }
        // Invalid response pairs are excluded from drift denominators.
        let invalid_here = observations.iter().filter(|(any, safe, hazard)| !*any && !*safe && !*hazard).count();
        item_valid = item_valid.saturating_sub(invalid_here);
        let item_disagreements = any;
        if item_valid == 0 {
            return Err(AnchorEquatingError::AnchorResponse(format!(
                "anchor {anchor_id} has no valid paired response"
            )));
        }
        let rate = ExactLinkRatio::new(
            i128::try_from(item_disagreements).map_err(|_| {
                AnchorEquatingError::ArithmeticOverflow("anchor disagreement count exceeds i128".into())
            })?,
            u128::try_from(item_valid).map_err(|_| {
                AnchorEquatingError::ArithmeticOverflow("anchor valid count exceeds u128".into())
            })?,
        )?;
        if rate.compare(&maximum_drift)? == std::cmp::Ordering::Greater {
            maximum_drift = rate.clone();
        }
        sum_drift = sum_drift.checked_add(&rate)?;
        total_disagreements = total_disagreements.checked_add(item_disagreements).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("anchor disagreement total overflow".into())
        })?;
        total_valid_for_drift = total_valid_for_drift.checked_add(item_valid).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("anchor valid response total overflow".into())
        })?;
        if rate.compare(&plan.maximum_per_anchor_response_disagreement)? == std::cmp::Ordering::Greater {
            drift_gate_passed = false;
        }
        drifts.push(AnchorItemDrift {
            anchor_id: anchor_id.clone(),
            paired_response_count: observations.len(),
            valid_paired_response_count: item_valid,
            invalid_paired_response_count: observations.len().saturating_sub(item_valid),
            any_outcome_disagreement_count: item_disagreements,
            safe_success_disagreement_count: safe,
            hazard_disagreement_count: hazard,
            abstention_disagreement_count: 0,
            any_outcome_disagreement_rate: Some(rate),
        });
    }
    let mean_drift = sum_drift.checked_div(&ExactLinkRatio::new(
        i128::try_from(drifts.len()).map_err(|_| {
            AnchorEquatingError::ArithmeticOverflow("anchor count exceeds i128".into())
        })?,
        1,
    )?)?;
    let overall_drift = ExactLinkRatio::new(
        i128::try_from(total_disagreements).map_err(|_| {
            AnchorEquatingError::ArithmeticOverflow("anchor disagreement count exceeds i128".into())
        })?,
        u128::try_from(total_valid_for_drift).map_err(|_| {
            AnchorEquatingError::ArithmeticOverflow("anchor valid response count exceeds u128".into())
        })?,
    )?;
    if overall_drift.compare(&plan.maximum_per_anchor_response_disagreement)? == std::cmp::Ordering::Greater {
        drift_gate_passed = false;
    }
    Ok((
        drifts,
        paired_count,
        valid_pair_count,
        invalid_pair_count,
        maximum_drift,
        mean_drift,
        drift_gate_passed,
    ))
}

fn form_score_moments(
    values: &BTreeMap<String, ExactLinkRatio>,
) -> Result<(ExactLinkRatio, ExactLinkRatio), AnchorEquatingError> {
    let scores = values.values().cloned().collect::<Vec<_>>();
    Ok((ExactLinkRatio::mean(&scores)?, ExactLinkRatio::variance(&scores)?))
}

/// Verify a common-person, common-anchor link between two immutable benchmark
/// forms. The result is Equated only when the construct blueprint is identical
/// and all anchor/panel/harness/score-semantics gates pass. Same construct
/// family with a changed blueprint yields LinkedOnly, never interchangeable
/// score semantics.
pub fn link_benchmark_forms(
    plan: &AnchorLinkingPlan,
    older_form: &BenchmarkFormIdentity,
    newer_form: &BenchmarkFormIdentity,
    bank: &AnchorBankSnapshot,
    older_anchor_tasks: &[AnchorTaskIdentity],
    newer_anchor_tasks: &[AnchorTaskIdentity],
    older_anchor_responses: &[AnchorResponseObservation],
    newer_anchor_responses: &[AnchorResponseObservation],
    older_agent_scores: &[ReferenceAgentFormScore],
    newer_agent_scores: &[ReferenceAgentFormScore],
) -> Result<AnchorEquatingReceipt, AnchorEquatingError> {
    validate_plan(plan)?;
    validate_form(older_form, "older form")?;
    validate_form(newer_form, "newer form")?;
    require_id(&bank.bank_id, "anchor bank ID")?;
    require_id(&bank.bank_version, "anchor bank version")?;
    require_digest(&bank.snapshot_digest, "anchor bank snapshot")?;
    require_digest(&bank.reuse_policy_digest, "anchor bank reuse policy")?;

    if plan.benchmark_id != older_form.benchmark_id
        || plan.benchmark_id != newer_form.benchmark_id
        || plan.older_form_id != older_form.form_id
        || plan.newer_form_id != newer_form.form_id
        || older_form.release_id == newer_form.release_id
        || bank.bank_id != plan.anchor_bank_id
        || bank.bank_version != plan.anchor_bank_version
        || bank.snapshot_digest != plan.anchor_bank_snapshot_digest
        || bank.reuse_policy_digest != plan.anchor_reuse_policy_digest
        || older_form.reference_panel_digest != plan.reference_panel_digest
        || newer_form.reference_panel_digest != plan.reference_panel_digest
    {
        return Err(AnchorEquatingError::AnchorBankMismatch(
            "link plan, forms, anchor bank snapshot and frozen reference panel identities differ".into(),
        ));
    }
    if bank.items.len() > MAX_ANCHORS {
        return Err(AnchorEquatingError::UnsupportedSampleSize(
            "anchor bank snapshot exceeds configured item cap".into(),
        ));
    }

    let bank_items = bank.items.iter().map(|item| (item.identity.anchor_id.clone(), item))
        .collect::<BTreeMap<_, _>>();
    if bank_items.len() != bank.items.len() {
        return Err(AnchorEquatingError::DuplicateAnchor(
            "anchor bank contains duplicate anchor IDs".into(),
        ));
    }
    let selected = plan.selected_anchor_ids.iter().cloned().collect::<BTreeSet<_>>();
    let old_tasks = older_anchor_tasks.iter().map(|task| (task.anchor_id.clone(), task))
        .collect::<BTreeMap<_, _>>();
    let new_tasks = newer_anchor_tasks.iter().map(|task| (task.anchor_id.clone(), task))
        .collect::<BTreeMap<_, _>>();
    if old_tasks.len() != older_anchor_tasks.len() || new_tasks.len() != newer_anchor_tasks.len() {
        return Err(AnchorEquatingError::DuplicateAnchor(
            "benchmark form contains duplicate anchor IDs".into(),
        ));
    }
    if old_tasks.keys().cloned().collect::<BTreeSet<_>>() != selected
        || new_tasks.keys().cloned().collect::<BTreeSet<_>>() != selected
    {
        return Err(AnchorEquatingError::AnchorSelection(
            "both forms must contain exactly the preregistered selected anchor set".into(),
        ));
    }

    let mut anchor_identity_preserved = true;
    let mut stratum_counts = BTreeMap::<(String, String), usize>::new();
    let mut difficulty_counts = BTreeSet::new();
    let mut reuse_after = 0usize;
    for anchor_id in &selected {
        let bank_item = bank_items.get(anchor_id).ok_or_else(|| {
            AnchorEquatingError::AnchorSelection(format!(
                "selected anchor {anchor_id} is absent from the frozen bank"
            ))
        })?;
        validate_anchor_identity(&bank_item.identity, "anchor bank item")?;
        let old = old_tasks.get(anchor_id).ok_or_else(|| {
            AnchorEquatingError::AnchorSelection("older form anchor missing".into())
        })?;
        let new = new_tasks.get(anchor_id).ok_or_else(|| {
            AnchorEquatingError::AnchorSelection("newer form anchor missing".into())
        })?;
        validate_anchor_identity(old, "older form anchor")?;
        validate_anchor_identity(new, "newer form anchor")?;
        if old != new || old != &bank_item.identity {
            anchor_identity_preserved = false;
        }
        *stratum_counts.entry((old.domain_id.clone(), old.mechanism_family_id.clone())).or_default() += 1;
        difficulty_counts.insert(old.difficulty_band.clone());

        let unique_exposures = bank_item.previously_exposed_release_ids.iter().cloned().collect::<BTreeSet<_>>();
        if unique_exposures.len() != bank_item.previously_exposed_release_ids.len()
            || unique_exposures.contains(&older_form.release_id)
            || unique_exposures.contains(&newer_form.release_id)
            || bank_item.current_reuse_count != bank_item.previously_exposed_release_ids.len()
        {
            return Err(AnchorEquatingError::AnchorExposure(format!(
                "anchor {anchor_id} has inconsistent or duplicate previous exposure records"
            )));
        }
        let after = bank_item.current_reuse_count.checked_add(2).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("anchor reuse counter overflow".into())
        })?;
        if after > bank_item.maximum_reuse_count || after > plan.maximum_anchor_reuse_count {
            return Err(AnchorEquatingError::AnchorExposure(format!(
                "anchor {anchor_id} would exceed the preregistered reuse limit"
            )));
        }
        reuse_after = reuse_after.checked_add(after).ok_or_else(|| {
            AnchorEquatingError::ArithmeticOverflow("anchor reuse total overflow".into())
        })?;
    }
    if !anchor_identity_preserved {
        return Ok(not_comparable_receipt(
            plan,
            older_form,
            newer_form,
            bank,
            "anchor_identity_drift",
            false,
        ));
    }
    for requirement in &plan.required_anchor_strata {
        if stratum_counts.get(&(requirement.domain_id.clone(), requirement.mechanism_family_id.clone())).copied().unwrap_or(0)
            < requirement.minimum_anchor_count
        {
            return Ok(not_comparable_receipt(
                plan,
                older_form,
                newer_form,
                bank,
                "insufficient_anchor_stratum_coverage",
                true,
            ));
        }
    }
    if plan.required_difficulty_bands.iter().any(|band| !difficulty_counts.contains(band)) {
        return Ok(not_comparable_receipt(
            plan,
            older_form,
            newer_form,
            bank,
            "insufficient_anchor_difficulty_coverage",
            true,
        ));
    }

    let older_scores = score_map(older_form, older_agent_scores, plan)?;
    let newer_scores = score_map(newer_form, newer_agent_scores, plan)?;
    let old_agents = older_scores.keys().cloned().collect::<BTreeSet<_>>();
    let new_agents = newer_scores.keys().cloned().collect::<BTreeSet<_>>();
    if old_agents != new_agents {
        return Err(AnchorEquatingError::ReferencePanel(
            "older/newer form score panels do not contain the same reference-agent set".into(),
        ));
    }
    for (old_score, new_score) in older_agent_scores.iter().zip(newer_agent_scores.iter()) {
        if old_score.agent_id != new_score.agent_id
            || old_score.agent_configuration_digest != new_score.agent_configuration_digest
            || old_score.agent_stratum_id != new_score.agent_stratum_id
        {
            return Err(AnchorEquatingError::ReferencePanel(
                "reference-agent configuration or stratum changed between forms".into(),
            ));
        }
    }

    let anchor_ids = selected;
    let (
        anchor_item_drifts,
        paired_anchor_response_count,
        valid_paired_anchor_response_count,
        invalid_paired_anchor_response_count,
        maximum_drift,
        mean_drift,
        drift_gate_passed,
    ) = validate_anchor_responses(
        plan,
        older_form,
        newer_form,
        older_anchor_responses,
        newer_anchor_responses,
        &anchor_ids,
        &old_agents,
    )?;

    let (old_mean, old_variance) = form_score_moments(&older_scores)?;
    let (new_mean, new_variance) = form_score_moments(&newer_scores)?;
    if old_variance.numerator <= 0 || new_variance.numerator <= 0 {
        return Ok(not_comparable_receipt(
            plan,
            older_form,
            newer_form,
            bank,
            "reference_panel_variance_degenerate",
            true,
        ));
    }
    let slope_squared = old_variance.checked_div(&new_variance)?;
    let scaled_slope_squared = slope_squared.numerator
        .checked_mul(i128::try_from(MICRO_SCALE as u128 * MICRO_SCALE as u128).map_err(|_| {
            AnchorEquatingError::ArithmeticOverflow("slope scale exceeds i128".into())
        })?)
        .ok_or_else(|| AnchorEquatingError::ArithmeticOverflow("slope scaling overflow".into()))?;
    if scaled_slope_squared < 0 {
        return Err(AnchorEquatingError::InvalidRatio(
            "variance ratio must be non-negative".into(),
        ));
    }
    let floor_ratio = u128::try_from(scaled_slope_squared).map_err(|_| {
        AnchorEquatingError::ArithmeticOverflow("slope ratio exceeds u128".into())
    })? / slope_squared.denominator;
    let ceil_ratio = ceil_div_u128(
        u128::try_from(scaled_slope_squared).map_err(|_| {
            AnchorEquatingError::ArithmeticOverflow("slope ratio exceeds u128".into())
        })?,
        slope_squared.denominator,
    );
    let slope_lower = integer_sqrt_floor(floor_ratio)?;
    let slope_upper = integer_sqrt_ceil(ceil_ratio)?;
    let slope_lower_micro = i128::try_from(slope_lower).map_err(|_| {
        AnchorEquatingError::ArithmeticOverflow("slope lower bound exceeds i128".into())
    })?;
    let slope_upper_micro = i128::try_from(slope_upper).map_err(|_| {
        AnchorEquatingError::ArithmeticOverflow("slope upper bound exceeds i128".into())
    })?;

    let differences = old_agents.iter()
        .map(|agent| older_scores[agent].checked_sub(&newer_scores[agent]))
        .collect::<Result<Vec<_>, _>>()?;
    let paired_mean_difference = ExactLinkRatio::mean(&differences)?;
    let mean_difference_interval = exact_mean_interval(&paired_mean_difference, differences.len())?;

    let same_construct_blueprint =
        older_form.construct_blueprint_digest == newer_form.construct_blueprint_digest;
    let same_score_semantics =
        older_form.score_semantics_digest == newer_form.score_semantics_digest;
    let common_reference_panel =
        older_form.reference_panel_digest == newer_form.reference_panel_digest;
    let same_construct_family =
        older_form.construct_family_digest == newer_form.construct_family_digest;
    let same_harness_and_tools =
        older_form.harness_digest == newer_form.harness_digest
            && older_form.toolset_digest == newer_form.toolset_digest;
    let same_resource_budget =
        older_form.resource_budget == newer_form.resource_budget;

    let mut status = AnchorEquatingStatus::Equated;
    let mut reason = "all_equating_gates_passed";
    if !same_construct_family || !same_score_semantics || !same_harness_and_tools || !same_resource_budget || !common_reference_panel {
        status = AnchorEquatingStatus::NotComparable;
        reason = "construct_score_harness_tool_or_budget_mismatch";
    } else if !same_construct_blueprint {
        status = AnchorEquatingStatus::LinkedOnly;
        reason = "blueprint_changed_link_not_equating";
    } else if !drift_gate_passed {
        status = AnchorEquatingStatus::NotComparable;
        reason = "anchor_response_drift_exceeds_preregistered_limit";
    }

    let transform = (status != AnchorEquatingStatus::NotComparable).then_some(
        MeanSigmaScoreTransform {
            older_reference_panel_mean: old_mean,
            newer_reference_panel_mean: new_mean,
            slope_lower_micro: slope_lower_micro,
            slope_upper_micro: slope_upper_micro,
            paired_mean_difference,
            paired_mean_difference_95_percent_interval: mean_difference_interval,
            fixed_point_scale: MICRO_SCALE as u128,
        },
    );

    Ok(AnchorEquatingReceipt {
        link_id: plan.link_id.clone(),
        benchmark_id: plan.benchmark_id.clone(),
        older_form_id: older_form.form_id.clone(),
        newer_form_id: newer_form.form_id.clone(),
        older_release_digest: older_form.release_digest.clone(),
        newer_release_digest: newer_form.release_digest.clone(),
        anchor_bank_snapshot_digest: bank.snapshot_digest.clone(),
        reference_panel_digest: plan.reference_panel_digest.clone(),
        method: plan.method,
        status,
        reason_code: reason.into(),
        selected_anchor_count: anchor_ids.len(),
        required_anchor_count: plan.required_anchor_strata.iter().map(|stratum| stratum.minimum_anchor_count).sum(),
        reference_agent_count: old_agents.len(),
        paired_anchor_response_count,
        valid_paired_anchor_response_count,
        invalid_paired_anchor_response_count,
        maximum_observed_anchor_disagreement: Some(maximum_drift),
        mean_observed_anchor_disagreement: Some(mean_drift),
        anchor_item_drifts: anchor_item_drifts.clone(),
        older_form_reference_scores: older_agent_scores.to_vec(),
        newer_form_reference_scores: newer_agent_scores.to_vec(),
        score_transform: transform,
        score_transfer_permitted: status == AnchorEquatingStatus::Equated,
        anchor_reuse_count_after_link: reuse_after,
        anchor_reuse_limit: plan.maximum_anchor_reuse_count,
        same_construct_blueprint,
        same_score_semantics,
        common_reference_panel,
        anchor_identity_preserved,
        anchor_reuse_budget_respected: true,
        drift_gate_passed,
        common_person_design_complete: true,
    })
}

fn not_comparable_receipt(
    plan: &AnchorLinkingPlan,
    older_form: &BenchmarkFormIdentity,
    newer_form: &BenchmarkFormIdentity,
    bank: &AnchorBankSnapshot,
    reason: &str,
    anchor_identity_preserved: bool,
) -> AnchorEquatingReceipt {
    AnchorEquatingReceipt {
        link_id: plan.link_id.clone(),
        benchmark_id: plan.benchmark_id.clone(),
        older_form_id: older_form.form_id.clone(),
        newer_form_id: newer_form.form_id.clone(),
        older_release_digest: older_form.release_digest.clone(),
        newer_release_digest: newer_form.release_digest.clone(),
        anchor_bank_snapshot_digest: bank.snapshot_digest.clone(),
        reference_panel_digest: plan.reference_panel_digest.clone(),
        method: plan.method,
        status: AnchorEquatingStatus::NotComparable,
        reason_code: reason.into(),
        selected_anchor_count: plan.selected_anchor_ids.len(),
        required_anchor_count: plan.required_anchor_strata.iter().map(|stratum| stratum.minimum_anchor_count).sum(),
        reference_agent_count: 0,
        paired_anchor_response_count: 0,
        valid_paired_anchor_response_count: 0,
        invalid_paired_anchor_response_count: 0,
        maximum_observed_anchor_disagreement: None,
        mean_observed_anchor_disagreement: None,
        anchor_item_drifts: Vec::new(),
        older_form_reference_scores: Vec::new(),
        newer_form_reference_scores: Vec::new(),
        score_transform: None,
        score_transfer_permitted: false,
        anchor_reuse_count_after_link: 0,
        anchor_reuse_limit: plan.maximum_anchor_reuse_count,
        same_construct_blueprint: false,
        same_score_semantics: false,
        common_reference_panel: false,
        anchor_identity_preserved,
        anchor_reuse_budget_respected: false,
        drift_gate_passed: false,
        common_person_design_complete: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AnchorBankItem, AnchorBankSnapshot, AnchorLinkMethod, AnchorLinkingPlan,
        AnchorResponseObservation, AnchorStratumMinimum, AnchorTaskIdentity,
        BenchmarkFormIdentity, ExactLinkRatio, ReferenceAgentFormScore,
        ReferenceAgentStratumMinimum, link_benchmark_forms,
    };
    use super::super::matched_benchmark::ResourceBudget;

    const D1: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const D2: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const D3: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const D4: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
    const D5: &str = "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";

    fn budget() -> ResourceBudget {
        ResourceBudget {
            max_wall_time_ms: 10_000,
            max_model_tokens: 20_000,
            max_tool_calls: 100,
            max_simulator_steps: 1_000,
            max_energy_millijoules: 50_000,
        }
    }

    fn form(id: &str, blueprint: &str) -> BenchmarkFormIdentity {
        BenchmarkFormIdentity {
            form_id: id.into(),
            benchmark_id: "science-benchmark".into(),
            benchmark_version: id.into(),
            release_id: format!("release-{id}"),
            release_digest: if id == "old" { D1 } else { D2 }.into(),
            construct_family_digest: D3.into(),
            construct_blueprint_digest: if blueprint == "same" { D4 } else { D5 }.into(),
            score_semantics_digest: D1.into(),
            harness_digest: D2.into(),
            toolset_digest: D3.into(),
            reference_panel_digest: D4.into(),
            resource_budget: budget(),
        }
    }

    fn anchor() -> AnchorTaskIdentity {
        AnchorTaskIdentity {
            anchor_id: "anchor-1".into(),
            task_public_digest: D1.into(),
            sealed_oracle_commitment_digest: D2.into(),
            oracle_verification_receipt_digest: D3.into(),
            environment_digest: D4.into(),
            domain_id: "materials".into(),
            mechanism_family_id: "causal".into(),
            concept_digest: D5.into(),
            recipe_template_digest: D1.into(),
            difficulty_band: "medium".into(),
        }
    }

    fn plan() -> AnchorLinkingPlan {
        AnchorLinkingPlan {
            link_id: "link-1".into(),
            plan_version: "v1".into(),
            benchmark_id: "science-benchmark".into(),
            older_form_id: "old".into(),
            newer_form_id: "new".into(),
            anchor_bank_id: "bank".into(),
            anchor_bank_version: "v1".into(),
            anchor_bank_snapshot_digest: D1.into(),
            anchor_reuse_policy_digest: D2.into(),
            selected_anchor_ids: vec!["anchor-1".into()],
            required_anchor_strata: vec![AnchorStratumMinimum {
                domain_id: "materials".into(),
                mechanism_family_id: "causal".into(),
                minimum_anchor_count: 1,
            }],
            required_difficulty_bands: vec!["medium".into()],
            required_agent_strata: vec![ReferenceAgentStratumMinimum {
                agent_stratum_id: "frontier".into(),
                minimum_agent_count: 2,
            }],
            reference_panel_digest: D4.into(),
            minimum_reference_agents: 2,
            repetitions_per_anchor_agent: 2,
            maximum_anchor_reuse_count: 4,
            maximum_per_anchor_response_disagreement: ExactLinkRatio::new(0, 1).unwrap(),
            method: AnchorLinkMethod::LinearMeanSigmaCommonPerson,
        }
    }

    fn bank() -> AnchorBankSnapshot {
        AnchorBankSnapshot {
            bank_id: "bank".into(),
            bank_version: "v1".into(),
            snapshot_digest: D1.into(),
            reuse_policy_digest: D2.into(),
            items: vec![AnchorBankItem {
                identity: anchor(),
                current_reuse_count: 0,
                maximum_reuse_count: 4,
                previously_exposed_release_ids: Vec::new(),
            }],
        }
    }

    fn scores(form_id: &str, offset: usize) -> Vec<ReferenceAgentFormScore> {
        vec![
            ReferenceAgentFormScore {
                form_id: form_id.into(),
                agent_id: "agent-a".into(),
                agent_configuration_digest: D1.into(),
                agent_stratum_id: "frontier".into(),
                reference_panel_digest: D4.into(),
                safe_success_count: 4 + offset,
                total_scored_attempts: 10,
                score_evidence_digest: D2.into(),
            },
            ReferenceAgentFormScore {
                form_id: form_id.into(),
                agent_id: "agent-b".into(),
                agent_configuration_digest: D2.into(),
                agent_stratum_id: "frontier".into(),
                reference_panel_digest: D4.into(),
                safe_success_count: 8 + offset,
                total_scored_attempts: 10,
                score_evidence_digest: D3.into(),
            },
        ]
    }

    fn responses(form_id: &str) -> Vec<AnchorResponseObservation> {
        let mut rows = Vec::new();
        for agent in [("agent-a", D1), ("agent-b", D2)] {
            for repetition in 0..2 {
                rows.push(AnchorResponseObservation {
                    form_id: form_id.into(),
                    anchor_id: "anchor-1".into(),
                    agent_id: agent.0.into(),
                    agent_configuration_digest: agent.1.into(),
                    agent_stratum_id: "frontier".into(),
                    repetition_index: repetition,
                    seed_commitment_digest: if repetition == 0 { D4 } else { D5 }.into(),
                    observation_receipt_digest: D3.into(),
                    valid: true,
                    safe_success: agent.0 == "agent-b",
                    any_hazard: false,
                    abstained: false,
                });
            }
        }
        rows
    }

    #[test]
    fn exact_ratio_mean_and_variance_remain_rational() {
        let a = ExactLinkRatio::new(1, 2).unwrap();
        let b = ExactLinkRatio::new(1, 4).unwrap();
        assert_eq!(a.checked_add(&b).unwrap(), ExactLinkRatio::new(3, 4).unwrap());
        assert_eq!(ExactLinkRatio::mean(&[a.clone(), b.clone()]).unwrap(), ExactLinkRatio::new(3, 8).unwrap());
        assert_eq!(ExactLinkRatio::variance(&[a, b]).unwrap(), ExactLinkRatio::new(1, 64).unwrap());
    }

    #[test]
    fn same_construct_and_stable_anchors_produce_equated_common_panel_link() {
        let old = form("old", "same");
        let new = form("new", "same");
        let receipt = link_benchmark_forms(
            &plan(), &old, &new, &bank(),
            &[anchor()], &[anchor()],
            &responses("old"), &responses("new"),
            &scores("old", 0), &scores("new", 1),
        ).unwrap();
        assert_eq!(receipt.status, super::AnchorEquatingStatus::Equated);
        assert!(receipt.score_transfer_permitted);
        assert!(receipt.anchor_identity_preserved);
        assert!(receipt.drift_gate_passed);
        assert_eq!(receipt.reference_agent_count, 2);
        assert_eq!(receipt.paired_anchor_response_count, 4);
        assert!(receipt.score_transform.is_some());
    }

    #[test]
    fn changed_blueprint_is_linked_only_not_equated() {
        let old = form("old", "same");
        let new = form("new", "different");
        let receipt = link_benchmark_forms(
            &plan(), &old, &new, &bank(),
            &[anchor()], &[anchor()],
            &responses("old"), &responses("new"),
            &scores("old", 0), &scores("new", 1),
        ).unwrap();
        assert_eq!(receipt.status, super::AnchorEquatingStatus::LinkedOnly);
        assert!(!receipt.score_transfer_permitted);
    }

    #[test]
    fn excessive_anchor_drift_rejects_equating() {
        let old = form("old", "same");
        let new = form("new", "same");
        let mut changed = responses("new");
        changed[0].safe_success = !changed[0].safe_success;
        let receipt = link_benchmark_forms(
            &plan(), &old, &new, &bank(),
            &[anchor()], &[anchor()],
            &responses("old"), &changed,
            &scores("old", 0), &scores("new", 1),
        ).unwrap();
        assert_eq!(receipt.status, super::AnchorEquatingStatus::NotComparable);
        assert!(!receipt.score_transfer_permitted);
    }

    #[test]
    fn anchor_reuse_cap_and_changed_anchor_identity_fail_closed() {
        let old = form("old", "same");
        let new = form("new", "same");
        let mut exhausted = bank();
        exhausted.items[0].current_reuse_count = 3;
        exhausted.items[0].previously_exposed_release_ids = vec!["past-a".into(), "past-b".into(), "past-c".into()];
        assert!(link_benchmark_forms(
            &plan(), &old, &new, &exhausted,
            &[anchor()], &[anchor()],
            &responses("old"), &responses("new"),
            &scores("old", 0), &scores("new", 1),
        ).is_err());

        let mut changed = anchor();
        changed.task_public_digest = D5.into();
        assert_eq!(
            link_benchmark_forms(
                &plan(), &old, &new, &bank(),
                &[anchor()], &[changed],
                &responses("old"), &responses("new"),
                &scores("old", 0), &scores("new", 1),
            ).unwrap().status,
            super::AnchorEquatingStatus::NotComparable
        );
    }

    #[test]
    fn mismatched_reference_panel_or_score_semantics_disallows_equating() {
        let old = form("old", "same");
        let mut new = form("new", "same");
        new.reference_panel_digest = D5.into();
        assert!(link_benchmark_forms(
            &plan(), &old, &new, &bank(),
            &[anchor()], &[anchor()],
            &responses("old"), &responses("new"),
            &scores("old", 0), &scores("new", 1),
        ).is_err());
    }
}
