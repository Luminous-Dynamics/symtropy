# Maritime Data Receipt Verifier

A Rust, offline-only check of a versioned maritime data receipt and its exact local artifact bytes. The v0.1 contract is defined by `docs/tech/MARITIME_DATA_RECEIPT_V0_1.schema.json`.

## Run

From the Symtropy workspace root:

```sh
cargo test --locked -p maritime-data-receipt
cargo run --locked -p maritime-data-receipt -- crates/tools/maritime-data-receipt/tests/fixtures/synthetic-water-level-receipt.json
```

The receipt names the expected artifact path. A second CLI argument may explicitly override the artifact path, allowing a locally fetched source artifact to be checked without storing that potentially large payload in Git:

```sh
cargo run --locked -p maritime-data-receipt -- path/to/receipt.json /path/to/exact-downloaded-bytes
```

No network request is made. The caller is responsible for preserving exactly the bytes received; this tool never normalizes line endings, decodes/re-encodes source files, transforms coordinates, or mutates artifacts.

## S-100 product-profile metadata mode

A separate metadata-only profile schema is defined in `docs/tech/MARITIME_S100_PROFILE_V0_1.schema.json`. It currently models only the metadata envelope for S-102 bathymetry, S-104 water-level information and S-111 surface currents. The positive/negative synthetic corpus is `crates/tools/maritime-data-receipt/tests/fixtures/s100-profile-corpus-v0.1.json`.

Validate a profile against its exact generic receipt and artifact:

```sh
cargo run --locked -p maritime-data-receipt -- \
  --validate-s100-profile \
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-s104-profile.json \
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-water-level-receipt.json
```

This mode checks profile metadata, pinned numeric edition shape, product-specific units, source/dataset/revision identity, CRS/vertical-reference/units/time-coverage/quality-statement matches, receipt ID and exact receipt SHA-256, and then verifies the receipt's exact local artifact bytes. For a source-native product all transform fields must remain absent. A derived S-102/S-104 profile must refer to a transform receipt and declare both input/output vertical references; this metadata guard does not independently validate the transformation itself.

A valid profile report still returns `NotAdmitted`. This is not an S-100 payload decoder, IHO validation engine, publisher-authentication mechanism or navigation/engineering qualification result. All committed sample profiles/corpus values are synthetic.

## Decision-time forecast applicability mode

The v0.1 metadata profile at `docs/tech/MARITIME_FORECAST_APPLICABILITY_V0_1.schema.json` evaluates whether a declared water-level forecast run covers a requested scenario time under an explicitly supplied policy. It is separate from the generic receipt schema and currently accepts only receipts whose product/domain are `water_level_prediction`.

```sh
cargo run --locked -p maritime-data-receipt -- \
  --evaluate-forecast-applicability \
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-water-level-forecast-profile.json \
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-water-level-forecast-receipt.json
```

The optional third positional argument overrides the exact artifact path, so a source file can stay outside the public worktree. The CLI verifies the receipt and artifact bytes first, then checks receipt ID/SHA-256, artifact SHA-256, internal product kind, dataset ID/revision, model-run identity, model initialization time, issue time, fixed assessment time, target time, declared validity interval, maximum issue age and maximum forecast lead. The assessment time is explicit rather than the wall clock, preserving deterministic replay. The provider-native product ID remains a distinct reported identifier; it is not inferred from the internal receipt product kind.

Statuses are metadata-only:
- `TEMPORALLY_APPLICABLE_NOT_ADMITTED`: the declared temporal bounds are satisfied and quality is not marked unknown.
- `INDETERMINATE_NOT_ADMITTED`: for example, the run was not issued at decision time, the target is before the decision or outside its validity window, an age/lead bound is exceeded, or quality is unknown.
- Invalid or receipt/profile/artifact mismatches fail with a non-zero exit status.

The 22-case synthetic corpus at `crates/tools/maritime-data-receipt/tests/fixtures/forecast-applicability-corpus-v0.1.json` covers issue-age and lead boundaries, validity-window endpoints, look-ahead, malformed calendar times, receipt binding, mutable revisions, provider-product identifiers and attempted authority escalation. A separate test-only oracle evaluates every corpus case using its own Gregorian timestamp parser and fixed baseline digests, rather than calling the production time parser.

This gate does **not** parse forecast observations/predictions from the artifact, authenticate a publisher, establish forecast skill, judge uncertainty quality, choose universal freshness thresholds, compute under-keel clearance, or authorize navigation, berth decisions or physical actions. A temporally applicable result always remains `NotAdmitted`; source licensing, payload semantics, model suitability and independent engineering review are separate gates. All committed forecast bytes are synthetic.

## UNCTADstat PLSCI macro-connectivity profile

The separate metadata contract at `docs/tech/MARITIME_PLSCI_PROFILE_V0_1.schema.json` pins the Port Liner Shipping Connectivity Index table `US.PLSCI` to port-level quarterly semantics, index points on a global-average benchmark of 100 for 2023-Q1 (100 is not assigned to a specific port), and the 2024 six-component methodology vintage and the explicit normalization formula (each component divided by its global Q1 2023 component mean, then the six ratios averaged and multiplied by 100). It requires source-native port IDs and explicit selected quarters, preserves incomplete coverage and source-reported estimates/suppression, and forbids silent zero-fill, forward-fill, interpolation, port-call/berth inference, and operational or navigation authority. It does not parse individual observations or prove that a source name maps to a real location.

Validate the profile, its exact local terms snapshot, the bound receipt, and payload bytes:

```sh
cargo run --locked -p maritime-data-receipt -- \\
  --validate-macro-connectivity-profile \\
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-plsci-profile.json \\
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-plsci-receipt.json
```

