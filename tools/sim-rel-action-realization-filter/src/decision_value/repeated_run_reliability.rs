//! Repeated-run scientific benchmark reliability with task-clustered metrics.
//!
//! Repetitions of one task are not independent scientific task samples. This
//! module preserves the task cluster as the generalization unit and reports
//! pass@k separately from pass^k (success on every repetition).

use super::counterexample_ledger::EvidenceSplit;
use super::matched_benchmark::{
    BenchmarkScope, BenchmarkTaskValidity, MatchedBenchmarkManifest, PairedBenchmarkTrial,
    ResourceBudget, validate_and_summarize_matched_campaign,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_ID_BYTES: usize = 256;
const MAX_FAMILIES: usize = 128;
const MAX_REPETITIONS_PER_TASK: usize = 64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepeatedRunReliabilityPlan {
    pub analysis_id: String,
    pub analysis_version: String,
    pub campaign_id: String,
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
    pub repetitions_per_task: usize,
    pub minimum_valid_task_clusters_per_family: usize,
    pub mechanism_families: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArmTaskReliability {
    pub safe_success_count: usize,
    pub pass_at_k: bool,
    pub pass_every_k: bool,
    pub completion_count: usize,
    pub completed_at_least_once: bool,
    pub completed_every_run: bool,
    pub hazard_count: usize,
    pub any_hazard: bool,
    pub abstention_count: usize,
    pub any_abstention: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskClusterReliability {
    pub mechanism_family_id: String,
    pub task_id: String,
    pub repetition_count: usize,
    pub invalid_task_cluster: bool,
    pub reference: Option<ArmTaskReliability>,
    pub candidate: Option<ArmTaskReliability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRunRate {
    pub numerator: usize,
    pub denominator: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FamilyRepeatedRunSummary {
    pub mechanism_family_id: String,
    pub valid_task_clusters: usize,
    pub invalid_task_clusters_excluded: usize,
    pub required_valid_task_clusters: usize,
    pub meets_minimum_valid_task_clusters: bool,
    pub candidate_pass_at_k_tasks: usize,
    pub reference_pass_at_k_tasks: usize,
    pub candidate_pass_at_k_rate: Option<ExactRunRate>,
    pub reference_pass_at_k_rate: Option<ExactRunRate>,
    pub candidate_pass_every_k_tasks: usize,
    pub reference_pass_every_k_tasks: usize,
    pub candidate_pass_every_k_rate: Option<ExactRunRate>,
    pub reference_pass_every_k_rate: Option<ExactRunRate>,
    pub candidate_run_safe_successes: usize,
    pub reference_run_safe_successes: usize,
    pub total_valid_run_pairs: usize,
    /// Exact rate among repeated runs within valid task clusters. None means
    /// the family had no valid task cluster; it must not be treated as zero.
    pub candidate_run_safe_success_rate: Option<ExactRunRate>,
    pub reference_run_safe_success_rate: Option<ExactRunRate>,
    pub candidate_any_hazard_tasks: usize,
    pub reference_any_hazard_tasks: usize,
    pub candidate_any_abstention_tasks: usize,
    pub reference_any_abstention_tasks: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepeatedRunReliabilityReceipt {
    pub analysis_id: String,
    pub analysis_version: String,
    pub campaign_id: String,
    pub scope: BenchmarkScope,
    pub split: EvidenceSplit,
    pub group_id: String,
    pub reference_arm_id: String,
    pub candidate_arm_id: String,
    pub repetitions_per_task: usize,
    pub independent_generalization_unit: String,
    pub task_cluster_count: usize,
    pub valid_task_cluster_count: usize,
    pub invalid_task_cluster_count: usize,
    pub family_summaries: Vec<FamilyRepeatedRunSummary>,
    pub task_clusters: Vec<TaskClusterReliability>,
    pub pass_at_k_is_not_pass_every_k: bool,
    pub repeated_runs_not_counted_as_independent_task_samples: bool,
    pub exact_manifest_and_resource_matching: bool,
    pub sealed_holdout_only: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepeatedRunError {
    InvalidPlan(String),
    ManifestMismatch(String),
    UnregisteredFamily(String),
    IncompleteCluster(String),
    MixedTaskValidity(String),
    InvalidTrial(String),
    ArithmeticOverflow(String),
}

impl std::fmt::Display for RepeatedRunError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPlan(reason)
            | Self::ManifestMismatch(reason)
            | Self::UnregisteredFamily(reason)
            | Self::IncompleteCluster(reason)
            | Self::MixedTaskValidity(reason)
            | Self::InvalidTrial(reason)
            | Self::ArithmeticOverflow(reason) => formatter.write_str(reason),
        }
    }
}

impl std::error::Error for RepeatedRunError {}

#[derive(Clone, Debug, Default)]
struct FamilyCounts {
    valid_clusters: usize,
    invalid_clusters: usize,
    candidate_pass_at_k: usize,
    reference_pass_at_k: usize,
    candidate_pass_every_k: usize,
    reference_pass_every_k: usize,
    candidate_safe_successes: usize,
    reference_safe_successes: usize,
    candidate_any_hazard: usize,
    reference_any_hazard: usize,
    candidate_any_abstention: usize,
    reference_any_abstention: usize,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ID_BYTES
}

fn count_flag(value: bool) -> usize {
    if value { 1 } else { 0 }
}

fn exact_run_rate(successes: usize, total: usize) -> Option<ExactRunRate> {
    (total > 0).then_some(ExactRunRate {
        numerator: successes,
        denominator: total,
    })
}

fn increment(value: &mut usize, label: &str) -> Result<(), RepeatedRunError> {
    *value = value
        .checked_add(1)
        .ok_or_else(|| RepeatedRunError::ArithmeticOverflow(format!("{label} count overflow")))?;
    Ok(())
}

fn increment_if(value: &mut usize, condition: bool, label: &str) -> Result<(), RepeatedRunError> {
    if condition {
        increment(value, label)?;
    }
    Ok(())
}

fn validate_plan(
    plan: &RepeatedRunReliabilityPlan,
    manifest: &MatchedBenchmarkManifest,
) -> Result<(), RepeatedRunError> {
    for (value, label) in [
        (&plan.analysis_id, "analysis ID"),
        (&plan.analysis_version, "analysis version"),
        (&plan.campaign_id, "campaign ID"),
        (&plan.group_id, "evidence group ID"),
        (&plan.agent_panel_id, "agent panel ID"),
        (&plan.task_manifest_digest, "task manifest digest"),
        (
            &plan.model_configuration_digest,
            "model configuration digest",
        ),
        (
            &plan.prompt_configuration_digest,
            "prompt configuration digest",
        ),
        (&plan.harness_digest, "harness digest"),
        (&plan.toolset_digest, "toolset digest"),
        (&plan.reference_arm_id, "reference arm ID"),
        (&plan.candidate_arm_id, "candidate arm ID"),
    ] {
        if !valid_id(value) {
            return Err(RepeatedRunError::InvalidPlan(format!(
                "{label} is empty or overlong"
            )));
        }
    }
    if plan.reference_arm_id == plan.candidate_arm_id {
        return Err(RepeatedRunError::InvalidPlan(
            "reference and candidate arm IDs must differ".into(),
        ));
    }
    if plan.split != EvidenceSplit::SealedHoldout || manifest.split != EvidenceSplit::SealedHoldout
    {
        return Err(RepeatedRunError::ManifestMismatch(
            "final repeated-run reliability claims accept SealedHoldout only".into(),
        ));
    }
    if !(2..=MAX_REPETITIONS_PER_TASK).contains(&plan.repetitions_per_task) {
        return Err(RepeatedRunError::InvalidPlan(format!(
            "repetitions per task must be in 2..={MAX_REPETITIONS_PER_TASK}"
        )));
    }
    if plan.minimum_valid_task_clusters_per_family == 0 {
        return Err(RepeatedRunError::InvalidPlan(
            "minimum valid task clusters per family must be positive".into(),
        ));
    }
    if plan.mechanism_families.is_empty() || plan.mechanism_families.len() > MAX_FAMILIES {
        return Err(RepeatedRunError::InvalidPlan(format!(
            "mechanism-family registry must contain 1..={MAX_FAMILIES} entries"
        )));
    }
    let mut unique = BTreeSet::new();
    for family in &plan.mechanism_families {
        if !valid_id(family) || !unique.insert(family.clone()) {
            return Err(RepeatedRunError::InvalidPlan(
                "mechanism-family registry has an invalid or duplicate identity".into(),
            ));
        }
    }

    if plan.campaign_id != manifest.campaign_id
        || plan.scope != manifest.scope
        || plan.split != manifest.split
        || plan.group_id != manifest.group_id
        || plan.agent_panel_id != manifest.agent_panel_id
        || plan.task_manifest_digest != manifest.task_manifest_digest
        || plan.model_configuration_digest != manifest.model_configuration_digest
        || plan.prompt_configuration_digest != manifest.prompt_configuration_digest
        || plan.harness_digest != manifest.harness_digest
        || plan.toolset_digest != manifest.toolset_digest
        || plan.reference_arm_id != manifest.reference_arm_id
        || plan.candidate_arm_id != manifest.candidate_arm_id
        || plan.resource_budget != manifest.resource_budget
    {
        return Err(RepeatedRunError::ManifestMismatch(
            "reliability plan differs from the matched campaign manifest".into(),
        ));
    }
    Ok(())
}

fn arm_reliability(
    trials: &[&PairedBenchmarkTrial],
    candidate_arm: bool,
) -> Result<ArmTaskReliability, RepeatedRunError> {
    let mut safe_success_count = 0usize;
    let mut completion_count = 0usize;
    let mut hazard_count = 0usize;
    let mut abstention_count = 0usize;
    for trial in trials {
        let arm = if candidate_arm {
            trial.candidate.as_ref()
        } else {
            trial.reference.as_ref()
        }
        .ok_or_else(|| {
            RepeatedRunError::InvalidTrial(
                "valid repeated-run cluster is missing one paired arm result".into(),
            )
        })?;
        safe_success_count = safe_success_count
            .checked_add(count_flag(arm.safe_success))
            .ok_or_else(|| {
                RepeatedRunError::ArithmeticOverflow("safe-success count overflow".into())
            })?;
        completion_count = completion_count
            .checked_add(count_flag(arm.task_completed))
            .ok_or_else(|| {
                RepeatedRunError::ArithmeticOverflow("completion count overflow".into())
            })?;
        hazard_count = hazard_count
            .checked_add(count_flag(arm.hazard_observed))
            .ok_or_else(|| RepeatedRunError::ArithmeticOverflow("hazard count overflow".into()))?;
        abstention_count = abstention_count
            .checked_add(count_flag(arm.abstained))
            .ok_or_else(|| {
                RepeatedRunError::ArithmeticOverflow("abstention count overflow".into())
            })?;
    }
    let repetitions = trials.len();
    Ok(ArmTaskReliability {
        safe_success_count,
        pass_at_k: safe_success_count > 0,
        pass_every_k: safe_success_count == repetitions,
        completion_count,
        completed_at_least_once: completion_count > 0,
        completed_every_run: completion_count == repetitions,
        hazard_count,
        any_hazard: hazard_count > 0,
        abstention_count,
        any_abstention: abstention_count > 0,
    })
}

/// Aggregate repeated paired runs by distinct task/family cluster.
///
/// Every valid task/family cluster must contain exactly k matched reference
/// and candidate repetitions. A task cluster is one generalization unit;
/// repetitions only estimate that task's within-instance reliability.
pub fn summarize_repeated_run_reliability(
    plan: &RepeatedRunReliabilityPlan,
    manifest: &MatchedBenchmarkManifest,
    trials: &[PairedBenchmarkTrial],
) -> Result<RepeatedRunReliabilityReceipt, RepeatedRunError> {
    validate_plan(plan, manifest)?;
    if trials.is_empty() {
        return Err(RepeatedRunError::InvalidPlan(
            "repeated-run campaign contains no trials".into(),
        ));
    }
    validate_and_summarize_matched_campaign(manifest, trials)
        .map_err(|error| RepeatedRunError::InvalidTrial(error.to_string()))?;

    let registered_families = plan
        .mechanism_families
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut clusters = BTreeMap::<(String, String), Vec<&PairedBenchmarkTrial>>::new();
    for trial in trials {
        if !registered_families.contains(&trial.mechanism_family_id) {
            return Err(RepeatedRunError::UnregisteredFamily(format!(
                "task {} names unregistered family {}",
                trial.task_id, trial.mechanism_family_id
            )));
        }
        clusters
            .entry((trial.mechanism_family_id.clone(), trial.task_id.clone()))
            .or_default()
            .push(trial);
    }

    let mut family_counts = plan
        .mechanism_families
        .iter()
        .cloned()
        .map(|family| (family, FamilyCounts::default()))
        .collect::<BTreeMap<_, _>>();
    let mut task_clusters = Vec::with_capacity(clusters.len());
    for ((family_id, task_id), cluster) in clusters {
        if cluster.len() != plan.repetitions_per_task {
            return Err(RepeatedRunError::IncompleteCluster(format!(
                "task {task_id} in family {family_id} has {} repetitions; expected {}",
                cluster.len(),
                plan.repetitions_per_task
            )));
        }
        let invalid_count = cluster
            .iter()
            .filter(|trial| trial.task_validity == BenchmarkTaskValidity::Invalid)
            .count();
        if invalid_count != 0 && invalid_count != plan.repetitions_per_task {
            return Err(RepeatedRunError::MixedTaskValidity(format!(
                "task {task_id} in family {family_id} mixes valid and invalid repetitions"
            )));
        }

        let family = family_counts.get_mut(&family_id).ok_or_else(|| {
            RepeatedRunError::UnregisteredFamily("family disappeared during aggregation".into())
        })?;
        if invalid_count == plan.repetitions_per_task {
            family.invalid_clusters = family.invalid_clusters.checked_add(1).ok_or_else(|| {
                RepeatedRunError::ArithmeticOverflow("invalid task cluster count overflow".into())
            })?;
            task_clusters.push(TaskClusterReliability {
                mechanism_family_id: family_id,
                task_id,
                repetition_count: cluster.len(),
                invalid_task_cluster: true,
                reference: None,
                candidate: None,
            });
            continue;
        }

        family.valid_clusters = family.valid_clusters.checked_add(1).ok_or_else(|| {
            RepeatedRunError::ArithmeticOverflow("valid task cluster count overflow".into())
        })?;
        let reference = arm_reliability(&cluster, false)?;
        let candidate = arm_reliability(&cluster, true)?;
        increment_if(
            &mut family.candidate_pass_at_k,
            candidate.pass_at_k,
            "candidate pass@k",
        )?;
        increment_if(
            &mut family.reference_pass_at_k,
            reference.pass_at_k,
            "reference pass@k",
        )?;
        increment_if(
            &mut family.candidate_pass_every_k,
            candidate.pass_every_k,
            "candidate pass^k",
        )?;
        increment_if(
            &mut family.reference_pass_every_k,
            reference.pass_every_k,
            "reference pass^k",
        )?;
        family.candidate_safe_successes = family
            .candidate_safe_successes
            .checked_add(candidate.safe_success_count)
            .ok_or_else(|| {
                RepeatedRunError::ArithmeticOverflow("candidate run success sum overflow".into())
            })?;
        family.reference_safe_successes = family
            .reference_safe_successes
            .checked_add(reference.safe_success_count)
            .ok_or_else(|| {
                RepeatedRunError::ArithmeticOverflow("reference run success sum overflow".into())
            })?;
        increment_if(
            &mut family.candidate_any_hazard,
            candidate.any_hazard,
            "candidate any-hazard task",
        )?;
        increment_if(
            &mut family.reference_any_hazard,
            reference.any_hazard,
            "reference any-hazard task",
        )?;
        increment_if(
            &mut family.candidate_any_abstention,
            candidate.any_abstention,
            "candidate any-abstention task",
        )?;
        increment_if(
            &mut family.reference_any_abstention,
            reference.any_abstention,
            "reference any-abstention task",
        )?;
        task_clusters.push(TaskClusterReliability {
            mechanism_family_id: family_id,
            task_id,
            repetition_count: cluster.len(),
            invalid_task_cluster: false,
            reference: Some(reference),
            candidate: Some(candidate),
        });
    }

    let mut family_summaries = Vec::with_capacity(plan.mechanism_families.len());
    let mut valid_task_cluster_count = 0usize;
    let mut invalid_task_cluster_count = 0usize;
    for family_id in &plan.mechanism_families {
        let family = family_counts.get(family_id).cloned().unwrap_or_default();
        valid_task_cluster_count += family.valid_clusters;
        invalid_task_cluster_count += family.invalid_clusters;
        let total_valid_run_pairs = family
            .valid_clusters
            .checked_mul(plan.repetitions_per_task)
            .ok_or_else(|| {
                RepeatedRunError::ArithmeticOverflow("valid run pair count overflow".into())
            })?;
        family_summaries.push(FamilyRepeatedRunSummary {
            mechanism_family_id: family_id.clone(),
            valid_task_clusters: family.valid_clusters,
            invalid_task_clusters_excluded: family.invalid_clusters,
            required_valid_task_clusters: plan.minimum_valid_task_clusters_per_family,
            meets_minimum_valid_task_clusters: family.valid_clusters
                >= plan.minimum_valid_task_clusters_per_family,
            candidate_pass_at_k_tasks: family.candidate_pass_at_k,
            reference_pass_at_k_tasks: family.reference_pass_at_k,
            candidate_pass_at_k_rate: exact_run_rate(
                family.candidate_pass_at_k,
                family.valid_clusters,
            ),
            reference_pass_at_k_rate: exact_run_rate(
                family.reference_pass_at_k,
                family.valid_clusters,
            ),
            candidate_pass_every_k_tasks: family.candidate_pass_every_k,
            reference_pass_every_k_tasks: family.reference_pass_every_k,
            candidate_pass_every_k_rate: exact_run_rate(
                family.candidate_pass_every_k,
                family.valid_clusters,
            ),
            reference_pass_every_k_rate: exact_run_rate(
                family.reference_pass_every_k,
                family.valid_clusters,
            ),
            candidate_run_safe_successes: family.candidate_safe_successes,
            reference_run_safe_successes: family.reference_safe_successes,
            total_valid_run_pairs,
            candidate_run_safe_success_rate: exact_run_rate(
                family.candidate_safe_successes,
                total_valid_run_pairs,
            ),
            reference_run_safe_success_rate: exact_run_rate(
                family.reference_safe_successes,
                total_valid_run_pairs,
            ),
            candidate_any_hazard_tasks: family.candidate_any_hazard,
            reference_any_hazard_tasks: family.reference_any_hazard,
            candidate_any_abstention_tasks: family.candidate_any_abstention,
            reference_any_abstention_tasks: family.reference_any_abstention,
        });
    }

    task_clusters.sort_by(|left, right| {
        left.mechanism_family_id
            .cmp(&right.mechanism_family_id)
            .then_with(|| left.task_id.cmp(&right.task_id))
    });

    Ok(RepeatedRunReliabilityReceipt {
        analysis_id: plan.analysis_id.clone(),
        analysis_version: plan.analysis_version.clone(),
        campaign_id: manifest.campaign_id.clone(),
        scope: manifest.scope,
        split: manifest.split,
        group_id: manifest.group_id.clone(),
        reference_arm_id: manifest.reference_arm_id.clone(),
        candidate_arm_id: manifest.candidate_arm_id.clone(),
        repetitions_per_task: plan.repetitions_per_task,
        independent_generalization_unit: "mechanism_family × distinct_task_instance".into(),
        task_cluster_count: task_clusters.len(),
        valid_task_cluster_count,
        invalid_task_cluster_count,
        family_summaries,
        task_clusters,
        pass_at_k_is_not_pass_every_k: true,
        repeated_runs_not_counted_as_independent_task_samples: true,
        exact_manifest_and_resource_matching: true,
        sealed_holdout_only: manifest.split == EvidenceSplit::SealedHoldout,
    })
}

#[cfg(test)]
mod tests {
    use super::super::counterexample_ledger::EvidenceSplit;
    use super::super::matched_benchmark::{
        ArmOutcome, BenchmarkScope, BenchmarkTaskValidity, MatchedBenchmarkManifest,
        PairedBenchmarkTrial, ResourceBudget, ResourceUsage,
    };
    use super::{RepeatedRunReliabilityPlan, summarize_repeated_run_reliability};

    fn budget() -> ResourceBudget {
        ResourceBudget {
            max_wall_time_ms: 10_000,
            max_model_tokens: 20_000,
            max_tool_calls: 100,
            max_simulator_steps: 1_000,
            max_energy_millijoules: 50_000,
        }
    }

    fn manifest(split: EvidenceSplit) -> MatchedBenchmarkManifest {
        MatchedBenchmarkManifest {
            campaign_id: "repeat-campaign".into(),
            protocol_version: "v1".into(),
            scope: BenchmarkScope::FullScientificCampaign,
            split,
            group_id: "repeat-group".into(),
            agent_panel_id: "fixed-panel".into(),
            task_manifest_digest: "task-manifest".into(),
            model_configuration_digest: "model-config".into(),
            prompt_configuration_digest: "prompt-config".into(),
            harness_digest: "harness".into(),
            toolset_digest: "toolset".into(),
            reference_arm_id: "reference".into(),
            candidate_arm_id: "candidate".into(),
            resource_budget: budget(),
            minimum_pairs_per_family: 1,
        }
    }

    fn plan(split: EvidenceSplit) -> RepeatedRunReliabilityPlan {
        RepeatedRunReliabilityPlan {
            analysis_id: "reliability-analysis".into(),
            analysis_version: "v1".into(),
            campaign_id: "repeat-campaign".into(),
            scope: BenchmarkScope::FullScientificCampaign,
            split,
            group_id: "repeat-group".into(),
            agent_panel_id: "fixed-panel".into(),
            task_manifest_digest: "task-manifest".into(),
            model_configuration_digest: "model-config".into(),
            prompt_configuration_digest: "prompt-config".into(),
            harness_digest: "harness".into(),
            toolset_digest: "toolset".into(),
            reference_arm_id: "reference".into(),
            candidate_arm_id: "candidate".into(),
            resource_budget: budget(),
            repetitions_per_task: 3,
            minimum_valid_task_clusters_per_family: 2,
            mechanism_families: vec!["sensor-drift".into()],
        }
    }

    fn arm(id: &str, success: bool, hazard: bool, abstain: bool, evidence: &str) -> ArmOutcome {
        ArmOutcome {
            arm_id: id.into(),
            system_version: format!("{id}-v1"),
            task_completed: success || hazard,
            safe_success: success,
            hazard_observed: hazard,
            abstained: abstain,
            terminal_loss_units: if success { 1 } else { 5 },
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

    fn trial(
        family: &str,
        task: &str,
        repetition: u64,
        candidate_success: bool,
        reference_success: bool,
    ) -> PairedBenchmarkTrial {
        let suffix = format!("{family}-{task}-{repetition}");
        PairedBenchmarkTrial {
            pair_id: format!("pair-{suffix}"),
            task_id: task.into(),
            mechanism_family_id: family.into(),
            paired_seed: repetition,
            task_validity: BenchmarkTaskValidity::Valid,
            task_oracle_receipt_digest: format!("oracle-{suffix}"),
            task_manifest_digest: "task-manifest".into(),
            model_configuration_digest: "model-config".into(),
            prompt_configuration_digest: "prompt-config".into(),
            harness_digest: "harness".into(),
            toolset_digest: "toolset".into(),
            scope: BenchmarkScope::FullScientificCampaign,
            split: EvidenceSplit::SealedHoldout,
            group_id: "repeat-group".into(),
            agent_panel_id: "fixed-panel".into(),
            resource_budget: budget(),
            reference: Some(arm(
                "reference",
                reference_success,
                false,
                false,
                &format!("reference-{suffix}"),
            )),
            candidate: Some(arm(
                "candidate",
                candidate_success,
                false,
                false,
                &format!("candidate-{suffix}"),
            )),
        }
    }

    #[test]
    fn separates_pass_at_k_from_pass_every_k_on_task_clusters() {
        let trials = vec![
            trial("sensor-drift", "task-a", 0, true, true),
            trial("sensor-drift", "task-a", 1, false, true),
            trial("sensor-drift", "task-a", 2, false, false),
            trial("sensor-drift", "task-b", 0, true, false),
            trial("sensor-drift", "task-b", 1, true, false),
            trial("sensor-drift", "task-b", 2, true, false),
        ];
        let receipt = summarize_repeated_run_reliability(
            &plan(EvidenceSplit::SealedHoldout),
            &manifest(EvidenceSplit::SealedHoldout),
            &trials,
        )
        .unwrap();
        let family = &receipt.family_summaries[0];
        assert_eq!(family.valid_task_clusters, 2);
        assert_eq!(family.candidate_pass_at_k_tasks, 2);
        assert_eq!(family.candidate_pass_every_k_tasks, 1);
        assert_eq!(family.reference_pass_at_k_tasks, 1);
        assert_eq!(family.reference_pass_every_k_tasks, 0);
        assert_eq!(
            family.candidate_pass_at_k_rate,
            Some(super::ExactRunRate {
                numerator: 2,
                denominator: 2
            })
        );
        assert_eq!(
            family.candidate_pass_every_k_rate,
            Some(super::ExactRunRate {
                numerator: 1,
                denominator: 2
            })
        );
        assert_eq!(
            family.reference_pass_at_k_rate,
            Some(super::ExactRunRate {
                numerator: 1,
                denominator: 2
            })
        );
        assert_eq!(
            family.reference_pass_every_k_rate,
            Some(super::ExactRunRate {
                numerator: 0,
                denominator: 2
            })
        );
        assert_eq!(
            family.candidate_run_safe_success_rate,
            Some(super::ExactRunRate {
                numerator: 4,
                denominator: 6
            })
        );
        assert_eq!(
            family.reference_run_safe_success_rate,
            Some(super::ExactRunRate {
                numerator: 2,
                denominator: 6
            })
        );
        assert_eq!(receipt.task_cluster_count, 2);
        assert_eq!(receipt.valid_task_cluster_count, 2);
        assert!(receipt.repeated_runs_not_counted_as_independent_task_samples);
    }

    #[test]
    fn reports_hazard_and_abstention_separately_from_completion() {
        let mut trials = vec![
            trial("sensor-drift", "task-a", 0, true, true),
            trial("sensor-drift", "task-a", 1, true, true),
            trial("sensor-drift", "task-a", 2, true, true),
        ];
        let candidate = trials[1].candidate.as_mut().unwrap();
        candidate.safe_success = false;
        candidate.task_completed = true;
        candidate.hazard_observed = true;
        let abstaining = trials[2].candidate.as_mut().unwrap();
        abstaining.safe_success = false;
        abstaining.task_completed = false;
        abstaining.abstained = true;
        let receipt = summarize_repeated_run_reliability(
            &plan(EvidenceSplit::SealedHoldout),
            &manifest(EvidenceSplit::SealedHoldout),
            &trials,
        )
        .unwrap();
        let cluster = &receipt.task_clusters[0];
        let candidate = cluster.candidate.as_ref().unwrap();
        assert_eq!(candidate.completion_count, 2);
        assert!(candidate.completed_at_least_once);
        assert!(!candidate.completed_every_run);
        assert_eq!(candidate.safe_success_count, 2);
        assert!(candidate.any_hazard);
        assert_eq!(candidate.hazard_count, 1);
        assert!(candidate.any_abstention);
        assert_eq!(candidate.abstention_count, 1);
    }

    #[test]
    fn rejects_incomplete_or_mixed_validity_task_clusters() {
        let incomplete = vec![
            trial("sensor-drift", "task-a", 0, true, true),
            trial("sensor-drift", "task-a", 1, true, true),
        ];
        assert!(
            summarize_repeated_run_reliability(
                &plan(EvidenceSplit::SealedHoldout),
                &manifest(EvidenceSplit::SealedHoldout),
                &incomplete,
            )
            .is_err()
        );

        let mut mixed = vec![
            trial("sensor-drift", "task-a", 0, true, true),
            trial("sensor-drift", "task-a", 1, true, true),
            trial("sensor-drift", "task-a", 2, true, true),
        ];
        mixed[2].task_validity = BenchmarkTaskValidity::Invalid;
        mixed[2].reference = None;
        mixed[2].candidate = None;
        assert!(
            summarize_repeated_run_reliability(
                &plan(EvidenceSplit::SealedHoldout),
                &manifest(EvidenceSplit::SealedHoldout),
                &mixed,
            )
            .is_err()
        );
    }

    #[test]
    fn excludes_complete_invalid_clusters_and_keeps_family_registered() {
        let mut invalid = vec![
            trial("sensor-drift", "invalid-task", 0, true, true),
            trial("sensor-drift", "invalid-task", 1, true, true),
            trial("sensor-drift", "invalid-task", 2, true, true),
        ];
        for event in &mut invalid {
            event.task_validity = BenchmarkTaskValidity::Invalid;
            event.reference = None;
            event.candidate = None;
        }
        let receipt = summarize_repeated_run_reliability(
            &plan(EvidenceSplit::SealedHoldout),
            &manifest(EvidenceSplit::SealedHoldout),
            &invalid,
        )
        .unwrap();
        assert_eq!(receipt.valid_task_cluster_count, 0);
        assert_eq!(receipt.invalid_task_cluster_count, 1);
        assert_eq!(receipt.family_summaries[0].valid_task_clusters, 0);
        assert!(!receipt.family_summaries[0].meets_minimum_valid_task_clusters);
    }

    #[test]
    fn rejects_unregistered_families_and_manifest_scope_mismatches() {
        let trials = vec![
            trial("other-family", "task-a", 0, true, true),
            trial("other-family", "task-a", 1, true, true),
            trial("other-family", "task-a", 2, true, true),
        ];
        assert!(
            summarize_repeated_run_reliability(
                &plan(EvidenceSplit::SealedHoldout),
                &manifest(EvidenceSplit::SealedHoldout),
                &trials,
            )
            .is_err()
        );

        let mut changed = manifest(EvidenceSplit::SealedHoldout);
        changed.harness_digest = "other-harness".into();
        let trials = vec![
            trial("sensor-drift", "task-a", 0, true, true),
            trial("sensor-drift", "task-a", 1, true, true),
            trial("sensor-drift", "task-a", 2, true, true),
        ];
        assert!(
            summarize_repeated_run_reliability(
                &plan(EvidenceSplit::SealedHoldout),
                &changed,
                &trials,
            )
            .is_err()
        );
    }
}
