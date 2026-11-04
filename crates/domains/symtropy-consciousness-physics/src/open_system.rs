// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Explicit energy-balance contracts for declared open-system boundaries.
//!
//! This module is deliberately separate from the legacy per-tick
//! `ThermodynamicLedger`. It evaluates a first-law balance only when the caller
//! supplies both stored-energy endpoints and an explicit inventory of every
//! energy flow crossing one declared system boundary.
//!
//! The sign convention is:
//! - work and heat entering the boundary are positive inputs;
//! - work and heat leaving the boundary are positive outputs;
//! - stored-energy change is final minus initial.
//!
//! Internal transfers between subsystems wholly inside the declared boundary
//! cancel in the total-system balance and must not be recorded as external
//! flows. Reservoir IDs identify the external source or sink for each boundary
//! crossing. This evaluator is an accounting validator, not a physical-model
//! validator: it cannot establish that the boundary or measurement inventory
//! is physically complete.

use std::collections::{BTreeSet, HashSet};

/// Energy terms required to evaluate a complete first-law balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyBalanceTerm {
    InitialStoredEnergy,
    FinalStoredEnergy,
    BoundaryFlows,
    UncertaintyBounds,
}

impl EnergyBalanceTerm {
    /// Stable machine-readable field name for reports and receipts.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InitialStoredEnergy => "initial_stored_energy_j",
            Self::FinalStoredEnergy => "final_stored_energy_j",
            Self::BoundaryFlows => "boundary_flows",
            Self::UncertaintyBounds => "uncertainty_bounds",
        }
    }
}

/// Energy carrier crossing the declared system boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnergyCarrier {
    Work,
    Heat,
}

/// Direction of an energy flow relative to the declared boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlowDirection {
    IntoBoundary,
    OutOfBoundary,
}

/// One explicit energy flow crossing the declared system boundary.
///
/// `flow_id` must be unique within the evaluated interval. `reservoir_id`
/// identifies the external source or sink. `energy_j` is a non-negative
/// magnitude; direction is represented separately and must not be encoded by
/// a negative amount. The conservative absolute uncertainty bound is required
/// for a determinate balance classification; unknown uncertainty is not zero.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundaryEnergyFlow {
    pub flow_id: String,
    pub reservoir_id: String,
    pub carrier: EnergyCarrier,
    pub direction: FlowDirection,
    pub energy_j: f64,
    /// Conservative absolute uncertainty bound for this flow, in joules.
    /// `None` means the uncertainty is unknown, not zero.
    pub uncertainty_bound_j: Option<f64>,
}

/// Tolerance used to classify an energy-balance residual.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyBalanceTolerance {
    /// Absolute tolerance in joules.
    pub absolute_j: f64,
    /// Relative tolerance against the larger of the stored-energy change and
    /// gross boundary-flow magnitude.
    pub relative: f64,
}

impl EnergyBalanceTolerance {
    /// Construct a tolerance, rejecting negative or non-finite values.
    pub fn new(absolute_j: f64, relative: f64) -> Result<Self, EnergyBalanceError> {
        if !absolute_j.is_finite() || absolute_j < 0.0 {
            return Err(EnergyBalanceError::InvalidTolerance {
                field: "absolute_j",
            });
        }
        if !relative.is_finite() || relative < 0.0 {
            return Err(EnergyBalanceError::InvalidTolerance { field: "relative" });
        }
        Ok(Self {
            absolute_j,
            relative,
        })
    }
}

