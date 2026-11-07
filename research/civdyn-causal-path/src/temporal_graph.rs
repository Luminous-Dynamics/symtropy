// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Time-indexed mechanism graph validation.
//!
//! Instantaneous (zero-lag) dependencies must form a DAG. Delayed feedback
//! cycles are permitted and explicit. A graph path is structural model
//! evidence only; it is not empirical proof of causality.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalVariable {
    pub id: String,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub lag_steps: u32,
    pub mechanism_id: String,
    /// Required whenever source and target variables have different units.
    pub unit_transform_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalGraph {
    pub variables: Vec<TemporalVariable>,
    pub edges: Vec<TemporalEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalGraphError {
    EmptyGraph,
    EmptyVariable(String),
    DuplicateVariable(String),
    EmptyUnit(String),
    EmptyEdge(String),
    DuplicateEdge(String),
    EmptyMechanism(String),
    UnknownEndpoint {
        edge_id: String,
        variable_id: String,
    },
    MissingUnitTransform(String),
    EmptyUnitTransform(String),
    InstantaneousCycle,
}

impl fmt::Display for TemporalGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGraph => write!(f, "temporal graph must contain at least one variable"),
            Self::EmptyVariable(id) => write!(f, "variable ID must not be empty: {id}"),
            Self::DuplicateVariable(id) => write!(f, "duplicate variable ID: {id}"),
            Self::EmptyUnit(id) => write!(f, "variable {id} has an empty unit"),
            Self::EmptyEdge(id) => write!(f, "edge ID must not be empty: {id}"),
            Self::DuplicateEdge(id) => write!(f, "duplicate edge ID: {id}"),
            Self::EmptyMechanism(id) => write!(f, "edge {id} has an empty mechanism ID"),
            Self::UnknownEndpoint {
                edge_id,
                variable_id,
            } => {
                write!(
                    f,
                    "edge {edge_id} references unknown variable {variable_id}"
                )
            }
            Self::MissingUnitTransform(id) => {
                write!(
                    f,
                    "edge {id} changes units without a declared transformation"
                )
            }
            Self::EmptyUnitTransform(id) => {
                write!(f, "edge {id} has an empty unit transformation ID")
            }
            Self::InstantaneousCycle => {
                write!(f, "zero-lag dependency subgraph contains a directed cycle")
            }
        }
    }
}

impl std::error::Error for TemporalGraphError {}

