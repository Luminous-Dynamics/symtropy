// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Revalidation for serialized or post-construction energy reconciliation evidence.
//!
//! [`EnergyStateSnapshot`] revalidates itself before reconciliation, but the
//! resulting [`EnergyReconciliationAudit`] is also public and serializable. A
//! downstream consumer must not assume that a deserialized or manually mutated
//! audit still satisfies the constructor's structural and finite-arithmetic
//! invariants.

use crate::energy::{EnergyOwner, EnergyPort};
use crate::energy_state::{
    EnergyReconciliationAudit, ReservoirPresenceChangeKind, ReservoirReconciliation,
};

/// Failures that make a reconciliation artifact unsuitable as validation evidence.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EnergyReconciliationEvidenceError {
    NonFiniteTolerance,
    NonFiniteSummary,
    InconsistentSummaryTotals,
    DuplicateReservoir(EnergyPort),
    NonFiniteReservoirEvidence(EnergyPort),
    InconsistentReservoirArithmetic(EnergyPort),
    DuplicatePresenceChange(EnergyPort),
    PresenceChangeWithoutReservoir(EnergyPort),
    PresenceShapeMismatch(EnergyPort),
    StableReservoirMissingNumericEvidence(EnergyPort),
    DuplicateUntrackedPort(EnergyPort),
    ExternalPortMarkedUntracked(EnergyPort),
    TrackedPortMarkedUntracked(EnergyPort),
    InconsistentBoundaryArithmetic,
}

/// Checked operations for reconciliation evidence that may have crossed a
/// serialization, FFI, network, or other mutation boundary.
pub trait EnergyReconciliationEvidenceExt {
    /// Revalidate the full audit structure and all represented arithmetic.
    fn validate_evidence(&self) -> Result<(), EnergyReconciliationEvidenceError>;

    /// Checked maximum absolute per-reservoir residual.
    fn max_abs_residual_joules_checked(
        &self,
    ) -> Result<f64, EnergyReconciliationEvidenceError>;

    /// Checked count of reservoirs not explained within an absolute tolerance.
    fn unexplained_reservoir_count_checked(
        &self,
        tolerance_joules: f64,
    ) -> Result<usize, EnergyReconciliationEvidenceError>;

    /// Strict checked equivalent of `fully_reconciled` for evidence consumers.
    fn fully_reconciled_checked(
        &self,
        tolerance_joules: f64,
    ) -> Result<bool, EnergyReconciliationEvidenceError>;
}

impl EnergyReconciliationEvidenceExt for EnergyReconciliationAudit {
    fn validate_evidence(&self) -> Result<(), EnergyReconciliationEvidenceError> {
        if !self.initial_total_joules.is_finite()
            || !self.final_total_joules.is_finite()
            || !self.net_external_joules.is_finite()
            || !self.total_closure_error_joules.is_finite()
        {
            return Err(EnergyReconciliationEvidenceError::NonFiniteSummary);
        }

        for (index, entry) in self.entries.iter().enumerate() {
            if self.entries[..index]
                .iter()
                .any(|previous| previous.port == entry.port)
            {
                return Err(EnergyReconciliationEvidenceError::DuplicateReservoir(
                    entry.port,
                ));
            }
            validate_entry_finite(entry)?;
        }

        validate_endpoint_summaries(self)?;

        for (index, change) in self.reservoir_presence_changes.iter().enumerate() {
            if self.reservoir_presence_changes[..index]
                .iter()
                .any(|previous| previous.port == change.port)
            {
                return Err(
                    EnergyReconciliationEvidenceError::DuplicatePresenceChange(change.port),
                );
            }

            let Some(entry) = self.entries.iter().find(|entry| entry.port == change.port) else {
                return Err(
                    EnergyReconciliationEvidenceError::PresenceChangeWithoutReservoir(change.port),
                );
            };

            let shape_matches = match change.kind {
                ReservoirPresenceChangeKind::Appeared => {
                    entry.initial_joules.is_none()
                        && entry.final_joules.is_some()
                        && entry.measured_delta_joules.is_none()
                        && entry.residual_joules.is_none()
                }
                ReservoirPresenceChangeKind::Disappeared => {
                    entry.initial_joules.is_some()
                        && entry.final_joules.is_none()
                        && entry.measured_delta_joules.is_none()
                        && entry.residual_joules.is_none()
                }
            };
            if !shape_matches {
                return Err(EnergyReconciliationEvidenceError::PresenceShapeMismatch(
                    change.port,
                ));
            }
        }

        for entry in &self.entries {
            let has_presence_change = self
                .reservoir_presence_changes
                .iter()
                .any(|change| change.port == entry.port);
            if has_presence_change {
                continue;
            }

            let (Some(initial), Some(final_energy), Some(measured), Some(residual)) = (
                entry.initial_joules,
                entry.final_joules,
                entry.measured_delta_joules,
                entry.residual_joules,
            ) else {
                return Err(
                    EnergyReconciliationEvidenceError::StableReservoirMissingNumericEvidence(
                        entry.port,
                    ),
                );
            };

            let recomputed_measured = final_energy - initial;
            let recomputed_residual = recomputed_measured - entry.ledger_delta_joules;
            if !recomputed_measured.is_finite()
                || !recomputed_residual.is_finite()
                || measured != recomputed_measured
                || residual != recomputed_residual
            {
                return Err(
                    EnergyReconciliationEvidenceError::InconsistentReservoirArithmetic(entry.port),
                );
            }
        }

        for (index, port) in self.untracked_ledger_ports.iter().enumerate() {
            if self.untracked_ledger_ports[..index].contains(port) {
                return Err(EnergyReconciliationEvidenceError::DuplicateUntrackedPort(
                    *port,
                ));
            }
            if matches!(port.owner, EnergyOwner::External(_)) {
                return Err(EnergyReconciliationEvidenceError::ExternalPortMarkedUntracked(
                    *port,
                ));
            }
            if self.entries.iter().any(|entry| entry.port == *port) {
                return Err(EnergyReconciliationEvidenceError::TrackedPortMarkedUntracked(
                    *port,
                ));
            }
        }

        let observed_delta = self.final_total_joules - self.initial_total_joules;
        let recomputed_closure = observed_delta - self.net_external_joules;
        if !observed_delta.is_finite()
            || !recomputed_closure.is_finite()
            || recomputed_closure != self.total_closure_error_joules
        {
            return Err(EnergyReconciliationEvidenceError::InconsistentBoundaryArithmetic);
        }

        Ok(())
    }

