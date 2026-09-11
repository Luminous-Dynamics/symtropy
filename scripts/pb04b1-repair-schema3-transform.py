#!/usr/bin/env python3
from pathlib import Path

lib_path = Path("crates/domains/symtropy-spatial-decomposition/src/lib.rs")
identity_path = Path("crates/domains/symtropy-spatial-decomposition/tests/local_identity.rs")
coverage_path = Path("crates/domains/symtropy-spatial-decomposition/tests/complete_local_coverage.rs")
lib = lib_path.read_text()
identity = identity_path.read_text()
coverage = coverage_path.read_text()

old_deprecated = '''    #[deprecated(note = "schema 3 requires an explicit complete LocalPartitionCensus")]
    pub fn derive(
'''
new_stub = '''    /// Compatibility stub retained so old sparse callers fail closed at runtime.
    /// Schema 3 requires `derive_from_census` with complete per-cell coverage.
    pub fn derive(
'''
if lib.count(old_deprecated) != 1:
    raise SystemExit("expected exactly one generated deprecated sparse derive stub")
lib = lib.replace(old_deprecated, new_stub, 1)

old_test = '''#[test]
fn successor_uses_schema_v2() {
    let snapshot = derive(
        geometry(1, "schema2-geometry"),
        &profile(1),
        doorway_cut(vec![barrier(1, "schema2-door")]),
    );
    assert_eq!(snapshot.schema_version(), 2);
}
'''
new_test = '''#[test]
fn successor_uses_schema_v3() {
    let snapshot = derive(
        geometry(1, "schema3-geometry"),
        &profile(1),
        doorway_cut(vec![barrier(1, "schema3-door")]),
    );
    assert_eq!(snapshot.schema_version(), 3);
}
'''
if identity.count(old_test) != 1:
    raise SystemExit("expected exactly one inherited schema2 identity test")
identity = identity.replace(old_test, new_test, 1)

old_fixture = '''    let left = clear(CellCoord::new(0, 0, 0), "coverage-left");
    let right = observed_cut(CellCoord::new(1, 0, 0), "coverage-right");
'''
new_fixture = '''    let left = observed_cut(CellCoord::new(0, 0, 0), "coverage-left");
    let right = clear(CellCoord::new(1, 0, 0), "coverage-right");
'''
if coverage.count(old_fixture) != 1:
    raise SystemExit("expected exactly one invalid shuffled-census fixture")
coverage = coverage.replace(old_fixture, new_fixture, 1)

lib_path.write_text(lib)
identity_path.write_text(identity)
coverage_path.write_text(coverage)
print("PB-04b1 schema3 generated-contract repair applied")
