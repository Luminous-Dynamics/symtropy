// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Passive, conservative solute exchange over an admitted hyphal graph.
//!
//! This is a numerical transport primitive, not a fungal physiology model.
//! Edge conductance is supplied by the caller in mL/hour and is never inferred
//! from geometry or diameter. The solver applies all edge fluxes simultaneously
//! and commits only when every donor has enough mass. It does not clamp stocks.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::hyphal_graph::{HyphalGraph, HyphalSegmentState, NodeId, SegmentId};

/// Identity of a transported solute or isotope tracer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SoluteId(pub u32);

/// State of one graph-associated transport compartment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoluteCompartment {
    pub node: NodeId,
    pub solute: SoluteId,
    pub volume_ml: f64,
    pub mass_mg: f64,
}

/// A passive transport edge bound to an existing hyphal segment.
///
/// Conductance is an effective volumetric conductance in mL/hour supplied by
/// an admitted model profile. It is not estimated from segment geometry here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassiveTransportEdge {
    pub segment: SegmentId,
    pub conductance_ml_per_hour: f64,
}

/// Signed flux from the segment's first node to its second node.
/// Positive values travel in the segment's declared direction; negative values
/// travel in the reverse direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeFlux {
    pub segment: SegmentId,
    pub signed_mass_mg: f64,
}

/// Result of one simultaneous passive-transport step.
#[derive(Debug, Clone, PartialEq)]
pub struct TransportReport {
    pub solute: SoluteId,
    pub dt_hours: f64,
    pub total_mass_before_mg: f64,
    pub total_mass_after_mg: f64,
    pub conservation_residual_mg: f64,
    pub edge_fluxes: Vec<EdgeFlux>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransportError {
    InvalidTimeStep,
    EmptyCompartments,
    NonFiniteTotalMass,
    InvalidVolume(NodeId),
    InvalidMass(NodeId),
    MixedSolutes,
    DuplicateCompartment(NodeId),
    DuplicateTransportEdge(SegmentId),
    MissingCompartment(NodeId),
    MissingSegment(SegmentId),
    InactiveSegment(SegmentId),
    InvalidConductance(SegmentId),
    ExplicitStepUnstable {
        node: NodeId,
        courant: f64,
        max_courant: f64,
    },
    InsufficientDonorMass {
        node: NodeId,
        requested_mg: f64,
        available_mg: f64,
    },
    NonFiniteResult(NodeId),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimeStep => write!(f, "time step must be finite and positive"),
            Self::EmptyCompartments => write!(f, "at least one transport compartment is required"),
            Self::NonFiniteTotalMass => write!(f, "total mass is not finite"),
            Self::InvalidVolume(node) => {
                write!(
                    f,
                    "compartment {:?} volume must be finite and positive",
                    node
                )
            }
            Self::InvalidMass(node) => {
                write!(
                    f,
                    "compartment {:?} mass must be finite and non-negative",
                    node
                )
            }
            Self::MixedSolutes => write!(f, "all compartments must contain the same solute"),
            Self::DuplicateCompartment(node) => {
                write!(f, "duplicate compartment for node {:?}", node)
            }
            Self::DuplicateTransportEdge(segment) => {
                write!(f, "duplicate transport edge for segment {:?}", segment)
            }
            Self::MissingCompartment(node) => {
                write!(f, "missing transport compartment for node {:?}", node)
            }
            Self::MissingSegment(segment) => {
                write!(
                    f,
                    "transport segment {:?} is absent from the graph",
                    segment
                )
            }
            Self::InactiveSegment(segment) => {
                write!(f, "transport segment {:?} is inactive", segment)
            }
            Self::InvalidConductance(segment) => {
                write!(
                    f,
                    "conductance for segment {:?} must be finite and positive",
                    segment
                )
            }
            Self::ExplicitStepUnstable {
                node,
                courant,
                max_courant,
            } => write!(
                f,
                "explicit transport step at node {:?} is unstable: courant {}, maximum {}",
                node, courant, max_courant
            ),
            Self::InsufficientDonorMass {
                node,
                requested_mg,
                available_mg,
            } => write!(
                f,
                "transport would overdraw node {:?}: requested {} mg, available {} mg",
                node, requested_mg, available_mg
            ),
            Self::NonFiniteResult(node) => {
                write!(f, "transport result for node {:?} is not finite", node)
            }
        }
    }
}

impl std::error::Error for TransportError {}

/// Conservative explicit-step ceiling for the sum of incident edge conductances.
///
/// The per-node Courant number is dt * sum(conductance) / compartment volume.
pub const MAX_EXPLICIT_TRANSPORT_COURANT: f64 = 1.0;

