use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};

const CORPUS_SCHEMA: &str = "luminous.orbital-interface-event-replay.corpus.v0.1";
const EVENT_SCHEMA: &str = "luminous.orbital-interface-lifecycle.event.v0.1";
const CLAIM_CEILING: &str = "synthetic-event-replay-semantics-only";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    QualificationIssued,
    CycleCompleted,
    InspectionRecorded,
    MaintenancePerformed,
    ComponentReplaced,
    ConfigurationChanged,
    ProcedureChanged,
    InterfaceChanged,
    QualificationRevoked,
    CycleCounterReset,
    EnvironmentExposure,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplayStatus {
    Qualified,
    RequalificationRequired,
    Withdrawn,
    Rejected,
    Disputed,
    Indeterminate,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub event_schema: String,
    pub stream_id: String,
    pub event_id: String,
    pub sequence: u64,
    pub previous_event_id: Option<String>,
    pub pair_id: String,
    pub service_kind: String,
    pub component_id: String,
    pub configuration_id: String,
    pub procedure_revision: String,
    pub interface_revision: String,
    pub event_type: EventKind,
    pub event_tick: u64,
    pub time_uncertainty_ticks: u64,
    pub observation_tick: u64,
    pub ingestion_tick: u64,
    pub producer_id: String,
    pub authority_scope: String,
    pub qualified_cycle_limit: Option<u64>,
    pub valid_until_tick: Option<u64>,
    pub inspection_interval_ticks: Option<u64>,
    pub calibration_valid: Option<bool>,
    pub measurement_in_range: Option<bool>,
    pub inspection_coverage_complete: Option<bool>,
    pub wear_within_limit: Option<bool>,
    pub uncertainty_clears_margin: Option<bool>,
    pub raw_artifact_sha256: Option<String>,
    pub new_component_id: Option<String>,
    pub new_configuration_id: Option<String>,
    pub new_procedure_revision: Option<String>,
    pub new_interface_revision: Option<String>,
    pub requalification: bool,
    pub environment_within_envelope: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventRecord {
    pub raw_event: String,
    pub claimed_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventCase {
    pub case_id: String,
    pub at_tick: u64,
    pub events: Vec<EventRecord>,
    pub expected_status: ReplayStatus,
    pub expected_reason: String,
    pub expected_cycle_count: u64,
    pub expected_duplicate_count: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<EventCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub actual_status: ReplayStatus,
    pub actual_reason: String,
    pub actual_cycle_count: u64,
    pub duplicate_event_count: usize,
    pub expected_status: ReplayStatus,
    pub expected_reason: String,
    pub expected_cycle_count: u64,
    pub expected_duplicate_count: usize,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub qualified: usize,
    pub requalification_required: usize,
    pub withdrawn: usize,
    pub rejected: usize,
    pub disputed: usize,
    pub indeterminate: usize,
    pub duplicate_events: usize,
    pub expected_result_mismatches: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub authority_granted: bool,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Decision {
    status: ReplayStatus,
    reason: &'static str,
    cycle_count: u64,
    duplicate_count: usize,
}

impl Decision {
    const fn new(
        status: ReplayStatus,
        reason: &'static str,
        cycle_count: u64,
        duplicate_count: usize,
    ) -> Self {
        Self {
            status,
            reason,
            cycle_count,
            duplicate_count,
        }
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn expected_authority(kind: EventKind) -> &'static str {
    match kind {
        EventKind::QualificationIssued | EventKind::QualificationRevoked => {
            "qualification-authority"
        }
        EventKind::CycleCompleted => "service-event-source",
        EventKind::InspectionRecorded | EventKind::EnvironmentExposure => "metrology-authority",
        EventKind::MaintenancePerformed | EventKind::ComponentReplaced => {
            "maintenance-authority"
        }
        EventKind::ConfigurationChanged
        | EventKind::ProcedureChanged
        | EventKind::InterfaceChanged
        | EventKind::CycleCounterReset => "configuration-authority",
    }
}

fn decide(case: &EventCase) -> Decision {
    let mut by_id: HashMap<String, String> = HashMap::new();
    let mut unique = Vec::with_capacity(case.events.len());
    let mut duplicate_count = 0usize;

    for record in &case.events {
        if !valid_digest(&record.claimed_sha256)
            || sha256_hex(record.raw_event.as_bytes()) != record.claimed_sha256
        {
            return Decision::new(
                ReplayStatus::Disputed,
                "event_byte_digest_mismatch",
                0,
                duplicate_count,
            );
        }

        let event: Event = match serde_json::from_str(&record.raw_event) {
            Ok(event) => event,
            Err(_) => {
                return Decision::new(
                    ReplayStatus::Rejected,
                    "event_schema_invalid",
                    0,
                    duplicate_count,
                );
            }
        };

        if event.event_schema != EVENT_SCHEMA {
            return Decision::new(
                ReplayStatus::Rejected,
                "unsupported_event_schema",
                0,
                duplicate_count,
            );
        }
        if event.event_id.is_empty() || event.stream_id.is_empty() || event.pair_id.is_empty() {
            return Decision::new(
                ReplayStatus::Indeterminate,
                "event_identity_missing",
                0,
                duplicate_count,
            );
        }

        if let Some(previous_bytes) = by_id.get(&event.event_id) {
            if previous_bytes == &record.raw_event {
                duplicate_count += 1;
                continue;
            }
            return Decision::new(
                ReplayStatus::Disputed,
                "event_id_reused_with_conflicting_bytes",
                0,
                duplicate_count,
            );
        }
        by_id.insert(event.event_id.clone(), record.raw_event.clone());
        unique.push(event);
    }

    if unique.is_empty() {
        return Decision::new(
            ReplayStatus::Indeterminate,
            "event_stream_empty",
            0,
            duplicate_count,
        );
    }

    let stream_id = unique[0].stream_id.as_str();
    let pair_id = unique[0].pair_id.as_str();
    let service_kind = unique[0].service_kind.as_str();
    if unique.iter().any(|event| {
        event.stream_id != stream_id
            || event.pair_id != pair_id
            || event.service_kind != service_kind
    }) {
        return Decision::new(
            ReplayStatus::Rejected,
            "stream_pair_or_service_scope_mismatch",
            0,
            duplicate_count,
        );
    }

    for event in &unique {
        if event.producer_id != "luminous-synthetic-lab" {
            return Decision::new(
                ReplayStatus::Indeterminate,
                "producer_not_in_v0_1_allowlist",
                0,
                duplicate_count,
            );
        }
        if event.authority_scope != expected_authority(event.event_type) {
            return Decision::new(
                ReplayStatus::Rejected,
                "event_authority_scope_mismatch",
                0,
                duplicate_count,
            );
        }
        if event.observation_tick < event.event_tick
            || event.ingestion_tick < event.observation_tick
        {
            return Decision::new(
                ReplayStatus::Indeterminate,
                "event_observation_ingestion_time_order_invalid",
                0,
                duplicate_count,
            );
        }
    }

    let mut ordered = BTreeMap::new();
    for event in unique {
        if ordered.insert(event.sequence, event).is_some() {
            return Decision::new(
                ReplayStatus::Disputed,
                "sequence_fork",
                0,
                duplicate_count,
            );
        }
    }

    let mut previous: Option<Event> = None;
    let mut current_component = String::new();
    let mut current_configuration = String::new();
    let mut current_procedure = String::new();
    let mut current_interface = String::new();
    let mut current_pair = String::new();
    let mut current_service = String::new();
    let mut cycle_count = 0u64;
    let mut cycle_limit = 0u64;
    let mut valid_until = 0u64;
    let mut inspection_interval = 0u64;
    let mut last_inspection_tick = 0u64;
    let mut status = ReplayStatus::Indeterminate;
    let mut reason = "event_stream_not_replayed";

    for (index, (sequence, event)) in ordered.iter().enumerate() {
        let expected_sequence = (index as u64) + 1;
        if *sequence != expected_sequence {
            return Decision::new(
                ReplayStatus::Indeterminate,
                "event_sequence_gap_or_nonzero_start",
                cycle_count,
                duplicate_count,
            );
        }

        if index == 0 {
            if event.previous_event_id.is_some()
                || event.event_type != EventKind::QualificationIssued
                || event.requalification
            {
                return Decision::new(
                    ReplayStatus::Rejected,
                    "stream_must_begin_with_initial_qualification",
                    cycle_count,
                    duplicate_count,
                );
            }
            current_component = event.component_id.clone();
            current_configuration = event.configuration_id.clone();
            current_procedure = event.procedure_revision.clone();
            current_interface = event.interface_revision.clone();
            current_pair = event.pair_id.clone();
            current_service = event.service_kind.clone();
            let (Some(limit), Some(until), Some(interval)) = (
                event.qualified_cycle_limit,
                event.valid_until_tick,
                event.inspection_interval_ticks,
            ) else {
                return Decision::new(
                    ReplayStatus::Indeterminate,
                    "initial_qualification_envelope_missing",
                    cycle_count,
                    duplicate_count,
                );
            };
            cycle_limit = limit;
            valid_until = until;
            inspection_interval = interval;
            last_inspection_tick = event.event_tick;
            status = ReplayStatus::Qualified;
            reason = "qualification_stream_initialized";
        } else {
            let previous_event = previous.as_ref().expect("previous event exists after index zero");
            if event.previous_event_id.as_deref() != Some(previous_event.event_id.as_str()) {
                return Decision::new(
                    ReplayStatus::Indeterminate,
                    "causal_predecessor_mismatch",
                    cycle_count,
                    duplicate_count,
                );
            }

            let previous_latest = previous_event
                .event_tick
                .saturating_add(previous_event.time_uncertainty_ticks);
            let current_earliest = event
                .event_tick
                .saturating_sub(event.time_uncertainty_ticks);
            if previous_latest >= current_earliest {
                return Decision::new(
                    ReplayStatus::Indeterminate,
                    "event_time_order_not_proven",
                    cycle_count,
                    duplicate_count,
                );
            }

            if event.pair_id != current_pair
                || event.service_kind != current_service
                || event.component_id != current_component
            {
                return Decision::new(
                    ReplayStatus::Rejected,
                    "article_pair_component_or_service_identity_mismatch",
                    cycle_count,
                    duplicate_count,
                );
            }
            if event.configuration_id != current_configuration
                || event.procedure_revision != current_procedure
                || event.interface_revision != current_interface
            {
                return Decision::new(
                    ReplayStatus::Rejected,
                    "event_uses_stale_configuration_or_interface",
                    cycle_count,
                    duplicate_count,
                );
            }

            match event.event_type {
                EventKind::QualificationIssued => {
                    if status == ReplayStatus::Withdrawn {
                        return Decision::new(
                            ReplayStatus::Withdrawn,
                            "withdrawn_qualification_is_terminal_in_v0_1",
                            cycle_count,
                            duplicate_count,
                        );
                    }
                    if status != ReplayStatus::RequalificationRequired || !event.requalification {
                        return Decision::new(
                            ReplayStatus::Rejected,
                            "unexpected_or_unscoped_requalification_event",
                            cycle_count,
                            duplicate_count,
                        );
                    }
                    let (Some(limit), Some(until), Some(interval)) = (
                        event.qualified_cycle_limit,
                        event.valid_until_tick,
                        event.inspection_interval_ticks,
                    ) else {
                        return Decision::new(
                            ReplayStatus::Indeterminate,
                            "requalification_envelope_missing",
                            cycle_count,
                            duplicate_count,
                        );
                    };
                    cycle_limit = limit;
                    valid_until = until;
                    inspection_interval = interval;
                    cycle_count = 0;
                    last_inspection_tick = event.event_tick;
                    status = ReplayStatus::Qualified;
                    reason = "qualification_reissued_after_requalification";
                }
                EventKind::CycleCompleted => {
                    if status != ReplayStatus::Qualified {
                        return Decision::new(
                            ReplayStatus::Rejected,
                            "service_cycle_attempted_while_not_qualified",
                            cycle_count,
                            duplicate_count,
                        );
                    }
                    cycle_count = match cycle_count.checked_add(1) {
                        Some(value) => value,
                        None => {
                            return Decision::new(
                                ReplayStatus::Disputed,
                                "cycle_counter_overflow",
                                cycle_count,
                                duplicate_count,
                            );
                        }
                    };
                    if cycle_count > cycle_limit {
                        status = ReplayStatus::RequalificationRequired;
                        reason = "qualified_cycle_limit_exceeded";
                    } else {
                        reason = "cycle_replayed_once";
                    }
                }
                EventKind::InspectionRecorded => {
                    if event.wear_within_limit == Some(false) {
                        status = ReplayStatus::Withdrawn;
                        reason = "wear_exceeds_declared_limit";
                    } else if event.calibration_valid != Some(true)
                        || event.measurement_in_range != Some(true)
                        || event.inspection_coverage_complete != Some(true)
                        || event.wear_within_limit != Some(true)
                        || event.uncertainty_clears_margin != Some(true)
                        || event.raw_artifact_sha256.as_deref().is_none_or(|value| !valid_digest(value))
                    {
                        return Decision::new(
                            ReplayStatus::Indeterminate,
                            "inspection_evidence_incomplete_or_invalid",
                            cycle_count,
                            duplicate_count,
                        );
                    } else if status == ReplayStatus::Qualified {
                        last_inspection_tick = event.event_tick;
                        reason = "inspection_evidence_admitted";
                    }
                }
                EventKind::MaintenancePerformed => {
                    status = ReplayStatus::RequalificationRequired;
                    reason = "maintenance_requires_requalification";
                }
                EventKind::ComponentReplaced => {
                    let Some(new_component) = event.new_component_id.as_ref() else {
                        return Decision::new(
                            ReplayStatus::Indeterminate,
                            "replacement_component_identity_missing",
                            cycle_count,
                            duplicate_count,
                        );
                    };
                    if new_component.is_empty() || new_component == &current_component {
                        return Decision::new(
                            ReplayStatus::Rejected,
                            "replacement_component_identity_invalid",
                            cycle_count,
                            duplicate_count,
                        );
                    }
                    current_component.clone_from(new_component);
                    cycle_count = 0;
                    status = ReplayStatus::RequalificationRequired;
                    reason = "component_replacement_requires_requalification";
                }
                EventKind::ConfigurationChanged => {
                    let Some(new_configuration) = event.new_configuration_id.as_ref() else {
                        return Decision::new(
                            ReplayStatus::Indeterminate,
                            "new_configuration_identity_missing",
                            cycle_count,
                            duplicate_count,
                        );
                    };
                    current_configuration.clone_from(new_configuration);
                    status = ReplayStatus::RequalificationRequired;
                    reason = "configuration_change_requires_requalification";
                }
                EventKind::ProcedureChanged => {
                    let Some(new_procedure) = event.new_procedure_revision.as_ref() else {
                        return Decision::new(
                            ReplayStatus::Indeterminate,
                            "new_procedure_revision_missing",
                            cycle_count,
                            duplicate_count,
                        );
                    };
                    current_procedure.clone_from(new_procedure);
                    status = ReplayStatus::RequalificationRequired;
                    reason = "procedure_change_requires_requalification";
                }
                EventKind::InterfaceChanged => {
                    let Some(new_interface) = event.new_interface_revision.as_ref() else {
                        return Decision::new(
                            ReplayStatus::Indeterminate,
                            "new_interface_revision_missing",
                            cycle_count,
                            duplicate_count,
                        );
                    };
                    current_interface.clone_from(new_interface);
                    status = ReplayStatus::RequalificationRequired;
                    reason = "interface_change_requires_requalification";
                }
                EventKind::QualificationRevoked => {
                    status = ReplayStatus::Withdrawn;
                    reason = "qualification_revoked";
                }
                EventKind::CycleCounterReset => {
                    return Decision::new(
                        ReplayStatus::Disputed,
                        "cycle_counter_reset_without_component_lineage",
                        cycle_count,
                        duplicate_count,
                    );
                }
                EventKind::EnvironmentExposure => match event.environment_within_envelope {
                    Some(false) => {
                        status = ReplayStatus::RequalificationRequired;
                        reason = "environment_exceeds_qualified_envelope";
                    }
                    Some(true) => reason = "environment_within_declared_envelope",
                    None => {
                        return Decision::new(
                            ReplayStatus::Indeterminate,
                            "environment_observation_missing",
                            cycle_count,
                            duplicate_count,
                        );
                    }
                },
            }
        }

        previous = Some(event.clone());
    }

    if case.at_tick > valid_until && status == ReplayStatus::Qualified {
        status = ReplayStatus::RequalificationRequired;
        reason = "qualification_expired_at_evaluation";
    } else if case.at_tick > last_inspection_tick.saturating_add(inspection_interval)
        && status == ReplayStatus::Qualified
    {
        status = ReplayStatus::RequalificationRequired;
        reason = "inspection_interval_expired_at_evaluation";
    }

    Decision::new(status, reason, cycle_count, duplicate_count)
}

pub fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    if corpus.schema != CORPUS_SCHEMA {
        return Err("unsupported event replay corpus schema".to_owned());
    }
    if corpus.evidence_class != "synthetic" {
        return Err("v0.1 accepts only synthetic event streams".to_owned());
    }
    if corpus.claim_ceiling != CLAIM_CEILING {
        return Err("unsupported event replay claim ceiling".to_owned());
    }
    if corpus.cases.is_empty() {
        return Err("event replay corpus must contain cases".to_owned());
    }

    let mut case_ids = HashSet::with_capacity(corpus.cases.len());
    let mut results = Vec::with_capacity(corpus.cases.len());
    let mut summary = Summary {
        case_count: corpus.cases.len(),
        qualified: 0,
        requalification_required: 0,
        withdrawn: 0,
        rejected: 0,
        disputed: 0,
        indeterminate: 0,
        duplicate_events: 0,
        expected_result_mismatches: 0,
    };

    for case in &corpus.cases {
        if case.case_id.is_empty() || !case_ids.insert(case.case_id.as_str()) {
            return Err("case IDs must be non-empty and unique".to_owned());
        }
        let decision = decide(case);
        match decision.status {
            ReplayStatus::Qualified => summary.qualified += 1,
            ReplayStatus::RequalificationRequired => summary.requalification_required += 1,
            ReplayStatus::Withdrawn => summary.withdrawn += 1,
            ReplayStatus::Rejected => summary.rejected += 1,
            ReplayStatus::Disputed => summary.disputed += 1,
            ReplayStatus::Indeterminate => summary.indeterminate += 1,
        }
        summary.duplicate_events += decision.duplicate_count;
        let matched = decision.status == case.expected_status
            && decision.reason == case.expected_reason
            && decision.cycle_count == case.expected_cycle_count
            && decision.duplicate_count == case.expected_duplicate_count;
        if !matched {
            summary.expected_result_mismatches += 1;
        }
        results.push(CaseResult {
            case_id: case.case_id.clone(),
            actual_status: decision.status,
            actual_reason: decision.reason.to_owned(),
            actual_cycle_count: decision.cycle_count,
            duplicate_event_count: decision.duplicate_count,
            expected_status: case.expected_status,
            expected_reason: case.expected_reason.clone(),
            expected_cycle_count: case.expected_cycle_count,
            expected_duplicate_count: case.expected_duplicate_count,
            expected_result_matched: matched,
        });
    }

    Ok(Report {
        result: if summary.expected_result_mismatches == 0 {
            "pass".to_owned()
        } else {
            "fail".to_owned()
        },
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        authority_granted: false,
        summary,
        cases: results,
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}