/// Complete or partial first-law data for one declared system boundary.
///
/// Every optional field is semantically meaningful: `None` means that the
/// quantity was not supplied or is unknown; `Some(0.0)` means it was
/// explicitly accounted for and measured/modelled as zero. For flow inventory,
/// `Some(vec![])` explicitly declares that no boundary-crossing flows occurred.
/// Missing values produce `Incomplete`, never an implicit zero.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenSystemEnergyBalance {
    /// Non-empty identifier for the exact system boundary being evaluated.
    pub boundary_id: String,
    /// Non-empty identifier for the exact time interval represented by both
    /// stored-energy endpoints and every listed boundary flow.
    pub interval_id: String,
    /// Total stored energy inside the boundary at the start, in joules.
    pub initial_stored_energy_j: Option<f64>,
    /// Conservative absolute uncertainty bound for the initial stored energy.
    pub initial_stored_energy_uncertainty_bound_j: Option<f64>,
    /// Total stored energy inside the boundary at the end, in joules.
    pub final_stored_energy_j: Option<f64>,
    /// Conservative absolute uncertainty bound for the final stored energy.
    pub final_stored_energy_uncertainty_bound_j: Option<f64>,
    /// Explicit inventory of every work/heat flow crossing the boundary.
    pub boundary_flows: Option<Vec<BoundaryEnergyFlow>>,
}

/// Classification of a balance record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnergyBalanceStatus {
    /// One or more required values or uncertainty bounds were not supplied.
    /// No residual is reported.
    Incomplete {
        missing_terms: Vec<EnergyBalanceTerm>,
    },
    /// The entire conservative residual interval lies within the tolerance.
    Balanced,
    /// The entire conservative residual interval lies outside the tolerance.
    Imbalanced,
    /// The residual interval overlaps the tolerance boundary; evidence is
    /// insufficient to classify the balance as either balanced or imbalanced.
    Inconclusive,
}

/// Result of evaluating the first-law balance for one declared boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyBalanceReport {
    pub boundary_id: String,
    pub interval_id: String,
    pub status: EnergyBalanceStatus,
    /// Final stored energy minus initial stored energy, in joules.
    pub delta_stored_energy_j: Option<f64>,
    /// Net work entering the boundary, in joules.
    pub net_work_j: Option<f64>,
    /// Net heat entering the boundary, in joules.
    pub net_heat_j: Option<f64>,
    /// Net boundary energy input: net work + net heat.
    pub net_boundary_energy_j: Option<f64>,
    /// Point-estimate residual: ΔE_stored - net_boundary_energy.
    pub residual_j: Option<f64>,
    /// Conservative worst-case absolute uncertainty bound on the residual,
    /// computed by summing declared absolute bounds without assuming
    /// statistical independence, in joules.
    pub residual_uncertainty_bound_j: Option<f64>,
    /// Conservative possible residual interval [lower, upper], in joules.
    pub residual_interval_j: Option<(f64, f64)>,
    /// Absolute acceptance threshold used for classification, in joules.
    pub allowed_residual_j: Option<f64>,
    /// Scale-normalized absolute residual. Zero when every energy scale is zero.
    pub normalized_residual: Option<f64>,
    /// Number of explicit boundary flows in the evaluated interval.
    pub flow_count: Option<usize>,
    /// Sorted unique external reservoir identifiers in the flow inventory.
    pub reservoir_ids: Option<Vec<String>>,
}

/// Invalid input supplied to the open-system balance evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnergyBalanceError {
    EmptyBoundaryId,
    EmptyIntervalId,
    InvalidEnergyValue {
        field: EnergyBalanceTerm,
    },
    InvalidFlowIdentity {
        field: &'static str,
    },
    DuplicateFlowId {
        flow_id: String,
    },
    InvalidFlowAmount {
        flow_id: String,
    },
    InvalidUncertaintyBound {
        field: &'static str,
        flow_id: Option<String>,
    },
    InvalidTolerance {
        field: &'static str,
    },
    OverflowedBalance {
        field: &'static str,
    },
}