/// Apply passive diffusion-like exchange over existing active graph segments.
///
/// For a segment directed from a to b:
///
/// signed_flux_mg = conductance_ml_per_hour * dt_hours * (c_a - c_b)
///
/// where concentrations are in mg/mL. All proposed fluxes are computed from
/// the same pre-step state, then settled together. The operation is atomic:
/// invalid edges, an unstable explicit time step, or donor overdraw reject the
/// complete step and leave compartments unchanged.
pub fn apply_passive_transport(
    graph: &HyphalGraph,
    compartments: &mut [SoluteCompartment],
    edges: &[PassiveTransportEdge],
    dt_hours: f64,
) -> Result<TransportReport, TransportError> {
    if !dt_hours.is_finite() || dt_hours <= 0.0 {
        return Err(TransportError::InvalidTimeStep);
    }

    let Some(first) = compartments.first() else {
        return Err(TransportError::EmptyCompartments);
    };
    let solute = first.solute;

    let mut index_by_node = BTreeMap::new();
    let mut total_before = 0.0;
    for (index, compartment) in compartments.iter().enumerate() {
        if compartment.solute != solute {
            return Err(TransportError::MixedSolutes);
        }
        if !compartment.volume_ml.is_finite() || compartment.volume_ml <= 0.0 {
            return Err(TransportError::InvalidVolume(compartment.node));
        }
        if !compartment.mass_mg.is_finite() || compartment.mass_mg < 0.0 {
            return Err(TransportError::InvalidMass(compartment.node));
        }
        if index_by_node.insert(compartment.node, index).is_some() {
            return Err(TransportError::DuplicateCompartment(compartment.node));
        }
        total_before += compartment.mass_mg;
        if !total_before.is_finite() {
            return Err(TransportError::NonFiniteTotalMass);
        }
    }

    let mut seen_edges = BTreeSet::new();
    let mut proposed_delta = vec![0.0_f64; compartments.len()];
    let mut proposed_outgoing = vec![0.0_f64; compartments.len()];
    let mut incident_conductance = vec![0.0_f64; compartments.len()];
    let mut fluxes = Vec::with_capacity(edges.len());

    for edge in edges {
        if !edge.conductance_ml_per_hour.is_finite() || edge.conductance_ml_per_hour <= 0.0 {
            return Err(TransportError::InvalidConductance(edge.segment));
        }
        if !seen_edges.insert(edge.segment) {
            return Err(TransportError::DuplicateTransportEdge(edge.segment));
        }

        let segment = graph
            .segment(edge.segment)
            .ok_or(TransportError::MissingSegment(edge.segment))?;
        if segment.state != HyphalSegmentState::Active {
            return Err(TransportError::InactiveSegment(edge.segment));
        }

        let from_index = *index_by_node
            .get(&segment.from)
            .ok_or(TransportError::MissingCompartment(segment.from))?;
        let to_index = *index_by_node
            .get(&segment.to)
            .ok_or(TransportError::MissingCompartment(segment.to))?;
        incident_conductance[from_index] += edge.conductance_ml_per_hour;
        incident_conductance[to_index] += edge.conductance_ml_per_hour;
        if !incident_conductance[from_index].is_finite()
            || !incident_conductance[to_index].is_finite()
        {
            return Err(TransportError::NonFiniteResult(segment.from));
        }
        let from = compartments[from_index];
        let to = compartments[to_index];
        let concentration_from = from.mass_mg / from.volume_ml;
        let concentration_to = to.mass_mg / to.volume_ml;
        let flux =
            edge.conductance_ml_per_hour * dt_hours * (concentration_from - concentration_to);

        if !flux.is_finite() {
            return Err(TransportError::NonFiniteResult(segment.from));
        }

        proposed_delta[from_index] -= flux;
        proposed_delta[to_index] += flux;
        if !proposed_delta[from_index].is_finite() || !proposed_delta[to_index].is_finite() {
            return Err(TransportError::NonFiniteResult(segment.from));
        }
        if flux > 0.0 {
            proposed_outgoing[from_index] += flux;
        } else {
            proposed_outgoing[to_index] += -flux;
        }
        if !proposed_outgoing[from_index].is_finite() || !proposed_outgoing[to_index].is_finite() {
            return Err(TransportError::NonFiniteResult(segment.from));
        }
        fluxes.push(EdgeFlux {
            segment: edge.segment,
            signed_mass_mg: flux,
        });
    }

    // Report a donor-budget violation before the stricter time-step stability
    // guard so the diagnostic identifies the immediate physical constraint.
    for (index, outgoing) in proposed_outgoing.iter().copied().enumerate() {
        let available = compartments[index].mass_mg;
        if outgoing > available {
            return Err(TransportError::InsufficientDonorMass {
                node: compartments[index].node,
                requested_mg: outgoing,
                available_mg: available,
            });
        }
    }

    for (index, total_conductance) in incident_conductance.iter().copied().enumerate() {
        let courant = dt_hours * total_conductance / compartments[index].volume_ml;
        if !courant.is_finite() || courant > MAX_EXPLICIT_TRANSPORT_COURANT {
            return Err(TransportError::ExplicitStepUnstable {
                node: compartments[index].node,
                courant,
                max_courant: MAX_EXPLICIT_TRANSPORT_COURANT,
            });
        }
    }

    let mut next = compartments.to_vec();
    let mut total_after = 0.0;
    for (index, delta) in proposed_delta.iter().copied().enumerate() {
        let next_mass = compartments[index].mass_mg + delta;
        if !next_mass.is_finite() || next_mass < 0.0 {
            return Err(TransportError::NonFiniteResult(compartments[index].node));
        }
        next[index].mass_mg = next_mass;
        total_after += next_mass;
        if !total_after.is_finite() {
            return Err(TransportError::NonFiniteTotalMass);
        }
    }

    let residual = total_after - total_before;
    if !residual.is_finite() {
        return Err(TransportError::NonFiniteTotalMass);
    }
    compartments.copy_from_slice(&next);

    Ok(TransportReport {
        solute,
        dt_hours,
        total_mass_before_mg: total_before,
        total_mass_after_mg: total_after,
        conservation_residual_mg: residual,
        edge_fluxes: fluxes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyphal_graph::{HyphalNode, HyphalNodeKind, HyphalSegment, Position3Mm};

    fn node(id: u64, x: f64) -> HyphalNode {
        HyphalNode {
            id: NodeId(id),
            kind: HyphalNodeKind::Anchor,
            position: Position3Mm::new(x, 0.0, 0.0),
        }
    }

    fn graph_with_edge(state: HyphalSegmentState) -> HyphalGraph {
        let mut graph = HyphalGraph::new();
        graph.add_node(node(1, 0.0)).unwrap();
        graph.add_node(node(2, 10.0)).unwrap();
        graph
            .add_segment(HyphalSegment {
                id: SegmentId(7),
                from: NodeId(1),
                to: NodeId(2),
                length_mm: 10.0,
                diameter_mm: Some(0.02),
                state,
            })
            .unwrap();
        graph
    }

    fn compartments(left_mass: f64, right_mass: f64) -> Vec<SoluteCompartment> {
        vec![
            SoluteCompartment {
                node: NodeId(1),
                solute: SoluteId(42),
                volume_ml: 1.0,
                mass_mg: left_mass,
            },
            SoluteCompartment {
                node: NodeId(2),
                solute: SoluteId(42),
                volume_ml: 1.0,
                mass_mg: right_mass,
            },
        ]
    }

    fn edge(conductance: f64) -> PassiveTransportEdge {
        PassiveTransportEdge {
            segment: SegmentId(7),
            conductance_ml_per_hour: conductance,
        }
    }

    #[test]
    fn equal_concentrations_produce_zero_flux() {
        let graph = graph_with_edge(HyphalSegmentState::Active);
        let mut state = compartments(5.0, 5.0);
        let report = apply_passive_transport(&graph, &mut state, &[edge(0.1)], 1.0).unwrap();
        assert_eq!(report.edge_fluxes[0].signed_mass_mg, 0.0);
        assert_eq!(state[0].mass_mg, 5.0);
        assert_eq!(state[1].mass_mg, 5.0);
    }

    #[test]
    fn passive_flux_moves_solute_down_gradient_and_conserves_mass() {
        let graph = graph_with_edge(HyphalSegmentState::Active);
        let mut state = compartments(10.0, 0.0);
        let report = apply_passive_transport(&graph, &mut state, &[edge(0.1)], 1.0).unwrap();
        assert_eq!(report.edge_fluxes[0].signed_mass_mg, 1.0);
        assert_eq!(state[0].mass_mg, 9.0);
        assert_eq!(state[1].mass_mg, 1.0);
        assert_eq!(report.total_mass_before_mg, report.total_mass_after_mg);
        assert_eq!(report.conservation_residual_mg, 0.0);
    }

    #[test]
    fn donor_overdraw_rejects_step_without_mutation() {
        let graph = graph_with_edge(HyphalSegmentState::Active);
        let mut state = compartments(1.0, 0.0);
        let before = state.clone();
        let result = apply_passive_transport(&graph, &mut state, &[edge(2.0)], 1.0);
        assert!(matches!(
            result,
            Err(TransportError::InsufficientDonorMass { .. })
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn unstable_explicit_step_is_rejected_even_without_donor_overdraw() {
        let graph = graph_with_edge(HyphalSegmentState::Active);
        let mut state = vec![
            SoluteCompartment {
                node: NodeId(1),
                solute: SoluteId(42),
                volume_ml: 100.0,
                mass_mg: 1_000.0,
            },
            SoluteCompartment {
                node: NodeId(2),
                solute: SoluteId(42),
                volume_ml: 1.0,
                mass_mg: 0.0,
            },
        ];
        let before = state.clone();
        let result = apply_passive_transport(&graph, &mut state, &[edge(2.0)], 1.0);
        assert!(matches!(
            result,
            Err(TransportError::ExplicitStepUnstable {
                node: NodeId(2),
                ..
            })
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn inactive_segment_cannot_transport() {
        let graph = graph_with_edge(HyphalSegmentState::Inactive);
        let mut state = compartments(10.0, 0.0);
        let before = state.clone();
        assert_eq!(
            apply_passive_transport(&graph, &mut state, &[edge(0.1)], 1.0),
            Err(TransportError::InactiveSegment(SegmentId(7)))
        );
        assert_eq!(state, before);
    }

    #[test]
    fn duplicate_edge_is_rejected_atomically() {
        let graph = graph_with_edge(HyphalSegmentState::Active);
        let mut state = compartments(10.0, 0.0);
        let before = state.clone();
        assert_eq!(
            apply_passive_transport(&graph, &mut state, &[edge(0.1), edge(0.1)], 1.0),
            Err(TransportError::DuplicateTransportEdge(SegmentId(7)))
        );
        assert_eq!(state, before);
    }

    #[test]
    fn empty_compartment_input_is_rejected() {
        let graph = graph_with_edge(HyphalSegmentState::Active);
        let mut state = Vec::new();
        assert_eq!(
            apply_passive_transport(&graph, &mut state, &[edge(0.1)], 1.0),
            Err(TransportError::EmptyCompartments)
        );
    }

    #[test]
    fn mixed_solutes_are_rejected_without_mutation() {
        let graph = graph_with_edge(HyphalSegmentState::Active);
        let mut state = compartments(10.0, 0.0);
        state[1].solute = SoluteId(43);
        let before = state.clone();
        assert_eq!(
            apply_passive_transport(&graph, &mut state, &[edge(0.1)], 1.0),
            Err(TransportError::MixedSolutes)
        );
        assert_eq!(state, before);
    }

    #[test]
    fn missing_compartment_rejects_transport_without_mutation() {
        let graph = graph_with_edge(HyphalSegmentState::Active);
        let mut state = vec![compartments(10.0, 0.0)[0]];
        let before = state.clone();
        assert_eq!(
            apply_passive_transport(&graph, &mut state, &[edge(0.1)], 1.0),
            Err(TransportError::MissingCompartment(NodeId(2)))
        );
        assert_eq!(state, before);
    }

    #[test]
    fn invalid_time_step_is_rejected() {
        let graph = graph_with_edge(HyphalSegmentState::Active);
        let mut state = compartments(10.0, 0.0);
        let before = state.clone();
        assert_eq!(
            apply_passive_transport(&graph, &mut state, &[edge(0.1)], f64::NAN),
            Err(TransportError::InvalidTimeStep)
        );
        assert_eq!(state, before);
    }

    #[test]
    fn multiple_edges_settle_simultaneously_and_conserve_total_mass() {
        let mut graph = HyphalGraph::new();
        for (id, x) in [(1, 0.0), (2, 5.0), (3, 10.0)] {
            graph.add_node(node(id, x)).unwrap();
        }
        for (id, from, to) in [(7, 1, 2), (8, 2, 3)] {
            graph
                .add_segment(HyphalSegment {
                    id: SegmentId(id),
                    from: NodeId(from),
                    to: NodeId(to),
                    length_mm: 5.0,
                    diameter_mm: Some(0.02),
                    state: HyphalSegmentState::Active,
                })
                .unwrap();
        }
        let mut state = vec![
            SoluteCompartment {
                node: NodeId(1),
                solute: SoluteId(42),
                volume_ml: 1.0,
                mass_mg: 10.0,
            },
            SoluteCompartment {
                node: NodeId(2),
                solute: SoluteId(42),
                volume_ml: 1.0,
                mass_mg: 0.0,
            },
            SoluteCompartment {
                node: NodeId(3),
                solute: SoluteId(42),
                volume_ml: 1.0,
                mass_mg: 0.0,
            },
        ];
        let before: f64 = state.iter().map(|item| item.mass_mg).sum();
        let report = apply_passive_transport(
            &graph,
            &mut state,
            &[
                PassiveTransportEdge {
                    segment: SegmentId(7),
                    conductance_ml_per_hour: 0.1,
                },
                PassiveTransportEdge {
                    segment: SegmentId(8),
                    conductance_ml_per_hour: 0.1,
                },
            ],
            1.0,
        )
        .unwrap();
        let after: f64 = state.iter().map(|item| item.mass_mg).sum();
        assert_eq!(before, after);
        assert_eq!(report.edge_fluxes.len(), 2);
        assert!(state.iter().all(|item| item.mass_mg >= 0.0));
    }
}
