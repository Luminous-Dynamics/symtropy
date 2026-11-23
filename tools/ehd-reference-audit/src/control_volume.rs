// SPDX-License-Identifier: AGPL-3.0-or-later

//! Auditable control-volume momentum reducer for EHD thrust qualification.
//!
//! Frozen sign convention:
//!
//!     S + A = P + V + B + F_solid_to_fluid
//!
//! Therefore:
//!
//!     F_solid_to_fluid = S + A - P - V - B
//!     F_fluid_to_solid = -F_solid_to_fluid.
//!
//! The reducer requires every term and a separate direct force-balance
//! measurement. Componentwise uncertainty bounds are combined by worst-case
//! addition; no statistical independence or diagonal covariance assumption is
//! made. This is a bookkeeping/reference reducer, not a PIV/LDV processor or
//! a fluid solver.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

/// A vector estimate and componentwise absolute uncertainty bound, all in N.
#[derive(Clone, Debug, PartialEq)]
pub struct VectorEvidence {
    pub value_n: [f64; 3],
    pub uncertainty_bound_n: [f64; 3],
    /// Identity of the raw data plus processing/calibration evidence.
    pub evidence_id: String,
}

/// Complete set of terms for one frozen control volume and experimental run.
#[derive(Clone, Debug, PartialEq)]
pub struct ControlVolumeInput {
    pub run_id: String,
    pub control_volume_id: String,
    /// Names outward normals, traction sign and instrument-to-fluid axes.
    pub sign_convention_id: String,
    /// d/dt integral_CV rho*u dV.
    pub momentum_storage: Option<VectorEvidence>,
    /// Net outward integral_CS rho*u*(u dot n) dA over every surface.
    pub advective_momentum_flux: Option<VectorEvidence>,
    /// Integral_CS (-p I) dot n dA over every surface.
    pub pressure_traction: Option<VectorEvidence>,
    /// Integral_CS tau dot n dA over every surface.
    pub viscous_traction: Option<VectorEvidence>,
    /// Integral_CV f_body dV, with all included body forces declared.
    pub body_force: Option<VectorEvidence>,
    /// Independently measured force exerted by fluid on the solid.
    pub balance_fluid_on_solid: Option<VectorEvidence>,
}

/// Reconciled control-volume and direct force-balance receipt.
#[derive(Clone, Debug, PartialEq)]
pub struct ControlVolumeReceipt {
    pub run_id: String,
    pub control_volume_id: String,
    pub sign_convention_id: String,
    pub evidence_ids_in_term_order: Vec<String>,
    pub solid_on_fluid_from_cv_n: [f64; 3],
    pub fluid_on_solid_from_cv_n: [f64; 3],
    pub measured_fluid_on_solid_n: [f64; 3],
    /// CV-derived fluid-on-solid force minus direct balance vector.
    pub residual_n: [f64; 3],
    /// Worst-case sum of the five CV term uncertainty bounds.
    pub cv_uncertainty_bound_n: [f64; 3],
    /// CV worst-case bound plus direct balance uncertainty bound.
    pub combined_uncertainty_bound_n: [f64; 3],
    pub within_combined_bound: [bool; 3],
    pub overall_within_combined_bound: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ControlVolumeError {
    EmptyRunId,
    EmptyControlVolumeId,
    EmptySignConventionId,
    MissingTerm(&'static str),
    EmptyEvidenceId {
        term: &'static str,
    },
    DuplicateEvidenceId(String),
    NonFiniteValue {
        term: &'static str,
        component: usize,
    },
    InvalidUncertainty {
        term: &'static str,
        component: usize,
    },
    NonFiniteDerivedValue {
        component: usize,
    },
}

impl fmt::Display for ControlVolumeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRunId => write!(f, "control-volume run ID is empty"),
            Self::EmptyControlVolumeId => write!(f, "control-volume geometry identity is empty"),
            Self::EmptySignConventionId => {
                write!(f, "control-volume sign-convention identity is empty")
            }
            Self::MissingTerm(term) => write!(f, "required control-volume term is missing: {term}"),
            Self::EmptyEvidenceId { term } => {
                write!(f, "control-volume term {term} has no evidence identity")
            }
            Self::DuplicateEvidenceId(id) => {
                write!(f, "control-volume evidence identity is duplicated: {id}")
            }
            Self::NonFiniteValue { term, component } => write!(
                f,
                "control-volume term {term} component {component} is non-finite"
            ),
            Self::InvalidUncertainty { term, component } => write!(
                f,
                "control-volume term {term} component {component} has invalid uncertainty"
            ),
            Self::NonFiniteDerivedValue { component } => write!(
                f,
                "control-volume derived force or uncertainty is non-finite in component {component}"
            ),
        }
    }
}

