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



/// Stable episode/replicate identity carried into every append-only result row.
/// The caller owns these identities and digests; this adapter never guesses them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentLedgerContext {
    pub benchmark_release_id: String,
    pub episode_id: String,
    pub replicate_id: String,
    pub initial_state_sha256: String,
    pub disturbance_tape_sha256: String,
    pub method_config_sha256: String,
    pub budget_profile_id: String,
    pub wall_time_limit_ms: u64,
    pub decision_compute_budget_units: u64,
    /// State explicitly whether the outer harness enforces the declared limits.
    pub budget_contract: String,
    pub policy_revision: String,
    pub evaluator_revision: String,
    pub synthetic_example: bool,
}

fn require_digest(name: &str, digest: &str) -> Result<(), String> {
    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{name} must be exactly 64 hexadecimal characters"));
    }
    Ok(())
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

fn optional_string_json(value: Option<&str>) -> String {
    value.map(json_quote).unwrap_or_else(|| "null".to_owned())
}

fn rule_id(rule: DispatchRule) -> &'static str {
    match rule {
        DispatchRule::FixedOrder => "fixed_order",
        DispatchRule::EarliestDueDate => "earliest_due_date",
        DispatchRule::CriticalRatio => "critical_ratio",
        DispatchRule::LeastSlack => "least_slack",
    }
}

fn policy_status_id(status: PlanStatus) -> &'static str {
    match status {
        PlanStatus::Complete => "complete",
        PlanStatus::Partial => "partial",
        PlanStatus::InvalidInput => "invalid_input",
        PlanStatus::FrozenConflict => "frozen_conflict",
    }
}

fn has_any_audit_code(result: &EvaluatedRule, codes: &[&str]) -> bool {
    result
        .audit
        .diagnostics
        .iter()
        .any(|diagnostic| codes.contains(&diagnostic.code.as_str()))
}

fn resource_gate_status(result: &EvaluatedRule) -> &'static str {
    if has_any_audit_code(
        result,
        &[
            "UNKNOWN_COMPLETED_RESOURCE",
            "COMPLETED_RESOURCE_KIND_MISMATCH",
            "UNKNOWN_EXPECTED_FROZEN_RESOURCE",
            "UNKNOWN_ASSIGNED_RESOURCE",
            "RESOURCE_KIND_MISMATCH",
            "UNARY_RESOURCE_OVERLAP",
            "UNKNOWN_EVALUATOR_OUTAGE_RESOURCE",
            "INVALID_EVALUATOR_OUTAGE_INTERVAL",
            "ASSIGNMENT_OVERLAPS_EFFECTIVE_OUTAGE",
            "INVALID_ASSIGNMENT_INTERVAL",
            "DURATION_MISMATCH",
            "STARTS_BEFORE_RELEASE",
            "NEW_ASSIGNMENT_BEFORE_DECISION",
            "ASSIGNMENT_AFTER_HORIZON",
            "FROZEN_COMMITMENT_CHANGED",
            "FROZEN_COMMITMENT_DROPPED",
            "UNDECLARED_FROZEN_ASSIGNMENT",
            "MANDATORY_OPERATION_OMITTED",
        ],
    ) {
        "failed"
    } else {
        "passed"
    }
}

fn precedence_gate_status(result: &EvaluatedRule) -> &'static str {
    if has_any_audit_code(
        result,
        &[
            "UNKNOWN_PREDECESSOR",
            "SELF_PRECEDENCE",
            "DUPLICATE_PREDECESSOR",
            "PRECEDENCE_VIOLATION",
            "PREDECESSOR_MISSING_FROM_REALIZED_STATE",
        ],
    ) {
        "failed"
    } else {
        "passed"
    }
}

fn terminal_status(result: &EvaluatedRule) -> &'static str {
    match (result.policy_status, result.audit.status) {
        // Full geometry, causal replay, and receipt-integrity gates are not run
        // here, so an abstract-model pass is deliberately recorded as incomplete.
        (PlanStatus::Complete, shipyard_dispatch_audit::AuditStatus::AbstractFeasible) => {
            "incomplete"
        }
        (PlanStatus::Partial, _) => "incomplete",
        (PlanStatus::InvalidInput | PlanStatus::FrozenConflict, _) => "invalid_input",
        (_, shipyard_dispatch_audit::AuditStatus::Rejected) => "invalid_candidate",
    }
}

