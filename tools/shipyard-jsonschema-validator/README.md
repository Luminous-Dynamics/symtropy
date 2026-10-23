Shipyard JSON Schema Draft 2020-12 validator
==============================================

This crate adds standards-based Draft 2020-12 validation in front of the existing dependency-free shipyard-benchmark-audit semantic checker.

Usage
-----

    cargo run --locked --manifest-path tools/shipyard-jsonschema-validator/Cargo.toml -- meta path/to/schema.json
    cargo run --locked --manifest-path tools/shipyard-jsonschema-validator/Cargo.toml -- instance path/to/schema.json path/to/instance.json
    cargo run --locked --manifest-path tools/shipyard-jsonschema-validator/Cargo.toml -- audit benchmark-manifest.json run-report.json

The audit command validates both bundled contract schemas against the Draft 2020-12 meta-schema, validates the supplied manifest/report against those schemas, then executes the separate semantic auditor. The semantic checker remains responsible for duplicate JSON object-key rejection, exact manifest-byte SHA-256 linkage, candidate/source revision binding, oracle truth consistency, and recomputed summary arithmetic.

Reproducibility and security
----------------------------

- Pin jsonschema to 0.60.0 and serde_json to 1.0.150, and commit the generated Cargo.lock.
- Pin the draft explicitly to jsonschema::draft202012; do not infer the draft from arbitrary input.
- Call offline() for instance validators. Unresolved external references fail at validator construction; these contract schemas intentionally use no remote references.
- Enable declared format validation explicitly. Draft 2020-12 treats format as an annotation unless the validator is configured to assert it.
- The maintenance workflow generates the lockfile and applies Rust 1.96 formatting as an explicit commit on the same-repository PR branch. The validation workflow then uses --locked and offline tests after fetching the pinned graph.

Evidence boundary
-----------------

Passing schema validation means the JSON fits the declared protocol. Passing the semantic auditor means additional consistency and provenance-linkage checks passed. Neither establishes that source data are true, authorized, complete or representative; neither is a geometry safety certificate or evidence of better-than-SOTA industrial performance.

References: https://docs.rs/jsonschema/ and https://json-schema.org/specification
