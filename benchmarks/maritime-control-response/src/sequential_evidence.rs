#![forbid(unsafe_code)]

//! Fixed-channel, anytime-valid sequential likelihood-ratio evidence over
//! predeclared calibrated evidence blocks.
//!
//! Streaming inference may emit one result per sample, but this evidence
//! accumulator accepts only non-overlapping calibrated blocks with an
//! independently reviewed dependence receipt and a frozen minimum separation.
//! The block aggregation and dependence assumptions remain model-conditional;
//! this gate cannot prove independence or make misspecified likelihoods valid.
//!
//! A crossed threshold means "evidence against this declared null under this
//! calibrated model", not "physical cause proved", "fault uniquely isolated",
//! or "safe to operate". Unknown outcomes outside the model support poison the
//! session instead of creating infinite evidence.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

pub const SEQUENTIAL_EVIDENCE_MODEL_SCHEMA: &str = "symtropy-sequential-evidence-model-v0.2";
pub const SEQUENTIAL_EVIDENCE_RECEIPT_SCHEMA: &str = "symtropy-sequential-evidence-receipt-v0.2";
const PROBABILITY_TOLERANCE: f64 = 1e-9;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SequentialOutcomeLikelihood {
    pub outcome_id: String,
    pub probability_under_null: f64,
    pub probability_under_alternative: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SequentialEvidenceUnitKind {
    NonOverlappingCalibratedBlock,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SequentialLikelihoodModel {
    pub schema_version: String,
    pub channel_id: String,
    pub null_hypothesis_id: String,
    pub alternative_hypothesis_id: String,
    /// Frozen model producing the outcome categories for this channel.
    pub model_sha256: String,
    /// Independent calibration dataset/procedure digest for block-level outcomes.
    pub calibration_sha256: String,
    /// Reference to an independently reviewed calibration receipt.
    /// This digest alone does not authenticate the issuer or reviewer.
    pub calibration_review_receipt_sha256: String,
    pub evidence_unit_kind: SequentialEvidenceUnitKind,
    /// Frozen block aggregation protocol and upstream dependence review.
    pub block_aggregation_protocol_sha256: String,
    pub dependence_review_receipt_sha256: String,
    /// Must be set only after upstream signature, issuer and scope checks succeed.
    pub dependence_review_verified_upstream: bool,
    /// Minimum temporal separation between the end of one block and start of the next.
    pub minimum_block_separation_seconds: f64,
    pub outcomes: Vec<SequentialOutcomeLikelihood>,
    /// Hash over the model and all outcome likelihoods, excluding this field.
    pub likelihood_table_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SequentialEvidenceConfig {
    /// Family-wise false-alarm budget in parts per million.
    pub familywise_alpha_ppm: u32,
    /// Number of alternative fault hypotheses tested in the same family.
    /// Bonferroni allocation is used; no independence assumption is required.
    pub alternative_count: u32,
}

impl SequentialEvidenceConfig {
    pub fn conservative() -> Self {
        Self {
            familywise_alpha_ppm: 1_000,
            alternative_count: 4,
        }
    }

    pub fn per_alternative_alpha_ppm(&self) -> Result<u32, SequentialEvidenceError> {
        if self.familywise_alpha_ppm == 0
            || self.familywise_alpha_ppm > 100_000
            || self.alternative_count == 0
            || self.alternative_count > self.familywise_alpha_ppm
        {
            return Err(SequentialEvidenceError::InvalidConfig);
        }
        Ok(self.familywise_alpha_ppm / self.alternative_count)
    }

    pub fn log_evidence_threshold(&self) -> Result<f64, SequentialEvidenceError> {
        let alpha_ppm = self.per_alternative_alpha_ppm()?;
        Ok((1_000_000.0 / f64::from(alpha_ppm)).ln())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SequentialEvidenceObservation {
    pub recording_id: String,
    /// Contiguous block ordinal, not the raw 20 Hz sensor sample index.
    pub sample_index: u64,
    pub time_seconds: f64,
    pub outcome_id: String,
    pub evidence_unit_id: String,
    pub block_start_seconds: f64,
    pub block_end_seconds: f64,
    pub block_manifest_sha256: String,
    /// Input receipt from the append-only block aggregation/streaming contract.
    pub input_receipt_sha256: String,
    pub model_sha256: String,
    pub likelihood_table_sha256: String,
    pub calibration_sha256: String,
    pub block_aggregation_protocol_sha256: String,
    pub dependence_review_receipt_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SequentialEvidenceReceipt {
    pub schema_version: String,
    pub recording_id: String,
    pub channel_id: String,
    pub sample_index: u64,
    pub time_seconds: f64,
    pub outcome_id: String,
    pub evidence_unit_id: String,
    pub block_start_seconds: f64,
    pub block_end_seconds: f64,
    pub block_manifest_sha256: String,
    pub null_hypothesis_id: String,
    pub alternative_hypothesis_id: String,
    pub model_sha256: String,
    pub likelihood_table_sha256: String,
    pub calibration_sha256: String,
    pub calibration_review_receipt_sha256: String,
    pub block_aggregation_protocol_sha256: String,
    pub dependence_review_receipt_sha256: String,
    pub dependence_review_verified_upstream: bool,
    pub input_receipt_sha256: String,
    pub log_evalue: f64,
    pub log_threshold: f64,
    pub per_alternative_alpha_ppm: u32,
    pub familywise_alpha_ppm: u32,
    pub alternative_count: u32,
    pub threshold_crossed: bool,
    pub previous_receipt_sha256: Option<String>,
    pub receipt_sha256: String,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SequentialEvidenceStatus {
    BelowThreshold,
    ThresholdCrossed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequentialEvidenceError {
    InvalidModel,
    InvalidConfig,
    InvalidProbability,
    InvalidIdentity,
    InvalidDigest,
    DuplicateOutcome(String),
    InvalidLikelihoodNormalization,
    ModelBindingMismatch,
    RecordingChanged,
    NonMonotonicSample,
    SampleSequenceGap,
    InvalidEvidenceBlock,
    DuplicateEvidenceUnit(String),
    OverlappingEvidenceBlock,
    InsufficientBlockSeparation,
    UnknownOutcome(String),
    SessionPoisoned,
    ReceiptChainBreak,
    ReceiptIntegrityFailure,
    MixedModelChain,
    Serialization,
}

impl fmt::Display for SequentialEvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidModel => write!(
                formatter,
                "sequential likelihood model identity or support is invalid"
            ),
            Self::InvalidConfig => {
                write!(formatter, "sequential evidence alpha allocation is invalid")
            }
            Self::InvalidProbability => write!(
                formatter,
                "sequential outcome probability is negative, zero, non-finite or above one"
            ),
            Self::InvalidIdentity => write!(formatter, "sequential evidence identity is empty"),
            Self::InvalidDigest => write!(
                formatter,
                "sequential evidence digest is not lowercase SHA-256"
            ),
            Self::DuplicateOutcome(id) => {
                write!(formatter, "duplicate sequential outcome identity: {id}")
            }
            Self::InvalidLikelihoodNormalization => write!(
                formatter,
                "null and alternative outcome distributions must each sum to one"
            ),
            Self::ModelBindingMismatch => write!(
                formatter,
                "sequential observation belongs to a different model, channel or calibration"
            ),
            Self::RecordingChanged => write!(
                formatter,
                "recording identity changed inside sequential evidence session"
            ),
            Self::NonMonotonicSample => {
                write!(formatter, "sample index or timestamp did not increase")
            }
            Self::SampleSequenceGap => write!(
                formatter,
                "sequential evidence stream skipped one or more block ordinals"
            ),
            Self::InvalidEvidenceBlock => write!(formatter, "sequential evidence unit is not a valid calibrated block"),
            Self::DuplicateEvidenceUnit(id) => write!(formatter, "duplicate sequential evidence unit: {id}"),
            Self::OverlappingEvidenceBlock => write!(formatter, "sequential evidence blocks overlap"),
            Self::InsufficientBlockSeparation => write!(formatter, "sequential evidence blocks violate the frozen minimum separation"),
            Self::UnknownOutcome(id) => write!(
                formatter,
                "observation is outside the frozen likelihood support: {id}"
            ),
            Self::SessionPoisoned => write!(
                formatter,
                "sequential evidence session is poisoned and requires reconciliation"
            ),
            Self::ReceiptChainBreak => write!(
                formatter,
                "sequential evidence receipt chain is incomplete or out of order"
            ),
            Self::ReceiptIntegrityFailure => write!(
                formatter,
                "sequential evidence receipt digest or log-evidence replay failed"
            ),
            Self::MixedModelChain => write!(
                formatter,
                "sequential evidence receipt chain mixes different frozen models"
            ),
            Self::Serialization => write!(formatter, "failed to serialize sequential evidence"),
        }
    }
}

impl std::error::Error for SequentialEvidenceError {}

pub fn sequential_likelihood_table_sha256(
    model: &SequentialLikelihoodModel,
) -> Result<String, SequentialEvidenceError> {
    let payload = (
        model.schema_version.as_str(),
        model.channel_id.as_str(),
        model.null_hypothesis_id.as_str(),
        model.alternative_hypothesis_id.as_str(),
        model.model_sha256.as_str(),
        model.calibration_sha256.as_str(),
        model.calibration_review_receipt_sha256.as_str(),
        model.evidence_unit_kind,
        model.block_aggregation_protocol_sha256.as_str(),
        model.dependence_review_receipt_sha256.as_str(),
        model.dependence_review_verified_upstream,
        model.minimum_block_separation_seconds,
        &model.outcomes,
    );
    sha256_serializable(b"symtropy-sequential-likelihood-table-v1\0", &payload)
}

impl SequentialLikelihoodModel {
    pub fn validate(&self) -> Result<(), SequentialEvidenceError> {
        if self.schema_version != SEQUENTIAL_EVIDENCE_MODEL_SCHEMA
            || self.channel_id.trim().is_empty()
            || self.null_hypothesis_id.trim().is_empty()
            || self.alternative_hypothesis_id.trim().is_empty()
            || self.null_hypothesis_id == self.alternative_hypothesis_id
            || !is_sha256_hex(&self.model_sha256)
            || !is_sha256_hex(&self.calibration_sha256)
            || !is_sha256_hex(&self.calibration_review_receipt_sha256)
            || !is_sha256_hex(&self.block_aggregation_protocol_sha256)
            || !is_sha256_hex(&self.dependence_review_receipt_sha256)
            || !self.dependence_review_verified_upstream
            || !self.minimum_block_separation_seconds.is_finite()
            || self.minimum_block_separation_seconds <= 0.0
            || self.evidence_unit_kind != SequentialEvidenceUnitKind::NonOverlappingCalibratedBlock
            || !is_sha256_hex(&self.likelihood_table_sha256)
            || self.outcomes.len() < 2
        {
            return Err(SequentialEvidenceError::InvalidModel);
        }
        let mut ids = BTreeSet::new();
        let mut null_sum = 0.0;
        let mut alternative_sum = 0.0;
        for outcome in &self.outcomes {
            if outcome.outcome_id.trim().is_empty() {
                return Err(SequentialEvidenceError::InvalidIdentity);
            }
            if !ids.insert(outcome.outcome_id.clone()) {
                return Err(SequentialEvidenceError::DuplicateOutcome(
                    outcome.outcome_id.clone(),
                ));
            }
            for probability in [
                outcome.probability_under_null,
                outcome.probability_under_alternative,
            ] {
                if !probability.is_finite() || probability <= 0.0 || probability > 1.0 {
                    return Err(SequentialEvidenceError::InvalidProbability);
                }
            }
            null_sum += outcome.probability_under_null;
            alternative_sum += outcome.probability_under_alternative;
        }
        if (null_sum - 1.0).abs() > PROBABILITY_TOLERANCE
            || (alternative_sum - 1.0).abs() > PROBABILITY_TOLERANCE
        {
            return Err(SequentialEvidenceError::InvalidLikelihoodNormalization);
        }
        if sequential_likelihood_table_sha256(self)? != self.likelihood_table_sha256 {
            return Err(SequentialEvidenceError::InvalidModel);
        }
        Ok(())
    }

    fn likelihood(&self, outcome_id: &str) -> Option<&SequentialOutcomeLikelihood> {
        self.outcomes
            .iter()
            .find(|outcome| outcome.outcome_id == outcome_id)
    }
}

pub struct SequentialEvidenceSession {
    model: SequentialLikelihoodModel,
    config: SequentialEvidenceConfig,
    recording_id: String,
    previous_sample_index: Option<u64>,
    previous_time_seconds: Option<f64>,
    previous_block_end_seconds: Option<f64>,
    evidence_unit_ids: BTreeSet<String>,
    previous_receipt_sha256: Option<String>,
    log_evalue: f64,
    observation_count: u64,
    threshold_crossed: bool,
    poisoned: bool,
}

impl SequentialEvidenceSession {
    pub fn new(
        model: SequentialLikelihoodModel,
        config: SequentialEvidenceConfig,
        recording_id: impl Into<String>,
    ) -> Result<Self, SequentialEvidenceError> {
        model.validate()?;
        config.per_alternative_alpha_ppm()?;
        let recording_id = recording_id.into();
        if recording_id.trim().is_empty() {
            return Err(SequentialEvidenceError::InvalidIdentity);
        }
        Ok(Self {
            model,
            config,
            recording_id,
            previous_sample_index: None,
            previous_time_seconds: None,
            previous_block_end_seconds: None,
            evidence_unit_ids: BTreeSet::new(),
            previous_receipt_sha256: None,
            log_evalue: 0.0,
            observation_count: 0,
            threshold_crossed: false,
            poisoned: false,
        })
    }

    /// Append one calibrated non-overlapping block outcome. Raw adjacent sensor
    /// samples are not independent evidence units and cannot be appended directly.
    ///
    /// An outcome outside the declared support poisons the session. It is not
    /// assigned infinite evidence, because that would turn model misspecification
    /// into a spurious certainty claim.
    pub fn append(
        &mut self,
        observation: &SequentialEvidenceObservation,
    ) -> Result<SequentialEvidenceReceipt, SequentialEvidenceError> {
        if self.poisoned {
            return Err(SequentialEvidenceError::SessionPoisoned);
        }
        if observation.recording_id.trim().is_empty() {
            return Err(SequentialEvidenceError::InvalidIdentity);
        }
        if observation.recording_id != self.recording_id {
            return Err(SequentialEvidenceError::RecordingChanged);
        }
        if observation.evidence_unit_id.trim().is_empty()
            || !observation.time_seconds.is_finite()
            || !observation.block_start_seconds.is_finite()
            || !observation.block_end_seconds.is_finite()
            || observation.block_start_seconds < 0.0
            || observation.block_end_seconds <= observation.block_start_seconds
            || (observation.time_seconds - observation.block_end_seconds).abs() > 1e-9
        {
            return Err(SequentialEvidenceError::InvalidEvidenceBlock);
        }
        if !is_sha256_hex(&observation.input_receipt_sha256)
            || !is_sha256_hex(&observation.block_manifest_sha256)
        {
            return Err(SequentialEvidenceError::InvalidDigest);
        }
        if observation.model_sha256 != self.model.model_sha256
            || observation.likelihood_table_sha256 != self.model.likelihood_table_sha256
            || observation.calibration_sha256 != self.model.calibration_sha256
            || observation.block_aggregation_protocol_sha256
                != self.model.block_aggregation_protocol_sha256
            || observation.dependence_review_receipt_sha256
                != self.model.dependence_review_receipt_sha256
        {
            return Err(SequentialEvidenceError::ModelBindingMismatch);
        }
        if self.evidence_unit_ids.contains(&observation.evidence_unit_id) {
            return Err(SequentialEvidenceError::DuplicateEvidenceUnit(
                observation.evidence_unit_id.clone(),
            ));
        }
        if let Some(previous_end) = self.previous_block_end_seconds {
            if observation.block_start_seconds < previous_end - 1e-9 {
                return Err(SequentialEvidenceError::OverlappingEvidenceBlock);
            }
            if observation.block_start_seconds - previous_end
                + 1e-9
                < self.model.minimum_block_separation_seconds
            {
                return Err(SequentialEvidenceError::InsufficientBlockSeparation);
            }
        }
        if let Some(previous) = self.previous_sample_index {
            if observation.sample_index <= previous {
                return Err(SequentialEvidenceError::NonMonotonicSample);
            }
            if observation.sample_index != previous.saturating_add(1) {
                return Err(SequentialEvidenceError::SampleSequenceGap);
            }
        } else if observation.sample_index != 0 {
            return Err(SequentialEvidenceError::SampleSequenceGap);
        }
        if self
            .previous_time_seconds
            .is_some_and(|time| observation.time_seconds <= time)
        {
            return Err(SequentialEvidenceError::NonMonotonicSample);
        }
        let Some(likelihood) = self.model.likelihood(&observation.outcome_id) else {
            self.poisoned = true;
            return Err(SequentialEvidenceError::UnknownOutcome(
                observation.outcome_id.clone(),
            ));
        };

        // E_t = product_i p_1(x_i) / p_0(x_i); accumulating in log space
        // avoids overflow and makes replay deterministic.
        let increment =
            likelihood.probability_under_alternative.ln() - likelihood.probability_under_null.ln();
        let next_log_evalue = self.log_evalue + increment;
        if !increment.is_finite() || !next_log_evalue.is_finite() {
            self.poisoned = true;
            return Err(SequentialEvidenceError::InvalidProbability);
        }
        let log_threshold = self.config.log_evidence_threshold()?;
        let crossed = self.threshold_crossed || next_log_evalue >= log_threshold;
        let mut receipt = SequentialEvidenceReceipt {
            schema_version: SEQUENTIAL_EVIDENCE_RECEIPT_SCHEMA.to_owned(),
            recording_id: self.recording_id.clone(),
            channel_id: self.model.channel_id.clone(),
            sample_index: observation.sample_index,
            time_seconds: observation.time_seconds,
            outcome_id: observation.outcome_id.clone(),
            evidence_unit_id: observation.evidence_unit_id.clone(),
            block_start_seconds: observation.block_start_seconds,
            block_end_seconds: observation.block_end_seconds,
            block_manifest_sha256: observation.block_manifest_sha256.clone(),
            null_hypothesis_id: self.model.null_hypothesis_id.clone(),
            alternative_hypothesis_id: self.model.alternative_hypothesis_id.clone(),
            model_sha256: self.model.model_sha256.clone(),
            likelihood_table_sha256: self.model.likelihood_table_sha256.clone(),
            calibration_sha256: self.model.calibration_sha256.clone(),
            calibration_review_receipt_sha256: self.model.calibration_review_receipt_sha256.clone(),
            block_aggregation_protocol_sha256: self.model.block_aggregation_protocol_sha256.clone(),
            dependence_review_receipt_sha256: self.model.dependence_review_receipt_sha256.clone(),
            dependence_review_verified_upstream: self.model.dependence_review_verified_upstream,
            input_receipt_sha256: observation.input_receipt_sha256.clone(),
            log_evalue: next_log_evalue,
            log_threshold,
            per_alternative_alpha_ppm: self.config.per_alternative_alpha_ppm()?,
            familywise_alpha_ppm: self.config.familywise_alpha_ppm,
            alternative_count: self.config.alternative_count,
            threshold_crossed: crossed,
            previous_receipt_sha256: self.previous_receipt_sha256.clone(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = sequential_receipt_sha256(&receipt)?;
        self.log_evalue = next_log_evalue;
        self.previous_sample_index = Some(observation.sample_index);
        self.previous_time_seconds = Some(observation.time_seconds);
        self.previous_block_end_seconds = Some(observation.block_end_seconds);
        self.evidence_unit_ids.insert(observation.evidence_unit_id.clone());
        self.previous_receipt_sha256 = Some(receipt.receipt_sha256.clone());
        self.observation_count += 1;
        self.threshold_crossed = crossed;
        Ok(receipt)
    }

    pub fn status(&self) -> SequentialEvidenceStatus {
        if self.threshold_crossed {
            SequentialEvidenceStatus::ThresholdCrossed
        } else {
            SequentialEvidenceStatus::BelowThreshold
        }
    }

    pub fn log_evalue(&self) -> f64 {
        self.log_evalue
    }

    pub fn observation_count(&self) -> u64 {
        self.observation_count
    }

    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    pub fn model(&self) -> &SequentialLikelihoodModel {
        &self.model
    }

    pub fn config(&self) -> &SequentialEvidenceConfig {
        &self.config
    }
}

/// Replay every likelihood increment and receipt parent under the same frozen
/// model/configuration. This verifies integrity, not the statistical truth of
/// the likelihood model or conditional-independence assumption.
pub fn validate_sequential_evidence_chain(
    model: &SequentialLikelihoodModel,
    config: &SequentialEvidenceConfig,
    receipts: &[SequentialEvidenceReceipt],
) -> Result<(), SequentialEvidenceError> {
    model.validate()?;
    config.per_alternative_alpha_ppm()?;
    if receipts.is_empty() {
        return Err(SequentialEvidenceError::ReceiptChainBreak);
    }
    let expected_threshold = config.log_evidence_threshold()?;
    let expected_alpha = config.per_alternative_alpha_ppm()?;
    let recording_id = receipts[0].recording_id.as_str();
    let mut previous_hash: Option<&str> = None;
    let mut previous_time: Option<f64> = None;
    let mut previous_block_end: Option<f64> = None;
    let mut evidence_unit_ids = BTreeSet::new();
    let mut log_evalue = 0.0;
    let mut threshold_crossed = false;

    for (index, receipt) in receipts.iter().enumerate() {
        if receipt.schema_version != SEQUENTIAL_EVIDENCE_RECEIPT_SCHEMA
            || receipt.recording_id != recording_id
            || receipt.recording_id.trim().is_empty()
            || receipt.evidence_unit_id.trim().is_empty()
            || receipt.block_start_seconds < 0.0
            || !receipt.block_start_seconds.is_finite()
            || !receipt.block_end_seconds.is_finite()
            || receipt.block_end_seconds <= receipt.block_start_seconds
            || (receipt.time_seconds - receipt.block_end_seconds).abs() > 1e-9
            || receipt.channel_id != model.channel_id
            || receipt.null_hypothesis_id != model.null_hypothesis_id
            || receipt.alternative_hypothesis_id != model.alternative_hypothesis_id
        {
            return Err(SequentialEvidenceError::ReceiptChainBreak);
        }
        if receipt.model_sha256 != model.model_sha256
            || receipt.likelihood_table_sha256 != model.likelihood_table_sha256
            || receipt.calibration_sha256 != model.calibration_sha256
            || receipt.calibration_review_receipt_sha256 != model.calibration_review_receipt_sha256
            || receipt.block_aggregation_protocol_sha256 != model.block_aggregation_protocol_sha256
            || receipt.dependence_review_receipt_sha256 != model.dependence_review_receipt_sha256
            || !model.dependence_review_verified_upstream
            || !receipt.dependence_review_verified_upstream
        {
            return Err(SequentialEvidenceError::MixedModelChain);
        }
        if !is_sha256_hex(&receipt.input_receipt_sha256)
            || !is_sha256_hex(&receipt.block_manifest_sha256)
            || !is_sha256_hex(&receipt.receipt_sha256)
            || !receipt.time_seconds.is_finite()
            || receipt.time_seconds < 0.0
        {
            return Err(SequentialEvidenceError::ReceiptChainBreak);
        }
        if receipt.sample_index != index as u64 {
            return Err(SequentialEvidenceError::SampleSequenceGap);
        }
        if previous_time.is_some_and(|time| receipt.time_seconds <= time) {
            return Err(SequentialEvidenceError::NonMonotonicSample);
        }
        if receipt.previous_receipt_sha256.as_deref() != previous_hash {
            return Err(SequentialEvidenceError::ReceiptChainBreak);
        }
        if !evidence_unit_ids.insert(receipt.evidence_unit_id.as_str()) {
            return Err(SequentialEvidenceError::DuplicateEvidenceUnit(
                receipt.evidence_unit_id.clone(),
            ));
        }
        if let Some(previous_end) = previous_block_end {
            if receipt.block_start_seconds < previous_end - 1e-9 {
                return Err(SequentialEvidenceError::OverlappingEvidenceBlock);
            }
            if receipt.block_start_seconds - previous_end + 1e-9
                < model.minimum_block_separation_seconds
            {
                return Err(SequentialEvidenceError::InsufficientBlockSeparation);
            }
        }
        if (receipt.log_threshold - expected_threshold).abs() > 1e-12
            || receipt.per_alternative_alpha_ppm != expected_alpha
            || receipt.familywise_alpha_ppm != config.familywise_alpha_ppm
            || receipt.alternative_count != config.alternative_count
        {
            return Err(SequentialEvidenceError::ReceiptIntegrityFailure);
        }
        let Some(likelihood) = model.likelihood(&receipt.outcome_id) else {
            return Err(SequentialEvidenceError::UnknownOutcome(
                receipt.outcome_id.clone(),
            ));
        };
        log_evalue +=
            likelihood.probability_under_alternative.ln() - likelihood.probability_under_null.ln();
        threshold_crossed |= log_evalue >= expected_threshold;
        if (receipt.log_evalue - log_evalue).abs() > 1e-10
            || receipt.threshold_crossed != threshold_crossed
            || sequential_receipt_sha256(receipt)? != receipt.receipt_sha256
        {
            return Err(SequentialEvidenceError::ReceiptIntegrityFailure);
        }
        previous_hash = Some(receipt.receipt_sha256.as_str());
        previous_time = Some(receipt.time_seconds);
        previous_block_end = Some(receipt.block_end_seconds);
    }
    Ok(())
}

#[derive(Serialize)]
struct SequentialEvidenceReceiptDigest<'a> {
    schema_version: &'a str,
    recording_id: &'a str,
    channel_id: &'a str,
    sample_index: u64,
    time_seconds: f64,
    outcome_id: &'a str,
    evidence_unit_id: &'a str,
    block_start_seconds: f64,
    block_end_seconds: f64,
    block_manifest_sha256: &'a str,
    null_hypothesis_id: &'a str,
    alternative_hypothesis_id: &'a str,
    model_sha256: &'a str,
    likelihood_table_sha256: &'a str,
    calibration_sha256: &'a str,
    calibration_review_receipt_sha256: &'a str,
    block_aggregation_protocol_sha256: &'a str,
    dependence_review_receipt_sha256: &'a str,
    dependence_review_verified_upstream: bool,
    input_receipt_sha256: &'a str,
    log_evalue: f64,
    log_threshold: f64,
    per_alternative_alpha_ppm: u32,
    familywise_alpha_ppm: u32,
    alternative_count: u32,
    threshold_crossed: bool,
    previous_receipt_sha256: &'a Option<String>,
}

fn sequential_receipt_sha256(
    receipt: &SequentialEvidenceReceipt,
) -> Result<String, SequentialEvidenceError> {
    let payload = SequentialEvidenceReceiptDigest {
        schema_version: &receipt.schema_version,
        recording_id: &receipt.recording_id,
        channel_id: &receipt.channel_id,
        sample_index: receipt.sample_index,
        time_seconds: receipt.time_seconds,
        outcome_id: &receipt.outcome_id,
        evidence_unit_id: &receipt.evidence_unit_id,
        block_start_seconds: receipt.block_start_seconds,
        block_end_seconds: receipt.block_end_seconds,
        block_manifest_sha256: &receipt.block_manifest_sha256,
        null_hypothesis_id: &receipt.null_hypothesis_id,
        alternative_hypothesis_id: &receipt.alternative_hypothesis_id,
        model_sha256: &receipt.model_sha256,
        likelihood_table_sha256: &receipt.likelihood_table_sha256,
        calibration_sha256: &receipt.calibration_sha256,
        calibration_review_receipt_sha256: &receipt.calibration_review_receipt_sha256,
        block_aggregation_protocol_sha256: &receipt.block_aggregation_protocol_sha256,
        dependence_review_receipt_sha256: &receipt.dependence_review_receipt_sha256,
        dependence_review_verified_upstream: receipt.dependence_review_verified_upstream,
        input_receipt_sha256: &receipt.input_receipt_sha256,
        log_evalue: receipt.log_evalue,
        log_threshold: receipt.log_threshold,
        per_alternative_alpha_ppm: receipt.per_alternative_alpha_ppm,
        familywise_alpha_ppm: receipt.familywise_alpha_ppm,
        alternative_count: receipt.alternative_count,
        threshold_crossed: receipt.threshold_crossed,
        previous_receipt_sha256: &receipt.previous_receipt_sha256,
    };
    sha256_serializable(b"symtropy-sequential-evidence-receipt-v1\0", &payload)
}

fn sha256_serializable<T: Serialize>(
    domain: &[u8],
    value: &T,
) -> Result<String, SequentialEvidenceError> {
    let encoded = serde_json::to_vec(value).map_err(|_| SequentialEvidenceError::Serialization)?;
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

    fn model() -> SequentialLikelihoodModel {
        let mut model = SequentialLikelihoodModel {
            schema_version: SEQUENTIAL_EVIDENCE_MODEL_SCHEMA.into(),
            channel_id: "intercooler_pressure_signature".into(),
            null_hypothesis_id: "no_fault".into(),
            alternative_hypothesis_id: "intercooler_pressure_sensor_fault".into(),
            model_sha256: digest('a'),
            calibration_sha256: digest('b'),
            calibration_review_receipt_sha256: digest('c'),
            evidence_unit_kind: SequentialEvidenceUnitKind::NonOverlappingCalibratedBlock,
            block_aggregation_protocol_sha256: digest('f'),
            dependence_review_receipt_sha256: digest('e'),
            dependence_review_verified_upstream: true,
            minimum_block_separation_seconds: 10.0,
            outcomes: vec![
                SequentialOutcomeLikelihood {
                    outcome_id: "nominal_signature".into(),
                    probability_under_null: 0.9,
                    probability_under_alternative: 0.2,
                },
                SequentialOutcomeLikelihood {
                    outcome_id: "fault_signature".into(),
                    probability_under_null: 0.1,
                    probability_under_alternative: 0.8,
                },
            ],
            likelihood_table_sha256: digest('0'),
        };
        model.likelihood_table_sha256 = sequential_likelihood_table_sha256(&model).unwrap();
        model
    }

    fn config() -> SequentialEvidenceConfig {
        SequentialEvidenceConfig {
            familywise_alpha_ppm: 1_000,
            alternative_count: 4,
        }
    }

    fn observation(index: u64, outcome_id: &str) -> SequentialEvidenceObservation {
        let model = model();
        SequentialEvidenceObservation {
            recording_id: "run-1".into(),
            sample_index: index,
            time_seconds: index as f64 * 20.0 + 10.0,
            outcome_id: outcome_id.into(),
            evidence_unit_id: format!("run-1-block-{index:03}"),
            block_start_seconds: index as f64 * 20.0,
            block_end_seconds: index as f64 * 20.0 + 10.0,
            block_manifest_sha256: digest('9'),
            input_receipt_sha256: digest('d'),
            model_sha256: model.model_sha256,
            likelihood_table_sha256: model.likelihood_table_sha256,
            calibration_sha256: model.calibration_sha256,
            block_aggregation_protocol_sha256: digest('f'),
            dependence_review_receipt_sha256: digest('e'),
        }
    }

    #[test]
    fn repeated_alternative_evidence_crosses_anytime_threshold_and_replays() {
        let model = model();
        let mut session = SequentialEvidenceSession::new(model.clone(), config(), "run-1").unwrap();
        let mut receipts = Vec::new();
        for index in 0..8 {
            receipts.push(
                session
                    .append(&observation(index, "fault_signature"))
                    .unwrap(),
            );
        }
        assert_eq!(session.status(), SequentialEvidenceStatus::ThresholdCrossed);
        assert!(session.log_evalue() >= config().log_evidence_threshold().unwrap());
        assert_eq!(
            receipts.last().unwrap().alternative_hypothesis_id,
            model.alternative_hypothesis_id
        );
        assert!(validate_sequential_evidence_chain(&model, &config(), &receipts).is_ok());
    }

    #[test]
    fn null_consistent_evidence_does_not_cross_the_fault_threshold() {
        let mut session = SequentialEvidenceSession::new(model(), config(), "run-1").unwrap();
        for index in 0..30 {
            session
                .append(&observation(index, "nominal_signature"))
                .unwrap();
        }
        assert_eq!(session.status(), SequentialEvidenceStatus::BelowThreshold);
        assert!(session.log_evalue() < 0.0);
    }

    #[test]
    fn familywise_alpha_is_conservatively_allocated_across_alternatives() {
        let allocation = config().per_alternative_alpha_ppm().unwrap();
        assert_eq!(allocation, 250);
        assert!(config().log_evidence_threshold().unwrap() > (1_000.0_f64).ln());
        assert_eq!(
            SequentialEvidenceConfig {
                familywise_alpha_ppm: 3,
                alternative_count: 4,
            }
            .per_alternative_alpha_ppm(),
            Err(SequentialEvidenceError::InvalidConfig)
        );
    }

    #[test]
    fn unsupported_outcome_poisons_instead_of_creating_infinite_evidence() {
        let mut session = SequentialEvidenceSession::new(model(), config(), "run-1").unwrap();
        let unsupported = observation(0, "unmodelled_pressure_pattern");
        assert_eq!(
            session.append(&unsupported),
            Err(SequentialEvidenceError::UnknownOutcome(
                "unmodelled_pressure_pattern".into()
            ))
        );
        assert!(session.is_poisoned());
        assert_eq!(
            session.append(&observation(0, "fault_signature")),
            Err(SequentialEvidenceError::SessionPoisoned)
        );
    }

    #[test]
    fn sequence_gaps_and_foreign_calibration_models_are_rejected() {
        let mut session = SequentialEvidenceSession::new(model(), config(), "run-1").unwrap();
        assert_eq!(
            session.append(&observation(1, "fault_signature")),
            Err(SequentialEvidenceError::SampleSequenceGap)
        );
        let mut foreign = observation(0, "fault_signature");
        foreign.calibration_sha256 = digest('e');
        assert_eq!(
            session.append(&foreign),
            Err(SequentialEvidenceError::ModelBindingMismatch)
        );
    }

    #[test]
    fn model_likelihood_and_receipt_tampering_fail_before_acceptance() {
        let mut invalid = model();
        invalid.outcomes[0].probability_under_null = 0.95;
        invalid.outcomes[1].probability_under_null = 0.05;
        assert_eq!(
            invalid.validate(),
            Err(SequentialEvidenceError::InvalidModel)
        );

        let model = model();
        let mut session = SequentialEvidenceSession::new(model.clone(), config(), "run-1").unwrap();
        let first = session.append(&observation(0, "fault_signature")).unwrap();
        let mut second = session
            .append(&observation(1, "nominal_signature"))
            .unwrap();
        second.log_evalue += 0.1;
        assert_eq!(
            validate_sequential_evidence_chain(&model, &config(), &[first, second]),
            Err(SequentialEvidenceError::ReceiptIntegrityFailure)
        );
    }

    #[test]
    fn unverified_dependence_review_cannot_authorize_sequential_evidence() {
        let mut model = model();
        model.dependence_review_verified_upstream = false;
        model.likelihood_table_sha256 = sequential_likelihood_table_sha256(&model).unwrap();
        assert_eq!(model.validate(), Err(SequentialEvidenceError::InvalidModel));
        assert!(matches!(
            SequentialEvidenceSession::new(model, config(), "run-1"),
            Err(SequentialEvidenceError::InvalidModel)
        ));
    }

    #[test]
    fn adjacent_sample_windows_are_not_counted_as_independent_evidence_blocks() {
        let model = model();
        let mut session = SequentialEvidenceSession::new(model, config(), "run-1").unwrap();
        session
            .append(&observation(0, "fault_signature"))
            .unwrap();

        let mut adjacent = observation(1, "fault_signature");
        adjacent.block_start_seconds = 10.05;
        adjacent.block_end_seconds = 10.10;
        adjacent.time_seconds = 10.10;
        assert_eq!(
            session.append(&adjacent),
            Err(SequentialEvidenceError::InsufficientBlockSeparation)
        );
        assert_eq!(session.observation_count(), 1);
    }

    #[test]
    fn overlapping_blocks_and_reused_independent_unit_ids_are_rejected() {
        let model = model();
        let mut overlapping_session =
            SequentialEvidenceSession::new(model.clone(), config(), "run-1").unwrap();
        overlapping_session
            .append(&observation(0, "fault_signature"))
            .unwrap();
        let mut overlap = observation(1, "fault_signature");
        overlap.block_start_seconds = 9.0;
        overlap.block_end_seconds = 19.0;
        overlap.time_seconds = 19.0;
        assert_eq!(
            overlapping_session.append(&overlap),
            Err(SequentialEvidenceError::OverlappingEvidenceBlock)
        );

        let mut duplicate_session =
            SequentialEvidenceSession::new(model, config(), "run-1").unwrap();
        duplicate_session
            .append(&observation(0, "fault_signature"))
            .unwrap();
        let mut duplicate = observation(1, "fault_signature");
        duplicate.evidence_unit_id = "run-1-block-000".into();
        assert_eq!(
            duplicate_session.append(&duplicate),
            Err(SequentialEvidenceError::DuplicateEvidenceUnit(
                "run-1-block-000".into()
            ))
        );
    }

    #[test]
    fn sequential_protocol_binds_calibrated_non_overlapping_block_semantics() {
        let protocol: serde_json::Value =
            serde_json::from_str(include_str!("../sequential-evidence-v0.2.json")).unwrap();
        assert_eq!(
            protocol["schema_version"],
            "symtropy-sequential-evidence-v0.2"
        );
        assert_eq!(protocol["status"], "protocol_only_not_executed");
        assert_eq!(
            protocol["protocol_constraints"]["one_input_one_receipt"],
            true
        );
        assert_eq!(
            protocol["protocol_constraints"]["contiguous_sample_indices"],
            true
        );
        assert_eq!(
            protocol["protocol_constraints"]["model_and_calibration_bound_per_session"],
            true
        );
        assert_eq!(
            protocol["protocol_constraints"]["receipt_chain_replay"],
            true
        );
        assert_eq!(
            protocol["protocol_constraints"]["unknown_outcome_poisons_session"],
            true
        );
        assert!(
            protocol["validity_conditions"]["channel"]
                .as_str()
                .unwrap()
                .contains("One frozen channel")
        );
        assert!(
            protocol["validity_conditions"]["channel"]
                .as_str()
                .unwrap()
                .contains("not supported")
        );
        assert_eq!(protocol["familywise_control"]["method"], "bonferroni");
        assert_eq!(
            protocol["familywise_control"]["default"]["alternative_count"],
            4
        );
        assert!(
            protocol["validity_conditions"]["block_independence"]
                .as_str()
                .unwrap()
                .contains("not inferred from non-overlap alone")
        );
        assert!(
            protocol["validity_conditions"]["optional_stopping"]
                .as_str()
                .unwrap()
                .contains("Ville")
        );
        assert!(
            protocol["claim_ceiling"]
                .as_str()
                .unwrap()
                .contains("dependence assumptions reviewed")
        );
        assert_eq!(
            protocol["model"]["evidence_unit_kind"],
            "non_overlapping_calibrated_block"
        );
        assert_eq!(
            protocol["protocol_constraints"]["no_raw_sample_pseudoreplication"],
            true
        );
        assert_eq!(
            protocol["protocol_constraints"]["minimum_block_separation_required"],
            true
        );
        assert_eq!(
            protocol["protocol_constraints"]["dependence_review_bound_to_model"],
            true
        );
        assert!(protocol["validity_conditions"]["block_independence"]
            .as_str()
            .unwrap()
            .contains("not proof of independence"));
        assert!(
            protocol["research_targets_not_achieved"]
                .as_array()
                .unwrap()
                .iter()
                .all(|target| target["status"] == "aspirational")
        );
    }
}
