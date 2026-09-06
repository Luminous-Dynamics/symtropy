// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Persistent assembly membership and derived topology.
//!
//! An assembly owns semantic membership only. It does not copy material state,
//! joint strength, solver health, or functional fitness. Connectivity is derived
//! from authoritative Interface and Joint records supplied to analysis.

use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, error::Error, fmt};
use symtropy_game_state::StableId;

use crate::{Interface, InterfaceId, Joint, JointId, JointLifecycle, WorkpieceId};

/// Stable identity of one intentional assembly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssemblyId(StableId);

impl AssemblyId {
    pub const fn new(id: StableId) -> Self {
        Self(id)
    }

    pub const fn stable_id(&self) -> &StableId {
        &self.0
    }
}

impl fmt::Display for AssemblyId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Persistent semantic membership. The referenced Interface and Joint records
/// remain authoritative for ownership, endpoints, method, and lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assembly {
    pub id: AssemblyId,
    pub revision: u64,
    workpieces: Vec<WorkpieceId>,
    interfaces: Vec<InterfaceId>,
    joints: Vec<JointId>,
}

impl Assembly {
    pub fn new(
        id: AssemblyId,
        revision: u64,
        mut workpieces: Vec<WorkpieceId>,
        mut interfaces: Vec<InterfaceId>,
        mut joints: Vec<JointId>,
    ) -> Result<Self, AssemblyError> {
        if workpieces.is_empty() {
            return Err(AssemblyError::WorkpieceRequired);
        }
        sort_unique_workpieces(&mut workpieces)?;
        sort_unique_interfaces(&mut interfaces)?;
        sort_unique_joints(&mut joints)?;
        Ok(Self {
            id,
            revision,
            workpieces,
            interfaces,
            joints,
        })
    }

    pub fn workpieces(&self) -> &[WorkpieceId] {
        &self.workpieces
    }

    pub fn interfaces(&self) -> &[InterfaceId] {
        &self.interfaces
    }

    pub fn joints(&self) -> &[JointId] {
        &self.joints
    }

    pub fn contains_workpiece(&self, id: &WorkpieceId) -> bool {
        self.workpieces.binary_search(id).is_ok()
    }

    pub fn contains_interface(&self, id: &InterfaceId) -> bool {
        self.interfaces.binary_search(id).is_ok()
    }

    /// Derives connectivity from exact authoritative interface/joint records.
    /// Released joints remain assembly history but do not connect components.
    pub fn analyze(
        &self,
        interface_records: &[Interface],
        joint_records: &[Joint],
    ) -> Result<AssemblyTopology, AssemblyError> {
        let interface_registry = index_interfaces(interface_records)?;
        let joint_registry = index_joints(joint_records)?;

        let mut owners = BTreeMap::<InterfaceId, WorkpieceId>::new();
        for interface_id in &self.interfaces {
            let interface = interface_registry
                .get(interface_id)
                .ok_or_else(|| AssemblyError::MissingInterface(interface_id.clone()))?;
            if !self.contains_workpiece(&interface.workpiece_id) {
                return Err(AssemblyError::InterfaceOwnedByExternalWorkpiece {
                    interface_id: interface_id.clone(),
                    workpiece_id: interface.workpiece_id.clone(),
                });
            }
            owners.insert(interface_id.clone(), interface.workpiece_id.clone());
        }

        let indices = self
            .workpieces
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, id)| (id, index))
            .collect::<BTreeMap<_, _>>();
        let mut parents = (0..self.workpieces.len()).collect::<Vec<_>>();
        let mut active_joint_count = 0usize;
        let mut released_joint_count = 0usize;
        let mut intra_workpiece_joint_count = 0usize;

        for joint_id in &self.joints {
            let joint = joint_registry
                .get(joint_id)
                .ok_or_else(|| AssemblyError::MissingJoint(joint_id.clone()))?;

            let [left_interface, right_interface] = &joint.interfaces;
            if !self.contains_interface(left_interface) {
                return Err(AssemblyError::JointReferencesExternalInterface {
                    joint_id: joint_id.clone(),
                    interface_id: left_interface.clone(),
                });
            }
            if !self.contains_interface(right_interface) {
                return Err(AssemblyError::JointReferencesExternalInterface {
                    joint_id: joint_id.clone(),
                    interface_id: right_interface.clone(),
                });
            }

            let left_owner = owners
                .get(left_interface)
                .ok_or_else(|| AssemblyError::MissingInterface(left_interface.clone()))?;
            let right_owner = owners
                .get(right_interface)
                .ok_or_else(|| AssemblyError::MissingInterface(right_interface.clone()))?;

            match joint.lifecycle {
                JointLifecycle::Released => {
                    released_joint_count += 1;
                }
                JointLifecycle::Active => {
                    active_joint_count += 1;
                    if left_owner == right_owner {
                        intra_workpiece_joint_count += 1;
                    } else {
                        let left_index = *indices
                            .get(left_owner)
                            .expect("validated interface owner is an assembly workpiece");
                        let right_index = *indices
                            .get(right_owner)
                            .expect("validated interface owner is an assembly workpiece");
                        union(&mut parents, left_index, right_index);
                    }
                }
            }
        }

        let mut grouped = BTreeMap::<usize, Vec<WorkpieceId>>::new();
        for (index, workpiece) in self.workpieces.iter().enumerate() {
            let root = find(&mut parents, index);
            grouped.entry(root).or_default().push(workpiece.clone());
        }
        let mut components = grouped.into_values().collect::<Vec<_>>();
        components.sort();

        Ok(AssemblyTopology {
            assembly_id: self.id.clone(),
            assembly_revision: self.revision,
            components,
            active_joint_count,
            released_joint_count,
            intra_workpiece_joint_count,
        })
    }
}

