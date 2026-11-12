// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic deconstruction preflight and atomic execution.
//!
//! This module reasons over declared graph topology and mass allocations only.
//! It does not prove load-path safety or predict structural collapse.

use crate::{Check, Component, Connection, Event, MassLedger, Operation, World, validate_token};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryAllocation {
    pub component_id: String,
    pub reused_mg: u64,
    pub recycled_mg: u64,
    pub contaminated_mg: u64,
    pub waste_mg: u64,
    pub unaccounted_mg: u64,
}

impl RecoveryAllocation {
    fn ledger_for(&self, component: &Component) -> MassLedger {
        MassLedger {
            input_mg: component.mass_mg,
            reused_mg: self.reused_mg,
            recycled_mg: self.recycled_mg,
            contaminated_mg: self.contaminated_mg,
            waste_mg: self.waste_mg,
            unaccounted_mg: self.unaccounted_mg,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DeconstructionStep {
    Disconnect { connection_id: String },
    Recover { allocation: RecoveryAllocation },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeconstructionPlan {
    pub plan_id: String,
    pub source_revision: u64,
    /// Canonical, revisioned source state used to preflight this plan. This is
    /// an equality binding, not a cryptographic digest or physical certificate.
    pub source_state_snapshot: String,
    pub target_component_ids: Vec<String>,
    pub allocations: Vec<RecoveryAllocation>,
    pub steps: Vec<DeconstructionStep>,
}

/// Preflight a recovery operation over a fixed set of components.
///
/// The target set must be closed over connections: connections to components
/// outside the set block the plan. Active supports also block because this
/// reference layer has no qualified temporary-support/load-path model.
pub fn preview(
    world: &World,
    plan_id: &str,
    allocations: Vec<RecoveryAllocation>,
) -> Check<DeconstructionPlan> {
    validate_token(plan_id, "plan ID")?;
    if allocations.is_empty() {
        return Err("deconstruction plan must target at least one component".to_owned());
    }

    let mut by_component = BTreeMap::<String, RecoveryAllocation>::new();
    for allocation in allocations {
        validate_token(&allocation.component_id, "component ID")?;
        if by_component
            .insert(allocation.component_id.clone(), allocation)
            .is_some()
        {
            return Err("duplicate component allocation in deconstruction plan".to_owned());
        }
    }

    let target_ids: BTreeSet<String> = by_component.keys().cloned().collect();
    for component_id in &target_ids {
        let component = world
            .components
            .get(component_id)
            .ok_or_else(|| format!("unknown component in plan: {component_id}"))?;
        if world.recovered_components.contains_key(component_id) {
            return Err(format!("component already recovered: {component_id}"));
        }
        if world.active_supports.contains(component_id) {
            return Err(format!(
                "active support blocks deconstruction of component: {component_id}"
            ));
        }
        by_component[component_id]
            .ledger_for(component)
            .validate()?;
    }

    let mut internal_connections = Vec::<Connection>::new();
    for connection in world.connections.values() {
        let a_in_target = target_ids.contains(&connection.a);
        let b_in_target = target_ids.contains(&connection.b);
        if a_in_target != b_in_target {
            return Err(format!(
                "connection {} crosses deconstruction target boundary ({} <-> {})",
                connection.id, connection.a, connection.b
            ));
        }
        if a_in_target && b_in_target {
            internal_connections.push(connection.clone());
        }
    }
    internal_connections.sort_by(|left, right| left.id.cmp(&right.id));

    let ordered_allocations: Vec<RecoveryAllocation> =
        by_component.into_values().collect::<Vec<_>>();
    let mut steps = Vec::with_capacity(internal_connections.len() + ordered_allocations.len());
    for connection in internal_connections {
        steps.push(DeconstructionStep::Disconnect {
            connection_id: connection.id,
        });
    }
    for allocation in &ordered_allocations {
        steps.push(DeconstructionStep::Recover {
            allocation: allocation.clone(),
        });
    }

    Ok(DeconstructionPlan {
        plan_id: plan_id.to_owned(),
        source_revision: world.revision,
        source_state_snapshot: world.canonical_snapshot(),
        target_component_ids: target_ids.into_iter().collect(),
        allocations: ordered_allocations,
        steps,
    })
}

/// Execute a preflighted plan atomically against the exact source revision.
///
/// A caller cannot reorder, edit, or reuse a plan: the canonical plan is
/// rebuilt from current world state and compared before execution. Operations
/// run against a clone, and state is committed only after every step succeeds.
pub fn execute(world: &mut World, plan: &DeconstructionPlan) -> Check<()> {
    if world.revision != plan.source_revision {
        return Err(format!(
            "stale deconstruction plan: source revision {}, current revision {}",
            plan.source_revision, world.revision
        ));
    }
    if world.canonical_snapshot() != plan.source_state_snapshot {
        return Err("deconstruction plan source-state snapshot mismatch".to_owned());
    }

    let expected = preview(world, &plan.plan_id, plan.allocations.clone())?;
    if &expected != plan {
        return Err("deconstruction plan differs from canonical preflight".to_owned());
    }

    let mut candidate = world.clone();
    for step in &plan.steps {
        let (event_id, operation) = match step {
            DeconstructionStep::Disconnect { connection_id } => (
                format!("plan:{}:disconnect:{connection_id}", plan.plan_id),
                Operation::RemoveConnection {
                    id: connection_id.clone(),
                },
            ),
            DeconstructionStep::Recover { allocation } => (
                format!("plan:{}:recover:{}", plan.plan_id, allocation.component_id),
                Operation::RecoverComponent {
                    component_id: allocation.component_id.clone(),
                    reused_mg: allocation.reused_mg,
                    recycled_mg: allocation.recycled_mg,
                    contaminated_mg: allocation.contaminated_mg,
                    waste_mg: allocation.waste_mg,
                    unaccounted_mg: allocation.unaccounted_mg,
                },
            ),
        };
        candidate.apply(Event {
            id: event_id,
            operation,
        })?;
    }

    *world = candidate;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install(world: &mut World, event_id: &str, component_id: &str, mass_mg: u64) {
        world
            .apply(Event {
                id: event_id.to_owned(),
                operation: Operation::InstallComponent(Component {
                    id: component_id.to_owned(),
                    material: "steel".to_owned(),
                    mass_mg,
                }),
            })
            .expect("fixture component installation should succeed");
    }

    fn connect(world: &mut World, event_id: &str, id: &str, a: &str, b: &str) {
        world
            .apply(Event {
                id: event_id.to_owned(),
                operation: Operation::CreateConnection(Connection {
                    id: id.to_owned(),
                    a: a.to_owned(),
                    b: b.to_owned(),
                }),
            })
            .expect("fixture connection should succeed");
    }

    fn allocation(component_id: &str, mass_mg: u64) -> RecoveryAllocation {
        RecoveryAllocation {
            component_id: component_id.to_owned(),
            reused_mg: mass_mg / 2,
            recycled_mg: mass_mg / 4,
            contaminated_mg: 0,
            waste_mg: mass_mg / 8,
            unaccounted_mg: mass_mg - (mass_mg / 2 + mass_mg / 4 + mass_mg / 8),
        }
    }

    fn connected_pair() -> World {
        let mut world = World::default();
        install(&mut world, "install-A", "A", 1_000);
        install(&mut world, "install-B", "B", 2_000);
        connect(&mut world, "connect-AB", "AB", "A", "B");
        world
    }

    #[test]
    fn plan_has_canonical_disconnect_then_recover_order() {
        let world = connected_pair();
        let plan = preview(
            &world,
            "dismantle-01",
            vec![allocation("B", 2_000), allocation("A", 1_000)],
        )
        .expect("closed target set should plan");
        assert_eq!(plan.target_component_ids, vec!["A", "B"]);
        assert_eq!(plan.steps.len(), 3);
        assert_eq!(
            plan.steps[0],
            DeconstructionStep::Disconnect {
                connection_id: "AB".to_owned()
            }
        );
        assert!(matches!(
            &plan.steps[1],
            DeconstructionStep::Recover { allocation } if allocation.component_id == "A"
        ));
        assert!(matches!(
            &plan.steps[2],
            DeconstructionStep::Recover { allocation } if allocation.component_id == "B"
        ));
    }

    #[test]
    fn executing_plan_is_atomic_and_recovers_all_declared_mass() {
        let mut world = connected_pair();
        let plan = preview(
            &world,
            "dismantle-02",
            vec![allocation("A", 1_000), allocation("B", 2_000)],
        )
        .expect("valid plan");
        execute(&mut world, &plan).expect("canonical plan should execute");
        assert_eq!(world.component_count(), 0);
        assert_eq!(world.connection_count(), 0);
        assert_eq!(world.recovered_component_count(), 2);
        assert_eq!(
            world.recovered_mass_mg().expect("sum recovered mass"),
            3_000
        );
    }

    #[test]
    fn external_connection_blocks_plan_without_mutation() {
        let mut world = connected_pair();
        install(&mut world, "install-C", "C", 500);
        connect(&mut world, "connect-BC", "BC", "B", "C");
        let before = world.canonical_snapshot();
        let revision = world.revision();
        let result = preview(
            &world,
            "dismantle-03",
            vec![allocation("A", 1_000), allocation("B", 2_000)],
        );
        assert!(
            result
                .unwrap_err()
                .contains("crosses deconstruction target boundary")
        );
        assert_eq!(world.canonical_snapshot(), before);
        assert_eq!(world.revision(), revision);
    }

    #[test]
    fn active_support_blocks_plan() {
        let mut world = World::default();
        install(&mut world, "install-A", "A", 1_000);
        world
            .apply(Event {
                id: "support-A".to_owned(),
                operation: Operation::SetSupport {
                    component_id: "A".to_owned(),
                    active: true,
                },
            })
            .expect("support fixture should be valid");
        let result = preview(&world, "dismantle-04", vec![allocation("A", 1_000)]);
        assert!(result.unwrap_err().contains("active support blocks"));
    }

    #[test]
    fn unbalanced_mass_allocation_blocks_plan() {
        let world = connected_pair();
        let mut invalid = allocation("A", 1_000);
        invalid.unaccounted_mg -= 1;
        let result = preview(
            &world,
            "dismantle-05",
            vec![invalid, allocation("B", 2_000)],
        );
        assert!(result.unwrap_err().contains("mass imbalance"));
    }

    #[test]
    fn stale_or_tampered_plan_is_rejected_without_mutation() {
        let mut world = connected_pair();
        let plan = preview(
            &world,
            "dismantle-06",
            vec![allocation("A", 1_000), allocation("B", 2_000)],
        )
        .expect("valid plan");
        world
            .apply(Event {
                id: "unrelated-revision".to_owned(),
                operation: Operation::SetSupport {
                    component_id: "A".to_owned(),
                    active: true,
                },
            })
            .expect("fixture revision event");
        let before_stale = world.canonical_snapshot();
        assert!(execute(&mut world, &plan).unwrap_err().contains("stale"));
        assert_eq!(world.canonical_snapshot(), before_stale);

        world
            .apply(Event {
                id: "release-A".to_owned(),
                operation: Operation::SetSupport {
                    component_id: "A".to_owned(),
                    active: false,
                },
            })
            .expect("support release");
        let fresh = preview(
            &world,
            "dismantle-07",
            vec![allocation("A", 1_000), allocation("B", 2_000)],
        )
        .expect("valid fresh plan");
        let mut tampered = fresh.clone();
        tampered.steps.reverse();
        let before_tamper = world.canonical_snapshot();
        assert!(
            execute(&mut world, &tampered)
                .unwrap_err()
                .contains("canonical preflight")
        );
        assert_eq!(world.canonical_snapshot(), before_tamper);
    }

    #[test]
    fn plan_cannot_be_replayed_against_a_different_world_at_same_revision() {
        let original = connected_pair();
        let plan = preview(
            &original,
            "dismantle-world-bound",
            vec![allocation("A", 1_000), allocation("B", 2_000)],
        )
        .expect("original world can be planned");

        let mut different = World::default();
        install(&mut different, "install-A", "A", 1_000);
        different
            .apply(Event {
                id: "install-B".to_owned(),
                operation: Operation::InstallComponent(Component {
                    id: "B".to_owned(),
                    material: "concrete".to_owned(),
                    mass_mg: 2_000,
                }),
            })
            .expect("alternative world component");
        connect(&mut different, "connect-AB", "AB", "A", "B");
        assert_eq!(different.revision(), plan.source_revision);
        assert_ne!(different.canonical_snapshot(), plan.source_state_snapshot);

        let before = different.canonical_snapshot();
        assert!(
            execute(&mut different, &plan)
                .unwrap_err()
                .contains("source-state snapshot mismatch")
        );
        assert_eq!(different.canonical_snapshot(), before);
        assert_eq!(different.recovered_component_count(), 0);
    }

    #[test]
    fn duplicate_unknown_and_empty_targets_fail_closed() {
        let world = connected_pair();
        assert!(preview(&world, "empty-plan", vec![]).is_err());
        assert!(
            preview(
                &world,
                "duplicate-target",
                vec![allocation("A", 1_000), allocation("A", 1_000)]
            )
            .unwrap_err()
            .contains("duplicate component allocation")
        );
        assert!(
            preview(&world, "unknown-target", vec![allocation("MISSING", 1)])
                .unwrap_err()
                .contains("unknown component")
        );
    }
}
