# Engineering Scenario Compiler v0.1

A standalone Rust tool that compiles the existing NACA0012 engineering scenario TOML into a deterministic typed IR and compile-obligation report. The dependency graph is committed in `Cargo.lock` and CI uses `--locked`.

## Run

From this directory:

    cargo test --locked
    cargo bench --bench compile_latency --locked
    cargo run --locked -- ../../../benchmarks/aero/naca0012-inc-rans/case.toml
    cargo run --locked -- ../../../benchmarks/aero/naca0012-inc-rans/case.toml --solver-config ./upstream/config.cfg --mesh ./upstream/mesh.su2

Exit codes: 0 = PlanReady, 2 = Invalid, 3 = Blocked, 4 = Unsupported, 64 = usage, 66 = source read error, 70 = report serialization error.

## Deliberate scope

- Offline parser and compiler only; it never fetches URLs or launches a solver. Optional `--solver-config` and `--mesh` paths are explicit caller-supplied local files, read under role-specific byte caps; no implicit includes or plugin loading.
- Strict Serde structs reject unknown keys; TOML parser rejects syntax and duplicate-key errors.
- Typed quantity records preserve dimension and unit separately from the numeric value; incidence-angle/velocity vectors and Reynolds number derived from density, speed, chord and viscosity are cross-checked.
- The parser and parser implementation are both pinned: toml_edit 0.23.10+spec-1.0.0 and toml_parser 1.0.5+spec-1.0.0. A TOML 1.1-only escape negative control is rejected, and the parser profile is included in semantic identity.
- Supplied config/mesh bytes are checked against their Git blob SHA-1 object IDs (`blob <length> NUL <bytes>`); separate SHA-256 identities are emitted in a role-sorted artifact manifest. Duplicate roles, oversized bytes and mismatches fail closed.
- Source SHA-256 is computed over exact input bytes. Semantic IR SHA-256 is computed over a versioned fixed-order Rust structure plus compiler and registry revisions.
- A separate domain-separated `compilation_subject_sha256` combines exact source SHA-256, semantic IR SHA-256 and the sorted verified-artifact manifest SHA-256. The three constituent identities remain individually visible, so callers can distinguish source provenance, semantic meaning and supplied artifact bytes.
- A frozen NACA0012 model profile is recognized, but this fixture correctly compiles to BLOCKED: a Git blob object ID is not a SHA-256 digest of the retrieved bytes; exact config/mesh bytes, reference data digests, and an identified solver executable/build are absent.

## Claim ceiling

A BLOCKED report is a successful compile of a structurally and dimensionally valid scenario with unresolved execution obligations. It is not a simulation plan admission, numerical result, model validation, evidence attestation, or physical claim.

The semantic identity is deterministic within this compiler/schema/registry version and ordered IR definition. Cross-language canonicalization and digest golden vectors are still a separate qualification requirement.


## Compiler latency benchmark

The dependency-free Rust benchmark measures source parsing, typed validation, normalized IR construction and digest work at 4 KiB, 16 KiB, 64 KiB, 256 KiB and 512 KiB. It performs 50 warmups and 500 measured iterations per size, and reports p50/p95/p99 plus min/max nanoseconds and the exact source SHA-256. This is an environment-specific implementation baseline, not an industry comparison; compare against CUE or other validators only after freezing a semantically matched corpus and equivalent validation rules.

## Structural metamorphic qualification

The follow-on ENG-SCENARIO-METAMORPHIC-001 test corpus takes 1,000 deterministic variants of the frozen NACA0012 input and independently permutes key order within every TOML table, preserves array-of-table instance order, switches the source to CRLF line endings, and adds unique source comments. It asserts three separate invariants:

- The disposition and normalized semantic IR digest remain unchanged. A syntax-preserving rewrite must not alter the interpreted scenario.
- Exact source-byte SHA-256 and the combined compilation-subject digest both change. The compiler must not erase provenance merely because the semantics are equivalent.
- All 1,000 source digests and compilation-subject digests are unique; the empty verified-artifact manifest remains stable.

This is a metamorphic correctness test, not a solver-accuracy or industry-performance result. It deliberately separates semantic equivalence from byte-level provenance. The TOML specification treats key ordering as non-semantic while preserving array-of-table order; the fixture-specific mutator follows that boundary.