impl OpenSystemEnergyBalance {
    /// Evaluate a complete or partial first-law balance.
    ///
    /// Complete balances use:
    ///
    /// `ΔE_stored = Σ(W_into - W_out) + Σ(Q_into - Q_out)`
    ///
    /// and residual:
    ///
    /// `r = ΔE_stored - (net_work + net_heat)`.
    ///
    /// Stored-energy values may be signed because potential-energy reference
    /// conventions vary. Boundary-flow magnitudes and tolerances must be
    /// finite and non-negative. Missing values are not interpreted as zero.
    pub fn evaluate(
        &self,
        tolerance: EnergyBalanceTolerance,
    ) -> Result<EnergyBalanceReport, EnergyBalanceError> {
        if self.boundary_id.trim().is_empty() {
            return Err(EnergyBalanceError::EmptyBoundaryId);
        }
        if self.interval_id.trim().is_empty() {
            return Err(EnergyBalanceError::EmptyIntervalId);
        }
        validate_tolerance(tolerance)?;

        // Validate all supplied values even when another required term is
        // missing. Incomplete records must not hide malformed evidence.
        for (term, value) in [
            (
                EnergyBalanceTerm::InitialStoredEnergy,
                self.initial_stored_energy_j,
            ),
            (
                EnergyBalanceTerm::FinalStoredEnergy,
                self.final_stored_energy_j,
            ),
        ] {
            if let Some(value) = value {
                if !value.is_finite() {
                    return Err(EnergyBalanceError::InvalidEnergyValue { field: term });
                }
            }
        }

        for (field, value) in [
            (
                "initial_stored_energy_uncertainty_bound_j",
                self.initial_stored_energy_uncertainty_bound_j,
            ),
            (
                "final_stored_energy_uncertainty_bound_j",
                self.final_stored_energy_uncertainty_bound_j,
            ),
        ] {
            if let Some(value) = value {
                if !value.is_finite() || value < 0.0 {
                    return Err(EnergyBalanceError::InvalidUncertaintyBound {
                        field,
                        flow_id: None,
                    });
                }
            }
        }

        if let Some(flows) = &self.boundary_flows {
            let mut flow_ids = HashSet::with_capacity(flows.len());
            for flow in flows {
                if flow.flow_id.trim().is_empty() {
                    return Err(EnergyBalanceError::InvalidFlowIdentity { field: "flow_id" });
                }
                if flow.reservoir_id.trim().is_empty() {
                    return Err(EnergyBalanceError::InvalidFlowIdentity {
                        field: "reservoir_id",
                    });
                }
                if !flow.energy_j.is_finite() || flow.energy_j < 0.0 {
                    return Err(EnergyBalanceError::InvalidFlowAmount {
                        flow_id: flow.flow_id.clone(),
                    });
                }
                if let Some(uncertainty) = flow.uncertainty_bound_j {
                    if !uncertainty.is_finite() || uncertainty < 0.0 {
                        return Err(EnergyBalanceError::InvalidUncertaintyBound {
                            field: "boundary_flow_uncertainty_bound_j",
                            flow_id: Some(flow.flow_id.clone()),
                        });
                    }
                }
                if !flow_ids.insert(flow.flow_id.as_str()) {
                    return Err(EnergyBalanceError::DuplicateFlowId {
                        flow_id: flow.flow_id.clone(),
                    });
                }
            }
        }

        let mut missing_terms = Vec::new();
        if self.initial_stored_energy_j.is_none() {
            missing_terms.push(EnergyBalanceTerm::InitialStoredEnergy);
        }
        if self.final_stored_energy_j.is_none() {
            missing_terms.push(EnergyBalanceTerm::FinalStoredEnergy);
        }
        if self.boundary_flows.is_none() {
            missing_terms.push(EnergyBalanceTerm::BoundaryFlows);
        }
        let flow_uncertainties_complete = self
            .boundary_flows
            .as_ref()
            .is_some_and(|flows| flows.iter().all(|flow| flow.uncertainty_bound_j.is_some()));
        if self.initial_stored_energy_uncertainty_bound_j.is_none()
            || self.final_stored_energy_uncertainty_bound_j.is_none()
            || !flow_uncertainties_complete
        {
            missing_terms.push(EnergyBalanceTerm::UncertaintyBounds);
        }

        if !missing_terms.is_empty() {
            return Ok(incomplete_report(
                &self.boundary_id,
                &self.interval_id,
                missing_terms,
            ));
        }

        let initial = self.initial_stored_energy_j.expect("validated above");
        let final_energy = self.final_stored_energy_j.expect("validated above");
        let flows = self.boundary_flows.as_deref().expect("validated above");

        let initial_uncertainty = self
            .initial_stored_energy_uncertainty_bound_j
            .expect("uncertainty completeness checked above");
        let final_uncertainty = self
            .final_stored_energy_uncertainty_bound_j
            .expect("uncertainty completeness checked above");

        let mut net_work = 0.0;
        let mut net_heat = 0.0;
        let mut gross_boundary_flow = 0.0;
        let mut residual_uncertainty_bound = initial_uncertainty + final_uncertainty;
        if !residual_uncertainty_bound.is_finite() {
            return Err(EnergyBalanceError::OverflowedBalance {
                field: "stored_energy_uncertainty_bounds",
            });
        }
        let mut reservoir_ids = BTreeSet::new();

        for flow in flows {
            let signed_energy = match flow.direction {
                FlowDirection::IntoBoundary => flow.energy_j,
                FlowDirection::OutOfBoundary => -flow.energy_j,
            };

            match flow.carrier {
                EnergyCarrier::Work => net_work += signed_energy,
                EnergyCarrier::Heat => net_heat += signed_energy,
            }
            gross_boundary_flow += flow.energy_j;
            residual_uncertainty_bound += flow
                .uncertainty_bound_j
                .expect("uncertainty completeness checked above");
            reservoir_ids.insert(flow.reservoir_id.clone());

            if !net_work.is_finite()
                || !net_heat.is_finite()
                || !gross_boundary_flow.is_finite()
                || !residual_uncertainty_bound.is_finite()
            {
                return Err(EnergyBalanceError::OverflowedBalance {
                    field: "boundary_flows",
                });
            }
        }

        let delta_stored = final_energy - initial;
        let net_boundary = net_work + net_heat;
        let residual = delta_stored - net_boundary;
        let scale = delta_stored.abs().max(gross_boundary_flow);
        let allowed = tolerance.absolute_j + tolerance.relative * scale;

        if !delta_stored.is_finite() {
            return Err(EnergyBalanceError::OverflowedBalance {
                field: "delta_stored_energy_j",
            });
        }
        if !net_boundary.is_finite() {
            return Err(EnergyBalanceError::OverflowedBalance {
                field: "net_boundary_energy_j",
            });
        }
        if !residual.is_finite() {
            return Err(EnergyBalanceError::OverflowedBalance {
                field: "residual_j",
            });
        }
        if !allowed.is_finite() {
            return Err(EnergyBalanceError::OverflowedBalance {
                field: "allowed_residual_j",
            });
        }

        let residual_lower = residual - residual_uncertainty_bound;
        let residual_upper = residual + residual_uncertainty_bound;
        if !residual_lower.is_finite() || !residual_upper.is_finite() {
            return Err(EnergyBalanceError::OverflowedBalance {
                field: "residual_interval_j",
            });
        }

        let normalized_residual = if scale == 0.0 {
            0.0
        } else {
            residual.abs() / scale
        };
        let status = if residual_lower >= -allowed && residual_upper <= allowed {
            EnergyBalanceStatus::Balanced
        } else if residual_lower > allowed || residual_upper < -allowed {
            EnergyBalanceStatus::Imbalanced
        } else {
            EnergyBalanceStatus::Inconclusive
        };

        Ok(EnergyBalanceReport {
            boundary_id: self.boundary_id.clone(),
            interval_id: self.interval_id.clone(),
            status,
            delta_stored_energy_j: Some(delta_stored),
            net_work_j: Some(net_work),
            net_heat_j: Some(net_heat),
            net_boundary_energy_j: Some(net_boundary),
            residual_j: Some(residual),
            residual_uncertainty_bound_j: Some(residual_uncertainty_bound),
            residual_interval_j: Some((residual_lower, residual_upper)),
            allowed_residual_j: Some(allowed),
            normalized_residual: Some(normalized_residual),
            flow_count: Some(flows.len()),
            reservoir_ids: Some(reservoir_ids.into_iter().collect()),
        })
    }
}

