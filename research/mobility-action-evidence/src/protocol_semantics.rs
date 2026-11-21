//! Semantic checks for CDAAB run receipts. JSON Schema checks shape; this
//! module checks cross-field and preregistration invariants. It does not parse
//! JSON, authenticate hashes, inspect private evaluator truth, or calculate
//! statistics.

use super::Digest;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunStatus {
    Complete,
    Incomplete,
    Invalidated,
    Aborted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EpisodeStatus {
    Completed,
    Incomplete,
    SafetyStop,
    IntegrityStop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedRunBinding {
    pub plan_digest: Digest,
    pub manifest_digest: Digest,
    pub evaluator_digest: Digest,
    pub candidate_source_commit: String,
    pub candidate_artifact_digest: Digest,
    pub candidate_runtime_digest: Digest,
    pub scenario_set_digest: Digest,
    pub expected_evaluator_identity: String,
    pub expected_evaluator_receipt_digest: Digest,
    /// Exact episode IDs and scenario identities frozen by the plan.
    pub planned_episodes: Vec<PlannedEpisode>,
    /// Exclusion reason codes explicitly allowed by the preregistered plan.
    pub permitted_exclusion_reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedEpisode {
    pub episode_id: String,
    pub scenario_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodeReceipt {
    pub episode_id: String,
    pub scenario_id: String,
    pub controller_event_log_digest: Digest,
    pub evaluator_result_digest: Digest,
    pub status: EpisodeStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExcludedEpisode {
    pub episode_id: String,
    pub scenario_id: String,
    pub reason_code: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregateCount {
    pub metric_id: String,
    pub numerator: u64,
    pub denominator: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunReceipt {
    pub run_id: String,
    pub plan_digest: Digest,
    pub manifest_digest: Digest,
    pub evaluator_digest: Digest,
    pub candidate_source_commit: String,
    pub candidate_artifact_digest: Digest,
    pub candidate_runtime_digest: Digest,
    pub scenario_set_digest: Digest,
    pub episode_receipts: Vec<EpisodeReceipt>,
    pub excluded_episodes: Vec<ExcludedEpisode>,
    pub aggregate_counts: Vec<AggregateCount>,
    pub independent_evaluator_identity: String,
    pub independent_evaluator_receipt_digest: Digest,
    pub run_status: RunStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunReceiptError {
    EmptyRunId,
    InvalidSourceCommit,
    EmptyEvaluatorIdentity,
    PlannedEpisodeSetIsEmpty,
    EmptyPlannedEpisodeId,
    EmptyPlannedScenarioId,
    DuplicatePlannedEpisodeId,
    UnplannedEpisodeId,
    PlannedScenarioMismatch,
    PlanDigestMismatch,
    ManifestDigestMismatch,
    EvaluatorDigestMismatch,
    CandidateCommitMismatch,
    CandidateArtifactMismatch,
    CandidateRuntimeMismatch,
    ScenarioSetDigestMismatch,
    EvaluatorIdentityMismatch,
    EvaluatorReceiptDigestMismatch,
    EmptyEpisodeSet,
    EmptyEpisodeId,
    EmptyScenarioId,
    DuplicateEpisodeId,
    EmptyExclusionReason,
    UnapprovedExclusionReason,
    DuplicateExcludedEpisodeId,
    EpisodeAlsoExcluded,
    EpisodeCountExceedsPlan,
    CompleteRunDoesNotAccountForPlan,
    CompleteRunContainsNonCompletedEpisode,
    EmptyMetricId,
    DuplicateMetricId,
    ZeroMetricDenominator,
    NumeratorExceedsDenominator,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidatedRun {
    pub complete: bool,
    pub executed_episodes: usize,
    pub excluded_episodes: usize,
    pub metrics: usize,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}
fn valid_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Rejects receipts whose run/build/evaluator bindings diverge from the
/// preregistered plan, or whose counts/episode dispositions are inconsistent.
/// It never promotes an incomplete, invalidated or aborted run to complete.
pub fn validate_run_receipt(
    expected: &ExpectedRunBinding,
    receipt: &RunReceipt,
) -> Result<ValidatedRun, RunReceiptError> {
    if !nonempty(&receipt.run_id) {
        return Err(RunReceiptError::EmptyRunId);
    }
    if !valid_commit(&receipt.candidate_source_commit) {
        return Err(RunReceiptError::InvalidSourceCommit);
    }
    if !nonempty(&receipt.independent_evaluator_identity)
        || !nonempty(&expected.expected_evaluator_identity)
    {
        return Err(RunReceiptError::EmptyEvaluatorIdentity);
    }
    if expected.planned_episodes.is_empty() {
        return Err(RunReceiptError::PlannedEpisodeSetIsEmpty);
    }
    let mut planned_by_id = BTreeMap::new();
    for planned in &expected.planned_episodes {
        if !nonempty(&planned.episode_id) {
            return Err(RunReceiptError::EmptyPlannedEpisodeId);
        }
        if !nonempty(&planned.scenario_id) {
            return Err(RunReceiptError::EmptyPlannedScenarioId);
        }
        if planned_by_id
            .insert(planned.episode_id.as_str(), planned.scenario_id.as_str())
            .is_some()
        {
            return Err(RunReceiptError::DuplicatePlannedEpisodeId);
        }
    }

    if receipt.plan_digest != expected.plan_digest {
        return Err(RunReceiptError::PlanDigestMismatch);
    }
    if receipt.manifest_digest != expected.manifest_digest {
        return Err(RunReceiptError::ManifestDigestMismatch);
    }
    if receipt.evaluator_digest != expected.evaluator_digest {
        return Err(RunReceiptError::EvaluatorDigestMismatch);
    }
    if receipt.candidate_source_commit != expected.candidate_source_commit {
        return Err(RunReceiptError::CandidateCommitMismatch);
    }
    if receipt.candidate_artifact_digest != expected.candidate_artifact_digest {
        return Err(RunReceiptError::CandidateArtifactMismatch);
    }
    if receipt.candidate_runtime_digest != expected.candidate_runtime_digest {
        return Err(RunReceiptError::CandidateRuntimeMismatch);
    }
    if receipt.scenario_set_digest != expected.scenario_set_digest {
        return Err(RunReceiptError::ScenarioSetDigestMismatch);
    }
    if receipt.independent_evaluator_identity != expected.expected_evaluator_identity {
        return Err(RunReceiptError::EvaluatorIdentityMismatch);
    }
    if receipt.independent_evaluator_receipt_digest != expected.expected_evaluator_receipt_digest {
        return Err(RunReceiptError::EvaluatorReceiptDigestMismatch);
    }
    if receipt.episode_receipts.is_empty() {
        return Err(RunReceiptError::EmptyEpisodeSet);
    }

    let mut episode_ids = BTreeSet::new();
    for episode in &receipt.episode_receipts {
        if !nonempty(&episode.episode_id) {
            return Err(RunReceiptError::EmptyEpisodeId);
        }
        if !nonempty(&episode.scenario_id) {
            return Err(RunReceiptError::EmptyScenarioId);
        }
        let planned_scenario = planned_by_id
            .get(episode.episode_id.as_str())
            .ok_or(RunReceiptError::UnplannedEpisodeId)?;
        if *planned_scenario != episode.scenario_id {
            return Err(RunReceiptError::PlannedScenarioMismatch);
        }
        if !episode_ids.insert(episode.episode_id.as_str()) {
            return Err(RunReceiptError::DuplicateEpisodeId);
        }
    }

    let mut excluded_ids = BTreeSet::new();
    for excluded in &receipt.excluded_episodes {
        if !nonempty(&excluded.episode_id) {
            return Err(RunReceiptError::EmptyEpisodeId);
        }
        if !nonempty(&excluded.scenario_id) {
            return Err(RunReceiptError::EmptyScenarioId);
        }
        let planned_scenario = planned_by_id
            .get(excluded.episode_id.as_str())
            .ok_or(RunReceiptError::UnplannedEpisodeId)?;
        if *planned_scenario != excluded.scenario_id {
            return Err(RunReceiptError::PlannedScenarioMismatch);
        }
        if !nonempty(&excluded.reason) {
            return Err(RunReceiptError::EmptyExclusionReason);
        }
        if !nonempty(&excluded.reason_code)
            || !expected
                .permitted_exclusion_reason_codes
                .contains(&excluded.reason_code)
        {
            return Err(RunReceiptError::UnapprovedExclusionReason);
        }
        if !excluded_ids.insert(excluded.episode_id.as_str()) {
            return Err(RunReceiptError::DuplicateExcludedEpisodeId);
        }
        if episode_ids.contains(excluded.episode_id.as_str()) {
            return Err(RunReceiptError::EpisodeAlsoExcluded);
        }
    }

    let accounted = receipt.episode_receipts.len() + receipt.excluded_episodes.len();
    if accounted > planned_by_id.len() {
        return Err(RunReceiptError::EpisodeCountExceedsPlan);
    }

    let mut metric_ids = BTreeSet::new();
    for metric in &receipt.aggregate_counts {
        if !nonempty(&metric.metric_id) {
            return Err(RunReceiptError::EmptyMetricId);
        }
        if !metric_ids.insert(metric.metric_id.as_str()) {
            return Err(RunReceiptError::DuplicateMetricId);
        }
        if metric.denominator == 0 {
            return Err(RunReceiptError::ZeroMetricDenominator);
        }
        if metric.numerator > metric.denominator {
            return Err(RunReceiptError::NumeratorExceedsDenominator);
        }
    }
    if receipt.aggregate_counts.is_empty() {
        return Err(RunReceiptError::EmptyMetricId);
    }

    if receipt.run_status == RunStatus::Complete {
        if accounted != planned_by_id.len() {
            return Err(RunReceiptError::CompleteRunDoesNotAccountForPlan);
        }
        if receipt
            .episode_receipts
            .iter()
            .any(|episode| episode.status != EpisodeStatus::Completed)
        {
            return Err(RunReceiptError::CompleteRunContainsNonCompletedEpisode);
        }
    }

    Ok(ValidatedRun {
        complete: receipt.run_status == RunStatus::Complete,
        executed_episodes: receipt.episode_receipts.len(),
        excluded_episodes: receipt.excluded_episodes.len(),
        metrics: receipt.aggregate_counts.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Digest {
        Digest([byte; 32])
    }

    fn expected() -> ExpectedRunBinding {
        ExpectedRunBinding {
            plan_digest: digest(1),
            manifest_digest: digest(2),
            evaluator_digest: digest(3),
            candidate_source_commit: "0123456789abcdef0123456789abcdef01234567".into(),
            candidate_artifact_digest: digest(4),
            candidate_runtime_digest: digest(5),
            scenario_set_digest: digest(6),
            expected_evaluator_identity: "evaluator-v1".into(),
            expected_evaluator_receipt_digest: digest(7),
            permitted_exclusion_reason_codes: vec![
                "simulator_reset_before_policy_invocation".into(),
            ],
            planned_episodes: vec![
                PlannedEpisode {
                    episode_id: "episode-001".into(),
                    scenario_id: "scenario-001".into(),
                },
                PlannedEpisode {
                    episode_id: "episode-002".into(),
                    scenario_id: "scenario-002".into(),
                },
            ],
        }
    }

    struct BindingMutation {
        apply: fn(&mut ExpectedRunBinding),
        expected_error: RunReceiptError,
    }

    fn receipt() -> RunReceipt {
        RunReceipt {
            run_id: "cdaab-run-001".into(),
            plan_digest: digest(1),
            manifest_digest: digest(2),
            evaluator_digest: digest(3),
            candidate_source_commit: "0123456789abcdef0123456789abcdef01234567".into(),
            candidate_artifact_digest: digest(4),
            candidate_runtime_digest: digest(5),
            scenario_set_digest: digest(6),
            episode_receipts: vec![
                EpisodeReceipt {
                    episode_id: "episode-001".into(),
                    scenario_id: "scenario-001".into(),
                    controller_event_log_digest: digest(8),
                    evaluator_result_digest: digest(9),
                    status: EpisodeStatus::Completed,
                },
                EpisodeReceipt {
                    episode_id: "episode-002".into(),
                    scenario_id: "scenario-002".into(),
                    controller_event_log_digest: digest(10),
                    evaluator_result_digest: digest(11),
                    status: EpisodeStatus::Completed,
                },
            ],
            excluded_episodes: vec![],
            aggregate_counts: vec![
                AggregateCount {
                    metric_id: "verified_success".into(),
                    numerator: 2,
                    denominator: 2,
                },
                AggregateCount {
                    metric_id: "false_completion".into(),
                    numerator: 0,
                    denominator: 2,
                },
            ],
            independent_evaluator_identity: "evaluator-v1".into(),
            independent_evaluator_receipt_digest: digest(7),
            run_status: RunStatus::Complete,
        }
    }

    #[test]
    fn complete_receipt_matches_preregistered_plan() {
        let result = validate_run_receipt(&expected(), &receipt()).expect("complete run");
        assert_eq!(
            result,
            ValidatedRun {
                complete: true,
                executed_episodes: 2,
                excluded_episodes: 0,
                metrics: 2
            }
        );
    }

    #[test]
    fn frozen_plan_build_and_evaluator_bindings_must_match() {
        let cases = vec![
            BindingMutation {
                apply: |x| x.plan_digest = digest(80),
                expected_error: RunReceiptError::PlanDigestMismatch,
            },
            BindingMutation {
                apply: |x| x.manifest_digest = digest(80),
                expected_error: RunReceiptError::ManifestDigestMismatch,
            },
            BindingMutation {
                apply: |x| x.evaluator_digest = digest(80),
                expected_error: RunReceiptError::EvaluatorDigestMismatch,
            },
            BindingMutation {
                apply: |x| x.candidate_artifact_digest = digest(80),
                expected_error: RunReceiptError::CandidateArtifactMismatch,
            },
            BindingMutation {
                apply: |x| x.candidate_runtime_digest = digest(80),
                expected_error: RunReceiptError::CandidateRuntimeMismatch,
            },
            BindingMutation {
                apply: |x| x.scenario_set_digest = digest(80),
                expected_error: RunReceiptError::ScenarioSetDigestMismatch,
            },
            BindingMutation {
                apply: |x| x.expected_evaluator_receipt_digest = digest(80),
                expected_error: RunReceiptError::EvaluatorReceiptDigestMismatch,
            },
        ];
        for case in cases {
            let mut binding = expected();
            (case.apply)(&mut binding);
            assert_eq!(
                validate_run_receipt(&binding, &receipt()),
                Err(case.expected_error)
            );
        }
        let mut run = receipt();
        run.candidate_source_commit = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into();
        assert_eq!(
            validate_run_receipt(&expected(), &run),
            Err(RunReceiptError::CandidateCommitMismatch)
        );
    }

    #[test]
    fn incomplete_or_safety_stopped_run_cannot_be_promoted_to_complete() {
        let mut run = receipt();
        run.episode_receipts[1].status = EpisodeStatus::Incomplete;
        assert_eq!(
            validate_run_receipt(&expected(), &run),
            Err(RunReceiptError::CompleteRunContainsNonCompletedEpisode)
        );
        run.run_status = RunStatus::Incomplete;
        assert!(
            !validate_run_receipt(&expected(), &run)
                .expect("incomplete evidence")
                .complete
        );

        for status in [EpisodeStatus::SafetyStop, EpisodeStatus::IntegrityStop] {
            let mut run = receipt();
            run.episode_receipts[0].status = status;
            assert_eq!(
                validate_run_receipt(&expected(), &run),
                Err(RunReceiptError::CompleteRunContainsNonCompletedEpisode)
            );
        }
    }

    #[test]
    fn zero_denominators_and_impossible_counts_are_rejected() {
        let mut run = receipt();
        run.aggregate_counts[0].denominator = 0;
        assert_eq!(
            validate_run_receipt(&expected(), &run),
            Err(RunReceiptError::ZeroMetricDenominator)
        );
        let mut run = receipt();
        run.aggregate_counts[0].numerator = 3;
        assert_eq!(
            validate_run_receipt(&expected(), &run),
            Err(RunReceiptError::NumeratorExceedsDenominator)
        );
    }

    #[test]
    fn duplicate_episode_metric_and_exclusion_ids_are_rejected() {
        let mut run = receipt();
        run.episode_receipts[1].episode_id = run.episode_receipts[0].episode_id.clone();
        assert_eq!(
            validate_run_receipt(&expected(), &run),
            Err(RunReceiptError::DuplicateEpisodeId)
        );
        let mut run = receipt();
        run.aggregate_counts[1].metric_id = run.aggregate_counts[0].metric_id.clone();
        assert_eq!(
            validate_run_receipt(&expected(), &run),
            Err(RunReceiptError::DuplicateMetricId)
        );
        let mut run = receipt();
        run.excluded_episodes.push(ExcludedEpisode {
            episode_id: "episode-001".into(),
            scenario_id: "scenario-001".into(),
            reason_code: "simulator_reset_before_policy_invocation".into(),
            reason: "excluded".into(),
        });
        assert_eq!(
            validate_run_receipt(&expected(), &run),
            Err(RunReceiptError::EpisodeAlsoExcluded)
        );
    }

    #[test]
    fn complete_run_must_account_for_every_planned_episode() {
        let mut binding = expected();
        binding.planned_episodes.push(PlannedEpisode {
            episode_id: "episode-003".into(),
            scenario_id: "scenario-003".into(),
        });
        assert_eq!(
            validate_run_receipt(&binding, &receipt()),
            Err(RunReceiptError::CompleteRunDoesNotAccountForPlan)
        );
        let mut run = receipt();
        run.episode_receipts.pop();
        run.run_status = RunStatus::Incomplete;
        assert!(
            !validate_run_receipt(&expected(), &run)
                .expect("partial evidence")
                .complete
        );
    }

    #[test]
    fn transparent_exclusions_are_accounted_but_never_hidden() {
        let mut binding = expected();
        binding.planned_episodes.push(PlannedEpisode {
            episode_id: "episode-003".into(),
            scenario_id: "scenario-003".into(),
        });
        let mut run = receipt();
        run.excluded_episodes.push(ExcludedEpisode {
            episode_id: "episode-003".into(),
            scenario_id: "scenario-003".into(),
            reason_code: "simulator_reset_before_policy_invocation".into(),
            reason: "simulator reset failed before invocation".into(),
        });
        let result = validate_run_receipt(&binding, &run).expect("documented exclusion");
        assert!(result.complete);
        assert_eq!(result.excluded_episodes, 1);

        let mut run = receipt();
        run.excluded_episodes.push(ExcludedEpisode {
            episode_id: "episode-003".into(),
            scenario_id: "scenario-003".into(),
            reason_code: "simulator_reset_before_policy_invocation".into(),
            reason: " ".into(),
        });
        assert_eq!(
            validate_run_receipt(&binding, &run),
            Err(RunReceiptError::EmptyExclusionReason)
        );
    }


    #[test]
    fn post_hoc_exclusion_reason_codes_are_rejected() {
        let mut binding = expected();
        binding.planned_episodes.push(PlannedEpisode {
            episode_id: "episode-003".into(),
            scenario_id: "scenario-003".into(),
        });
        let mut run = receipt();
        run.excluded_episodes.push(ExcludedEpisode {
            episode_id: "episode-003".into(),
            scenario_id: "scenario-003".into(),
            reason_code: "candidate_failed".into(),
            reason: "candidate did not complete the task".into(),
        });
        assert_eq!(
            validate_run_receipt(&binding, &run),
            Err(RunReceiptError::UnapprovedExclusionReason)
        );

        let mut run = receipt();
        run.excluded_episodes.push(ExcludedEpisode {
            episode_id: "episode-003".into(),
            scenario_id: "scenario-003".into(),
            reason_code: "simulator_reset_before_policy_invocation".into(),
            reason: "pre-invocation simulator reset failure".into(),
        });
        run.excluded_episodes[0].scenario_id = "wrong-scenario".into();
        assert_eq!(
            validate_run_receipt(&binding, &run),
            Err(RunReceiptError::PlannedScenarioMismatch)
        );
    }

    #[test]
    fn empty_planned_episode_set_is_rejected() {
        let mut binding = expected();
        binding.planned_episodes.clear();
        assert_eq!(
            validate_run_receipt(&binding, &receipt()),
            Err(RunReceiptError::PlannedEpisodeSetIsEmpty)
        );
    }

    #[test]
    fn executed_and_excluded_episodes_must_match_preregistered_ids_and_scenarios() {
        let mut run = receipt();
        run.episode_receipts[0].episode_id = "unplanned-episode".into();
        assert_eq!(
            validate_run_receipt(&expected(), &run),
            Err(RunReceiptError::UnplannedEpisodeId)
        );

        let mut run = receipt();
        run.episode_receipts[0].scenario_id = "different-scenario".into();
        assert_eq!(
            validate_run_receipt(&expected(), &run),
            Err(RunReceiptError::PlannedScenarioMismatch)
        );

        let mut binding = expected();
        binding.planned_episodes[1].episode_id = binding.planned_episodes[0].episode_id.clone();
        assert_eq!(
            validate_run_receipt(&binding, &receipt()),
            Err(RunReceiptError::DuplicatePlannedEpisodeId)
        );

        let mut run = receipt();
        run.excluded_episodes.push(ExcludedEpisode {
            episode_id: "unplanned-exclusion".into(),
            scenario_id: "scenario-003".into(),
            reason_code: "simulator_reset_before_policy_invocation".into(),
            reason: "not in the frozen plan".into(),
        });
        assert_eq!(
            validate_run_receipt(&expected(), &run),
            Err(RunReceiptError::UnplannedEpisodeId)
        );
    }

    #[test]
    fn evaluator_identity_is_independently_pinned() {
        let mut run = receipt();
        run.independent_evaluator_identity = "candidate-controller".into();
        assert_eq!(
            validate_run_receipt(&expected(), &run),
            Err(RunReceiptError::EvaluatorIdentityMismatch)
        );
        let mut run = receipt();
        run.independent_evaluator_identity.clear();
        assert_eq!(
            validate_run_receipt(&expected(), &run),
            Err(RunReceiptError::EmptyEvaluatorIdentity)
        );
    }
}
