# GeoFusion Evidence Receipt Verifier v0.1

**Tracking:** [GEOFUSION-EVIDENCE-RECEIPTS-001 / #6158](https://github.com/Luminous-Dynamics/symtropy/issues/6158)  
**Schema owner:** [evidence schema PR #6162](https://github.com/Luminous-Dynamics/symtropy/pull/6162)  
**Parent architecture:** [#6142](https://github.com/Luminous-Dynamics/symtropy/issues/6142)

## Purpose

Verify raw source-interval payloads, evidence envelopes and source-allocation receipts without trusting declared digests or reported balance results. The batch API also checks duplicate and overlapping source intervals within one supplied batch.

This is a deliberately bounded Rust runtime slice. It is not a full Draft 2020-12 JSON Schema engine, persistent cross-campaign registry, measurement/calibration authority, solver runner or physical-truth oracle.

## What it verifies

1. Validates the evidence and allocation-receipt schema versions and required envelope fields.
2. Recomputes SHA-256 over the **exact raw payload bytes before parsing the raw payload**; it never hashes a parsed-and-reserialized JSON object.
3. Recomputes and checks payload byte length, then parses the payload and checks its version.
4. For heat, recomputes `average_thermal_power_mw × duration_ns / 3.6e12`; for electricity, recomputes `average_electric_power_mw × duration_ns / 3.6e12`. A mismatched energy/power/duration triple is rejected.
5. Joins evidence class, source asset, measurement boundary, explicit source interval ID, carrier and bounded time window across the raw payload, evidence envelope and allocation receipt.
6. Parses strict UTC `Z` timestamps into integer nanoseconds, enforces the declared clock-quality profile, and checks calibration-window coverage for measured/audited evidence.
7. Joins the allocation receipt to the exact evidence ID and recomputed payload digest.
8. Rejects duplicate allocation edge IDs, invalid energy terms and service/carrier mismatches.
9. Recomputes allocated energy, curtailment, residual and normalized absolute residual using the same scale-normalized formula as the GeoFusion Rust source-allocation ledger.
10. Compares the recomputed result to the declared evaluation instead of trusting a prefilled `within_tolerance: true`.
11. Reports byte integrity and allocation closure as separate outputs. A byte-integrity pass with an unbalanced allocation is not a qualified receipt.
12. Batch verification rejects duplicate and overlapping source windows from the same physical source asset, boundary and carrier, including duplicate intervals whose caller-assigned IDs have been renamed.


## Schema validation

The CI workflow uses `check-jsonschema==0.39.0` **as a CI-only tool** to validate the four Draft 2020-12 schemas against their declared metaschemas, check positive source/evidence/allocation and batch-manifest fixtures, and assert that the null-asset negative control is rejected. The runtime library and CLI remain Rust-only; the external CLI is not a production dependency. Raw source-payload schemas now cover both heat and electricity with separate, carrier-specific power fields.

## Standards composition

The companion [GeoFusion standards interoperability crosswalk](../../docs/research/geothermal-fusion/evidence/STANDARDS_INTEROPERABILITY.md) composes SSP 2.0.1, SSP-LS-Traceability 1.0.0, FMI 3.0.2, BagIt 1.0, RO-Crate 1.3 and PROV-O, with attestations delegated to existing DSSE/in-toto owner #5560. SSP-LS carries simulation-task/resource metadata and derivation-chain GUIDs; it is not a meter-calibration authority. The [machine-readable interoperability matrix](../../docs/research/geothermal-fusion/evidence/STANDARDS_INTEROPERABILITY_MATRIX.v0.1.json) keeps resource GUIDs, cryptographic content digests and provenance relationships distinct.

## Run

From the repository root:

```sh
cargo test --locked --package geofusion-evidence-verifier
cargo clippy --locked --package geofusion-evidence-verifier --all-targets -- -D warnings

cargo run --locked --package geofusion-evidence-verifier -- \
  docs/research/geothermal-fusion/evidence/fixtures/synthetic-geothermal-source-interval.v0.1.json \
  docs/research/geothermal-fusion/evidence/fixtures/synthetic-source-interval-evidence.v0.1.json \
  docs/research/geothermal-fusion/evidence/fixtures/synthetic-source-allocation-receipt.v0.1.json

cargo run --locked --package geofusion-evidence-verifier -- --batch \
  docs/research/geothermal-fusion/evidence/fixtures/source-interval-batch-manifest.v0.1.json
```

Exit codes:
- `0`: raw bytes verified and allocation within the declared relative tolerance;
- `1`: invalid input, identity mismatch, tampered payload, invalid receipt or forged evaluation;
- `2`: bytes verified, but allocation is unbalanced and therefore not qualified.

## Adversarial and positive test coverage

The 43 current unit tests cover exact fixture success, byte tampering (including malformed tampered JSON), declared byte-length mismatch, interval-ID mismatch, boundary mismatch, synthetic-to-measured evidence-class promotion, duplicate allocation IDs, service/carrier mismatch, over-allocation and negative residual, forged reported balance, allocation digest mismatch, rejected integrity state, zero tolerance, stale/unknown validity, clock-quality refusal, mismatched and non-UTC time windows, unsupported digest algorithms, malformed digests, relative comparisons at tiny energy scales, electrical energy/power consistency and allocation, heat/electricity batch coexistence, duplicate source intervals across batches (including fractional-second spellings), overlapping intervals, invalid calendar dates, duplicate evidence/receipt IDs, source power-duration mismatch and zero-duration rejection.

## Batch uniqueness

`verify_source_interval_batch` verifies a set of source-interval bundles and enforces one receipt per physical source interval in that batch. Its duplicate key is source asset + measurement boundary + carrier + canonical UTC start instant in integer nanoseconds + duration. It also rejects duplicate evidence IDs and allocation receipt IDs. Renaming an interval/evidence/receipt ID cannot make the same source interval allocatable twice in one batch.

The check is **batch-scoped**. It parses canonical UTC `Z` timestamps into integer nanoseconds, so equivalent fractional-second spellings cannot bypass duplicate detection. It also rejects overlapping windows for the same source asset, measurement boundary and carrier. It is capped at 256 intervals per call. It is not a persistent cross-campaign registry; a future archive-level ledger should retain committed source intervals across independent batches.

## Explicit limitations

- The runtime API currently supports source-interval envelopes and source-allocation receipts with separate heat/electricity raw-payload schemas. Additional carriers (e.g. mechanical work, exergy or fluid mass) are not part of v0.1.
- The batch API prevents duplicate and overlapping interval reuse within one supplied batch, using a canonical UTC nanosecond start key.
- The verifier performs runtime semantic checks on the fields it consumes; it does not execute the complete JSON Schema Draft 2020-12 specification.
- It cannot detect reuse of the same source interval across separately verified batches; a persistent cross-campaign ledger is not implemented.
- For measured/audited classes it requires unique instrument IDs and declared calibration metadata, and verifies that the declared calibration time window covers the full source interval. This is a timestamp consistency check only; it does not authenticate the certificate or prove the instrument's accuracy.
- For calibrated/independently validated model classes it requires a declared applicable, converged solver record; it does not reproduce the solver or validate its physics.
- It does not prove meter accuracy, clock synchronization, covariance, model-domain validity, physical source truth or operational authorization.
- A matching SHA-256 proves byte identity, not the truth or quality of the data.

All fixtures are synthetic. No production provider, measurement system, geothermal site or fusion plant is qualified by this verifier.
