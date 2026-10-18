// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Dependency-free reference evaluator for MINE-SYSTEM-001.
//!
//! This crate tests accounting and fail-closed admission semantics only. It is
//! not a rock-cutting model, mine planner, geotechnical solver, safety system,
//! energy-source qualification, or field-performance predictor.
//!
//! All quantities use explicitly named SI-derived units in field names.
//! Synthetic fixtures are not empirical mine measurements.

use std::fmt;

/// Evidence must be current, applicable and qualified before it supports an
/// eligible campaign result. Similar-looking but weaker states are distinct.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceState {
    Qualified,
    Unknown,
    Stale,
    Conflicting,
    OutOfDomain,
}

/// A required hard constraint is either satisfied, violated, or unresolved.
/// Unknown must never be interpreted as Satisfied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintState {
    Satisfied,
    Violated,
    Unknown,
}

/// Product specification disposition is separate from tonnage and economics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductState {
    MeetsSpecification,
    Rejected,
    Unknown,
}

/// One declared balance boundary, e.g. a material ledger (kg) or energy
/// ledger (kWh). Keep unlike material species and energy carriers in separate
/// ledgers; this type does not authorize adding incompatible quantities.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BalanceLedger {
    pub opening: f64,
    pub inputs: f64,
    pub recoveries: f64,
    pub outputs: f64,
    pub losses: f64,
    pub closing: f64,
    /// Absolute residual tolerance in the same declared unit as the ledger.
    pub tolerance: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BalanceAudit {
    /// Positive means the left-hand side exceeds the accounted right-hand side.
    pub residual: f64,
    pub within_tolerance: bool,
}

impl BalanceLedger {
    /// Residual = opening + inputs + recoveries - outputs - losses - closing.
    pub fn audit(self) -> Result<BalanceAudit, &'static str> {
        let values = [
            self.opening,
            self.inputs,
            self.recoveries,
            self.outputs,
            self.losses,
            self.closing,
            self.tolerance,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err("balance contains a non-finite quantity");
        }
        if values[..6].iter().any(|value| *value < 0.0) {
            return Err("balance quantities must be non-negative");
        }
        if self.tolerance < 0.0 {
            return Err("balance tolerance must be non-negative");
        }

        let residual = self.opening + self.inputs + self.recoveries
            - self.outputs
            - self.losses
            - self.closing;
        if !residual.is_finite() {
            return Err("balance arithmetic overflowed");
        }

        Ok(BalanceAudit {
            residual,
            within_tolerance: residual.abs() <= self.tolerance,
        })
    }
}

/// Candidate outputs stay multi-objective: throughput is maximized; unit cost,
/// energy intensity, water intensity and emissions intensity are minimized.
/// None of these metrics replaces hard safety/quality constraints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateMetrics {
    pub accepted_tonnes_per_shift: f64,
    pub lifecycle_cost_per_saleable_tonne: f64,
    pub energy_kwh_per_saleable_tonne: f64,
    pub water_m3_per_saleable_tonne: f64,
    pub emissions_kg_co2e_per_saleable_tonne: f64,
}

