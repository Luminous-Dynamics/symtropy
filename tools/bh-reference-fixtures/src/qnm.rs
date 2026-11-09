//! Typed Kerr QNM sequence fixtures.
//!
//! A solver's overtone index is metadata, not proof of family identity.
//! Prolate/oblate labels therefore carry explicit multi-mode asymptotic
//! evidence, while ambiguous or unresolved sequences remain representable.

use serde::{Deserialize, Serialize};

use crate::{
    AngularEigenvalueConvention, ComplexValue, FixtureProvenance, FourierConvention,
    FrequencyNormalization,
};

pub const QNM_SEQUENCE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QnmSolverMethod {
    Leaver,
    IncidenceAmplitude,
    SpheroidalEigenvalue,
    SpectralTwoDimensional,
    KerrNormalProlateGenusOneWkb,
    PotentialPeakPadeWkb,
    PotentialPeakBorelPadeWkb,
    ExactExtremalWkb,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QnmFamilyClassification {
    Prolate,
    Oblate,
    Unresolved,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmModeResiduals {
    pub angular_ode_relative: Option<f64>,
    pub radial_ode_relative: Option<f64>,
    pub continued_fraction_relative: Option<f64>,
    pub incidence_amplitude_relative: Option<f64>,
}

impl QnmModeResiduals {
    fn validate(&self) -> bool {
        let residuals = [
            self.angular_ode_relative,
            self.radial_ode_relative,
            self.continued_fraction_relative,
            self.incidence_amplitude_relative,
        ];
        residuals
            .into_iter()
            .flatten()
            .all(|value| value.is_finite() && value >= 0.0)
            && residuals.into_iter().any(|value| value.is_some())
    }
}

/// Multi-mode asymptotic summary used to justify a prolate or oblate label.
///
/// angular_scaling_ratio is compared with the explicitly recorded
/// expected_angular_scaling_limit. For the normal prolate branch in Hatsuda
/// (2026), that limit is eta = 2 Lbar + 1, not universally 1; Lbar is the
/// equatorial oscillator level. An oblate-family convention may use a
/// different limit and must state it explicitly. Frequency spacings are
/// expressed as dimensionless M delta-omega.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmFamilyEvidence {
    pub angular_scaling_ratio: ComplexValue,
    pub expected_angular_scaling_limit: ComplexValue,
    pub observed_frequency_spacing_m: ComplexValue,
    pub predicted_frequency_spacing_m: ComplexValue,
    pub angular_scaling_relative_residual: f64,
    pub frequency_spacing_relative_residual: f64,
    pub consecutive_modes_used: u32,
}

/// One computed mode. sequence_position preserves the order in the
/// continuation/analysis sequence. overtone_index is optional because a
/// unique traditional n label may be unavailable near crossings or in
/// nonstandard high-overtone sectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QnmMethodApplicability {
    WithinDocumentedRegime,
    Extrapolated,
    KnownBreakdown,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmModeRecord {
    pub sequence_position: u32,
    pub overtone_index: Option<u32>,
    pub omega_m: ComplexValue,
    pub angular_eigenvalue: ComplexValue,
    pub angular_branch_id: String,
    /// Producer-defined ID for the path used to continue the angular branch.
    pub angular_continuation_path_id: Option<String>,
    pub solver_method: QnmSolverMethod,
    pub method_applicability: QnmMethodApplicability,
    pub initial_guess: Option<ComplexValue>,
    pub precision_bits: u16,
    pub residuals: Option<QnmModeResiduals>,
    /// Hatsuda's normal-prolate WKB integer N. It is not generally the
    /// conventional overtone index n.
    pub wkb_quantization_index: Option<u32>,
    pub wkb_truncation_order: Option<u32>,
    pub wkb_optimal_truncation_order: Option<u32>,
    /// Higher quantum periods in the selected genus-one cycle basis.
    pub wkb_cycle_period_j0: Option<ComplexValue>,
    pub wkb_cycle_period_j1: Option<ComplexValue>,
    pub wkb_quantization_residual: Option<f64>,
    pub wkb_truncation_error_estimate: Option<f64>,
    /// Explicit cycle label; the spectral curve alone does not select it.
    pub wkb_cycle_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KerrQnmSequenceFixture {
    pub schema_version: u32,
    pub sequence_id: String,
    pub provenance: FixtureProvenance,
    pub spin_over_mass: f64,
    pub spin_weight: i32,
    pub ell: u32,
    pub m: i32,
    pub angular_eigenvalue_convention: AngularEigenvalueConvention,
    pub angular_eigenvalue_definition: Option<String>,
    pub fourier_convention: FourierConvention,
    pub frequency_normalization: FrequencyNormalization,
    pub family_classification: QnmFamilyClassification,
    /// Equatorial oscillator level Lbar for the normal prolate angular branch.
    /// For scalar fields Lbar = ell - |m|; for nonzero spin weight the mapping
    /// must be established by branch continuation and is not inferred here.
    pub normal_prolate_oscillator_level: Option<u32>,
    pub family_evidence: Option<QnmFamilyEvidence>,
    pub modes: Vec<QnmModeRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QnmSequenceValidationError {
    UnsupportedSchemaVersion(u32),
    EmptyField(&'static str),
    InvalidPrecision,
    InvalidSha256,
    InvalidSpin,
    InvalidModeNumbers,
    AngularEigenvalueConventionIncomplete,
    EmptySequence,
    SequencePositionsNotStrictlyIncreasing(usize),
    DuplicateOvertoneIndex(u32),
    NonFiniteModeValue(usize),
    InvalidModeResiduals(usize),
    InvalidFamilyEvidence,
    MissingFamilyEvidence,
    MissingWkbMetadata(usize),
    InvalidWkbMetadata(usize),
    InvalidWkbSpinDomain(usize),
    ExtremalFixtureRequiresScalarExactWkb,
}

impl core::fmt::Display for QnmSequenceValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported QNM sequence schema version {version}")
            }
            Self::EmptyField(name) => write!(f, "required QNM sequence field {name} is empty"),
            Self::InvalidPrecision => write!(f, "all QNM modes require precision_bits >= 53"),
            Self::InvalidSha256 => write!(
                f,
                "producer artifact SHA-256 must contain 64 hex characters"
            ),
            Self::InvalidSpin => write!(f, "spin_over_mass must be finite and satisfy |a/M| < 1"),
            Self::InvalidModeNumbers => {
                write!(f, "mode numbers must satisfy ell >= |s| and |m| <= ell")
            }
            Self::AngularEigenvalueConventionIncomplete => {
                write!(f, "producer-defined angular eigenvalue convention requires a definition")
            }
            Self::EmptySequence => write!(f, "at least one QNM mode is required"),
            Self::SequencePositionsNotStrictlyIncreasing(index) => {
                write!(
                    f,
                    "sequence position at mode {index} is not strictly increasing"
                )
            }
            Self::DuplicateOvertoneIndex(index) => {
                write!(
                    f,
                    "overtone index {index} occurs more than once in the sequence"
                )
            }
            Self::NonFiniteModeValue(index) => {
                write!(
                    f,
                    "QNM mode {index} has a non-finite frequency/eigenvalue/guess"
                )
            }
            Self::InvalidModeResiduals(index) => {
                write!(
                    f,
                    "QNM mode {index} has invalid or absent residual metadata"
                )
            }
            Self::InvalidFamilyEvidence => {
                write!(
                    f,
                    "family evidence contains invalid ratios, spacings, or residuals"
                )
            }
            Self::MissingFamilyEvidence => {
                write!(
                    f,
                    "prolate/oblate classification requires multi-mode asymptotic evidence"
                )
            }
            Self::MissingWkbMetadata(index) => {
                write!(
                    f,
                    "QNM mode {index} requires WKB index, truncation order, branch path and cycle metadata"
                )
            }
            Self::InvalidWkbMetadata(index) => {
                write!(
                    f,
                    "QNM mode {index} has non-finite or negative WKB periods/residual/error estimate"
                )
            }
            Self::InvalidWkbSpinDomain(index) => {
                write!(
                    f,
                    "QNM mode {index} uses the fixed-spin normal-prolate WKB method outside 0 < a/M < 1"
                )
            }
            Self::ExtremalFixtureRequiresScalarExactWkb => {
                write!(f, "exact extremality is represented only by a scalar exact-WKB fixture")
            }
        }
    }
}