    fn max_abs_residual_joules_checked(
        &self,
    ) -> Result<f64, EnergyReconciliationEvidenceError> {
        self.validate_evidence()?;
        let mut max_residual = 0.0_f64;
        for residual in self.entries.iter().filter_map(|entry| entry.residual_joules) {
            max_residual = max_residual.max(residual.abs());
        }
        Ok(max_residual)
    }

    fn unexplained_reservoir_count_checked(
        &self,
        tolerance_joules: f64,
    ) -> Result<usize, EnergyReconciliationEvidenceError> {
        validate_tolerance(tolerance_joules)?;
        self.validate_evidence()?;
        Ok(self
            .entries
            .iter()
            .filter(|entry| {
                entry
                    .residual_joules
                    .is_none_or(|residual| residual.abs() > tolerance_joules)
            })
            .count())
    }

    fn fully_reconciled_checked(
        &self,
        tolerance_joules: f64,
    ) -> Result<bool, EnergyReconciliationEvidenceError> {
        validate_tolerance(tolerance_joules)?;
        self.validate_evidence()?;
        Ok(self.reservoir_presence_changes.is_empty()
            && self.untracked_ledger_ports.is_empty()
            && self
                .entries
                .iter()
                .all(|entry| entry.residual_joules.is_some_and(|r| r.abs() <= tolerance_joules))
            && self.total_closure_error_joules.abs() <= tolerance_joules)
    }
}

/// Recompute endpoint totals from the represented reservoirs and prove that the
/// stored summary totals are consistent with those entries.
///
/// A small forward-error bound is necessary because the original snapshots and
/// the unioned reconciliation entries can sum the same finite reservoirs in a
/// different deterministic order when reservoirs appear or disappear. The bound
/// scales only with machine epsilon, entry count, and total absolute magnitude;
/// it is not a user-controlled physics tolerance.
fn validate_endpoint_summaries(
    audit: &EnergyReconciliationAudit,
) -> Result<(), EnergyReconciliationEvidenceError> {
    let mut initial_sum = 0.0_f64;
    let mut final_sum = 0.0_f64;
    let mut initial_abs_sum = 0.0_f64;
    let mut final_abs_sum = 0.0_f64;
    let mut initial_count = 0_usize;
    let mut final_count = 0_usize;

    for entry in &audit.entries {
        if let Some(value) = entry.initial_joules {
            checked_summary_add(&mut initial_sum, value)?;
            checked_summary_add(&mut initial_abs_sum, value.abs())?;
            initial_count = initial_count.saturating_add(1);
        }
        if let Some(value) = entry.final_joules {
            checked_summary_add(&mut final_sum, value)?;
            checked_summary_add(&mut final_abs_sum, value.abs())?;
            final_count = final_count.saturating_add(1);
        }
    }

    if !summary_total_matches(
        audit.initial_total_joules,
        initial_sum,
        initial_abs_sum,
        initial_count,
    )? || !summary_total_matches(
        audit.final_total_joules,
        final_sum,
        final_abs_sum,
        final_count,
    )? {
        return Err(EnergyReconciliationEvidenceError::InconsistentSummaryTotals);
    }

    Ok(())
}

