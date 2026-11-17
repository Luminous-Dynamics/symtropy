//! Deterministic parent-before-child assembly planning for tetrahedral modules.
//!
//! This compiles the structural grammar into a discrete assembly order. It
//! does not solve robot reach, grasp, collision, pose correction, insertion
//! force, tool choice, connector locking or post-assembly inspection.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use crate::tetrahedral::{
    TetraAttachmentInterface, TetrahedralAssembly, TetrahedralError,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssemblyOperationKind {
    AttachTetrahedronAcrossTriangularFaceV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyStep {
    pub order_index: usize,
    pub cell_id: u32,
    pub parent_cell_id: Option<u32>,
    pub attachment_face: Option<[u32; 3]>,
    pub introduced_node_id: Option<u32>,
    pub generation: u8,
    pub operation: AssemblyOperationKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TetrahedralAssemblyPlan {
    pub root_cell_id: u32,
    pub steps: Vec<AssemblyStep>,
    pub maximum_generation: u8,
    pub blocked_attachment_faces: usize,
    /// FNV-1a regression identity only; not a cryptographic digest.
    pub digest_non_security: u64,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssemblyPlanError {
    InvalidGeometry(TetrahedralError),
    EmptyAssembly,
    NonContiguousCellId { expected: u32, actual: u32 },
    RootCount { actual: usize },
    InvalidRootCell { cell_id: u32 },
    InvalidParentOrder { cell_id: u32, parent_cell_id: u32 },
    InvalidGeneration { cell_id: u32, parent_cell_id: u32 },
    UnknownParent { cell_id: u32, parent_cell_id: u32 },
    InvalidAttachmentFace { cell_id: u32 },
    NonUniqueIntroducedNode { cell_id: u32 },
    ReusedIntroducedNode { cell_id: u32, node_id: u32 },
}

impl fmt::Display for AssemblyPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGeometry(error) => write!(f, "source geometry audit failed: {error}"),
            Self::EmptyAssembly => write!(f, "cannot plan an empty tetrahedral assembly"),
            Self::NonContiguousCellId { expected, actual } => {
                write!(f, "cell ID {actual} found where {expected} was expected")
            }
            Self::RootCount { actual } => {
                write!(f, "expected exactly one root cell, found {actual}")
            }
            Self::InvalidRootCell { cell_id } => {
                write!(f, "root cell {cell_id} has child attachment metadata")
            }
            Self::InvalidParentOrder {
                cell_id,
                parent_cell_id,
            } => write!(
                f,
                "cell {cell_id} must be ordered after parent cell {parent_cell_id}"
            ),
            Self::InvalidGeneration {
                cell_id,
                parent_cell_id,
            } => write!(
                f,
                "cell {cell_id} generation does not follow parent cell {parent_cell_id}"
            ),
            Self::UnknownParent {
                cell_id,
                parent_cell_id,
            } => write!(
                f,
                "cell {cell_id} references unknown parent cell {parent_cell_id}"
            ),
            Self::InvalidAttachmentFace { cell_id } => {
                write!(f, "cell {cell_id} has an invalid shared triangular attachment face")
            }
            Self::NonUniqueIntroducedNode { cell_id } => {
                write!(f, "cell {cell_id} does not introduce exactly one apex node")
            }
            Self::ReusedIntroducedNode { cell_id, node_id } => write!(
                f,
                "cell {cell_id} attempts to introduce node {node_id} more than once"
            ),
        }
    }
}
impl Error for AssemblyPlanError {}

