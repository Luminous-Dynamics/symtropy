#![forbid(unsafe_code)]

//! Append-only, sample-at-a-time diagnosis contract for LiU-ICE.
//!
//! The benchmark interface supplies one new observation per call and does not
//! permit rewriting an earlier output. This module enforces monotonic input,
//! one validated five-slot diagnosis per accepted sample, a model-bound receipt
//! chain, and fail-closed poisoning after an uncertain model-side mutation.
//!
//! It is a model-agnostic contract. It does not train a diagnosis model, provide
//! a competition-ready Python/MATLAB adapter, or claim benchmark performance.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;

use crate::temporal::{BoundedEventTimeInterval, TemporalIntervalError};

pub const STREAMING_ISOLATION_SLOTS: usize = 5;
pub const STREAMING_RECEIPT_SCHEMA: &str = "liu-ice-streaming-diagnosis-receipt-v1";
const PROBABILITY_TOLERANCE: f64 = 1e-6;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StreamingInputSample {
    pub recording_id: String,
    pub sample_index: u64,
    pub time_seconds: f64,
    /// Half-width of the event-time interval in seconds.
    pub timestamp_uncertainty_seconds: f64,
    /// Source clock domain after mapping into the recording's declared timebase.
    pub timebase_id: String,
    /// True only when an upstream timebase qualification is available.
    pub timebase_qualified: bool,
    /// Digest of the timebase qualification evidence; hash integrity is not signer authentication.
    pub timebase_qualification_sha256: String,
    /// Missing channels remain absent. No nominal-value imputation is performed.
    pub signals: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StreamingDiagnosisOutput {
    pub recording_id: String,
    pub sample_index: u64,
    pub detection: bool,
    /// LiU-ICE order: f_pic, f_pim, f_waf, f_iml, f_x.
    pub isolation_probabilities: [f64; STREAMING_ISOLATION_SLOTS],
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StreamingDiagnosisReceipt {
    pub schema_version: String,
    pub recording_id: String,
    pub sample_index: u64,
    pub time_seconds: f64,
    pub timestamp_uncertainty_seconds: f64,
    pub timebase_id: String,
    pub timebase_qualified: bool,
    pub timebase_qualification_sha256: String,
    pub model_sha256: String,
    pub input_sha256: String,
    pub output: StreamingDiagnosisOutput,
    pub output_sha256: String,
    pub previous_receipt_sha256: Option<String>,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamingError {
    EmptyIdentity,
    InvalidModelDigest,
    InvalidInputSample,
    InvalidTimebase,
    TimebaseChanged,
    TemporalOrderUnproven,
    RecordingChanged,
    NonMonotonicSample,
    SampleSequenceGap,
    SessionPoisoned,
    ModelRejectedInput(String),
    OutputIdentityMismatch,
    InvalidOutputProbability,
    ReceiptIntegrityFailure,
    ReceiptChainBreak,
    MixedModelChain,
    Serialization,
}

impl fmt::Display for StreamingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentity => write!(formatter, "recording or signal identity is empty"),
            Self::InvalidModelDigest => write!(
                formatter,
                "model identity is not a lowercase SHA-256 digest"
            ),
            Self::InvalidInputSample => write!(
                formatter,
                "streaming input sample has invalid time or signal values"
            ),
            Self::InvalidTimebase => write!(
                formatter,
                "input requires a qualified timebase identity and SHA-256 evidence digest"
            ),
            Self::TimebaseChanged => write!(
                formatter,
                "timebase identity or qualification changed inside a streaming session"
            ),
            Self::TemporalOrderUnproven => write!(
                formatter,
                "timestamp uncertainty intervals overlap; strict event-time order is unproven"
            ),
            Self::RecordingChanged => write!(
                formatter,
                "recording identity changed inside a streaming session"
            ),
            Self::NonMonotonicSample => {
                write!(formatter, "sample index or timestamp did not increase")
            }
            Self::SampleSequenceGap => write!(
                formatter,
                "streaming sample index skipped one or more samples"
            ),
            Self::SessionPoisoned => write!(
                formatter,
                "streaming session is poisoned and must be reconciled or restarted"
            ),
            Self::ModelRejectedInput(message) => {
                write!(formatter, "diagnoser rejected input after call: {message}")
            }
            Self::OutputIdentityMismatch => write!(
                formatter,
                "diagnosis output identity does not match the input sample"
            ),
            Self::InvalidOutputProbability => write!(
                formatter,
                "diagnosis output violates the five-slot probability contract"
            ),
            Self::ReceiptIntegrityFailure => write!(
                formatter,
                "streaming receipt digest does not match its contents"
            ),
            Self::ReceiptChainBreak => write!(
                formatter,
                "streaming receipt chain is incomplete or out of order"
            ),
            Self::MixedModelChain => write!(
                formatter,
                "streaming receipt chain contains multiple model identities"
            ),
            Self::Serialization => write!(
                formatter,
                "failed to serialize canonical streaming evidence"
            ),
        }
    }
}