fn checked_summary_add(
    accumulator: &mut f64,
    value: f64,
) -> Result<(), EnergyReconciliationEvidenceError> {
    let next = *accumulator + value;
    if !next.is_finite() {
        return Err(EnergyReconciliationEvidenceError::NonFiniteSummary);
    }
    *accumulator = next;
    Ok(())
}

fn summary_total_matches(
    stored: f64,
    recomputed: f64,
    absolute_sum: f64,
    count: usize,
) -> Result<bool, EnergyReconciliationEvidenceError> {
    let scale = absolute_sum
        .max(stored.abs())
        .max(recomputed.abs())
        .max(1.0);
    let count_factor = 8.0 * count.max(1) as f64;
    let tolerance = f64::EPSILON * count_factor * scale;
    if !scale.is_finite() || !count_factor.is_finite() || !tolerance.is_finite() {
        return Err(EnergyReconciliationEvidenceError::NonFiniteSummary);
    }

    let difference = (stored - recomputed).abs();
    Ok(difference.is_finite() && difference <= tolerance)
}

fn validate_entry_finite(
    entry: &ReservoirReconciliation,
) -> Result<(), EnergyReconciliationEvidenceError> {
    let options_are_finite = [
        entry.initial_joules,
        entry.final_joules,
        entry.measured_delta_joules,
        entry.residual_joules,
    ]
    .into_iter()
    .flatten()
    .all(f64::is_finite);

    if !entry.ledger_delta_joules.is_finite() || !options_are_finite {
        Err(EnergyReconciliationEvidenceError::NonFiniteReservoirEvidence(
            entry.port,
        ))
    } else {
        Ok(())
    }
}

