#!/usr/bin/env python3
from pathlib import Path

lib_path = Path("crates/domains/symtropy-spatial-decomposition/src/lib.rs")
test_path = Path("crates/domains/symtropy-spatial-decomposition/tests/local_identity.rs")
text = lib_path.read_text()
test_text = test_path.read_text()


def replace_once(old: str, new: str, label: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    text = text.replace(old, new, 1)


def replace_test_once(old: str, new: str, label: str) -> None:
    global test_text
    count = test_text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one test match, found {count}")
    test_text = test_text.replace(old, new, 1)


replace_once(
    "pub const DECOMPOSITION_SCHEMA_VERSION: u32 = 2;",
    "pub const DECOMPOSITION_SCHEMA_VERSION: u32 = 3;",
    "schema3",
)

replace_once(
    '''    pub fn coordinate_frame_ref(&self) -> &ExactSourceRef {
        &self.coordinate_frame_ref
    }

    fn contains(&self, cell: CellCoord) -> bool {
''',
    '''    pub fn coordinate_frame_ref(&self) -> &ExactSourceRef {
        &self.coordinate_frame_ref
    }

    pub const fn dimensions(&self) -> [u32; 3] {
        self.dimensions
    }

    fn contains(&self, cell: CellCoord) -> bool {
''',
    "domain dimensions accessor",
)

census_types = r'''
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellPartitionObservation {
    cell: CellCoord,
    coverage_ref: ExactSourceRef,
    cuts: Vec<LocalPartitionCut>,
}

impl CellPartitionObservation {
    pub fn new(
        cell: CellCoord,
        coverage_ref: ExactSourceRef,
        cuts: Vec<LocalPartitionCut>,
    ) -> Result<Self, DecompositionError> {
        coverage_ref.validate()?;
        for cut in &cuts {
            if cut.cell != cell {
                return Err(DecompositionError::CutObservationCellMismatch {
                    observation: cell,
                    cut: cut.cell,
                });
            }
        }
        if cuts.len() > MAX_PARTITION_FACTS {
            return Err(bound(
                "cell_partition_observation.cuts",
                cuts.len(),
                MAX_PARTITION_FACTS,
            ));
        }
        Ok(Self {
            cell,
            coverage_ref,
            cuts,
        })
    }

    pub const fn cell(&self) -> CellCoord {
        self.cell
    }

    pub fn coverage_ref(&self) -> &ExactSourceRef {
        &self.coverage_ref
    }

    pub fn cuts(&self) -> &[LocalPartitionCut] {
        &self.cuts
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPartitionCensus {
    geometry: RealizedGeometrySnapshotRef,
    domain: AnalysisDomainRef,
    observations: Vec<CellPartitionObservation>,
}

impl LocalPartitionCensus {
    pub fn new(
        geometry: RealizedGeometrySnapshotRef,
        domain: &AnalysisDomain,
        observations: Vec<CellPartitionObservation>,
    ) -> Result<Self, DecompositionError> {
        geometry.0.validate()?;
        let expected = cell_count(domain.dimensions)?;
        if observations.len() > MAX_CELLS {
            return Err(bound(
                "local_partition_census.observations",
                observations.len(),
                MAX_CELLS,
            ));
        }

        let mut by_cell = BTreeMap::new();
        let mut coverage_refs = Vec::with_capacity(observations.len());
        let mut cut_count = 0_usize;
        for observation in observations {
            if !domain.contains(observation.cell) {
                return Err(DecompositionError::CellOutsideDomain(observation.cell));
            }
            observation.coverage_ref.validate()?;
            for cut in &observation.cuts {
                if cut.cell != observation.cell {
                    return Err(DecompositionError::CutObservationCellMismatch {
                        observation: observation.cell,
                        cut: cut.cell,
                    });
                }
            }
            cut_count = cut_count
                .checked_add(observation.cuts.len())
                .ok_or(DecompositionError::ArithmeticOverflow)?;
            if cut_count > MAX_PARTITION_FACTS {
                return Err(bound(
                    "local_partition_census.partition_facts",
                    cut_count,
                    MAX_PARTITION_FACTS,
                ));
            }
            coverage_refs.push(observation.coverage_ref.clone());
            let cell = observation.cell;
            if by_cell.insert(cell, observation).is_some() {
                return Err(DecompositionError::DuplicateCellObservation(cell));
            }
        }

        canonical_refs(coverage_refs, "local_partition_census.coverage_refs")?;
        for cell in cells(domain) {
            if !by_cell.contains_key(&cell) {
                return Err(DecompositionError::MissingCellObservation(cell));
            }
        }
        if by_cell.len() != expected {
            return Err(bound(
                "local_partition_census.observations",
                by_cell.len(),
                expected,
            ));
        }

        Ok(Self {
            geometry,
            domain: domain.exact_ref(),
            observations: by_cell.into_values().collect(),
        })
    }

    pub fn geometry_ref(&self) -> &RealizedGeometrySnapshotRef {
        &self.geometry
    }

    pub fn domain_ref(&self) -> &AnalysisDomainRef {
        &self.domain
    }

    pub fn observations(&self) -> &[CellPartitionObservation] {
        &self.observations
    }
}

'''
replace_once(
    '''#[derive(Debug, Clone, PartialEq, Eq)]
struct CutEvidence {
''',
    census_types + '''#[derive(Debug, Clone, PartialEq, Eq)]
struct CutEvidence {
''',
    "census types",
)

replace_once(
    '''    pub fn separator_sources(&self) -> &[ExactSourceRef] {
        &self.separators
    }
    pub const fn has_material_barrier(&self) -> bool {
''',
    '''    pub fn separator_sources(&self) -> &[ExactSourceRef] {
        &self.separators
    }
    pub fn source_refs(&self) -> &[ExactSourceRef] {
        &self.sources
    }
    pub const fn has_material_barrier(&self) -> bool {
''',
    "interface source accessor",
)

old_derive_head = r'''    pub fn derive(
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
'''
new_derive_head = r'''    #[deprecated(note = "schema 3 requires an explicit complete LocalPartitionCensus")]
    pub fn derive(
        _geometry: RealizedGeometrySnapshotRef,
        _profile: &DecompositionProfile,
        _domain: &AnalysisDomain,
        _cuts: Vec<LocalPartitionCut>,
    ) -> Result<Self, DecompositionError> {
        Err(DecompositionError::CompleteCensusRequired)
    }

    pub fn derive_from_census(
        profile: &DecompositionProfile,
        domain: &AnalysisDomain,
        census: LocalPartitionCensus,
    ) -> Result<Self, DecompositionError> {
        if census.domain != domain.exact_ref() {
            return Err(DecompositionError::CensusDomainMismatch);
        }
        let geometry = census.geometry;
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

        let partition_facts = census.observations.iter().try_fold(
            0_usize,
            |count, observation| {
                count
                    .checked_add(observation.cuts.len())
                    .ok_or(DecompositionError::ArithmeticOverflow)
            },
        )?;
        if partition_facts > profile.budget.max_partition_facts {
            return Err(bound(
                "decomposition.partition_facts",
                partition_facts,
                profile.budget.max_partition_facts,
            ));
        }

        let mut coverage = BTreeMap::new();
        let mut sparse_cuts = Vec::with_capacity(partition_facts);
        for observation in census.observations {
            coverage.insert(observation.cell, observation.coverage_ref);
            sparse_cuts.extend(observation.cuts);
        }
        let cuts = normalize_cuts(domain, sparse_cuts)?;
        let common = canonical_refs(
            vec![profile.source_ref()?, domain.source_ref()?],
            "decomposition.common_sources",
        )?;
'''
replace_once(old_derive_head, new_derive_head, "derive census authority")

replace_once(
    '''        for cell in cells(domain) {
            if let Some(cut) = cuts.get(&cell) {
                prove_cut(cut.plane, cell, profile, domain)?;
                let mut sources = common.clone();
                sources.extend(cut.sources()?);
                let sources = canonical_refs(sources, "fragment.sources")?;
''',
    '''        for cell in cells(domain) {
            let coverage_ref = coverage
                .get(&cell)
                .ok_or(DecompositionError::MissingCellObservation(cell))?;
            if let Some(cut) = cuts.get(&cell) {
                prove_cut(cut.plane, cell, profile, domain)?;
                let mut sources = common.clone();
                sources.push(coverage_ref.clone());
                sources.extend(cut.sources()?);
                let sources = canonical_refs(sources, "fragment.sources")?;
''',
    "cut fragment coverage",
)

replace_once(
    '''            } else {
                let position = fragments.len();
                fragments.push(FreeSpaceFragment {
                    id: fragment_id(profile, domain, cell, FragmentSide::Whole, None)?,
                    cell,
                    side: FragmentSide::Whole,
                    sources: common.clone(),
                    domain_boundary: on_domain_boundary(cell, domain),
                });
''',
    '''            } else {
                let position = fragments.len();
                let mut sources = common.clone();
                sources.push(coverage_ref.clone());
                let sources = canonical_refs(sources, "fragment.sources")?;
                fragments.push(FreeSpaceFragment {
                    id: fragment_id(profile, domain, cell, FragmentSide::Whole, None)?,
                    cell,
                    side: FragmentSide::Whole,
                    sources,
                    domain_boundary: on_domain_boundary(cell, domain),
                });
''',
    "clear fragment coverage",
)

replace_once(
    '''        for (&cell, cut) in &cuts {
            let negative = get_fragment(&fragments, &index, cell, FragmentSide::Negative)?;
            let positive = get_fragment(&fragments, &index, cell, FragmentSide::Positive)?;
            push_interface(
''',
    '''        for (&cell, cut) in &cuts {
            let coverage_ref = coverage
                .get(&cell)
                .ok_or(DecompositionError::MissingCellObservation(cell))?
                .clone();
            let negative = get_fragment(&fragments, &index, cell, FragmentSide::Negative)?;
            let positive = get_fragment(&fragments, &index, cell, FragmentSide::Positive)?;
            push_interface(
''',
    "partition coverage lookup",
)

replace_once(
    '''                negative.id.clone(),
                positive.id.clone(),
                cut.barriers.clone(),
                cut.separators.clone(),
''',
    '''                negative.id.clone(),
                positive.id.clone(),
                vec![coverage_ref],
                cut.barriers.clone(),
                cut.separators.clone(),
''',
    "partition coverage argument",
)

replace_once(
    '''                        &cuts,
                        &geometry,
                        profile,
''',
    '''                        &cuts,
                        &coverage,
                        &geometry,
                        profile,
''',
    "cross-face coverage argument",
)

replace_once(
    '''    cuts: &BTreeMap<CellCoord, CutEvidence>,
    geometry: &RealizedGeometrySnapshotRef,
    profile: &DecompositionProfile,
''',
    '''    cuts: &BTreeMap<CellCoord, CutEvidence>,
    coverage: &BTreeMap<CellCoord, ExactSourceRef>,
    geometry: &RealizedGeometrySnapshotRef,
    profile: &DecompositionProfile,
''',
    "cross-face coverage signature",
)

replace_once(
    '''            let sources = canonical_refs(
                vec![
                    geometry.0.clone(),
                    profile.source_ref()?,
                    domain.source_ref()?,
                ],
                "open_face.sources",
            )?;
''',
    '''            let left_coverage = coverage
                .get(&cell)
                .ok_or(DecompositionError::MissingCellObservation(cell))?
                .clone();
            let right_coverage = coverage
                .get(&other)
                .ok_or(DecompositionError::MissingCellObservation(other))?
                .clone();
            let sources = canonical_refs(
                vec![
                    profile.source_ref()?,
                    domain.source_ref()?,
                    left_coverage,
                    right_coverage,
                ],
                "open_face.sources",
            )?;
''',
    "cross-face local provenance",
)

replace_once(
    '''    first: SpatialRegionId,
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
''',
    '''    first: SpatialRegionId,
    second: SpatialRegionId,
    coverage_refs: Vec<ExactSourceRef>,
    barriers: Vec<ExactSourceRef>,
    separators: Vec<ExactSourceRef>,
) -> Result<(), DecompositionError> {
    let mut sources = vec![profile.source_ref()?, domain.source_ref()?];
    sources.extend(coverage_refs);
    sources.extend(barriers.clone());
''',
    "partition local provenance",
)

replace_once(
    '''    CellOutsideDomain(CellCoord),
    PartitionEvidenceRequired(CellCoord),
''',
    '''    CellOutsideDomain(CellCoord),
    CompleteCensusRequired,
    DuplicateCellObservation(CellCoord),
    MissingCellObservation(CellCoord),
    CensusDomainMismatch,
    CutObservationCellMismatch {
        observation: CellCoord,
        cut: CellCoord,
    },
    PartitionEvidenceRequired(CellCoord),
''',
    "census errors",
)

replace_once(
    '''            Self::CellOutsideDomain(cell) => write!(f, "cell {cell:?} is outside analysis domain"),
            Self::PartitionEvidenceRequired(cell) => {
''',
    '''            Self::CellOutsideDomain(cell) => write!(f, "cell {cell:?} is outside analysis domain"),
            Self::CompleteCensusRequired => write!(
                f,
                "schema 3 requires a complete local partition census; sparse omission is not clear-space evidence"
            ),
            Self::DuplicateCellObservation(cell) => {
                write!(f, "duplicate partition observation for cell {cell:?}")
            }
            Self::MissingCellObservation(cell) => {
                write!(f, "missing partition observation for cell {cell:?}")
            }
            Self::CensusDomainMismatch => {
                write!(f, "partition census was proven for a different analysis domain")
            }
            Self::CutObservationCellMismatch { observation, cut } => write!(
                f,
                "partition cut for {cut:?} was supplied under observation {observation:?}"
            ),
            Self::PartitionEvidenceRequired(cell) => {
''',
    "census display",
)

# Add a test-only adapter that supplies explicit coverage for every cell, then
# migrate the existing unit tests without preserving a production sparse path.
test_helper_marker = r'''    fn cut(
        cell: CellCoord,
        plane: CanonicalPlane,
        barriers: Vec<ExactSourceRef>,
        separators: Vec<ExactSourceRef>,
    ) -> LocalPartitionCut {
        LocalPartitionCut::new(cell, plane, barriers, separators).unwrap()
    }
'''
test_helper = test_helper_marker + r'''

    fn test_coverage(cell: CellCoord) -> ExactSourceRef {
        ExactSourceRef::new(
            id("coverage"),
            id(&format!("cell-{}-{}-{}", cell.x, cell.y, cell.z)),
            1,
            format!("sha256:coverage-{}-{}-{}", cell.x, cell.y, cell.z),
        )
        .unwrap()
    }

    fn derive_sparse_for_test(
        geometry: RealizedGeometrySnapshotRef,
        profile: &DecompositionProfile,
        domain: &AnalysisDomain,
        cuts: Vec<LocalPartitionCut>,
    ) -> Result<GeometricDecompositionSnapshot, DecompositionError> {
        let observations = cells(domain)
            .map(|cell| {
                let cell_cuts = cuts
                    .iter()
                    .filter(|cut| cut.cell == cell)
                    .cloned()
                    .collect();
                CellPartitionObservation::new(cell, test_coverage(cell), cell_cuts).unwrap()
            })
            .collect();
        let census = LocalPartitionCensus::new(geometry, domain, observations).unwrap();
        GeometricDecompositionSnapshot::derive_from_census(profile, domain, census)
    }
'''
replace_once(test_helper_marker, test_helper, "unit-test complete census helper")

prefix, marker, suffix = text.partition("#[cfg(test)]")
if not marker:
    raise SystemExit("unit test marker missing")
suffix_count = suffix.count("GeometricDecompositionSnapshot::derive(")
if suffix_count == 0:
    raise SystemExit("expected legacy derive calls in unit tests")
suffix = suffix.replace("GeometricDecompositionSnapshot::derive(", "derive_sparse_for_test(")
text = prefix + marker + suffix

replace_test_once(
    '''    AnalysisDomain, CanonicalPlane, CellCoord, DecompositionProfile,
    GeometricDecompositionSnapshot, GeometricInterfaceKind, LocalPartitionCut, Point3i,
    RealizedGeometrySnapshotRef, WorkBudget,
''',
    '''    AnalysisDomain, CanonicalPlane, CellCoord, CellPartitionObservation, DecompositionProfile,
    GeometricDecompositionSnapshot, GeometricInterfaceKind, LocalPartitionCensus,
    LocalPartitionCut, Point3i, RealizedGeometrySnapshotRef, WorkBudget,
''',
    "locality imports",
)

replace_test_once(
    '''fn derive_in_domain(
    geometry: RealizedGeometrySnapshotRef,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    cut: LocalPartitionCut,
) -> GeometricDecompositionSnapshot {
    GeometricDecompositionSnapshot::derive(geometry, profile, domain, vec![cut])
        .expect("test decomposition")
}
''',
    '''fn coverage(cell: CellCoord) -> ExactSourceRef {
    exact(
        "symtropy.test.coverage",
        &format!("cell-{}-{}-{}", cell.x, cell.y, cell.z),
        1,
        &format!("coverage-{}-{}-{}-v1", cell.x, cell.y, cell.z),
    )
}

fn derive_in_domain(
    geometry: RealizedGeometrySnapshotRef,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    cut: LocalPartitionCut,
) -> GeometricDecompositionSnapshot {
    let dimensions = domain.dimensions();
    let mut observations = Vec::new();
    for z in 0..dimensions[2] {
        for y in 0..dimensions[1] {
            for x in 0..dimensions[0] {
                let cell = CellCoord::new(x, y, z);
                let cuts = if cell == cut.cell {
                    vec![cut.clone()]
                } else {
                    Vec::new()
                };
                observations.push(
                    CellPartitionObservation::new(cell, coverage(cell), cuts)
                        .expect("coverage observation"),
                );
            }
        }
    }
    let census = LocalPartitionCensus::new(geometry, domain, observations).expect("census");
    GeometricDecompositionSnapshot::derive_from_census(profile, domain, census)
        .expect("test decomposition")
}
''',
    "locality derive helper",
)

lib_path.write_text(text)
test_path.write_text(test_text)
print("PB-04b1 complete local coverage transform applied")
