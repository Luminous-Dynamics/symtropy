use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;

const SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_v02_challenge_corpus.schema.json");
const EVALUATOR_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_v02_evaluator_labels.schema.json");
const STREAMS_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_v02_scenario_streams.schema.json");
const ROOT_SEED: u64 = 20_261_124;
const FAMILY_COUNT: usize = 6;
const REPLICATIONS_PER_FAMILY: usize = 20;
const HORIZON_EPOCHS: usize = 4;
const SCENARIOS_PER_CASE: usize = 4;
const PROBABILITY_TOTAL_PPM: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DataSplit {
    Development,
    HeldOut,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum OperationStatus {
    Waiting,
    InProcess,
    Completed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum QualificationDisposition {
    Admissible,
    Unknown,
    Blocked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum ResourceEventKind {
    Failed,
    Restored,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum QualityDisposition {
    Accepted,
    Rejected,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct GenerationManifest {
    root_seed: u64,
    seed_algorithm: String,
    seed_derivation: String,
    pairing_rule: String,
    split_rule: String,
    tuning_rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct FamilySpec {
    scenario_family_id: String,
    split: DataSplit,
    replication_seeds: Vec<u64>,
    primary_mechanism: String,
    decision_tension_under_test: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Corpus {
    schema_id: String,
    benchmark_id: String,
    generator_version: String,
    evidence_class: String,
    generation: GenerationManifest,
    horizon_epochs: usize,
    family_count: usize,
    family_manifest: Vec<FamilySpec>,
    case_count: usize,
    cases: Vec<ChallengeCase>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ChallengeCase {
    case_id: String,
    scenario_family_id: String,
    split: DataSplit,
    replication_seed: u64,
    generator_seed: u64,
    paired_seed_id: String,
    horizon_epochs: usize,
    decision_visibility_rule: String,
    jobs: Vec<Job>,
    resources: Vec<Resource>,
    setup_transitions: Vec<SetupTransition>,
    inspection_station: InspectionStation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Job {
    job_id: String,
    due_epoch: usize,
    value_units: u64,
    tardiness_penalty_per_epoch: u64,
    unserved_penalty_units: u64,
    operations: Vec<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Operation {
    operation_id: String,
    sequence: usize,
    process_family: String,
    setup_family: String,
    predecessor_operation_ids: Vec<String>,
    eligible_resource_ids: Vec<String>,
    processing_time_ticks_by_resource: BTreeMap<String, u64>,
    initial_status: OperationStatus,
    committed_resource_id: Option<String>,
    remaining_processing_ticks: u64,
    inspection_required: bool,
    inspection_method_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Resource {
    resource_id: String,
    capability_families: Vec<String>,
    common_cause_domain: String,
    capacity_units_by_epoch: Vec<u64>,
    initial_setup_family: String,
    initial_qualifications: Vec<ResourceQualification>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ResourceQualification {
    process_family: String,
    disposition: QualificationDisposition,
    evidence_receipt_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SetupTransition {
    resource_id: String,
    from_setup_family: String,
    to_setup_family: String,
    duration_ticks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct InspectionStation {
    station_id: String,
    capacity_units_by_epoch: Vec<u64>,
    methods: Vec<InspectionMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct InspectionMethod {
    method_id: String,
    applicable_process_families: Vec<String>,
    duration_ticks: u64,
    cost_units: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    probability_ppm: u64,
    observations_by_epoch: Vec<EpochObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ScenarioStreams {
    schema_id: String,
    benchmark_id: String,
    evidence_class: String,
    case_count: usize,
    cases: Vec<ScenarioStreamCase>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ScenarioStreamCase {
    case_id: String,
    scenario_family_id: String,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct EvaluatorLabels {
    schema_id: String,
    benchmark_id: String,
    evidence_class: String,
    case_count: usize,
    cases: Vec<EvaluatorCase>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct EvaluatorCase {
    case_id: String,
    scenario_family_id: String,
    scenario_outcomes: Vec<EvaluatorScenarioOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct EvaluatorScenarioOutcome {
    scenario_id: String,
    hidden_terminal_quality_outcomes: Vec<HiddenQualityOutcome>,
}

struct ScenarioBundle {
    streams: Vec<Scenario>,
    evaluator_outcomes: Vec<EvaluatorScenarioOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct EpochObservation {
    epoch: usize,
    resource_events: Vec<ResourceEvent>,
    qualification_updates: Vec<QualificationUpdate>,
    inspection_capacity_events: Vec<InspectionCapacityEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ResourceEvent {
    source_event_id: String,
    resource_id: String,
    kind: ResourceEventKind,
    occurred_at_epoch: usize,
    observed_at_epoch: usize,
    common_cause_id: Option<String>,
    recovery_forecast_min_epochs: Option<u64>,
    recovery_forecast_max_epochs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct QualificationUpdate {
    evidence_receipt_id: String,
    resource_id: String,
    process_family: String,
    previous_disposition: QualificationDisposition,
    new_disposition: QualificationDisposition,
    occurred_at_epoch: usize,
    observed_at_epoch: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct InspectionCapacityEvent {
    source_event_id: String,
    station_id: String,
    available_capacity_units: u64,
    occurred_at_epoch: usize,
    observed_at_epoch: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct HiddenQualityOutcome {
    outcome_id: String,
    operation_id: String,
    disposition: QualityDisposition,
}

struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix64(self.state)
    }

    fn between(&mut self, lower: u64, upper: u64) -> u64 {
        debug_assert!(lower <= upper);
        lower + (self.next_u64() % (upper - lower + 1))
    }
}

fn mix64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

fn derive_seed(family_index: usize, replication_seed: u64) -> u64 {
    mix64(ROOT_SEED ^ ((family_index as u64 + 1) << 32) ^ replication_seed)
}

fn family_manifest() -> Vec<FamilySpec> {
    let seeds: Vec<u64> = (1..=REPLICATIONS_PER_FAMILY as u64).collect();
    vec![
        FamilySpec {
            scenario_family_id: "crossed-bottlenecks".into(),
            split: DataSplit::Development,
            replication_seeds: seeds.clone(),
            primary_mechanism: "overlapping eligibility with two interacting capacity bottlenecks"
                .into(),
            decision_tension_under_test: "proactive reservation versus local greedy dispatch"
                .into(),
        },
        FamilySpec {
            scenario_family_id: "common-cause-outage".into(),
            split: DataSplit::Development,
            replication_seeds: seeds.clone(),
            primary_mechanism:
                "correlated loss and restoration of resources sharing a failure domain".into(),
            decision_tension_under_test: "resource diversity versus nominal processing cost".into(),
        },
        FamilySpec {
            scenario_family_id: "uncertain-recovery".into(),
            split: DataSplit::Development,
            replication_seeds: seeds.clone(),
            primary_mechanism: "uncertain outage duration with delayed source observations".into(),
            decision_tension_under_test: "bounded hold versus premature reassignment".into(),
        },
        FamilySpec {
            scenario_family_id: "qualification-transition".into(),
            split: DataSplit::Development,
            replication_seeds: seeds.clone(),
            primary_mechanism:
                "Unknown qualification becomes admissible only after a visible scoped receipt"
                    .into(),
            decision_tension_under_test: "defer an Unknown route until evidence is visible".into(),
        },
        FamilySpec {
            scenario_family_id: "setup-wip-pressure".into(),
            split: DataSplit::HeldOut,
            replication_seeds: seeds.clone(),
            primary_mechanism: "sequence-dependent setups with committed and in-process work"
                .into(),
            decision_tension_under_test: "preserve committed WIP versus setup-aware rerouting"
                .into(),
        },
        FamilySpec {
            scenario_family_id: "inspection-value".into(),
            split: DataSplit::HeldOut,
            replication_seeds: seeds,
            primary_mechanism:
                "inspection-capacity contention and hidden terminal quality outcomes".into(),
            decision_tension_under_test:
                "inspect when measurement can change disposition, not merely reduce entropy".into(),
        },
    ]
}

fn eligible_resources(process_family: &str) -> &'static [&'static str] {
    match process_family {
        "CUTTING" => &["RESOURCE-A", "RESOURCE-B"],
        "FORMING" => &["RESOURCE-A", "RESOURCE-C"],
        "FINISHING" => &["RESOURCE-B", "RESOURCE-C"],
        _ => unreachable!("generator uses only declared process families"),
    }
}

fn process_families_for_job(job_index: usize) -> [&'static str; 2] {
    match job_index {
        0 => ["CUTTING", "FORMING"],
        1 => ["FORMING", "FINISHING"],
        _ => ["CUTTING", "FINISHING"],
    }
}

fn make_jobs(family_id: &str, rng: &mut DeterministicRng) -> Vec<Job> {
    (0..3)
        .map(|job_index| {
            let job_id = format!("JOB-{}", job_index + 1);
            let process_families = process_families_for_job(job_index);
            let operations = (0..2)
                .map(|operation_index| {
                    let operation_id = format!("{job_id}-OP-{}", operation_index + 1);
                    let process_family = process_families[operation_index].to_string();
                    let eligible = eligible_resources(&process_family);
                    let processing_time_ticks_by_resource = eligible
                        .iter()
                        .map(|resource_id| ((*resource_id).to_string(), rng.between(1, 4)))
                        .collect();

                    let in_process = job_index == 2 && operation_index == 0;
                    let completed = job_index == 0 && operation_index == 0;
                    let inspection_required =
                        family_id == "inspection-value" && operation_index == 1;
                    let inspection_method_id =
                        inspection_required.then(|| "INDEPENDENT-DIMENSIONAL".to_string());

                    Operation {
                        operation_id,
                        sequence: operation_index + 1,
                        setup_family: (match process_family.as_str() {
                            "CUTTING" => "SETUP-A",
                            "FORMING" => "SETUP-B",
                            "FINISHING" => "SETUP-C",
                            _ => unreachable!("known process family"),
                        })
                        .to_string(),
                        process_family,
                        predecessor_operation_ids: if operation_index == 0 {
                            Vec::new()
                        } else {
                            vec![format!("{job_id}-OP-1")]
                        },
                        eligible_resource_ids: eligible
                            .iter()
                            .map(|id| (*id).to_string())
                            .collect(),
                        processing_time_ticks_by_resource,
                        initial_status: if in_process {
                            OperationStatus::InProcess
                        } else if completed {
                            OperationStatus::Completed
                        } else {
                            OperationStatus::Waiting
                        },
                        committed_resource_id: in_process.then(|| "RESOURCE-A".to_string()),
                        remaining_processing_ticks: if in_process { 1 } else { 0 },
                        inspection_required,
                        inspection_method_id,
                    }
                })
                .collect();

            Job {
                job_id,
                due_epoch: rng.between(2, 4) as usize,
                value_units: rng.between(10, 40),
                tardiness_penalty_per_epoch: rng.between(2, 8),
                unserved_penalty_units: rng.between(30, 80),
                operations,
            }
        })
        .collect()
}

fn make_resources(family_id: &str) -> Vec<Resource> {
    let definitions = [
        (
            "RESOURCE-A",
            vec!["CUTTING", "FORMING"],
            "POWER-DOMAIN-1",
            "SETUP-A",
        ),
        (
            "RESOURCE-B",
            vec!["CUTTING", "FINISHING"],
            "POWER-DOMAIN-1",
            "SETUP-B",
        ),
        (
            "RESOURCE-C",
            vec!["FORMING", "FINISHING"],
            "POWER-DOMAIN-2",
            "SETUP-C",
        ),
    ];

    definitions
        .into_iter()
        .map(
            |(resource_id, capabilities, common_cause_domain, initial_setup_family)| {
                let initial_qualifications = capabilities
                    .iter()
                    .map(|process_family| {
                        let is_forming_transition =
                            family_id == "qualification-transition" && *process_family == "FORMING";
                        let is_unknown_transition =
                            is_forming_transition && resource_id == "RESOURCE-C";
                        let is_blocked_competitor =
                            is_forming_transition && resource_id == "RESOURCE-A";
                        ResourceQualification {
                            process_family: (*process_family).to_string(),
                            disposition: if is_unknown_transition {
                                QualificationDisposition::Unknown
                            } else if is_blocked_competitor {
                                QualificationDisposition::Blocked
                            } else {
                                QualificationDisposition::Admissible
                            },
                            evidence_receipt_id: format!(
                                "QUAL-INITIAL-{resource_id}-{process_family}"
                            ),
                        }
                    })
                    .collect();

                let capacity_units_by_epoch = if family_id == "crossed-bottlenecks" {
                    match resource_id {
                        "RESOURCE-A" => vec![1, 0, 1, 1],
                        "RESOURCE-B" => vec![1, 1, 0, 1],
                        _ => vec![1, 1, 1, 1],
                    }
                } else {
                    vec![1, 1, 1, 1]
                };

                Resource {
                    resource_id: resource_id.to_string(),
                    capability_families: capabilities
                        .iter()
                        .map(|value| (*value).to_string())
                        .collect(),
                    common_cause_domain: common_cause_domain.to_string(),
                    capacity_units_by_epoch,
                    initial_setup_family: initial_setup_family.to_string(),
                    initial_qualifications,
                }
            },
        )
        .collect()
}

fn make_setup_transitions(rng: &mut DeterministicRng) -> Vec<SetupTransition> {
    let mut transitions = Vec::new();
    for resource_id in ["RESOURCE-A", "RESOURCE-B", "RESOURCE-C"] {
        for from in ["SETUP-A", "SETUP-B", "SETUP-C"] {
            for to in ["SETUP-A", "SETUP-B", "SETUP-C"] {
                if from != to {
                    transitions.push(SetupTransition {
                        resource_id: resource_id.to_string(),
                        from_setup_family: from.to_string(),
                        to_setup_family: to.to_string(),
                        duration_ticks: rng.between(1, 3),
                    });
                }
            }
        }
    }
    transitions
}

fn make_inspection_station() -> InspectionStation {
    InspectionStation {
        station_id: "INSPECTION-01".into(),
        capacity_units_by_epoch: vec![1, 1, 1, 1],
        methods: vec![
            InspectionMethod {
                method_id: "INDEPENDENT-DIMENSIONAL".into(),
                applicable_process_families: vec![
                    "CUTTING".into(),
                    "FORMING".into(),
                    "FINISHING".into(),
                ],
                duration_ticks: 1,
                cost_units: 3,
            },
            InspectionMethod {
                method_id: "INDEPENDENT-NONDESTRUCTIVE".into(),
                applicable_process_families: vec!["FINISHING".into()],
                duration_ticks: 2,
                cost_units: 5,
            },
        ],
    }
}

fn empty_observations() -> Vec<EpochObservation> {
    (0..HORIZON_EPOCHS)
        .map(|epoch| EpochObservation {
            epoch,
            resource_events: Vec::new(),
            qualification_updates: Vec::new(),
            inspection_capacity_events: Vec::new(),
        })
        .collect()
}

#[derive(Debug, Clone)]
struct ResourceEventDraft {
    source_event_id: String,
    resource_id: String,
    kind: ResourceEventKind,
    occurred_at_epoch: usize,
    observed_at_epoch: usize,
    common_cause_id: Option<String>,
    forecast: Option<(u64, u64)>,
}

fn add_resource_event(observations: &mut [EpochObservation], event: ResourceEventDraft) {
    observations[event.observed_at_epoch]
        .resource_events
        .push(ResourceEvent {
            source_event_id: event.source_event_id,
            resource_id: event.resource_id,
            kind: event.kind,
            occurred_at_epoch: event.occurred_at_epoch,
            observed_at_epoch: event.observed_at_epoch,
            common_cause_id: event.common_cause_id,
            recovery_forecast_min_epochs: event.forecast.map(|value| value.0),
            recovery_forecast_max_epochs: event.forecast.map(|value| value.1),
        });
}

fn add_qualification_update(observations: &mut [EpochObservation], case_prefix: &str) {
    observations[2]
        .qualification_updates
        .push(QualificationUpdate {
            evidence_receipt_id: format!("{case_prefix}-QUAL-RC-FORMING-ADMITTED"),
            resource_id: "RESOURCE-C".into(),
            process_family: "FORMING".into(),
            previous_disposition: QualificationDisposition::Unknown,
            new_disposition: QualificationDisposition::Admissible,
            occurred_at_epoch: 1,
            observed_at_epoch: 2,
        });
}

fn add_inspection_capacity_event(observations: &mut [EpochObservation], case_prefix: &str) {
    observations[2]
        .inspection_capacity_events
        .push(InspectionCapacityEvent {
            source_event_id: format!("{case_prefix}-INSPECTION-CAPACITY-LOSS"),
            station_id: "INSPECTION-01".into(),
            available_capacity_units: 0,
            occurred_at_epoch: 2,
            observed_at_epoch: 2,
        });
    observations[3]
        .inspection_capacity_events
        .push(InspectionCapacityEvent {
            source_event_id: format!("{case_prefix}-INSPECTION-CAPACITY-RESTORED"),
            station_id: "INSPECTION-01".into(),
            available_capacity_units: 1,
            occurred_at_epoch: 3,
            observed_at_epoch: 3,
        });
}

fn make_scenarios(
    family_id: &str,
    case_prefix: &str,
    rng: &mut DeterministicRng,
) -> ScenarioBundle {
    let nominal_history = empty_observations();

    let mut disruption_history = empty_observations();
    if family_id == "common-cause-outage" {
        for resource_id in ["RESOURCE-A", "RESOURCE-B"] {
            add_resource_event(
                &mut disruption_history,
                ResourceEventDraft {
                    source_event_id: format!("{case_prefix}-CC-{resource_id}-FAIL"),
                    resource_id: resource_id.to_string(),
                    kind: ResourceEventKind::Failed,
                    occurred_at_epoch: 1,
                    observed_at_epoch: 1,
                    common_cause_id: Some("COMMON-POWER-FEEDER-01".into()),
                    forecast: Some((1, 3)),
                },
            );
            add_resource_event(
                &mut disruption_history,
                ResourceEventDraft {
                    source_event_id: format!("{case_prefix}-CC-{resource_id}-RESTORE"),
                    resource_id: resource_id.to_string(),
                    kind: ResourceEventKind::Restored,
                    occurred_at_epoch: 3,
                    observed_at_epoch: 3,
                    common_cause_id: Some("COMMON-POWER-FEEDER-01".into()),
                    forecast: None,
                },
            );
        }
    } else {
        let recovery_epoch = if family_id == "uncertain-recovery" {
            rng.between(2, 3) as usize
        } else {
            3
        };
        add_resource_event(
            &mut disruption_history,
            ResourceEventDraft {
                source_event_id: format!("{case_prefix}-RA-FAIL-LATE-ARRIVAL"),
                resource_id: "RESOURCE-A".to_string(),
                kind: ResourceEventKind::Failed,
                occurred_at_epoch: 1,
                observed_at_epoch: 2,
                common_cause_id: None,
                forecast: Some((1, 3)),
            },
        );
        add_resource_event(
            &mut disruption_history,
            ResourceEventDraft {
                source_event_id: format!("{case_prefix}-RA-RESTORE"),
                resource_id: "RESOURCE-A".to_string(),
                kind: ResourceEventKind::Restored,
                occurred_at_epoch: recovery_epoch,
                observed_at_epoch: recovery_epoch,
                common_cause_id: None,
                forecast: None,
            },
        );
    }

    let mut alternate_history = empty_observations();
    add_resource_event(
        &mut alternate_history,
        ResourceEventDraft {
            source_event_id: format!("{case_prefix}-RC-FAIL"),
            resource_id: "RESOURCE-C".to_string(),
            kind: ResourceEventKind::Failed,
            occurred_at_epoch: 2,
            observed_at_epoch: 2,
            common_cause_id: None,
            forecast: Some((1, 2)),
        },
    );
    add_resource_event(
        &mut alternate_history,
        ResourceEventDraft {
            source_event_id: format!("{case_prefix}-RC-RESTORE"),
            resource_id: "RESOURCE-C".to_string(),
            kind: ResourceEventKind::Restored,
            occurred_at_epoch: 3,
            observed_at_epoch: 3,
            common_cause_id: None,
            forecast: None,
        },
    );

    if family_id == "qualification-transition" {
        add_qualification_update(&mut alternate_history, case_prefix);
    }
    if family_id == "inspection-value" {
        add_inspection_capacity_event(&mut alternate_history, case_prefix);
    }
    if family_id == "setup-wip-pressure" {
        add_resource_event(
            &mut alternate_history,
            ResourceEventDraft {
                source_event_id: format!("{case_prefix}-RB-SETUP-WIP-FAIL"),
                resource_id: "RESOURCE-B".to_string(),
                kind: ResourceEventKind::Failed,
                occurred_at_epoch: 1,
                observed_at_epoch: 1,
                common_cause_id: None,
                forecast: Some((1, 2)),
            },
        );
        add_resource_event(
            &mut alternate_history,
            ResourceEventDraft {
                source_event_id: format!("{case_prefix}-RB-RESTORE"),
                resource_id: "RESOURCE-B".to_string(),
                kind: ResourceEventKind::Restored,
                occurred_at_epoch: 2,
                observed_at_epoch: 2,
                common_cause_id: None,
                forecast: None,
            },
        );
    }

    let quality_operation = "JOB-1-OP-2";
    let policy_scenarios = vec![
        Scenario {
            scenario_id: "NOMINAL-01".into(),
            probability_ppm: 300_000,
            observations_by_epoch: nominal_history.clone(),
        },
        Scenario {
            scenario_id: "NOMINAL-02".into(),
            probability_ppm: 200_000,
            observations_by_epoch: nominal_history,
        },
        Scenario {
            scenario_id: "PRIMARY-DISRUPTION".into(),
            probability_ppm: 300_000,
            observations_by_epoch: disruption_history,
        },
        Scenario {
            scenario_id: "ALTERNATE-DISRUPTION".into(),
            probability_ppm: 200_000,
            observations_by_epoch: alternate_history,
        },
    ];
    let evaluator_outcomes = vec![
        EvaluatorScenarioOutcome {
            scenario_id: "NOMINAL-01".into(),
            hidden_terminal_quality_outcomes: vec![HiddenQualityOutcome {
                outcome_id: format!("{case_prefix}-Q-NOMINAL"),
                operation_id: quality_operation.into(),
                disposition: QualityDisposition::Accepted,
            }],
        },
        EvaluatorScenarioOutcome {
            scenario_id: "NOMINAL-02".into(),
            hidden_terminal_quality_outcomes: vec![HiddenQualityOutcome {
                outcome_id: format!("{case_prefix}-Q-HIDDEN-REJECT"),
                operation_id: quality_operation.into(),
                disposition: QualityDisposition::Rejected,
            }],
        },
        EvaluatorScenarioOutcome {
            scenario_id: "PRIMARY-DISRUPTION".into(),
            hidden_terminal_quality_outcomes: vec![HiddenQualityOutcome {
                outcome_id: format!("{case_prefix}-Q-DISRUPTION"),
                operation_id: "JOB-2-OP-2".into(),
                disposition: if rng.between(0, 1) == 0 {
                    QualityDisposition::Accepted
                } else {
                    QualityDisposition::Indeterminate
                },
            }],
        },
        EvaluatorScenarioOutcome {
            scenario_id: "ALTERNATE-DISRUPTION".into(),
            hidden_terminal_quality_outcomes: vec![HiddenQualityOutcome {
                outcome_id: format!("{case_prefix}-Q-ALTERNATE"),
                operation_id: "JOB-3-OP-2".into(),
                disposition: QualityDisposition::Accepted,
            }],
        },
    ];

    ScenarioBundle {
        policy_scenarios,
        evaluator_outcomes,
    }
}

fn generate_case(
    family: &FamilySpec,
    family_index: usize,
    replication_seed: u64,
) -> (ChallengeCase, EvaluatorCase) {
    let generator_seed = derive_seed(family_index, replication_seed);
    let mut rng = DeterministicRng::new(generator_seed);
    let case_prefix = format!(
        "{}-{:02}",
        family.scenario_family_id.to_ascii_uppercase(),
        replication_seed
    );

    let jobs = make_jobs(&family.scenario_family_id, &mut rng);
    let resources = make_resources(&family.scenario_family_id);
    let setup_transitions = make_setup_transitions(&mut rng);
    let inspection_station = make_inspection_station();
    let scenario_bundle = make_scenarios(&family.scenario_family_id, &case_prefix, &mut rng);

    let case = ChallengeCase {
        case_id: case_prefix.clone(),
        scenario_family_id: family.scenario_family_id.clone(),
        split: family.split,
        replication_seed,
        generator_seed,
        paired_seed_id: format!("PAIR-SEED-{replication_seed:02}"),
        horizon_epochs: HORIZON_EPOCHS,
        decision_visibility_rule:
            "At epoch t, a policy receives only observations_by_epoch[0..=t]. Evaluator-only terminal outcomes are in a separately stored sidecar that must never be mounted in policy execution."
                .into(),
        jobs,
        resources,
        setup_transitions,
        inspection_station,
        scenarios: scenario_bundle.policy_scenarios,
    };
    let evaluator_case = EvaluatorCase {
        case_id: case_prefix,
        scenario_family_id: family.scenario_family_id.clone(),
        scenario_outcomes: scenario_bundle.evaluator_outcomes,
    };
    (case, evaluator_case)
}

fn generate_corpus() -> (Corpus, EvaluatorLabels) {
    let family_manifest = family_manifest();
    let mut cases = Vec::with_capacity(FAMILY_COUNT * REPLICATIONS_PER_FAMILY);
    let mut evaluator_cases = Vec::with_capacity(FAMILY_COUNT * REPLICATIONS_PER_FAMILY);
    for (family_index, family) in family_manifest.iter().enumerate() {
        for seed in &family.replication_seeds {
            let (case, evaluator_case) = generate_case(family, family_index, *seed);
            cases.push(case);
            evaluator_cases.push(evaluator_case);
        }
    }

    let policy_corpus = Corpus {
        schema_id: "https://luminousdynamics.io/schemas/mfg-policy-v0.2-challenge-corpus".into(),
        benchmark_id: "MFG-POLICY-V02-CHALLENGE-001".into(),
        generator_version: "mfg-policy-v02-challenge-generator/0.2.0".into(),
        evidence_class: "synthetic_offline_policy_visible".into(),
        generation: GenerationManifest {
            root_seed: ROOT_SEED,
            seed_algorithm: "SplitMix64-v1".into(),
            seed_derivation: "mix64(root_seed XOR ((family_index + 1) << 32) XOR replication_seed)".into(),
            pairing_rule: "Each policy receives the same policy-visible case bytes, scenario leaves and event prefixes; paired seeds are shared across policies.".into(),
            split_rule: "Whole scenario families are assigned to development or held_out before generation; no case-level random split.".into(),
            tuning_rule: "Tune only on development families; held_out families are evaluation-only and must not drive policy selection or hyperparameter changes.".into(),
        },
        horizon_epochs: HORIZON_EPOCHS,
        family_count: FAMILY_COUNT,
        case_count: cases.len(),
        family_manifest,
        cases,
        claim_ceiling: "Policy-visible deterministic synthetic challenge corpus only; evaluator outcomes are distributed separately and must be access-controlled; no scheduler superiority, real provider reliability, physical acceptance, product conformity, factory recovery, production authorization, or ISO conformance claim.".into(),
    };
    let evaluator_labels = EvaluatorLabels {
        schema_id: "https://luminousdynamics.io/schemas/mfg-policy-v0.2-evaluator-labels".into(),
        benchmark_id: "MFG-POLICY-V02-CHALLENGE-001".into(),
        evidence_class: "synthetic_evaluator_only".into(),
        case_count: evaluator_cases.len(),
        cases: evaluator_cases,
        claim_ceiling: "Synthetic terminal outcome labels for independent evaluation only; never mount or expose to policy processes.".into(),
    };
    (policy_corpus, evaluator_labels)
}

fn validate_with_schema(instance: &serde_json::Value, schema_source: &str) -> Result<(), String> {
    let schema: serde_json::Value =
        serde_json::from_str(schema_source).map_err(|error| format!("schema_parse:{error}"))?;
    let validator = jsonschema::draft202012::options()
        .offline()
        .build(&schema)
        .map_err(|error| format!("schema_compile:{error}"))?;
    validator
        .validate(instance)
        .map_err(|error| format!("schema_validation:{error}"))
}

fn validate_schema(instance: &serde_json::Value) -> Result<(), String> {
    validate_with_schema(instance, SCHEMA_JSON)
}

fn validate_evaluator_schema(instance: &serde_json::Value) -> Result<(), String> {
    validate_with_schema(instance, EVALUATOR_SCHEMA_JSON)
}

fn validate_corpus(corpus: &Corpus) -> Result<(), String> {
    if corpus.schema_id != "https://luminousdynamics.io/schemas/mfg-policy-v0.2-challenge-corpus"
        || corpus.benchmark_id != "MFG-POLICY-V02-CHALLENGE-001"
        || corpus.generator_version != "mfg-policy-v02-challenge-generator/0.2.0"
        || corpus.evidence_class != "synthetic_offline_policy_visible"
        || corpus.horizon_epochs != HORIZON_EPOCHS
        || corpus.family_count != FAMILY_COUNT
        || corpus.family_manifest.len() != FAMILY_COUNT
        || corpus.case_count != FAMILY_COUNT * REPLICATIONS_PER_FAMILY
        || corpus.cases.len() != corpus.case_count
        || corpus.generation.root_seed != ROOT_SEED
        || corpus.claim_ceiling.trim().is_empty()
    {
        return Err("corpus_metadata_or_count_invalid".into());
    }

    let mut family_by_id = BTreeMap::new();
    let mut family_index_by_id = BTreeMap::new();
    let mut development_families = 0;
    let mut held_out_families = 0;
    for family in &corpus.family_manifest {
        let family_index = family_by_id.len();
        if family.replication_seeds != (1..=REPLICATIONS_PER_FAMILY as u64).collect::<Vec<_>>()
            || family.primary_mechanism.trim().is_empty()
            || family.decision_tension_under_test.trim().is_empty()
            || family_by_id
                .insert(family.scenario_family_id.as_str(), family)
                .is_some()
        {
            return Err("family_manifest_invalid_or_duplicate".into());
        }
        family_index_by_id.insert(family.scenario_family_id.as_str(), family_index);
        match family.split {
            DataSplit::Development => development_families += 1,
            DataSplit::HeldOut => held_out_families += 1,
        }
    }
    if development_families != 4 || held_out_families != 2 {
        return Err("whole_family_split_must_be_four_development_and_two_held_out".into());
    }

    let mut case_ids = BTreeSet::new();
    for case in &corpus.cases {
        let family = family_by_id
            .get(case.scenario_family_id.as_str())
            .ok_or_else(|| "case_references_unknown_family".to_string())?;
        let family_index = *family_index_by_id
            .get(case.scenario_family_id.as_str())
            .ok_or_else(|| "case_family_index_missing".to_string())?;
        if case.case_id.trim().is_empty()
            || case.generator_seed != derive_seed(family_index, case.replication_seed)
            || case.case_id
                != format!(
                    "{}-{:02}",
                    case.scenario_family_id.to_ascii_uppercase(),
                    case.replication_seed
                )
            || !case_ids.insert(case.case_id.as_str())
            || case.split != family.split
            || !family.replication_seeds.contains(&case.replication_seed)
            || case.paired_seed_id != format!("PAIR-SEED-{:02}", case.replication_seed)
            || case.horizon_epochs != HORIZON_EPOCHS
            || case.jobs.len() != 3
            || case.resources.len() != 3
            || case.scenarios.len() != SCENARIOS_PER_CASE
            || case.inspection_station.capacity_units_by_epoch.len() != HORIZON_EPOCHS
        {
            return Err("case_identity_split_or_shape_invalid".into());
        }

        let resource_ids: BTreeSet<&str> = case
            .resources
            .iter()
            .map(|resource| resource.resource_id.as_str())
            .collect();
        if resource_ids.len() != 3 {
            return Err("resource_ids_not_unique".into());
        }

        let resources_by_id: BTreeMap<&str, &Resource> = case
            .resources
            .iter()
            .map(|resource| (resource.resource_id.as_str(), resource))
            .collect();
        let mut operation_ids = BTreeSet::new();
        for job in &case.jobs {
            if job.operations.len() != 2 {
                return Err("each_job_must_have_two_operations".into());
            }
            for (index, operation) in job.operations.iter().enumerate() {
                if !operation_ids.insert(operation.operation_id.as_str())
                    || operation.sequence != index + 1
                    || operation.setup_family
                        != (match operation.process_family.as_str() {
                            "CUTTING" => "SETUP-A",
                            "FORMING" => "SETUP-B",
                            "FINISHING" => "SETUP-C",
                            _ => "",
                        })
                    || operation.eligible_resource_ids.len() != 2
                    || operation
                        .eligible_resource_ids
                        .iter()
                        .any(|id| !resource_ids.contains(id.as_str()))
                    || operation.processing_time_ticks_by_resource.len() != 2
                    || operation
                        .processing_time_ticks_by_resource
                        .keys()
                        .map(String::as_str)
                        .collect::<BTreeSet<_>>()
                        != operation
                            .eligible_resource_ids
                            .iter()
                            .map(String::as_str)
                            .collect::<BTreeSet<_>>()
                    || operation.eligible_resource_ids.iter().any(|resource_id| {
                        !resources_by_id
                            .get(resource_id.as_str())
                            .is_some_and(|resource| {
                                resource
                                    .capability_families
                                    .contains(&operation.process_family)
                            })
                    })
                    || operation
                        .processing_time_ticks_by_resource
                        .values()
                        .any(|value| *value == 0)
                    || (index == 0 && !operation.predecessor_operation_ids.is_empty())
                    || (index == 1
                        && operation.predecessor_operation_ids
                            != vec![job.operations[0].operation_id.clone()])
                {
                    return Err("operation_identity_precedence_or_eligibility_invalid".into());
                }
                match operation.initial_status {
                    OperationStatus::InProcess => {
                        if operation.committed_resource_id.as_deref().is_none_or(|id| {
                            !operation
                                .eligible_resource_ids
                                .iter()
                                .any(|eligible| eligible == id)
                        }) || operation.remaining_processing_ticks == 0
                        {
                            return Err(
                                "in_process_operation_requires_committed_eligible_resource".into(),
                            );
                        }
                    }
                    OperationStatus::Waiting | OperationStatus::Completed => {
                        if operation.committed_resource_id.is_some()
                            || operation.remaining_processing_ticks != 0
                        {
                            return Err(
                                "non_running_operation_has_commitment_or_remaining_work".into()
                            );
                        }
                    }
                }
                if operation.inspection_required != operation.inspection_method_id.is_some() {
                    return Err("inspection_requirement_and_method_mismatch".into());
                }
                if let Some(method_id) = &operation.inspection_method_id
                    && !case.inspection_station.methods.iter().any(|method| {
                        method.method_id == *method_id
                            && method
                                .applicable_process_families
                                .contains(&operation.process_family)
                    })
                {
                    return Err("operation_inspection_method_not_qualified_for_process".into());
                }
            }
        }

        if case
            .resources
            .iter()
            .any(|resource| resource.capacity_units_by_epoch.len() != HORIZON_EPOCHS)
        {
            return Err("resource_capacity_calendar_length_invalid".into());
        }
        if case.scenario_family_id == "crossed-bottlenecks"
            && !(case.resources.iter().any(|resource| {
                resource.resource_id == "RESOURCE-A"
                    && resource.capacity_units_by_epoch == [1, 0, 1, 1]
            }) && case.resources.iter().any(|resource| {
                resource.resource_id == "RESOURCE-B"
                    && resource.capacity_units_by_epoch == [1, 1, 0, 1]
            }))
        {
            return Err("interacting_bottleneck_capacity_windows_missing".into());
        }
        let setup_keys: BTreeSet<(&str, &str, &str)> = case
            .setup_transitions
            .iter()
            .map(|transition| {
                (
                    transition.resource_id.as_str(),
                    transition.from_setup_family.as_str(),
                    transition.to_setup_family.as_str(),
                )
            })
            .collect();
        if case.setup_transitions.len() != 18
            || setup_keys.len() != 18
            || case.setup_transitions.iter().any(|transition| {
                transition.duration_ticks == 0
                    || !resource_ids.contains(transition.resource_id.as_str())
                    || transition.from_setup_family == transition.to_setup_family
            })
        {
            return Err("sequence_dependent_setup_matrix_missing_invalid_or_zero".into());
        }

        let mut probability_total = 0_u64;
        let mut scenario_ids = BTreeSet::new();
        for scenario in &case.scenarios {
            if !scenario_ids.insert(scenario.scenario_id.as_str())
                || scenario.observations_by_epoch.len() != HORIZON_EPOCHS
                || scenario
                    .observations_by_epoch
                    .iter()
                    .enumerate()
                    .any(|(epoch, observation)| observation.epoch != epoch)
            {
                return Err("scenario_identity_or_observation_horizon_invalid".into());
            }
            probability_total = probability_total
                .checked_add(scenario.probability_ppm)
                .ok_or_else(|| "scenario_probability_overflow".to_string())?;
            for observation in &scenario.observations_by_epoch {
                for event in &observation.resource_events {
                    if event.observed_at_epoch != observation.epoch
                        || event.occurred_at_epoch > event.observed_at_epoch
                        || !resource_ids.contains(event.resource_id.as_str())
                        || (event.kind == ResourceEventKind::Failed
                            && (event.recovery_forecast_min_epochs.is_none()
                                || event.recovery_forecast_max_epochs.is_none()
                                || event.recovery_forecast_min_epochs
                                    > event.recovery_forecast_max_epochs))
                        || (event.kind == ResourceEventKind::Restored
                            && (event.recovery_forecast_min_epochs.is_some()
                                || event.recovery_forecast_max_epochs.is_some()))
                    {
                        return Err("resource_event_chronology_scope_or_forecast_invalid".into());
                    }
                }
                for update in &observation.qualification_updates {
                    if update.observed_at_epoch != observation.epoch
                        || update.occurred_at_epoch > update.observed_at_epoch
                        || !resource_ids.contains(update.resource_id.as_str())
                        || update.previous_disposition == update.new_disposition
                    {
                        return Err("qualification_update_chronology_or_scope_invalid".into());
                    }
                }
                for event in &observation.inspection_capacity_events {
                    if event.observed_at_epoch != observation.epoch
                        || event.occurred_at_epoch > event.observed_at_epoch
                        || event.station_id != case.inspection_station.station_id
                    {
                        return Err("inspection_capacity_event_invalid".into());
                    }
                }
            }
        }
        if probability_total != PROBABILITY_TOTAL_PPM {
            return Err("scenario_probabilities_must_sum_to_one_million".into());
        }

        if case.scenarios[0].observations_by_epoch != case.scenarios[1].observations_by_epoch {
            return Err(
                "hidden_quality_counterfactuals_must_share_identical_visible_history".into(),
            );
        }
    }

    for family in &corpus.family_manifest {
        let count = corpus
            .cases
            .iter()
            .filter(|case| case.scenario_family_id == family.scenario_family_id)
            .count();
        if count != REPLICATIONS_PER_FAMILY {
            return Err("family_replication_coverage_invalid".into());
        }
    }
    Ok(())
}

fn validate_evaluator_labels(corpus: &Corpus, labels: &EvaluatorLabels) -> Result<(), String> {
    if labels.schema_id != "https://luminousdynamics.io/schemas/mfg-policy-v0.2-evaluator-labels"
        || labels.benchmark_id != corpus.benchmark_id
        || labels.evidence_class != "synthetic_evaluator_only"
        || labels.case_count != corpus.case_count
        || labels.cases.len() != corpus.case_count
        || labels.claim_ceiling.trim().is_empty()
    {
        return Err("evaluator_label_manifest_or_count_invalid".into());
    }

    let mut evaluator_cases = BTreeMap::new();
    for evaluator_case in &labels.cases {
        if evaluator_cases
            .insert(evaluator_case.case_id.as_str(), evaluator_case)
            .is_some()
        {
            return Err("duplicate_evaluator_case_id".into());
        }
    }

    for case in &corpus.cases {
        let evaluator_case = evaluator_cases
            .get(case.case_id.as_str())
            .ok_or_else(|| "evaluator_labels_missing_case".to_string())?;
        if evaluator_case.scenario_family_id != case.scenario_family_id
            || evaluator_case.scenario_outcomes.len() != case.scenarios.len()
        {
            return Err("evaluator_case_scope_or_scenario_count_mismatch".into());
        }

        let policy_scenarios: BTreeMap<&str, &Scenario> = case
            .scenarios
            .iter()
            .map(|scenario| (scenario.scenario_id.as_str(), scenario))
            .collect();
        let operation_ids: BTreeSet<&str> = case
            .jobs
            .iter()
            .flat_map(|job| &job.operations)
            .map(|operation| operation.operation_id.as_str())
            .collect();
        let mut outcome_scenario_ids = BTreeSet::new();

        for scenario_outcome in &evaluator_case.scenario_outcomes {
            if !outcome_scenario_ids.insert(scenario_outcome.scenario_id.as_str())
                || !policy_scenarios.contains_key(scenario_outcome.scenario_id.as_str())
                || scenario_outcome.hidden_terminal_quality_outcomes.is_empty()
            {
                return Err("evaluator_scenario_identity_or_scope_invalid".into());
            }
            let mut outcome_ids = BTreeSet::new();
            for outcome in &scenario_outcome.hidden_terminal_quality_outcomes {
                if outcome.outcome_id.trim().is_empty()
                    || !outcome_ids.insert(outcome.outcome_id.as_str())
                    || !operation_ids.contains(outcome.operation_id.as_str())
                {
                    return Err("evaluator_outcome_identity_or_operation_invalid".into());
                }
            }
        }
        if outcome_scenario_ids != policy_scenarios.keys().copied().collect() {
            return Err("evaluator_scenario_coverage_mismatch".into());
        }

        let accepted = evaluator_case
            .scenario_outcomes
            .iter()
            .find(|outcome| outcome.scenario_id == "NOMINAL-01")
            .ok_or_else(|| "nominal_accepted_evaluator_label_missing".to_string())?;
        let rejected = evaluator_case
            .scenario_outcomes
            .iter()
            .find(|outcome| outcome.scenario_id == "NOMINAL-02")
            .ok_or_else(|| "nominal_reject_evaluator_label_missing".to_string())?;
        if accepted.hidden_terminal_quality_outcomes == rejected.hidden_terminal_quality_outcomes
            || accepted.hidden_terminal_quality_outcomes.len() != 1
            || accepted.hidden_terminal_quality_outcomes[0].disposition
                != QualityDisposition::Accepted
            || rejected.hidden_terminal_quality_outcomes.len() != 1
            || rejected.hidden_terminal_quality_outcomes[0].disposition
                != QualityDisposition::Rejected
        {
            return Err("paired_hidden_quality_labels_do_not_create_counterfactual".into());
        }
    }

    if evaluator_cases.len() != corpus.case_count {
        return Err("evaluator_case_coverage_mismatch".into());
    }
    Ok(())
}

fn build_and_validate() -> Result<(Corpus, EvaluatorLabels), String> {
    let (corpus, labels) = generate_corpus();

    let corpus_value =
        serde_json::to_value(&corpus).map_err(|error| format!("corpus_to_json:{error}"))?;
    validate_schema(&corpus_value)?;
    validate_corpus(&corpus)?;

    let labels_value = serde_json::to_value(&labels)
        .map_err(|error| format!("evaluator_labels_to_json:{error}"))?;
    validate_evaluator_schema(&labels_value)?;
    validate_evaluator_labels(&corpus, &labels)?;

    Ok((corpus, labels))
}

fn main() -> ExitCode {
    let output_kind = std::env::args().nth(1);
    let result = build_and_validate().and_then(|(corpus, labels)| {
        let json = match output_kind.as_deref() {
            None | Some("--policy-corpus") => serde_json::to_string_pretty(&corpus)
                .map_err(|error| format!("policy_corpus_serialize:{error}"))?,
            Some("--evaluator-labels") => serde_json::to_string_pretty(&labels)
                .map_err(|error| format!("evaluator_labels_serialize:{error}"))?,
            Some(other) => return Err(format!("unsupported_output_kind:{other}")),
        };
        Ok(json)
    });
    match result {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle() -> (Corpus, EvaluatorLabels) {
        build_and_validate().expect("policy corpus and isolated evaluator labels must validate")
    }

    fn corpus() -> Corpus {
        bundle().0
    }

    #[test]
    fn six_families_twenty_seeds_with_whole_family_holdout() {
        let corpus = corpus();
        assert_eq!(corpus.case_count, 120);
        assert_eq!(corpus.family_count, 6);
        assert_eq!(
            corpus
                .family_manifest
                .iter()
                .filter(|family| family.split == DataSplit::Development)
                .count(),
            4
        );
        assert_eq!(
            corpus
                .family_manifest
                .iter()
                .filter(|family| family.split == DataSplit::HeldOut)
                .count(),
            2
        );
        assert!(
            corpus
                .family_manifest
                .iter()
                .all(|family| family.replication_seeds.len() == 20)
        );
        assert_eq!(
            corpus
                .cases
                .iter()
                .filter(|case| case.split == DataSplit::Development)
                .count(),
            80
        );
        assert_eq!(
            corpus
                .cases
                .iter()
                .filter(|case| case.split == DataSplit::HeldOut)
                .count(),
            40
        );
    }

    #[test]
    fn each_instance_has_three_jobs_three_shared_resources_four_epochs_and_overlapping_routes() {
        let corpus = corpus();
        for case in &corpus.cases {
            assert_eq!(case.jobs.len(), 3);
            assert_eq!(case.resources.len(), 3);
            assert_eq!(case.horizon_epochs, 4);
            assert_eq!(
                case.jobs
                    .iter()
                    .map(|job| job.operations.len())
                    .sum::<usize>(),
                6
            );
            assert!(
                case.jobs
                    .iter()
                    .flat_map(|job| &job.operations)
                    .all(|operation| operation.eligible_resource_ids.len() == 2)
            );
        }
    }

    #[test]
    fn crossed_bottleneck_family_has_interacting_time_window_limits() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.scenario_family_id == "crossed-bottlenecks")
            .expect("crossed bottlenecks family exists");
        let a = case
            .resources
            .iter()
            .find(|resource| resource.resource_id == "RESOURCE-A")
            .unwrap();
        let b = case
            .resources
            .iter()
            .find(|resource| resource.resource_id == "RESOURCE-B")
            .unwrap();
        assert_eq!(a.capacity_units_by_epoch, vec![1, 0, 1, 1]);
        assert_eq!(b.capacity_units_by_epoch, vec![1, 1, 0, 1]);
    }

    #[test]
    fn operations_bind_to_sequence_dependent_setup_families() {
        let corpus = corpus();
        for case in &corpus.cases {
            for operation in case.jobs.iter().flat_map(|job| &job.operations) {
                assert!(
                    case.setup_transitions
                        .iter()
                        .any(
                            |transition| transition.to_setup_family == operation.setup_family
                                && transition.resource_id == operation.eligible_resource_ids[0]
                        )
                );
            }
        }
    }

    #[test]
    fn evaluator_counterfactuals_are_separate_from_policy_visible_event_histories() {
        let (corpus, labels) = bundle();
        let policy_json = serde_json::to_string(&corpus).unwrap();
        assert!(!policy_json.contains("hidden_terminal_quality_outcomes"));
        assert!(!policy_json.contains("Q-HIDDEN-REJECT"));

        for case in &corpus.cases {
            assert_eq!(
                case.scenarios[0].observations_by_epoch,
                case.scenarios[1].observations_by_epoch
            );
            assert_eq!(case.scenarios[0].scenario_id, "NOMINAL-01");
            assert_eq!(case.scenarios[1].scenario_id, "NOMINAL-02");
            assert!(
                case.scenarios.iter().all(|scenario| {
                    !scenario.scenario_id.contains("ACCEPTED")
                        && !scenario.scenario_id.contains("REJECT")
                })
            );
            let evaluator_case = labels
                .cases
                .iter()
                .find(|labels| labels.case_id == case.case_id)
                .expect("matching evaluator sidecar case exists");
            let accepted = evaluator_case
                .scenario_outcomes
                .iter()
                .find(|outcome| outcome.scenario_id == "NOMINAL-01")
                .expect("nominal acceptance label");
            let rejected = evaluator_case
                .scenario_outcomes
                .iter()
                .find(|outcome| outcome.scenario_id == "NOMINAL-02")
                .expect("nominal rejection label");
            assert_ne!(
                accepted.hidden_terminal_quality_outcomes,
                rejected.hidden_terminal_quality_outcomes
            );
            assert!(
                rejected
                    .hidden_terminal_quality_outcomes
                    .iter()
                    .any(|outcome| outcome.disposition == QualityDisposition::Rejected)
            );
        }
    }

    #[test]
    fn evaluator_sidecar_rejects_unknown_operation_labels() {
        let (corpus, mut labels) = bundle();
        labels.cases[0].scenario_outcomes[0].hidden_terminal_quality_outcomes[0].operation_id =
            "UNKNOWN-OPERATION".into();
        assert_eq!(
            validate_evaluator_labels(&corpus, &labels),
            Err("evaluator_outcome_identity_or_operation_invalid".into())
        );
    }

    #[test]
    fn late_observations_never_rewrite_the_event_occurrence_time() {
        let corpus = corpus();
        let cases = corpus
            .cases
            .iter()
            .filter(|case| case.scenario_family_id == "uncertain-recovery");
        let mut found_late_event = false;
        for case in cases {
            for scenario in &case.scenarios {
                for observation in &scenario.observations_by_epoch {
                    for event in &observation.resource_events {
                        assert!(event.occurred_at_epoch <= event.observed_at_epoch);
                        if event.occurred_at_epoch < event.observed_at_epoch {
                            found_late_event = true;
                        }
                    }
                }
            }
        }
        assert!(found_late_event);
    }

    #[test]
    fn correlated_outages_share_one_explicit_common_cause_id() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.scenario_family_id == "common-cause-outage")
            .expect("common-cause family exists");
        let failed: Vec<&ResourceEvent> = case.scenarios[2]
            .observations_by_epoch
            .iter()
            .flat_map(|observation| &observation.resource_events)
            .filter(|event| event.kind == ResourceEventKind::Failed)
            .collect();
        assert!(failed.iter().any(|event| event.resource_id == "RESOURCE-A"));
        assert!(failed.iter().any(|event| event.resource_id == "RESOURCE-B"));
        assert!(
            failed
                .iter()
                .all(|event| event.common_cause_id.as_deref() == Some("COMMON-POWER-FEEDER-01"))
        );
    }

    #[test]
    fn unknown_qualification_receipt_changes_a_real_ready_route_from_unknown_to_admissible() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.scenario_family_id == "qualification-transition")
            .expect("qualification-transition family exists");
        let ready_operation = &case.jobs[0].operations[1];
        assert_eq!(ready_operation.initial_status, OperationStatus::Waiting);
        assert_eq!(
            case.jobs[0].operations[0].initial_status,
            OperationStatus::Completed
        );
        assert_eq!(
            ready_operation.eligible_resource_ids,
            vec!["RESOURCE-A".to_string(), "RESOURCE-C".to_string()]
        );

        let resource_a = case
            .resources
            .iter()
            .find(|resource| resource.resource_id == "RESOURCE-A")
            .unwrap();
        let resource_c = case
            .resources
            .iter()
            .find(|resource| resource.resource_id == "RESOURCE-C")
            .unwrap();
        let qualification_a = resource_a
            .initial_qualifications
            .iter()
            .find(|qualification| qualification.process_family == "FORMING")
            .unwrap();
        let qualification_c = resource_c
            .initial_qualifications
            .iter()
            .find(|qualification| qualification.process_family == "FORMING")
            .unwrap();
        assert_eq!(
            qualification_a.disposition,
            QualificationDisposition::Blocked
        );
        assert_eq!(
            qualification_c.disposition,
            QualificationDisposition::Unknown
        );

        let update = case.scenarios[3].observations_by_epoch[2]
            .qualification_updates
            .iter()
            .find(|update| update.resource_id == "RESOURCE-C" && update.process_family == "FORMING")
            .expect("scoped qualification update appears in visible event stream");
        assert_eq!(
            update.previous_disposition,
            QualificationDisposition::Unknown
        );
        assert_eq!(update.new_disposition, QualificationDisposition::Admissible);
        assert_eq!(update.occurred_at_epoch, 1);
        assert_eq!(update.observed_at_epoch, 2);
        assert!(
            case.scenarios[3].observations_by_epoch[1]
                .qualification_updates
                .is_empty()
        );
    }

    #[test]
    fn setup_sequences_and_in_process_commitments_are_explicit() {
        let corpus = corpus();
        for case in &corpus.cases {
            assert_eq!(case.setup_transitions.len(), 18);
            assert!(
                case.setup_transitions
                    .iter()
                    .any(|transition| transition.from_setup_family != transition.to_setup_family)
            );
            assert!(
                case.jobs
                    .iter()
                    .flat_map(|job| &job.operations)
                    .any(
                        |operation| operation.initial_status == OperationStatus::InProcess
                            && operation.committed_resource_id.as_deref() == Some("RESOURCE-A")
                    )
            );
        }
    }

    #[test]
    fn held_out_families_are_structurally_unseen_regimes() {
        let corpus = corpus();
        let held_out: BTreeSet<&str> = corpus
            .family_manifest
            .iter()
            .filter(|family| family.split == DataSplit::HeldOut)
            .map(|family| family.scenario_family_id.as_str())
            .collect();
        assert_eq!(
            held_out,
            ["setup-wip-pressure", "inspection-value"]
                .into_iter()
                .collect()
        );
        assert!(
            corpus
                .family_manifest
                .iter()
                .filter(|family| family.split == DataSplit::HeldOut)
                .all(|family| family.replication_seeds == (1..=20).collect::<Vec<_>>())
        );
    }

    #[test]
    fn generator_is_deterministic_and_seed_provenance_is_explicit() {
        let first = bundle();
        let second = bundle();
        assert_eq!(
            serde_json::to_vec(&first.0).unwrap(),
            serde_json::to_vec(&second.0).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&first.1).unwrap(),
            serde_json::to_vec(&second.1).unwrap()
        );
        assert_eq!(first.0.generation.root_seed, ROOT_SEED);
        assert_eq!(first.0.generation.seed_algorithm, "SplitMix64-v1");
    }

    #[test]
    fn inspection_capacity_is_a_real_shared_bottleneck_in_the_held_out_regime() {
        let corpus = corpus();
        let case = corpus
            .cases
            .iter()
            .find(|case| case.scenario_family_id == "inspection-value")
            .expect("inspection family exists");
        assert!(
            case.jobs
                .iter()
                .flat_map(|job| &job.operations)
                .any(|operation| operation.inspection_required)
        );
        assert_eq!(
            case.inspection_station.capacity_units_by_epoch.len(),
            HORIZON_EPOCHS
        );
        assert!(
            case.scenarios[3].observations_by_epoch[2]
                .inspection_capacity_events
                .iter()
                .any(|event| event.available_capacity_units == 0)
        );
    }

    #[test]
    fn generator_manifest_declares_hold_and_proactive_reservation_tensions_without_claiming_a_winner()
     {
        let corpus = corpus();
        assert!(
            corpus
                .family_manifest
                .iter()
                .any(|family| family.decision_tension_under_test.contains("defer"))
        );
        assert!(corpus.family_manifest.iter().any(|family| {
            family
                .decision_tension_under_test
                .contains("proactive reservation")
        }));
        assert!(corpus.claim_ceiling.contains("no scheduler superiority"));
    }
}