impl std::error::Error for StreamingError {}

/// One-sample interface: implementations receive the newly arrived sample
/// and return only its current diagnosis. There is intentionally no API for
/// rewriting an earlier output or receiving future samples.
pub trait StreamingDiagnoser {
    fn input(&mut self, sample: &StreamingInputSample) -> Result<StreamingDiagnosisOutput, String>;
}

/// A session binds one model and one recording. Any uncertain failure after
/// calling the stateful model poisons the session; no new input is accepted
/// until the caller reconciles the state and starts a new session.
pub struct StreamingDiagnosisSession<D> {
    diagnoser: D,
    recording_id: String,
    model_sha256: String,
    previous_sample_index: Option<u64>,
    previous_time_interval: Option<BoundedEventTimeInterval>,
    timebase_id: Option<String>,
    timebase_qualification_sha256: Option<String>,
    previous_receipt_sha256: Option<String>,
    sample_count: u64,
    poisoned: bool,
}

impl<D: StreamingDiagnoser> StreamingDiagnosisSession<D> {
    pub fn new(
        diagnoser: D,
        recording_id: impl Into<String>,
        model_sha256: impl Into<String>,
    ) -> Result<Self, StreamingError> {
        let recording_id = recording_id.into();
        let model_sha256 = model_sha256.into();
        if recording_id.trim().is_empty() {
            return Err(StreamingError::EmptyIdentity);
        }
        if !is_sha256_hex(&model_sha256) {
            return Err(StreamingError::InvalidModelDigest);
        }
        Ok(Self {
            diagnoser,
            recording_id,
            model_sha256,
            previous_sample_index: None,
            previous_time_interval: None,
            timebase_id: None,
            timebase_qualification_sha256: None,
            previous_receipt_sha256: None,
            sample_count: 0,
            poisoned: false,
        })
    }