impl Error for ControlVolumeError {}

fn validate_term(
    term: &'static str,
    evidence: &VectorEvidence,
    seen_ids: &mut BTreeSet<String>,
) -> Result<(), ControlVolumeError> {
    if evidence.evidence_id.trim().is_empty() {
        return Err(ControlVolumeError::EmptyEvidenceId { term });
    }
    if !seen_ids.insert(evidence.evidence_id.clone()) {
        return Err(ControlVolumeError::DuplicateEvidenceId(
            evidence.evidence_id.clone(),
        ));
    }
    for component in 0..3 {
        if !evidence.value_n[component].is_finite() {
            return Err(ControlVolumeError::NonFiniteValue { term, component });
        }
        let uncertainty = evidence.uncertainty_bound_n[component];
        if !uncertainty.is_finite() || uncertainty < 0.0 {
            return Err(ControlVolumeError::InvalidUncertainty { term, component });
        }
    }
    Ok(())
}

/// Reconstruct the fluid-on-solid force from the complete control-volume
/// momentum equation and compare it with an independent direct balance vector.
///
/// All six evidence records are mandatory. Missing pressure, viscous, side
/// flux, storage or body-force terms are never assumed to be zero. Uncertainty
/// bounds are combined by the conservative triangle inequality:
///
///     U_CV = U_storage + U_advective + U_pressure + U_viscous + U_body
///
/// This remains valid without assuming independence. A later covariance-aware
/// implementation may tighten the bound only when a validated covariance
/// artifact is available.
pub fn reconcile_control_volume(
    input: &ControlVolumeInput,
) -> Result<ControlVolumeReceipt, ControlVolumeError> {
    if input.run_id.trim().is_empty() {
        return Err(ControlVolumeError::EmptyRunId);
    }
    if input.control_volume_id.trim().is_empty() {
        return Err(ControlVolumeError::EmptyControlVolumeId);
    }
    if input.sign_convention_id.trim().is_empty() {
        return Err(ControlVolumeError::EmptySignConventionId);
    }

    let storage = input
        .momentum_storage
        .as_ref()
        .ok_or(ControlVolumeError::MissingTerm("momentum_storage"))?;
    let advective = input
        .advective_momentum_flux
        .as_ref()
        .ok_or(ControlVolumeError::MissingTerm("advective_momentum_flux"))?;
    let pressure = input
        .pressure_traction
        .as_ref()
        .ok_or(ControlVolumeError::MissingTerm("pressure_traction"))?;
    let viscous = input
        .viscous_traction
        .as_ref()
        .ok_or(ControlVolumeError::MissingTerm("viscous_traction"))?;
    let body = input
        .body_force
        .as_ref()
        .ok_or(ControlVolumeError::MissingTerm("body_force"))?;
    let balance = input
        .balance_fluid_on_solid
        .as_ref()
        .ok_or(ControlVolumeError::MissingTerm("balance_fluid_on_solid"))?;

    let term_records = [
        ("momentum_storage", storage),
        ("advective_momentum_flux", advective),
        ("pressure_traction", pressure),
        ("viscous_traction", viscous),
        ("body_force", body),
        ("balance_fluid_on_solid", balance),
    ];
    let mut seen_ids = BTreeSet::new();
    let mut evidence_ids_in_term_order = Vec::with_capacity(term_records.len());
    for (name, evidence) in term_records {
        validate_term(name, evidence, &mut seen_ids)?;
        evidence_ids_in_term_order.push(evidence.evidence_id.clone());
    }

    let mut solid_on_fluid_from_cv_n = [0.0_f64; 3];
    let mut fluid_on_solid_from_cv_n = [0.0_f64; 3];
    let mut residual_n = [0.0_f64; 3];
    let mut cv_uncertainty_bound_n = [0.0_f64; 3];
    let mut combined_uncertainty_bound_n = [0.0_f64; 3];
    let mut within_combined_bound = [false; 3];

    for component in 0..3 {
        // F_solid->fluid = storage + advective - pressure - viscous - body.
        let solid_force = storage.value_n[component] + advective.value_n[component]
            - pressure.value_n[component]
            - viscous.value_n[component]
            - body.value_n[component];
        let fluid_force = -solid_force;
        let cv_bound = storage.uncertainty_bound_n[component]
            + advective.uncertainty_bound_n[component]
            + pressure.uncertainty_bound_n[component]
            + viscous.uncertainty_bound_n[component]
            + body.uncertainty_bound_n[component];
        let combined_bound = cv_bound + balance.uncertainty_bound_n[component];
        let residual = fluid_force - balance.value_n[component];

        if !solid_force.is_finite()
            || !fluid_force.is_finite()
            || !cv_bound.is_finite()
            || !combined_bound.is_finite()
            || !residual.is_finite()
        {
            return Err(ControlVolumeError::NonFiniteDerivedValue { component });
        }

        solid_on_fluid_from_cv_n[component] = solid_force;
        fluid_on_solid_from_cv_n[component] = fluid_force;
        residual_n[component] = residual;
        cv_uncertainty_bound_n[component] = cv_bound;
        combined_uncertainty_bound_n[component] = combined_bound;
        within_combined_bound[component] = residual.abs() <= combined_bound;
    }

    Ok(ControlVolumeReceipt {
        run_id: input.run_id.clone(),
        control_volume_id: input.control_volume_id.clone(),
        sign_convention_id: input.sign_convention_id.clone(),
        evidence_ids_in_term_order,
        solid_on_fluid_from_cv_n,
        fluid_on_solid_from_cv_n,
        measured_fluid_on_solid_n: balance.value_n,
        residual_n,
        cv_uncertainty_bound_n,
        combined_uncertainty_bound_n,
        within_combined_bound,
        overall_within_combined_bound: within_combined_bound.iter().all(|passed| *passed),
    })
}