impl CandidateMetrics {
    pub fn validate(self) -> Result<(), &'static str> {
        let values = [
            self.accepted_tonnes_per_shift,
            self.lifecycle_cost_per_saleable_tonne,
            self.energy_kwh_per_saleable_tonne,
            self.water_m3_per_saleable_tonne,
            self.emissions_kg_co2e_per_saleable_tonne,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err("candidate metrics contain a non-finite quantity");
        }
        if values.iter().any(|value| *value < 0.0) {
            return Err("candidate metrics must be non-negative");
        }
        if self.accepted_tonnes_per_shift <= 0.0 {
            return Err("eligible productive candidate must have positive accepted output");
        }
        Ok(())
    }

    /// Return true only when self is no worse in every declared objective
    /// and strictly better in at least one. Exact finite comparisons are used;
    /// there is no hidden tolerance or weighted aggregate score.
    pub fn dominates(self, other: Self) -> bool {
        if self.validate().is_err() || other.validate().is_err() {
            return false;
        }

        let no_worse = self.accepted_tonnes_per_shift >= other.accepted_tonnes_per_shift
            && self.lifecycle_cost_per_saleable_tonne
                <= other.lifecycle_cost_per_saleable_tonne
            && self.energy_kwh_per_saleable_tonne <= other.energy_kwh_per_saleable_tonne
            && self.water_m3_per_saleable_tonne <= other.water_m3_per_saleable_tonne
            && self.emissions_kg_co2e_per_saleable_tonne
                <= other.emissions_kg_co2e_per_saleable_tonne;
        let strictly_better =
            self.accepted_tonnes_per_shift > other.accepted_tonnes_per_shift
                || self.lifecycle_cost_per_saleable_tonne
                    < other.lifecycle_cost_per_saleable_tonne
                || self.energy_kwh_per_saleable_tonne
                    < other.energy_kwh_per_saleable_tonne
                || self.water_m3_per_saleable_tonne
                    < other.water_m3_per_saleable_tonne
                || self.emissions_kg_co2e_per_saleable_tonne
                    < other.emissions_kg_co2e_per_saleable_tonne;

        no_worse && strictly_better
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampaignCase {
    pub scenario_id: String,
    pub evidence: Vec<EvidenceState>,
    pub hard_constraints: Vec<ConstraintState>,
    pub product_state: ProductState,
    /// Each ledger must represent one declared material stream or energy
    /// carrier, with consistent units and a documented system boundary.
    pub mass_ledger_kg: BalanceLedger,
    pub energy_ledger_kwh: BalanceLedger,
    pub metrics: CandidateMetrics,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    /// All supplied gates are satisfied for this synthetic reference case.
    Eligible,
    /// A required input, evidence state or constraint remains unresolved.
    NeedsEvidence,
    /// A required hard constraint is violated or product fails specification.
    Infeasible,
    /// A numerically valid ledger does not close within its declared tolerance.
    BalanceFailure,
    /// Input is malformed, non-finite, negative or internally unrepresentable.
    InvalidInput,
}

impl fmt::Display for Disposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Eligible => "ELIGIBLE_SYNTHETIC",
            Self::NeedsEvidence => "NEEDS_EVIDENCE",
            Self::Infeasible => "INFEASIBLE",
            Self::BalanceFailure => "BALANCE_FAILURE",
            Self::InvalidInput => "INVALID_INPUT",
        };
        f.write_str(label)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    pub disposition: Disposition,
    pub mass_residual_kg: Option<f64>,
    pub energy_residual_kwh: Option<f64>,
    pub reasons: Vec<&'static str>,
}

fn invalid_metrics(metrics: CandidateMetrics) -> bool {
    metrics.validate().is_err()
}

