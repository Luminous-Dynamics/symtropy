// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Test-only conservative contact graph construction from J and M^-1 support.
//!
//! This deliberately does not threshold floating Delassus blocks. A missing
//! graph edge is admitted only when a complete declared inverse-mass support
//! pattern proves that no generalized-coordinate support path exists.

use nalgebra::SMatrix;

type Matrix12 = SMatrix<f64, 12, 12>;
type Jacobian9x12 = SMatrix<f64, 9, 12>;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ContactEdgeWitness {
    contact_i: usize,
    contact_j: usize,
    generalized_dof_i: usize,
    generalized_dof_j: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OmittedContactEdgeProof {
    contact_i: usize,
    contact_j: usize,
    support_pairs_checked: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StructuralContactGraph {
    adjacency: Vec<Vec<bool>>,
    edge_witnesses: Vec<ContactEdgeWitness>,
    omitted_edge_proofs: Vec<OmittedContactEdgeProof>,
    support_complete: bool,
}

impl StructuralContactGraph {
    fn separator_graph_is_certified(&self) -> bool {
        self.support_complete
    }

    fn has_edge(&self, contact_i: usize, contact_j: usize) -> bool {
        self.adjacency[contact_i][contact_j]
    }
}

/// Derive the union of generalized-coordinate support for each contact from
/// a Boolean contact-Jacobian pattern. Each contact occupies rows_per_contact
/// contiguous rows (three for 3D point contact).
fn contact_supports_from_jacobian(
    jacobian_support: &[Vec<bool>],
    rows_per_contact: usize,
) -> Result<Vec<Vec<usize>>, &'static str> {
    if rows_per_contact == 0 || jacobian_support.is_empty() {
        return Err("Jacobian support has no contact rows or zero contact dimension");
    }
    if jacobian_support.len() % rows_per_contact != 0 {
        return Err("Jacobian support row count is not divisible by contact dimension");
    }
    let dof_count = jacobian_support[0].len();
    if dof_count == 0 || jacobian_support.iter().any(|row| row.len() != dof_count) {
        return Err("Jacobian support rows have inconsistent generalized dimensions");
    }

    let mut supports = Vec::with_capacity(jacobian_support.len() / rows_per_contact);
    for contact_rows in jacobian_support.chunks_exact(rows_per_contact) {
        let support = (0..dof_count)
            .filter(|dof| contact_rows.iter().any(|row| row[*dof]))
            .collect::<Vec<_>>();
        supports.push(support);
    }
    Ok(supports)
}

/// Build a conservative undirected contact graph from the structural support
/// of M^-1. An edge exists iff some generalized-coordinate support pair from
/// the two contacts is structurally coupled by M^-1.
///
/// If support is incomplete, return a complete graph and mark it unusable for
/// certified separator discovery. Unknown coupling must not become a false zero.
fn derive_contact_graph(
    contact_supports: &[Vec<usize>],
    inverse_mass_support: &[Vec<bool>],
    support_complete: bool,
) -> Result<StructuralContactGraph, &'static str> {
    let dof_count = inverse_mass_support.len();
    if dof_count == 0 || inverse_mass_support.iter().any(|row| row.len() != dof_count) {
        return Err("inverse-mass support must be a non-empty square matrix");
    }
    for row in 0..dof_count {
        for column in 0..dof_count {
            if inverse_mass_support[row][column] != inverse_mass_support[column][row] {
                return Err("inverse-mass support must be structurally symmetric");
            }
        }
    }
    for support in contact_supports {
        if support.iter().any(|dof| *dof >= dof_count) {
            return Err("contact support references an out-of-range generalized coordinate");
        }
        for (index, dof) in support.iter().enumerate() {
            if support[..index].contains(dof) {
                return Err("contact support contains a duplicate generalized coordinate");
            }
        }
    }

    let contact_count = contact_supports.len();
    let mut adjacency = vec![vec![false; contact_count]; contact_count];
    let mut edge_witnesses = Vec::new();
    let mut omitted_edge_proofs = Vec::new();

    if !support_complete {
        // The unknown remainder could couple any pair. Keep the complete graph
        // and refuse certified separator use until the support is completed.
        for contact_i in 0..contact_count {
            for contact_j in (contact_i + 1)..contact_count {
                adjacency[contact_i][contact_j] = true;
                adjacency[contact_j][contact_i] = true;
            }
        }
        return Ok(StructuralContactGraph {
            adjacency,
            edge_witnesses,
            omitted_edge_proofs,
            support_complete: false,
        });
    }

    for contact_i in 0..contact_count {
        for contact_j in (contact_i + 1)..contact_count {
            let mut witness = None;
            let mut support_pairs_checked = 0;
            for dof_i in &contact_supports[contact_i] {
                for dof_j in &contact_supports[contact_j] {
                    support_pairs_checked += 1;
                    if inverse_mass_support[*dof_i][*dof_j] {
                        witness = Some((*dof_i, *dof_j));
                        break;
                    }
                }
                if witness.is_some() {
                    break;
                }
            }

            if let Some((generalized_dof_i, generalized_dof_j)) = witness {
                adjacency[contact_i][contact_j] = true;
                adjacency[contact_j][contact_i] = true;
                edge_witnesses.push(ContactEdgeWitness {
                    contact_i,
                    contact_j,
                    generalized_dof_i,
                    generalized_dof_j,
                });
            } else {
                omitted_edge_proofs.push(OmittedContactEdgeProof {
                    contact_i,
                    contact_j,
                    support_pairs_checked,
                });
            }
        }
    }

    Ok(StructuralContactGraph {
        adjacency,
        edge_witnesses,
        omitted_edge_proofs,
        support_complete: true,
    })
}

