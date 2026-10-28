# CyPort As-of Feature Evidence Receipt v0.1

**Status:** synthetic artifact checks only; no feature is approved.  
**JSON Schema:** [CYPORT_ASOF_FEATURE_RECEIPT_V0_1.schema.json](./CYPORT_ASOF_FEATURE_RECEIPT_V0_1.schema.json)  
**Synthetic receipt:** [synthetic-asof-feature-receipt.json](../../crates/tools/maritime-data-receipt/tests/fixtures/synthetic-asof-feature-receipt.json)  
**Verifier:** `cargo run --locked -p maritime-data-receipt -- --validate-cyport-asof-feature-receipt crates/tools/maritime-data-receipt/tests/fixtures/synthetic-asof-feature-receipt.json .`

## What this receipt proves—and what it does not

A retrospective table becomes a defensible prediction dataset only when each candidate feature has a source artifact, an identity, and an availability time. This receipt records the feature's source kind, publication and retrieval times, feature availability time, forecast-valid interval, units, exact artifact hash and length, derivation ID, join method, storm crosswalk evidence, and target-time semantics review.

The Rust validator performs four narrow checks:

1. **Identity and schema:** the receipt is pinned to the exact CyPort revision, resource path, Git blob and byte length. Unknown fields, duplicate keys, unsupported source kinds, duplicate feature names, malformed calendar timestamps, and unsafe artifact paths fail closed.
2. **As-of chronology:** timestamps must use canonical UTC form. Source publication cannot be after the registered decision cutoff; feature availability cannot predate publication or postdate the cutoff; retrieval cannot predate publication; the valid interval cannot end before it starts. This proves only that the supplied timestamps are internally consistent, not that the publisher or historical availability claim is true.
3. **Artifact integrity:** for each relative artifact path, the verifier resolves the path under a caller-supplied artifact root and checks exact byte length and SHA-256. Paths escaping the root, missing files, symlinks resolving outside the root, length differences, and digest mismatches are rejected.
4. **Claim boundaries:** the report always says that CyPort row membership and target-time alignment are unverified; predictor eligibility, model fitting, replay and operational authority remain false. A field is not promoted because its receipt is syntactically valid.

## Why the synthetic fixture is intentionally synthetic

The fixture artifact literally labels itself as synthetic and uses an `example.invalid` URL, synthetic storm/port/interaction IDs, and a `SYNTHETIC_ONLY` crosswalk. It tests the verifier path without embedding source data or suggesting that a particular NHC forecast record has been found. The reported artifact byte length (77) and SHA-256 apply to the checked-in fixture bytes only.

For real evidence, an archived product must be pinned to a stable provider product/revision and captured as immutable bytes. Preserve publication time as shown by the source, capture/retrieval time, valid interval and units, and record any parsing/transformation. The source may be downloaded today while having been published before a historic cutoff; the artifact's current retrieval time does not itself prove the historical operator could access it, which is why it remains a separate field and separate review gate.

## What still blocks a real replay

- **Storm/port mapping:** verify the CyPort SID/PID to archive/port crosswalk. Exact or defensible joins must be recorded with ambiguous and unmatched counts. Synthetic IDs do not demonstrate a real mapping.
- **Target-time semantics:** CyPort `start_date` is date-only; timezone and within-day event-time meaning are unresolved. Do not claim 24/48/72-hour warning lead time by manufacturing midnight timestamps. Even a feature receipt cannot repair a target whose event time is unknown.
- **Archive coverage:** the NHC advisory archive is a candidate source, not a promise of complete products. Audit each storm, product family and relevant issuance time. Do not substitute final HURDAT2/IBTrACS best-track estimates for forecasts actually published before the cutoff.
- **Access and truthfulness:** timestamps, URLs and product IDs in a receipt are assertions unless independently checked. A production admission process needs source capture receipts, source-specific terms/attribution review, exact product hashes, independent crosswalk review and a separate target-time semantics decision.
- **Feature whitelist:** remains empty in the upstream contract. This receipt validator verifies inputs, not eligibility. Only a separately reviewed register may allow a field into an as-of experiment.

## Next benchmark step

First test the validator against synthetic hostile cases: post-cutoff publication/availability, impossible UTC timestamps, missing values without reasons, wrong artifact digest/length, path traversal, duplicate feature names, retrospective best-track source kind and unknown units. Then build an archive inventory and crosswalk with exact product receipts, but do not fit a model until both target timing and feature availability pass independent review.

The future benchmark should report rare-event PR-AUC alongside test prevalence, Brier score/Brier skill and log loss against training-only reference forecasts, precision/recall at thresholds selected only on calibration data, and uncertainty clustered by cyclone SID. Current temporal test size (18 cyclone groups) does not justify broad operational or subgroup calibration claims.

## Claim ceiling

This receipt format verifies recorded metadata and local artifact bytes. It does **not** establish authentic publication time, historic availability, source-row membership, valid geographic joins, forecast lead time, predictive skill, causality, physical truth, or operational fitness. Keep raw upstream data out of the public repository unless source-specific redistribution rights have been reviewed.