/// Derived semantic topology. This is not structural-integrity or service-fit
/// evidence; it only reports intentional active connectivity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssemblyTopology {
    pub assembly_id: AssemblyId,
    pub assembly_revision: u64,
    pub components: Vec<Vec<WorkpieceId>>,
    pub active_joint_count: usize,
    pub released_joint_count: usize,
    pub intra_workpiece_joint_count: usize,
}

impl AssemblyTopology {
    pub fn is_connected(&self) -> bool {
        self.components.len() == 1
    }

    pub fn component_count(&self) -> usize {
        self.components.len()
    }
}

#[derive(Debug)]
pub enum AssemblyError {
    WorkpieceRequired,
    DuplicateWorkpiece(WorkpieceId),
    DuplicateInterface(InterfaceId),
    DuplicateJoint(JointId),
    DuplicateInterfaceRecord(InterfaceId),
    DuplicateJointRecord(JointId),
    MissingInterface(InterfaceId),
    MissingJoint(JointId),
    InterfaceOwnedByExternalWorkpiece {
        interface_id: InterfaceId,
        workpiece_id: WorkpieceId,
    },
    JointReferencesExternalInterface {
        joint_id: JointId,
        interface_id: InterfaceId,
    },
}

impl fmt::Display for AssemblyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkpieceRequired => write!(formatter, "assembly requires at least one workpiece"),
            Self::DuplicateWorkpiece(id) => write!(formatter, "assembly repeats workpiece {id}"),
            Self::DuplicateInterface(id) => write!(formatter, "assembly repeats interface {id}"),
            Self::DuplicateJoint(id) => write!(formatter, "assembly repeats joint {id}"),
            Self::DuplicateInterfaceRecord(id) => {
                write!(formatter, "interface registry repeats record {id}")
            }
            Self::DuplicateJointRecord(id) => write!(formatter, "joint registry repeats record {id}"),
            Self::MissingInterface(id) => write!(formatter, "assembly interface {id} is unavailable"),
            Self::MissingJoint(id) => write!(formatter, "assembly joint {id} is unavailable"),
            Self::InterfaceOwnedByExternalWorkpiece {
                interface_id,
                workpiece_id,
            } => write!(
                formatter,
                "interface {interface_id} belongs to workpiece {workpiece_id}, which is outside the assembly"
            ),
            Self::JointReferencesExternalInterface {
                joint_id,
                interface_id,
            } => write!(
                formatter,
                "joint {joint_id} references interface {interface_id}, which is outside the assembly"
            ),
        }
    }
}

impl Error for AssemblyError {}

fn sort_unique_workpieces(values: &mut Vec<WorkpieceId>) -> Result<(), AssemblyError> {
    values.sort();
    for pair in values.windows(2) {
        if pair[0] == pair[1] {
            return Err(AssemblyError::DuplicateWorkpiece(pair[0].clone()));
        }
    }
    Ok(())
}

fn sort_unique_interfaces(values: &mut Vec<InterfaceId>) -> Result<(), AssemblyError> {
    values.sort();
    for pair in values.windows(2) {
        if pair[0] == pair[1] {
            return Err(AssemblyError::DuplicateInterface(pair[0].clone()));
        }
    }
    Ok(())
}

fn sort_unique_joints(values: &mut Vec<JointId>) -> Result<(), AssemblyError> {
    values.sort();
    for pair in values.windows(2) {
        if pair[0] == pair[1] {
            return Err(AssemblyError::DuplicateJoint(pair[0].clone()));
        }
    }
    Ok(())
}

fn index_interfaces(records: &[Interface]) -> Result<BTreeMap<InterfaceId, &Interface>, AssemblyError> {
    let mut registry = BTreeMap::new();
    for record in records {
        if registry.insert(record.id.clone(), record).is_some() {
            return Err(AssemblyError::DuplicateInterfaceRecord(record.id.clone()));
        }
    }
    Ok(registry)
}

fn index_joints(records: &[Joint]) -> Result<BTreeMap<JointId, &Joint>, AssemblyError> {
    let mut registry = BTreeMap::new();
    for record in records {
        if registry.insert(record.id.clone(), record).is_some() {
            return Err(AssemblyError::DuplicateJointRecord(record.id.clone()));
        }
    }
    Ok(registry)
}