impl std::error::Error for QnmSequenceValidationError {}

impl KerrQnmSequenceFixture {
    /// Return the dimensionless small-spin parameter governing the
    /// non-uniformity of the normal-prolate genus-one expansion,
    /// chi^(-4/3) / (N + 1/2). The asymptotic regime requires this parameter
    /// to be small; this accessor does not impose a universal cutoff.
    pub fn normal_prolate_wkb_small_spin_parameter(
        &self,
        mode: &QnmModeRecord,
    ) -> Option<f64> {
        if mode.solver_method != QnmSolverMethod::KerrNormalProlateGenusOneWkb
            || !(self.spin_over_mass > 0.0 && self.spin_over_mass < 1.0)
        {
            return None;
        }
        let quantization_index = mode.wkb_quantization_index?;
        Some(
            self.spin_over_mass.powf(-4.0 / 3.0)
                / (f64::from(quantization_index) + 0.5),
        )
    }

    /// Validate sequence structure and evidence presence. This does not prove
    /// that the producer's family label is mathematically correct.
    pub fn validate(&self) -> Result<(), QnmSequenceValidationError> {
        if self.schema_version != QNM_SEQUENCE_SCHEMA_VERSION {
            return Err(QnmSequenceValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.sequence_id.trim().is_empty() {
            return Err(QnmSequenceValidationError::EmptyField("sequence_id"));
        }
        if self.provenance.engine.trim().is_empty()
            || self.provenance.engine_version.trim().is_empty()
            || self.provenance.generated_at_utc.trim().is_empty()
        {
            return Err(QnmSequenceValidationError::EmptyField("provenance"));
        }
        if self.provenance.precision_bits < 53 {
            return Err(QnmSequenceValidationError::InvalidPrecision);
        }
        if let Some(digest) = &self.provenance.producer_artifact_sha256
            && (digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(QnmSequenceValidationError::InvalidSha256);
        }
        if !self.spin_over_mass.is_finite() || self.spin_over_mass.abs() > 1.0 {
            return Err(QnmSequenceValidationError::InvalidSpin);
        }
        if self.spin_over_mass.abs() == 1.0
            && (self.spin_weight != 0
                || self
                    .modes
                    .iter()
                    .any(|mode| mode.solver_method != QnmSolverMethod::ExactExtremalWkb))
        {
            return Err(QnmSequenceValidationError::ExtremalFixtureRequiresScalarExactWkb);
        }
        if self.ell < self.spin_weight.unsigned_abs() || self.m.unsigned_abs() > self.ell {
            return Err(QnmSequenceValidationError::InvalidModeNumbers);
        }
        if self.angular_eigenvalue_convention == AngularEigenvalueConvention::Other
            && self
                .angular_eigenvalue_definition
                .as_deref()
                .is_none_or(str::is_empty)
        {
            return Err(QnmSequenceValidationError::AngularEigenvalueConventionIncomplete);
        }

        if self.modes.is_empty() {
            return Err(QnmSequenceValidationError::EmptySequence);
        }

        let mut previous_position = None;
        let mut overtone_indices = std::collections::BTreeSet::new();
        for (index, mode) in self.modes.iter().enumerate() {
            if previous_position.is_some_and(|previous| mode.sequence_position <= previous) {
                return Err(
                    QnmSequenceValidationError::SequencePositionsNotStrictlyIncreasing(index),
                );
            }
            previous_position = Some(mode.sequence_position);

            if let Some(overtone) = mode.overtone_index
                && !overtone_indices.insert(overtone)
            {
                return Err(QnmSequenceValidationError::DuplicateOvertoneIndex(overtone));
            }
            if !mode.omega_m.is_finite()
                || !mode.angular_eigenvalue.is_finite()
                || mode.initial_guess.is_some_and(|guess| !guess.is_finite())
                || mode.angular_branch_id.trim().is_empty()
            {
                return Err(QnmSequenceValidationError::NonFiniteModeValue(index));
            }
            if mode.precision_bits < 53
                || mode
                    .residuals
                    .as_ref()
                    .is_none_or(|residuals| !residuals.validate())
            {
                return Err(QnmSequenceValidationError::InvalidModeResiduals(index));
            }
            if mode.solver_method == QnmSolverMethod::KerrNormalProlateGenusOneWkb {
                if self.spin_over_mass <= 0.0
                    || self.spin_over_mass >= 1.0
                {
                    return Err(QnmSequenceValidationError::InvalidWkbSpinDomain(index));
                }
                if mode.wkb_quantization_index.is_none()
                    || mode.wkb_truncation_order.is_none()
                    || mode.wkb_cycle_id.as_deref().is_none_or(str::is_empty)
                    || mode
                        .angular_continuation_path_id
                        .as_deref()
                        .is_none_or(str::is_empty)
                    || self.family_classification != QnmFamilyClassification::Prolate
                    || self.normal_prolate_oscillator_level.is_none()
                {
                    return Err(QnmSequenceValidationError::MissingWkbMetadata(index));
                }
                if mode.wkb_cycle_period_j0.is_none_or(|value| !value.is_finite())
                    || mode.wkb_cycle_period_j1.is_none_or(|value| !value.is_finite())
                    || mode
                        .wkb_quantization_residual
                        .is_none_or(|value| !value.is_finite() || value < 0.0)
                    || mode
                        .wkb_truncation_error_estimate
                        .is_none_or(|value| !value.is_finite() || value < 0.0)
                {
                    return Err(QnmSequenceValidationError::InvalidWkbMetadata(index));
                }
            }
        }

        match self.family_classification {
            QnmFamilyClassification::Prolate | QnmFamilyClassification::Oblate => {
                let evidence = self
                    .family_evidence
                    .as_ref()
                    .ok_or(QnmSequenceValidationError::MissingFamilyEvidence)?;
                if !evidence.angular_scaling_ratio.is_finite()
                    || !evidence.expected_angular_scaling_limit.is_finite()
                    || !evidence.observed_frequency_spacing_m.is_finite()
                    || !evidence.predicted_frequency_spacing_m.is_finite()
                    || !evidence.angular_scaling_relative_residual.is_finite()
                    || evidence.angular_scaling_relative_residual < 0.0
                    || !evidence.frequency_spacing_relative_residual.is_finite()
                    || evidence.frequency_spacing_relative_residual < 0.0
                    || evidence.consecutive_modes_used < 2
                    || self.modes.len() < evidence.consecutive_modes_used as usize
                {
                    return Err(QnmSequenceValidationError::InvalidFamilyEvidence);
                }
            }
            QnmFamilyClassification::Unresolved | QnmFamilyClassification::Ambiguous => {
                if let Some(evidence) = &self.family_evidence
                    && (!evidence.angular_scaling_ratio.is_finite()
                        || !evidence.expected_angular_scaling_limit.is_finite()
                        || !evidence.observed_frequency_spacing_m.is_finite()
                        || !evidence.predicted_frequency_spacing_m.is_finite()
                        || !evidence.angular_scaling_relative_residual.is_finite()
                        || evidence.angular_scaling_relative_residual < 0.0
                        || !evidence.frequency_spacing_relative_residual.is_finite()
                        || evidence.frequency_spacing_relative_residual < 0.0)
                {
                    return Err(QnmSequenceValidationError::InvalidFamilyEvidence);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sequence(classification: QnmFamilyClassification) -> KerrQnmSequenceFixture {
        let provenance = FixtureProvenance {
            engine: "bhpt-qnm".to_owned(),
            engine_version: "0.4.1".to_owned(),
            source_revision: Some("pinned-source".to_owned()),
            producer_artifact_sha256: Some("b".repeat(64)),
            generated_at_utc: "2026-11-09T00:00:00Z".to_owned(),
            precision_bits: 128,
        };
        let evidence = (classification == QnmFamilyClassification::Prolate
            || classification == QnmFamilyClassification::Oblate)
            .then_some(QnmFamilyEvidence {
                angular_scaling_ratio: ComplexValue { re: 0.99, im: 0.01 },
                expected_angular_scaling_limit: ComplexValue { re: 1.0, im: 0.0 },
                observed_frequency_spacing_m: ComplexValue {
                    re: 0.25,
                    im: -0.01,
                },
                predicted_frequency_spacing_m: ComplexValue { re: 0.25, im: 0.0 },
                angular_scaling_relative_residual: 0.02,
                frequency_spacing_relative_residual: 0.03,
                consecutive_modes_used: 3,
            });
        let modes = (0..3)
            .map(|position| QnmModeRecord {
                sequence_position: position,
                overtone_index: Some(position),
                omega_m: ComplexValue {
                    re: 0.5,
                    im: -0.08 - f64::from(position) * 0.25,
                },
                angular_eigenvalue: ComplexValue {
                    re: 3.0 + f64::from(position),
                    im: -0.1,
                },
                angular_branch_id: "continued-angular-branch".to_owned(),
                angular_continuation_path_id: None,
                solver_method: QnmSolverMethod::Leaver,
                method_applicability: QnmMethodApplicability::WithinDocumentedRegime,
                initial_guess: Some(ComplexValue { re: 0.5, im: -0.1 }),
                precision_bits: 128,
                residuals: Some(QnmModeResiduals {
                    angular_ode_relative: Some(1.0e-14),
                    radial_ode_relative: Some(1.0e-13),
                    continued_fraction_relative: Some(1.0e-15),
                    incidence_amplitude_relative: None,
                }),
                wkb_quantization_index: None,
                wkb_truncation_order: None,
                wkb_optimal_truncation_order: None,
                wkb_cycle_period_j0: None,
                wkb_cycle_period_j1: None,
                wkb_quantization_residual: None,
                wkb_truncation_error_estimate: None,
                wkb_cycle_id: None,
            })
            .collect();

        KerrQnmSequenceFixture {
            schema_version: QNM_SEQUENCE_SCHEMA_VERSION,
            sequence_id: "kerr-a03-s-2-l2-m2".to_owned(),
            provenance,
            spin_over_mass: 0.3,
            spin_weight: -2,
            ell: 2,
            m: 2,
            angular_eigenvalue_convention: AngularEigenvalueConvention::SpinWeightedSpheroidalA,
            angular_eigenvalue_definition: None,
            fourier_convention: FourierConvention::EMinusIOmegaT,
            frequency_normalization: FrequencyNormalization::DimensionlessMOmega,
            family_classification: classification,
            normal_prolate_oscillator_level: None,
            family_evidence: evidence,
            modes,
        }
    }

    #[test]
    fn producer_defined_qnm_angular_eigenvalue_requires_a_definition() {
        let mut value = sequence(QnmFamilyClassification::Unresolved);
        value.angular_eigenvalue_convention = AngularEigenvalueConvention::Other;
        assert_eq!(
            value.validate(),
            Err(QnmSequenceValidationError::AngularEigenvalueConventionIncomplete)
        );
        value.angular_eigenvalue_definition =
            Some("producer-defined angular separation constant".to_owned());
        assert_eq!(value.validate(), Ok(()));
    }

    #[test]
    fn unresolved_qnm_sequence_is_valid_without_a_forced_family_label() {
        assert_eq!(
            sequence(QnmFamilyClassification::Unresolved).validate(),
            Ok(())
        );
    }

    #[test]
    fn prolate_and_oblate_labels_require_multi_mode_evidence() {
        assert_eq!(
            sequence(QnmFamilyClassification::Prolate).validate(),
            Ok(())
        );
        assert_eq!(sequence(QnmFamilyClassification::Oblate).validate(), Ok(()));

        let mut missing = sequence(QnmFamilyClassification::Prolate);
        missing.family_evidence = None;
        assert_eq!(
            missing.validate(),
            Err(QnmSequenceValidationError::MissingFamilyEvidence)
        );
    }

    #[test]
    fn rejects_duplicate_overtone_labels_and_bad_residuals() {
        let mut duplicate = sequence(QnmFamilyClassification::Unresolved);
        duplicate.modes[1].overtone_index = Some(0);
        assert_eq!(
            duplicate.validate(),
            Err(QnmSequenceValidationError::DuplicateOvertoneIndex(0))
        );

        let mut invalid_residual = sequence(QnmFamilyClassification::Unresolved);
        invalid_residual.modes[0]
            .residuals
            .as_mut()
            .unwrap()
            .radial_ode_relative = Some(-1.0);
        assert_eq!(
            invalid_residual.validate(),
            Err(QnmSequenceValidationError::InvalidModeResiduals(0))
        );
    }

    #[test]
    fn accepts_unlabeled_overtone_when_sequence_position_is_preserved() {
        let mut value = sequence(QnmFamilyClassification::Ambiguous);
        value.modes[1].overtone_index = None;
        assert_eq!(value.validate(), Ok(()));
    }

    #[test]
    fn genus_one_prolate_wkb_preserves_quantization_index_separately_from_overtone() {
        let mut value = sequence(QnmFamilyClassification::Prolate);
        value.spin_weight = 0;
        value.normal_prolate_oscillator_level = Some(0);
        let mode = &mut value.modes[0];
        mode.angular_continuation_path_id =
            Some("schwarzschild-to-negative-imaginary-c-one-sided".to_owned());
        mode.solver_method = QnmSolverMethod::KerrNormalProlateGenusOneWkb;
        mode.overtone_index = Some(7);
        mode.wkb_quantization_index = Some(120);
        mode.wkb_truncation_order = Some(47);
        mode.wkb_optimal_truncation_order = Some(47);
        mode.wkb_cycle_period_j0 = Some(ComplexValue { re: 1.2, im: -0.3 });
        mode.wkb_cycle_period_j1 = Some(ComplexValue { re: 0.7, im: 0.1 });
        mode.wkb_quantization_residual = Some(1.0e-12);
        mode.wkb_truncation_error_estimate = Some(1.0e-8);
        mode.wkb_cycle_id = Some("outer-scattering-cycle".to_owned());
        assert_eq!(mode.overtone_index, Some(7));
        assert_eq!(mode.wkb_quantization_index, Some(120));
        assert_eq!(value.validate(), Ok(()));
    }

    #[test]
    fn genus_one_wkb_rejects_oblate_or_missing_cycle_metadata() {
        let mut wrong_family = sequence(QnmFamilyClassification::Oblate);
        wrong_family.modes[0].solver_method = QnmSolverMethod::KerrNormalProlateGenusOneWkb;
        wrong_family.modes[0].wkb_quantization_index = Some(120);
        wrong_family.modes[0].wkb_truncation_order = Some(47);
        wrong_family.modes[0].angular_continuation_path_id = Some("path".to_owned());
        wrong_family.modes[0].wkb_cycle_period_j0 =
            Some(ComplexValue { re: 1.0, im: 0.0 });
        wrong_family.modes[0].wkb_cycle_period_j1 =
            Some(ComplexValue { re: 1.0, im: 0.0 });
        wrong_family.modes[0].wkb_quantization_residual = Some(1.0e-12);
        wrong_family.modes[0].wkb_truncation_error_estimate = Some(1.0e-8);
        wrong_family.modes[0].wkb_cycle_id = Some("outer".to_owned());
        assert_eq!(
            wrong_family.validate(),
            Err(QnmSequenceValidationError::MissingWkbMetadata(0))
        );

        let mut missing_cycle = sequence(QnmFamilyClassification::Prolate);
        missing_cycle.spin_weight = 0;
        missing_cycle.normal_prolate_oscillator_level = Some(0);
        missing_cycle.modes[0].solver_method = QnmSolverMethod::KerrNormalProlateGenusOneWkb;
        missing_cycle.modes[0].wkb_quantization_index = Some(120);
        missing_cycle.modes[0].wkb_truncation_order = Some(47);
        missing_cycle.modes[0].angular_continuation_path_id = Some("path".to_owned());
        missing_cycle.modes[0].wkb_cycle_period_j0 =
            Some(ComplexValue { re: 1.0, im: 0.0 });
        missing_cycle.modes[0].wkb_cycle_period_j1 =
            Some(ComplexValue { re: 1.0, im: 0.0 });
        missing_cycle.modes[0].wkb_quantization_residual = Some(1.0e-12);
        missing_cycle.modes[0].wkb_truncation_error_estimate = Some(1.0e-8);
        assert_eq!(
            missing_cycle.validate(),
            Err(QnmSequenceValidationError::MissingWkbMetadata(0))
        );
    }

    #[test]
    fn genus_one_wkb_rejects_zero_or_negative_spin_domain() {
        let mut zero_spin = sequence(QnmFamilyClassification::Prolate);
        zero_spin.spin_weight = 0;
        zero_spin.normal_prolate_oscillator_level = Some(0);
        zero_spin.modes[0].solver_method = QnmSolverMethod::KerrNormalProlateGenusOneWkb;
        assert_eq!(
            zero_spin.validate(),
            Err(QnmSequenceValidationError::InvalidWkbSpinDomain(0))
        );

        let mut negative_spin = zero_spin;
        negative_spin.spin_over_mass = -0.7;
        assert_eq!(
            negative_spin.validate(),
            Err(QnmSequenceValidationError::InvalidWkbSpinDomain(0))
        );
    }

    #[test]
    fn normal_prolate_wkb_small_spin_parameter_uses_quantization_index_n_not_overtone_n() {
        let mut value = sequence(QnmFamilyClassification::Prolate);
        value.spin_weight = 0;
        value.normal_prolate_oscillator_level = Some(0);
        let mode = &mut value.modes[0];
        mode.solver_method = QnmSolverMethod::KerrNormalProlateGenusOneWkb;
        mode.wkb_quantization_index = Some(120);
        mode.overtone_index = Some(7);
        let parameter = value
            .normal_prolate_wkb_small_spin_parameter(mode)
            .unwrap();
        let expected = 0.3_f64.powf(-4.0 / 3.0) / 120.5;
        assert!((parameter - expected).abs() <= 4.0 * f64::EPSILON * expected);
        assert!(parameter < 1.0);
    }

    #[test]
    fn genus_one_wkb_requires_finite_periods_and_error_metadata() {
        let mut value = sequence(QnmFamilyClassification::Prolate);
        value.spin_weight = 0;
        value.normal_prolate_oscillator_level = Some(0);
        let mode = &mut value.modes[0];
        mode.solver_method = QnmSolverMethod::KerrNormalProlateGenusOneWkb;
        mode.angular_continuation_path_id = Some("branch-path".to_owned());
        mode.wkb_quantization_index = Some(120);
        mode.wkb_truncation_order = Some(47);
        mode.wkb_cycle_id = Some("outer".to_owned());
        mode.wkb_cycle_period_j0 = Some(ComplexValue { re: f64::NAN, im: 0.0 });
        mode.wkb_cycle_period_j1 = Some(ComplexValue { re: 1.0, im: 0.0 });
        mode.wkb_quantization_residual = Some(1.0e-12);
        mode.wkb_truncation_error_estimate = Some(1.0e-8);
        assert_eq!(
            value.validate(),
            Err(QnmSequenceValidationError::InvalidWkbMetadata(0))
        );
    }

    #[test]
    fn exact_extremal_wkb_fixture_is_scalar_only() {
        let mut scalar = sequence(QnmFamilyClassification::Unresolved);
        scalar.spin_over_mass = 1.0;
        for mode in &mut scalar.modes {
            mode.solver_method = QnmSolverMethod::ExactExtremalWkb;
        }
        assert_eq!(scalar.validate(), Ok(()));

        let mut gravitational = scalar.clone();
        gravitational.spin_weight = -2;
        assert_eq!(
            gravitational.validate(),
            Err(QnmSequenceValidationError::ExtremalFixtureRequiresScalarExactWkb)
        );
    }

    #[test]
    fn serde_roundtrip_preserves_sequence_and_family_evidence() {
        let original = sequence(QnmFamilyClassification::Prolate);
        let encoded = serde_json::to_string(&original).unwrap();
        let decoded: KerrQnmSequenceFixture = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, original);
        decoded.validate().unwrap();
    }
}
