# plasma-dataset-admission

Standalone, offline Rust auditor for plasma research dataset provenance, rights-review status and byte integrity. The source manifest records upstream metadata only; the CLI reads actual staged bytes from an optional local directory and recomputes their SHA-256 itself.

This tool intentionally does not download files, interpret license terms automatically, or redistribute data. It consumes a version-pinned JSON manifest and reports what remains blocked.

## Run

    cargo test --locked --manifest-path crates/tools/plasma-dataset-admission/Cargo.toml
    cargo run --locked --manifest-path crates/tools/plasma-dataset-admission/Cargo.toml -- benchmarks/plasma/dataset-admission-pnas-2025-v0.json [artifact-directory]
    cargo clippy --locked --manifest-path crates/tools/plasma-dataset-admission/Cargo.toml --all-targets -- -D warnings

A syntactically valid manifest may correctly produce BLOCKED_PENDING_EVIDENCE. When an artifact directory is supplied, the CLI reads only declared regular files from that directory, refuses symlinks/path traversal, and recomputes each file's size and SHA-256 in fixed 64 KiB chunks. Large archives are never loaded wholly into memory. Without a directory, all artifacts are treated as not locally acquired. The CLI exits successfully when the audit runs; it does not turn a blocked source into an admitted dataset.

## Admission semantics

- READY_FOR_LOCAL_EVALUATION requires a human-reviewed license decision that explicitly permits evaluation, all artifact bytes locally observed, internally computed SHA-256 values matching source-published SHA-256 values, and no unresolved digest anomaly. MD5-only records remain blocked in v0.1 because the auditor does not compute MD5.
- READY_FOR_REDISTRIBUTION additionally requires an explicit redistribution permission decision.
- BLOCKED_PENDING_EVIDENCE covers unknown rights, incomplete acquisition and duplicate source-reported MD5 values across different artifact IDs.
- REJECTED covers malformed identity, invalid digest syntax, inconsistent local observations, exact size mismatch and source SHA-256 mismatch.

A repeated upstream MD5 is a quarantine signal, not proof that two files are byte-identical. Actual local bytes are supplied to the library or streamed from the artifact directory; a hash value written into the manifest is never trusted. Local SHA-256 identifies staged bytes but does not authenticate them against a source release when the source did not publish SHA-256. Therefore v0.1 requires a source-published SHA-256 match before admitting data for evaluation; a local-only digest is insufficient.

## Current frozen fixture

The PNAS 2025 Zenodo record lists two 41.0 MB MAT files with the same source-reported MD5 and no displayed license value. The fixture preserves the original source filename spelling, including the apparent typo in the nonlinear file name. Exact byte counts and local hashes are intentionally null because they have not been acquired or independently measured.

Expected audit disposition is BLOCKED_PENDING_EVIDENCE. This is a metadata-only fixture, not a statement that the files are identical.

## Boundaries

No network access, legal advice, automatic interpretation of licenses, download, mirroring, dataset redistribution, PDE solver, closure training, physical reproduction or experiment control. Public accessibility is not a license grant. A report records the result of the supplied human review and integrity metadata; it does not certify legal rights independently.