/// Compile a geometry-audited tetrahedral cell hierarchy into an explicit,
/// deterministic parent-before-child assembly plan.
pub fn plan_tetrahedral_assembly(
    assembly: &TetrahedralAssembly,
) -> Result<TetrahedralAssemblyPlan, AssemblyPlanError> {
    assembly
        .audit()
        .map_err(AssemblyPlanError::InvalidGeometry)?;
    if assembly.cells.is_empty() {
        return Err(AssemblyPlanError::EmptyAssembly);
    }

    let mut roots = Vec::new();
    let mut known_nodes = HashSet::new();
    let mut steps = Vec::with_capacity(assembly.cells.len());
    let mut maximum_generation = 0u8;

    for (index, cell) in assembly.cells.iter().enumerate() {
        let expected_id =
            u32::try_from(index).map_err(|_| AssemblyPlanError::EmptyAssembly)?;
        if cell.id != expected_id {
            return Err(AssemblyPlanError::NonContiguousCellId {
                expected: expected_id,
                actual: cell.id,
            });
        }
        maximum_generation = maximum_generation.max(cell.generation);

        match (cell.parent_cell, cell.attachment_face, cell.attachment_interface) {
            (None, None, None) => {
                roots.push(cell.id);
                if cell.generation != 0 {
                    return Err(AssemblyPlanError::InvalidRootCell { cell_id: cell.id });
                }
                known_nodes.extend(cell.vertices);
                steps.push(AssemblyStep {
                    order_index: steps.len(),
                    cell_id: cell.id,
                    parent_cell_id: None,
                    attachment_face: None,
                    introduced_node_id: None,
                    generation: cell.generation,
                    operation: AssemblyOperationKind::AttachTetrahedronAcrossTriangularFaceV1,
                });
            }
            (
                Some(parent_id),
                Some(face),
                Some(TetraAttachmentInterface::TriangularThreePointV1),
            ) => {
                if parent_id >= cell.id {
                    return Err(AssemblyPlanError::InvalidParentOrder {
                        cell_id: cell.id,
                        parent_cell_id: parent_id,
                    });
                }
                let parent = assembly
                    .cells
                    .get(parent_id as usize)
                    .filter(|candidate| candidate.id == parent_id)
                    .ok_or(AssemblyPlanError::UnknownParent {
                        cell_id: cell.id,
                        parent_cell_id: parent_id,
                    })?;

                if parent.generation.checked_add(1) != Some(cell.generation) {
                    return Err(AssemblyPlanError::InvalidGeneration {
                        cell_id: cell.id,
                        parent_cell_id: parent_id,
                    });
                }

                let face_is_valid = face.iter().copied().collect::<HashSet<_>>().len() == 3
                    && face.iter().all(|vertex| parent.vertices.contains(vertex))
                    && face.iter().all(|vertex| cell.vertices.contains(vertex))
                    && face.iter().all(|vertex| known_nodes.contains(vertex));
                let shared_vertices = cell
                    .vertices
                    .iter()
                    .filter(|vertex| parent.vertices.contains(vertex))
                    .count();
                if !face_is_valid || shared_vertices != 3 {
                    return Err(AssemblyPlanError::InvalidAttachmentFace {
                        cell_id: cell.id,
                    });
                }

                let introduced = cell
                    .vertices
                    .iter()
                    .copied()
                    .filter(|vertex| !face.contains(vertex))
                    .collect::<Vec<_>>();
                if introduced.len() != 1 {
                    return Err(AssemblyPlanError::NonUniqueIntroducedNode {
                        cell_id: cell.id,
                    });
                }
                let introduced_node_id = introduced[0];
                if known_nodes.contains(&introduced_node_id) {
                    return Err(AssemblyPlanError::ReusedIntroducedNode {
                        cell_id: cell.id,
                        node_id: introduced_node_id,
                    });
                }
                known_nodes.insert(introduced_node_id);

                steps.push(AssemblyStep {
                    order_index: steps.len(),
                    cell_id: cell.id,
                    parent_cell_id: Some(parent_id),
                    attachment_face: Some(face),
                    introduced_node_id: Some(introduced_node_id),
                    generation: cell.generation,
                    operation: AssemblyOperationKind::AttachTetrahedronAcrossTriangularFaceV1,
                });
            }
            _ => {
                return Err(AssemblyPlanError::InvalidRootCell { cell_id: cell.id });
            }
        }
    }

    if roots.len() != 1 {
        return Err(AssemblyPlanError::RootCount {
            actual: roots.len(),
        });
    }

    let root_cell_id = roots[0];
    let digest_non_security = assembly_plan_digest(
        root_cell_id,
        &steps,
        maximum_generation,
        assembly.blocked_attachment_faces,
    );

    Ok(TetrahedralAssemblyPlan {
        root_cell_id,
        steps,
        maximum_generation,
        blocked_attachment_faces: assembly.blocked_attachment_faces,
        digest_non_security,
        claim_ceiling: "deterministic parent-before-child cell order only; no robot motion, grasp, collision, connector or safety qualification",
    })
}

