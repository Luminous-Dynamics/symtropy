//! Typed Kerr QNM sequence fixtures.
//!
//! A solver's overtone index is metadata, not proof of family identity.
//! Prolate/oblate labels therefore carry explicit multi-mode asymptotic
//! evidence, while ambiguous or unresolved sequences remain representable.

use serde::{Deserialize, Serialize};

use crate::{ComplexValue, FixtureProvenance, FourierConvention, FrequencyNormalization};

pub const QNM_SEQUENCE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QnmSolverMethod {
    Leaver,
    IncidenceAmplitude,
    SpheroidalEigenvalue,
    SpectralTwoDimensional,
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
            && residuals.into_iter().any(Option::is_some)
    }
}

/// Multi-mode asymptotic summary used to justify a prolate or oblate label.
///
/// For a prolate sequence, angular_scaling_ratio is A/(i a omega); for an
/// oblate sequence it is A/(-a^2 omega^2). The corresponding ratio should
/// approach 1 in the declared convention. Frequency spacings are expressed
/// as dimensionless M delta-omega.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmFamilyEvidence {
    pub angular_scaling_ratio: ComplexValue,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QnmModeRecord {
    pub sequence_position: u32,
    pub overtone_index: Option<u32>,
    pub omega_m: ComplexValue,
    pub angular_eigenvalue: ComplexValue,
    pub angular_branch_id: String,
    pub solver_method: QnmSolverMethod,
    pub initial_guess: Option<ComplexValue>,
    pub precision_bits: u16,
    pub residuals: Option<QnmModeResiduals>,
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
    pub fourier_convention: FourierConvention,
    pub frequency_normalization: FrequencyNormalization,
    pub family_classification: QnmFamilyClassification,
    pub family_evidence: Option<QnmFamilyEvidence>,
    pub modes: Vec<QnmModeRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QnmSequenceValidationError {
    UnsupportedSchemaVersion(u32),
    EmptyField(&'static str),
    InvalidPrecision,
    InvalidSpin,
    InvalidModeNumbers,
    EmptySequence,
    SequencePositionsNotStrictlyIncreasing(usize),
    DuplicateOvertoneIndex(u32),
    NonFiniteModeValue(usize),
    InvalidModeResiduals(usize),
    InvalidFamilyEvidence,
    MissingFamilyEvidence,
}

impl core::fmt::Display for QnmSequenceValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported QNM sequence schema version {version}")
            }
            Self::EmptyField(name) => write!(f, "required QNM sequence field {name} is empty"),
            Self::InvalidPrecision => write!(f, "all QNM modes require precision_bits >= 53"),
            Self::InvalidSpin => write!(f, "spin_over_mass must be finite and satisfy |a/M| < 1"),
            Self::InvalidModeNumbers => {
                write!(f, "mode numbers must satisfy ell >= |s| and |m| <= ell")
            }
            Self::EmptySequence => write!(f, "at least one QNM mode is required"),
            Self::SequencePositionsNotStrictlyIncreasing(index) => {
                write!(f, "sequence position at mode {index} is not strictly increasing")
            }
            Self::DuplicateOvertoneIndex(index) => {
                write!(f, "overtone index {index} occurs more than once in the sequence")
            }
            Self::NonFiniteModeValue(index) => {
                write!(f, "QNM mode {index} has a non-finite frequency/eigenvalue/guess")
            }
            Self::InvalidModeResiduals(index) => {
                write!(f, "QNM mode {index} has invalid or absent residual metadata")
            }
            Self::InvalidFamilyEvidence => {
                write!(f, "family evidence contains invalid ratios, spacings, or residuals")
            }
            Self::MissingFamilyEvidence => {
                write!(f, "prolate/oblate classification requires multi-mode asymptotic evidence")
            }
        }
    }
}

impl std::error::Error for QnmSequenceValidationError {}

impl KerrQnmSequenceFixture {
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
        if let Some(digest) = &self.provenance.producer_artifact_sha256 {
            if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(QnmSequenceValidationError::EmptyField(
                    "producer_artifact_sha256",
                ));
            }
        }
        if !self.spin_over_mass.is_finite() || self.spin_over_mass.abs() >= 1.0 {
            return Err(QnmSequenceValidationError::InvalidSpin);
        }
        if self.ell < self.spin_weight.unsigned_abs() || self.m.unsigned_abs() > self.ell {
            return Err(QnmSequenceValidationError::InvalidModeNumbers);
        }
        if self.modes.is_empty() {
            return Err(QnmSequenceValidationError::EmptySequence);
        }

        let mut previous_position = None;
        let mut overtone_indices = std::collections::BTreeSet::new();
        for (index, mode) in self.modes.iter().enumerate() {
            if previous_position.is_some_and(|previous| mode.sequence_position <= previous) {
                return Err(QnmSequenceValidationError::SequencePositionsNotStrictlyIncreasing(
                    index,
                ));
            }
            previous_position = Some(mode.sequence_position);

            if let Some(overtone) = mode.overtone_index {
                if !overtone_indices.insert(overtone) {
                    return Err(QnmSequenceValidationError::DuplicateOvertoneIndex(overtone));
                }
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
        }

        match self.family_classification {
            QnmFamilyClassification::Prolate | QnmFamilyClassification::Oblate => {
                let evidence = self
                    .family_evidence
                    .as_ref()
                    .ok_or(QnmSequenceValidationError::MissingFamilyEvidence)?;
                if !evidence.angular_scaling_ratio.is_finite()
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
                if let Some(evidence) = &self.family_evidence {
                    if !evidence.angular_scaling_ratio.is_finite()
                        || !evidence.observed_frequency_spacing_m.is_finite()
                        || !evidence.predicted_frequency_spacing_m.is_finite()
                        || !evidence.angular_scaling_relative_residual.is_finite()
                        || evidence.angular_scaling_relative_residual < 0.0
                        || !evidence.frequency_spacing_relative_residual.is_finite()
                        || evidence.frequency_spacing_relative_residual < 0.0
                    {
                        return Err(QnmSequenceValidationError::InvalidFamilyEvidence);
                    }
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
                observed_frequency_spacing_m: ComplexValue { re: 0.25, im: -0.01 },
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
                solver_method: QnmSolverMethod::Leaver,
                initial_guess: Some(ComplexValue { re: 0.5, im: -0.1 }),
                precision_bits: 128,
                residuals: Some(QnmModeResiduals {
                    angular_ode_relative: Some(1.0e-14),
                    radial_ode_relative: Some(1.0e-13),
                    continued_fraction_relative: Some(1.0e-15),
                    incidence_amplitude_relative: None,
                }),
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
            fourier_convention: FourierConvention::EMinusIOmegaT,
            frequency_normalization: FrequencyNormalization::DimensionlessMOmega,
            family_classification: classification,
            family_evidence: evidence,
            modes,
        }
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
        assert_eq!(sequence(QnmFamilyClassification::Prolate).validate(), Ok(()));
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
    fn serde_roundtrip_preserves_sequence_and_family_evidence() {
        let original = sequence(QnmFamilyClassification::Prolate);
        let encoded = serde_json::to_string(&original).unwrap();
        let decoded: KerrQnmSequenceFixture = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, original);
        decoded.validate().unwrap();
    }
}
