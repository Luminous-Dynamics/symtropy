# Protective Systems B0 verifier

This is an isolated Rust crate for the synthetic record-conformance tranche of
[PROTECTIVE-SYSTEMS-001](https://github.com/Luminous-Dynamics/symtropy/issues/5246).

## Run

From the repository root:

```sh
cargo test --manifest-path research/protective-systems/verifier/Cargo.toml
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml -- path/to/fixture-corpus.json
```

With no argument, the binary evaluates the frozen v0.1 corpus embedded at compile
time. With a path argument, it evaluates that JSON file. It emits a deterministic
JSON report and exits:

- `0`: every declared expected decision matches;
- `1`: corpus is structurally valid but one or more decisions mismatch;
- `2`: input, JSON, or corpus structure is invalid.

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
