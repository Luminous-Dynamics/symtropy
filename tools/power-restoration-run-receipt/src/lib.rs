use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const HEX_SHA256: usize = 64;
const HEX_GIT_SHA1: usize = 40;
const TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiptEnvelope {
    pub receipt: RunReceipt,
    pub receipt_sha256: Option<String>,
    pub collector_attestation: Option<CollectorAttestation>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunReceipt {
    pub schema_version: String,
    pub run_id: String,
    pub run_status: RunStatus,
    pub maturity: EvidenceMaturity,
    pub case_ref: String,
    pub case_sha256: String,
    pub paired_scenario_id: String,
    pub replicate_id: String,
    pub random_seed: u64,
    pub planner: PlannerIdentity,
    pub adapter: ComponentIdentity,
    pub transition_engine: ComponentIdentity,
    pub independent_validator: ComponentIdentity,
    pub independent_scorer: ComponentIdentity,
    pub hardware: HardwareProfile,
    pub budget: RunBudget,
    pub measured_usage: MeasuredUsage,
    pub decision_epochs: Vec<DecisionEpoch>,
    pub attempts: Vec<AttemptRecord>,
    pub declared_attempt_count: u32,
    pub attempt_log_complete: bool,
    pub validator_receipts: Vec<ValidatorReceipt>,
    pub transition_receipts: Vec<TransitionReceipt>,
    pub trajectory_sha256: Option<String>,
    pub scoring_receipt: Option<ScoringReceipt>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Completed,
    HorizonReached,
    Aborted,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceMaturity {
    SyntheticFixture,
    HistoricalReplay,
    FieldObserved,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlannerIdentity {
    pub planner_id: String,
    pub planner_revision: String,
    pub source_commit: String,
    pub configuration_sha256: String,
    pub prompt_sha256: Option<String>,
    pub model_id: String,
    pub model_revision: String,
    pub provider: String,
    pub quantization: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentIdentity {
    pub component_id: String,
    pub revision: String,
    pub source_commit: String,
    pub configuration_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HardwareProfile {
    pub operating_system: String,
    pub architecture: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub ram_bytes: u64,
    pub gpu_model: Option<String>,
    pub gpu_vram_bytes: Option<u64>,
    pub runtime_name: String,
    pub runtime_version: String,
    pub container_image_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunBudget {
    pub wall_clock_limit_ms: u64,
    pub decision_limit_ms: u64,
    pub token_limit: Option<u64>,
    pub tool_call_limit: u32,
    pub solver_time_limit_ms: Option<u64>,
    pub solver_relative_gap_limit: Option<f64>,
    pub api_cost_limit: Option<CostAmount>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeasuredUsage {
    pub wall_clock_ms: u64,
    pub cpu_time_ms: u64,
    pub gpu_time_ms: Option<u64>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub tool_calls: u64,
    pub solver_calls: u64,
    pub api_retries: u64,
    pub timeout_count: u64,
    pub human_override_count: u64,
    pub api_cost: Option<CostAmount>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CostAmount {
    pub amount: f64,
    pub currency: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecisionEpoch {
    pub decision_step: u32,
    pub observation_ref: String,
    pub observation_sha256: String,
    pub observation_step: u32,
    pub observation_time_ms: u64,
    pub decision_time_ms: u64,
    pub proposal_ref: Option<String>,
    pub proposal_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttemptRecord {
    pub attempt_id: String,
    pub sequence: u32,
    pub decision_step: u32,
    pub attempt_kind: AttemptKind,
    pub outcome: AttemptOutcome,
    pub action_id: Option<String>,
    pub proposal_sha256: Option<String>,
    pub validator_receipt_id: Option<String>,
    pub transition_receipt_id: Option<String>,
    pub started_at_ms: u64,
    pub ended_at_ms: u64,
    pub latency_ms: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost: Option<CostAmount>,
    pub failure_ref: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttemptKind {
    PlannerCall,
    SolverCall,
    ToolCall,
    ActionSubmission,
    HumanOverride,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttemptOutcome {
    Succeeded,
    Malformed,
    TimedOut,
    Rejected,
    Applied,
    ExecutionFailed,
    HumanOverride,
    Abstained,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ValidatorReceipt {
    pub receipt_id: String,
    pub decision_step: u32,
    pub proposal_sha256: String,
    pub status: ValidatorStatus,
    pub validator: ComponentIdentity,
    pub rejected_action_ids: Vec<String>,
    pub receipt_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorStatus {
    Accepted,
    Rejected,
    Inconclusive,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransitionReceipt {
    pub receipt_id: String,
    pub decision_step: u32,
    pub proposal_sha256: String,
    pub status: TransitionStatus,
    pub engine: ComponentIdentity,
    pub state_before_sha256: String,
    pub state_after_sha256: String,
    pub applied_action_ids: Vec<String>,
    pub failed_action_ids: Vec<String>,
    pub receipt_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransitionStatus {
    Applied,
    Failed,
    Rejected,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScoringReceipt {
    pub scorer: ComponentIdentity,
    pub case_sha256: String,
    pub trajectory_sha256: String,
    pub score_definition_ref: String,
    pub output_sha256: String,
    pub acceptance_status: ScoringAcceptance,
    pub acceptance_ref: String,
    pub acceptance_sha256: String,
    pub metrics: Vec<MetricResult>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScoringAcceptance {
    Accepted,
    Pending,
    Rejected,
    Inconclusive,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetricResult {
    pub metric_id: String,
    pub unit: String,
    pub value: f64,
    pub beneficiary_scope: Option<String>,
    pub cohort_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CollectorAttestation {
    pub signer_ref: String,
    pub signed_receipt_sha256: String,
    pub verification_status: AttestationStatus,
    pub verification_receipt_ref: Option<String>,
    pub verification_receipt_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttestationStatus {
    Verified,
    Pending,
    Rejected,
    NotProvided,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptReport {
    pub run_id: String,
    pub receipt_sha256: String,
    pub run_status: RunStatus,
    pub maturity: EvidenceMaturity,
    pub result_status: String,
    pub declared_attempt_count: u32,
    pub recorded_attempt_count: usize,
    pub failed_attempt_count: usize,
    pub applied_action_count: usize,
    pub rejected_action_count: usize,
    pub timed_out_attempt_count: usize,
    pub independent_score_present: bool,
    pub score_metrics: Vec<MetricResult>,
    pub claim_boundary: String,
}

pub fn canonical_receipt_sha256(receipt: &RunReceipt) -> Result<String, serde_json::Error> {
    let canonical_value = serde_json::to_value(receipt)?;
    let bytes = serde_json::to_vec(&canonical_value)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha(value: &str, length: usize) -> bool {
    value.len() == length
        && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_commit(value: &str) -> bool {
    value.len() == HEX_GIT_SHA1 && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_cost(cost: &CostAmount) -> bool {
    cost.amount.is_finite() && cost.amount >= 0.0 && valid_id(&cost.currency)
}

fn validate_component(component: &ComponentIdentity, label: &str, errors: &mut Vec<String>) {
    if !valid_id(&component.component_id)
        || !valid_id(&component.revision)
        || !valid_commit(&component.source_commit)
        || !valid_sha(&component.configuration_sha256, HEX_SHA256)
    {
        errors.push(format!("{label} identity, commit or configuration digest is invalid"));
    }
}

fn validate_receipt(receipt: &RunReceipt, errors: &mut Vec<String>) {
    if receipt.schema_version != "0.1.0"
        || !valid_id(&receipt.run_id)
        || !valid_id(&receipt.case_ref)
        || !valid_sha(&receipt.case_sha256, HEX_SHA256)
        || !valid_id(&receipt.paired_scenario_id)
        || !valid_id(&receipt.replicate_id)
    {
        errors.push("run/case/paired-replicate identity or case digest is invalid".to_string());
    }

    let planner = &receipt.planner;
    if !valid_id(&planner.planner_id)
        || !valid_id(&planner.planner_revision)
        || !valid_commit(&planner.source_commit)
        || !valid_sha(&planner.configuration_sha256, HEX_SHA256)
        || planner.prompt_sha256.as_ref().is_some_and(|value| !valid_sha(value, HEX_SHA256))
        || !valid_id(&planner.model_id)
        || !valid_id(&planner.model_revision)
        || !valid_id(&planner.provider)
        || !valid_id(&planner.quantization)
    {
        errors.push("planner identity, source commit, model or configuration digest is invalid".to_string());
    }

    validate_component(&receipt.adapter, "adapter", errors);
    validate_component(&receipt.transition_engine, "transition engine", errors);
    validate_component(&receipt.independent_validator, "independent validator", errors);
    validate_component(&receipt.independent_scorer, "independent scorer", errors);

    let hardware = &receipt.hardware;
    if !valid_id(&hardware.operating_system)
        || !valid_id(&hardware.architecture)
        || !valid_id(&hardware.cpu_model)
        || hardware.cpu_cores == 0
        || hardware.ram_bytes == 0
        || !valid_id(&hardware.runtime_name)
        || !valid_id(&hardware.runtime_version)
        || hardware.container_image_sha256.as_ref().is_some_and(|value| !valid_sha(value, HEX_SHA256))
    {
        errors.push("hardware/runtime profile is incomplete or invalid".to_string());
    }
    if hardware.gpu_model.is_some() != hardware.gpu_vram_bytes.is_some()
        || hardware.gpu_vram_bytes.is_some_and(|value| value == 0)
    {
        errors.push("GPU model and VRAM must be declared together with positive VRAM".to_string());
    }

    let budget = &receipt.budget;
    if budget.wall_clock_limit_ms == 0
        || budget.decision_limit_ms == 0
        || budget.tool_call_limit == 0
        || budget.solver_relative_gap_limit.is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        || budget.api_cost_limit.as_ref().is_some_and(|value| !valid_cost(value))
    {
        errors.push("run budget values are invalid".to_string());
    }
    if let Some(cost) = &receipt.measured_usage.api_cost {
        if !valid_cost(cost) {
            errors.push("measured API cost is invalid".to_string());
        }
    }
    if receipt.measured_usage.wall_clock_ms == 0 || receipt.measured_usage.wall_clock_ms > budget.wall_clock_limit_ms {
        errors.push("measured wall-clock exceeds the declared run budget".to_string());
    }
    if receipt.measured_usage.tool_calls > budget.tool_call_limit as u64 {
        errors.push("measured tool calls exceed the declared run budget".to_string());
    }
    let recorded_tool_calls = receipt.attempts.iter().filter(|attempt| attempt.attempt_kind == AttemptKind::ToolCall).count() as u64;
    let recorded_solver_calls = receipt.attempts.iter().filter(|attempt| attempt.attempt_kind == AttemptKind::SolverCall).count() as u64;
    let recorded_timeouts = receipt.attempts.iter().filter(|attempt| attempt.outcome == AttemptOutcome::TimedOut).count() as u64;
    let recorded_overrides = receipt.attempts.iter().filter(|attempt| attempt.outcome == AttemptOutcome::HumanOverride).count() as u64;
    if receipt.measured_usage.tool_calls < recorded_tool_calls
        || receipt.measured_usage.solver_calls < recorded_solver_calls
        || receipt.measured_usage.timeout_count < recorded_timeouts
        || receipt.measured_usage.human_override_count < recorded_overrides
    {
        errors.push("measured usage counters under-report events in the attempt log".to_string());
    }
    if let Some(limit) = &budget.token_limit {
        let actual = receipt.measured_usage.input_tokens.saturating_add(receipt.measured_usage.output_tokens);
        if actual > *limit {
            errors.push("measured tokens exceed the declared run budget".to_string());
        }
    }
    if let (Some(limit), Some(actual)) = (&budget.api_cost_limit, &receipt.measured_usage.api_cost) {
        if limit.currency != actual.currency || actual.amount > limit.amount + TOLERANCE {
            errors.push("measured API cost exceeds or mismatches the declared cost budget".to_string());
        }
    }

    if receipt.decision_epochs.is_empty() {
        errors.push("run receipt must contain at least one decision epoch".to_string());
    }
    let mut epoch_ids = BTreeSet::new();
    let mut prior_step = None;
    let mut prior_time = None;
    for epoch in &receipt.decision_epochs {
        if !epoch_ids.insert(epoch.decision_step)
            || prior_step.is_some_and(|value| epoch.decision_step <= value)
            || prior_time.is_some_and(|value| epoch.observation_time_ms < value)
            || !valid_id(&epoch.observation_ref)
            || !valid_sha(&epoch.observation_sha256, HEX_SHA256)
            || epoch.observation_step > epoch.decision_step
            || epoch.observation_time_ms > epoch.decision_time_ms
            || epoch.decision_time_ms > receipt.measured_usage.wall_clock_ms
        {
            errors.push(format!("decision epoch {} has invalid identity or non-monotonic time", epoch.decision_step));
        }
        match (&epoch.proposal_ref, &epoch.proposal_sha256) {
            (Some(reference), Some(digest)) if valid_id(reference) && valid_sha(digest, HEX_SHA256) => {}
            (None, None) => {}
            _ => errors.push(format!("decision epoch {} has incomplete proposal reference/digest", epoch.decision_step)),
        }
        prior_step = Some(epoch.decision_step);
        prior_time = Some(epoch.observation_time_ms);
    }

    if receipt.attempts.len() != receipt.declared_attempt_count as usize {
        errors.push(format!(
            "declared attempt count {} differs from {} recorded attempt events",
            receipt.declared_attempt_count,
            receipt.attempts.len()
        ));
    }
    if !receipt.attempt_log_complete {
        errors.push("attempt log is explicitly incomplete; failed calls/actions cannot be assumed absent".to_string());
    }

    let epochs: BTreeMap<u32, &DecisionEpoch> = receipt.decision_epochs.iter()
        .map(|epoch| (epoch.decision_step, epoch)).collect();
    let validators: BTreeMap<&str, &ValidatorReceipt> = receipt.validator_receipts.iter()
        .map(|item| (item.receipt_id.as_str(), item)).collect();
    let transitions: BTreeMap<&str, &TransitionReceipt> = receipt.transition_receipts.iter()
        .map(|item| (item.receipt_id.as_str(), item)).collect();

    if validators.len() != receipt.validator_receipts.len()
        || transitions.len() != receipt.transition_receipts.len()
    {
        errors.push("validator/transition receipt IDs must be unique".to_string());
    }

    let mut attempt_ids = BTreeSet::new();
    for (index, attempt) in receipt.attempts.iter().enumerate() {
        if !valid_id(&attempt.attempt_id)
            || !attempt_ids.insert(attempt.attempt_id.as_str())
            || attempt.sequence as usize != index
            || !epochs.contains_key(&attempt.decision_step)
        {
            errors.push(format!("attempt {} has invalid identity, sequence or decision step", attempt.attempt_id));
            continue;
        }
        let epoch = epochs[&attempt.decision_step];
        if attempt.started_at_ms > attempt.ended_at_ms
            || attempt.ended_at_ms.saturating_sub(attempt.started_at_ms) != attempt.latency_ms
            || attempt.ended_at_ms > receipt.measured_usage.wall_clock_ms
            || attempt.latency_ms > receipt.budget.decision_limit_ms
        {
            errors.push(format!("attempt {} has invalid timing or exceeds per-decision latency budget", attempt.attempt_id));
        }
        if index > 0 && attempt.started_at_ms < receipt.attempts[index - 1].ended_at_ms {
            errors.push(format!("attempt {} overlaps or precedes the prior attempt in the event log", attempt.attempt_id));
        }
        if matches!(attempt.outcome, AttemptOutcome::Malformed | AttemptOutcome::TimedOut | AttemptOutcome::Rejected | AttemptOutcome::ExecutionFailed | AttemptOutcome::Failed)
            && attempt.failure_ref.as_deref().is_none_or(|value| !valid_id(value))
        {
            errors.push(format!("failed attempt {} requires a failure reference", attempt.attempt_id));
        }
        if (attempt.outcome == AttemptOutcome::HumanOverride) != (attempt.attempt_kind == AttemptKind::HumanOverride) {
            errors.push(format!("attempt {} has an inconsistent human-override kind/outcome", attempt.attempt_id));
        }
        if attempt.proposal_sha256.as_ref().is_some_and(|digest| !valid_sha(digest, HEX_SHA256)) {
            errors.push(format!("attempt {} has an invalid proposal digest", attempt.attempt_id));
        }
        if attempt.attempt_kind == AttemptKind::ActionSubmission {
            if attempt.proposal_sha256.as_deref() != epoch.proposal_sha256.as_deref()
                || epoch.proposal_sha256.is_none()
            {
                errors.push(format!("action attempt {} is not bound to the epoch's exact proposal", attempt.attempt_id));
            }
            let needs_action_id = matches!(attempt.outcome, AttemptOutcome::Applied | AttemptOutcome::Rejected | AttemptOutcome::ExecutionFailed);
            if needs_action_id && !valid_id(attempt.action_id.as_deref().unwrap_or("")) {
                errors.push(format!("action attempt {} requires an action ID for its outcome", attempt.attempt_id));
            }
            if !needs_action_id && attempt.action_id.is_some() {
                errors.push(format!("action attempt {} must not invent an action ID for a non-action outcome", attempt.attempt_id));
            }
        } else if attempt.action_id.is_some() || attempt.validator_receipt_id.is_some() || attempt.transition_receipt_id.is_some() {
            errors.push(format!("non-action attempt {} cannot declare action/validator/transition IDs", attempt.attempt_id));
        }
        if attempt.cost.as_ref().is_some_and(|value| !valid_cost(value)) {
            errors.push(format!("attempt {} has invalid cost", attempt.attempt_id));
        }
        match attempt.outcome {
            AttemptOutcome::Applied => {
                let Some(action_id) = attempt.action_id.as_deref() else { continue };
                let validator = attempt.validator_receipt_id.as_deref().and_then(|id| validators.get(id));
                let transition = attempt.transition_receipt_id.as_deref().and_then(|id| transitions.get(id));
                if attempt.attempt_kind != AttemptKind::ActionSubmission
                    || validator.is_none_or(|item| item.status != ValidatorStatus::Accepted
                        || item.decision_step != attempt.decision_step
                        || Some(item.proposal_sha256.as_str()) != attempt.proposal_sha256.as_deref()
                        || item.rejected_action_ids.iter().any(|id| id == action_id))
                    || transition.is_none_or(|item| item.status != TransitionStatus::Applied
                        || item.decision_step != attempt.decision_step
                        || Some(item.proposal_sha256.as_str()) != attempt.proposal_sha256.as_deref()
                        || !item.applied_action_ids.iter().any(|id| id == action_id))
                {
                    errors.push(format!("applied action attempt {} lacks matching accepted validator and applied transition receipts", attempt.attempt_id));
                }
            }
            AttemptOutcome::Rejected => {
                let validator = attempt.validator_receipt_id.as_deref().and_then(|id| validators.get(id));
                if attempt.attempt_kind != AttemptKind::ActionSubmission
                    || validator.is_none_or(|item| item.decision_step != attempt.decision_step
                        || item.proposal_sha256 != attempt.proposal_sha256.clone().unwrap_or_default()
                        || !matches!(item.status, ValidatorStatus::Rejected | ValidatorStatus::Inconclusive)
                        || !item.rejected_action_ids.iter().any(|id| Some(id.as_str()) == attempt.action_id.as_deref()))
                    || attempt.transition_receipt_id.is_some()
                {
                    errors.push(format!("rejected action attempt {} lacks a matching non-mutating validator receipt", attempt.attempt_id));
                }
            }
            AttemptOutcome::ExecutionFailed => {
                let action_id = attempt.action_id.as_deref().unwrap_or("");
                let validator = attempt.validator_receipt_id.as_deref().and_then(|id| validators.get(id));
                let transition = attempt.transition_receipt_id.as_deref().and_then(|id| transitions.get(id));
                if attempt.attempt_kind != AttemptKind::ActionSubmission
                    || validator.is_none_or(|item| item.status != ValidatorStatus::Accepted
                        || item.decision_step != attempt.decision_step
                        || Some(item.proposal_sha256.as_str()) != attempt.proposal_sha256.as_deref())
                    || transition.is_none_or(|item| item.status != TransitionStatus::Failed
                        || item.decision_step != attempt.decision_step
                        || Some(item.proposal_sha256.as_str()) != attempt.proposal_sha256.as_deref()
                        || !item.failed_action_ids.iter().any(|id| id == action_id))
                {
                    errors.push(format!("failed execution attempt {} lacks accepted validation and failed transition receipts", attempt.attempt_id));
                }
            }
            _ => {}
        }
    }

    for item in &receipt.validator_receipts {
        if !valid_id(&item.receipt_id)
            || !valid_sha(&item.proposal_sha256, HEX_SHA256)
            || !valid_sha(&item.receipt_sha256, HEX_SHA256)
            || !epochs.get(&item.decision_step).is_some_and(|epoch| epoch.proposal_sha256.as_deref() == Some(item.proposal_sha256.as_str()))
            || receipt.independent_validator != item.validator
            || !item.rejected_action_ids.iter().all(|id| valid_id(id))
            || item.rejected_action_ids.iter().collect::<BTreeSet<_>>().len() != item.rejected_action_ids.len()
            || (item.status == ValidatorStatus::Accepted && !item.rejected_action_ids.is_empty())
            || !receipt.attempts.iter().any(|attempt| attempt.validator_receipt_id.as_deref() == Some(item.receipt_id.as_str()))
        {
            errors.push(format!("validator receipt {} is not bound to a frozen decision proposal", item.receipt_id));
        }
        validate_component(&item.validator, "validator receipt component", errors);
    }
    for item in &receipt.transition_receipts {
        if !valid_id(&item.receipt_id)
            || !valid_sha(&item.proposal_sha256, HEX_SHA256)
            || !valid_sha(&item.receipt_sha256, HEX_SHA256)
            || !valid_sha(&item.state_before_sha256, HEX_SHA256)
            || !valid_sha(&item.state_after_sha256, HEX_SHA256)
            || !epochs.get(&item.decision_step).is_some_and(|epoch| epoch.proposal_sha256.as_deref() == Some(item.proposal_sha256.as_str()))
            || receipt.transition_engine != item.engine
            || item.applied_action_ids.iter().collect::<BTreeSet<_>>().len() != item.applied_action_ids.len()
            || item.failed_action_ids.iter().collect::<BTreeSet<_>>().len() != item.failed_action_ids.len()
            || item.applied_action_ids.iter().any(|id| item.failed_action_ids.contains(id))
            || !receipt.attempts.iter().any(|attempt| attempt.transition_receipt_id.as_deref() == Some(item.receipt_id.as_str()))
        {
            errors.push(format!("transition receipt {} is not bound to a frozen proposal/state", item.receipt_id));
        }
        validate_component(&item.engine, "transition receipt component", errors);
    }

    match (&receipt.trajectory_sha256, &receipt.scoring_receipt) {
        (Some(trajectory), Some(scoring)) => {
            if !valid_sha(trajectory, HEX_SHA256)
                || scoring.trajectory_sha256 != *trajectory
                || scoring.case_sha256 != receipt.case_sha256
                || !valid_id(&scoring.score_definition_ref)
                || !valid_sha(&scoring.output_sha256, HEX_SHA256)
                || !valid_id(&scoring.acceptance_ref)
                || !valid_sha(&scoring.acceptance_sha256, HEX_SHA256)
                || scoring.metrics.is_empty()
            {
                errors.push("scoring receipt is incomplete or not bound to this case/trajectory".to_string());
            }
            let mut metric_ids = BTreeSet::new();
            for metric in &scoring.metrics {
                if !valid_id(&metric.metric_id)
                    || !valid_id(&metric.unit)
                    || !metric.value.is_finite()
                    || !metric_ids.insert(metric.metric_id.as_str())
                    || metric.beneficiary_scope.as_ref().is_some_and(|value| !valid_id(value))
                    || metric.cohort_id.as_ref().is_some_and(|value| !valid_id(value))
                {
                    errors.push("scoring receipt has invalid or duplicate metric output".to_string());
                }
            }
            validate_component(&scoring.scorer, "scoring receipt component", errors);
            if scoring.scorer != receipt.independent_scorer {
                errors.push("scoring receipt scorer identity does not match the declared independent scorer".to_string());
            }
        }
        (None, None) => {}
        _ => errors.push("trajectory digest and scoring receipt must be present together".to_string()),
    }
}

pub fn qualify(envelope: &ReceiptEnvelope) -> Result<ReceiptReport, Vec<String>> {
    let mut errors = Vec::new();
    validate_receipt(&envelope.receipt, &mut errors);
    let actual_digest = canonical_receipt_sha256(&envelope.receipt)
        .map_err(|error| vec![format!("receipt canonicalization failed: {error}")])?;
    if envelope.receipt_sha256.as_ref().is_some_and(|digest| {
        !valid_sha(digest, HEX_SHA256) || digest != &actual_digest
    }) {
        errors.push("receipt SHA-256 does not match canonical run receipt content".to_string());
    }
    if let Some(attestation) = &envelope.collector_attestation {
        if !valid_id(&attestation.signer_ref)
            || !valid_sha(&attestation.signed_receipt_sha256, HEX_SHA256)
            || attestation.signed_receipt_sha256 != actual_digest
        {
            errors.push("collector attestation is invalid or signs a different receipt digest".to_string());
        }
        if attestation.verification_status == AttestationStatus::Verified
            && (!valid_id(attestation.verification_receipt_ref.as_deref().unwrap_or(""))
                || !valid_sha(attestation.verification_receipt_sha256.as_deref().unwrap_or(""), HEX_SHA256))
        {
            errors.push("verified collector attestation requires an external verification receipt and digest".to_string());
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let receipt = &envelope.receipt;
    let attempts = &receipt.attempts;
    let failed_attempt_count = attempts.iter().filter(|attempt| matches!(
        attempt.outcome,
        AttemptOutcome::Malformed | AttemptOutcome::TimedOut | AttemptOutcome::Rejected | AttemptOutcome::ExecutionFailed | AttemptOutcome::Failed
    )).count();
    let applied_action_count = attempts.iter().filter(|attempt| attempt.outcome == AttemptOutcome::Applied).count();
    let rejected_action_count = attempts.iter().filter(|attempt| attempt.outcome == AttemptOutcome::Rejected).count();
    let timed_out_attempt_count = attempts.iter().filter(|attempt| attempt.outcome == AttemptOutcome::TimedOut).count();

    let independent_score_present = receipt.scoring_receipt.as_ref().is_some_and(|scoring| scoring.acceptance_status == ScoringAcceptance::Accepted);
    let score_metrics = receipt.scoring_receipt.as_ref()
        .filter(|scoring| scoring.acceptance_status == ScoringAcceptance::Accepted)
        .map(|scoring| scoring.metrics.clone())
        .unwrap_or_default();

    let result_status = if !receipt.attempt_log_complete {
        "incomplete_attempt_log"
    } else if !matches!(receipt.run_status, RunStatus::Completed | RunStatus::HorizonReached) {
        "run_terminated_without_complete_benchmark_score"
    } else if receipt.scoring_receipt.is_none() {
        "completed_without_independent_score_receipt"
    } else if receipt.scoring_receipt.as_ref().is_some_and(|scoring| scoring.acceptance_status != ScoringAcceptance::Accepted) {
        "independent_score_not_accepted"
    } else if envelope.collector_attestation.as_ref().is_none_or(|attestation| attestation.verification_status != AttestationStatus::Verified) {
        "score_present_collector_attestation_unverified"
    } else {
        "independently_scored_run_receipt"
    };

    Ok(ReceiptReport {
        run_id: receipt.run_id.clone(),
        receipt_sha256: actual_digest,
        run_status: receipt.run_status,
        maturity: receipt.maturity,
        result_status: result_status.to_string(),
        declared_attempt_count: receipt.declared_attempt_count,
        recorded_attempt_count: attempts.len(),
        failed_attempt_count,
        applied_action_count,
        rejected_action_count,
        timed_out_attempt_count,
        independent_score_present,
        score_metrics,
        claim_boundary: "Content-bound benchmark run accounting only; collector trust requires the referenced external attestation, and physical/service claims require their own independent domain receipts".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ReceiptEnvelope {
        serde_json::from_str(include_str!("../fixtures/synthetic-run-receipt-v0.1.json"))
            .expect("fixture must deserialize")
    }

    fn refresh_envelope_digest(envelope: &mut ReceiptEnvelope) {
        envelope.receipt_sha256 = Some(canonical_receipt_sha256(&envelope.receipt).unwrap());
        if let Some(attestation) = &mut envelope.collector_attestation {
            attestation.signed_receipt_sha256 = envelope.receipt_sha256.clone();
        }
    }

    fn verify_attestation(envelope: &mut ReceiptEnvelope) {
        refresh_envelope_digest(envelope);
        envelope.collector_attestation = Some(CollectorAttestation {
            signer_ref: "synthetic://runner/attestor-01".to_string(),
            signed_receipt_sha256: envelope.receipt_sha256.clone().unwrap(),
            verification_status: AttestationStatus::Verified,
            verification_receipt_ref: Some("synthetic://trust/verification/receipt-01".to_string()),
            verification_receipt_sha256: Some("a".repeat(64)),
        });
    }

    #[test]
    fn complete_fixture_has_all_failed_attempts_and_preserves_failed_runs_in_denominator() {
        let report = qualify(&fixture()).expect("fixture structurally valid");
        assert_eq!(report.declared_attempt_count, 5);
        assert_eq!(report.recorded_attempt_count, 5);
        assert_eq!(report.failed_attempt_count, 2);
        assert_eq!(report.applied_action_count, 1);
        assert_eq!(report.rejected_action_count, 1);
        assert_eq!(report.timed_out_attempt_count, 1);
        assert_eq!(report.result_status, "score_present_collector_attestation_unverified");
    }

    #[test]
    fn verified_collector_attestation_can_qualify_the_complete_receipt() {
        let mut envelope = fixture();
        verify_attestation(&mut envelope);
        let report = qualify(&envelope).expect("verified attestation qualifies");
        assert_eq!(report.result_status, "independently_scored_run_receipt");
        assert!(report.independent_score_present);
    }

    #[test]
    fn receipt_tampering_is_rejected() {
        let mut envelope = fixture();
        envelope.receipt.measured_usage.wall_clock_ms += 1;
        assert!(qualify(&envelope).is_err());
    }

    #[test]
    fn signed_digest_must_match_the_receipt_payload() {
        let mut envelope = fixture();
        verify_attestation(&mut envelope);
        envelope.collector_attestation.as_mut().unwrap().signed_receipt_sha256 = "0".repeat(64);
        assert!(qualify(&envelope).is_err());
    }

    #[test]
    fn failed_action_cannot_be_counted_as_applied_without_two_independent_receipts() {
        let mut envelope = fixture();
        let attempt = envelope.receipt.attempts.iter_mut()
            .find(|attempt| attempt.outcome == AttemptOutcome::Rejected).unwrap();
        attempt.outcome = AttemptOutcome::Applied;
        attempt.transition_receipt_id = None;
        refresh_envelope_digest(&mut envelope);
        assert!(qualify(&envelope).is_err());
    }

    #[test]
    fn an_unaccepted_validator_cannot_be_relabelled_as_applied() {
        let mut envelope = fixture();
        envelope.receipt.validator_receipts[1].status = ValidatorStatus::Rejected;
        refresh_envelope_digest(&mut envelope);
        assert!(qualify(&envelope).is_err());
    }

    #[test]
    fn attempt_count_mismatch_is_rejected() {
        let mut envelope = fixture();
        envelope.receipt.declared_attempt_count += 1;
        refresh_envelope_digest(&mut envelope);
        assert!(qualify(&envelope).is_err());
    }

    #[test]
    fn an_explicitly_incomplete_attempt_log_is_not_a_complete_run() {
        let mut envelope = fixture();
        envelope.receipt.attempt_log_complete = false;
        refresh_envelope_digest(&mut envelope);
        assert_eq!(qualify(&envelope).unwrap().result_status, "incomplete_attempt_log");
    }

    #[test]
    fn failed_or_aborted_runs_must_still_have_an_attempt_receipt() {
        let mut envelope = fixture();
        envelope.receipt.run_status = RunStatus::Aborted;
        refresh_envelope_digest(&mut envelope);
        assert_eq!(qualify(&envelope).unwrap().result_status, "run_terminated_without_complete_benchmark_score");
        assert_eq!(qualify(&envelope).unwrap().recorded_attempt_count, 5);
    }

    #[test]
    fn incomplete_or_unaccepted_score_never_produces_metrics() {
        let mut envelope = fixture();
        envelope.receipt.scoring_receipt.as_mut().unwrap().acceptance_status = ScoringAcceptance::Pending;
        refresh_envelope_digest(&mut envelope);
        let report = qualify(&envelope).unwrap();
        assert_eq!(report.result_status, "independent_score_not_accepted");
        assert!(report.score_metrics.is_empty());
    }

    #[test]
    fn case_maturity_is_not_inferred_from_the_model_or_repository() {
        let mut envelope = fixture();
        envelope.receipt.maturity = EvidenceMaturity::SyntheticFixture;
        refresh_envelope_digest(&mut envelope);
        assert_eq!(qualify(&envelope).unwrap().maturity, EvidenceMaturity::SyntheticFixture);
    }

    #[test]
    fn separate_service_units_are_preserved() {
        let mut envelope = fixture();
        let scoring = envelope.receipt.scoring_receipt.as_mut().unwrap();
        scoring.metrics.push(MetricResult {
            metric_id: "potable-water".to_string(),
            unit: "m3".to_string(),
            value: 4.5,
            beneficiary_scope: Some("town_municipality".to_string()),
            cohort_id: Some("water-essential-services".to_string()),
        });
        refresh_envelope_digest(&mut envelope);
        let report = qualify(&envelope).unwrap();
        assert!(report.score_metrics.iter().any(|metric| metric.unit == "MWh"));
        assert!(report.score_metrics.iter().any(|metric| metric.unit == "m3"));
    }

    #[test]
    fn duplicate_metric_ids_are_rejected() {
        let mut envelope = fixture();
        let duplicate = envelope.receipt.scoring_receipt.as_ref().unwrap().metrics[0].clone();
        envelope.receipt.scoring_receipt.as_mut().unwrap().metrics.push(duplicate);
        refresh_envelope_digest(&mut envelope);
        assert!(qualify(&envelope).is_err());
    }

    #[test]
    fn deterministic_receipt_digest_and_report() {
        let envelope = fixture();
        let first = qualify(&envelope).unwrap();
        let second = qualify(&envelope).unwrap();
        assert_eq!(first.receipt_sha256, second.receipt_sha256);
        assert_eq!(serde_json::to_string(&first).unwrap(), serde_json::to_string(&second).unwrap());
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut value = serde_json::to_value(fixture()).unwrap();
        value["receipt"]["authorized"] = serde_json::Value::Bool(true);
        assert!(serde_json::from_value::<ReceiptEnvelope>(value).is_err());
    }
}
