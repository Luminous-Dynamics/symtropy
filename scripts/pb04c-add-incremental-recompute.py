#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/domains/symtropy-spatial-decomposition/src/lib.rs")
text = path.read_text()

anchor = '''#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecompositionError {
'''
if text.count(anchor) != 1:
    raise SystemExit("expected exactly one DecompositionError insertion anchor")

block = r'''#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncrementalFullRebuildReason {
    ProfileChanged,
    DomainChanged,
    SchemaChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncrementalDecompositionState {
    profile: DecompositionProfileRef,
    domain: AnalysisDomainRef,
    census: LocalPartitionCensus,
    snapshot: GeometricDecompositionSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncrementalDecompositionUpdate {
    state: IncrementalDecompositionState,
    changed_cells: Vec<CellCoord>,
    recompute_cells: Vec<CellCoord>,
}

impl IncrementalDecompositionUpdate {
    pub fn state(&self) -> &IncrementalDecompositionState {
        &self.state
    }

    pub fn snapshot(&self) -> &GeometricDecompositionSnapshot {
        self.state.snapshot()
    }

    pub fn changed_cells(&self) -> &[CellCoord] {
        &self.changed_cells
    }

    pub fn recompute_cells(&self) -> &[CellCoord] {
        &self.recompute_cells
    }

    pub fn into_state(self) -> IncrementalDecompositionState {
        self.state
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncrementalRecomputeOutcome {
    Updated(IncrementalDecompositionUpdate),
    FullRebuildRequired(IncrementalFullRebuildReason),
}

impl IncrementalDecompositionState {
    pub fn new(
        profile: &DecompositionProfile,
        domain: &AnalysisDomain,
        census: LocalPartitionCensus,
    ) -> Result<Self, DecompositionError> {
        let snapshot = GeometricDecompositionSnapshot::derive_from_census(
            profile,
            domain,
            census.clone(),
        )?;
        Ok(Self {
            profile: profile.exact_ref(),
            domain: domain.exact_ref(),
            census,
            snapshot,
        })
    }

    pub fn from_parts(
        profile: &DecompositionProfile,
        domain: &AnalysisDomain,
        census: LocalPartitionCensus,
        snapshot: GeometricDecompositionSnapshot,
    ) -> Result<Self, DecompositionError> {
        let expected = GeometricDecompositionSnapshot::derive_from_census(
            profile,
            domain,
            census.clone(),
        )?;
        if expected != snapshot {
            return Err(DecompositionError::PredecessorSnapshotMismatch);
        }
        Ok(Self {
            profile: profile.exact_ref(),
            domain: domain.exact_ref(),
            census,
            snapshot,
        })
    }

    pub fn census(&self) -> &LocalPartitionCensus {
        &self.census
    }

    pub fn snapshot(&self) -> &GeometricDecompositionSnapshot {
        &self.snapshot
    }

    pub fn advance(
        &self,
        profile: &DecompositionProfile,
        domain: &AnalysisDomain,
        successor_census: LocalPartitionCensus,
    ) -> Result<IncrementalRecomputeOutcome, DecompositionError> {
        if self.snapshot.schema_version != DECOMPOSITION_SCHEMA_VERSION {
            return Ok(IncrementalRecomputeOutcome::FullRebuildRequired(
                IncrementalFullRebuildReason::SchemaChanged,
            ));
        }
        if profile.exact_ref() != self.profile {
            return Ok(IncrementalRecomputeOutcome::FullRebuildRequired(
                IncrementalFullRebuildReason::ProfileChanged,
            ));
        }
        if domain.exact_ref() != self.domain {
            return Ok(IncrementalRecomputeOutcome::FullRebuildRequired(
                IncrementalFullRebuildReason::DomainChanged,
            ));
        }
        if successor_census.domain != self.domain {
            return Err(DecompositionError::CensusDomainMismatch);
        }

        let (snapshot, changed_cells, recompute_cells) = incremental_snapshot(
            self,
            profile,
            domain,
            &successor_census,
        )?;
        let state = Self {
            profile: self.profile.clone(),
            domain: self.domain.clone(),
            census: successor_census,
            snapshot,
        };
        Ok(IncrementalRecomputeOutcome::Updated(
            IncrementalDecompositionUpdate {
                state,
                changed_cells,
                recompute_cells,
            },
        ))
    }
}

fn incremental_snapshot(
    previous: &IncrementalDecompositionState,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    successor: &LocalPartitionCensus,
) -> Result<
    (GeometricDecompositionSnapshot, Vec<CellCoord>, Vec<CellCoord>),
    DecompositionError,
> {
    validate_extent(profile, domain)?;
    let cells_total = cell_count(domain.dimensions)?;
    if cells_total > profile.budget.max_cells {
        return Err(bound(
            "decomposition.cells",
            cells_total,
            profile.budget.max_cells,
        ));
    }

    let partition_facts = successor.observations.iter().try_fold(
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

    let previous_observations = previous
        .census
        .observations
        .iter()
        .map(|observation| (observation.cell, observation))
        .collect::<BTreeMap<_, _>>();
    let mut changed_cells = Vec::new();
    for observation in &successor.observations {
        let predecessor = previous_observations
            .get(&observation.cell)
            .ok_or(DecompositionError::PredecessorSnapshotMismatch)?;
        if *predecessor != observation {
            changed_cells.push(observation.cell);
        }
    }
    if changed_cells.len() > cells_total {
        return Err(DecompositionError::PredecessorSnapshotMismatch);
    }

    let mut recompute = BTreeSet::new();
    for &cell in &changed_cells {
        recompute.insert(cell);
        for neighbor in all_face_neighbors(cell, domain) {
            recompute.insert(neighbor);
        }
    }
    let recompute_cells = recompute.iter().copied().collect::<Vec<_>>();

    let mut coverage = BTreeMap::new();
    let mut sparse_cuts = Vec::with_capacity(partition_facts);
    for observation in &successor.observations {
        coverage.insert(observation.cell, observation.coverage_ref.clone());
        sparse_cuts.extend(observation.cuts.clone());
    }
    let cuts = normalize_cuts(domain, sparse_cuts)?;
    let common = canonical_refs(
        vec![profile.source_ref()?, domain.source_ref()?],
        "decomposition.common_sources",
    )?;

    let mut previous_index = BTreeMap::new();
    for (position, fragment) in previous.snapshot.fragments.iter().enumerate() {
        if previous_index
            .insert((fragment.cell, fragment.side), position)
            .is_some()
        {
            return Err(DecompositionError::PredecessorSnapshotMismatch);
        }
    }

    let mut fragments = Vec::with_capacity(cells_total.saturating_mul(2));
    let mut index = BTreeMap::new();
    for cell in cells(domain) {
        if recompute.contains(&cell) {
            let coverage_ref = coverage
                .get(&cell)
                .ok_or(DecompositionError::MissingCellObservation(cell))?;
            if let Some(cut) = cuts.get(&cell) {
                prove_cut(cut.plane, cell, profile, domain)?;
                let mut sources = common.clone();
                sources.push(coverage_ref.clone());
                sources.extend(cut.sources()?);
                let sources = canonical_refs(sources, "fragment.sources")?;
                for side in [FragmentSide::Negative, FragmentSide::Positive] {
                    let position = fragments.len();
                    if index.insert((cell, side), position).is_some() {
                        return Err(DecompositionError::IdentityCollision("fragment-slot"));
                    }
                    fragments.push(FreeSpaceFragment {
                        id: fragment_id(profile, domain, cell, side, Some(cut))?,
                        cell,
                        side,
                        sources: sources.clone(),
                        domain_boundary: on_domain_boundary(cell, domain),
                    });
                }
            } else {
                let position = fragments.len();
                let mut sources = common.clone();
                sources.push(coverage_ref.clone());
                let sources = canonical_refs(sources, "fragment.sources")?;
                if index
                    .insert((cell, FragmentSide::Whole), position)
                    .is_some()
                {
                    return Err(DecompositionError::IdentityCollision("fragment-slot"));
                }
                fragments.push(FreeSpaceFragment {
                    id: fragment_id(profile, domain, cell, FragmentSide::Whole, None)?,
                    cell,
                    side: FragmentSide::Whole,
                    sources,
                    domain_boundary: on_domain_boundary(cell, domain),
                });
            }
        } else {
            for side in sides(cuts.get(&cell)) {
                let old_position = previous_index
                    .get(&(cell, side))
                    .ok_or(DecompositionError::PredecessorSnapshotMismatch)?;
                let fragment = previous
                    .snapshot
                    .fragments
                    .get(*old_position)
                    .ok_or(DecompositionError::PredecessorSnapshotMismatch)?
                    .clone();
                let position = fragments.len();
                if index.insert((cell, side), position).is_some() {
                    return Err(DecompositionError::IdentityCollision("fragment-slot"));
                }
                fragments.push(fragment);
            }
        }
    }

    let mut seen = BTreeSet::new();
    if fragments
        .iter()
        .any(|fragment| !seen.insert(fragment.id.clone()))
    {
        return Err(DecompositionError::IdentityCollision("fragment"));
    }

    let geometry = successor.geometry.clone();
    let mut interfaces = Vec::new();
    for (&cell, cut) in &cuts {
        if recompute.contains(&cell) {
            let coverage_ref = coverage
                .get(&cell)
                .ok_or(DecompositionError::MissingCellObservation(cell))?
                .clone();
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
                vec![coverage_ref],
                cut.barriers.clone(),
                cut.separators.clone(),
            )?;
        } else {
            let mut matched = 0_usize;
            for interface in &previous.snapshot.interfaces {
                if let GeometricInterfaceKind::LocalPartition {
                    cell: old_cell,
                    plane,
                } = &interface.kind
                    && *old_cell == cell
                    && *plane == cut.plane
                {
                    interfaces.push(interface.clone());
                    matched += 1;
                }
            }
            if matched != 1 {
                return Err(DecompositionError::PredecessorSnapshotMismatch);
            }
        }
    }

    for cell in cells(domain) {
        for axis in [FaceAxis::X, FaceAxis::Y, FaceAxis::Z] {
            if let Some(other) = neighbor(cell, axis, domain) {
                if recompute.contains(&cell) || recompute.contains(&other) {
                    emit_cross_face(
                        &mut interfaces,
                        &fragments,
                        &index,
                        &cuts,
                        &coverage,
                        &geometry,
                        profile,
                        domain,
                        cell,
                        other,
                        axis,
                    )?;
                } else {
                    for interface in &previous.snapshot.interfaces {
                        if let GeometricInterfaceKind::OpenCrossFace {
                            lower_cell,
                            axis: old_axis,
                        } = &interface.kind
                            && *lower_cell == cell
                            && *old_axis == axis
                        {
                            interfaces.push(interface.clone());
                        }
                    }
                }
            }
        }
    }

    if interfaces.len() > profile.budget.max_interfaces {
        return Err(bound(
            "decomposition.interfaces",
            interfaces.len(),
            profile.budget.max_interfaces,
        ));
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
    let snapshot = GeometricDecompositionSnapshot {
        schema_version: DECOMPOSITION_SCHEMA_VERSION,
        geometry,
        profile: profile_ref,
        domain: domain_ref,
        digest,
        fragments,
        interfaces,
    };
    Ok((snapshot, changed_cells, recompute_cells))
}

fn all_face_neighbors(cell: CellCoord, domain: &AnalysisDomain) -> Vec<CellCoord> {
    let mut output = Vec::with_capacity(6);
    let candidates = [
        cell.x.checked_sub(1).map(|x| CellCoord::new(x, cell.y, cell.z)),
        cell.x.checked_add(1).map(|x| CellCoord::new(x, cell.y, cell.z)),
        cell.y.checked_sub(1).map(|y| CellCoord::new(cell.x, y, cell.z)),
        cell.y.checked_add(1).map(|y| CellCoord::new(cell.x, y, cell.z)),
        cell.z.checked_sub(1).map(|z| CellCoord::new(cell.x, cell.y, z)),
        cell.z.checked_add(1).map(|z| CellCoord::new(cell.x, cell.y, z)),
    ];
    for candidate in candidates.into_iter().flatten() {
        if domain.contains(candidate) {
            output.push(candidate);
        }
    }
    output
}

'''

text = text.replace(anchor, block + anchor, 1)

variant_anchor = '''    CensusDomainMismatch,
    CutObservationCellMismatch {
'''
if text.count(variant_anchor) != 1:
    raise SystemExit("expected exactly one PB-04c error variant anchor")
text = text.replace(
    variant_anchor,
    '''    CensusDomainMismatch,
    PredecessorSnapshotMismatch,
    CutObservationCellMismatch {
''',
    1,
)

display_anchor = '''            Self::CensusDomainMismatch => {
                write!(
                    f,
                    "partition census was proven for a different analysis domain"
                )
            }
            Self::CutObservationCellMismatch { observation, cut } => write!(
'''
if text.count(display_anchor) != 1:
    raise SystemExit("expected exactly one PB-04c error display anchor")
text = text.replace(
    display_anchor,
    '''            Self::CensusDomainMismatch => {
                write!(
                    f,
                    "partition census was proven for a different analysis domain"
                )
            }
            Self::PredecessorSnapshotMismatch => write!(
                f,
                "incremental predecessor snapshot does not exactly match its census"
            ),
            Self::CutObservationCellMismatch { observation, cut } => write!(
''',
    1,
)

path.write_text(text)
print("PB-04c incremental recomputation transform applied")
