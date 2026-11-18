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
    pub source_clock_domain_id: String,
    pub clock_mapping_source_domain_id: String,
    pub clock_mapping_target_domain_id: String,
    pub clock_mapping_id: String,
    pub clock_mapping_revision: String,
    /// Closed uncertainty interval for event time after source-to-yard mapping.
    pub event_time_lower_tick: u64,
    pub event_time_upper_tick: u64,
    /// Ingestion/receipt time in the target execution clock domain.
    pub received_tick: u64,
    pub clock_domain_id: String,
    pub calibration_or_source_evidence: EvidenceRef,
    /// Evidence for the source-clock to execution-clock mapping used above.
    pub clock_mapping_evidence: EvidenceRef,
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
    pub clock_domain_id: String,
    pub covered_start_tick: u64,
    /// Exclusive end of coverage. Must reach or exceed the lift end.
    pub covered_end_tick: u64,
    /// Source-owner assertion that all events in the sequence interval are present.
    /// This flag is not cryptographically authenticated by this crate.
    pub complete: bool,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreflightDisposition {
    ReadyForHumanReview,
    Rejected,
    Indeterminate,
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
    /// SHA-256 of the canonical raw telemetry ledger supplied to this run.
    pub event_ledger_sha256: String,
    /// SHA-256 of the canonical source-coverage bundle supplied to this run.
    pub coverage_bundle_sha256: String,
    pub preflight_disposition: PreflightDisposition,
    /// Separately authenticated by the owner-authority workflow before admission.
    pub authorisation_id: String,
    pub authorisation_revision: String,
    pub authorisation_evidence: EvidenceRef,
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
    pub hook_load_limit_evidence: EvidenceRef,
    /// Source-owned operational gust limit profile for the declared lift session.
    pub maximum_gust_mm_per_s: u64,
    pub gust_limit_profile_id: String,
    pub gust_limit_profile_revision: String,
    pub gust_limit_evidence: EvidenceRef,
    pub motion_report_id: String,
    pub motion_report_sha256: String,
    pub occupancy_projection_id: String,
    pub occupancy_projection_sha256: String,
    pub outcome: ExecutionOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationPolicy {
    pub policy_id: String,
    pub policy_revision: String,
    pub policy_sha256: String,
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
    pub plan_id: String,
    pub plan_revision: String,
    pub plan_input_sha256: String,
    pub preflight_report_sha256: String,
    pub event_ledger_sha256: String,
    pub coverage_bundle_sha256: String,
    pub authorisation_id: String,
    pub authorisation_revision: String,
    pub preflight_disposition: PreflightDisposition,
    pub block_id: String,
    pub block_revision: String,
    pub crane_id: String,
    pub crane_configuration_revision: String,
    pub site_epoch_id: String,
    pub frame_id: String,
    pub clock_domain_id: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub maximum_hook_load_mn: u64,
    pub maximum_gust_mm_per_s: u64,
    pub motion_report_id: String,
    pub motion_report_sha256: String,
    pub occupancy_projection_id: String,
    pub occupancy_projection_sha256: String,
    pub execution_outcome: ExecutionOutcome,
    pub policy_id: String,
    pub policy_revision: String,
    pub policy_sha256: String,
    pub matched_event_count: usize,
    pub exact_retransmission_count: usize,
    pub maximum_hook_load_upper_mn: Option<u64>,
    pub maximum_wind_gust_upper_mm_per_s: Option<u64>,
    pub maximum_ingest_lateness_ticks: Option<u64>,
    pub unknown_intervals: Vec<UnknownInterval>,
    pub diagnostics: Vec<Diagnostic>,
}

fn json_quote(input: &str) -> String {
    use std::fmt::Write as _;

    let mut output = String::with_capacity(input.len() + 2);
    output.push('"');
    for character in input.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{001f}' => {
                let _ = write!(output, "\\u{:04x}", character as u32);
            }
            character => output.push(character),
        }
    }
    output.push('"');
    output
}

fn evidence_json(evidence: &EvidenceRef) -> String {
    let state = match evidence.state {
        EvidenceState::Current => "current",
        EvidenceState::Revoked => "revoked",
        EvidenceState::Superseded => "superseded",
    };
    format!(
        "{{\"source_record_id\":{},\"owner_id\":{},\"revision\":{},\"subject_id\":{},\"subject_revision\":{},\"sha256\":{},\"validity_clock_domain_id\":{},\"valid_from_tick\":{},\"valid_until_tick\":{},\"state\":{}}}",
        json_quote(&evidence.source_record_id),
        json_quote(&evidence.owner_id),
        json_quote(&evidence.revision),
        json_quote(&evidence.subject_id),
        json_quote(&evidence.subject_revision),
        json_quote(&evidence.sha256.to_ascii_lowercase()),
        json_quote(&evidence.validity_clock_domain_id),
        evidence.valid_from_tick,
        evidence.valid_until_tick,
        json_quote(state),
    )
}

fn observation_json(observation: &Observation) -> String {
    match observation {
        Observation::HookLoad { upper_mn } => {
            format!("{{\"kind\":\"hook_load\",\"upper_mn\":{upper_mn}}}")
        }
        Observation::WindGust { upper_mm_per_s } => {
            format!("{{\"kind\":\"wind_gust\",\"upper_mm_per_s\":{upper_mm_per_s}}}")
        }
        Observation::CraneConfiguration { crane_id, configuration_revision } => format!(
            "{{\"kind\":\"crane_configuration\",\"crane_id\":{},\"configuration_revision\":{}}}",
            json_quote(crane_id),
            json_quote(configuration_revision),
        ),
        Observation::MotionEnvelope { report_id, report_sha256, within_envelope } => format!(
            "{{\"kind\":\"motion_envelope\",\"report_id\":{},\"report_sha256\":{},\"within_envelope\":{}}}",
            json_quote(report_id),
            json_quote(&report_sha256.to_ascii_lowercase()),
            within_envelope,
        ),
        Observation::OccupancyProjection { projection_id, projection_sha256, covers_lift } => format!(
            "{{\"kind\":\"occupancy_projection\",\"projection_id\":{},\"projection_sha256\":{},\"covers_lift\":{}}}",
            json_quote(projection_id),
            json_quote(&projection_sha256.to_ascii_lowercase()),
            covers_lift,
        ),
        Observation::OverloadAlarm { active } => {
            format!("{{\"kind\":\"overload_alarm\",\"active\":{active}}}")
        }
        Observation::HoistState { state } => {
            let name = match state {
                HoistState::Hoisting => "hoisting",
                HoistState::Lowering => "lowering",
                HoistState::Paused => "paused",
                HoistState::Idle => "idle",
                HoistState::Unknown => "unknown",
            };
            format!("{{\"kind\":\"hoist_state\",\"state\":{}}}", json_quote(name))
        }
    }
}

