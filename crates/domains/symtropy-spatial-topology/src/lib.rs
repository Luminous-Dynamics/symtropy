// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! PB-04a: dependency-light, read-only spatial topology projections.
//!
//! This crate deliberately begins *after* geometric decomposition. A boundary
//! provider supplies exact region/interface snapshots; this crate validates that
//! snapshot and derives independent typed topology facets. It does not own
//! geometry, door/device state, navigation, atmosphere, acoustics, heat,
//! visibility, weather, privacy, place identity, or physical mutation.

use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, error::Error, fmt};
use symtropy_game_state::StableId;

pub const SPATIAL_TOPOLOGY_SCHEMA_VERSION: u32 = 1;
pub const MAX_REGIONS: usize = 65_536;
pub const MAX_INTERFACES: usize = 262_144;
pub const MAX_SOURCE_REFS: usize = 4_096;
pub const MAX_FACETS_PER_INTERFACE: usize = 32;
pub const MAX_DIGEST_BYTES: usize = 256;
/// Aggregate exact-source refs admitted by one boundary snapshot. Large worlds
/// compose bounded snapshots rather than allowing nested per-region/interface
/// limits to multiply into an unbounded validation allocation.
pub const MAX_BOUNDARY_EXACT_REFS: usize = 262_144;

const BOUNDARY_DIGEST_DOMAIN: &[u8] = b"symtropy.spatial-topology.boundary.v1\0";
const PROFILE_DIGEST_DOMAIN: &[u8] = b"symtropy.spatial-topology.profile.v1\0";

/// Exact content-bearing reference to a fact owned by another authority.
///
/// Presence here never makes the referenced subject authoritative. A real
/// adapter must resolve/revalidate this ref against its owner before claiming
/// the source is current.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExactSourceRef {
    pub authority_id: StableId,
    pub subject_id: StableId,
    pub revision: u64,
    pub digest: String,
}

