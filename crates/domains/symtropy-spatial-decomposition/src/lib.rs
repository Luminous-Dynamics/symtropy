// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! PB-04b deterministic cut-cell decomposition.
//!
//! V1 starts from exact local planar-cut facts. A qualified geometry adapter
//! must prove raw realized geometry into these cuts. The lattice is bounded
//! addressing/broad-phase scaffolding; an oblique cut splits one bucket into
//! two local fragments. PB-04b emits fragment/interface geometry only and never
//! fabricates PB-04a occupancy, air, acoustic, visibility, thermal, or weather
//! relations.

use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;
use symtropy_spatial_topology::{
    BoundaryInterfaceId, BoundaryInterfaceSnapshot, BoundarySnapshot, ExactSourceRef,
    SpatialRegionId, SpatialRegionSnapshot, TopologyError,
};

pub const DECOMPOSITION_SCHEMA_VERSION: u32 = 2;
pub const QUALIFIED_PB04A_PRODUCT_HEAD: &str = "1d500c62d93082e49f7a889656a1c92c552977e4";
pub const MAX_DIMENSION: u32 = 64;
pub const MAX_CELLS: usize = 4_096;
pub const MAX_PARTITION_FACTS: usize = 8_192;
pub const MAX_INTERFACES: usize = 32_768;
pub const MAX_SOURCES_PER_CUT: usize = 128;
pub const MAX_QUANTUM_UM: i64 = 1_000_000_000;
pub const MAX_ABS_COORD_UM: i64 = 1_000_000_000_000;
pub const MAX_ABS_PLANE_COEFF: i64 = 1_000_000_000_000;

const PROFILE_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.profile.v1\0";
const DOMAIN_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.domain.v1\0";
const FRAGMENT_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.fragment.v1\0";
const INTERFACE_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.interface.v1\0";
const SNAPSHOT_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.snapshot.v1\0";
const LOCUS_SEMANTICS_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.locus-semantics.v1\0";
const LEGACY_FRAME_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.legacy-frame.v1\0";
const PB04A_ADAPTER_PROFILE_DOMAIN: &[u8] =
    b"symtropy.spatial-decomposition.pb04a-adapter-profile.v1\0";
const PB04A_BOUNDARY_SUBJECT_DOMAIN: &[u8] =
    b"symtropy.spatial-decomposition.pb04a-boundary-subject.v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point3i {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

impl Point3i {
    pub const fn new(x: i64, y: i64, z: i64) -> Self {
        Self { x, y, z }
    }

