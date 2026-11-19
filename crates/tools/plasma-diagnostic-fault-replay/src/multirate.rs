//! Timestamp-native, causal alignment for heterogeneous plasma diagnostics.
//!
//! This module preserves each diagnostic's native event times and evidence
//! disposition. It does not resample, interpolate, reconstruct, or authorize
//! physical control. A receipt qualifies alignment integrity only.

use super::{ReplayError, digest_serialized, non_empty};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MULTIRATE_SCHEMA_VERSION: &str = "luminous.plasma-multirate-alignment.v0.1";
pub const MULTIRATE_BENCHMARK_SCHEMA_VERSION: &str =
    "luminous.plasma-multirate-alignment-benchmark.v0.1";
pub const ALIGNMENT_EVIDENCE_CLASS: &str = "SOFTWARE_OBSERVATION_ALIGNMENT";
pub const ALIGNMENT_CLAIM_CEILING: &str =
    "TIMESTAMP_AND_PROVENANCE_INTEGRITY_ONLY_NO_PHYSICAL_VALIDITY_CLAIM";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    Measured,
    Synthetic,
    Reconstructed,
    Unavailable,
    Invalid,
    Delayed,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationLineage {
    pub source_id: String,
    pub source_sha256: String,
    pub calibration_id: Option<String>,
    pub calibration_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationSample {
    pub sample_id: String,
    pub timestamp_ns: u64,
    pub value: Option<f64>,
    pub evidence_state: EvidenceState,
    pub lineage: Option<ObservationLineage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationStream {
    pub stream_id: String,
    pub channel_id: String,
    pub diagnostic_system_id: String,
    pub units: String,
    pub clock_domain: String,
    pub native_sample_period_ns: Option<u64>,
    pub samples: Vec<ObservationSample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlignmentRequest {
    pub schema_version: String,
    pub scenario_id: String,
    pub clock_domain: String,
    pub decision_time_ns: u64,
    pub required_channels: Vec<String>,
    pub max_age_ns_by_channel: BTreeMap<String, u64>,
    pub streams: Vec<ObservationStream>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentDisposition {
    MeasuredAvailable,
    SyntheticOnly,
    ReconstructedOnly,
    Unavailable,
    Invalid,
    Delayed,
    Degraded,
    Stale,
    NoPastSample,
    MissingStream,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlignmentEntry {
    pub stream_id: Option<String>,
    pub channel_id: String,
    pub required: bool,
    pub disposition: AlignmentDisposition,
    pub selected_sample_id: Option<String>,
    pub selected_timestamp_ns: Option<u64>,
    pub age_ns: Option<u64>,
    pub native_sample_period_ns: Option<u64>,
    pub value: Option<f64>,
    pub evidence_state: Option<EvidenceState>,
    pub lineage: Option<ObservationLineage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlignmentReceiptBody {
    pub schema_version: String,
    pub scenario_id: String,
    pub clock_domain: String,
    pub decision_time_ns: u64,
    pub evidence_class: String,
    pub physical_measurements: bool,
    pub claim_ceiling: String,
    pub request_sha256: String,
    pub source_streams_sha256: String,
    pub future_samples_excluded: usize,
    pub entries: Vec<AlignmentEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlignmentReceipt {
    pub body: AlignmentReceiptBody,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MultiRateCaseAudit {
    pub case_id: String,
    pub actual_result: String,
    pub expected_result: String,
    pub actual_entry_dispositions: Vec<String>,
    pub matched_expectation: bool,
    pub error: Option<String>,
    pub receipt_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MultiRateAuditReport {
    pub benchmark_id: String,
    pub case_count: usize,
    pub matched_expectations: usize,
    pub mismatched_expectations: usize,
    pub results: Vec<MultiRateCaseAudit>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MultiRateBenchmarkManifest {
    schema_version: String,
    benchmark_id: String,
    evidence_class: String,
    physical_measurements: bool,
    claim_ceiling: String,
    cases: Vec<MultiRateFixtureCase>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct MultiRateFixtureCase {
    case_id: String,
    request: AlignmentRequest,
    expected_result: String,
    expected_entry_dispositions: Vec<String>,
    expected_future_samples_excluded: usize,
}

fn alignment_error(message: impl Into<String>) -> ReplayError {
    ReplayError::new(message)
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_sample(sample: &ObservationSample) -> Result<(), ReplayError> {
    non_empty(&sample.sample_id, "sample_id")?;

    if let Some(value) = sample.value {
        if !value.is_finite() {
            return Err(alignment_error("observation value must be finite"));
        }
    }

    match sample.evidence_state {
        EvidenceState::Measured => {
            if sample.value.is_none() {
                return Err(alignment_error("measured sample must contain a numeric value"));
            }
            let lineage = sample
                .lineage
                .as_ref()
                .ok_or_else(|| alignment_error("measured sample requires source and calibration lineage"))?;
            non_empty(&lineage.source_id, "lineage.source_id")?;
            if !is_sha256(&lineage.source_sha256) {
                return Err(alignment_error("measured sample source_sha256 must be 64 hexadecimal characters"));
            }
            non_empty(
                lineage.calibration_id.as_deref().unwrap_or(""),
                "lineage.calibration_id",
            )?;
            non_empty(
                lineage.calibration_revision.as_deref().unwrap_or(""),
                "lineage.calibration_revision",
            )?;
        }
        EvidenceState::Synthetic | EvidenceState::Reconstructed => {
            if sample.value.is_none() {
                return Err(alignment_error("synthetic or reconstructed sample must contain a numeric value"));
            }
        }
        EvidenceState::Unavailable | EvidenceState::Invalid => {
            if sample.value.is_some() {
                return Err(alignment_error("unavailable or invalid sample must not expose a numeric value"));
            }
        }
        EvidenceState::Delayed | EvidenceState::Degraded => {}
    }
    Ok(())
}

fn validate_request(request: &AlignmentRequest) -> Result<(), ReplayError> {
    if request.schema_version != MULTIRATE_SCHEMA_VERSION {
        return Err(alignment_error("unsupported multi-rate alignment schema version"));
    }
    non_empty(&request.scenario_id, "scenario_id")?;
    non_empty(&request.clock_domain, "clock_domain")?;

    if request.streams.is_empty() {
        return Err(alignment_error("at least one observation stream is required"));
    }

    let mut stream_ids = BTreeSet::new();
    let mut channel_ids = BTreeSet::new();

    for stream in &request.streams {
        non_empty(&stream.stream_id, "stream_id")?;
        non_empty(&stream.channel_id, "channel_id")?;
        non_empty(&stream.diagnostic_system_id, "diagnostic_system_id")?;
        non_empty(&stream.units, "units")?;
        non_empty(&stream.clock_domain, "stream.clock_domain")?;

        if !stream_ids.insert(stream.stream_id.as_str()) {
            return Err(alignment_error(format!("duplicate stream_id: {}", stream.stream_id)));
        }
        if !channel_ids.insert(stream.channel_id.as_str()) {
            return Err(alignment_error(format!("multiple streams declare channel_id: {}", stream.channel_id)));
        }
        if stream.clock_domain != request.clock_domain {
            return Err(alignment_error(format!(
                "clock-domain mismatch for stream {}: {} != {}",
                stream.stream_id, stream.clock_domain, request.clock_domain
            )));
        }
        if stream.native_sample_period_ns == Some(0) {
            return Err(alignment_error("native_sample_period_ns must be positive when supplied"));
        }
        if stream.samples.is_empty() {
            return Err(alignment_error(format!("stream {} contains no samples", stream.stream_id)));
        }

        let mut sample_ids = BTreeSet::new();
        let mut previous_timestamp = None;
        for sample in &stream.samples {
            validate_sample(sample)?;
            if !sample_ids.insert(sample.sample_id.as_str()) {
                return Err(alignment_error(format!(
                    "duplicate sample_id {} in stream {}",
                    sample.sample_id, stream.stream_id
                )));
            }
            if let Some(previous) = previous_timestamp {
                if sample.timestamp_ns <= previous {
                    return Err(alignment_error(format!(
                        "timestamps in stream {} must be strictly increasing",
                        stream.stream_id
                    )));
                }
            }
            previous_timestamp = Some(sample.timestamp_ns);
        }
    }

    let mut required = BTreeSet::new();
    for channel in &request.required_channels {
        non_empty(channel, "required_channel")?;
        if !required.insert(channel.as_str()) {
            return Err(alignment_error(format!("duplicate required channel: {channel}")));
        }
        if !request.max_age_ns_by_channel.contains_key(channel) {
            return Err(alignment_error(format!("required channel {channel} has no maximum age")));
        }
    }

    for channel in request.max_age_ns_by_channel.keys() {
        non_empty(channel, "max_age channel")?;
        if !required.contains(channel.as_str()) && !channel_ids.contains(channel.as_str()) {
            return Err(alignment_error(format!("maximum-age policy refers to unknown channel {channel}")));
        }
    }

    Ok(())
}

fn disposition_for(state: EvidenceState) -> AlignmentDisposition {
    match state {
        EvidenceState::Measured => AlignmentDisposition::MeasuredAvailable,
        EvidenceState::Synthetic => AlignmentDisposition::SyntheticOnly,
        EvidenceState::Reconstructed => AlignmentDisposition::ReconstructedOnly,
        EvidenceState::Unavailable => AlignmentDisposition::Unavailable,
        EvidenceState::Invalid => AlignmentDisposition::Invalid,
        EvidenceState::Delayed => AlignmentDisposition::Delayed,
        EvidenceState::Degraded => AlignmentDisposition::Degraded,
    }
}

/// Selects the latest sample at or before the decision timestamp for each
/// native-rate stream. No interpolation, fill, resampling or future access occurs.
pub fn align_as_of(request: &AlignmentRequest) -> Result<AlignmentReceipt, ReplayError> {
    validate_request(request)?;

    let request_sha256 = digest_serialized(request)?;
    let source_streams_sha256 = digest_serialized(&request.streams)?;
    let required: BTreeSet<&str> = request.required_channels.iter().map(String::as_str).collect();

    let mut entries = Vec::with_capacity(request.streams.len() + required.len());
    let mut future_samples_excluded = 0usize;

    for stream in &request.streams {
        future_samples_excluded = future_samples_excluded
            .checked_add(
                stream
                    .samples
                    .iter()
                    .filter(|sample| sample.timestamp_ns > request.decision_time_ns)
                    .count(),
            )
            .ok_or_else(|| alignment_error("future sample count overflow"))?;

        let selected = stream
            .samples
            .iter()
            .rev()
            .find(|sample| sample.timestamp_ns <= request.decision_time_ns);

        let is_required = required.contains(stream.channel_id.as_str());
        let maximum_age = request.max_age_ns_by_channel.get(&stream.channel_id).copied();

        let entry = match selected {
            None => AlignmentEntry {
                stream_id: Some(stream.stream_id.clone()),
                channel_id: stream.channel_id.clone(),
                required: is_required,
                disposition: AlignmentDisposition::NoPastSample,
                selected_sample_id: None,
                selected_timestamp_ns: None,
                age_ns: None,
                native_sample_period_ns: stream.native_sample_period_ns,
                value: None,
                evidence_state: None,
                lineage: None,
            },
            Some(sample) => {
                let age_ns = request.decision_time_ns - sample.timestamp_ns;
                let stale = maximum_age.is_some_and(|limit| age_ns > limit);
                AlignmentEntry {
                    stream_id: Some(stream.stream_id.clone()),
                    channel_id: stream.channel_id.clone(),
                    required: is_required,
                    disposition: if stale {
                        AlignmentDisposition::Stale
                    } else {
                        disposition_for(sample.evidence_state)
                    },
                    selected_sample_id: Some(sample.sample_id.clone()),
                    selected_timestamp_ns: Some(sample.timestamp_ns),
                    age_ns: Some(age_ns),
                    native_sample_period_ns: stream.native_sample_period_ns,
                    value: if stale { None } else { sample.value },
                    evidence_state: Some(sample.evidence_state),
                    lineage: sample.lineage.clone(),
                }
            }
        };
        entries.push(entry);
    }

    for channel in &request.required_channels {
        if !request.streams.iter().any(|stream| stream.channel_id == *channel) {
            entries.push(AlignmentEntry {
                stream_id: None,
                channel_id: channel.clone(),
                required: true,
                disposition: AlignmentDisposition::MissingStream,
                selected_sample_id: None,
                selected_timestamp_ns: None,
                age_ns: None,
                native_sample_period_ns: None,
                value: None,
                evidence_state: None,
                lineage: None,
            });
        }
    }

    entries.sort_by(|left, right| left.channel_id.cmp(&right.channel_id));

    let body = AlignmentReceiptBody {
        schema_version: MULTIRATE_SCHEMA_VERSION.to_owned(),
        scenario_id: request.scenario_id.clone(),
        clock_domain: request.clock_domain.clone(),
        decision_time_ns: request.decision_time_ns,
        evidence_class: ALIGNMENT_EVIDENCE_CLASS.to_owned(),
        physical_measurements: false,
        claim_ceiling: ALIGNMENT_CLAIM_CEILING.to_owned(),
        request_sha256,
        source_streams_sha256,
        future_samples_excluded,
        entries,
    };

    let receipt_sha256 = digest_serialized(&body)?;
    let receipt = AlignmentReceipt { body, receipt_sha256 };
    verify_alignment_receipt(&receipt)?;
    Ok(receipt)
}

/// Replays the declared request and compares the complete deterministic receipt.
/// This binds the receipt to the supplied source streams; SHA-256 still does not
/// authenticate who supplied those streams or establish physical validity.
pub fn verify_alignment_receipt_for_request(
    request: &AlignmentRequest,
    receipt: &AlignmentReceipt,
) -> Result<(), ReplayError> {
    let expected = align_as_of(request)?;
    if &expected != receipt {
        return Err(alignment_error(
            "alignment receipt does not match deterministic replay of the supplied request",
        ));
    }
    Ok(())
}

/// Checks receipt self-consistency and causal-time invariants. It does not
/// authenticate input sources or establish physical measurement validity.
pub fn verify_alignment_receipt(receipt: &AlignmentReceipt) -> Result<(), ReplayError> {
    let body = &receipt.body;
    if body.schema_version != MULTIRATE_SCHEMA_VERSION
        || body.evidence_class != ALIGNMENT_EVIDENCE_CLASS
        || body.physical_measurements
        || body.claim_ceiling != ALIGNMENT_CLAIM_CEILING
    {
        return Err(alignment_error("alignment receipt has an invalid schema or claim ceiling"));
    }
    if !is_sha256(&body.request_sha256) || !is_sha256(&body.source_streams_sha256) {
        return Err(alignment_error("alignment receipt contains an invalid source digest"));
    }
    if digest_serialized(body)? != receipt.receipt_sha256 {
        return Err(alignment_error("alignment receipt digest mismatch"));
    }

    let mut channel_ids = BTreeSet::new();
    for entry in &body.entries {
        non_empty(&entry.channel_id, "entry.channel_id")?;
        if !channel_ids.insert(entry.channel_id.as_str()) {
            return Err(alignment_error("alignment receipt contains duplicate channel entries"));
        }

        match (entry.selected_timestamp_ns, entry.age_ns) {
            (Some(timestamp), Some(age)) => {
                if timestamp > body.decision_time_ns
                    || body.decision_time_ns.checked_sub(timestamp) != Some(age)
                    || entry.selected_sample_id.as_deref().is_none_or(str::is_empty)
                    || entry.stream_id.as_deref().is_none_or(str::is_empty)
                    || entry.evidence_state.is_none()
                {
                    return Err(alignment_error("alignment receipt violates causal timestamp or age invariants"));
                }
            }
            (None, None) => {
                if entry.selected_sample_id.is_some()
                    || entry.value.is_some()
                    || entry.lineage.is_some()
                    || entry.evidence_state.is_some()
                {
                    return Err(alignment_error("empty alignment entry contains selected-sample data"));
                }
            }
            _ => return Err(alignment_error("alignment receipt has incomplete timestamp metadata")),
        }

        if entry.disposition == AlignmentDisposition::Stale {
            if entry.value.is_some() || entry.evidence_state.is_none() {
                return Err(alignment_error("stale sample must withhold its value and retain its evidence state"));
            }
        }

        if let Some(state) = entry.evidence_state {
            if entry.disposition != AlignmentDisposition::Stale
                && entry.disposition != disposition_for(state)
            {
                return Err(alignment_error("alignment receipt changes the selected sample's evidence disposition"));
            }
            if state == EvidenceState::Measured {
                let lineage = entry
                    .lineage
                    .as_ref()
                    .ok_or_else(|| alignment_error("measured sample is missing source/calibration lineage"))?;
                if !is_sha256(&lineage.source_sha256)
                    || lineage.source_id.trim().is_empty()
                    || lineage.calibration_id.as_deref().is_none_or(str::is_empty)
                    || lineage.calibration_revision.as_deref().is_none_or(str::is_empty)
                {
                    return Err(alignment_error("measured sample has incomplete source/calibration lineage"));
                }
            }
        } else if entry.selected_sample_id.is_some() {
            return Err(alignment_error("selected sample is missing its evidence state"));
        }

        if matches!(
            entry.evidence_state,
            Some(EvidenceState::Unavailable | EvidenceState::Invalid)
        ) && entry.value.is_some()
        {
            return Err(alignment_error("unavailable or invalid evidence must not expose a value"));
        }
        if matches!(
            entry.disposition,
            AlignmentDisposition::NoPastSample | AlignmentDisposition::MissingStream
        ) && entry.value.is_some()
        {
            return Err(alignment_error("missing or future-only entry must not expose a value"));
        }

        if entry.disposition == AlignmentDisposition::MeasuredAvailable {
            if entry.evidence_state != Some(EvidenceState::Measured) || entry.value.is_none() {
                return Err(alignment_error("measured disposition does not bind a measured value"));
            }
            let lineage = entry
                .lineage
                .as_ref()
                .ok_or_else(|| alignment_error("measured disposition is missing lineage"))?;
            if !is_sha256(&lineage.source_sha256)
                || lineage.source_id.trim().is_empty()
                || lineage.calibration_id.as_deref().is_none_or(str::is_empty)
                || lineage.calibration_revision.as_deref().is_none_or(str::is_empty)
            {
                return Err(alignment_error("measured disposition has incomplete source/calibration lineage"));
            }
        }

        if entry.disposition == AlignmentDisposition::SyntheticOnly
            && entry.evidence_state != Some(EvidenceState::Synthetic)
        {
            return Err(alignment_error("synthetic disposition changed its evidence state"));
        }
        if entry.disposition == AlignmentDisposition::ReconstructedOnly
            && entry.evidence_state != Some(EvidenceState::Reconstructed)
        {
            return Err(alignment_error("reconstructed disposition changed its evidence state"));
        }
        if matches!(
            entry.disposition,
            AlignmentDisposition::NoPastSample | AlignmentDisposition::MissingStream
        ) && entry.stream_id.is_none()
            != (entry.disposition == AlignmentDisposition::MissingStream)
        {
            return Err(alignment_error("missing-stream identity does not match its disposition"));
        }
    }

    Ok(())
}

pub fn audit_multirate_manifest_json(source: &str) -> Result<MultiRateAuditReport, ReplayError> {
    let manifest: MultiRateBenchmarkManifest = serde_json::from_str(source)
        .map_err(|error| alignment_error(format!("invalid multi-rate benchmark JSON: {error}")))?;

    if manifest.schema_version != MULTIRATE_BENCHMARK_SCHEMA_VERSION
        || manifest.evidence_class != ALIGNMENT_EVIDENCE_CLASS
        || manifest.physical_measurements
        || manifest.claim_ceiling != ALIGNMENT_CLAIM_CEILING
    {
        return Err(alignment_error("multi-rate benchmark manifest claim ceiling is invalid"));
    }
    non_empty(&manifest.benchmark_id, "benchmark_id")?;
    if manifest.cases.is_empty() {
        return Err(alignment_error("multi-rate benchmark has no cases"));
    }

    let mut case_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(manifest.cases.len());
    let mut matched_expectations = 0usize;

    for case in manifest.cases {
        non_empty(&case.case_id, "case_id")?;
        if !case_ids.insert(case.case_id.clone()) {
            return Err(alignment_error(format!("duplicate case_id: {}", case.case_id)));
        }
        if case.expected_result != "COMPLETED" && case.expected_result != "REJECTED" {
            return Err(alignment_error(format!("unsupported expected result in case {}", case.case_id)));
        }

        let attempt = align_as_of(&case.request);
        let (actual_result, actual_dispositions, error, receipt_sha256, future_count) =
            match attempt {
                Ok(receipt) => {
                    let mut dispositions: Vec<String> = receipt
                        .body
                        .entries
                        .iter()
                        .map(|entry| format!("{:?}", entry.disposition).to_ascii_uppercase())
                        .collect();
                    dispositions.sort();
                    (
                        "COMPLETED".to_owned(),
                        dispositions,
                        None,
                        Some(receipt.receipt_sha256),
                        receipt.body.future_samples_excluded,
                    )
                }
                Err(error) => (
                    "REJECTED".to_owned(),
                    Vec::new(),
                    Some(error.to_string()),
                    None,
                    0,
                ),
            };

        let mut expected_dispositions = case.expected_entry_dispositions;
        expected_dispositions.sort();
        let matched = actual_result == case.expected_result
            && actual_dispositions == expected_dispositions
            && future_count == case.expected_future_samples_excluded;
        if matched {
            matched_expectations += 1;
        }

        results.push(MultiRateCaseAudit {
            case_id: case.case_id,
            actual_result,
            expected_result: case.expected_result,
            actual_entry_dispositions: actual_dispositions,
            matched_expectation: matched,
            error,
            receipt_sha256,
        });
    }

    let case_count = results.len();
    Ok(MultiRateAuditReport {
        benchmark_id: manifest.benchmark_id,
        case_count,
        matched_expectations,
        mismatched_expectations: case_count - matched_expectations,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured(sample_id: &str, timestamp_ns: u64, value: f64) -> ObservationSample {
        ObservationSample {
            sample_id: sample_id.to_owned(),
            timestamp_ns,
            value: Some(value),
            evidence_state: EvidenceState::Measured,
            lineage: Some(ObservationLineage {
                source_id: format!("source-{sample_id}"),
                source_sha256: "a".repeat(64),
                calibration_id: Some("calibration-1".to_owned()),
                calibration_revision: Some("rev-1".to_owned()),
            }),
        }
    }

    fn stream(
        stream_id: &str,
        channel_id: &str,
        period: Option<u64>,
        samples: Vec<ObservationSample>,
    ) -> ObservationStream {
        ObservationStream {
            stream_id: stream_id.to_owned(),
            channel_id: channel_id.to_owned(),
            diagnostic_system_id: format!("diag-{channel_id}"),
            units: "synthetic-unit".to_owned(),
            clock_domain: "shot-relative-ns".to_owned(),
            native_sample_period_ns: period,
            samples,
        }
    }

    fn request(streams: Vec<ObservationStream>, decision_time_ns: u64) -> AlignmentRequest {
        let mut max_age_ns_by_channel = BTreeMap::new();
        max_age_ns_by_channel.insert("density".to_owned(), 100);
        max_age_ns_by_channel.insert("magnetics".to_owned(), 100);
        AlignmentRequest {
            schema_version: MULTIRATE_SCHEMA_VERSION.to_owned(),
            scenario_id: "synthetic-multirate-test".to_owned(),
            clock_domain: "shot-relative-ns".to_owned(),
            decision_time_ns,
            required_channels: vec!["density".to_owned(), "magnetics".to_owned()],
            max_age_ns_by_channel,
            streams,
        }
    }

    #[test]
    fn asynchronous_streams_keep_native_timestamps_and_do_not_interpolate() {
        let req = request(
            vec![
                stream("density-stream", "density", Some(10), vec![
                    measured("d0", 10, 1.0),
                    measured("d1", 20, 2.0),
                    measured("d2", 30, 3.0),
                ]),
                stream("mag-stream", "magnetics", Some(20), vec![
                    measured("m0", 15, 4.0),
                    measured("m1", 35, 5.0),
                ]),
            ],
            25,
        );
        let receipt = align_as_of(&req).expect("alignment");
        assert_eq!(receipt.body.entries[0].channel_id, "density");
        assert_eq!(receipt.body.entries[0].selected_timestamp_ns, Some(20));
        assert_eq!(receipt.body.entries[0].value, Some(2.0));
        assert_eq!(receipt.body.entries[1].channel_id, "magnetics");
        assert_eq!(receipt.body.entries[1].selected_timestamp_ns, Some(15));
        assert_eq!(receipt.body.entries[1].age_ns, Some(10));
        assert_eq!(receipt.body.entries[1].value, Some(4.0));
        assert_eq!(receipt.body.future_samples_excluded, 2);
        assert!(receipt.body.entries.iter().all(|entry| entry.disposition == AlignmentDisposition::MeasuredAvailable));
    }

    #[test]
    fn measured_zero_remains_distinct_from_missing() {
        let req = request(
            vec![
                stream("density-stream", "density", Some(10), vec![measured("d-zero", 20, 0.0)]),
                stream("mag-stream", "magnetics", Some(10), vec![measured("m-one", 20, 1.0)]),
            ],
            20,
        );
        let receipt = align_as_of(&req).expect("alignment");
        assert_eq!(receipt.body.entries[0].disposition, AlignmentDisposition::MeasuredAvailable);
        assert_eq!(receipt.body.entries[0].value, Some(0.0));
        assert_eq!(receipt.body.entries[0].evidence_state, Some(EvidenceState::Measured));
    }

    #[test]
    fn future_only_stream_is_explicitly_unavailable_to_as_of_query() {
        let req = request(
            vec![
                stream("density-stream", "density", Some(10), vec![measured("d1", 30, 2.0)]),
                stream("mag-stream", "magnetics", Some(10), vec![measured("m1", 40, 4.0)]),
            ],
            20,
        );
        let receipt = align_as_of(&req).expect("alignment");
        assert!(receipt.body.entries.iter().all(|entry| entry.disposition == AlignmentDisposition::NoPastSample));
        assert_eq!(receipt.body.future_samples_excluded, 2);
    }

    #[test]
    fn staleness_boundary_is_inclusive_and_stale_value_is_withheld() {
        let mut req = request(
            vec![
                stream("density-stream", "density", Some(10), vec![measured("d1", 50, 2.0)]),
                stream("mag-stream", "magnetics", Some(10), vec![measured("m1", 51, 4.0)]),
            ],
            100,
        );
        req.max_age_ns_by_channel.insert("density".to_owned(), 50);
        req.max_age_ns_by_channel.insert("magnetics".to_owned(), 48);
        let receipt = align_as_of(&req).expect("alignment");
        assert_eq!(receipt.body.entries[0].disposition, AlignmentDisposition::MeasuredAvailable);
        assert_eq!(receipt.body.entries[0].value, Some(2.0));
        assert_eq!(receipt.body.entries[1].disposition, AlignmentDisposition::Stale);
        assert_eq!(receipt.body.entries[1].value, None);
        assert_eq!(receipt.body.entries[1].evidence_state, Some(EvidenceState::Measured));
    }

    #[test]
    fn synthetic_and_reconstructed_states_are_never_promoted() {
        let mut synthetic = measured("synthetic", 10, 1.0);
        synthetic.evidence_state = EvidenceState::Synthetic;
        synthetic.lineage = None;
        let mut reconstructed = measured("reconstructed", 10, 2.0);
        reconstructed.evidence_state = EvidenceState::Reconstructed;
        reconstructed.lineage = None;
        let req = request(
            vec![
                stream("density-stream", "density", Some(10), vec![synthetic]),
                stream("mag-stream", "magnetics", Some(10), vec![reconstructed]),
            ],
            20,
        );
        let receipt = align_as_of(&req).expect("alignment");
        assert_eq!(receipt.body.entries[0].disposition, AlignmentDisposition::SyntheticOnly);
        assert_eq!(receipt.body.entries[1].disposition, AlignmentDisposition::ReconstructedOnly);
    }

    #[test]
    fn unavailable_invalid_delayed_and_degraded_states_remain_distinct() {
        let samples = vec![
            ObservationSample {
                sample_id: "unavailable".to_owned(),
                timestamp_ns: 10,
                value: None,
                evidence_state: EvidenceState::Unavailable,
                lineage: None,
            },
            ObservationSample {
                sample_id: "invalid".to_owned(),
                timestamp_ns: 20,
                value: None,
                evidence_state: EvidenceState::Invalid,
                lineage: None,
            },
            ObservationSample {
                sample_id: "delayed".to_owned(),
                timestamp_ns: 30,
                value: Some(3.0),
                evidence_state: EvidenceState::Delayed,
                lineage: None,
            },
            ObservationSample {
                sample_id: "degraded".to_owned(),
                timestamp_ns: 40,
                value: Some(4.0),
                evidence_state: EvidenceState::Degraded,
                lineage: None,
            },
        ];
        let mut req = request(
            vec![
                stream("density-stream", "density", Some(10), samples),
                stream(
                    "mag-stream",
                    "magnetics",
                    Some(10),
                    vec![
                        measured("m0", 10, 1.0),
                        measured("m1", 20, 2.0),
                        measured("m2", 30, 3.0),
                        measured("m3", 40, 4.0),
                    ],
                ),
            ],
            10,
        );
        req.max_age_ns_by_channel.insert("density".to_owned(), 100);
        for (time, expected_state, expected_disposition) in [
            (10, EvidenceState::Unavailable, AlignmentDisposition::Unavailable),
            (20, EvidenceState::Invalid, AlignmentDisposition::Invalid),
            (30, EvidenceState::Delayed, AlignmentDisposition::Delayed),
            (40, EvidenceState::Degraded, AlignmentDisposition::Degraded),
        ] {
            req.decision_time_ns = time;
            let receipt = align_as_of(&req).expect("alignment");
            let density = receipt
                .body
                .entries
                .iter()
                .find(|entry| entry.channel_id == "density")
                .expect("density entry");
            assert_eq!(density.evidence_state, Some(expected_state));
            assert_eq!(density.disposition, expected_disposition);
            if matches!(expected_state, EvidenceState::Unavailable | EvidenceState::Invalid) {
                assert_eq!(density.value, None);
            }
            verify_alignment_receipt(&receipt).expect("evidence-preserving receipt");
        }
    }

    #[test]
    fn absent_required_stream_is_explicit() {
        let req = request(
            vec![stream("density-stream", "density", Some(10), vec![measured("d", 10, 1.0)])],
            20,
        );
        let receipt = align_as_of(&req).expect("alignment");
        assert_eq!(receipt.body.entries.len(), 2);
        assert_eq!(receipt.body.entries[1].channel_id, "magnetics");
        assert_eq!(receipt.body.entries[1].disposition, AlignmentDisposition::MissingStream);
    }

    #[test]
    fn invalid_measured_lineage_and_non_monotonic_time_are_rejected() {
        let mut no_lineage = measured("d", 10, 1.0);
        no_lineage.lineage = None;
        let bad_lineage = request(
            vec![
                stream("density-stream", "density", Some(10), vec![no_lineage]),
                stream("mag-stream", "magnetics", Some(10), vec![measured("m", 10, 2.0)]),
            ],
            20,
        );
        assert!(align_as_of(&bad_lineage).is_err());

        let non_monotonic = request(
            vec![
                stream("density-stream", "density", Some(10), vec![
                    measured("d1", 20, 1.0),
                    measured("d0", 10, 2.0),
                ]),
                stream("mag-stream", "magnetics", Some(10), vec![measured("m", 10, 2.0)]),
            ],
            30,
        );
        assert!(align_as_of(&non_monotonic).is_err());
    }

    #[test]
    fn receipt_tampering_and_clock_domain_mismatch_are_rejected() {
        let req = request(
            vec![
                stream("density-stream", "density", Some(10), vec![measured("d", 10, 1.0)]),
                stream("mag-stream", "magnetics", Some(10), vec![measured("m", 10, 2.0)]),
            ],
            20,
        );
        let mut receipt = align_as_of(&req).expect("alignment");
        receipt.body.entries[0].value = Some(999.0);
        assert!(verify_alignment_receipt(&receipt).is_err());
        receipt.receipt_sha256 = digest_serialized(&receipt.body).expect("tampered receipt digest");
        assert!(verify_alignment_receipt(&receipt).is_ok());
        assert!(verify_alignment_receipt_for_request(&req, &receipt).is_err());

        let mut wrong_clock = req;
        wrong_clock.streams[0].clock_domain = "different-clock".to_owned();
        assert!(align_as_of(&wrong_clock).is_err());
    }

    #[test]
    fn frozen_multirate_benchmark_matches_expected_results() {
        let source = include_str!("../../../../benchmarks/plasma/multirate-alignment-v0.json");
        let report = audit_multirate_manifest_json(source).expect("benchmark manifest");
        assert_eq!(report.case_count, 8);
        assert_eq!(report.mismatched_expectations, 0, "{report:#?}");
    }
}
