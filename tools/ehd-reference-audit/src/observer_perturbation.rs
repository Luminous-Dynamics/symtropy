//! EHD diagnostic observer-perturbation qualification reference.
//!
//! This module treats a diagnostic intervention as potentially changing the
//! physical EHD state, not merely adding measurement noise. It compares a
//! diagnostic measurement against an independent matched baseline using
//! conservative absolute uncertainty bounds. It is not a universal sensor
//! model or a physical correction factor.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectClassification {
    WithinTolerance,
    PerturbationDetected,
    Unresolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObserverDisposition {
    WithinTolerance,
    ObserverPerturbationDetected,
    ObserverPerturbationUnresolved,
    IndependentBaselineRequired,
    OutOfEnvelope,
    PhysicalLedgerFailure,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObserverMeasurement {
    pub value: f64,
    /// Conservative absolute uncertainty bound in the declared metric unit.
    pub absolute_uncertainty_bound: f64,
    pub evidence_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectInterval {
    /// Diagnostic minus baseline; signed to retain direction.
    pub signed_difference: f64,
    /// Baseline and diagnostic absolute bounds are added, not RSS-combined.
    pub combined_absolute_uncertainty_bound: f64,
    /// Lower and upper bounds on absolute physical change.
    pub lower_absolute_effect: f64,
    pub upper_absolute_effect: f64,
    pub maximum_allowed_absolute_effect: f64,
    pub classification: EffectClassification,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObserverPerturbationInput {
    pub comparison_id: String,
    pub operating_condition_id: String,
    pub diagnostic_id: String,
    pub metric_unit: String,
    pub baseline_method: String,
    pub diagnostic_method: String,
    pub baseline: ObserverMeasurement,
    pub during_diagnostic: ObserverMeasurement,
    /// Optional measurement after the diagnostic intervention has been
    /// removed. Required to qualify absence of persistent fouling/hysteresis.
    pub after_removal: Option<ObserverMeasurement>,
    pub maximum_allowed_active_effect: f64,
    pub maximum_allowed_persistent_effect: f64,
    /// Set only when the baseline is independently qualified and not derived
    /// from the same diagnostic channel being evaluated.
    pub independent_baseline_qualified: bool,
    /// Supplied by a separate validation-envelope audit.
    pub within_validation_envelope: bool,
    /// Supplied by independent charge/mass/momentum/energy ledger gates.
    pub conservation_ledgers_pass: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObserverPerturbationReport {
    pub comparison_id: String,
    pub operating_condition_id: String,
    pub diagnostic_id: String,
    pub metric_unit: String,
    pub active_effect: EffectInterval,
    pub persistent_effect: Option<EffectInterval>,
    pub persistence_assessed: bool,
    pub within_validation_envelope: bool,
    pub conservation_ledgers_pass: bool,
    pub disposition: ObserverDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ObserverPerturbationError {
    EmptyIdentity { field: &'static str },
    DuplicateEvidenceId,
    NonFiniteMeasurement { slot: &'static str },
    InvalidUncertaintyBound { slot: &'static str },
    InvalidEffectTolerance { field: &'static str },
    NonFiniteDerivedValue,
}

impl fmt::Display for ObserverPerturbationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentity { field } => {
                write!(f, "observer perturbation {field} must be non-empty")
            }
            Self::DuplicateEvidenceId => {
                write!(f, "baseline, diagnostic and post-removal evidence IDs must be distinct")
            }
            Self::NonFiniteMeasurement { slot } => {
                write!(f, "observer perturbation {slot} measurement is non-finite")
            }
            Self::InvalidUncertaintyBound { slot } => {
                write!(f, "observer perturbation {slot} uncertainty bound must be finite and non-negative")
            }
            Self::InvalidEffectTolerance { field } => {
                write!(f, "observer perturbation {field} must be finite and non-negative")
            }
            Self::NonFiniteDerivedValue => {
                write!(f, "observer perturbation calculation produced a non-finite value")
            }
        }
    }
}

impl Error for ObserverPerturbationError {}

fn validate_measurement(
    measurement: &ObserverMeasurement,
    slot: &'static str,
) -> Result<(), ObserverPerturbationError> {
    if !measurement.value.is_finite() {
        return Err(ObserverPerturbationError::NonFiniteMeasurement { slot });
    }
    if !measurement.absolute_uncertainty_bound.is_finite()
        || measurement.absolute_uncertainty_bound < 0.0
    {
        return Err(ObserverPerturbationError::InvalidUncertaintyBound { slot });
    }
    if measurement.evidence_id.trim().is_empty() {
        return Err(ObserverPerturbationError::EmptyIdentity { field: "evidence_id" });
    }
    Ok(())
}

fn compare_effect(
    baseline: &ObserverMeasurement,
    comparison: &ObserverMeasurement,
    maximum_allowed_absolute_effect: f64,
) -> Result<EffectInterval, ObserverPerturbationError> {
    if !maximum_allowed_absolute_effect.is_finite()
        || maximum_allowed_absolute_effect < 0.0
    {
        return Err(ObserverPerturbationError::InvalidEffectTolerance {
            field: "maximum_allowed_absolute_effect",
        });
    }

    let difference = comparison.value - baseline.value;
    let combined_bound = baseline.absolute_uncertainty_bound
        + comparison.absolute_uncertainty_bound;
    let absolute_difference = difference.abs();
    let lower = (absolute_difference - combined_bound).max(0.0);
    let upper = absolute_difference + combined_bound;
    if !difference.is_finite()
        || !combined_bound.is_finite()
        || !lower.is_finite()
        || !upper.is_finite()
    {
        return Err(ObserverPerturbationError::NonFiniteDerivedValue);
    }

    let classification = if lower > maximum_allowed_absolute_effect {
        EffectClassification::PerturbationDetected
    } else if upper <= maximum_allowed_absolute_effect {
        EffectClassification::WithinTolerance
    } else {
        EffectClassification::Unresolved
    };

    Ok(EffectInterval {
        signed_difference: difference,
        combined_absolute_uncertainty_bound: combined_bound,
        lower_absolute_effect: lower,
        upper_absolute_effect: upper,
        maximum_allowed_absolute_effect,
        classification,
    })
}

/// Evaluate a diagnostic intervention against an independently qualified
/// baseline. Uncertainty is conservatively added, with no independence or
/// covariance assumption. A missing post-removal observation leaves
/// persistence unresolved rather than assuming the system recovered.
pub fn qualify_observer_perturbation(
    input: &ObserverPerturbationInput,
) -> Result<ObserverPerturbationReport, ObserverPerturbationError> {
    for (field, value) in [
        ("comparison_id", input.comparison_id.as_str()),
        ("operating_condition_id", input.operating_condition_id.as_str()),
        ("diagnostic_id", input.diagnostic_id.as_str()),
        ("metric_unit", input.metric_unit.as_str()),
        ("baseline_method", input.baseline_method.as_str()),
        ("diagnostic_method", input.diagnostic_method.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ObserverPerturbationError::EmptyIdentity { field });
        }
    }
    if !input.maximum_allowed_active_effect.is_finite()
        || input.maximum_allowed_active_effect < 0.0
    {
        return Err(ObserverPerturbationError::InvalidEffectTolerance {
            field: "maximum_allowed_active_effect",
        });
    }
    if !input.maximum_allowed_persistent_effect.is_finite()
        || input.maximum_allowed_persistent_effect < 0.0
    {
        return Err(ObserverPerturbationError::InvalidEffectTolerance {
            field: "maximum_allowed_persistent_effect",
        });
    }

    validate_measurement(&input.baseline, "baseline")?;
    validate_measurement(&input.during_diagnostic, "during_diagnostic")?;
    if let Some(after) = &input.after_removal {
        validate_measurement(after, "after_removal")?;
        if after.evidence_id == input.baseline.evidence_id
            || after.evidence_id == input.during_diagnostic.evidence_id
        {
            return Err(ObserverPerturbationError::DuplicateEvidenceId);
        }
    }
    if input.baseline.evidence_id == input.during_diagnostic.evidence_id {
        return Err(ObserverPerturbationError::DuplicateEvidenceId);
    }

    let active_effect = compare_effect(
        &input.baseline,
        &input.during_diagnostic,
        input.maximum_allowed_active_effect,
    )?;
    let persistent_effect = input.after_removal
        .as_ref()
        .map(|after| {
            compare_effect(
                &input.baseline,
                after,
                input.maximum_allowed_persistent_effect,
            )
        })
        .transpose()?;

    // Independent physical gates take precedence over a good-looking
    // diagnostic comparison. This code cannot repair or override either gate.
    let disposition = if !input.conservation_ledgers_pass {
        ObserverDisposition::PhysicalLedgerFailure
    } else if !input.within_validation_envelope {
        ObserverDisposition::OutOfEnvelope
    } else if !input.independent_baseline_qualified {
        ObserverDisposition::IndependentBaselineRequired
    } else if active_effect.classification == EffectClassification::PerturbationDetected
        || persistent_effect.is_some_and(|effect| {
            effect.classification == EffectClassification::PerturbationDetected
        })
    {
        ObserverDisposition::ObserverPerturbationDetected
    } else if active_effect.classification == EffectClassification::Unresolved
        || persistent_effect.is_none()
        || persistent_effect.is_some_and(|effect| {
            effect.classification == EffectClassification::Unresolved
        })
    {
        ObserverDisposition::ObserverPerturbationUnresolved
    } else {
        ObserverDisposition::WithinTolerance
    };

    Ok(ObserverPerturbationReport {
        comparison_id: input.comparison_id.clone(),
        operating_condition_id: input.operating_condition_id.clone(),
        diagnostic_id: input.diagnostic_id.clone(),
        metric_unit: input.metric_unit.clone(),
        active_effect,
        persistence_assessed: persistent_effect.is_some(),
        persistent_effect,
        within_validation_envelope: input.within_validation_envelope,
        conservation_ledgers_pass: input.conservation_ledgers_pass,
        disposition,
    })
}

fn measurement(value: f64, uncertainty: f64, evidence_id: &str) -> ObserverMeasurement {
    ObserverMeasurement {
        value,
        absolute_uncertainty_bound: uncertainty,
        evidence_id: evidence_id.to_owned(),
    }
}

fn reference_input() -> ObserverPerturbationInput {
    ObserverPerturbationInput {
        comparison_id: "DPM-MANUFACTURED-001".to_owned(),
        operating_condition_id: "needle-ring-14kv-rh36-matched".to_owned(),
        diagnostic_id: "dehs-piv-seeding".to_owned(),
        metric_unit: "m/s".to_owned(),
        baseline_method: "independent-unseeded-reference".to_owned(),
        diagnostic_method: "seeded-piv".to_owned(),
        baseline: measurement(5.9, 0.05, "baseline-evidence"),
        during_diagnostic: measurement(3.8, 0.05, "diagnostic-evidence"),
        after_removal: Some(measurement(4.0, 0.05, "post-removal-evidence")),
        maximum_allowed_active_effect: 0.2,
        maximum_allowed_persistent_effect: 0.2,
        independent_baseline_qualified: true,
        within_validation_envelope: true,
        conservation_ledgers_pass: true,
    }
}

/// Manufactured references demonstrate detection, tolerance, uncertainty
/// overlap, persistent effects and independent-gate precedence. The 5.9→3.8
/// values mirror a published paper only in magnitude; uncertainty and
/// tolerances here are synthetic and are not assigned to that experiment.
pub fn reference_self_check() -> Result<(), ObserverPerturbationError> {
    let report = qualify_observer_perturbation(&reference_input())?;
    if (report.active_effect.signed_difference + 2.1).abs() > 1.0e-12
        || report.active_effect.classification != EffectClassification::PerturbationDetected
        || report.disposition != ObserverDisposition::ObserverPerturbationDetected
        || !report.persistence_assessed
        || report.persistent_effect.is_none_or(|effect| {
            effect.classification != EffectClassification::PerturbationDetected
        })
    {
        return Err(ObserverPerturbationError::NonFiniteDerivedValue);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_dehs_like_effect_is_not_treated_as_passive_measurement_noise() {
        let report = qualify_observer_perturbation(&reference_input()).unwrap();
        assert!((report.active_effect.lower_absolute_effect - 2.0).abs() < 1.0e-12);
        assert!((report.active_effect.upper_absolute_effect - 2.2).abs() < 1.0e-12);
        assert_eq!(
            report.active_effect.classification,
            EffectClassification::PerturbationDetected
        );
        assert_eq!(report.disposition, ObserverDisposition::ObserverPerturbationDetected);
    }

    #[test]
    fn effect_wholly_within_predeclared_tolerance_can_be_qualified() {
        let mut input = reference_input();
        input.during_diagnostic = measurement(5.88, 0.05, "diagnostic-evidence");
        input.after_removal = Some(measurement(5.91, 0.05, "post-removal-evidence"));
        let report = qualify_observer_perturbation(&input).unwrap();
        assert_eq!(report.active_effect.classification, EffectClassification::WithinTolerance);
        assert_eq!(
            report.persistent_effect.unwrap().classification,
            EffectClassification::WithinTolerance
        );
        assert_eq!(report.disposition, ObserverDisposition::WithinTolerance);
    }

    #[test]
    fn interval_overlapping_tolerance_is_unresolved_not_passed() {
        let mut input = reference_input();
        input.during_diagnostic = measurement(5.65, 0.1, "diagnostic-evidence");
        input.after_removal = Some(measurement(5.9, 0.05, "post-removal-evidence"));
        let report = qualify_observer_perturbation(&input).unwrap();
        assert_eq!(report.active_effect.classification, EffectClassification::Unresolved);
        assert_eq!(
            report.disposition,
            ObserverDisposition::ObserverPerturbationUnresolved
        );
    }

    #[test]
    fn missing_post_removal_observation_never_implies_recovery() {
        let mut input = reference_input();
        input.during_diagnostic = measurement(5.9, 0.05, "diagnostic-evidence");
        input.after_removal = None;
        let report = qualify_observer_perturbation(&input).unwrap();
        assert!(!report.persistence_assessed);
        assert_eq!(
            report.disposition,
            ObserverDisposition::ObserverPerturbationUnresolved
        );
    }

    #[test]
    fn residual_post_removal_effect_is_reported_as_persistent_perturbation() {
        let mut input = reference_input();
        input.during_diagnostic = measurement(3.8, 0.05, "diagnostic-evidence");
        input.after_removal = Some(measurement(4.7, 0.05, "post-removal-evidence"));
        let report = qualify_observer_perturbation(&input).unwrap();
        assert_eq!(
            report.persistent_effect.unwrap().classification,
            EffectClassification::PerturbationDetected
        );
        assert_eq!(report.disposition, ObserverDisposition::ObserverPerturbationDetected);
    }

    #[test]
    fn non_independent_baseline_requires_abstention() {
        let mut input = reference_input();
        input.independent_baseline_qualified = false;
        let report = qualify_observer_perturbation(&input).unwrap();
        assert_eq!(
            report.disposition,
            ObserverDisposition::IndependentBaselineRequired
        );
    }

    #[test]
    fn independent_physical_ledger_failure_has_priority() {
        let mut input = reference_input();
        input.conservation_ledgers_pass = false;
        let report = qualify_observer_perturbation(&input).unwrap();
        assert_eq!(report.disposition, ObserverDisposition::PhysicalLedgerFailure);
    }

    #[test]
    fn out_of_envelope_is_not_rescued_by_a_small_observer_effect() {
        let mut input = reference_input();
        input.during_diagnostic = measurement(5.9, 0.05, "diagnostic-evidence");
        input.after_removal = Some(measurement(5.9, 0.05, "post-removal-evidence"));
        input.within_validation_envelope = false;
        let report = qualify_observer_perturbation(&input).unwrap();
        assert_eq!(report.disposition, ObserverDisposition::OutOfEnvelope);
    }

    #[test]
    fn duplicate_evidence_identity_is_rejected() {
        let mut input = reference_input();
        input.during_diagnostic.evidence_id = input.baseline.evidence_id.clone();
        assert_eq!(
            qualify_observer_perturbation(&input).unwrap_err(),
            ObserverPerturbationError::DuplicateEvidenceId
        );
    }
}
