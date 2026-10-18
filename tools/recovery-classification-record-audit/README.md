# Recovery Classification Record Audit

Standalone Rust tool for verifying the content integrity and case/profile binding of the synthetic record packet in Symtropy. The outer packet envelope is documented in `schemas/recovery/classification-record-packet-v0.1.schema.json`; its schema does not replace the Rust semantic and digest checks. It is layered on the scenario applicability audit branch and is intentionally a separate executable so the record-packet receipt can be qualified independently.

## Run

Use Cargo to format/test the crate and execute the audit. With no path argument, it uses the embedded packet. Optionally pass a packet JSON path and `--source-commit` with the exact 40-character Git SHA. The tool emits deterministic JSON and returns a nonzero exit code when the packet fails structural, digest, semantic, lifecycle or claim-ceiling checks.

## Important limits

- SHA-256 checks content integrity relative to a cited digest; it does not authenticate the issuer.
- All records and revocation states in v0.1 are synthetic; authenticity is explicitly `UNVERIFIED_SYNTHETIC`.
- A successful result is finite contract conformance, not evidence that the classification is true or a professional signed it.
- Outputs are review-only; operations are never authorized.

See `docs/tech/PROOF_CARRYING_RECOVERY_CLASSIFICATION_RECORDS_V0_1.md` and issue #1915.
