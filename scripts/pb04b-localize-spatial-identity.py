#!/usr/bin/env python3
from pathlib import Path

PATH = Path("crates/domains/symtropy-spatial-decomposition/src/lib.rs")
text = PATH.read_text()


def replace_exact(old: str, new: str, count: int = 1) -> None:
    global text
    actual = text.count(old)
    if actual != count:
        raise SystemExit(
            f"PB-04b locality transform expected {count} occurrence(s), found {actual}: {old!r}"
        )
    text = text.replace(old, new)


replace_exact(
    "id: fragment_id(&geometry, profile, domain, cell, side, Some(cut))?,",
    "id: fragment_id(profile, domain, cell, side, Some(cut))?,",
)
replace_exact(
    "id: fragment_id(&geometry, profile, domain, cell, FragmentSide::Whole, None)?,",
    "id: fragment_id(profile, domain, cell, FragmentSide::Whole, None)?,",
)
replace_exact(
    "fn fragment_id(\n    geometry: &RealizedGeometrySnapshotRef,\n    profile: &DecompositionProfile,",
    "fn fragment_id(\n    profile: &DecompositionProfile,",
)
replace_exact(
    "    hash.update(FRAGMENT_DOMAIN);\n    hash_exact(&mut hash, &geometry.0);\n    hash_profile(&mut hash, &profile.exact_ref());",
    "    hash.update(FRAGMENT_DOMAIN);\n    hash_profile(&mut hash, &profile.exact_ref());",
)
replace_exact(
    "            hash_plane(&mut hash, cut.plane);\n            hash_refs(&mut hash, &cut.barriers);\n            hash_refs(&mut hash, &cut.separators);",
    "            hash_plane(&mut hash, cut.plane);",
)
replace_exact(
    "    let id = interface_id(\n        geometry,\n        profile,\n        domain,\n        &kind,\n        &first,\n        &second,\n        &barriers,\n        &separators,\n    )?;",
    "    let id = interface_id(profile, domain, &kind, &first, &second)?;",
)
replace_exact(
    "fn interface_id(\n    geometry: &RealizedGeometrySnapshotRef,\n    profile: &DecompositionProfile,\n    domain: &AnalysisDomain,\n    kind: &GeometricInterfaceKind,\n    first: &SpatialRegionId,\n    second: &SpatialRegionId,\n    barriers: &[ExactSourceRef],\n    separators: &[ExactSourceRef],\n) -> Result<BoundaryInterfaceId, DecompositionError> {",
    "fn interface_id(\n    profile: &DecompositionProfile,\n    domain: &AnalysisDomain,\n    kind: &GeometricInterfaceKind,\n    first: &SpatialRegionId,\n    second: &SpatialRegionId,\n) -> Result<BoundaryInterfaceId, DecompositionError> {",
)
replace_exact(
    "    hash.update(INTERFACE_DOMAIN);\n    hash_exact(&mut hash, &geometry.0);\n    hash_profile(&mut hash, &profile.exact_ref());",
    "    hash.update(INTERFACE_DOMAIN);\n    hash_profile(&mut hash, &profile.exact_ref());",
)
replace_exact(
    "    hash_text(&mut hash, second.0.as_str());\n    hash_kind(&mut hash, kind);\n    hash_refs(&mut hash, barriers);\n    hash_refs(&mut hash, separators);",
    "    hash_text(&mut hash, second.0.as_str());\n    hash_kind(&mut hash, kind);",
)

PATH.write_text(text)