fn validate_tolerance(tolerance_joules: f64) -> Result<(), EnergyReconciliationEvidenceError> {
    if tolerance_joules.is_finite() && tolerance_joules >= 0.0 {
        Ok(())
    } else {
        Err(EnergyReconciliationEvidenceError::NonFiniteTolerance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;
    use crate::energy::EnergyForm;
    use crate::energy_state::{EnergyReconciliationAudit, ReservoirPresenceChange};

    fn thermal_port() -> EnergyPort {
        EnergyPort::new(
            EnergyOwner::Body(BodyHandle(0)),
            EnergyForm::ThermalSensible,
        )
    }

    fn valid_audit() -> EnergyReconciliationAudit {
        EnergyReconciliationAudit {
            entries: vec![ReservoirReconciliation {
                port: thermal_port(),
                initial_joules: Some(100.0),
                final_joules: Some(90.0),
                measured_delta_joules: Some(-10.0),
                ledger_delta_joules: -10.0,
                residual_joules: Some(0.0),
            }],
            reservoir_presence_changes: Vec::new(),
            untracked_ledger_ports: Vec::new(),
            initial_total_joules: 100.0,
            final_total_joules: 90.0,
            net_external_joules: -10.0,
            total_closure_error_joules: 0.0,
        }
    }

    #[test]
    fn valid_serialized_shape_revalidates() {
        let audit = valid_audit();
        assert_eq!(audit.validate_evidence(), Ok(()));
        assert_eq!(audit.max_abs_residual_joules_checked().unwrap(), 0.0);
        assert_eq!(audit.unexplained_reservoir_count_checked(0.0).unwrap(), 0);
        assert!(audit.fully_reconciled_checked(0.0).unwrap());
    }

    #[test]
    fn nan_residual_cannot_disappear_from_checked_reporting() {
        let mut audit = valid_audit();
        audit.entries[0].residual_joules = Some(f64::NAN);
        assert_eq!(
            audit.validate_evidence(),
            Err(EnergyReconciliationEvidenceError::NonFiniteReservoirEvidence(
                thermal_port()
            ))
        );
        assert!(audit.max_abs_residual_joules_checked().is_err());
        assert!(audit.unexplained_reservoir_count_checked(1.0).is_err());
        assert!(audit.fully_reconciled_checked(1.0).is_err());
    }

    #[test]
    fn inconsistent_numeric_residual_is_rejected() {
        let mut audit = valid_audit();
        audit.entries[0].residual_joules = Some(1.0);
        assert_eq!(
            audit.validate_evidence(),
            Err(EnergyReconciliationEvidenceError::InconsistentReservoirArithmetic(
                thermal_port()
            ))
        );
    }

    #[test]
    fn appeared_reservoir_requires_presence_shape() {
        let port = thermal_port();
        let audit = EnergyReconciliationAudit {
            entries: vec![ReservoirReconciliation {
                port,
                initial_joules: None,
                final_joules: Some(0.0),
                measured_delta_joules: None,
                ledger_delta_joules: 0.0,
                residual_joules: None,
            }],
            reservoir_presence_changes: vec![ReservoirPresenceChange {
                port,
                kind: ReservoirPresenceChangeKind::Appeared,
            }],
            untracked_ledger_ports: Vec::new(),
            initial_total_joules: 0.0,
            final_total_joules: 0.0,
            net_external_joules: 0.0,
            total_closure_error_joules: 0.0,
        };
        assert_eq!(audit.validate_evidence(), Ok(()));
        assert!(!audit.fully_reconciled_checked(0.0).unwrap());
    }

    #[test]
    fn presence_change_cannot_hide_numeric_delta() {
        let port = thermal_port();
        let mut audit = valid_audit();
        audit.reservoir_presence_changes = vec![ReservoirPresenceChange {
            port,
            kind: ReservoirPresenceChangeKind::Appeared,
        }];
        assert_eq!(
            audit.validate_evidence(),
            Err(EnergyReconciliationEvidenceError::PresenceShapeMismatch(port))
        );
    }

    #[test]
    fn boundary_summary_must_recompute_exactly() {
        let mut audit = valid_audit();
        audit.total_closure_error_joules = 1.0;
        assert_eq!(
            audit.validate_evidence(),
            Err(EnergyReconciliationEvidenceError::InconsistentBoundaryArithmetic)
        );
    }

    #[test]
    fn endpoint_summary_totals_are_bound_to_reservoir_entries() {
        let mut initial_forgery = valid_audit();
        initial_forgery.initial_total_joules = 110.0;
        initial_forgery.net_external_joules = -20.0;
        initial_forgery.total_closure_error_joules = 0.0;
        assert_eq!(
            initial_forgery.validate_evidence(),
            Err(EnergyReconciliationEvidenceError::InconsistentSummaryTotals)
        );

        let mut final_forgery = valid_audit();
        final_forgery.final_total_joules = 80.0;
        final_forgery.net_external_joules = -20.0;
        final_forgery.total_closure_error_joules = 0.0;
        assert_eq!(
            final_forgery.validate_evidence(),
            Err(EnergyReconciliationEvidenceError::InconsistentSummaryTotals)
        );
    }

    #[test]
    fn summary_revalidation_tolerates_only_roundoff_scale_reordering() {
        let kinetic = EnergyPort::new(EnergyOwner::Body(BodyHandle(1)), EnergyForm::Kinetic);
        let audit = EnergyReconciliationAudit {
            entries: vec![
                ReservoirReconciliation {
                    port: thermal_port(),
                    initial_joules: Some(1.0e16),
                    final_joules: Some(1.0e16),
                    measured_delta_joules: Some(0.0),
                    ledger_delta_joules: 0.0,
                    residual_joules: Some(0.0),
                },
                ReservoirReconciliation {
                    port: kinetic,
                    initial_joules: Some(1.0),
                    final_joules: Some(1.0),
                    measured_delta_joules: Some(0.0),
                    ledger_delta_joules: 0.0,
                    residual_joules: Some(0.0),
                },
            ],
            reservoir_presence_changes: Vec::new(),
            untracked_ledger_ports: Vec::new(),
            // At this scale adding 1 J can round away. The summary integrity
            // check permits only the machine-error-sized ambiguity.
            initial_total_joules: 1.0e16,
            final_total_joules: 1.0e16,
            net_external_joules: 0.0,
            total_closure_error_joules: 0.0,
        };
        assert_eq!(audit.validate_evidence(), Ok(()));
    }

    #[test]
    fn duplicate_or_external_untracked_ports_are_rejected() {
        let mut duplicate = valid_audit();
        let chemical = EnergyPort::new(EnergyOwner::Body(BodyHandle(1)), EnergyForm::Chemical);
        duplicate.untracked_ledger_ports = vec![chemical, chemical];
        assert_eq!(
            duplicate.validate_evidence(),
            Err(EnergyReconciliationEvidenceError::DuplicateUntrackedPort(
                chemical
            ))
        );

        let mut external = valid_audit();
        let external_port =
            EnergyPort::new(EnergyOwner::External(2), EnergyForm::Electrical);
        external.untracked_ledger_ports = vec![external_port];
        assert_eq!(
            external.validate_evidence(),
            Err(EnergyReconciliationEvidenceError::ExternalPortMarkedUntracked(
                external_port
            ))
        );
    }

    #[test]
    fn invalid_tolerance_is_an_error_not_a_boolean_pass_or_count() {
        let audit = valid_audit();
        assert_eq!(
            audit.fully_reconciled_checked(f64::NAN),
            Err(EnergyReconciliationEvidenceError::NonFiniteTolerance)
        );
        assert_eq!(
            audit.unexplained_reservoir_count_checked(-1.0),
            Err(EnergyReconciliationEvidenceError::NonFiniteTolerance)
        );
    }
}