    fn validate(self) -> Result<(), DecompositionError> {
        for value in [self.x, self.y, self.z] {
            if value == i64::MIN || value.abs() > MAX_ABS_COORD_UM {
                return Err(DecompositionError::CoordinateOutOfRange(value));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellCoord {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl CellCoord {
    pub const fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FragmentSide {
    Whole,
    Negative,
    Positive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FaceAxis {
    X,
    Y,
    Z,
}

/// Reduced, sign-normalized exact plane `a*x + b*y + c*z + d = 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalPlane {
    pub a: i64,
    pub b: i64,
    pub c: i64,
    pub d: i64,
}

impl CanonicalPlane {
    pub fn new(a: i64, b: i64, c: i64, d: i64) -> Result<Self, DecompositionError> {
        if a == 0 && b == 0 && c == 0 {
            return Err(DecompositionError::ZeroPlaneNormal);
        }
        for value in [a, b, c, d] {
            if value == i64::MIN || value.abs() > MAX_ABS_PLANE_COEFF {
                return Err(DecompositionError::PlaneCoefficientOutOfRange(value));
            }
        }
        let divisor = [a, b, c, d]
            .into_iter()
            .fold(0_i64, |g, value| gcd(g, value.abs()))
            .max(1);
        let mut result = Self {
            a: a / divisor,
            b: b / divisor,
            c: c / divisor,
            d: d / divisor,
        };
        if [result.a, result.b, result.c]
            .into_iter()
            .find(|value| *value != 0)
            .expect("normal checked")
            < 0
        {
            result.a = result
                .a
                .checked_neg()
                .ok_or(DecompositionError::ArithmeticOverflow)?;
            result.b = result
                .b
                .checked_neg()
                .ok_or(DecompositionError::ArithmeticOverflow)?;
            result.c = result
                .c
                .checked_neg()
                .ok_or(DecompositionError::ArithmeticOverflow)?;
            result.d = result
                .d
                .checked_neg()
                .ok_or(DecompositionError::ArithmeticOverflow)?;
        }
        Ok(result)
    }

    fn evaluate(self, point: Point3i) -> Result<i128, DecompositionError> {
        [self.a, self.b, self.c]
            .into_iter()
            .zip([point.x, point.y, point.z])
            .try_fold(i128::from(self.d), |sum, (coefficient, coordinate)| {
                let product = i128::from(coefficient)
                    .checked_mul(i128::from(coordinate))
                    .ok_or(DecompositionError::ArithmeticOverflow)?;
                sum.checked_add(product)
                    .ok_or(DecompositionError::ArithmeticOverflow)
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkBudget {
    pub max_cells: usize,
    pub max_partition_facts: usize,
    pub max_interfaces: usize,
}

impl WorkBudget {
    pub const fn reference_v1() -> Self {
        Self {
            max_cells: MAX_CELLS,
            max_partition_facts: MAX_PARTITION_FACTS,
            max_interfaces: MAX_INTERFACES,
        }
    }

    fn validate(self) -> Result<(), DecompositionError> {
        for (field, value, hard_max) in [
            ("max_cells", self.max_cells, MAX_CELLS),
            (
                "max_partition_facts",
                self.max_partition_facts,
                MAX_PARTITION_FACTS,
            ),
            ("max_interfaces", self.max_interfaces, MAX_INTERFACES),
        ] {
            if value == 0 || value > hard_max {
                return Err(DecompositionError::InvalidWorkBudget(field));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecompositionProfileRef {
    pub profile_id: StableId,
    pub revision: u64,
    pub content_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecompositionProfile {
    profile_id: StableId,
    revision: u64,
    backend_id: StableId,
    quantum_um: i64,
    budget: WorkBudget,
    digest: String,
}

impl DecompositionProfile {
    pub fn reference_v1(
        profile_id: StableId,
        revision: u64,
        quantum_um: i64,
    ) -> Result<Self, DecompositionError> {
        let backend = sid("pb04b.fixed-cut-cell.v1")?;
        Self::new(
            profile_id,
            revision,
            backend,
            quantum_um,
            WorkBudget::reference_v1(),
        )
    }

    pub fn new(
        profile_id: StableId,
        revision: u64,
        backend_id: StableId,
        quantum_um: i64,
        budget: WorkBudget,
    ) -> Result<Self, DecompositionError> {
        validate_id(&profile_id)?;
        validate_id(&backend_id)?;
        if quantum_um <= 0 || quantum_um > MAX_QUANTUM_UM {
            return Err(DecompositionError::InvalidQuantum(quantum_um));
        }
        budget.validate()?;
        let digest = profile_digest(&profile_id, revision, &backend_id, quantum_um, budget);
        Ok(Self {
            profile_id,
            revision,
            backend_id,
            quantum_um,
            budget,
            digest,
        })
    }

    pub fn exact_ref(&self) -> DecompositionProfileRef {
        DecompositionProfileRef {
            profile_id: self.profile_id.clone(),
            revision: self.revision,
            content_digest: self.digest.clone(),
        }
    }

    pub const fn quantum_um(&self) -> i64 {
        self.quantum_um
    }

    pub fn backend_id(&self) -> &StableId {
        &self.backend_id
    }

    fn source_ref(&self) -> Result<ExactSourceRef, DecompositionError> {
        Ok(ExactSourceRef::new(
            sid("symtropy.spatial-decomposition.profile")?,
            self.profile_id.clone(),
            self.revision,
            self.digest.clone(),
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnalysisDomainRef {
    pub domain_id: StableId,
    pub revision: u64,
    pub coordinate_frame_ref: ExactSourceRef,
    pub content_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalysisDomain {
    domain_id: StableId,
    revision: u64,
    coordinate_frame_ref: ExactSourceRef,
    origin: Point3i,
    dimensions: [u32; 3],
    digest: String,
}

impl AnalysisDomain {
    pub fn new(
        domain_id: StableId,
        revision: u64,
        origin: Point3i,
        dimensions: [u32; 3],
    ) -> Result<Self, DecompositionError> {
        let coordinate_frame_ref = legacy_coordinate_frame_ref(&domain_id)?;
        Self::new_in_frame(
            domain_id,
            revision,
            coordinate_frame_ref,
            origin,
            dimensions,
        )
    }

    pub fn new_in_frame(
        domain_id: StableId,
        revision: u64,
        coordinate_frame_ref: ExactSourceRef,
        origin: Point3i,
        dimensions: [u32; 3],
    ) -> Result<Self, DecompositionError> {
        validate_id(&domain_id)?;
        coordinate_frame_ref.validate()?;
        origin.validate()?;
        if dimensions
            .into_iter()
            .any(|value| value == 0 || value > MAX_DIMENSION)
        {
            return Err(DecompositionError::InvalidDimensions(dimensions));
        }
        let cells = cell_count(dimensions)?;
        if cells > MAX_CELLS {
            return Err(bound("analysis_domain.cells", cells, MAX_CELLS));
        }
        let digest = domain_digest(
            &domain_id,
            revision,
            &coordinate_frame_ref,
            origin,
            dimensions,
        );
        Ok(Self {
            domain_id,
            revision,
            coordinate_frame_ref,
            origin,
            dimensions,
            digest,
        })
    }

    pub fn exact_ref(&self) -> AnalysisDomainRef {
        AnalysisDomainRef {
            domain_id: self.domain_id.clone(),
            revision: self.revision,
            coordinate_frame_ref: self.coordinate_frame_ref.clone(),
            content_digest: self.digest.clone(),
        }
    }

    pub fn coordinate_frame_ref(&self) -> &ExactSourceRef {
        &self.coordinate_frame_ref
    }

    fn contains(&self, cell: CellCoord) -> bool {
        cell.x < self.dimensions[0] && cell.y < self.dimensions[1] && cell.z < self.dimensions[2]
    }

    fn source_ref(&self) -> Result<ExactSourceRef, DecompositionError> {
        Ok(ExactSourceRef::new(
            sid("symtropy.spatial-decomposition.domain")?,
            self.domain_id.clone(),
            self.revision,
            self.digest.clone(),
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RealizedGeometrySnapshotRef(pub ExactSourceRef);

impl RealizedGeometrySnapshotRef {
    pub fn new(reference: ExactSourceRef) -> Result<Self, DecompositionError> {
        reference.validate()?;
        Ok(Self(reference))
    }
}

/// A qualified adapter must prove that this exact plane partitions this local
/// bucket. PB-04b never silently extends a finite polygon or renderer mesh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPartitionCut {
    pub cell: CellCoord,
    pub plane: CanonicalPlane,
    barrier_sources: Vec<ExactSourceRef>,
    separator_sources: Vec<ExactSourceRef>,
}

impl LocalPartitionCut {
    pub fn new(
        cell: CellCoord,
        plane: CanonicalPlane,
        barrier_sources: Vec<ExactSourceRef>,
        separator_sources: Vec<ExactSourceRef>,
    ) -> Result<Self, DecompositionError> {
        let barrier_sources = canonical_refs(barrier_sources, "cut.barrier_sources")?;
        let separator_sources = canonical_refs(separator_sources, "cut.separator_sources")?;
        if barrier_sources.is_empty() && separator_sources.is_empty() {
            return Err(DecompositionError::PartitionEvidenceRequired(cell));
        }
        for (field, count) in [
            ("cut.barrier_sources", barrier_sources.len()),
            ("cut.separator_sources", separator_sources.len()),
        ] {
            if count > MAX_SOURCES_PER_CUT {
                return Err(bound(field, count, MAX_SOURCES_PER_CUT));
            }
        }
        Ok(Self {
            cell,
            plane,
            barrier_sources,
            separator_sources,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CutEvidence {
    plane: CanonicalPlane,
    barriers: Vec<ExactSourceRef>,
    separators: Vec<ExactSourceRef>,
}

impl CutEvidence {
    fn sources(&self) -> Result<Vec<ExactSourceRef>, DecompositionError> {
        let mut refs = self.barriers.clone();
        refs.extend(self.separators.clone());
        canonical_refs(refs, "cut.sources")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreeSpaceFragment {
    id: SpatialRegionId,
    cell: CellCoord,
    side: FragmentSide,
    sources: Vec<ExactSourceRef>,
    domain_boundary: bool,
}

impl FreeSpaceFragment {
    pub fn id(&self) -> &SpatialRegionId {
        &self.id
    }
    pub const fn cell(&self) -> CellCoord {
        self.cell
    }
    pub const fn side(&self) -> FragmentSide {
        self.side
    }
    pub fn source_refs(&self) -> &[ExactSourceRef] {
        &self.sources
    }
    pub const fn touches_analysis_boundary(&self) -> bool {
        self.domain_boundary
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometricInterfaceKind {
    OpenCrossFace {
        lower_cell: CellCoord,
        axis: FaceAxis,
    },
    LocalPartition {
        cell: CellCoord,
        plane: CanonicalPlane,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeometricInterface {
    id: BoundaryInterfaceId,
    first: SpatialRegionId,
    second: SpatialRegionId,
    kind: GeometricInterfaceKind,
    barriers: Vec<ExactSourceRef>,
    separators: Vec<ExactSourceRef>,
    sources: Vec<ExactSourceRef>,
}

impl GeometricInterface {
    pub fn id(&self) -> &BoundaryInterfaceId {
        &self.id
    }
    pub fn first_region(&self) -> &SpatialRegionId {
        &self.first
    }
    pub fn second_region(&self) -> &SpatialRegionId {
        &self.second
    }
    pub fn kind(&self) -> &GeometricInterfaceKind {
        &self.kind
    }
    pub fn barrier_sources(&self) -> &[ExactSourceRef] {
        &self.barriers
    }
    pub fn separator_sources(&self) -> &[ExactSourceRef] {
        &self.separators
    }
    pub const fn has_material_barrier(&self) -> bool {
        !self.barriers.is_empty()
    }
    pub const fn has_portal_separator(&self) -> bool {
        !self.separators.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeometricDecompositionSnapshot {
    schema_version: u32,
    geometry: RealizedGeometrySnapshotRef,
    profile: DecompositionProfileRef,
    domain: AnalysisDomainRef,
    digest: String,
    fragments: Vec<FreeSpaceFragment>,
    interfaces: Vec<GeometricInterface>,
}

impl GeometricDecompositionSnapshot {
    pub fn derive(
        geometry: RealizedGeometrySnapshotRef,
        profile: &DecompositionProfile,
        domain: &AnalysisDomain,
        cuts: Vec<LocalPartitionCut>,
    ) -> Result<Self, DecompositionError> {
        geometry.0.validate()?;
        validate_extent(profile, domain)?;
        let cells_total = cell_count(domain.dimensions)?;
        if cells_total > profile.budget.max_cells {
            return Err(bound(
                "decomposition.cells",
                cells_total,
                profile.budget.max_cells,
            ));
        }
        if cuts.len() > profile.budget.max_partition_facts {
            return Err(bound(
                "decomposition.partition_facts",
                cuts.len(),
                profile.budget.max_partition_facts,
            ));
        }

        let cuts = normalize_cuts(domain, cuts)?;
        let common = canonical_refs(
            vec![
                geometry.0.clone(),
                profile.source_ref()?,
                domain.source_ref()?,
            ],
            "decomposition.common_sources",
        )?;

        let mut fragments = Vec::with_capacity(cells_total.saturating_mul(2));
        let mut index = BTreeMap::new();
        for cell in cells(domain) {
            if let Some(cut) = cuts.get(&cell) {
                prove_cut(cut.plane, cell, profile, domain)?;
                let mut sources = common.clone();
                sources.extend(cut.sources()?);
                let sources = canonical_refs(sources, "fragment.sources")?;
                for side in [FragmentSide::Negative, FragmentSide::Positive] {
                    let position = fragments.len();
                    fragments.push(FreeSpaceFragment {
                        id: fragment_id(profile, domain, cell, side, Some(cut))?,
                        cell,
                        side,
                        sources: sources.clone(),
                        domain_boundary: on_domain_boundary(cell, domain),
                    });
                    index.insert((cell, side), position);
                }
            } else {
                let position = fragments.len();
                fragments.push(FreeSpaceFragment {
                    id: fragment_id(profile, domain, cell, FragmentSide::Whole, None)?,
                    cell,
                    side: FragmentSide::Whole,
                    sources: common.clone(),
                    domain_boundary: on_domain_boundary(cell, domain),
                });
                index.insert((cell, FragmentSide::Whole), position);
            }
        }

        let mut seen = BTreeSet::new();
        if fragments
            .iter()
            .any(|fragment| !seen.insert(fragment.id.clone()))
        {
            return Err(DecompositionError::IdentityCollision("fragment"));
        }

        let mut interfaces = Vec::new();
        for (&cell, cut) in &cuts {
            let negative = get_fragment(&fragments, &index, cell, FragmentSide::Negative)?;
            let positive = get_fragment(&fragments, &index, cell, FragmentSide::Positive)?;
            push_interface(
                &mut interfaces,
                &geometry,
                profile,
                domain,
                GeometricInterfaceKind::LocalPartition {
                    cell,
                    plane: cut.plane,
                },
                negative.id.clone(),
                positive.id.clone(),
                cut.barriers.clone(),
                cut.separators.clone(),
            )?;
        }

        for cell in cells(domain) {
            for axis in [FaceAxis::X, FaceAxis::Y, FaceAxis::Z] {
                if let Some(neighbor) = neighbor(cell, axis, domain) {
                    emit_cross_face(
                        &mut interfaces,
                        &fragments,
                        &index,
                        &cuts,
                        &geometry,
                        profile,
                        domain,
                        cell,
                        neighbor,
                        axis,
                    )?;
                }
            }
        }

        fragments.sort_by(|left, right| left.id.cmp(&right.id));
        interfaces.sort_by(|left, right| left.id.cmp(&right.id));
        if interfaces.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err(DecompositionError::IdentityCollision("interface"));
        }

        let profile_ref = profile.exact_ref();
        let domain_ref = domain.exact_ref();
        let digest = snapshot_digest(
            &geometry,
            &profile_ref,
            &domain_ref,
            &fragments,
            &interfaces,
        );
        Ok(Self {
            schema_version: DECOMPOSITION_SCHEMA_VERSION,
            geometry,
            profile: profile_ref,
            domain: domain_ref,
            digest,
            fragments,
            interfaces,
        })
    }

    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub fn content_digest(&self) -> &str {
        &self.digest
    }
    pub fn fragments(&self) -> &[FreeSpaceFragment] {
        &self.fragments
    }
    pub fn interfaces(&self) -> &[GeometricInterface] {
        &self.interfaces
    }
    pub fn profile_ref(&self) -> &DecompositionProfileRef {
        &self.profile
    }

    /// Converts geometry atoms into qualified PB-04a vocabulary while storing
    /// zero facet states. Geometry does not get to answer semantic questions.
    pub fn to_pb04a_boundary(
        &self,
        frame_ref: ExactSourceRef,
        environment_ref: ExactSourceRef,
    ) -> Result<BoundarySnapshot, DecompositionError> {
        let regions = self
            .fragments
            .iter()
            .map(|fragment| {
                SpatialRegionSnapshot::new(fragment.id.clone(), fragment.sources.clone())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let interfaces = self
            .interfaces
            .iter()
            .map(|interface| {
                BoundaryInterfaceSnapshot::new(
                    interface.id.clone(),
                    interface.first.clone(),
                    interface.second.clone(),
                    Vec::new(),
                    interface.sources.clone(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        frame_ref.validate()?;
        environment_ref.validate()?;
        let profile_source = ExactSourceRef::new(
            sid("symtropy.spatial-decomposition.profile")?,
            self.profile.profile_id.clone(),
            self.profile.revision,
            self.profile.content_digest.clone(),
        )?;
        let domain_source = ExactSourceRef::new(
            sid("symtropy.spatial-decomposition.domain")?,
            self.domain.domain_id.clone(),
            self.domain.revision,
            self.domain.content_digest.clone(),
        )?;
        let adapter_source = pb04a_adapter_source_ref()?;
        let subject_digest = pb04a_boundary_subject_digest(
            &self.digest,
            &frame_ref,
            &environment_ref,
            &adapter_source,
        );

        Ok(BoundarySnapshot::new(
            sid_digest("pb04b-boundary", &subject_digest)?,
            0,
            self.digest.clone(),
            frame_ref,
            environment_ref,
            regions,
            interfaces,
            canonical_refs(
                vec![
                    self.geometry.0.clone(),
                    profile_source,
                    domain_source,
                    adapter_source,
                ],
                "boundary.sources",
            )?,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecompositionError {
    Topology(TopologyError),
    InvalidStableId(String),
    InvalidQuantum(i64),
    InvalidDimensions([u32; 3]),
    InvalidWorkBudget(&'static str),
    CoordinateOutOfRange(i64),
    PlaneCoefficientOutOfRange(i64),
    ZeroPlaneNormal,
    ArithmeticOverflow,
    BoundExceeded {
        field: &'static str,
        actual: usize,
        maximum: usize,
    },
    CellOutsideDomain(CellCoord),
    PartitionEvidenceRequired(CellCoord),
    PlaneDoesNotCutCell(CellCoord),
    MultipleCutPlanesUnsupported(CellCoord),
    AdjacentDifferentCutPlanesUnsupported {
        first: CellCoord,
        second: CellCoord,
    },
    CoincidentFaceUnsupported {
        cell: CellCoord,
        axis: FaceAxis,
    },
    ConflictingExactRef {
        field: &'static str,
        authority_id: StableId,
        subject_id: StableId,
        revision: u64,
    },
    MissingFragment {
        cell: CellCoord,
        side: FragmentSide,
    },
    IdentityCollision(&'static str),
}

impl From<TopologyError> for DecompositionError {
    fn from(value: TopologyError) -> Self {
        Self::Topology(value)
    }
}

impl fmt::Display for DecompositionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Topology(error) => error.fmt(f),
            Self::InvalidStableId(value) => write!(f, "invalid stable identifier {value:?}"),
            Self::InvalidQuantum(value) => write!(f, "invalid quantum {value} um"),
            Self::InvalidDimensions(value) => write!(f, "invalid dimensions {value:?}"),
            Self::InvalidWorkBudget(field) => write!(f, "invalid work budget {field}"),
            Self::CoordinateOutOfRange(value) => write!(f, "coordinate out of range: {value}"),
            Self::PlaneCoefficientOutOfRange(value) => {
                write!(f, "plane coefficient out of range: {value}")
            }
            Self::ZeroPlaneNormal => write!(f, "partition plane has zero normal"),
            Self::ArithmeticOverflow => write!(f, "checked PB-04b arithmetic overflow"),
            Self::BoundExceeded {
                field,
                actual,
                maximum,
            } => write!(f, "{field} has {actual} entries, maximum is {maximum}"),
            Self::CellOutsideDomain(cell) => write!(f, "cell {cell:?} is outside analysis domain"),
            Self::PartitionEvidenceRequired(cell) => {
                write!(f, "cut {cell:?} has no barrier/separator evidence")
            }
            Self::PlaneDoesNotCutCell(cell) => {
                write!(f, "plane does not cut cell {cell:?} interior")
            }
            Self::MultipleCutPlanesUnsupported(cell) => {
                write!(f, "multiple planes in {cell:?} require refinement")
            }
            Self::AdjacentDifferentCutPlanesUnsupported { first, second } => write!(
                f,
                "different planes in adjacent cut cells {first:?}/{second:?} require refinement"
            ),
            Self::CoincidentFaceUnsupported { cell, axis } => {
                write!(f, "plane coincides with {axis:?} face of {cell:?}")
            }
            Self::ConflictingExactRef {
                field,
                authority_id,
                subject_id,
                revision,
            } => write!(
                f,
                "{field} has competing digests for {authority_id}/{subject_id}@{revision}"
            ),
            Self::MissingFragment { cell, side } => {
                write!(f, "missing fragment {cell:?}/{side:?}")
            }
            Self::IdentityCollision(subject) => write!(f, "{subject} identity collision"),
        }
    }
}

impl Error for DecompositionError {}

fn normalize_cuts(
    domain: &AnalysisDomain,
    cuts: Vec<LocalPartitionCut>,
) -> Result<BTreeMap<CellCoord, CutEvidence>, DecompositionError> {
    let mut result = BTreeMap::<CellCoord, CutEvidence>::new();
    for cut in cuts {
        if !domain.contains(cut.cell) {
            return Err(DecompositionError::CellOutsideDomain(cut.cell));
        }
        if let Some(existing) = result.get_mut(&cut.cell) {
            if existing.plane != cut.plane {
                return Err(DecompositionError::MultipleCutPlanesUnsupported(cut.cell));
            }
            existing.barriers = merge_refs(
                &existing.barriers,
                &cut.barrier_sources,
                "cut.barrier_sources",
            )?;
            existing.separators = merge_refs(
                &existing.separators,
                &cut.separator_sources,
                "cut.separator_sources",
            )?;
        } else {
            result.insert(
                cut.cell,
                CutEvidence {
                    plane: cut.plane,
                    barriers: cut.barrier_sources,
                    separators: cut.separator_sources,
                },
            );
        }
    }
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn emit_cross_face(
    output: &mut Vec<GeometricInterface>,
    fragments: &[FreeSpaceFragment],
    index: &BTreeMap<(CellCoord, FragmentSide), usize>,
    cuts: &BTreeMap<CellCoord, CutEvidence>,
    geometry: &RealizedGeometrySnapshotRef,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    cell: CellCoord,
    other: CellCoord,
    axis: FaceAxis,
) -> Result<(), DecompositionError> {
    let left = cuts.get(&cell);
    let right = cuts.get(&other);
    if let (Some(left), Some(right)) = (left, right)
        && left.plane != right.plane
    {
        return Err(DecompositionError::AdjacentDifferentCutPlanesUnsupported {
            first: cell,
            second: other,
        });
    }
    let face = face_corners(cell, axis, profile, domain)?;
    for left_side in sides(left) {
        if !touches_face_area(left_side, left.map(|cut| cut.plane), &face, cell, axis)? {
            continue;
        }
        for right_side in sides(right) {
            if !touches_face_area(right_side, right.map(|cut| cut.plane), &face, other, axis)? {
                continue;
            }
            if left.is_some()
                && right.is_some()
                && matches!(
                    (left_side, right_side),
                    (FragmentSide::Negative, FragmentSide::Positive)
                        | (FragmentSide::Positive, FragmentSide::Negative)
                )
            {
                continue;
            }
            let first = get_fragment(fragments, index, cell, left_side)?;
            let second = get_fragment(fragments, index, other, right_side)?;
            let sources = canonical_refs(
                vec![
                    geometry.0.clone(),
                    profile.source_ref()?,
                    domain.source_ref()?,
                ],
                "open_face.sources",
            )?;
            push_interface_with_sources(
                output,
                geometry,
                profile,
                domain,
                GeometricInterfaceKind::OpenCrossFace {
                    lower_cell: cell,
                    axis,
                },
                first.id.clone(),
                second.id.clone(),
                Vec::new(),
                Vec::new(),
                sources,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn push_interface(
    output: &mut Vec<GeometricInterface>,
    geometry: &RealizedGeometrySnapshotRef,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    kind: GeometricInterfaceKind,
    first: SpatialRegionId,
    second: SpatialRegionId,
    barriers: Vec<ExactSourceRef>,
    separators: Vec<ExactSourceRef>,
) -> Result<(), DecompositionError> {
    let mut sources = vec![
        geometry.0.clone(),
        profile.source_ref()?,
        domain.source_ref()?,
    ];
    sources.extend(barriers.clone());
    sources.extend(separators.clone());
    push_interface_with_sources(
        output,
        geometry,
        profile,
        domain,
        kind,
        first,
        second,
        barriers,
        separators,
        canonical_refs(sources, "interface.sources")?,
    )
}

#[allow(clippy::too_many_arguments)]
fn push_interface_with_sources(
    output: &mut Vec<GeometricInterface>,
    _geometry: &RealizedGeometrySnapshotRef,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    kind: GeometricInterfaceKind,
    mut first: SpatialRegionId,
    mut second: SpatialRegionId,
    barriers: Vec<ExactSourceRef>,
    separators: Vec<ExactSourceRef>,
    sources: Vec<ExactSourceRef>,
) -> Result<(), DecompositionError> {
    if output.len() >= profile.budget.max_interfaces {
        return Err(bound(
            "decomposition.interfaces",
            output.len() + 1,
            profile.budget.max_interfaces,
        ));
    }
    if second < first {
        std::mem::swap(&mut first, &mut second);
    }
    let id = interface_id(profile, domain, &kind, &first, &second)?;
    output.push(GeometricInterface {
        id,
        first,
        second,
        kind,
        barriers,
        separators,
        sources,
    });
    Ok(())
}

fn get_fragment<'a>(
    fragments: &'a [FreeSpaceFragment],
    index: &BTreeMap<(CellCoord, FragmentSide), usize>,
    cell: CellCoord,
    side: FragmentSide,
) -> Result<&'a FreeSpaceFragment, DecompositionError> {
    index
        .get(&(cell, side))
        .and_then(|position| fragments.get(*position))
        .ok_or(DecompositionError::MissingFragment { cell, side })
}

fn sides(cut: Option<&CutEvidence>) -> impl Iterator<Item = FragmentSide> {
    let values = if cut.is_some() {
        [Some(FragmentSide::Negative), Some(FragmentSide::Positive)]
    } else {
        [Some(FragmentSide::Whole), None]
    };
    values.into_iter().flatten()
}

fn touches_face_area(
    side: FragmentSide,
    plane: Option<CanonicalPlane>,
    face: &[Point3i; 4],
    cell: CellCoord,
    axis: FaceAxis,
) -> Result<bool, DecompositionError> {
    if side == FragmentSide::Whole {
        return Ok(true);
    }
    let plane = plane.ok_or(DecompositionError::MissingFragment { cell, side })?;
    let mut negative = false;
    let mut positive = false;
    let mut zero = true;
    for &point in face {
        let value = plane.evaluate(point)?;
        negative |= value < 0;
        positive |= value > 0;
        zero &= value == 0;
    }
    if zero {
        return Err(DecompositionError::CoincidentFaceUnsupported { cell, axis });
    }
    Ok(match side {
        FragmentSide::Negative => negative,
        FragmentSide::Positive => positive,
        FragmentSide::Whole => true,
    })
}

fn prove_cut(
    plane: CanonicalPlane,
    cell: CellCoord,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
) -> Result<(), DecompositionError> {
    let mut negative = false;
    let mut positive = false;
    for point in cell_corners(cell, profile, domain)? {
        let value = plane.evaluate(point)?;
        negative |= value < 0;
        positive |= value > 0;
    }
    if negative && positive {
        Ok(())
    } else {
        Err(DecompositionError::PlaneDoesNotCutCell(cell))
    }
}

fn validate_extent(
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
) -> Result<(), DecompositionError> {
    let last = CellCoord::new(
        domain.dimensions[0] - 1,
        domain.dimensions[1] - 1,
        domain.dimensions[2] - 1,
    );
    let (_, max) = cell_bounds(last, profile, domain)?;
    max.validate()
}

fn cell_bounds(
    cell: CellCoord,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
) -> Result<(Point3i, Point3i), DecompositionError> {
    if !domain.contains(cell) {
        return Err(DecompositionError::CellOutsideDomain(cell));
    }
    let q = i128::from(profile.quantum_um);
    let coordinate = |origin: i64, index: u32| -> Result<(i64, i64), DecompositionError> {
        let min = i128::from(origin)
            .checked_add(
                i128::from(index)
                    .checked_mul(q)
                    .ok_or(DecompositionError::ArithmeticOverflow)?,
            )
            .ok_or(DecompositionError::ArithmeticOverflow)?;
        let max = min
            .checked_add(q)
            .ok_or(DecompositionError::ArithmeticOverflow)?;
        Ok((
            i64::try_from(min).map_err(|_| DecompositionError::ArithmeticOverflow)?,
            i64::try_from(max).map_err(|_| DecompositionError::ArithmeticOverflow)?,
        ))
    };
    let (x0, x1) = coordinate(domain.origin.x, cell.x)?;
    let (y0, y1) = coordinate(domain.origin.y, cell.y)?;
    let (z0, z1) = coordinate(domain.origin.z, cell.z)?;
    let min = Point3i::new(x0, y0, z0);
    let max = Point3i::new(x1, y1, z1);
    min.validate()?;
    max.validate()?;
    Ok((min, max))
}

fn cell_corners(
    cell: CellCoord,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
) -> Result<[Point3i; 8], DecompositionError> {
    let (a, b) = cell_bounds(cell, profile, domain)?;
    Ok([
        Point3i::new(a.x, a.y, a.z),
        Point3i::new(b.x, a.y, a.z),
        Point3i::new(a.x, b.y, a.z),
        Point3i::new(b.x, b.y, a.z),
        Point3i::new(a.x, a.y, b.z),
        Point3i::new(b.x, a.y, b.z),
        Point3i::new(a.x, b.y, b.z),
        Point3i::new(b.x, b.y, b.z),
    ])
}

fn face_corners(
    cell: CellCoord,
    axis: FaceAxis,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
) -> Result<[Point3i; 4], DecompositionError> {
    let (a, b) = cell_bounds(cell, profile, domain)?;
    Ok(match axis {
        FaceAxis::X => [
            Point3i::new(b.x, a.y, a.z),
            Point3i::new(b.x, b.y, a.z),
            Point3i::new(b.x, a.y, b.z),
            Point3i::new(b.x, b.y, b.z),
        ],
        FaceAxis::Y => [
            Point3i::new(a.x, b.y, a.z),
            Point3i::new(b.x, b.y, a.z),
            Point3i::new(a.x, b.y, b.z),
            Point3i::new(b.x, b.y, b.z),
        ],
        FaceAxis::Z => [
            Point3i::new(a.x, a.y, b.z),
            Point3i::new(b.x, a.y, b.z),
            Point3i::new(a.x, b.y, b.z),
            Point3i::new(b.x, b.y, b.z),
        ],
    })
}

fn cells(domain: &AnalysisDomain) -> impl Iterator<Item = CellCoord> + '_ {
    (0..domain.dimensions[2]).flat_map(move |z| {
        (0..domain.dimensions[1])
            .flat_map(move |y| (0..domain.dimensions[0]).map(move |x| CellCoord::new(x, y, z)))
    })
}

fn neighbor(cell: CellCoord, axis: FaceAxis, domain: &AnalysisDomain) -> Option<CellCoord> {
    let candidate = match axis {
        FaceAxis::X => CellCoord::new(cell.x.checked_add(1)?, cell.y, cell.z),
        FaceAxis::Y => CellCoord::new(cell.x, cell.y.checked_add(1)?, cell.z),
        FaceAxis::Z => CellCoord::new(cell.x, cell.y, cell.z.checked_add(1)?),
    };
    domain.contains(candidate).then_some(candidate)
}

fn on_domain_boundary(cell: CellCoord, domain: &AnalysisDomain) -> bool {
    cell.x == 0
        || cell.y == 0
        || cell.z == 0
        || cell.x + 1 == domain.dimensions[0]
        || cell.y + 1 == domain.dimensions[1]
        || cell.z + 1 == domain.dimensions[2]
}

fn cell_count(dimensions: [u32; 3]) -> Result<usize, DecompositionError> {
    dimensions.into_iter().try_fold(1_usize, |product, value| {
        product
            .checked_mul(value as usize)
            .ok_or(DecompositionError::ArithmeticOverflow)
    })
}

fn fragment_id(
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    cell: CellCoord,
    side: FragmentSide,
    cut: Option<&CutEvidence>,
) -> Result<SpatialRegionId, DecompositionError> {
    let mut hash = Sha256::new();
    hash.update(FRAGMENT_DOMAIN);
    hash_locus_semantics(&mut hash, profile);
    hash_exact(&mut hash, &domain.coordinate_frame_ref);
    hash_physical_cell(&mut hash, profile, domain, cell)?;
    hash_side(&mut hash, side);
    match cut {
        Some(cut) => {
            hash.update([1]);
            hash_plane(&mut hash, cut.plane);
        }
        None => hash.update([0]),
    }
    Ok(SpatialRegionId::new(sid_digest(
        "pb04b-region",
        &hex(&hash.finalize()),
    )?)?)
}

#[allow(clippy::too_many_arguments)]
fn interface_id(
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    kind: &GeometricInterfaceKind,
    first: &SpatialRegionId,
    second: &SpatialRegionId,
) -> Result<BoundaryInterfaceId, DecompositionError> {
    let mut hash = Sha256::new();
    hash.update(INTERFACE_DOMAIN);
    hash_locus_semantics(&mut hash, profile);
    hash_exact(&mut hash, &domain.coordinate_frame_ref);
    hash_text(&mut hash, first.0.as_str());
    hash_text(&mut hash, second.0.as_str());
    hash_kind_locus(&mut hash, kind, profile, domain)?;
    Ok(BoundaryInterfaceId::new(sid_digest(
        "pb04b-interface",
        &hex(&hash.finalize()),
    )?)?)
}

fn snapshot_digest(
    geometry: &RealizedGeometrySnapshotRef,
    profile: &DecompositionProfileRef,
    domain: &AnalysisDomainRef,
    fragments: &[FreeSpaceFragment],
    interfaces: &[GeometricInterface],
) -> String {
    let mut hash = Sha256::new();
    hash.update(SNAPSHOT_DOMAIN);
    hash_u32(&mut hash, DECOMPOSITION_SCHEMA_VERSION);
    hash_exact(&mut hash, &geometry.0);
    hash_profile(&mut hash, profile);
    hash_analysis_domain(&mut hash, domain);
    hash_u64(&mut hash, fragments.len() as u64);
    for fragment in fragments {
        hash_text(&mut hash, fragment.id.0.as_str());
        hash_cell(&mut hash, fragment.cell);
        hash_side(&mut hash, fragment.side);
        hash.update([u8::from(fragment.domain_boundary)]);
        hash_refs(&mut hash, &fragment.sources);
    }
    hash_u64(&mut hash, interfaces.len() as u64);
    for interface in interfaces {
        hash_text(&mut hash, interface.id.0.as_str());
        hash_text(&mut hash, interface.first.0.as_str());
        hash_text(&mut hash, interface.second.0.as_str());
        hash_kind(&mut hash, &interface.kind);
        hash_refs(&mut hash, &interface.barriers);
        hash_refs(&mut hash, &interface.separators);
        hash_refs(&mut hash, &interface.sources);
    }
    hex(&hash.finalize())
}

fn profile_digest(
    id: &StableId,
    revision: u64,
    backend: &StableId,
    quantum: i64,
    budget: WorkBudget,
) -> String {
    let mut hash = Sha256::new();
    hash.update(PROFILE_DOMAIN);
    hash_text(&mut hash, id.as_str());
    hash_u64(&mut hash, revision);
    hash_text(&mut hash, backend.as_str());
    hash_i64(&mut hash, quantum);
    for value in [
        budget.max_cells,
        budget.max_partition_facts,
        budget.max_interfaces,
    ] {
        hash_u64(&mut hash, value as u64);
    }
    hex(&hash.finalize())
}

fn legacy_coordinate_frame_ref(domain_id: &StableId) -> Result<ExactSourceRef, DecompositionError> {
    let mut hash = Sha256::new();
    hash.update(LEGACY_FRAME_DOMAIN);
    hash_text(&mut hash, domain_id.as_str());
    let digest = hex(&hash.finalize());
    Ok(ExactSourceRef::new(
        sid("symtropy.spatial-decomposition.frame")?,
        domain_id.clone(),
        0,
        digest,
    )?)
}

fn domain_digest(
    id: &StableId,
    revision: u64,
    coordinate_frame_ref: &ExactSourceRef,
    origin: Point3i,
    dimensions: [u32; 3],
) -> String {
    let mut hash = Sha256::new();
    hash.update(DOMAIN_DOMAIN);
    hash_text(&mut hash, id.as_str());
    hash_u64(&mut hash, revision);
    hash_exact(&mut hash, coordinate_frame_ref);
    for value in [origin.x, origin.y, origin.z] {
        hash_i64(&mut hash, value);
    }
    for value in dimensions {
        hash_u32(&mut hash, value);
    }
    hex(&hash.finalize())
}

fn hash_locus_semantics(hash: &mut Sha256, profile: &DecompositionProfile) {
    hash.update(LOCUS_SEMANTICS_DOMAIN);
    hash_u32(hash, DECOMPOSITION_SCHEMA_VERSION);
    hash_text(hash, profile.backend_id.as_str());
    hash_i64(hash, profile.quantum_um);
}

fn hash_point(hash: &mut Sha256, point: Point3i) {
    hash_i64(hash, point.x);
    hash_i64(hash, point.y);
    hash_i64(hash, point.z);
}

fn hash_physical_cell(
    hash: &mut Sha256,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    cell: CellCoord,
) -> Result<(), DecompositionError> {
    let (min, max) = cell_bounds(cell, profile, domain)?;
    hash_point(hash, min);
    hash_point(hash, max);
    Ok(())
}

fn hash_kind_locus(
    hash: &mut Sha256,
    kind: &GeometricInterfaceKind,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
) -> Result<(), DecompositionError> {
    match kind {
        GeometricInterfaceKind::OpenCrossFace { lower_cell, axis } => {
            hash.update([0]);
            hash.update([match axis {
                FaceAxis::X => 0,
                FaceAxis::Y => 1,
                FaceAxis::Z => 2,
            }]);
            for point in face_corners(*lower_cell, *axis, profile, domain)? {
                hash_point(hash, point);
            }
        }
        GeometricInterfaceKind::LocalPartition { cell, plane } => {
            hash.update([1]);
            hash_physical_cell(hash, profile, domain, *cell)?;
            hash_plane(hash, *plane);
        }
    }
    Ok(())
}

fn pb04a_adapter_source_ref() -> Result<ExactSourceRef, DecompositionError> {
    let mut hash = Sha256::new();
    hash.update(PB04A_ADAPTER_PROFILE_DOMAIN);
    hash_u32(&mut hash, DECOMPOSITION_SCHEMA_VERSION);
    hash_text(&mut hash, QUALIFIED_PB04A_PRODUCT_HEAD);
    let digest = hex(&hash.finalize());
    Ok(ExactSourceRef::new(
        sid("symtropy.spatial-decomposition.adapter")?,
        sid("pb04b.to-pb04a.v2")?,
        1,
        digest,
    )?)
}

fn pb04a_boundary_subject_digest(
    decomposition_digest: &str,
    frame_ref: &ExactSourceRef,
    environment_ref: &ExactSourceRef,
    adapter_ref: &ExactSourceRef,
) -> String {
    let mut hash = Sha256::new();
    hash.update(PB04A_BOUNDARY_SUBJECT_DOMAIN);
    hash_text(&mut hash, decomposition_digest);
    hash_exact(&mut hash, frame_ref);
    hash_exact(&mut hash, environment_ref);
    hash_exact(&mut hash, adapter_ref);
    hex(&hash.finalize())
}

fn hash_kind(hash: &mut Sha256, kind: &GeometricInterfaceKind) {
    match kind {
        GeometricInterfaceKind::OpenCrossFace { lower_cell, axis } => {
            hash.update([0]);
            hash_cell(hash, *lower_cell);
            hash.update([match axis {
                FaceAxis::X => 0,
                FaceAxis::Y => 1,
                FaceAxis::Z => 2,
            }]);
        }
        GeometricInterfaceKind::LocalPartition { cell, plane } => {
            hash.update([1]);
            hash_cell(hash, *cell);
            hash_plane(hash, *plane);
        }
    }
}

fn hash_profile(hash: &mut Sha256, value: &DecompositionProfileRef) {
    hash_text(hash, value.profile_id.as_str());
    hash_u64(hash, value.revision);
    hash_text(hash, &value.content_digest);
}

fn hash_analysis_domain(hash: &mut Sha256, value: &AnalysisDomainRef) {
    hash_text(hash, value.domain_id.as_str());
    hash_u64(hash, value.revision);
    hash_exact(hash, &value.coordinate_frame_ref);
    hash_text(hash, &value.content_digest);
}

fn hash_side(hash: &mut Sha256, side: FragmentSide) {
    hash.update([match side {
        FragmentSide::Whole => 0,
        FragmentSide::Negative => 1,
        FragmentSide::Positive => 2,
    }]);
}

fn hash_plane(hash: &mut Sha256, plane: CanonicalPlane) {
    for value in [plane.a, plane.b, plane.c, plane.d] {
        hash_i64(hash, value);
    }
}

fn hash_cell(hash: &mut Sha256, cell: CellCoord) {
    hash_u32(hash, cell.x);
    hash_u32(hash, cell.y);
    hash_u32(hash, cell.z);
}

fn hash_refs(hash: &mut Sha256, refs: &[ExactSourceRef]) {
    hash_u64(hash, refs.len() as u64);
    for reference in refs {
        hash_exact(hash, reference);
    }
}

fn hash_exact(hash: &mut Sha256, value: &ExactSourceRef) {
    hash_text(hash, value.authority_id.as_str());
    hash_text(hash, value.subject_id.as_str());
    hash_u64(hash, value.revision);
    hash_text(hash, &value.digest);
}

fn hash_text(hash: &mut Sha256, value: &str) {
    hash_u64(hash, value.len() as u64);
    hash.update(value.as_bytes());
}

fn hash_u32(hash: &mut Sha256, value: u32) {
    hash.update(value.to_le_bytes());
}
fn hash_u64(hash: &mut Sha256, value: u64) {
    hash.update(value.to_le_bytes());
}
fn hash_i64(hash: &mut Sha256, value: i64) {
    hash.update(value.to_le_bytes());
}

fn canonical_refs(
    mut refs: Vec<ExactSourceRef>,
    field: &'static str,
) -> Result<Vec<ExactSourceRef>, DecompositionError> {
    for reference in &refs {
        reference.validate()?;
    }
    refs.sort();
    let mut output: Vec<ExactSourceRef> = Vec::with_capacity(refs.len());
    for reference in refs {
        if let Some(previous) = output.last() {
            let same = previous.authority_id == reference.authority_id
                && previous.subject_id == reference.subject_id
                && previous.revision == reference.revision;
            if same {
                if previous.digest != reference.digest {
                    return Err(DecompositionError::ConflictingExactRef {
                        field,
                        authority_id: reference.authority_id,
                        subject_id: reference.subject_id,
                        revision: reference.revision,
                    });
                }
                continue;
            }
        }
        output.push(reference);
    }
    Ok(output)
}

fn merge_refs(
    left: &[ExactSourceRef],
    right: &[ExactSourceRef],
    field: &'static str,
) -> Result<Vec<ExactSourceRef>, DecompositionError> {
    let mut all = left.to_vec();
    all.extend_from_slice(right);
    canonical_refs(all, field)
}

fn sid(value: &str) -> Result<StableId, DecompositionError> {
    StableId::parse(value).map_err(|_| DecompositionError::InvalidStableId(value.into()))
}

fn sid_digest(namespace: &str, digest: &str) -> Result<StableId, DecompositionError> {
    let prefix = digest
        .get(..32)
        .ok_or_else(|| DecompositionError::InvalidStableId(digest.into()))?;
    sid(&format!("{namespace}:{prefix}"))
}

fn validate_id(value: &StableId) -> Result<(), DecompositionError> {
    StableId::parse(value.as_str())
        .map(|_| ())
        .map_err(|_| DecompositionError::InvalidStableId(value.as_str().into()))
}

fn bound(field: &'static str, actual: usize, maximum: usize) -> DecompositionError {
    DecompositionError::BoundExceeded {
        field,
        actual,
        maximum,
    }
}

fn gcd(mut left: i64, mut right: i64) -> i64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.abs()
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn exact(subject: &str, digest: &str) -> ExactSourceRef {
        ExactSourceRef::new(id("geometry"), id(subject), 1, digest).unwrap()
    }

    fn geometry() -> RealizedGeometrySnapshotRef {
        RealizedGeometrySnapshotRef::new(
            ExactSourceRef::new(
                id("geometry"),
                id("crooked-shelter"),
                7,
                "sha256:geometry-v7",
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn profile(quantum: i64) -> DecompositionProfile {
        DecompositionProfile::reference_v1(id("pb04b-profile"), 1, quantum).unwrap()
    }

    fn domain(dimensions: [u32; 3]) -> AnalysisDomain {
        AnalysisDomain::new(id("pb04b-domain"), 1, Point3i::new(0, 0, 0), dimensions).unwrap()
    }

    fn cut(
        cell: CellCoord,
        plane: CanonicalPlane,
        barriers: Vec<ExactSourceRef>,
        separators: Vec<ExactSourceRef>,
    ) -> LocalPartitionCut {
        LocalPartitionCut::new(cell, plane, barriers, separators).unwrap()
    }

    #[test]
    fn canonical_plane_rejects_representation_drift() {
        assert_eq!(
            CanonicalPlane::new(2, -2, 0, 0).unwrap(),
            CanonicalPlane::new(-1, 1, 0, 0).unwrap()
        );
    }

    #[test]
    fn diagonal_cut_has_two_fragments_and_one_retained_boundary() {
        let snapshot = GeometricDecompositionSnapshot::derive(
            geometry(),
            &profile(10),
            &domain([1, 1, 1]),
            vec![cut(
                CellCoord::new(0, 0, 0),
                CanonicalPlane::new(1, -1, 0, 0).unwrap(),
                vec![exact("wall", "sha256:wall")],
                Vec::new(),
            )],
        )
        .unwrap();
        assert_eq!(snapshot.fragments().len(), 2);
        assert_eq!(snapshot.interfaces().len(), 1);
        assert!(snapshot.interfaces()[0].has_material_barrier());
        assert_ne!(
            snapshot.interfaces()[0].first_region(),
            snapshot.interfaces()[0].second_region()
        );
    }

    #[test]
    fn finite_wall_is_not_erased_by_open_incidence_around_its_end() {
        let snapshot = GeometricDecompositionSnapshot::derive(
            geometry(),
            &profile(10),
            &domain([2, 1, 1]),
            vec![cut(
                CellCoord::new(0, 0, 0),
                CanonicalPlane::new(0, 2, 0, -10).unwrap(),
                vec![exact("short-wall", "sha256:short-wall")],
                Vec::new(),
            )],
        )
        .unwrap();
        assert_eq!(snapshot.fragments().len(), 3);
        assert_eq!(
            snapshot
                .interfaces()
                .iter()
                .filter(|value| matches!(
                    value.kind(),
                    GeometricInterfaceKind::LocalPartition { .. }
                ))
                .count(),
            1
        );
        assert_eq!(
            snapshot
                .interfaces()
                .iter()
                .filter(|value| matches!(
                    value.kind(),
                    GeometricInterfaceKind::OpenCrossFace { .. }
                ))
                .count(),
            2
        );
    }

    #[test]
    fn barrier_and_separator_are_orthogonal_evidence_channels() {
        let snapshot = GeometricDecompositionSnapshot::derive(
            geometry(),
            &profile(10),
            &domain([1, 1, 1]),
            vec![cut(
                CellCoord::new(0, 0, 0),
                CanonicalPlane::new(0, 1, 0, -5).unwrap(),
                vec![exact("door-panel", "sha256:door-closed")],
                vec![exact("doorway", "sha256:doorway")],
            )],
        )
        .unwrap();
        assert!(snapshot.interfaces()[0].has_material_barrier());
        assert!(snapshot.interfaces()[0].has_portal_separator());
    }

    #[test]
    fn shuffled_same_plane_facts_have_identical_identity() {
        let plane = CanonicalPlane::new(0, 1, 0, -5).unwrap();
        let a = cut(
            CellCoord::new(0, 0, 0),
            plane,
            vec![exact("wall-a", "sha256:a")],
            Vec::new(),
        );
        let b = cut(
            CellCoord::new(1, 0, 0),
            plane,
            vec![exact("wall-b", "sha256:b")],
            Vec::new(),
        );
        let first = GeometricDecompositionSnapshot::derive(
            geometry(),
            &profile(10),
            &domain([2, 1, 1]),
            vec![a.clone(), b.clone()],
        )
        .unwrap();
        let second = GeometricDecompositionSnapshot::derive(
            geometry(),
            &profile(10),
            &domain([2, 1, 1]),
            vec![b, a],
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.content_digest(), second.content_digest());
    }

    #[test]
    fn distinct_planes_in_one_bucket_fail_closed() {
        let result = GeometricDecompositionSnapshot::derive(
            geometry(),
            &profile(10),
            &domain([1, 1, 1]),
            vec![
                cut(
                    CellCoord::new(0, 0, 0),
                    CanonicalPlane::new(1, 0, 0, -5).unwrap(),
                    vec![exact("x", "sha256:x")],
                    Vec::new(),
                ),
                cut(
                    CellCoord::new(0, 0, 0),
                    CanonicalPlane::new(0, 1, 0, -5).unwrap(),
                    vec![exact("y", "sha256:y")],
                    Vec::new(),
                ),
            ],
        );
        assert!(matches!(
            result,
            Err(DecompositionError::MultipleCutPlanesUnsupported(_))
        ));
    }

    #[test]
    fn profile_change_changes_exact_snapshot_identity() {
        let make = |quantum| {
            GeometricDecompositionSnapshot::derive(
                geometry(),
                &profile(quantum),
                &domain([1, 1, 1]),
                vec![cut(
                    CellCoord::new(0, 0, 0),
                    CanonicalPlane::new(0, 1, 0, -5).unwrap(),
                    vec![exact("wall", "sha256:wall")],
                    Vec::new(),
                )],
            )
            .unwrap()
        };
        let a = make(10);
        let b = make(12);
        assert_ne!(a.profile_ref(), b.profile_ref());
        assert_ne!(a.content_digest(), b.content_digest());
    }

    #[test]
    fn pb04a_adapter_makes_no_facet_claims() {
        let snapshot = GeometricDecompositionSnapshot::derive(
            geometry(),
            &profile(10),
            &domain([1, 1, 1]),
            vec![cut(
                CellCoord::new(0, 0, 0),
                CanonicalPlane::new(0, 1, 0, -5).unwrap(),
                vec![exact("wall", "sha256:wall")],
                vec![exact("portal", "sha256:portal")],
            )],
        )
        .unwrap();
        let boundary = snapshot
            .to_pb04a_boundary(
                ExactSourceRef::new(id("frame"), id("local"), 1, "sha256:frame").unwrap(),
                ExactSourceRef::new(id("environment"), id("world"), 1, "sha256:world").unwrap(),
            )
            .unwrap();
        assert_eq!(boundary.regions().len(), 2);
        assert_eq!(boundary.interfaces().len(), 1);
        assert!(boundary.interfaces()[0].facet_states().is_empty());
        boundary.validate_canonical().unwrap();
    }

    #[test]
    fn analysis_limit_is_not_emitted_as_physical_boundary() {
        let snapshot = GeometricDecompositionSnapshot::derive(
            geometry(),
            &profile(10),
            &domain([1, 1, 1]),
            Vec::new(),
        )
        .unwrap();
        assert_eq!(snapshot.fragments().len(), 1);
        assert!(snapshot.fragments()[0].touches_analysis_boundary());
        assert!(snapshot.interfaces().is_empty());
    }
}
