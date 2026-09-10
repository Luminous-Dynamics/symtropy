#!/usr/bin/env python3
# Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Fourth fail-closed PB-04a hardening pass: seal identity-bearing state.

Run after the three prior PB-04a transforms. Canonically hashed objects must not
leave identity-bearing fields publicly mutable, and read-only projection does
not need to clone the entire boundary merely to assert the Rust type system.
"""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIB = ROOT / "crates/domains/symtropy-spatial-topology/src/lib.rs"
HOSTILE = ROOT / "crates/domains/symtropy-spatial-topology/tests/hostile_projection.rs"


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


s = LIB.read_text()
for marker in [
    "provider_content_digest: String",
    "pub struct TopologyProfileRef",
    "fn validate_snapshot_exact_ref_consistency(",
    "pub fn neighbors(\n        &self,\n        facet: TopologyFacet,",
]:
    if s.count(marker) != 1:
        raise SystemExit(
            f"refusing PB-04a sealing: expected one prior-pass marker {marker!r}"
        )

s = replace_once(
    s,
    "pub struct BoundarySnapshot {\n    pub schema_version: u32,\n    pub snapshot_id: StableId,\n    pub revision: u64,\n    provider_content_digest: String,\n    content_digest: String,\n    pub frame_ref: ExactSourceRef,\n    pub environment_ref: ExactSourceRef,",
    "pub struct BoundarySnapshot {\n    schema_version: u32,\n    snapshot_id: StableId,\n    revision: u64,\n    provider_content_digest: String,\n    content_digest: String,\n    frame_ref: ExactSourceRef,\n    environment_ref: ExactSourceRef,",
    "seal boundary identity fields",
)

s = replace_once(
    s,
    "    pub fn provider_content_digest(&self) -> &str {\n        &self.provider_content_digest\n    }",
    "    pub const fn schema_version(&self) -> u32 {\n        self.schema_version\n    }\n\n    pub fn snapshot_id(&self) -> &StableId {\n        &self.snapshot_id\n    }\n\n    pub const fn revision(&self) -> u64 {\n        self.revision\n    }\n\n    pub fn provider_content_digest(&self) -> &str {\n        &self.provider_content_digest\n    }",
    "boundary identity accessors",
)

s = replace_once(
    s,
    "    pub fn regions(&self) -> &[SpatialRegionSnapshot] {",
    "    pub fn frame_ref(&self) -> &ExactSourceRef {\n        &self.frame_ref\n    }\n\n    pub fn environment_ref(&self) -> &ExactSourceRef {\n        &self.environment_ref\n    }\n\n    pub fn regions(&self) -> &[SpatialRegionSnapshot] {",
    "boundary authority accessors",
)

s = replace_once(
    s,
    "pub struct TopologyProfile {\n    pub profile_id: StableId,\n    pub revision: u64,\n    content_digest: String,",
    "pub struct TopologyProfile {\n    profile_id: StableId,\n    revision: u64,\n    content_digest: String,",
    "seal profile identity fields",
)

s = replace_once(
    s,
    "    pub fn facets(&self) -> &[TopologyFacet] {\n        &self.facets\n    }",
    "    pub fn profile_id(&self) -> &StableId {\n        &self.profile_id\n    }\n\n    pub const fn revision(&self) -> u64 {\n        self.revision\n    }\n\n    pub fn content_digest(&self) -> &str {\n        &self.content_digest\n    }\n\n    pub fn facets(&self) -> &[TopologyFacet] {\n        &self.facets\n    }",
    "profile identity accessors",
)

s = replace_once(
    s,
    "pub struct TopologySnapshot {\n    pub schema_version: u32,\n    pub boundary_ref: BoundarySnapshotRef,\n    pub profile_ref: TopologyProfileRef,",
    "pub struct TopologySnapshot {\n    schema_version: u32,\n    boundary_ref: BoundarySnapshotRef,\n    profile_ref: TopologyProfileRef,",
    "seal projection identity fields",
)

s = replace_once(
    s,
    "        profile.validate_canonical()?;\n        let before = boundary.clone();\n        let region_ids = boundary",
    "        profile.validate_canonical()?;\n        let region_ids = boundary",
    "remove whole-boundary debug clone",
)

s = replace_once(
    s,
    "\n        debug_assert_eq!(&before, boundary);\n        Ok(Self {",
    "\n        Ok(Self {",
    "remove redundant non-mutation assertion",
)

s = replace_once(
    s,
    "    pub fn region_ids(&self) -> &[SpatialRegionId] {",
    "    pub const fn schema_version(&self) -> u32 {\n        self.schema_version\n    }\n\n    pub fn boundary_ref(&self) -> &BoundarySnapshotRef {\n        &self.boundary_ref\n    }\n\n    pub fn profile_ref(&self) -> &TopologyProfileRef {\n        &self.profile_ref\n    }\n\n    pub fn region_ids(&self) -> &[SpatialRegionId] {",
    "projection identity accessors",
)

LIB.write_text(s)

hostile = HOSTILE.read_text()
for marker in [
    "derived_topology_binds_exact_profile_content",
    "unknown_region_is_not_silently_treated_as_isolated",
]:
    if hostile.count(marker) != 1:
        raise SystemExit(f"refusing PB-04a sealing: missing hostile fixture {marker!r}")

hostile = replace_once(
    hostile,
    "    assert_eq!(topology.profile_ref, profile.exact_ref());",
    "    assert_eq!(topology.profile_ref(), &profile.exact_ref());",
    "external profile-ref accessor",
)

hostile += r'''

#[test]
fn sealed_identity_accessors_preserve_exact_snapshot_subject() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:sealed-subject")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![InterfaceFacetState::new(
            TopologyFacet::Occupancy,
            FacetRelation::Disconnected,
        )
        .unwrap()],
        vec![],
    )
    .unwrap();
    let boundary = boundary(interface);
    let exact = boundary.exact_ref();

    assert_eq!(boundary.snapshot_id(), &exact.snapshot_id);
    assert_eq!(boundary.revision(), exact.revision);
    assert_eq!(boundary.content_digest(), exact.content_digest);
    assert_eq!(boundary.schema_version(), 1);
}
'''
HOSTILE.write_text(hostile)

print("PB-04a identity-bearing state sealed and redundant boundary clone removed")
