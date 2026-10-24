//! Deterministic, paired synthetic corpus for shipyard dispatch benchmarking.
//!
//! This harness freezes identical episode inputs across four deterministic
//! policies, separates policy-visible outage observations from evaluator truth,
//! hashes exact input bytes, and retains every policy/evaluator outcome.
//! The generated corpus is synthetic and does not establish industrial validity.

use shipyard_dispatch_audit::{AuditStatus, EvaluatorOutage};
use shipyard_dispatch_experiment::{
    build_result_ledger_bundle, evaluate_four_rules, sha256_hex, EvaluatedRule,
    ExperimentLedgerContext, LedgerArtifact,
};
use shipyard_observed_dispatch::{
    Commitment, DispatchInput, DispatchRule, Operation, Resource, VisibleOutage,
};
use std::collections::BTreeSet;
use std::fmt::Write as _;

const GENERATOR_VERSION: &str = "shipyard-paired-dispatch-corpus-v0";
const RELEASE_ID: &str = "shipyard-paired-dispatch-synthetic-v0";
const BASE_CONFIG: &str = "fixed-order|earliest-due-date|critical-ratio|least-slack|serial-schedule-generation-v0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScenarioFamily {
    Nominal,
    KnownResourceOutage,
    HiddenSurprise,
    FrozenCommitmentConflict,
}

