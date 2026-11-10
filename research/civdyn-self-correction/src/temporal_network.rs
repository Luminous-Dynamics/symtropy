// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic temporal-network summaries for CIVDYN.
//!
//! This v0.1 reference distinguishes static union-graph structure from
//! time-respecting reachability. Contacts are undirected, unweighted and
//! instantaneous at integer ticks. Temporal paths require strictly increasing
//! ticks; simultaneous contacts cannot be chained within the same tick.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

use crate::joint_adequacy::JointComponentValue;
use crate::network_adequacy::{
    NetworkAdequacyError, NetworkSummary, UndirectedNetwork,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TemporalContact {
    pub tick: u64,
    pub left: usize,
    pub right: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalNetworkError {
    ZeroNodes,
    NodeOutOfRange { node: usize, node_count: usize },
    SelfLoop { node: usize },
    DuplicateContact { tick: u64, left: usize, right: usize },
    CountOverflow,
    UndefinedTemporalDelay,
    Network(NetworkAdequacyError),
}

impl fmt::Display for TemporalNetworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TemporalNetworkError {}

impl From<NetworkAdequacyError> for TemporalNetworkError {
    fn from(value: NetworkAdequacyError) -> Self {
        Self::Network(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalNetwork {
    node_count: usize,
    canonical_contacts: Vec<TemporalContact>,
    union_graph: UndirectedNetwork,
}

impl TemporalNetwork {
    pub fn new(
        node_count: usize,
        contacts: impl IntoIterator<Item = TemporalContact>,
    ) -> Result<Self, TemporalNetworkError> {
        if node_count == 0 {
            return Err(TemporalNetworkError::ZeroNodes);
        }

        let mut canonical_contacts = Vec::new();
        for contact in contacts {
            if contact.left >= node_count {
                return Err(TemporalNetworkError::NodeOutOfRange {
                    node: contact.left,
                    node_count,
                });
            }
            if contact.right >= node_count {
                return Err(TemporalNetworkError::NodeOutOfRange {
                    node: contact.right,
                    node_count,
                });
            }
            if contact.left == contact.right {
                return Err(TemporalNetworkError::SelfLoop {
                    node: contact.left,
                });
            }
            canonical_contacts.push(TemporalContact {
                tick: contact.tick,
                left: contact.left.min(contact.right),
                right: contact.left.max(contact.right),
            });
        }

        canonical_contacts.sort_unstable();
        for pair in canonical_contacts.windows(2) {
            if pair[0] == pair[1] {
                return Err(TemporalNetworkError::DuplicateContact {
                    tick: pair[0].tick,
                    left: pair[0].left,
                    right: pair[0].right,
                });
            }
        }

        let unique_edges: BTreeSet<(usize, usize)> = canonical_contacts
            .iter()
            .map(|contact| (contact.left, contact.right))
            .collect();
        let union_graph = UndirectedNetwork::new(node_count, unique_edges)?;

        Ok(Self {
            node_count,
            canonical_contacts,
            union_graph,
        })
    }

    pub fn node_count(&self) -> usize {
        self.node_count
    }

    pub fn canonical_contacts(&self) -> &[TemporalContact] {
        &self.canonical_contacts
    }

    pub fn union_graph(&self) -> &UndirectedNetwork {
        &self.union_graph
    }

    pub fn summarize(&self) -> Result<TemporalNetworkSummary, TemporalNetworkError> {
        let static_union = self.union_graph.summarize();
        let event_count = self.canonical_contacts.len();
        let unique_edge_count = self.union_graph.edge_count();
        let active_tick_count = self
            .canonical_contacts
            .iter()
            .map(|contact| contact.tick)
            .collect::<BTreeSet<_>>()
            .len();
        let first_tick = self.canonical_contacts.first().map(|contact| contact.tick);

        let mut reachable_pairs = Vec::new();
        let mut total_delay = 0_u128;

        if let Some(first_tick) = first_tick {
            for source in 0..self.node_count {
                let mut reached = vec![false; self.node_count];
                // None is reserved for the source, which is available
                // immediately before the first event tick. Other Some(t)
                // values are earliest arrival ticks.
                let mut reached_at = vec![None; self.node_count];
                reached[source] = true;

                let mut start = 0;
                while start < self.canonical_contacts.len() {
                    let tick = self.canonical_contacts[start].tick;
                    let mut end = start + 1;
                    while end < self.canonical_contacts.len()
                        && self.canonical_contacts[end].tick == tick
                    {
                        end += 1;
                    }

                    // Apply every contact at this tick against the state that
                    // existed strictly before this tick. Newly reached nodes
                    // are committed only after the complete timestamp batch.
                    let mut newly_reached = BTreeSet::new();
                    for contact in &self.canonical_contacts[start..end] {
                        let left_available = reached[contact.left]
                            && reached_at[contact.left].is_none_or(|arrival| arrival < tick);
                        let right_available = reached[contact.right]
                            && reached_at[contact.right].is_none_or(|arrival| arrival < tick);

                        if left_available && !reached[contact.right] {
                            newly_reached.insert(contact.right);
                        }
                        if right_available && !reached[contact.left] {
                            newly_reached.insert(contact.left);
                        }
                    }

                    for node in newly_reached {
                        if !reached[node] {
                            reached[node] = true;
                            reached_at[node] = Some(tick);
                        }
                    }
                    start = end;
                }

                for destination in 0..self.node_count {
                    if destination == source || !reached[destination] {
                        continue;
                    }
                    let arrival_tick = reached_at[destination]
                        .ok_or(TemporalNetworkError::UndefinedTemporalDelay)?;
                    let delay = arrival_tick
                        .checked_sub(first_tick)
                        .ok_or(TemporalNetworkError::CountOverflow)?;
                    total_delay = total_delay
                        .checked_add(u128::from(delay))
                        .ok_or(TemporalNetworkError::CountOverflow)?;
                    reachable_pairs.push(TemporalReachabilityReceipt {
                        source,
                        destination,
                        arrival_tick,
                        delay_from_first_tick: delay,
                    });
                }
            }
        }

        let temporal_reachable_ordered_pairs = reachable_pairs.len();
        let possible_ordered_pairs = (self.node_count as u128)
            .checked_mul(self.node_count.saturating_sub(1) as u128)
            .ok_or(TemporalNetworkError::CountOverflow)?;
        let temporal_reachability_fraction = if possible_ordered_pairs == 0 {
            0.0
        } else {
            temporal_reachable_ordered_pairs as f64 / possible_ordered_pairs as f64
        };
        let mean_earliest_arrival_delay = (temporal_reachable_ordered_pairs > 0)
            .then(|| total_delay as f64 / temporal_reachable_ordered_pairs as f64);

        Ok(TemporalNetworkSummary {
            static_union,
            event_count,
            unique_edge_count,
            active_tick_count,
            temporal_reachable_ordered_pairs,
            temporal_reachability_fraction,
            mean_earliest_arrival_delay,
            reachable_pairs,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalReachabilityReceipt {
    pub source: usize,
    pub destination: usize,
    pub arrival_tick: u64,
    pub delay_from_first_tick: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemporalNetworkSummary {
    pub static_union: NetworkSummary,
    pub event_count: usize,
    pub unique_edge_count: usize,
    pub active_tick_count: usize,
    pub temporal_reachable_ordered_pairs: usize,
    pub temporal_reachability_fraction: f64,
    /// Mean delay from the first event tick over reachable ordered pairs.
    /// None means there are no time-respecting paths between distinct nodes.
    pub mean_earliest_arrival_delay: Option<f64>,
    /// Canonically ordered by source then destination.
    pub reachable_pairs: Vec<TemporalReachabilityReceipt>,
}

impl TemporalNetworkSummary {
    /// Append temporal components to the seven static union-network components.
    /// A temporal mean delay is required; no sentinel is invented for a
    /// network without any time-respecting path between distinct nodes.
    pub fn joint_component_values(
        &self,
    ) -> Result<Vec<JointComponentValue>, TemporalNetworkError> {
        let mut values = self.static_union.joint_component_values()?;
        let mean_delay = self
            .mean_earliest_arrival_delay
            .ok_or(TemporalNetworkError::UndefinedTemporalDelay)?;
        values.extend([
            JointComponentValue {
                component_id: "temporal-event-count".to_string(),
                measurement_unit: "contacts".to_string(),
                value: self.event_count as f64,
            },
            JointComponentValue {
                component_id: "temporal-unique-edge-count".to_string(),
                measurement_unit: "edges".to_string(),
                value: self.unique_edge_count as f64,
            },
            JointComponentValue {
                component_id: "temporal-active-tick-count".to_string(),
                measurement_unit: "ticks".to_string(),
                value: self.active_tick_count as f64,
            },
            JointComponentValue {
                component_id: "temporal-reachable-ordered-pairs".to_string(),
                measurement_unit: "ordered_pairs".to_string(),
                value: self.temporal_reachable_ordered_pairs as f64,
            },
            JointComponentValue {
                component_id: "temporal-reachability-fraction".to_string(),
                measurement_unit: "ratio".to_string(),
                value: self.temporal_reachability_fraction,
            },
            JointComponentValue {
                component_id: "temporal-mean-earliest-arrival-delay".to_string(),
                measurement_unit: "ticks".to_string(),
                value: mean_delay,
            },
        ]);
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::joint_adequacy::{
        assess_joint_predictive_draws, JointComponentSpec, JointMetricConfig, JointObservation,
        JointPredictiveDraw, JointStatisticSpec,
    };
    use crate::model_adequacy::{IndependentUnitKind, SplitManifest, SplitRole};

    fn forward_path() -> TemporalNetwork {
        TemporalNetwork::new(
            4,
            [
                TemporalContact {
                    tick: 1,
                    left: 0,
                    right: 1,
                },
                TemporalContact {
                    tick: 2,
                    left: 1,
                    right: 2,
                },
                TemporalContact {
                    tick: 3,
                    left: 2,
                    right: 3,
                },
            ],
        )
        .expect("valid temporal path")
    }

    fn scrambled_path() -> TemporalNetwork {
        TemporalNetwork::new(
            4,
            [
                TemporalContact {
                    tick: 1,
                    left: 0,
                    right: 1,
                },
                TemporalContact {
                    tick: 2,
                    left: 2,
                    right: 3,
                },
                TemporalContact {
                    tick: 3,
                    left: 1,
                    right: 2,
                },
            ],
        )
        .expect("valid scrambled temporal path")
    }

    #[test]
    fn identical_static_union_graphs_can_have_different_temporal_reachability() {
        let forward = forward_path().summarize().expect("forward summary");
        let scrambled = scrambled_path().summarize().expect("scrambled summary");

        assert_eq!(forward.static_union, scrambled.static_union);
        assert_eq!(forward.static_union.degree_sequence, vec![2, 2, 1, 1]);
        assert_eq!(forward.event_count, scrambled.event_count);
        assert_eq!(forward.unique_edge_count, scrambled.unique_edge_count);
        assert_eq!(forward.active_tick_count, scrambled.active_tick_count);

        assert_eq!(forward.temporal_reachable_ordered_pairs, 9);
        assert_eq!(scrambled.temporal_reachable_ordered_pairs, 8);
        assert_eq!(forward.temporal_reachability_fraction, 0.75);
        assert!((scrambled.temporal_reachability_fraction - (8.0 / 12.0)).abs() < 1e-12);
        assert!((forward.mean_earliest_arrival_delay.expect("forward delay") - (11.0 / 9.0)).abs() < 1e-12);
        assert!((scrambled.mean_earliest_arrival_delay.expect("scrambled delay") - 1.25).abs() < 1e-12);
    }

    #[test]
    fn same_tick_contacts_cannot_form_a_forwarding_cascade() {
        let graph = TemporalNetwork::new(
            3,
            [
                TemporalContact {
                    tick: 0,
                    left: 0,
                    right: 1,
                },
                TemporalContact {
                    tick: 0,
                    left: 1,
                    right: 2,
                },
            ],
        )
        .expect("simultaneous contacts");
        let summary = graph.summarize().expect("simultaneous summary");
        assert_eq!(summary.temporal_reachable_ordered_pairs, 4);
        assert!(!summary
            .reachable_pairs
            .iter()
            .any(|pair| pair.source == 0 && pair.destination == 2));
        assert!(!summary
            .reachable_pairs
            .iter()
            .any(|pair| pair.source == 2 && pair.destination == 0));
    }

    #[test]
    fn contact_input_order_and_endpoint_orientation_are_canonical() {
        let first = TemporalNetwork::new(
            4,
            [
                TemporalContact { tick: 1, left: 0, right: 1 },
                TemporalContact { tick: 2, left: 1, right: 2 },
                TemporalContact { tick: 3, left: 2, right: 3 },
            ],
        )
        .expect("canonical input");
        let reordered = TemporalNetwork::new(
            4,
            [
                TemporalContact { tick: 3, left: 3, right: 2 },
                TemporalContact { tick: 1, left: 1, right: 0 },
                TemporalContact { tick: 2, left: 2, right: 1 },
            ],
        )
        .expect("reordered input");
        assert_eq!(first.canonical_contacts(), reordered.canonical_contacts());
        assert_eq!(first.summarize(), reordered.summarize());
    }

    #[test]
    fn malformed_temporal_networks_fail_closed() {
        assert_eq!(
            TemporalNetwork::new(0, []).unwrap_err(),
            TemporalNetworkError::ZeroNodes
        );
        assert_eq!(
            TemporalNetwork::new(
                3,
                [TemporalContact {
                    tick: 1,
                    left: 2,
                    right: 2,
                }]
            )
            .unwrap_err(),
            TemporalNetworkError::SelfLoop { node: 2 }
        );
        assert_eq!(
            TemporalNetwork::new(
                3,
                [TemporalContact {
                    tick: 1,
                    left: 0,
                    right: 3,
                }]
            )
            .unwrap_err(),
            TemporalNetworkError::NodeOutOfRange {
                node: 3,
                node_count: 3
            }
        );
        assert_eq!(
            TemporalNetwork::new(
                3,
                [
                    TemporalContact { tick: 7, left: 0, right: 1 },
                    TemporalContact { tick: 7, left: 1, right: 0 },
                ]
            )
            .unwrap_err(),
            TemporalNetworkError::DuplicateContact {
                tick: 7,
                left: 0,
                right: 1,
            }
        );
    }

    fn statistic_schema(observed: &[JointComponentValue]) -> JointStatisticSpec {
        let components = observed
            .iter()
            .map(|component| JointComponentSpec {
                id: component.component_id.clone(),
                measurement_unit: component.measurement_unit.clone(),
                reference_center: component.value,
                reference_scale: component.value.abs().max(1.0),
            })
            .collect();
        JointStatisticSpec {
            id: "held-out-temporal-network".to_string(),
            source_family: "synthetic-temporal-network-known-answer".to_string(),
            source_digest: "sha256-temporal-network-source".to_string(),
            population: "four-node-temporal-network".to_string(),
            context: "same-union-different-event-order".to_string(),
            independent_unit: IndependentUnitKind::Network,
            role: SplitRole::Validation,
            primary: true,
            required: true,
            minimum_draws: 20,
            components,
        }
    }

    fn split_manifest() -> SplitManifest {
        SplitManifest {
            id: "temporal-network-split-v1".to_string(),
            independent_unit: IndependentUnitKind::Network,
            calibration_data_digest: "sha256-temporal-network-calibration".to_string(),
            manifest_digest: "sha256-temporal-network-manifest".to_string(),
            calibration_units: vec!["temporal-network-calibration".to_string()],
            validation_units: vec!["temporal-network-held-out".to_string()],
            challenge_units: Vec::new(),
        }
    }

    fn assess_candidate(
        predicted: &TemporalNetwork,
        model_digest: &str,
    ) -> crate::joint_adequacy::JointAdequacyResult {
        let observed_summary = forward_path().summarize().expect("observed summary");
        let predicted_summary = predicted.summarize().expect("predicted summary");
        let observed_components = observed_summary
            .joint_component_values()
            .expect("observed joint components");
        let predicted_components = predicted_summary
            .joint_component_values()
            .expect("predicted joint components");
        let statistic = statistic_schema(&observed_components);
        let manifest = split_manifest();
        let observation = JointObservation {
            statistic_id: statistic.id.clone(),
            independent_unit_id: "temporal-network-held-out".to_string(),
            sample_size: 4,
            source_digest: statistic.source_digest.clone(),
            observation_digest: "sha256-temporal-network-observation".to_string(),
            split_manifest_digest: manifest.manifest_digest.clone(),
            components: observed_components,
        };
        let draws: Vec<JointPredictiveDraw> = (0..20)
            .map(|draw_index| JointPredictiveDraw {
                statistic_id: statistic.id.clone(),
                independent_unit_id: "temporal-network-held-out".to_string(),
                sample_size: 4,
                draw_index,
                model_digest: model_digest.to_string(),
                draw_set_digest: format!("sha256-temporal-network-draws-{model_digest}"),
                calibration_data_digest: Some(manifest.calibration_data_digest.clone()),
                split_manifest_digest: manifest.manifest_digest.clone(),
                components: predicted_components.clone(),
            })
            .collect();
        assess_joint_predictive_draws(
            &statistic,
            &manifest,
            SplitRole::Validation,
            &observation,
            &draws,
            &JointMetricConfig {
                energy_beta: 1.0,
                variogram_power: 1.0,
                pair_weights: vec![1.0; 78],
            },
        )
        .expect("temporal network joint assessment")
    }

    #[test]
    fn held_out_joint_adequacy_detects_wrong_event_order() {
        let correct = assess_candidate(&forward_path(), "sha256-temporal-model-correct");
        let wrong = assess_candidate(&scrambled_path(), "sha256-temporal-model-wrong");
        assert_eq!(correct.energy_score, 0.0);
        assert_eq!(correct.variogram_score, 0.0);
        assert!(wrong.energy_score > 0.0);
        assert!(wrong.variogram_score > 0.0);
    }
}
