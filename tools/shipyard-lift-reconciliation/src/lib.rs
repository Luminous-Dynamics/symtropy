//! Synthetic planned-versus-observed lift reconciliation.
//!
//! This crate consumes preflight, telemetry, motion and occupancy evidence.
//! It never controls a crane, issues stop/continue commands, or proves that
//! sensors or physical measurements are true.

use std::collections::{BTreeMap, BTreeSet};
use shipyard_lift_envelope::{EvidenceRef, EvidenceState};

pub const RECONCILIATION_CONTRACT_VERSION: &str = "shipyard-lift-reconciliation-v0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Channel {
    HookLoad,
    WindGust,
    CraneConfiguration,
    MotionEnvelope,
    OccupancyProjection,
    OverloadAlarm,
    HoistState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionOutcome {
    Completed,
    Aborted,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoistState {
    Hoisting,
    Lowering,
    Paused,
    Idle,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Observation {
    /// Conservative upper bound from a calibrated load measurement.
    HookLoad { upper_mn: u64 },
    /// Conservative gust upper bound from a metocean/weather observation.
    WindGust { upper_mm_per_s: u64 },
    CraneConfiguration {
        crane_id: String,
        configuration_revision: String,
    },
    MotionEnvelope {
        report_id: String,
        report_sha256: String,
        within_envelope: bool,
    },
    OccupancyProjection {
        projection_id: String,
        projection_sha256: String,
        covers_lift: bool,
    },
    OverloadAlarm { active: bool },
    HoistState { state: HoistState },
}

impl Observation {
    pub const fn channel(&self) -> Channel {
        match self {
            Self::HookLoad { .. } => Channel::HookLoad,
            Self::WindGust { .. } => Channel::WindGust,
            Self::CraneConfiguration { .. } => Channel::CraneConfiguration,
            Self::MotionEnvelope { .. } => Channel::MotionEnvelope,
            Self::OccupancyProjection { .. } => Channel::OccupancyProjection,
            Self::OverloadAlarm { .. } => Channel::OverloadAlarm,
            Self::HoistState { .. } => Channel::HoistState,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryEvent {
    pub event_id: String,
    pub source_id: String,
    pub source_revision: String,
    /// Monotonic sequence number within this channel/source stream.
    pub sequence: u64,
    /// Source event time in the declared lift-plan clock domain.
    pub event_tick: u64,
    /// Ingestion/receipt time in the same declared clock domain.
    pub received_tick: u64,
    pub clock_domain_id: String,
    pub calibration_or_source_evidence: EvidenceRef,
    pub observation: Observation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamCoverage {
    pub channel: Channel,
    pub source_id: String,
    pub source_revision: String,
    /// Inclusive first sequence represented by this coverage statement.
    pub first_sequence: u64,
    /// Exclusive end of the contiguous sequence interval.
    pub next_sequence_exclusive: u64,
    pub covered_start_tick: u64,
    /// Exclusive end of coverage. Must reach or exceed the lift end.
    pub covered_end_tick: u64,
    /// Source-owner assertion that all events in the sequence interval are present.
    /// This flag is not cryptographically authenticated by this crate.
    pub complete: bool,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionBinding {
    pub session_id: String,
    pub plan_id: String,
    pub plan_revision: String,
    pub plan_input_sha256: String,
    /// Input digest to which the preflight report says it applies.
    pub preflight_bound_input_sha256: String,
    pub preflight_report_sha256: String,
    pub block_id: String,
    pub block_revision: String,
    pub crane_id: String,
    pub crane_configuration_revision: String,
    pub site_epoch_id: String,
    pub frame_id: String,
    pub clock_domain_id: String,
    /// Half-open interval [start_tick, end_tick).
    pub start_tick: u64,
    pub end_tick: u64,
    /// Source-owned operational limit for observed hook-load upper bounds.
    pub maximum_hook_load_mn: u64,
    /// Source-owned operational gust limit for the declared lift session.
    pub maximum_gust_mm_per_s: u64,
    pub motion_report_id: String,
    pub motion_report_sha256: String,
    pub occupancy_projection_id: String,
    pub occupancy_projection_sha256: String,
    pub outcome: ExecutionOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationPolicy {
    /// Every channel listed here requires a source coverage attestation.
    pub required_channels: Vec<Channel>,
    /// Maximum permitted interval between samples for continuous channels.
    pub maximum_sample_gap_ticks: u64,
    /// Maximum event-time to ingestion-time delay.
    pub maximum_ingest_lateness_ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Indeterminate,
    Deviation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: DiagnosticSeverity,
    pub channel: Option<Channel>,
    pub subject_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownInterval {
    pub channel: Channel,
    pub start_tick: u64,
    pub end_tick: u64,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    ConformingToDeclaredEnvelope,
    DeviationObserved,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationReport {
    pub contract_version: &'static str,
    pub disposition: Disposition,
    pub session_id: String,
    pub verified_event_count: usize,
    pub exact_retransmission_count: usize,
    pub maximum_hook_load_upper_mn: Option<u64>,
    pub maximum_wind_gust_upper_mm_per_s: Option<u64>,
    pub maximum_ingest_lateness_ticks: Option<u64>,
    pub unknown_intervals: Vec<UnknownInterval>,
    pub diagnostics: Vec<Diagnostic>,
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn add_diagnostic(
    diagnostics: &mut Vec<Diagnostic>,
    code: &'static str,
    severity: DiagnosticSeverity,
    channel: Option<Channel>,
    subject_id: Option<&str>,
    detail: impl Into<String>,
) {
    diagnostics.push(Diagnostic {
        code,
        severity,
        channel,
        subject_id: subject_id.map(str::to_owned),
        detail: detail.into(),
    });
}

fn add_unknown(
    unknown: &mut Vec<UnknownInterval>,
    channel: Channel,
    start_tick: u64,
    end_tick: u64,
    reason: &'static str,
) {
    if start_tick < end_tick {
        unknown.push(UnknownInterval {
            channel,
            start_tick,
            end_tick,
            reason,
        });
    }
}

fn validate_evidence(
    evidence: &EvidenceRef,
    subject_id: &str,
    subject_revision: &str,
    clock_domain_id: &str,
    valid_from: u64,
    valid_until: u64,
    seen_evidence: &mut BTreeMap<String, EvidenceRef>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let id = evidence.source_record_id.as_str();
    if !nonblank(id)
        || !nonblank(&evidence.owner_id)
        || !nonblank(&evidence.revision)
        || !nonblank(&evidence.subject_id)
        || !nonblank(&evidence.subject_revision)
    {
        add_diagnostic(
            diagnostics,
            "EVIDENCE_IDENTITY_INCOMPLETE",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(id),
            "evidence source, owner, document revision and subject binding must be nonblank",
        );
    }
    if !valid_sha256(&evidence.sha256) {
        add_diagnostic(
            diagnostics,
            "EVIDENCE_DIGEST_INVALID",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(id),
            "evidence digest must have 64 hexadecimal characters",
        );
    }
    if evidence.subject_id != subject_id || evidence.subject_revision != subject_revision {
        add_diagnostic(
            diagnostics,
            "EVIDENCE_SUBJECT_MISMATCH",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(id),
            "evidence does not bind to the exact telemetry source and revision",
        );
    }
    if evidence.validity_clock_domain_id != clock_domain_id {
        add_diagnostic(
            diagnostics,
            "EVIDENCE_CLOCK_DOMAIN_MISMATCH",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(id),
            "evidence validity ticks are not in the execution clock domain",
        );
    }
    if evidence.valid_from_tick > valid_from
        || evidence.valid_until_tick < valid_until
        || evidence.valid_from_tick >= evidence.valid_until_tick
    {
        add_diagnostic(
            diagnostics,
            "EVIDENCE_NOT_VALID_FOR_EXECUTION",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(id),
            "evidence validity does not cover the required event/operation interval",
        );
    }
    match evidence.state {
        EvidenceState::Current => {}
        EvidenceState::Revoked => add_diagnostic(
            diagnostics,
            "EVIDENCE_REVOKED",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(id),
            "revoked evidence cannot support execution reconciliation",
        ),
        EvidenceState::Superseded => add_diagnostic(
            diagnostics,
            "EVIDENCE_SUPERSEDED",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(id),
            "superseded evidence cannot support execution reconciliation",
        ),
    }
    if nonblank(id) {
        match seen_evidence.get(id) {
            Some(existing) if existing != evidence => add_diagnostic(
                diagnostics,
                "EVIDENCE_ID_CONFLICT",
                DiagnosticSeverity::Indeterminate,
                None,
                Some(id),
                "one evidence record ID was supplied with conflicting metadata",
            ),
            Some(_) => {}
            None => {
                seen_evidence.insert(id.to_owned(), evidence.clone());
            }
        }
    }
}

fn disposition_for(diagnostics: &[Diagnostic]) -> Disposition {
    if diagnostics.iter().any(|item| item.severity == DiagnosticSeverity::Deviation) {
        // Preserve an observed deviation even if another channel is incomplete.
        Disposition::DeviationObserved
    } else if diagnostics
        .iter()
        .any(|item| item.severity == DiagnosticSeverity::Indeterminate)
    {
        Disposition::Indeterminate
    } else {
        Disposition::ConformingToDeclaredEnvelope
    }
}

fn required_channels(policy: &ReconciliationPolicy, diagnostics: &mut Vec<Diagnostic>) -> Vec<Channel> {
    let mut channels = policy.required_channels.clone();
    channels.sort();
    channels.dedup();
    if channels.len() != policy.required_channels.len() {
        add_diagnostic(
            diagnostics,
            "REQUIRED_CHANNEL_DUPLICATE",
            DiagnosticSeverity::Indeterminate,
            None,
            None,
            "required channel list must not contain duplicates",
        );
    }
    channels
}

/// Reconcile a single frozen execution session.
///
/// Events are normalised by source/sequence, exact retransmissions are
/// idempotently deduplicated, and conflicting identities/gaps remain visible.
/// The function has no control-plane outputs and cannot command a crane.
pub fn reconcile(
    binding: &ExecutionBinding,
    policy: &ReconciliationPolicy,
    coverage: &[StreamCoverage],
    events: &[TelemetryEvent],
) -> ReconciliationReport {
    let mut diagnostics = Vec::new();
    let mut unknown_intervals = Vec::new();
    let channels = required_channels(policy, &mut diagnostics);

    for (field, value) in [
        ("session_id", binding.session_id.as_str()),
        ("plan_id", binding.plan_id.as_str()),
        ("plan_revision", binding.plan_revision.as_str()),
        ("block_id", binding.block_id.as_str()),
        ("block_revision", binding.block_revision.as_str()),
        ("crane_id", binding.crane_id.as_str()),
        ("crane_configuration_revision", binding.crane_configuration_revision.as_str()),
        ("site_epoch_id", binding.site_epoch_id.as_str()),
        ("frame_id", binding.frame_id.as_str()),
        ("clock_domain_id", binding.clock_domain_id.as_str()),
        ("motion_report_id", binding.motion_report_id.as_str()),
        ("occupancy_projection_id", binding.occupancy_projection_id.as_str()),
    ] {
        if !nonblank(value) {
            add_diagnostic(
                &mut diagnostics,
                "PLAN_BINDING_FIELD_EMPTY",
                DiagnosticSeverity::Indeterminate,
                None,
                Some(&binding.session_id),
                format!("{field} must be nonblank"),
            );
        }
    }
    if binding.end_tick <= binding.start_tick {
        add_diagnostic(
            &mut diagnostics,
            "EXECUTION_INTERVAL_INVALID",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(&binding.session_id),
            "execution interval must be a non-empty half-open interval",
        );
    }
    if !valid_sha256(&binding.plan_input_sha256)
        || !valid_sha256(&binding.preflight_bound_input_sha256)
        || !valid_sha256(&binding.preflight_report_sha256)
        || !valid_sha256(&binding.motion_report_sha256)
        || !valid_sha256(&binding.occupancy_projection_sha256)
    {
        add_diagnostic(
            &mut diagnostics,
            "PLAN_OR_REPORT_DIGEST_INVALID",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(&binding.session_id),
            "plan, preflight, motion and occupancy digests must be SHA-256 lexical values",
        );
    }
    if binding.plan_input_sha256 != binding.preflight_bound_input_sha256 {
        add_diagnostic(
            &mut diagnostics,
            "PREFLIGHT_PLAN_BINDING_MISMATCH",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(&binding.session_id),
            "preflight report is bound to a different canonical plan-input digest",
        );
    }
    if binding.maximum_hook_load_mn == 0 || binding.maximum_gust_mm_per_s == 0 {
        add_diagnostic(
            &mut diagnostics,
            "EXECUTION_LIMIT_MISSING",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(&binding.session_id),
            "hook-load and gust limits must be positive source-owned values",
        );
    }
    if policy.maximum_sample_gap_ticks == 0 {
        add_diagnostic(
            &mut diagnostics,
            "SAMPLE_GAP_POLICY_INVALID",
            DiagnosticSeverity::Indeterminate,
            None,
            None,
            "maximum sample gap must be positive",
        );
    }
    if binding.outcome != ExecutionOutcome::Completed {
        add_diagnostic(
            &mut diagnostics,
            "EXECUTION_NOT_COMPLETED",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(&binding.session_id),
            "aborted or unknown sessions remain incomplete and cannot be declared conforming",
        );
    }

    let mut coverage_by_channel: BTreeMap<Channel, &StreamCoverage> = BTreeMap::new();
    for item in coverage {
        if coverage_by_channel.insert(item.channel, item).is_some() {
            add_diagnostic(
                &mut diagnostics,
                "COVERAGE_CHANNEL_DUPLICATE",
                DiagnosticSeverity::Indeterminate,
                Some(item.channel),
                Some(&item.source_id),
                "exactly one coverage statement is allowed per channel in V0.1",
            );
        }
    }

    let mut events_by_channel: BTreeMap<Channel, Vec<TelemetryEvent>> = BTreeMap::new();
    let mut event_ids: BTreeMap<String, TelemetryEvent> = BTreeMap::new();
    let mut seen_evidence: BTreeMap<String, EvidenceRef> = BTreeMap::new();
    let mut verified_event_count = 0_usize;
    let mut sequence_records: BTreeMap<(Channel, String, u64), TelemetryEvent> = BTreeMap::new();
    let mut exact_retransmissions = 0_usize;

    for event in events {
        if !nonblank(&event.event_id) || !nonblank(&event.source_id) || !nonblank(&event.source_revision) {
            add_diagnostic(
                &mut diagnostics,
                "EVENT_IDENTITY_INCOMPLETE",
                DiagnosticSeverity::Indeterminate,
                Some(event.observation.channel()),
                Some(&event.event_id),
                "event, source and source-revision identities must be nonblank",
            );
            continue;
        }
        if event.clock_domain_id != binding.clock_domain_id {
            add_diagnostic(
                &mut diagnostics,
                "EVENT_CLOCK_DOMAIN_MISMATCH",
                DiagnosticSeverity::Indeterminate,
                Some(event.observation.channel()),
                Some(&event.event_id),
                "event time and receipt time must use the execution clock domain",
            );
            add_unknown(
                &mut unknown_intervals,
                event.observation.channel(),
                binding.start_tick,
                binding.end_tick,
                "event_clock_domain_mismatch",
            );
            continue;
        }
        if event.event_tick < binding.start_tick || event.event_tick >= binding.end_tick {
            add_diagnostic(
                &mut diagnostics,
                "EVENT_OUTSIDE_EXECUTION_INTERVAL",
                DiagnosticSeverity::Indeterminate,
                Some(event.observation.channel()),
                Some(&event.event_id),
                "source event time is outside the half-open execution interval",
            );
            continue;
        }
        if event.received_tick < event.event_tick {
            add_diagnostic(
                &mut diagnostics,
                "EVENT_RECEIPT_PRECEDES_EVENT",
                DiagnosticSeverity::Indeterminate,
                Some(event.observation.channel()),
                Some(&event.event_id),
                "receipt time precedes source event time in the declared monotonic clock",
            );
        } else {
            let lateness = event.received_tick - event.event_tick;
            if lateness > policy.maximum_ingest_lateness_ticks {
                add_diagnostic(
                    &mut diagnostics,
                    "EVENT_INGESTION_TOO_LATE",
                    DiagnosticSeverity::Indeterminate,
                    Some(event.observation.channel()),
                    Some(&event.event_id),
                    "event ingestion delay exceeds the frozen lateness policy",
                );
            }
        }

        validate_evidence(
            &event.calibration_or_source_evidence,
            &event.source_id,
            &event.source_revision,
            &binding.clock_domain_id,
            event.event_tick,
            event.event_tick.saturating_add(1),
            &mut seen_evidence,
            &mut diagnostics,
        );

        match event_ids.get(&event.event_id) {
            Some(existing) if existing == event => {
                exact_retransmissions += 1;
                continue;
            }
            Some(_) => {
                add_diagnostic(
                    &mut diagnostics,
                    "EVENT_ID_CONFLICT",
                    DiagnosticSeverity::Indeterminate,
                    Some(event.observation.channel()),
                    Some(&event.event_id),
                    "same event ID was observed with conflicting event content",
                );
                continue;
            }
            None => {
                event_ids.insert(event.event_id.clone(), event.clone());
            }
        }

        let sequence_key = (event.observation.channel(), event.source_id.clone(), event.sequence);
        match sequence_records.get(&sequence_key) {
            Some(existing) if existing == event => {
                exact_retransmissions += 1;
                continue;
            }
            Some(_) => {
                add_diagnostic(
                    &mut diagnostics,
                    "EVENT_SEQUENCE_CONFLICT",
                    DiagnosticSeverity::Indeterminate,
                    Some(event.observation.channel()),
                    Some(&event.source_id),
                    "one source sequence number identifies different event payloads",
                );
                continue;
            }
            None => {
                sequence_records.insert(sequence_key, event.clone());
            }
        }
        events_by_channel
            .entry(event.observation.channel())
            .or_default()
            .push(event.clone());
        // Count after the channel is matched to its source coverage below.
    }

    let mut maximum_hook_load = None;
    let mut maximum_wind_gust = None;
    let mut maximum_lateness = None;

    for channel in &channels {
        let Some(stream) = coverage_by_channel.get(channel) else {
            add_diagnostic(
                &mut diagnostics,
                "COVERAGE_ATTESTATION_MISSING",
                DiagnosticSeverity::Indeterminate,
                Some(*channel),
                None,
                "required channel has no source-owned coverage statement",
            );
            add_unknown(
                &mut unknown_intervals,
                *channel,
                binding.start_tick,
                binding.end_tick,
                "coverage_attestation_missing",
            );
            continue;
        };
        if !stream.complete
            || stream.covered_start_tick > binding.start_tick
            || stream.covered_end_tick < binding.end_tick
            || stream.next_sequence_exclusive < stream.first_sequence
        {
            add_diagnostic(
                &mut diagnostics,
                "COVERAGE_INCOMPLETE",
                DiagnosticSeverity::Indeterminate,
                Some(*channel),
                Some(&stream.source_id),
                "source coverage must be complete and span the full execution interval",
            );
            add_unknown(
                &mut unknown_intervals,
                *channel,
                binding.start_tick,
                binding.end_tick,
                "coverage_incomplete",
            );
        }
        validate_evidence(
            &stream.evidence,
            &stream.source_id,
            &stream.source_revision,
            &binding.clock_domain_id,
            binding.start_tick,
            binding.end_tick,
            &mut seen_evidence,
            &mut diagnostics,
        );

        let mut channel_events = events_by_channel.remove(channel).unwrap_or_default();
        channel_events.sort_by(|a, b| {
            a.source_id
                .cmp(&b.source_id)
                .then_with(|| a.sequence.cmp(&b.sequence))
                .then_with(|| a.event_id.cmp(&b.event_id))
        });
        for event in &channel_events {
            if event.source_id != stream.source_id || event.source_revision != stream.source_revision {
                add_diagnostic(
                    &mut diagnostics,
                    "EVENT_SOURCE_MISMATCH",
                    DiagnosticSeverity::Indeterminate,
                    Some(*channel),
                    Some(&event.event_id),
                    "event source identity/revision does not match the channel coverage statement",
                );
            } else {
                maximum_lateness = Some(
                    maximum_lateness
                        .unwrap_or(0_u64)
                        .max(event.received_tick.saturating_sub(event.event_tick)),
                );
            }
        }
        channel_events.retain(|event| {
            event.source_id == stream.source_id
                && event.source_revision == stream.source_revision
        });
        verified_event_count += channel_events.len();

        let unique_sequences: BTreeSet<u64> = channel_events.iter().map(|e| e.sequence).collect();
        let expected_sequence_count = stream
            .next_sequence_exclusive
            .saturating_sub(stream.first_sequence);
        if stream.next_sequence_exclusive < stream.first_sequence
            || unique_sequences.len() as u64 != expected_sequence_count
            || unique_sequences
                .iter()
                .copied()
                .ne(stream.first_sequence..stream.next_sequence_exclusive)
        {
            add_diagnostic(
                &mut diagnostics,
                "EVENT_SEQUENCE_GAP",
                DiagnosticSeverity::Indeterminate,
                Some(*channel),
                Some(&stream.source_id),
                "observed unique sequences do not exactly cover the attested contiguous sequence interval",
            );
            add_unknown(
                &mut unknown_intervals,
                *channel,
                binding.start_tick,
                binding.end_tick,
                "event_sequence_gap",
            );
        }

        let must_have_sample = matches!(
            channel,
            Channel::HookLoad
                | Channel::WindGust
                | Channel::CraneConfiguration
                | Channel::MotionEnvelope
                | Channel::OccupancyProjection
                | Channel::HoistState
        );
        if must_have_sample && channel_events.is_empty() {
            add_diagnostic(
                &mut diagnostics,
                "REQUIRED_CHANNEL_HAS_NO_EVENTS",
                DiagnosticSeverity::Indeterminate,
                Some(*channel),
                Some(&stream.source_id),
                "required channel has no event in the execution interval",
            );
            add_unknown(
                &mut unknown_intervals,
                *channel,
                binding.start_tick,
                binding.end_tick,
                "required_channel_empty",
            );
        }

        for pair in channel_events.windows(2) {
            if pair[1].event_tick < pair[0].event_tick {
                add_diagnostic(
                    &mut diagnostics,
                    "SOURCE_EVENT_TIME_REGRESSION",
                    DiagnosticSeverity::Indeterminate,
                    Some(*channel),
                    Some(&stream.source_id),
                    "source event time regresses as sequence number increases",
                );
            }
        }

        if matches!(channel, Channel::HookLoad | Channel::WindGust) {
            let mut ticks: Vec<u64> = channel_events.iter().map(|e| e.event_tick).collect();
            ticks.sort_unstable();
            if let Some(first) = ticks.first().copied() {
                if first.saturating_sub(binding.start_tick) > policy.maximum_sample_gap_ticks {
                    add_unknown(
                        &mut unknown_intervals,
                        *channel,
                        binding.start_tick,
                        first,
                        "leading_sample_gap_exceeds_policy",
                    );
                    add_diagnostic(
                        &mut diagnostics,
                        "SAMPLE_GAP_EXCEEDED",
                        DiagnosticSeverity::Indeterminate,
                        Some(*channel),
                        Some(&stream.source_id),
                        "leading sample gap exceeds the frozen policy",
                    );
                }
                for pair in ticks.windows(2) {
                    if pair[1].saturating_sub(pair[0]) > policy.maximum_sample_gap_ticks {
                        add_unknown(
                            &mut unknown_intervals,
                            *channel,
                            pair[0],
                            pair[1],
                            "inter_sample_gap_exceeds_policy",
                        );
                        add_diagnostic(
                            &mut diagnostics,
                            "SAMPLE_GAP_EXCEEDED",
                            DiagnosticSeverity::Indeterminate,
                            Some(*channel),
                            Some(&stream.source_id),
                            "inter-sample gap exceeds the frozen policy",
                        );
                    }
                }
                let trailing = binding.end_tick.saturating_sub(*ticks.last().unwrap_or(&first));
                if trailing > policy.maximum_sample_gap_ticks {
                    add_unknown(
                        &mut unknown_intervals,
                        *channel,
                        *ticks.last().unwrap_or(&first),
                        binding.end_tick,
                        "trailing_sample_gap_exceeds_policy",
                    );
                    add_diagnostic(
                        &mut diagnostics,
                        "SAMPLE_GAP_EXCEEDED",
                        DiagnosticSeverity::Indeterminate,
                        Some(*channel),
                        Some(&stream.source_id),
                        "trailing sample gap exceeds the frozen policy",
                    );
                }
            }
        }

        for event in channel_events {
            match event.observation {
                Observation::HookLoad { upper_mn } => {
                    maximum_hook_load = Some(maximum_hook_load.unwrap_or(0).max(upper_mn));
                    if upper_mn > binding.maximum_hook_load_mn {
                        add_diagnostic(
                            &mut diagnostics,
                            "HOOK_LOAD_LIMIT_EXCEEDED",
                            DiagnosticSeverity::Deviation,
                            Some(Channel::HookLoad),
                            Some(&event.event_id),
                            "conservative observed hook-load upper bound exceeds the declared limit",
                        );
                    }
                }
                Observation::WindGust { upper_mm_per_s } => {
                    maximum_wind_gust =
                        Some(maximum_wind_gust.unwrap_or(0).max(upper_mm_per_s));
                    if upper_mm_per_s > binding.maximum_gust_mm_per_s {
                        add_diagnostic(
                            &mut diagnostics,
                            "WIND_GUST_LIMIT_EXCEEDED",
                            DiagnosticSeverity::Deviation,
                            Some(Channel::WindGust),
                            Some(&event.event_id),
                            "conservative observed gust upper bound exceeds the declared operation limit",
                        );
                    }
                }
                Observation::CraneConfiguration {
                    crane_id,
                    configuration_revision,
                } => {
                    if crane_id != binding.crane_id
                        || configuration_revision != binding.crane_configuration_revision
                    {
                        add_diagnostic(
                            &mut diagnostics,
                            "CRANE_CONFIGURATION_DEVIATION",
                            DiagnosticSeverity::Deviation,
                            Some(Channel::CraneConfiguration),
                            Some(&event.event_id),
                            "observed crane identity/configuration differs from the bound lift plan",
                        );
                    }
                }
                Observation::MotionEnvelope {
                    report_id,
                    report_sha256,
                    within_envelope,
                } => {
                    if report_id != binding.motion_report_id
                        || report_sha256 != binding.motion_report_sha256
                    {
                        add_diagnostic(
                            &mut diagnostics,
                            "MOTION_REPORT_BINDING_MISMATCH",
                            DiagnosticSeverity::Indeterminate,
                            Some(Channel::MotionEnvelope),
                            Some(&event.event_id),
                            "observed motion evidence references a different report or digest",
                        );
                    } else if !within_envelope {
                        add_diagnostic(
                            &mut diagnostics,
                            "MOTION_ENVELOPE_DEVIATION",
                            DiagnosticSeverity::Deviation,
                            Some(Channel::MotionEnvelope),
                            Some(&event.event_id),
                            "independent motion-envelope evidence reports an excursion",
                        );
                    }
                }
                Observation::OccupancyProjection {
                    projection_id,
                    projection_sha256,
                    covers_lift,
                } => {
                    if projection_id != binding.occupancy_projection_id
                        || projection_sha256 != binding.occupancy_projection_sha256
                    {
                        add_diagnostic(
                            &mut diagnostics,
                            "OCCUPANCY_BINDING_MISMATCH",
                            DiagnosticSeverity::Indeterminate,
                            Some(Channel::OccupancyProjection),
                            Some(&event.event_id),
                            "observed occupancy evidence references a different projection or digest",
                        );
                    } else if !covers_lift {
                        add_diagnostic(
                            &mut diagnostics,
                            "OCCUPANCY_DEVIATION",
                            DiagnosticSeverity::Deviation,
                            Some(Channel::OccupancyProjection),
                            Some(&event.event_id),
                            "independent occupancy evidence reports a conflict or uncovered interval",
                        );
                    }
                }
                Observation::OverloadAlarm { active: true } => {
                    add_diagnostic(
                        &mut diagnostics,
                        "OVERLOAD_ALARM_ACTIVE",
                        DiagnosticSeverity::Deviation,
                        Some(Channel::OverloadAlarm),
                        Some(&event.event_id),
                        "crane telemetry reports an active overload alarm",
                    );
                }
                Observation::OverloadAlarm { active: false } => {}
                Observation::HoistState { state: HoistState::Unknown } => {
                    add_diagnostic(
                        &mut diagnostics,
                        "HOIST_STATE_UNKNOWN",
                        DiagnosticSeverity::Indeterminate,
                        Some(Channel::HoistState),
                        Some(&event.event_id),
                        "hoist state is not classified by the admitted source",
                    );
                }
                Observation::HoistState { .. } => {}
            }
        }
    }

    for channel in coverage_by_channel.keys() {
        if !channels.contains(channel) {
            add_diagnostic(
                &mut diagnostics,
                "UNDECLARED_COVERAGE_CHANNEL",
                DiagnosticSeverity::Indeterminate,
                Some(*channel),
                None,
                "coverage supplied for a channel not declared by the frozen policy",
            );
        }
    }
    for channel in events_by_channel.keys() {
        if !channels.contains(channel) {
            add_diagnostic(
                &mut diagnostics,
                "UNDECLARED_EVENT_CHANNEL",
                DiagnosticSeverity::Indeterminate,
                Some(*channel),
                None,
                "event supplied for a channel not declared by the frozen policy",
            );
        }
    }

    diagnostics.sort_by(|a, b| {
        a.code
            .cmp(b.code)
            .then_with(|| a.channel.cmp(&b.channel))
            .then_with(|| a.subject_id.cmp(&b.subject_id))
            .then_with(|| a.detail.cmp(&b.detail))
    });
    unknown_intervals.sort_by(|a, b| {
        a.channel
            .cmp(&b.channel)
            .then_with(|| a.start_tick.cmp(&b.start_tick))
            .then_with(|| a.end_tick.cmp(&b.end_tick))
            .then_with(|| a.reason.cmp(b.reason))
    });
    unknown_intervals.dedup();

    ReconciliationReport {
        contract_version: RECONCILIATION_CONTRACT_VERSION,
        disposition: disposition_for(&diagnostics),
        session_id: binding.session_id.clone(),
        verified_event_count,
        exact_retransmission_count: exact_retransmissions,
        maximum_hook_load_upper_mn: maximum_hook_load,
        maximum_wind_gust_upper_mm_per_s: maximum_wind_gust,
        maximum_ingest_lateness_ticks: maximum_lateness,
        unknown_intervals,
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(id: &str, subject: &str, revision: &str) -> EvidenceRef {
        EvidenceRef {
            source_record_id: id.to_owned(),
            owner_id: format!("owner::{id}"),
            revision: "r1".to_owned(),
            subject_id: subject.to_owned(),
            subject_revision: revision.to_owned(),
            sha256: "a".repeat(64),
            validity_clock_domain_id: "yard-clock".to_owned(),
            valid_from_tick: 0,
            valid_until_tick: 1_000,
            state: EvidenceState::Current,
        }
    }

    fn binding() -> ExecutionBinding {
        ExecutionBinding {
            session_id: "session-001".to_owned(),
            plan_id: "plan-001".to_owned(),
            plan_revision: "plan-r4".to_owned(),
            plan_input_sha256: "1".repeat(64),
            preflight_bound_input_sha256: "1".repeat(64),
            preflight_report_sha256: "2".repeat(64),
            block_id: "block-001".to_owned(),
            block_revision: "block-r7".to_owned(),
            crane_id: "gantry-01".to_owned(),
            crane_configuration_revision: "gantry-r3".to_owned(),
            site_epoch_id: "yard-epoch-2".to_owned(),
            frame_id: "yard-frame-mm".to_owned(),
            clock_domain_id: "yard-clock".to_owned(),
            start_tick: 100,
            end_tick: 200,
            maximum_hook_load_mn: 500_000_000,
            maximum_gust_mm_per_s: 10_000,
            motion_report_id: "motion-01".to_owned(),
            motion_report_sha256: "3".repeat(64),
            occupancy_projection_id: "occupancy-01".to_owned(),
            occupancy_projection_sha256: "4".repeat(64),
            outcome: ExecutionOutcome::Completed,
        }
    }

    fn policy() -> ReconciliationPolicy {
        ReconciliationPolicy {
            required_channels: vec![
                Channel::HookLoad,
                Channel::WindGust,
                Channel::CraneConfiguration,
                Channel::MotionEnvelope,
                Channel::OccupancyProjection,
                Channel::OverloadAlarm,
                Channel::HoistState,
            ],
            maximum_sample_gap_ticks: 60,
            maximum_ingest_lateness_ticks: 10,
        }
    }

    fn channel_name(channel: Channel) -> (&'static str, &'static str) {
        match channel {
            Channel::HookLoad => ("load-cell-01", "load-cell-r2"),
            Channel::WindGust => ("wind-sensor-01", "wind-sensor-r3"),
            Channel::CraneConfiguration => ("crane-controller-01", "controller-r5"),
            Channel::MotionEnvelope => ("motion-oracle-01", "oracle-r1"),
            Channel::OccupancyProjection => ("occupancy-oracle-01", "oracle-r1"),
            Channel::OverloadAlarm => ("overload-relay-01", "relay-r2"),
            Channel::HoistState => ("hoist-controller-01", "controller-r5"),
        }
    }

    fn stream(channel: Channel) -> StreamCoverage {
        let (source_id, source_revision) = channel_name(channel);
        StreamCoverage {
            channel,
            source_id: source_id.to_owned(),
            source_revision: source_revision.to_owned(),
            first_sequence: 0,
            next_sequence_exclusive: 1,
            covered_start_tick: 100,
            covered_end_tick: 200,
            complete: true,
            evidence: evidence(
                &format!("coverage::{source_id}"),
                source_id,
                source_revision,
            ),
        }
    }

    fn event(
        channel: Channel,
        sequence: u64,
        tick: u64,
        observation: Observation,
    ) -> TelemetryEvent {
        let (source_id, source_revision) = channel_name(channel);
        TelemetryEvent {
            event_id: format!("event::{channel:?}::{sequence}"),
            source_id: source_id.to_owned(),
            source_revision: source_revision.to_owned(),
            sequence,
            event_tick: tick,
            received_tick: tick + 2,
            clock_domain_id: "yard-clock".to_owned(),
            calibration_or_source_evidence: evidence(
                &format!("calibration::{source_id}"),
                source_id,
                source_revision,
            ),
            observation,
        }
    }

    fn baseline() -> (ExecutionBinding, ReconciliationPolicy, Vec<StreamCoverage>, Vec<TelemetryEvent>) {
        let binding = binding();
        let policy = policy();
        let coverage = policy.required_channels.iter().copied().map(stream).collect();
        let events = vec![
            event(Channel::HookLoad, 0, 110, Observation::HookLoad { upper_mn: 400_000_000 }),
            event(Channel::HookLoad, 1, 150, Observation::HookLoad { upper_mn: 410_000_000 }),
            event(Channel::HookLoad, 2, 190, Observation::HookLoad { upper_mn: 405_000_000 }),
            event(Channel::WindGust, 0, 110, Observation::WindGust { upper_mm_per_s: 8_000 }),
            event(Channel::WindGust, 1, 150, Observation::WindGust { upper_mm_per_s: 9_000 }),
            event(Channel::WindGust, 2, 190, Observation::WindGust { upper_mm_per_s: 8_500 }),
            event(Channel::CraneConfiguration, 0, 110, Observation::CraneConfiguration {
                crane_id: "gantry-01".to_owned(),
                configuration_revision: "gantry-r3".to_owned(),
            }),
            event(Channel::MotionEnvelope, 0, 150, Observation::MotionEnvelope {
                report_id: "motion-01".to_owned(),
                report_sha256: "3".repeat(64),
                within_envelope: true,
            }),
            event(Channel::OccupancyProjection, 0, 150, Observation::OccupancyProjection {
                projection_id: "occupancy-01".to_owned(),
                projection_sha256: "4".repeat(64),
                covers_lift: true,
            }),
            event(Channel::OverloadAlarm, 0, 110, Observation::OverloadAlarm { active: false }),
            event(Channel::HoistState, 0, 110, Observation::HoistState { state: HoistState::Hoisting }),
            event(Channel::HoistState, 1, 190, Observation::HoistState { state: HoistState::Lowering }),
        ];
        let mut coverage = coverage;
        // Every source stream declares the actual sequence window in this synthetic session.
        for item in &mut coverage {
            let count = events.iter().filter(|e| e.observation.channel() == item.channel).count() as u64;
            item.next_sequence_exclusive = count;
        }
        (binding, policy, coverage, events)
    }

    fn has_code(report: &ReconciliationReport, code: &str) -> bool {
        report.diagnostics.iter().any(|d| d.code == code)
    }

    #[test]
    fn mismatched_source_cannot_satisfy_attested_sequence_coverage() {
        let (binding, policy, mut coverage, mut events) = baseline();
        let stream = coverage
            .iter_mut()
            .find(|item| item.channel == Channel::HookLoad)
            .unwrap();
        stream.source_id = "replacement-load-cell".to_owned();
        stream.source_revision = "replacement-r2".to_owned();
        stream.evidence = evidence("replacement-cal", "replacement-load-cell", "replacement-r2");
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVENT_SOURCE_MISMATCH"));
        assert!(has_code(&report, "EVENT_SEQUENCE_GAP"));
        assert!(report.maximum_hook_load_upper_mn.is_none());

        // Also exercise the reverse direction: an unbound source cannot fill a gap.
        events.retain(|event| event.observation.channel() != Channel::HookLoad);
        events.push(event(
            Channel::HookLoad,
            0,
            110,
            Observation::HookLoad { upper_mn: 100_000_000 },
        ));
        let second = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(second.disposition, Disposition::Indeterminate);
    }

    #[test]
    fn conflicting_source_evidence_record_identity_is_indeterminate() {
        let (binding, policy, mut coverage, events) = baseline();
        let item = coverage
            .iter_mut()
            .find(|item| item.channel == Channel::HookLoad)
            .unwrap();
        item.evidence.source_record_id = "shared-record".to_owned();
        let wind = coverage
            .iter_mut()
            .find(|item| item.channel == Channel::WindGust)
            .unwrap();
        wind.evidence.source_record_id = "shared-record".to_owned();
        wind.evidence.subject_id = wind.source_id.clone();
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_ID_CONFLICT"));
    }

    #[test]
    fn missing_required_motion_event_is_indeterminate_even_with_complete_empty_stream() {
        let (binding, policy, mut coverage, mut events) = baseline();
        events.retain(|event| event.observation.channel() != Channel::MotionEnvelope);
        coverage
            .iter_mut()
            .find(|item| item.channel == Channel::MotionEnvelope)
            .unwrap()
            .next_sequence_exclusive = 0;
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "REQUIRED_CHANNEL_HAS_NO_EVENTS"));
    }

    #[test]
    fn occupancy_conflict_is_a_deviation() {
        let (binding, policy, coverage, mut events) = baseline();
        let event = events
            .iter_mut()
            .find(|event| event.observation.channel() == Channel::OccupancyProjection)
            .unwrap();
        event.observation = Observation::OccupancyProjection {
            projection_id: "occupancy-01".to_owned(),
            projection_sha256: "4".repeat(64),
            covers_lift: false,
        };
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::DeviationObserved);
        assert!(has_code(&report, "OCCUPANCY_DEVIATION"));
    }

    #[test]
    fn exact_limit_equality_is_conforming_but_unknown_interval_is_not_hidden() {
        let (mut binding, policy, mut coverage, mut events) = baseline();
        binding.maximum_hook_load_mn = 410_000_000;
        for event in &mut events {
            if let Observation::HookLoad { upper_mn } = &mut event.observation {
                *upper_mn = 410_000_000;
            }
        }
        coverage
            .iter_mut()
            .find(|item| item.channel == Channel::WindGust)
            .unwrap()
            .complete = false;
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "COVERAGE_INCOMPLETE"));
        assert!(!has_code(&report, "HOOK_LOAD_LIMIT_EXCEEDED"));
    }

    #[test]
    fn complete_bounded_session_is_conforming_to_declared_envelope() {
        let (binding, policy, coverage, events) = baseline();
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::ConformingToDeclaredEnvelope);
        assert_eq!(report.verified_event_count, events.len());
        assert_eq!(report.maximum_hook_load_upper_mn, Some(410_000_000));
        assert_eq!(report.maximum_wind_gust_upper_mm_per_s, Some(9_000));
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn load_one_unit_above_limit_is_a_deviation() {
        let (binding, policy, coverage, mut events) = baseline();
        if let Observation::HookLoad { upper_mn } = &mut events[0].observation {
            *upper_mn = binding.maximum_hook_load_mn + 1;
        }
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::DeviationObserved);
        assert!(has_code(&report, "HOOK_LOAD_LIMIT_EXCEEDED"));
    }

    #[test]
    fn gust_one_unit_above_limit_is_a_deviation() {
        let (binding, policy, coverage, mut events) = baseline();
        let event = events.iter_mut().find(|e| e.observation.channel() == Channel::WindGust).unwrap();
        event.observation = Observation::WindGust {
            upper_mm_per_s: binding.maximum_gust_mm_per_s + 1,
        };
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::DeviationObserved);
        assert!(has_code(&report, "WIND_GUST_LIMIT_EXCEEDED"));
    }

    #[test]
    fn exact_limit_equality_is_not_an_exceedance() {
        let (mut binding, policy, coverage, mut events) = baseline();
        binding.maximum_hook_load_mn = 410_000_000;
        binding.maximum_gust_mm_per_s = 9_000;
        for event in &mut events {
            match &mut event.observation {
                Observation::HookLoad { upper_mn } => *upper_mn = 410_000_000,
                Observation::WindGust { upper_mm_per_s } => *upper_mm_per_s = 9_000,
                _ => {}
            }
        }
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::ConformingToDeclaredEnvelope);
    }

    #[test]
    fn missing_stream_coverage_is_indeterminate_not_conforming() {
        let (binding, policy, mut coverage, events) = baseline();
        coverage.retain(|item| item.channel != Channel::HookLoad);
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "COVERAGE_ATTESTATION_MISSING"));
    }

    #[test]
    fn incomplete_stream_coverage_is_indeterminate() {
        let (binding, policy, mut coverage, events) = baseline();
        coverage.iter_mut().find(|item| item.channel == Channel::HookLoad).unwrap().complete = false;
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "COVERAGE_INCOMPLETE"));
    }

    #[test]
    fn sequence_gap_is_indeterminate() {
        let (binding, policy, mut coverage, mut events) = baseline();
        let selected = events.iter_mut().find(|e| e.observation.channel() == Channel::HookLoad && e.sequence == 1).unwrap();
        selected.sequence = 4;
        coverage.iter_mut().find(|item| item.channel == Channel::HookLoad).unwrap().next_sequence_exclusive = 5;
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVENT_SEQUENCE_GAP"));
    }

    #[test]
    fn exact_event_retransmission_is_idempotently_deduplicated() {
        let (binding, policy, coverage, mut events) = baseline();
        events.push(events[0].clone());
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::ConformingToDeclaredEnvelope);
        assert_eq!(report.exact_retransmission_count, 1);
        assert_eq!(report.verified_event_count, events.len() - 1);
    }

    #[test]
    fn same_event_id_with_conflicting_payload_is_indeterminate() {
        let (binding, policy, coverage, mut events) = baseline();
        let mut conflicting = events[0].clone();
        if let Observation::HookLoad { upper_mn } = &mut conflicting.observation {
            *upper_mn += 1;
        }
        events.push(conflicting);
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVENT_ID_CONFLICT"));
    }

    #[test]
    fn sensor_calibration_for_wrong_source_revision_is_indeterminate() {
        let (binding, policy, coverage, mut events) = baseline();
        events[0].calibration_or_source_evidence.subject_revision = "old-calibration".to_owned();
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_SUBJECT_MISMATCH"));
    }

    #[test]
    fn sample_gap_beyond_policy_creates_unknown_interval() {
        let (binding, policy, coverage, mut events) = baseline();
        events.retain(|e| !(e.observation.channel() == Channel::HookLoad && e.sequence == 1));
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVENT_SEQUENCE_GAP"));
        assert!(has_code(&report, "SAMPLE_GAP_EXCEEDED"));
        assert!(!report.unknown_intervals.is_empty());
    }

    #[test]
    fn crane_configuration_change_is_a_deviation() {
        let (binding, policy, coverage, mut events) = baseline();
        let event = events.iter_mut().find(|e| e.observation.channel() == Channel::CraneConfiguration).unwrap();
        event.observation = Observation::CraneConfiguration {
            crane_id: "gantry-01".to_owned(),
            configuration_revision: "gantry-r4".to_owned(),
        };
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::DeviationObserved);
        assert!(has_code(&report, "CRANE_CONFIGURATION_DEVIATION"));
    }

    #[test]
    fn motion_excursion_is_a_deviation_even_without_controller_alarm() {
        let (binding, policy, coverage, mut events) = baseline();
        let event = events.iter_mut().find(|e| e.observation.channel() == Channel::MotionEnvelope).unwrap();
        event.observation = Observation::MotionEnvelope {
            report_id: "motion-01".to_owned(),
            report_sha256: "3".repeat(64),
            within_envelope: false,
        };
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::DeviationObserved);
        assert!(has_code(&report, "MOTION_ENVELOPE_DEVIATION"));
    }

    #[test]
    fn active_overload_alarm_is_a_deviation_even_when_load_sensor_is_below_limit() {
        let (binding, policy, coverage, mut events) = baseline();
        let event = events.iter_mut().find(|e| e.observation.channel() == Channel::OverloadAlarm).unwrap();
        event.observation = Observation::OverloadAlarm { active: true };
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::DeviationObserved);
        assert!(has_code(&report, "OVERLOAD_ALARM_ACTIVE"));
    }

    #[test]
    fn preflight_must_be_bound_to_exact_plan_input_digest() {
        let (mut binding, policy, coverage, events) = baseline();
        binding.preflight_bound_input_sha256 = "f".repeat(64);
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "PREFLIGHT_PLAN_BINDING_MISMATCH"));
    }

    #[test]
    fn missing_or_delayed_sensor_evidence_is_not_a_clean_execution() {
        let (binding, policy, coverage, mut events) = baseline();
        events[0].received_tick = events[0].event_tick + policy.maximum_ingest_lateness_ticks + 1;
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVENT_INGESTION_TOO_LATE"));
    }

    #[test]
    fn completed_abort_unknown_distinctions_are_preserved_as_indeterminate() {
        let (mut binding, policy, coverage, events) = baseline();
        binding.outcome = ExecutionOutcome::Aborted;
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EXECUTION_NOT_COMPLETED"));
    }

    #[test]
    fn permitted_transport_ordering_does_not_change_report() {
        let (binding, policy, coverage, events) = baseline();
        let first = reconcile(&binding, &policy, &coverage, &events);
        let mut permuted = events.clone();
        permuted.reverse();
        let second = reconcile(&binding, &policy, &coverage, &permuted);
        assert_eq!(first, second);
    }

    #[test]
    fn clock_domain_mismatch_is_indeterminate() {
        let (binding, policy, coverage, mut events) = baseline();
        events[0].clock_domain_id = "unknown-clock".to_owned();
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVENT_CLOCK_DOMAIN_MISMATCH"));
    }

    #[test]
    fn duplicate_sequence_with_new_event_id_is_indeterminate() {
        let (binding, policy, coverage, mut events) = baseline();
        let mut duplicate = events[0].clone();
        duplicate.event_id = "different-event-id".to_owned();
        events.push(duplicate);
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVENT_SEQUENCE_CONFLICT"));
    }
}