impl ScenarioFamily {
    pub const ALL: [Self; 4] = [
        Self::Nominal,
        Self::KnownResourceOutage,
        Self::HiddenSurprise,
        Self::FrozenCommitmentConflict,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::KnownResourceOutage => "known_resource_outage",
            Self::HiddenSurprise => "hidden_surprise",
            Self::FrozenCommitmentConflict => "frozen_commitment_conflict",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visibility {
    PolicyVisible,
    EvaluatorOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TapeOutage {
    event_id: String,
    resource_id: String,
    effective_from_tick: u64,
    effective_until_tick: u64,
    observed_tick: u64,
    delivery_tick: u64,
    visibility: Visibility,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkEpisode {
    pub episode_id: String,
    pub family: ScenarioFamily,
    pub partition: String,
    pub episode_seed: u64,
    pub initial_state_json: String,
    pub initial_state_sha256: String,
    pub disturbance_tape_json: String,
    pub disturbance_tape_sha256: String,
    pub policy_input: DispatchInput,
    pub evaluator_outages: Vec<EvaluatorOutage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusConfig {
    pub master_seed: u64,
    pub episode_count: usize,
    pub policy_revision: String,
    pub evaluator_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusArtifact {
    pub file_name: String,
    pub sha256: String,
    pub media_type: String,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusBundle {
    pub manifest_json: String,
    pub manifest_sha256: String,
    pub ledger_jsonl: String,
    pub ledger_sha256: String,
    pub paired_summary_json: String,
    pub paired_summary_sha256: String,
    pub run_receipt_json: String,
    pub run_receipt_sha256: String,
    pub artifacts: Vec<CorpusArtifact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EpisodeEvaluation {
    episode: BenchmarkEpisode,
    results: Vec<EvaluatedRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct MetricComparison {
    eligible_pairs: usize,
    candidate_wins: usize,
    ties: usize,
    reference_wins: usize,
    sum_delta: i128,
    median_lower: Option<i128>,
    median_upper: Option<i128>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComparisonRow {
    family: ScenarioFamily,
    partition: String,
    candidate_rule: &'static str,
    reference_rule: &'static str,
    episodes: usize,
    candidate_abstract_feasible: usize,
    reference_abstract_feasible: usize,
    both_abstract_feasible: usize,
    candidate_only_feasible: usize,
    reference_only_feasible: usize,
    neither_feasible: usize,
    candidate_partial: usize,
    candidate_invalid_input: usize,
    candidate_frozen_conflict: usize,
    candidate_audit_rejected: usize,
    makespan: MetricComparison,
    total_tardiness: MetricComparison,
    occupied_resource_ticks: MetricComparison,
}

#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }

    fn bounded(&mut self, upper_exclusive: u64) -> u64 {
        if upper_exclusive == 0 {
            0
        } else {
            self.next_u64() % upper_exclusive
        }
    }
}

fn json_quote(input: &str) -> String {
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

fn optional_u64(value: Option<u64>) -> String {
    value.map(|item| item.to_string()).unwrap_or_else(|| "null".to_owned())
}

fn serialize_initial_state(input: &DispatchInput, episode_id: &str) -> String {
    let operations = input
        .operations
        .iter()
        .map(|operation| {
            let predecessors = operation
                .predecessor_ids
                .iter()
                .map(|id| json_quote(id))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                "{{\"id\":{},\"order\":{},\"release_tick\":{},\"due_tick\":{},\"duration_ticks\":{},\"resource_kind\":{},\"predecessor_ids\":[{}]}}",
                json_quote(&operation.id),
                operation.order,
                operation.release_tick,
                operation.due_tick,
                operation.duration_ticks,
                json_quote(&operation.resource_kind),
                predecessors
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let resources = input
        .resources
        .iter()
        .map(|resource| {
            format!(
                "{{\"id\":{},\"kind\":{}}}",
                json_quote(&resource.id),
                json_quote(&resource.kind)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let completed = input
        .completed
        .iter()
        .map(|item| {
            format!(
                "{{\"operation_id\":{},\"completed_at_tick\":{}}}",
                json_quote(&item.operation_id),
                item.completed_at_tick
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let commitments = input
        .commitments
        .iter()
        .map(|item| {
            format!(
                "{{\"operation_id\":{},\"resource_id\":{},\"start_tick\":{},\"end_tick\":{}}}",
                json_quote(&item.operation_id),
                json_quote(&item.resource_id),
                item.start_tick,
                item.end_tick
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"state_contract_version\":\"shipyard-initial-state-v0\",\"episode_id\":{},\"decision_tick\":{},\"horizon_end_tick\":{},\"operations\":[{}],\"resources\":[{}],\"completed\":[{}],\"frozen_commitments\":[{}]}}",
        json_quote(episode_id),
        input.decision_tick,
        input.horizon_end_tick,
        operations,
        resources,
        completed,
        commitments
    )
}

fn serialize_disturbance_tape(episode_id: &str, events: &[TapeOutage]) -> String {
    let mut sorted = events.to_vec();
    sorted.sort_by(|left, right| {
        left.effective_from_tick
            .cmp(&right.effective_from_tick)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
    let event_json = sorted
        .iter()
        .map(|event| {
            let visibility = match event.visibility {
                Visibility::PolicyVisible => "policy_visible",
                Visibility::EvaluatorOnly => "evaluator_only",
            };
            let observation = match event.visibility {
                Visibility::PolicyVisible => format!(
                    "{{\"workstation_id\":{},\"status\":\"unavailable\",\"reported_at_tick\":{}}}",
                    json_quote(&event.resource_id),
                    event.observed_tick
                ),
                Visibility::EvaluatorOnly => "null".to_owned(),
            };
            format!(
                "{{\"event_id\":{},\"event_type\":\"workstation_outage\",\"effective_tick\":{},\"observed_tick\":{},\"delivery_tick\":{},\"visibility\":{},\"truth\":{{\"workstation_id\":{},\"unavailable_from_tick\":{},\"unavailable_until_tick\":{},\"cause\":\"synthetic_outage\"}},\"observation\":{}}}",
                json_quote(&event.event_id),
                event.effective_from_tick,
                event.observed_tick,
                event.delivery_tick,
                json_quote(visibility),
                json_quote(&event.resource_id),
                event.effective_from_tick,
                event.effective_until_tick,
                observation
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"contract_version\":\"shipyard-disturbance-tape-v0\",\"episode_id\":{},\"tick_duration_ms\":1000,\"events\":[{}],\"notes\":[\"Synthetic generated tape; not calibrated to a real yard.\",\"Evaluator-only outage truth is excluded from the policy input.\"]}}",
        json_quote(episode_id),
        event_json
    )
}

fn make_operation(
    id: String,
    order: u64,
    release_tick: u64,
    due_tick: u64,
    duration_ticks: u64,
    resource_kind: &str,
    predecessor_ids: Vec<String>,
) -> Operation {
    Operation {
        id,
        order,
        release_tick,
        due_tick,
        duration_ticks,
        resource_kind: resource_kind.to_owned(),
        predecessor_ids,
    }
}

fn make_episode(
    master_seed: u64,
    episode_index: usize,
    episode_count: usize,
) -> BenchmarkEpisode {
    let family = ScenarioFamily::ALL[episode_index % ScenarioFamily::ALL.len()];
    let family_position = episode_index / ScenarioFamily::ALL.len();
    let family_count = episode_count / ScenarioFamily::ALL.len();
    let partition = if family_position < family_count / 2 {
        "development"
    } else {
        "held_out"
    };
    let episode_seed = master_seed
        ^ (episode_index as u64).wrapping_mul(0xd6e8feb86659fd93)
        ^ 0xa0761d6478bd642f;
    let mut rng = SplitMix64::new(episode_seed);
    let operation_count = 6 + rng.bounded(3) as usize;
    let decision_tick = if family == ScenarioFamily::FrozenCommitmentConflict {
        1
    } else {
        0
    };
    let horizon_end_tick = if family == ScenarioFamily::KnownResourceOutage {
        18 + operation_count as u64
    } else if family == ScenarioFamily::FrozenCommitmentConflict {
        18
    } else if family == ScenarioFamily::HiddenSurprise {
        18 + operation_count as u64
    } else {
        8 + operation_count as u64
    };

    let mut operations = Vec::with_capacity(operation_count);
    let mut weld_ids = Vec::new();
    let mut weld_index = 0usize;
    let mut paint_index = 0usize;
    for index in 0..operation_count {
        let due_tick = 5 + rng.bounded((operation_count as u64) * 2 + 8);
        if index % 3 == 2 {
            let predecessor = weld_ids.last().cloned().unwrap_or_else(|| "weld-00".to_owned());
            let operation = make_operation(
                format!("paint-{paint_index:02}"),
                index as u64,
                0,
                due_tick,
                1 + rng.bounded(3),
                "paint",
                vec![predecessor],
            );
            paint_index += 1;
            operations.push(operation);
        } else {
            let duration = if family == ScenarioFamily::FrozenCommitmentConflict && index == 0 {
                2
            } else {
                1 + rng.bounded(3)
            };
            let release = if family == ScenarioFamily::FrozenCommitmentConflict && index == 0 {
                0
            } else if rng.bounded(7) == 0 {
                1
            } else {
                0
            };
            let id = format!("weld-{weld_index:02}");
            weld_index += 1;
            weld_ids.push(id.clone());
            operations.push(make_operation(
                id,
                index as u64,
                release,
                due_tick,
                duration,
                "welder",
                Vec::new(),
            ));
        }
    }

    let mut resources = vec![Resource {
        id: "welder-01".to_owned(),
        kind: "welder".to_owned(),
    }];
    if family != ScenarioFamily::KnownResourceOutage {
        resources.push(Resource {
            id: "welder-02".to_owned(),
            kind: "welder".to_owned(),
        });
    } else {
        resources.push(Resource {
            id: "welder-02".to_owned(),
            kind: "welder".to_owned(),
        });
    }
    resources.push(Resource {
        id: "paint-01".to_owned(),
        kind: "paint".to_owned(),
    });
    if family == ScenarioFamily::FrozenCommitmentConflict {
        // Keep this family focused on the immutable commitment, not scarcity.
        resources.push(Resource {
            id: "inspection-01".to_owned(),
            kind: "inspection".to_owned(),
        });
    }

    let mut commitments = Vec::new();
    if family == ScenarioFamily::FrozenCommitmentConflict {
        let operation = &operations[0];
        commitments.push(Commitment {
            operation_id: operation.id.clone(),
            resource_id: "welder-01".to_owned(),
            start_tick: 0,
            end_tick: operation.duration_ticks,
        });
    }
    let mut input = DispatchInput {
        decision_tick,
        horizon_end_tick,
        operations,
        resources,
        completed: Vec::new(),
        commitments,
        visible_outages: Vec::new(),
    };

    // The outage inventory is the source-owned truth; only delivered, explicitly
    // policy-visible entries are projected into DispatchInput.
    let mut tape_events = Vec::new();
    match family {
        ScenarioFamily::Nominal => {}
        ScenarioFamily::KnownResourceOutage => tape_events.push(TapeOutage {
            event_id: format!("evt-known-{episode_index:05}"),
            resource_id: "welder-02".to_owned(),
            effective_from_tick: 2,
            effective_until_tick: 5,
            observed_tick: decision_tick,
            delivery_tick: decision_tick,
            visibility: Visibility::PolicyVisible,
        }),
        ScenarioFamily::HiddenSurprise => tape_events.push(TapeOutage {
            event_id: format!("evt-hidden-{episode_index:05}"),
            resource_id: "welder-01".to_owned(),
            effective_from_tick: 1,
            effective_until_tick: 4,
            observed_tick: 4,
            delivery_tick: 5,
            visibility: Visibility::EvaluatorOnly,
        }),
        ScenarioFamily::FrozenCommitmentConflict => tape_events.push(TapeOutage {
            event_id: format!("evt-frozen-conflict-{episode_index:05}"),
            resource_id: "welder-01".to_owned(),
            effective_from_tick: 1,
            effective_until_tick: 4,
            observed_tick: decision_tick,
            delivery_tick: decision_tick,
            visibility: Visibility::PolicyVisible,
        }),
    }
    input.visible_outages = tape_events
        .iter()
        .filter(|event| {
            event.visibility == Visibility::PolicyVisible && event.delivery_tick <= decision_tick
        })
        .map(|event| VisibleOutage {
            event_id: event.event_id.clone(),
            resource_id: event.resource_id.clone(),
            effective_from_tick: event.effective_from_tick,
            effective_until_tick: event.effective_until_tick,
            observed_at_tick: event.observed_tick,
        })
        .collect();

    let mut evaluator_outages = tape_events
        .iter()
        .map(|event| EvaluatorOutage {
            event_id: event.event_id.clone(),
            resource_id: event.resource_id.clone(),
            effective_from_tick: event.effective_from_tick,
            effective_until_tick: event.effective_until_tick,
        })
        .collect::<Vec<_>>();
    evaluator_outages.sort_by(|left, right| {
        left.effective_from_tick
            .cmp(&right.effective_from_tick)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });

    let episode_id = format!("episode-{episode_index:05}");
    let initial_state_json = serialize_initial_state(&input, &episode_id);
    let disturbance_tape_json = serialize_disturbance_tape(&episode_id, &tape_events);
    let initial_state_sha256 = sha256_hex(initial_state_json.as_bytes());
    let disturbance_tape_sha256 = sha256_hex(disturbance_tape_json.as_bytes());

    BenchmarkEpisode {
        episode_id,
        family,
        partition: partition.to_owned(),
        episode_seed,
        initial_state_json,
        initial_state_sha256,
        disturbance_tape_json,
        disturbance_tape_sha256,
        policy_input: input,
        evaluator_outages,
    }
}

/// Generate an immutable corpus. The count must be a multiple of eight so that
/// every family has equal-size development and held-out partitions.
pub fn generate_corpus(config: &CorpusConfig) -> Result<Vec<BenchmarkEpisode>, String> {
    if config.episode_count < 8 || config.episode_count % 8 != 0 {
        return Err("episode_count must be a multiple of eight and at least eight".to_owned());
    }
    if config.policy_revision.trim().is_empty() || config.evaluator_revision.trim().is_empty() {
        return Err("policy_revision and evaluator_revision must not be empty".to_owned());
    }
    Ok((0..config.episode_count)
        .map(|index| make_episode(config.master_seed, index, config.episode_count))
        .collect())
}

fn rule_name(rule: DispatchRule) -> &'static str {
    match rule {
        DispatchRule::FixedOrder => "fixed_order",
        DispatchRule::EarliestDueDate => "earliest_due_date",
        DispatchRule::CriticalRatio => "critical_ratio",
        DispatchRule::LeastSlack => "least_slack",
    }
}

fn abstract_feasible(result: &EvaluatedRule) -> bool {
    result.policy_status == shipyard_observed_dispatch::PlanStatus::Complete
        && result.audit.status == AuditStatus::AbstractFeasible
}

fn result_for(results: &[EvaluatedRule], rule: DispatchRule) -> &EvaluatedRule {
    results
        .iter()
        .find(|result| result.rule == rule)
        .expect("the four-rule evaluation always returns every registered rule")
}

fn metric_deltas(
    deltas: &mut Vec<i128>,
    candidate: &EvaluatedRule,
    reference: &EvaluatedRule,
    metric: &str,
) {
    if !abstract_feasible(candidate) || !abstract_feasible(reference) {
        return;
    }
    let candidate_value = match metric {
        "makespan" => candidate.audit.metrics.makespan_tick,
        "total_tardiness" => Some(candidate.audit.metrics.total_tardiness_ticks),
        "occupied_resource_ticks" => Some(candidate.audit.metrics.occupied_resource_ticks),
        _ => None,
    };
    let reference_value = match metric {
        "makespan" => reference.audit.metrics.makespan_tick,
        "total_tardiness" => Some(reference.audit.metrics.total_tardiness_ticks),
        "occupied_resource_ticks" => Some(reference.audit.metrics.occupied_resource_ticks),
        _ => None,
    };
    if let (Some(candidate_value), Some(reference_value)) = (candidate_value, reference_value) {
        deltas.push(candidate_value as i128 - reference_value as i128);
    }
}

fn summarize_deltas(deltas: &mut [i128]) -> MetricComparison {
    deltas.sort_unstable();
    let mut summary = MetricComparison {
        eligible_pairs: deltas.len(),
        sum_delta: deltas.iter().sum(),
        ..MetricComparison::default()
    };
    for delta in deltas.iter().copied() {
        if delta < 0 {
            summary.candidate_wins += 1;
        } else if delta == 0 {
            summary.ties += 1;
        } else {
            summary.reference_wins += 1;
        }
    }
    if !deltas.is_empty() {
        summary.median_lower = Some(deltas[(deltas.len() - 1) / 2]);
        summary.median_upper = Some(deltas[deltas.len() / 2]);
    }
    summary
}

fn build_comparison_rows(evaluations: &[EpisodeEvaluation]) -> Vec<ComparisonRow> {
    let mut rows = Vec::new();
    for family in ScenarioFamily::ALL {
        for partition in ["development", "held_out"] {
            let subset: Vec<&EpisodeEvaluation> = evaluations
                .iter()
                .filter(|entry| entry.episode.family == family && entry.episode.partition == partition)
                .collect();
            if subset.is_empty() {
                continue;
            }
            for candidate_rule in [
                DispatchRule::FixedOrder,
                DispatchRule::EarliestDueDate,
                DispatchRule::CriticalRatio,
                DispatchRule::LeastSlack,
            ] {
                let mut candidate_feasible = 0usize;
                let mut reference_feasible = 0usize;
                let mut both_feasible = 0usize;
                let mut candidate_only = 0usize;
                let mut reference_only = 0usize;
                let mut neither = 0usize;
                let mut candidate_partial = 0usize;
                let mut candidate_invalid_input = 0usize;
                let mut candidate_frozen_conflict = 0usize;
                let mut candidate_audit_rejected = 0usize;
                let mut makespan_deltas = Vec::new();
                let mut tardiness_deltas = Vec::new();
                let mut occupied_deltas = Vec::new();

                for entry in &subset {
                    let candidate = result_for(&entry.results, candidate_rule);
                    let reference = result_for(&entry.results, DispatchRule::EarliestDueDate);
                    let candidate_ok = abstract_feasible(candidate);
                    let reference_ok = abstract_feasible(reference);
                    candidate_feasible += usize::from(candidate_ok);
                    reference_feasible += usize::from(reference_ok);
                    match (candidate_ok, reference_ok) {
                        (true, true) => both_feasible += 1,
                        (true, false) => candidate_only += 1,
                        (false, true) => reference_only += 1,
                        (false, false) => neither += 1,
                    }
                    if candidate.policy_status == shipyard_observed_dispatch::PlanStatus::Partial {
                        candidate_partial += 1;
                    }
                    if candidate.policy_status == shipyard_observed_dispatch::PlanStatus::InvalidInput {
                        candidate_invalid_input += 1;
                    }
                    if candidate.policy_status == shipyard_observed_dispatch::PlanStatus::FrozenConflict {
                        candidate_frozen_conflict += 1;
                    }
                    if candidate.audit.status == AuditStatus::Rejected {
                        candidate_audit_rejected += 1;
                    }
                    metric_deltas(&mut makespan_deltas, candidate, reference, "makespan");
                    metric_deltas(&mut tardiness_deltas, candidate, reference, "total_tardiness");
                    metric_deltas(&mut occupied_deltas, candidate, reference, "occupied_resource_ticks");
                }

                rows.push(ComparisonRow {
                    family,
                    partition: partition.to_owned(),
                    candidate_rule: rule_name(candidate_rule),
                    reference_rule: "earliest_due_date",
                    episodes: subset.len(),
                    candidate_abstract_feasible: candidate_feasible,
                    reference_abstract_feasible: reference_feasible,
                    both_abstract_feasible: both_feasible,
                    candidate_only_feasible: candidate_only,
                    reference_only_feasible: reference_only,
                    neither_feasible: neither,
                    candidate_partial,
                    candidate_invalid_input,
                    candidate_frozen_conflict,
                    candidate_audit_rejected,
                    makespan: summarize_deltas(&mut makespan_deltas),
                    total_tardiness: summarize_deltas(&mut tardiness_deltas),
                    occupied_resource_ticks: summarize_deltas(&mut occupied_deltas),
                });
            }
        }
    }
    rows
}

fn metric_comparison_json(metric: &MetricComparison) -> String {
    format!(
        "{{\"eligible_pairs\":{},\"candidate_wins\":{},\"ties\":{},\"reference_wins\":{},\"sum_delta_candidate_minus_reference\":{},\"mean_delta_numerator\":{},\"mean_delta_denominator\":{},\"median_delta_lower\":{},\"median_delta_upper\":{}}}",
        metric.eligible_pairs,
        metric.candidate_wins,
        metric.ties,
        metric.reference_wins,
        metric.sum_delta,
        metric.sum_delta,
        metric.eligible_pairs,
        optional_i128(metric.median_lower),
        optional_i128(metric.median_upper)
    )
}

fn optional_i128(value: Option<i128>) -> String {
    value.map(|item| item.to_string()).unwrap_or_else(|| "null".to_owned())
}

fn summarize_json(rows: &[ComparisonRow], evaluations: &[EpisodeEvaluation]) -> String {
    let comparison_json = rows
        .iter()
        .map(|row| {
            format!(
                "{{\"family\":{},\"partition\":{},\"candidate_rule\":{},\"reference_rule\":{},\"episodes\":{},\"candidate_abstract_feasible\":{},\"reference_abstract_feasible\":{},\"both_abstract_feasible\":{},\"candidate_only_feasible\":{},\"reference_only_feasible\":{},\"neither_feasible\":{},\"candidate_partial\":{},\"candidate_invalid_input\":{},\"candidate_frozen_conflict\":{},\"candidate_audit_rejected\":{},\"paired_metrics\":{{\"makespan_tick\":{},\"total_tardiness_ticks\":{},\"occupied_resource_ticks\":{}}}}}",
                json_quote(row.family.as_str()),
                json_quote(&row.partition),
                json_quote(row.candidate_rule),
                json_quote(row.reference_rule),
                row.episodes,
                row.candidate_abstract_feasible,
                row.reference_abstract_feasible,
                row.both_abstract_feasible,
                row.candidate_only_feasible,
                row.reference_only_feasible,
                row.neither_feasible,
                row.candidate_partial,
                row.candidate_invalid_input,
                row.candidate_frozen_conflict,
                row.candidate_audit_rejected,
                metric_comparison_json(&row.makespan),
                metric_comparison_json(&row.total_tardiness),
                metric_comparison_json(&row.occupied_resource_ticks)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let mut family_coverage = Vec::new();
    for family in ScenarioFamily::ALL {
        for partition in ["development", "held_out"] {
            let count = evaluations
                .iter()
                .filter(|entry| entry.episode.family == family && entry.episode.partition == partition)
                .count();
            if count > 0 {
                family_coverage.push(format!(
                    "{{\"family\":{},\"partition\":{},\"episodes\":{}}}",
                    json_quote(family.as_str()),
                    json_quote(partition),
                    count
                ));
            }
        }
    }
    format!(
        "{{\"summary_contract_version\":\"shipyard-paired-comparison-summary-v0\",\"interpretation\":\"descriptive synthetic corpus only; metric comparisons use paired episodes where both candidate and reference pass the abstract independent audit\",\"higher_is_better_for_feasibility\":true,\"lower_is_better_metrics\":[\"makespan_tick\",\"total_tardiness_ticks\",\"occupied_resource_ticks\"],\"episode_coverage\":[{}],\"comparisons\":[{}]}}",
        family_coverage.join(","),
        comparison_json
    )
}

fn manifest_json(config: &CorpusConfig, episodes: &[BenchmarkEpisode]) -> String {
    let entries = episodes
        .iter()
        .map(|episode| {
            format!(
                "{{\"episode_id\":{},\"family\":{},\"partition\":{},\"episode_seed_hex\":{},\"initial_state_sha256\":{},\"disturbance_tape_sha256\":{}}}",
                json_quote(&episode.episode_id),
                json_quote(episode.family.as_str()),
                json_quote(&episode.partition),
                json_quote(&format!("{:016x}", episode.episode_seed)),
                json_quote(&episode.initial_state_sha256),
                json_quote(&episode.disturbance_tape_sha256)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"corpus_contract_version\":\"shipyard-paired-benchmark-corpus-v0\",\"generator_version\":{},\"benchmark_release_id\":{},\"master_seed_hex\":{},\"episode_count\":{},\"partition_policy\":\"first half within each family is development; second half is held_out\",\"policy_revision\":{},\"evaluator_revision\":{},\"budget_contract\":\"not_enforced; runtime comparisons prohibited\",\"scenario_families\":[\"nominal\",\"known_resource_outage\",\"hidden_surprise\",\"frozen_commitment_conflict\"],\"episodes\":[{}]}}",
        json_quote(GENERATOR_VERSION),
        json_quote(RELEASE_ID),
        json_quote(&format!("{:016x}", config.master_seed)),
        config.episode_count,
        json_quote(&config.policy_revision),
        json_quote(&config.evaluator_revision),
        entries
    )
}

/// Generate and run all four baselines for every corpus episode. Exact initial
/// state, full truth tape, ledger records, and candidate/audit sidecars are
/// retained as digest-addressed artifacts. Runtime budgets are not enforced.
pub fn build_corpus_bundle(config: &CorpusConfig) -> Result<CorpusBundle, String> {
    let episodes = generate_corpus(config)?;
    let manifest_json = manifest_json(config, &episodes);
    let manifest_sha256 = sha256_hex(manifest_json.as_bytes());
    let mut evaluations = Vec::with_capacity(episodes.len());
    let mut artifacts = Vec::<CorpusArtifact>::with_capacity(episodes.len() * 14);
    let base_method_config_sha256 = sha256_hex(BASE_CONFIG.as_bytes());
    let budget_contract =
        "no wall-clock or compute-unit enforcement in this runner; do not use for runtime claims";

    for episode in episodes {
        artifacts.push(CorpusArtifact {
            file_name: format!("initial-state-{}.json", episode.initial_state_sha256),
            sha256: episode.initial_state_sha256.clone(),
            media_type: "application/json".to_owned(),
            contents: episode.initial_state_json.clone(),
        });
        artifacts.push(CorpusArtifact {
            file_name: format!("disturbance-tape-{}.json", episode.disturbance_tape_sha256),
            sha256: episode.disturbance_tape_sha256.clone(),
            media_type: "application/json".to_owned(),
            contents: episode.disturbance_tape_json.clone(),
        });
        let results = evaluate_four_rules(
            &episode.policy_input,
            &[],
            &episode.evaluator_outages,
        )?;
        let context = ExperimentLedgerContext {
            benchmark_release_id: RELEASE_ID.to_owned(),
            episode_id: episode.episode_id.clone(),
            replicate_id: format!("seed-{:016x}", episode.episode_seed),
            initial_state_sha256: episode.initial_state_sha256.clone(),
            disturbance_tape_sha256: episode.disturbance_tape_sha256.clone(),
            base_method_config_sha256: base_method_config_sha256.clone(),
            budget_profile_id: "unbounded-deterministic-smoke-v0".to_owned(),
            wall_time_limit_ms: 0,
            decision_compute_budget_units: 0,
            budget_contract: budget_contract.to_owned(),
            policy_revision: config.policy_revision.clone(),
            evaluator_revision: config.evaluator_revision.clone(),
            synthetic_example: true,
        };
        let ledger_bundle = build_result_ledger_bundle(&context, &results)?;
        for artifact in ledger_bundle.artifacts {
            artifacts.push(CorpusArtifact {
                file_name: artifact.file_name,
                sha256: artifact.sha256,
                media_type: artifact.media_type,
                contents: artifact.contents,
            });
        }
        evaluations.push(EpisodeEvaluation { episode, results });
    }

    let ledger_jsonl = {
        let mut text = String::new();
        for evaluation in &evaluations {
            let context = ExperimentLedgerContext {
                benchmark_release_id: RELEASE_ID.to_owned(),
                episode_id: evaluation.episode.episode_id.clone(),
                replicate_id: format!("seed-{:016x}", evaluation.episode.episode_seed),
                initial_state_sha256: evaluation.episode.initial_state_sha256.clone(),
                disturbance_tape_sha256: evaluation.episode.disturbance_tape_sha256.clone(),
                base_method_config_sha256: base_method_config_sha256.clone(),
                budget_profile_id: "unbounded-deterministic-smoke-v0".to_owned(),
                wall_time_limit_ms: 0,
                decision_compute_budget_units: 0,
                budget_contract: budget_contract.to_owned(),
                policy_revision: config.policy_revision.clone(),
                evaluator_revision: config.evaluator_revision.clone(),
                synthetic_example: true,
            };
            text.push_str(&build_result_ledger_bundle(&context, &evaluation.results)?.ledger_jsonl);
        }
        text
    };
    let ledger_sha256 = sha256_hex(ledger_jsonl.as_bytes());
    let paired_summary_json = summarize_json(&build_comparison_rows(&evaluations), &evaluations);
    let paired_summary_sha256 = sha256_hex(paired_summary_json.as_bytes());

    let run_receipt_json = format!(
        "{{\"receipt_contract_version\":\"shipyard-benchmark-run-receipt-v0\",\"benchmark_release_id\":{},\"generator_version\":{},\"master_seed_hex\":{},\"policy_revision\":{},\"evaluator_revision\":{},\"manifest_sha256\":{},\"ledger_sha256\":{},\"paired_summary_sha256\":{},\"artifact_count\":{},\"budget_contract\":\"not_enforced; runtime comparisons prohibited\",\"result_status_scope\":\"all records incomplete until full geometry, causality, receipt-integrity, and runtime-budget gates are executed\"}}",
        json_quote(RELEASE_ID),
        json_quote(GENERATOR_VERSION),
        json_quote(&format!("{:016x}", config.master_seed)),
        json_quote(&config.policy_revision),
        json_quote(&config.evaluator_revision),
        json_quote(&manifest_sha256),
        json_quote(&ledger_sha256),
        json_quote(&paired_summary_sha256),
        artifacts.len()
    );
    let run_receipt_sha256 = sha256_hex(run_receipt_json.as_bytes());

    Ok(CorpusBundle {
        manifest_json,
        manifest_sha256,
        ledger_jsonl,
        ledger_sha256,
        paired_summary_json,
        paired_summary_sha256,
        run_receipt_json,
        run_receipt_sha256,
        artifacts,
    })
}

/// Persist exact output bytes. Existing files are not silently overwritten.
pub fn write_corpus_bundle(bundle: &CorpusBundle, output_dir: &std::path::Path) -> Result<(), String> {
    use std::fs::{self, OpenOptions};
    use std::io::Write;

    fs::create_dir_all(output_dir)
        .map_err(|error| format!("cannot create {}: {error}", output_dir.display()))?;
    let write_new = |name: &str, content: &[u8]| -> Result<(), String> {
        let path = output_dir.join(name);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| format!("refusing to overwrite {}: {error}", path.display()))?;
        file.write_all(content)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        file.sync_all()
            .map_err(|error| format!("cannot sync {}: {error}", path.display()))?;
        Ok(())
    };

    write_new("benchmark-manifest.json", bundle.manifest_json.as_bytes())?;
    write_new("results.jsonl", bundle.ledger_jsonl.as_bytes())?;
    write_new("paired-comparison-summary.json", bundle.paired_summary_json.as_bytes())?;
    write_new("run-receipt.json", bundle.run_receipt_json.as_bytes())?;
    for artifact in &bundle.artifacts {
        write_new(&artifact.file_name, artifact.contents.as_bytes())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(episode_count: usize) -> CorpusConfig {
        CorpusConfig {
            master_seed: 20261024,
            episode_count,
            policy_revision: "test-policy-revision".to_owned(),
            evaluator_revision: "test-evaluator-revision".to_owned(),
        }
    }

    #[test]
    fn generator_requires_balanced_development_and_held_out_partitions() {
        assert!(generate_corpus(&config(7)).is_err());
        assert!(generate_corpus(&config(10)).is_err());
        let episodes = generate_corpus(&config(16)).expect("valid episode count");
        assert_eq!(episodes.len(), 16);
        for family in ScenarioFamily::ALL {
            assert_eq!(
                episodes
                    .iter()
                    .filter(|episode| episode.family == family && episode.partition == "development")
                    .count(),
                2
            );
            assert_eq!(
                episodes
                    .iter()
                    .filter(|episode| episode.family == family && episode.partition == "held_out")
                    .count(),
                2
            );
        }
    }

    #[test]
    fn same_seed_yields_byte_identical_manifest_inputs_and_tapes() {
        let first = generate_corpus(&config(16)).expect("first corpus");
        let second = generate_corpus(&config(16)).expect("second corpus");
        assert_eq!(first, second);
        assert!(first.iter().all(|episode| {
            sha256_hex(episode.initial_state_json.as_bytes()) == episode.initial_state_sha256
                && sha256_hex(episode.disturbance_tape_json.as_bytes())
                    == episode.disturbance_tape_sha256
        }));
    }

    #[test]
    fn policy_view_excludes_evaluator_only_outage_truth() {
        let episodes = generate_corpus(&config(16)).expect("corpus");
        let hidden = episodes
            .iter()
            .find(|episode| episode.family == ScenarioFamily::HiddenSurprise)
            .expect("hidden scenario family");
        assert_eq!(hidden.policy_input.visible_outages.len(), 0);
        assert_eq!(hidden.evaluator_outages.len(), 1);
        assert!(hidden.disturbance_tape_json.contains("\"visibility\":\"evaluator_only\""));
        assert!(!hidden.policy_input.visible_outages.iter().any(|item| item.event_id == hidden.evaluator_outages[0].event_id));
    }

    #[test]
    fn frozen_commitment_conflict_is_preserved_for_auditor_to_reject() {
        let episodes = generate_corpus(&config(16)).expect("corpus");
        let episode = episodes
            .iter()
            .find(|entry| entry.family == ScenarioFamily::FrozenCommitmentConflict)
            .expect("frozen-conflict family");
        assert_eq!(episode.policy_input.commitments.len(), 1);
        let results = evaluate_four_rules(&episode.policy_input, &[], &episode.evaluator_outages)
            .expect("evaluate four rules");
        assert!(results.iter().all(|result| {
            result.plan.assignments.iter().any(|assignment| {
                assignment.operation_id == episode.policy_input.commitments[0].operation_id
                    && assignment.frozen
            })
        }));
        assert!(results.iter().all(|result| result.audit.status == AuditStatus::Rejected));
    }

    #[test]
    fn ledger_and_comparison_summary_keep_all_episodes_and_method_outcomes() {
        let bundle = build_corpus_bundle(&config(16)).expect("build synthetic corpus");
        assert_eq!(bundle.ledger_jsonl.lines().count(), 16 * 4);
        assert_eq!(sha256_hex(bundle.ledger_jsonl.as_bytes()), bundle.ledger_sha256);
        assert_eq!(sha256_hex(bundle.manifest_json.as_bytes()), bundle.manifest_sha256);
        assert_eq!(
            sha256_hex(bundle.paired_summary_json.as_bytes()),
            bundle.paired_summary_sha256
        );
        assert!(bundle.paired_summary_json.contains("\"partition\":\"held_out\""));
        assert!(bundle.paired_summary_json.contains("\"candidate_audit_rejected\""));
        assert!(!bundle.paired_summary_json.contains("winner_score"));
        assert!(bundle.run_receipt_json.contains(&bundle.manifest_sha256));
        assert!(bundle.run_receipt_json.contains(&bundle.ledger_sha256));
        assert_eq!(bundle.artifacts.len(), 16 * 14);
        for artifact in &bundle.artifacts {
            assert_eq!(sha256_hex(artifact.contents.as_bytes()), artifact.sha256);
        }
    }

    #[test]
    fn hidden_truth_is_absent_from_the_policy_visible_projection() {
        let episodes = generate_corpus(&config(16)).expect("corpus");
        for episode in episodes
            .iter()
            .filter(|item| item.family == ScenarioFamily::HiddenSurprise)
        {
            assert_eq!(episode.policy_input.visible_outages.len(), 0);
            assert_eq!(episode.evaluator_outages.len(), 1);
            let hidden_id = &episode.evaluator_outages[0].event_id;
            assert!(episode
                .policy_input
                .visible_outages
                .iter()
                .all(|visible| &visible.event_id != hidden_id));
            assert!(episode.disturbance_tape_json.contains("\"visibility\":\"evaluator_only\""));
        }
    }
}
