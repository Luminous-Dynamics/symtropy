# Protective Systems B0 verifier

This is an isolated Rust crate for the synthetic record-conformance tranche of
[PROTECTIVE-SYSTEMS-001](https://github.com/Luminous-Dynamics/symtropy/issues/5246).

## Run

From the repository root:

```sh
cargo test --manifest-path research/protective-systems/verifier/Cargo.toml
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml -- path/to/fixture-corpus.json
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-voi
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-robust

```

With no argument, the binary evaluates the frozen v0.1 corpus embedded at compile
time. With a path argument, it evaluates that JSON file. It emits a deterministic
JSON report and exits:

- `0`: every declared expected decision matches;
- `1`: corpus is structurally valid but one or more decisions mismatch;
- `2`: input, JSON, or corpus structure is invalid.

The crate also contains the exact finite VOI microbenchmark:

```sh
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-voi
```

It is a synthetic binary decision fixture, not evidence of improved materials
discovery. Its known-answer negative controls include an anti-correlated proxy
and a cheapest experiment that is worse than doing nothing.

The exact finite robust-design oracle is available as:

```sh
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-robust
```

It compares a finite ambiguity set, scenario safety floors, worst expected loss
and discrete upper-tail CVaR. Incomplete uncertainty sets abstain, and a problem
with no robust-feasible design returns an explicit no-design disposition.

The library also exposes `damage`, a synthetic claim-state transition oracle.
It invalidates directly affected claims, moves claims with unresolved dependency
scope to review, and requires new independent evidence bound to the exact
current configuration revision before requalification. INVALIDATED is monotonic
until explicit requalification; later unknown changes cannot weaken it to
NEEDS_REVIEW. A repair record alone cannot restore a claim.

## Claim ceiling

The only positive decision is `ACCEPT_RECORD_ONLY`. This means the required
synthetic record fields are present for the declared scope. It does **not** mean
a physical test passed, that a product is safe, or that a standard/certificate
has been satisfied. A signature or hash is not physical corroboration.

The report includes a SHA-256 digest of compact serde_json serialization. The
corpus is parsed into serde_json's ordered object representation before
serialization, so inconsequential whitespace does not change the digest.

This crate is intentionally an isolated workspace so it can be tested without
adding dependencies to Symtropy's main workspace. It uses Rust, serde, serde_json
and sha2; no Python runtime or network access is needed at evaluation time.
