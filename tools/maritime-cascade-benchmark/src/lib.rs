#![forbid(unsafe_code)]

//! Deterministic, dependency-free reference propagation for synthetic port-call
//! service dependencies. This is a fixed-schedule counterfactual, not a planner:
//! it reports knock-on delay and physical-resource overlap but never repairs or
//! writes a real schedule.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub id: String,
    pub scheduled_start_minute: i64,
    pub duration_minutes: i64,
    /// Canonical physical resource IDs; aliases must be resolved before input.
    pub resource_ids: Vec<String>,
    /// Optional latest service-start minute.
    pub deadline_minute: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dependency {
    pub predecessor_id: String,
    pub successor_id: String,
    pub lag_minutes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Shock {
    pub id: String,
    pub operation_id: String,
    pub additional_start_delay_minutes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelError {
    EmptyOperations,
    InvalidIdentifier { kind: &'static str, value: String },
    DuplicateOperationId(String),
    DuplicateResource { operation_id: String, resource_id: String },
    NegativeDuration(String),
    NegativeLag { predecessor_id: String, successor_id: String },
    MissingOperation { role: &'static str, operation_id: String },
    DuplicateDependency { predecessor_id: String, successor_id: String },
    Cycle,
    DuplicateShockId(String),
    NegativeShockDelay(String),
    ArithmeticOverflow,
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ModelError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationImpact {
    pub operation_id: String,
    pub baseline_start_minute: i64,
    pub scenario_start_minute: i64,
    pub scenario_end_minute: i64,
    pub delay_minutes: i64,
    pub direct_shock_minutes: i64,
    pub deadline_lateness_minutes: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ResourceConflict {
    pub resource_id: String,
    pub overlap_start_minute: i64,
    pub overlap_end_minute: i64,
    /// The two operation IDs are lexically canonicalized, not temporally ranked.
    pub operation_a_id: String,
    pub operation_b_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioMetrics {
    pub direct_shock_operations: usize,
    pub affected_operations: usize,
    pub downstream_delay_minutes: u128,
    pub missed_deadlines: usize,
    pub resource_conflicts: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioReport {
    pub case_id: String,
    /// Sorted by operation ID for deterministic receipts.
    pub operations: Vec<OperationImpact>,
    pub metrics: ScenarioMetrics,
    /// Sorted by resource, overlap start, and operation identity.
    pub resource_conflicts: Vec<ResourceConflict>,
}

impl ScenarioReport {
    /// Canonical JSON: stable key order, no floats, and identifiers restricted to
    /// an ASCII allowlist by validation. The frozen receipt is compared bytewise.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        write!(&mut out, "{{\"case_id\":\"{}\",\"operations\":[", self.case_id)
            .expect("writing into String cannot fail");

        for (index, item) in self.operations.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            write!(
                &mut out,
                "{{\"id\":\"{}\",\"baseline_start_minute\":{},\"scenario_start_minute\":{},\"scenario_end_minute\":{},\"delay_minutes\":{},\"direct_shock_minutes\":{},\"deadline_lateness_minutes\":{}}}",
                item.operation_id,
                item.baseline_start_minute,
                item.scenario_start_minute,
                item.scenario_end_minute,
                item.delay_minutes,
                item.direct_shock_minutes,
                item.deadline_lateness_minutes
            )
            .expect("writing into String cannot fail");
        }

        write!(
            &mut out,
            "],\"metrics\":{{\"direct_shock_operations\":{},\"affected_operations\":{},\"downstream_delay_minutes\":{},\"missed_deadlines\":{},\"resource_conflicts\":{}}},\"resource_conflicts\":[",
            self.metrics.direct_shock_operations,
            self.metrics.affected_operations,
            self.metrics.downstream_delay_minutes,
            self.metrics.missed_deadlines,
            self.metrics.resource_conflicts
        )
        .expect("writing into String cannot fail");

        for (index, conflict) in self.resource_conflicts.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            write!(
                &mut out,
                "{{\"resource_id\":\"{}\",\"operation_a_id\":\"{}\",\"operation_b_id\":\"{}\",\"overlap_start_minute\":{},\"overlap_end_minute\":{}}}",
                conflict.resource_id,
                conflict.operation_a_id,
                conflict.operation_b_id,
                conflict.overlap_start_minute,
                conflict.overlap_end_minute
            )
            .expect("writing into String cannot fail");
        }

        out.push_str("]}");
        out
    }
}

#[derive(Clone, Debug)]
struct Timing {
    start_minute: i64,
    end_minute: i64,
}

#[derive(Clone, Debug)]
struct ValidatedModel {
    operations: BTreeMap<String, Operation>,
    predecessors: BTreeMap<String, Vec<(String, i64)>>,
    topological_order: Vec<String>,
}

fn validate_identifier(kind: &'static str, value: &str) -> Result<(), ModelError> {
    let allowed = value.bytes().all(|byte| {
        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-')
    });
    if value.is_empty() || !allowed {
        return Err(ModelError::InvalidIdentifier {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn validate_model(
    operations: &[Operation],
    dependencies: &[Dependency],
) -> Result<ValidatedModel, ModelError> {
    if operations.is_empty() {
        return Err(ModelError::EmptyOperations);
    }

    let mut operation_map = BTreeMap::new();
    for operation in operations {
        validate_identifier("operation", &operation.id)?;
        if operation.duration_minutes < 0 {
            return Err(ModelError::NegativeDuration(operation.id.clone()));
        }

        let mut resources = BTreeSet::new();
        for resource_id in &operation.resource_ids {
            validate_identifier("resource", resource_id)?;
            if !resources.insert(resource_id.clone()) {
                return Err(ModelError::DuplicateResource {
                    operation_id: operation.id.clone(),
                    resource_id: resource_id.clone(),
                });
            }
        }

        if operation_map
            .insert(operation.id.clone(), operation.clone())
            .is_some()
        {
            return Err(ModelError::DuplicateOperationId(operation.id.clone()));
        }
    }

    let mut predecessor_map: BTreeMap<String, Vec<(String, i64)>> = operation_map
        .keys()
        .cloned()
        .map(|id| (id, Vec::new()))
        .collect();
    let mut successor_map: BTreeMap<String, Vec<String>> = operation_map
        .keys()
        .cloned()
        .map(|id| (id, Vec::new()))
        .collect();
    let mut indegree: BTreeMap<String, usize> =
        operation_map.keys().cloned().map(|id| (id, 0)).collect();
    let mut seen_edges = BTreeSet::new();

    for dependency in dependencies {
        validate_identifier("dependency predecessor", &dependency.predecessor_id)?;
        validate_identifier("dependency successor", &dependency.successor_id)?;
        if dependency.lag_minutes < 0 {
            return Err(ModelError::NegativeLag {
                predecessor_id: dependency.predecessor_id.clone(),
                successor_id: dependency.successor_id.clone(),
            });
        }
        if !operation_map.contains_key(&dependency.predecessor_id) {
            return Err(ModelError::MissingOperation {
                role: "dependency predecessor",
                operation_id: dependency.predecessor_id.clone(),
            });
        }
        if !operation_map.contains_key(&dependency.successor_id) {
            return Err(ModelError::MissingOperation {
                role: "dependency successor",
                operation_id: dependency.successor_id.clone(),
            });
        }
        if dependency.predecessor_id == dependency.successor_id {
            return Err(ModelError::Cycle);
        }
        let edge_key = (
            dependency.predecessor_id.clone(),
            dependency.successor_id.clone(),
        );
        if !seen_edges.insert(edge_key) {
            return Err(ModelError::DuplicateDependency {
                predecessor_id: dependency.predecessor_id.clone(),
                successor_id: dependency.successor_id.clone(),
            });
        }

        predecessor_map
            .get_mut(&dependency.successor_id)
            .expect("validated operation exists")
            .push((
                dependency.predecessor_id.clone(),
                dependency.lag_minutes,
            ));
        successor_map
            .get_mut(&dependency.predecessor_id)
            .expect("validated operation exists")
            .push(dependency.successor_id.clone());
        *indegree
            .get_mut(&dependency.successor_id)
            .expect("validated operation exists") += 1;
    }

    for parents in predecessor_map.values_mut() {
        parents.sort();
    }
    for children in successor_map.values_mut() {
        children.sort();
    }

    // Kahn's algorithm with an ordered ready set makes topological replay
    // independent of caller input order.
    let mut ready = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
        .collect::<BTreeSet<_>>();
    let mut topological_order = Vec::with_capacity(operation_map.len());
    while let Some(id) = ready.pop_first() {
        topological_order.push(id.clone());
        for successor in successor_map.get(&id).expect("operation map is complete") {
            let degree = indegree
                .get_mut(successor)
                .expect("validated operation exists");
            *degree -= 1;
            if *degree == 0 {
                ready.insert(successor.clone());
            }
        }
    }
    if topological_order.len() != operation_map.len() {
        return Err(ModelError::Cycle);
    }

    Ok(ValidatedModel {
        operations: operation_map,
        predecessors: predecessor_map,
        topological_order,
    })
}

fn aggregate_shocks(
    model: &ValidatedModel,
    shocks: &[Shock],
) -> Result<BTreeMap<String, i64>, ModelError> {
    let mut shock_ids = BTreeSet::new();
    let mut delays = BTreeMap::new();

    for shock in shocks {
        validate_identifier("shock", &shock.id)?;
        if !shock_ids.insert(shock.id.clone()) {
            return Err(ModelError::DuplicateShockId(shock.id.clone()));
        }
        if shock.additional_start_delay_minutes < 0 {
            return Err(ModelError::NegativeShockDelay(shock.id.clone()));
        }
        if !model.operations.contains_key(&shock.operation_id) {
            return Err(ModelError::MissingOperation {
                role: "shock target",
                operation_id: shock.operation_id.clone(),
            });
        }

        let current = delays.entry(shock.operation_id.clone()).or_insert(0_i64);
        *current = current
            .checked_add(shock.additional_start_delay_minutes)
            .ok_or(ModelError::ArithmeticOverflow)?;
    }

    Ok(delays)
}

fn evaluate_timeline(
    model: &ValidatedModel,
    delay_floor_by_operation: &BTreeMap<String, i64>,
) -> Result<BTreeMap<String, Timing>, ModelError> {
    let mut timings = BTreeMap::new();

    for operation_id in &model.topological_order {
        let operation = model
            .operations
            .get(operation_id)
            .expect("topological order refers to validated operation");
        let shock_delay = delay_floor_by_operation
            .get(operation_id)
            .copied()
            .unwrap_or(0);
        let shifted_start = operation
            .scheduled_start_minute
            .checked_add(shock_delay)
            .ok_or(ModelError::ArithmeticOverflow)?;
        let mut start_minute = shifted_start;

        for (predecessor_id, lag_minutes) in model
            .predecessors
            .get(operation_id)
            .expect("validated predecessor list exists")
        {
            let predecessor = timings
                .get(predecessor_id)
                .expect("predecessor appears earlier in topological order");
            let ready_minute = predecessor
                .end_minute
                .checked_add(*lag_minutes)
                .ok_or(ModelError::ArithmeticOverflow)?;
            start_minute = start_minute.max(ready_minute);
        }

        let end_minute = start_minute
            .checked_add(operation.duration_minutes)
            .ok_or(ModelError::ArithmeticOverflow)?;
        timings.insert(
            operation_id.clone(),
            Timing {
                start_minute,
                end_minute,
            },
        );
    }

    Ok(timings)
}

fn find_resource_conflicts(
    model: &ValidatedModel,
    timings: &BTreeMap<String, Timing>,
) -> Vec<ResourceConflict> {
    let mut intervals: BTreeMap<String, Vec<(i64, i64, String)>> = BTreeMap::new();

    for (operation_id, timing) in timings {
        if timing.start_minute == timing.end_minute {
            continue;
        }
        let operation = model
            .operations
            .get(operation_id)
            .expect("timing refers to validated operation");
        for resource_id in &operation.resource_ids {
            intervals.entry(resource_id.clone()).or_default().push((
                timing.start_minute,
                timing.end_minute,
                operation_id.clone(),
            ));
        }
    }

    let mut conflicts = Vec::new();
    for (resource_id, mut items) in intervals {
        items.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then(left.1.cmp(&right.1))
                .then(left.2.cmp(&right.2))
        });

        // Enumerate actual overlaps. Non-overlapping resources scan in O(n log n);
        // if all intervals overlap, output size itself is quadratic.
        for left_index in 0..items.len() {
            let (left_start, left_end, left_id) = &items[left_index];
            for (right_start, right_end, right_id) in items.iter().skip(left_index + 1) {
                if right_start >= left_end {
                    break;
                }
                let overlap_start_minute = (*left_start).max(*right_start);
                let overlap_end_minute = (*left_end).min(*right_end);
                if overlap_start_minute >= overlap_end_minute {
                    continue;
                }
                let (operation_a_id, operation_b_id) = if left_id <= right_id {
                    (left_id.clone(), right_id.clone())
                } else {
                    (right_id.clone(), left_id.clone())
                };
                conflicts.push(ResourceConflict {
                    resource_id: resource_id.clone(),
                    overlap_start_minute,
                    overlap_end_minute,
                    operation_a_id,
                    operation_b_id,
                });
            }
        }
    }

    conflicts.sort();
    conflicts
}

/// Replays a frozen plan without a shock and with the supplied shock set, then
/// reports the difference. This does not reschedule peers to resolve conflicts.
pub fn evaluate_scenario(
    case_id: &str,
    operations: &[Operation],
    dependencies: &[Dependency],
    shocks: &[Shock],
) -> Result<ScenarioReport, ModelError> {
    validate_identifier("case", case_id)?;
    let model = validate_model(operations, dependencies)?;
    let shock_delays = aggregate_shocks(&model, shocks)?;
    let baseline = evaluate_timeline(&model, &BTreeMap::new())?;
    let scenario = evaluate_timeline(&model, &shock_delays)?;

    let mut impacts = Vec::with_capacity(model.operations.len());
    for (operation_id, operation) in &model.operations {
        let baseline_timing = baseline.get(operation_id).expect("baseline is complete");
        let scenario_timing = scenario.get(operation_id).expect("scenario is complete");
        let delay_minutes = scenario_timing
            .start_minute
            .checked_sub(baseline_timing.start_minute)
            .ok_or(ModelError::ArithmeticOverflow)?;
        if delay_minutes < 0 {
            return Err(ModelError::ArithmeticOverflow);
        }
        let direct_shock_minutes = shock_delays.get(operation_id).copied().unwrap_or(0);
        let deadline_lateness_minutes = match operation.deadline_minute {
            Some(deadline) if scenario_timing.start_minute > deadline => scenario_timing
                .start_minute
                .checked_sub(deadline)
                .ok_or(ModelError::ArithmeticOverflow)?,
            _ => 0,
        };
        impacts.push(OperationImpact {
            operation_id: operation_id.clone(),
            baseline_start_minute: baseline_timing.start_minute,
            scenario_start_minute: scenario_timing.start_minute,
            scenario_end_minute: scenario_timing.end_minute,
            delay_minutes,
            direct_shock_minutes,
            deadline_lateness_minutes,
        });
    }

    let direct_shock_operations = shock_delays.values().filter(|delay| **delay > 0).count();
    let affected_operations = impacts
        .iter()
        .filter(|impact| impact.delay_minutes > 0)
        .count();
    let mut downstream_delay_minutes = 0_u128;
    for impact in &impacts {
        if impact.direct_shock_minutes == 0 {
            downstream_delay_minutes = downstream_delay_minutes
                .checked_add(
                    u128::try_from(impact.delay_minutes).map_err(|_| ModelError::ArithmeticOverflow)?,
                )
                .ok_or(ModelError::ArithmeticOverflow)?;
        }
    }
    let missed_deadlines = impacts
        .iter()
        .filter(|impact| impact.deadline_lateness_minutes > 0)
        .count();
    let resource_conflicts = find_resource_conflicts(&model, &scenario);
    let metrics = ScenarioMetrics {
        direct_shock_operations,
        affected_operations,
        downstream_delay_minutes,
        missed_deadlines,
        resource_conflicts: resource_conflicts.len(),
    };

    Ok(ScenarioReport {
        case_id: case_id.to_owned(),
        operations: impacts,
        metrics,
        resource_conflicts,
    })
}

/// Hand-checkable frozen scenario used for local and CI receipt qualification.
pub fn run_demo() -> Result<ScenarioReport, ModelError> {
    let operations = vec![
        Operation {
            id: "BERTH-A".to_owned(),
            scheduled_start_minute: 0,
            duration_minutes: 30,
            resource_ids: vec!["BERTH-RESOURCE-1".to_owned()],
            deadline_minute: None,
        },
        Operation {
            id: "PILOT-1".to_owned(),
            scheduled_start_minute: 30,
            duration_minutes: 15,
            resource_ids: vec!["PILOT-RESOURCE-1".to_owned()],
            deadline_minute: None,
        },
        Operation {
            id: "CRANE-1".to_owned(),
            scheduled_start_minute: 45,
            duration_minutes: 20,
            resource_ids: vec!["CRANE-RESOURCE-1".to_owned()],
            deadline_minute: None,
        },
        Operation {
            id: "CUTOFF-RAIL-1".to_owned(),
            scheduled_start_minute: 70,
            duration_minutes: 0,
            resource_ids: Vec::new(),
            deadline_minute: Some(70),
        },
        Operation {
            id: "CRANE-2".to_owned(),
            scheduled_start_minute: 70,
            duration_minutes: 15,
            resource_ids: vec!["CRANE-RESOURCE-1".to_owned()],
            deadline_minute: None,
        },
        Operation {
            id: "ISOLATED-CONTROL".to_owned(),
            scheduled_start_minute: 100,
            duration_minutes: 10,
            resource_ids: vec!["CONTROL-RESOURCE".to_owned()],
            deadline_minute: None,
        },
    ];
    let dependencies = vec![
        Dependency {
            predecessor_id: "BERTH-A".to_owned(),
            successor_id: "PILOT-1".to_owned(),
            lag_minutes: 0,
        },
        Dependency {
            predecessor_id: "PILOT-1".to_owned(),
            successor_id: "CRANE-1".to_owned(),
            lag_minutes: 0,
        },
        Dependency {
            predecessor_id: "CRANE-1".to_owned(),
            successor_id: "CUTOFF-RAIL-1".to_owned(),
            lag_minutes: 0,
        },
    ];
    let shocks = vec![Shock {
        id: "DELAY-BERTH-20".to_owned(),
        operation_id: "BERTH-A".to_owned(),
        additional_start_delay_minutes: 20,
    }];

    evaluate_scenario(
        "cascade-linear-shock-v0.1",
        &operations,
        &dependencies,
        &shocks,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation(
        id: &str,
        start: i64,
        duration: i64,
        resources: &[&str],
        deadline: Option<i64>,
    ) -> Operation {
        Operation {
            id: id.to_owned(),
            scheduled_start_minute: start,
            duration_minutes: duration,
            resource_ids: resources.iter().map(|value| (*value).to_owned()).collect(),
            deadline_minute: deadline,
        }
    }

    fn dependency(predecessor: &str, successor: &str, lag: i64) -> Dependency {
        Dependency {
            predecessor_id: predecessor.to_owned(),
            successor_id: successor.to_owned(),
            lag_minutes: lag,
        }
    }

    fn shock(id: &str, operation_id: &str, delay: i64) -> Shock {
        Shock {
            id: id.to_owned(),
            operation_id: operation_id.to_owned(),
            additional_start_delay_minutes: delay,
        }
    }

    #[test]
    fn demo_matches_frozen_receipt_and_hand_calculated_effects() {
        let report = run_demo().unwrap();
        assert_eq!(
            report.to_json(),
            include_str!("../fixtures/linear-shock-v0.1.expected.json").trim_end()
        );
        assert_eq!(report.metrics.direct_shock_operations, 1);
        assert_eq!(report.metrics.affected_operations, 4);
        assert_eq!(report.metrics.downstream_delay_minutes, 55);
        assert_eq!(report.metrics.missed_deadlines, 1);
        assert_eq!(report.metrics.resource_conflicts, 1);
    }

    #[test]
    fn declared_slack_absorbs_a_delay_without_cascading() {
        let operations = vec![
            operation("A", 0, 10, &["R-A"], None),
            operation("B", 20, 5, &["R-B"], None),
        ];
        let report = evaluate_scenario(
            "slack-control",
            &operations,
            &[dependency("A", "B", 0)],
            &[shock("delay-A", "A", 5)],
        )
        .unwrap();
        let b = report.operations.iter().find(|item| item.operation_id == "B").unwrap();
        assert_eq!(b.baseline_start_minute, 20);
        assert_eq!(b.scenario_start_minute, 20);
        assert_eq!(b.delay_minutes, 0);
    }

    #[test]
    fn half_open_back_to_back_resource_intervals_do_not_conflict() {
        let operations = vec![
            operation("A", 0, 10, &["CRANE"], None),
            operation("B", 10, 10, &["CRANE"], None),
        ];
        let report = evaluate_scenario("back-to-back", &operations, &[], &[]).unwrap();
        assert!(report.resource_conflicts.is_empty());
    }

    #[test]
    fn overlapping_intervals_on_one_canonical_resource_are_reported() {
        let operations = vec![
            operation("A", 0, 10, &["CRANE"], None),
            operation("B", 5, 10, &["CRANE"], None),
        ];
        let report = evaluate_scenario("overlap", &operations, &[], &[]).unwrap();
        assert_eq!(report.resource_conflicts.len(), 1);
        assert_eq!(report.resource_conflicts[0].overlap_start_minute, 5);
        assert_eq!(report.resource_conflicts[0].overlap_end_minute, 10);
    }

    #[test]
    fn cycles_and_dangling_edges_fail_closed() {
        let operations = vec![operation("A", 0, 1, &[], None), operation("B", 1, 1, &[], None)];
        assert_eq!(
            evaluate_scenario(
                "cycle",
                &operations,
                &[dependency("A", "B", 0), dependency("B", "A", 0)],
                &[],
            ),
            Err(ModelError::Cycle)
        );
        assert_eq!(
            evaluate_scenario("dangling", &operations, &[dependency("A", "MISSING", 0)], &[]),
            Err(ModelError::MissingOperation {
                role: "dependency successor",
                operation_id: "MISSING".to_owned(),
            })
        );
    }

    #[test]
    fn input_order_permutations_have_identical_canonical_output() {
        let operations = vec![
            operation("A", 0, 10, &["R-A"], None),
            operation("B", 10, 10, &["R-B"], Some(25)),
            operation("C", 20, 5, &["R-C"], None),
        ];
        let dependencies = vec![dependency("A", "B", 0), dependency("B", "C", 0)];
        let shocks = vec![shock("shock-1", "A", 3), shock("shock-2", "A", 4)];
        let first = evaluate_scenario("permutation", &operations, &dependencies, &shocks).unwrap();

        let mut permuted_operations = operations.clone();
        permuted_operations.reverse();
        let mut permuted_dependencies = dependencies.clone();
        permuted_dependencies.reverse();
        let mut permuted_shocks = shocks.clone();
        permuted_shocks.reverse();
        let second = evaluate_scenario(
            "permutation",
            &permuted_operations,
            &permuted_dependencies,
            &permuted_shocks,
        )
        .unwrap();
        assert_eq!(first.to_json(), second.to_json());
        assert_eq!(first.operations[0].direct_shock_minutes, 7);
    }

    #[test]
    fn duplicate_ids_resources_and_shock_ids_are_rejected() {
        let duplicate_ops = vec![
            operation("A", 0, 1, &[], None),
            operation("A", 2, 1, &[], None),
        ];
        assert_eq!(
            evaluate_scenario("duplicate-op", &duplicate_ops, &[], &[]),
            Err(ModelError::DuplicateOperationId("A".to_owned()))
        );

        let duplicate_resources = vec![operation("A", 0, 1, &["R", "R"], None)];
        assert_eq!(
            evaluate_scenario("duplicate-resource", &duplicate_resources, &[], &[]),
            Err(ModelError::DuplicateResource {
                operation_id: "A".to_owned(),
                resource_id: "R".to_owned(),
            })
        );

        let operations = vec![operation("A", 0, 1, &[], None)];
        assert_eq!(
            evaluate_scenario(
                "duplicate-shock",
                &operations,
                &[],
                &[shock("SAME", "A", 1), shock("SAME", "A", 2)],
            ),
            Err(ModelError::DuplicateShockId("SAME".to_owned()))
        );
    }

    #[test]
    fn arithmetic_overflow_is_not_silently_saturated() {
        let operations = vec![operation("A", i64::MAX, 1, &[], None)];
        assert_eq!(
            evaluate_scenario("overflow", &operations, &[], &[]),
            Err(ModelError::ArithmeticOverflow)
        );
    }
}
