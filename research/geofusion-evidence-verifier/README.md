# GeoFusion Evidence Receipt Verifier v0.1

**Tracking:** [GEOFUSION-EVIDENCE-RECEIPTS-001 / #6158](https://github.com/Luminous-Dynamics/symtropy/issues/6158)  
**Schema owner:** [evidence schema PR #6162](https://github.com/Luminous-Dynamics/symtropy/pull/6162)  
**Parent architecture:** [#6142](https://github.com/Luminous-Dynamics/symtropy/issues/6142)

## Purpose

Verify one raw source-interval payload, its evidence envelope and one source-allocation receipt without trusting declared digests or reported balance results.

This is a deliberately bounded Rust runtime slice. It is not a full Draft 2020-12 JSON Schema engine, persistent cross-campaign registry, measurement/calibration authority, solver runner or physical-truth oracle.

## What it verifies

1. Parses the raw payload, evidence envelope and allocation receipt.
2. Recomputes SHA-256 over the **exact raw payload bytes**; it never hashes a parsed-and-reserialized JSON object.
3. Recomputes and checks payload byte length.
4. Joins evidence class, source asset, measurement boundary, explicit source interval ID, carrier and bounded time window across all three records.
5. Joins the allocation receipt to the exact evidence ID and recomputed payload digest.
6. Rejects duplicate allocation edge IDs, invalid energy terms and service/carrier mismatches.
7. Recomputes allocated energy, curtailment, residual and normalized absolute residual using the same scale-normalized formula as the GeoFusion Rust source-allocation ledger.
8. Compares the recomputed result to the declared evaluation instead of trusting a prefilled `within_tolerance: true`.
9. Reports byte integrity and allocation closure as separate outputs. A byte-integrity pass with an unbalanced allocation is not a qualified receipt.

## Run

From the repository root:

```sh
cargo test --locked --package geofusion-evidence-verifier
cargo clippy --locked --package geofusion-evidence-verifier --all-targets -- -D warnings

cargo run --locked --package geofusion-evidence-verifier -- \
  docs/research/geothermal-fusion/evidence/fixtures/synthetic-geothermal-source-interval.v0.1.json \
  docs/research/geothermal-fusion/evidence/fixtures/synthetic-source-interval-evidence.v0.1.json \
  docs/research/geothermal-fusion/evidence/fixtures/synthetic-source-allocation-receipt.v0.1.json
```

Exit codes:
- `0`: raw bytes verified and allocation within the declared relative tolerance;
- `1`: invalid input, identity mismatch, tampered payload, invalid receipt or forged evaluation;
- `2`: bytes verified, but allocation is unbalanced and therefore not qualified.

## Adversarial and positive test coverage

The 22 current unit tests cover exact fixture success, byte tampering, declared byte-length mismatch, interval-ID mismatch, boundary mismatch, synthetic-to-measured evidence-class promotion, duplicate allocation IDs, service/carrier mismatch, over-allocation and negative residual, forged reported balance, allocation digest mismatch, rejected integrity state, zero tolerance, stale/unknown validity, mismatched time windows, unsupported digest algorithms and malformed digests.

## Batch uniqueness

`verify_source_interval_batch` verifies a set of source-interval bundles and enforces one receipt per physical source interval in that batch. Its duplicate key is source asset + measurement boundary + carrier + exact UTC start representation + duration. It also rejects duplicate evidence IDs and allocation receipt IDs. Renaming an interval/evidence/receipt ID cannot make the same source interval allocatable twice in one batch.

The check is **batch-scoped**. It is not a persistent cross-campaign registry, and the current v0.1 key compares the declared UTC start string exactly rather than normalizing equivalent RFC 3339 spellings. A future archive-level ledger should canonicalize instants and retain committed source intervals across independent batches.

## Explicit limitations

- Only `source_interval` envelopes and source-allocation receipts are currently supported; the batch API prevents duplicate interval reuse within one supplied batch.
- The verifier performs runtime semantic checks on the fields it consumes; it does not execute the complete JSON Schema Draft 2020-12 specification.
- It cannot detect reuse of the same source interval across separately verified batches; a persistent cross-campaign ledger is not implemented.
- For measured/audited classes it requires the declared measurement and calibration fields; it does not authenticate certificates or prove that a calibration covers the observation interval.
- For calibrated/independently validated model classes it requires a declared applicable, converged solver record; it does not reproduce the solver or validate its physics.
- It does not prove meter accuracy, clock synchronization, covariance, model-domain validity, physical source truth or operational authorization.
- A matching SHA-256 proves byte identity, not the truth or quality of the data.

All fixtures are synthetic. No production provider, measurement system, geothermal site or fusion plant is qualified by this verifier.
