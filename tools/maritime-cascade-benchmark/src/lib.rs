#![forbid(unsafe_code)]

//! Deterministic, dependency-free reference propagation for synthetic port-call
//! service dependencies. This is a fixed-schedule counterfactual, not a planner:
//! it reports knock-on delay and physical-resource overlap but never repairs or
//! writes a real schedule.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CausalDriver {
    DirectShock { shock_id: String },
    DelayedPredecessor { operation_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationImpact {
    pub operation_id: String,
    pub baseline_start_minute: i64,
    pub scenario_start_minute: i64,
    pub scenario_end_minute: i64,
    pub delay_minutes: i64,
    pub direct_shock_minutes: i64,
    pub deadline_lateness_minutes: i64,
    /// Only binding inputs that increased this operation's start versus baseline.
    /// Follow DelayedPredecessor references to reconstruct a causal chain.
    pub causal_drivers: Vec<CausalDriver>,
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
            let mut causes_json = String::new();
            for (cause_index, cause) in item.causal_drivers.iter().enumerate() {
                if cause_index > 0 {
                    causes_json.push(',');
                }
                match cause {
                    CausalDriver::DirectShock { shock_id } => {
                        write!(
                            &mut causes_json,
                            "{{\"kind\":\"direct_shock\",\"id\":\"{}\"}}",
                            shock_id
                        )
                        .expect("writing into String cannot fail");
                    }
                    CausalDriver::DelayedPredecessor { operation_id } => {
                        write!(
                            &mut causes_json,
                            "{{\"kind\":\"delayed_predecessor\",\"id\":\"{}\"}}",
                            operation_id
                        )
                        .expect("writing into String cannot fail");
                    }
                }
            }
            write!(
                &mut out,
                "{{\"id\":\"{}\",\"baseline_start_minute\":{},\"scenario_start_minute\":{},\"scenario_end_minute\":{},\"delay_minutes\":{},\"direct_shock_minutes\":{},\"deadline_lateness_minutes\":{},\"causal_drivers\":[{}]}}",
                item.operation_id,
                item.baseline_start_minute,
                item.scenario_start_minute,
                item.scenario_end_minute,
                item.delay_minutes,
                item.direct_shock_minutes,
                item.deadline_lateness_minutes,
                causes_json
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

fn causal_drivers_for(
    operation_id: &str,
    operation: &Operation,
    shocks: &[Shock],
    shock_delays: &BTreeMap<String, i64>,
    model: &ValidatedModel,
    baseline: &BTreeMap<String, Timing>,
    scenario: &BTreeMap<String, Timing>,
) -> Result<Vec<CausalDriver>, ModelError> {
    let baseline_start = baseline
        .get(operation_id)
        .expect("baseline contains validated operation")
        .start_minute;
    let scenario_start = scenario
        .get(operation_id)
        .expect("scenario contains validated operation")
        .start_minute;
    let shock_delay = shock_delays.get(operation_id).copied().unwrap_or(0);
    let shifted_start = operation
        .scheduled_start_minute
        .checked_add(shock_delay)
        .ok_or(ModelError::ArithmeticOverflow)?;

    let mut drivers = Vec::new();

    // A local shock is causal only if its shifted scheduled start is a binding
    // constraint and it actually changed this operation's start versus baseline.
    if shock_delay > 0 && shifted_start == scenario_start && scenario_start > baseline_start {
        for shock in shocks
            .iter()
            .filter(|shock| shock.operation_id == operation_id && shock.additional_start_delay_minutes > 0)
        {
            drivers.push(CausalDriver::DirectShock {
                shock_id: shock.id.clone(),
            });
        }
    }

    // A predecessor is causal only when its finish constraint binds the scenario
    // start and is later than the same predecessor's baseline finish constraint.
    for (predecessor_id, lag_minutes) in model
        .predecessors
        .get(operation_id)
        .expect("validated predecessor list exists")
    {
        let baseline_predecessor = baseline
            .get(predecessor_id)
            .expect("baseline contains validated predecessor");
        let scenario_predecessor = scenario
            .get(predecessor_id)
            .expect("scenario contains validated predecessor");
        let baseline_ready = baseline_predecessor
            .end_minute
            .checked_add(*lag_minutes)
            .ok_or(ModelError::ArithmeticOverflow)?;
        let scenario_ready = scenario_predecessor
            .end_minute
            .checked_add(*lag_minutes)
            .ok_or(ModelError::ArithmeticOverflow)?;
        if scenario_ready == scenario_start && scenario_ready > baseline_ready {
            drivers.push(CausalDriver::DelayedPredecessor {
                operation_id: predecessor_id.clone(),
            });
        }
    }

    drivers.sort();
    drivers.dedup();
    Ok(drivers)
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
        items.sort_by_key(|item| (item.0, item.1, item.2.clone()));

        // Enumerate actual overlaps. Non-overlapping resources scan in O(n log n);
        // if all intervals overlap, output size itself is quadratic.
        for (left_index, (left_start, left_end, left_id)) in items.iter().enumerate() {
            for (right_start, right_end, right_id) in items.iter().skip(left_index + 1) {
                if *right_start >= *left_end {
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

    conflicts.sort_by_key(|conflict| (
        conflict.resource_id.clone(),
        conflict.overlap_start_minute,
        conflict.operation_a_id.clone(),
        conflict.operation_b_id.clone(),
        conflict.overlap_end_minute,
    ));
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
        let causal_drivers = causal_drivers_for(
            operation_id,
            operation,
            shocks,
            &shock_delays,
            &model,
            &baseline,
            &scenario,
        )?;
        impacts.push(OperationImpact {
            operation_id: operation_id.clone(),
            baseline_start_minute: baseline_timing.start_minute,
            scenario_start_minute: scenario_timing.start_minute,
            scenario_end_minute: scenario_timing.end_minute,
            delay_minutes,
            direct_shock_minutes,
            deadline_lateness_minutes,
            causal_drivers,
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
    fn independent_relaxation_oracle(
        operations: &[Operation],
        dependencies: &[Dependency],
        shocks: &[Shock],
    ) -> Vec<i64> {
        // Independent fixed-point formulation: begin at each operation's
        // scheduled start and repeatedly enforce predecessor completion.
        // This intentionally does not call the production topological traversal.
        let mut starts = operations
            .iter()
            .map(|operation| operation.scheduled_start_minute)
            .collect::<Vec<_>>();

        for shock in shocks {
            let target = operations
                .iter()
                .position(|operation| operation.id == shock.operation_id)
                .expect("fixture shock target exists");
            starts[target] = starts[target]
                .checked_add(shock.additional_start_delay_minutes)
                .expect("small oracle fixture does not overflow");
        }

        let iteration_limit = operations
            .len()
            .saturating_mul(operations.len())
            .saturating_add(1);
        for _ in 0..iteration_limit {
            let mut changed = false;
            for dependency in dependencies {
                let predecessor = operations
                    .iter()
                    .position(|operation| operation.id == dependency.predecessor_id)
                    .expect("fixture predecessor exists");
                let successor = operations
                    .iter()
                    .position(|operation| operation.id == dependency.successor_id)
                    .expect("fixture successor exists");
                let required_start = starts[predecessor]
                    .checked_add(operations[predecessor].duration_minutes)
                    .and_then(|end| end.checked_add(dependency.lag_minutes))
                    .expect("small oracle fixture does not overflow");
                if starts[successor] < required_start {
                    starts[successor] = required_start;
                    changed = true;
                }
            }
            if !changed {
                return starts;
            }
        }

        panic!("acyclic tiny oracle fixture did not converge");
    }

    #[test]
    fn exhaustive_tiny_dag_sweep_matches_independent_fixed_point_oracle() {
        let mut configurations = 0usize;

        for node_count in 1..=4 {
            let possible_edges = (0..node_count)
                .flat_map(|from| ((from + 1)..node_count).map(move |to| (from, to)))
                .collect::<Vec<_>>();
            let graph_count = 1usize << possible_edges.len();

            for edge_mask in 0..graph_count {
                let operations = (0..node_count)
                    .map(|index| Operation {
                        id: format!("OP-{index}"),
                        scheduled_start_minute: i64::try_from(index * 2).unwrap(),
                        duration_minutes: i64::try_from((index * 3) % 4 + 1).unwrap(),
                        resource_ids: Vec::new(),
                        deadline_minute: None,
                    })
                    .collect::<Vec<_>>();
                let dependencies = possible_edges
                    .iter()
                    .enumerate()
                    .filter(|(edge_index, _)| (edge_mask & (1usize << edge_index)) != 0)
                    .map(|(_, (from, to))| Dependency {
                        predecessor_id: format!("OP-{from}"),
                        successor_id: format!("OP-{to}"),
                        lag_minutes: i64::try_from((from + to) % 2).unwrap(),
                    })
                    .collect::<Vec<_>>();

                let baseline = independent_relaxation_oracle(&operations, &dependencies, &[]);
                for target in 0..node_count {
                    let target_id = format!("OP-{target}");
                    for delay in [0_i64, 1, 3] {
                        let shocks = vec![shock(
                            &format!("SHOCK-{target}-{delay}"),
                            &target_id,
                            delay,
                        )];
                        let scenario =
                            independent_relaxation_oracle(&operations, &dependencies, &shocks);
                        let case_id =
                            format!("oracle-n{node_count}-mask{edge_mask}-target{target}-delay{delay}");
                        let report = evaluate_scenario(
                            &case_id,
                            &operations,
                            &dependencies,
                            &shocks,
                        )
                        .unwrap();

                        let mut expected_affected = 0usize;
                        let mut expected_downstream_delay = 0_u128;
                        for (index, operation) in operations.iter().enumerate() {
                            let impact = report
                                .operations
                                .iter()
                                .find(|item| item.operation_id == operation.id)
                                .expect("report contains each operation");
                            let expected_delay = scenario[index] - baseline[index];
                            let expected_end = scenario[index] + operation.duration_minutes;
                            let expected_direct_shock =
                                if operation.id == target_id { delay } else { 0 };

                            assert_eq!(
                                impact.baseline_start_minute, baseline[index],
                                "baseline mismatch in {case_id}"
                            );
                            assert_eq!(
                                impact.scenario_start_minute, scenario[index],
                                "scenario mismatch in {case_id}"
                            );
                            assert_eq!(
                                impact.scenario_end_minute, expected_end,
                                "end mismatch in {case_id}"
                            );
                            assert_eq!(
                                impact.delay_minutes, expected_delay,
                                "delay mismatch in {case_id}"
                            );
                            assert_eq!(
                                impact.direct_shock_minutes, expected_direct_shock,
                                "direct shock mismatch in {case_id}"
                            );

                            if expected_delay > 0 {
                                expected_affected += 1;
                            }
                            if operation.id != target_id {
                                expected_downstream_delay +=
                                    u128::try_from(expected_delay).unwrap();
                            }
                        }

                        assert_eq!(
                            report.metrics.affected_operations, expected_affected,
                            "affected count mismatch in {case_id}"
                        );
                        assert_eq!(
                            report.metrics.downstream_delay_minutes, expected_downstream_delay,
                            "downstream delay mismatch in {case_id}"
                        );
                        assert_eq!(report.metrics.missed_deadlines, 0);
                        assert_eq!(report.metrics.resource_conflicts, 0);
                        configurations += 1;
                    }
                }
            }
        }

        assert_eq!(configurations, 855);
    }

    #[test]
    fn causal_attribution_distinguishes_propagation_from_resource_coincidence() {
        let report = run_demo().unwrap();
        let causes = |id: &str| {
            report
                .operations
                .iter()
                .find(|item| item.operation_id == id)
                .expect("known demo operation")
                .causal_drivers
                .clone()
        };

        assert_eq!(
            causes("BERTH-A"),
            vec![CausalDriver::DirectShock {
                shock_id: "DELAY-BERTH-20".to_owned(),
            }]
        );
        assert_eq!(
            causes("PILOT-1"),
            vec![CausalDriver::DelayedPredecessor {
                operation_id: "BERTH-A".to_owned(),
            }]
        );
        assert_eq!(
            causes("CRANE-1"),
            vec![CausalDriver::DelayedPredecessor {
                operation_id: "PILOT-1".to_owned(),
            }]
        );
        assert_eq!(
            causes("CUTOFF-RAIL-1"),
            vec![CausalDriver::DelayedPredecessor {
                operation_id: "CRANE-1".to_owned(),
            }]
        );
        // Sharing a crane explains the overlap, not the start-time cause.
        assert!(causes("CRANE-2").is_empty());
        assert!(causes("ISOLATED-CONTROL").is_empty());
    }

    #[test]
    fn equally_binding_delayed_predecessors_are_both_attributed() {
        let operations = vec![
            operation("A", 0, 10, &[], None),
            operation("B", 0, 10, &[], None),
            operation("C", 10, 5, &[], None),
        ];
        let report = evaluate_scenario(
            "fanin-tie",
            &operations,
            &[dependency("A", "C", 0), dependency("B", "C", 0)],
            &[shock("shock-A", "A", 5), shock("shock-B", "B", 5)],
        )
        .unwrap();
        let c = report.operations.iter().find(|item| item.operation_id == "C").unwrap();
        assert_eq!(c.scenario_start_minute, 15);
        assert_eq!(
            c.causal_drivers,
            vec![
                CausalDriver::DelayedPredecessor {
                    operation_id: "A".to_owned(),
                },
                CausalDriver::DelayedPredecessor {
                    operation_id: "B".to_owned(),
                },
            ]
        );
    }


}
