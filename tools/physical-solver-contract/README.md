# Physical Solver Contract Validator v0.1

A dependency-pinned Rust CLI for checking critical semantic invariants in Symtropy's versioned physical-solver request and receipt documents.

## Run

```sh
cargo test --locked --manifest-path tools/physical-solver-contract/Cargo.toml
cargo run --locked --manifest-path tools/physical-solver-contract/Cargo.toml -- \
  validate-request schemas/physical-transformation/fixtures/analytical-bar-request-v0.1.json
cargo run --locked --manifest-path tools/physical-solver-contract/Cargo.toml -- \
  validate-receipt /path/to/receipt.json /path/to/exact-request.json
```

Exit code 0 means the checked contract invariants passed. Exit code 2 means invalid JSON or a failed invariant. Exit code 64 indicates CLI usage error. Request reports include the SHA-256 of the exact input bytes, including whitespace; receipts must bind to those exact bytes rather than a reserialized JSON object.

## What it checks

- Request/receipt version tags, Git SHA and SHA-256 formats, required high-level fields, identifier uniqueness and key numeric constraints.
- Exact SI unit declarations and consistency between each load's coordinate frame and the parent request frame.
- Allowed physics/output labels, unique requests and degrees of freedom, three-component finite load vectors, material-property ranges and evidence-provenance shape.
- Receipt identity and digest binding to the exact request bytes; receipt request ID/source revision must match the request.
- Fail-closed status rules: a `PASS` needs output artifacts, acceptable convergence, an identified solver build, every requested output, all requested physics marked evaluated, and no physics incompatible with the declared model fidelity. An `UNRESOLVED` receipt must describe its claim as unresolved/unqualified.

## Claim boundaries

This is a *contract-specific semantic validator*, **not** a full generic JSON Schema Draft 2020-12 implementation. CI separately checks JSON syntax and frozen schema/fixture invariants. Unknown additional fields are not rejected by this Rust subset validator. Full JSON Schema validation and additional cross-field rules remain future work.

The CLI does not execute solvers, inspect artifact bytes, authenticate the producer, or prove that a solver actually ran. Hash consistency binds declared bytes; it is not an execution attestation. A valid request is not a simulation result. A consistent receipt is not proof of numerical verification, physical validation, safety, or an engineering-qualified model. The current PR deliberately stops before implementing an adapter.

## Negative controls

The unit tests retain wrong-unit and frame-mismatch requests, duplicate IDs/DOFs, stale exact-request digest, a PASS with missing outputs, and a PASS that attempts to report dynamic fracture under a linear-elastic fidelity class.
