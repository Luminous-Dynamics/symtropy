#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/domains/symtropy-spatial-decomposition/src/lib.rs")
text = path.read_text()


def replace_once(old: str, new: str, label: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    text = text.replace(old, new, 1)


replace_once(
    '''const PROFILE_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.profile.v1\\0";
const DOMAIN_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.domain.v1\\0";
const FRAGMENT_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.fragment.v1\\0";
const INTERFACE_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.interface.v1\\0";
const SNAPSHOT_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.snapshot.v1\\0";
''',
    '''const PROFILE_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.profile.v1\\0";
const DOMAIN_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.domain.v1\\0";
const FRAGMENT_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.fragment.v1\\0";
const INTERFACE_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.interface.v1\\0";
const SNAPSHOT_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.snapshot.v1\\0";
const LOCUS_SEMANTICS_DOMAIN: &[u8] = b"symtropy.spatial-decomposition.locus-semantics.v1\\0";
const PB04A_ADAPTER_PROFILE_DOMAIN: &[u8] =
    b"symtropy.spatial-decomposition.pb04a-adapter-profile.v1\\0";
const PB04A_BOUNDARY_SUBJECT_DOMAIN: &[u8] =
    b"symtropy.spatial-decomposition.pb04a-boundary-subject.v1\\0";
''',
    "identity domains",
)

replace_once(
    '''pub struct AnalysisDomainRef {
    pub domain_id: StableId,
    pub revision: u64,
    pub content_digest: String,
}
''',
    '''pub struct AnalysisDomainRef {
    pub domain_id: StableId,
    pub revision: u64,
    pub coordinate_frame_id: StableId,
    pub content_digest: String,
}
''',
    "analysis domain ref",
)

replace_once(
    '''pub struct AnalysisDomain {
    domain_id: StableId,
    revision: u64,
    origin: Point3i,
    dimensions: [u32; 3],
    digest: String,
}
''',
    '''pub struct AnalysisDomain {
    domain_id: StableId,
    revision: u64,
    coordinate_frame_id: StableId,
    origin: Point3i,
    dimensions: [u32; 3],
    digest: String,
}
''',
    "analysis domain fields",
)

replace_once(
    '''    pub fn new(
        domain_id: StableId,
        revision: u64,
        origin: Point3i,
        dimensions: [u32; 3],
    ) -> Result<Self, DecompositionError> {
        validate_id(&domain_id)?;
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
        let digest = domain_digest(&domain_id, revision, origin, dimensions);
        Ok(Self {
            domain_id,
            revision,
            origin,
            dimensions,
            digest,
        })
    }
''',
    '''    pub fn new(
        domain_id: StableId,
        revision: u64,
        origin: Point3i,
        dimensions: [u32; 3],
    ) -> Result<Self, DecompositionError> {
        let coordinate_frame_id = domain_id.clone();
        Self::new_in_frame(
            domain_id,
            revision,
            coordinate_frame_id,
            origin,
            dimensions,
        )
    }

    pub fn new_in_frame(
        domain_id: StableId,
        revision: u64,
        coordinate_frame_id: StableId,
        origin: Point3i,
        dimensions: [u32; 3],
    ) -> Result<Self, DecompositionError> {
        validate_id(&domain_id)?;
        validate_id(&coordinate_frame_id)?;
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
            &coordinate_frame_id,
            origin,
            dimensions,
        );
        Ok(Self {
            domain_id,
            revision,
            coordinate_frame_id,
            origin,
            dimensions,
            digest,
        })
    }
''',
    "analysis domain constructors",
)

replace_once(
    '''    pub fn exact_ref(&self) -> AnalysisDomainRef {
        AnalysisDomainRef {
            domain_id: self.domain_id.clone(),
            revision: self.revision,
            content_digest: self.digest.clone(),
        }
    }

    fn contains(&self, cell: CellCoord) -> bool {
''',
    '''    pub fn exact_ref(&self) -> AnalysisDomainRef {
        AnalysisDomainRef {
            domain_id: self.domain_id.clone(),
            revision: self.revision,
            coordinate_frame_id: self.coordinate_frame_id.clone(),
            content_digest: self.digest.clone(),
        }
    }

    pub fn coordinate_frame_id(&self) -> &StableId {
        &self.coordinate_frame_id
    }

    fn contains(&self, cell: CellCoord) -> bool {
''',
    "analysis domain exact ref",
)

replace_once(
    '''        let profile_source = ExactSourceRef::new(
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

        Ok(BoundarySnapshot::new(
            sid_digest("pb04b-boundary", &self.digest)?,
            0,
            self.digest.clone(),
            frame_ref,
            environment_ref,
            regions,
            interfaces,
            canonical_refs(
                vec![self.geometry.0.clone(), profile_source, domain_source],
                "boundary.sources",
            )?,
        )?)
''',
    '''        frame_ref.validate()?;
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
''',
    "PB-04a adapter context identity",
)

replace_once(
    '''fn fragment_id(
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    cell: CellCoord,
    side: FragmentSide,
    cut: Option<&CutEvidence>,
) -> Result<SpatialRegionId, DecompositionError> {
    let mut hash = Sha256::new();
    hash.update(FRAGMENT_DOMAIN);
    hash_profile(&mut hash, &profile.exact_ref());
    hash_analysis_domain(&mut hash, &domain.exact_ref());
    hash_cell(&mut hash, cell);
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
    hash_profile(&mut hash, &profile.exact_ref());
    hash_analysis_domain(&mut hash, &domain.exact_ref());
    hash_text(&mut hash, first.0.as_str());
    hash_text(&mut hash, second.0.as_str());
    hash_kind(&mut hash, kind);
    Ok(BoundaryInterfaceId::new(sid_digest(
        "pb04b-interface",
        &hex(&hash.finalize()),
    )?)?)
}
''',
    '''fn fragment_id(
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    cell: CellCoord,
    side: FragmentSide,
    cut: Option<&CutEvidence>,
) -> Result<SpatialRegionId, DecompositionError> {
    let mut hash = Sha256::new();
    hash.update(FRAGMENT_DOMAIN);
    hash_locus_semantics(&mut hash, profile);
    hash_text(&mut hash, domain.coordinate_frame_id.as_str());
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
    hash_text(&mut hash, domain.coordinate_frame_id.as_str());
    hash_text(&mut hash, first.0.as_str());
    hash_text(&mut hash, second.0.as_str());
    hash_kind_locus(&mut hash, kind, profile, domain)?;
    Ok(BoundaryInterfaceId::new(sid_digest(
        "pb04b-interface",
        &hex(&hash.finalize()),
    )?)?)
}
''',
    "local locus identity",
)

replace_once(
    '''fn domain_digest(id: &StableId, revision: u64, origin: Point3i, dimensions: [u32; 3]) -> String {
    let mut hash = Sha256::new();
    hash.update(DOMAIN_DOMAIN);
    hash_text(&mut hash, id.as_str());
    hash_u64(&mut hash, revision);
    for value in [origin.x, origin.y, origin.z] {
        hash_i64(&mut hash, value);
    }
    for value in dimensions {
        hash_u32(&mut hash, value);
    }
    hex(&hash.finalize())
}

fn hash_kind(hash: &mut Sha256, kind: &GeometricInterfaceKind) {
''',
    '''fn domain_digest(
    id: &StableId,
    revision: u64,
    coordinate_frame_id: &StableId,
    origin: Point3i,
    dimensions: [u32; 3],
) -> String {
    let mut hash = Sha256::new();
    hash.update(DOMAIN_DOMAIN);
    hash_text(&mut hash, id.as_str());
    hash_u64(&mut hash, revision);
    hash_text(&mut hash, coordinate_frame_id.as_str());
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
''',
    "local identity helpers",
)

replace_once(
    '''fn hash_analysis_domain(hash: &mut Sha256, value: &AnalysisDomainRef) {
    hash_text(hash, value.domain_id.as_str());
    hash_u64(hash, value.revision);
    hash_text(hash, &value.content_digest);
}
''',
    '''fn hash_analysis_domain(hash: &mut Sha256, value: &AnalysisDomainRef) {
    hash_text(hash, value.domain_id.as_str());
    hash_u64(hash, value.revision);
    hash_text(hash, value.coordinate_frame_id.as_str());
    hash_text(hash, &value.content_digest);
}
''',
    "analysis domain snapshot hash",
)

path.write_text(text)
print("PB-04b locus/context transform applied")