fn star_jacobian_support() -> Vec<Vec<bool>> {
    let mut support = vec![vec![false; 12]; 9];
    for axis in 0..3 {
        // Separator (B,C).
        support[axis][axis + 3] = true;
        support[axis][axis + 6] = true;
        // Leaf 1 (A,B).
        support[axis + 3][axis] = true;
        support[axis + 3][axis + 3] = true;
        // Leaf 2 (C,D).
        support[axis + 6][axis + 6] = true;
        support[axis + 6][axis + 9] = true;
    }
    support
}

fn body_local_inverse_mass_support() -> Vec<Vec<bool>> {
    let mut support = vec![vec![false; 12]; 12];
    for body in 0..4 {
        for row in 0..3 {
            for column in 0..3 {
                support[body * 3 + row][body * 3 + column] = true;
            }
        }
    }
    support
}

fn star_numeric_jacobian() -> Jacobian9x12 {
    let mut jacobian = Jacobian9x12::zeros();
    for axis in 0..3 {
        jacobian[(axis, axis + 3)] = 1.0;
        jacobian[(axis, axis + 6)] = -1.0;
        jacobian[(axis + 3, axis)] = 1.0;
        jacobian[(axis + 3, axis + 3)] = -1.0;
        jacobian[(axis + 6, axis + 6)] = 1.0;
        jacobian[(axis + 6, axis + 9)] = -1.0;
    }
    jacobian
}

#[test]
fn body_support_graph_recovers_the_exact_three_contact_star() {
    let jacobian_support = star_jacobian_support();
    let contact_supports = contact_supports_from_jacobian(&jacobian_support, 3).unwrap();
    let inverse_mass_support = body_local_inverse_mass_support();
    let graph = derive_contact_graph(&contact_supports, &inverse_mass_support, true).unwrap();

    assert!(graph.separator_graph_is_certified());
    assert_eq!(graph.adjacency.len(), 3);
    assert!(graph.has_edge(0, 1));
    assert!(graph.has_edge(1, 0));
    assert!(graph.has_edge(0, 2));
    assert!(graph.has_edge(2, 0));
    assert!(!graph.has_edge(1, 2));
    assert!(!graph.has_edge(2, 1));

    assert_eq!(graph.edge_witnesses.len(), 2);
    assert!(graph.edge_witnesses.iter().any(|edge| {
        edge.contact_i == 0
            && edge.contact_j == 1
            && (3..6).contains(&edge.generalized_dof_i)
            && (3..6).contains(&edge.generalized_dof_j)
    }));
    assert!(graph.edge_witnesses.iter().any(|edge| {
        edge.contact_i == 0
            && edge.contact_j == 2
            && (6..9).contains(&edge.generalized_dof_i)
            && (6..9).contains(&edge.generalized_dof_j)
    }));

    let omitted = graph
        .omitted_edge_proofs
        .iter()
        .find(|proof| proof.contact_i == 1 && proof.contact_j == 2)
        .unwrap();
    assert_eq!(omitted.support_pairs_checked, 36);
}

