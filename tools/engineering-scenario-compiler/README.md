# Engineering Scenario Compiler v0.1

A standalone Rust tool that compiles the existing NACA0012 engineering scenario TOML into a deterministic typed IR and compile-obligation report. The Rust dependency graph is committed/generated in `Cargo.lock` during exact-head qualification. The portable identity profile pins `serde_jcs = 0.2.0`; its independent Go verifier uses the source commit recorded in the workflow.

## Run

From this directory:

    cargo test --locked
    cargo bench --bench compile_latency --locked
    cargo run --locked -- ../../../benchmarks/aero/naca0012-inc-rans/case.toml
    cargo run --locked -- ../../../benchmarks/aero/naca0012-inc-rans/case.toml --solver-config ./upstream/config.cfg --mesh ./upstream/mesh.su2
    cargo run --locked -- ../../../benchmarks/aero/naca0012-inc-rans/case.toml --portable-ir-dir ./portable-identity

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


## Portable semantic identity v1

The compiler now emits a parallel, language-neutral identity profile:

`engineering-scenario-ir-jcs-rfc8785-v1`

The canonicalizer is pinned to Rust `serde_jcs = 0.2.0`, which implements RFC
8785. The independently implemented Go verifier is pinned to
`cyberphone/json-canonicalization` commit
`32a48195b9b20fe67b1795e39e4509f14d860b45`. Both must produce the exact same
bytes for the checked-in `interop/golden/basic-input.json` vector.

The identity model deliberately separates three levels:

1. **Source identity** — SHA-256 over the exact TOML source bytes. Whitespace and
   comment changes alter this digest.
2. **Portable semantic identity** — SHA-256 over the RFC 8785 canonical bytes
   of a profile/version/compiler/parser envelope plus the typed Scenario IR.
   Source-only whitespace/comment changes preserve this digest.
3. **Portable compilation-subject identity** — a domain-separated SHA-256 over
   JCS canonical bytes binding the exact source digest, portable semantic
   digest, sorted verified-artifact manifest digest, compiler/schema/parser
   identities, compile disposition, and the full compile-obligation list.
   Changing source bytes, an artifact, an obligation, or the profile changes
   this identity.

The existing v0.1 `semantic_ir_sha256`, `artifact_manifest_sha256` and
`compilation_subject_sha256` fields remain unchanged and continue to use the
legacy Rust `serde_json` serialization path. The new fields
`canonicalization_profile`, `semantic_ir_jcs_sha256`,
`artifact_manifest_jcs_sha256`, and `compilation_subject_jcs_sha256` are
additive. This is intentionally not a silent migration of previous digests.

### JCS numeric and Unicode boundaries

The current typed IR contains finite IEEE-754 binary64 quantities and bounded
unsigned 32-bit integer fields. Those integer fields are within JCS's exact
interoperable integer range. Non-finite values are rejected by source
validation. JCS preserves Unicode string content without applying a Unicode
normalization form, sorts object property names by UTF-16 code units, and
preserves array order. If a future schema introduces exact integer values
outside the interoperable binary64-safe range, represent those values as
canonical decimal strings or publish a new identity profile; do not round them
through binary64. Duplicate JSON object names are not representable in the
typed IR serializer; untrusted JSON canonicalization must separately reject
duplicates before constructing typed values.

Deterministic CBOR remains a viable alternative for a future profile where
exact wide integers or compact binary transport become requirements. JCS is
the v1 choice because the present IR's numeric model fits its boundaries and
canonical JSON remains inspectable in review artifacts. Switching encoding
requires a new explicit profile ID and golden vectors, not a rewrite of
existing digests.

### Strict pre-canonicalization input validation

The independent Go verifier now validates raw input before invoking the pinned
JCS canonicalizer. Its preflight rejects duplicate object member names at any
nesting depth (including names that become equal after JSON escape decoding),
invalid UTF-8, unpaired high/low surrogate escapes, non-finite binary64
overflow, malformed JSON and trailing JSON values. The existing golden-vector
test runs through this preflight, and negative controls cover each rejection
class. A valid paired surrogate remains accepted.

This is an independent consumer-side guard in the Go qualification verifier;
it does not yet establish that every future Rust JSON ingestion path performs
the same duplicate-aware preflight. The full frozen adversarial corpus and
Rust-side raw-input guard remain follow-on work.

### Interoperability qualification

The compiler's Rust tests compare `serde_jcs` against the shared frozen byte
vector. CI separately checks the same input and expected output using the
pinned Go implementation, built in an isolated GOPATH from its exact upstream
commit.

More importantly, the compiler CLI can export both the actual typed-IR profile
envelope as ordinary JSON and the Rust JCS byte sequence via
`--portable-ir-dir <path>`. The workflow runs the independent Go
canonicalizer over that exact compiler-produced JSON and requires it to
reproduce the exact Rust JCS bytes. This checks the real Scenario IR field set,
values, nested structure, compiler/parser/profile envelope and numeric
formatting—not merely a generic library example. The JSON preimage and JCS
output are published with SHA-256 digests as qualification artifacts.

The compile-latency benchmark continues to report whole compile p50/p95/p99
and now reports standalone canonical IR JCS serialization p50/p95/p99 as a
separate measurement. The two timings must not be conflated.

This profile establishes portable identity of the typed representation only.
It does not validate CFD numerics, authenticate inputs, prove reference-data
quality, or authorize solver execution.

## Compiler latency benchmark

The dependency-free Rust benchmark measures source parsing, typed validation, normalized IR construction and digest work at 4 KiB, 16 KiB, 64 KiB, 256 KiB and 512 KiB. It performs 50 warmups and 500 measured iterations per size, and reports p50/p95/p99 plus min/max nanoseconds and the exact source SHA-256. This is an environment-specific implementation baseline, not an industry comparison; compare against CUE or other validators only after freezing a semantically matched corpus and equivalent validation rules.
