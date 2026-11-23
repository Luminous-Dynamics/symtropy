// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed separation of conditional Pc, collision-detection performance,
//! collision-null p-values, expected collision counts and mission-level risk.
//!
//! A conditional conjunction Pc is not automatically a long-run frequency.
//! A p-value is not a probability of collision. A collision-detection
//! probability is policy performance conditional on a true collision. An
//! expected collision count is not constrained to [0, 1].
//!
//! When event dependence is unknown, mission aggregation returns sharp
//! elementary union bounds. The independence product is only computed when
//! the caller supplies an explicit assumption digest. The digest records the
//! declared assumption; this module does not prove independence.

use std::collections::BTreeSet;

use crate::space_time_partition::Rational;

/// A validated finite mission scope for event aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissionRiskScope {
    /// Stable digest of the mission or spacecraft exposure identity.
    pub mission_digest: [u8; 32],
    /// Stable digest of the common model and conditioning regime.
    pub model_digest: [u8; 32],
    /// Stable identity of the common target probability measure.
    pub probability_measure_digest: [u8; 32],
    /// Digest of the complete common information set on which every event Pc
    /// was conditioned. Per-event posteriors from different conditioning sets
    /// cannot be combined as if they were marginals under one common posterior.
    pub conditioning_set_digest: [u8; 32],
    /// Inclusive start of the assessment horizon in integer seconds.
    pub horizon_start_seconds: u64,
    /// Exclusive end of the assessment horizon in integer seconds.
    pub horizon_end_seconds: u64,
}

impl MissionRiskScope {
    /// Construct a mission scope with an increasing finite horizon.
    pub fn new(
        mission_digest: [u8; 32],
        model_digest: [u8; 32],
        probability_measure_digest: [u8; 32],
        conditioning_set_digest: [u8; 32],
        horizon_start_seconds: u64,
        horizon_end_seconds: u64,
    ) -> Result<Self, RiskSemanticsError> {
        if horizon_end_seconds <= horizon_start_seconds {
            return Err(RiskSemanticsError::InvalidMissionHorizon);
        }
        Ok(Self {
            mission_digest,
            model_digest,
            probability_measure_digest,
            conditioning_set_digest,
            horizon_start_seconds,
            horizon_end_seconds,
        })
    }
}

/// Conditional probability that one declared conjunction produces collision.
///
/// The value is conditional on a frozen conjunction assessment, its inputs
/// and the declared model/conditioning assumptions. It is not automatically
/// an empirical long-run frequency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalCollisionProbability {
    value: Rational,
    probability_measure_digest: [u8; 32],
    conditioning_digest: [u8; 32],
}

impl ConditionalCollisionProbability {
    /// Construct a Pc value in [0, 1] with its conditioning identity.
    pub fn new(
        value: Rational,
        probability_measure_digest: [u8; 32],
        conditioning_digest: [u8; 32],
    ) -> Result<Self, RiskSemanticsError> {
        if value > Rational::from_integer(1) {
            return Err(RiskSemanticsError::ProbabilityOutsideUnitInterval);
        }
        Ok(Self {
            value,
            probability_measure_digest,
            conditioning_digest,
        })
    }

    /// Return the conditional Pc value.
    pub const fn value(self) -> Rational {
        self.value
    }

    /// Return the shared target probability measure identity.
    pub const fn probability_measure_digest(self) -> [u8; 32] {
        self.probability_measure_digest
    }

    /// Return the digest binding this value to its event-specific conditioning inputs.
    pub const fn conditioning_digest(self) -> [u8; 32] {
        self.conditioning_digest
    }
}

/// Null hypothesis used by a collision-consistency p-value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionNullHypothesis {
    /// The true miss vector is a direct hit at the origin.
    DirectHit,
    /// The true miss vector lies within the combined hard-body radius.
    TrueMissWithinHardBody,
}

/// Tail probability from a declared collision null-hypothesis test.
///
/// This value is not a collision probability. A small p-value is evidence
/// against the declared collision null under the test assumptions; it is not
/// proof of safety and cannot be passed to mission Pc aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionNullPValue {
    value: Rational,
    null_hypothesis: CollisionNullHypothesis,
    calibration_digest: [u8; 32],
}

