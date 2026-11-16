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
`ABSTAIN_SYNTHETIC_NOT_EXECUTED`. A completed external run with missing software version, dataset revision,
rights declaration, coordinate reference or uncertainty metadata abstains.
An explicit `UNKNOWN` rights state with `NOT_ADMITTED` use is schema-valid but
cannot be admitted for external comparison. A fully populated envelope is labelled only
`ENVELOPE_METADATA_COMPLETE_SOURCE_AND_RIGHTS_AUTHENTICITY_UNVERIFIED`: declared
rights metadata and a byte digest do not authenticate the provider, license
grant or model validity, and do not prove physical truth.

The v0.1 executable deliberately uses a tiny JSON-only artifact allow-list.
It is a contract/negative-control tranche, not a live pyIncore/IN-CORE adapter,
a generic raster/vector/Parquet importer or an authorization to retrieve
external/private datasets.


## E1 model evidence envelope — interoperability boundary

The versioned contract is
`research/protective-systems/schemas/protective-model-evidence-envelope-v0.1.schema.json`.
It fixes the evidence class, intended use, model/analysis identity, system
context, datasets, native units, artifact identity and lineage, rights metadata,
uncertainty state and claim ceiling.

The `protective-model-evidence` binary checks this schema's pinned metadata,
required field set and the corresponding semantic invariants. It verifies
artifact bytes, lengths and SHA-256 digests against the exact embedded fixture
payloads. Its E1 fixture is an IN-CORE-shaped compatibility example only; it is
not generated by an IN-CORE service and must abstain from a model-performance
claim.

From the repository root:

```sh
cargo fmt --manifest-path research/protective-systems/verifier/Cargo.toml --all -- --check
cargo test --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-model-evidence
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-model-evidence
```

A complete metadata envelope still does not authenticate the provider, prove
the licence grant, validate the scientific model, or establish physical truth.
The disposition remains
`ENVELOPE_METADATA_COMPLETE_SOURCE_AND_RIGHTS_AUTHENTICITY_UNVERIFIED`.
Any later trust decision needs a pinned authority/trust-root verification
policy and independent model/physical validation. A locally calculated SHA-256
proves only that the retrieved artifact bytes match the declared digest.

The E1 v0.1 fixture preserves six artifacts: input/output dataset metadata,
input/output file-metadata endpoint snapshots, and the input/output data
payloads. For both datasets the verifier reconciles the provider dataset ID and
dataType, embedded fileDescriptors against the separate file-metadata endpoint,
and the payload ID, filename, MIME type and byte length against the provider
descriptor. It also preserves the provider MD5 field while calculating local
SHA-256 independently.

The fixture uses exact pinned files and a path/role allow-list. This is an
intentionally narrow conformance tranche, not a generic file importer or a
live external API client. The validator has 25 unit tests written. The previous hosted attempt passed
formatting and Clippy but exposed six negative-test expectation/schema-layer
mismatches; these have been corrected. A fresh run is still required before
claiming that the 25 tests pass.


### Provider-native metadata snapshots

The E1 fixture preserves six distinct artifacts: two raw dataset metadata responses,
two file-metadata endpoint responses and two data payloads. For each dataset the
verifier checks:

- provider dataset ID matches the envelope dataset ID;
- provider dataType matches the preserved semantic type;
- file IDs in embedded fileDescriptors match the separate file-metadata response;
- the payload file ID, filename, MIME type and byte size match its provider
  fileDescriptor;
- each raw metadata/payload byte sequence has a pinned length and SHA-256.

The fixture mirrors the read-only IN-CORE pyIncore DataService boundary:
get_dataset_metadata, get_dataset_files_metadata, get_dataset_file_metadata
and get_dataset_blob. The provider's own MD5 field is preserved in the native
metadata snapshot; the envelope separately computes SHA-256 over locally
retrieved bytes. Neither digest authenticates the provider.

