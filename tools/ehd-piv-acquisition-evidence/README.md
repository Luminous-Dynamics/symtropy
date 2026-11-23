# EHD-PIV acquisition evidence verifier

A small Rust reference implementation for the first executable part of UQ-008. It verifies original source bytes against a canonical manifest and checks cohort partitions at the connected shared-cause component level.

## Run

    cargo fmt --manifest-path tools/ehd-piv-acquisition-evidence/Cargo.toml -- --check
    cargo test --manifest-path tools/ehd-piv-acquisition-evidence/Cargo.toml
    cargo clippy --manifest-path tools/ehd-piv-acquisition-evidence/Cargo.toml --all-targets -- -D warnings

Dependencies are exact-version pinned in Cargo.toml. The first clean CI run should generate and review a Cargo.lock for this standalone tool before freezing a release artifact.

## What is implemented

- Streaming SHA-256 over exact bytes read, with byte count from the same stream.
- Canonical manifest serialization with fixed schema/canonicalization versions, fixed struct field order, and sorted asset/dependency inventories.
- Manifest digest binding over asset metadata and the dependency-graph digest.
- Duplicate asset ID/path, empty identity, malformed digest, unsupported schema, unsafe relative path and unknown dependency acquisition rejection.
- Filesystem verification that rehashes each file before accepting the manifest; symlink path components and path escapes are rejected.
- Shared-cause connected components computed by union-find. A cohort cannot select only part of a connected dependence component.
- Pairwise training/model-selection/uncertainty-calibration/sealed-evaluation/out-of-domain cohort disjointness checks across acquisition IDs, dependence components, assets and reference-evidence IDs.
- A metadata-only reference claim-scope check. Missing reference uncertainty or PIV/reference cross-covariance evidence leaves the claim at residual covariance only.

## Example use

Construct an AcquisitionManifest from an independently prepared asset inventory, seal its dependency graph with set_dependency_graph_digest(), then freeze its manifest digest with manifest_sha256(). Before any decoder or PIV processor is invoked, call verify_manifest_directory(root, &manifest, expected_digest).

Construct cohort partitions using cohort_partition(). It rejects a requested split that omits another acquisition connected by a declared shared-cause chain. Pass the resulting partitions to validate_disjoint_cohorts() before fitting or calibration.

## Security and scientific limits

- A SHA-256 digest proves byte identity relative to the declared digest; it does not prove authorship, custody, calibration validity, instrument traceability, reference independence or physical truth.
- The reference assessment type checks that supporting evidence IDs/statuses are present. It does not authenticate the evidence, quantify covariance, or perform reference-error deconvolution.
- The filesystem implementation rejects symlinks and verifies canonical paths, but it is not a race-hardened hostile-local-user sandbox. Hash from a read-only, immutable snapshot/content store; a multi-user adversarial deployment should use an OS-level beneath/no-symlink open primitive and controlled file-descriptor custody.
- Canonicalization v0.1 uses fixed Rust struct field order, sorted arrays, UTF-8 JSON and no floating-point manifest fields. A future schema must use a new canonicalization version; do not change v0.1 serialization in place.
- The crate verifies the declared dependency graph; it cannot discover omitted common causes. The experiment owner must identify calibration, timing, geometry, tracer, illumination and session relationships honestly.
- The JSON example is a schema template with placeholder digests and is not an ingestible acquisition manifest.
- It does not ingest images into a PIV algorithm, establish physical reference truth, qualify a covariance family, or authorize an engineering claim.

The test fixtures are manufactured software controls, not experimental EHD observations.
