#!/usr/bin/env python3
# Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Fail-closed PB-04a content-identity hardening.

This transformer is deliberately bound to the reviewed source blobs. It refuses
rather than guessing if the PB-04a preimage changes before hosted execution.
"""

from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
LIB = ROOT / "crates/domains/symtropy-spatial-topology/src/lib.rs"
CARGO = ROOT / "crates/domains/symtropy-spatial-topology/Cargo.toml"
HOSTILE = ROOT / "crates/domains/symtropy-spatial-topology/tests/hostile_projection.rs"

EXPECTED = {
    LIB: "01ac9cfb159fe5c87b41e386a2b70c6edb6f6ae9",
    CARGO: "66d2d48ef30208bf1389983a5e3dd5708d5eab3f",
    HOSTILE: "859392660ab0a722d76892ee317aceb2f2522be4",
}


def git_blob(path: Path) -> str:
    return subprocess.check_output(
        ["git", "hash-object", str(path.relative_to(ROOT))], cwd=ROOT, text=True
    ).strip()


for path, expected in EXPECTED.items():
    actual = git_blob(path)
    if actual != expected:
        raise SystemExit(
            f"refusing PB-04a identity hardening: {path.relative_to(ROOT)} "
            f"blob drifted: expected {expected}, got {actual}"
        )


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


s = LIB.read_text()

s = replace_once(
    s,
    "use std::{collections::BTreeSet, error::Error, fmt};\nuse symtropy_game_state::StableId;",
    "use sha2::{Digest, Sha256};\nuse std::{collections::BTreeSet, error::Error, fmt};\nuse symtropy_game_state::StableId;",
    "sha2 import",
)

s = replace_once(
    s,
    "pub const MAX_DIGEST_BYTES: usize = 256;",
    "pub const MAX_DIGEST_BYTES: usize = 256;\n\nconst BOUNDARY_DIGEST_DOMAIN: &[u8] = b\"symtropy.spatial-topology.boundary.v1\\0\";\nconst PROFILE_DIGEST_DOMAIN: &[u8] = b\"symtropy.spatial-topology.profile.v1\\0\";",
    "digest domains",
)

s = replace_once(
    s,
    "        first_region: SpatialRegionId,\n        second_region: SpatialRegionId,",
    "        mut first_region: SpatialRegionId,\n        mut second_region: SpatialRegionId,",
    "canonical interface endpoint params",
)

s = replace_once(
    s,
    "        first_region.validate()?;\n        second_region.validate()?;\n        validate_len(",
    "        first_region.validate()?;\n        second_region.validate()?;\n        if second_region < first_region {\n            std::mem::swap(&mut first_region, &mut second_region);\n        }\n        validate_len(",
    "canonical interface endpoint ordering",
)

s = replace_once(
    s,
    "    pub content_digest: String,\n    pub frame_ref: ExactSourceRef,",
    "    provider_content_digest: String,\n    content_digest: String,\n    pub frame_ref: ExactSourceRef,",
    "boundary digest fields",
)

s = replace_once(
    s,
    "        content_digest: impl Into<String>,\n        frame_ref: ExactSourceRef,",
    "        provider_content_digest: impl Into<String>,\n        frame_ref: ExactSourceRef,",
    "boundary constructor digest parameter",
)

s = replace_once(
    s,
    "        regions.sort_by(|left, right| left.id.cmp(&right.id));\n        interfaces.sort_by(|left, right| left.id.cmp(&right.id));\n        source_refs.sort();\n        let value = Self {",
    "        regions.sort_by(|left, right| left.id.cmp(&right.id));\n        interfaces.sort_by(|left, right| left.id.cmp(&right.id));\n        source_refs.sort();\n        let provider_content_digest = provider_content_digest.into();\n        validate_digest(&provider_content_digest)?;\n        let mut value = Self {",
    "boundary constructor provider digest validation",
)

s = replace_once(
    s,
    "            revision,\n            content_digest: content_digest.into(),\n            frame_ref,",
    "            revision,\n            provider_content_digest,\n            content_digest: String::new(),\n            frame_ref,",
    "boundary constructor digest fields",
)

s = replace_once(
    s,
    "            source_refs,\n        };\n        value.validate_canonical()?;\n        Ok(value)\n    }\n\n    pub fn regions(&self)",
    "            source_refs,\n        };\n        value.content_digest = boundary_content_digest(&value);\n        value.validate_canonical()?;\n        Ok(value)\n    }\n\n    pub fn provider_content_digest(&self) -> &str {\n        &self.provider_content_digest\n    }\n\n    pub fn content_digest(&self) -> &str {\n        &self.content_digest\n    }\n\n    pub fn regions(&self)",
    "boundary constructor computed digest",
)

s = replace_once(
    s,
    "        validate_id(&self.snapshot_id)?;\n        validate_digest(&self.content_digest)?;\n        self.frame_ref.validate()?;",
    "        validate_id(&self.snapshot_id)?;\n        validate_digest(&self.provider_content_digest)?;\n        validate_digest(&self.content_digest)?;\n        self.frame_ref.validate()?;",
    "boundary validates both digests",
)

s = replace_once(
    s,
    "        for pair in self.interfaces.windows(2) {\n            if pair[0].id > pair[1].id {\n                return Err(TopologyError::NonCanonicalOrder(\"boundary.interfaces\"));\n            }\n            if pair[0].id == pair[1].id {\n                return Err(TopologyError::DuplicateInterface(pair[0].id.clone()));\n            }\n        }\n        Ok(())\n    }\n}",
    "        for pair in self.interfaces.windows(2) {\n            if pair[0].id > pair[1].id {\n                return Err(TopologyError::NonCanonicalOrder(\"boundary.interfaces\"));\n            }\n            if pair[0].id == pair[1].id {\n                return Err(TopologyError::DuplicateInterface(pair[0].id.clone()));\n            }\n        }\n        let expected_digest = boundary_content_digest(self);\n        if self.content_digest != expected_digest {\n            return Err(TopologyError::DigestMismatch {\n                subject: \"boundary snapshot\",\n            });\n        }\n        Ok(())\n    }\n}",
    "boundary digest verification",
)

s = replace_once(
    s,
    "pub struct TopologyProfile {\n    pub profile_id: StableId,\n    pub revision: u64,\n    facets: Vec<TopologyFacet>,\n}",
    "pub struct TopologyProfile {\n    pub profile_id: StableId,\n    pub revision: u64,\n    content_digest: String,\n    facets: Vec<TopologyFacet>,\n}",
    "profile digest field",
)

s = replace_once(
    s,
    "        Ok(Self {\n            profile_id,\n            revision,\n            facets,\n        })",
    "        let content_digest = profile_content_digest(&profile_id, revision, &facets);\n        Ok(Self {\n            profile_id,\n            revision,\n            content_digest,\n            facets,\n        })",
    "profile computed digest",
)

s = replace_once(
    s,
    "    pub fn facets(&self) -> &[TopologyFacet] {\n        &self.facets\n    }\n\n    fn validate_canonical(&self) -> Result<(), TopologyError> {",
    "    pub fn facets(&self) -> &[TopologyFacet] {\n        &self.facets\n    }\n\n    pub fn exact_ref(&self) -> TopologyProfileRef {\n        TopologyProfileRef {\n            profile_id: self.profile_id.clone(),\n            revision: self.revision,\n            content_digest: self.content_digest.clone(),\n        }\n    }\n\n    fn validate_canonical(&self) -> Result<(), TopologyError> {",
    "profile exact ref accessor",
)

s = replace_once(
    s,
    "        validate_id(&self.profile_id)?;\n        if self.facets.is_empty() {",
    "        validate_id(&self.profile_id)?;\n        validate_digest(&self.content_digest)?;\n        if self.facets.is_empty() {",
    "profile digest validation",
)

s = replace_once(
    s,
    "        for pair in self.facets.windows(2) {\n            if pair[0] >= pair[1] {\n                return Err(TopologyError::NonCanonicalOrder(\"topology_profile.facets\"));\n            }\n        }\n        Ok(())\n    }\n}",
    "        for pair in self.facets.windows(2) {\n            if pair[0] >= pair[1] {\n                return Err(TopologyError::NonCanonicalOrder(\"topology_profile.facets\"));\n            }\n        }\n        if self.content_digest != profile_content_digest(&self.profile_id, self.revision, &self.facets) {\n            return Err(TopologyError::DigestMismatch {\n                subject: \"topology profile\",\n            });\n        }\n        Ok(())\n    }\n}\n\n#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]\npub struct TopologyProfileRef {\n    pub profile_id: StableId,\n    pub revision: u64,\n    pub content_digest: String,\n}\n\nimpl TopologyProfileRef {\n    pub fn validate(&self) -> Result<(), TopologyError> {\n        validate_id(&self.profile_id)?;\n        validate_digest(&self.content_digest)\n    }\n}",
    "profile digest verification and exact ref type",
)

s = replace_once(
    s,
    "    pub boundary_ref: BoundarySnapshotRef,\n    pub profile_id: StableId,\n    pub profile_revision: u64,\n    region_ids: Vec<SpatialRegionId>,",
    "    pub boundary_ref: BoundarySnapshotRef,\n    pub profile_ref: TopologyProfileRef,\n    region_ids: Vec<SpatialRegionId>,",
    "topology exact profile ref",
)

s = replace_once(
    s,
    "            boundary_ref: boundary.exact_ref(),\n            profile_id: profile.profile_id.clone(),\n            profile_revision: profile.revision,\n            region_ids,",
    "            boundary_ref: boundary.exact_ref(),\n            profile_ref: profile.exact_ref(),\n            region_ids,",
    "topology derive exact profile ref",
)

s = replace_once(
    s,
    "    UnsupportedSchema(u32),\n    RegionsRequired,",
    "    UnsupportedSchema(u32),\n    DigestMismatch {\n        subject: &'static str,\n    },\n    RegionsRequired,",
    "digest mismatch error",
)

s = replace_once(
    s,
    "            Self::UnsupportedSchema(version) => {\n                write!(formatter, \"unsupported spatial topology schema {version}\")\n            }\n            Self::RegionsRequired =>",
    "            Self::UnsupportedSchema(version) => {\n                write!(formatter, \"unsupported spatial topology schema {version}\")\n            }\n            Self::DigestMismatch { subject } => {\n                write!(formatter, \"{subject} canonical content digest mismatch\")\n            }\n            Self::RegionsRequired =>",
    "digest mismatch display",
)

HASH_HELPERS = r'''
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

'''

s = replace_once(
    s,
    "fn validate_id(id: &StableId) -> Result<(), TopologyError> {",
    HASH_HELPERS + "fn validate_id(id: &StableId) -> Result<(), TopologyError> {",
    "canonical hashing helpers",
)

LIB.write_text(s)

cargo = CARGO.read_text()
cargo = replace_once(
    cargo,
    "[dependencies]\nsymtropy-game-state = { path = \"../symtropy-game-state\" }\n",
    "[dependencies]\nsha2 = \"0.10\"\nsymtropy-game-state = { path = \"../symtropy-game-state\" }\n",
    "sha2 dependency",
)
CARGO.write_text(cargo)

hostile = HOSTILE.read_text()
hostile += r'''

#[test]
fn same_claimed_provider_identity_cannot_launder_different_boundary_bodies() {
    let make = |relation| {
        boundary(
            BoundaryInterfaceSnapshot::new(
                BoundaryInterfaceId::new(id("interface:body-bound")).unwrap(),
                region("region:a"),
                region("region:b"),
                vec![InterfaceFacetState::new(TopologyFacet::Occupancy, relation).unwrap()],
                vec![source("device:body-bound", 1, "same-source")],
            )
            .unwrap(),
        )
    };

    let blocked = make(FacetRelation::Disconnected);
    let connected = make(FacetRelation::Connected);
    assert_eq!(blocked.provider_content_digest(), connected.provider_content_digest());
    assert_ne!(blocked.exact_ref(), connected.exact_ref());
}

#[test]
fn same_profile_id_revision_cannot_launder_different_facet_sets() {
    let occupancy = TopologyProfile::new(
        id("topology-profile:same"),
        7,
        vec![TopologyFacet::Occupancy],
    )
    .unwrap();
    let pressure = TopologyProfile::new(
        id("topology-profile:same"),
        7,
        vec![TopologyFacet::AirPressure],
    )
    .unwrap();

    assert_ne!(occupancy.exact_ref(), pressure.exact_ref());
}

#[test]
fn undirected_interface_endpoint_order_is_canonical() {
    let facet = || {
        vec![InterfaceFacetState::new(
            TopologyFacet::Occupancy,
            FacetRelation::Connected,
        )
        .unwrap()]
    };
    let refs = || vec![source("device:opening", 1, "same")];

    let forward = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:opening")).unwrap(),
        region("region:a"),
        region("region:b"),
        facet(),
        refs(),
    )
    .unwrap();
    let reverse = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:opening")).unwrap(),
        region("region:b"),
        region("region:a"),
        facet(),
        refs(),
    )
    .unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(boundary(forward).exact_ref(), boundary(reverse).exact_ref());
}

#[test]
fn derived_topology_binds_exact_profile_content() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:profile-bound")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![InterfaceFacetState::new(
            TopologyFacet::Occupancy,
            FacetRelation::Connected,
        )
        .unwrap()],
        vec![],
    )
    .unwrap();
    let profile = occupancy_profile();
    let topology = TopologySnapshot::derive(&boundary(interface), &profile).unwrap();
    assert_eq!(topology.profile_ref, profile.exact_ref());
}
'''
HOSTILE.write_text(hostile)

print("PB-04a content-identity hardening applied to exact reviewed preimage")
