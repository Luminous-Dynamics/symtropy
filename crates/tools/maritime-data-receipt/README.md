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

## Fail-closed metadata gates

The verifier rejects duplicate JSON object keys; missing or unknown fields; schema/version drift; traversal or absolute artifact paths declared inside the receipt; byte length or SHA-256 mismatches; invalid source/retrieval semantics; public-dataset records without HTTPS source/request URIs, retrieval time, a recorded reviewed-permitted terms decision, reviewer, review timestamp and terms digest; water-level records without a vertical datum; hydrography records without a CRS; and invalid unit, temporal, status or claim-ceiling declarations.

A declared licence review is still a statement recorded by the receipt author. This tool does not independently interpret legal terms, authenticate publisher identity, check a source server, assess source quality/freshness, or establish that published values describe the physical world.

## Output claim ceiling

The output binds the receipt bytes and artifact bytes to SHA-256 digests, records the local source-commit binding when supplied, and identifies the fixture/source class. It always states that the result is provenance/byte-integrity only and **not admission**. It does not perform physics, navigation calculations, under-keel clearance, engineering review, safety qualification, or operational control. Unknown or stale quality data must not be represented as verified by this receipt check.

The included payload is explicitly synthetic. It is not a NOAA measurement.
