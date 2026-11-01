//! Matched benchmark admission and per-family comparison summaries.
//!
//! This module prevents invalid planner/agent comparisons caused by changing
//! task, model, prompt, harness, toolset, scope, split or resource budgets
//! between the reference and candidate arms.

use super::counterexample_ledger::EvidenceSplit;
use std::collections::{BTreeMap, BTreeSet};

const MAX_ID_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum BenchmarkScope {
    PlannerOnly,
    ModelInference,
    PhysicalExecution,
    FullScientificCampaign,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BenchmarkTaskValidity {
    Valid,
    Invalid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceBudget {
    pub max_wall_time_ms: u64,
    pub max_model_tokens: u64,
    pub max_tool_calls: u64,
    pub max_simulator_steps: u64,
    pub max_energy_millijoules: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResourceUsage {
    pub wall_time_ms: u64,
    pub model_tokens: u64,
    pub tool_calls: u64,
    pub simulator_steps: u64,
    pub energy_millijoules: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchedBenchmarkManifest {
    pub campaign_id: String,
    pub protocol_version: String,
    pub scope: BenchmarkScope,
    pub split: EvidenceSplit,
    pub group_id: String,
    pub agent_panel_id: String,
    pub task_manifest_digest: String,
    pub model_configuration_digest: String,
    pub prompt_configuration_digest: String,
    pub harness_digest: String,
    pub toolset_digest: String,
    pub reference_arm_id: String,
    pub candidate_arm_id: String,
    pub resource_budget: ResourceBudget,
    pub minimum_pairs_per_family: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArmOutcome {
    pub arm_id: String,
    pub system_version: String,
    pub task_completed: bool,
    pub safe_success: bool,
    pub hazard_observed: bool,
    pub abstained: bool,
    /// Exact integer units from the pre-registered loss table.
    pub terminal_loss_units: u64,
    pub usage: ResourceUsage,
    pub evidence_artifact_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairedBenchmarkTrial {
    pub pair_id: String,
    pub task_id: String,
    pub mechanism_family_id: String,
    pub paired_seed: u64,
    pub task_validity: BenchmarkTaskValidity,
    pub task_oracle_receipt_digest: String,
    pub task_manifest_digest: String,
    pub model_configuration_digest: String,
    pub prompt_configuration_digest: String,
    pub harness_digest: String,
    pub toolset_digest: String,
    pub scope: BenchmarkScope,
    pub split: EvidenceSplit,
    pub group_id: String,
    pub agent_panel_id: String,
    pub resource_budget: ResourceBudget,
    pub reference: Option<ArmOutcome>,
    pub candidate: Option<ArmOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyComparisonSummary {
    pub mechanism_family_id: String,
    pub valid_pairs: usize,
    pub invalid_tasks_excluded: usize,
    pub candidate_lower_loss_count: usize,
    pub equal_loss_count: usize,
    pub candidate_higher_loss_count: usize,
    /// Sum(candidate terminal loss - reference terminal loss); positive is worse.
    /// Divide by valid_pairs for the exact mean loss delta.
    pub candidate_minus_reference_loss_sum: i128,
    pub reference_completed_count: usize,
    pub candidate_completed_count: usize,
    pub reference_safe_success_count: usize,
    pub candidate_safe_success_count: usize,
    pub reference_hazard_count: usize,
    pub candidate_hazard_count: usize,
    pub reference_abstention_count: usize,
    pub candidate_abstention_count: usize,
    pub meets_minimum_pair_count: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchedBenchmarkReceipt {
    pub campaign_id: String,
    pub scope: BenchmarkScope,
    pub split: EvidenceSplit,
    pub group_id: String,
    pub reference_arm_id: String,
    pub candidate_arm_id: String,
    pub valid_pair_count: usize,
    pub invalid_task_count: usize,
    pub family_summaries: Vec<FamilyComparisonSummary>,
    pub harness_and_budget_matched: bool,
    pub no_cross_scope_or_split_pooling: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchedBenchmarkError {
    InvalidManifest(String),
    PairIdentity(String),
    MismatchedEvaluationConditions(String),
    InvalidTaskOutcome(String),
    MissingArmOutcome(String),
    ResourceBudgetExceeded(String),
    InvalidArmOutcome(String),
    DuplicateEvidence(String),
    ArithmeticOverflow(String),
}

impl std::fmt::Display for MatchedBenchmarkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidManifest(reason)
            | Self::PairIdentity(reason)
            | Self::MismatchedEvaluationConditions(reason)
            | Self::InvalidTaskOutcome(reason)
            | Self::MissingArmOutcome(reason)
            | Self::ResourceBudgetExceeded(reason)
            | Self::InvalidArmOutcome(reason)
            | Self::DuplicateEvidence(reason)
            | Self::ArithmeticOverflow(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for MatchedBenchmarkError {}

#[derive(Clone, Debug, Default)]
struct FamilyAccumulator {
    valid_pairs: usize,
    invalid_tasks_excluded: usize,
    candidate_lower_loss_count: usize,
    equal_loss_count: usize,
    candidate_higher_loss_count: usize,
    candidate_minus_reference_loss_sum: i128,
    reference_completed_count: usize,
    candidate_completed_count: usize,
    reference_safe_success_count: usize,
    candidate_safe_success_count: usize,
    reference_hazard_count: usize,
    candidate_hazard_count: usize,
    reference_abstention_count: usize,
    candidate_abstention_count: usize,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn count_flag(value: bool) -> usize {
    if value { 1 } else { 0 }
}

fn validate_manifest(manifest: &MatchedBenchmarkManifest) -> Result<(), MatchedBenchmarkError> {
    for (value, label) in [
        (&manifest.campaign_id, "campaign ID"),
        (&manifest.protocol_version, "protocol version"),
        (&manifest.group_id, "evidence group ID"),
        (&manifest.agent_panel_id, "agent panel ID"),
        (&manifest.task_manifest_digest, "task manifest digest"),
        (&manifest.model_configuration_digest, "model configuration digest"),
        (&manifest.prompt_configuration_digest, "prompt configuration digest"),
        (&manifest.harness_digest, "harness digest"),
        (&manifest.toolset_digest, "toolset digest"),
        (&manifest.reference_arm_id, "reference arm ID"),
        (&manifest.candidate_arm_id, "candidate arm ID"),
    ] {
        if !valid_id(value) {
            return Err(MatchedBenchmarkError::InvalidManifest(format!(
                "{label} is empty or overlong"
            )));
        }
    }
    if manifest.reference_arm_id == manifest.candidate_arm_id {
        return Err(MatchedBenchmarkError::InvalidManifest(
            "reference and candidate arm IDs must differ".into(),
        ));
    }
    if manifest.minimum_pairs_per_family == 0 {
        return Err(MatchedBenchmarkError::InvalidManifest(
            "minimum pairs per family must be positive".into(),
        ));
    }
    Ok(())
}

fn validate_usage(
    usage: &ResourceUsage,
    budget: &ResourceBudget,
    arm_label: &str,
) -> Result<(), MatchedBenchmarkError> {
    if usage.wall_time_ms > budget.max_wall_time_ms
        || usage.model_tokens > budget.max_model_tokens
        || usage.tool_calls > budget.max_tool_calls
        || usage.simulator_steps > budget.max_simulator_steps
        || usage.energy_millijoules > budget.max_energy_millijoules
    {
        return Err(MatchedBenchmarkError::ResourceBudgetExceeded(format!(
            "{arm_label} arm exceeded the shared campaign resource budget"
        )));
    }
    Ok(())
}

fn validate_arm(
    arm: &ArmOutcome,
    expected_arm_id: &str,
    budget: &ResourceBudget,
    label: &str,
) -> Result<(), MatchedBenchmarkError> {
    if arm.arm_id != expected_arm_id
        || !valid_id(&arm.system_version)
        || !valid_id(&arm.evidence_artifact_digest)
    {
        return Err(MatchedBenchmarkError::InvalidArmOutcome(format!(
            "{label} arm identity, system version or evidence digest is invalid"
        )));
    }
    if arm.safe_success && (!arm.task_completed || arm.hazard_observed || arm.abstained) {
        return Err(MatchedBenchmarkError::InvalidArmOutcome(format!(
            "{label} arm marks safe success despite non-completion, hazard or abstention"
        )));
    }
    validate_usage(&arm.usage, budget, label)
}

fn check_pair_conditions(
    manifest: &MatchedBenchmarkManifest,
    trial: &PairedBenchmarkTrial,
) -> Result<(), MatchedBenchmarkError> {
    if trial.task_manifest_digest != manifest.task_manifest_digest
        || trial.model_configuration_digest != manifest.model_configuration_digest
        || trial.prompt_configuration_digest != manifest.prompt_configuration_digest
        || trial.harness_digest != manifest.harness_digest
        || trial.toolset_digest != manifest.toolset_digest
        || trial.scope != manifest.scope
        || trial.split != manifest.split
        || trial.group_id != manifest.group_id
        || trial.agent_panel_id != manifest.agent_panel_id
        || trial.resource_budget != manifest.resource_budget
    {
        return Err(MatchedBenchmarkError::MismatchedEvaluationConditions(
            "paired task differs from the campaign's model, prompt, harness, toolset, scope, split or resource budget".into(),
        ));
    }
    Ok(())
}

/// Validate matched candidate/reference trials and summarize each mechanism
/// family separately. One manifest represents exactly one evaluation scope
/// and one evidence split; callers must not pool planner-only and campaign
/// results or Development and SealedHoldout results in one report.
pub fn validate_and_summarize_matched_campaign(
    manifest: &MatchedBenchmarkManifest,
    trials: &[PairedBenchmarkTrial],
) -> Result<MatchedBenchmarkReceipt, MatchedBenchmarkError> {
    validate_manifest(manifest)?;
    if trials.is_empty() {
        return Err(MatchedBenchmarkError::InvalidManifest(
            "matched campaign contains no paired tasks".into(),
        ));
    }

    let mut pair_ids = BTreeSet::new();
    let mut paired_task_seeds = BTreeSet::new();
    let mut evidence_digests = BTreeSet::new();
    let mut families = BTreeMap::<String, FamilyAccumulator>::new();

    for trial in trials {
        for (value, label) in [
            (&trial.pair_id, "pair ID"),
            (&trial.task_id, "task ID"),
            (&trial.mechanism_family_id, "mechanism family ID"),
            (&trial.task_oracle_receipt_digest, "task oracle receipt digest"),
        ] {
            if !valid_id(value) {
                return Err(MatchedBenchmarkError::PairIdentity(format!(
                    "{label} is empty or overlong"
                )));
            }
        }
        if !pair_ids.insert(trial.pair_id.clone()) {
            return Err(MatchedBenchmarkError::PairIdentity(
                "paired trial ID is duplicated".into(),
            ));
        }
        if !paired_task_seeds.insert((
            trial.task_id.clone(),
            trial.mechanism_family_id.clone(),
            trial.paired_seed,
        )) {
            return Err(MatchedBenchmarkError::PairIdentity(
                "same task/family/seed appears more than once in the campaign".into(),
            ));
        }
        check_pair_conditions(manifest, trial)?;

        let accumulator = families
            .entry(trial.mechanism_family_id.clone())
            .or_default();

        match trial.task_validity {
            BenchmarkTaskValidity::Invalid => {
                if trial.reference.is_some() || trial.candidate.is_some() {
                    return Err(MatchedBenchmarkError::InvalidTaskOutcome(
                        "invalid task must not contain solver outcomes".into(),
                    ));
                }
                accumulator.invalid_tasks_excluded = accumulator
                    .invalid_tasks_excluded
                    .checked_add(1)
                    .ok_or_else(|| MatchedBenchmarkError::ArithmeticOverflow(
                        "invalid task count overflow".into(),
                    ))?;
                continue;
            }
            BenchmarkTaskValidity::Valid => {}
        }

        let reference = trial.reference.as_ref().ok_or_else(|| {
            MatchedBenchmarkError::MissingArmOutcome(
                "valid paired task is missing its reference arm outcome".into(),
            )
        })?;
        let candidate = trial.candidate.as_ref().ok_or_else(|| {
            MatchedBenchmarkError::MissingArmOutcome(
                "valid paired task is missing its candidate arm outcome".into(),
            )
        })?;

        validate_arm(
            reference,
            &manifest.reference_arm_id,
            &manifest.resource_budget,
            "reference",
        )?;
        validate_arm(
            candidate,
            &manifest.candidate_arm_id,
            &manifest.resource_budget,
            "candidate",
        )?;

        for digest in [
            reference.evidence_artifact_digest.as_str(),
            candidate.evidence_artifact_digest.as_str(),
        ] {
            if !evidence_digests.insert(digest.to_string()) {
                return Err(MatchedBenchmarkError::DuplicateEvidence(
                    "evidence artifact digest is reused across arms or trials".into(),
                ));
            }
        }
        if reference.evidence_artifact_digest == candidate.evidence_artifact_digest {
            return Err(MatchedBenchmarkError::DuplicateEvidence(
                "reference and candidate arms share one evidence artifact".into(),
            ));
        }

        let candidate_delta = i128::from(candidate.terminal_loss_units)
            - i128::from(reference.terminal_loss_units);
        accumulator.candidate_minus_reference_loss_sum = accumulator
            .candidate_minus_reference_loss_sum
            .checked_add(candidate_delta)
            .ok_or_else(|| MatchedBenchmarkError::ArithmeticOverflow(
                "paired terminal-loss sum overflow".into(),
            ))?;
        match candidate_delta.cmp(&0) {
            std::cmp::Ordering::Less => {
                accumulator.candidate_lower_loss_count += 1;
            }
            std::cmp::Ordering::Equal => {
                accumulator.equal_loss_count += 1;
            }
            std::cmp::Ordering::Greater => {
                accumulator.candidate_higher_loss_count += 1;
            }
        }

        accumulator.valid_pairs += 1;
        accumulator.reference_completed_count += count_flag(reference.task_completed);
        accumulator.candidate_completed_count += count_flag(candidate.task_completed);
        accumulator.reference_safe_success_count += count_flag(reference.safe_success);
        accumulator.candidate_safe_success_count += count_flag(candidate.safe_success);
        accumulator.reference_hazard_count += count_flag(reference.hazard_observed);
        accumulator.candidate_hazard_count += count_flag(candidate.hazard_observed);
        accumulator.reference_abstention_count += count_flag(reference.abstained);
        accumulator.candidate_abstention_count += count_flag(candidate.abstained);
    }

    let mut family_summaries = Vec::with_capacity(families.len());
    let mut valid_pair_count = 0usize;
    let mut invalid_task_count = 0usize;
    for (mechanism_family_id, accumulator) in families {
        valid_pair_count += accumulator.valid_pairs;
        invalid_task_count += accumulator.invalid_tasks_excluded;
        family_summaries.push(FamilyComparisonSummary {
            mechanism_family_id,
            valid_pairs: accumulator.valid_pairs,
            invalid_tasks_excluded: accumulator.invalid_tasks_excluded,
            candidate_lower_loss_count: accumulator.candidate_lower_loss_count,
            equal_loss_count: accumulator.equal_loss_count,
            candidate_higher_loss_count: accumulator.candidate_higher_loss_count,
            candidate_minus_reference_loss_sum: accumulator.candidate_minus_reference_loss_sum,
            reference_completed_count: accumulator.reference_completed_count,
            candidate_completed_count: accumulator.candidate_completed_count,
            reference_safe_success_count: accumulator.reference_safe_success_count,
            candidate_safe_success_count: accumulator.candidate_safe_success_count,
            reference_hazard_count: accumulator.reference_hazard_count,
            candidate_hazard_count: accumulator.candidate_hazard_count,
            reference_abstention_count: accumulator.reference_abstention_count,
            candidate_abstention_count: accumulator.candidate_abstention_count,
            meets_minimum_pair_count: accumulator.valid_pairs
                >= manifest.minimum_pairs_per_family,
        });
    }

    Ok(MatchedBenchmarkReceipt {
        campaign_id: manifest.campaign_id.clone(),
        scope: manifest.scope,
        split: manifest.split,
        group_id: manifest.group_id.clone(),
        reference_arm_id: manifest.reference_arm_id.clone(),
        candidate_arm_id: manifest.candidate_arm_id.clone(),
        valid_pair_count,
        invalid_task_count,
        family_summaries,
        harness_and_budget_matched: true,
        no_cross_scope_or_split_pooling: true,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        ArmOutcome, BenchmarkScope, MatchedBenchmarkManifest, PairedBenchmarkTrial,
        ResourceBudget, ResourceUsage, BenchmarkTaskValidity,
        validate_and_summarize_matched_campaign,
    };
    use super::super::counterexample_ledger::EvidenceSplit;

    fn budget() -> ResourceBudget {
        ResourceBudget {
            max_wall_time_ms: 10_000,
            max_model_tokens: 20_000,
            max_tool_calls: 100,
            max_simulator_steps: 1_000,
            max_energy_millijoules: 50_000,
        }
    }

    fn manifest() -> MatchedBenchmarkManifest {
        MatchedBenchmarkManifest {
            campaign_id: "campaign-1".into(),
            protocol_version: "v1".into(),
            scope: BenchmarkScope::PlannerOnly,
            split: EvidenceSplit::Development,
            group_id: "dev-group-1".into(),
            agent_panel_id: "panel-fixed-v1".into(),
            task_manifest_digest: "task-manifest-sha256".into(),
            model_configuration_digest: "model-config-sha256".into(),
            prompt_configuration_digest: "prompt-config-sha256".into(),
            harness_digest: "harness-sha256".into(),
            toolset_digest: "toolset-sha256".into(),
            reference_arm_id: "baseline-eig".into(),
            candidate_arm_id: "symtropy-decision-aware".into(),
            resource_budget: budget(),
            minimum_pairs_per_family: 2,
        }
    }

    fn arm(
        arm_id: &str,
        terminal_loss_units: u64,
        evidence: &str,
        safe_success: bool,
    ) -> ArmOutcome {
        ArmOutcome {
            arm_id: arm_id.into(),
            system_version: format!("{arm_id}-v1"),
            task_completed: safe_success,
            safe_success,
            hazard_observed: false,
            abstained: false,
            terminal_loss_units,
            usage: ResourceUsage {
                wall_time_ms: 1_000,
                model_tokens: 2_000,
                tool_calls: 10,
                simulator_steps: 40,
                energy_millijoules: 3_000,
            },
            evidence_artifact_digest: evidence.into(),
        }
    }

    fn trial(pair: &str, task: &str, seed: u64) -> PairedBenchmarkTrial {
        PairedBenchmarkTrial {
            pair_id: pair.into(),
            task_id: task.into(),
            mechanism_family_id: "sensor-drift".into(),
            paired_seed: seed,
            task_validity: BenchmarkTaskValidity::Valid,
            task_oracle_receipt_digest: format!("oracle-{task}-{seed}"),
            task_manifest_digest: "task-manifest-sha256".into(),
            model_configuration_digest: "model-config-sha256".into(),
            prompt_configuration_digest: "prompt-config-sha256".into(),
            harness_digest: "harness-sha256".into(),
            toolset_digest: "toolset-sha256".into(),
            scope: BenchmarkScope::PlannerOnly,
            split: EvidenceSplit::Development,
            group_id: "dev-group-1".into(),
            agent_panel_id: "panel-fixed-v1".into(),
            resource_budget: budget(),
            reference: Some(arm("baseline-eig", 10, &format!("ref-{pair}"), true)),
            candidate: Some(arm(
                "symtropy-decision-aware",
                8,
                &format!("cand-{pair}"),
                true,
            )),
        }
    }

    #[test]
    fn accepts_matched_trials_and_reports_per_family_without_composite_score() {
        let receipt = validate_and_summarize_matched_campaign(
            &manifest(),
            &[trial("pair-1", "task-1", 11), trial("pair-2", "task-2", 12)],
        )
        .unwrap();
        assert_eq!(receipt.valid_pair_count, 2);
        assert!(receipt.harness_and_budget_matched);
        assert!(receipt.no_cross_scope_or_split_pooling);
        assert_eq!(receipt.family_summaries.len(), 1);
        let family = &receipt.family_summaries[0];
        assert_eq!(family.candidate_lower_loss_count, 2);
        assert_eq!(family.candidate_minus_reference_loss_sum, -4);
        assert!(family.meets_minimum_pair_count);
    }

    #[test]
    fn rejects_model_harness_toolset_scope_split_and_budget_mismatch() {
        let mut mismatched = trial("pair-1", "task-1", 1);
        mismatched.harness_digest = "different-harness".into();
        assert!(validate_and_summarize_matched_campaign(&manifest(), &[mismatched]).is_err());

        mismatched = trial("pair-1", "task-1", 1);
        mismatched.model_configuration_digest = "different-model".into();
        assert!(validate_and_summarize_matched_campaign(&manifest(), &[mismatched]).is_err());

        mismatched = trial("pair-1", "task-1", 1);
        mismatched.resource_budget.max_tool_calls += 1;
        assert!(validate_and_summarize_matched_campaign(&manifest(), &[mismatched]).is_err());

        mismatched = trial("pair-1", "task-1", 1);
        mismatched.scope = BenchmarkScope::FullScientificCampaign;
        assert!(validate_and_summarize_matched_campaign(&manifest(), &[mismatched]).is_err());

        mismatched = trial("pair-1", "task-1", 1);
        mismatched.split = EvidenceSplit::SealedHoldout;
        assert!(validate_and_summarize_matched_campaign(&manifest(), &[mismatched]).is_err());
    }

    #[test]
    fn rejects_over_budget_arm_even_when_the_other_arm_is_under_budget() {
        let mut over = trial("pair-1", "task-1", 1);
        over.candidate.as_mut().unwrap().usage.tool_calls = 101;
        assert!(validate_and_summarize_matched_campaign(&manifest(), &[over]).is_err());
    }

    #[test]
    fn invalid_tasks_are_excluded_and_cannot_contain_arm_outcomes() {
        let mut invalid = trial("pair-1", "task-1", 1);
        invalid.task_validity = BenchmarkTaskValidity::Invalid;
        invalid.reference = None;
        invalid.candidate = None;
        let receipt = validate_and_summarize_matched_campaign(&manifest(), &[invalid]).unwrap();
        assert_eq!(receipt.valid_pair_count, 0);
        assert_eq!(receipt.invalid_task_count, 1);
        assert_eq!(receipt.family_summaries[0].valid_pairs, 0);
    }

    #[test]
    fn rejects_duplicate_pair_task_seed_and_shared_evidence_digests() {
        let duplicate = trial("pair-1", "task-1", 11);
        let mut same = duplicate.clone();
        same.pair_id = "pair-2".into();
        assert!(validate_and_summarize_matched_campaign(
            &manifest(),
            &[duplicate, same]
        )
        .is_err());

        let mut reused = trial("pair-2", "task-2", 12);
        reused.reference.as_mut().unwrap().evidence_artifact_digest = "ref-pair-1".into();
        reused.candidate.as_mut().unwrap().evidence_artifact_digest = "cand-pair-1".into();
        let first = trial("pair-1", "task-1", 11);
        assert!(validate_and_summarize_matched_campaign(&manifest(), &[first, reused]).is_err());
    }

    #[test]
    fn safe_success_cannot_coexist_with_hazard_or_abstention() {
        let mut invalid = trial("pair-1", "task-1", 1);
        invalid.candidate.as_mut().unwrap().hazard_observed = true;
        assert!(validate_and_summarize_matched_campaign(&manifest(), &[invalid]).is_err());
    }
}
