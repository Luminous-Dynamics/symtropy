#![forbid(unsafe_code)]

//! Set-valued fault diagnosis using calibrated behavioral envelopes.
//!
//! Instead of forcing an observation into the highest-scoring class, this
//! module returns every declared hypothesis whose multivariate response
//! envelope intersects the measurement-uncertainty box. Overlapping envelopes
//! remain an explicit ambiguity set. A singleton is unique only within the
//! frozen, reviewed hypothesis catalogue; it is not proof of a globally unique
//! physical root cause.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BehavioralChannelBand {
    pub signal_id: String,
    pub lower: f64,
    pub upper: f64,
    /// Positive scale used to normalize the distance from the behavioral set.
    pub normalization_scale: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BehavioralHypothesisEnvelope {
    pub hypothesis_id: String,
    pub regime_id: String,
    pub model_sha256: String,
    pub training_recordings_manifest_sha256: String,
    pub calibration_recordings_manifest_sha256: String,
    pub target_coverage_ppm: u32,
    pub independent_calibration_episodes: usize,
    /// Calibrated maximum normalized distance allowed outside the nominal box.
    pub maximum_standardized_gap: f64,
    pub calibration_review_receipt_sha256: String,
    /// Caller-supplied review flag; reviewer identity/signature are checked upstream.
    pub calibration_reviewed_by_independent_validator: bool,
    pub channels: Vec<BehavioralChannelBand>,
    /// Digest over every envelope field except this digest.
    pub envelope_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BehavioralSignalQuality {
    Current,
    Missing,
    Stale,
    Conflicted,
    Unsupported,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BehavioralMeasurementInterval {
    pub lower: f64,
    pub upper: f64,
    pub quality: BehavioralSignalQuality,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BehavioralObservation {
    pub recording_id: String,
    pub sample_index: u64,
    pub time_seconds: f64,
    pub regime_id: String,
    pub signals: BTreeMap<String, BehavioralMeasurementInterval>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BehavioralDiagnosisStatus {
    UniqueWithinDeclaredCatalogue,
    AmbiguousHypothesisSet,
    UnknownOrOutOfSupport,
    NotEvaluableUnsupportedRegime,
    NotEvaluableMissingOrDegradedInput,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct BehavioralHypothesisAssessment {
    pub hypothesis_id: String,
    pub admissible: bool,
    pub maximum_standardized_gap: f64,
    pub channel_gaps: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct BehavioralSetEvaluation {
    pub schema_version: String,
    pub recording_id: String,
    pub sample_index: u64,
    pub time_seconds: f64,
    pub regime_id: String,
    pub model_sha256: String,
    pub envelope_catalogue_sha256: String,
    pub observation_sha256: String,
    pub status: BehavioralDiagnosisStatus,
    /// A singleton means unique only relative to the complete declared catalogue.
    pub unique_hypothesis_id: Option<String>,
    pub admissible_hypothesis_ids: Vec<String>,
    pub unavailable_signal_ids: Vec<String>,
    pub assessments: Vec<BehavioralHypothesisAssessment>,
    pub global_unique_physical_cause_claim_allowed: bool,
    pub evaluation_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BehavioralSetError {
    EmptyCatalogue,
    EmptyIdentity,
    DuplicateHypothesis(String),
    DuplicateChannel(String),
    InvalidDigest(String),
    EnvelopeDigestMismatch(String),
    CatalogueMismatch,
    InsufficientCalibrationEpisodes {
        hypothesis_id: String,
        required: usize,
        available: usize,
    },
    CalibrationNotIndependentlyReviewed(String),
    InvalidEnvelopeBounds(String),
    InvalidObservation(String),
    Serialization,
}

impl fmt::Display for BehavioralSetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCatalogue => write!(formatter, "behavioral envelope catalogue is empty"),
            Self::EmptyIdentity => write!(
                formatter,
                "behavioral hypothesis, regime, recording or signal identity is empty"
            ),
            Self::DuplicateHypothesis(id) => {
                write!(formatter, "duplicate behavioral hypothesis: {id}")
            }
            Self::DuplicateChannel(id) => {
                write!(formatter, "duplicate channel in behavioral envelope: {id}")
            }
            Self::InvalidDigest(id) => write!(
                formatter,
                "behavioral envelope provenance digest is invalid: {id}"
            ),
            Self::EnvelopeDigestMismatch(id) => write!(
                formatter,
                "behavioral envelope digest does not match its contents: {id}"
            ),
            Self::CatalogueMismatch => write!(
                formatter,
                "behavioral envelopes do not share the same regime, model, provenance and channel set"
            ),
            Self::InsufficientCalibrationEpisodes {
                hypothesis_id,
                required,
                available,
            } => write!(
                formatter,
                "behavioral envelope {hypothesis_id} needs {required} independent calibration episodes, has {available}"
            ),
            Self::CalibrationNotIndependentlyReviewed(id) => write!(
                formatter,
                "behavioral envelope lacks an independent calibration review: {id}"
            ),
            Self::InvalidEnvelopeBounds(id) => write!(
                formatter,
                "behavioral envelope has invalid bounds or calibration margin: {id}"
            ),
            Self::InvalidObservation(id) => {
                write!(formatter, "behavioral observation is invalid: {id}")
            }
            Self::Serialization => write!(formatter, "failed to serialize behavioral-set evidence"),
        }
    }
}

impl std::error::Error for BehavioralSetError {}

/// Minimum independent calibration episodes for the finite-sample split
/// conformal order statistic ceil((n + 1) * coverage).
pub fn minimum_behavioral_calibration_episodes(target_coverage_ppm: u32) -> usize {
    if target_coverage_ppm == 0 || target_coverage_ppm >= 1_000_000 {
        return usize::MAX;
    }
    let numerator = u128::from(target_coverage_ppm);
    let denominator = 1_000_000_u128 - numerator;
    numerator.div_ceil(denominator) as usize
}

/// Compute the canonical envelope digest required by envelope_sha256.
pub fn behavioral_envelope_sha256(
    envelope: &BehavioralHypothesisEnvelope,
) -> Result<String, BehavioralSetError> {
    let payload = (
        envelope.hypothesis_id.as_str(),
        envelope.regime_id.as_str(),
        envelope.model_sha256.as_str(),
        envelope.training_recordings_manifest_sha256.as_str(),
        envelope.calibration_recordings_manifest_sha256.as_str(),
        envelope.target_coverage_ppm,
        envelope.independent_calibration_episodes,
        envelope.maximum_standardized_gap,
        envelope.calibration_review_receipt_sha256.as_str(),
        envelope.calibration_reviewed_by_independent_validator,
        &envelope.channels,
    );
    sha256_serializable(b"symtropy-behavioral-hypothesis-envelope-v1\0", &payload)
}

/// Compute a canonical digest for a validated envelope catalogue.
pub fn behavioral_envelope_catalogue_sha256(
    envelopes: &[BehavioralHypothesisEnvelope],
) -> Result<String, BehavioralSetError> {
    validate_behavioral_envelope_catalogue(envelopes)?;
    let mut ordered = envelopes
        .iter()
        .map(|envelope| {
            (
                envelope.hypothesis_id.clone(),
                envelope.envelope_sha256.clone(),
            )
        })
        .collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.0.cmp(&right.0));
    sha256_serializable(b"symtropy-behavioral-envelope-catalogue-v1\0", &ordered)
}

/// Evaluate whether each declared behavioral envelope intersects the observed
/// measurement-uncertainty box. Missing, stale, conflicted or unsupported inputs
/// are not interpreted as normal. A non-singleton result is an explicit set.
pub fn evaluate_behavioral_hypothesis_set(
    envelopes: &[BehavioralHypothesisEnvelope],
    observation: &BehavioralObservation,
) -> Result<BehavioralSetEvaluation, BehavioralSetError> {
    let (model_sha256, regime_id, required_signals) =
        validate_behavioral_envelope_catalogue(envelopes)?;
    validate_behavioral_observation(observation)?;

    let catalogue_sha256 = behavioral_envelope_catalogue_sha256(envelopes)?;
    let observation_sha256 =
        sha256_serializable(b"symtropy-behavioral-observation-v1\0", observation)?;
    let mut assessments = Vec::with_capacity(envelopes.len());
    let mut unavailable_signal_ids = Vec::new();

    let unsupported_regime = observation.regime_id != regime_id;
    for signal_id in &required_signals {
        match observation.signals.get(signal_id) {
            None => unavailable_signal_ids.push(signal_id.clone()),
            Some(measurement) if measurement.quality != BehavioralSignalQuality::Current => {
                unavailable_signal_ids.push(signal_id.clone());
            }
            Some(_) => {}
        }
    }
    unavailable_signal_ids.sort();
    unavailable_signal_ids.dedup();

    let status = if unsupported_regime {
        BehavioralDiagnosisStatus::NotEvaluableUnsupportedRegime
    } else if !unavailable_signal_ids.is_empty() {
        BehavioralDiagnosisStatus::NotEvaluableMissingOrDegradedInput
    } else {
        for envelope in envelopes {
            let mut channel_gaps = BTreeMap::new();
            let mut maximum_gap = 0.0_f64;
            for band in &envelope.channels {
                let measurement = observation
                    .signals
                    .get(&band.signal_id)
                    .expect("catalogue channel presence was checked above");
                let raw_gap = (band.lower - measurement.upper)
                    .max(measurement.lower - band.upper)
                    .max(0.0);
                let standardized_gap = raw_gap / band.normalization_scale;
                maximum_gap = maximum_gap.max(standardized_gap);
                channel_gaps.insert(band.signal_id.clone(), standardized_gap);
            }
            assessments.push(BehavioralHypothesisAssessment {
                hypothesis_id: envelope.hypothesis_id.clone(),
                admissible: maximum_gap <= envelope.maximum_standardized_gap,
                maximum_standardized_gap: maximum_gap,
                channel_gaps,
            });
        }
        assessments.sort_by(|left, right| left.hypothesis_id.cmp(&right.hypothesis_id));
        let admissible = assessments
            .iter()
            .filter(|assessment| assessment.admissible)
            .map(|assessment| assessment.hypothesis_id.clone())
            .collect::<Vec<_>>();
        match admissible.len() {
            0 => BehavioralDiagnosisStatus::UnknownOrOutOfSupport,
            1 => BehavioralDiagnosisStatus::UniqueWithinDeclaredCatalogue,
            _ => BehavioralDiagnosisStatus::AmbiguousHypothesisSet,
        }
    };

    let admissible_hypothesis_ids = assessments
        .iter()
        .filter(|assessment| assessment.admissible)
        .map(|assessment| assessment.hypothesis_id.clone())
        .collect::<Vec<_>>();
    let unique_hypothesis_id = (status == BehavioralDiagnosisStatus::UniqueWithinDeclaredCatalogue)
        .then(|| admissible_hypothesis_ids[0].clone());
    let mut evaluation = BehavioralSetEvaluation {
        schema_version: "symtropy-behavioral-set-evaluation-v0.1".into(),
        recording_id: observation.recording_id.clone(),
        sample_index: observation.sample_index,
        time_seconds: observation.time_seconds,
        regime_id: observation.regime_id.clone(),
        model_sha256: model_sha256.to_owned(),
        envelope_catalogue_sha256: catalogue_sha256,
        observation_sha256,
        status,
        unique_hypothesis_id,
        admissible_hypothesis_ids,
        unavailable_signal_ids,
        assessments,
        global_unique_physical_cause_claim_allowed: false,
        evaluation_sha256: String::new(),
    };
    evaluation.evaluation_sha256 = behavioral_evaluation_sha256(&evaluation)?;
    Ok(evaluation)
}

fn behavioral_evaluation_sha256(
    evaluation: &BehavioralSetEvaluation,
) -> Result<String, BehavioralSetError> {
    let payload = (
        evaluation.schema_version.as_str(),
        evaluation.recording_id.as_str(),
        evaluation.sample_index,
        evaluation.time_seconds,
        evaluation.regime_id.as_str(),
        evaluation.model_sha256.as_str(),
        evaluation.envelope_catalogue_sha256.as_str(),
        evaluation.observation_sha256.as_str(),
        &evaluation.status,
        &evaluation.unique_hypothesis_id,
        &evaluation.admissible_hypothesis_ids,
        &evaluation.unavailable_signal_ids,
        &evaluation.assessments,
        evaluation.global_unique_physical_cause_claim_allowed,
    );
    sha256_serializable(b"symtropy-behavioral-set-evaluation-v1\0", &payload)
}

fn validate_behavioral_envelope_catalogue(
    envelopes: &[BehavioralHypothesisEnvelope],
) -> Result<(&str, &str, BTreeSet<String>), BehavioralSetError> {
    if envelopes.is_empty() {
        return Err(BehavioralSetError::EmptyCatalogue);
    }
    let first = &envelopes[0];
    if first.regime_id.trim().is_empty() || first.hypothesis_id.trim().is_empty() {
        return Err(BehavioralSetError::EmptyIdentity);
    }
    let mut hypothesis_ids = BTreeSet::new();
    let mut shared_signals = BTreeSet::new();

    for (index, envelope) in envelopes.iter().enumerate() {
        if envelope.hypothesis_id.trim().is_empty() || envelope.regime_id.trim().is_empty() {
            return Err(BehavioralSetError::EmptyIdentity);
        }
        if !hypothesis_ids.insert(envelope.hypothesis_id.clone()) {
            return Err(BehavioralSetError::DuplicateHypothesis(
                envelope.hypothesis_id.clone(),
            ));
        }
        if envelope.regime_id != first.regime_id
            || envelope.model_sha256 != first.model_sha256
            || envelope.training_recordings_manifest_sha256
                != first.training_recordings_manifest_sha256
            || envelope.calibration_recordings_manifest_sha256
                != first.calibration_recordings_manifest_sha256
            || envelope.target_coverage_ppm != first.target_coverage_ppm
        {
            return Err(BehavioralSetError::CatalogueMismatch);
        }
        if !is_sha256_hex(&envelope.model_sha256)
            || !is_sha256_hex(&envelope.training_recordings_manifest_sha256)
            || !is_sha256_hex(&envelope.calibration_recordings_manifest_sha256)
            || !is_sha256_hex(&envelope.calibration_review_receipt_sha256)
            || !is_sha256_hex(&envelope.envelope_sha256)
        {
            return Err(BehavioralSetError::InvalidDigest(
                envelope.hypothesis_id.clone(),
            ));
        }
        if envelope.training_recordings_manifest_sha256
            == envelope.calibration_recordings_manifest_sha256
        {
            return Err(BehavioralSetError::CatalogueMismatch);
        }
        if !envelope.calibration_reviewed_by_independent_validator {
            return Err(BehavioralSetError::CalibrationNotIndependentlyReviewed(
                envelope.hypothesis_id.clone(),
            ));
        }
        if envelope.target_coverage_ppm == 0 || envelope.target_coverage_ppm >= 1_000_000 {
            return Err(BehavioralSetError::InvalidEnvelopeBounds(
                envelope.hypothesis_id.clone(),
            ));
        }
        let required_episodes =
            minimum_behavioral_calibration_episodes(envelope.target_coverage_ppm);
        if envelope.independent_calibration_episodes < required_episodes {
            return Err(BehavioralSetError::InsufficientCalibrationEpisodes {
                hypothesis_id: envelope.hypothesis_id.clone(),
                required: required_episodes,
                available: envelope.independent_calibration_episodes,
            });
        }
        if !envelope.maximum_standardized_gap.is_finite()
            || envelope.maximum_standardized_gap < 0.0
            || envelope.channels.is_empty()
        {
            return Err(BehavioralSetError::InvalidEnvelopeBounds(
                envelope.hypothesis_id.clone(),
            ));
        }
        let mut channel_ids = BTreeSet::new();
        for channel in &envelope.channels {
            if channel.signal_id.trim().is_empty() {
                return Err(BehavioralSetError::EmptyIdentity);
            }
            if !channel_ids.insert(channel.signal_id.clone()) {
                return Err(BehavioralSetError::DuplicateChannel(
                    channel.signal_id.clone(),
                ));
            }
            if !channel.lower.is_finite()
                || !channel.upper.is_finite()
                || channel.lower > channel.upper
                || !channel.normalization_scale.is_finite()
                || channel.normalization_scale <= 0.0
            {
                return Err(BehavioralSetError::InvalidEnvelopeBounds(
                    envelope.hypothesis_id.clone(),
                ));
            }
        }
        if index == 0 {
            shared_signals = channel_ids;
        } else if channel_ids != shared_signals {
            return Err(BehavioralSetError::CatalogueMismatch);
        }
        if behavioral_envelope_sha256(envelope)? != envelope.envelope_sha256 {
            return Err(BehavioralSetError::EnvelopeDigestMismatch(
                envelope.hypothesis_id.clone(),
            ));
        }
    }
    Ok((
        first.model_sha256.as_str(),
        first.regime_id.as_str(),
        shared_signals,
    ))
}

fn validate_behavioral_observation(
    observation: &BehavioralObservation,
) -> Result<(), BehavioralSetError> {
    if observation.recording_id.trim().is_empty() || observation.regime_id.trim().is_empty() {
        return Err(BehavioralSetError::EmptyIdentity);
    }
    if !observation.time_seconds.is_finite() || observation.time_seconds < 0.0 {
        return Err(BehavioralSetError::InvalidObservation(
            observation.recording_id.clone(),
        ));
    }
    for (signal_id, measurement) in &observation.signals {
        if signal_id.trim().is_empty()
            || !measurement.lower.is_finite()
            || !measurement.upper.is_finite()
            || measurement.lower > measurement.upper
        {
            return Err(BehavioralSetError::InvalidObservation(
                observation.recording_id.clone(),
            ));
        }
    }
    Ok(())
}

fn sha256_serializable<T: Serialize>(
    domain: &[u8],
    value: &T,
) -> Result<String, BehavioralSetError> {
    let encoded = serde_json::to_vec(value).map_err(|_| BehavioralSetError::Serialization)?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((encoded.len() as u64).to_le_bytes());
    hasher.update(encoded);
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn envelope(hypothesis_id: &str, lower: f64, upper: f64) -> BehavioralHypothesisEnvelope {
        envelope_in_regime(hypothesis_id, "wltp_mid_load", lower, upper)
    }

    fn envelope_in_regime(
        hypothesis_id: &str,
        regime_id: &str,
        lower: f64,
        upper: f64,
    ) -> BehavioralHypothesisEnvelope {
        let mut item = BehavioralHypothesisEnvelope {
            hypothesis_id: hypothesis_id.into(),
            regime_id: regime_id.into(),
            model_sha256: digest('a'),
            training_recordings_manifest_sha256: digest('b'),
            calibration_recordings_manifest_sha256: digest('c'),
            target_coverage_ppm: 950_000,
            independent_calibration_episodes: 19,
            maximum_standardized_gap: 0.0,
            calibration_review_receipt_sha256: digest('d'),
            calibration_reviewed_by_independent_validator: true,
            channels: vec![
                BehavioralChannelBand {
                    signal_id: "ypic".into(),
                    lower,
                    upper,
                    normalization_scale: 0.1,
                },
                BehavioralChannelBand {
                    signal_id: "yomega".into(),
                    lower: 100.0,
                    upper: 110.0,
                    normalization_scale: 1.0,
                },
            ],
            envelope_sha256: digest('0'),
        };
        item.envelope_sha256 = behavioral_envelope_sha256(&item).unwrap();
        item
    }

    fn observation(pressure: f64, speed: f64) -> BehavioralObservation {
        observation_in_regime("wltp_mid_load", pressure, speed)
    }

    fn observation_in_regime(regime_id: &str, pressure: f64, speed: f64) -> BehavioralObservation {
        BehavioralObservation {
            recording_id: "run-1".into(),
            sample_index: 10,
            time_seconds: 0.5,
            regime_id: regime_id.into(),
            signals: BTreeMap::from([
                (
                    "ypic".into(),
                    BehavioralMeasurementInterval {
                        lower: pressure,
                        upper: pressure,
                        quality: BehavioralSignalQuality::Current,
                    },
                ),
                (
                    "yomega".into(),
                    BehavioralMeasurementInterval {
                        lower: speed,
                        upper: speed,
                        quality: BehavioralSignalQuality::Current,
                    },
                ),
            ]),
        }
    }

    #[test]
    fn overlapping_behavioral_envelopes_return_an_ambiguity_set() {
        let envelopes = vec![
            envelope("normal", 1.0, 1.5),
            envelope("intercooler_fault", 1.3, 1.8),
        ];
        let result =
            evaluate_behavioral_hypothesis_set(&envelopes, &observation(1.4, 105.0)).unwrap();
        assert_eq!(
            result.status,
            BehavioralDiagnosisStatus::AmbiguousHypothesisSet
        );
        assert_eq!(
            result.admissible_hypothesis_ids,
            vec!["intercooler_fault".to_owned(), "normal".to_owned()]
        );
        assert_eq!(result.unique_hypothesis_id, None);
        assert!(!result.global_unique_physical_cause_claim_allowed);
    }

    #[test]
    fn a_disjoint_singleton_is_unique_only_within_the_declared_catalogue() {
        let envelopes = vec![
            envelope("normal", 1.0, 1.2),
            envelope("intercooler_fault", 1.5, 1.8),
        ];
        let result =
            evaluate_behavioral_hypothesis_set(&envelopes, &observation(1.65, 105.0)).unwrap();
        assert_eq!(
            result.status,
            BehavioralDiagnosisStatus::UniqueWithinDeclaredCatalogue
        );
        assert_eq!(
            result.unique_hypothesis_id.as_deref(),
            Some("intercooler_fault")
        );
        assert!(!result.global_unique_physical_cause_claim_allowed);
    }

    #[test]
    fn no_behavioral_envelope_match_is_unknown_not_a_forced_known_fault() {
        let envelopes = vec![
            envelope("normal", 1.0, 1.2),
            envelope("intercooler_fault", 1.5, 1.8),
        ];
        let result =
            evaluate_behavioral_hypothesis_set(&envelopes, &observation(2.4, 105.0)).unwrap();
        assert_eq!(
            result.status,
            BehavioralDiagnosisStatus::UnknownOrOutOfSupport
        );
        assert_eq!(result.admissible_hypothesis_ids, Vec::<String>::new());
    }

    #[test]
    fn missing_stale_or_conflicted_inputs_are_not_interpreted_as_normal() {
        let envelopes = vec![
            envelope("normal", 1.0, 1.2),
            envelope("intercooler_fault", 1.5, 1.8),
        ];
        let mut missing = observation(1.1, 105.0);
        missing.signals.remove("ypic");
        assert_eq!(
            evaluate_behavioral_hypothesis_set(&envelopes, &missing)
                .unwrap()
                .status,
            BehavioralDiagnosisStatus::NotEvaluableMissingOrDegradedInput
        );
        let mut stale = observation(1.1, 105.0);
        stale.signals.get_mut("ypic").unwrap().quality = BehavioralSignalQuality::Stale;
        assert_eq!(
            evaluate_behavioral_hypothesis_set(&envelopes, &stale)
                .unwrap()
                .status,
            BehavioralDiagnosisStatus::NotEvaluableMissingOrDegradedInput
        );
    }

    #[test]
    fn unsupported_regime_is_not_compared_to_another_regimes_envelopes() {
        let envelopes = vec![
            envelope("normal", 1.0, 1.2),
            envelope("intercooler_fault", 1.5, 1.8),
        ];
        let mut sample = observation(1.1, 105.0);
        sample.regime_id = "unsupported_transient".into();
        assert_eq!(
            evaluate_behavioral_hypothesis_set(&envelopes, &sample)
                .unwrap()
                .status,
            BehavioralDiagnosisStatus::NotEvaluableUnsupportedRegime
        );
    }

    #[test]
    fn finite_sample_calibration_minima_are_enforced_per_hypothesis() {
        assert_eq!(minimum_behavioral_calibration_episodes(900_000), 9);
        assert_eq!(minimum_behavioral_calibration_episodes(950_000), 19);
        assert_eq!(minimum_behavioral_calibration_episodes(990_000), 99);
        let mut underpowered = envelope("normal", 1.0, 1.2);
        underpowered.independent_calibration_episodes = 18;
        underpowered.envelope_sha256 = behavioral_envelope_sha256(&underpowered).unwrap();
        let second = envelope("intercooler_fault", 1.5, 1.8);
        assert_eq!(
            evaluate_behavioral_hypothesis_set(&[underpowered, second], &observation(1.1, 105.0)),
            Err(BehavioralSetError::InsufficientCalibrationEpisodes {
                hypothesis_id: "normal".into(),
                required: 19,
                available: 18
            })
        );
    }

    #[test]
    fn tampered_or_unreviewed_envelope_fails_before_diagnosis() {
        let mut tampered = envelope("normal", 1.0, 1.2);
        tampered.channels[0].upper += 0.1;
        assert!(matches!(
            evaluate_behavioral_hypothesis_set(
                &[tampered.clone(), envelope("fault", 1.5, 1.8)],
                &observation(1.1, 105.0)
            ),
            Err(BehavioralSetError::EnvelopeDigestMismatch(_))
        ));
        let mut unreviewed = envelope("normal", 1.0, 1.2);
        unreviewed.calibration_reviewed_by_independent_validator = false;
        unreviewed.envelope_sha256 = behavioral_envelope_sha256(&unreviewed).unwrap();
        assert!(matches!(
            evaluate_behavioral_hypothesis_set(
                &[unreviewed, envelope("fault", 1.5, 1.8)],
                &observation(1.1, 105.0)
            ),
            Err(BehavioralSetError::CalibrationNotIndependentlyReviewed(_))
        ));
    }

    #[test]
    fn published_ftp75_standard_fpim_minus_ten_remains_ambiguous() {
        let envelopes = vec![
            envelope_in_regime("normal", "ftp75_standard_low_load", 1.0, 1.5),
            envelope_in_regime("fpim_negative_bias", "ftp75_standard_low_load", 1.3, 1.8),
        ];
        let result = evaluate_behavioral_hypothesis_set(
            &envelopes,
            &observation_in_regime("ftp75_standard_low_load", 1.4, 105.0),
        )
        .unwrap();
        assert_eq!(
            result.status,
            BehavioralDiagnosisStatus::AmbiguousHypothesisSet
        );
        assert_eq!(
            result.admissible_hypothesis_ids,
            vec!["fpim_negative_bias".to_owned(), "normal".to_owned()]
        );
        assert_eq!(result.unique_hypothesis_id, None);
        assert!(!result.global_unique_physical_cause_claim_allowed);
    }

    #[test]
    fn published_ftp75_highway_fpim_minus_ten_is_separable_only_in_its_regime() {
        let envelopes = vec![
            envelope_in_regime("normal", "ftp75_highway", 1.0, 1.2),
            envelope_in_regime("fpim_negative_bias", "ftp75_highway", 1.5, 1.8),
        ];
        let result = evaluate_behavioral_hypothesis_set(
            &envelopes,
            &observation_in_regime("ftp75_highway", 1.65, 105.0),
        )
        .unwrap();
        assert_eq!(
            result.status,
            BehavioralDiagnosisStatus::UniqueWithinDeclaredCatalogue
        );
        assert_eq!(
            result.unique_hypothesis_id.as_deref(),
            Some("fpim_negative_bias")
        );
        assert!(!result.global_unique_physical_cause_claim_allowed);
        let wrong_regime = observation_in_regime("ftp75_standard_low_load", 1.65, 105.0);
        assert_eq!(
            evaluate_behavioral_hypothesis_set(&envelopes, &wrong_regime)
                .unwrap()
                .status,
            BehavioralDiagnosisStatus::NotEvaluableUnsupportedRegime
        );
    }

    #[test]
    fn published_wltp_fpic_plus_eight_is_unknown_not_forced_into_a_known_fault() {
        let envelopes = vec![
            envelope_in_regime("normal", "wltp", 1.0, 1.2),
            envelope_in_regime("fpic_positive_bias", "wltp", 1.5, 1.8),
            envelope_in_regime("fpim_negative_bias", "wltp", 1.9, 2.1),
        ];
        let result = evaluate_behavioral_hypothesis_set(
            &envelopes,
            &observation_in_regime("wltp", 2.4, 105.0),
        )
        .unwrap();
        assert_eq!(
            result.status,
            BehavioralDiagnosisStatus::UnknownOrOutOfSupport
        );
        assert!(result.admissible_hypothesis_ids.is_empty());
        assert_eq!(result.unique_hypothesis_id, None);
        assert!(!result.global_unique_physical_cause_claim_allowed);
    }

    #[test]
    fn published_boundary_fixture_matches_official_reference_cells() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../dx2026-boundary-falsification-v0.1.json"))
                .unwrap();
        assert_eq!(
            fixture["schema_version"],
            "liu-ice-dx2026-boundary-falsification-v0.1"
        );
        let cases = fixture["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 3);
        assert_eq!(cases[0]["published"]["tdr_percent"], 21.12);
        assert_eq!(cases[1]["published"]["score_percent"], 97.05);
        assert_eq!(cases[2]["published"]["fia_percent"], 0.0);
        assert_eq!(
            cases[0]["synthetic_behavioral_set_fixture"]["expected_status"],
            "ambiguous_hypothesis_set"
        );
        assert_eq!(
            cases[1]["synthetic_behavioral_set_fixture"]["expected_status"],
            "unique_within_declared_catalogue"
        );
        assert_eq!(
            cases[2]["synthetic_behavioral_set_fixture"]["expected_status"],
            "unknown_or_out_of_support"
        );
        assert!(
            fixture["research_targets_not_achieved"]
                .as_array()
                .unwrap()
                .iter()
                .all(|target| target["status"] == "aspirational")
        );
    }

    #[test]
    fn envelope_catalogue_is_order_invariant_and_digest_bound() {
        let first = vec![envelope("normal", 1.0, 1.2), envelope("fault", 1.5, 1.8)];
        let second = first.iter().cloned().rev().collect::<Vec<_>>();
        assert_eq!(
            behavioral_envelope_catalogue_sha256(&first).unwrap(),
            behavioral_envelope_catalogue_sha256(&second).unwrap()
        );
        let result = evaluate_behavioral_hypothesis_set(&first, &observation(1.1, 105.0)).unwrap();
        assert_eq!(
            result.envelope_catalogue_sha256,
            behavioral_envelope_catalogue_sha256(&first).unwrap()
        );
        assert_eq!(
            result.evaluation_sha256,
            behavioral_evaluation_sha256(&result).unwrap()
        );
    }
}