impl CollisionNullPValue {
    /// Construct a p-value in [0, 1] bound to its null and calibration model.
    pub fn new(
        value: Rational,
        null_hypothesis: CollisionNullHypothesis,
        calibration_digest: [u8; 32],
    ) -> Result<Self, RiskSemanticsError> {
        if value > Rational::from_integer(1) {
            return Err(RiskSemanticsError::ProbabilityOutsideUnitInterval);
        }
        Ok(Self {
            value,
            null_hypothesis,
            calibration_digest,
        })
    }

    /// Return the p-value, without changing its statistical interpretation.
    pub const fn value(self) -> Rational {
        self.value
    }

    /// Return the tested collision null.
    pub const fn null_hypothesis(self) -> CollisionNullHypothesis {
        self.null_hypothesis
    }

    /// Return the calibration/source identity for the test statistic.
    pub const fn calibration_digest(self) -> [u8; 32] {
        self.calibration_digest
    }

    /// Apply a predeclared type-I false-rejection threshold.
    ///
    /// Rejecting the collision null is not the same as returning Pc = 0.
    pub fn test_at(self, alpha: Rational) -> Result<CollisionNullTestReport, RiskSemanticsError> {
        if alpha.numerator() == 0 || alpha > Rational::from_integer(1) {
            return Err(RiskSemanticsError::InvalidTestThreshold);
        }
        let decision = if self.value < alpha {
            CollisionNullTestDecision::RejectCollisionNull
        } else {
            CollisionNullTestDecision::FailToRejectCollisionNull
        };
        Ok(CollisionNullTestReport {
            null_hypothesis: self.null_hypothesis,
            observed_p_value: self.value,
            false_rejection_bound: alpha,
            decision,
            calibration_digest: self.calibration_digest,
        })
    }
}

/// Decision from a collision-null p-value threshold test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionNullTestDecision {
    /// The observed statistic is in the predeclared rejection region.
    RejectCollisionNull,
    /// The observed statistic is not in the rejection region.
    FailToRejectCollisionNull,
}

/// Auditable result of a collision-null p-value threshold test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionNullTestReport {
    /// Null hypothesis that was tested.
    pub null_hypothesis: CollisionNullHypothesis,
    /// Observed p-value.
    pub observed_p_value: Rational,
    /// Declared upper bound on false rejection under the null assumptions.
    pub false_rejection_bound: Rational,
    /// Whether the null was rejected.
    pub decision: CollisionNullTestDecision,
    /// Digest of the measurement-error and test-calibration assumptions.
    pub calibration_digest: [u8; 32],
}

/// Condition under which a collision-detection frequency is defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionDetectionCondition {
    /// Probability of exceeding the action threshold given a direct hit.
    DirectHit,
    /// Probability of exceeding the action threshold for a true miss anywhere
    /// inside the combined hard-body region.
    AnyTrueMissWithinHardBody,
}

/// Frequentist probability that a declared decision policy detects a collision.
///
/// This is a policy-performance quantity conditional on a true collision,
/// not the probability that the conjunction itself will collide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionDetectionProbability {
    value: Rational,
    condition: CollisionDetectionCondition,
    policy_digest: [u8; 32],
}

impl CollisionDetectionProbability {
    /// Construct a calibrated detection probability in [0, 1].
    pub fn new(
        value: Rational,
        condition: CollisionDetectionCondition,
        policy_digest: [u8; 32],
    ) -> Result<Self, RiskSemanticsError> {
        if value > Rational::from_integer(1) {
            return Err(RiskSemanticsError::ProbabilityOutsideUnitInterval);
        }
        Ok(Self {
            value,
            condition,
            policy_digest,
        })
    }

    /// Return the collision-detection probability.
    pub const fn value(self) -> Rational {
        self.value
    }

    /// Return the true-collision condition under which it was calibrated.
    pub const fn condition(self) -> CollisionDetectionCondition {
        self.condition
    }

    /// Return the decision-policy identity.
    pub const fn policy_digest(self) -> [u8; 32] {
        self.policy_digest
    }
}

/// Expected number of collisions over a declared exposure horizon.
///
/// Unlike probability quantities, an expected count may be greater than one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedCollisionCount {
    value: Rational,
    exposure_digest: [u8; 32],
    environment_digest: [u8; 32],
}

impl ExpectedCollisionCount {
    /// Construct a nonnegative expected collision count and bind its evidence.
    pub const fn new(
        value: Rational,
        exposure_digest: [u8; 32],
        environment_digest: [u8; 32],
    ) -> Self {
        Self {
            value,
            exposure_digest,
            environment_digest,
        }
    }

