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

## Port identity crosswalk metadata mode

The port identity profile schema is `docs/tech/MARITIME_PORT_CROSSWALK_V0_1.schema.json`, with 23 frozen adversarial cases in `crates/tools/maritime-data-receipt/tests/fixtures/port-crosswalk-corpus-v0.1.json`.

Validate the structure and internal policy consistency of a crosswalk:

```sh
cargo run --locked -p maritime-data-receipt -- \
  --validate-port-crosswalk \
  crates/tools/maritime-data-receipt/tests/fixtures/synthetic-port-crosswalk.json
```

The metadata guard requires a production UN/LOCODE release identifier (for example, `2025-1`), source/release digest fields, and explicit source/release terms states. It rejects mutable release/revision aliases, malformed codes/function/status fields, duplicate source entities, unsupported name-only resolution, port-statistic-to-terminal joins, unresolved mappings with targets, missing evidence on resolved/ambiguous mappings, and policy changes that equate unknown with zero. The bundled crosswalk is **synthetic**; its shape-test code/candidate is not an assertion that a real source statistic maps to a specific real feature.

The report's structural pass still returns `NotAdmitted`: this CLI validates the declared structure and hash syntax but does not fetch the UN/LOCODE package, check the digests against external bytes, validate whether a location really matches a port, or confer logistics/navigation authority. Capture and hash the exact published release and source series before using an actual crosswalk.

## Fail-closed metadata gates

The verifier rejects duplicate JSON object keys; missing or unknown fields; schema/version drift; traversal or absolute artifact paths declared inside the receipt; byte length or SHA-256 mismatches; invalid source/retrieval semantics; public-dataset records without HTTPS source/request URIs, retrieval time, a recorded reviewed-permitted terms decision, reviewer, review timestamp and terms digest; water-level records without a vertical datum; hydrography records without a CRS; and invalid unit, temporal, status or claim-ceiling declarations.

The public-dataset `terms_sha256` is a declared digest: v0.1 checks its syntax but does not fetch or hash a separate terms-document artifact. A declared licence review is still a statement recorded by the receipt author. This tool does not independently interpret legal terms, authenticate publisher identity, check a source server, assess source quality/freshness, or establish that published values describe the physical world.

## Output claim ceiling

The output binds the receipt bytes and artifact bytes to SHA-256 digests, records the local source-commit binding when supplied, and identifies the fixture/source class. It always states that the result is provenance/byte-integrity only and **not admission**. It does not perform physics, navigation calculations, under-keel clearance, engineering review, safety qualification, or operational control. Unknown or stale quality data must not be represented as verified by this receipt check.

The included payload is explicitly synthetic. It is not a NOAA measurement.
