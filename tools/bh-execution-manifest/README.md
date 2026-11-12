# Black-hole execution manifest

This crate defines the per-method execution-envelope contract referenced by
bh-response-manifest v0.2.0. The response manifest states what was compared;
this receipt states how a particular method was built and run.

## Contract

The schema is published in schema/bh-execution-manifest-1.0.0.schema.json.
The Rust validator adds fail-closed cross-field checks for exact source/build/
dependency identities; argv and working directory; hashed environment-variable
values (never raw values or credentials); canonical named input identity;
runtime, OS/kernel, architecture, hardware and accelerators; arithmetic
precision/rounding; solver identity, algorithm, settings and stopping
criteria; timestamps, elapsed time, exit status, stdout/stderr and output
artifact digests.

The raw SHA-256 in MethodEvidence.execution_manifest_sha256 is computed over
the exact preserved JSON bytes. Do not reformat or reserialize a receipt after
recording its digest. Its method_id, implementation_sha256 and input_set_sha256
must agree with the referring MethodEvidence record.

## Fixture boundary

The examples are explicitly illustrative_not_executed. They demonstrate the
contract and are cross-linked by integration tests to the paired-comparison
response fixture. They are not command logs, run receipts or solver results.

A real run must preserve source revision and working-tree state, executable
and lockfile digests, argv, non-secret environment-variable value digests,
runtime/hardware, arithmetic mode, solver parameters, stopping criteria,
timestamps, exit status, and output artifact digests.

## Commands

cargo fmt --manifest-path tools/bh-execution-manifest/Cargo.toml -- --check
cargo clippy --manifest-path tools/bh-execution-manifest/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/bh-execution-manifest/Cargo.toml --all-targets

A valid receipt establishes only internal consistency. It does not attest
hardware authenticity, executable/source equivalence, solver correctness, or
a physical interpretation. Those require preserved artifacts, recomputation,
independent execution and scientific review.