The frozen metadata-only hostile corpus is `crates/tools/maritime-data-receipt/tests/fixtures/plsci-profile-corpus-v0.1.json` (26 cases). The paired payload, receipt, profile, and corpus are invented synthetic test inputs—not captured UNCTAD rows and not a claim about any South African port. The profile CLI's success report remains `NotAdmitted` and explicitly says source authenticity, observed values, port identity, navigation and operational authority are not established. The JSON report also emits the exact profile SHA-256/byte length plus the selected quarter range, geographic scope, source-port count, methodology/normalization, missingness policy, and terms status so downstream receipts can bind to what was checked. The declared terms snapshot byte length and SHA-256 are checked against the exact file, the checked path is reported, and a public capture must align `terms_sha256` with that same snapshot digest. If captured terms remain outside the repository, use the fourth CLI positional argument for the exact local terms file (with an empty third argument when using the receipt's default artifact path). For `public_dataset`, the receipt must name the canonical official PLSCI metadata URI `https://unctadstat.unctad.org/datacentre/reportInfo/US.PLSCI` as `source.source_uri`; the exact export request URI remains a separate retrieval field. This exact URI guard is a metadata constraint, not publisher authentication.

For an exact captured payload and terms file stored outside the public worktree, supply both local file paths as the optional third and fourth arguments. The full argument shape is `--validate-macro-connectivity-profile <profile-json-path> <receipt-json-path> [artifact-path-override] [terms-snapshot-path-override]`. To override only the terms snapshot while using the receipt's artifact path, pass an empty quoted third argument:

```sh
cargo run --locked -p maritime-data-receipt -- \\
  --validate-macro-connectivity-profile \\
  path/to/plsci-profile.json path/to/receipt.json \\
  /private/captures/plsci-export.csv /private/captures/unctad-about.html

# Optional: use the default artifact path from the receipt, but override the terms file
cargo run --locked -p maritime-data-receipt -- \\
  --validate-macro-connectivity-profile \\
  path/to/plsci-profile.json path/to/receipt.json \\
  '' /private/captures/unctad-about.html
```

Research refs: [official PLSCI table metadata](https://unctadstat.unctad.org/datacentre/reportInfo/US.PLSCI), [UNCTAD Data Hub reuse terms](https://unctadstat.unctad.org/EN/About.html), and [source-capture runbook](../../../docs/tech/MARITIME_SOURCE_CAPTURE_RUNBOOK_V0_1.md). The 2024 scale reset makes methodology vintage a mandatory field; 100 refers to the global average port-connectivity benchmark in 2023-Q1. Actual source export capture remains a separate open milestone.

## Port identity crosswalk metadata mode

The port identity profile schema is `docs/tech/MARITIME_PORT_CROSSWALK_V0_1.schema.json`, with 25 frozen adversarial cases in `crates/tools/maritime-data-receipt/tests/fixtures/port-crosswalk-corpus-v0.1.json`.

Validate the structure and internal policy consistency of a crosswalk:

```sh
cargo run --locked -p maritime-data-receipt -- \
  --validate-port-crosswalk \
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-port-crosswalk.json
```

The metadata guard requires a production UN/LOCODE release identifier (for example, `2025-1`), source/release digest fields, and explicit source/release terms states. It rejects mutable release/revision aliases, malformed codes/function/status fields, duplicate source entities, unsupported name-only resolution, port-statistic-to-terminal joins, unresolved mappings with targets, missing evidence on resolved/ambiguous mappings, and policy changes that equate unknown with zero. The bundled crosswalk is **synthetic**; its shape-test code/candidate is not an assertion that a real source statistic maps to a specific real feature.

The report's structural pass still returns `NotAdmitted`: the `--validate-port-crosswalk` mode checks the declared structure and digest syntax but does not fetch the UN/LOCODE package or compare its digests with bytes on disk. The separate `--verify-port-crosswalk-snapshots` mode accepts a crosswalk plus explicit source-snapshot and registry-file paths; it hashes those exact file bytes and fails closed on either mismatch. It still does not establish publisher authenticity, licence permission, freshness, semantic correctness, or a real location match. The paired `synthetic-source-snapshot.json` and `synthetic-locode-registry-snapshot.json` are only test bytes, not copied authoritative data.

To run the exact-byte mode against the synthetic positive fixture:

```sh
cargo run --locked -p maritime-data-receipt -- \
  --verify-port-crosswalk-snapshots \
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-port-crosswalk.json \
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-source-snapshot.json \
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-locode-registry-snapshot.json
```

## Fail-closed metadata gates

The verifier rejects duplicate JSON object keys; missing or unknown fields; schema/version drift; traversal or absolute artifact paths declared inside the receipt; byte length or SHA-256 mismatches; invalid source/retrieval semantics; public-dataset records without HTTPS source/request URIs, retrieval time, a recorded reviewed-permitted terms decision, reviewer, review timestamp and terms digest; water-level records without a vertical datum; hydrography records without a CRS; and invalid unit, temporal, status or claim-ceiling declarations.

The public-dataset `terms_sha256` is a declared digest: v0.1 checks its syntax but does not fetch or hash a separate terms-document artifact. A declared licence review is still a statement recorded by the receipt author. This tool does not independently interpret legal terms, authenticate publisher identity, check a source server, assess source quality/freshness, or establish that published values describe the physical world.

## Output claim ceiling

The output binds the receipt bytes and artifact bytes to SHA-256 digests, records the local source-commit binding when supplied, and identifies the fixture/source class. It always states that the result is provenance/byte-integrity only and **not admission**. It does not perform physics, navigation calculations, under-keel clearance, engineering review, safety qualification, or operational control. Unknown or stale quality data must not be represented as verified by this receipt check.

The included payload is explicitly synthetic. It is not a NOAA measurement.
