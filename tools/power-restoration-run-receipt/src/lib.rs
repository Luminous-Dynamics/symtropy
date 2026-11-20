use power_restoration_action_proposal::{ActionProposal, validate_proposal};
use power_restoration_benchmark_case::{BenchmarkCase, CaseMaturity, canonical_case_sha256, validate_case};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunReceipt {
    pub schema_version: String,
    pub run_id: String,
    pub maturity: CaseMaturity,
    pub pairing: PairingIdentity,
    pub case: CaseIdentity,
    pub planner: PlannerIdentity,
    pub adapter: ComponentIdentity,
    pub simulator: ComponentIdentity,
    pub independent_validator: ComponentIdentity,
    pub objective: ObjectiveIdentity,
    pub hardware: HardwareProfile,
    pub budget: ExecutionBudget,
    pub run_status: RunStatus,
    pub terminal_reason: Option<TerminalReason>,
    pub decision_steps: Vec<DecisionRecord>,
    pub final_service_trajectory: Option<ArtifactRef>,
    pub aggregate_score: Option<AggregateScore>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairingIdentity {
    pub scenario_group_id: String,
    pub scenario_seed: u64,
    pub paired_replicate_id: String,
    pub observation_profile_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaseIdentity {
    pub case_ref: String,
    pub case_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlannerIdentity {
    pub planner_id: String,
    pub planner_revision: String,
    pub source_commit: String,
    pub configuration_sha256: String,
    pub prompt_sha256: Option<String>,
    pub model_provider: String,
    pub model_id: String,
    pub model_revision: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentIdentity {
    pub component_ref: String,
    pub component_id: String,
    pub version: String,
    pub source_commit: String,
    pub configuration_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObjectiveIdentity {
    pub objective_id: String,
    pub metric: String,
    pub unit: String,
    pub expression_ref: String,
    pub scorer: ComponentIdentity,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareProfile {
    pub measurement_status: HardwareMeasurementStatus,
    pub operating_system: Option<String>,
    pub architecture: Option<String>,
    pub cpu_model: Option<String>,
    pub logical_cpu_count: Option<u32>,
    pub memory_bytes: Option<u64>,
    pub accelerators: Vec<Accelerator>,
    pub runtime_version: String,
    pub compiler_or_solver_versions: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HardwareMeasurementStatus {
    Measured,
    NotMeasured,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Accelerator {
    pub device_id: String,
    pub model: String,
    pub memory_bytes: u64,
    pub driver_version: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutionBudget {
    pub wall_clock_limit_ms: u64,
    pub solver_time_limit_ms: Option<u64>,
    pub solver_gap_limit_basis_points: Option<u32>,
    pub token_limit: Option<u64>,
    pub api_cost_limit_microusd: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Completed,
    Aborted,
    TimedOut,
    Incomplete,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TerminalReason {
    RestorationComplete,
    HorizonExhausted,
    OperatorStopped,
    RunAborted,
    DeadlineExceeded,
    SystemFailure,
    EvidenceIncomplete,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecisionRecord {
    pub decision_step: u32,
    pub disposition: StepDisposition,
    pub not_run_reason: Option<NotRunReason>,
    pub observation: Option<ArtifactRef>,
    pub planner_outcome: PlannerOutcome,
    pub planner_latency_ms: Option<u64>,
    pub planner_output: Option<ArtifactRef>,
    pub retry_count: u32,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub api_cost_microusd: Option<u64>,
    pub proposal: Option<ActionProposal>,
    pub transition_outcome: TransitionOutcome,
    pub transition_receipt: Option<ArtifactRef>,
    pub validator_outcome: ValidatorOutcome,
    pub validator_receipt: Option<ArtifactRef>,
    pub service_outcome: ServiceOutcome,
    pub service_trajectory: Option<ArtifactRef>,
    pub service_metrics: Vec<MetricValue>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepDisposition {
    Executed,
    NotRunAfterTermination,
    NotRunAfterAbort,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotRunReason {
    RestorationComplete,
    HorizonExhausted,
    OperatorStopped,
    NoValidActions,
    RunAborted,
    DeadlineExceeded,
    SystemFailure,
    EvidenceIncomplete,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlannerOutcome {
    ProposalEmitted,
    MalformedOutput,
    Timeout,
    Abstained,
    Error,
    NotRun,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransitionOutcome {
    Applied,
    Rejected,
    Failed,
    NotAttempted,
    HumanOverride,
    NotRun,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorOutcome {
    Accepted,
    Rejected,
    Inconclusive,
    NotRun,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceOutcome {
    Scored,
    NotScored,
    NotRun,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactRef {
    pub artifact_ref: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetricValue {
    pub metric_id: String,
    pub unit: String,
    pub value: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AggregateScore {
    pub metric_id: String,
    pub unit: String,
    pub value: f64,
    pub scorer_receipt: ArtifactRef,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RunReceiptReport {
    pub run_id: String,
    pub receipt_sha256: String,
    pub result_status: String,
    pub planned_decision_steps: usize,
    pub executed_decision_steps: usize,
    pub not_run_after_termination: usize,
    pub not_run_after_abort: usize,
    pub proposals_emitted: usize,
    pub malformed_outputs: usize,
    pub planner_timeouts: usize,
    pub planner_abstentions: usize,
    pub validator_rejections: usize,
    pub validator_inconclusive: usize,
    pub transitions_applied: usize,
    pub transitions_rejected: usize,
    pub transitions_failed: usize,
    pub human_overrides: usize,
    pub scorer_steps: usize,
    pub aggregate_score: Option<AggregateScore>,
    pub claim_boundary: String,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
}

fn lowercase_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub fn canonical_sha256<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    // serde_json::Value has deterministic map-key ordering; array order is significant.
    let canonical_value = serde_json::to_value(value)?;
    let bytes = serde_json::to_vec(&canonical_value)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn validate_artifact(artifact: &ArtifactRef, label: &str, errors: &mut Vec<String>) {
    if !valid_id(&artifact.artifact_ref) || !lowercase_hex(&artifact.sha256, 64) {
        errors.push(format!("{label} reference or lowercase SHA-256 is invalid"));
    }
}

fn validate_component(component: &ComponentIdentity, label: &str, errors: &mut Vec<String>) {
    if !valid_id(&component.component_ref)
        || !valid_id(&component.component_id)
        || !valid_id(&component.version)
        || !lowercase_hex(&component.source_commit, 40)
        || !lowercase_hex(&component.configuration_sha256, 64)
    {
        errors.push(format!("{label} component identity is invalid"));
    }
}

fn no_execution_payload(step: &DecisionRecord, label: &str, errors: &mut Vec<String>) {
    if step.observation.is_some()
        || step.planner_latency_ms.is_some()
        || step.planner_output.is_some()
        || step.input_tokens.is_some()
        || step.output_tokens.is_some()
        || step.api_cost_microusd.is_some()
        || step.proposal.is_some()
        || step.transition_receipt.is_some()
        || step.validator_receipt.is_some()
        || step.service_trajectory.is_some()
        || !step.service_metrics.is_empty()
        || step.retry_count != 0
        || step.planner_outcome != PlannerOutcome::NotRun
        || step.transition_outcome != TransitionOutcome::NotRun
        || step.validator_outcome != ValidatorOutcome::NotRun
        || step.service_outcome != ServiceOutcome::NotRun
    {
        errors.push(format!("{label} carries execution payload despite being marked not-run"));
    }
}

fn validate_decision(
    step: &DecisionRecord,
    receipt: &RunReceipt,
    case: &BenchmarkCase,
    case_sha256: &str,
    errors: &mut Vec<String>,
) {
    let label = format!("decision step {}", step.decision_step);
    match step.disposition {
        StepDisposition::NotRunAfterTermination | StepDisposition::NotRunAfterAbort => {
            if step.not_run_reason.is_none() {
                errors.push(format!("{label} has no explicit not-run reason"));
            }
            no_execution_payload(step, &label, errors);
            return;
        }
        StepDisposition::Executed => {
            if step.not_run_reason.is_some() {
                errors.push(format!("{label} executed step cannot carry a not-run reason"));
            }
        }
    }

    let Some(observation) = &step.observation else {
        errors.push(format!("{label} is missing its observation snapshot"));
        return;
    };
    validate_artifact(observation, &format!("{label} observation"), errors);
    if step.planner_outcome == PlannerOutcome::NotRun
        || step.transition_outcome == TransitionOutcome::NotRun
        || step.service_outcome == ServiceOutcome::NotRun
    {
        errors.push(format!("{label} executed step has a not-run stage outcome"));
    }

    if let Some(value) = step.planner_latency_ms {
        if value == 0 {
            errors.push(format!("{label} planner latency must be positive when measured"));
        }
    }
    if step.input_tokens.is_some() != step.output_tokens.is_some() {
        errors.push(format!("{label} token accounting must report input and output together"));
    }
    if step.api_cost_microusd.is_some() && step.planner_outcome == PlannerOutcome::NotRun {
        errors.push(format!("{label} API cost cannot exist for a planner stage that did not run"));
    }

    match step.planner_outcome {
        PlannerOutcome::ProposalEmitted => {
            let Some(proposal) = &step.proposal else {
                errors.push(format!("{label} says proposal_emitted but contains no proposal"));
                return;
            };
            if step.planner_output.is_some() {
                errors.push(format!("{label} valid proposal cannot also use malformed raw-output artifact"));
            }
            match validate_proposal(proposal, case, case_sha256) {
                Ok(_) => {}
                Err(proposal_errors) => {
                    for error in proposal_errors {
                        errors.push(format!("{label} proposal: {error}"));
                    }
                }
            }
            if proposal.decision_step != step.decision_step
                || proposal.observation.observation_ref != observation.artifact_ref
                || proposal.observation.observation_sha256 != observation.sha256
            {
                errors.push(format!("{label} proposal does not bind to the exact decision observation"));
            }
            if proposal.planner.planner_id != receipt.planner.planner_id
                || proposal.planner.source_commit != receipt.planner.source_commit
                || proposal.planner.configuration_sha256 != receipt.planner.configuration_sha256
                || proposal.planner.prompt_sha256 != receipt.planner.prompt_sha256
            {
                errors.push(format!("{label} proposal planner identity differs from the run identity"));
            }
        }
        PlannerOutcome::MalformedOutput => {
            if step.proposal.is_some() || step.planner_output.is_none() {
                errors.push(format!("{label} malformed output must carry a raw output artifact and no parsed proposal"));
            }
            if let Some(output) = &step.planner_output {
                validate_artifact(output, &format!("{label} raw planner output"), errors);
            }
        }
        PlannerOutcome::Timeout | PlannerOutcome::Abstained | PlannerOutcome::Error => {
            if step.proposal.is_some() {
                errors.push(format!("{label} non-proposal planner outcome carries a proposal"));
            }
            if let Some(output) = &step.planner_output {
                validate_artifact(output, &format!("{label} partial planner output"), errors);
            }
        }
        PlannerOutcome::NotRun => errors.push(format!("{label} executed step has planner_outcome=not_run")),
    }

    let proposal_exists = step.proposal.is_some();
    if !proposal_exists {
        if step.transition_outcome != TransitionOutcome::NotAttempted
            || step.transition_receipt.is_some()
            || step.validator_outcome != ValidatorOutcome::NotRun
            || step.validator_receipt.is_some()
        {
            errors.push(format!("{label} cannot transition or validate without a parsed proposal"));
        }
    }

    match step.validator_outcome {
        ValidatorOutcome::Accepted | ValidatorOutcome::Rejected | ValidatorOutcome::Inconclusive => {
            if step.validator_receipt.is_none() {
                errors.push(format!("{label} validator result lacks an independent validator receipt"));
            }
            if let Some(receipt) = &step.validator_receipt {
                validate_artifact(receipt, &format!("{label} validator receipt"), errors);
            }
        }
        ValidatorOutcome::NotRun => {
            if step.validator_receipt.is_some() {
                errors.push(format!("{label} has a validator receipt but validator_outcome=not_run"));
            }
        }
    }

    match step.transition_outcome {
        TransitionOutcome::Applied | TransitionOutcome::Failed => {
            if step.validator_outcome != ValidatorOutcome::Accepted || step.transition_receipt.is_none() {
                errors.push(format!("{label} applied/failed transition requires accepted validation and a transition receipt"));
            }
        }
        TransitionOutcome::Rejected => {
            if step.transition_receipt.is_none() {
                errors.push(format!("{label} rejected transition requires a transition receipt"));
            }
        }
        TransitionOutcome::NotAttempted => {
            if step.transition_receipt.is_some() {
                errors.push(format!("{label} not_attempted transition cannot carry a transition receipt"));
            }
        }
        TransitionOutcome::HumanOverride => {
            if step.transition_receipt.is_none() {
                errors.push(format!("{label} human override requires a transition/override receipt"));
            }
        }
        TransitionOutcome::NotRun => errors.push(format!("{label} executed step has transition_outcome=not_run")),
    }
    if let Some(transition) = &step.transition_receipt {
        validate_artifact(transition, &format!("{label} transition receipt"), errors);
    }

    match step.service_outcome {
        ServiceOutcome::Scored => {
            if step.service_trajectory.is_none() || step.service_metrics.is_empty() {
                errors.push(format!("{label} scored service outcome requires trajectory reference and metric values"));
            }
            if let Some(trajectory) = &step.service_trajectory {
                validate_artifact(trajectory, &format!("{label} service trajectory"), errors);
            }
            let mut metric_ids = BTreeSet::new();
            for metric in &step.service_metrics {
                if !valid_id(&metric.metric_id)
                    || !valid_id(&metric.unit)
                    || !metric.value.is_finite()
                    || !metric_ids.insert(metric.metric_id.as_str())
                {
                    errors.push(format!("{label} has invalid or duplicate service metric values"));
                }
            }
        }
        ServiceOutcome::NotScored => {
            if step.service_trajectory.is_some() || !step.service_metrics.is_empty() {
                errors.push(format!("{label} not_scored outcome cannot carry scored service values"));
            }
        }
        ServiceOutcome::NotRun => errors.push(format!("{label} executed step has service_outcome=not_run")),
    }
}

fn validate_run(receipt: &RunReceipt, case: &BenchmarkCase) -> Vec<String> {
    let mut errors = Vec::new();
    if receipt.schema_version != "0.1.0" || !valid_id(&receipt.run_id) {
        errors.push("run receipt schema version or run ID is invalid".to_string());
    }
    if receipt.maturity != case.maturity {
        errors.push("run maturity differs from the benchmark case maturity".to_string());
    }

    let case_sha = match canonical_case_sha256(case) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("benchmark case canonicalization failed: {error}"));
            String::new()
        }
    };
    if receipt.case.case_sha256 != case_sha
        || receipt.case.case_ref != format!("power-restoration-case://{}", case.case_id)
    {
        errors.push("run receipt case reference/digest does not match the supplied frozen case".to_string());
    }

    if !valid_id(&receipt.pairing.scenario_group_id)
        || !valid_id(&receipt.pairing.paired_replicate_id)
        || !valid_id(&receipt.pairing.observation_profile_id)
    {
        errors.push("paired scenario/replicate/observation identity is incomplete".to_string());
    }

    let planner = &receipt.planner;
    if !valid_id(&planner.planner_id)
        || !valid_id(&planner.planner_revision)
        || !lowercase_hex(&planner.source_commit, 40)
        || !lowercase_hex(&planner.configuration_sha256, 64)
        || planner.prompt_sha256.as_ref().is_some_and(|value| !lowercase_hex(value, 64))
        || !valid_id(&planner.model_provider)
        || !valid_id(&planner.model_id)
        || !valid_id(&planner.model_revision)
    {
        errors.push("planner identity, model revision or configuration/prompt digest is invalid".to_string());
    }
    validate_component(&receipt.adapter, "adapter", &mut errors);
    validate_component(&receipt.simulator, "simulator", &mut errors);
    validate_component(&receipt.independent_validator, "independent validator", &mut errors);
    validate_component(&receipt.objective.scorer, "objective scorer", &mut errors);
    if receipt.independent_validator.component_ref != case.model_profile.independent_validator_ref {
        errors.push("run independent validator does not match the validator declared by the benchmark case".to_string());
    }

    let objective = &receipt.objective;
    if objective.objective_id != case.objective.objective_id
        || objective.metric != case.objective.metric
        || objective.expression_ref != case.objective.expression_ref
        || !valid_id(&objective.unit)
    {
        errors.push("run objective identity does not match the benchmark case objective".to_string());
    }

    match receipt.hardware.measurement_status {
        HardwareMeasurementStatus::Measured => {
            if receipt.hardware.operating_system.as_ref().is_none_or(|value| !valid_id(value))
                || receipt.hardware.architecture.as_ref().is_none_or(|value| !valid_id(value))
                || receipt.hardware.cpu_model.as_ref().is_none_or(|value| !valid_id(value))
                || receipt.hardware.logical_cpu_count.is_none_or(|value| value == 0)
                || receipt.hardware.memory_bytes.is_none_or(|value| value == 0)
            {
                errors.push("measured hardware profile is missing host/CPU/memory measurements".to_string());
            }
        }
        HardwareMeasurementStatus::NotMeasured => {
            if receipt.hardware.logical_cpu_count.is_some()
                || receipt.hardware.memory_bytes.is_some()
                || !receipt.hardware.accelerators.is_empty()
            {
                errors.push("not_measured hardware profile must not contain invented capacity measurements".to_string());
            }
        }
    }
    if !valid_id(&receipt.hardware.runtime_version)
        || receipt.hardware.compiler_or_solver_versions.iter().any(|value| !valid_id(value))
    {
        errors.push("runtime/compiler/solver version evidence is incomplete".to_string());
    }
    for accelerator in &receipt.hardware.accelerators {
        if !valid_id(&accelerator.device_id)
            || !valid_id(&accelerator.model)
            || accelerator.memory_bytes == 0
            || !valid_id(&accelerator.driver_version)
        {
            errors.push("accelerator inventory contains an invalid device record".to_string());
        }
    }

    if receipt.budget.wall_clock_limit_ms == 0
        || receipt.budget.solver_time_limit_ms == Some(0)
        || receipt.budget.solver_gap_limit_basis_points.is_some_and(|value| value > 10_000)
    {
        errors.push("wall-clock/solver budget or gap limit is invalid".to_string());
    }

    let horizon = case.timebase.horizon_steps as usize;
    if receipt.decision_steps.len() != horizon {
        errors.push(format!("run must contain exactly {horizon} decision records, including not-run steps"));
    }
    for (index, step) in receipt.decision_steps.iter().enumerate() {
        if step.decision_step as usize != index {
            errors.push("decision records must be complete, ordered and zero-based from step 0".to_string());
        }
        validate_decision(step, receipt, case, &case_sha, &mut errors);
    }

    let mut saw_not_run_tail = false;
    let mut seen_abort_tail = false;
    for step in &receipt.decision_steps {
        match step.disposition {
            StepDisposition::Executed if saw_not_run_tail || seen_abort_tail => {
                errors.push("executed decision appears after a not-run tail".to_string());
            }
            StepDisposition::NotRunAfterTermination => {
                saw_not_run_tail = true;
                if receipt.run_status != RunStatus::Completed {
                    errors.push("not_run_after_termination is only valid for a completed run".to_string());
                }
            }
            StepDisposition::NotRunAfterAbort => {
                seen_abort_tail = true;
                if receipt.run_status == RunStatus::Completed {
                    errors.push("not_run_after_abort is invalid for a completed run".to_string());
                }
            }
            StepDisposition::Executed => {}
        }
    }

    match receipt.run_status {
        RunStatus::Completed => {
            if !matches!(receipt.terminal_reason, Some(TerminalReason::RestorationComplete | TerminalReason::HorizonExhausted | TerminalReason::OperatorStopped)) {
                errors.push("completed run requires a valid completion terminal reason".to_string());
            }
            if receipt.aggregate_score.is_none() || receipt.final_service_trajectory.is_none() {
                errors.push("completed run requires an independent aggregate score and final service trajectory receipt".to_string());
            }
        }
        RunStatus::Aborted | RunStatus::TimedOut | RunStatus::Incomplete => {
            if receipt.aggregate_score.is_some() || receipt.final_service_trajectory.is_some() {
                errors.push("aborted/timed-out/incomplete run cannot claim a completed aggregate score or final trajectory".to_string());
            }
            let expected = match receipt.run_status {
                RunStatus::Aborted => Some(TerminalReason::RunAborted),
                RunStatus::TimedOut => Some(TerminalReason::DeadlineExceeded),
                RunStatus::Incomplete => Some(TerminalReason::EvidenceIncomplete),
                RunStatus::Completed => None,
            };
            if receipt.terminal_reason != expected {
                errors.push("non-completed run terminal reason does not match run_status".to_string());
            }
        }
    }

    if let Some(final_trajectory) = &receipt.final_service_trajectory {
        validate_artifact(final_trajectory, "final service trajectory", &mut errors);
    }
    if let Some(score) = &receipt.aggregate_score {
        if !valid_id(&score.metric_id)
            || score.metric_id != receipt.objective.metric
            || score.unit != receipt.objective.unit
            || !score.value.is_finite()
        {
            errors.push("aggregate score metric, unit or value is invalid or differs from the declared objective".to_string());
        }
        validate_artifact(&score.scorer_receipt, "aggregate scorer receipt", &mut errors);
    }

    errors
}

pub fn qualify(receipt: &RunReceipt, case: &BenchmarkCase) -> Result<RunReceiptReport, Vec<String>> {
    let case_errors = validate_case(case);
    if !case_errors.is_empty() {
        return Err(case_errors.into_iter().map(|error| format!("benchmark case: {error}")).collect());
    }
    let errors = validate_run(receipt, case);
    if !errors.is_empty() {
        return Err(errors);
    }

    let receipt_sha256 = canonical_sha256(receipt)
        .map_err(|error| vec![format!("run receipt canonicalization failed: {error}")])?;
    let executed: Vec<&DecisionRecord> = receipt.decision_steps.iter()
        .filter(|step| step.disposition == StepDisposition::Executed).collect();
    let count = |predicate: fn(&DecisionRecord) -> bool| executed.iter().filter(|step| predicate(step)).count();

    Ok(RunReceiptReport {
        run_id: receipt.run_id.clone(),
        receipt_sha256,
        result_status: "run_receipt_consistent".to_string(),
        planned_decision_steps: receipt.decision_steps.len(),
        executed_decision_steps: executed.len(),
        not_run_after_termination: receipt.decision_steps.iter().filter(|step| step.disposition == StepDisposition::NotRunAfterTermination).count(),
        not_run_after_abort: receipt.decision_steps.iter().filter(|step| step.disposition == StepDisposition::NotRunAfterAbort).count(),
        proposals_emitted: count(|step| step.planner_outcome == PlannerOutcome::ProposalEmitted),
        malformed_outputs: count(|step| step.planner_outcome == PlannerOutcome::MalformedOutput),
        planner_timeouts: count(|step| step.planner_outcome == PlannerOutcome::Timeout),
        planner_abstentions: count(|step| step.planner_outcome == PlannerOutcome::Abstained),
        validator_rejections: count(|step| step.validator_outcome == ValidatorOutcome::Rejected),
        validator_inconclusive: count(|step| step.validator_outcome == ValidatorOutcome::Inconclusive),
        transitions_applied: count(|step| step.transition_outcome == TransitionOutcome::Applied),
        transitions_rejected: count(|step| step.transition_outcome == TransitionOutcome::Rejected),
        transitions_failed: count(|step| step.transition_outcome == TransitionOutcome::Failed),
        human_overrides: count(|step| step.transition_outcome == TransitionOutcome::HumanOverride),
        scorer_steps: count(|step| step.service_outcome == ServiceOutcome::Scored),
        aggregate_score: receipt.aggregate_score.clone(),
        claim_boundary: "Benchmark run provenance and stage-accounting receipt only; does not itself establish physical feasibility, operational authorization or field restoration".to_string(),
    })
}

pub fn parse_and_qualify(receipt_json: &str, case_json: &str) -> Result<RunReceiptReport, Vec<String>> {
    let receipt: RunReceipt = serde_json::from_str(receipt_json)
        .map_err(|error| vec![format!("run receipt JSON decode error: {error}")])?;
    let case: BenchmarkCase = serde_json::from_str(case_json)
        .map_err(|error| vec![format!("benchmark case JSON decode error: {error}")])?;
    qualify(&receipt, &case)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case() -> BenchmarkCase {
        serde_json::from_str(include_str!("../../power-restoration-benchmark-case/fixtures/synthetic-restoration-case-v0.1.json"))
            .expect("case fixture must deserialize")
    }

    fn fixture() -> RunReceipt {
        serde_json::from_str(include_str!("../fixtures/synthetic-restoration-run-receipt-v0.1.json"))
            .expect("run receipt fixture must deserialize")
    }

    #[test]
    fn synthetic_receipt_is_qualified_and_every_epoch_is_accounted_for() {
        let report = qualify(&fixture(), &case()).expect("synthetic run receipt should qualify");
        assert_eq!(report.result_status, "run_receipt_consistent");
        assert_eq!(report.planned_decision_steps, 24);
        assert_eq!(report.executed_decision_steps, 1);
        assert_eq!(report.not_run_after_termination, 23);
        assert_eq!(report.proposals_emitted, 1);
        assert_eq!(report.transitions_applied, 1);
        assert_eq!(report.scorer_steps, 1);
        assert_eq!(report.receipt_sha256.len(), 64);
    }

    #[test]
    fn case_digest_mismatch_is_rejected() {
        let mut receipt = fixture();
        receipt.case.case_sha256 = "0".repeat(64);
        assert!(qualify(&receipt, &case()).is_err());
    }

    #[test]
    fn missing_decision_epoch_is_rejected() {
        let mut receipt = fixture();
        receipt.decision_steps.pop();
        assert!(qualify(&receipt, &case()).is_err());
    }

    #[test]
    fn duplicate_or_out_of_order_decision_epoch_is_rejected() {
        let mut receipt = fixture();
        receipt.decision_steps[1].decision_step = 0;
        assert!(qualify(&receipt, &case()).is_err());
    }

    #[test]
    fn not_run_step_must_name_termination_reason() {
        let mut receipt = fixture();
        receipt.decision_steps[1].not_run_reason = None;
        assert!(qualify(&receipt, &case()).is_err());
    }

    #[test]
    fn not_run_step_cannot_carry_fake_observation_or_result() {
        let mut receipt = fixture();
        receipt.decision_steps[1].observation = Some(ArtifactRef {
            artifact_ref: "synthetic://fake-observation".to_string(),
            sha256: "a".repeat(64),
        });
        assert!(qualify(&receipt, &case()).is_err());
    }

    #[test]
    fn proposal_must_bind_to_exact_case_step_and_observation() {
        let mut receipt = fixture();
        receipt.decision_steps[0].proposal.as_mut().unwrap().observation.observed_step = 1;
        assert!(qualify(&receipt, &case()).is_err());
    }

    #[test]
    fn proposal_planner_identity_must_match_run_identity() {
        let mut receipt = fixture();
        receipt.decision_steps[0].proposal.as_mut().unwrap().planner.planner_id = "other-planner".to_string();
        assert!(qualify(&receipt, &case()).is_err());
    }

    #[test]
    fn proposal_requires_independent_validation_receipt() {
        let mut receipt = fixture();
        receipt.decision_steps[0].validator_receipt = None;
        assert!(qualify(&receipt, &case()).is_err());
    }

    #[test]
    fn applied_transition_requires_accepted_validation_and_receipt() {
        let mut receipt = fixture();
        receipt.decision_steps[0].validator_outcome = ValidatorOutcome::Rejected;
        assert!(qualify(&receipt, &case()).is_err());
    }

    #[test]
    fn failed_transition_is_not_dropped_from_attempt_accounting() {
        let mut receipt = fixture();
        let step = &mut receipt.decision_steps[0];
        step.transition_outcome = TransitionOutcome::Failed;
        step.transition_receipt = Some(ArtifactRef {
            artifact_ref: "synthetic://transition/failed".to_string(),
            sha256: "b".repeat(64),
        });
        let report = qualify(&receipt, &case()).expect("failed attempt remains a valid run event");
        assert_eq!(report.transitions_failed, 1);
    }

    #[test]
    fn completed_run_requires_independent_aggregate_score() {
        let mut receipt = fixture();
        receipt.aggregate_score = None;
        assert!(qualify(&receipt, &case()).is_err());
    }

    #[test]
    fn incomplete_run_cannot_publish_completed_score() {
        let mut receipt = fixture();
        receipt.run_status = RunStatus::TimedOut;
        receipt.terminal_reason = Some(TerminalReason::DeadlineExceeded);
        receipt.aggregate_score = None;
        receipt.final_service_trajectory = None;
        for step in &mut receipt.decision_steps[1..] {
            step.disposition = StepDisposition::NotRunAfterAbort;
            step.not_run_reason = Some(NotRunReason::DeadlineExceeded);
        }
        assert!(qualify(&receipt, &case()).is_ok());
    }

    #[test]
    fn measured_hardware_requires_real_host_measurements() {
        let mut receipt = fixture();
        receipt.hardware.measurement_status = HardwareMeasurementStatus::Measured;
        receipt.hardware.logical_cpu_count = None;
        assert!(qualify(&receipt, &case()).is_err());
    }

    #[test]
    fn unmeasured_hardware_cannot_claim_accelerator_capacity() {
        let mut receipt = fixture();
        receipt.hardware.measurement_status = HardwareMeasurementStatus::NotMeasured;
        receipt.hardware.accelerators.push(Accelerator {
            device_id: "gpu0".to_string(),
            model: "not-measured".to_string(),
            memory_bytes: 8_000_000_000,
            driver_version: "unknown".to_string(),
        });
        assert!(qualify(&receipt, &case()).is_err());
    }

    #[test]
    fn objective_cannot_change_between_case_and_run_receipt() {
        let mut receipt = fixture();
        receipt.objective.metric = "unrelated_scalar".to_string();
        assert!(qualify(&receipt, &case()).is_err());
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut value = serde_json::to_value(fixture()).unwrap();
        value["authorized"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<RunReceipt>(value).is_err());
    }

    #[test]
    fn deterministic_receipt_hash_is_stable() {
        let receipt = fixture();
        assert_eq!(canonical_sha256(&receipt).unwrap(), canonical_sha256(&receipt).unwrap());
    }
}