    /// Return the expected collision count; this is not a probability.
    pub const fn value(self) -> Rational {
        self.value
    }

    /// Return the exposure-horizon identity.
    pub const fn exposure_digest(self) -> [u8; 32] {
        self.exposure_digest
    }

    /// Return the debris-environment / rate-model identity.
    pub const fn environment_digest(self) -> [u8; 32] {
        self.environment_digest
    }
}

/// One unique conjunction event in a mission-level aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissionConjunctionEvent {
    /// Common mission/model/time-horizon scope.
    pub scope: MissionRiskScope,
    /// Stable identity of the physical conjunction, not a CDM message revision.
    pub conjunction_id: u64,
    /// Digest of the specific CDM / orbit-assessment revision used.
    pub cdm_revision_digest: [u8; 32],
    /// Conditional event probability for this assessment revision.
    pub conditional_pc: ConditionalCollisionProbability,
}

/// Dependence treatment for aggregating marginal event probabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDependenceMode {
    /// No defensible event-dependence model was supplied; use union bounds.
    Unknown,
    /// Caller explicitly asserts mutual independence and supplies the digest
    /// of the assumption/qualification record. This module does not prove it.
    MutuallyIndependent {
        /// Identity of the caller's independence-assumption evidence.
        assumption_digest: [u8; 32],
    },
}

/// Exact mission-level risk result and dependence-free reference bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionCollisionRiskReport {
    /// Mission/model/time-horizon scope.
    pub scope: MissionRiskScope,
    /// Explicit event-dependence treatment used for the reported interval.
    pub dependence_mode: EventDependenceMode,
    /// Lower bound valid without a dependence assumption: maximum marginal Pc.
    pub dependence_free_lower_bound: Rational,
    /// Upper bound valid without a dependence assumption: min(1, sum of Pc).
    pub dependence_free_upper_bound: Rational,
    /// Reported lower probability under the selected dependence treatment.
    pub reported_lower_bound: Rational,
    /// Reported upper probability under the selected dependence treatment.
    pub reported_upper_bound: Rational,
    /// Number of unique conjunction events included.
    pub event_count: usize,
    /// Canonically ordered (conjunction ID, CDM revision digest) inventory.
    pub event_inventory: Vec<(u64, [u8; 32])>,
}

/// Errors that invalidate mission-level risk aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskSemanticsError {
    /// Mission horizon has equal or reversed endpoints.
    InvalidMissionHorizon,
    /// A probability quantity lies outside [0, 1].
    ProbabilityOutsideUnitInterval,
    /// P-value test threshold is not in (0, 1].
    InvalidTestThreshold,
    /// Event does not belong to the mission scope being aggregated.
    EventScopeMismatch,
    /// Event Pc values refer to a different target probability measure.
    ProbabilityMeasureMismatch,
    /// Event Pc values were conditioned on different mission information sets.
    ConditioningSetMismatch,
    /// Multiple CDM revisions were supplied as separate events for one
    /// physical conjunction, which would double-count the same encounter.
    DuplicateConjunctionIdentity,
    /// Exact rational arithmetic overflowed its u128 representation.
    ArithmeticOverflow,
}

impl core::fmt::Display for RiskSemanticsError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidMissionHorizon => "mission risk horizon must have increasing endpoints",
            Self::ProbabilityOutsideUnitInterval => "probability quantity must lie in [0, 1]",
            Self::InvalidTestThreshold => "p-value test threshold must lie in (0, 1]",
            Self::EventScopeMismatch => "mission event scope differs from aggregation scope",
            Self::ProbabilityMeasureMismatch => {
                "conditional Pc probability measure differs from mission target measure"
            }
            Self::ConditioningSetMismatch => {
                "conditional Pc information set differs from mission conditioning set"
            }
            Self::DuplicateConjunctionIdentity => {
                "physical conjunction is duplicated, possibly by multiple CDM revisions"
            }
            Self::ArithmeticOverflow => "mission risk aggregation arithmetic overflowed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RiskSemanticsError {}

