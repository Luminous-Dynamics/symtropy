//! Provenance-preserving adapter from bounded damage outputs to condition events.
//!
//! This module does not calculate fatigue or crack growth. It validates a typed
//! envelope around outputs from the separate material-damage reference and
//! converts an admitted result into the existing append-only condition ledger.
//! Digests are syntax-checked and linked; source bytes and physical truth are
//! not authenticated here.

use std::collections::BTreeSet;
use std::fmt;

use crate::{
    ComponentIdentity, ConditionClaim, ConditionDisposition, ConditionEvent, ConditionQuantity,
    DamageMechanism, EvidenceClass, EvidenceRef, EventTime, TransitionError, TransitionKind,
};

const PARIS_CLOSURE_RELATIVE_TOLERANCE: f64 = 64.0 * f64::EPSILON;

/// Which bounded reference produced the output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageModelFamily {
    LinearMiner,
    ConstantDeltaKParis,
}

/// Declared model applicability envelope. Inputs outside this envelope are
/// rejected; the adapter does not extrapolate or infer material applicability.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModelApplicability {
    MinerStressAmplitudeMpa {
        minimum: f64,
        maximum: f64,
    },
    ParisDeltaKMpaSqrtM {
        minimum: f64,
        maximum: f64,
    },
}

/// Exact model/profile identity and its source artifact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelProfileArtifact {
    pub family: DamageModelFamily,
    pub profile_id: String,
    pub version: String,
    pub artifact: EvidenceRef,
}

/// A cycle-count artifact produced upstream from a source load history.
///
/// canonical_blocks_sha256 must identify the canonical ordered block payload
/// passed to the damage calculation. The adapter compares it to the digest
/// declared by the damage calculation; it does not hash either payload.
#[derive(Clone, Debug, PartialEq)]
pub struct CycleCountArtifact {
    pub artifact: EvidenceRef,
    pub algorithm_profile_id: String,
    pub total_cycles: u64,
    pub canonical_blocks_sha256: String,
    /// Minimum and maximum stress amplitudes represented by the block payload.
    /// These are relevant only to the linear Miner profile.
    pub minimum_stress_amplitude_mpa: Option<f64>,
    pub maximum_stress_amplitude_mpa: Option<f64>,
}

/// Calibration context is explicit because fatigue data are conditional on
/// material grade, environment, stress ratio and potentially material lot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MaterialCalibrationArtifact {
    pub artifact: EvidenceRef,
    pub material_grade_id: String,
    /// None means population/grade-level calibration, not lot-specific evidence.
    pub calibrated_material_lot_id: Option<String>,
    pub environment_profile_id: String,
    pub stress_ratio_profile_id: String,
}