The actual implementation references are pinned to a public pyIncore commit:
- https://github.com/IN-CORE/pyincore/blob/540b3cc568643ff788479ffd4004853d059d7067/pyincore/dataservice.py
- https://github.com/IN-CORE/pyincore/blob/540b3cc568643ff788479ffd4004853d059d7067/tests/pyincore/test_dataservice.py
- https://github.com/IN-CORE/pyincore/blob/540b3cc568643ff788479ffd4004853d059d7067/tests/data/dataset_metadata.json

The current fixture is still synthetic and no IN-CORE endpoint is called.


## E2 PROV-O and RO-Crate 1.3 portability

The `protective-evidence-portability` binary validates the synthetic E2 package
at `research/protective-systems/fixtures/protective-evidence-crate-v0.1/`.

From the repository root:

```sh
cargo fmt --manifest-path research/protective-systems/verifier/Cargo.toml --all -- --check
cargo clippy --manifest-path research/protective-systems/verifier/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-evidence-portability
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-evidence-portability
```

The E2 gate checks the RO-Crate 1.3 descriptor, root Dataset, exact ten-file
inventory, file byte lengths and SHA-256 values, embedded E1 envelope byte
identity, the packaged Draft 2020-12 binding schema, E1 dataset IDs/directions/
artifact paths, the explicit PROV binding manifest, rights and claim ceilings,
and the sparse PROV-O graph. Each graph
entity is labelled either E1_ENVELOPE_DATASET or SYNTHETIC_CONTROL_ONLY; a
synthetic control has no provider artifact paths and cannot be promoted. The synthetic graph has two
used inputs and two generated outputs, but exactly one explicitly declared
per-output derivation. Duplicate input edges, extra generation edges and
invented derivations are rejected.

CI also runs the independently maintained `roc-validator==0.13.1` package
against the crate's base RO-Crate 1.3 profile. This is a test-only external
oracle; the production verifier remains Rust. CI selects `ro-crate-1.3`
explicitly and enables advisory extra checks. An independent validator pass
does not authenticate a provider, grant a license or establish physical truth.

The E2 fixture has 29 unit tests written. Their result is pending exact-head
hosted execution. The gate returns `ABSTAIN_SYNTHETIC_NOT_EXECUTED`.


**Standards conformance scope:** the Rust E2 PROV checker enforces the project's
compact JSON-LD profile only. It does not implement all JSON-LD 1.1 processing
algorithms or certify full W3C PROV-DM constraints/equivalence. The emitted
report makes these limitations explicit. CI runs the separate
`roc-validator==0.13.1` implementation against the RO-Crate 1.3 base profile.


## E3 DataCite, OGC Records and OpenLineage portability gate

The `protective-multiprovider-e3` binary validates the synthetic E3 package at
`research/protective-systems/fixtures/protective-evidence-multiprovider-e3-v0.1/`.

From the repository root:

```sh
cargo fmt --manifest-path research/protective-systems/verifier/Cargo.toml --all -- --check
cargo clippy --manifest-path research/protective-systems/verifier/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-multiprovider-e3
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protective-multiprovider-e3
```

The E3 gate checks three static provider-native fixtures: DataCite REST JSON:API
with selected Metadata Schema 4.7 field controls (including SWHID core syntax),
OGC API - Records Core 1.0 GeoJSON Feature semantics, and OpenLineage RunEvent
2.0.2 against a vendored official JSON Schema and exact schema URL. It verifies native byte lengths and SHA-256, source
identity/profile metadata, the exact E1 envelope bytes, the E3 manifest/schema,
and the RO-Crate 1.3 file inventory.

The E1 envelope stays `NOT_EXECUTED`; its manifest analysis identity is checked
against the exact embedded E1 envelope and null run ID. The OpenLineage
START/COMPLETE pair belongs only to the synthetic E3 adapter conformance
fixture, must reference the exact pinned 2.0.2 RunEvent schema URL, and uses
canonical UTC timestamps for deterministic ordering. The DataCite
and OGC adapters perform selected profile checks, not full XML/XSD or live
server conformance. Unknown provider extension roots are retained in the
native source snapshots and reported rather than flattened away.