impl TelemetryEvent {
    pub fn canonical_json(&self) -> String {
        format!(
            "{{\"event_id\":{},\"source_id\":{},\"source_revision\":{},\"sequence\":{},\"source_clock_domain_id\":{},\"clock_mapping_source_domain_id\":{},\"clock_mapping_target_domain_id\":{},\"clock_mapping_id\":{},\"clock_mapping_revision\":{},\"event_time_lower_tick\":{},\"event_time_upper_tick\":{},\"received_tick\":{},\"clock_domain_id\":{},\"calibration_or_source_evidence\":{},\"clock_mapping_evidence\":{},\"observation\":{}}}",
            json_quote(&self.event_id),
            json_quote(&self.source_id),
            json_quote(&self.source_revision),
            self.sequence,
            json_quote(&self.source_clock_domain_id),
            json_quote(&self.clock_mapping_source_domain_id),
            json_quote(&self.clock_mapping_target_domain_id),
            json_quote(&self.clock_mapping_id),
            json_quote(&self.clock_mapping_revision),
            self.event_time_lower_tick,
            self.event_time_upper_tick,
            self.received_tick,
            json_quote(&self.clock_domain_id),
            evidence_json(&self.calibration_or_source_evidence),
            evidence_json(&self.clock_mapping_evidence),
            observation_json(&self.observation),
        )
    }
}

impl StreamCoverage {
    pub fn canonical_json(&self) -> String {
        format!(
            "{{\"channel\":{},\"source_id\":{},\"source_revision\":{},\"first_sequence\":{},\"next_sequence_exclusive\":{},\"clock_domain_id\":{},\"covered_start_tick\":{},\"covered_end_tick\":{},\"complete\":{},\"evidence\":{}}}",
            json_quote(channel_name(self.channel)),
            json_quote(&self.source_id),
            json_quote(&self.source_revision),
            self.first_sequence,
            self.next_sequence_exclusive,
            json_quote(&self.clock_domain_id),
            self.covered_start_tick,
            self.covered_end_tick,
            self.complete,
            evidence_json(&self.evidence),
        )
    }
}

/// Canonical source event ledger. Permitted transport reordering does not alter
/// these bytes; duplicate retransmissions remain represented and auditable.
pub fn canonical_event_ledger_json(events: &[TelemetryEvent]) -> String {
    // Sort complete canonical event bytes, not a partial key. Invalid/conflicting
    // records must still have permutation-invariant evidence receipts.
    let mut ordered = events
        .iter()
        .map(TelemetryEvent::canonical_json)
        .collect::<Vec<_>>();
    ordered.sort();
    format!("[{}]", ordered.join(","))
}

/// Canonical coverage bundle. Source attestations are retained as records and
/// never inferred from the mere absence of alarm events.
pub fn canonical_coverage_bundle_json(coverage: &[StreamCoverage]) -> String {
    let mut ordered = coverage
        .iter()
        .map(StreamCoverage::canonical_json)
        .collect::<Vec<_>>();
    ordered.sort();
    format!("[{}]", ordered.join(","))
}

impl ReconciliationPolicy {
    pub fn canonical_json(&self) -> String {
        let mut channels = self.required_channels.clone();
        // Sort but preserve duplicates: reconcile() rejects them, and
        // canonical input must not erase an invalid caller-supplied policy.
        channels.sort();
        let names = channels
            .iter()
            .map(|channel| json_quote(channel_name(*channel)))
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"policy_id\":{},\"policy_revision\":{},\"required_channels\":[{}],\"maximum_sample_gap_ticks\":{},\"maximum_ingest_lateness_ticks\":{}}}",
            json_quote(&self.policy_id),
            json_quote(&self.policy_revision),
            names,
            self.maximum_sample_gap_ticks,
            self.maximum_ingest_lateness_ticks,
        )
    }
}

fn channel_name(channel: Channel) -> &'static str {
    match channel {
        Channel::HookLoad => "hook_load",
        Channel::WindGust => "wind_gust",
        Channel::CraneConfiguration => "crane_configuration",
        Channel::MotionEnvelope => "motion_envelope",
        Channel::OccupancyProjection => "occupancy_projection",
        Channel::OverloadAlarm => "overload_alarm",
        Channel::HoistState => "hoist_state",
    }
}

fn disposition_name(disposition: Disposition) -> &'static str {
    match disposition {
        Disposition::ConformingToDeclaredEnvelope => "conforming_to_declared_envelope",
        Disposition::DeviationObserved => "deviation_observed",
        Disposition::Indeterminate => "indeterminate",
    }
}

fn preflight_disposition_name(value: PreflightDisposition) -> &'static str {
    match value {
        PreflightDisposition::ReadyForHumanReview => "ready_for_human_review",
        PreflightDisposition::Rejected => "rejected",
        PreflightDisposition::Indeterminate => "indeterminate",
    }
}

fn execution_outcome_name(value: ExecutionOutcome) -> &'static str {
    match value {
        ExecutionOutcome::Completed => "completed",
        ExecutionOutcome::Aborted => "aborted",
        ExecutionOutcome::Unknown => "unknown",
    }
}

fn severity_name(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Indeterminate => "indeterminate",
        DiagnosticSeverity::Deviation => "deviation",
    }
}