/// Evaluate only the explicit contract fields. This function does not run a
/// physical model and cannot qualify a real mine, power plant or work plan.
pub fn evaluate(case: &CampaignCase) -> Evaluation {
    let mut reasons = Vec::new();

    if case.scenario_id.trim().is_empty() {
        reasons.push("scenario identity is empty");
    }
    if invalid_metrics(case.metrics) {
        reasons.push("candidate metrics are invalid");
    }

    let mass_audit = case.mass_ledger_kg.audit();
    let energy_audit = case.energy_ledger_kwh.audit();

    if mass_audit.is_err() {
        reasons.push("mass ledger is invalid");
    }
    if energy_audit.is_err() {
        reasons.push("energy ledger is invalid");
    }

    if !reasons.is_empty() {
        return Evaluation {
            disposition: Disposition::InvalidInput,
            mass_residual_kg: mass_audit.ok().map(|audit| audit.residual),
            energy_residual_kwh: energy_audit.ok().map(|audit| audit.residual),
            reasons,
        };
    }

    let mass_audit = mass_audit.expect("validated above");
    let energy_audit = energy_audit.expect("validated above");

    if !mass_audit.within_tolerance {
        reasons.push("mass ledger does not close within declared tolerance");
    }
    if !energy_audit.within_tolerance {
        reasons.push("energy ledger does not close within declared tolerance");
    }
    if !reasons.is_empty() {
        return Evaluation {
            disposition: Disposition::BalanceFailure,
            mass_residual_kg: Some(mass_audit.residual),
            energy_residual_kwh: Some(energy_audit.residual),
            reasons,
        };
    }

    if case.hard_constraints.contains(&ConstraintState::Violated) {
        reasons.push("one or more required hard constraints are violated");
    }
    if case.product_state == ProductState::Rejected {
        reasons.push("product does not meet its declared specification");
    }
    if !reasons.is_empty() {
        return Evaluation {
            disposition: Disposition::Infeasible,
            mass_residual_kg: Some(mass_audit.residual),
            energy_residual_kwh: Some(energy_audit.residual),
            reasons,
        };
    }

    // Empty evidence/constraint lists are explicitly unresolved, never vacuous
    // proof that all required evidence or safety constraints passed.
    if case.evidence.is_empty() {
        reasons.push("required evidence list is empty");
    }
    if case.hard_constraints.is_empty() {
        reasons.push("required hard-constraint list is empty");
    }
    if case.evidence.iter().any(|state| *state != EvidenceState::Qualified) {
        reasons.push("one or more evidence states are not qualified and current");
    }
    if case
        .hard_constraints
        .iter()
        .any(|state| *state != ConstraintState::Satisfied)
    {
        reasons.push("one or more hard constraints are unresolved");
    }
    if case.product_state == ProductState::Unknown {
        reasons.push("product-specification disposition is unknown");
    }

    if !reasons.is_empty() {
        return Evaluation {
            disposition: Disposition::NeedsEvidence,
            mass_residual_kg: Some(mass_audit.residual),
            energy_residual_kwh: Some(energy_audit.residual),
            reasons,
        };
    }

    Evaluation {
        disposition: Disposition::Eligible,
        mass_residual_kg: Some(mass_audit.residual),
        energy_residual_kwh: Some(energy_audit.residual),
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balanced_ledger() -> BalanceLedger {
        BalanceLedger {
            opening: 100.0,
            inputs: 50.0,
            recoveries: 0.0,
            outputs: 120.0,
            losses: 10.0,
            closing: 20.0,
            tolerance: 1e-9,
        }
    }

    fn metrics() -> CandidateMetrics {
        CandidateMetrics {
            accepted_tonnes_per_shift: 42.0,
            lifecycle_cost_per_saleable_tonne: 80.0,
            energy_kwh_per_saleable_tonne: 12.0,
            water_m3_per_saleable_tonne: 0.2,
            emissions_kg_co2e_per_saleable_tonne: 4.0,
        }
    }

    fn eligible_case() -> CampaignCase {
        let mass_ledger_kg = balanced_ledger();
        let energy_ledger_kwh = BalanceLedger {
            opening: 30.0,
            inputs: 100.0,
            recoveries: 5.0,
            outputs: 110.0,
            losses: 5.0,
            closing: 20.0,
            tolerance: 1e-9,
        };
        CampaignCase {
            scenario_id: "MS-001-NOMINAL-SYNTHETIC".to_owned(),
            evidence: vec![EvidenceState::Qualified],
            hard_constraints: vec![ConstraintState::Satisfied],
            product_state: ProductState::MeetsSpecification,
            mass_ledger_kg,
            energy_ledger_kwh,
            metrics: metrics(),
        }
    }

    #[test]
    fn balanced_synthetic_fixture_is_only_contract_eligible() {
        let evaluated = evaluate(&eligible_case());
        assert_eq!(evaluated.disposition, Disposition::Eligible);
        assert!(evaluated.mass_residual_kg.unwrap().abs() <= 1e-9);
        assert!(evaluated.energy_residual_kwh.unwrap().abs() <= 1e-9);
    }

    #[test]
    fn unknown_or_stale_evidence_never_passes() {
        for state in [
            EvidenceState::Unknown,
            EvidenceState::Stale,
            EvidenceState::Conflicting,
            EvidenceState::OutOfDomain,
        ] {
            let mut case = eligible_case();
            case.evidence = vec![EvidenceState::Qualified, state];
            assert_eq!(evaluate(&case).disposition, Disposition::NeedsEvidence);
        }
    }

    #[test]
    fn empty_evidence_or_constraint_set_is_not_vacuous_success() {
        let mut no_evidence = eligible_case();
        no_evidence.evidence.clear();
        assert_eq!(evaluate(&no_evidence).disposition, Disposition::NeedsEvidence);

        let mut no_constraints = eligible_case();
        no_constraints.hard_constraints.clear();
        assert_eq!(
            evaluate(&no_constraints).disposition,
            Disposition::NeedsEvidence
        );
    }

    #[test]
    fn violated_constraint_and_rejected_product_are_infeasible() {
        let mut unsafe_case = eligible_case();
        unsafe_case.hard_constraints = vec![ConstraintState::Violated];
        assert_eq!(evaluate(&unsafe_case).disposition, Disposition::Infeasible);

        let mut rejected_case = eligible_case();
        rejected_case.product_state = ProductState::Rejected;
        assert_eq!(evaluate(&rejected_case).disposition, Disposition::Infeasible);
    }

    #[test]
    fn unresolved_constraint_does_not_pass() {
        let mut case = eligible_case();
        case.hard_constraints = vec![ConstraintState::Satisfied, ConstraintState::Unknown];
        assert_eq!(evaluate(&case).disposition, Disposition::NeedsEvidence);
    }

    #[test]
    fn unbalanced_mass_or_energy_is_not_eligible() {
        let mut mass_case = eligible_case();
        mass_case.mass_ledger_kg.outputs += 1.0;
        assert_eq!(evaluate(&mass_case).disposition, Disposition::BalanceFailure);

        let mut energy_case = eligible_case();
        energy_case.energy_ledger_kwh.inputs += 3.0;
        assert_eq!(evaluate(&energy_case).disposition, Disposition::BalanceFailure);
    }

    #[test]
    fn nonfinite_or_negative_quantity_is_invalid_input() {
        let mut nan_case = eligible_case();
        nan_case.mass_ledger_kg.inputs = f64::NAN;
        assert_eq!(evaluate(&nan_case).disposition, Disposition::InvalidInput);

        let mut negative_case = eligible_case();
        negative_case.energy_ledger_kwh.losses = -1.0;
        assert_eq!(evaluate(&negative_case).disposition, Disposition::InvalidInput);

        let mut no_output = eligible_case();
        no_output.metrics.accepted_tonnes_per_shift = 0.0;
        assert_eq!(evaluate(&no_output).disposition, Disposition::InvalidInput);
    }

    #[test]
    fn pareto_dominance_does_not_hide_tradeoffs() {
        let better = CandidateMetrics {
            accepted_tonnes_per_shift: 45.0,
            lifecycle_cost_per_saleable_tonne: 75.0,
            energy_kwh_per_saleable_tonne: 10.0,
            water_m3_per_saleable_tonne: 0.18,
            emissions_kg_co2e_per_saleable_tonne: 3.5,
        };
        let baseline = metrics();
        assert!(better.dominates(baseline));
        assert!(!baseline.dominates(better));
        assert!(!baseline.dominates(baseline));

        let tradeoff = CandidateMetrics {
            accepted_tonnes_per_shift: 47.0,
            lifecycle_cost_per_saleable_tonne: 83.0,
            energy_kwh_per_saleable_tonne: 11.0,
            water_m3_per_saleable_tonne: 0.19,
            emissions_kg_co2e_per_saleable_tonne: 3.8,
        };
        assert!(!tradeoff.dominates(baseline));
        assert!(!baseline.dominates(tradeoff));
    }
}
