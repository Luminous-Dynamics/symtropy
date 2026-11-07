# WPT independent trajectory-artifact verifier v0

This crate closes the independent replay gap in [trajectory coupling #3905](https://github.com/Luminous-Dynamics/symtropy/issues/3905), [trajectory replay #3887](https://github.com/Luminous-Dynamics/symtropy/pull/3887) and [trajectory-derived segment ledger #3895](https://github.com/Luminous-Dynamics/symtropy/pull/3895).

## What it verifies

The input bundle carries the exact base64-encoded UTF-8 bytes for five JSON artifacts:
1. canonical vehicle trajectory;
2. electromagnetic projection;
3. pavement projection;
4. road-to-EM coordinate transform;
5. road-to-pavement coordinate transform.

The verifier:
- decodes the supplied bytes and recomputes SHA-256 over the exact bytes (it does not reserialize JSON before hashing);
- parses each artifact and checks the strict, versioned record shape;
- checks unique canonical sample IDs and strictly increasing canonical timestamps;
- requires the EM and pavement projections to have exactly one sample per canonical pose, in the same order and with the same sample IDs and canonical timestamps;
- recomputes each affine transform from the supplied transform matrix and translation using integer ppm/mm arithmetic and checks every projected point exactly;
- checks each projection's transform ID and SHA-256 receipt against the actual transform bytes;
- recomputes raw timestamp + signed clock correction and requires the residual from the canonical pose timestamp to be within the stated clock uncertainty and frozen bundle/lane maxima.

This is an independent *integrity and consistency* verifier. A digest proves that the verifier received bytes matching a supplied digest; it does not authenticate who produced those bytes. An independent custodian or trusted acquisition process must authenticate source identity and custody.

## Coordinate arithmetic

Each transform carries a 3×3 integer matrix in parts per million and an integer-mm translation. For each output axis:

`q = round_half_away_from_zero((M_ppm * p_mm) / 1,000,000) + translation_mm`

The v0 verifier re-derives the output point with checked integer arithmetic. It does not validate that the transform represents a physically correct survey, that its axes are handed correctly, or that the matrix is a valid rotation. Those require surveyed control points and independent metrology evidence.

## Clock semantics

For each projection sample:
- `corrected_timestamp_ns = raw_timestamp_ns + clock_correction_ns`;
- `abs(corrected_timestamp_ns - canonical_timestamp_ns) <= clock_uncertainty_ns`;
- sample uncertainty must not exceed the projection lane maximum or the global bundle policy maximum.

The correction is a reported input; this verifier does not estimate clock offset/drift. A future measured v1 should carry calibration records for each clock, synchronization method, drift model, raw time source and uncertainty budget. The synthetic v0 proves arithmetic and fail-closed consistency only.

## Synthetic known-truth fixture

Two canonical poses are projected into two distinct frames. The fixture includes:
- five exact-byte artifact digests;
- two affine transforms;
- two EM and two pavement projection samples;
- signed clock corrections and uncertainty;
- derived EM/pavement point coordinates.

It is not measured WPT evidence, and the physical values in its projection rows are not a validated coupling, strain, heat-flux or service model.

## Validation

`cargo test --locked -p wpt-trajectory-artifact-verifier`

Twelve Rust tests cover known-truth replay, raw-byte tampering, clock residual violation, canonical-time relabeling, coordinate mismatch, transform digest mismatch, sample order, duplicate IDs, uncertainty overflow, wrong trajectory identity, unknown fields, non-synthetic evidence, invalid base64 and malformed transform dimensions.

## Claim ceiling

Exact artifact-byte integrity and declared lineage/clock/affine-coordinate consistency for a synthetic bundle only. This does not authenticate external source custody, calibrate sensors/clocks, compute coordinate transforms from survey control points, infer coupling or pavement mechanics, establish fatigue/thermal response, verify battery energy, prove standards compliance, safety or field performance.
