//! Fixed-sample cluster-valid adverse-event risk bounds and paired
//! candidate/baseline non-inferiority.
//!
//! Consumes source-qualified trial-design receipts and already-classified
//! episode/traversal outcomes. It does not prove independence, detect events,
//! or provide optional-stopping guarantees.

use std::collections::{BTreeMap, BTreeSet};

pub const PPM_SCALE: u32 = 1_000_000;
pub const NUMERIC_OUTWARD_GUARD_PPM: u32 = 2;
pub const MAX_TRIALS: usize = 200_000;
pub const MAX_CATEGORIES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceTier {
    Simulation,
    Hardware,
    Biological,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellIdentity {
    pub scenario_digest: String,
    pub morphology_digest: String,
    pub evidence_tier: EvidenceTier,
    pub platform_physics_digest: String,
    pub dynamics_model_digest: String,
    pub transition_abstraction_digest: String,
    pub controller_digest: String,
    pub operating_mode_id: String,
    pub task_digest: String,
    pub terrain_cell_id: String,
    pub sensor_health_digest: String,
    pub calibration_digest: String,
    pub taxonomy_digest: String,
    pub detector_digest: String,
    pub actuator_digest: String,
    pub initial_state_digest: String,
    pub payload_digest: String,
    pub energy_boundary_id: String,
    pub exposure_unit_id: String,
    pub exposure_value: u64,
    pub heldout_manifest_digest: String,
}

impl CellIdentity {
    fn validate(&self) -> Result<(), RiskBoundError> {
        let values = [
            self.scenario_digest.as_str(),
            self.morphology_digest.as_str(),
            self.platform_physics_digest.as_str(),
            self.dynamics_model_digest.as_str(),
            self.transition_abstraction_digest.as_str(),
            self.controller_digest.as_str(),
            self.operating_mode_id.as_str(),
            self.task_digest.as_str(),
            self.terrain_cell_id.as_str(),
            self.sensor_health_digest.as_str(),
            self.calibration_digest.as_str(),
            self.taxonomy_digest.as_str(),
            self.detector_digest.as_str(),
            self.actuator_digest.as_str(),
            self.initial_state_digest.as_str(),
            self.payload_digest.as_str(),
            self.energy_boundary_id.as_str(),
            self.exposure_unit_id.as_str(),
            self.heldout_manifest_digest.as_str(),
        ];
        if values.iter().any(|value| !nonempty(value)) || self.exposure_value == 0 {
            return Err(RiskBoundError::InvalidCellIdentity);
        }
        Ok(())
    }

    fn same_cell_except_controller(&self, other: &Self) -> bool {
        self.scenario_digest == other.scenario_digest
            && self.morphology_digest == other.morphology_digest
            && self.evidence_tier == other.evidence_tier
            && self.platform_physics_digest == other.platform_physics_digest
            && self.dynamics_model_digest == other.dynamics_model_digest
            && self.transition_abstraction_digest == other.transition_abstraction_digest
            && self.operating_mode_id == other.operating_mode_id
            && self.task_digest == other.task_digest
            && self.terrain_cell_id == other.terrain_cell_id
            && self.sensor_health_digest == other.sensor_health_digest
            && self.calibration_digest == other.calibration_digest
            && self.taxonomy_digest == other.taxonomy_digest
            && self.detector_digest == other.detector_digest
            && self.actuator_digest == other.actuator_digest
            && self.initial_state_digest == other.initial_state_digest
            && self.payload_digest == other.payload_digest
            && self.energy_boundary_id == other.energy_boundary_id
            && self.exposure_unit_id == other.exposure_unit_id
            && self.exposure_value == other.exposure_value
            && self.heldout_manifest_digest == other.heldout_manifest_digest
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrialUnitKind {
    Episode,
    Traversal,
    StairAscentDescentPair,
    RobotHourBlock,
    Session,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialDesignReceipt {
    pub receipt_id: String,
    pub identity: CellIdentity,
    pub trial_unit_kind: TrialUnitKind,
    pub cluster_definition_digest: String,
    pub sampling_frame_digest: String,
    pub randomization_protocol_digest: String,
    pub independence_review_digest: String,
    pub fixed_sample_stop_rule: bool,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
    pub result_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorySpec {
    pub category_id: String,
    pub maximum_event_rate_ppm: u32,
    /// Candidate-minus-baseline upper bound must be <= this margin.
    pub noninferiority_margin_ppm: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryOutcome {
    pub category_id: String,
    /// None means the event detector or measurement coverage was Unknown.
    pub event_observed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialRecord {
    /// A run-level identifier; not used as the independent denominator.
    pub trial_id: String,
    /// The independently qualified physical cluster ID; unique within batch.
    pub independent_unit_id: String,
    pub session_id: String,
    pub training_seed_id: String,
    pub hardware_unit_id: String,
    /// Metadata only. Never contributes to n_independent_units.
    pub control_tick_count: u64,
    pub outcomes: Vec<CategoryOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialBatch {
    pub identity: CellIdentity,
    pub design_receipt: TrialDesignReceipt,
    pub trials: Vec<TrialRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskBoundProblem {
    pub now_ns: u64,
    pub family_alpha_ppm: u32,
    pub categories: Vec<CategorySpec>,
    pub candidate: TrialBatch,
    pub baseline: Option<TrialBatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundMethod {
    ExactZeroEventBinomial,
    HoeffdingBernoulli,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus {
    Valid,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairedStatus {
    Valid,
    Withheld,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedEventTable {
    pub neither: u64,
    pub candidate_only: u64,
    pub baseline_only: u64,
    pub both: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryRiskReport {
    pub category_id: String,
    pub independent_unit_count: u64,
    pub event_unit_count: u64,
    pub control_tick_count_metadata: u64,
    pub absolute_alpha_ppm: u32,
    pub upper_bound_ppm: u32,
    pub method: BoundMethod,
    pub maximum_event_rate_ppm: u32,
    pub absolute_limit_pass: bool,
    pub paired_status: PairedStatus,
    pub paired_alpha_ppm: Option<u32>,
    pub paired_unit_count: Option<u64>,
    pub paired_event_table: Option<PairedEventTable>,
    /// Candidate minus baseline; positive values are worse for the candidate.
    pub paired_difference_upper_bound_ppm: Option<i32>,
    pub noninferiority_margin_ppm: i32,
    pub paired_noninferiority_pass: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskBoundReport {
    pub evidence_status: EvidenceStatus,
    pub candidate_receipt_id: String,
    pub baseline_receipt_id: Option<String>,
    pub family_alpha_ppm: u32,
    pub numeric_outward_guard_ppm: u32,
    pub candidate_independent_unit_count: u64,
    pub candidate_trial_record_count: u64,
    pub candidate_control_tick_count_metadata: u64,
    pub matched_baseline_eligible: bool,
    pub baseline_comparison_reason: Option<String>,
    pub unknown_reasons: Vec<String>,
    pub categories: Vec<CategoryRiskReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskBoundError {
    InvalidCellIdentity,
    InvalidTrialDesignReceipt,
    StaleTrialDesignReceipt,
    OptionalStoppingNotAllowed,
    TrialDesignIdentityMismatch,
    EmptyCategories,
    TooManyCategories,
    InvalidAlpha,
    AlphaAllocationTooSmall,
    DuplicateCategory,
    InvalidRiskLimit,
    InvalidNoninferiorityMargin,
    EmptyTrialBatch,
    TooManyTrials,
    EmptyTrialIdentifier,
    DuplicateTrialId,
    DuplicateIndependentUnitId,
    DuplicateReceiptId,
    InvalidTrialOutcome,
    DuplicateCategoryOutcome,
    UnknownCategoryOutcome,
    ImpossibleEventCount,
    ArithmeticOverflow,
    InvalidBaselineIdentity,
}

#[derive(Debug, Clone)]
struct BatchIndex<'a> {
    by_unit: BTreeMap<&'a str, &'a TrialRecord>,
    total_control_ticks: u64,
}

#[derive(Debug, Clone, Copy)]
struct AlphaAllocation {
    absolute_ppm: u32,
    paired_ppm: Option<u32>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn round_upper_ppm(probability: f64) -> Result<u32, RiskBoundError> {
    if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
        return Err(RiskBoundError::ArithmeticOverflow);
    }
    let rounded = (probability * f64::from(PPM_SCALE)).ceil();
    let guarded = rounded + f64::from(NUMERIC_OUTWARD_GUARD_PPM);
    Ok((guarded as u64).min(u64::from(PPM_SCALE)) as u32)
}

fn round_signed_upper_ppm(value: f64) -> Result<i32, RiskBoundError> {
    if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
        return Err(RiskBoundError::ArithmeticOverflow);
    }
    let rounded = (value * f64::from(PPM_SCALE)).ceil()
        + f64::from(NUMERIC_OUTWARD_GUARD_PPM);
    Ok((rounded as i64).clamp(-i64::from(PPM_SCALE), i64::from(PPM_SCALE)) as i32)
}

/// Computes a one-sided upper bound. The zero-event branch is the exact
/// Clopper-Pearson closed form; k>0 uses a conservative Hoeffding bound.
pub fn one_sided_event_upper_bound_ppm(
    n: u64,
    k: u64,
    alpha_ppm: u32,
) -> Result<(u32, BoundMethod), RiskBoundError> {
    if n == 0 || k > n {
        return Err(RiskBoundError::ImpossibleEventCount);
    }
    if alpha_ppm == 0 || alpha_ppm >= PPM_SCALE {
        return Err(RiskBoundError::InvalidAlpha);
    }
    let alpha = f64::from(alpha_ppm) / f64::from(PPM_SCALE);
    let probability = if k == 0 {
        let exponent = alpha.ln() / n as f64;
        -exponent.exp_m1()
    } else {
        let observed = k as f64 / n as f64;
        let margin = ((-alpha.ln()) / (2.0 * n as f64)).sqrt();
        (observed + margin).min(1.0)
    };
    let method = if k == 0 {
        BoundMethod::ExactZeroEventBinomial
    } else {
        BoundMethod::HoeffdingBernoulli
    };
    Ok((round_upper_ppm(probability)?, method))
}

fn paired_difference_upper_bound_ppm(
    n: u64,
    candidate_only: u64,
    baseline_only: u64,
    alpha_ppm: u32,
) -> Result<i32, RiskBoundError> {
    if n == 0 || candidate_only > n || baseline_only > n {
        return Err(RiskBoundError::ImpossibleEventCount);
    }
    if alpha_ppm == 0 || alpha_ppm >= PPM_SCALE {
        return Err(RiskBoundError::InvalidAlpha);
    }
    let alpha = f64::from(alpha_ppm) / f64::from(PPM_SCALE);
    let mean_difference = (candidate_only as f64 - baseline_only as f64) / n as f64;
    let margin = ((2.0 * -alpha.ln()) / n as f64).sqrt();
    round_signed_upper_ppm((mean_difference + margin).min(1.0))
}

fn validate_design_receipt(
    now_ns: u64,
    identity: &CellIdentity,
    receipt: &TrialDesignReceipt,
) -> Result<(), RiskBoundError> {
    if !nonempty(&receipt.receipt_id)
        || !nonempty(&receipt.cluster_definition_digest)
        || !nonempty(&receipt.sampling_frame_digest)
        || !nonempty(&receipt.randomization_protocol_digest)
        || !nonempty(&receipt.independence_review_digest)
        || !nonempty(&receipt.result_digest)
        || receipt.valid_from_ns > receipt.valid_until_ns
    {
        return Err(RiskBoundError::InvalidTrialDesignReceipt);
    }
    if now_ns < receipt.valid_from_ns || now_ns > receipt.valid_until_ns {
        return Err(RiskBoundError::StaleTrialDesignReceipt);
    }
    if !receipt.fixed_sample_stop_rule {
        return Err(RiskBoundError::OptionalStoppingNotAllowed);
    }
    if &receipt.identity != identity {
        return Err(RiskBoundError::TrialDesignIdentityMismatch);
    }
    Ok(())
}

fn validate_categories(
    categories: &[CategorySpec],
) -> Result<Vec<CategorySpec>, RiskBoundError> {
    if categories.is_empty() {
        return Err(RiskBoundError::EmptyCategories);
    }
    if categories.len() > MAX_CATEGORIES {
        return Err(RiskBoundError::TooManyCategories);
    }
    let mut sorted = categories.to_vec();
    sorted.sort_by(|left, right| left.category_id.cmp(&right.category_id));
    let mut ids = BTreeSet::new();
    for category in &sorted {
        if !nonempty(&category.category_id) || !ids.insert(category.category_id.as_str()) {
            return Err(RiskBoundError::DuplicateCategory);
        }
        if category.maximum_event_rate_ppm > PPM_SCALE {
            return Err(RiskBoundError::InvalidRiskLimit);
        }
        if !(-1_000_000..=1_000_000).contains(&category.noninferiority_margin_ppm) {
            return Err(RiskBoundError::InvalidNoninferiorityMargin);
        }
    }
    Ok(sorted)
}

fn allocate_alpha(
    family_alpha_ppm: u32,
    category_count: usize,
    paired: bool,
) -> Result<Vec<AlphaAllocation>, RiskBoundError> {
    if family_alpha_ppm == 0 || family_alpha_ppm >= PPM_SCALE {
        return Err(RiskBoundError::InvalidAlpha);
    }
    let count = u32::try_from(category_count).map_err(|_| RiskBoundError::ArithmeticOverflow)?;
    let quotient = family_alpha_ppm / count;
    let remainder = family_alpha_ppm % count;
    let mut allocations = Vec::with_capacity(category_count);
    for index in 0..category_count {
        let category_alpha = quotient + u32::from((index as u32) < remainder);
        if category_alpha == 0 {
            return Err(RiskBoundError::AlphaAllocationTooSmall);
        }
        if paired {
            let absolute = category_alpha.div_ceil(2);
            let paired_alpha = category_alpha / 2;
            if absolute == 0 || paired_alpha == 0 {
                return Err(RiskBoundError::AlphaAllocationTooSmall);
            }
            allocations.push(AlphaAllocation {
                absolute_ppm: absolute,
                paired_ppm: Some(paired_alpha),
            });
        } else {
            allocations.push(AlphaAllocation {
                absolute_ppm: category_alpha,
                paired_ppm: None,
            });
        }
    }
    Ok(allocations)
}

fn index_batch<'a>(
    batch: &'a TrialBatch,
    categories: &[CategorySpec],
) -> Result<BatchIndex<'a>, RiskBoundError> {
    if batch.trials.is_empty() {
        return Err(RiskBoundError::EmptyTrialBatch);
    }
    if batch.trials.len() > MAX_TRIALS {
        return Err(RiskBoundError::TooManyTrials);
    }
    let category_ids: BTreeSet<&str> = categories
        .iter()
        .map(|item| item.category_id.as_str())
        .collect();
    let mut trial_ids = BTreeSet::new();
    let mut unit_ids = BTreeSet::new();
    let mut by_unit = BTreeMap::new();
    let mut total_ticks = 0_u64;

    for trial in &batch.trials {
        if !nonempty(&trial.trial_id)
            || !nonempty(&trial.independent_unit_id)
            || !nonempty(&trial.session_id)
            || !nonempty(&trial.training_seed_id)
            || !nonempty(&trial.hardware_unit_id)
            || trial.control_tick_count == 0
        {
            return Err(RiskBoundError::EmptyTrialIdentifier);
        }
        if !trial_ids.insert(trial.trial_id.as_str()) {
            return Err(RiskBoundError::DuplicateTrialId);
        }
        if !unit_ids.insert(trial.independent_unit_id.as_str()) {
            return Err(RiskBoundError::DuplicateIndependentUnitId);
        }
        total_ticks = total_ticks
            .checked_add(trial.control_tick_count)
            .ok_or(RiskBoundError::ArithmeticOverflow)?;

        let mut observed_categories = BTreeSet::new();
        for outcome in &trial.outcomes {
            if !category_ids.contains(outcome.category_id.as_str()) {
                return Err(RiskBoundError::UnknownCategoryOutcome);
            }
            if !observed_categories.insert(outcome.category_id.as_str()) {
                return Err(RiskBoundError::DuplicateCategoryOutcome);
            }
        }
        if trial.outcomes.len() != categories.len() {
            return Err(RiskBoundError::InvalidTrialOutcome);
        }
        if category_ids != observed_categories {
            return Err(RiskBoundError::InvalidTrialOutcome);
        }
        by_unit.insert(trial.independent_unit_id.as_str(), trial);
    }

    Ok(BatchIndex {
        by_unit,
        total_control_ticks: total_ticks,
    })
}

fn matched_design(
    candidate: &TrialBatch,
    baseline: &TrialBatch,
    candidate_index: &BatchIndex<'_>,
    baseline_index: &BatchIndex<'_>,
) -> Result<Option<String>, RiskBoundError> {
    if !candidate
        .identity
        .same_cell_except_controller(&baseline.identity)
        || candidate.identity.controller_digest == baseline.identity.controller_digest
    {
        return Ok(Some("candidate/baseline cell identity does not match or controller digests are identical".to_owned()));
    }
    let left = &candidate.design_receipt;
    let right = &baseline.design_receipt;
    if left.trial_unit_kind != right.trial_unit_kind
        || left.cluster_definition_digest != right.cluster_definition_digest
        || left.sampling_frame_digest != right.sampling_frame_digest
        || left.randomization_protocol_digest != right.randomization_protocol_digest
        || left.independence_review_digest != right.independence_review_digest
    {
        return Ok(Some("candidate/baseline trial-design identities differ".to_owned()));
    }
    if candidate_index.by_unit.keys().ne(baseline_index.by_unit.keys()) {
        return Ok(Some("candidate/baseline independent-unit IDs do not form exact pairs".to_owned()));
    }
    for (unit_id, candidate_trial) in &candidate_index.by_unit {
        let baseline_trial = baseline_index
            .by_unit
            .get(unit_id)
            .ok_or(RiskBoundError::InvalidBaselineIdentity)?;
        if candidate_trial.session_id != baseline_trial.session_id
            || candidate_trial.training_seed_id != baseline_trial.training_seed_id
            || candidate_trial.hardware_unit_id != baseline_trial.hardware_unit_id
        {
            return Ok(Some("paired independent units differ in session, training seed or hardware unit".to_owned()));
        }
    }
    Ok(None)
}

fn event_map<'a>(trial: &'a TrialRecord) -> BTreeMap<&'a str, Option<bool>> {
    trial
        .outcomes
        .iter()
        .map(|outcome| (outcome.category_id.as_str(), outcome.event_observed))
        .collect()
}

pub fn calculate_risk_bounds(
    problem: &RiskBoundProblem,
) -> Result<RiskBoundReport, RiskBoundError> {
    problem.candidate.identity.validate()?;
    validate_design_receipt(
        problem.now_ns,
        &problem.candidate.identity,
        &problem.candidate.design_receipt,
    )?;
    let categories = validate_categories(&problem.categories)?;
    let candidate_index = index_batch(&problem.candidate, &categories)?;

    let baseline_index_result = if let Some(baseline) = &problem.baseline {
        baseline.identity.validate()?;
        validate_design_receipt(problem.now_ns, &baseline.identity, &baseline.design_receipt)?;
        Some(index_batch(baseline, &categories)?)
    } else {
        None
    };

    let matched_reason = match (&problem.baseline, &baseline_index_result) {
        (Some(baseline), Some(index)) => {
            matched_design(&problem.candidate, baseline, &candidate_index, index)?
        }
        _ => Some("no baseline batch supplied".to_owned()),
    };
    let matched = matched_reason.is_none();
    let alpha_allocations =
        allocate_alpha(problem.family_alpha_ppm, categories.len(), matched)?;

    let mut unknown_reasons = Vec::new();
    for trial in problem.candidate.trials.iter() {
        if trial.outcomes.iter().any(|outcome| outcome.event_observed.is_none()) {
            unknown_reasons.push(format!(
                "candidate independent unit '{}' has unknown category coverage",
                trial.independent_unit_id
            ));
        }
    }
    if matched {
        for trial in problem.baseline.as_ref().expect("matched baseline").trials.iter() {
            if trial.outcomes.iter().any(|outcome| outcome.event_observed.is_none()) {
                unknown_reasons.push(format!(
                    "baseline independent unit '{}' has unknown category coverage",
                    trial.independent_unit_id
                ));
            }
        }
    }

    if !unknown_reasons.is_empty() {
        unknown_reasons.sort();
        return Ok(RiskBoundReport {
            evidence_status: EvidenceStatus::Unknown,
            candidate_receipt_id: problem.candidate.design_receipt.receipt_id.clone(),
            baseline_receipt_id: problem.baseline.as_ref().map(|item| item.design_receipt.receipt_id.clone()),
            family_alpha_ppm: problem.family_alpha_ppm,
            numeric_outward_guard_ppm: NUMERIC_OUTWARD_GUARD_PPM,
            candidate_independent_unit_count: candidate_index.by_unit.len() as u64,
            candidate_trial_record_count: problem.candidate.trials.len() as u64,
            candidate_control_tick_count_metadata: candidate_index.total_control_ticks,
            matched_baseline_eligible: matched,
            baseline_comparison_reason: matched_reason,
            unknown_reasons,
            categories: Vec::new(),
        });
    }

    let mut category_reports = Vec::with_capacity(categories.len());
    for (index, category) in categories.iter().enumerate() {
        let mut event_count = 0_u64;
        for trial in problem.candidate.trials.iter() {
            let value = trial
                .outcomes
                .iter()
                .find(|outcome| outcome.category_id == category.category_id)
                .and_then(|outcome| outcome.event_observed)
                .ok_or(RiskBoundError::InvalidTrialOutcome)?;
            if value {
                event_count = event_count
                    .checked_add(1)
                    .ok_or(RiskBoundError::ArithmeticOverflow)?;
            }
        }
        let n = u64::try_from(candidate_index.by_unit.len())
            .map_err(|_| RiskBoundError::ArithmeticOverflow)?;
        let allocation = alpha_allocations[index];
        let (upper_bound_ppm, method) =
            one_sided_event_upper_bound_ppm(n, event_count, allocation.absolute_ppm)?;
        let absolute_limit_pass = upper_bound_ppm <= category.maximum_event_rate_ppm;

        let mut paired_status = PairedStatus::Withheld;
        let mut paired_table = None;
        let mut paired_upper = None;
        let mut paired_n = None;
        let mut paired_noninferiority_pass = None;

        if matched {
            let baseline = problem.baseline.as_ref().expect("matched baseline");
            let baseline_index = baseline_index_result.as_ref().expect("matched index");
            let mut neither = 0_u64;
            let mut candidate_only = 0_u64;
            let mut baseline_only = 0_u64;
            let mut both = 0_u64;
            for (unit_id, candidate_trial) in &candidate_index.by_unit {
                let baseline_trial = baseline_index
                    .by_unit
                    .get(unit_id)
                    .ok_or(RiskBoundError::InvalidBaselineIdentity)?;
                let candidate_value = event_map(candidate_trial)
                    .get(category.category_id.as_str())
                    .copied()
                    .flatten()
                    .ok_or(RiskBoundError::InvalidTrialOutcome)?;
                let baseline_value = event_map(baseline_trial)
                    .get(category.category_id.as_str())
                    .copied()
                    .flatten()
                    .ok_or(RiskBoundError::InvalidTrialOutcome)?;
                match (candidate_value, baseline_value) {
                    (false, false) => neither += 1,
                    (true, false) => candidate_only += 1,
                    (false, true) => baseline_only += 1,
                    (true, true) => both += 1,
                }
            }
            let pair_alpha = allocation
                .paired_ppm
                .ok_or(RiskBoundError::AlphaAllocationTooSmall)?;
            let paired_bound = paired_difference_upper_bound_ppm(
                n,
                candidate_only,
                baseline_only,
                pair_alpha,
            )?;
            paired_status = PairedStatus::Valid;
            paired_table = Some(PairedEventTable {
                neither,
                candidate_only,
                baseline_only,
                both,
            });
            paired_upper = Some(paired_bound);
            paired_n = Some(n);
            paired_noninferiority_pass =
                Some(paired_bound <= category.noninferiority_margin_ppm);
            let _ = baseline;
        }

        category_reports.push(CategoryRiskReport {
            category_id: category.category_id.clone(),
            independent_unit_count: n,
            event_unit_count: event_count,
            control_tick_count_metadata: candidate_index.total_control_ticks,
            absolute_alpha_ppm: allocation.absolute_ppm,
            upper_bound_ppm,
            method,
            maximum_event_rate_ppm: category.maximum_event_rate_ppm,
            absolute_limit_pass,
            paired_status,
            paired_alpha_ppm: allocation.paired_ppm,
            paired_unit_count: paired_n,
            paired_event_table: paired_table,
            paired_difference_upper_bound_ppm: paired_upper,
            noninferiority_margin_ppm: category.noninferiority_margin_ppm,
            paired_noninferiority_pass,
        });
    }

    Ok(RiskBoundReport {
        evidence_status: EvidenceStatus::Valid,
        candidate_receipt_id: problem.candidate.design_receipt.receipt_id.clone(),
        baseline_receipt_id: problem.baseline.as_ref().map(|item| item.design_receipt.receipt_id.clone()),
        family_alpha_ppm: problem.family_alpha_ppm,
        numeric_outward_guard_ppm: NUMERIC_OUTWARD_GUARD_PPM,
        candidate_independent_unit_count: candidate_index.by_unit.len() as u64,
        candidate_trial_record_count: problem.candidate.trials.len() as u64,
        candidate_control_tick_count_metadata: candidate_index.total_control_ticks,
        matched_baseline_eligible: matched,
        baseline_comparison_reason: matched_reason,
        unknown_reasons: Vec::new(),
        categories: category_reports,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(controller: &str) -> CellIdentity {
        CellIdentity {
            scenario_digest: "scenario-v1".to_owned(),
            morphology_digest: "go2-morph-v1".to_owned(),
            evidence_tier: EvidenceTier::Hardware,
            platform_physics_digest: "go2-physics-v1".to_owned(),
            dynamics_model_digest: "go2-dynamics-v1".to_owned(),
            transition_abstraction_digest: "go2-abstraction-v1".to_owned(),
            controller_digest: controller.to_owned(),
            operating_mode_id: "full-perception-v1".to_owned(),
            task_digest: "stairs-task-v1".to_owned(),
            terrain_cell_id: "stair-20cm-v1".to_owned(),
            sensor_health_digest: "health-mode-v1".to_owned(),
            calibration_digest: "cal-v1".to_owned(),
            taxonomy_digest: "taxonomy-v1".to_owned(),
            detector_digest: "detector-v1".to_owned(),
            actuator_digest: "actuator-v1".to_owned(),
            initial_state_digest: "state-v1".to_owned(),
            payload_digest: "payload-v1".to_owned(),
            energy_boundary_id: "battery-terminal-v1".to_owned(),
            exposure_unit_id: "crossings".to_owned(),
            exposure_value: 100,
            heldout_manifest_digest: "heldout-v1".to_owned(),
        }
    }

    fn design_receipt(receipt_id: &str, identity: &CellIdentity) -> TrialDesignReceipt {
        TrialDesignReceipt {
            receipt_id: receipt_id.to_owned(),
            identity: identity.clone(),
            trial_unit_kind: TrialUnitKind::Traversal,
            cluster_definition_digest: "independent-traversal-v1".to_owned(),
            sampling_frame_digest: "stairs-frame-v1".to_owned(),
            randomization_protocol_digest: "randomization-v1".to_owned(),
            independence_review_digest: "independence-review-v1".to_owned(),
            fixed_sample_stop_rule: true,
            valid_from_ns: 100,
            valid_until_ns: 1_000,
            result_digest: format!("result-{receipt_id}"),
        }
    }

    fn trial(
        id: &str,
        unit: &str,
        events: &[(&str, Option<bool>)],
        ticks: u64,
    ) -> TrialRecord {
        TrialRecord {
            trial_id: id.to_owned(),
            independent_unit_id: unit.to_owned(),
            session_id: "session-1".to_owned(),
            training_seed_id: "seed-1".to_owned(),
            hardware_unit_id: "robot-1".to_owned(),
            control_tick_count: ticks,
            outcomes: events
                .iter()
                .map(|(category_id, event_observed)| CategoryOutcome {
                    category_id: (*category_id).to_owned(),
                    event_observed: *event_observed,
                })
                .collect(),
        }
    }

    fn category(id: &str) -> CategorySpec {
        CategorySpec {
            category_id: id.to_owned(),
            maximum_event_rate_ppm: 100_000,
            noninferiority_margin_ppm: 50_000,
        }
    }

    fn batch(controller: &str, receipt: &str, trials: Vec<TrialRecord>) -> TrialBatch {
        let identity = identity(controller);
        TrialBatch {
            identity: identity.clone(),
            design_receipt: design_receipt(receipt, &identity),
            trials,
        }
    }

    fn common_trials(fall_events: &[Option<bool>]) -> Vec<TrialRecord> {
        fall_events
            .iter()
            .enumerate()
            .map(|(index, fall)| {
                trial(
                    &format!("trial-{index}"),
                    &format!("unit-{index}"),
                    &[
                        ("fall", *fall),
                        ("foot_scuff", Some(false)),
                    ],
                    500,
                )
            })
            .collect()
    }

    #[test]
    fn zero_event_bound_n1_is_about_95_percent() {
        let (bound, method) =
            one_sided_event_upper_bound_ppm(1, 0, 50_000).expect("zero event bound");
        assert_eq!(method, BoundMethod::ExactZeroEventBinomial);
        assert!((950_000..=950_002).contains(&bound));
    }

    #[test]
    fn zero_event_bound_n299_is_below_one_percent() {
        let (bound, _) =
            one_sided_event_upper_bound_ppm(299, 0, 50_000).expect("bound");
        assert!(bound < 10_000);
    }

    #[test]
    fn zero_event_bound_n2995_is_below_point_one_percent() {
        let (bound, _) =
            one_sided_event_upper_bound_ppm(2_995, 0, 50_000).expect("bound");
        assert!(bound < 1_000);
    }

    #[test]
    fn nonzero_event_bound_uses_hoeffding_and_exceeds_observed_rate() {
        let (bound, method) =
            one_sided_event_upper_bound_ppm(10, 1, 50_000).expect("bound");
        assert_eq!(method, BoundMethod::HoeffdingBernoulli);
        assert!(bound >= 100_000);
        assert!(bound <= 1_000_000);
    }

    #[test]
    fn bonferroni_allocation_sums_exactly() {
        let allocations = allocate_alpha(50_000, 5, true).expect("allocation");
        let absolute: u32 = allocations.iter().map(|item| item.absolute_ppm).sum();
        let paired: u32 = allocations
            .iter()
            .map(|item| item.paired_ppm.unwrap_or(0))
            .sum();
        assert_eq!(absolute + paired, 50_000);
        assert!(allocations.iter().all(|item| item.absolute_ppm > 0 && item.paired_ppm.unwrap_or(0) > 0));
    }

    #[test]
    fn alpha_too_small_for_all_tests_is_rejected() {
        assert_eq!(
            allocate_alpha(4, 5, true),
            Err(RiskBoundError::AlphaAllocationTooSmall)
        );
    }

    #[test]
    fn control_ticks_do_not_inflate_independent_trial_count() {
        let batch = batch(
            "candidate",
            "candidate-design",
            vec![trial(
                "traversal-1",
                "traversal-1",
                &[("fall", Some(false)), ("foot_scuff", Some(false))],
                10_000,
            )],
        );
        let problem = RiskBoundProblem {
            now_ns: 500,
            family_alpha_ppm: 50_000,
            categories: vec![category("fall"), category("foot_scuff")],
            candidate: batch,
            baseline: None,
        };
        let report = calculate_risk_bounds(&problem).expect("risk report");
        assert_eq!(report.candidate_independent_unit_count, 1);
        assert_eq!(report.candidate_control_tick_count_metadata, 10_000);
        assert_eq!(report.categories[0].independent_unit_count, 1);
    }

    #[test]
    fn duplicate_independent_unit_is_rejected() {
        let mut batch = batch(
            "candidate",
            "candidate-design",
            common_trials(&[Some(false), Some(false)]),
        );
        batch.trials[1].independent_unit_id = batch.trials[0].independent_unit_id.clone();
        assert_eq!(
            index_batch(&batch, &[category("fall"), category("foot_scuff")]),
            Err(RiskBoundError::DuplicateIndependentUnitId)
        );
    }

    #[test]
    fn unknown_event_coverage_returns_unknown_not_event_free() {
        let candidate = batch(
            "candidate",
            "candidate-design",
            vec![trial(
                "t1",
                "u1",
                &[("fall", None), ("foot_scuff", Some(false))],
                100,
            )],
        );
        let report = calculate_risk_bounds(&RiskBoundProblem {
            now_ns: 500,
            family_alpha_ppm: 50_000,
            categories: vec![category("fall"), category("foot_scuff")],
            candidate,
            baseline: None,
        })
        .expect("unknown report");
        assert_eq!(report.evidence_status, EvidenceStatus::Unknown);
        assert!(report.categories.is_empty());
        assert!(!report.unknown_reasons.is_empty());
    }

    #[test]
    fn mismatched_baseline_units_withhold_pairing_but_keep_candidate_bound() {
        let candidate = batch(
            "candidate",
            "candidate-design",
            common_trials(&[Some(false), Some(false)]),
        );
        let mut baseline = batch(
            "baseline",
            "baseline-design",
            common_trials(&[Some(false), Some(false)]),
        );
        baseline.trials[1].independent_unit_id = "another-unit".to_owned();
        let report = calculate_risk_bounds(&RiskBoundProblem {
            now_ns: 500,
            family_alpha_ppm: 50_000,
            categories: vec![category("fall"), category("foot_scuff")],
            candidate,
            baseline: Some(baseline),
        })
        .expect("withheld baseline");
        assert!(!report.matched_baseline_eligible);
        assert!(report.categories.iter().all(|item| item.paired_status == PairedStatus::Withheld));
        assert!(report.categories.iter().all(|item| item.upper_bound_ppm > 0));
    }

    #[test]
    fn candidate_baseline_pairs_use_full_two_by_two_table() {
        let candidate = batch(
            "candidate",
            "candidate-design",
            vec![
                trial("c1", "u1", &[("fall", Some(true)), ("foot_scuff", Some(false))], 100),
                trial("c2", "u2", &[("fall", Some(false)), ("foot_scuff", Some(false))], 100),
            ],
        );
        let baseline = batch(
            "baseline",
            "baseline-design",
            vec![
                trial("b1", "u1", &[("fall", Some(false)), ("foot_scuff", Some(false))], 100),
                trial("b2", "u2", &[("fall", Some(true)), ("foot_scuff", Some(false))], 100),
            ],
        );
        let report = calculate_risk_bounds(&RiskBoundProblem {
            now_ns: 500,
            family_alpha_ppm: 50_000,
            categories: vec![category("fall"), category("foot_scuff")],
            candidate,
            baseline: Some(baseline),
        })
        .expect("paired report");
        assert!(report.matched_baseline_eligible);
        let fall = report.categories.iter().find(|item| item.category_id == "fall").expect("fall");
        let table = fall.paired_event_table.as_ref().expect("2x2 table");
        assert_eq!(table.neither, 0);
        assert_eq!(table.candidate_only, 1);
        assert_eq!(table.baseline_only, 1);
        assert_eq!(table.both, 0);
        assert_eq!(fall.paired_unit_count, Some(2));
    }

    #[test]
    fn optional_stopping_is_rejected_for_fixed_sample_bound() {
        let mut candidate = batch(
            "candidate",
            "candidate-design",
            common_trials(&[Some(false)]),
        );
        candidate.design_receipt.fixed_sample_stop_rule = false;
        assert_eq!(
            validate_design_receipt(500, &candidate.identity, &candidate.design_receipt),
            Err(RiskBoundError::OptionalStoppingNotAllowed)
        );
    }

    #[test]
    fn candidate_and_baseline_controller_identity_must_differ() {
        let candidate = batch(
            "same-controller",
            "candidate-design",
            common_trials(&[Some(false)]),
        );
        let baseline = batch(
            "same-controller",
            "baseline-design",
            common_trials(&[Some(false)]),
        );
        let problem = RiskBoundProblem {
            now_ns: 500,
            family_alpha_ppm: 50_000,
            categories: vec![category("fall"), category("foot_scuff")],
            candidate,
            baseline: Some(baseline),
        };
        let report = calculate_risk_bounds(&problem).expect("mismatched baseline");
        assert!(!report.matched_baseline_eligible);
        assert!(report.baseline_comparison_reason.is_some());
    }

    #[test]
    fn trial_input_permutation_preserves_canonical_category_report() {
        let mut candidate = batch(
            "candidate",
            "candidate-design",
            common_trials(&[Some(false), Some(true), Some(false)]),
        );
        let categories = vec![category("fall"), category("foot_scuff")];
        let original = calculate_risk_bounds(&RiskBoundProblem {
            now_ns: 500,
            family_alpha_ppm: 50_000,
            categories: categories.clone(),
            candidate: candidate.clone(),
            baseline: None,
        })
        .expect("original");
        candidate.trials.reverse();
        for trial in &mut candidate.trials {
            trial.outcomes.reverse();
        }
        let permuted = calculate_risk_bounds(&RiskBoundProblem {
            now_ns: 500,
            family_alpha_ppm: 50_000,
            categories,
            candidate,
            baseline: None,
        })
        .expect("permuted");
        assert_eq!(original, permuted);
    }
}
