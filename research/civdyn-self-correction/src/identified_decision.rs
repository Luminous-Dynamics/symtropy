// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Auditable policy comparison with partially identified utility intervals.
//!
//! V0.1 uses a declared rectangular ambiguity set: policy utility intervals
//! are treated as independently attainable. Worst-case regret is therefore a
//! conservative upper bound when cross-policy dependence is unknown. This is
//! a transparent deterministic baseline, not a universal optimal decision
//! rule under partial identification.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::causal_transport::{CausalTransportResult, TransportIdentificationStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtilityDirection {
    Maximize,
    Minimize,
}

impl UtilityDirection {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Maximize => "maximize",
            Self::Minimize => "minimize",
        }
    }

    fn oriented(self, lower: f64, upper: f64) -> (f64, f64) {
        match self {
            Self::Maximize => (lower, upper),
            Self::Minimize => (-upper, -lower),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardConstraintRelation {
    AtMost,
    AtLeast,
}

impl HardConstraintRelation {
    pub const fn name(self) -> &'static str {
        match self {
            Self::AtMost => "at_most",
            Self::AtLeast => "at_least",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintIntervalStatus {
    RobustlyFeasible,
    DefinitelyInfeasible,
    Indeterminate,
}

impl ConstraintIntervalStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::RobustlyFeasible => "robustly_feasible",
            Self::DefinitelyInfeasible => "definitely_infeasible",
            Self::Indeterminate => "indeterminate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyIntervalFeasibility {
    RobustlyFeasible,
    DefinitelyInfeasible,
    Indeterminate,
}

impl PolicyIntervalFeasibility {
    pub const fn name(self) -> &'static str {
        match self {
            Self::RobustlyFeasible => "robustly_feasible",
            Self::DefinitelyInfeasible => "definitely_infeasible",
            Self::Indeterminate => "indeterminate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifiedDecisionClaimCeiling {
    MinimaxRegretWithinDeclaredRectangularSet,
    NoRobustlyFeasiblePolicy,
}

impl IdentifiedDecisionClaimCeiling {
    pub const fn name(self) -> &'static str {
        match self {
            Self::MinimaxRegretWithinDeclaredRectangularSet => {
                "minimax_regret_within_declared_rectangular_set"
            }
            Self::NoRobustlyFeasiblePolicy => "no_robustly_feasible_policy",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyUtilityBounds {
    pub policy_id: String,
    pub metric_id: String,
    pub estimand: String,
    pub measurement_unit: String,
    pub direction: UtilityDirection,
    pub lower: f64,
    pub upper: f64,
    pub evidence_digest: String,
    pub source_data_digest: String,
    pub target_population_digest: String,
    pub transport_protocol_digest: String,
}

impl PolicyUtilityBounds {
    /// Convert an identified or partially identified causal-transport result
    /// into a policy utility interval. Unidentified effects cannot be treated
    /// as finite utility bounds.
    pub fn from_causal_transport(
        policy_id: impl Into<String>,
        metric_id: impl Into<String>,
        measurement_unit: impl Into<String>,
        direction: UtilityDirection,
        transport: &CausalTransportResult,
    ) -> Result<Self, IdentifiedDecisionError> {
        let (lower, upper) = match transport.identification_status {
            TransportIdentificationStatus::Identified => {
                let point = transport
                    .target_ate_point
                    .ok_or(IdentifiedDecisionError::CausalTransportResultInconsistent)?;
                (point, point)
            }
            TransportIdentificationStatus::PartiallyIdentified => (
                transport
                    .target_ate_lower
                    .ok_or(IdentifiedDecisionError::CausalTransportResultInconsistent)?,
                transport
                    .target_ate_upper
                    .ok_or(IdentifiedDecisionError::CausalTransportResultInconsistent)?,
            ),
            TransportIdentificationStatus::NotIdentified => {
                return Err(IdentifiedDecisionError::CausalEffectNotIdentified);
            }
        };

        let policy = Self {
            policy_id: policy_id.into(),
            metric_id: metric_id.into(),
            estimand: transport.estimand.to_string(),
            measurement_unit: measurement_unit.into(),
            direction,
            lower,
            upper,
            evidence_digest: transport.protocol_digest.clone(),
            source_data_digest: transport.source_data_digest.clone(),
            target_population_digest: transport.target_population_digest.clone(),
            transport_protocol_digest: transport.protocol_digest.clone(),
        };
        validate_utility(&policy)?;
        Ok(policy)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HardConstraintSpec {
    pub id: String,
    pub measurement_unit: String,
    pub relation: HardConstraintRelation,
    pub threshold: f64,
    pub evidence_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyConstraintInterval {
    pub policy_id: String,
    pub constraint_id: String,
    pub lower: f64,
    pub upper: f64,
    pub evidence_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdentifiedDecisionInput {
    pub metric_id: String,
    pub measurement_unit: String,
    pub direction: UtilityDirection,
    pub utilities: Vec<PolicyUtilityBounds>,
    pub constraints: Vec<HardConstraintSpec>,
    pub constraint_intervals: Vec<PolicyConstraintInterval>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintIntervalResult {
    pub constraint_id: String,
    pub relation: HardConstraintRelation,
    pub measurement_unit: String,
    pub status: ConstraintIntervalStatus,
    pub evidence_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyIntervalResult {
    pub policy_id: String,
    pub utility_lower: f64,
    pub utility_upper: f64,
    pub utility_evidence_digest: String,
    pub source_data_digest: String,
    pub target_population_digest: String,
    pub transport_protocol_digest: String,
    pub feasibility: PolicyIntervalFeasibility,
    pub constraints: Vec<ConstraintIntervalResult>,
    /// Conservative regret upper bound in native utility units; present only
    /// for robustly feasible policies.
    pub worst_case_regret: Option<f64>,
    pub selected_as_minimax_regret: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdentifiedDecisionReport {
    pub metric_id: String,
    pub measurement_unit: String,
    pub direction: UtilityDirection,
    pub uncertainty_set: &'static str,
    pub robustly_feasible_policy_ids: Vec<String>,
    pub minimax_regret_policy_ids: Vec<String>,
    pub claim_ceiling: IdentifiedDecisionClaimCeiling,
    pub policies: Vec<PolicyIntervalResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifiedDecisionError {
    InvalidPortfolio,
    InvalidPolicyIdentity { policy_id: String },
    DuplicatePolicy { policy_id: String },
    MetricIdentityMismatch { policy_id: String },
    EstimandMismatch { policy_id: String },
    UnitMismatch { policy_id: String },
    TargetPopulationMismatch { policy_id: String },
    InvalidUtilityInterval { policy_id: String },
    MissingUtilityEvidence { policy_id: String },
    MissingHardConstraintIdentity,
    DuplicateHardConstraint { constraint_id: String },
    InvalidHardConstraintThreshold { constraint_id: String },
    MissingConstraintEvidence { constraint_id: String },
    UnknownPolicyConstraint { policy_id: String, constraint_id: String },
    UnknownConstraint { constraint_id: String },
    DuplicatePolicyConstraint { policy_id: String, constraint_id: String },
    InvalidConstraintInterval { policy_id: String, constraint_id: String },
    MissingPolicyConstraint { policy_id: String, constraint_id: String },
    CausalEffectNotIdentified,
    CausalTransportResultInconsistent,
    NonFiniteRegret { policy_id: String },
}

impl fmt::Display for IdentifiedDecisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for IdentifiedDecisionError {}

fn validate_utility(policy: &PolicyUtilityBounds) -> Result<(), IdentifiedDecisionError> {
    if policy.policy_id.trim().is_empty() || policy.metric_id.trim().is_empty() {
        return Err(IdentifiedDecisionError::InvalidPolicyIdentity {
            policy_id: policy.policy_id.clone(),
        });
    }
    if !policy.lower.is_finite() || !policy.upper.is_finite() || policy.lower > policy.upper {
        return Err(IdentifiedDecisionError::InvalidUtilityInterval {
            policy_id: policy.policy_id.clone(),
        });
    }
    if policy.evidence_digest.trim().is_empty()
        || policy.source_data_digest.trim().is_empty()
        || policy.target_population_digest.trim().is_empty()
        || policy.transport_protocol_digest.trim().is_empty()
    {
        return Err(IdentifiedDecisionError::MissingUtilityEvidence {
            policy_id: policy.policy_id.clone(),
        });
    }
    Ok(())
}


fn constraint_status(
    spec: &HardConstraintSpec,
    interval: &PolicyConstraintInterval,
) -> ConstraintIntervalStatus {
    let threshold = spec.threshold;
    match spec.relation {
        HardConstraintRelation::AtMost if interval.upper <= threshold => {
            ConstraintIntervalStatus::RobustlyFeasible
        }
        HardConstraintRelation::AtMost if interval.lower > threshold => {
            ConstraintIntervalStatus::DefinitelyInfeasible
        }
        HardConstraintRelation::AtLeast if interval.lower >= threshold => {
            ConstraintIntervalStatus::RobustlyFeasible
        }
        HardConstraintRelation::AtLeast if interval.upper < threshold => {
            ConstraintIntervalStatus::DefinitelyInfeasible
        }
        _ => ConstraintIntervalStatus::Indeterminate,
    }
}

/// Evaluate a finite policy set under rectangular utility ambiguity.
///
/// For maximizing utility, the regret upper bound of p is
/// max_{q != p} max(0, upper(q) - lower(p)). Minimization reverses the
/// utility interval before applying the same expression. A policy enters
/// the recommendation set only when every hard constraint is robustly
/// feasible across its full interval.
pub fn evaluate_identified_decision(
    input: &IdentifiedDecisionInput,
) -> Result<IdentifiedDecisionReport, IdentifiedDecisionError> {
    if input.metric_id.trim().is_empty()
        || input.measurement_unit.trim().is_empty()
        || input.utilities.len() < 2
    {
        return Err(IdentifiedDecisionError::InvalidPortfolio);
    }

    let mut policy_by_id = BTreeMap::new();
    let mut target_population: Option<&str> = None;
    let mut estimand: Option<&str> = None;
    for utility in &input.utilities {
        validate_utility(utility)?;
        if utility.metric_id != input.metric_id {
            return Err(IdentifiedDecisionError::MetricIdentityMismatch {
                policy_id: utility.policy_id.clone(),
            });
        }
        if utility.measurement_unit != input.measurement_unit {
            return Err(IdentifiedDecisionError::UnitMismatch {
                policy_id: utility.policy_id.clone(),
            });
        }
        if utility.direction != input.direction {
            return Err(IdentifiedDecisionError::MetricIdentityMismatch {
                policy_id: utility.policy_id.clone(),
            });
        }
        if estimand.is_some_and(|value| value != utility.estimand) {
            return Err(IdentifiedDecisionError::EstimandMismatch {
                policy_id: utility.policy_id.clone(),
            });
        }
        if target_population
            .is_some_and(|value| value != utility.target_population_digest)
        {
            return Err(IdentifiedDecisionError::TargetPopulationMismatch {
                policy_id: utility.policy_id.clone(),
            });
        }
        estimand.get_or_insert(utility.estimand.as_str());
        target_population.get_or_insert(utility.target_population_digest.as_str());
        if policy_by_id.insert(utility.policy_id.as_str(), utility).is_some() {
            return Err(IdentifiedDecisionError::DuplicatePolicy {
                policy_id: utility.policy_id.clone(),
            });
        }
    }

    let mut constraint_by_id = BTreeMap::new();
    for constraint in &input.constraints {
        let threshold = constraint.threshold;
        if constraint.id.trim().is_empty() || constraint.measurement_unit.trim().is_empty() {
            return Err(IdentifiedDecisionError::MissingHardConstraintIdentity);
        }
        if !threshold.is_finite() {
            return Err(IdentifiedDecisionError::InvalidHardConstraintThreshold {
                constraint_id: constraint.id.clone(),
            });
        }
        if constraint.evidence_digest.trim().is_empty() {
            return Err(IdentifiedDecisionError::MissingConstraintEvidence {
                constraint_id: constraint.id.clone(),
            });
        }
        if constraint_by_id
            .insert(constraint.id.as_str(), constraint)
            .is_some()
        {
            return Err(IdentifiedDecisionError::DuplicateHardConstraint {
                constraint_id: constraint.id.clone(),
            });
        }
    }

    let mut intervals_by_key = BTreeMap::new();
    for interval in &input.constraint_intervals {
        if !policy_by_id.contains_key(interval.policy_id.as_str()) {
            return Err(IdentifiedDecisionError::UnknownPolicyConstraint {
                policy_id: interval.policy_id.clone(),
                constraint_id: interval.constraint_id.clone(),
            });
        }
        if !constraint_by_id.contains_key(interval.constraint_id.as_str()) {
            return Err(IdentifiedDecisionError::UnknownConstraint {
                constraint_id: interval.constraint_id.clone(),
            });
        }
        if !interval.lower.is_finite()
            || !interval.upper.is_finite()
            || interval.lower > interval.upper
            || interval.evidence_digest.trim().is_empty()
        {
            return Err(IdentifiedDecisionError::InvalidConstraintInterval {
                policy_id: interval.policy_id.clone(),
                constraint_id: interval.constraint_id.clone(),
            });
        }
        let key = (interval.policy_id.as_str(), interval.constraint_id.as_str());
        if intervals_by_key.insert(key, interval).is_some() {
            return Err(IdentifiedDecisionError::DuplicatePolicyConstraint {
                policy_id: interval.policy_id.clone(),
                constraint_id: interval.constraint_id.clone(),
            });
        }
    }

    let mut constraint_rows_by_policy: BTreeMap<&str, Vec<ConstraintIntervalResult>> =
        BTreeMap::new();
    let mut feasibility_by_policy = BTreeMap::new();
    for utility in &input.utilities {
        let mut results = Vec::with_capacity(input.constraints.len());
        let mut any_definitely_infeasible = false;
        let mut any_indeterminate = false;

        for spec in &input.constraints {
            let interval = intervals_by_key
                .get(&(utility.policy_id.as_str(), spec.id.as_str()))
                .ok_or_else(|| IdentifiedDecisionError::MissingPolicyConstraint {
                    policy_id: utility.policy_id.clone(),
                    constraint_id: spec.id.clone(),
                })?;
            let status = constraint_status(spec, interval);
            any_definitely_infeasible |= status == ConstraintIntervalStatus::DefinitelyInfeasible;
            any_indeterminate |= status == ConstraintIntervalStatus::Indeterminate;
            results.push(ConstraintIntervalResult {
                constraint_id: spec.id.clone(),
                relation: spec.relation,
                measurement_unit: spec.measurement_unit.clone(),
                status,
                evidence_digest: interval.evidence_digest.clone(),
            });
        }

        let feasibility = if any_definitely_infeasible {
            PolicyIntervalFeasibility::DefinitelyInfeasible
        } else if any_indeterminate {
            PolicyIntervalFeasibility::Indeterminate
        } else {
            PolicyIntervalFeasibility::RobustlyFeasible
        };
        feasibility_by_policy.insert(utility.policy_id.as_str(), feasibility);
        constraint_rows_by_policy.insert(utility.policy_id.as_str(), results);
    }

    let robust_utilities: Vec<_> = input
        .utilities
        .iter()
        .filter(|utility| {
            feasibility_by_policy.get(utility.policy_id.as_str())
                == Some(&PolicyIntervalFeasibility::RobustlyFeasible)
        })
        .collect();

    let mut regret_by_policy = BTreeMap::new();
    for policy in &robust_utilities {
        let (own_lower, _) = input.direction.oriented(policy.lower, policy.upper);
        let mut worst_regret = 0.0_f64;
        for competitor in &robust_utilities {
            if competitor.policy_id == policy.policy_id {
                continue;
            }
            let (_, competitor_upper) =
                input.direction.oriented(competitor.lower, competitor.upper);
            worst_regret = worst_regret.max(competitor_upper - own_lower);
        }
        if !worst_regret.is_finite() {
            return Err(IdentifiedDecisionError::NonFiniteRegret {
                policy_id: policy.policy_id.clone(),
            });
        }
        regret_by_policy.insert(policy.policy_id.as_str(), worst_regret.max(0.0));
    }

    let minimum_regret = regret_by_policy.values().copied().reduce(f64::min);
    let minimax_policy_ids: BTreeSet<&str> = match minimum_regret {
        Some(minimum) => regret_by_policy
            .iter()
            .filter(|(_, regret)| **regret == minimum)
            .map(|(policy_id, _)| *policy_id)
            .collect(),
        None => BTreeSet::new(),
    };

    let mut policies = Vec::with_capacity(input.utilities.len());
    for utility in &input.utilities {
        let feasibility = feasibility_by_policy[utility.policy_id.as_str()];
        policies.push(PolicyIntervalResult {
            policy_id: utility.policy_id.clone(),
            utility_lower: utility.lower,
            utility_upper: utility.upper,
            utility_evidence_digest: utility.evidence_digest.clone(),
            source_data_digest: utility.source_data_digest.clone(),
            target_population_digest: utility.target_population_digest.clone(),
            transport_protocol_digest: utility.transport_protocol_digest.clone(),
            feasibility,
            constraints: constraint_rows_by_policy
                .remove(utility.policy_id.as_str())
                .unwrap_or_default(),
            worst_case_regret: regret_by_policy.get(utility.policy_id.as_str()).copied(),
            selected_as_minimax_regret: minimax_policy_ids
                .contains(utility.policy_id.as_str()),
        });
    }

    let robustly_feasible_policy_ids = robust_utilities
        .iter()
        .map(|policy| policy.policy_id.clone())
        .collect::<Vec<_>>();
    let minimax_regret_policy_ids = minimax_policy_ids
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let claim_ceiling = if robustly_feasible_policy_ids.is_empty() {
        IdentifiedDecisionClaimCeiling::NoRobustlyFeasiblePolicy
    } else {
        IdentifiedDecisionClaimCeiling::MinimaxRegretWithinDeclaredRectangularSet
    };

    Ok(IdentifiedDecisionReport {
        metric_id: input.metric_id.clone(),
        measurement_unit: input.measurement_unit.clone(),
        direction: input.direction,
        uncertainty_set: "independent_rectangular_policy_utility_intervals",
        robustly_feasible_policy_ids,
        minimax_regret_policy_ids,
        claim_ceiling,
        policies,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::causal_transport::{
        AssumptionDisposition, AssumptionEvidence, CausalTransportAssumptions,
        CausalTransportResult, TransportIdentificationReason,
    };

    fn utility(
        policy_id: &str,
        lower: f64,
        upper: f64,
    ) -> PolicyUtilityBounds {
        PolicyUtilityBounds {
            policy_id: policy_id.to_string(),
            metric_id: "service-benefit".to_string(),
            estimand: "target_average_treatment_effect_risk_difference".to_string(),
            measurement_unit: "risk_difference".to_string(),
            direction: UtilityDirection::Maximize,
            lower,
            upper,
            evidence_digest: format!("sha256-evidence-{policy_id}"),
            source_data_digest: "sha256-source-data".to_string(),
            target_population_digest: "sha256-target-population".to_string(),
            transport_protocol_digest: format!("sha256-protocol-{policy_id}"),
        }
    }

    fn constraint_spec() -> HardConstraintSpec {
        HardConstraintSpec {
            id: "water-use-limit".to_string(),
            measurement_unit: "litres_per_household_day".to_string(),
            relation: HardConstraintRelation::AtMost,
            threshold: 100.0,
            evidence_digest: "sha256-water-threshold".to_string(),
        }
    }

    fn constraint(policy_id: &str, lower: f64, upper: f64) -> PolicyConstraintInterval {
        PolicyConstraintInterval {
            policy_id: policy_id.to_string(),
            constraint_id: "water-use-limit".to_string(),
            lower,
            upper,
            evidence_digest: format!("sha256-water-{policy_id}"),
        }
    }

    fn input(utilities: Vec<PolicyUtilityBounds>) -> IdentifiedDecisionInput {
        let intervals = utilities
            .iter()
            .map(|utility| constraint(&utility.policy_id, 40.0, 60.0))
            .collect();
        IdentifiedDecisionInput {
            metric_id: "service-benefit".to_string(),
            measurement_unit: "risk_difference".to_string(),
            direction: UtilityDirection::Maximize,
            utilities,
            constraints: vec![constraint_spec()],
            constraint_intervals: intervals,
        }
    }

    fn transport_result(
        status: TransportIdentificationStatus,
        point: Option<f64>,
        lower: Option<f64>,
        upper: Option<f64>,
    ) -> CausalTransportResult {
        let evidence = AssumptionEvidence {
            disposition: AssumptionDisposition::SupportedByDesign,
            evidence_digest: "sha256-assumption".to_string(),
        };
        CausalTransportResult {
            estimand: "target_average_treatment_effect_risk_difference",
            identification_status: status,
            identification_reason: TransportIdentificationReason::FullSupportAndAssumptionsSupported,
            source_ate: 0.2,
            target_ate_point: point,
            target_ate_lower: lower,
            target_ate_upper: upper,
            source_population_total: 100,
            target_population_total: 100,
            supported_target_population_count: 100,
            unsupported_target_population_count: 0,
            supported_target_mass: 1.0,
            unsupported_target_mass: 0.0,
            assumptions: CausalTransportAssumptions {
                source_internal_validity: evidence.clone(),
                selection_exchangeability: evidence.clone(),
                no_interference: evidence.clone(),
                treatment_version_invariance: evidence,
            },
            source_data_digest: "sha256-source-data".to_string(),
            target_population_digest: "sha256-target-population".to_string(),
            protocol_digest: "sha256-transport-protocol".to_string(),
            strata: Vec::new(),
        }
    }

    #[test]
    fn converts_identified_and_partially_identified_transport_results() {
        let point = PolicyUtilityBounds::from_causal_transport(
            "point-policy",
            "service-benefit",
            "risk_difference",
            UtilityDirection::Maximize,
            &transport_result(
                TransportIdentificationStatus::Identified,
                Some(0.28),
                None,
                None,
            ),
        )
        .expect("point identified policy");
        assert_eq!(point.lower, 0.28);
        assert_eq!(point.upper, 0.28);

        let partial = PolicyUtilityBounds::from_causal_transport(
            "partial-policy",
            "service-benefit",
            "risk_difference",
            UtilityDirection::Maximize,
            &transport_result(
                TransportIdentificationStatus::PartiallyIdentified,
                None,
                Some(0.02),
                Some(0.42),
            ),
        )
        .expect("partially identified policy");
        assert_eq!(partial.lower, 0.02);
        assert_eq!(partial.upper, 0.42);

        assert!(matches!(
            PolicyUtilityBounds::from_causal_transport(
                "unknown-policy",
                "service-benefit",
                "risk_difference",
                UtilityDirection::Maximize,
                &transport_result(
                    TransportIdentificationStatus::NotIdentified,
                    None,
                    None,
                    None
                )
            ),
            Err(IdentifiedDecisionError::CausalEffectNotIdentified)
        ));
    }

    #[test]
    fn interval_minimax_regret_preserves_ties_and_native_units() {
        let candidate = input(vec![
            utility("alpha", 0.2, 0.4),
            utility("beta", 0.1, 0.5),
            utility("gamma", 0.0, 0.2),
        ]);
        let report = evaluate_identified_decision(&candidate).expect("valid interval portfolio");
        assert_eq!(report.measurement_unit, "risk_difference");
        assert_eq!(report.uncertainty_set, "independent_rectangular_policy_utility_intervals");
        assert_eq!(report.minimax_regret_policy_ids, vec!["alpha", "beta"]);

        let alpha = report.policies.iter().find(|row| row.policy_id == "alpha").unwrap();
        let beta = report.policies.iter().find(|row| row.policy_id == "beta").unwrap();
        let gamma = report.policies.iter().find(|row| row.policy_id == "gamma").unwrap();
        assert_eq!(alpha.worst_case_regret, Some(0.3));
        assert_eq!(beta.worst_case_regret, Some(0.3));
        assert_eq!(gamma.worst_case_regret, Some(0.5));
        assert!(alpha.selected_as_minimax_regret);
        assert!(beta.selected_as_minimax_regret);
        assert!(!gamma.selected_as_minimax_regret);
    }

    #[test]
    fn minimax_regret_matches_exhaustive_rectangular_endpoint_oracle() {
        let candidate = input(vec![
            utility("alpha", 0.2, 0.4),
            utility("beta", 0.1, 0.5),
            utility("gamma", 0.0, 0.2),
        ]);
        let report = evaluate_identified_decision(&candidate).expect("valid interval portfolio");

        for (policy_index, policy) in report.policies.iter().enumerate() {
            let mut maximum_realized_regret = 0.0_f64;
            for corner in 0..8 {
                let values: Vec<f64> = candidate
                    .utilities
                    .iter()
                    .enumerate()
                    .map(|(index, utility)| {
                        if (corner >> index) & 1 == 0 {
                            utility.lower
                        } else {
                            utility.upper
                        }
                    })
                    .collect();
                let best = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                maximum_realized_regret =
                    maximum_realized_regret.max(best - values[policy_index]);
            }
            assert_eq!(policy.worst_case_regret, Some(maximum_realized_regret));
        }
    }

    #[test]
    fn only_fully_robust_constraint_satisfaction_is_recommendable() {
        let mut candidate = input(vec![
            utility("robust", 0.2, 0.4),
            utility("straddling", 0.3, 0.5),
            utility("infeasible", 0.5, 0.7),
        ]);
        candidate.constraint_intervals = vec![
            constraint("robust", 40.0, 60.0),
            constraint("straddling", 90.0, 110.0),
            constraint("infeasible", 120.0, 140.0),
        ];
        let report = evaluate_identified_decision(&candidate).expect("valid constraints");
        assert_eq!(report.robustly_feasible_policy_ids, vec!["robust"]);
        let straddling = report.policies.iter().find(|row| row.policy_id == "straddling").unwrap();
        let infeasible = report.policies.iter().find(|row| row.policy_id == "infeasible").unwrap();
        assert_eq!(straddling.feasibility, PolicyIntervalFeasibility::Indeterminate);
        assert_eq!(infeasible.feasibility, PolicyIntervalFeasibility::DefinitelyInfeasible);
        assert_eq!(
            report.policies.iter().find(|row| row.policy_id == "robust").unwrap().worst_case_regret,
            Some(0.0)
        );
    }

    #[test]
    fn no_robustly_feasible_policy_returns_no_recommendation() {
        let mut candidate = input(vec![utility("a", 0.2, 0.3), utility("b", 0.1, 0.4)]);
        candidate.constraint_intervals = vec![constraint("a", 110.0, 120.0), constraint("b", 101.0, 130.0)];
        let report = evaluate_identified_decision(&candidate).expect("reportable no-feasible case");
        assert!(report.robustly_feasible_policy_ids.is_empty());
        assert!(report.minimax_regret_policy_ids.is_empty());
        assert_eq!(
            report.claim_ceiling,
            IdentifiedDecisionClaimCeiling::NoRobustlyFeasiblePolicy
        );
    }

    #[test]
    fn rejects_incompatible_estimands_targets_and_invalid_evidence() {
        let mut estimand = input(vec![utility("a", 0.1, 0.2), utility("b", 0.2, 0.3)]);
        estimand.utilities[1].estimand = "different_estimand".to_string();
        assert!(matches!(
            evaluate_identified_decision(&estimand),
            Err(IdentifiedDecisionError::EstimandMismatch { .. })
        ));

        let mut target = input(vec![utility("a", 0.1, 0.2), utility("b", 0.2, 0.3)]);
        target.utilities[1].target_population_digest = "sha256-other-target".to_string();
        assert!(matches!(
            evaluate_identified_decision(&target),
            Err(IdentifiedDecisionError::TargetPopulationMismatch { .. })
        ));

        let mut evidence = input(vec![utility("a", 0.1, 0.2), utility("b", 0.2, 0.3)]);
        evidence.utilities[0].evidence_digest.clear();
        assert!(matches!(
            evaluate_identified_decision(&evidence),
            Err(IdentifiedDecisionError::MissingUtilityEvidence { .. })
        ));
    }

    #[test]
    fn rejects_incomplete_duplicate_and_nonfinite_constraint_evidence() {
        let mut missing = input(vec![utility("a", 0.1, 0.2), utility("b", 0.2, 0.3)]);
        missing.constraint_intervals.pop();
        assert!(matches!(
            evaluate_identified_decision(&missing),
            Err(IdentifiedDecisionError::MissingPolicyConstraint { .. })
        ));

        let mut duplicate = input(vec![utility("a", 0.1, 0.2), utility("b", 0.2, 0.3)]);
        duplicate.constraint_intervals.push(duplicate.constraint_intervals[0].clone());
        assert!(matches!(
            evaluate_identified_decision(&duplicate),
            Err(IdentifiedDecisionError::DuplicatePolicyConstraint { .. })
        ));

        let mut nonfinite = input(vec![utility("a", 0.1, 0.2), utility("b", 0.2, 0.3)]);
        nonfinite.constraint_intervals[0].upper = f64::INFINITY;
        assert!(matches!(
            evaluate_identified_decision(&nonfinite),
            Err(IdentifiedDecisionError::InvalidConstraintInterval { .. })
        ));
    }

    #[test]
    fn minimizing_objectives_reverse_the_interval_before_regret() {
        let mut candidate = input(vec![utility("cheap", 10.0, 15.0), utility("expensive", 12.0, 20.0)]);
        candidate.metric_id = "cost".to_string();
        candidate.measurement_unit = "litres".to_string();
        candidate.direction = UtilityDirection::Minimize;
        for utility in &mut candidate.utilities {
            utility.metric_id = "cost".to_string();
            utility.measurement_unit = "litres".to_string();
            utility.direction = UtilityDirection::Minimize;
        }
        let report = evaluate_identified_decision(&candidate).expect("minimization portfolio");
        assert_eq!(report.minimax_regret_policy_ids, vec!["cheap"]);
        assert_eq!(report.policies[0].worst_case_regret, Some(3.0));
        assert_eq!(report.policies[1].worst_case_regret, Some(10.0));
    }
}
