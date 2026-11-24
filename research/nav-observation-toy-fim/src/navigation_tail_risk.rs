//! Exact discrete tail-risk evaluation for finite navigation mission outcomes.
//!
//! This module evaluates a fixed policy's finite loss distribution. It does
//! not optimize a policy tree, estimate physical probabilities, or grant
//! navigation authority. Persistent-model semantics are represented by
//! evaluating each complete mission-model distribution separately and taking
//! the outer maximum; models are never re-sampled inside a distribution.

use core::cmp::Ordering;

use crate::navigation_temporal_mission_minimax::{
    ActionOutcome, ObservableBranchChoice, ObservablePolicy, TemporalMinimaxProblem,
    TerminalDecision, TimedJointAction, PROBABILITY_SCALE,
};

pub const PROBABILITY_SCALE_PPM: u128 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LossMass {
    /// Terminal mission loss in caller-defined, non-negative integer units.
    pub loss: u64,
    /// Non-negative integer probability mass. All entries in one distribution
    /// share an arbitrary common denominator; normalization is unnecessary.
    pub mass: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactRisk {
    numerator: u128,
    denominator: u128,
}

impl ExactRisk {
    /// Construct and reduce a non-negative exact fraction. A zero denominator
    /// is rejected; all successful values are safe for exact comparison.
    pub fn from_ratio(numerator: u128, denominator: u128) -> Result<Self, TailRiskError> {
        Self::new(numerator, denominator)
    }

    fn new(numerator: u128, denominator: u128) -> Result<Self, TailRiskError> {
        if denominator == 0 {
            return Err(TailRiskError::ZeroTotalMass);
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub const fn numerator(self) -> u128 {
        self.numerator
    }

    pub const fn denominator(self) -> u128 {
        self.denominator
    }

    /// Compare two non-negative exact fractions without cross multiplication,
    /// avoiding overflow even when both fractions are close to u128 limits.
    pub fn compare(self, other: Self) -> Ordering {
        compare_fractions(
            self.numerator,
            self.denominator,
            other.numerator,
            other.denominator,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TailRiskError {
    EmptyDistribution,
    ZeroTotalMass,
    InvalidAlpha(u32),
    DuplicateModelId(u64),
    InvalidModelId,
    InvalidBranchId,
    DuplicateBranchId(u64),
    PersistentPolicyMismatch,
    InvalidParticleId,
    DuplicateParticleId(u64),
    InvalidTailMass(u32),
    MissingAction(u64),
    InvalidPolicyTree,
    ArithmeticOverflow,
}

impl core::fmt::Display for TailRiskError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyDistribution => write!(f, "loss distribution must not be empty"),
            Self::ZeroTotalMass => write!(f, "loss distribution must have positive total mass"),
            Self::InvalidAlpha(alpha) => write!(
                f,
                "CVaR alpha must be below one million PPM, got {alpha}"
            ),
            Self::DuplicateModelId(id) => write!(f, "mission model identity is duplicated: {id}"),
            Self::InvalidModelId => write!(f, "mission model identity must be nonzero"),
            Self::InvalidBranchId => write!(f, "observable branch identity and probability mass must be nonzero"),
            Self::DuplicateBranchId(id) => write!(f, "observable branch identity is duplicated: {id}"),
            Self::PersistentPolicyMismatch => write!(f, "persistent models do not share the same observable branch-to-action policy"),
            Self::InvalidParticleId => write!(f, "belief particle identity must be nonzero"),
            Self::DuplicateParticleId(id) => write!(f, "belief particle identity is duplicated: {id}"),
            Self::InvalidTailMass(mass) => write!(f, "belief CVaR tail mass must be in 1..=1000000 PPM, got {mass}"),
            Self::MissingAction(id) => write!(f, "policy references an unavailable action: {id}"),
            Self::InvalidPolicyTree => write!(f, "observable policy tree is incomplete or inconsistent"),
            Self::ArithmeticOverflow => write!(f, "checked exact tail-risk arithmetic overflowed"),
        }
    }
}

impl std::error::Error for TailRiskError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelLossDistribution<'a> {
    /// Stable identity of one complete mission-persistent model.
    pub model_id: u64,
    /// Complete stochastic mission-path loss distribution under one fixed
    /// model and one fixed observable policy.
    pub outcomes: &'a [LossMass],
}

/// One weighted particle in the current belief for a state-dependent
/// immediate action cost. The caller must construct these particles using
/// only information available at the current decision epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeliefCostParticle {
    /// Stable, nonzero identity unique within this belief snapshot.
    pub particle_id: u64,
    /// Immediate cost if the hidden state represented by this particle is true.
    pub immediate_cost: u64,
    /// Non-negative importance / posterior weight. Weights need not sum to a
    /// particular constant, but their total must be positive.
    pub weight: u128,
}

/// Risk of the immediate state-dependent cost under one current belief.
///
/// This is the belief-state risk layer, not a full mission-path risk measure.
/// It evaluates the cost distribution across hidden states represented by the
/// belief particles. Call it separately for each persistent candidate model;
/// do not combine particles from different epistemic models before the
/// outer robust-model aggregation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeliefImmediateRisk {
    pub expected_cost: ExactRisk,
    pub cvar_cost: ExactRisk,
    /// Upper-tail probability mass in PPM. 50_000 means the worst 5%.
    pub tail_mass_ppm: u32,
    pub particle_count: usize,
}

