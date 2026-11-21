# GAUGE contact conformal pose-score adapter v0

**Parent programme:** [GAUGE contact prediction sets #6635](https://github.com/Luminous-Dynamics/symtropy/issues/6635)  
**Score-construction issue:** [#6655](https://github.com/Luminous-Dynamics/symtropy/issues/6655)  
**Conformal rank owner:** [#4146 / PR #4147](https://github.com/Luminous-Dynamics/symtropy/pull/4147)  
**Pose discrepancy owner:** [SO(3) reference PR #6567](https://github.com/Luminous-Dynamics/symtropy/pull/6567)  
**Raw-record source owner:** [GAUGE parser PR #6582](https://github.com/Luminous-Dynamics/symtropy/pull/6582)

This small Rust crate is a score-construction adapter only. It does not parse raw GAUGE JSON, reconstruct frame conventions, compute quaternion errors, estimate a trajectory, or implement a conformal rank algorithm.

## Complete-trial score

An upstream, independently qualified pose evaluator supplies translation discrepancy in metres and SO(3) geodesic discrepancy in radians for every expected body and exported frame. This adapter requires an exact frozen manifest of expected body IDs and sample counts.

For each body/frame sample:

r = max(translation_error_m / translation_scale_m, rotation_error_rad / rotation_scale_rad)

The one trial-level score is:

S_trial = ceil(1e9 * max(all expected r))

The translation and rotation normalizers are positive preregistered engineering scales. They are not inferred from the final test set and are not themselves confidence intervals. The score-definition SHA-256 must bind the scales, units, body manifest, target horizon, frame policy and quantization version.

Quantization uses one billion integer units per normalized-error unit and rounds upward. This prevents floating-point quantization from making the reported maximum smaller. Translation and rotation diagnostics remain separate upstream; the conformal scorer only receives their declared dimensionless maximum.

## Completeness and no-pseudoreplication rules

- The expected body IDs and frame counts are explicit and immutable for the score definition.
- Each (body ID, zero-based exported frame index) pair must occur exactly once.
- Unexpected bodies, duplicate body/frame pairs, missing frames, out-of-range indices, negative/non-finite errors, invalid normalizers and score overflow fail closed.
- A trial produces exactly one maximum score, irrespective of body count or frame count. Its frames never become independent conformal samples.
- Every trial score carries the exact 40-hex dataset revision, raw-trial SHA-256, forecast SHA-256, model digest, target digest and domain digest.
- The calibration-block builder rejects scores whose model, target, domain or score-definition identity differs from the frozen block identity.
- The block receipt retains a deterministically ordered copy of every per-trial source/forecast record alongside the conformal block. The #4147 core itself remains unchanged and receives exactly one maximum score per independent block.
- A failed or aborted episode with an incomplete target horizon remains in the calibration block using the explicit u64::MAX incomplete-horizon sentinel. It is never silently filtered.
- A #4147 receipt with threshold u64::MAX maps to an unbounded pose set, not a finite-width guarantee.
- A completed trial cannot use the incomplete sentinel.

The adapter's block builder consumes the actual #4147 CalibrationBlock type by a local path dependency. It preserves every trial ID/outcome and computes one maximum score over all trials in a declared independent session/article/trial block. #4147 remains the sole owner of rank arithmetic, exchangeability-unit uniqueness, fit/test leakage and application identity.

### Dataset-bound calibration receipt

The generic #4147 calibration receipt intentionally does not carry a dataset revision or source-manifest digest. The GAUGE adapter therefore wraps it in `PoseCalibrationReceipt` after checking all supplied `CalibrationBlockReceipt` values:
- every block has the same exact 40-hex dataset revision and identical model/target/domain/score-definition/split identities;
- exchangeability-unit IDs and trial IDs are unique across the complete supplied calibration set;
- trial records match each #4147 block's episode IDs, outcomes and maximum score;
- independent-unit and episode counts match the generic conformal receipt exactly;
- a caller-supplied source-manifest SHA-256 and calibration prediction-manifest SHA-256 are retained for audit.

Applying a pose-set threshold first checks exact dataset revision and source-manifest SHA-256, then delegates model/target/domain/score identity to #4147. The prediction-manifest digest is retained as calibration provenance; it is not compared with a future test prediction manifest because the test predictions necessarily refer to different trials. These digests are caller-provided receipts and their construction must be independently reviewed. Before exposing a `PoseCalibrationReceipt`, the adapter replays the supplied calibration blocks through #4147's own `calibrate()` API and compares the complete resulting receipt (including rank, threshold, counts and coverage statement) with the supplied conformal receipt. The adapter therefore does not copy the rank formula and does not accept a hand-built threshold that does not follow from the supplied scores.

## Temporal and semantic boundary

Frame indices are zero-based exported sample indices. They are not absolute sensor timestamps or 180-Hz acquisition indices. The GAUGE JSON exports trajectories at 30 FPS; high-frequency contact information cannot be reconstructed by interpolation. The upstream temporal observability contract #6587 owns what claims the exported cadence supports.

The pose discrepancies supplied here must already come from an admitted frame convention and metric. The raw GAUGE JSON alone does not establish quaternion algebra, transform direction, handedness, Euler units or absolute clock origin. Until those semantics and source rights are admitted, real-record score construction remains NOT_EVALUABLE.

## Important statistical boundary

The conformal rank owner #4147 supplies finite-sample marginal whole-unit coverage only under exchangeability of the declared independent session/trial blocks within the exact frozen domain. This adapter does not prove exchangeability, measurement calibration, or any physical law. A score-definition digest and frozen split manifest must be independently reviewed. Do not split adjacent frames or same-session recordings into independent calibration/test units.

The score is a nonconformity measure, not a probability. A high score threshold means a wider pose set; a sentinel threshold means the pose set is unbounded for decision purposes. Set coverage must always be reported with set efficiency, task success and abstention separately.

## Tests

The deterministic tests cover:
- maximum over bodies and frames;
- upward quantization;
- input permutation invariance;
- missing samples, duplicate pairs and unexpected body IDs;
- non-finite/negative errors;
- invalid normalization and score digest;
- incomplete failed/aborted retention using the max sentinel;
- one block maximum across multiple trials;
- duplicate trial identity rejection;
- mixed model identity rejection;
- preservation of raw-trial and forecast digests in the block receipt;
- sentinel threshold becoming an unbounded pose set;
- calibration-receipt binding across complete source blocks and refusing a different dataset revision or raw-source manifest.

## Run

    cargo fmt --manifest-path research/gauge-contact-conformal-score-v0/Cargo.toml -- --check
    cargo test --offline --locked --manifest-path research/gauge-contact-conformal-score-v0/Cargo.toml
    cargo clippy --offline --locked --manifest-path research/gauge-contact-conformal-score-v0/Cargo.toml --all-targets -- -D warnings

The crate has one local dependency on #4147 and no third-party crates. It is kept in an isolated Cargo workspace.

## Claim ceiling

Synthetic contract and API composition only until the score inputs are produced by an independently admitted GAUGE pose evaluator and the raw source tree, frame semantics, rights, temporal observability, and exchangeability units are qualified. No real GAUGE trial has been scored here; no calibrated physical coverage, simulator fidelity, task improvement, safety, or industry superiority is claimed.
