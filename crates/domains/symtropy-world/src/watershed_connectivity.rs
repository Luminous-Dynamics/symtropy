// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Evidence-bound directed watershed topology and causal reachability.
//!
//! This module propagates causal relevance only. It never propagates water,
//! discharge, depth, salinity, sediment, or other hydrology state. A changed
//! downstream physical observation must still be produced by Hydrology authority.

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    error::Error,
    fmt,
};

use sha2::{Digest, Sha256};
use symtropy_sim_contracts::{
    AuthorityId, ContractError, DigestAlgorithm, ReferenceFrameId, ScopeId, SimInstant,
    TypedDigest32,
};

pub const WATERSHED_TOPOLOGY_SCHEMA_VERSION: u32 = 1;
pub const WATERSHED_TOPOLOGY_DIGEST_DOMAIN: &str = "symtropy.watershed.topology.v1";
pub const WATERSHED_RELATION_DOMAIN_PREFIX: &str =
    "symtropy.hydrology.watershed-connectivity.";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatershedConnectionEvidence {
    pub hydrology_authority: AuthorityId,
    pub upstream_scope: ScopeId,
    pub downstream_scope: ScopeId,
    pub reference_frame: ReferenceFrameId,
    pub observed_at: SimInstant,
    /// Hydrology-owned identity/evidence for this directed drainage relation.
    pub relation_digest: TypedDigest32,
}

impl WatershedConnectionEvidence {
    pub fn new(
        hydrology_authority: AuthorityId,
        upstream_scope: ScopeId,
        downstream_scope: ScopeId,
        reference_frame: ReferenceFrameId,
        observed_at: SimInstant,
        relation_digest: TypedDigest32,
    ) -> Result<Self, WatershedConnectivityError> {
        let edge = Self {
            hydrology_authority,
            upstream_scope,
            downstream_scope,
            reference_frame,
            observed_at,
            relation_digest,
        };
        edge.validate()?;
        Ok(edge)
    }

    pub fn validate(&self) -> Result<(), WatershedConnectivityError> {
        if self.upstream_scope == self.downstream_scope {
            return Err(WatershedConnectivityError::SelfEdge(
                self.upstream_scope.clone(),
            ));
        }
        self.relation_digest
            .validate()
            .map_err(WatershedConnectivityError::Contract)?;
        if !self
            .relation_digest
            .domain
            .starts_with(WATERSHED_RELATION_DOMAIN_PREFIX)
        {
            return Err(WatershedConnectivityError::InvalidRelationDigestDomain(
                self.relation_digest.domain.clone(),
            ));
        }
        Ok(())
    }
}

/// One exact Hydrology-authority view of a one-way drainage DAG.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatershedTopologySnapshot {
    pub schema_version: u32,
    pub hydrology_authority: AuthorityId,
    pub reference_frame: ReferenceFrameId,
    pub observed_at: SimInstant,
    /// Canonically sorted by `(upstream_scope, downstream_scope)`.
    pub edges: Vec<WatershedConnectionEvidence>,
}

