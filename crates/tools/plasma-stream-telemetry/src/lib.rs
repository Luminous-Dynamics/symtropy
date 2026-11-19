use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: &str = "luminous.plasma-stream-telemetry.v0.1";
const DRIFT_DENOMINATOR: i128 = 1_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockCalibration {
    pub calibration_id: String,
    pub clock_domain_id: String,
    pub reference_clock_domain_id: String,
    pub anchor_clock_ns: u64,
    pub anchor_reference_ns: u64,
    pub drift_ppb: i64,
    pub valid_from_clock_ns: u64,
    pub valid_until_clock_ns: u64,
    pub uncertainty_ns: u64,
    pub evidence_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockStamp {
    pub time_ns: u64,
    pub clock_domain_id: String,
    pub calibration_id: String,
    pub uncertainty_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngressDisposition {
    Received,
    Dropped,
    TransportRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcknowledgementState {
    Acknowledged,
    Rejected,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownstreamAcknowledgement {
    pub acknowledgement_id: String,
    pub state: AcknowledgementState,
    pub timestamp: Option<ClockStamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelIdentity {
    pub artifact_id: String,
    pub revision: String,
    pub artifact_sha256: String,
    pub runtime_id: String,
    /// Digest reference to separately retained model-verification evidence.
    /// This is not a signature and does not authenticate the evidence itself.
    pub verification_evidence_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamSpec {
    pub pipeline_id: String,
    pub diagnostic_id: String,
    pub reference_clock_domain_id: String,
    pub expected_period_ns: u64,
    pub max_source_period_deviation_ns: u64,
    pub max_source_age_ns: u64,
    pub deadline_from_acquisition_ns: u64,
    pub max_clock_uncertainty_ns: u64,
    pub max_queue_depth: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelemetryEvent {
    pub event_id: String,
    pub sample_id: String,
    pub sequence_number: u64,
    pub ingress: IngressDisposition,
    pub source_time: ClockStamp,
    pub acquisition_time: ClockStamp,
    pub receive_time: Option<ClockStamp>,
    pub processing_start_time: Option<ClockStamp>,
    pub processing_end_time: Option<ClockStamp>,
    pub decision_time: Option<ClockStamp>,
    pub queue_depth: u32,
    pub model: ModelIdentity,
    pub acknowledgement: Option<DownstreamAcknowledgement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayRequest {
    pub schema_version: String,
    pub trace_id: String,
    pub spec: StreamSpec,
    pub calibrations: Vec<ClockCalibration>,
    /// Events are in receiver-observation order; the auditor never sorts them.
    pub events: Vec<TelemetryEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventDisposition {
    OnTimeValid,
    LateValidButStaleForDecision,
    Degraded,
    Dropped,
    TransportRejected,
    Duplicate,
    OutOfOrder,
    ClockUnqualified,
    ModelUnqualified,
    DeadlineMissed,
    QueueOverflow,
    ProcessingTimeout,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LatencyMetrics {
    pub source_to_acquisition_latency_ns: Option<u64>,
    pub observed_source_interarrival_ns: Option<u64>,
    pub expected_source_interarrival_ns: Option<u64>,
    pub source_period_deviation_ns: Option<u64>,
    pub source_period_consistent: Option<bool>,
    pub transport_latency_ns: Option<u64>,
    pub queue_wait_ns: Option<u64>,
    pub processing_latency_ns: Option<u64>,
    pub acquisition_to_decision_ns: Option<u64>,
    pub downstream_ack_latency_ns: Option<u64>,
    pub source_age_at_decision_ns: Option<u64>,
    pub source_period_ns: u64,
    pub deadline_budget_ns: u64,
    pub deadline_missed: Option<bool>,
    pub sequence_gap_before_event: bool,
    pub clock_uncertainty_bound_ns: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventReport {
    pub event_id: String,
    pub sample_id: String,
    pub sequence_number: u64,
    pub disposition: EventDisposition,
    pub metrics: LatencyMetrics,
    pub acknowledgement_state: Option<AcknowledgementState>,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaySummary {
    pub event_count: usize,
    pub on_time_valid: usize,
    pub late_or_deadline_missed: usize,
    pub degraded: usize,
    pub dropped_or_rejected: usize,
    pub duplicate_or_out_of_order: usize,
    pub clock_unqualified: usize,
    pub model_unqualified: usize,
    pub processing_timeout_or_unknown: usize,
    pub sequence_gaps: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayReportBody {
    pub schema_version: String,
    pub trace_id: String,
    pub pipeline_id: String,
    pub diagnostic_id: String,
    pub reference_clock_domain_id: String,
    pub request_sha256: String,
    pub reports: Vec<EventReport>,
    pub summary: ReplaySummary,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayReport {
    pub body: ReplayReportBody,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MappedStamp {
    time_ns: u64,
    uncertainty_ns: u64,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn map_stamp(
    stamp: &ClockStamp,
    calibrations: &BTreeMap<String, &ClockCalibration>,
    reference_clock_id: &str,
) -> Result<MappedStamp, String> {
    let calibration = calibrations
        .get(&stamp.calibration_id)
        .ok_or_else(|| "timestamp references an unknown clock calibration".to_owned())?;

    if !non_empty(&stamp.clock_domain_id)
        || calibration.clock_domain_id != stamp.clock_domain_id
        || calibration.reference_clock_domain_id != reference_clock_id
    {
        return Err("clock calibration does not map this domain to the declared reference".to_owned());
    }
    if stamp.time_ns < calibration.valid_from_clock_ns
        || stamp.time_ns >= calibration.valid_until_clock_ns
        || !is_sha256(&calibration.evidence_sha256)
    {
        return Err("clock timestamp lies outside a valid evidenced calibration".to_owned());
    }

    let delta = i128::from(stamp.time_ns) - i128::from(calibration.anchor_clock_ns);
    let drift_correction = delta
        .checked_mul(i128::from(calibration.drift_ppb))
        .ok_or_else(|| "clock drift multiplication overflow".to_owned())?
        / DRIFT_DENOMINATOR;
    let mapped = i128::from(calibration.anchor_reference_ns)
        .checked_add(delta)
        .and_then(|value| value.checked_add(drift_correction))
        .ok_or_else(|| "clock mapping overflow".to_owned())?;

    if !(0..=i128::from(u64::MAX)).contains(&mapped) {
        return Err("mapped timestamp is outside u64 nanosecond range".to_owned());
    }
    let uncertainty_ns = stamp
        .uncertainty_ns
        .checked_add(calibration.uncertainty_ns)
        .ok_or_else(|| "clock uncertainty overflow".to_owned())?;

    Ok(MappedStamp {
        time_ns: mapped as u64,
        uncertainty_ns,
    })
}

fn map_optional(
    stamp: &Option<ClockStamp>,
    calibrations: &BTreeMap<String, &ClockCalibration>,
    reference_clock_id: &str,
) -> Result<Option<MappedStamp>, String> {
    stamp
        .as_ref()
        .map(|value| map_stamp(value, calibrations, reference_clock_id))
        .transpose()
}

fn interval(
    start: MappedStamp,
    end: MappedStamp,
    max_uncertainty_ns: u64,
) -> Result<(u64, u64), String> {
    let uncertainty = start
        .uncertainty_ns
        .checked_add(end.uncertainty_ns)
        .ok_or_else(|| "interval uncertainty overflow".to_owned())?;
    if uncertainty > max_uncertainty_ns {
        return Err("combined clock uncertainty exceeds declared budget".to_owned());
    }
    let duration = end
        .time_ns
        .checked_sub(start.time_ns)
        .ok_or_else(|| "timestamp ordering is inconsistent after calibration".to_owned())?;
    Ok((duration, uncertainty))
}

fn validate_request(request: &ReplayRequest) -> Result<(), String> {
    if request.schema_version != SCHEMA_VERSION {
        return Err("unsupported stream telemetry schema version".to_owned());
    }
    for (name, value) in [
        ("trace_id", request.trace_id.as_str()),
        ("pipeline_id", request.spec.pipeline_id.as_str()),
        ("diagnostic_id", request.spec.diagnostic_id.as_str()),
        ("reference_clock_domain_id", request.spec.reference_clock_domain_id.as_str()),
    ] {
        if !non_empty(value) {
            return Err(format!("{name} must be non-empty"));
        }
    }
    if request.spec.expected_period_ns == 0
        || request.spec.max_source_age_ns == 0
        || request.spec.deadline_from_acquisition_ns == 0
        || request.events.is_empty()
    {
        return Err("period, age and deadline budgets must be positive; events must be non-empty".to_owned());
    }

    let mut calibration_ids = BTreeSet::new();
    for calibration in &request.calibrations {
        if !non_empty(&calibration.calibration_id)
            || !non_empty(&calibration.clock_domain_id)
            || !non_empty(&calibration.reference_clock_domain_id)
            || !calibration_ids.insert(calibration.calibration_id.clone())
            || calibration.valid_from_clock_ns >= calibration.valid_until_clock_ns
            || calibration.anchor_clock_ns < calibration.valid_from_clock_ns
            || calibration.anchor_clock_ns >= calibration.valid_until_clock_ns
            || !is_sha256(&calibration.evidence_sha256)
        {
            return Err("clock calibration identity, interval or evidence is invalid".to_owned());
        }
    }

    let mut event_ids = BTreeSet::new();
    let mut sample_ids = BTreeSet::new();
    for event in &request.events {
        if !non_empty(&event.event_id)
            || !non_empty(&event.sample_id)
            || !event_ids.insert(event.event_id.clone())
            || !sample_ids.insert(event.sample_id.clone())
            || !non_empty(&event.model.artifact_id)
            || !non_empty(&event.model.revision)
            || !non_empty(&event.model.runtime_id)
            || !is_sha256(&event.model.artifact_sha256)
        {
            return Err("event or model identity is empty, duplicated or malformed".to_owned());
        }
    }
    Ok(())
}

fn empty_metrics(spec: &StreamSpec, sequence_gap: bool) -> LatencyMetrics {
    LatencyMetrics {
        source_to_acquisition_latency_ns: None,
        observed_source_interarrival_ns: None,
        expected_source_interarrival_ns: None,
        source_period_deviation_ns: None,
        source_period_consistent: None,
        transport_latency_ns: None,
        queue_wait_ns: None,
        processing_latency_ns: None,
        acquisition_to_decision_ns: None,
        downstream_ack_latency_ns: None,
        source_age_at_decision_ns: None,
        source_period_ns: spec.expected_period_ns,
        deadline_budget_ns: spec.deadline_from_acquisition_ns,
        deadline_missed: None,
        sequence_gap_before_event: sequence_gap,
        clock_uncertainty_bound_ns: None,
    }
}

fn report_event(
    event: &TelemetryEvent,
    spec: &StreamSpec,
    calibrations: &BTreeMap<String, &ClockCalibration>,
    sequence_gap: bool,
    previous_source: Option<(u64, MappedStamp)>,
) -> EventReport {
    let mut findings = Vec::new();
    let mut metrics = empty_metrics(spec, sequence_gap);

    let mut disposition = match event.ingress {
        IngressDisposition::Dropped => {
            findings.push("sample was dropped before receipt".to_owned());
            EventDisposition::Dropped
        }
        IngressDisposition::TransportRejected => {
            findings.push("transport rejected the sample".to_owned());
            EventDisposition::TransportRejected
        }
        IngressDisposition::Received => {
            if event.model.verification_evidence_sha256
                .as_ref()
                .is_none_or(|digest| !is_sha256(digest))
            {
                findings.push("model artifact identity is not verified".to_owned());
                EventDisposition::ModelUnqualified
            } else if event.queue_depth > spec.max_queue_depth {
                findings.push("queue depth exceeds the declared capacity".to_owned());
                EventDisposition::QueueOverflow
            } else if event.receive_time.is_none() {
                findings.push("received sample has no receive timestamp".to_owned());
                EventDisposition::Unknown
            } else if event.processing_start_time.is_none()
                || event.processing_end_time.is_none()
                || event.decision_time.is_none()
            {
                findings.push("processing or decision timestamp is missing".to_owned());
                EventDisposition::ProcessingTimeout
            } else {
                let source = map_stamp(&event.source_time, calibrations, &spec.reference_clock_domain_id);
                let acquisition = map_stamp(&event.acquisition_time, calibrations, &spec.reference_clock_domain_id);
                let receive = map_optional(&event.receive_time, calibrations, &spec.reference_clock_domain_id);
                let start = map_optional(&event.processing_start_time, calibrations, &spec.reference_clock_domain_id);
                let end = map_optional(&event.processing_end_time, calibrations, &spec.reference_clock_domain_id);
                let decision = map_optional(&event.decision_time, calibrations, &spec.reference_clock_domain_id);

                match (source, acquisition, receive, start, end, decision) {
                    (
                        Ok(source),
                        Ok(acquisition),
                        Ok(Some(receive)),
                        Ok(Some(start)),
                        Ok(Some(end)),
                        Ok(Some(decision)),
                    ) => {
                        let mapped = [source, acquisition, receive, start, end, decision];
                        let max_uncertainty = mapped.iter().map(|stamp| stamp.uncertainty_ns).max().unwrap_or(0);
                        let source_to_acquisition =
                            interval(source, acquisition, spec.max_clock_uncertainty_ns);
                        let transport = interval(acquisition, receive, spec.max_clock_uncertainty_ns);
                        let queue = interval(receive, start, spec.max_clock_uncertainty_ns);
                        let processing = interval(start, end, spec.max_clock_uncertainty_ns);
                        let total = interval(acquisition, decision, spec.max_clock_uncertainty_ns);
                        let age = interval(source, decision, spec.max_clock_uncertainty_ns);

                        let total_uncertainty = mapped.iter().try_fold(
                            0_u64,
                            |sum, stamp| sum.checked_add(stamp.uncertainty_ns),
                        );
                        let all_intervals_valid = source_to_acquisition.is_ok()
                            && transport.is_ok()
                            && queue.is_ok()
                            && processing.is_ok()
                            && total.is_ok()
                            && age.is_ok()
                            && total_uncertainty.is_some();
                        if !all_intervals_valid {
                            findings.push("clock mapping, ordering or interval uncertainty is unqualified".to_owned());
                            metrics.clock_uncertainty_bound_ns = Some(max_uncertainty);
                            EventDisposition::ClockUnqualified
                        } else {
                            metrics.source_to_acquisition_latency_ns =
                                source_to_acquisition.as_ref().ok().map(|item| item.0);
                            if let Some((previous_sequence, previous_stamp)) = previous_source
                                && event.sequence_number > previous_sequence
                            {
                                let sequence_delta = event.sequence_number - previous_sequence;
                                let expected = spec
                                    .expected_period_ns
                                    .checked_mul(sequence_delta);
                                match (interval(previous_stamp, source, spec.max_clock_uncertainty_ns), expected) {
                                    (Ok((observed, _)), Some(expected)) => {
                                        let deviation = observed.abs_diff(expected);
                                        let consistent = deviation <= spec.max_source_period_deviation_ns;
                                        metrics.observed_source_interarrival_ns = Some(observed);
                                        metrics.expected_source_interarrival_ns = Some(expected);
                                        metrics.source_period_deviation_ns = Some(deviation);
                                        metrics.source_period_consistent = Some(consistent);
                                        if !consistent {
                                            findings.push(
                                                "source inter-arrival deviates from the declared sample cadence"
                                                    .to_owned(),
                                            );
                                        }
                                    }
                                    _ => {
                                        findings.push(
                                            "source cadence cannot be compared under the declared clock budget"
                                                .to_owned(),
                                        );
                                        if disposition == EventDisposition::OnTimeValid {
                                            // A timing-quality issue must not retain an ON_TIME disposition.
                                        }
                                    }
                                }
                            }
                            metrics.transport_latency_ns = transport.as_ref().ok().map(|item| item.0);
                            metrics.queue_wait_ns = queue.as_ref().ok().map(|item| item.0);
                            metrics.processing_latency_ns = processing.as_ref().ok().map(|item| item.0);
                            metrics.acquisition_to_decision_ns = total.as_ref().ok().map(|item| item.0);
                            metrics.source_age_at_decision_ns = age.as_ref().ok().map(|item| item.0);
                            metrics.clock_uncertainty_bound_ns = total_uncertainty;

                            let total_latency = metrics.acquisition_to_decision_ns.unwrap_or(0);
                            let source_age = metrics.source_age_at_decision_ns.unwrap_or(0);
                            let deadline_missed = total_latency > spec.deadline_from_acquisition_ns;
                            metrics.deadline_missed = Some(deadline_missed);

                            if sequence_gap {
                                findings.push("one or more sequence numbers were skipped".to_owned());
                            }
                            let cadence_degraded = metrics.source_period_consistent == Some(false);
                            if deadline_missed {
                                findings.push("decision completed after the acquisition-relative deadline".to_owned());
                                EventDisposition::DeadlineMissed
                            } else if source_age > spec.max_source_age_ns {
                                findings.push(
                                    "sample is valid historical evidence but stale for this decision"
                                        .to_owned(),
                                );
                                EventDisposition::LateValidButStaleForDecision
                            } else if sequence_gap || cadence_degraded {
                                EventDisposition::Degraded
                            } else {
                                EventDisposition::OnTimeValid
                            }
                        }
                    }
                    _ => {
                        findings.push("one or more timestamps cannot be mapped into the reference clock".to_owned());
                        EventDisposition::ClockUnqualified
                    }
                }
            }
        }
    };

    if let Some(acknowledgement) = &event.acknowledgement {
        let mut acknowledgement_qualified = !non_empty(&acknowledgement.acknowledgement_id);
        match acknowledgement.state {
            AcknowledgementState::Acknowledged => {
                match (
                    event.decision_time.as_ref().map(|stamp| {
                        map_stamp(stamp, calibrations, &spec.reference_clock_domain_id)
                    }),
                    acknowledgement.timestamp.as_ref().map(|stamp| {
                        map_stamp(stamp, calibrations, &spec.reference_clock_domain_id)
                    }),
                ) {
                    (Some(Ok(decision)), Some(Ok(ack))) => {
                        match interval(decision, ack, spec.max_clock_uncertainty_ns) {
                            Ok((latency, _)) => {
                                metrics.downstream_ack_latency_ns = Some(latency);
                            }
                            Err(_) => {
                                acknowledgement_qualified = true;
                                findings.push(
                                    "acknowledgement timestamp is inconsistent or exceeds clock uncertainty"
                                        .to_owned(),
                                );
                            }
                        }
                    }
                    _ => {
                        acknowledgement_qualified = true;
                        findings.push(
                            "acknowledged outcome has no qualified timestamp or decision time"
                                .to_owned(),
                        );
                    }
                }
            }
            AcknowledgementState::Rejected => {
                acknowledgement_qualified = true;
                findings.push("downstream explicitly rejected the request".to_owned());
            }
            AcknowledgementState::Unknown => {
                acknowledgement_qualified = true;
                findings.push("downstream acknowledgement is unknown; no success is inferred".to_owned());
            }
        }

        if acknowledgement_qualified && disposition == EventDisposition::OnTimeValid {
            disposition = EventDisposition::Degraded;
        }
    }

    EventReport {
        event_id: event.event_id.clone(),
        sample_id: event.sample_id.clone(),
        sequence_number: event.sequence_number,
        disposition,
        metrics,
        acknowledgement_state: event.acknowledgement.as_ref().map(|ack| ack.state.clone()),
        findings,
    }
}

pub fn replay_trace(request: &ReplayRequest) -> Result<ReplayReport, String> {
    validate_request(request)?;
    let request_bytes =
        serde_json::to_vec(request).map_err(|error| format!("request serialization failed: {error}"))?;
    let request_sha256 = sha256_hex(&request_bytes);
    let calibrations = request
        .calibrations
        .iter()
        .map(|calibration| (calibration.calibration_id.clone(), calibration))
        .collect::<BTreeMap<_, _>>();

    let mut last_sequence: Option<u64> = None;
    let mut last_source: Option<(u64, MappedStamp)> = None;
    let mut seen_sequences = BTreeSet::new();
    let mut reports = Vec::with_capacity(request.events.len());
    let mut sequence_gaps = 0;

    for event in &request.events {
        let duplicate = seen_sequences.contains(&event.sequence_number);
        let out_of_order = last_sequence.is_some_and(|previous| event.sequence_number < previous);
        let sequence_gap = last_sequence.is_some_and(|previous| {
            previous.checked_add(1).is_some_and(|expected| event.sequence_number > expected)
        });
        let mut report = report_event(
            event,
            &request.spec,
            &calibrations,
            sequence_gap,
            last_source,
        );
        if event.ingress == IngressDisposition::Received
            && last_sequence.is_none_or(|previous| event.sequence_number > previous)
        {
            if let Ok(mapped_source) = map_stamp(
                &event.source_time,
                &calibrations,
                &request.spec.reference_clock_domain_id,
            ) {
                last_source = Some((event.sequence_number, mapped_source));
            }
        }

        if duplicate {
            report.disposition = EventDisposition::Duplicate;
            report.findings.push("sequence number duplicates a prior event".to_owned());
        } else if out_of_order {
            report.disposition = EventDisposition::OutOfOrder;
            report.findings.push("sequence number is lower than a previously received event".to_owned());
        }
        if sequence_gap {
            sequence_gaps += 1;
        }
        seen_sequences.insert(event.sequence_number);
        last_sequence = Some(last_sequence.map_or(event.sequence_number, |value| value.max(event.sequence_number)));
        reports.push(report);
    }

    let count = reports.len();
    let count_where = |predicate: fn(EventDisposition) -> bool| {
        reports.iter().filter(|report| predicate(report.disposition)).count()
    };
    let summary = ReplaySummary {
        event_count: count,
        on_time_valid: count_where(|status| status == EventDisposition::OnTimeValid),
        late_or_deadline_missed: count_where(|status| matches!(
            status,
            EventDisposition::LateValidButStaleForDecision | EventDisposition::DeadlineMissed
        )),
        degraded: count_where(|status| status == EventDisposition::Degraded),
        dropped_or_rejected: count_where(|status| matches!(
            status,
            EventDisposition::Dropped | EventDisposition::TransportRejected
        )),
        duplicate_or_out_of_order: count_where(|status| matches!(
            status,
            EventDisposition::Duplicate | EventDisposition::OutOfOrder
        )),
        clock_unqualified: count_where(|status| status == EventDisposition::ClockUnqualified),
        model_unqualified: count_where(|status| status == EventDisposition::ModelUnqualified),
        processing_timeout_or_unknown: count_where(|status| matches!(
            status,
            EventDisposition::ProcessingTimeout | EventDisposition::Unknown
        )),
        sequence_gaps,
    };

    let body = ReplayReportBody {
        schema_version: SCHEMA_VERSION.to_owned(),
        trace_id: request.trace_id.clone(),
        pipeline_id: request.spec.pipeline_id.clone(),
        diagnostic_id: request.spec.diagnostic_id.clone(),
        reference_clock_domain_id: request.spec.reference_clock_domain_id.clone(),
        request_sha256,
        reports,
        summary,
        claim_ceiling: "TIMING_AND_STREAM_INTEGRITY_ONLY_NO_CONTROL_OR_ACTUATOR_AUTHORITY".to_owned(),
    };
    let receipt_bytes =
        serde_json::to_vec(&body).map_err(|error| format!("report serialization failed: {error}"))?;

    Ok(ReplayReport {
        body,
        receipt_sha256: sha256_hex(&receipt_bytes),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn stamp(time_ns: u64) -> ClockStamp {
        ClockStamp {
            time_ns,
            clock_domain_id: "host".to_owned(),
            calibration_id: "host-to-ref".to_owned(),
            uncertainty_ns: 1,
        }
    }

    fn event(sequence_number: u64, source_ns: u64, acquisition_ns: u64) -> TelemetryEvent {
        TelemetryEvent {
            event_id: format!("event-{sequence_number}"),
            sample_id: format!("sample-{sequence_number}"),
            sequence_number,
            ingress: IngressDisposition::Received,
            source_time: stamp(source_ns),
            acquisition_time: stamp(acquisition_ns),
            receive_time: Some(stamp(acquisition_ns + 10)),
            processing_start_time: Some(stamp(acquisition_ns + 20)),
            processing_end_time: Some(stamp(acquisition_ns + 30)),
            decision_time: Some(stamp(acquisition_ns + 40)),
            queue_depth: 1,
            model: ModelIdentity {
                artifact_id: "model-a".to_owned(),
                revision: "rev-1".to_owned(),
                artifact_sha256: DIGEST.to_owned(),
                runtime_id: "cpu-host".to_owned(),
                verification_evidence_sha256: Some(DIGEST.to_owned()),
            },
            acknowledgement: None,
        }
    }

    fn request(events: Vec<TelemetryEvent>) -> ReplayRequest {
        ReplayRequest {
            schema_version: SCHEMA_VERSION.to_owned(),
            trace_id: "trace-a".to_owned(),
            spec: StreamSpec {
                pipeline_id: "shield-like-pipeline".to_owned(),
                diagnostic_id: "BES".to_owned(),
                reference_clock_domain_id: "ref".to_owned(),
                expected_period_ns: 100,
                max_source_period_deviation_ns: 10,
                max_source_age_ns: 200,
                deadline_from_acquisition_ns: 80,
                max_clock_uncertainty_ns: 20,
                max_queue_depth: 8,
            },
            calibrations: vec![ClockCalibration {
                calibration_id: "host-to-ref".to_owned(),
                clock_domain_id: "host".to_owned(),
                reference_clock_domain_id: "ref".to_owned(),
                anchor_clock_ns: 0,
                anchor_reference_ns: 0,
                drift_ppb: 0,
                valid_from_clock_ns: 0,
                valid_until_clock_ns: u64::MAX,
                uncertainty_ns: 2,
                evidence_sha256: DIGEST.to_owned(),
            }],
            events,
        }
    }

    #[test]
    fn on_time_event_emits_stage_latencies_and_decision_age() {
        let report = replay_trace(&request(vec![event(1, 900, 1000)])).expect("replay");
        let row = &report.body.reports[0];
        assert_eq!(row.disposition, EventDisposition::OnTimeValid);
        assert_eq!(row.metrics.source_to_acquisition_latency_ns, Some(10));
        assert_eq!(row.metrics.transport_latency_ns, Some(40));
        assert_eq!(row.metrics.queue_wait_ns, Some(20));
        assert_eq!(row.metrics.processing_latency_ns, Some(50));
        assert_eq!(row.metrics.acquisition_to_decision_ns, Some(40));
        assert_eq!(row.metrics.source_age_at_decision_ns, Some(140));
    }

    #[test]
    fn source_age_budget_marks_historical_sample_stale() {
        let mut input = request(vec![event(1, 800, 1000)]);
        input.spec.max_source_age_ns = 100;
        let report = replay_trace(&input).expect("replay");
        assert_eq!(report.body.reports[0].disposition, EventDisposition::LateValidButStaleForDecision);
    }

    #[test]
    fn acquisition_deadline_miss_is_reported_separately() {
        let mut input = event(1, 990, 1000);
        input.processing_end_time = Some(stamp(1190));
        input.decision_time = Some(stamp(1200));
        let report = replay_trace(&request(vec![input])).expect("replay");
        assert_eq!(report.body.reports[0].disposition, EventDisposition::DeadlineMissed);
        assert_eq!(report.body.reports[0].metrics.deadline_missed, Some(true));
    }

    #[test]
    fn sequence_gap_is_degraded_and_not_silently_reordered() {
        let report = replay_trace(&request(vec![event(1, 900, 1000), event(3, 1000, 1100)])).expect("replay");
        assert_eq!(report.body.reports[1].disposition, EventDisposition::Degraded);
        assert!(report.body.reports[1].metrics.sequence_gap_before_event);
        assert_eq!(report.body.summary.sequence_gaps, 1);
    }

    #[test]
    fn duplicate_and_out_of_order_sequences_are_explicit() {
        let report = replay_trace(&request(vec![
            event(5, 900, 1000),
            event(5, 1000, 1100),
            event(4, 1000, 1200),
        ])).expect("replay");
        assert_eq!(report.body.reports[1].disposition, EventDisposition::Duplicate);
        assert_eq!(report.body.reports[2].disposition, EventDisposition::OutOfOrder);
    }

    #[test]
    fn second_device_clock_is_mapped_to_reference_domain() {
        let mut input = event(1, 900, 1000);
        for stamp in [
            &mut input.source_time,
            &mut input.acquisition_time,
            input.receive_time.as_mut().unwrap(),
            input.processing_start_time.as_mut().unwrap(),
            input.processing_end_time.as_mut().unwrap(),
            input.decision_time.as_mut().unwrap(),
        ] {
            stamp.clock_domain_id = "digitizer".to_owned();
        }
        let mut request = request(vec![input]);
        request.calibrations[0].clock_domain_id = "digitizer".to_owned();
        let report = replay_trace(&request).expect("replay");
        assert_eq!(report.body.reports[0].disposition, EventDisposition::OnTimeValid);
    }

    #[test]
    fn missing_calibration_yields_unqualified_without_latency() {
        let mut input = event(1, 900, 1000);
        input.receive_time.as_mut().unwrap().calibration_id = "missing".to_owned();
        let report = replay_trace(&request(vec![input])).expect("replay");
        assert_eq!(report.body.reports[0].disposition, EventDisposition::ClockUnqualified);
        assert_eq!(report.body.reports[0].metrics.transport_latency_ns, None);
    }

    #[test]
    fn missing_processing_end_yields_timeout() {
        let mut input = event(1, 900, 1000);
        input.processing_end_time = None;
        let report = replay_trace(&request(vec![input])).expect("replay");
        assert_eq!(report.body.reports[0].disposition, EventDisposition::ProcessingTimeout);
    }

    #[test]
    fn unverified_model_cannot_receive_valid_disposition() {
        let mut input = event(1, 900, 1000);
        input.model.verification_evidence_sha256 = None;
        let report = replay_trace(&request(vec![input])).expect("replay");
        assert_eq!(report.body.reports[0].disposition, EventDisposition::ModelUnqualified);
    }

    #[test]
    fn queue_overflow_is_reported() {
        let mut input = event(1, 900, 1000);
        input.queue_depth = 9;
        let report = replay_trace(&request(vec![input])).expect("replay");
        assert_eq!(report.body.reports[0].disposition, EventDisposition::QueueOverflow);
    }

    #[test]
    fn dropped_event_needs_no_processing_timestamps() {
        let mut input = event(1, 900, 1000);
        input.ingress = IngressDisposition::Dropped;
        input.receive_time = None;
        input.processing_start_time = None;
        input.processing_end_time = None;
        input.decision_time = None;
        let report = replay_trace(&request(vec![input])).expect("replay");
        assert_eq!(report.body.reports[0].disposition, EventDisposition::Dropped);
    }

    #[test]
    fn clock_domain_mismatch_is_unqualified() {
        let mut input = event(1, 900, 1000);
        input.processing_start_time.as_mut().unwrap().clock_domain_id = "other".to_owned();
        let report = replay_trace(&request(vec![input])).expect("replay");
        assert_eq!(report.body.reports[0].disposition, EventDisposition::ClockUnqualified);
    }

    #[test]
    fn future_source_timestamp_is_unqualified() {
        let report = replay_trace(&request(vec![event(1, 1100, 1000)])).expect("replay");
        assert_eq!(report.body.reports[0].disposition, EventDisposition::ClockUnqualified);
    }

    #[test]
    fn deterministic_receipt_for_identical_trace() {
        let trace = request(vec![event(1, 900, 1000), event(2, 1000, 1100)]);
        let first = replay_trace(&trace).expect("first");
        let second = replay_trace(&trace).expect("second");
        assert_eq!(first, second);
        assert_eq!(first.receipt_sha256, second.receipt_sha256);
    }

    #[test]
    fn unknown_schema_fields_are_refused() {
        let json = r#"{"schema_version":"luminous.plasma-stream-telemetry.v0.1","trace_id":"x","unexpected":true}"#;
        assert!(serde_json::from_str::<ReplayRequest>(json).is_err());
    }
}
