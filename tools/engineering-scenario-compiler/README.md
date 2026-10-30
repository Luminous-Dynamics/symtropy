# Engineering Scenario Compiler v0.1

A standalone Rust tool that compiles the existing NACA0012 engineering scenario TOML into a deterministic typed IR and compile-obligation report.

## Run

From this directory:

    cargo test --locked
    cargo run --locked -- ../../../benchmarks/aero/naca0012-inc-rans/case.toml

Exit codes: 0 = PlanReady, 2 = Invalid, 3 = Blocked, 4 = Unsupported, 64 = usage, 66 = source read error, 70 = report serialization error.

## Deliberate scope

- Offline parser and compiler only; no network calls, shell execution, solver launch, includes or plugin loading.
- Strict Serde structs reject unknown keys; TOML parser rejects syntax and duplicate-key errors.
- Typed quantity records preserve dimension and unit separately from the numeric value.
- Source SHA-256 is computed over exact input bytes. Semantic IR SHA-256 is computed over a versioned fixed-order Rust structure plus compiler and registry revisions.
- A frozen NACA0012 model profile is recognized, but this fixture correctly compiles to BLOCKED: a Git blob object ID is not a SHA-256 digest of the retrieved bytes; exact config/mesh bytes, reference data digests, and an identified solver executable/build are absent.

## Claim ceiling

A BLOCKED report is a successful compile of a structurally and dimensionally valid scenario with unresolved execution obligations. It is not a simulation plan admission, numerical result, model validation, evidence attestation, or physical claim.

The semantic identity is deterministic within this compiler/schema/registry version and ordered IR definition. Cross-language canonicalization and digest golden vectors are still a separate qualification requirement.