impl WatershedTopologySnapshot {
    pub fn new(
        hydrology_authority: AuthorityId,
        reference_frame: ReferenceFrameId,
        observed_at: SimInstant,
        edges: impl IntoIterator<Item = WatershedConnectionEvidence>,
    ) -> Result<Self, WatershedConnectivityError> {
        let mut edges: Vec<_> = edges.into_iter().collect();
        edges.sort_by(|left, right| {
            left.upstream_scope
                .cmp(&right.upstream_scope)
                .then_with(|| left.downstream_scope.cmp(&right.downstream_scope))
        });
        let snapshot = Self {
            schema_version: WATERSHED_TOPOLOGY_SCHEMA_VERSION,
            hydrology_authority,
            reference_frame,
            observed_at,
            edges,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<(), WatershedConnectivityError> {
        if self.schema_version != WATERSHED_TOPOLOGY_SCHEMA_VERSION {
            return Err(WatershedConnectivityError::UnsupportedSchema {
                expected: WATERSHED_TOPOLOGY_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        if self.edges.is_empty() {
            return Err(WatershedConnectivityError::NoEdges);
        }

        for edge in &self.edges {
            edge.validate()?;
            if edge.hydrology_authority != self.hydrology_authority {
                return Err(WatershedConnectivityError::AuthorityMismatch {
                    expected: self.hydrology_authority.clone(),
                    actual: edge.hydrology_authority.clone(),
                });
            }
            if edge.reference_frame != self.reference_frame {
                return Err(WatershedConnectivityError::ReferenceFrameMismatch {
                    expected: self.reference_frame.clone(),
                    actual: edge.reference_frame.clone(),
                });
            }
            if edge.observed_at != self.observed_at {
                return Err(WatershedConnectivityError::ObservationTimeMismatch {
                    expected: self.observed_at,
                    actual: edge.observed_at,
                });
            }
        }

        for pair in self.edges.windows(2) {
            let left = (&pair[0].upstream_scope, &pair[0].downstream_scope);
            let right = (&pair[1].upstream_scope, &pair[1].downstream_scope);
            if left == right {
                return Err(WatershedConnectivityError::DuplicateEdge {
                    upstream: pair[1].upstream_scope.clone(),
                    downstream: pair[1].downstream_scope.clone(),
                });
            }
            if left > right {
                return Err(WatershedConnectivityError::NonCanonicalEdgeOrder);
            }
        }

        let mut nodes = BTreeSet::new();
        let mut adjacency: BTreeMap<ScopeId, Vec<ScopeId>> = BTreeMap::new();
        let mut indegree: BTreeMap<ScopeId, usize> = BTreeMap::new();
        for edge in &self.edges {
            nodes.insert(edge.upstream_scope.clone());
            nodes.insert(edge.downstream_scope.clone());
            adjacency
                .entry(edge.upstream_scope.clone())
                .or_default()
                .push(edge.downstream_scope.clone());
            indegree.entry(edge.upstream_scope.clone()).or_insert(0);
            *indegree.entry(edge.downstream_scope.clone()).or_insert(0) += 1;
        }
        for children in adjacency.values_mut() {
            children.sort();
        }

        let mut ready: BTreeSet<ScopeId> = indegree
            .iter()
            .filter_map(|(scope, degree)| (*degree == 0).then_some(scope.clone()))
            .collect();
        let mut visited = 0_usize;
        while let Some(scope) = ready.pop_first() {
            visited += 1;
            if let Some(children) = adjacency.get(&scope) {
                for child in children {
                    let degree = indegree
                        .get_mut(child)
                        .expect("all adjacency targets were inserted into indegree");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }
        if visited != nodes.len() {
            return Err(WatershedConnectivityError::CycleDetected);
        }
        Ok(())
    }

    /// Stable identity independent of edge arrival order passed to `new`.
    pub fn digest(&self) -> Result<TypedDigest32, WatershedConnectivityError> {
        self.validate()?;
        let mut hasher = Sha256::new();
        hasher.update(b"symtropy.watershed.topology.v1\0");
        hasher.update(self.schema_version.to_le_bytes());
        hash_string(&mut hasher, self.hydrology_authority.as_str());
        hash_string(&mut hasher, self.reference_frame.as_str());
        hasher.update(self.observed_at.seconds_from_genesis.to_le_bytes());
        hasher.update(self.observed_at.nanos.to_le_bytes());
        hash_u64(
            &mut hasher,
            u64::try_from(self.edges.len())
                .map_err(|_| WatershedConnectivityError::LengthOverflow("edges"))?,
        );
        for edge in &self.edges {
            hash_string(&mut hasher, edge.upstream_scope.as_str());
            hash_string(&mut hasher, edge.downstream_scope.as_str());
            hash_typed_digest(&mut hasher, &edge.relation_digest);
        }
        TypedDigest32::new(
            WATERSHED_TOPOLOGY_DIGEST_DOMAIN,
            DigestAlgorithm::Sha256,
            WATERSHED_TOPOLOGY_SCHEMA_VERSION,
            hasher.finalize().into(),
        )
        .map_err(WatershedConnectivityError::Contract)
    }

    /// Potential downstream causal relevance only.
    ///
    /// `minimum_hops` is graph-theoretic. It is not travel time, distance,
    /// attenuation, discharge, probability, or any physical transition value.
    pub fn downstream_reachability(
        &self,
        source: &ScopeId,
    ) -> Result<Vec<DownstreamCausalScope>, WatershedConnectivityError> {
        self.validate()?;
        let mut adjacency: BTreeMap<ScopeId, Vec<ScopeId>> = BTreeMap::new();
        let mut known = BTreeSet::new();
        for edge in &self.edges {
            known.insert(edge.upstream_scope.clone());
            known.insert(edge.downstream_scope.clone());
            adjacency
                .entry(edge.upstream_scope.clone())
                .or_default()
                .push(edge.downstream_scope.clone());
        }
        if !known.contains(source) {
            return Err(WatershedConnectivityError::UnknownSource(source.clone()));
        }
        for children in adjacency.values_mut() {
            children.sort();
        }

        let mut queue = VecDeque::new();
        let mut minimum_hops: BTreeMap<ScopeId, u32> = BTreeMap::new();
        queue.push_back((source.clone(), 0_u32));
        while let Some((scope, hops)) = queue.pop_front() {
            if let Some(children) = adjacency.get(&scope) {
                for child in children {
                    let child_hops = hops
                        .checked_add(1)
                        .ok_or(WatershedConnectivityError::HopOverflow)?;
                    let should_visit = minimum_hops
                        .get(child)
                        .is_none_or(|existing| child_hops < *existing);
                    if should_visit {
                        minimum_hops.insert(child.clone(), child_hops);
                        queue.push_back((child.clone(), child_hops));
                    }
                }
            }
        }

        let mut result: Vec<_> = minimum_hops
            .into_iter()
            .map(|(scope, minimum_hops)| DownstreamCausalScope {
                scope,
                minimum_hops,
            })
            .collect();
        result.sort_by(|left, right| {
            left.minimum_hops
                .cmp(&right.minimum_hops)
                .then_with(|| left.scope.cmp(&right.scope))
        });
        Ok(result)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownstreamCausalScope {
    pub scope: ScopeId,
    /// Graph hops only; no physical hydrology quantity is encoded here.
    pub minimum_hops: u32,
}

fn hash_string(hasher: &mut Sha256, value: &str) {
    hash_u64(hasher, value.len() as u64);
    hasher.update(value.as_bytes());
}

fn hash_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_le_bytes());
}

fn hash_typed_digest(hasher: &mut Sha256, digest: &TypedDigest32) {
    hash_string(hasher, &digest.domain);
    match &digest.algorithm {
        DigestAlgorithm::Sha256 => hasher.update([0]),
        DigestAlgorithm::Other(name) => {
            hasher.update([255]);
            hash_string(hasher, name);
        }
    }
    hasher.update(digest.schema_version.to_le_bytes());
    hasher.update(digest.value);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatershedConnectivityError {
    Contract(ContractError),
    UnsupportedSchema { expected: u32, actual: u32 },
    NoEdges,
    SelfEdge(ScopeId),
    InvalidRelationDigestDomain(String),
    AuthorityMismatch {
        expected: AuthorityId,
        actual: AuthorityId,
    },
    ReferenceFrameMismatch {
        expected: ReferenceFrameId,
        actual: ReferenceFrameId,
    },
    ObservationTimeMismatch {
        expected: SimInstant,
        actual: SimInstant,
    },
    DuplicateEdge {
        upstream: ScopeId,
        downstream: ScopeId,
    },
    NonCanonicalEdgeOrder,
    CycleDetected,
    UnknownSource(ScopeId),
    HopOverflow,
    LengthOverflow(&'static str),
}

impl fmt::Display for WatershedConnectivityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contract(error) => write!(formatter, "watershed connectivity contract error: {error}"),
            Self::UnsupportedSchema { expected, actual } => write!(
                formatter,
                "unsupported watershed topology schema {actual}; expected {expected}"
            ),
            Self::NoEdges => write!(formatter, "watershed topology must contain at least one edge"),
            Self::SelfEdge(scope) => write!(formatter, "watershed topology contains self-edge at {scope}"),
            Self::InvalidRelationDigestDomain(domain) => write!(
                formatter,
                "watershed relation digest domain {domain:?} must start with {WATERSHED_RELATION_DOMAIN_PREFIX:?}"
            ),
            Self::AuthorityMismatch { expected, actual } => write!(
                formatter,
                "watershed edge authority {actual} does not match snapshot authority {expected}"
            ),
            Self::ReferenceFrameMismatch { expected, actual } => write!(
                formatter,
                "watershed edge frame {actual} does not match snapshot frame {expected}"
            ),
            Self::ObservationTimeMismatch { expected, actual } => write!(
                formatter,
                "watershed edge time {actual:?} does not match snapshot time {expected:?}"
            ),
            Self::DuplicateEdge { upstream, downstream } => {
                write!(formatter, "duplicate watershed edge {upstream} -> {downstream}")
            }
            Self::NonCanonicalEdgeOrder => write!(formatter, "watershed edges are not in canonical order"),
            Self::CycleDetected => write!(
                formatter,
                "v1 one-way watershed topology contains a cycle; use a future reversible topology contract for tidal/canal-loop hydraulics"
            ),
            Self::UnknownSource(scope) => write!(formatter, "source scope {scope} is not present in watershed topology"),
            Self::HopOverflow => write!(formatter, "watershed causal hop count overflow"),
            Self::LengthOverflow(kind) => write!(formatter, "{kind} length does not fit canonical u64 encoding"),
        }
    }
}

impl Error for WatershedConnectivityError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn authority() -> AuthorityId {
        AuthorityId::parse("hydrology.authority.v1").unwrap()
    }

    fn frame() -> ReferenceFrameId {
        ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap()
    }

    fn at() -> SimInstant {
        SimInstant::new(2_000, 0).unwrap()
    }

    fn scope(name: &str) -> ScopeId {
        ScopeId::parse(format!("body-cell:sol:earth/r7/{name}")).unwrap()
    }

    fn edge(upstream: &str, downstream: &str) -> WatershedConnectionEvidence {
        WatershedConnectionEvidence::new(
            authority(),
            scope(upstream),
            scope(downstream),
            frame(),
            at(),
            TypedDigest32::sha256(
                "symtropy.hydrology.watershed-connectivity.edge.v1",
                1,
                format!("{upstream}->{downstream}").as_bytes(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn topology(edges: Vec<WatershedConnectionEvidence>) -> WatershedTopologySnapshot {
        WatershedTopologySnapshot::new(authority(), frame(), at(), edges).unwrap()
    }

    #[test]
    fn arrival_order_does_not_change_topology_identity() {
        let forward = topology(vec![edge("a", "b"), edge("b", "c")]);
        let reverse = topology(vec![edge("b", "c"), edge("a", "b")]);
        assert_eq!(forward, reverse);
        assert_eq!(forward.digest().unwrap(), reverse.digest().unwrap());
    }

    #[test]
    fn three_cell_chain_has_deterministic_minimum_hops() {
        let topology = topology(vec![edge("a", "b"), edge("b", "c")]);
        assert_eq!(
            topology.downstream_reachability(&scope("a")).unwrap(),
            vec![
                DownstreamCausalScope {
                    scope: scope("b"),
                    minimum_hops: 1,
                },
                DownstreamCausalScope {
                    scope: scope("c"),
                    minimum_hops: 2,
                },
            ]
        );
    }

    #[test]
    fn converging_paths_use_minimum_graph_hops() {
        let topology = topology(vec![
            edge("a", "b"),
            edge("a", "c"),
            edge("b", "d"),
            edge("c", "d"),
        ]);
        let reach = topology.downstream_reachability(&scope("a")).unwrap();
        let d = reach.iter().find(|item| item.scope == scope("d")).unwrap();
        assert_eq!(d.minimum_hops, 2);
    }

    #[test]
    fn cycles_are_rejected_in_one_way_v1_topology() {
        assert!(matches!(
            WatershedTopologySnapshot::new(
                authority(),
                frame(),
                at(),
                vec![edge("a", "b"), edge("b", "a")],
            ),
            Err(WatershedConnectivityError::CycleDetected)
        ));
    }

    #[test]
    fn duplicate_directed_edge_is_rejected() {
        assert!(matches!(
            WatershedTopologySnapshot::new(
                authority(),
                frame(),
                at(),
                vec![edge("a", "b"), edge("a", "b")],
            ),
            Err(WatershedConnectivityError::DuplicateEdge { .. })
        ));
    }

    #[test]
    fn topology_is_connectivity_evidence_not_hydrology_state() {
        let topology = topology(vec![edge("a", "b")]);
        let reach = topology.downstream_reachability(&scope("a")).unwrap();
        assert_eq!(reach[0].scope, scope("b"));
        assert_eq!(reach[0].minimum_hops, 1);
    }

    #[test]
    fn relation_digest_domain_is_hydrology_namespaced() {
        let result = WatershedConnectionEvidence::new(
            authority(),
            scope("a"),
            scope("b"),
            frame(),
            at(),
            TypedDigest32::sha256("generic.edge.v1", 1, b"a-b").unwrap(),
        );
        assert!(matches!(
            result,
            Err(WatershedConnectivityError::InvalidRelationDigestDomain(_))
        ));
    }
}
