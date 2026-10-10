#!/usr/bin/env python3
# Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Third fail-closed PB-04a hardening pass: checked public queries.

Run after the content-identity and snapshot-wide source-consistency transforms.
The public API must distinguish an unknown region / omitted facet from a valid
projected region that simply has no participating neighbors. It must also keep
opaque qualified relations distinct from unconditional connectivity: definite
neighbors include only `Connected`, while candidate neighbors may additionally
include `QualifiedClass` for explicit downstream policy/solver interpretation.
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
    "pub struct TopologyProfileRef",
    "pub profile_ref: TopologyProfileRef",
    "fn validate_snapshot_exact_ref_consistency(",
    "pub const MAX_BOUNDARY_EXACT_REFS: usize = 262_144;",
]:
    if s.count(marker) != 1:
        raise SystemExit(
            f"refusing PB-04a query hardening: expected one prior-pass marker {marker!r}"
        )

s = replace_once(
    s,
    "    pub const fn participates_in_graph(&self) -> bool {\n        matches!(self, Self::Connected | Self::QualifiedClass { .. })\n    }",
    "    /// True only for unconditional connectivity asserted by the owning projection.\n    pub const fn is_definitely_connected(&self) -> bool {\n        matches!(self, Self::Connected)\n    }\n\n    /// True when connectivity semantics require interpretation of an opaque class.\n    pub const fn is_qualified(&self) -> bool {\n        matches!(self, Self::QualifiedClass { .. })\n    }\n\n    const fn participates_in_candidate_graph(&self) -> bool {\n        matches!(self, Self::Connected | Self::QualifiedClass { .. })\n    }",
    "separate definite and candidate connectivity",
)

s = replace_once(
    s,
    "    pub fn neighbors(&self, region: &SpatialRegionId) -> Vec<SpatialRegionId> {",
    "    fn neighbors(&self, region: &SpatialRegionId) -> Vec<SpatialRegionId> {",
    "make raw graph traversal private",
)

s = replace_once(
    s,
    "            if !interface.relation.participates_in_graph() {",
    "            if !interface.relation.participates_in_candidate_graph() {",
    "rename candidate graph predicate",
)

s = replace_once(
    s,
    "    pub fn graph(&self, facet: TopologyFacet) -> Option<&FacetGraph> {\n        self.graphs\n            .binary_search_by_key(&facet, |graph| graph.facet)\n            .ok()\n            .map(|index| &self.graphs[index])\n    }\n}",
    "    pub fn graph(&self, facet: TopologyFacet) -> Option<&FacetGraph> {\n        self.graphs\n            .binary_search_by_key(&facet, |graph| graph.facet)\n            .ok()\n            .map(|index| &self.graphs[index])\n    }\n\n    /// Checked universal adjacency query. Only an unqualified `Connected`\n    /// relation is a definite edge. Opaque `QualifiedClass` relations require\n    /// explicit downstream interpretation and therefore do not appear here.\n    pub fn definite_neighbors(\n        &self,\n        facet: TopologyFacet,\n        region: &SpatialRegionId,\n    ) -> Result<Vec<SpatialRegionId>, TopologyError> {\n        let graph = self.checked_graph(facet, region)?;\n        let mut result = BTreeSet::new();\n        for interface in graph.interfaces() {\n            if !interface.relation.is_definitely_connected() {\n                continue;\n            }\n            if &interface.first_region == region {\n                result.insert(interface.second_region.clone());\n            } else if &interface.second_region == region {\n                result.insert(interface.first_region.clone());\n            }\n        }\n        Ok(result.into_iter().collect())\n    }\n\n    /// Checked candidate adjacency query. Includes `QualifiedClass` relations\n    /// only as candidate edges; callers must inspect the relation/class and\n    /// apply their owning capability, policy, or solver semantics before using\n    /// such an edge as traversable/connected truth.\n    pub fn candidate_neighbors(\n        &self,\n        facet: TopologyFacet,\n        region: &SpatialRegionId,\n    ) -> Result<Vec<SpatialRegionId>, TopologyError> {\n        let graph = self.checked_graph(facet, region)?;\n        Ok(graph.neighbors(region))\n    }\n\n    fn checked_graph(\n        &self,\n        facet: TopologyFacet,\n        region: &SpatialRegionId,\n    ) -> Result<&FacetGraph, TopologyError> {\n        if self.region_ids.binary_search(region).is_err() {\n            return Err(TopologyError::UnknownProjectedRegion(region.clone()));\n        }\n        self.graph(facet)\n            .ok_or(TopologyError::FacetNotProjected(facet))\n    }\n}",
    "checked definite/candidate topology queries",
)

