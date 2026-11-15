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


## Defensive threat portfolio gate

The `protective-threat-portfolio` binary checks the defensive threat-family
registry and ten synthetic decision-boundary cases. It does **not** calculate
real-world probabilities, damage magnitudes, weapon effects or operational
threat values, and it does not replace the robust optimization owner in
PROTECT-ROBUST-001.

From the repository root:

```sh
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-threat-portfolio
```

The gate rejects:
- speculative exotic-physics claims promoted to dedicated investment without a credible fieldable pathway;
- numerical event probabilities attached to unknown or ordinal-only likelihood categories;
- an expected-loss claim derived from ordinal category scores;
- a decision that trades away a violated hard safety floor;
- overlapping multi-hazard benefits that have not been reconciled.

Its positive result means only that the qualitative registry and synthetic
decision-boundary corpus satisfy their declared v0.1 structure and rules. It is
not a threat forecast, a physical protection qualification or a recommendation
for a specific real-world installation.


## Defensive resilience evidence bridge

The `protective-resilience-bridge` binary validates six synthetic conformance
cases connecting the threat portfolio to the existing service-cascade,
cross-scale resilience, recovery-quality, robust-decision, value-of-information
and measurement owners.

From the repository root:

```sh
cargo fmt --manifest-path research/protective-systems/verifier/Cargo.toml --all -- --check
cargo test --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-resilience-bridge
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-resilience-bridge
```

The gate checks owner references, hazard-family references, frozen case
semantics, separate service/buffer recovery, single-credit shared benefits,
non-compensatory human-safety floors, null service benefit from observations
that do not change a decision, and abstention when event frequency is unknown.
Its digest is a canonical JSON-fixture digest, not a physical-evidence receipt.
The positive result is synthetic contract conformance only.


## E0 exact resilience reference oracle

The `protective-resilience-e0` binary independently checks a frozen,
synthetic portfolio comparison. It consumes declared service trajectories; it
does not simulate hazards, networks, physical systems or recovery dynamics.
It checks matched scope and beneficiary strata, recomputes per-service deficit
and sustained recovery, verifies unique cost/benefit identities, and enforces
non-compensatory safety and separate buffer-recovery states.

From the repository root:

```sh
cargo fmt --manifest-path research/protective-systems/verifier/Cargo.toml --all -- --check
cargo test --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-resilience-e0
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-resilience-e0
```

The reference case is in
`research/protective-systems/fixtures/protective-resilience-e0-reference-v0.1.json`.
It is synthetic and bounded-deterministic, with no event probabilities. The
service-deficit units are milli-beneficiary-minutes and remain separate by
service ID; water, energy, healthcare, money and other unlike outcomes are not
summed into a universal score. The exact expected reports are frozen in the
fixture. Passing means deterministic evidence-accounting conformance only, not
physical protection, field performance, legal compliance or certification.


## E1 external model evidence envelope

The `protective-model-evidence` binary validates the v0.1 evidence envelope,
its dataset input/output lineage, rights-state fields and local artifact bytes.
The synthetic fixture uses an IN-CORE-shaped dataset graph but is not an
IN-CORE run. Each artifact length and SHA-256 is checked against the exact
embedded payload bytes.

From the repository root:

```sh
cargo fmt --manifest-path research/protective-systems/verifier/Cargo.toml --all -- --check
cargo test --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-model-evidence
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-model-evidence
```

A synthetic or not-executed envelope returns
`ABSTAIN_SYNTHETIC_NOT_EXECUTED`. A completed external run with missing
software version, dataset revision, rights, coordinate reference or uncertainty
metadata abstains. A fully populated envelope is labelled only
`ENVELOPE_METADATA_COMPLETE_SOURCE_AUTHENTICITY_UNVERIFIED`: a byte digest
proves local byte identity, not provider authenticity, model validity or
physical truth.

The v0.1 executable deliberately uses a tiny JSON-only artifact allow-list.
It is a contract/negative-control tranche, not a live pyIncore/IN-CORE adapter,
a generic raster/vector/Parquet importer or an authorization to retrieve
external/private datasets.