fn incomplete_report(
    boundary_id: &str,
    interval_id: &str,
    missing_terms: Vec<EnergyBalanceTerm>,
) -> EnergyBalanceReport {
    EnergyBalanceReport {
        boundary_id: boundary_id.to_string(),
        interval_id: interval_id.to_string(),
        status: EnergyBalanceStatus::Incomplete { missing_terms },
        delta_stored_energy_j: None,
        net_work_j: None,
        net_heat_j: None,
        net_boundary_energy_j: None,
        residual_j: None,
        residual_uncertainty_bound_j: None,
        residual_interval_j: None,
        allowed_residual_j: None,
        normalized_residual: None,
        flow_count: None,
        reservoir_ids: None,
    }
}

fn validate_tolerance(
    tolerance: EnergyBalanceTolerance,
) -> Result<(), EnergyBalanceError> {
    if !tolerance.absolute_j.is_finite() || tolerance.absolute_j < 0.0 {
        return Err(EnergyBalanceError::InvalidTolerance {
            field: "absolute_j",
        });
    }
    if !tolerance.relative.is_finite() || tolerance.relative < 0.0 {
        return Err(EnergyBalanceError::InvalidTolerance { field: "relative" });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flow(
        flow_id: &str,
        reservoir_id: &str,
        carrier: EnergyCarrier,
        direction: FlowDirection,
        energy_j: f64,
    ) -> BoundaryEnergyFlow {
        BoundaryEnergyFlow {
            flow_id: flow_id.to_string(),
            reservoir_id: reservoir_id.to_string(),
            carrier,
            direction,
            energy_j,
            uncertainty_bound_j: Some(0.0),
        }
    }

    fn complete_balance(
        initial: f64,
        final_energy: f64,
        flows: Vec<BoundaryEnergyFlow>,
    ) -> OpenSystemEnergyBalance {
        OpenSystemEnergyBalance {
            boundary_id: "test-system".to_string(),
            interval_id: "tick-0001".to_string(),
            initial_stored_energy_j: Some(initial),
            initial_stored_energy_uncertainty_bound_j: Some(0.0),
            final_stored_energy_j: Some(final_energy),
            final_stored_energy_uncertainty_bound_j: Some(0.0),
            boundary_flows: Some(flows),
        }
    }

    fn zero_tolerance() -> EnergyBalanceTolerance {
        EnergyBalanceTolerance {
            absolute_j: 0.0,
            relative: 0.0,
        }
    }

    #[test]
    fn closed_system_with_no_flows_is_balanced() {
        let report = complete_balance(12.0, 12.0, vec![])
            .evaluate(zero_tolerance())
            .unwrap();
        assert_eq!(report.status, EnergyBalanceStatus::Balanced);
        assert_eq!(report.residual_j, Some(0.0));
        assert_eq!(report.flow_count, Some(0));
        assert_eq!(report.reservoir_ids, Some(vec![]));
    }

    #[test]
    fn external_work_and_heat_close_the_balance() {
        let report = complete_balance(
            10.0,
            23.0,
            vec![
                flow("w-in", "motor", EnergyCarrier::Work, FlowDirection::IntoBoundary, 8.0),
                flow("w-out", "load", EnergyCarrier::Work, FlowDirection::OutOfBoundary, 1.0),
                flow("q-in", "heater", EnergyCarrier::Heat, FlowDirection::IntoBoundary, 7.0),
                flow("q-out", "ambient", EnergyCarrier::Heat, FlowDirection::OutOfBoundary, 1.0),
            ],
        )
        .evaluate(zero_tolerance())
        .unwrap();
        assert_eq!(report.status, EnergyBalanceStatus::Balanced);
        assert_eq!(report.delta_stored_energy_j, Some(13.0));
        assert_eq!(report.net_work_j, Some(7.0));
        assert_eq!(report.net_heat_j, Some(6.0));
        assert_eq!(report.net_boundary_energy_j, Some(13.0));
        assert_eq!(report.reservoir_ids, Some(vec![
            "ambient".to_string(),
            "heater".to_string(),
            "load".to_string(),
            "motor".to_string(),
        ]));
    }

    #[test]
    fn zero_flows_and_positive_stored_energy_gain_fail_closed() {
        let report = complete_balance(0.0, 10.0, vec![])
            .evaluate(zero_tolerance())
            .unwrap();
        assert_eq!(report.status, EnergyBalanceStatus::Imbalanced);
        assert_eq!(report.residual_j, Some(10.0));
    }

    #[test]
    fn stored_energy_loss_accounts_for_heat_outflow() {
        let report = complete_balance(
            10.0,
            0.0,
            vec![flow(
                "q-out",
                "ambient",
                EnergyCarrier::Heat,
                FlowDirection::OutOfBoundary,
                10.0,
            )],
        )
        .evaluate(zero_tolerance())
        .unwrap();
        assert_eq!(report.status, EnergyBalanceStatus::Balanced);
        assert_eq!(report.residual_j, Some(0.0));
    }

    #[test]
    fn positive_outflow_without_stored_energy_or_input_is_imbalanced() {
        let report = complete_balance(
            0.0,
            0.0,
            vec![flow(
                "q-out",
                "ambient",
                EnergyCarrier::Heat,
                FlowDirection::OutOfBoundary,
                10.0,
            )],
        )
        .evaluate(zero_tolerance())
        .unwrap();
        assert_eq!(report.status, EnergyBalanceStatus::Imbalanced);
        assert_eq!(report.residual_j, Some(10.0));
    }

    #[test]
    fn missing_terms_are_incomplete_not_implicit_zero() {
        let record = OpenSystemEnergyBalance {
            boundary_id: "test-system".to_string(),
            interval_id: "tick-0001".to_string(),
            initial_stored_energy_j: Some(0.0),
            initial_stored_energy_uncertainty_bound_j: Some(0.0),
            final_stored_energy_j: Some(10.0),
            final_stored_energy_uncertainty_bound_j: Some(0.0),
            boundary_flows: None,
        };
        let report = record.evaluate(zero_tolerance()).unwrap();
        assert_eq!(
            report.status,
            EnergyBalanceStatus::Incomplete {
                missing_terms: vec![EnergyBalanceTerm::BoundaryFlows],
            }
        );
        assert_eq!(report.residual_j, None);
    }

    #[test]
    fn malformed_flow_is_not_hidden_by_missing_stored_energy() {
        let record = OpenSystemEnergyBalance {
            boundary_id: "test-system".to_string(),
            interval_id: "tick-0001".to_string(),
            initial_stored_energy_j: Some(0.0),
            initial_stored_energy_uncertainty_bound_j: Some(0.0),
            final_stored_energy_j: None,
            final_stored_energy_uncertainty_bound_j: None,
            boundary_flows: Some(vec![flow(
                "bad-flow",
                "reservoir",
                EnergyCarrier::Heat,
                FlowDirection::IntoBoundary,
                f64::NAN,
            )]),
        };
        assert_eq!(
            record.evaluate(zero_tolerance()),
            Err(EnergyBalanceError::InvalidFlowAmount {
                flow_id: "bad-flow".to_string(),
            })
        );
    }

    #[test]
    fn duplicate_flow_ids_are_rejected() {
        let record = complete_balance(
            0.0,
            2.0,
            vec![
                flow("same", "source-a", EnergyCarrier::Work, FlowDirection::IntoBoundary, 1.0),
                flow("same", "source-b", EnergyCarrier::Work, FlowDirection::IntoBoundary, 1.0),
            ],
        );
        assert_eq!(
            record.evaluate(zero_tolerance()),
            Err(EnergyBalanceError::DuplicateFlowId {
                flow_id: "same".to_string(),
            })
        );
    }

    #[test]
    fn unnamed_flow_or_reservoir_is_rejected() {
        let no_flow_id = complete_balance(
            0.0,
            1.0,
            vec![flow("", "source", EnergyCarrier::Work, FlowDirection::IntoBoundary, 1.0)],
        );
        assert_eq!(
            no_flow_id.evaluate(zero_tolerance()),
            Err(EnergyBalanceError::InvalidFlowIdentity { field: "flow_id" })
        );

        let no_reservoir_id = complete_balance(
            0.0,
            1.0,
            vec![flow("flow", " ", EnergyCarrier::Work, FlowDirection::IntoBoundary, 1.0)],
        );
        assert_eq!(
            no_reservoir_id.evaluate(zero_tolerance()),
            Err(EnergyBalanceError::InvalidFlowIdentity {
                field: "reservoir_id",
            })
        );
    }

    #[test]
    fn negative_and_non_finite_flow_amounts_are_rejected() {
        for energy_j in [-1.0, f64::NAN, f64::INFINITY] {
            let record = complete_balance(
                0.0,
                1.0,
                vec![flow("bad", "source", EnergyCarrier::Work, FlowDirection::IntoBoundary, energy_j)],
            );
            assert!(matches!(
                record.evaluate(zero_tolerance()),
                Err(EnergyBalanceError::InvalidFlowAmount { .. })
            ));
        }
    }

    #[test]
    fn invalid_boundary_and_tolerances_are_rejected() {
        let mut record = complete_balance(0.0, 0.0, vec![]);
        record.boundary_id = "  ".to_string();
        assert_eq!(
            record.evaluate(zero_tolerance()),
            Err(EnergyBalanceError::EmptyBoundaryId)
        );
        record.boundary_id = "test-system".to_string();
        record.interval_id = "  ".to_string();
        assert_eq!(
            record.evaluate(zero_tolerance()),
            Err(EnergyBalanceError::EmptyIntervalId)
        );

        assert_eq!(
            EnergyBalanceTolerance::new(-1.0, 0.0),
            Err(EnergyBalanceError::InvalidTolerance {
                field: "absolute_j",
            })
        );
        assert_eq!(
            EnergyBalanceTolerance::new(0.0, f64::INFINITY),
            Err(EnergyBalanceError::InvalidTolerance { field: "relative" })
        );
    }

    #[test]
    fn relative_tolerance_scales_with_energy_magnitude() {
        let tolerance = EnergyBalanceTolerance::new(0.0, 1e-3).unwrap();
        let small = complete_balance(
            0.0,
            100.0,
            vec![flow("in", "source", EnergyCarrier::Work, FlowDirection::IntoBoundary, 100.05)],
        )
        .evaluate(tolerance)
        .unwrap();
        let large = complete_balance(
            0.0,
            100_000.0,
            vec![flow("in", "source", EnergyCarrier::Work, FlowDirection::IntoBoundary, 100_050.0)],
        )
        .evaluate(tolerance)
        .unwrap();
        assert_eq!(small.status, EnergyBalanceStatus::Balanced);
        assert_eq!(large.status, EnergyBalanceStatus::Balanced);
    }

    #[test]
    fn missing_uncertainty_is_incomplete_not_assumed_zero() {
        let mut record = complete_balance(0.0, 1.0, vec![]);
        record.initial_stored_energy_uncertainty_bound_j = None;
        let report = record.evaluate(zero_tolerance()).unwrap();
        assert_eq!(
            report.status,
            EnergyBalanceStatus::Incomplete {
                missing_terms: vec![EnergyBalanceTerm::UncertaintyBounds],
            }
        );
        assert_eq!(report.residual_uncertainty_bound_j, None);
    }

    #[test]
    fn residual_interval_fully_within_tolerance_is_balanced() {
        let mut record = complete_balance(
            0.0,
            0.05,
            vec![flow(
                "in",
                "source",
                EnergyCarrier::Work,
                FlowDirection::IntoBoundary,
                0.0,
            )],
        );
        record.initial_stored_energy_uncertainty_bound_j = Some(0.01);
        record.final_stored_energy_uncertainty_bound_j = Some(0.01);
        let report = record
            .evaluate(EnergyBalanceTolerance::new(0.1, 0.0).unwrap())
            .unwrap();
        assert_eq!(report.status, EnergyBalanceStatus::Balanced);
        assert_eq!(report.residual_interval_j, Some((0.03, 0.07)));
    }

    #[test]
    fn residual_interval_outside_tolerance_is_imbalanced() {
        let mut record = complete_balance(
            0.0,
            0.5,
            vec![flow(
                "in",
                "source",
                EnergyCarrier::Work,
                FlowDirection::IntoBoundary,
                0.0,
            )],
        );
        record.initial_stored_energy_uncertainty_bound_j = Some(0.02);
        record.final_stored_energy_uncertainty_bound_j = Some(0.03);
        let report = record
            .evaluate(EnergyBalanceTolerance::new(0.1, 0.0).unwrap())
            .unwrap();
        assert_eq!(report.status, EnergyBalanceStatus::Imbalanced);
        assert_eq!(report.residual_interval_j, Some((0.45, 0.55)));
    }

    #[test]
    fn uncertainty_overlapping_tolerance_is_inconclusive() {
        let mut record = complete_balance(
            0.0,
            0.2,
            vec![flow(
                "in",
                "source",
                EnergyCarrier::Work,
                FlowDirection::IntoBoundary,
                0.0,
            )],
        );
        record.initial_stored_energy_uncertainty_bound_j = Some(0.1);
        record.final_stored_energy_uncertainty_bound_j = Some(0.1);
        let report = record
            .evaluate(EnergyBalanceTolerance::new(0.1, 0.0).unwrap())
            .unwrap();
        assert_eq!(report.status, EnergyBalanceStatus::Inconclusive);
        assert_eq!(report.residual_interval_j, Some((0.0, 0.4)));
    }

    #[test]
    fn invalid_uncertainty_bounds_are_rejected() {
        let mut record = complete_balance(0.0, 0.0, vec![]);
        record.initial_stored_energy_uncertainty_bound_j = Some(-0.1);
        assert_eq!(
            record.evaluate(zero_tolerance()),
            Err(EnergyBalanceError::InvalidUncertaintyBound {
                field: "initial_stored_energy_uncertainty_bound_j",
                flow_id: None,
            })
        );

        let mut record = complete_balance(
            0.0,
            0.0,
            vec![flow(
                "bad",
                "source",
                EnergyCarrier::Work,
                FlowDirection::IntoBoundary,
                0.0,
            )],
        );
        record.boundary_flows.as_mut().unwrap()[0].uncertainty_bound_j = Some(f64::NAN);
        assert_eq!(
            record.evaluate(zero_tolerance()),
            Err(EnergyBalanceError::InvalidUncertaintyBound {
                field: "boundary_flow_uncertainty_bound_j",
                flow_id: Some("bad".to_string()),
            })
        );
    }

    #[test]
    fn uncertainty_sum_overflow_is_rejected() {
        let mut record = complete_balance(0.0, 0.0, vec![]);
        record.initial_stored_energy_uncertainty_bound_j = Some(f64::MAX);
        record.final_stored_energy_uncertainty_bound_j = Some(f64::MAX);
        assert_eq!(
            record.evaluate(zero_tolerance()),
            Err(EnergyBalanceError::OverflowedBalance {
                field: "stored_energy_uncertainty_bounds",
            })
        );
    }

    #[test]
    fn flow_sum_overflow_is_rejected() {
        let record = complete_balance(
            0.0,
            0.0,
            vec![
                flow("a", "source-a", EnergyCarrier::Heat, FlowDirection::IntoBoundary, f64::MAX),
                flow("b", "source-b", EnergyCarrier::Heat, FlowDirection::IntoBoundary, f64::MAX),
            ],
        );
        assert_eq!(
            record.evaluate(zero_tolerance()),
            Err(EnergyBalanceError::OverflowedBalance {
                field: "boundary_flows",
            })
        );
    }

    #[test]
    fn stored_energy_difference_overflow_is_rejected() {
        let record = complete_balance(-f64::MAX, f64::MAX, vec![]);
        assert_eq!(
            record.evaluate(zero_tolerance()),
            Err(EnergyBalanceError::OverflowedBalance {
                field: "delta_stored_energy_j",
            })
        );
    }
}