    /// Accept exactly one new sample and return exactly one immutable receipt.
    pub fn input(
        &mut self,
        sample: &StreamingInputSample,
    ) -> Result<StreamingDiagnosisReceipt, StreamingError> {
        if self.poisoned {
            return Err(StreamingError::SessionPoisoned);
        }
        let time_interval = validate_input_sample(sample)?;
        if sample.recording_id != self.recording_id {
            return Err(StreamingError::RecordingChanged);
        }
        if let Some(previous) = self.previous_sample_index {
            if sample.sample_index <= previous {
                return Err(StreamingError::NonMonotonicSample);
            }
            if sample.sample_index != previous.saturating_add(1) {
                return Err(StreamingError::SampleSequenceGap);
            }
        }
        if let (Some(timebase_id), Some(qualification_sha256)) =
            (&self.timebase_id, &self.timebase_qualification_sha256)
        {
            if sample.timebase_id.as_str() != timebase_id.as_str()
                || sample.timebase_qualification_sha256.as_str() != qualification_sha256.as_str()
            {
                return Err(StreamingError::TimebaseChanged);
            }
        }
        if let Some(previous) = self.previous_time_interval {
            if time_interval.is_strictly_before(previous) {
                return Err(StreamingError::NonMonotonicSample);
            }
            if !time_interval.is_strictly_after(previous) {
                return Err(StreamingError::TemporalOrderUnproven);
            }
        }

        let input_sha256 = sha256_serializable(b"symtropy-liu-ice-streaming-input-v1\0", sample)?;

        // A model error may have mutated hidden state; poison rather than retry
        // or silently accept a new sample against an unknown internal state.
        let output = match self.diagnoser.input(sample) {
            Ok(output) => output,
            Err(message) => {
                self.poisoned = true;
                return Err(StreamingError::ModelRejectedInput(message));
            }
        };
        if output.recording_id != sample.recording_id || output.sample_index != sample.sample_index
        {
            self.poisoned = true;
            return Err(StreamingError::OutputIdentityMismatch);
        }
        if validate_output(&output).is_err() {
            self.poisoned = true;
            return Err(StreamingError::InvalidOutputProbability);
        }

        let output_sha256 =
            match sha256_serializable(b"symtropy-liu-ice-streaming-output-v1\0", &output) {
                Ok(digest) => digest,
                Err(error) => {
                    self.poisoned = true;
                    return Err(error);
                }
            };
        let mut receipt = StreamingDiagnosisReceipt {
            schema_version: STREAMING_RECEIPT_SCHEMA.to_owned(),
            recording_id: sample.recording_id.clone(),
            sample_index: sample.sample_index,
            time_seconds: sample.time_seconds,
            timestamp_uncertainty_seconds: sample.timestamp_uncertainty_seconds,
            timebase_id: sample.timebase_id.clone(),
            timebase_qualified: sample.timebase_qualified,
            timebase_qualification_sha256: sample.timebase_qualification_sha256.clone(),
            model_sha256: self.model_sha256.clone(),
            input_sha256,
            output,
            output_sha256,
            previous_receipt_sha256: self.previous_receipt_sha256.clone(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = match receipt_digest(&receipt) {
            Ok(digest) => digest,
            Err(error) => {
                self.poisoned = true;
                return Err(error);
            }
        };

        self.previous_sample_index = Some(sample.sample_index);
        self.previous_time_interval = Some(time_interval);
        self.timebase_id = Some(sample.timebase_id.clone());
        self.timebase_qualification_sha256 = Some(sample.timebase_qualification_sha256.clone());
        self.previous_receipt_sha256 = Some(receipt.receipt_sha256.clone());
        self.sample_count += 1;
        Ok(receipt)
    }

    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    /// Access to the implementation is intentionally read-only. It cannot be
    /// replaced in place after a sample has been accepted.
    pub fn model_sha256(&self) -> &str {
        &self.model_sha256
    }
}

/// Verify ordered receipt continuity, model identity, per-output digest and
/// receipt hash. Input sample bytes are not embedded; their source is bound
/// by input_sha256 and must be retained by the upstream archive manifest.
pub fn validate_receipt_chain(
    receipts: &[StreamingDiagnosisReceipt],
) -> Result<(), StreamingError> {
    if receipts.is_empty() {
        return Err(StreamingError::ReceiptChainBreak);
    }
    let model_sha256 = receipts[0].model_sha256.as_str();
    if !is_sha256_hex(model_sha256) {
        return Err(StreamingError::InvalidModelDigest);
    }
    let mut previous_receipt: Option<&str> = None;
    let mut previous_index = None;
    let mut previous_time_interval: Option<BoundedEventTimeInterval> = None;
    let timebase_id = receipts[0].timebase_id.as_str();
    let timebase_qualification_sha256 = receipts[0].timebase_qualification_sha256.as_str();
    let recording_id = receipts[0].recording_id.as_str();

    for receipt in receipts {
        if receipt.schema_version != STREAMING_RECEIPT_SCHEMA
            || receipt.recording_id.trim().is_empty()
            || receipt.recording_id != recording_id
            || !is_sha256_hex(&receipt.input_sha256)
            || !is_sha256_hex(&receipt.output_sha256)
            || !is_sha256_hex(&receipt.receipt_sha256)
            || !receipt.time_seconds.is_finite()
            || receipt.time_seconds < 0.0
            || !receipt.timestamp_uncertainty_seconds.is_finite()
            || receipt.timestamp_uncertainty_seconds < 0.0
            || !receipt.timebase_qualified
            || receipt.timebase_id.trim().is_empty()
            || !is_sha256_hex(&receipt.timebase_qualification_sha256)
        {
            return Err(StreamingError::ReceiptChainBreak);
        }
        if receipt.timebase_id.as_str() != timebase_id
            || receipt.timebase_qualification_sha256.as_str() != timebase_qualification_sha256
        {
            return Err(StreamingError::TimebaseChanged);
        }
        if receipt.model_sha256 != model_sha256 {
            return Err(StreamingError::MixedModelChain);
        }
        if let Some(previous) = previous_index {
            if receipt.sample_index <= previous {
                return Err(StreamingError::NonMonotonicSample);
            }
            if receipt.sample_index != previous.saturating_add(1) {
                return Err(StreamingError::SampleSequenceGap);
            }
        }
        let current_time_interval = BoundedEventTimeInterval::new(
            receipt.time_seconds,
            receipt.timestamp_uncertainty_seconds,
        )
        .map_err(|_| StreamingError::ReceiptChainBreak)?;
        if let Some(previous) = previous_time_interval {
            if current_time_interval.is_strictly_before(previous) {
                return Err(StreamingError::NonMonotonicSample);
            }
            if !current_time_interval.is_strictly_after(previous) {
                return Err(StreamingError::TemporalOrderUnproven);
            }
        }
        if receipt.previous_receipt_sha256.as_deref() != previous_receipt {
            return Err(StreamingError::ReceiptChainBreak);
        }
        if receipt.output.recording_id != receipt.recording_id
            || receipt.output.sample_index != receipt.sample_index
            || validate_output(&receipt.output).is_err()
        {
            return Err(StreamingError::InvalidOutputProbability);
        }
        let expected_output =
            sha256_serializable(b"symtropy-liu-ice-streaming-output-v1\0", &receipt.output)?;
        if expected_output != receipt.output_sha256 {
            return Err(StreamingError::ReceiptIntegrityFailure);
        }
        if receipt_digest(receipt)? != receipt.receipt_sha256 {
            return Err(StreamingError::ReceiptIntegrityFailure);
        }
        previous_index = Some(receipt.sample_index);
        previous_time_interval = Some(current_time_interval);
        previous_receipt = Some(receipt.receipt_sha256.as_str());
    }
    Ok(())
}

fn validate_input_sample(
    sample: &StreamingInputSample,
) -> Result<BoundedEventTimeInterval, StreamingError> {
    if sample.recording_id.trim().is_empty() {
        return Err(StreamingError::EmptyIdentity);
    }
    if !sample.timebase_qualified
        || sample.timebase_id.trim().is_empty()
        || !is_sha256_hex(&sample.timebase_qualification_sha256)
    {
        return Err(StreamingError::InvalidTimebase);
    }
    if sample
        .signals
        .iter()
        .any(|(name, value)| name.trim().is_empty() || !value.is_finite())
    {
        return Err(StreamingError::InvalidInputSample);
    }
    BoundedEventTimeInterval::new(
        sample.time_seconds,
        sample.timestamp_uncertainty_seconds,
    )
    .map_err(|_| StreamingError::InvalidInputSample)
}

fn validate_output(output: &StreamingDiagnosisOutput) -> Result<(), StreamingError> {
    if output.recording_id.trim().is_empty()
        || output
            .isolation_probabilities
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0 || *value > 1.0)
    {
        return Err(StreamingError::InvalidOutputProbability);
    }
    let sum = output.isolation_probabilities.iter().sum::<f64>();
    if output.detection {
        if (sum - 1.0).abs() > PROBABILITY_TOLERANCE {
            return Err(StreamingError::InvalidOutputProbability);
        }
    } else if output
        .isolation_probabilities
        .iter()
        .any(|value| *value != 0.0)
    {
        return Err(StreamingError::InvalidOutputProbability);
    }
    Ok(())
}

fn receipt_digest(receipt: &StreamingDiagnosisReceipt) -> Result<String, StreamingError> {
    let payload = (
        receipt.schema_version.as_str(),
        receipt.recording_id.as_str(),
        receipt.sample_index,
        receipt.time_seconds,
        receipt.timestamp_uncertainty_seconds,
        receipt.timebase_id.as_str(),
        receipt.timebase_qualified,
        receipt.timebase_qualification_sha256.as_str(),
        receipt.model_sha256.as_str(),
        receipt.input_sha256.as_str(),
        &receipt.output,
        receipt.output_sha256.as_str(),
        &receipt.previous_receipt_sha256,
    );
    sha256_serializable(b"symtropy-liu-ice-streaming-receipt-v1\0", &payload)
}

fn sha256_serializable<T: Serialize>(domain: &[u8], value: &T) -> Result<String, StreamingError> {
    let encoded = serde_json::to_vec(value).map_err(|_| StreamingError::Serialization)?;
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

    #[derive(Default)]
    struct SyntheticStreamingDiagnoser {
        calls: u64,
    }

    impl StreamingDiagnoser for SyntheticStreamingDiagnoser {
        fn input(
            &mut self,
            sample: &StreamingInputSample,
        ) -> Result<StreamingDiagnosisOutput, String> {
            self.calls += 1;
            let pressure = sample.signals.get("ypic").copied().unwrap_or(0.0);
            let detection = pressure > 2.0;
            let mut isolation_probabilities = [0.0; STREAMING_ISOLATION_SLOTS];
            if detection {
                isolation_probabilities[0] = 0.9;
                isolation_probabilities[4] = 0.1;
            }
            Ok(StreamingDiagnosisOutput {
                recording_id: sample.recording_id.clone(),
                sample_index: sample.sample_index,
                detection,
                isolation_probabilities,
            })
        }
    }

    fn sample(index: u64, time_seconds: f64) -> StreamingInputSample {
        StreamingInputSample {
            recording_id: "run-1".into(),
            sample_index: index,
            time_seconds,
            timestamp_uncertainty_seconds: 0.0,
            timebase_id: "run-1-clock".into(),
            timebase_qualified: true,
            timebase_qualification_sha256: "e".repeat(64),
            signals: BTreeMap::from([("ypic".into(), 1.0)]),
        }
    }

    fn session() -> StreamingDiagnosisSession<SyntheticStreamingDiagnoser> {
        StreamingDiagnosisSession::new(
            SyntheticStreamingDiagnoser::default(),
            "run-1",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .unwrap()
    }

    #[test]
    fn one_input_produces_one_valid_receipt_and_chain_is_replayable() {
        let mut session = session();
        let first = session.input(&sample(0, 0.0)).unwrap();
        let second = session.input(&sample(1, 0.05)).unwrap();
        assert_eq!(session.sample_count(), 2);
        assert!(!first.output.detection);
        assert_eq!(
            second.previous_receipt_sha256,
            Some(first.receipt_sha256.clone())
        );
        assert!(validate_receipt_chain(&[first, second]).is_ok());
    }

    #[test]
    fn prior_output_is_immutable_when_later_samples_arrive() {
        let mut session = session();
        let first = session.input(&sample(0, 0.0)).unwrap();
        let frozen = first.clone();
        let _second = session.input(&sample(1, 0.05)).unwrap();
        let _third = session.input(&sample(2, 0.10)).unwrap();
        assert_eq!(first, frozen);
        assert_eq!(session.sample_count(), 3);
    }

    #[test]
    fn out_of_order_sample_is_rejected_without_calling_the_model() {
        let mut session = session();
        let _first = session.input(&sample(2, 0.10)).unwrap();
        assert_eq!(
            session.input(&sample(1, 0.05)),
            Err(StreamingError::NonMonotonicSample)
        );
        assert_eq!(session.sample_count(), 1);
        assert!(!session.is_poisoned());
    }

    #[test]
    fn wrong_recording_and_invalid_sensor_data_are_rejected_before_model_call() {
        let mut session = session();
        let mut wrong_recording = sample(0, 0.0);
        wrong_recording.recording_id = "other".into();
        assert_eq!(
            session.input(&wrong_recording),
            Err(StreamingError::RecordingChanged)
        );

        let mut invalid = sample(0, 0.0);
        invalid.signals.insert("ypic".into(), f64::NAN);
        assert_eq!(
            session.input(&invalid),
            Err(StreamingError::InvalidInputSample)
        );
        assert_eq!(session.sample_count(), 0);
        assert!(!session.is_poisoned());
    }

    #[test]
    fn model_output_with_wrong_identity_or_probability_contract_poisons_session() {
        struct InvalidOutput;
        impl StreamingDiagnoser for InvalidOutput {
            fn input(
                &mut self,
                sample: &StreamingInputSample,
            ) -> Result<StreamingDiagnosisOutput, String> {
                Ok(StreamingDiagnosisOutput {
                    recording_id: sample.recording_id.clone(),
                    sample_index: sample.sample_index,
                    detection: false,
                    isolation_probabilities: [0.0, 0.0, 0.0, 0.0, 1.0],
                })
            }
        }
        let mut session = StreamingDiagnosisSession::new(
            InvalidOutput,
            "run-1",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .unwrap();
        assert_eq!(
            session.input(&sample(0, 0.0)),
            Err(StreamingError::InvalidOutputProbability)
        );
        assert!(session.is_poisoned());
        assert_eq!(
            session.input(&sample(1, 0.05)),
            Err(StreamingError::SessionPoisoned)
        );
    }

    #[test]
    fn model_error_poisons_session_and_prevents_ambiguous_retry() {
        struct FailingDiagnoser;
        impl StreamingDiagnoser for FailingDiagnoser {
            fn input(
                &mut self,
                _sample: &StreamingInputSample,
            ) -> Result<StreamingDiagnosisOutput, String> {
                Err("internal transition uncertain".into())
            }
        }
        let mut session = StreamingDiagnosisSession::new(
            FailingDiagnoser,
            "run-1",
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        )
        .unwrap();
        assert_eq!(
            session.input(&sample(0, 0.0)),
            Err(StreamingError::ModelRejectedInput(
                "internal transition uncertain".into()
            ))
        );
        assert!(session.is_poisoned());
        assert_eq!(
            session.input(&sample(1, 0.05)),
            Err(StreamingError::SessionPoisoned)
        );
    }

    #[test]
    fn tampered_output_or_broken_receipt_parent_is_rejected() {
        let mut session = session();
        let first = session.input(&sample(0, 0.0)).unwrap();
        let second = session.input(&sample(1, 0.05)).unwrap();
        assert!(validate_receipt_chain(&[first.clone(), second.clone()]).is_ok());

        let mut tampered_output = second.clone();
        tampered_output.output.detection = true;
        tampered_output.output.isolation_probabilities = [1.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(
            validate_receipt_chain(&[first.clone(), tampered_output]),
            Err(StreamingError::ReceiptIntegrityFailure)
        );

        let mut broken_parent = second;
        broken_parent.previous_receipt_sha256 = None;
        assert_eq!(
            validate_receipt_chain(&[first, broken_parent]),
            Err(StreamingError::ReceiptChainBreak)
        );
    }

    #[test]
    fn overlapping_time_uncertainty_is_rejected_before_a_receipt_is_issued() {
        let mut session = session();
        let mut first = sample(0, 1.0);
        first.timestamp_uncertainty_seconds = 0.02;
        session.input(&first).unwrap();

        let mut overlapping = sample(1, 1.03);
        overlapping.timestamp_uncertainty_seconds = 0.02;
        assert_eq!(
            session.input(&overlapping),
            Err(StreamingError::TemporalOrderUnproven)
        );
        assert_eq!(session.sample_count(), 1);
        assert!(!session.is_poisoned());
    }

    #[test]
    fn timebase_change_is_rejected_before_model_invocation() {
        let mut session = session();
        session.input(&sample(0, 1.0)).unwrap();
        let mut changed = sample(1, 1.05);
        changed.timebase_id = "unmapped-controller-clock".into();
        assert_eq!(session.input(&changed), Err(StreamingError::TimebaseChanged));
        assert_eq!(session.sample_count(), 1);
        assert!(!session.is_poisoned());
    }

    #[test]
    fn unqualified_or_unbound_clock_evidence_is_rejected() {
        let mut session = session();
        let mut unqualified = sample(0, 1.0);
        unqualified.timebase_qualified = false;
        assert_eq!(session.input(&unqualified), Err(StreamingError::InvalidTimebase));

        let mut malformed_digest = sample(0, 1.0);
        malformed_digest.timebase_qualification_sha256 = "not-a-digest".into();
        assert_eq!(session.input(&malformed_digest), Err(StreamingError::InvalidTimebase));
        assert_eq!(session.sample_count(), 0);
    }

    #[test]
    fn receipt_chain_binds_timebase_and_uncertainty_intervals() {
        let mut session = session();
        let mut first = sample(0, 1.0);
        first.timestamp_uncertainty_seconds = 0.01;
        let first = session.input(&first).unwrap();
        let mut second = sample(1, 1.05);
        second.timestamp_uncertainty_seconds = 0.01;
        let second = session.input(&second).unwrap();
        assert!(validate_receipt_chain(&[first.clone(), second.clone()]).is_ok());

        let mut tampered_clock = second;
        tampered_clock.timebase_qualification_sha256 = "f".repeat(64);
        assert_eq!(
            validate_receipt_chain(&[first, tampered_clock]),
            Err(StreamingError::TimebaseChanged)
        );
    }

    #[test]
    fn sample_index_gaps_are_not_silently_accepted() {
        let mut session = session();
        let _first = session.input(&sample(0, 0.0)).unwrap();
        assert_eq!(
            session.input(&sample(2, 0.10)),
            Err(StreamingError::SampleSequenceGap)
        );
        assert_eq!(session.sample_count(), 1);
        assert!(!session.is_poisoned());
    }

    #[test]
    fn streaming_protocol_freezes_no_rewrite_and_python_matlab_adapter_gap() {
        let protocol: serde_json::Value =
            serde_json::from_str(include_str!("../streaming-contract-v0.1.json")).unwrap();
        assert_eq!(
            protocol["schema_version"],
            "liu-ice-streaming-contract-v0.1"
        );
        assert_eq!(protocol["status"], "protocol_only_not_executed");
        assert_eq!(protocol["interface"]["historical_output_mutation"], false);
        assert_eq!(
            protocol["implementation"]["rust_module"],
            "src/streaming_contract.rs"
        );
        assert_eq!(
            protocol["competition_adapter"]["rust_core_direct_submission_compatible"],
            false
        );
        assert_eq!(
            protocol["competition_adapter"]["adapter_status"],
            "not_implemented"
        );
        assert!(
            protocol["competition_adapter"]["official_languages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|language| language == "Python")
        );
        assert!(
            protocol["competition_adapter"]["official_languages"]
                .as_array()
                .unwrap()
                .iter()
                .any(|language| language == "MATLAB")
        );
        assert!(protocol["hard_gates"].as_array().unwrap().len() >= 10);
        assert!(
            protocol["research_targets_not_achieved"]
                .as_array()
                .unwrap()
                .iter()
                .all(|target| target["status"] == "aspirational")
        );
    }

    #[test]
    fn detection_uses_all_five_slots_and_unknown_fault_is_not_silently_dropped() {
        struct UnknownFaultDiagnoser;
        impl StreamingDiagnoser for UnknownFaultDiagnoser {
            fn input(
                &mut self,
                sample: &StreamingInputSample,
            ) -> Result<StreamingDiagnosisOutput, String> {
                Ok(StreamingDiagnosisOutput {
                    recording_id: sample.recording_id.clone(),
                    sample_index: sample.sample_index,
                    detection: true,
                    isolation_probabilities: [0.0, 0.0, 0.0, 0.0, 1.0],
                })
            }
        }
        let mut session = StreamingDiagnosisSession::new(
            UnknownFaultDiagnoser,
            "run-1",
            "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        )
        .unwrap();
        let receipt = session.input(&sample(0, 0.0)).unwrap();
        assert!(receipt.output.detection);
        assert_eq!(receipt.output.isolation_probabilities[4], 1.0);
    }
}