s = replace_once(
    s,
    "    ProfileFacetsRequired,\n    ExplicitUnspecifiedFacet(TopologyFacet),",
    "    ProfileFacetsRequired,\n    FacetNotProjected(TopologyFacet),\n    UnknownProjectedRegion(SpatialRegionId),\n    ExplicitUnspecifiedFacet(TopologyFacet),",
    "query errors",
)

s = replace_once(
    s,
    "            Self::ProfileFacetsRequired => write!(formatter, \"topology profile requires at least one facet\"),\n            Self::ExplicitUnspecifiedFacet(facet) => write!(",
    "            Self::ProfileFacetsRequired => write!(formatter, \"topology profile requires at least one facet\"),\n            Self::FacetNotProjected(facet) => {\n                write!(formatter, \"topology facet {facet:?} was not projected by this profile\")\n            }\n            Self::UnknownProjectedRegion(region) => {\n                write!(formatter, \"unknown projected spatial region {}\", region.0)\n            }\n            Self::ExplicitUnspecifiedFacet(facet) => write!(",
    "query error display",
)

LIB.write_text(s)

hostile = HOSTILE.read_text()
for marker in [
    "same_claimed_provider_identity_cannot_launder_different_boundary_bodies",
    "cross_scope_exact_source_equivocation_fails_closed",
]:
    if hostile.count(marker) != 1:
        raise SystemExit(
            f"refusing PB-04a query hardening: missing prior hostile fixture {marker!r}"
        )

old = '''    assert_eq!(
        projection.graph(TopologyFacet::Occupancy).unwrap().neighbors(&region("region:a")),
        vec![region("region:b")]
    );'''
new = '''    assert_eq!(
        projection
            .candidate_neighbors(TopologyFacet::Occupancy, &region("region:a"))
            .unwrap(),
        vec![region("region:b")]
    );'''
hostile = replace_once(hostile, old, new, "external qualified-candidate query")

hostile += r'''

#[test]
fn qualified_relation_is_candidate_but_not_definite_connectivity() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:qualified-only")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![InterfaceFacetState::new(
            TopologyFacet::Occupancy,
            FacetRelation::QualifiedClass {
                class_id: id("relation:passable-under-profile"),
            },
        )
        .unwrap()],
        vec![],
    )
    .unwrap();
    let projection = TopologySnapshot::derive(&boundary(interface), &occupancy_profile()).unwrap();

    assert!(projection
        .definite_neighbors(TopologyFacet::Occupancy, &region("region:a"))
        .unwrap()
        .is_empty());
    assert_eq!(
        projection
            .candidate_neighbors(TopologyFacet::Occupancy, &region("region:a"))
            .unwrap(),
        vec![region("region:b")]
    );
}

#[test]
fn unconditional_connected_relation_is_both_definite_and_candidate() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:definite" )).unwrap(),
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
    let projection = TopologySnapshot::derive(&boundary(interface), &occupancy_profile()).unwrap();

    let expected = vec![region("region:b")];
    assert_eq!(
        projection
            .definite_neighbors(TopologyFacet::Occupancy, &region("region:a"))
            .unwrap(),
        expected
    );
    assert_eq!(
        projection
            .candidate_neighbors(TopologyFacet::Occupancy, &region("region:a"))
            .unwrap(),
        vec![region("region:b")]
    );
}

#[test]
fn unknown_region_is_not_silently_treated_as_isolated() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:known-regions")).unwrap(),
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
    let projection = TopologySnapshot::derive(&boundary(interface), &occupancy_profile()).unwrap();

    assert!(matches!(
        projection.definite_neighbors(TopologyFacet::Occupancy, &region("region:missing")),
        Err(TopologyError::UnknownProjectedRegion(_))
    ));
}

#[test]
fn facet_omitted_by_profile_is_not_silently_treated_as_disconnected() {
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:facet-profile")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![
            InterfaceFacetState::new(TopologyFacet::Occupancy, FacetRelation::Disconnected)
                .unwrap(),
            InterfaceFacetState::new(TopologyFacet::Visibility, FacetRelation::Connected).unwrap(),
        ],
        vec![],
    )
    .unwrap();
    let projection = TopologySnapshot::derive(&boundary(interface), &occupancy_profile()).unwrap();

    assert!(matches!(
        projection.definite_neighbors(TopologyFacet::Visibility, &region("region:a")),
        Err(TopologyError::FacetNotProjected(TopologyFacet::Visibility))
    ));
}
'''
HOSTILE.write_text(hostile)

print("PB-04a checked definite/candidate public query semantics applied")
