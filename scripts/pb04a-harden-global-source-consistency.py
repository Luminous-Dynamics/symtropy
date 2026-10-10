#!/usr/bin/env python3
# Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Second fail-closed PB-04a identity pass.

This script is intentionally run only after pb04a-harden-content-identity.py has
validated the original reviewed blobs and applied its deterministic transform.
It closes cross-scope exact-source equivocation without changing source-owner
semantics: identical exact refs may support several derived facts, but competing
digests for one authority/subject/revision inside a snapshot fail closed.
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

# Prove this is the output shape of the first transformer, not the original file
# or an unrelated future revision.
required_markers = [
    "const BOUNDARY_DIGEST_DOMAIN",
    "provider_content_digest: String",
    "pub struct TopologyProfileRef",
    "pub profile_ref: TopologyProfileRef",
    "fn boundary_content_digest(snapshot: &BoundarySnapshot)",
]
for marker in required_markers:
    if s.count(marker) != 1:
        raise SystemExit(
            f"refusing PB-04a global-source hardening: expected one marker {marker!r}"
        )

s = replace_once(
    s,
    "pub const MAX_DIGEST_BYTES: usize = 256;\n\nconst BOUNDARY_DIGEST_DOMAIN",
    "pub const MAX_DIGEST_BYTES: usize = 256;\n/// Aggregate exact-source refs admitted by one boundary snapshot. Large worlds\n/// compose bounded snapshots rather than allowing nested per-region/interface\n/// limits to multiply into an unbounded validation allocation.\npub const MAX_BOUNDARY_EXACT_REFS: usize = 262_144;\n\nconst BOUNDARY_DIGEST_DOMAIN",
    "aggregate exact-source bound",
)

s = replace_once(
    s,
    "        let expected_digest = boundary_content_digest(self);\n        if self.content_digest != expected_digest {",
    "        validate_snapshot_exact_ref_consistency(self)?;\n\n        let expected_digest = boundary_content_digest(self);\n        if self.content_digest != expected_digest {",
    "snapshot-wide exact-ref consistency call",
)

GLOBAL_HELPER = r'''
fn validate_snapshot_exact_ref_consistency(
    snapshot: &BoundarySnapshot,
) -> Result<(), TopologyError> {
    let field = "boundary.all_exact_refs";
    let mut total = 2usize;

    let mut add = |count: usize| -> Result<(), TopologyError> {
        total = total.checked_add(count).ok_or(TopologyError::BoundExceeded {
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

'''

s = replace_once(
    s,
    "fn boundary_content_digest(snapshot: &BoundarySnapshot) -> String {",
    GLOBAL_HELPER + "fn boundary_content_digest(snapshot: &BoundarySnapshot) -> String {",
    "snapshot-wide exact-ref consistency helper",
)

LIB.write_text(s)

hostile = HOSTILE.read_text()
if hostile.count("same_claimed_provider_identity_cannot_launder_different_boundary_bodies") != 1:
    raise SystemExit("first identity transformer hostile fixtures are not present exactly once")

hostile += r'''

#[test]
fn cross_scope_exact_source_equivocation_fails_closed() {
    let shared_a = source("source:shared", 9, "digest-a");
    let shared_b = source("source:shared", 9, "digest-b");
    let regions = vec![
        SpatialRegionSnapshot::new(region("region:a"), vec![shared_a]).unwrap(),
        SpatialRegionSnapshot::new(region("region:b"), vec![]).unwrap(),
    ];
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:cross-scope")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![InterfaceFacetState::new(
            TopologyFacet::Occupancy,
            FacetRelation::Connected,
        )
        .unwrap()],
        vec![shared_b],
    )
    .unwrap();

    let result = BoundarySnapshot::new(
        id("boundary:cross-scope-conflict"),
        1,
        "provider-digest",
        source("frame:cross-scope", 1, "frame"),
        source("environment:cross-scope", 1, "environment"),
        regions,
        vec![interface],
        vec![],
    );

    assert!(matches!(
        result,
        Err(TopologyError::ConflictingExactRef {
            field: "boundary.all_exact_refs",
            ..
        })
    ));
}

#[test]
fn identical_exact_source_may_support_multiple_derived_facts() {
    let shared = source("source:shared", 9, "same-digest");
    let regions = vec![
        SpatialRegionSnapshot::new(region("region:a"), vec![shared.clone()]).unwrap(),
        SpatialRegionSnapshot::new(region("region:b"), vec![]).unwrap(),
    ];
    let interface = BoundaryInterfaceSnapshot::new(
        BoundaryInterfaceId::new(id("interface:shared-source")).unwrap(),
        region("region:a"),
        region("region:b"),
        vec![InterfaceFacetState::new(
            TopologyFacet::Visibility,
            FacetRelation::Connected,
        )
        .unwrap()],
        vec![shared],
    )
    .unwrap();

    let result = BoundarySnapshot::new(
        id("boundary:shared-source"),
        1,
        "provider-digest",
        source("frame:shared-source", 1, "frame"),
        source("environment:shared-source", 1, "environment"),
        regions,
        vec![interface],
        vec![],
    );
    assert!(result.is_ok());
}
'''
HOSTILE.write_text(hostile)

print("PB-04a snapshot-wide exact-source consistency hardening applied")
