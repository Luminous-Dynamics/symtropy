// SPDX-License-Identifier: AGPL-3.0-or-later
//! Minimal, dependency-free reference kernel.
//!
//! This crate validates structural graph transitions and exact integer-basis
//! material accounting. It does not solve forces, predict collapse, or model
//! fracture propagation.

use std::collections::{BTreeMap, BTreeSet};

pub type Check<T> = Result<T, String>;

pub mod beam_element;
pub mod beam_line;
pub mod beam_line_sparse;
pub mod beam_modal;
pub mod beam_pin_assembly;
pub mod deconstruction;
pub mod mechanics;
pub mod modal_bar;
pub mod nafems_nbc05;
pub mod planar_truss;
pub mod sofistik_be25_audit;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Component {
    pub id: String,
    pub material: String,
    /// Declared mass in milligrams. This is accounting metadata, not a measurement.
    pub mass_mg: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Connection {
    pub id: String,
    pub a: String,
    pub b: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operation {
    InstallComponent(Component),
    RemoveComponent {
        id: String,
    },
    CreateConnection(Connection),
    RemoveConnection {
        id: String,
    },
    SetSupport {
        component_id: String,
        active: bool,
    },
    RecoverComponent {
        component_id: String,
        reused_mg: u64,
        recycled_mg: u64,
        contaminated_mg: u64,
        waste_mg: u64,
        unaccounted_mg: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialRecoveryRecord {
    pub component_id: String,
    pub material: String,
    pub input_mass_mg: u64,
    pub reused_mg: u64,
    pub recycled_mg: u64,
    pub contaminated_mg: u64,
    pub waste_mg: u64,
    pub unaccounted_mg: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub id: String,
    pub operation: Operation,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct World {
    components: BTreeMap<String, Component>,
    connections: BTreeMap<String, Connection>,
    active_supports: BTreeSet<String>,
    recovered_components: BTreeMap<String, MaterialRecoveryRecord>,
    applied_event_ids: BTreeSet<String>,
    revision: u64,
}

impl World {
    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    pub fn active_support_count(&self) -> usize {
        self.active_supports.len()
    }

    pub fn recovered_component_count(&self) -> usize {
        self.recovered_components.len()
    }

    pub fn recovered_mass_mg(&self) -> Check<u64> {
        self.recovered_components
            .values()
            .try_fold(0_u64, |sum, record| {
                sum.checked_add(record.input_mass_mg)
                    .ok_or_else(|| "recovered mass total overflow".to_owned())
            })
    }

    /// Apply one event transactionally: any rejected operation leaves every
    /// state field, including the revision and event-ID set, unchanged.
    pub fn apply(&mut self, event: Event) -> Check<()> {
        validate_token(&event.id, "event ID")?;
        if self.applied_event_ids.contains(&event.id) {
            return Err(format!("duplicate event ID: {}", event.id));
        }

        let mut candidate = self.clone();
        candidate.apply_operation(event.operation)?;
        candidate.applied_event_ids.insert(event.id);
        candidate.revision = candidate
            .revision
            .checked_add(1)
            .ok_or_else(|| "revision overflow".to_owned())?;
        *self = candidate;
        Ok(())
    }

    fn apply_operation(&mut self, operation: Operation) -> Check<()> {
        match operation {
            Operation::InstallComponent(component) => {
                validate_token(&component.id, "component ID")?;
                validate_token(&component.material, "material ID")?;
                if component.mass_mg == 0 {
                    return Err(format!("component {} has zero declared mass", component.id));
                }
                if self.components.contains_key(&component.id) {
                    return Err(format!("duplicate component ID: {}", component.id));
                }
                if self.recovered_components.contains_key(&component.id) {
                    return Err(format!(
                        "component ID already has a recovery record: {}",
                        component.id
                    ));
                }
                self.components.insert(component.id.clone(), component);
            }
            Operation::RemoveComponent { id } => {
                validate_token(&id, "component ID")?;
                if !self.components.contains_key(&id) {
                    return Err(format!("unknown component: {id}"));
                }
                if self.active_supports.contains(&id) {
                    return Err(format!("cannot remove supported component: {id}"));
                }
                if self
                    .connections
                    .values()
                    .any(|connection| connection.a == id || connection.b == id)
                {
                    return Err(format!("cannot remove connected component: {id}"));
                }
                self.components.remove(&id);
            }
            Operation::CreateConnection(connection) => {
                validate_token(&connection.id, "connection ID")?;
                validate_token(&connection.a, "connection endpoint")?;
                validate_token(&connection.b, "connection endpoint")?;
                if connection.a == connection.b {
                    return Err("self-connections are not allowed".to_owned());
                }
                if self.connections.contains_key(&connection.id) {
                    return Err(format!("duplicate connection ID: {}", connection.id));
                }
                if !self.components.contains_key(&connection.a) {
                    return Err(format!("unknown connection endpoint: {}", connection.a));
                }
                if !self.components.contains_key(&connection.b) {
                    return Err(format!("unknown connection endpoint: {}", connection.b));
                }
                self.connections.insert(connection.id.clone(), connection);
            }
            Operation::RemoveConnection { id } => {
                validate_token(&id, "connection ID")?;
                if self.connections.remove(&id).is_none() {
                    return Err(format!("unknown connection: {id}"));
                }
            }
            Operation::SetSupport {
                component_id,
                active,
            } => {
                validate_token(&component_id, "component ID")?;
                if !self.components.contains_key(&component_id) {
                    return Err(format!("unknown support target: {component_id}"));
                }
                if active {
                    if !self.active_supports.insert(component_id.clone()) {
                        return Err(format!("support already active: {component_id}"));
                    }
                } else if !self.active_supports.remove(&component_id) {
                    return Err(format!("support is not active: {component_id}"));
                }
            }
            Operation::RecoverComponent {
                component_id,
                reused_mg,
                recycled_mg,
                contaminated_mg,
                waste_mg,
                unaccounted_mg,
            } => {
                validate_token(&component_id, "component ID")?;
                let component = self
                    .components
                    .get(&component_id)
                    .cloned()
                    .ok_or_else(|| format!("unknown component to recover: {component_id}"))?;

                if self.recovered_components.contains_key(&component_id) {
                    return Err(format!(
                        "component already has recovery record: {component_id}"
                    ));
                }
                if self.active_supports.contains(&component_id) {
                    return Err(format!(
                        "cannot recover supported component: {component_id}"
                    ));
                }
                if self
                    .connections
                    .values()
                    .any(|connection| connection.a == component_id || connection.b == component_id)
                {
                    return Err(format!(
                        "cannot recover connected component: {component_id}"
                    ));
                }

                MassLedger {
                    input_mg: component.mass_mg,
                    reused_mg,
                    recycled_mg,
                    contaminated_mg,
                    waste_mg,
                    unaccounted_mg,
                }
                .validate()?;

                self.components.remove(&component_id);
                self.recovered_components.insert(
                    component_id.clone(),
                    MaterialRecoveryRecord {
                        component_id,
                        material: component.material,
                        input_mass_mg: component.mass_mg,
                        reused_mg,
                        recycled_mg,
                        contaminated_mg,
                        waste_mg,
                        unaccounted_mg,
                    },
                );
            }
        }
        Ok(())
    }

    /// Return connected components, each sorted by ID, with groups in stable
    /// lexicographic order. Connectivity is topological only; it is not a
    /// structural stability or collapse prediction.
    pub fn connected_components(&self) -> Vec<Vec<String>> {
        let mut adjacency: BTreeMap<String, BTreeSet<String>> = self
            .components
            .keys()
            .map(|id| (id.clone(), BTreeSet::new()))
            .collect();

        for connection in self.connections.values() {
            if let Some(neighbors) = adjacency.get_mut(&connection.a) {
                neighbors.insert(connection.b.clone());
            }
            if let Some(neighbors) = adjacency.get_mut(&connection.b) {
                neighbors.insert(connection.a.clone());
            }
        }

        let mut visited = BTreeSet::new();
        let mut groups = Vec::new();

        for start in adjacency.keys() {
            if visited.contains(start) {
                continue;
            }
            let mut stack = vec![start.clone()];
            let mut group = Vec::new();
            while let Some(current) = stack.pop() {
                if !visited.insert(current.clone()) {
                    continue;
                }
                group.push(current.clone());
                if let Some(neighbors) = adjacency.get(&current) {
                    for neighbor in neighbors.iter().rev() {
                        if !visited.contains(neighbor) {
                            stack.push(neighbor.clone());
                        }
                    }
                }
            }
            group.sort();
            groups.push(group);
        }
        groups.sort();
        groups
    }

    /// Canonical text snapshot for replay comparison. This is not a
    /// cryptographic digest or physical-evidence certificate.
    pub fn canonical_snapshot(&self) -> String {
        let mut out = format!("revision={}\n", self.revision);
        for component in self.components.values() {
            out.push_str(&format!(
                "component|{}|{}|{}\n",
                component.id, component.material, component.mass_mg
            ));
        }
        for connection in self.connections.values() {
            out.push_str(&format!(
                "connection|{}|{}|{}\n",
                connection.id, connection.a, connection.b
            ));
        }
        for support in &self.active_supports {
            out.push_str(&format!("support|{support}\n"));
        }
        for record in self.recovered_components.values() {
            out.push_str(&format!(
                "recovered|{}|{}|{}|{}|{}|{}|{}|{}\n",
                record.component_id,
                record.material,
                record.input_mass_mg,
                record.reused_mg,
                record.recycled_mg,
                record.contaminated_mg,
                record.waste_mg,
                record.unaccounted_mg
            ));
        }
        out
    }
}

/// Exact mass accounting in an integer milligram basis. Integer accounting
/// avoids rounding policy in the first contract layer; it does not certify
/// the accuracy of declared or measured masses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MassLedger {
    pub input_mg: u64,
    pub reused_mg: u64,
    pub recycled_mg: u64,
    pub contaminated_mg: u64,
    pub waste_mg: u64,
    pub unaccounted_mg: u64,
}

impl MassLedger {
    pub fn output_total_mg(self) -> Check<u64> {
        [
            self.reused_mg,
            self.recycled_mg,
            self.contaminated_mg,
            self.waste_mg,
            self.unaccounted_mg,
        ]
        .into_iter()
        .try_fold(0_u64, |sum, value| {
            sum.checked_add(value)
                .ok_or_else(|| "mass output total overflow".to_owned())
        })
    }

    pub fn validate(self) -> Check<()> {
        if self.input_mg == 0 {
            return Err("mass ledger input must be positive".to_owned());
        }
        let output = self.output_total_mg()?;
        if output != self.input_mg {
            return Err(format!(
                "mass imbalance: input={} mg, outputs={} mg",
                self.input_mg, output
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureOutcome {
    pub initial_connected_components: usize,
    pub after_cut_connected_components: usize,
    pub after_reconnect_connected_components: usize,
    pub component_count: usize,
    pub connection_count: usize,
    pub accepted_balanced_mass_ledger: bool,
    pub rejected_imbalanced_mass_ledger: bool,
    pub rejected_mutation_was_atomic: bool,
    pub recovered_component_count: usize,
    pub recovered_mass_mg: u64,
    pub deconstruction_step_count: usize,
    pub deconstruction_recovered_component_count: usize,
    pub deconstruction_recovered_mass_mg: u64,
}

pub fn run_reference_fixture() -> Check<FixtureOutcome> {
    let mut world = World::default();
    for component in [
        Component {
            id: "A".to_owned(),
            material: "concrete".to_owned(),
            mass_mg: 1_000_000,
        },
        Component {
            id: "B".to_owned(),
            material: "steel".to_owned(),
            mass_mg: 250_000,
        },
        Component {
            id: "C".to_owned(),
            material: "timber".to_owned(),
            mass_mg: 150_000,
        },
    ] {
        world.apply(Event {
            id: format!("install-{}", component.id),
            operation: Operation::InstallComponent(component),
        })?;
    }
    world.apply(Event {
        id: "connect-AB".to_owned(),
        operation: Operation::CreateConnection(Connection {
            id: "AB".to_owned(),
            a: "A".to_owned(),
            b: "B".to_owned(),
        }),
    })?;
    world.apply(Event {
        id: "connect-BC".to_owned(),
        operation: Operation::CreateConnection(Connection {
            id: "BC".to_owned(),
            a: "B".to_owned(),
            b: "C".to_owned(),
        }),
    })?;

    let initial_connected_components = world.connected_components().len();
    world.apply(Event {
        id: "remove-BC".to_owned(),
        operation: Operation::RemoveConnection {
            id: "BC".to_owned(),
        },
    })?;
    let after_cut_connected_components = world.connected_components().len();

    let snapshot_before_rejection = world.canonical_snapshot();
    let revision_before_rejection = world.revision();
    let rejected_mutation_was_atomic = world
        .apply(Event {
            id: "invalid-remove-B".to_owned(),
            operation: Operation::RemoveComponent { id: "B".to_owned() },
        })
        .is_err()
        && world.canonical_snapshot() == snapshot_before_rejection
        && world.revision() == revision_before_rejection;

    world.apply(Event {
        id: "restore-BC".to_owned(),
        operation: Operation::CreateConnection(Connection {
            id: "BC".to_owned(),
            a: "B".to_owned(),
            b: "C".to_owned(),
        }),
    })?;
    let after_reconnect_connected_components = world.connected_components().len();

    let balanced = MassLedger {
        input_mg: 120_000,
        reused_mg: 20_000,
        recycled_mg: 70_000,
        contaminated_mg: 10_000,
        waste_mg: 15_000,
        unaccounted_mg: 5_000,
    };
    balanced.validate()?;
    let accepted_balanced_mass_ledger = true;

    let imbalanced = MassLedger {
        input_mg: 120_000,
        reused_mg: 20_000,
        recycled_mg: 70_000,
        contaminated_mg: 10_000,
        waste_mg: 15_000,
        unaccounted_mg: 4_999,
    };
    let rejected_imbalanced_mass_ledger = imbalanced.validate().is_err();

    let mut recovery_world = World::default();
    recovery_world.apply(Event {
        id: "install-recovery-component".to_owned(),
        operation: Operation::InstallComponent(Component {
            id: "RECOVER-001".to_owned(),
            material: "steel".to_owned(),
            mass_mg: 1_000,
        }),
    })?;
    recovery_world.apply(Event {
        id: "recover-component".to_owned(),
        operation: Operation::RecoverComponent {
            component_id: "RECOVER-001".to_owned(),
            reused_mg: 500,
            recycled_mg: 300,
            contaminated_mg: 50,
            waste_mg: 100,
            unaccounted_mg: 50,
        },
    })?;
    let recovered_component_count = recovery_world.recovered_component_count();
    let recovered_mass_mg = recovery_world.recovered_mass_mg()?;

    let mut deconstruction_world = World::default();
    deconstruction_world.apply(Event {
        id: "install-DECON-A".to_owned(),
        operation: Operation::InstallComponent(Component {
            id: "DECON-A".to_owned(),
            material: "steel".to_owned(),
            mass_mg: 1_000,
        }),
    })?;
    deconstruction_world.apply(Event {
        id: "install-DECON-B".to_owned(),
        operation: Operation::InstallComponent(Component {
            id: "DECON-B".to_owned(),
            material: "concrete".to_owned(),
            mass_mg: 2_000,
        }),
    })?;
    deconstruction_world.apply(Event {
        id: "connect-DECON-AB".to_owned(),
        operation: Operation::CreateConnection(Connection {
            id: "DECON-AB".to_owned(),
            a: "DECON-A".to_owned(),
            b: "DECON-B".to_owned(),
        }),
    })?;
    let deconstruction_plan = deconstruction::preview(
        &deconstruction_world,
        "fixture-deconstruction",
        vec![
            deconstruction::RecoveryAllocation {
                component_id: "DECON-A".to_owned(),
                reused_mg: 500,
                recycled_mg: 300,
                contaminated_mg: 50,
                waste_mg: 100,
                unaccounted_mg: 50,
            },
            deconstruction::RecoveryAllocation {
                component_id: "DECON-B".to_owned(),
                reused_mg: 1_000,
                recycled_mg: 500,
                contaminated_mg: 100,
                waste_mg: 200,
                unaccounted_mg: 200,
            },
        ],
    )?;
    let deconstruction_step_count = deconstruction_plan.steps.len();
    deconstruction::execute(&mut deconstruction_world, &deconstruction_plan)?;
    let deconstruction_recovered_component_count = deconstruction_world.recovered_component_count();
    let deconstruction_recovered_mass_mg = deconstruction_world.recovered_mass_mg()?;

    let outcome = FixtureOutcome {
        initial_connected_components,
        after_cut_connected_components,
        after_reconnect_connected_components,
        component_count: world.component_count(),
        connection_count: world.connection_count(),
        accepted_balanced_mass_ledger,
        rejected_imbalanced_mass_ledger,
        rejected_mutation_was_atomic,
        recovered_component_count,
        recovered_mass_mg,
        deconstruction_step_count,
        deconstruction_recovered_component_count,
        deconstruction_recovered_mass_mg,
    };

    if outcome.initial_connected_components != 1
        || outcome.after_cut_connected_components != 2
        || outcome.after_reconnect_connected_components != 1
        || outcome.component_count != 3
        || outcome.connection_count != 2
        || !outcome.accepted_balanced_mass_ledger
        || !outcome.rejected_imbalanced_mass_ledger
        || !outcome.rejected_mutation_was_atomic
        || outcome.recovered_component_count != 1
        || outcome.recovered_mass_mg != 1_000
        || outcome.deconstruction_step_count != 3
        || outcome.deconstruction_recovered_component_count != 2
        || outcome.deconstruction_recovered_mass_mg != 3_000
    {
        return Err(format!(
            "frozen fixture did not meet expected contract: {outcome:?}"
        ));
    }
    Ok(outcome)
}

fn validate_token(value: &str, label: &str) -> Check<()> {
    if value.is_empty()
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
    {
        return Err(format!(
            "invalid {label}; allowed characters are ASCII letters, digits, '_', '-', '.', ':'"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn install(world: &mut World, event_id: &str, component_id: &str) {
        world
            .apply(Event {
                id: event_id.to_owned(),
                operation: Operation::InstallComponent(Component {
                    id: component_id.to_owned(),
                    material: "steel".to_owned(),
                    mass_mg: 1_000,
                }),
            })
            .expect("valid component install");
    }

    #[test]
    fn fixture_meets_pinned_topology_and_accounting_expectations() {
        let result = run_reference_fixture().expect("frozen fixture should pass");
        assert_eq!(result.initial_connected_components, 1);
        assert_eq!(result.after_cut_connected_components, 2);
        assert_eq!(result.after_reconnect_connected_components, 1);
        assert_eq!((result.component_count, result.connection_count), (3, 2));
        assert!(result.accepted_balanced_mass_ledger);
        assert!(result.rejected_imbalanced_mass_ledger);
        assert!(result.rejected_mutation_was_atomic);
        assert_eq!(result.recovered_component_count, 1);
        assert_eq!(result.recovered_mass_mg, 1_000);
        assert_eq!(result.deconstruction_step_count, 3);
        assert_eq!(result.deconstruction_recovered_component_count, 2);
        assert_eq!(result.deconstruction_recovered_mass_mg, 3_000);
    }

    #[test]
    fn duplicate_event_is_rejected_without_mutation() {
        let mut world = World::default();
        install(&mut world, "install-A", "A");
        let before = world.canonical_snapshot();
        let revision = world.revision();
        let error = world
            .apply(Event {
                id: "install-A".to_owned(),
                operation: Operation::InstallComponent(Component {
                    id: "B".to_owned(),
                    material: "steel".to_owned(),
                    mass_mg: 1,
                }),
            })
            .expect_err("duplicate event IDs must fail");
        assert!(error.contains("duplicate event ID"));
        assert_eq!(world.canonical_snapshot(), before);
        assert_eq!(world.revision(), revision);
    }

    #[test]
    fn failed_operation_does_not_consume_event_id() {
        let mut world = World::default();
        install(&mut world, "install-A", "A");
        install(&mut world, "install-B", "B");
        world
            .apply(Event {
                id: "connect-AB".to_owned(),
                operation: Operation::CreateConnection(Connection {
                    id: "AB".to_owned(),
                    a: "A".to_owned(),
                    b: "B".to_owned(),
                }),
            })
            .expect("valid connection");
        assert!(
            world
                .apply(Event {
                    id: "remove-B".to_owned(),
                    operation: Operation::RemoveComponent { id: "B".to_owned() },
                })
                .is_err()
        );
        world
            .apply(Event {
                id: "disconnect-AB".to_owned(),
                operation: Operation::RemoveConnection {
                    id: "AB".to_owned(),
                },
            })
            .expect("connection can be removed");
        world
            .apply(Event {
                id: "remove-B".to_owned(),
                operation: Operation::RemoveComponent { id: "B".to_owned() },
            })
            .expect("failed event ID was not consumed");
        assert_eq!(world.component_count(), 1);
    }

    #[test]
    fn dangling_endpoints_and_self_connections_are_rejected() {
        let mut world = World::default();
        install(&mut world, "install-A", "A");
        let before = world.canonical_snapshot();
        assert!(
            world
                .apply(Event {
                    id: "self-edge".to_owned(),
                    operation: Operation::CreateConnection(Connection {
                        id: "AA".to_owned(),
                        a: "A".to_owned(),
                        b: "A".to_owned(),
                    }),
                })
                .is_err()
        );
        assert!(
            world
                .apply(Event {
                    id: "dangling-edge".to_owned(),
                    operation: Operation::CreateConnection(Connection {
                        id: "AX".to_owned(),
                        a: "A".to_owned(),
                        b: "X".to_owned(),
                    }),
                })
                .is_err()
        );
        assert_eq!(world.canonical_snapshot(), before);
    }

    #[test]
    fn support_prevents_removal_until_released() {
        let mut world = World::default();
        install(&mut world, "install-A", "A");
        world
            .apply(Event {
                id: "support-A".to_owned(),
                operation: Operation::SetSupport {
                    component_id: "A".to_owned(),
                    active: true,
                },
            })
            .expect("support can be activated");
        assert!(
            world
                .apply(Event {
                    id: "remove-A-while-supported".to_owned(),
                    operation: Operation::RemoveComponent { id: "A".to_owned() },
                })
                .is_err()
        );
        world
            .apply(Event {
                id: "release-A".to_owned(),
                operation: Operation::SetSupport {
                    component_id: "A".to_owned(),
                    active: false,
                },
            })
            .expect("support can be released");
        world
            .apply(Event {
                id: "remove-A".to_owned(),
                operation: Operation::RemoveComponent { id: "A".to_owned() },
            })
            .expect("component can be removed after support release");
        assert_eq!(world.component_count(), 0);
    }

    #[test]
    fn component_recovery_is_atomic_and_binds_ledger_to_declared_component_mass() {
        let mut world = World::default();
        install(&mut world, "install-A", "A");
        install(&mut world, "install-B", "B");
        world
            .apply(Event {
                id: "connect-AB".to_owned(),
                operation: Operation::CreateConnection(Connection {
                    id: "AB".to_owned(),
                    a: "A".to_owned(),
                    b: "B".to_owned(),
                }),
            })
            .expect("valid connection");

        let before_connected_recovery = world.canonical_snapshot();
        let revision_before_connected_recovery = world.revision();
        assert!(
            world
                .apply(Event {
                    id: "recover-A".to_owned(),
                    operation: Operation::RecoverComponent {
                        component_id: "A".to_owned(),
                        reused_mg: 500,
                        recycled_mg: 300,
                        contaminated_mg: 50,
                        waste_mg: 100,
                        unaccounted_mg: 50,
                    },
                })
                .is_err()
        );
        assert_eq!(world.canonical_snapshot(), before_connected_recovery);
        assert_eq!(world.revision(), revision_before_connected_recovery);

        world
            .apply(Event {
                id: "disconnect-AB".to_owned(),
                operation: Operation::RemoveConnection {
                    id: "AB".to_owned(),
                },
            })
            .expect("connection can be removed");

        let before_imbalanced_recovery = world.canonical_snapshot();
        let revision_before_imbalanced_recovery = world.revision();
        assert!(
            world
                .apply(Event {
                    id: "recover-A".to_owned(),
                    operation: Operation::RecoverComponent {
                        component_id: "A".to_owned(),
                        reused_mg: 500,
                        recycled_mg: 300,
                        contaminated_mg: 50,
                        waste_mg: 100,
                        unaccounted_mg: 49,
                    },
                })
                .is_err()
        );
        assert_eq!(world.canonical_snapshot(), before_imbalanced_recovery);
        assert_eq!(world.revision(), revision_before_imbalanced_recovery);

        world
            .apply(Event {
                id: "recover-A".to_owned(),
                operation: Operation::RecoverComponent {
                    component_id: "A".to_owned(),
                    reused_mg: 500,
                    recycled_mg: 300,
                    contaminated_mg: 50,
                    waste_mg: 100,
                    unaccounted_mg: 50,
                },
            })
            .expect("balanced recovery succeeds after preconditions are satisfied");
        assert_eq!(world.component_count(), 1);
        assert_eq!(world.recovered_component_count(), 1);
        assert_eq!(
            world.recovered_mass_mg().expect("valid recovered mass"),
            1_000
        );
        let after_recovery = world.canonical_snapshot();
        let revision_after_recovery = world.revision();
        assert!(
            world
                .apply(Event {
                    id: "reinstall-A".to_owned(),
                    operation: Operation::InstallComponent(Component {
                        id: "A".to_owned(),
                        material: "steel".to_owned(),
                        mass_mg: 1_000,
                    }),
                })
                .is_err()
        );
        assert_eq!(world.canonical_snapshot(), after_recovery);
        assert_eq!(world.revision(), revision_after_recovery);
        assert!(
            world
                .canonical_snapshot()
                .contains("recovered|A|steel|1000|500|300|50|100|50")
        );
    }

    #[test]
    fn mass_ledger_rejects_imbalance_and_overflow() {
        let balanced = MassLedger {
            input_mg: 10,
            reused_mg: 2,
            recycled_mg: 3,
            contaminated_mg: 1,
            waste_mg: 3,
            unaccounted_mg: 1,
        };
        assert!(balanced.validate().is_ok());
        assert!(
            MassLedger {
                input_mg: 10,
                reused_mg: 2,
                recycled_mg: 3,
                contaminated_mg: 1,
                waste_mg: 3,
                unaccounted_mg: 0,
            }
            .validate()
            .is_err()
        );
        assert!(
            MassLedger {
                input_mg: u64::MAX,
                reused_mg: u64::MAX,
                recycled_mg: 1,
                contaminated_mg: 0,
                waste_mg: 0,
                unaccounted_mg: 0,
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn identifiers_are_validated_before_state_mutation() {
        let mut world = World::default();
        assert!(
            world
                .apply(Event {
                    id: "bad id".to_owned(),
                    operation: Operation::InstallComponent(Component {
                        id: "A".to_owned(),
                        material: "steel".to_owned(),
                        mass_mg: 1,
                    }),
                })
                .is_err()
        );
        assert_eq!(world.revision(), 0);
        assert_eq!(world.component_count(), 0);
    }
}