fn status_reason(result: &EvaluatedRule) -> &'static str {
    match (result.policy_status, result.audit.status) {
        (PlanStatus::Complete, shipyard_dispatch_audit::AuditStatus::AbstractFeasible) => {
            "abstract unary-resource/time audit passed; spatial geometry, causality, and receipt integrity were not evaluated by this adapter"
        }
        (PlanStatus::Partial, _) => {
            "dispatch policy left mandatory work unscheduled; no infeasibility proof is implied"
        }
        (PlanStatus::InvalidInput, _) => {
            "dispatch policy rejected the supplied input; candidate was not admitted"
        }
        (PlanStatus::FrozenConflict, _) => {
            "source frozen commitments conflict with the declared abstract model; they were not rewritten"
        }
        (_, shipyard_dispatch_audit::AuditStatus::Rejected) => {
            "independent abstract evaluator rejected the candidate; inspect hard gates and diagnostics"
        }
    }
}

fn metric_record(
    metric_id: &str,
    value_json: &str,
    unit: &str,
    direction: &str,
    result: &EvaluatedRule,
) -> String {
    let validity = if result.audit.status == shipyard_dispatch_audit::AuditStatus::AbstractFeasible {
        "valid_under_abstract_model"
    } else {
        "descriptive_candidate_metric"
    };
    format!(
        "{{\"metric_id\":{},\"value\":{},\"unit\":{},\"aggregation_boundary\":\"one_episode_including_completed_history\",\"direction\":{},\"hard_gate\":false,\"recomputed_by\":\"shipyard-dispatch-audit\",\"validity_status\":{},\"quality_qualifier\":\"abstract unary-resource/time model only; not a physical-yard metric\"}}",
        json_quote(metric_id),
        value_json,
        json_quote(unit),
        json_quote(direction),
        json_quote(validity)
    )
}