#[test]
fn articulated_inverse_mass_support_connects_disjoint_contact_body_supports() {
    let jacobian_support = star_jacobian_support();
    let contact_supports = contact_supports_from_jacobian(&jacobian_support, 3).unwrap();
    let mut inverse_mass_support = body_local_inverse_mass_support();

    // A declared articulated/generalized-coordinate coupling joins DOF 0
    // (leaf 1 only) with DOF 9 (leaf 2 only).
    inverse_mass_support[0][9] = true;
    inverse_mass_support[9][0] = true;
    let graph = derive_contact_graph(&contact_supports, &inverse_mass_support, true).unwrap();

    assert!(graph.has_edge(1, 2));
    assert!(graph.edge_witnesses.iter().any(|edge| {
        edge.contact_i == 1
            && edge.contact_j == 2
            && edge.generalized_dof_i == 0
            && edge.generalized_dof_j == 9
    }));
}

#[test]
fn tiny_nonzero_inverse_mass_coupling_is_never_thresholded_away() {
    let jacobian_support = star_jacobian_support();
    let contact_supports = contact_supports_from_jacobian(&jacobian_support, 3).unwrap();
    let mut inverse_mass_support = body_local_inverse_mass_support();
    inverse_mass_support[0][9] = true;
    inverse_mass_support[9][0] = true;

    let mut inverse_mass = Matrix12::identity() * 0.5;
    inverse_mass[(0, 9)] = 1.0e-14;
    inverse_mass[(9, 0)] = 1.0e-14;
    let jacobian = star_numeric_jacobian();
    let delassus = jacobian * inverse_mass * jacobian.transpose();

    assert!(delassus[(3, 6)].abs() > 0.0);
    assert!(delassus[(3, 6)].abs() < 1.0e-13);

    let graph = derive_contact_graph(&contact_supports, &inverse_mass_support, true).unwrap();
    assert!(graph.has_edge(1, 2));
}

#[test]
fn incomplete_inverse_mass_support_overapproximates_and_forbids_separator_use() {
    let jacobian_support = star_jacobian_support();
    let contact_supports = contact_supports_from_jacobian(&jacobian_support, 3).unwrap();
    let inverse_mass_support = body_local_inverse_mass_support();
    let graph = derive_contact_graph(&contact_supports, &inverse_mass_support, false).unwrap();

    assert!(!graph.separator_graph_is_certified());
    for contact_i in 0..3 {
        for contact_j in 0..3 {
            if contact_i != contact_j {
                assert!(graph.has_edge(contact_i, contact_j));
            }
        }
    }
    assert!(graph.omitted_edge_proofs.is_empty());
}

#[test]
fn contact_graph_support_validation_rejects_malformed_patterns() {
    let supports = vec![vec![0, 1], vec![2, 3]];
    let asymmetric_pattern = vec![
        vec![true, false, false, false],
        vec![false, true, false, false],
        vec![false, false, true, true],
        vec![false, false, false, true],
    ];
    assert!(derive_contact_graph(&supports, &asymmetric_pattern, true).is_err());

    let symmetric_pattern = vec![
        vec![true, false, false, false],
        vec![false, true, false, false],
        vec![false, false, true, false],
        vec![false, false, false, true],
    ];
    assert!(derive_contact_graph(&[vec![0, 4]], &symmetric_pattern, true).is_err());
    assert!(derive_contact_graph(&[vec![0, 0]], &symmetric_pattern, true).is_err());
}