impl ExactSourceRef {
    pub fn new(
        authority_id: StableId,
        subject_id: StableId,
        revision: u64,
        digest: impl Into<String>,
    ) -> Result<Self, TopologyError> {
        let value = Self {
            authority_id,
            subject_id,
            revision,
            digest: digest.into(),
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), TopologyError> {
        validate_id(&self.authority_id)?;
        validate_id(&self.subject_id)?;
        validate_digest(&self.digest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpatialRegionId(pub StableId);

impl SpatialRegionId {
    pub fn new(id: StableId) -> Result<Self, TopologyError> {
        validate_id(&id)?;
        Ok(Self(id))
    }

    fn validate(&self) -> Result<(), TopologyError> {
        validate_id(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundaryInterfaceId(pub StableId);

impl BoundaryInterfaceId {
    pub fn new(id: StableId) -> Result<Self, TopologyError> {
        validate_id(&id)?;
        Ok(Self(id))
    }

    fn validate(&self) -> Result<(), TopologyError> {
        validate_id(&self.0)
    }
}

/// A geometric cell/region supplied by an external decomposition authority.
/// Region identity is projection identity, not social/place identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatialRegionSnapshot {
    id: SpatialRegionId,
    source_refs: Vec<ExactSourceRef>,
}

impl SpatialRegionSnapshot {
    pub fn new(
        id: SpatialRegionId,
        mut source_refs: Vec<ExactSourceRef>,
    ) -> Result<Self, TopologyError> {
        id.validate()?;
        validate_len("region.source_refs", source_refs.len(), MAX_SOURCE_REFS)?;
        source_refs.sort();
        validate_exact_refs("region.source_refs", &source_refs)?;
        Ok(Self { id, source_refs })
    }

    pub fn id(&self) -> &SpatialRegionId {
        &self.id
    }

    pub fn source_refs(&self) -> &[ExactSourceRef] {
        &self.source_refs
    }

    fn validate_canonical(&self) -> Result<(), TopologyError> {
        self.id.validate()?;
        validate_len(
            "region.source_refs",
            self.source_refs.len(),
            MAX_SOURCE_REFS,
        )?;
        validate_exact_refs("region.source_refs", &self.source_refs)
    }
}

/// Independent semantic question asked of one boundary interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TopologyFacet {
    Occupancy,
    AirPressure,
    Acoustic,
    Visibility,
    Thermal,
    WeatherExposure,
}

/// What the exact boundary snapshot says for one facet.
///
/// `Unspecified` is deliberately distinct from `Disconnected`. Missing evidence
/// must never become an assertion that a pressure boundary seals, a wall is
/// opaque, or a path is blocked. Consumers may treat `Unspecified` as
/// fail-closed for their own operation, but they must retain that epistemic
/// distinction.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FacetRelation {
    Unspecified,
    Disconnected,
    Connected,
    /// Opaque relation class supplied by an owning boundary/physics projection.
    /// This is not a numerical permeability, dB attenuation, conductance, etc.
    QualifiedClass {
        class_id: StableId,
    },
}

impl FacetRelation {
    fn validate(&self) -> Result<(), TopologyError> {
        match self {
            Self::Unspecified | Self::Disconnected | Self::Connected => Ok(()),
            Self::QualifiedClass { class_id } => validate_id(class_id),
        }
    }

    /// True only for unconditional connectivity asserted by the owning projection.
    pub const fn is_definitely_connected(&self) -> bool {
        matches!(self, Self::Connected)
    }

    /// True when connectivity semantics require interpretation of an opaque class.
    pub const fn is_qualified(&self) -> bool {
        matches!(self, Self::QualifiedClass { .. })
    }

    const fn participates_in_candidate_graph(&self) -> bool {
        matches!(self, Self::Connected | Self::QualifiedClass { .. })
    }

    pub const fn is_known_disconnected(&self) -> bool {
        matches!(self, Self::Disconnected)
    }

    pub const fn is_unspecified(&self) -> bool {
        matches!(self, Self::Unspecified)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceFacetState {
    pub facet: TopologyFacet,
    pub relation: FacetRelation,
}

impl InterfaceFacetState {
    pub fn new(facet: TopologyFacet, relation: FacetRelation) -> Result<Self, TopologyError> {
        if relation.is_unspecified() {
            return Err(TopologyError::ExplicitUnspecifiedFacet(facet));
        }
        relation.validate()?;
        Ok(Self { facet, relation })
    }
}

/// One interface between exactly two externally decomposed regions.
///
/// Facets are independent. There is deliberately no global `is_open` bit from
/// which body, air, sound, light, heat, and weather semantics are guessed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryInterfaceSnapshot {
    id: BoundaryInterfaceId,
    first_region: SpatialRegionId,
    second_region: SpatialRegionId,
    facet_states: Vec<InterfaceFacetState>,
    source_refs: Vec<ExactSourceRef>,
}

impl BoundaryInterfaceSnapshot {
    pub fn new(
        id: BoundaryInterfaceId,
        mut first_region: SpatialRegionId,
        mut second_region: SpatialRegionId,
        mut facet_states: Vec<InterfaceFacetState>,
        mut source_refs: Vec<ExactSourceRef>,
    ) -> Result<Self, TopologyError> {
        id.validate()?;
        first_region.validate()?;
        second_region.validate()?;
        if second_region < first_region {
            std::mem::swap(&mut first_region, &mut second_region);
        }
        validate_len(
            "interface.facet_states",
            facet_states.len(),
            MAX_FACETS_PER_INTERFACE,
        )?;
        validate_len("interface.source_refs", source_refs.len(), MAX_SOURCE_REFS)?;
        facet_states.sort_by_key(|state| state.facet);
        source_refs.sort();
        let value = Self {
            id,
            first_region,
            second_region,
            facet_states,
            source_refs,
        };
        value.validate_canonical()?;
        Ok(value)
    }

    pub fn id(&self) -> &BoundaryInterfaceId {
        &self.id
    }

    pub fn first_region(&self) -> &SpatialRegionId {
        &self.first_region
    }

    pub fn second_region(&self) -> &SpatialRegionId {
        &self.second_region
    }

    pub fn facet_states(&self) -> &[InterfaceFacetState] {
        &self.facet_states
    }

    pub fn source_refs(&self) -> &[ExactSourceRef] {
        &self.source_refs
    }

    /// Missing facet data is `Unspecified`, never silently `Disconnected`.
    pub fn relation(&self, facet: TopologyFacet) -> FacetRelation {
        self.facet_states
            .binary_search_by_key(&facet, |state| state.facet)
            .ok()
            .map(|index| self.facet_states[index].relation.clone())
            .unwrap_or(FacetRelation::Unspecified)
    }

    fn validate_canonical(&self) -> Result<(), TopologyError> {
        self.id.validate()?;
        self.first_region.validate()?;
        self.second_region.validate()?;
        if self.first_region == self.second_region {
            return Err(TopologyError::SelfInterface(self.id.clone()));
        }
        validate_len(
            "interface.facet_states",
            self.facet_states.len(),
            MAX_FACETS_PER_INTERFACE,
        )?;
        for state in &self.facet_states {
            if state.relation.is_unspecified() {
                return Err(TopologyError::ExplicitUnspecifiedFacet(state.facet));
            }
            state.relation.validate()?;
        }
        for pair in self.facet_states.windows(2) {
            if pair[0].facet > pair[1].facet {
                return Err(TopologyError::NonCanonicalOrder("interface.facet_states"));
            }
            if pair[0].facet == pair[1].facet {
                return Err(TopologyError::DuplicateFacet {
                    interface_id: self.id.clone(),
                    facet: pair[0].facet,
                });
            }
        }
        validate_len(
            "interface.source_refs",
            self.source_refs.len(),
            MAX_SOURCE_REFS,
        )?;
        validate_exact_refs("interface.source_refs", &self.source_refs)
    }
}

/// Exact read-only input snapshot from a geometric/boundary provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundarySnapshot {
    schema_version: u32,
    snapshot_id: StableId,
    revision: u64,
    provider_content_digest: String,
    content_digest: String,
    frame_ref: ExactSourceRef,
    environment_ref: ExactSourceRef,
    regions: Vec<SpatialRegionSnapshot>,
    interfaces: Vec<BoundaryInterfaceSnapshot>,
    source_refs: Vec<ExactSourceRef>,
}

impl BoundarySnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        snapshot_id: StableId,
        revision: u64,
        provider_content_digest: impl Into<String>,
        frame_ref: ExactSourceRef,
        environment_ref: ExactSourceRef,
        mut regions: Vec<SpatialRegionSnapshot>,
        mut interfaces: Vec<BoundaryInterfaceSnapshot>,
        mut source_refs: Vec<ExactSourceRef>,
    ) -> Result<Self, TopologyError> {
        validate_len("boundary.regions", regions.len(), MAX_REGIONS)?;
        validate_len("boundary.interfaces", interfaces.len(), MAX_INTERFACES)?;
        validate_len("boundary.source_refs", source_refs.len(), MAX_SOURCE_REFS)?;
        regions.sort_by(|left, right| left.id.cmp(&right.id));
        interfaces.sort_by(|left, right| left.id.cmp(&right.id));
        source_refs.sort();
        let provider_content_digest = provider_content_digest.into();
        validate_digest(&provider_content_digest)?;
        let mut value = Self {
            schema_version: SPATIAL_TOPOLOGY_SCHEMA_VERSION,
            snapshot_id,
            revision,
            provider_content_digest,
            content_digest: String::new(),
            frame_ref,
            environment_ref,
            regions,
            interfaces,
            source_refs,
        };
        value.content_digest = boundary_content_digest(&value);
        value.validate_canonical()?;
        Ok(value)
    }

    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn snapshot_id(&self) -> &StableId {
        &self.snapshot_id
    }

    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub fn provider_content_digest(&self) -> &str {
        &self.provider_content_digest
    }

    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    pub fn frame_ref(&self) -> &ExactSourceRef {
        &self.frame_ref
    }

    pub fn environment_ref(&self) -> &ExactSourceRef {
        &self.environment_ref
    }

    pub fn regions(&self) -> &[SpatialRegionSnapshot] {
        &self.regions
    }

    pub fn interfaces(&self) -> &[BoundaryInterfaceSnapshot] {
        &self.interfaces
    }

    pub fn source_refs(&self) -> &[ExactSourceRef] {
        &self.source_refs
    }

    pub fn exact_ref(&self) -> BoundarySnapshotRef {
        BoundarySnapshotRef {
            snapshot_id: self.snapshot_id.clone(),
            revision: self.revision,
            content_digest: self.content_digest.clone(),
        }
    }

    pub fn validate_canonical(&self) -> Result<(), TopologyError> {
        if self.schema_version != SPATIAL_TOPOLOGY_SCHEMA_VERSION {
            return Err(TopologyError::UnsupportedSchema(self.schema_version));
        }
        validate_id(&self.snapshot_id)?;
        validate_digest(&self.provider_content_digest)?;
        validate_digest(&self.content_digest)?;
        self.frame_ref.validate()?;
        self.environment_ref.validate()?;
        validate_len("boundary.regions", self.regions.len(), MAX_REGIONS)?;
        if self.regions.is_empty() {
            return Err(TopologyError::RegionsRequired);
        }
        validate_len("boundary.interfaces", self.interfaces.len(), MAX_INTERFACES)?;
        validate_len(
            "boundary.source_refs",
            self.source_refs.len(),
            MAX_SOURCE_REFS,
        )?;
        validate_exact_refs("boundary.source_refs", &self.source_refs)?;

        for region in &self.regions {
            region.validate_canonical()?;
        }
        for pair in self.regions.windows(2) {
            if pair[0].id > pair[1].id {
                return Err(TopologyError::NonCanonicalOrder("boundary.regions"));
            }
            if pair[0].id == pair[1].id {
                return Err(TopologyError::DuplicateRegion(pair[0].id.clone()));
            }
        }

        let region_ids: BTreeSet<_> = self
            .regions
            .iter()
            .map(|region| region.id.clone())
            .collect();
        for interface in &self.interfaces {
            interface.validate_canonical()?;
            if !region_ids.contains(&interface.first_region) {
                return Err(TopologyError::UnknownRegion {
                    interface_id: interface.id.clone(),
                    region_id: interface.first_region.clone(),
                });
            }
            if !region_ids.contains(&interface.second_region) {
                return Err(TopologyError::UnknownRegion {
                    interface_id: interface.id.clone(),
                    region_id: interface.second_region.clone(),
                });
            }
        }
        for pair in self.interfaces.windows(2) {
            if pair[0].id > pair[1].id {
                return Err(TopologyError::NonCanonicalOrder("boundary.interfaces"));
            }
            if pair[0].id == pair[1].id {
                return Err(TopologyError::DuplicateInterface(pair[0].id.clone()));
            }
        }
        validate_snapshot_exact_ref_consistency(self)?;

        let expected_digest = boundary_content_digest(self);
        if self.content_digest != expected_digest {
            return Err(TopologyError::DigestMismatch {
                subject: "boundary snapshot",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundarySnapshotRef {
    pub snapshot_id: StableId,
    pub revision: u64,
    pub content_digest: String,
}

impl BoundarySnapshotRef {
    pub fn validate(&self) -> Result<(), TopologyError> {
        validate_id(&self.snapshot_id)?;
        validate_digest(&self.content_digest)
    }
}

/// Deterministic facet-selection profile. Geometric decomposition itself is
/// outside PB-04a and requires its own exact profile in a later tranche.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyProfile {
    profile_id: StableId,
    revision: u64,
    content_digest: String,
    facets: Vec<TopologyFacet>,
}

impl TopologyProfile {
    pub fn new(
        profile_id: StableId,
        revision: u64,
        mut facets: Vec<TopologyFacet>,
    ) -> Result<Self, TopologyError> {
        validate_id(&profile_id)?;
        facets.sort();
        facets.dedup();
        if facets.is_empty() {
            return Err(TopologyError::ProfileFacetsRequired);
        }
        let content_digest = profile_content_digest(&profile_id, revision, &facets);
        Ok(Self {
            profile_id,
            revision,
            content_digest,
            facets,
        })
    }

    pub fn profile_id(&self) -> &StableId {
        &self.profile_id
    }

    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub fn content_digest(&self) -> &str {
        &self.content_digest
    }

    pub fn facets(&self) -> &[TopologyFacet] {
        &self.facets
    }

    pub fn exact_ref(&self) -> TopologyProfileRef {
        TopologyProfileRef {
            profile_id: self.profile_id.clone(),
            revision: self.revision,
            content_digest: self.content_digest.clone(),
        }
    }

    fn validate_canonical(&self) -> Result<(), TopologyError> {
        validate_id(&self.profile_id)?;
        validate_digest(&self.content_digest)?;
        if self.facets.is_empty() {
            return Err(TopologyError::ProfileFacetsRequired);
        }
        for pair in self.facets.windows(2) {
            if pair[0] >= pair[1] {
                return Err(TopologyError::NonCanonicalOrder("topology_profile.facets"));
            }
        }
        if self.content_digest
            != profile_content_digest(&self.profile_id, self.revision, &self.facets)
        {
            return Err(TopologyError::DigestMismatch {
                subject: "topology profile",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TopologyProfileRef {
    pub profile_id: StableId,
    pub revision: u64,
    pub content_digest: String,
}

impl TopologyProfileRef {
    pub fn validate(&self) -> Result<(), TopologyError> {
        validate_id(&self.profile_id)?;
        validate_digest(&self.content_digest)
    }
}

/// One interface's state in one facet projection, retaining source provenance.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TopologyInterfaceRelation {
    pub interface_id: BoundaryInterfaceId,
    pub first_region: SpatialRegionId,
    pub second_region: SpatialRegionId,
    pub relation: FacetRelation,
    source_refs: Vec<ExactSourceRef>,
}

impl TopologyInterfaceRelation {
    pub fn source_refs(&self) -> &[ExactSourceRef] {
        &self.source_refs
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetGraph {
    pub facet: TopologyFacet,
    interfaces: Vec<TopologyInterfaceRelation>,
}

impl FacetGraph {
    /// Includes connected, disconnected, and unspecified interfaces so a
    /// consumer can distinguish a known barrier from missing facet evidence.
    pub fn interfaces(&self) -> &[TopologyInterfaceRelation] {
        &self.interfaces
    }

    pub fn relation(&self, interface_id: &BoundaryInterfaceId) -> Option<&FacetRelation> {
        self.interfaces
            .binary_search_by(|value| value.interface_id.cmp(interface_id))
            .ok()
            .map(|index| &self.interfaces[index].relation)
    }

    /// Returns only neighbors for relations known to participate in this facet.
    /// `Unspecified` and known `Disconnected` interfaces never create a path.
    fn neighbors(&self, region: &SpatialRegionId) -> Vec<SpatialRegionId> {
        let mut result = BTreeSet::new();
        for interface in &self.interfaces {
            if !interface.relation.participates_in_candidate_graph() {
                continue;
            }
            if &interface.first_region == region {
                result.insert(interface.second_region.clone());
            } else if &interface.second_region == region {
                result.insert(interface.first_region.clone());
            }
        }
        result.into_iter().collect()
    }
}

/// Deterministic, read-only facet projection over one exact boundary snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologySnapshot {
    schema_version: u32,
    boundary_ref: BoundarySnapshotRef,
    profile_ref: TopologyProfileRef,
    region_ids: Vec<SpatialRegionId>,
    graphs: Vec<FacetGraph>,
}

impl TopologySnapshot {
    pub fn derive(
        boundary: &BoundarySnapshot,
        profile: &TopologyProfile,
    ) -> Result<Self, TopologyError> {
        boundary.validate_canonical()?;
        profile.validate_canonical()?;
        let region_ids = boundary
            .regions()
            .iter()
            .map(|region| region.id.clone())
            .collect::<Vec<_>>();

        let mut graphs = Vec::with_capacity(profile.facets().len());
        for &facet in profile.facets() {
            let mut interfaces = boundary
                .interfaces()
                .iter()
                .map(|interface| TopologyInterfaceRelation {
                    interface_id: interface.id.clone(),
                    first_region: interface.first_region.clone(),
                    second_region: interface.second_region.clone(),
                    relation: interface.relation(facet),
                    source_refs: interface.source_refs().to_vec(),
                })
                .collect::<Vec<_>>();
            interfaces.sort_by(|left, right| left.interface_id.cmp(&right.interface_id));
            graphs.push(FacetGraph { facet, interfaces });
        }

        Ok(Self {
            schema_version: SPATIAL_TOPOLOGY_SCHEMA_VERSION,
            boundary_ref: boundary.exact_ref(),
            profile_ref: profile.exact_ref(),
            region_ids,
            graphs,
        })
    }

    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn boundary_ref(&self) -> &BoundarySnapshotRef {
        &self.boundary_ref
    }

    pub fn profile_ref(&self) -> &TopologyProfileRef {
        &self.profile_ref
    }

    pub fn region_ids(&self) -> &[SpatialRegionId] {
        &self.region_ids
    }

    pub fn graphs(&self) -> &[FacetGraph] {
        &self.graphs
    }

    pub fn graph(&self, facet: TopologyFacet) -> Option<&FacetGraph> {
        self.graphs
            .binary_search_by_key(&facet, |graph| graph.facet)
            .ok()
            .map(|index| &self.graphs[index])
    }

    /// Checked universal adjacency query. Only an unqualified `Connected`
    /// relation is a definite edge. Opaque `QualifiedClass` relations require
    /// explicit downstream interpretation and therefore do not appear here.
    pub fn definite_neighbors(
        &self,
        facet: TopologyFacet,
        region: &SpatialRegionId,
    ) -> Result<Vec<SpatialRegionId>, TopologyError> {
        let graph = self.checked_graph(facet, region)?;
        let mut result = BTreeSet::new();
        for interface in graph.interfaces() {
            if !interface.relation.is_definitely_connected() {
                continue;
            }
            if &interface.first_region == region {
                result.insert(interface.second_region.clone());
            } else if &interface.second_region == region {
                result.insert(interface.first_region.clone());
            }
        }
        Ok(result.into_iter().collect())
    }

    /// Checked candidate adjacency query. Includes `QualifiedClass` relations
    /// only as candidate edges; callers must inspect the relation/class and
    /// apply their owning capability, policy, or solver semantics before using
    /// such an edge as traversable/connected truth.
    pub fn candidate_neighbors(
        &self,
        facet: TopologyFacet,
        region: &SpatialRegionId,
    ) -> Result<Vec<SpatialRegionId>, TopologyError> {
        let graph = self.checked_graph(facet, region)?;
        Ok(graph.neighbors(region))
    }

    fn checked_graph(
        &self,
        facet: TopologyFacet,
        region: &SpatialRegionId,
    ) -> Result<&FacetGraph, TopologyError> {
        if self.region_ids.binary_search(region).is_err() {
            return Err(TopologyError::UnknownProjectedRegion(region.clone()));
        }
        self.graph(facet)
            .ok_or(TopologyError::FacetNotProjected(facet))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyError {
    InvalidStableId(String),
    InvalidDigest,
    BoundExceeded {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    UnsupportedSchema(u32),
    DigestMismatch {
        subject: &'static str,
    },
    RegionsRequired,
    ProfileFacetsRequired,
    FacetNotProjected(TopologyFacet),
    UnknownProjectedRegion(SpatialRegionId),
    ExplicitUnspecifiedFacet(TopologyFacet),
    DuplicateRegion(SpatialRegionId),
    DuplicateInterface(BoundaryInterfaceId),
    DuplicateFacet {
        interface_id: BoundaryInterfaceId,
        facet: TopologyFacet,
    },
    DuplicateExactRef {
        field: &'static str,
        authority_id: StableId,
        subject_id: StableId,
        revision: u64,
    },
    ConflictingExactRef {
        field: &'static str,
        authority_id: StableId,
        subject_id: StableId,
        revision: u64,
    },
    NonCanonicalOrder(&'static str),
    SelfInterface(BoundaryInterfaceId),
    UnknownRegion {
        interface_id: BoundaryInterfaceId,
        region_id: SpatialRegionId,
    },
}

impl fmt::Display for TopologyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStableId(value) => {
                write!(formatter, "invalid stable identifier {value:?}")
            }
            Self::InvalidDigest => write!(formatter, "invalid exact-source digest"),
            Self::BoundExceeded {
                field,
                actual,
                maximum,
            } => {
                write!(
                    formatter,
                    "{field} has {actual} entries, maximum is {maximum}"
                )
            }
            Self::UnsupportedSchema(version) => {
                write!(formatter, "unsupported spatial topology schema {version}")
            }
            Self::DigestMismatch { subject } => {
                write!(formatter, "{subject} canonical content digest mismatch")
            }
            Self::RegionsRequired => {
                write!(formatter, "boundary snapshot requires at least one region")
            }
            Self::ProfileFacetsRequired => {
                write!(formatter, "topology profile requires at least one facet")
            }
            Self::FacetNotProjected(facet) => {
                write!(
                    formatter,
                    "topology facet {facet:?} was not projected by this profile"
                )
            }
            Self::UnknownProjectedRegion(region) => {
                write!(formatter, "unknown projected spatial region {}", region.0)
            }
            Self::ExplicitUnspecifiedFacet(facet) => write!(
                formatter,
                "facet {facet:?} cannot be explicitly stored as unspecified; omit it instead"
            ),
            Self::DuplicateRegion(id) => write!(formatter, "duplicate spatial region {}", id.0),
            Self::DuplicateInterface(id) => {
                write!(formatter, "duplicate boundary interface {}", id.0)
            }
            Self::DuplicateFacet {
                interface_id,
                facet,
            } => write!(
                formatter,
                "boundary interface {} repeats facet {facet:?}",
                interface_id.0
            ),
            Self::DuplicateExactRef {
                field,
                authority_id,
                subject_id,
                revision,
            } => write!(
                formatter,
                "{field} repeats exact ref {authority_id}/{subject_id}@{revision}"
            ),
            Self::ConflictingExactRef {
                field,
                authority_id,
                subject_id,
                revision,
            } => write!(
                formatter,
                "{field} contains competing digests for {authority_id}/{subject_id}@{revision}"
            ),
            Self::NonCanonicalOrder(field) => {
                write!(formatter, "{field} is not canonically ordered")
            }
            Self::SelfInterface(id) => write!(
                formatter,
                "boundary interface {} connects a region to itself",
                id.0
            ),
            Self::UnknownRegion {
                interface_id,
                region_id,
            } => write!(
                formatter,
                "boundary interface {} references unknown region {}",
                interface_id.0, region_id.0
            ),
        }
    }
}

impl Error for TopologyError {}

fn validate_snapshot_exact_ref_consistency(
    snapshot: &BoundarySnapshot,
) -> Result<(), TopologyError> {
    let field = "boundary.all_exact_refs";
    let mut total = 2usize;

    let mut add = |count: usize| -> Result<(), TopologyError> {
        total = total
            .checked_add(count)
            .ok_or(TopologyError::BoundExceeded {
                field,
                actual: usize::MAX,
                maximum: MAX_BOUNDARY_EXACT_REFS,
            })?;
        if total > MAX_BOUNDARY_EXACT_REFS {
            return Err(TopologyError::BoundExceeded {
                field,
                actual: total,
                maximum: MAX_BOUNDARY_EXACT_REFS,
            });
        }
        Ok(())
    };

    add(snapshot.source_refs.len())?;
    for region in &snapshot.regions {
        add(region.source_refs.len())?;
    }
    for interface in &snapshot.interfaces {
        add(interface.source_refs.len())?;
    }

    let mut refs = Vec::with_capacity(total);
    refs.push(&snapshot.frame_ref);
    refs.push(&snapshot.environment_ref);
    refs.extend(snapshot.source_refs.iter());
    for region in &snapshot.regions {
        refs.extend(region.source_refs.iter());
    }
    for interface in &snapshot.interfaces {
        refs.extend(interface.source_refs.iter());
    }

    refs.sort_by(|left, right| {
        (
            &left.authority_id,
            &left.subject_id,
            left.revision,
            &left.digest,
        )
            .cmp(&(
                &right.authority_id,
                &right.subject_id,
                right.revision,
                &right.digest,
            ))
    });

    for pair in refs.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let same_identity = left.authority_id == right.authority_id
            && left.subject_id == right.subject_id
            && left.revision == right.revision;
        if same_identity && left.digest != right.digest {
            return Err(TopologyError::ConflictingExactRef {
                field,
                authority_id: left.authority_id.clone(),
                subject_id: left.subject_id.clone(),
                revision: left.revision,
            });
        }
    }
    Ok(())
}

fn boundary_content_digest(snapshot: &BoundarySnapshot) -> String {
    let mut hasher = Sha256::new();
    hasher.update(BOUNDARY_DIGEST_DOMAIN);
    hash_u32(&mut hasher, snapshot.schema_version);
    hash_id(&mut hasher, &snapshot.snapshot_id);
    hash_u64(&mut hasher, snapshot.revision);
    hash_text(&mut hasher, &snapshot.provider_content_digest);
    hash_exact_source(&mut hasher, &snapshot.frame_ref);
    hash_exact_source(&mut hasher, &snapshot.environment_ref);

    hash_u64(&mut hasher, snapshot.regions.len() as u64);
    for region in &snapshot.regions {
        hash_id(&mut hasher, &region.id.0);
        hash_exact_sources(&mut hasher, &region.source_refs);
    }

    hash_u64(&mut hasher, snapshot.interfaces.len() as u64);
    for interface in &snapshot.interfaces {
        hash_id(&mut hasher, &interface.id.0);
        hash_id(&mut hasher, &interface.first_region.0);
        hash_id(&mut hasher, &interface.second_region.0);
        hash_u64(&mut hasher, interface.facet_states.len() as u64);
        for state in &interface.facet_states {
            hash_facet(&mut hasher, state.facet);
            hash_relation(&mut hasher, &state.relation);
        }
        hash_exact_sources(&mut hasher, &interface.source_refs);
    }

    hash_exact_sources(&mut hasher, &snapshot.source_refs);
    hex_digest(&hasher.finalize())
}

fn profile_content_digest(
    profile_id: &StableId,
    revision: u64,
    facets: &[TopologyFacet],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(PROFILE_DIGEST_DOMAIN);
    hash_id(&mut hasher, profile_id);
    hash_u64(&mut hasher, revision);
    hash_u64(&mut hasher, facets.len() as u64);
    for &facet in facets {
        hash_facet(&mut hasher, facet);
    }
    hex_digest(&hasher.finalize())
}

fn hash_exact_sources(hasher: &mut Sha256, refs: &[ExactSourceRef]) {
    hash_u64(hasher, refs.len() as u64);
    for reference in refs {
        hash_exact_source(hasher, reference);
    }
}

fn hash_exact_source(hasher: &mut Sha256, reference: &ExactSourceRef) {
    hash_id(hasher, &reference.authority_id);
    hash_id(hasher, &reference.subject_id);
    hash_u64(hasher, reference.revision);
    hash_text(hasher, &reference.digest);
}

fn hash_relation(hasher: &mut Sha256, relation: &FacetRelation) {
    match relation {
        FacetRelation::Unspecified => hasher.update([0]),
        FacetRelation::Disconnected => hasher.update([1]),
        FacetRelation::Connected => hasher.update([2]),
        FacetRelation::QualifiedClass { class_id } => {
            hasher.update([3]);
            hash_id(hasher, class_id);
        }
    }
}

fn hash_facet(hasher: &mut Sha256, facet: TopologyFacet) {
    let tag = match facet {
        TopologyFacet::Occupancy => 0,
        TopologyFacet::AirPressure => 1,
        TopologyFacet::Acoustic => 2,
        TopologyFacet::Visibility => 3,
        TopologyFacet::Thermal => 4,
        TopologyFacet::WeatherExposure => 5,
    };
    hasher.update([tag]);
}

fn hash_id(hasher: &mut Sha256, value: &StableId) {
    hash_text(hasher, value.as_str());
}

fn hash_text(hasher: &mut Sha256, value: &str) {
    hash_u64(hasher, value.len() as u64);
    hasher.update(value.as_bytes());
}

fn hash_u32(hasher: &mut Sha256, value: u32) {
    hasher.update(value.to_le_bytes());
}

fn hash_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_le_bytes());
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

fn validate_id(id: &StableId) -> Result<(), TopologyError> {
    StableId::parse(id.as_str())
        .map(|_| ())
        .map_err(|_| TopologyError::InvalidStableId(id.as_str().to_owned()))
}

fn validate_digest(value: &str) -> Result<(), TopologyError> {
    if value.is_empty()
        || value.len() > MAX_DIGEST_BYTES
        || !value.bytes().all(|byte| byte.is_ascii_graphic())
    {
        Err(TopologyError::InvalidDigest)
    } else {
        Ok(())
    }
}

fn validate_len(field: &'static str, actual: usize, maximum: usize) -> Result<(), TopologyError> {
    if actual > maximum {
        Err(TopologyError::BoundExceeded {
            field,
            actual,
            maximum,
        })
    } else {
        Ok(())
    }
}

fn validate_exact_refs(field: &'static str, refs: &[ExactSourceRef]) -> Result<(), TopologyError> {
    for reference in refs {
        reference.validate()?;
    }
    for pair in refs.windows(2) {
        let left_key = (&pair[0].authority_id, &pair[0].subject_id, pair[0].revision);
        let right_key = (&pair[1].authority_id, &pair[1].subject_id, pair[1].revision);
        if left_key > right_key {
            return Err(TopologyError::NonCanonicalOrder(field));
        }
        if left_key == right_key {
            if pair[0].digest == pair[1].digest {
                return Err(TopologyError::DuplicateExactRef {
                    field,
                    authority_id: pair[0].authority_id.clone(),
                    subject_id: pair[0].subject_id.clone(),
                    revision: pair[0].revision,
                });
            }
            return Err(TopologyError::ConflictingExactRef {
                field,
                authority_id: pair[0].authority_id.clone(),
                subject_id: pair[0].subject_id.clone(),
                revision: pair[0].revision,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn region(value: &str) -> SpatialRegionId {
        SpatialRegionId::new(id(value)).unwrap()
    }

    fn interface_id(value: &str) -> BoundaryInterfaceId {
        BoundaryInterfaceId::new(id(value)).unwrap()
    }

    fn source(subject: &str, revision: u64, digest: &str) -> ExactSourceRef {
        ExactSourceRef::new(
            id("authority:synthetic-boundary"),
            id(subject),
            revision,
            digest,
        )
        .unwrap()
    }

    fn class(value: &str) -> FacetRelation {
        FacetRelation::QualifiedClass {
            class_id: id(value),
        }
    }

    fn facets(values: &[(TopologyFacet, FacetRelation)]) -> Vec<InterfaceFacetState> {
        values
            .iter()
            .map(|(facet, relation)| InterfaceFacetState::new(*facet, relation.clone()).unwrap())
            .collect()
    }

    fn base_regions() -> Vec<SpatialRegionSnapshot> {
        vec![
            SpatialRegionSnapshot::new(
                region("region:inside"),
                vec![source("geom:inside", 1, "aa")],
            )
            .unwrap(),
            SpatialRegionSnapshot::new(
                region("region:outside"),
                vec![source("geom:outside", 1, "bb")],
            )
            .unwrap(),
        ]
    }

    fn snapshot(interfaces: Vec<BoundaryInterfaceSnapshot>) -> BoundarySnapshot {
        BoundarySnapshot::new(
            id("boundary:shelter"),
            7,
            "boundary-digest-v7",
            source("frame:shelter", 2, "frame-digest"),
            source("environment:earth-air", 4, "env-digest"),
            base_regions(),
            interfaces,
            vec![source("geometry:shelter", 7, "geometry-digest")],
        )
        .unwrap()
    }

    fn full_profile() -> TopologyProfile {
        TopologyProfile::new(
            id("topology-profile:pb04a-synthetic-v1"),
            1,
            vec![
                TopologyFacet::WeatherExposure,
                TopologyFacet::Visibility,
                TopologyFacet::Thermal,
                TopologyFacet::Occupancy,
                TopologyFacet::AirPressure,
                TopologyFacet::Acoustic,
            ],
        )
        .unwrap()
    }

    #[test]
    fn insertion_order_does_not_change_boundary_or_projection() {
        let door = BoundaryInterfaceSnapshot::new(
            interface_id("interface:door"),
            region("region:inside"),
            region("region:outside"),
            facets(&[
                (TopologyFacet::Occupancy, FacetRelation::Disconnected),
                (TopologyFacet::Acoustic, class("transmission:door-closed")),
            ]),
            vec![source("device:door", 3, "door-closed")],
        )
        .unwrap();
        let vent = BoundaryInterfaceSnapshot::new(
            interface_id("interface:vent"),
            region("region:inside"),
            region("region:outside"),
            facets(&[
                (TopologyFacet::Occupancy, FacetRelation::Disconnected),
                (TopologyFacet::AirPressure, FacetRelation::Connected),
            ]),
            vec![source("device:vent", 1, "vent-open")],
        )
        .unwrap();

        let left = snapshot(vec![door.clone(), vent.clone()]);
        let right = snapshot(vec![vent, door]);
        assert_eq!(left, right);
        assert_eq!(
            TopologySnapshot::derive(&left, &full_profile()).unwrap(),
            TopologySnapshot::derive(&right, &full_profile()).unwrap()
        );
    }

    #[test]
    fn missing_facet_is_unknown_not_a_false_blocking_claim() {
        let interface = BoundaryInterfaceSnapshot::new(
            interface_id("interface:unknown-pressure"),
            region("region:inside"),
            region("region:outside"),
            facets(&[(TopologyFacet::Visibility, FacetRelation::Connected)]),
            vec![source("boundary:unknown-pressure", 1, "digest")],
        )
        .unwrap();
        assert_eq!(
            interface.relation(TopologyFacet::AirPressure),
            FacetRelation::Unspecified
        );

        let topology =
            TopologySnapshot::derive(&snapshot(vec![interface]), &full_profile()).unwrap();
        let graph = topology.graph(TopologyFacet::AirPressure).unwrap();
        let relation = graph
            .relation(&interface_id("interface:unknown-pressure"))
            .unwrap();
        assert!(relation.is_unspecified());
        assert!(graph.neighbors(&region("region:inside")).is_empty());
    }

    #[test]
    fn closed_door_blocks_bodies_without_claiming_perfect_air_or_sound_isolation() {
        let door = BoundaryInterfaceSnapshot::new(
            interface_id("interface:door"),
            region("region:inside"),
            region("region:outside"),
            facets(&[
                (TopologyFacet::Occupancy, FacetRelation::Disconnected),
                (
                    TopologyFacet::AirPressure,
                    class("permeability:door-leakage"),
                ),
                (TopologyFacet::Acoustic, class("attenuation:door-closed")),
                (TopologyFacet::Visibility, FacetRelation::Disconnected),
            ]),
            vec![source("device:door", 3, "closed")],
        )
        .unwrap();
        let topology = TopologySnapshot::derive(&snapshot(vec![door]), &full_profile()).unwrap();

        let occupancy = topology.graph(TopologyFacet::Occupancy).unwrap();
        assert!(occupancy.neighbors(&region("region:inside")).is_empty());
        assert!(
            occupancy
                .relation(&interface_id("interface:door"))
                .unwrap()
                .is_known_disconnected()
        );
        assert_eq!(
            topology
                .graph(TopologyFacet::Acoustic)
                .unwrap()
                .neighbors(&region("region:inside")),
            vec![region("region:outside")]
        );
        assert_eq!(
            topology
                .graph(TopologyFacet::AirPressure)
                .unwrap()
                .neighbors(&region("region:inside")),
            vec![region("region:outside")]
        );
    }

    #[test]
    fn window_blocks_occupancy_but_preserves_visibility() {
        let window = BoundaryInterfaceSnapshot::new(
            interface_id("interface:window"),
            region("region:inside"),
            region("region:outside"),
            facets(&[
                (TopologyFacet::Occupancy, FacetRelation::Disconnected),
                (TopologyFacet::AirPressure, FacetRelation::Disconnected),
                (TopologyFacet::Visibility, FacetRelation::Connected),
                (TopologyFacet::Thermal, class("conductance:glazing")),
            ]),
            vec![source("device:window", 1, "closed-glazing")],
        )
        .unwrap();
        let topology = TopologySnapshot::derive(&snapshot(vec![window]), &full_profile()).unwrap();

        assert!(
            topology
                .graph(TopologyFacet::Occupancy)
                .unwrap()
                .neighbors(&region("region:inside"))
                .is_empty()
        );
        assert_eq!(
            topology
                .graph(TopologyFacet::Visibility)
                .unwrap()
                .neighbors(&region("region:inside")),
            vec![region("region:outside")]
        );
        assert_eq!(
            topology
                .graph(TopologyFacet::Thermal)
                .unwrap()
                .neighbors(&region("region:inside")),
            vec![region("region:outside")]
        );
    }

    #[test]
    fn vent_connects_air_without_body_passage() {
        let vent = BoundaryInterfaceSnapshot::new(
            interface_id("interface:vent"),
            region("region:inside"),
            region("region:outside"),
            facets(&[
                (TopologyFacet::Occupancy, FacetRelation::Disconnected),
                (TopologyFacet::AirPressure, FacetRelation::Connected),
                (TopologyFacet::Acoustic, class("attenuation:small-vent")),
            ]),
            vec![source("device:vent", 1, "open")],
        )
        .unwrap();
        let topology = TopologySnapshot::derive(&snapshot(vec![vent]), &full_profile()).unwrap();

        assert!(
            topology
                .graph(TopologyFacet::Occupancy)
                .unwrap()
                .neighbors(&region("region:inside"))
                .is_empty()
        );
        assert_eq!(
            topology
                .graph(TopologyFacet::AirPressure)
                .unwrap()
                .neighbors(&region("region:inside")),
            vec![region("region:outside")]
        );
    }

    #[test]
    fn projection_retains_interface_source_provenance() {
        let door_ref = source("device:door", 3, "closed");
        let door = BoundaryInterfaceSnapshot::new(
            interface_id("interface:door"),
            region("region:inside"),
            region("region:outside"),
            facets(&[(TopologyFacet::Occupancy, FacetRelation::Disconnected)]),
            vec![door_ref.clone()],
        )
        .unwrap();
        let topology = TopologySnapshot::derive(&snapshot(vec![door]), &full_profile()).unwrap();
        let occupancy = topology.graph(TopologyFacet::Occupancy).unwrap();
        assert_eq!(occupancy.interfaces()[0].source_refs(), &[door_ref]);
    }

    #[test]
    fn open_and_closed_door_are_distinct_exact_inputs_and_topologies() {
        let closed = BoundaryInterfaceSnapshot::new(
            interface_id("interface:door"),
            region("region:inside"),
            region("region:outside"),
            facets(&[
                (TopologyFacet::Occupancy, FacetRelation::Disconnected),
                (
                    TopologyFacet::AirPressure,
                    class("permeability:door-leakage"),
                ),
            ]),
            vec![source("device:door", 3, "closed")],
        )
        .unwrap();
        let opened = BoundaryInterfaceSnapshot::new(
            interface_id("interface:door"),
            region("region:inside"),
            region("region:outside"),
            facets(&[
                (TopologyFacet::Occupancy, FacetRelation::Connected),
                (TopologyFacet::AirPressure, FacetRelation::Connected),
            ]),
            vec![source("device:door", 4, "open")],
        )
        .unwrap();

        let closed_topology =
            TopologySnapshot::derive(&snapshot(vec![closed]), &full_profile()).unwrap();
        let open_topology =
            TopologySnapshot::derive(&snapshot(vec![opened]), &full_profile()).unwrap();
        assert_ne!(closed_topology, open_topology);
    }

    #[test]
    fn facet_profile_filters_without_mutating_or_reinterpreting_boundary() {
        let window = BoundaryInterfaceSnapshot::new(
            interface_id("interface:window"),
            region("region:inside"),
            region("region:outside"),
            facets(&[
                (TopologyFacet::Visibility, FacetRelation::Connected),
                (TopologyFacet::Thermal, class("conductance:glazing")),
            ]),
            vec![source("device:window", 1, "closed")],
        )
        .unwrap();
        let boundary = snapshot(vec![window]);
        let before = boundary.clone();
        let profile = TopologyProfile::new(
            id("topology-profile:visibility-only"),
            1,
            vec![TopologyFacet::Visibility],
        )
        .unwrap();
        let topology = TopologySnapshot::derive(&boundary, &profile).unwrap();

        assert_eq!(boundary, before);
        assert_eq!(topology.graphs().len(), 1);
        assert!(topology.graph(TopologyFacet::Visibility).is_some());
        assert!(topology.graph(TopologyFacet::Thermal).is_none());
    }

    #[test]
    fn conflicting_same_revision_source_ref_fails_closed() {
        let result = SpatialRegionSnapshot::new(
            region("region:inside"),
            vec![
                source("geom:wall", 9, "digest-a"),
                source("geom:wall", 9, "digest-b"),
            ],
        );
        assert!(matches!(
            result,
            Err(TopologyError::ConflictingExactRef { .. })
        ));
    }

    #[test]
    fn interface_must_reference_known_regions() {
        let orphan = BoundaryInterfaceSnapshot::new(
            interface_id("interface:orphan"),
            region("region:inside"),
            region("region:missing"),
            facets(&[(TopologyFacet::Occupancy, FacetRelation::Connected)]),
            vec![],
        )
        .unwrap();
        let result = BoundarySnapshot::new(
            id("boundary:invalid"),
            1,
            "digest",
            source("frame:invalid", 1, "frame"),
            source("environment:earth-air", 1, "env"),
            base_regions(),
            vec![orphan],
            vec![],
        );
        assert!(matches!(result, Err(TopologyError::UnknownRegion { .. })));
    }

    #[test]
    fn neighbors_are_deterministic_and_symmetric() {
        let a = region("region:a");
        let b = region("region:b");
        let c = region("region:c");
        let regions = vec![
            SpatialRegionSnapshot::new(a.clone(), vec![]).unwrap(),
            SpatialRegionSnapshot::new(b.clone(), vec![]).unwrap(),
            SpatialRegionSnapshot::new(c.clone(), vec![]).unwrap(),
        ];
        let interfaces = vec![
            BoundaryInterfaceSnapshot::new(
                interface_id("interface:a-c"),
                a.clone(),
                c.clone(),
                facets(&[(TopologyFacet::Occupancy, FacetRelation::Connected)]),
                vec![],
            )
            .unwrap(),
            BoundaryInterfaceSnapshot::new(
                interface_id("interface:a-b"),
                a.clone(),
                b.clone(),
                facets(&[(TopologyFacet::Occupancy, FacetRelation::Connected)]),
                vec![],
            )
            .unwrap(),
        ];
        let boundary = BoundarySnapshot::new(
            id("boundary:three-regions"),
            1,
            "digest",
            source("frame:test", 1, "frame"),
            source("environment:test", 1, "env"),
            regions,
            interfaces,
            vec![],
        )
        .unwrap();
        let profile = TopologyProfile::new(
            id("topology-profile:occupancy"),
            1,
            vec![TopologyFacet::Occupancy],
        )
        .unwrap();
        let topology = TopologySnapshot::derive(&boundary, &profile).unwrap();
        let occupancy = topology.graph(TopologyFacet::Occupancy).unwrap();

        assert_eq!(occupancy.neighbors(&a), vec![b.clone(), c.clone()]);
        assert_eq!(occupancy.neighbors(&b), vec![a.clone()]);
        assert_eq!(occupancy.neighbors(&c), vec![a]);
    }
}