/// Damage output copied from the bounded reference result. These values are
/// treated as model estimates, never as direct measurements.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DamageModelOutput {
    Miner {
        damage_fraction: f64,
        total_cycles: u64,
        assessment: MinerModelAssessment,
    },
    Paris {
        initial_crack_length_m: f64,
        crack_extension_m: f64,
        final_crack_length_m: f64,
        delta_k_mpa_sqrt_m: f64,
        cycles: u64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinerModelAssessment {
    BelowUnity,
    AtOrAboveUnity,
}

/// A complete upstream-to-condition binding for one damage calculation.
#[derive(Clone, Debug, PartialEq)]
pub struct DamageReceipt {
    pub claim_id: String,
    pub subject: ComponentIdentity,
    pub load_history: EvidenceRef,
    pub cycle_count: CycleCountArtifact,
    pub calibration: MaterialCalibrationArtifact,
    pub model_profile: ModelProfileArtifact,
    /// Digest of the exact canonical block payload supplied to the damage model.
    pub model_input_blocks_sha256: String,
    /// For Paris outputs, the upstream evidence used to establish Delta-K.
    pub delta_k_input: Option<EvidenceRef>,
    /// Output artifact, required to be ModelEstimate.
    pub output: EvidenceRef,
    pub output_value: DamageModelOutput,
    /// Absolute bound in the output quantity's unit. None remains unknown.
    pub absolute_uncertainty: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DamageReceiptError {
    EmptyClaimId,
    EmptyProfileId,
    EmptyProfileVersion,
    EmptyCycleCountProfile,
    EmptyMaterialGrade,
    EmptyEnvironmentProfile,
    EmptyStressRatioProfile,
    InvalidDigest,
    InvalidEvidence(&'static str),
    InvalidEvidenceClass(&'static str),
    DuplicateEvidenceId,
    ModelFamilyMismatch,
    ApplicabilityMismatch,
    InvalidApplicability,
    MissingStressRange,
    InvalidStressRange,
    StressOutsideApplicability,
    MissingDeltaKInput,
    UnexpectedDeltaKInput,
    InvalidCycleCount,
    CycleCountMismatch,
    BlockDigestMismatch,
    CalibrationLotMismatch,
    InvalidDamageQuantity,
    MinerAssessmentMismatch,
    DeltaKOutsideApplicability,
    ParisClosureMismatch,
    InvalidUncertainty,
    EmptyEventId,
}

impl fmt::Display for DamageReceiptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for DamageReceiptError {}

impl DamageReceipt {
    /// Validate all subject, provenance, applicability and result bindings,
    /// then create a normal ConditionEvent for ConditionLedger::apply.
    pub fn into_condition_event(
        &self,
        event_id: impl Into<String>,
        expected_revision: u64,
        time: EventTime,
    ) -> Result<ConditionEvent, DamageReceiptError> {
        if self.claim_id.trim().is_empty() {
            return Err(DamageReceiptError::EmptyClaimId);
        }
        let event_id = event_id.into();
        if event_id.trim().is_empty() {
            return Err(DamageReceiptError::EmptyEventId);
        }
        self.subject
            .validate()
            .map_err(|_| DamageReceiptError::InvalidEvidence("component identity"))?;

        validate_evidence(&self.load_history, "load history")?;
        validate_evidence(&self.cycle_count.artifact, "cycle-count artifact")?;
        validate_evidence(&self.calibration.artifact, "material calibration")?;
        validate_evidence(&self.model_profile.artifact, "model profile")?;
        validate_evidence(&self.output, "damage output")?;

        if self.output.class != EvidenceClass::ModelEstimate {
            return Err(DamageReceiptError::InvalidEvidenceClass("damage output must be ModelEstimate"));
        }
        if !matches!(
            self.cycle_count.artifact.class,
            EvidenceClass::Derived | EvidenceClass::Synthetic
        ) {
            return Err(DamageReceiptError::InvalidEvidenceClass(
                "cycle-count artifact must be Derived or Synthetic",
            ));
        }
        if self.calibration.artifact.class == EvidenceClass::ModelEstimate {
            return Err(DamageReceiptError::InvalidEvidenceClass(
                "material calibration cannot be only a model estimate",
            ));
        }
        if !matches!(
            self.model_profile.artifact.class,
            EvidenceClass::Derived | EvidenceClass::Synthetic
        ) {
            return Err(DamageReceiptError::InvalidEvidenceClass(
                "model profile artifact must be Derived or Synthetic",
            ));
        }

        if self.model_profile.profile_id.trim().is_empty() {
            return Err(DamageReceiptError::EmptyProfileId);
        }
        if self.model_profile.version.trim().is_empty() {
            return Err(DamageReceiptError::EmptyProfileVersion);
        }
        if self.cycle_count.algorithm_profile_id.trim().is_empty() {
            return Err(DamageReceiptError::EmptyCycleCountProfile);
        }
        if self.calibration.material_grade_id.trim().is_empty() {
            return Err(DamageReceiptError::EmptyMaterialGrade);
        }
        if self.calibration.environment_profile_id.trim().is_empty() {
            return Err(DamageReceiptError::EmptyEnvironmentProfile);
        }
        if self.calibration.stress_ratio_profile_id.trim().is_empty() {
            return Err(DamageReceiptError::EmptyStressRatioProfile);
        }
        if self
            .calibration
            .calibrated_material_lot_id
            .as_deref()
            .is_some_and(|lot| lot != self.subject.material_lot_id)
        {
            return Err(DamageReceiptError::CalibrationLotMismatch);
        }

        if self.cycle_count.total_cycles == 0 {
            return Err(DamageReceiptError::InvalidCycleCount);
        }
        validate_digest(&self.cycle_count.canonical_blocks_sha256)?;
        validate_digest(&self.model_input_blocks_sha256)?;
        if self.cycle_count.canonical_blocks_sha256 != self.model_input_blocks_sha256 {
            return Err(DamageReceiptError::BlockDigestMismatch);
        }

        let mut evidence_ids = BTreeSet::new();
        let mut sources = vec![
            self.load_history.clone(),
            self.cycle_count.artifact.clone(),
            self.calibration.artifact.clone(),
            self.model_profile.artifact.clone(),
            self.output.clone(),
        ];
        if let Some(delta_k) = &self.delta_k_input {
            validate_evidence(delta_k, "Delta-K input")?;
            sources.push(delta_k.clone());
        }
        for source in &sources {
            if !evidence_ids.insert(source.evidence_id.as_str()) {
                return Err(DamageReceiptError::DuplicateEvidenceId);
            }
        }

        let (mechanism, quantity, model_profile_id) = match self.output_value {
            DamageModelOutput::Miner {
                damage_fraction,
                total_cycles,
                assessment,
            } => {
                if self.model_profile.family != DamageModelFamily::LinearMiner {
                    return Err(DamageReceiptError::ModelFamilyMismatch);
                }
                if self.delta_k_input.is_some() {
                    return Err(DamageReceiptError::UnexpectedDeltaKInput);
                }
                let (minimum, maximum) = match self.model_profile.applicability() {
                    ModelApplicability::MinerStressAmplitudeMpa { minimum, maximum } => {
                        (minimum, maximum)
                    }
                    _ => return Err(DamageReceiptError::ApplicabilityMismatch),
                };
                validate_positive_range(minimum, maximum)?;
                let input_min = self
                    .cycle_count
                    .minimum_stress_amplitude_mpa
                    .ok_or(DamageReceiptError::MissingStressRange)?;
                let input_max = self
                    .cycle_count
                    .maximum_stress_amplitude_mpa
                    .ok_or(DamageReceiptError::MissingStressRange)?;
                validate_positive_range(input_min, input_max)?;
                if input_min < minimum || input_max > maximum {
                    return Err(DamageReceiptError::StressOutsideApplicability);
                }
                if total_cycles == 0 {
                    return Err(DamageReceiptError::InvalidCycleCount);
                }
                if total_cycles != self.cycle_count.total_cycles {
                    return Err(DamageReceiptError::CycleCountMismatch);
                }
                if !damage_fraction.is_finite() || damage_fraction < 0.0 {
                    return Err(DamageReceiptError::InvalidDamageQuantity);
                }
                let expected_assessment = if damage_fraction >= 1.0 {
                    MinerModelAssessment::AtOrAboveUnity
                } else {
                    MinerModelAssessment::BelowUnity
                };
                if assessment != expected_assessment {
                    return Err(DamageReceiptError::MinerAssessmentMismatch);
                }
                (
                    DamageMechanism::FatigueAccumulation,
                    ConditionQuantity::MinerDamage(damage_fraction),
                    format!("{}@{}", self.model_profile.profile_id, self.model_profile.version),
                )
            }
            DamageModelOutput::Paris {
                initial_crack_length_m,
                crack_extension_m,
                final_crack_length_m,
                delta_k_mpa_sqrt_m,
                cycles,
            } => {
                if self.model_profile.family != DamageModelFamily::ConstantDeltaKParis {
                    return Err(DamageReceiptError::ModelFamilyMismatch);
                }
                let (minimum, maximum) = match self.model_profile.applicability() {
                    ModelApplicability::ParisDeltaKMpaSqrtM { minimum, maximum } => {
                        (minimum, maximum)
                    }
                    _ => return Err(DamageReceiptError::ApplicabilityMismatch),
                };
                validate_positive_range(minimum, maximum)?;
                if self.delta_k_input.is_none() {
                    return Err(DamageReceiptError::MissingDeltaKInput);
                }
                if cycles == 0 {
                    return Err(DamageReceiptError::InvalidCycleCount);
                }
                if cycles != self.cycle_count.total_cycles {
                    return Err(DamageReceiptError::CycleCountMismatch);
                }
                if !initial_crack_length_m.is_finite()
                    || initial_crack_length_m <= 0.0
                    || !crack_extension_m.is_finite()
                    || crack_extension_m < 0.0
                    || !final_crack_length_m.is_finite()
                    || final_crack_length_m <= 0.0
                    || !delta_k_mpa_sqrt_m.is_finite()
                    || delta_k_mpa_sqrt_m <= 0.0
                {
                    return Err(DamageReceiptError::InvalidDamageQuantity);
                }
                if delta_k_mpa_sqrt_m < minimum || delta_k_mpa_sqrt_m > maximum {
                    return Err(DamageReceiptError::DeltaKOutsideApplicability);
                }
                let expected_final = initial_crack_length_m + crack_extension_m;
                let scale = initial_crack_length_m
                    .abs()
                    .max(crack_extension_m.abs())
                    .max(final_crack_length_m.abs())
                    .max(1.0e-12);
                let tolerance = PARIS_CLOSURE_RELATIVE_TOLERANCE * scale;
                if !expected_final.is_finite()
                    || (expected_final - final_crack_length_m).abs() > tolerance
                {
                    return Err(DamageReceiptError::ParisClosureMismatch);
                }
                (
                    DamageMechanism::CrackGrowth,
                    ConditionQuantity::CrackLengthMetres(final_crack_length_m),
                    format!("{}@{}", self.model_profile.profile_id, self.model_profile.version),
                )
            }
        };

        if self
            .absolute_uncertainty
            .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(DamageReceiptError::InvalidUncertainty);
        }

        let claim = ConditionClaim {
            claim_id: self.claim_id.clone(),
            mechanism,
            disposition: ConditionDisposition::InDomain,
            quantity: Some(quantity),
            uncertainty: self.absolute_uncertainty,
            evidence_id: self.output.evidence_id.clone(),
            evidence_class: EvidenceClass::ModelEstimate,
            model_profile_id: Some(model_profile_id),
        };

        Ok(ConditionEvent {
            event_id,
            expected_revision,
            subject: self.subject.clone(),
            time,
            sources,
            kind: TransitionKind::Assessment {
                claims: vec![claim],
            },
        })
    }
}

impl ModelProfileArtifact {
    fn applicability(&self) -> ModelApplicability {
        // Stored on the profile artifact to keep applicability inseparable
        // from the exact versioned profile identity.
        self.applicability
    }
}

fn validate_evidence(
    evidence: &EvidenceRef,
    role: &'static str,
) -> Result<(), DamageReceiptError> {
    evidence
        .validate()
        .map_err(|_| DamageReceiptError::InvalidEvidence(role))
}

fn validate_digest(value: &str) -> Result<(), DamageReceiptError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(DamageReceiptError::InvalidDigest);
    }
    Ok(())
}

fn validate_positive_range(minimum: f64, maximum: f64) -> Result<(), DamageReceiptError> {
    if !minimum.is_finite()
        || !maximum.is_finite()
        || minimum <= 0.0
        || maximum <= minimum
    {
        return Err(DamageReceiptError::InvalidApplicability);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConditionLedger, EventTime};

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn evidence(id: &str, class: EvidenceClass, character: char) -> EvidenceRef {
        EvidenceRef {
            evidence_id: id.into(),
            sha256: digest(character),
            class,
        }
    }

    fn subject() -> ComponentIdentity {
        ComponentIdentity {
            physical_article_id: "article-001".into(),
            component_instance_id: "beam-001-generation-1".into(),
            configuration_revision: 1,
            material_lot_id: "heat-42".into(),
        }
    }

    fn base_receipt() -> DamageReceipt {
        DamageReceipt {
            claim_id: "claim-miner-001".into(),
            subject: subject(),
            load_history: evidence("load-history-001", EvidenceClass::Measured, 'a'),
            cycle_count: CycleCountArtifact {
                artifact: evidence("cycle-record-001", EvidenceClass::Derived, 'b'),
                algorithm_profile_id: "rainflow-profile-v1".into(),
                total_cycles: 1000,
                canonical_blocks_sha256: digest('c'),
                minimum_stress_amplitude_mpa: Some(80.0),
                maximum_stress_amplitude_mpa: Some(160.0),
            },
            calibration: MaterialCalibrationArtifact {
                artifact: evidence("sn-calibration-001", EvidenceClass::Measured, 'd'),
                material_grade_id: "S355".into(),
                calibrated_material_lot_id: Some("heat-42".into()),
                environment_profile_id: "air-20c".into(),
                stress_ratio_profile_id: "r-0.1".into(),
            },
            model_profile: ModelProfileArtifact {
                family: DamageModelFamily::LinearMiner,
                profile_id: "sn-miner-s355".into(),
                version: "1".into(),
                artifact: evidence("model-profile-001", EvidenceClass::Derived, 'e'),
                applicability: ModelApplicability::MinerStressAmplitudeMpa {
                    minimum: 60.0,
                    maximum: 200.0,
                },
            },
            model_input_blocks_sha256: digest('c'),
            delta_k_input: None,
            output: evidence("damage-output-001", EvidenceClass::ModelEstimate, 'f'),
            output_value: DamageModelOutput::Miner {
                damage_fraction: 0.25,
                total_cycles: 1000,
                assessment: MinerModelAssessment::BelowUnity,
            },
            absolute_uncertainty: None,
        }
    }

    fn paris_receipt() -> DamageReceipt {
        let mut receipt = base_receipt();
        receipt.claim_id = "claim-paris-001".into();
        receipt.cycle_count.artifact =
            evidence("cycle-record-paris", EvidenceClass::Derived, '1');
        receipt.cycle_count.total_cycles = 10_000;
        receipt.cycle_count.minimum_stress_amplitude_mpa = None;
        receipt.cycle_count.maximum_stress_amplitude_mpa = None;
        receipt.model_profile = ModelProfileArtifact {
            family: DamageModelFamily::ConstantDeltaKParis,
            profile_id: "paris-s355".into(),
            version: "1".into(),
            artifact: evidence("model-profile-paris", EvidenceClass::Derived, '2'),
            applicability: ModelApplicability::ParisDeltaKMpaSqrtM {
                minimum: 1.0,
                maximum: 20.0,
            },
        };
        receipt.model_input_blocks_sha256 = digest('3');
        receipt.delta_k_input = Some(evidence("delta-k-001", EvidenceClass::Derived, '4'));
        receipt.output = evidence("paris-output-001", EvidenceClass::ModelEstimate, '5');
        receipt.output_value = DamageModelOutput::Paris {
            initial_crack_length_m: 0.010,
            crack_extension_m: 0.001,
            final_crack_length_m: 0.011,
            delta_k_mpa_sqrt_m: 12.0,
            cycles: 10_000,
        };
        receipt
    }

    fn time() -> EventTime {
        EventTime {
            clock_id: "test-clock".into(),
            event_time_ns: 10,
            recorded_time_ns: 11,
        }
    }

    #[test]
    fn valid_miner_receipt_becomes_condition_event_and_applies() {
        let receipt = base_receipt();
        let event = receipt
            .into_condition_event("event-miner-001", 0, time())
            .unwrap();
        let mut ledger = ConditionLedger::new(subject(), "test-clock").unwrap();
        let applied = ledger.apply(event).unwrap();

        assert_eq!(applied.revision, 1);
        assert_eq!(ledger.history().len(), 1);
        assert_eq!(
            ledger.history()[0].claim.quantity,
            Some(ConditionQuantity::MinerDamage(0.25))
        );
        assert_eq!(
            ledger.history()[0].claim.evidence_class,
            EvidenceClass::ModelEstimate
        );
    }

    #[test]
    fn valid_paris_receipt_preserves_final_crack_length_not_extension_as_length() {
        let receipt = paris_receipt();
        let event = receipt
            .into_condition_event("event-paris-001", 0, time())
            .unwrap();
        let crate::TransitionKind::Assessment { claims } = event.kind else {
            panic!("damage receipt must produce an assessment event");
        };
        assert_eq!(
            claims[0].quantity,
            Some(ConditionQuantity::CrackLengthMetres(0.011))
        );
    }

    #[test]
    fn cycle_count_mismatch_is_rejected() {
        let mut receipt = base_receipt();
        receipt.cycle_count.total_cycles = 999;
        assert_eq!(
            receipt.into_condition_event("event-1", 0, time()),
            Err(DamageReceiptError::CycleCountMismatch)
        );
    }

    #[test]
    fn canonical_input_block_digest_mismatch_is_rejected() {
        let mut receipt = base_receipt();
        receipt.model_input_blocks_sha256 = digest('9');
        assert_eq!(
            receipt.into_condition_event("event-1", 0, time()),
            Err(DamageReceiptError::BlockDigestMismatch)
        );
    }

    #[test]
    fn lot_specific_calibration_must_match_component_lot() {
        let mut receipt = base_receipt();
        receipt.calibration.calibrated_material_lot_id = Some("different-heat".into());
        assert_eq!(
            receipt.into_condition_event("event-1", 0, time()),
            Err(DamageReceiptError::CalibrationLotMismatch)
        );
    }

    #[test]
    fn profile_applicability_rejects_stress_range_extrapolation() {
        let mut receipt = base_receipt();
        receipt.cycle_count.maximum_stress_amplitude_mpa = Some(240.0);
        assert_eq!(
            receipt.into_condition_event("event-1", 0, time()),
            Err(DamageReceiptError::StressOutsideApplicability)
        );
    }

    #[test]
    fn paris_requires_driving_force_evidence() {
        let mut receipt = paris_receipt();
        receipt.delta_k_input = None;
        assert_eq!(
            receipt.into_condition_event("event-1", 0, time()),
            Err(DamageReceiptError::MissingDeltaKInput)
        );
    }

    #[test]
    fn paris_result_must_close_initial_plus_extension_to_final() {
        let mut receipt = paris_receipt();
        receipt.output_value = DamageModelOutput::Paris {
            initial_crack_length_m: 0.010,
            crack_extension_m: 0.001,
            final_crack_length_m: 0.013,
            delta_k_mpa_sqrt_m: 12.0,
            cycles: 10_000,
        };
        assert_eq!(
            receipt.into_condition_event("event-1", 0, time()),
            Err(DamageReceiptError::ParisClosureMismatch)
        );
    }

    #[test]
    fn output_cannot_be_mislabeled_as_measured() {
        let mut receipt = base_receipt();
        receipt.output.class = EvidenceClass::Measured;
        assert_eq!(
            receipt.into_condition_event("event-1", 0, time()),
            Err(DamageReceiptError::InvalidEvidenceClass(
                "damage output must be ModelEstimate"
            ))
        );
    }

    #[test]
    fn cycle_count_transformation_cannot_be_mislabeled_as_raw_measurement() {
        let mut receipt = base_receipt();
        receipt.cycle_count.artifact.class = EvidenceClass::Measured;
        assert_eq!(
            receipt.into_condition_event("event-1", 0, time()),
            Err(DamageReceiptError::InvalidEvidenceClass(
                "cycle-count artifact must be Derived or Synthetic"
            ))
        );
    }

    #[test]
    fn duplicate_evidence_ids_are_rejected() {
        let mut receipt = base_receipt();
        receipt.output.evidence_id = receipt.load_history.evidence_id.clone();
        assert_eq!(
            receipt.into_condition_event("event-1", 0, time()),
            Err(DamageReceiptError::DuplicateEvidenceId)
        );
    }

    #[test]
    fn unknown_uncertainty_stays_unknown() {
        let receipt = base_receipt();
        let event = receipt
            .into_condition_event("event-1", 0, time())
            .unwrap();
        let crate::TransitionKind::Assessment { claims } = event.kind else {
            panic!("damage receipt must produce an assessment event");
        };
        assert_eq!(claims[0].uncertainty, None);
    }

    #[test]
    fn calibration_evidence_cannot_be_only_a_model_prediction() {
        let mut receipt = base_receipt();
        receipt.calibration.artifact.class = EvidenceClass::ModelEstimate;
        assert_eq!(
            receipt.into_condition_event("event-1", 0, time()),
            Err(DamageReceiptError::InvalidEvidenceClass(
                "material calibration cannot be only a model estimate"
            ))
        );
    }

    #[test]
    fn calibration_profile_ids_must_be_declared() {
        let mut receipt = base_receipt();
        receipt.calibration.stress_ratio_profile_id.clear();
        assert_eq!(
            receipt.into_condition_event("event-1", 0, time()),
            Err(DamageReceiptError::EmptyStressRatioProfile)
        );
    }

    #[test]
    fn negative_or_non_finite_damage_is_rejected_without_ledger_mutation() {
        let mut receipt = base_receipt();
        receipt.output_value = DamageModelOutput::Miner {
            damage_fraction: f64::NAN,
            total_cycles: 1000,
            assessment: MinerModelAssessment::BelowUnity,
        };
        let mut ledger = ConditionLedger::new(subject(), "test-clock").unwrap();
        let before = ledger.clone();
        assert_eq!(
            receipt.into_condition_event("event-1", 0, time()),
            Err(DamageReceiptError::InvalidDamageQuantity)
        );
        assert_eq!(ledger, before);
    }

    #[test]
    fn stale_condition_revision_is_rejected_by_the_existing_ledger() {
        let receipt = base_receipt();
        let event = receipt
            .into_condition_event("event-1", 3, time())
            .unwrap();
        let mut ledger = ConditionLedger::new(subject(), "test-clock").unwrap();
        let before = ledger.clone();
        assert_eq!(ledger.apply(event), Err(TransitionError::StaleRevision));
        assert_eq!(ledger, before);
    }
}