/// Small synthetic reference case. The fluid-on-solid CV estimate is
/// [-2,-1,-1] N; the independent balance is within the conservative bounds.
pub fn reference_self_check() -> Result<(), ControlVolumeError> {
    let make = |id: &str, value: [f64; 3], uncertainty: [f64; 3]| VectorEvidence {
        value_n: value,
        uncertainty_bound_n: uncertainty,
        evidence_id: id.to_owned(),
    };
    let input = ControlVolumeInput {
        run_id: "ehd-cv-reference-run".to_owned(),
        control_volume_id: "cv-reference-box".to_owned(),
        sign_convention_id: "outward-normal-solid-to-fluid-v1".to_owned(),
        momentum_storage: Some(make("storage", [1.0, 2.0, 3.0], [0.01; 3])),
        advective_momentum_flux: Some(make("advective", [2.0, 0.0, -1.0], [0.01; 3])),
        pressure_traction: Some(make("pressure", [0.5; 3], [0.01; 3])),
        viscous_traction: Some(make("viscous", [0.1; 3], [0.01; 3])),
        body_force: Some(make("body", [0.4; 3], [0.01; 3])),
        balance_fluid_on_solid: Some(make("balance", [-2.04, -1.03, -0.96], [0.02; 3])),
    };
    let receipt = reconcile_control_volume(&input)?;
    if receipt.fluid_on_solid_from_cv_n != [-2.0, -1.0, -1.0]
        || receipt
            .cv_uncertainty_bound_n
            .iter()
            .any(|value| (*value - 0.05).abs() > 1.0e-12)
        || receipt
            .combined_uncertainty_bound_n
            .iter()
            .any(|value| (*value - 0.07).abs() > 1.0e-12)
        || !receipt.overall_within_combined_bound
    {
        return Err(ControlVolumeError::NonFiniteDerivedValue { component: 0 });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(id: &str, value: [f64; 3], uncertainty: [f64; 3]) -> VectorEvidence {
        VectorEvidence {
            value_n: value,
            uncertainty_bound_n: uncertainty,
            evidence_id: id.to_owned(),
        }
    }

    fn input() -> ControlVolumeInput {
        ControlVolumeInput {
            run_id: "run-1".into(),
            control_volume_id: "cv-1".into(),
            sign_convention_id: "outward-normal-v1".into(),
            momentum_storage: Some(evidence("storage", [1.0, 2.0, 3.0], [0.01; 3])),
            advective_momentum_flux: Some(evidence("advective", [2.0, 0.0, -1.0], [0.01; 3])),
            pressure_traction: Some(evidence("pressure", [0.5; 3], [0.01; 3])),
            viscous_traction: Some(evidence("viscous", [0.1; 3], [0.01; 3])),
            body_force: Some(evidence("body", [0.4; 3], [0.01; 3])),
            balance_fluid_on_solid: Some(evidence("balance", [-2.04, -1.03, -0.96], [0.02; 3])),
        }
    }

    #[test]
    fn reconstructs_fluid_on_solid_force_with_frozen_sign_convention() {
        let receipt = reconcile_control_volume(&input()).unwrap();
        assert_eq!(receipt.solid_on_fluid_from_cv_n, [2.0, 1.0, 1.0]);
        assert_eq!(receipt.fluid_on_solid_from_cv_n, [-2.0, -1.0, -1.0]);
        assert_eq!(receipt.within_combined_bound, [true; 3]);
    }

    #[test]
    fn missing_any_required_term_is_rejected_not_zero_filled() {
        let mut value = input();
        value.momentum_storage = None;
        assert_eq!(
            reconcile_control_volume(&value).unwrap_err(),
            ControlVolumeError::MissingTerm("momentum_storage")
        );
        let mut value = input();
        value.advective_momentum_flux = None;
        assert_eq!(
            reconcile_control_volume(&value).unwrap_err(),
            ControlVolumeError::MissingTerm("advective_momentum_flux")
        );
        let mut value = input();
        value.pressure_traction = None;
        assert_eq!(
            reconcile_control_volume(&value).unwrap_err(),
            ControlVolumeError::MissingTerm("pressure_traction")
        );
        let mut value = input();
        value.viscous_traction = None;
        assert_eq!(
            reconcile_control_volume(&value).unwrap_err(),
            ControlVolumeError::MissingTerm("viscous_traction")
        );
        let mut value = input();
        value.body_force = None;
        assert_eq!(
            reconcile_control_volume(&value).unwrap_err(),
            ControlVolumeError::MissingTerm("body_force")
        );
        let mut value = input();
        value.balance_fluid_on_solid = None;
        assert_eq!(
            reconcile_control_volume(&value).unwrap_err(),
            ControlVolumeError::MissingTerm("balance_fluid_on_solid")
        );
    }

    #[test]
    fn discrepancy_beyond_combined_uncertainty_is_flagged_not_hidden() {
        let mut value = input();
        value.balance_fluid_on_solid.as_mut().unwrap().value_n[0] = -2.20;
        let receipt = reconcile_control_volume(&value).unwrap();
        assert!(!receipt.within_combined_bound[0]);
        assert!(!receipt.overall_within_combined_bound);
    }

    #[test]
    fn equality_at_conservative_uncertainty_bound_is_accepted() {
        let mut value = input();
        value.balance_fluid_on_solid.as_mut().unwrap().value_n[0] = -2.07;
        let receipt = reconcile_control_volume(&value).unwrap();
        assert!(receipt.within_combined_bound[0]);
    }

    #[test]
    fn uncertainty_uses_worst_case_sum_not_root_sum_square() {
        let receipt = reconcile_control_volume(&input()).unwrap();
        assert_eq!(receipt.cv_uncertainty_bound_n, [0.05; 3]);
        assert_eq!(receipt.combined_uncertainty_bound_n, [0.07; 3]);
        // RSS of the five 0.01 N CV term bounds would be only sqrt(5)*0.01.
        assert!(receipt.cv_uncertainty_bound_n[0] > (5.0_f64).sqrt() * 0.01);
    }

    #[test]
    fn rejects_duplicate_evidence_identities() {
        let mut value = input();
        value.pressure_traction.as_mut().unwrap().evidence_id = "storage".into();
        assert_eq!(
            reconcile_control_volume(&value).unwrap_err(),
            ControlVolumeError::DuplicateEvidenceId("storage".into())
        );
    }

    #[test]
    fn rejects_non_finite_values_and_negative_uncertainty() {
        let mut value = input();
        value.advective_momentum_flux.as_mut().unwrap().value_n[1] = f64::NAN;
        assert_eq!(
            reconcile_control_volume(&value).unwrap_err(),
            ControlVolumeError::NonFiniteValue {
                term: "advective_momentum_flux",
                component: 1,
            }
        );
        let mut value = input();
        value.body_force.as_mut().unwrap().uncertainty_bound_n[2] = -0.01;
        assert_eq!(
            reconcile_control_volume(&value).unwrap_err(),
            ControlVolumeError::InvalidUncertainty {
                term: "body_force",
                component: 2,
            }
        );
    }

    #[test]
    fn rejects_empty_run_and_control_volume_identities() {
        let mut value = input();
        value.run_id.clear();
        assert_eq!(
            reconcile_control_volume(&value).unwrap_err(),
            ControlVolumeError::EmptyRunId
        );
        let mut value = input();
        value.control_volume_id.clear();
        assert_eq!(
            reconcile_control_volume(&value).unwrap_err(),
            ControlVolumeError::EmptyControlVolumeId
        );
    }

    #[test]
    fn requires_nonempty_sign_and_evidence_identities() {
        let mut value = input();
        value.sign_convention_id.clear();
        assert_eq!(
            reconcile_control_volume(&value).unwrap_err(),
            ControlVolumeError::EmptySignConventionId
        );
        let mut value = input();
        value
            .balance_fluid_on_solid
            .as_mut()
            .unwrap()
            .evidence_id
            .clear();
        assert_eq!(
            reconcile_control_volume(&value).unwrap_err(),
            ControlVolumeError::EmptyEvidenceId {
                term: "balance_fluid_on_solid"
            }
        );
    }

    #[test]
    fn reference_self_check_passes() {
        reference_self_check().unwrap();
    }
}