impl TemporalGraph {
    pub fn validate(&self) -> Result<(), TemporalGraphError> {
        if self.variables.is_empty() {
            return Err(TemporalGraphError::EmptyGraph);
        }
        let mut variables = BTreeMap::new();
        for variable in &self.variables {
            if variable.id.trim().is_empty() {
                return Err(TemporalGraphError::EmptyVariable(variable.id.clone()));
            }
            if variable.unit.trim().is_empty() {
                return Err(TemporalGraphError::EmptyUnit(variable.id.clone()));
            }
            if variables.insert(variable.id.as_str(), variable).is_some() {
                return Err(TemporalGraphError::DuplicateVariable(variable.id.clone()));
            }
        }

        let mut edge_ids = BTreeSet::new();
        let mut indegree = variables
            .keys()
            .map(|id| (*id, 0usize))
            .collect::<BTreeMap<_, _>>();
        let mut outgoing: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

        for edge in &self.edges {
            if edge.id.trim().is_empty() {
                return Err(TemporalGraphError::EmptyEdge(edge.id.clone()));
            }
            if !edge_ids.insert(edge.id.as_str()) {
                return Err(TemporalGraphError::DuplicateEdge(edge.id.clone()));
            }
            if edge.mechanism_id.trim().is_empty() {
                return Err(TemporalGraphError::EmptyMechanism(edge.id.clone()));
            }
            let source = variables.get(edge.from.as_str()).ok_or_else(|| {
                TemporalGraphError::UnknownEndpoint {
                    edge_id: edge.id.clone(),
                    variable_id: edge.from.clone(),
                }
            })?;
            let target = variables.get(edge.to.as_str()).ok_or_else(|| {
                TemporalGraphError::UnknownEndpoint {
                    edge_id: edge.id.clone(),
                    variable_id: edge.to.clone(),
                }
            })?;

            if source.unit != target.unit {
                match edge.unit_transform_id.as_deref() {
                    None => return Err(TemporalGraphError::MissingUnitTransform(edge.id.clone())),
                    Some(id) if id.trim().is_empty() => {
                        return Err(TemporalGraphError::EmptyUnitTransform(edge.id.clone()));
                    }
                    Some(_) => {}
                }
            } else if edge.unit_transform_id.as_deref().is_some_and(str::is_empty) {
                return Err(TemporalGraphError::EmptyUnitTransform(edge.id.clone()));
            }

            // Delayed feedback is valid. Only instantaneous edges constrain
            // same-time causal ordering.
            if edge.lag_steps == 0 {
                *indegree.get_mut(edge.to.as_str()).expect("endpoint exists") += 1;
                outgoing
                    .entry(edge.from.as_str())
                    .or_default()
                    .push(edge.to.as_str());
            }
        }

        let mut queue = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| *id)
            .collect::<VecDeque<_>>();
        let mut visited = 0usize;
        while let Some(node) = queue.pop_front() {
            visited += 1;
            for next in outgoing.get(node).into_iter().flatten() {
                let degree = indegree.get_mut(next).expect("endpoint exists");
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(next);
                }
            }
        }
        if visited != self.variables.len() {
            return Err(TemporalGraphError::InstantaneousCycle);
        }
        Ok(())
    }

    pub fn has_path(&self, from: &str, to: &str) -> Result<bool, TemporalGraphError> {
        self.validate()?;
        let known = self
            .variables
            .iter()
            .map(|variable| variable.id.as_str())
            .collect::<BTreeSet<_>>();
        if !known.contains(from) {
            return Err(TemporalGraphError::UnknownEndpoint {
                edge_id: "path_query".into(),
                variable_id: from.into(),
            });
        }
        if !known.contains(to) {
            return Err(TemporalGraphError::UnknownEndpoint {
                edge_id: "path_query".into(),
                variable_id: to.into(),
            });
        }
        if from == to {
            return Ok(true);
        }

        let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for edge in &self.edges {
            adjacency
                .entry(edge.from.as_str())
                .or_default()
                .push(edge.to.as_str());
        }
        let mut visited = BTreeSet::from([from]);
        let mut queue = VecDeque::from([from]);
        while let Some(node) = queue.pop_front() {
            for next in adjacency.get(node).into_iter().flatten() {
                if *next == to {
                    return Ok(true);
                }
                if visited.insert(next) {
                    queue.push_back(next);
                }
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variable(id: &str, unit: &str) -> TemporalVariable {
        TemporalVariable {
            id: id.into(),
            unit: unit.into(),
        }
    }

    fn edge(id: &str, from: &str, to: &str, lag: u32, transform: Option<&str>) -> TemporalEdge {
        TemporalEdge {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            lag_steps: lag,
            mechanism_id: format!("mechanism_{id}"),
            unit_transform_id: transform.map(str::to_owned),
        }
    }

    #[test]
    fn zero_lag_graph_must_be_acyclic() {
        let graph = TemporalGraph {
            variables: vec![
                variable("a", "count"),
                variable("b", "count"),
                variable("c", "count"),
            ],
            edges: vec![
                edge("ab", "a", "b", 0, None),
                edge("bc", "b", "c", 0, None),
                edge("ca", "c", "a", 0, None),
            ],
        };
        assert_eq!(
            graph.validate(),
            Err(TemporalGraphError::InstantaneousCycle)
        );
    }

    #[test]
    fn delayed_feedback_cycle_is_explicit_and_valid() {
        let graph = TemporalGraph {
            variables: vec![
                variable("resource_stock", "m3"),
                variable("service", "fraction"),
                variable("maintenance", "hours"),
            ],
            edges: vec![
                edge(
                    "stock_to_service",
                    "resource_stock",
                    "service",
                    0,
                    Some("m3_to_fraction"),
                ),
                edge(
                    "service_to_maintenance",
                    "service",
                    "maintenance",
                    0,
                    Some("fraction_to_hours"),
                ),
                edge(
                    "maintenance_to_stock",
                    "maintenance",
                    "resource_stock",
                    2,
                    Some("hours_to_m3"),
                ),
            ],
        };
        assert!(graph.validate().is_ok());
        assert!(graph.has_path("resource_stock", "maintenance").unwrap());
        assert!(graph.has_path("maintenance", "resource_stock").unwrap());
    }

    #[test]
    fn unit_change_requires_an_explicit_transformation_identifier() {
        let graph = TemporalGraph {
            variables: vec![variable("water", "m3"), variable("coverage", "fraction")],
            edges: vec![edge("water_to_coverage", "water", "coverage", 0, None)],
        };
        assert_eq!(
            graph.validate(),
            Err(TemporalGraphError::MissingUnitTransform(
                "water_to_coverage".into()
            ))
        );
    }

    #[test]
    fn disconnected_variables_are_not_mistaken_for_a_path() {
        let graph = TemporalGraph {
            variables: vec![
                variable("reputation", "count"),
                variable("service", "fraction"),
            ],
            edges: vec![],
        };
        assert!(!graph.has_path("reputation", "service").unwrap());
    }
}
