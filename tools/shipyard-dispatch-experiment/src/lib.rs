//! Matched-input dispatch evaluation with an independent feasibility evaluator.
//!
//! The policy receives only DispatchInput. The evaluator receives source-owned
//! completed intervals and evaluator-side outage truth as separate arguments.
//! This adapter retains every rule outcome and emits no blended score.

use shipyard_dispatch_audit::{
    audit_schedule, Assignment as AuditAssignment, AuditInput, AuditReport,
    Completion as AuditCompletion, EvaluatorOutage, FrozenCommitment, Operation as AuditOperation,
    Resource as AuditResource,
};
use shipyard_observed_dispatch::{
    build_dispatch_plan, Commitment, DispatchInput, DispatchPlan, DispatchRule, PlanStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatedRule {
    pub rule: DispatchRule,
    pub policy_status: PlanStatus,
    pub plan: DispatchPlan,
    pub audit: AuditReport,
}

/// Check that the independent historical intervals describe exactly the
/// completion IDs and end ticks in the policy snapshot. Start/resource details
/// come from the source-owned execution ledger and are not inferred by policy.
fn validate_completion_alignment(
    policy_input: &DispatchInput,
    audit_completed: &[AuditCompletion],
) -> Result<(), String> {
    let mut policy = std::collections::BTreeMap::new();
    for completed in &policy_input.completed {
        if policy
            .insert(completed.operation_id.as_str(), completed.completed_at_tick)
            .is_some()
        {
            return Err(format!(
                "policy snapshot repeats completion {}",
                completed.operation_id
            ));
        }
    }
    let mut evaluator = std::collections::BTreeMap::new();
    for completed in audit_completed {
        if evaluator
            .insert(completed.operation_id.as_str(), completed.completed_at_tick)
            .is_some()
        {
            return Err(format!(
                "execution ledger repeats completion {}",
                completed.operation_id
            ));
        }
    }
    if policy != evaluator {
        return Err(
            "policy and evaluator completion IDs/ticks do not match source-owned history".to_owned(),
        );
    }
    Ok(())
}

fn to_audit_input(
    policy_input: &DispatchInput,
    plan: &DispatchPlan,
    audit_completed: &[AuditCompletion],
    evaluator_outages: &[EvaluatorOutage],
) -> AuditInput {
    AuditInput {
        decision_tick: policy_input.decision_tick,
        horizon_end_tick: policy_input.horizon_end_tick,
        operations: policy_input
            .operations
            .iter()
            .map(|operation| AuditOperation {
                id: operation.id.clone(),
                release_tick: operation.release_tick,
                due_tick: operation.due_tick,
                duration_ticks: operation.duration_ticks,
                resource_kind: operation.resource_kind.clone(),
                predecessor_ids: operation.predecessor_ids.clone(),
            })
            .collect(),
        resources: policy_input
            .resources
            .iter()
            .map(|resource| AuditResource {
                id: resource.id.clone(),
                kind: resource.kind.clone(),
            })
            .collect(),
        completed: audit_completed.to_vec(),
        frozen_commitments: policy_input
            .commitments
            .iter()
            .map(|item: &Commitment| FrozenCommitment {
                operation_id: item.operation_id.clone(),
                resource_id: item.resource_id.clone(),
                start_tick: item.start_tick,
                end_tick: item.end_tick,
            })
            .collect(),
        assignments: plan
            .assignments
            .iter()
            .map(|item| AuditAssignment {
                operation_id: item.operation_id.clone(),
                resource_id: item.resource_id.clone(),
                start_tick: item.start_tick,
                end_tick: item.end_tick,
                frozen: item.frozen,
            })
            .collect(),
        evaluator_outages: evaluator_outages.to_vec(),
    }
}

/// Run each deterministic baseline on the same policy snapshot, then score each
/// full plan against the separately supplied evaluator truth. Partial and
/// rejected plans are returned rather than filtered out.
pub fn evaluate_four_rules(
    policy_input: &DispatchInput,
    audit_completed: &[AuditCompletion],
    evaluator_outages: &[EvaluatorOutage],
) -> Result<Vec<EvaluatedRule>, String> {
    validate_completion_alignment(policy_input, audit_completed)?;
    let rules = [
        DispatchRule::FixedOrder,
        DispatchRule::EarliestDueDate,
        DispatchRule::CriticalRatio,
        DispatchRule::LeastSlack,
    ];
    Ok(rules
        .into_iter()
        .map(|rule| {
            let plan = build_dispatch_plan(policy_input, rule);
            let audit = audit_schedule(&to_audit_input(
                policy_input,
                &plan,
                audit_completed,
                evaluator_outages,
            ));
            EvaluatedRule {
                rule,
                policy_status: plan.status,
                plan,
                audit,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use shipyard_observed_dispatch::{
        Commitment, DispatchInput, Operation, Resource, VisibleOutage,
    };

    fn synthetic_input() -> DispatchInput {
        DispatchInput {
            decision_tick: 0,
            horizon_end_tick: 20,
            operations: vec![
                Operation {
                    id: "weld-long".to_owned(),
                    order: 0,
                    release_tick: 0,
                    due_tick: 10,
                    duration_ticks: 3,
                    resource_kind: "welder".to_owned(),
                    predecessor_ids: vec![],
                },
                Operation {
                    id: "weld-urgent".to_owned(),
                    order: 1,
                    release_tick: 0,
                    due_tick: 8,
                    duration_ticks: 2,
                    resource_kind: "welder".to_owned(),
                    predecessor_ids: vec![],
                },
                Operation {
                    id: "paint-after-weld".to_owned(),
                    order: 2,
                    release_tick: 0,
                    due_tick: 14,
                    duration_ticks: 2,
                    resource_kind: "paint".to_owned(),
                    predecessor_ids: vec!["weld-long".to_owned()],
                },
            ],
            resources: vec![
                Resource {
                    id: "weld-bay-1".to_owned(),
                    kind: "welder".to_owned(),
                },
                Resource {
                    id: "paint-booth-1".to_owned(),
                    kind: "paint".to_owned(),
                },
            ],
            completed: vec![],
            commitments: vec![],
            visible_outages: vec![VisibleOutage {
                event_id: "synthetic-known-maintenance".to_owned(),
                resource_id: "weld-bay-1".to_owned(),
                effective_from_tick: 2,
                effective_until_tick: 5,
                observed_at_tick: 0,
            }],
        }
    }

    #[test]
    fn all_baselines_are_retained_and_independently_audited() {
        let input = synthetic_input();
        let outages = vec![EvaluatorOutage {
            event_id: "synthetic-known-maintenance".to_owned(),
            resource_id: "weld-bay-1".to_owned(),
            effective_from_tick: 2,
            effective_until_tick: 5,
        }];
        let results = evaluate_four_rules(&input, &[], &outages).expect("aligned source history");
        assert_eq!(results.len(), 4);
        for result in &results {
            assert_eq!(result.policy_status, PlanStatus::Complete);
            assert_eq!(result.audit.status, shipyard_dispatch_audit::AuditStatus::AbstractFeasible);
            assert!(result.audit.diagnostics.is_empty());
        }
        assert!(results.iter().any(|item| item.rule == DispatchRule::FixedOrder));
        assert!(results
            .iter()
            .any(|item| item.rule == DispatchRule::EarliestDueDate));
        assert!(results.iter().any(|item| item.rule == DispatchRule::CriticalRatio));
        assert!(results.iter().any(|item| item.rule == DispatchRule::LeastSlack));
    }

    #[test]
    fn evaluator_truth_can_reject_a_plan_without_leaking_into_policy_input() {
        let mut input = synthetic_input();
        input.visible_outages.clear();
        let evaluator_outages = vec![EvaluatorOutage {
            event_id: "evaluator-only-surprise".to_owned(),
            resource_id: "weld-bay-1".to_owned(),
            effective_from_tick: 0,
            effective_until_tick: 5,
        }];
        let results =
            evaluate_four_rules(&input, &[], &evaluator_outages).expect("empty history aligns");
        assert!(results
            .iter()
            .all(|item| item.audit.status == shipyard_dispatch_audit::AuditStatus::Rejected));
        assert!(results.iter().all(|item| item
            .audit
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "ASSIGNMENT_OVERLAPS_EFFECTIVE_OUTAGE")));
        assert!(input.visible_outages.is_empty());
    }

    #[test]
    fn source_completion_timestamps_must_match_policy_snapshot() {
        let mut input = synthetic_input();
        input.completed.push(shipyard_observed_dispatch::Completion {
            operation_id: "weld-long".to_owned(),
            completed_at_tick: 2,
        });
        assert!(evaluate_four_rules(&input, &[], &[])
            .expect_err("missing evaluator execution detail")
            .contains("completion IDs/ticks do not match"));

        // The same policy completion cannot silently become a different
        // physical completion timestamp in the independent evaluation ledger.
        let audit = vec![AuditCompletion {
            operation_id: "weld-long".to_owned(),
            resource_id: "weld-bay-1".to_owned(),
            start_tick: 0,
            completed_at_tick: 3,
        }];
        assert!(evaluate_four_rules(&input, &audit, &[])
            .expect_err("different completion tick")
            .contains("completion IDs/ticks do not match"));
    }

    #[test]
    fn frozen_commitments_are_checked_against_source_owned_records() {
        let mut input = synthetic_input();
        input.commitments.push(Commitment {
            operation_id: "weld-long".to_owned(),
            resource_id: "weld-bay-1".to_owned(),
            start_tick: 0,
            end_tick: 3,
        });
        let results = evaluate_four_rules(
            &input,
            &[],
            &[EvaluatorOutage {
                event_id: "known".to_owned(),
                resource_id: "weld-bay-1".to_owned(),
                effective_from_tick: 2,
                effective_until_tick: 5,
            }],
        )
        .expect("no completions");
        assert!(results.iter().all(|item| item
            .plan
            .assignments
            .iter()
            .any(|assignment| assignment.operation_id == "weld-long" && assignment.frozen)));
        // The frozen interval overlaps the outage, so it cannot be admitted to
        // the feasible frontier even though the policy does not rewrite it.
        assert!(results.iter().all(|item| item.audit.status == shipyard_dispatch_audit::AuditStatus::Rejected));
    }
}
