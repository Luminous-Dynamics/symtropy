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
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-systems-verifier -- research/protective-systems/fixtures/body-armor-cases-v0.1.json

```

The separate `body-armor-cases-v0.1.json` corpus is a synthetic extension of
the parent record-conformance fixtures. It checks torso-scope and addenda
handling, prevents NIJ 0123.00 threat-level specifications from being promoted
to product test methods, makes unresolved SANS currentness abstain, requires
independent review after a configuration/lot change, and requires separate
wearer-assessment references for fit/mobility/thermal usability claims.
`ACCEPT_RECORD_ONLY` still means record completeness only; it is never a
physical-performance PASS.

With no argument, the binary evaluates the frozen v0.1 corpus embedded at compile
time. With a path argument, it evaluates that JSON file. It emits a deterministic
JSON report and exits:

- `0`: every declared expected decision matches;
- `1`: corpus is structurally valid but one or more decisions mismatch;
- `2`: input, JSON, or corpus structure is invalid.

The programme's NASA B2 reproduction protocol is documented at
[`NASA_BO_BENCHMARK_CROSSWALK_V0_1.md`](../voi/NASA_BO_BENCHMARK_CROSSWALK_V0_1.md).
It pins the source benchmark families and 100-evaluation / five-seed comparison
before any Luminous optimizer superiority claim.

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

## Public dataset integrity gate

The `protective-data-gate` binary is a separate integrity step for the B1
public-data benchmark. It does not change the B0 record verifier's semantics.
It validates the NIST registry entry and checks a local archive against the
SHA-256 published by the NIST Public Data Repository.

From the repository root:

```sh
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-data-gate -- research/protective-systems/fixtures/body-armor-public-datasets-v0.1.json /path/to/V50.zip
```

Exit behavior:

- `0`: registry entry is eligible and the archive digest matches;
- `1`: input is valid but the archive digest does not match;
- `2`: usage, registry, rights metadata, file access or hashing error.

The gate prints the expected and observed digest and byte count. It does not
download, extract, parse, train on or score the archive. A digest mismatch must
stop the pipeline. The registry's Zenodo CFRP candidate remains excluded while
its item-specific licence and permitted-use scope are unresolved.

The gate includes known-answer SHA-256 tests and registry fail-closed tests.
A passing digest proves byte identity only; it does not validate the dataset's
experimental design, statistics, current relevance, armour performance or
product conformity.
