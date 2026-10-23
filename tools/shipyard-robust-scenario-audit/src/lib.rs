use shipyard_polygon_geometry::{GeometryError, Polygon, TimeWindow};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpaceReservation {
    pub reservation_id: String,
    pub block_id: String,
    pub footprint: Polygon,
    pub window: TimeWindow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationReservation {
    pub operation_id: String,
    pub window: TimeWindow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceUse {
    pub use_id: String,
    pub operation_id: String,
    pub resource_id: String,
    pub window: TimeWindow,
}

/// Immutable disruption data. Candidate plans do not carry this field and
/// therefore cannot silently remove or reschedule a declared outage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceOutage {
    pub outage_id: String,
    pub resource_id: String,
    pub window: TimeWindow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiredSpaceReservation {
    pub reservation_id: String,
    pub block_id: String,
    pub minimum_duration_ticks: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiredOperation {
    pub operation_id: String,
    /// The modeled processing duration is fixed; start time may move during recovery.
    pub duration_ticks: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequiredResourceUse {
    pub use_id: String,
    pub operation_id: String,
    pub resource_id: String,
    pub minimum_duration_ticks: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PrecedenceConstraint {
    pub predecessor_operation_id: String,
    pub successor_operation_id: String,
    pub minimum_lag_ticks: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioDefinition {
    pub scenario_id: String,
    /// Blocks that must still appear in the plan for this scenario. A cancellation
    /// must be represented by a separate, explicit protocol rather than omission.
    pub required_block_ids: Vec<String>,
    /// Specific space-reservation identities that cannot disappear. Their start
    /// times may change, but their minimum occupancy durations cannot be shortened.
    pub required_reservations: Vec<RequiredSpaceReservation>,
    /// Operations that cannot disappear or silently change modeled duration.
    pub required_operations: Vec<RequiredOperation>,
    /// Specific resource assignments/occupancies that must remain represented.
    pub required_resource_uses: Vec<RequiredResourceUse>,
    /// Immutable finish-to-start dependencies from the benchmark.
    pub precedence_constraints: Vec<PrecedenceConstraint>,
    /// Ground-truth outages supplied by the benchmark, not the candidate.
    pub resource_outages: Vec<ResourceOutage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioPlan {
    pub scenario_id: String,
    pub space_reservations: Vec<SpaceReservation>,
    pub operations: Vec<OperationReservation>,
    pub resource_uses: Vec<ResourceUse>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Violation {
    SpaceTimeConflict {
        first_reservation_id: String,
        second_reservation_id: String,
    },
    BlockDoubleOccupancy {
        block_id: String,
        first_reservation_id: String,
        second_reservation_id: String,
    },
    UnaryResourceOverlap {
        resource_id: String,
        first_use_id: String,
        second_use_id: String,
    },
    ResourceUnavailable {
        resource_id: String,
        use_id: String,
        outage_id: String,
    },
    PrecedenceViolation {
        predecessor_operation_id: String,
        successor_operation_id: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub violations: Vec<Violation>,
    /// Structural or evidence gaps that prevent a complete robustness claim.
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RobustStatus {
    /// Every declared scenario has exactly one candidate plan; all required
    /// commitments are present and every spatial/resource predicate passes.
    RobustlyFeasible,
    /// The scenario set or one or more candidate plans are incomplete/ambiguous.
    /// This state must never be advertised as robustly feasible.
    Incomplete {
        diagnostics: Vec<String>,
    },
    /// The scenario set is complete and at least one scenario violates a declared
    /// space-time or unary-resource constraint.
    NotRobust {
        failing_scenario_ids: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RobustAudit {
    pub status: RobustStatus,
    pub scenario_results: Vec<ScenarioResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuditError {
    Geometry(GeometryError),
}

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Geometry(error) => write!(f, "geometry predicate failed: {error}"),
        }
    }
}

impl std::error::Error for AuditError {}

impl From<GeometryError> for AuditError {
    fn from(value: GeometryError) -> Self {
        Self::Geometry(value)
    }
}

/// Audit a candidate recovery policy against every explicitly enumerated
/// scenario definition. All scenario IDs must be unique and the definition/plan
/// ID sets must match exactly.
///
/// This is a finite-scenario checker. Passing means only that the supplied plan
/// passes every supplied scenario and the declared static footprint/unary-resource
/// model; it is not a guarantee against disturbances omitted from the set.
pub fn audit_robust_plan(
    definitions: &[ScenarioDefinition],
    plans: &[ScenarioPlan],
) -> Result<RobustAudit, AuditError> {
    let mut global_diagnostics = Vec::new();
    if definitions.is_empty() {
        global_diagnostics.push("scenario definition set must not be empty".to_owned());
    }
    let mut definition_ids = BTreeSet::new();
    let mut plan_ids = BTreeSet::new();

    for definition in definitions {
        if definition.scenario_id.trim().is_empty() {
            global_diagnostics.push("scenario definition has an empty scenario_id".to_owned());
        }
        if !definition_ids.insert(definition.scenario_id.clone()) {
            global_diagnostics.push(format!(
                "duplicate scenario definition {}",
                definition.scenario_id
            ));
        }
        validate_unique_nonempty(
            &definition.required_block_ids,
            &format!("scenario {} required_block_ids", definition.scenario_id),
            &mut global_diagnostics,
        );
        validate_unique_nonempty(
            &definition.required_reservations.iter().map(|item| item.reservation_id.clone()).collect::<Vec<_>>(),
            &format!("scenario {} required_reservations", definition.scenario_id),
            &mut global_diagnostics,
        );
        validate_unique_nonempty(
            &definition.required_operations.iter().map(|item| item.operation_id.clone()).collect::<Vec<_>>(),
            &format!("scenario {} required_operations", definition.scenario_id),
            &mut global_diagnostics,
        );
        validate_unique_nonempty(
            &definition.required_resource_uses.iter().map(|item| item.use_id.clone()).collect::<Vec<_>>(),
            &format!("scenario {} required_resource_uses", definition.scenario_id),
            &mut global_diagnostics,
        );
        let required_operation_ids: BTreeSet<String> = definition.required_operations.iter().map(|item| item.operation_id.clone()).collect();
        for required in &definition.required_reservations {
            if !definition.required_block_ids.contains(&required.block_id) {
                global_diagnostics.push(format!(
                    "scenario {} required reservation {} refers to unlisted block {}",
                    definition.scenario_id, required.reservation_id, required.block_id
                ));
            }
            if required.minimum_duration_ticks <= 0 {
                global_diagnostics.push(format!(
                    "scenario {} required reservation {} has non-positive minimum duration",
                    definition.scenario_id, required.reservation_id
                ));
            }
        }
        for required in &definition.required_operations {
            if required.duration_ticks <= 0 {
                global_diagnostics.push(format!(
                    "scenario {} required operation {} has non-positive duration",
                    definition.scenario_id, required.operation_id
                ));
            }
        }
        for required in &definition.required_resource_uses {
            if !required_operation_ids.contains(&required.operation_id) {
                global_diagnostics.push(format!(
                    "scenario {} required resource use {} refers to unlisted operation {}",
                    definition.scenario_id, required.use_id, required.operation_id
                ));
            }
            if required.use_id.trim().is_empty() || required.resource_id.trim().is_empty() || required.minimum_duration_ticks <= 0 {
                global_diagnostics.push(format!(
                    "scenario {} required resource use {} has invalid ID/resource/duration",
                    definition.scenario_id, required.use_id
                ));
            }
        }
        let mut precedence_seen = BTreeSet::new();
        for dependency in &definition.precedence_constraints {
            if dependency.predecessor_operation_id == dependency.successor_operation_id
                || dependency.minimum_lag_ticks < 0
                || !required_operation_ids.contains(&dependency.predecessor_operation_id)
                || !required_operation_ids.contains(&dependency.successor_operation_id)
            {
                global_diagnostics.push(format!(
                    "scenario {} has invalid precedence {} -> {}",
                    definition.scenario_id, dependency.predecessor_operation_id, dependency.successor_operation_id
                ));
            }
            if !precedence_seen.insert(dependency.clone()) {
                global_diagnostics.push(format!(
                    "scenario {} has duplicate precedence {} -> {}",
                    definition.scenario_id, dependency.predecessor_operation_id, dependency.successor_operation_id
                ));
            }
        }
        let mut outage_ids = BTreeSet::new();
        for outage in &definition.resource_outages {
            if outage.outage_id.trim().is_empty() || outage.resource_id.trim().is_empty() {
                global_diagnostics.push(format!(
                    "scenario {} contains an outage with an empty ID/resource",
                    definition.scenario_id
                ));
            }
            if !outage_ids.insert(outage.outage_id.clone()) {
                global_diagnostics.push(format!(
                    "scenario {} has duplicate outage ID {}",
                    definition.scenario_id, outage.outage_id
                ));
            }
        }
    }

    for plan in plans {
        if plan.scenario_id.trim().is_empty() {
            global_diagnostics.push("candidate scenario plan has an empty scenario_id".to_owned());
        }
        if !plan_ids.insert(plan.scenario_id.clone()) {
            global_diagnostics.push(format!(
                "duplicate candidate plan for scenario {}",
                plan.scenario_id
            ));
        }
        if !definition_ids.contains(&plan.scenario_id) {
            global_diagnostics.push(format!(
                "candidate plan references undeclared scenario {}",
                plan.scenario_id
            ));
        }
    }

    for definition in definitions {
        if !plan_ids.contains(&definition.scenario_id) {
            global_diagnostics.push(format!(
                "missing candidate plan for declared scenario {}",
                definition.scenario_id
            ));
        }
    }

    let mut scenario_results = Vec::new();
    for definition in definitions {
        if let Some(plan) = plans.iter().find(|plan| plan.scenario_id == definition.scenario_id) {
            scenario_results.push(audit_scenario(definition, plan)?);
        }
    }
    scenario_results.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    global_diagnostics.sort();
    global_diagnostics.dedup();

    let scenario_gaps = scenario_results
        .iter()
        .any(|result| !result.diagnostics.is_empty());
    if !global_diagnostics.is_empty() || scenario_gaps {
        let mut all = global_diagnostics;
        for result in &scenario_results {
            for diagnostic in &result.diagnostics {
                all.push(format!("scenario {}: {}", result.scenario_id, diagnostic));
            }
        }
        all.sort();
        all.dedup();
        return Ok(RobustAudit {
            status: RobustStatus::Incomplete { diagnostics: all },
            scenario_results,
        });
    }

    let mut failing_scenarios: Vec<String> = scenario_results
        .iter()
        .filter(|result| !result.violations.is_empty())
        .map(|result| result.scenario_id.clone())
        .collect();
    failing_scenarios.sort();
    failing_scenarios.dedup();

    let status = if failing_scenarios.is_empty() {
        RobustStatus::RobustlyFeasible
    } else {
        RobustStatus::NotRobust {
            failing_scenario_ids: failing_scenarios,
        }
    };

    Ok(RobustAudit {
        status,
        scenario_results,
    })
}

fn validate_unique_nonempty(values: &[String], label: &str, diagnostics: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() {
            diagnostics.push(format!("{label} contains an empty ID"));
        }
        if !seen.insert(value.clone()) {
            diagnostics.push(format!("{label} contains duplicate ID {value}"));
        }
    }
}

fn audit_scenario(
    definition: &ScenarioDefinition,
    plan: &ScenarioPlan,
) -> Result<ScenarioResult, AuditError> {
    let mut diagnostics = Vec::new();
    let mut violations = Vec::new();

    let mut reservation_ids = BTreeSet::new();
    let mut block_ids_present = BTreeSet::new();
    for reservation in &plan.space_reservations {
        if reservation.reservation_id.trim().is_empty() || reservation.block_id.trim().is_empty() {
            diagnostics.push("space reservation has an empty reservation_id or block_id".to_owned());
        }
        if !reservation_ids.insert(reservation.reservation_id.clone()) {
            diagnostics.push(format!(
                "duplicate space reservation ID {}",
                reservation.reservation_id
            ));
        }
        block_ids_present.insert(reservation.block_id.clone());
    }
    for block_id in &definition.required_block_ids {
        if !block_ids_present.contains(block_id) {
            diagnostics.push(format!("required block {block_id} has no space reservation"));
        }
    }
    for required in &definition.required_reservations {
        match plan.space_reservations.iter().find(|item| item.reservation_id == required.reservation_id) {
            Some(actual) => {
                if actual.block_id != required.block_id {
                    diagnostics.push(format!(
                        "required reservation {} changed block identity from {} to {}",
                        required.reservation_id, required.block_id, actual.block_id
                    ));
                }
                let duration = i128::from(actual.window.end()) - i128::from(actual.window.start());
                if duration < i128::from(required.minimum_duration_ticks) {
                    diagnostics.push(format!(
                        "required reservation {} was shortened below minimum duration {}",
                        required.reservation_id, required.minimum_duration_ticks
                    ));
                }
            }
            None => diagnostics.push(format!(
                "required space reservation {} is missing",
                required.reservation_id
            )),
        }
    }

    for first_index in 0..plan.space_reservations.len() {
        for second_index in (first_index + 1)..plan.space_reservations.len() {
            let first = &plan.space_reservations[first_index];
            let second = &plan.space_reservations[second_index];
            if !first.window.overlaps(&second.window) {
                continue;
            }
            if first.block_id == second.block_id {
                violations.push(Violation::BlockDoubleOccupancy {
                    block_id: first.block_id.clone(),
                    first_reservation_id: first.reservation_id.clone(),
                    second_reservation_id: second.reservation_id.clone(),
                });
            } else if first.footprint.conflicts_with(&second.footprint)? {
                violations.push(Violation::SpaceTimeConflict {
                    first_reservation_id: first.reservation_id.clone(),
                    second_reservation_id: second.reservation_id.clone(),
                });
            }
        }
    }

    let mut operation_map = BTreeMap::new();
    for operation in &plan.operations {
        if operation.operation_id.trim().is_empty() {
            diagnostics.push("operation reservation has an empty operation_id".to_owned());
        }
        if operation_map
            .insert(operation.operation_id.clone(), operation.window)
            .is_some()
        {
            diagnostics.push(format!(
                "duplicate operation reservation {}",
                operation.operation_id
            ));
        }
    }
    for required in &definition.required_operations {
        match operation_map.get(&required.operation_id) {
            None => diagnostics.push(format!("required operation {} is missing", required.operation_id)),
            Some(window) => {
                let duration = i128::from(window.end()) - i128::from(window.start());
                if duration != i128::from(required.duration_ticks) {
                    diagnostics.push(format!(
                        "required operation {} has duration {}, expected {}",
                        required.operation_id, duration, required.duration_ticks
                    ));
                }
            }
        }
    }
    for dependency in &definition.precedence_constraints {
        match (
            operation_map.get(&dependency.predecessor_operation_id),
            operation_map.get(&dependency.successor_operation_id),
        ) {
            (Some(predecessor), Some(successor)) => {
                let required_start = i128::from(predecessor.end()) + i128::from(dependency.minimum_lag_ticks);
                if required_start > i128::from(successor.start()) {
                    violations.push(Violation::PrecedenceViolation {
                        predecessor_operation_id: dependency.predecessor_operation_id.clone(),
                        successor_operation_id: dependency.successor_operation_id.clone(),
                    });
                }
            }
            _ => diagnostics.push(format!(
                "precedence {} -> {} references a missing operation reservation",
                dependency.predecessor_operation_id, dependency.successor_operation_id
            )),
        }
    }

    let mut resource_use_ids = BTreeSet::new();
    for resource_use in &plan.resource_uses {
        if resource_use.use_id.trim().is_empty()
            || resource_use.operation_id.trim().is_empty()
            || resource_use.resource_id.trim().is_empty()
        {
            diagnostics.push("resource use has an empty use/operation/resource ID".to_owned());
        }
        if !resource_use_ids.insert(resource_use.use_id.clone()) {
            diagnostics.push(format!("duplicate resource use ID {}", resource_use.use_id));
        }
        match operation_map.get(&resource_use.operation_id) {
            Some(operation_window)
                if resource_use.window.start() >= operation_window.start()
                    && resource_use.window.end() <= operation_window.end() => {}
            Some(_) => diagnostics.push(format!(
                "resource use {} falls outside operation {} window",
                resource_use.use_id, resource_use.operation_id
            )),
            None => diagnostics.push(format!(
                "resource use {} references missing operation {}",
                resource_use.use_id, resource_use.operation_id
            )),
        }
    }

    for required in &definition.required_resource_uses {
        match plan.resource_uses.iter().find(|item| item.use_id == required.use_id) {
            None => diagnostics.push(format!("required resource use {} is missing", required.use_id)),
            Some(actual) => {
                if actual.operation_id != required.operation_id || actual.resource_id != required.resource_id {
                    diagnostics.push(format!(
                        "required resource use {} changed operation/resource identity",
                        required.use_id
                    ));
                }
                let duration = i128::from(actual.window.end()) - i128::from(actual.window.start());
                if duration < i128::from(required.minimum_duration_ticks) {
                    diagnostics.push(format!(
                        "required resource use {} was shortened below minimum duration {}",
                        required.use_id, required.minimum_duration_ticks
                    ));
                }
            }
        }
    }

    for first_index in 0..plan.resource_uses.len() {
        for second_index in (first_index + 1)..plan.resource_uses.len() {
            let first = &plan.resource_uses[first_index];
            let second = &plan.resource_uses[second_index];
            if first.resource_id == second.resource_id
                && first.window.overlaps(&second.window)
            {
                violations.push(Violation::UnaryResourceOverlap {
                    resource_id: first.resource_id.clone(),
                    first_use_id: first.use_id.clone(),
                    second_use_id: second.use_id.clone(),
                });
            }
        }
    }

    for resource_use in &plan.resource_uses {
        for outage in &definition.resource_outages {
            if resource_use.resource_id == outage.resource_id
                && resource_use.window.overlaps(&outage.window)
            {
                violations.push(Violation::ResourceUnavailable {
                    resource_id: resource_use.resource_id.clone(),
                    use_id: resource_use.use_id.clone(),
                    outage_id: outage.outage_id.clone(),
                });
            }
        }
    }

    violations.sort();
    violations.dedup();
    diagnostics.sort();
    diagnostics.dedup();
    Ok(ScenarioResult {
        scenario_id: definition.scenario_id.clone(),
        violations,
        diagnostics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use shipyard_polygon_geometry::Point;

    fn rectangle(x0: i64, y0: i64, x1: i64, y1: i64) -> Polygon {
        Polygon::new(vec![
            Point::new(x0, y0),
            Point::new(x1, y0),
            Point::new(x1, y1),
            Point::new(x0, y1),
        ]).unwrap()
    }

    fn outage(id: &str, resource: &str, start: i64, end: i64) -> ResourceOutage {
        ResourceOutage {
            outage_id: id.to_owned(),
            resource_id: resource.to_owned(),
            window: TimeWindow::new(start, end).unwrap(),
        }
    }

    fn space(id: &str, block: &str, polygon: Polygon, start: i64, end: i64) -> SpaceReservation {
        SpaceReservation {
            reservation_id: id.to_owned(),
            block_id: block.to_owned(),
            footprint: polygon,
            window: TimeWindow::new(start, end).unwrap(),
        }
    }

    fn operation(id: &str, start: i64, end: i64) -> OperationReservation {
        OperationReservation {
            operation_id: id.to_owned(),
            window: TimeWindow::new(start, end).unwrap(),
        }
    }

    fn use_resource(id: &str, operation: &str, resource: &str, start: i64, end: i64) -> ResourceUse {
        ResourceUse {
            use_id: id.to_owned(),
            operation_id: operation.to_owned(),
            resource_id: resource.to_owned(),
            window: TimeWindow::new(start, end).unwrap(),
        }
    }

    fn definition(
        id: &str,
        blocks: &[&str],
        operations: &[(&str, i64)],
        outages: Vec<ResourceOutage>,
    ) -> ScenarioDefinition {
        ScenarioDefinition {
            scenario_id: id.to_owned(),
            required_block_ids: blocks.iter().map(|value| (*value).to_owned()).collect(),
            required_reservations: blocks
                .iter()
                .map(|block_id| RequiredSpaceReservation {
                    reservation_id: format!("{block_id}-yard"),
                    block_id: (*block_id).to_owned(),
                    minimum_duration_ticks: 1,
                })
                .collect(),
            required_operations: operations
                .iter()
                .map(|(operation_id, duration_ticks)| RequiredOperation {
                    operation_id: (*operation_id).to_owned(),
                    duration_ticks: *duration_ticks,
                })
                .collect(),
            required_resource_uses: Vec::new(),
            precedence_constraints: Vec::new(),
            resource_outages: outages,
        }
    }

    fn plan(
        id: &str,
        spaces: Vec<SpaceReservation>,
        operations: Vec<OperationReservation>,
        uses: Vec<ResourceUse>,
    ) -> ScenarioPlan {
        ScenarioPlan {
            scenario_id: id.to_owned(),
            space_reservations: spaces,
            operations,
            resource_uses: uses,
        }
    }

    #[test]
    fn robust_acceptance_requires_every_declared_scenario() {
        let definitions = vec![
            definition("nominal", &["a", "b"], &[("weld", 2)], vec![]),
            definition("late-clearance", &["a", "b"], &[("weld", 2)], vec![]),
        ];
        let plans = vec![
            plan(
                "nominal",
                vec![
                    space("a-yard", "a", rectangle(0, 0, 2, 2), 0, 5),
                    space("b-yard", "b", rectangle(0, 0, 2, 2), 5, 10),
                ],
                vec![operation("weld", 0, 2)],
                vec![use_resource("weld-use", "weld", "station-1", 0, 2)],
            ),
            plan(
                "late-clearance",
                vec![
                    space("a-yard", "a", rectangle(0, 0, 2, 2), 0, 6),
                    space("b-yard", "b", rectangle(0, 0, 2, 2), 6, 10),
                ],
                vec![operation("weld", 0, 2)],
                vec![use_resource("weld-use", "weld", "station-1", 0, 2)],
            ),
        ];
        let audit = audit_robust_plan(&definitions, &plans).unwrap();
        assert_eq!(audit.status, RobustStatus::RobustlyFeasible);
        assert!(audit.scenario_results.iter().all(|result| result.violations.is_empty()));
    }

    #[test]
    fn one_delayed_clearance_counterexample_invalidates_robust_acceptance() {
        let definitions = vec![
            definition("nominal", &["a", "b"], &[], vec![]),
            definition("delayed", &["a", "b"], &[], vec![]),
        ];
        let plans = vec![
            plan(
                "nominal",
                vec![
                    space("a-yard", "a", rectangle(0, 0, 2, 2), 0, 5),
                    space("b-yard", "b", rectangle(0, 0, 2, 2), 5, 10),
                ],
                vec![],
                vec![],
            ),
            plan(
                "delayed",
                vec![
                    space("a-yard", "a", rectangle(0, 0, 2, 2), 0, 6),
                    space("b-yard", "b", rectangle(0, 0, 2, 2), 5, 10),
                ],
                vec![],
                vec![],
            ),
        ];
        let audit = audit_robust_plan(&definitions, &plans).unwrap();
        assert_eq!(
            audit.status,
            RobustStatus::NotRobust {
                failing_scenario_ids: vec!["delayed".to_owned()]
            }
        );
        assert!(audit.scenario_results.iter().any(|result| {
            result.scenario_id == "delayed"
                && result.violations.iter().any(|violation| matches!(violation, Violation::SpaceTimeConflict { .. }))
        }));
    }

    #[test]
    fn missing_scenario_is_incomplete_not_robustly_feasible() {
        let definitions = vec![
            definition("nominal", &["a"], &[], vec![]),
            definition("outage", &["a"], &[], vec![]),
        ];
        let plans = vec![plan(
            "nominal",
            vec![space("a-yard", "a", rectangle(0, 0, 1, 1), 0, 5)],
            vec![],
            vec![],
        )];
        let audit = audit_robust_plan(&definitions, &plans).unwrap();
        assert!(matches!(audit.status, RobustStatus::Incomplete { .. }));
        assert!(matches!(&audit.status, RobustStatus::Incomplete { diagnostics } if diagnostics.iter().any(|text| text.contains("missing candidate plan for declared scenario outage"))));
    }

    #[test]
    fn immutable_resource_outage_and_unary_reservation_are_enforced() {
        let definitions = vec![definition(
            "crane-outage",
            &["a"],
            &[("lift", 7)],
            vec![outage("crane-down", "crane-1", 4, 8)],
        )];
        let plans = vec![plan(
            "crane-outage",
            vec![space("a-yard", "a", rectangle(0, 0, 1, 1), 0, 10)],
            vec![operation("lift", 2, 9)],
            vec![use_resource("lift-use", "lift", "crane-1", 5, 7)],
        )];
        let audit = audit_robust_plan(&definitions, &plans).unwrap();
        assert_eq!(
            audit.status,
            RobustStatus::NotRobust {
                failing_scenario_ids: vec!["crane-outage".to_owned()]
            }
        );
        assert!(audit.scenario_results[0].violations.iter().any(|v| matches!(v, Violation::ResourceUnavailable { .. })));
    }

    #[test]
    fn overlapping_unary_uses_fail_even_without_a_spatial_conflict() {
        let definitions = vec![definition("nominal", &[], &[("lift-a", 5), ("lift-b", 5)], vec![])];
        let plans = vec![plan(
            "nominal",
            vec![],
            vec![operation("lift-a", 0, 5), operation("lift-b", 2, 7)],
            vec![
                use_resource("use-a", "lift-a", "crane-1", 0, 5),
                use_resource("use-b", "lift-b", "crane-1", 2, 7),
            ],
        )];
        let audit = audit_robust_plan(&definitions, &plans).unwrap();
        assert!(matches!(audit.status, RobustStatus::NotRobust { .. }));
        assert!(audit.scenario_results[0].violations.iter().any(|v| matches!(v, Violation::UnaryResourceOverlap { .. })));
    }

    #[test]
    fn missing_required_block_or_operation_is_incomplete() {
        let definitions = vec![definition("nominal", &["a", "b"], &[("weld", 2), ("inspect", 2)], vec![])];
        let plans = vec![plan(
            "nominal",
            vec![space("a-yard", "a", rectangle(0, 0, 1, 1), 0, 5)],
            vec![operation("weld", 0, 2)],
            vec![],
        )];
        let audit = audit_robust_plan(&definitions, &plans).unwrap();
        assert!(matches!(audit.status, RobustStatus::Incomplete { .. }));
        if let RobustStatus::Incomplete { diagnostics } = &audit.status {
            assert!(diagnostics.iter().any(|text| text.contains("required block b")));
            assert!(diagnostics.iter().any(|text| text.contains("required operation inspect")));
        }
    }

    #[test]
    fn finish_to_start_precedence_is_immutable_and_checked() {
        let mut definition = definition("weld-after-cut", &[], &[("cut", 3), ("weld", 2)], vec![]);
        definition.precedence_constraints.push(PrecedenceConstraint {
            predecessor_operation_id: "cut".to_owned(),
            successor_operation_id: "weld".to_owned(),
            minimum_lag_ticks: 0,
        });
        definition.required_resource_uses.push(RequiredResourceUse {
            use_id: "weld-station-use".to_owned(),
            operation_id: "weld".to_owned(),
            resource_id: "station-1".to_owned(),
            minimum_duration_ticks: 2,
        });

        let invalid = plan(
            "weld-after-cut",
            vec![],
            vec![operation("cut", 0, 3), operation("weld", 2, 4)],
            vec![use_resource("weld-station-use", "weld", "station-1", 2, 4)],
        );
        let rejected = audit_robust_plan(&[definition.clone()], &[invalid]).unwrap();
        assert!(matches!(rejected.status, RobustStatus::NotRobust { .. }));
        assert!(rejected.scenario_results[0].violations.iter().any(|value| matches!(
            value,
            Violation::PrecedenceViolation {
                predecessor_operation_id,
                successor_operation_id
            } if predecessor_operation_id == "cut" && successor_operation_id == "weld"
        )));

        let valid = plan(
            "weld-after-cut",
            vec![],
            vec![operation("cut", 0, 3), operation("weld", 3, 5)],
            vec![use_resource("weld-station-use", "weld", "station-1", 3, 5)],
        );
        let accepted = audit_robust_plan(&[definition], &[valid]).unwrap();
        assert_eq!(accepted.status, RobustStatus::RobustlyFeasible);
    }

    #[test]
    fn empty_scenario_set_cannot_receive_a_robust_certificate() {
        let audit = audit_robust_plan(&[], &[]).unwrap();
        assert!(matches!(audit.status, RobustStatus::Incomplete { .. }));
    }

    #[test]
    fn mandatory_space_reservation_cannot_be_dropped_or_shortened() {
        let mut definition = definition("mandatory-placement", &["a"], &[], vec![]);
        definition.required_reservations[0].minimum_duration_ticks = 3;
        let dropped = plan("mandatory-placement", vec![], vec![], vec![]);
        let audit = audit_robust_plan(&[definition.clone()], &[dropped]).unwrap();
        assert!(matches!(audit.status, RobustStatus::Incomplete { .. }));
        assert!(audit.status.clone().to_string().is_err());

        let shortened = plan(
            "mandatory-placement",
            vec![space("a-yard", "a", rectangle(0, 0, 1, 1), 0, 2)],
            vec![],
            vec![],
        );
        let audit = audit_robust_plan(&[definition], &[shortened]).unwrap();
        assert!(matches!(audit.status, RobustStatus::Incomplete { .. }));
        if let RobustStatus::Incomplete { diagnostics } = audit.status {
            assert!(diagnostics.iter().any(|text| text.contains("shortened below minimum duration")));
        }
    }

    #[test]
    fn half_open_resource_and_space_windows_allow_endpoint_reuse() {
        let definitions = vec![definition("handoff", &["a", "b"], &[("first", 5), ("second", 2)], vec![outage("down", "crane", 8, 10)])];
        let plans = vec![plan(
            "handoff",
            vec![
                space("first-yard", "a", rectangle(0, 0, 2, 2), 0, 5),
                space("second-yard", "b", rectangle(0, 0, 2, 2), 5, 10),
            ],
            vec![operation("first", 0, 5), operation("second", 5, 7)],
            vec![
                use_resource("first-use", "first", "crane", 0, 5),
                use_resource("second-use", "second", "crane", 5, 7),
            ],
        )];
        let audit = audit_robust_plan(&definitions, &plans).unwrap();
        assert_eq!(audit.status, RobustStatus::RobustlyFeasible);
    }
}
