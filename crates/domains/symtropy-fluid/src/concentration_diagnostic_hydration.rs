// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Measurement-only hydration of the solver-independent continuum diagnostic
//! sample with the qualified periodic-MAC vorticity concentration estimator.
//!
//! This adapter does not assign a validity state, resolution threshold,
//! refinement action, semantic profile transition, or execution backend. It
//! preserves the reference solver's existing diagnostics and attaches the full
//! estimator report so estimator-specific unavailable reasons are never erased.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::reference::{PeriodicMac2d, ReferenceDiagnosticError};
use crate::validation::{
    ContinuumDiagnosticSample, ContinuumSampleError, DiagnosticUnavailableReason,
    DiagnosticValueError, NonNegativeDiagnostic,
};
use crate::vorticity_concentration_scale::{
    VORTICITY_CONCENTRATION_SCALE_OPERATOR_ID, VORTICITY_CONCENTRATION_SCALE_SCHEMA_ID,
    VorticityConcentrationScaleError, VorticityConcentrationScaleReport,
    VorticityConcentrationScaleValue, measure_vorticity_concentration_scale,
};

pub const CONCENTRATION_DIAGNOSTIC_HYDRATION_SCHEMA_ID: &str =
    "periodic-mac-vorticity-concentration-diagnostic-hydration-v0.1";

/// One enriched continuum diagnostic sample plus the exact estimator evidence
/// used to hydrate `concentration_scale_m`.
///
/// When the estimator reports an unavailable physical state such as zero
/// vorticity, the generic diagnostic sample uses `NotApplicable` because its V0
/// vocabulary cannot encode the estimator-specific reason. The attached report
/// retains that exact typed reason.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConcentrationDiagnosticHydration {
    pub schema_id: String,
    pub diagnostic_sample: ContinuumDiagnosticSample,
    pub concentration_report: VorticityConcentrationScaleReport,
}

impl ConcentrationDiagnosticHydration {
    /// Validate the serialized relationship between the generic sample and the
    /// estimator evidence without requiring the original solver state.
    pub fn validate(&self) -> Result<(), ConcentrationDiagnosticHydrationError> {
        if self.schema_id != CONCENTRATION_DIAGNOSTIC_HYDRATION_SCHEMA_ID {
            return Err(ConcentrationDiagnosticHydrationError::UnsupportedSchemaId);
        }
        self.diagnostic_sample.validate()?;

        if self.concentration_report.schema_id != VORTICITY_CONCENTRATION_SCALE_SCHEMA_ID
            || self.concentration_report.operator_id != VORTICITY_CONCENTRATION_SCALE_OPERATOR_ID
        {
            return Err(ConcentrationDiagnosticHydrationError::EstimatorIdentityMismatch);
        }

        let expected_diagnostic_profile = format!(
            "{};concentration_scale_operator={VORTICITY_CONCENTRATION_SCALE_OPERATOR_ID}",
            self.concentration_report.solver_profile
        );
        if self.concentration_report.diagnostic_profile != expected_diagnostic_profile
            || self.diagnostic_sample.diagnostic_profile
                != self.concentration_report.diagnostic_profile
        {
            return Err(ConcentrationDiagnosticHydrationError::DiagnosticProfileMismatch);
        }

        if self.diagnostic_sample.time_s.to_bits()
            != self.concentration_report.time_s.to_bits()
        {
            return Err(ConcentrationDiagnosticHydrationError::TimeMismatch);
        }

        match (
            &self.concentration_report.scale,
            &self.diagnostic_sample.concentration_scale_m,
        ) {
            (
                VorticityConcentrationScaleValue::Measured { length_m, .. },
                NonNegativeDiagnostic::Measured(sample_length_m),
            ) if length_m.to_bits() == sample_length_m.to_bits() => Ok(()),
            (
                VorticityConcentrationScaleValue::Unavailable(_),
                NonNegativeDiagnostic::Unavailable(DiagnosticUnavailableReason::NotApplicable),
            ) => Ok(()),
            _ => Err(ConcentrationDiagnosticHydrationError::ConcentrationMismatch),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConcentrationDiagnosticHydrationError {
    Reference(ReferenceDiagnosticError),
    Concentration(VorticityConcentrationScaleError),
    DiagnosticValue(DiagnosticValueError),
    Sample(ContinuumSampleError),
    UnsupportedSchemaId,
    EstimatorIdentityMismatch,
    DiagnosticProfileMismatch,
    TimeMismatch,
    ConcentrationMismatch,
}

impl fmt::Display for ConcentrationDiagnosticHydrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reference(source) => write!(f, "reference diagnostic failed: {source}"),
            Self::Concentration(source) => {
                write!(f, "vorticity concentration estimator failed: {source}")
            }
            Self::DiagnosticValue(source) => {
                write!(f, "hydrated concentration diagnostic is invalid: {source}")
            }
            Self::Sample(source) => write!(f, "hydrated diagnostic sample is invalid: {source}"),
            Self::UnsupportedSchemaId => write!(f, "unsupported concentration hydration schema id"),
            Self::EstimatorIdentityMismatch => write!(
                f,
                "concentration report schema/operator identity does not match the hydration contract"
            ),
            Self::DiagnosticProfileMismatch => write!(
                f,
                "solver and concentration estimator diagnostic profile identities do not compose exactly"
            ),
            Self::TimeMismatch => write!(
                f,
                "continuum diagnostic sample and concentration estimator were not sampled at the same physical time"
            ),
            Self::ConcentrationMismatch => write!(
                f,
                "generic concentration_scale_m does not match the attached estimator evidence"
            ),
        }
    }
}

impl std::error::Error for ConcentrationDiagnosticHydrationError {}