fn assembly_plan_digest(
    root_cell_id: u32,
    steps: &[AssemblyStep],
    maximum_generation: u8,
    blocked_attachment_faces: usize,
) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    fn mix(hash: &mut u64, bytes: &[u8]) {
        for byte in bytes {
            *hash ^= u64::from(*byte);
            *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    mix(&mut hash, &root_cell_id.to_le_bytes());
    mix(&mut hash, &[maximum_generation]);
    mix(
        &mut hash,
        &(blocked_attachment_faces as u64).to_le_bytes(),
    );
    for step in steps {
        mix(&mut hash, &(step.order_index as u64).to_le_bytes());
        mix(&mut hash, &step.cell_id.to_le_bytes());
        mix(
            &mut hash,
            &step.parent_cell_id.unwrap_or(u32::MAX).to_le_bytes(),
        );
        mix(&mut hash, &[step.generation]);
        mix(
            &mut hash,
            &step.introduced_node_id.unwrap_or(u32::MAX).to_le_bytes(),
        );
        if let Some(face) = step.attachment_face {
            for vertex in face {
                mix(&mut hash, &vertex.to_le_bytes());
            }
        }
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tetrahedral::{TetrahedralAssembly, TetrahedralConfig};

    #[test]
    fn generated_assembly_compiles_to_parent_before_child_plan() {
        let geometry =
            TetrahedralAssembly::generate(TetrahedralConfig::default()).unwrap();
        let plan = plan_tetrahedral_assembly(&geometry).unwrap();

        assert_eq!(plan.root_cell_id, 0);
        assert_eq!(plan.steps.len(), geometry.cells.len());
        assert_eq!(plan.steps[0].parent_cell_id, None);
        for step in plan.steps.iter().skip(1) {
            assert!(step.parent_cell_id.unwrap() < step.cell_id);
            assert!(step.introduced_node_id.is_some());
            assert!(step.attachment_face.is_some());
        }
        assert_eq!(
            plan.steps.last().unwrap().order_index,
            plan.steps.len() - 1
        );
    }

    #[test]
    fn assembly_plan_digest_is_deterministic() {
        let geometry =
            TetrahedralAssembly::generate(TetrahedralConfig::default()).unwrap();
        let left = plan_tetrahedral_assembly(&geometry).unwrap();
        let right = plan_tetrahedral_assembly(&geometry).unwrap();
        assert_eq!(left, right);
        assert_eq!(left.digest_non_security, right.digest_non_security);
    }

    #[test]
    fn child_generation_must_follow_parent_generation() {
        let mut geometry =
            TetrahedralAssembly::generate(TetrahedralConfig::default()).unwrap();
        let child = geometry
            .cells
            .iter_mut()
            .find(|cell| cell.parent_cell.is_some())
            .unwrap();
        child.generation += 1;

        assert!(matches!(
            plan_tetrahedral_assembly(&geometry),
            Err(AssemblyPlanError::InvalidGeometry(_))
                | Err(AssemblyPlanError::InvalidGeneration { .. })
        ));
    }

    #[test]
    fn child_cell_with_self_parent_fails_closed() {
        let mut geometry =
            TetrahedralAssembly::generate(TetrahedralConfig::default()).unwrap();
        let child = geometry
            .cells
            .iter_mut()
            .find(|cell| cell.parent_cell.is_some())
            .unwrap();
        child.parent_cell = Some(child.id);

        assert!(matches!(
            plan_tetrahedral_assembly(&geometry),
            Err(AssemblyPlanError::InvalidGeometry(_))
                | Err(AssemblyPlanError::InvalidParentOrder { .. })
        ));
    }
}