impl ReconciliationReport {
    /// Canonical JSON for the deterministic result. Hash this byte string
    /// externally and bind it with the plan, preflight, policy, coverage and
    /// raw-event digests. It is a receipt, not a signature or physical proof.
    pub fn canonical_json(&self) -> String {
        let unknown_json = self
            .unknown_intervals
            .iter()
            .map(|item| {
                format!(
                    "{{\"channel\":{},\"start_tick\":{},\"end_tick\":{},\"reason\":{}}}",
                    json_quote(channel_name(item.channel)),
                    item.start_tick,
                    item.end_tick,
                    json_quote(item.reason),
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let diagnostic_json = self
            .diagnostics
            .iter()
            .map(|item| {
                format!(
                    "{{\"code\":{},\"severity\":{},\"channel\":{},\"subject_id\":{},\"detail\":{}}}",
                    json_quote(item.code),
                    json_quote(severity_name(item.severity)),
                    item.channel
                        .map(|channel| json_quote(channel_name(channel)))
                        .unwrap_or_else(|| "null".to_owned()),
                    item.subject_id
                        .as_deref()
                        .map(json_quote)
                        .unwrap_or_else(|| "null".to_owned()),
                    json_quote(&item.detail),
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"contract_version\":{},\"disposition\":{},\"session_id\":{},\"plan_id\":{},\"plan_revision\":{},\"plan_input_sha256\":{},\"preflight_report_sha256\":{},\"event_ledger_sha256\":{},\"coverage_bundle_sha256\":{},\"authorisation_id\":{},\"authorisation_revision\":{},\"preflight_disposition\":{},\"block_id\":{},\"block_revision\":{},\"crane_id\":{},\"crane_configuration_revision\":{},\"site_epoch_id\":{},\"frame_id\":{},\"clock_domain_id\":{},\"start_tick\":{},\"end_tick\":{},\"maximum_hook_load_mn\":{},\"maximum_gust_mm_per_s\":{},\"motion_report_id\":{},\"motion_report_sha256\":{},\"occupancy_projection_id\":{},\"occupancy_projection_sha256\":{},\"execution_outcome\":{},\"policy_id\":{},\"policy_revision\":{},\"policy_sha256\":{},\"matched_event_count\":{},\"exact_retransmission_count\":{},\"maximum_hook_load_upper_mn\":{},\"maximum_wind_gust_upper_mm_per_s\":{},\"maximum_ingest_lateness_ticks\":{},\"unknown_intervals\":[{}],\"diagnostics\":[{}]}}",
            json_quote(self.contract_version),
            json_quote(disposition_name(self.disposition)),
            json_quote(&self.session_id),
            json_quote(&self.plan_id),
            json_quote(&self.plan_revision),
            json_quote(&self.plan_input_sha256.to_ascii_lowercase()),
            json_quote(&self.preflight_report_sha256.to_ascii_lowercase()),
            json_quote(&self.event_ledger_sha256.to_ascii_lowercase()),
            json_quote(&self.coverage_bundle_sha256.to_ascii_lowercase()),
            json_quote(&self.authorisation_id),
            json_quote(&self.authorisation_revision),
            json_quote(preflight_disposition_name(self.preflight_disposition)),
            json_quote(&self.block_id),
            json_quote(&self.block_revision),
            json_quote(&self.crane_id),
            json_quote(&self.crane_configuration_revision),
            json_quote(&self.site_epoch_id),
            json_quote(&self.frame_id),
            json_quote(&self.clock_domain_id),
            self.start_tick,
            self.end_tick,
            self.maximum_hook_load_mn,
            self.maximum_gust_mm_per_s,
            json_quote(&self.motion_report_id),
            json_quote(&self.motion_report_sha256.to_ascii_lowercase()),
            json_quote(&self.occupancy_projection_id),
            json_quote(&self.occupancy_projection_sha256.to_ascii_lowercase()),
            json_quote(execution_outcome_name(self.execution_outcome)),
            json_quote(&self.policy_id),
            json_quote(&self.policy_revision),
            json_quote(&self.policy_sha256.to_ascii_lowercase()),
            self.matched_event_count,
            self.exact_retransmission_count,
            self.maximum_hook_load_upper_mn
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            self.maximum_wind_gust_upper_mm_per_s
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            self.maximum_ingest_lateness_ticks
                .map(|value| value.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            unknown_json,
            diagnostic_json,
        )
    }
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
    conflicted_evidence_ids: &mut BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let diagnostic_start = diagnostics.len();
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
            Some(existing) if existing != evidence => {
                conflicted_evidence_ids.insert(id.to_owned());
                add_diagnostic(
                    diagnostics,
                    "EVIDENCE_ID_CONFLICT",
                    DiagnosticSeverity::Indeterminate,
                    None,
                    Some(id),
                    "one evidence record ID was supplied with conflicting metadata",
                );
            }
            Some(_) => {}
            None => {
                seen_evidence.insert(id.to_owned(), evidence.clone());
            }
        }
    }
    diagnostics.len() == diagnostic_start
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
        ("authorisation_id", binding.authorisation_id.as_str()),
        ("authorisation_revision", binding.authorisation_revision.as_str()),
        ("gust_limit_profile_id", binding.gust_limit_profile_id.as_str()),
        ("gust_limit_profile_revision", binding.gust_limit_profile_revision.as_str()),
        ("policy_id", policy.policy_id.as_str()),
        ("policy_revision", policy.policy_revision.as_str()),
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
        || !valid_sha256(&binding.event_ledger_sha256)
        || !valid_sha256(&binding.coverage_bundle_sha256)
        || !valid_sha256(&policy.policy_sha256)
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
    if binding.preflight_disposition != PreflightDisposition::ReadyForHumanReview {
        add_diagnostic(
            &mut diagnostics,
            "PREFLIGHT_NOT_REVIEWABLE",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(&binding.session_id),
            "the source preflight report must be human-reviewable before execution reconciliation",
        );
    }
    if binding.authorisation_evidence.source_record_id != binding.authorisation_id {
        add_diagnostic(
            &mut diagnostics,
            "AUTHORISATION_REFERENCE_MISMATCH",
            DiagnosticSeverity::Indeterminate,
            None,
            Some(&binding.authorisation_id),
            "authorisation evidence must bind the exact admitted authorisation record ID",
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
    let mut conflicted_event_ids: BTreeSet<String> = BTreeSet::new();
    let mut sequence_records: BTreeMap<(Channel, String, u64), TelemetryEvent> = BTreeMap::new();
    let mut conflicted_sequences: BTreeSet<(Channel, String, u64)> = BTreeSet::new();
    let mut seen_evidence: BTreeMap<String, EvidenceRef> = BTreeMap::new();
    let mut conflicted_evidence_ids: BTreeSet<String> = BTreeSet::new();
    let _authorisation_evidence_valid = validate_evidence(
        &binding.authorisation_evidence,
        &binding.plan_id,
        &binding.plan_revision,
        &binding.clock_domain_id,
        binding.start_tick,
        binding.end_tick,
        &mut seen_evidence,
        &mut conflicted_evidence_ids,
        &mut diagnostics,
    );
    let hook_load_limit_evidence_valid = validate_evidence(
        &binding.hook_load_limit_evidence,
        &binding.crane_id,
        &binding.crane_configuration_revision,
        &binding.clock_domain_id,
        binding.start_tick,
        binding.end_tick,
        &mut seen_evidence,
        &mut conflicted_evidence_ids,
        &mut diagnostics,
    );
    let gust_limit_evidence_valid = validate_evidence(
        &binding.gust_limit_evidence,
        &binding.gust_limit_profile_id,
        &binding.gust_limit_profile_revision,
        &binding.clock_domain_id,
        binding.start_tick,
        binding.end_tick,
        &mut seen_evidence,
        &mut conflicted_evidence_ids,
        &mut diagnostics,
    );
    let mut matched_event_count = 0_usize;
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
                "mapped event time and receipt time must use the execution clock domain",
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
        if event.event_time_lower_tick > event.event_time_upper_tick
            || event.event_time_lower_tick < binding.start_tick
            || event.event_time_upper_tick >= binding.end_tick
        {
            add_diagnostic(
                &mut diagnostics,
                "EVENT_TIME_INTERVAL_OUTSIDE_EXECUTION",
                DiagnosticSeverity::Indeterminate,
                Some(event.observation.channel()),
                Some(&event.event_id),
                "the complete mapped event-time uncertainty interval must fit inside the half-open execution interval",
            );
            add_unknown(
                &mut unknown_intervals,
                event.observation.channel(),
                binding.start_tick,
                binding.end_tick,
                "event_time_interval_outside_execution",
            );
            continue;
        }
        if !nonblank(&event.source_clock_domain_id)
            || !nonblank(&event.clock_mapping_source_domain_id)
            || !nonblank(&event.clock_mapping_target_domain_id)
            || !nonblank(&event.clock_mapping_id)
            || !nonblank(&event.clock_mapping_revision)
        {
            add_diagnostic(
                &mut diagnostics,
                "CLOCK_MAPPING_IDENTITY_INCOMPLETE",
                DiagnosticSeverity::Indeterminate,
                Some(event.observation.channel()),
                Some(&event.event_id),
                "source clock and clock-mapping identities/revisions must be explicit",
            );
        }
        if event.clock_mapping_source_domain_id != event.source_clock_domain_id
            || event.clock_mapping_target_domain_id != binding.clock_domain_id
        {
            add_diagnostic(
                &mut diagnostics,
                "CLOCK_MAPPING_DOMAIN_MISMATCH",
                DiagnosticSeverity::Indeterminate,
                Some(event.observation.channel()),
                Some(&event.event_id),
                "clock mapping must bind the exact source clock to the execution clock domain",
            );
        }
        if event.received_tick < event.event_time_upper_tick {
            add_diagnostic(
                &mut diagnostics,
                "EVENT_TIME_MAPPING_EXCEEDS_RECEIPT",
                DiagnosticSeverity::Indeterminate,
                Some(event.observation.channel()),
                Some(&event.event_id),
                "mapped event-time uncertainty extends beyond its ingestion/receipt time",
            );
        }
        if event.received_tick < event.event_time_lower_tick {
            add_diagnostic(
                &mut diagnostics,
                "EVENT_RECEIPT_PRECEDES_POSSIBLE_EVENT",
                DiagnosticSeverity::Indeterminate,
                Some(event.observation.channel()),
                Some(&event.event_id),
                "receipt time precedes the earliest possible mapped event time",
            );
        } else {
            let lateness = event.received_tick - event.event_time_lower_tick;
            if lateness > policy.maximum_ingest_lateness_ticks {
                add_diagnostic(
                    &mut diagnostics,
                    "EVENT_INGESTION_TOO_LATE",
                    DiagnosticSeverity::Indeterminate,
                    Some(event.observation.channel()),
                    Some(&event.event_id),
                    "conservative event-time to ingestion-time delay exceeds the frozen lateness policy",
                );
            }
        }

        let calibration_evidence_valid = validate_evidence(
            &event.calibration_or_source_evidence,
            &event.source_id,
            &event.source_revision,
            &binding.clock_domain_id,
            event.event_time_lower_tick,
            event.event_time_upper_tick.saturating_add(1),
            &mut seen_evidence,
            &mut conflicted_evidence_ids,
            &mut diagnostics,
        );
        let clock_mapping_evidence_valid = validate_evidence(
            &event.clock_mapping_evidence,
            &event.clock_mapping_id,
            &event.clock_mapping_revision,
            &binding.clock_domain_id,
            event.event_time_lower_tick,
            event.event_time_upper_tick.saturating_add(1),
            &mut seen_evidence,
            &mut conflicted_evidence_ids,
            &mut diagnostics,
        );
        let channel = event.observation.channel();
        if !calibration_evidence_valid || !clock_mapping_evidence_valid {
            add_unknown(
                &mut unknown_intervals,
                channel,
                event.event_time_lower_tick,
                event.event_time_upper_tick.saturating_add(1),
                "event_calibration_or_clock_mapping_evidence_invalid",
            );
            continue;
        }
        let sequence_key = (channel, event.source_id.clone(), event.sequence);

        if conflicted_event_ids.contains(&event.event_id) {
            continue;
        }
        if conflicted_sequences.contains(&sequence_key) {
            continue;
        }

        match event_ids.get(&event.event_id).cloned() {
            Some(existing) if existing == *event => {
                exact_retransmissions += 1;
                continue;
            }
            Some(existing) => {
                add_diagnostic(
                    &mut diagnostics,
                    "EVENT_ID_CONFLICT",
                    DiagnosticSeverity::Indeterminate,
                    Some(channel),
                    Some(&event.event_id),
                    "same event ID was observed with conflicting event content; neither record is admitted",
                );
                let prior_key = (
                    existing.observation.channel(),
                    existing.source_id.clone(),
                    existing.sequence,
                );
                event_ids.remove(&event.event_id);
                conflicted_event_ids.insert(event.event_id.clone());
                sequence_records.remove(&prior_key);
                conflicted_sequences.insert(prior_key.clone());
                conflicted_sequences.insert(sequence_key.clone());

                // A conflicting event ID can also point at a sequence already
                // occupied by another event. Quarantine that record too, so
                // no arrival order can decide which payload reaches metrics.
                if let Some(occupied) = sequence_records.remove(&sequence_key) {
                    event_ids.remove(&occupied.event_id);
                    conflicted_event_ids.insert(occupied.event_id.clone());
                    if let Some(items) = events_by_channel.get_mut(&sequence_key.0) {
                        items.retain(|item| item.event_id != occupied.event_id);
                    }
                }
                if let Some(items) = events_by_channel.get_mut(&prior_key.0) {
                    items.retain(|item| item.event_id != existing.event_id);
                }
                if let Some(items) = events_by_channel.get_mut(&channel) {
                    items.retain(|item| item.event_id != event.event_id);
                }
                add_unknown(
                    &mut unknown_intervals,
                    channel,
                    binding.start_tick,
                    binding.end_tick,
                    "conflicting_event_identity",
                );
                continue;
            }
            None => {}
        }

        match sequence_records.get(&sequence_key).cloned() {
            Some(existing) => {
                add_diagnostic(
                    &mut diagnostics,
                    "EVENT_SEQUENCE_CONFLICT",
                    DiagnosticSeverity::Indeterminate,
                    Some(channel),
                    Some(&event.source_id),
                    "one source sequence number identifies different event payloads; neither record is admitted",
                );
                event_ids.remove(&existing.event_id);
                conflicted_event_ids.insert(existing.event_id.clone());
                conflicted_event_ids.insert(event.event_id.clone());
                sequence_records.remove(&sequence_key);
                conflicted_sequences.insert(sequence_key.clone());
                if let Some(items) = events_by_channel.get_mut(&channel) {
                    items.retain(|item| {
                        item.event_id != existing.event_id && item.event_id != event.event_id
                    });
                }
                add_unknown(
                    &mut unknown_intervals,
                    channel,
                    binding.start_tick,
                    binding.end_tick,
                    "conflicting_source_sequence",
                );
                continue;
            }
            None => {}
        }

        event_ids.insert(event.event_id.clone(), event.clone());
        sequence_records.insert(sequence_key, event.clone());
        events_by_channel.entry(channel).or_default().push(event.clone());
        // Count only conflict-free events after source coverage matches below.
    }

    // Validate every coverage attestation before interpreting any sensor data.
    // This lets a later evidence-ID collision invalidate all earlier references.
    for channel in &channels {
        if let Some(stream) = coverage_by_channel.get(channel) {
            let _ = validate_evidence(
                &stream.evidence,
                &stream.source_id,
                &stream.source_revision,
                &binding.clock_domain_id,
                binding.start_tick,
                binding.end_tick,
                &mut seen_evidence,
                &mut conflicted_evidence_ids,
                &mut diagnostics,
            );
        }
    }
    let hook_limit_trustworthy = hook_load_limit_evidence_valid
        && !conflicted_evidence_ids.contains(&binding.hook_load_limit_evidence.source_record_id);
    let gust_limit_trustworthy = gust_limit_evidence_valid
        && !conflicted_evidence_ids.contains(&binding.gust_limit_evidence.source_record_id);
    if !hook_limit_trustworthy {
        add_unknown(
            &mut unknown_intervals,
            Channel::HookLoad,
            binding.start_tick,
            binding.end_tick,
            "hook_load_limit_evidence_invalid",
        );
    }
    if !gust_limit_trustworthy {
        add_unknown(
            &mut unknown_intervals,
            Channel::WindGust,
            binding.start_tick,
            binding.end_tick,
            "gust_limit_evidence_invalid",
        );
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
        if stream.clock_domain_id != binding.clock_domain_id {
            add_diagnostic(
                &mut diagnostics,
                "COVERAGE_CLOCK_DOMAIN_MISMATCH",
                DiagnosticSeverity::Indeterminate,
                Some(*channel),
                Some(&stream.source_id),
                "coverage interval is not in the execution clock domain",
            );
        }
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
                        .max(event.received_tick.saturating_sub(event.event_time_lower_tick)),
                );
            }
        }
        channel_events.retain(|event| {
            let evidence_conflicted = conflicted_evidence_ids.contains(
                &event.calibration_or_source_evidence.source_record_id,
            ) || conflicted_evidence_ids.contains(
                &event.clock_mapping_evidence.source_record_id,
            );
            if evidence_conflicted {
                add_unknown(
                    &mut unknown_intervals,
                    *channel,
                    event.event_time_lower_tick,
                    event.event_time_upper_tick.saturating_add(1),
                    "conflicting_event_source_evidence",
                );
            }
            !evidence_conflicted
                && event.source_id == stream.source_id
                && event.source_revision == stream.source_revision
        });
        matched_event_count += channel_events.len();

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
            if pair[1].event_time_lower_tick < pair[0].event_time_upper_tick {
                add_diagnostic(
                    &mut diagnostics,
                    "SOURCE_EVENT_TIME_ORDER_UNCERTAIN",
                    DiagnosticSeverity::Indeterminate,
                    Some(*channel),
                    Some(&stream.source_id),
                    "mapped event-time uncertainty intervals do not establish sequence order",
                );
                add_unknown(
                    &mut unknown_intervals,
                    *channel,
                    pair[0].event_time_lower_tick,
                    pair[1].event_time_upper_tick,
                    "source_event_time_order_uncertain",
                );
            }
        }

        if matches!(channel, Channel::HookLoad | Channel::WindGust) {
            let mut ordered_intervals: Vec<(u64, u64)> = channel_events
                .iter()
                .map(|e| (e.event_time_lower_tick, e.event_time_upper_tick))
                .collect();
            ordered_intervals.sort_unstable();
            if let Some((first_lower, first_upper)) = ordered_intervals.first().copied() {
                let leading_gap = first_upper.saturating_sub(binding.start_tick);
                if leading_gap > policy.maximum_sample_gap_ticks {
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
                        "conservative leading sample gap exceeds the frozen policy",
                    );
                }
                for pair in ordered_intervals.windows(2) {
                    // Worst possible gap: the next sample occurs at its latest
                    // mapped time and the previous sample at its earliest.
                    let maximum_gap = pair[1].1.saturating_sub(pair[0].0);
                    if maximum_gap > policy.maximum_sample_gap_ticks {
                        add_unknown(
                            &mut unknown_intervals,
                            *channel,
                            pair[0].0,
                            pair[1].1,
                            "inter_sample_gap_exceeds_policy",
                        );
                        add_diagnostic(
                            &mut diagnostics,
                            "SAMPLE_GAP_EXCEEDED",
                            DiagnosticSeverity::Indeterminate,
                            Some(*channel),
                            Some(&stream.source_id),
                            "conservative inter-sample gap exceeds the frozen policy",
                        );
                    }
                }
                let last_lower = ordered_intervals.last().map(|item| item.0).unwrap_or(first_lower);
                let trailing = binding.end_tick.saturating_sub(last_lower);
                if trailing > policy.maximum_sample_gap_ticks {
                    add_unknown(
                        &mut unknown_intervals,
                        *channel,
                        last_lower,
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
                    if upper_mn > binding.maximum_hook_load_mn && hook_limit_trustworthy {
                        add_diagnostic(
                            &mut diagnostics,
                            "HOOK_LOAD_LIMIT_EXCEEDED",
                            DiagnosticSeverity::Deviation,
                            Some(Channel::HookLoad),
                            Some(&event.event_id),
                            "conservative observed hook-load upper bound exceeds the source-bound declared limit",
                        );
                    }
                }
                Observation::WindGust { upper_mm_per_s } => {
                    maximum_wind_gust =
                        Some(maximum_wind_gust.unwrap_or(0).max(upper_mm_per_s));
                    if upper_mm_per_s > binding.maximum_gust_mm_per_s && gust_limit_trustworthy {
                        add_diagnostic(
                            &mut diagnostics,
                            "WIND_GUST_LIMIT_EXCEEDED",
                            DiagnosticSeverity::Deviation,
                            Some(Channel::WindGust),
                            Some(&event.event_id),
                            "conservative observed gust upper bound exceeds the source-bound declared operation limit",
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
        plan_id: binding.plan_id.clone(),
        plan_revision: binding.plan_revision.clone(),
        plan_input_sha256: binding.plan_input_sha256.clone(),
        preflight_report_sha256: binding.preflight_report_sha256.clone(),
        event_ledger_sha256: binding.event_ledger_sha256.clone(),
        coverage_bundle_sha256: binding.coverage_bundle_sha256.clone(),
        authorisation_id: binding.authorisation_id.clone(),
        authorisation_revision: binding.authorisation_revision.clone(),
        preflight_disposition: binding.preflight_disposition,
        block_id: binding.block_id.clone(),
        block_revision: binding.block_revision.clone(),
        crane_id: binding.crane_id.clone(),
        crane_configuration_revision: binding.crane_configuration_revision.clone(),
        site_epoch_id: binding.site_epoch_id.clone(),
        frame_id: binding.frame_id.clone(),
        clock_domain_id: binding.clock_domain_id.clone(),
        start_tick: binding.start_tick,
        end_tick: binding.end_tick,
        maximum_hook_load_mn: binding.maximum_hook_load_mn,
        maximum_gust_mm_per_s: binding.maximum_gust_mm_per_s,
        motion_report_id: binding.motion_report_id.clone(),
        motion_report_sha256: binding.motion_report_sha256.clone(),
        occupancy_projection_id: binding.occupancy_projection_id.clone(),
        occupancy_projection_sha256: binding.occupancy_projection_sha256.clone(),
        execution_outcome: binding.outcome,
        policy_id: policy.policy_id.clone(),
        policy_revision: policy.policy_revision.clone(),
        policy_sha256: policy.policy_sha256.clone(),
        matched_event_count,
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
            event_ledger_sha256: "6".repeat(64),
            coverage_bundle_sha256: "7".repeat(64),
            preflight_disposition: PreflightDisposition::ReadyForHumanReview,
            authorisation_id: "authorisation-01".to_owned(),
            authorisation_revision: "auth-r2".to_owned(),
            authorisation_evidence: evidence("authorisation-evidence-01", "plan-001", "plan-r4"),
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
            hook_load_limit_evidence: evidence("hook-limit-evidence", "gantry-01", "gantry-r3"),
            maximum_gust_mm_per_s: 10_000,
            gust_limit_profile_id: "wind-limit-01".to_owned(),
            gust_limit_profile_revision: "wind-limit-r2".to_owned(),
            gust_limit_evidence: evidence("gust-limit-evidence", "wind-limit-01", "wind-limit-r2"),
            motion_report_id: "motion-01".to_owned(),
            motion_report_sha256: "3".repeat(64),
            occupancy_projection_id: "occupancy-01".to_owned(),
            occupancy_projection_sha256: "4".repeat(64),
            outcome: ExecutionOutcome::Completed,
        }
    }

    fn policy() -> ReconciliationPolicy {
        ReconciliationPolicy {
            policy_id: "reconciliation-policy-01".to_owned(),
            policy_revision: "policy-r1".to_owned(),
            policy_sha256: "5".repeat(64),
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
            clock_domain_id: "yard-clock".to_owned(),
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
            source_clock_domain_id: format!("source-clock::{source_id}"),
            clock_mapping_source_domain_id: format!("source-clock::{source_id}"),
            clock_mapping_target_domain_id: "yard-clock".to_owned(),
            clock_mapping_id: format!("clock-map::{source_id}"),
            clock_mapping_revision: "map-r1".to_owned(),
            event_time_lower_tick: tick,
            event_time_upper_tick: tick,
            received_tick: tick + 2,
            clock_domain_id: "yard-clock".to_owned(),
            calibration_or_source_evidence: evidence(
                &format!("calibration::{source_id}"),
                source_id,
                source_revision,
            ),
            clock_mapping_evidence: evidence(
                &format!("clock-map-evidence::{source_id}"),
                &format!("clock-map::{source_id}"),
                "map-r1",
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
    fn frozen_conformance_fixture_maps_all_vectors_to_rust_tests() {
        const FIXTURE: &str = include_str!("../fixtures/conformance-vectors-v0.1.json");
        let test_functions: [fn(); 45] = [
            mismatched_source_cannot_satisfy_attested_sequence_coverage,
            conflicting_source_evidence_record_identity_is_indeterminate,
            missing_required_motion_event_is_indeterminate_even_with_complete_empty_stream,
            occupancy_conflict_is_a_deviation,
            exact_limit_equality_is_conforming_but_unknown_interval_is_not_hidden,
            clock_mapping_for_wrong_source_or_target_domain_is_indeterminate,
            uncertainty_interval_must_not_extend_beyond_receipt_time,
            conservative_time_uncertainty_expands_the_sample_gap,
            coverage_in_a_different_clock_domain_is_indeterminate,
            mapping_evidence_for_another_mapping_revision_is_indeterminate,
            preflight_rejection_cannot_be_upgraded_by_clean_telemetry,
            expired_authorisation_evidence_is_indeterminate,
            authorisation_evidence_for_another_plan_cannot_be_reused,
            revoked_sensor_calibration_is_indeterminate,
            exactly_maximum_sample_gap_is_accepted,
            event_time_regression_within_sequence_is_indeterminate,
            event_outside_execution_interval_is_indeterminate,
            duplicate_coverage_statement_is_indeterminate,
            canonical_report_json_is_invariant_to_transport_order,
            canonical_report_json_escapes_untrusted_diagnostic_text,
            canonical_event_and_coverage_bundles_ignore_transport_order,
            malformed_raw_evidence_bundle_digest_is_indeterminate,
            complete_bounded_session_is_conforming_to_declared_envelope,
            load_one_unit_above_limit_is_a_deviation,
            gust_one_unit_above_limit_is_a_deviation,
            exact_limit_equality_is_not_an_exceedance,
            missing_stream_coverage_is_indeterminate_not_conforming,
            incomplete_stream_coverage_is_indeterminate,
            sequence_gap_is_indeterminate,
            exact_event_retransmission_is_idempotently_deduplicated,
            same_event_id_with_conflicting_payload_is_indeterminate,
            sensor_calibration_for_wrong_source_revision_is_indeterminate,
            sample_gap_beyond_policy_creates_unknown_interval,
            crane_configuration_change_is_a_deviation,
            motion_excursion_is_a_deviation_even_without_controller_alarm,
            active_overload_alarm_is_a_deviation_even_when_load_sensor_is_below_limit,
            preflight_must_be_bound_to_exact_plan_input_digest,
            missing_or_delayed_sensor_evidence_is_not_a_clean_execution,
            completed_abort_unknown_distinctions_are_preserved_as_indeterminate,
            permitted_transport_ordering_does_not_change_report,
            clock_domain_mismatch_is_indeterminate,
            duplicate_sequence_with_new_event_id_is_indeterminate,
            revoked_over_limit_sensor_event_is_not_promoted_to_deviation,
            expired_limit_evidence_does_not_produce_formal_limit_deviation,
            later_evidence_id_conflict_retroactively_quarantines_prior_sensor_event,
        ];
        let test_names = [
            stringify!(mismatched_source_cannot_satisfy_attested_sequence_coverage),
            stringify!(conflicting_source_evidence_record_identity_is_indeterminate),
            stringify!(missing_required_motion_event_is_indeterminate_even_with_complete_empty_stream),
            stringify!(occupancy_conflict_is_a_deviation),
            stringify!(exact_limit_equality_is_conforming_but_unknown_interval_is_not_hidden),
            stringify!(clock_mapping_for_wrong_source_or_target_domain_is_indeterminate),
            stringify!(uncertainty_interval_must_not_extend_beyond_receipt_time),
            stringify!(conservative_time_uncertainty_expands_the_sample_gap),
            stringify!(coverage_in_a_different_clock_domain_is_indeterminate),
            stringify!(mapping_evidence_for_another_mapping_revision_is_indeterminate),
            stringify!(preflight_rejection_cannot_be_upgraded_by_clean_telemetry),
            stringify!(expired_authorisation_evidence_is_indeterminate),
            stringify!(authorisation_evidence_for_another_plan_cannot_be_reused),
            stringify!(revoked_sensor_calibration_is_indeterminate),
            stringify!(exactly_maximum_sample_gap_is_accepted),
            stringify!(event_time_regression_within_sequence_is_indeterminate),
            stringify!(event_outside_execution_interval_is_indeterminate),
            stringify!(duplicate_coverage_statement_is_indeterminate),
            stringify!(canonical_report_json_is_invariant_to_transport_order),
            stringify!(canonical_report_json_escapes_untrusted_diagnostic_text),
            stringify!(canonical_event_and_coverage_bundles_ignore_transport_order),
            stringify!(malformed_raw_evidence_bundle_digest_is_indeterminate),
            stringify!(complete_bounded_session_is_conforming_to_declared_envelope),
            stringify!(load_one_unit_above_limit_is_a_deviation),
            stringify!(gust_one_unit_above_limit_is_a_deviation),
            stringify!(exact_limit_equality_is_not_an_exceedance),
            stringify!(missing_stream_coverage_is_indeterminate_not_conforming),
            stringify!(incomplete_stream_coverage_is_indeterminate),
            stringify!(sequence_gap_is_indeterminate),
            stringify!(exact_event_retransmission_is_idempotently_deduplicated),
            stringify!(same_event_id_with_conflicting_payload_is_indeterminate),
            stringify!(sensor_calibration_for_wrong_source_revision_is_indeterminate),
            stringify!(sample_gap_beyond_policy_creates_unknown_interval),
            stringify!(crane_configuration_change_is_a_deviation),
            stringify!(motion_excursion_is_a_deviation_even_without_controller_alarm),
            stringify!(active_overload_alarm_is_a_deviation_even_when_load_sensor_is_below_limit),
            stringify!(preflight_must_be_bound_to_exact_plan_input_digest),
            stringify!(missing_or_delayed_sensor_evidence_is_not_a_clean_execution),
            stringify!(completed_abort_unknown_distinctions_are_preserved_as_indeterminate),
            stringify!(permitted_transport_ordering_does_not_change_report),
            stringify!(clock_domain_mismatch_is_indeterminate),
            stringify!(duplicate_sequence_with_new_event_id_is_indeterminate),
            stringify!(revoked_over_limit_sensor_event_is_not_promoted_to_deviation),
            stringify!(expired_limit_evidence_does_not_produce_formal_limit_deviation),
            stringify!(later_evidence_id_conflict_retroactively_quarantines_prior_sensor_event),
        ];
        let vector_ids = [
            "REC-SOURCE-001",
            "REC-EVIDENCE-001",
            "REC-CHANNEL-001",
            "REC-OCC-001",
            "REC-UNKNOWN-001",
            "REC-CLOCK-001",
            "REC-CLOCK-002",
            "REC-CLOCK-003",
            "REC-CLOCK-004",
            "REC-CLOCK-005",
            "REC-PREFLIGHT-001",
            "REC-AUTH-001",
            "REC-AUTH-002",
            "REC-CAL-001",
            "REC-GAP-001",
            "REC-CLOCK-006",
            "REC-TIME-001",
            "REC-COVERAGE-001",
            "REC-META-001",
            "REC-SERIAL-001",
            "REC-META-002",
            "REC-DIGEST-001",
            "REC-BASE-001",
            "REC-LIMIT-001",
            "REC-LIMIT-002",
            "REC-LIMIT-003",
            "REC-COVERAGE-002",
            "REC-COVERAGE-003",
            "REC-SEQUENCE-001",
            "REC-RETRY-001",
            "REC-EVENT-001",
            "REC-CAL-002",
            "REC-GAP-002",
            "REC-CONFIG-001",
            "REC-MOTION-001",
            "REC-ALARM-001",
            "REC-PREFLIGHT-002",
            "REC-INGEST-001",
            "REC-OUTCOME-001",
            "REC-META-003",
            "REC-CLOCK-007",
            "REC-SEQUENCE-002",
            "REC-CAL-003",
            "REC-LIMIT-004",
            "REC-EVID-005",
        ];

        assert_eq!(test_functions.len(), 45);
        assert_eq!(test_names.len(), vector_ids.len());
        assert_eq!(FIXTURE.matches(r#""id": "REC-"#).count(), 45);
        for (test_name, vector_id) in test_names.iter().zip(vector_ids.iter()) {
            assert!(FIXTURE.contains(&format!(r#""id": "{vector_id}""#)), "missing vector {vector_id}");
            assert!(FIXTURE.contains(&format!(r#""test_name": "{test_name}""#)), "missing test mapping {test_name}");
        }
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
    fn clock_mapping_for_wrong_source_or_target_domain_is_indeterminate() {
        let (binding, policy, coverage, mut events) = baseline();
        events[0].clock_mapping_source_domain_id = "another-source-clock".to_owned();
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "CLOCK_MAPPING_DOMAIN_MISMATCH"));
    }

    #[test]
    fn uncertainty_interval_must_not_extend_beyond_receipt_time() {
        let (binding, policy, coverage, mut events) = baseline();
        events[0].event_time_upper_tick = events[0].received_tick + 1;
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVENT_TIME_MAPPING_EXCEEDS_RECEIPT"));
    }

    #[test]
    fn conservative_time_uncertainty_expands_the_sample_gap() {
        let (binding, mut policy, coverage, mut events) = baseline();
        policy.maximum_sample_gap_ticks = 40;
        let first = events
            .iter_mut()
            .find(|item| item.observation.channel() == Channel::HookLoad && item.sequence == 0)
            .unwrap();
        first.event_time_lower_tick = 109;
        first.event_time_upper_tick = 111;
        let second = events
            .iter_mut()
            .find(|item| item.observation.channel() == Channel::HookLoad && item.sequence == 1)
            .unwrap();
        second.event_time_lower_tick = 149;
        second.event_time_upper_tick = 151;
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "SAMPLE_GAP_EXCEEDED"));
    }

    #[test]
    fn coverage_in_a_different_clock_domain_is_indeterminate() {
        let (binding, policy, mut coverage, events) = baseline();
        coverage[0].clock_domain_id = "another-clock".to_owned();
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "COVERAGE_CLOCK_DOMAIN_MISMATCH"));
    }

    #[test]
    fn mapping_evidence_for_another_mapping_revision_is_indeterminate() {
        let (binding, policy, coverage, mut events) = baseline();
        events[0].clock_mapping_evidence.subject_revision = "obsolete-map-r0".to_owned();
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_SUBJECT_MISMATCH"));
    }

    #[test]
    fn preflight_rejection_cannot_be_upgraded_by_clean_telemetry() {
        let (mut binding, policy, coverage, events) = baseline();
        binding.preflight_disposition = PreflightDisposition::Rejected;
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "PREFLIGHT_NOT_REVIEWABLE"));
    }

    #[test]
    fn expired_authorisation_evidence_is_indeterminate() {
        let (mut binding, policy, coverage, events) = baseline();
        binding.authorisation_evidence.valid_until_tick = 199;
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_NOT_VALID_FOR_EXECUTION"));
    }

    #[test]
    fn authorisation_evidence_for_another_plan_cannot_be_reused() {
        let (mut binding, policy, coverage, events) = baseline();
        binding.authorisation_evidence.subject_id = "another-plan".to_owned();
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_SUBJECT_MISMATCH"));
    }

    #[test]
    fn revoked_sensor_calibration_is_indeterminate() {
        let (binding, policy, coverage, mut events) = baseline();
        events[0].calibration_or_source_evidence.state = EvidenceState::Revoked;
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_REVOKED"));
    }

    #[test]
    fn exactly_maximum_sample_gap_is_accepted() {
        let (binding, mut policy, coverage, mut events) = baseline();
        policy.maximum_sample_gap_ticks = 40;
        for event in &mut events {
            if event.observation.channel() == Channel::HookLoad
                || event.observation.channel() == Channel::WindGust
            {
                event.received_tick = event.event_time_lower_tick;
            }
        }
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::ConformingToDeclaredEnvelope);
    }

    #[test]
    fn event_time_regression_within_sequence_is_indeterminate() {
        let (binding, policy, coverage, mut events) = baseline();
        let event = events
            .iter_mut()
            .find(|item| item.observation.channel() == Channel::HookLoad && item.sequence == 2)
            .unwrap();
        event.event_time_lower_tick = 120;
        event.event_time_upper_tick = 120;
        event.received_tick = 122;
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "SOURCE_EVENT_TIME_ORDER_UNCERTAIN"));
    }

    #[test]
    fn event_outside_execution_interval_is_indeterminate() {
        let (binding, policy, coverage, mut events) = baseline();
        events[0].event_time_lower_tick = binding.end_tick;
        events[0].event_time_upper_tick = binding.end_tick;
        events[0].received_tick = binding.end_tick + 1;
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVENT_TIME_INTERVAL_OUTSIDE_EXECUTION"));
    }

    #[test]
    fn duplicate_coverage_statement_is_indeterminate() {
        let (binding, policy, mut coverage, events) = baseline();
        let duplicate = coverage
            .iter()
            .find(|item| item.channel == Channel::HookLoad)
            .unwrap()
            .clone();
        coverage.push(duplicate);
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "COVERAGE_CHANNEL_DUPLICATE"));
    }

    #[test]
    fn canonical_report_json_is_invariant_to_transport_order() {
        let (binding, policy, coverage, events) = baseline();
        let first = reconcile(&binding, &policy, &coverage, &events).canonical_json();
        let mut reversed_events = events.clone();
        reversed_events.reverse();
        let reversed = reconcile(&binding, &policy, &coverage, &reversed_events).canonical_json();
        assert_eq!(first, reversed);
    }

    #[test]
    fn canonical_report_json_escapes_untrusted_diagnostic_text() {
        let (mut binding, policy, coverage, events) = baseline();
        binding.session_id = "session\ninvalid".to_owned();
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert!(report.canonical_json().contains("\\n"));
    }

    #[test]
    fn canonical_event_and_coverage_bundles_ignore_transport_order() {
        let (_binding, policy, coverage, events) = baseline();
        let event_bytes = canonical_event_ledger_json(&events);
        let coverage_bytes = canonical_coverage_bundle_json(&coverage);
        let mut reversed_events = events.clone();
        reversed_events.reverse();
        let mut reversed_coverage = coverage.clone();
        reversed_coverage.reverse();
        assert_eq!(event_bytes, canonical_event_ledger_json(&reversed_events));
        assert_eq!(coverage_bytes, canonical_coverage_bundle_json(&reversed_coverage));
        let mut reversed_policy = policy.clone();
        reversed_policy.required_channels.reverse();
        assert_eq!(policy.canonical_json(), reversed_policy.canonical_json());
        reversed_policy.required_channels.push(Channel::HookLoad);
        assert_ne!(policy.canonical_json(), reversed_policy.canonical_json());
    }

    #[test]
    fn malformed_raw_evidence_bundle_digest_is_indeterminate() {
        let (mut binding, policy, coverage, events) = baseline();
        binding.event_ledger_sha256 = "not-a-digest".to_owned();
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "PLAN_OR_REPORT_DIGEST_INVALID"));
    }

    #[test]
    fn revoked_over_limit_sensor_event_is_not_promoted_to_deviation() {
        let (binding, policy, coverage, mut events) = baseline();
        events[0].calibration_or_source_evidence.state = EvidenceState::Revoked;
        if let Observation::HookLoad { upper_mn } = &mut events[0].observation {
            *upper_mn = binding.maximum_hook_load_mn + 1;
        }
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_REVOKED"));
        assert!(!has_code(&report, "HOOK_LOAD_LIMIT_EXCEEDED"));
        assert_eq!(report.maximum_hook_load_upper_mn, Some(410_000_000));
    }

    #[test]
    fn expired_limit_evidence_does_not_produce_formal_limit_deviation() {
        let (mut binding, policy, coverage, mut events) = baseline();
        binding.hook_load_limit_evidence.valid_until_tick = binding.end_tick - 1;
        if let Observation::HookLoad { upper_mn } = &mut events[0].observation {
            *upper_mn = binding.maximum_hook_load_mn + 1;
        }
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_NOT_VALID_FOR_EXECUTION"));
        assert!(!has_code(&report, "HOOK_LOAD_LIMIT_EXCEEDED"));
        assert!(report
            .unknown_intervals
            .iter()
            .any(|interval| interval.reason == "hook_load_limit_evidence_invalid"));
    }

    #[test]
    fn later_evidence_id_conflict_retroactively_quarantines_prior_sensor_event() {
        let (binding, policy, coverage, mut events) = baseline();
        events[0].calibration_or_source_evidence.source_record_id = "shared-evidence-record".to_owned();
        events[0].calibration_or_source_evidence.sha256 = "a".repeat(64);
        events[1].clock_mapping_evidence.source_record_id = "shared-evidence-record".to_owned();
        events[1].clock_mapping_evidence.sha256 = "b".repeat(64);
        if let Observation::HookLoad { upper_mn } = &mut events[0].observation {
            *upper_mn = binding.maximum_hook_load_mn + 1;
        }
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_ID_CONFLICT"));
        assert!(!has_code(&report, "HOOK_LOAD_LIMIT_EXCEEDED"));
        assert_eq!(report.maximum_hook_load_upper_mn, Some(405_000_000));
    }

    #[test]
    fn complete_bounded_session_is_conforming_to_declared_envelope() {
        let (binding, policy, coverage, events) = baseline();
        let report = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(report.disposition, Disposition::ConformingToDeclaredEnvelope);
        assert_eq!(report.matched_event_count, events.len());
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
        assert_eq!(report.matched_event_count, events.len() - 1);
    }

    #[test]
    fn same_event_id_with_conflicting_payload_is_indeterminate() {
        let (binding, policy, coverage, mut events) = baseline();
        let mut conflicting = events[1].clone();
        if let Observation::HookLoad { upper_mn } = &mut conflicting.observation {
            *upper_mn += 80_000_000;
        }
        events.push(conflicting.clone());
        let forward = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(forward.disposition, Disposition::Indeterminate);
        assert!(has_code(&forward, "EVENT_ID_CONFLICT"));
        assert_eq!(forward.maximum_hook_load_upper_mn, Some(405_000_000));

        events.reverse();
        let reverse = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(forward, reverse);
        assert_eq!(
            canonical_event_ledger_json(&events),
            canonical_event_ledger_json(&{
                let mut original_order = events.clone();
                original_order.reverse();
                original_order
            })
        );
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
        events[0].received_tick =
            events[0].event_time_lower_tick + policy.maximum_ingest_lateness_ticks + 1;
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
        if let Observation::HookLoad { upper_mn } = &mut duplicate.observation {
            *upper_mn = 490_000_000;
        }
        events.push(duplicate);
        let forward = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(forward.disposition, Disposition::Indeterminate);
        assert!(has_code(&forward, "EVENT_SEQUENCE_CONFLICT"));

        events.reverse();
        let reverse = reconcile(&binding, &policy, &coverage, &events);
        assert_eq!(forward, reverse);
        assert_eq!(forward.maximum_hook_load_upper_mn, Some(410_000_000));
    }
}