impl From<ReferenceDiagnosticError> for ConcentrationDiagnosticHydrationError {
    fn from(value: ReferenceDiagnosticError) -> Self {
        Self::Reference(value)
    }
}

impl From<VorticityConcentrationScaleError> for ConcentrationDiagnosticHydrationError {
    fn from(value: VorticityConcentrationScaleError) -> Self {
        Self::Concentration(value)
    }
}

impl From<DiagnosticValueError> for ConcentrationDiagnosticHydrationError {
    fn from(value: DiagnosticValueError) -> Self {
        Self::DiagnosticValue(value)
    }
}

impl From<ContinuumSampleError> for ConcentrationDiagnosticHydrationError {
    fn from(value: ContinuumSampleError) -> Self {
        Self::Sample(value)
    }
}

/// Enrich the reference solver's generic diagnostic sample with the exact
/// measurement-only vorticity concentration estimator.
///
/// The function intentionally performs no validity classification and contains
/// no resolution/refinement threshold.
pub fn hydrate_vorticity_concentration_diagnostic(
    state: &PeriodicMac2d,
) -> Result<ConcentrationDiagnosticHydration, ConcentrationDiagnosticHydrationError> {
    let mut diagnostic_sample = state.diagnostics()?;
    let concentration_report = measure_vorticity_concentration_scale(state)?;

    if diagnostic_sample.time_s.to_bits() != concentration_report.time_s.to_bits() {
        return Err(ConcentrationDiagnosticHydrationError::TimeMismatch);
    }
    if diagnostic_sample.diagnostic_profile != concentration_report.solver_profile {
        return Err(ConcentrationDiagnosticHydrationError::DiagnosticProfileMismatch);
    }

    diagnostic_sample.diagnostic_profile = concentration_report.diagnostic_profile.clone();
    diagnostic_sample.concentration_scale_m = match &concentration_report.scale {
        VorticityConcentrationScaleValue::Measured { length_m, .. } => {
            NonNegativeDiagnostic::measured(*length_m)?
        }
        VorticityConcentrationScaleValue::Unavailable(_) => NonNegativeDiagnostic::unavailable(
            DiagnosticUnavailableReason::NotApplicable,
        ),
    };

    let hydration = ConcentrationDiagnosticHydration {
        schema_id: CONCENTRATION_DIAGNOSTIC_HYDRATION_SCHEMA_ID.to_owned(),
        diagnostic_sample,
        concentration_report,
    };
    hydration.validate()?;
    Ok(hydration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::PeriodicMacConfig;
    use crate::vorticity_concentration_scale::VorticityConcentrationScaleUnavailableReason;

    #[test]
    fn measured_hydration_changes_only_profile_and_concentration_field() {
        let state = PeriodicMac2d::taylor_green(PeriodicMacConfig::default(), 1.0).unwrap();
        let baseline = state.diagnostics().unwrap();
        let hydrated = hydrate_vorticity_concentration_diagnostic(&state).unwrap();

        let length_m = match hydrated.concentration_report.scale {
            VorticityConcentrationScaleValue::Measured { length_m, .. } => length_m,
            other => panic!("expected measured concentration scale, got {other:?}"),
        };

        let mut expected = baseline;
        expected.diagnostic_profile = hydrated.concentration_report.diagnostic_profile.clone();
        expected.concentration_scale_m = NonNegativeDiagnostic::measured(length_m).unwrap();

        assert_eq!(hydrated.diagnostic_sample, expected);
        hydrated.validate().unwrap();
    }

    #[test]
    fn zero_vorticity_keeps_exact_reason_in_attached_report() {
        let state = PeriodicMac2d::zeros(PeriodicMacConfig::default()).unwrap();
        let hydrated = hydrate_vorticity_concentration_diagnostic(&state).unwrap();

        assert_eq!(
            hydrated.concentration_report.scale,
            VorticityConcentrationScaleValue::Unavailable(
                VorticityConcentrationScaleUnavailableReason::ZeroVorticity
            )
        );
        assert_eq!(
            hydrated.diagnostic_sample.concentration_scale_m,
            NonNegativeDiagnostic::Unavailable(DiagnosticUnavailableReason::NotApplicable)
        );
        hydrated.validate().unwrap();
    }

    #[test]
    fn hydration_is_deterministic_for_identical_state() {
        let state = PeriodicMac2d::taylor_green(PeriodicMacConfig::default(), 0.75).unwrap();
        let first = hydrate_vorticity_concentration_diagnostic(&state).unwrap();
        let second = hydrate_vorticity_concentration_diagnostic(&state).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn validator_rejects_profile_time_and_value_drift() {
        let state = PeriodicMac2d::taylor_green(PeriodicMacConfig::default(), 1.0).unwrap();
        let hydrated = hydrate_vorticity_concentration_diagnostic(&state).unwrap();

        let mut wrong_profile = hydrated.clone();
        wrong_profile.diagnostic_sample.diagnostic_profile.push_str(";drift");
        assert_eq!(
            wrong_profile.validate().unwrap_err(),
            ConcentrationDiagnosticHydrationError::DiagnosticProfileMismatch
        );

        let mut wrong_time = hydrated.clone();
        wrong_time.diagnostic_sample.time_s += 1.0;
        assert_eq!(
            wrong_time.validate().unwrap_err(),
            ConcentrationDiagnosticHydrationError::TimeMismatch
        );

        let mut wrong_value = hydrated;
        wrong_value.diagnostic_sample.concentration_scale_m =
            NonNegativeDiagnostic::measured(123.0).unwrap();
        assert_eq!(
            wrong_value.validate().unwrap_err(),
            ConcentrationDiagnosticHydrationError::ConcentrationMismatch
        );
    }
}