fn find(parents: &mut [usize], index: usize) -> usize {
    if parents[index] != index {
        parents[index] = find(parents, parents[index]);
    }
    parents[index]
}

fn union(parents: &mut [usize], left: usize, right: usize) {
    let left_root = find(parents, left);
    let right_root = find(parents, right);
    if left_root != right_root {
        let (low, high) = if left_root < right_root {
            (left_root, right_root)
        } else {
            (right_root, left_root)
        };
        parents[high] = low;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        InterfaceKind, JointEvidenceRef, JointKind, MatingGeometry,
    };

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn workpiece(value: &str) -> WorkpieceId {
        WorkpieceId::new(id(value))
    }

    fn interface(interface_id: &str, workpiece_id: &str) -> Interface {
        Interface {
            id: InterfaceId::new(id(interface_id)),
            workpiece_id: workpiece(workpiece_id),
            kind: InterfaceKind::Structural {
                geometry: MatingGeometry {
                    profile_id: id("profile:rail:a"),
                    nominal_size_um: 40_000,
                    tolerance_um: 100,
                },
                load_transfer_profile: id("load:fixed"),
            },
        }
    }

    fn evidence(name: &str) -> JointEvidenceRef {
        JointEvidenceRef::new(
            id("authority:fabrication-process"),
            id(name),
            1,
            format!("digest:{name}"),
        )
        .unwrap()
    }

    fn established_joint(id_value: &str, left: &Interface, right: &Interface) -> Joint {
        Joint::establish(
            JointId::new(id(id_value)),
            JointKind::Welded,
            left,
            right,
            evidence(&format!("evidence:{id_value}")),
        )
        .unwrap()
    }

    #[test]
    fn active_joint_connects_two_workpieces_without_claiming_integrity() {
        let left = interface("interface:left", "workpiece:left");
        let right = interface("interface:right", "workpiece:right");
        let joint = established_joint("joint:bridge", &left, &right);
        let assembly = Assembly::new(
            AssemblyId::new(id("assembly:test")),
            3,
            vec![workpiece("workpiece:left"), workpiece("workpiece:right")],
            vec![left.id.clone(), right.id.clone()],
            vec![joint.id.clone()],
        )
        .unwrap();

        let topology = assembly.analyze(&[left, right], &[joint]).unwrap();
        assert!(topology.is_connected());
        assert_eq!(topology.active_joint_count, 1);

        let value = serde_json::to_value(topology).unwrap();
        assert!(value.get("integrity").is_none());
        assert!(value.get("health").is_none());
        assert!(value.get("strength").is_none());
        assert!(value.get("complete").is_none());
    }

    #[test]
    fn released_joint_no_longer_connects_components() {
        let left = interface("interface:left", "workpiece:left");
        let right = interface("interface:right", "workpiece:right");
        let mut joint = established_joint("joint:bridge", &left, &right);
        joint.release(evidence("evidence:release")).unwrap();
        let assembly = Assembly::new(
            AssemblyId::new(id("assembly:test")),
            4,
            vec![workpiece("workpiece:left"), workpiece("workpiece:right")],
            vec![left.id.clone(), right.id.clone()],
            vec![joint.id.clone()],
        )
        .unwrap();

        let topology = assembly.analyze(&[left, right], &[joint]).unwrap();
        assert_eq!(topology.component_count(), 2);
        assert_eq!(topology.released_joint_count, 1);
        assert!(!topology.is_connected());
    }

    #[test]
    fn intra_workpiece_seam_does_not_fake_connectivity_to_other_workpiece() {
        let seam_left = interface("interface:seam-left", "workpiece:sheet");
        let seam_right = interface("interface:seam-right", "workpiece:sheet");
        let spare = interface("interface:spare", "workpiece:spare");
        let seam = established_joint("joint:seam", &seam_left, &seam_right);
        let assembly = Assembly::new(
            AssemblyId::new(id("assembly:seam")),
            1,
            vec![workpiece("workpiece:sheet"), workpiece("workpiece:spare")],
            vec![seam_left.id.clone(), seam_right.id.clone(), spare.id.clone()],
            vec![seam.id.clone()],
        )
        .unwrap();

        let topology = assembly
            .analyze(&[seam_left, seam_right, spare], &[seam])
            .unwrap();
        assert_eq!(topology.component_count(), 2);
        assert_eq!(topology.intra_workpiece_joint_count, 1);
    }

    #[test]
    fn joint_endpoint_must_be_explicit_assembly_member() {
        let left = interface("interface:left", "workpiece:left");
        let right = interface("interface:right", "workpiece:right");
        let joint = established_joint("joint:bridge", &left, &right);
        let assembly = Assembly::new(
            AssemblyId::new(id("assembly:invalid")),
            1,
            vec![workpiece("workpiece:left"), workpiece("workpiece:right")],
            vec![left.id.clone()],
            vec![joint.id.clone()],
        )
        .unwrap();

        let result = assembly.analyze(&[left, right], &[joint]);
        assert!(matches!(
            result,
            Err(AssemblyError::JointReferencesExternalInterface { .. })
        ));
    }
}