/// Evaluate state uncertainty in one current belief using upper-tail CVaR of
/// the immediate state-dependent cost.
///
/// The public input uses tail mass (e.g. 50_000 for the worst 5%), matching
/// the convention in Pariente & Indelman's CVaR-cost POMDP formulation. The
/// lower-level cvar() API uses confidence alpha (e.g. 950_000 for CVaR95), so
/// this function performs the explicit conversion. This API does not optimize
/// future trajectory return or replace mission-path CVaR.
pub fn belief_immediate_cvar(
    particles: &[BeliefCostParticle],
    tail_mass_ppm: u32,
) -> Result<BeliefImmediateRisk, TailRiskError> {
    if particles.is_empty() {
        return Err(TailRiskError::EmptyDistribution);
    }
    if tail_mass_ppm == 0 || tail_mass_ppm as u128 > PROBABILITY_SCALE_PPM {
        return Err(TailRiskError::InvalidTailMass(tail_mass_ppm));
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut costs = Vec::with_capacity(particles.len());
    for particle in particles {
        if particle.particle_id == 0 {
            return Err(TailRiskError::InvalidParticleId);
        }
        if !seen.insert(particle.particle_id) {
            return Err(TailRiskError::DuplicateParticleId(particle.particle_id));
        }
        costs.push(LossMass {
            loss: particle.immediate_cost,
            mass: particle.weight,
        });
    }

    let expected = expected_loss(&costs)?;
    let confidence_alpha_ppm =
        u32::try_from(PROBABILITY_SCALE_PPM - tail_mass_ppm as u128)
            .map_err(|_| TailRiskError::ArithmeticOverflow)?;
    let tail = cvar(&costs, confidence_alpha_ppm)?;

    Ok(BeliefImmediateRisk {
        expected_cost: expected,
        cvar_cost: tail,
        tail_mass_ppm,
        particle_count: particles.len(),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelTailRisk {
    pub model_id: u64,
    pub cvar: ExactRisk,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RobustTailRisk {
    /// Sorted by model identity for deterministic reporting.
    pub per_model: Vec<ModelTailRisk>,
    /// Worst conditional CVaR over complete mission-persistent models.
    pub worst_case_model_id: u64,
    pub worst_case_cvar: ExactRisk,
    pub alpha_ppm: u32,
}

/// Exact expected loss of a finite discrete distribution.
///
/// mass is interpreted as an arbitrary non-negative integer weight. The
/// resulting fraction is reduced, so all distributions can use their own
/// common integer denominator.
pub fn expected_loss(outcomes: &[LossMass]) -> Result<ExactRisk, TailRiskError> {
    let total_mass = total_mass(outcomes)?;
    let mut weighted_loss = 0_u128;
    for outcome in outcomes {
        let contribution = outcome
            .mass
            .checked_mul(outcome.loss as u128)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
        weighted_loss = weighted_loss
            .checked_add(contribution)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
    }
    ExactRisk::new(weighted_loss, total_mass)
}

/// Exact upper-tail conditional value at risk for a discrete loss distribution.
///
/// alpha_ppm is the confidence level in parts per million: 950_000 means
/// CVaR_0.95. A fractional atom at the quantile is included exactly, rather
/// than rounding the tail count to a whole observation.
///
/// The returned fraction is the average loss in the upper (1 - alpha) tail.
/// All arithmetic is integer and checked. The function assumes the supplied
/// weights represent a valid finite distribution with one common denominator;
/// it does not validate the physical or statistical provenance of those
/// probabilities.
pub fn cvar(
    outcomes: &[LossMass],
    alpha_ppm: u32,
) -> Result<ExactRisk, TailRiskError> {
    if alpha_ppm as u128 >= PROBABILITY_SCALE_PPM {
        return Err(TailRiskError::InvalidAlpha(alpha_ppm));
    }
    let total_mass = total_mass(outcomes)?;
    let tail_ppm = PROBABILITY_SCALE_PPM - alpha_ppm as u128;
    let target_scaled_mass = total_mass
        .checked_mul(tail_ppm)
        .ok_or(TailRiskError::ArithmeticOverflow)?;

    // Sorting a copy keeps caller ordering and data unchanged.
    let mut descending = outcomes.to_vec();
    descending.sort_unstable_by(|a, b| {
        b.loss.cmp(&a.loss).then_with(|| b.mass.cmp(&a.mass))
    });

    // A mass unit represents 1 / total_mass of the distribution. Multiplying
    // each atom by one million gives a common grid on which the upper-tail
    // boundary may split an atom exactly.
    let mut remaining = target_scaled_mass;
    let mut tail_loss_numerator = 0_u128;
    for outcome in descending {
        if remaining == 0 {
            break;
        }
        let atom_scaled_mass = outcome
            .mass
            .checked_mul(PROBABILITY_SCALE_PPM)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
        let included_mass = remaining.min(atom_scaled_mass);
        let contribution = included_mass
            .checked_mul(outcome.loss as u128)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
        tail_loss_numerator = tail_loss_numerator
            .checked_add(contribution)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
        remaining -= included_mass;
    }

    if remaining != 0 {
        return Err(TailRiskError::ArithmeticOverflow);
    }
    ExactRisk::new(tail_loss_numerator, target_scaled_mass)
}

/// Evaluate a fixed observable policy under each complete persistent mission
/// model, then return the worst model-conditional CVaR.
///
/// This is the correct outer/inner decomposition for the stated research
/// objective: the model is fixed outside the stochastic path distribution.
/// It is not a policy optimizer and makes no assumption that epistemic model
/// identities have a qualified probability distribution.
pub fn worst_case_cvar(
    models: &[ModelLossDistribution<'_>],
    alpha_ppm: u32,
) -> Result<RobustTailRisk, TailRiskError> {
    if models.is_empty() {
        return Err(TailRiskError::EmptyDistribution);
    }
    if alpha_ppm as u128 >= PROBABILITY_SCALE_PPM {
        return Err(TailRiskError::InvalidAlpha(alpha_ppm));
    }

    let mut ordered = models.to_vec();
    ordered.sort_unstable_by_key(|model| model.model_id);
    for pair in ordered.windows(2) {
        if pair[0].model_id == pair[1].model_id {
            return Err(TailRiskError::DuplicateModelId(pair[0].model_id));
        }
    }

    let mut per_model = Vec::with_capacity(ordered.len());
    let mut worst_case: Option<ModelTailRisk> = None;
    for model in ordered {
        let value = ModelTailRisk {
            model_id: model.model_id,
            cvar: cvar(model.outcomes, alpha_ppm)?,
        };
        let replace = match worst_case {
            None => true,
            Some(current) => {
                value.cvar.compare(current.cvar) == Ordering::Greater
                    || (value.cvar == current.cvar && value.model_id < current.model_id)
            }
        };
        if replace {
            worst_case = Some(value);
        }
        per_model.push(value);
    }

    let worst = worst_case.ok_or(TailRiskError::EmptyDistribution)?;
    Ok(RobustTailRisk {
        per_model,
        worst_case_model_id: worst.model_id,
        worst_case_cvar: worst.cvar,
        alpha_ppm,
    })
}



/// One observable branch at the current decision epoch, with its conditional
/// continuation-loss distribution under one fixed persistent mission model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConditionalLossBranch {
    /// Stable, nonzero identity for the observable branch.
    pub branch_id: u64,
    /// The continuation action selected from observable history on this branch.
    /// It must be identical for this branch across every persistent model.
    pub continuation_action_id: u64,
    /// Terminal decision chosen for each observable second-stage outcome
    /// (no-observation, observed-zero, observed-one). Must match across models.
    pub terminal_decisions: [TerminalDecision; 3],
    /// Unnormalized probability mass of reaching this branch under its model.
    pub probability_mass: u128,
    /// Conditional future-loss distribution after this branch is observed.
    pub continuation_losses: Vec<LossMass>,
}

/// A complete persistent mission model with its own observable-branch
/// probabilities and conditional continuation distributions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentMissionBranchSet {
    pub model_id: u64,
    /// First-stage acquisition action selected at the mission root. It must
    /// be identical across every persistent model.
    pub root_action_id: u64,
    pub branches: Vec<ConditionalLossBranch>,
}

/// Conditional tail risk after one observable branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IteratedBranchRisk {
    pub branch_id: u64,
    pub conditional_cvar: ExactRisk,
}

/// Two-level iterated CVaR for one fixed persistent mission model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IteratedModelRisk {
    pub model_id: u64,
    pub branch_risks: Vec<IteratedBranchRisk>,
    /// Root CVaR over the branch-conditional CVaR values.
    pub iterated_cvar: ExactRisk,
}

/// Robust two-level ICVaR, with the outer maximum applied only after each
/// complete persistent mission model has been evaluated separately.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RobustIteratedCvar {
    pub per_model: Vec<IteratedModelRisk>,
    pub worst_case_model_id: u64,
    pub worst_case_cvar: ExactRisk,
    /// Tail probability mass at each recursion level (PPM, not confidence).
    pub tail_mass_ppm: u32,
}

/// Exact two-level Iterated CVaR for one complete persistent mission model.
///
/// Each branch first receives a conditional continuation CVaR. The root then
/// takes CVaR over those branch values using the branch's probability mass.
/// tail_mass_ppm=50_000 means the worst 5% at each level; 1_000_000
/// recovers expectation at each level.
///
/// The model identity is not mixed or resampled here. Call separately for
/// every complete model, then use worst_case_iterated_cvar for the outer
/// persistent-model maximum.
pub fn iterated_cvar(
    branches: &[ConditionalLossBranch],
    tail_mass_ppm: u32,
) -> Result<IteratedModelRisk, TailRiskError> {
    validate_tail_mass(tail_mass_ppm)?;
    if branches.is_empty() {
        return Err(TailRiskError::EmptyDistribution);
    }

    let mut ordered = branches.to_vec();
    ordered.sort_unstable_by_key(|branch| branch.branch_id);
    for branch in &ordered {
        if branch.branch_id == 0
            || branch.continuation_action_id == 0
            || branch.probability_mass == 0
        {
            return Err(TailRiskError::InvalidBranchId);
        }
    }
    for pair in ordered.windows(2) {
        if pair[0].branch_id == pair[1].branch_id {
            return Err(TailRiskError::DuplicateBranchId(pair[0].branch_id));
        }
    }

    let confidence_alpha_ppm =
        u32::try_from(PROBABILITY_SCALE_PPM - tail_mass_ppm as u128)
            .map_err(|_| TailRiskError::ArithmeticOverflow)?;
    let mut branch_risks = Vec::with_capacity(ordered.len());
    let mut root_values = Vec::with_capacity(ordered.len());
    for branch in &ordered {
        let conditional_cvar = cvar(&branch.continuation_losses, confidence_alpha_ppm)?;
        branch_risks.push(IteratedBranchRisk {
            branch_id: branch.branch_id,
            conditional_cvar,
        });
        root_values.push(RationalLossMass {
            value: conditional_cvar,
            mass: branch.probability_mass,
        });
    }

    let root = cvar_rational_masses(&root_values, tail_mass_ppm)?;
    Ok(IteratedModelRisk {
        model_id: 0,
        branch_risks,
        iterated_cvar: root,
    })
}

/// Evaluate each complete persistent model separately, then maximize its
/// two-level ICVaR value. Model identity is never reselected within a model's
/// branch tree.
pub fn worst_case_iterated_cvar(
    models: &[PersistentMissionBranchSet],
    tail_mass_ppm: u32,
) -> Result<RobustIteratedCvar, TailRiskError> {
    validate_tail_mass(tail_mass_ppm)?;
    if models.is_empty() {
        return Err(TailRiskError::EmptyDistribution);
    }

    let mut ordered = models.to_vec();
    ordered.sort_unstable_by_key(|model| model.model_id);
    if ordered[0].model_id == 0 {
        return Err(TailRiskError::InvalidModelId);
    }
    for pair in ordered.windows(2) {
        if pair[0].model_id == pair[1].model_id {
            return Err(TailRiskError::DuplicateModelId(pair[0].model_id));
        }
    }

    if ordered[0].root_action_id == 0 {
        return Err(TailRiskError::InvalidBranchId);
    }
    let reference_root_action = ordered[0].root_action_id;
    let reference_policy = branch_policy_signature(&ordered[0].branches)?;
    for model in ordered.iter().skip(1) {
        if model.root_action_id != reference_root_action
            || branch_policy_signature(&model.branches)? != reference_policy
        {
            return Err(TailRiskError::PersistentPolicyMismatch);
        }
    }

    let mut per_model = Vec::with_capacity(ordered.len());
    let mut worst_case: Option<IteratedModelRisk> = None;
    for model in ordered {
        let mut result = iterated_cvar(&model.branches, tail_mass_ppm)?;
        result.model_id = model.model_id;
        let replace = match &worst_case {
            None => true,
            Some(current) => {
                result.iterated_cvar.compare(current.iterated_cvar) == Ordering::Greater
                    || (result.iterated_cvar == current.iterated_cvar
                        && result.model_id < current.model_id)
            }
        };
        if replace {
            worst_case = Some(result.clone());
        }
        per_model.push(result);
    }

    let worst = worst_case.ok_or(TailRiskError::EmptyDistribution)?;
    Ok(RobustIteratedCvar {
        per_model,
        worst_case_model_id: worst.model_id,
        worst_case_cvar: worst.iterated_cvar,
        tail_mass_ppm,
    })
}


/// Return a sorted branch-to-continuation-action signature, validating that
/// every branch identity, action identity, and probability mass is nonzero
/// and that branch IDs are unique.
fn branch_policy_signature(
    branches: &[ConditionalLossBranch],
) -> Result<Vec<(u64, u64, [TerminalDecision; 3])>, TailRiskError> {
    if branches.is_empty() {
        return Err(TailRiskError::EmptyDistribution);
    }
    let mut signature = branches
        .iter()
        .map(|branch| {
            if branch.branch_id == 0
                || branch.continuation_action_id == 0
                || branch.probability_mass == 0
            {
                return Err(TailRiskError::InvalidBranchId);
            }
            Ok((
                branch.branch_id,
                branch.continuation_action_id,
                branch.terminal_decisions,
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    signature.sort_unstable_by_key(|(branch_id, _, _)| *branch_id);
    for pair in signature.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(TailRiskError::DuplicateBranchId(pair[0].0));
        }
    }
    Ok(signature)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RationalLossMass {
    value: ExactRisk,
    mass: u128,
}

/// Exact CVaR over rational-valued outcomes, using tail mass rather than
/// confidence alpha. This is needed because a conditional CVaR at one level
/// becomes a rational-valued outcome at the preceding level.
fn cvar_rational_masses(
    outcomes: &[RationalLossMass],
    tail_mass_ppm: u32,
) -> Result<ExactRisk, TailRiskError> {
    validate_tail_mass(tail_mass_ppm)?;
    if outcomes.is_empty() {
        return Err(TailRiskError::EmptyDistribution);
    }
    let total = outcomes.iter().try_fold(0_u128, |sum, item| {
        sum.checked_add(item.mass)
            .ok_or(TailRiskError::ArithmeticOverflow)
    })?;
    if total == 0 {
        return Err(TailRiskError::ZeroTotalMass);
    }
    let target_scaled_mass = total
        .checked_mul(tail_mass_ppm as u128)
        .ok_or(TailRiskError::ArithmeticOverflow)?;
    let mut descending = outcomes.to_vec();
    descending.sort_unstable_by(|left, right| {
        right
            .value
            .compare(left.value)
            .then_with(|| right.mass.cmp(&left.mass))
    });

    let mut remaining = target_scaled_mass;
    let mut weighted_sum = ExactRisk::new(0, 1)?;
    for outcome in descending {
        if remaining == 0 {
            break;
        }
        let scaled_atom_mass = outcome
            .mass
            .checked_mul(PROBABILITY_SCALE_PPM)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
        let included = remaining.min(scaled_atom_mass);
        weighted_sum = add_weighted_fraction(weighted_sum, outcome.value, included)?;
        remaining -= included;
    }
    if remaining != 0 {
        return Err(TailRiskError::ArithmeticOverflow);
    }
    let denominator = weighted_sum
        .denominator()
        .checked_mul(target_scaled_mass)
        .ok_or(TailRiskError::ArithmeticOverflow)?;
    ExactRisk::new(weighted_sum.numerator(), denominator)
}

fn add_weighted_fraction(
    accumulator: ExactRisk,
    value: ExactRisk,
    weight: u128,
) -> Result<ExactRisk, TailRiskError> {
    if weight == 0 {
        return Ok(accumulator);
    }
    let term_numerator = value
        .numerator()
        .checked_mul(weight)
        .ok_or(TailRiskError::ArithmeticOverflow)?;
    let common = gcd(accumulator.denominator(), value.denominator());
    let accumulator_factor = value.denominator() / common;
    let value_factor = accumulator.denominator() / common;
    let numerator = accumulator
        .numerator()
        .checked_mul(accumulator_factor)
        .and_then(|left| {
            term_numerator
                .checked_mul(value_factor)
                .and_then(|right| left.checked_add(right))
        })
        .ok_or(TailRiskError::ArithmeticOverflow)?;
    let denominator = accumulator
        .denominator()
        .checked_mul(accumulator_factor)
        .ok_or(TailRiskError::ArithmeticOverflow)?;
    ExactRisk::new(numerator, denominator)
}

fn validate_tail_mass(tail_mass_ppm: u32) -> Result<(), TailRiskError> {
    if tail_mass_ppm == 0 || tail_mass_ppm as u128 > PROBABILITY_SCALE_PPM {
        return Err(TailRiskError::InvalidTailMass(tail_mass_ppm));
    }
    Ok(())
}

fn total_mass(outcomes: &[LossMass]) -> Result<u128, TailRiskError> {
    if outcomes.is_empty() {
        return Err(TailRiskError::EmptyDistribution);
    }
    let mut total = 0_u128;
    for outcome in outcomes {
        total = total
            .checked_add(outcome.mass)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
    }
    if total == 0 {
        return Err(TailRiskError::ZeroTotalMass);
    }
    Ok(total)
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a.max(1)
}

fn compare_fractions(
    mut left_numerator: u128,
    mut left_denominator: u128,
    mut right_numerator: u128,
    mut right_denominator: u128,
) -> Ordering {
    let mut reverse = false;
    loop {
        let left_integer = left_numerator / left_denominator;
        let right_integer = right_numerator / right_denominator;
        if left_integer != right_integer {
            let result = left_integer.cmp(&right_integer);
            return if reverse { result.reverse() } else { result };
        }

        let left_remainder = left_numerator % left_denominator;
        let right_remainder = right_numerator % right_denominator;
        match (left_remainder == 0, right_remainder == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => {
                return if reverse { Ordering::Greater } else { Ordering::Less };
            }
            (false, true) => {
                return if reverse { Ordering::Less } else { Ordering::Greater };
            }
            (false, false) => {
                left_numerator = left_denominator;
                left_denominator = left_remainder;
                right_numerator = right_denominator;
                right_denominator = right_remainder;
                reverse = !reverse;
            }
        }
    }
}



/// Reconstruct complete mission-path loss distributions for a fixed temporal
/// mission-minimax policy.
///
/// This is an independent evaluator-side propagation of the public model and
/// policy fields. It reconstructs all observable outcome branches and splits
/// each terminal state mass into its realized loss atom. The resulting mass
/// denominator is PROBABILITY_SCALE_PPM^5 for every model, matching the
/// finite oracle's two-stage integer probability scale.
///
/// The two candidate mission models are evaluated separately and remain fixed
/// through each complete path. This function does not optimize the policy and
/// must not be exposed to the controller's observation history.
pub fn mission_path_loss_distributions(
    problem: &TemporalMinimaxProblem,
    actions: &[TimedJointAction],
    policy: &ObservablePolicy,
) -> Result<[Vec<LossMass>; 2], TailRiskError> {
    for model in 0..2 {
        let total = problem.prior_by_model_ppm[model]
            .iter()
            .try_fold(0_u64, |sum, value| sum.checked_add(*value));
        if total != Some(PROBABILITY_SCALE) {
            return Err(TailRiskError::InvalidPolicyTree);
        }
    }

    if problem.false_safe_loss == 0 || problem.false_degraded_loss == 0 {
        return Err(TailRiskError::InvalidPolicyTree);
    }

    for (index, action) in actions.iter().enumerate() {
        if action.action_id == 0
            || action.evidence_age_ns > action.cost.latency_ns
            || actions[..index]
                .iter()
                .any(|prior| prior.action_id == action.action_id)
        {
            return Err(TailRiskError::InvalidPolicyTree);
        }
        for model in 0..2 {
            for state in 0..4 {
                let likelihoods = action.likelihoods[model][state];
                let total = likelihoods
                    .no_observation_ppm
                    .checked_add(likelihoods.observed_zero_ppm)
                    .and_then(|value| value.checked_add(likelihoods.observed_one_ppm));
                if total != Some(PROBABILITY_SCALE) {
                    return Err(TailRiskError::InvalidPolicyTree);
                }
            }
            for cause in 0..2 {
                let transition = action.transitions[model][cause];
                if transition.inside_to_outside_ppm > PROBABILITY_SCALE
                    || transition.outside_to_inside_ppm > PROBABILITY_SCALE
                    || (action.evidence_age_ns == 0
                        && (transition.inside_to_outside_ppm != 0
                            || transition.outside_to_inside_ppm != 0))
                    || action.outcome_model_sha256[model][cause]
                        .iter()
                        .all(|byte| *byte == 0)
                    || action.transition_model_sha256[model][cause]
                        .iter()
                        .all(|byte| *byte == 0)
                {
                    return Err(TailRiskError::InvalidPolicyTree);
                }
            }
        }
    }

    let padding_one_action = scale_power(2)?;
    let padding_no_action = scale_power(4)?;
    let expected_total_mass = scale_power(5)?;
    let mut distributions: [Vec<LossMass>; 2] = [Vec::new(), Vec::new()];

    let Some(first_action_id) = policy.first_action_id else {
        if !policy.branches.is_empty() {
            return Err(TailRiskError::InvalidPolicyTree);
        }
        let decision = policy
            .root_terminal_decision
            .ok_or(TailRiskError::InvalidPolicyTree)?;
        for (model, distribution) in distributions.iter_mut().enumerate() {
            append_terminal_state_masses(
                distribution,
                problem.prior_by_model_ppm[model].map(u128::from),
                decision,
                problem,
                padding_no_action,
            )?;
        }
        for distribution in &distributions {
            if total_mass(distribution)? != expected_total_mass {
                return Err(TailRiskError::InvalidPolicyTree);
            }
        }
        return Ok(distributions);
    };

    if policy.root_terminal_decision.is_some() || policy.branches.len() != 3 {
        return Err(TailRiskError::InvalidPolicyTree);
    }
    let first_action = actions
        .iter()
        .find(|action| action.action_id == first_action_id)
        .ok_or(TailRiskError::MissingAction(first_action_id))?;

    let mut seen = [false; 3];
    for branch in &policy.branches {
        let first_index = outcome_index(branch.first_outcome);
        if seen[first_index] {
            return Err(TailRiskError::InvalidPolicyTree);
        }
        seen[first_index] = true;

        for (model, distribution) in distributions.iter_mut().enumerate() {
            let initial = problem.prior_by_model_ppm[model].map(u128::from);
            let first_outcomes = propagate_action(model, initial, first_action)?;
            let first_state_mass = first_outcomes[first_index];

            match &branch.choice {
                ObservableBranchChoice::Stop { terminal_decision } => {
                    append_terminal_state_masses(
                        distribution,
                        first_state_mass,
                        *terminal_decision,
                        problem,
                        padding_one_action,
                    )?;
                }
                ObservableBranchChoice::Acquire {
                    second_action_id,
                    terminal_decisions,
                } => {
                    if *second_action_id == first_action_id {
                        return Err(TailRiskError::InvalidPolicyTree);
                    }
                    let second_action = actions
                        .iter()
                        .find(|action| action.action_id == *second_action_id)
                        .ok_or(TailRiskError::MissingAction(*second_action_id))?;
                    let second_outcomes = propagate_action(model, first_state_mass, second_action)?;
                    for (second_outcome_index, second_outcome_mass) in
                        second_outcomes.iter().enumerate()
                    {
                        append_terminal_state_masses(
                            distribution,
                            *second_outcome_mass,
                            terminal_decisions[second_outcome_index],
                            problem,
                            1,
                        )?;
                    }
                }
            }
        }
    }

    if seen.iter().any(|present| !present) {
        return Err(TailRiskError::InvalidPolicyTree);
    }
    for distribution in &distributions {
        if total_mass(distribution)? != expected_total_mass {
            return Err(TailRiskError::InvalidPolicyTree);
        }
    }
    Ok(distributions)
}

fn outcome_index(outcome: ActionOutcome) -> usize {
    match outcome {
        ActionOutcome::NoObservation => 0,
        ActionOutcome::ObservedZero => 1,
        ActionOutcome::ObservedOne => 2,
    }
}

fn scale_power(exponent: u8) -> Result<u128, TailRiskError> {
    let mut value = 1_u128;
    for _ in 0..exponent {
        value = value
            .checked_mul(PROBABILITY_SCALE_PPM)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
    }
    Ok(value)
}

/// Returns post-use state mass for no-observation, observed-zero and
/// observed-one. State indices are nominal/inside, nominal/outside,
/// shared-cause/inside, shared-cause/outside.
fn propagate_action(
    model: usize,
    mass: [u128; 4],
    action: &TimedJointAction,
) -> Result<[[u128; 4]; 3], TailRiskError> {
    let scale = PROBABILITY_SCALE_PPM;
    let mut outcomes = [[0_u128; 4]; 3];
    for (outcome_index, outcome_mass) in outcomes.iter_mut().enumerate() {
        let likelihood_for_state = |state: usize| {
            let likelihoods = action.likelihoods[model][state];
            match outcome_index {
                0 => likelihoods.no_observation_ppm,
                1 => likelihoods.observed_zero_ppm,
                _ => likelihoods.observed_one_ppm,
            }
        };

        let mut capture = [0_u128; 4];
        for (state, capture_mass) in capture.iter_mut().enumerate() {
            *capture_mass = mass[state]
                .checked_mul(likelihood_for_state(state) as u128)
                .ok_or(TailRiskError::ArithmeticOverflow)?;
        }

        for cause in 0..2 {
            let inside = cause * 2;
            let outside = inside + 1;
            let transition = action.transitions[model][cause];
            let inside_to_outside = transition.inside_to_outside_ppm as u128;
            let outside_to_inside = transition.outside_to_inside_ppm as u128;
            if inside_to_outside > scale || outside_to_inside > scale {
                return Err(TailRiskError::InvalidPolicyTree);
            }
            let inside_to_inside = scale - inside_to_outside;
            let outside_to_outside = scale - outside_to_inside;

            outcome_mass[inside] = capture[inside]
                .checked_mul(inside_to_inside)
                .and_then(|value| value.checked_add(
                    capture[outside].checked_mul(outside_to_inside)?,
                ))
                .ok_or(TailRiskError::ArithmeticOverflow)?;
            outcome_mass[outside] = capture[inside]
                .checked_mul(inside_to_outside)
                .and_then(|value| value.checked_add(
                    capture[outside].checked_mul(outside_to_outside)?,
                ))
                .ok_or(TailRiskError::ArithmeticOverflow)?;
        }
    }
    Ok(outcomes)
}

fn append_terminal_state_masses(
    distribution: &mut Vec<LossMass>,
    state_mass: [u128; 4],
    decision: TerminalDecision,
    problem: &TemporalMinimaxProblem,
    multiplier: u128,
) -> Result<(), TailRiskError> {
    for (state_index, mass) in state_mass.into_iter().enumerate() {
        if mass == 0 {
            continue;
        }
        let loss = match (state_index % 2, decision) {
            (0, TerminalDecision::DeclareDegraded) => problem.false_degraded_loss,
            (1, TerminalDecision::DeclareSafe) => problem.false_safe_loss,
            _ => 0,
        };
        let scaled_mass = mass
            .checked_mul(multiplier)
            .ok_or(TailRiskError::ArithmeticOverflow)?;
        if let Some(existing) = distribution.iter_mut().find(|item| item.loss == loss) {
            existing.mass = existing
                .mass
                .checked_add(scaled_mass)
                .ok_or(TailRiskError::ArithmeticOverflow)?;
        } else {
            distribution.push(LossMass {
                loss,
                mass: scaled_mass,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn risk(numerator: u128, denominator: u128) -> ExactRisk {
        ExactRisk::new(numerator, denominator).unwrap()
    }

    #[test]
    fn cvar_includes_fractional_atom_at_tail_boundary_exactly() {
        let outcomes = [
            LossMass { loss: 100, mass: 1 },
            LossMass { loss: 0, mass: 99 },
        ];
        assert_eq!(expected_loss(&outcomes).unwrap(), risk(1, 1));
        assert_eq!(cvar(&outcomes, 950_000).unwrap(), risk(20, 1));
    }

    #[test]
    fn cvar_equals_expectation_at_alpha_zero() {
        let outcomes = [
            LossMass { loss: 10, mass: 2 },
            LossMass { loss: 3, mass: 5 },
            LossMass { loss: 0, mass: 3 },
        ];
        assert_eq!(cvar(&outcomes, 0).unwrap(), expected_loss(&outcomes).unwrap());
    }

    #[test]
    fn cvar_of_constant_loss_equals_that_loss() {
        let outcomes = [
            LossMass { loss: 7, mass: 13 },
            LossMass { loss: 7, mass: 2 },
            LossMass { loss: 7, mass: 85 },
        ];
        assert_eq!(cvar(&outcomes, 950_000).unwrap(), risk(7, 1));
    }

    #[test]
    fn cvar_is_monotone_in_confidence_level() {
        let outcomes = [
            LossMass { loss: 10, mass: 1 },
            LossMass { loss: 1, mass: 9 },
            LossMass { loss: 0, mass: 90 },
        ];
        let levels = [0, 500_000, 900_000, 950_000, 990_000, 999_000];
        let values = levels.map(|alpha| cvar(&outcomes, alpha).unwrap());
        assert!(values.windows(2).all(|pair| pair[0].compare(pair[1]) != Ordering::Greater));
    }

    #[test]
    fn cvar_obeys_positive_translation_and_scaling() {
        let base = [
            LossMass { loss: 8, mass: 1 },
            LossMass { loss: 2, mass: 3 },
            LossMass { loss: 1, mass: 6 },
        ];
        let transformed = [
            LossMass { loss: 26, mass: 1 },
            LossMass { loss: 14, mass: 3 },
            LossMass { loss: 12, mass: 6 },
        ];
        let expected = cvar(&base, 800_000).unwrap();
        let actual = cvar(&transformed, 800_000).unwrap();
        assert_eq!(
            actual,
            risk(expected.numerator * 2 + 10 * expected.denominator, expected.denominator)
        );
    }

    #[test]
    fn outcome_order_does_not_change_exact_tail_risk() {
        let forward = [
            LossMass { loss: 9, mass: 3 },
            LossMass { loss: 4, mass: 11 },
            LossMass { loss: 0, mass: 7 },
        ];
        let reverse = [
            LossMass { loss: 0, mass: 7 },
            LossMass { loss: 4, mass: 11 },
            LossMass { loss: 9, mass: 3 },
        ];
        assert_eq!(cvar(&forward, 900_000), cvar(&reverse, 900_000));
    }

    #[test]
    fn model_set_uses_outer_maximum_of_model_conditional_cvar() {
        let nominal = [LossMass { loss: 1, mass: 100 }];
        let faulted = [
            LossMass { loss: 10, mass: 1 },
            LossMass { loss: 0, mass: 99 },
        ];
        let result = worst_case_cvar(
            &[
                ModelLossDistribution { model_id: 22, outcomes: &faulted },
                ModelLossDistribution { model_id: 11, outcomes: &nominal },
            ],
            950_000,
        )
        .unwrap();
        assert_eq!(result.worst_case_model_id, 22);
        assert_eq!(result.worst_case_cvar, risk(2, 1));
        assert_eq!(result.per_model.iter().map(|item| item.model_id).collect::<Vec<_>>(), [11, 22]);
    }

    #[test]
    fn adding_a_persistent_model_cannot_reduce_worst_case_cvar() {
        let a = [LossMass { loss: 4, mass: 100 }];
        let b = [LossMass { loss: 8, mass: 100 }];
        let c = [LossMass { loss: 12, mass: 100 }];
        let before = worst_case_cvar(
            &[
                ModelLossDistribution { model_id: 1, outcomes: &a },
                ModelLossDistribution { model_id: 2, outcomes: &b },
            ],
            990_000,
        )
        .unwrap();
        let after = worst_case_cvar(
            &[
                ModelLossDistribution { model_id: 1, outcomes: &a },
                ModelLossDistribution { model_id: 2, outcomes: &b },
                ModelLossDistribution { model_id: 3, outcomes: &c },
            ],
            990_000,
        )
        .unwrap();
        assert_ne!(after.worst_case_cvar.compare(before.worst_case_cvar), Ordering::Less);
    }

    #[test]
    fn fraction_comparison_avoids_cross_product_overflow() {
        let left = ExactRisk { numerator: u128::MAX - 1, denominator: u128::MAX };
        let right = ExactRisk { numerator: u128::MAX - 2, denominator: u128::MAX - 1 };
        assert_eq!(left.compare(right), Ordering::Greater);
    }

    #[test]
    fn rejects_empty_zero_mass_and_alpha_one() {
        assert_eq!(cvar(&[], 950_000), Err(TailRiskError::EmptyDistribution));
        assert_eq!(
            cvar(&[LossMass { loss: 1, mass: 0 }], 950_000),
            Err(TailRiskError::ZeroTotalMass)
        );
        assert_eq!(
            cvar(&[LossMass { loss: 1, mass: 1 }], 1_000_000),
            Err(TailRiskError::InvalidAlpha(1_000_000))
        );
    }

    #[test]
    fn rejects_overflow_in_total_mass_and_loss_products() {
        assert_eq!(
            expected_loss(&[
                LossMass { loss: 0, mass: u128::MAX },
                LossMass { loss: 0, mass: 1 },
            ]),
            Err(TailRiskError::ArithmeticOverflow)
        );
        assert_eq!(
            expected_loss(&[LossMass { loss: u64::MAX, mass: u128::MAX }]),
            Err(TailRiskError::ArithmeticOverflow)
        );
    }

    #[test]
    fn independently_expanded_integer_oracle_matches_cvar() {
        fn expanded_oracle(outcomes: &[LossMass], alpha_ppm: u32) -> ExactRisk {
            let mut expanded = Vec::new();
            for outcome in outcomes {
                for _ in 0..outcome.mass {
                    expanded.push(outcome.loss);
                }
            }
            expanded.sort_unstable_by(|a, b| b.cmp(a));
            let tail = (expanded.len() as u128)
                * (PROBABILITY_SCALE_PPM - alpha_ppm as u128);
            let mut left = tail;
            let mut numerator = 0_u128;
            for loss in expanded {
                if left == 0 {
                    break;
                }
                let included = left.min(PROBABILITY_SCALE_PPM);
                numerator += included * loss as u128;
                left -= included;
            }
            ExactRisk::new(numerator, tail).unwrap()
        }

        let cases: [&[LossMass]; 4] = [
            &[
                LossMass { loss: 7, mass: 1 },
                LossMass { loss: 3, mass: 2 },
                LossMass { loss: 0, mass: 4 },
            ],
            &[
                LossMass { loss: 100, mass: 1 },
                LossMass { loss: 0, mass: 99 },
            ],
            &[
                LossMass { loss: 4, mass: 3 },
                LossMass { loss: 4, mass: 2 },
                LossMass { loss: 1, mass: 2 },
            ],
            &[
                LossMass { loss: 9, mass: 1 },
                LossMass { loss: 8, mass: 1 },
                LossMass { loss: 7, mass: 1 },
            ],
        ];
        for case in cases {
            for alpha in [0, 250_000, 500_000, 750_000, 900_000, 950_000, 999_000] {
                assert_eq!(cvar(case, alpha).unwrap(), expanded_oracle(case, alpha));
            }
        }
    }

    #[test]
    fn robust_model_ties_choose_smallest_identity_deterministically() {
        let outcomes = [LossMass { loss: 5, mass: 10 }];
        let result = worst_case_cvar(
            &[
                ModelLossDistribution { model_id: 9, outcomes: &outcomes },
                ModelLossDistribution { model_id: 3, outcomes: &outcomes },
            ],
            950_000,
        )
        .unwrap();
        assert_eq!(result.worst_case_model_id, 3);
    }

    #[test]
    fn rejects_duplicate_persistent_model_identity() {
        let outcomes = [LossMass { loss: 5, mass: 10 }];
        assert_eq!(
            worst_case_cvar(
                &[
                    ModelLossDistribution { model_id: 1, outcomes: &outcomes },
                    ModelLossDistribution { model_id: 1, outcomes: &outcomes },
                ],
                950_000,
            ),
            Err(TailRiskError::DuplicateModelId(1))
        );
    }
}


#[cfg(test)]
mod mission_path_distribution_tests {
    use super::*;
    use crate::navigation_temporal_mission_minimax::{
        plan_temporal_mission_minimax, OutcomeLikelihoods, ResourceVector,
        StateTransition, TimedJointAction,
    };

    fn likelihoods(no: u64, zero: u64, one: u64) -> OutcomeLikelihoods {
        OutcomeLikelihoods {
            no_observation_ppm: no,
            observed_zero_ppm: zero,
            observed_one_ppm: one,
        }
    }

    fn problem() -> TemporalMinimaxProblem {
        TemporalMinimaxProblem {
            prior_by_model_ppm: [
                [250_000, 250_000, 250_000, 250_000],
                [250_000, 250_000, 250_000, 250_000],
            ],
            false_safe_loss: 1,
            false_degraded_loss: 1,
        }
    }

    fn action(id: u64, age: u64, latency: u64, inside_error: u64, outside_error: u64) -> TimedJointAction {
        let inside = likelihoods(0, PROBABILITY_SCALE - inside_error, inside_error);
        let outside = likelihoods(0, outside_error, PROBABILITY_SCALE - outside_error);
        let rows = [inside, outside, inside, outside];
        TimedJointAction {
            action_id: id,
            cost: ResourceVector { latency_ns: latency, energy_uj: 1, bandwidth_bytes: 1 },
            evidence_age_ns: age,
            likelihoods: [rows; 2],
            transitions: [[
                StateTransition { inside_to_outside_ppm: 0, outside_to_inside_ppm: 0 },
                StateTransition { inside_to_outside_ppm: 0, outside_to_inside_ppm: 0 },
            ]; 2],
            outcome_model_sha256: [[[0x31; 32]; 2]; 2],
            transition_model_sha256: [[[0x42; 32]; 2]; 2],
        }
    }

    fn assert_matches_expected_loss(
        policy: &ObservablePolicy,
        distributions: &[Vec<LossMass>; 2],
    ) {
        for model in 0..2 {
            assert_eq!(
                expected_loss(&distributions[model]).unwrap(),
                ExactRisk::new(
                    policy.model_loss_numerators[model],
                    policy.risk_denominator,
                )
                .unwrap()
            );
            assert_eq!(
                total_mass(&distributions[model]).unwrap(),
                scale_power(5).unwrap()
            );
        }
    }

    #[test]
    fn extracted_complete_path_distributions_reproduce_exact_minimax_expected_risk() {
        let p = problem();
        let actions = [
            action(1, 100, 100, 0, 0),
            action(2, 0, 100, 100_000, 100_000),
        ];
        let plans = plan_temporal_mission_minimax(
            &p,
            &actions,
            ResourceVector { latency_ns: 200, energy_uj: 10, bandwidth_bytes: 10 },
        )
        .unwrap();

        for policy in [
            &plans.mission_global_minimax_risk,
            &plans.mission_global_minimax_regret,
            &plans.rectangular_switching_diagnostic,
        ] {
            let distributions = mission_path_loss_distributions(&p, &actions, policy).unwrap();
            assert_matches_expected_loss(policy, &distributions);
            let robust_tail = worst_case_cvar(
                &[
                    ModelLossDistribution { model_id: 0, outcomes: &distributions[0] },
                    ModelLossDistribution { model_id: 1, outcomes: &distributions[1] },
                ],
                950_000,
            )
            .unwrap();
            assert!(robust_tail.worst_case_cvar.denominator > 0);
        }
    }

    #[test]
    fn no_action_policy_reconstructs_padded_full_mission_distribution() {
        let p = problem();
        let plans = plan_temporal_mission_minimax(&p, &[], ResourceVector::default()).unwrap();
        let policy = &plans.mission_global_minimax_risk;
        let distributions = mission_path_loss_distributions(&p, &[], policy).unwrap();
        assert_matches_expected_loss(policy, &distributions);
    }

    #[test]
    fn mission_path_extraction_rejects_missing_action_and_incomplete_branches() {
        let p = problem();
        let actions = [action(1, 0, 100, 0, 0)];
        let plans = plan_temporal_mission_minimax(
            &p,
            &actions,
            ResourceVector { latency_ns: 100, energy_uj: 10, bandwidth_bytes: 10 },
        )
        .unwrap();
        let mut missing_action = plans.mission_global_minimax_risk.clone();
        missing_action.first_action_id = Some(99);
        assert_eq!(
            mission_path_loss_distributions(&p, &actions, &missing_action),
            Err(TailRiskError::MissingAction(99))
        );

        let mut incomplete = plans.mission_global_minimax_risk.clone();
        incomplete.branches.pop();
        assert_eq!(
            mission_path_loss_distributions(&p, &actions, &incomplete),
            Err(TailRiskError::InvalidPolicyTree)
        );
    }

    #[test]
    fn contingent_second_action_distribution_reproduces_model_loss_numerators() {
        let p = problem();
        let mut reveal_cause = action(1, 0, 100, 0, 0);
        let reveal_nominal = likelihoods(0, PROBABILITY_SCALE, 0);
        let reveal_shared = likelihoods(0, 0, PROBABILITY_SCALE);
        for likelihood_rows in &mut reveal_cause.likelihoods {
            *likelihood_rows = [
                reveal_nominal,
                reveal_nominal,
                reveal_shared,
                reveal_shared,
            ];
        }

        let mut nominal_specialist = action(2, 0, 100, 0, 0);
        let mut shared_specialist = action(3, 0, 100, 0, 0);
        let uninformative = likelihoods(0, 500_000, 500_000);
        for (nominal_rows, shared_rows) in nominal_specialist
            .likelihoods
            .iter_mut()
            .zip(shared_specialist.likelihoods.iter_mut())
        {
            for (nominal_slot, shared_slot) in
                nominal_rows[2..].iter_mut().zip(shared_rows[..2].iter_mut())
            {
                *nominal_slot = uninformative;
                *shared_slot = uninformative;
            }
        }

        let actions = [reveal_cause, nominal_specialist, shared_specialist];
        let plans = plan_temporal_mission_minimax(
            &p,
            &actions,
            ResourceVector { latency_ns: 200, energy_uj: 10, bandwidth_bytes: 10 },
        )
        .unwrap();
        let policy = &plans.mission_global_minimax_risk;
        assert_eq!(policy.first_action_id, Some(1));
        assert!(policy.branches.iter().any(|branch| {
            matches!(&branch.choice, ObservableBranchChoice::Acquire { second_action_id: 2, .. })
        }));
        assert!(policy.branches.iter().any(|branch| {
            matches!(&branch.choice, ObservableBranchChoice::Acquire { second_action_id: 3, .. })
        }));

        let distributions = mission_path_loss_distributions(&p, &actions, policy).unwrap();
        assert_matches_expected_loss(policy, &distributions);
        assert_eq!(cvar(&distributions[0], 950_000).unwrap(), ExactRisk::from_ratio(0, 1).unwrap());
        assert_eq!(cvar(&distributions[1], 950_000).unwrap(), ExactRisk::from_ratio(0, 1).unwrap());
    }
}


#[cfg(test)]
mod belief_immediate_cvar_tests {
    use super::*;

    fn particle(id: u64, cost: u64, weight: u128) -> BeliefCostParticle {
        BeliefCostParticle {
            particle_id: id,
            immediate_cost: cost,
            weight,
        }
    }

    #[test]
    fn belief_tail_risk_separates_rare_high_consequence_states_at_equal_mean() {
        // Both actions have expected immediate cost 5. The first places 5%
        // of the current belief in a high-cost state; the second is uniform.
        let rare_catastrophic = [
            particle(1, 100, 1),
            particle(2, 0, 19),
        ];
        let steady = [particle(3, 5, 20)];

        let first = belief_immediate_cvar(&rare_catastrophic, 50_000).unwrap();
        let second = belief_immediate_cvar(&steady, 50_000).unwrap();

        assert_eq!(first.expected_cost, ExactRisk::from_ratio(5, 1).unwrap());
        assert_eq!(second.expected_cost, ExactRisk::from_ratio(5, 1).unwrap());
        assert_eq!(first.cvar_cost, ExactRisk::from_ratio(100, 1).unwrap());
        assert_eq!(second.cvar_cost, ExactRisk::from_ratio(5, 1).unwrap());
        assert_eq!(first.tail_mass_ppm, 50_000);
        assert_eq!(first.particle_count, 2);
    }

    #[test]
    fn full_tail_mass_recovers_expected_immediate_cost() {
        let belief = [
            particle(1, 10, 2),
            particle(2, 3, 5),
            particle(3, 0, 3),
        ];
        let risk = belief_immediate_cvar(&belief, 1_000_000).unwrap();
        assert_eq!(risk.cvar_cost, risk.expected_cost);
    }

    #[test]
    fn belief_particle_order_does_not_change_immediate_risk() {
        let forward = [
            particle(1, 100, 1),
            particle(2, 0, 19),
            particle(3, 5, 10),
        ];
        let reverse = [forward[2], forward[1], forward[0]];
        assert_eq!(
            belief_immediate_cvar(&forward, 50_000),
            belief_immediate_cvar(&reverse, 50_000)
        );
    }

    #[test]
    fn invalid_or_duplicate_belief_particle_id_rejects() {
        assert_eq!(
            belief_immediate_cvar(&[particle(0, 1, 1)], 50_000),
            Err(TailRiskError::InvalidParticleId)
        );
        assert_eq!(
            belief_immediate_cvar(&[particle(7, 1, 1), particle(7, 2, 1)], 50_000),
            Err(TailRiskError::DuplicateParticleId(7))
        );
    }

    #[test]
    fn invalid_tail_mass_empty_belief_and_zero_total_weight_reject() {
        assert_eq!(
            belief_immediate_cvar(&[], 50_000),
            Err(TailRiskError::EmptyDistribution)
        );
        assert_eq!(
            belief_immediate_cvar(&[particle(1, 1, 1)], 0),
            Err(TailRiskError::InvalidTailMass(0))
        );
        assert_eq!(
            belief_immediate_cvar(&[particle(1, 1, 1)], 1_000_001),
            Err(TailRiskError::InvalidTailMass(1_000_001))
        );
        assert_eq!(
            belief_immediate_cvar(&[particle(1, 1, 0)], 50_000),
            Err(TailRiskError::ZeroTotalMass)
        );
    }
}


#[cfg(test)]
mod static_cvar_time_consistency_tests {
    use super::*;

    #[test]
    fn static_root_cvar_and_conditional_replanning_can_prefer_opposite_actions() {
        // Two first-observation branches:
        // H has 1% mission probability; L has 99%.
        // On L, a common exogenous event causes loss 300 with 4% total
        // mission probability. On H, the controller chooses A or B.
        //
        // Root policy A on H:
        //   mission mass 4% at 300, 0.05% at 200, 0.95% at 150, 95% at 0.
        // Root policy B on H:
        //   mission mass 4% at 300, 1% at 180, 95% at 0.
        let root_a = [
            LossMass { loss: 300, mass: 80 },
            LossMass { loss: 200, mass: 1 },
            LossMass { loss: 150, mass: 19 },
            LossMass { loss: 0, mass: 1_900 },
        ];
        let root_b = [
            LossMass { loss: 300, mass: 80 },
            LossMass { loss: 180, mass: 20 },
            LossMass { loss: 0, mass: 1_900 },
        ];

        // After H is observed, conditional A has a 5% tail at 200 while B
        // is constant at 180. A fresh conditional CVaR replan therefore
        // changes the continuation from A to B.
        let conditional_h_a = [
            LossMass { loss: 200, mass: 1 },
            LossMass { loss: 150, mass: 19 },
        ];
        let conditional_h_b = [LossMass { loss: 180, mass: 20 }];

        let root_cvar_a = cvar(&root_a, 950_000).unwrap();
        let root_cvar_b = cvar(&root_b, 950_000).unwrap();
        let branch_cvar_a = cvar(&conditional_h_a, 950_000).unwrap();
        let branch_cvar_b = cvar(&conditional_h_b, 950_000).unwrap();

        assert_eq!(root_cvar_a, ExactRisk::from_ratio(541, 2).unwrap()); // 270.5
        assert_eq!(root_cvar_b, ExactRisk::from_ratio(276, 1).unwrap());
        assert_eq!(branch_cvar_a, ExactRisk::from_ratio(200, 1).unwrap());
        assert_eq!(branch_cvar_b, ExactRisk::from_ratio(180, 1).unwrap());

        // The original static root objective selects A (270.5 < 276), but
        // after H is observed a new conditional CVaR solve selects B (180 < 200).
        assert_eq!(root_cvar_a.compare(root_cvar_b), core::cmp::Ordering::Less);
        assert_eq!(branch_cvar_b.compare(branch_cvar_a), core::cmp::Ordering::Less);

        // Replacing A by B after H is realized changes the original root
        // distribution from root_a to root_b, raising static mission CVaR
        // from 270.5 to 276. This is the finite counterexample to assuming
        // static trajectory CVaR is automatically time-consistent.
        assert_eq!(
            root_cvar_b.compare(root_cvar_a),
            core::cmp::Ordering::Greater
        );
    }
}


#[cfg(test)]
mod iterated_cvar_tests {
    use super::*;

    fn losses(items: &[(u64, u128)]) -> Vec<LossMass> {
        items
            .iter()
            .map(|(loss, mass)| LossMass { loss: *loss, mass: *mass })
            .collect()
    }

    fn branch(id: u64, mass: u128, outcomes: &[(u64, u128)]) -> ConditionalLossBranch {
        ConditionalLossBranch {
            branch_id: id,
            continuation_action_id: id,
            terminal_decisions: [TerminalDecision::DeclareSafe; 3],
            probability_mass: mass,
            continuation_losses: losses(outcomes),
        }
    }

    #[test]
    fn iterated_cvar_computes_conditional_then_root_tail_exactly() {
        // H has 1% probability. L has 99%; within L, loss 300 has
        // conditional mass 4/99. Its branch CVaR is 8000/33.
        let branches = [
            branch(1, 99, &[(300, 4), (0, 95)]),
            branch(2, 1, &[(200, 1), (150, 19)]),
        ];
        let result = iterated_cvar(&branches, 50_000).unwrap();
        assert_eq!(result.branch_risks[0].branch_id, 1);
        assert_eq!(
            result.branch_risks[0].conditional_cvar,
            ExactRisk::from_ratio(8_000, 33).unwrap()
        );
        assert_eq!(
            result.branch_risks[1].conditional_cvar,
            ExactRisk::from_ratio(200, 1).unwrap()
        );
        assert_eq!(
            result.iterated_cvar,
            ExactRisk::from_ratio(8_000, 33).unwrap()
        );
    }

    #[test]
    fn iterated_cvar_exposes_branchwise_replan_choice_without_claiming_root_static_equivalence() {
        let root_a = [
            LossMass { loss: 300, mass: 80 },
            LossMass { loss: 200, mass: 1 },
            LossMass { loss: 150, mass: 19 },
            LossMass { loss: 0, mass: 1_900 },
        ];
        let root_b = [
            LossMass { loss: 300, mass: 80 },
            LossMass { loss: 180, mass: 20 },
            LossMass { loss: 0, mass: 1_900 },
        ];
        assert_eq!(
            cvar(&root_a, 950_000).unwrap(),
            ExactRisk::from_ratio(541, 2).unwrap()
        );
        assert_eq!(
            cvar(&root_b, 950_000).unwrap(),
            ExactRisk::from_ratio(276, 1).unwrap()
        );

        let branches_a = [
            branch(1, 99, &[(300, 4), (0, 95)]),
            branch(2, 1, &[(200, 1), (150, 19)]),
        ];
        let branches_b = [
            branch(1, 99, &[(300, 4), (0, 95)]),
            branch(2, 1, &[(180, 20)]),
        ];
        let dynamic_a = iterated_cvar(&branches_a, 50_000).unwrap();
        let dynamic_b = iterated_cvar(&branches_b, 50_000).unwrap();

        assert_eq!(
            dynamic_a.branch_risks[1].conditional_cvar,
            ExactRisk::from_ratio(200, 1).unwrap()
        );
        assert_eq!(
            dynamic_b.branch_risks[1].conditional_cvar,
            ExactRisk::from_ratio(180, 1).unwrap()
        );
        assert_eq!(dynamic_a.iterated_cvar, dynamic_b.iterated_cvar);
        // The root nested risk is indifferent because H is outside the root
        // worst-5% branch tail. A time-consistent Bellman planner still
        // evaluates the H continuation locally and prefers B there.
    }

    #[test]
    fn iterated_cvar_risk_neutral_limit_equals_joint_expected_loss() {
        let branches = [
            branch(1, 3, &[(10, 1), (0, 2)]),
            branch(2, 1, &[(4, 1)]),
        ];
        let nested = iterated_cvar(&branches, 1_000_000).unwrap();
        assert_eq!(
            nested.iterated_cvar,
            ExactRisk::from_ratio(7, 2).unwrap()
        );
    }

    #[test]
    fn outer_model_max_keeps_persistent_models_separate_and_deterministic() {
        let model_a = PersistentMissionBranchSet {
            model_id: 11,
            root_action_id: 10,
            branches: vec![
                branch(1, 99, &[(100, 1), (0, 98)]),
                branch(2, 1, &[(10, 1)]),
            ],
        };
        let model_b = PersistentMissionBranchSet {
            model_id: 22,
            root_action_id: 10,
            branches: vec![
                branch(1, 99, &[(200, 1), (0, 98)]),
                branch(2, 1, &[(20, 1)]),
            ],
        };
        let result = worst_case_iterated_cvar(&[model_b, model_a], 50_000).unwrap();
        assert_eq!(result.per_model.iter().map(|item| item.model_id).collect::<Vec<_>>(), [11, 22]);
        assert_eq!(result.worst_case_model_id, 22);
        assert!(
            result.worst_case_cvar.compare(result.per_model[0].iterated_cvar)
                != Ordering::Less
        );
    }

    #[test]
    fn iterated_cvar_rejects_malformed_branches_models_and_tail_mass() {
        assert_eq!(iterated_cvar(&[], 50_000), Err(TailRiskError::EmptyDistribution));
        assert_eq!(
            iterated_cvar(&[branch(0, 1, &[(1, 1)])], 50_000),
            Err(TailRiskError::InvalidBranchId)
        );
        assert_eq!(
            iterated_cvar(&[branch(1, 1, &[(1, 1)]), branch(1, 1, &[(2, 1)])], 50_000),
            Err(TailRiskError::DuplicateBranchId(1))
        );
        assert_eq!(
            iterated_cvar(&[branch(1, 0, &[(1, 1)])], 50_000),
            Err(TailRiskError::InvalidBranchId)
        );
        assert_eq!(
            iterated_cvar(&[branch(1, 1, &[(1, 1)])], 0),
            Err(TailRiskError::InvalidTailMass(0))
        );
        assert_eq!(
            worst_case_iterated_cvar(&[
                PersistentMissionBranchSet {
                    model_id: 0,
                    root_action_id: 10,
                    branches: vec![branch(1, 1, &[(1, 1)])],
                }
            ], 50_000),
            Err(TailRiskError::InvalidModelId)
        );
    }

    #[test]
    fn persistent_models_must_share_the_same_observable_branch_actions() {
        let model_a = PersistentMissionBranchSet {
            model_id: 1,
            root_action_id: 10,
            branches: vec![
                branch(1, 99, &[(100, 1), (0, 98)]),
                branch(2, 1, &[(10, 1)]),
            ],
        };
        let mut model_b = PersistentMissionBranchSet {
            model_id: 2,
            root_action_id: 10,
            branches: vec![
                branch(1, 99, &[(200, 1), (0, 98)]),
                branch(2, 1, &[(20, 1)]),
            ],
        };
        model_b.branches[1].continuation_action_id = 99;
        assert_eq!(
            worst_case_iterated_cvar(&[model_a.clone(), model_b.clone()], 50_000),
            Err(TailRiskError::PersistentPolicyMismatch)
        );

        let mut root_mismatch = model_b.clone();
        root_mismatch.root_action_id = 77;
        assert_eq!(
            worst_case_iterated_cvar(&[model_a.clone(), root_mismatch], 50_000),
            Err(TailRiskError::PersistentPolicyMismatch)
        );

        let mut terminal_mismatch = model_b;
        terminal_mismatch.branches[0].terminal_decisions[2] =
            TerminalDecision::DeclareDegraded;
        assert_eq!(
            worst_case_iterated_cvar(&[model_a, terminal_mismatch], 50_000),
            Err(TailRiskError::PersistentPolicyMismatch)
        );
    }

}