/// Aggregate unique event-conditional Pc values into a mission-level risk result.
///
/// With unknown dependence, the sharp elementary union bounds are:
/// maximum marginal Pc <= probability of at least one event <= min(1, sum Pc).
///
/// With an explicit mutual-independence assumption, the reported interval is
/// the exact product result one minus the product of each one-minus-Pc.
/// The dependence-free bounds are still included for context. The assumption
/// digest is provenance, not proof of independence. An empty event set returns
/// exact zero.
///
/// The independence product is not even computed in Unknown mode. This avoids
/// failing a valid dependence-bounded result because an unused point estimate
/// would overflow the exact rational representation.
pub fn aggregate_mission_collision_risk(
    scope: MissionRiskScope,
    events: &[MissionConjunctionEvent],
    dependence_mode: EventDependenceMode,
) -> Result<MissionCollisionRiskReport, RiskSemanticsError> {
    let mut ids = BTreeSet::new();
    let mut inventory = Vec::with_capacity(events.len());
    let mut lower = Rational::from_integer(0);
    let mut sum = Rational::from_integer(0);
    let mut survival = match dependence_mode {
        EventDependenceMode::Unknown => None,
        EventDependenceMode::MutuallyIndependent { .. } => Some(Rational::from_integer(1)),
    };

    for event in events {
        if event.scope != scope {
            return Err(RiskSemanticsError::EventScopeMismatch);
        }
        if !ids.insert(event.conjunction_id) {
            return Err(RiskSemanticsError::DuplicateConjunctionIdentity);
        }

        if event.conditional_pc.probability_measure_digest() != scope.probability_measure_digest {
            return Err(RiskSemanticsError::ProbabilityMeasureMismatch);
        }
        if event.conditional_pc.conditioning_digest() != scope.conditioning_set_digest {
            return Err(RiskSemanticsError::ConditioningSetMismatch);
        }
        let pc = event.conditional_pc.value();
        if pc > Rational::from_integer(1) {
            return Err(RiskSemanticsError::ProbabilityOutsideUnitInterval);
        }

        if pc > lower {
            lower = pc;
        }
        sum = sum
            .checked_add(pc)
            .map_err(|_| RiskSemanticsError::ArithmeticOverflow)?;

        if let Some(product) = survival {
            let complement = Rational::from_integer(1)
                .checked_sub(pc)
                .map_err(|_| RiskSemanticsError::ProbabilityOutsideUnitInterval)?;
            survival = Some(
                product
                    .checked_mul(complement)
                    .map_err(|_| RiskSemanticsError::ArithmeticOverflow)?,
            );
        }
        inventory.push((event.conjunction_id, event.cdm_revision_digest));
    }

    let upper = if sum > Rational::from_integer(1) {
        Rational::from_integer(1)
    } else {
        sum
    };
    let (reported_lower_bound, reported_upper_bound) = match dependence_mode {
        EventDependenceMode::Unknown => (lower, upper),
        EventDependenceMode::MutuallyIndependent { .. } => {
            let probability = Rational::from_integer(1)
                .checked_sub(survival.expect("independence mode creates survival product"))
                .map_err(|_| RiskSemanticsError::ArithmeticOverflow)?;
            (probability, probability)
        }
    };

    inventory.sort_by_key(|item| item.0);

    Ok(MissionCollisionRiskReport {
        scope,
        dependence_mode,
        dependence_free_lower_bound: lower,
        dependence_free_upper_bound: upper,
        reported_lower_bound,
        reported_upper_bound,
        event_count: events.len(),
        event_inventory: inventory,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> MissionRiskScope {
        MissionRiskScope::new([1; 32], [2; 32], [3; 32], [4; 32], 0, 86_400).unwrap()
    }

    fn rational(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn pc(value: Rational) -> ConditionalCollisionProbability {
        ConditionalCollisionProbability::new(value, [3; 32], [4; 32]).unwrap()
    }

    fn event(id: u64, numerator: u128, denominator: u128) -> MissionConjunctionEvent {
        MissionConjunctionEvent {
            scope: scope(),
            conjunction_id: id,
            cdm_revision_digest: [id as u8; 32],
            conditional_pc: pc(rational(numerator, denominator)),
        }
    }

    #[test]
    fn unknown_dependence_returns_sharp_two_event_union_bounds() {
        let result = aggregate_mission_collision_risk(
            scope(),
            &[event(1, 1, 10), event(2, 1, 5)],
            EventDependenceMode::Unknown,
        )
        .unwrap();

        assert_eq!(result.dependence_free_lower_bound, rational(1, 5));
        assert_eq!(result.dependence_free_upper_bound, rational(3, 10));
        assert_eq!(result.reported_lower_bound, rational(1, 5));
        assert_eq!(result.reported_upper_bound, rational(3, 10));
    }

    #[test]
    fn explicit_independence_returns_exact_product_point_inside_bounds() {
        let result = aggregate_mission_collision_risk(
            scope(),
            &[event(1, 1, 10), event(2, 1, 5)],
            EventDependenceMode::MutuallyIndependent {
                assumption_digest: [9; 32],
            },
        )
        .unwrap();

        assert_eq!(result.reported_lower_bound, rational(7, 25));
        assert_eq!(result.reported_upper_bound, rational(7, 25));
        assert_eq!(result.dependence_free_lower_bound, rational(1, 5));
        assert_eq!(result.dependence_free_upper_bound, rational(3, 10));
    }

    #[test]
    fn dependence_free_upper_bound_is_capped_at_one() {
        let result = aggregate_mission_collision_risk(
            scope(),
            &[event(1, 2, 3), event(2, 3, 4)],
            EventDependenceMode::Unknown,
        )
        .unwrap();

        assert_eq!(result.dependence_free_lower_bound, rational(3, 4));
        assert_eq!(result.dependence_free_upper_bound, rational(1, 1));
    }

    #[test]
    fn identical_event_set_is_order_invariant() {
        let first = event(1, 1, 10);
        let second = event(2, 1, 5);
        let forward = aggregate_mission_collision_risk(
            scope(),
            &[first, second],
            EventDependenceMode::Unknown,
        )
        .unwrap();
        let reverse = aggregate_mission_collision_risk(
            scope(),
            &[second, first],
            EventDependenceMode::Unknown,
        )
        .unwrap();

        assert_eq!(forward, reverse);
    }

    #[test]
    fn duplicate_conjunction_with_two_cdm_revisions_is_rejected() {
        let mut duplicate_revision = event(1, 1, 10);
        duplicate_revision.cdm_revision_digest = [8; 32];
        assert_eq!(
            aggregate_mission_collision_risk(
                scope(),
                &[event(1, 1, 10), duplicate_revision],
                EventDependenceMode::Unknown,
            ),
            Err(RiskSemanticsError::DuplicateConjunctionIdentity)
        );
    }

    #[test]
    fn event_from_another_mission_or_model_fails_closed() {
        let mut changed_scope = scope();
        changed_scope.model_digest = [7; 32];
        let mut input = event(1, 1, 10);
        input.scope = changed_scope;
        assert_eq!(
            aggregate_mission_collision_risk(scope(), &[input], EventDependenceMode::Unknown,),
            Err(RiskSemanticsError::EventScopeMismatch)
        );
    }

    #[test]
    fn conditional_pcs_from_a_different_target_measure_cannot_be_aggregated() {
        let mut input = event(1, 1, 10);
        input.conditional_pc =
            ConditionalCollisionProbability::new(rational(1, 10), [99; 32], [4; 32]).unwrap();

        assert_eq!(
            aggregate_mission_collision_risk(
                scope(),
                &[input],
                EventDependenceMode::Unknown,
            ),
            Err(RiskSemanticsError::ProbabilityMeasureMismatch)
        );
    }

    #[test]
    fn event_pc_from_a_different_conditioning_set_cannot_be_aggregated() {
        let mut input = event(1, 1, 10);
        input.conditional_pc =
            ConditionalCollisionProbability::new(rational(1, 10), [3; 32], [99; 32]).unwrap();

        assert_eq!(
            aggregate_mission_collision_risk(
                scope(),
                &[input],
                EventDependenceMode::Unknown,
            ),
            Err(RiskSemanticsError::ConditioningSetMismatch)
        );
    }

    #[test]
    fn invalid_mission_horizon_is_rejected() {
        assert_eq!(
            MissionRiskScope::new([1; 32], [2; 32], [3; 32], [4; 32], 5, 5),
            Err(RiskSemanticsError::InvalidMissionHorizon)
        );
        assert_eq!(
            MissionRiskScope::new([1; 32], [2; 32], [3; 32], [4; 32], 6, 5),
            Err(RiskSemanticsError::InvalidMissionHorizon)
        );
    }

    #[test]
    fn conditional_pc_outside_unit_interval_is_rejected() {
        assert_eq!(
            ConditionalCollisionProbability::new(rational(11, 10), [3; 32], [4; 32]),
            Err(RiskSemanticsError::ProbabilityOutsideUnitInterval)
        );
    }

    #[test]
    fn p_value_is_a_distinct_typed_quantity_and_has_a_null_decision() {
        let p_value = CollisionNullPValue::new(
            rational(1, 100_000),
            CollisionNullHypothesis::DirectHit,
            [5; 32],
        )
        .unwrap();
        let result = p_value.test_at(rational(1, 10_000)).unwrap();

        assert_eq!(
            result.decision,
            CollisionNullTestDecision::RejectCollisionNull
        );
        assert_eq!(result.false_rejection_bound, rational(1, 10_000));
        assert_eq!(p_value.value(), rational(1, 100_000));
    }

    #[test]
    fn p_value_threshold_uses_strict_less_than() {
        let p_value = CollisionNullPValue::new(
            rational(1, 10_000),
            CollisionNullHypothesis::DirectHit,
            [5; 32],
        )
        .unwrap();
        let result = p_value.test_at(rational(1, 10_000)).unwrap();

        assert_eq!(
            result.decision,
            CollisionNullTestDecision::FailToRejectCollisionNull
        );
    }

    #[test]
    fn invalid_p_value_threshold_is_rejected() {
        let p_value =
            CollisionNullPValue::new(rational(1, 10), CollisionNullHypothesis::DirectHit, [5; 32])
                .unwrap();
        assert_eq!(
            p_value.test_at(Rational::from_integer(0)),
            Err(RiskSemanticsError::InvalidTestThreshold)
        );
        assert_eq!(
            p_value.test_at(rational(11, 10)),
            Err(RiskSemanticsError::InvalidTestThreshold)
        );
    }

    #[test]
    fn detection_probability_is_distinct_from_conditional_pc() {
        let detection = CollisionDetectionProbability::new(
            rational(7, 10),
            CollisionDetectionCondition::DirectHit,
            [6; 32],
        )
        .unwrap();

        assert_eq!(detection.value(), rational(7, 10));
        assert_eq!(
            detection.condition(),
            CollisionDetectionCondition::DirectHit
        );
    }

    #[test]
    fn expected_collision_count_is_not_capped_at_one() {
        let count = ExpectedCollisionCount::new(rational(7, 2), [7; 32], [8; 32]);
        assert_eq!(count.value(), rational(7, 2));
    }

    #[test]
    fn empty_event_set_returns_exact_zero_risk() {
        let result =
            aggregate_mission_collision_risk(scope(), &[], EventDependenceMode::Unknown).unwrap();

        assert_eq!(result.event_count, 0);
        assert_eq!(result.reported_lower_bound, Rational::from_integer(0));
        assert_eq!(result.reported_upper_bound, Rational::from_integer(0));
        assert!(result.event_inventory.is_empty());
    }

    #[test]
    fn unknown_dependence_does_not_compute_an_unneeded_independence_product() {
        let tiny = pc(rational(1, u128::MAX));
        let input = [
            MissionConjunctionEvent {
                scope: scope(),
                conjunction_id: 1,
                cdm_revision_digest: [1; 32],
                conditional_pc: tiny,
            },
            MissionConjunctionEvent {
                scope: scope(),
                conjunction_id: 2,
                cdm_revision_digest: [2; 32],
                conditional_pc: tiny,
            },
        ];

        let result =
            aggregate_mission_collision_risk(scope(), &input, EventDependenceMode::Unknown)
                .unwrap();
        assert_eq!(result.dependence_free_lower_bound, rational(1, u128::MAX));
        assert_eq!(result.dependence_free_upper_bound, rational(2, u128::MAX));
    }

    #[test]
    fn independent_product_overflow_fails_closed_when_exact_result_is_unrepresentable() {
        let tiny = pc(rational(1, u128::MAX));
        let input = [
            MissionConjunctionEvent {
                scope: scope(),
                conjunction_id: 1,
                cdm_revision_digest: [1; 32],
                conditional_pc: tiny,
            },
            MissionConjunctionEvent {
                scope: scope(),
                conjunction_id: 2,
                cdm_revision_digest: [2; 32],
                conditional_pc: tiny,
            },
        ];

        assert_eq!(
            aggregate_mission_collision_risk(
                scope(),
                &input,
                EventDependenceMode::MutuallyIndependent {
                    assumption_digest: [9; 32],
                },
            ),
            Err(RiskSemanticsError::ArithmeticOverflow)
        );
    }
}