fn encode_result(context: &ExperimentLedgerContext, result: &EvaluatedRule) -> String {
    let method_id = format!("{}::{}", rule_id(result.rule), context.policy_revision);
    let identity_tuple = format!(
        "[{},{},{},{}]",
        json_quote(&context.benchmark_release_id),
        json_quote(&context.episode_id),
        json_quote(&context.replicate_id),
        json_quote(&method_id)
    );
    let record_id = json_quote(&identity_tuple);
    let makespan_value = result
        .audit
        .metrics
        .makespan_tick
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned());
    let metrics = [
        metric_record("makespan", &makespan_value, "tick", "minimize", result),
        metric_record(
            "total_tardiness",
            &result.audit.metrics.total_tardiness_ticks.to_string(),
            "tick",
            "minimize",
            result,
        ),
        metric_record(
            "occupied_resource_ticks",
            &result.audit.metrics.occupied_resource_ticks.to_string(),
            "tick",
            "minimize",
            result,
        ),
    ]
    .join(",");

    let missing = result
        .audit
        .metrics
        .missing_operation_ids
        .iter()
        .map(|item| json_quote(item))
        .collect::<Vec<_>>()
        .join(",");
    let assignments = result
        .plan
        .assignments
        .iter()
        .map(|item| {
            format!(
                "{{\"operation_id\":{},\"resource_id\":{},\"start_tick\":{},\"end_tick\":{},\"frozen\":{}}}",
                json_quote(&item.operation_id),
                json_quote(&item.resource_id),
                item.start_tick,
                item.end_tick,
                item.frozen
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let diagnostics = result
        .audit
        .diagnostics
        .iter()
        .map(|item| {
            format!(
                "{{\"code\":{},\"operation_id\":{},\"detail\":{}}}",
                json_quote(&item.code),
                optional_string_json(item.operation_id.as_deref()),
                json_quote(&item.detail)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let policy_diagnostics = result
        .plan
        .diagnostics
        .iter()
        .map(|item| {
            format!(
                "{{\"code\":{},\"operation_id\":{},\"detail\":{}}}",
                json_quote(&item.code),
                optional_string_json(item.operation_id.as_deref()),
                json_quote(&item.detail)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let status = terminal_status(result);
    let reason = status_reason(result);
    let budget_contract = format!("{}: {}", context.budget_profile_id, context.budget_contract);

    format!(
        concat!(
            "{",
            "\"ledger_contract_version\":\"shipyard-result-ledger-v0\",",
            "\"record_id\":{},",
            "\"record_kind\":\"raw_run_record\",",
            "\"synthetic_example\":{},",
            "\"benchmark_release\":{},",
            "\"method_id\":{},",
            "\"episode_id\":{},",
            "\"replicate_id\":{},",
            "\"status\":{},",
            "\"status_reason\":{},",
            "\"input_digests\":{",
                "\"initial_state_sha256\":{},",
                "\"disturbance_tape_sha256\":{},",
                "\"method_config_sha256\":{}",
            "},",
            "\"evaluator_revision\":{},",
            "\"budget_profile\":{",
                "\"wall_time_limit_ms\":{},",
                "\"decision_compute_budget_units\":{},",
                "\"budget_contract\":{}",
            "},",
            "\"hard_gates\":{",
                "\"geometry\":\"not_evaluated\",",
                "\"resource\":{},",
                "\"precedence\":{},",
                "\"causality\":\"not_evaluated\",",
                "\"receipt_integrity\":\"not_evaluated\"",
            "},",
            "\"metrics\":[{}],",
            "\"artifact_digests\":{},",
            "\"supersedes_record_id\":null,",
            "\"adapter_detail\":{",
                "\"policy_status\":{},",
                "\"abstract_audit_status\":{},",
                "\"abstract_feasibility_scope\":\"unary_resource_time_v0\",",
                "\"makespan_tick\":{},",
                "\"total_tardiness_ticks\":{},",
                "\"occupied_resource_ticks\":{},",
                "\"completed_operation_count\":{},",
                "\"assigned_operation_count\":{},",
                "\"mandatory_operation_count\":{},",
                "\"missing_operation_ids\":[{}],",
                "\"assignments\":[{}],",
                "\"audit_diagnostics\":[{}],",
                "\"policy_diagnostics\":[{}],",
                "\"abstract_audit_passed\":{}",
            "}",
            "}"
        ),
        record_id,
        context.synthetic_example,
        json_quote(&context.benchmark_release_id),
        json_quote(&method_id),
        json_quote(&context.episode_id),
        json_quote(&context.replicate_id),
        json_quote(status),
        json_quote(reason),
        json_quote(&context.initial_state_sha256.to_ascii_lowercase()),
        json_quote(&context.disturbance_tape_sha256.to_ascii_lowercase()),
        json_quote(&context.method_config_sha256.to_ascii_lowercase()),
        json_quote(&context.evaluator_revision),
        context.wall_time_limit_ms,
        context.decision_compute_budget_units,
        json_quote(&budget_contract),
        json_quote(resource_gate_status(result)),
        json_quote(precedence_gate_status(result)),
        metrics,
        json_quote(policy_status_id(result.policy_status)),
        json_quote(match result.audit.status {
            shipyard_dispatch_audit::AuditStatus::AbstractFeasible => "abstract_feasible",
            shipyard_dispatch_audit::AuditStatus::Rejected => "rejected",
        }),
        makespan_value,
        result.audit.metrics.total_tardiness_ticks,
        result.audit.metrics.occupied_resource_ticks,
        result.audit.metrics.completed_operation_count,
        result.audit.metrics.assigned_operation_count,
        result.audit.metrics.mandatory_operation_count,
        missing,
        assignments,
        diagnostics,
        policy_diagnostics,
        result.audit.status == shipyard_dispatch_audit::AuditStatus::AbstractFeasible
    )
}

/// Produce deterministic JSON Lines, one raw-run record per baseline outcome.
/// The row uses the result-ledger v0 envelope. Because this adapter does not
/// execute full geometry, causal replay, or receipt-integrity gates, even an
/// abstract-model pass is terminally incomplete, never a physical feasible claim.
/// The caller must append these rows and retain exact runner receipts separately.
pub fn serialize_result_ledger(
    context: &ExperimentLedgerContext,
    results: &[EvaluatedRule],
) -> Result<String, String> {
    for (name, value) in [
        ("benchmark_release_id", context.benchmark_release_id.as_str()),
        ("episode_id", context.episode_id.as_str()),
        ("replicate_id", context.replicate_id.as_str()),
        ("budget_profile_id", context.budget_profile_id.as_str()),
        ("budget_contract", context.budget_contract.as_str()),
        ("policy_revision", context.policy_revision.as_str()),
        ("evaluator_revision", context.evaluator_revision.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{name} must not be empty"));
        }
    }
    require_digest("initial_state_sha256", &context.initial_state_sha256)?;
    require_digest("disturbance_tape_sha256", &context.disturbance_tape_sha256)?;
    require_digest("method_config_sha256", &context.method_config_sha256)?;

    if results.len() != 4 {
        return Err(format!(
            "expected exactly four baseline outcomes, received {}",
            results.len()
        ));
    }
    let mut seen_rules = std::collections::BTreeSet::new();
    for result in results {
        let id = rule_id(result.rule);
        if !seen_rules.insert(id) {
            return Err(format!("duplicate baseline outcome for policy {id}"));
        }
    }
    let mut ordered: Vec<&EvaluatedRule> = results.iter().collect();
    ordered.sort_by_key(|result| rule_id(result.rule));
    let mut output = String::new();
    for result in ordered {
        output.push_str(&encode_result(context, result));
        output.push('\n');
    }
    Ok(output)
}

#[cfg(test)]
mod ledger_tests {
    use super::*;
    use shipyard_dispatch_audit::EvaluatorOutage;
    use shipyard_observed_dispatch::{Commitment, DispatchInput, Operation, Resource, VisibleOutage};

    fn context() -> ExperimentLedgerContext {
        ExperimentLedgerContext {
            benchmark_release_id: "synthetic-dispatch-v0".to_owned(),
            episode_id: "episode-1".to_owned(),
            replicate_id: "seed-0001".to_owned(),
            initial_state_sha256: "a".repeat(64),
            disturbance_tape_sha256: "b".repeat(64),
            budget_profile_id: "no-search-deterministic-v0".to_owned(),
            policy_revision: "policy-revision-sha".to_owned(),
            evaluator_revision: "evaluator-revision-sha".to_owned(),
        }
    }

    fn input() -> DispatchInput {
        DispatchInput {
            decision_tick: 0,
            horizon_end_tick: 10,
            operations: vec![Operation {
                id: "weld".to_owned(),
                order: 0,
                release_tick: 0,
                due_tick: 5,
                duration_ticks: 2,
                resource_kind: "welder".to_owned(),
                predecessor_ids: vec![],
            }],
            resources: vec![Resource {
                id: "w-1".to_owned(),
                kind: "welder".to_owned(),
            }],
            completed: vec![],
            commitments: Vec::<Commitment>::new(),
            visible_outages: vec![],
        }
    }

    #[test]
    fn ledger_emits_four_stable_rows_with_separate_metrics() {
        let results = evaluate_four_rules(&input(), &[], &[]).expect("input aligns");
        let rows = serialize_result_ledger(&context(), &results).expect("valid identity");
        assert_eq!(rows.lines().count(), 4);
        assert!(rows.contains("\"terminal_status\":\"feasible\""));
        assert!(rows.contains("\"feasibility_scope\":\"abstract-unary-resource-time-v0\""));
        assert!(rows.contains("\"total_tardiness_ticks\":0"));
        assert!(rows.contains("\"occupied_resource_ticks\":2"));
        assert!(!rows.contains("winner_score"));
        let keys = rows
            .lines()
            .map(|line| {
                let marker = "\"policy_id\":\"";
                let start = line.find(marker).expect("policy ID") + marker.len();
                let end = line[start..].find('"').expect("quote") + start;
                line[start..end].to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            keys,
            vec!["critical_ratio", "earliest_due_date", "fixed_order", "least_slack"]
        );
        for line in rows.lines() {
            assert!(line.starts_with('{') && line.ends_with('}'));
        }
    }

    #[test]
    fn ledger_preserves_rejections_and_every_candidate_assignment() {
        let mut scenario = input();
        scenario.visible_outages.push(VisibleOutage {
            event_id: "known-outage".to_owned(),
            resource_id: "w-1".to_owned(),
            effective_from_tick: 0,
            effective_until_tick: 5,
            observed_at_tick: 0,
        });
        let evaluator = vec![EvaluatorOutage {
            event_id: "known-outage".to_owned(),
            resource_id: "w-1".to_owned(),
            effective_from_tick: 0,
            effective_until_tick: 5,
        }];
        let results = evaluate_four_rules(&scenario, &[], &evaluator).expect("valid history");
        let rows = serialize_result_ledger(&context(), &results).expect("identity valid");
        assert_eq!(rows.lines().count(), 4);
        assert!(rows.contains("\"terminal_status\":\"invalid_candidate\""));
        assert!(rows.contains("\"ASSIGNMENT_OVERLAPS_EFFECTIVE_OUTAGE\""));
        assert!(rows.contains("\"assignments\":[{"));
    }

    #[test]
    fn ledger_rejects_missing_or_malformed_source_digests() {
        let results = evaluate_four_rules(&input(), &[], &[]).expect("input aligns");
        let mut identity = context();
        identity.disturbance_tape_sha256 = "not-a-digest".to_owned();
        assert!(serialize_result_ledger(&identity, &results)
            .expect_err("invalid tape digest")
            .contains("64 hexadecimal"));
    }

    #[test]
    fn json_string_encoding_is_valid_for_controls_quotes_and_unicode() {
        let encoded = json_quote("quote=\"; slash=\\; line=\n; rocket=🚀");
        assert_eq!(
            encoded,
            r#""quote=\"; slash=\\; line=\n; rocket=🚀""#
        );
    }

    #[test]
    fn ledger_requires_all_four_unique_baseline_outcomes() {
        let results = evaluate_four_rules(&input(), &[], &[]).expect("input aligns");
        assert!(serialize_result_ledger(&context(), &results[..3])
            .expect_err("missing policy outcome")
            .contains("expected exactly four"));

        let mut duplicate = results.clone();
        duplicate[1].rule = duplicate[0].rule;
        assert!(serialize_result_ledger(&context(), &duplicate)
            .expect_err("duplicate policy outcome")
            .contains("duplicate baseline outcome"));
    }

}