The E3 fixture has 30 unit tests written. Their result is pending exact-head
hosted execution. The gate returns `ABSTAIN_SYNTHETIC_METADATA_ONLY`.


## E4 typed semantic projection and loss-accounted roundtrip gate

The `protect-evidence-semantic-projection-e4` binary builds a provider-tagged
semantic projection for the E3 DataCite, OGC Records and OpenLineage source
fixtures. It does not flatten all three standards into one generic record.

From the repository root:

```sh
cargo fmt --manifest-path research/protective-systems/verifier/Cargo.toml --all -- --check
cargo clippy --manifest-path research/protective-systems/verifier/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path research/protective-systems/verifier/Cargo.toml --bin protect-evidence-semantic-projection-e4
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protect-evidence-semantic-projection-e4
```

Each provider has a distinct tagged native view. Mapping decisions classify
fields as `EXACT_NATIVE`, `NORMALIZED_EQUIVALENT`, `DERIVED`,
`PRESERVED_RAW_ONLY` or `UNKNOWN_OR_UNSUPPORTED`. The projection retains
unmapped DataCite attributes, OGC properties and OpenLineage event/facet JSON.
Unknown vendor fields are retained and explicitly reported, including new
JSON:API top-level members, OGC record members and OpenLineage RunEvent
members; they are not coerced to defaults or silently dropped. The frozen
baseline has 52 mapping decisions: 36 exact-native, one normalized-equivalent,
one derived and 14 preserved-raw-only, with no unclassified baseline fields.

The E4 report serializes and deserializes the typed envelope and checks
structural equality. The OpenLineage projection additionally checks START and
terminal job/schema identity, canonical UTC timestamps and event ordering. Source byte digests are separate from semantic
projection equality: changing whitespace may change a raw SHA-256 while
leaving provider-native typed values and mapping decisions unchanged.
OGC bbox coordinates are explicitly classified as a normalized-equivalent
numeric representation; no CRS or coordinate reprojection is performed.
OpenLineage terminal state is explicitly classified as derived from the
observed START/terminal event pair, not as a prediction or E1 model result.

The exact E1 context envelope is separately hashed into a context receipt.
The OpenLineage E3 facet must match its embedded E1 analysis ID and
`NOT_EXECUTED` status with null run ID; it remains context only.

The E4 prototype has 32 unit tests written. Hosted formatting, Clippy,
compilation, test and executable evidence remain pending exact-head CI.
The executable returns `ABSTAIN_SYNTHETIC_METADATA_ONLY`.


## E5 continuous capability assurance and requalification oracle

The `protect-capability-assurance-e5` binary evaluates the synthetic
capability graph at
`research/protective-systems/fixtures/protect-capability-assurance-e5-v0.1/scenario.json`.

From the repository root:

```sh
cargo fmt --manifest-path research/protective-systems/verifier/Cargo.toml --all -- --check
cargo clippy --manifest-path research/protective-systems/verifier/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path research/protective-systems/verifier/Cargo.toml --bin protect-capability-assurance-e5
cargo run --manifest-path research/protective-systems/verifier/Cargo.toml --bin protect-capability-assurance-e5
```

The E5 gate computes configuration/evidence invalidation, transitive claim
impact, current claim states and an exact minimum-cost admissible requalification
set. It applies mandatory methods and independence as hard constraints.
Expired, superseded, unknown or otherwise invalid evidence never promotes a
claim. The output is a plan only; it does not execute work or qualify assets.

The fixture has 23 unit tests written. Exact-head hosted formatting, Clippy,
compilation, tests and executable output remain pending. The gate's result is
`ABSTAIN_REQUALIFICATION_NOT_EXECUTED`.
