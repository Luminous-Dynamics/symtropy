# ErgoStream affine clock-registration baselines v0.1

**Status:** candidate baseline code staged; exact-head Rust qualification pending.  
**Owner:** Symtropy / ERGO-STREAM-CLOCK-BASELINE-001 #7495.  
**Composes with:** multimodal microcosm #7442 / draft PR #7493, AV-STREAM #12837 and Symthaea ERGOstream preregistration #13402.

## Scope

The module `src/clock_estimators.rs` provides bounded, dependency-free affine clock estimators over public paired calibration-anchor timestamps.

It estimates:

`alpha_local_time ~= offset + scale * beta_local_time`

The estimator receives **measurement timestamps only**. It does not read:

- evaluator reference time or hidden clock parameters;
- event identities, modality association labels or generator regime;
- arrival tick, transport delay or future/held-out anchors.

The episode owner scores each frozen proposal against withheld anchors and evaluator-held reference parameters. This prevents transport delay from being fitted as device-clock skew.

## Estimators

1. **OLS affine regression:** least-squares reference for approximately Gaussian, low-contamination anchor noise.
2. **Theil–Sen:** median of pairwise slopes and median intercept; quadratic in anchor count, bounded to 128 anchors in v0.1.
3. **Huber IRLS:** Theil–Sen initialization, signed-residual MAD scale, Huber reweighting and bounded iteration count/tolerance.

The robust Huber implementation is a generic baseline, not a claimed reimplementation of MATH-SYNC. A future matched reproduction must follow MATH-SYNC's own inverse-affine convention, anchor-identifiability rules, weighting, scale update and evaluation cohort.

## Identifiability and numerics

- Fewer than two anchors returns `TooFewAnchors`.
- Duplicate anchor IDs are rejected.
- Repeated Beta timestamps / rank-deficient designs return `NonIdentifiableTimebase`.
- Non-positive clock scale and non-finite estimates fail closed.
- Input is bounded to 128 anchors; Huber iterations are bounded to 100.
- Timestamp coordinates are centered using integer differences before conversion to f64. `ClockFitV0` retains exact integer reference timestamps and a local intercept; use `predict_alpha_delta_v0` for a numerically stable prediction relative to the reference Alpha timestamp.
- The global `offset` field is a convenience f64 intercept and may lose sub-unit precision for extremely large absolute timestamp epochs. The reference-time representation is preferred for those cases.
- Residual RMS, median absolute residual and maximum absolute residual are computed on fitted anchors. They are not held-out validation metrics.

## Focused tests

Eleven tests currently cover:

1. exact affine OLS recovery;
2. exact Theil–Sen recovery;
3. resistance to a gross endpoint anchor outlier, compared with OLS;
4. one-anchor non-identifiability;
5. repeated Beta timestamp non-identifiability;
6. duplicate anchor identity rejection;
7. input-order invariance under robust fitting;
8. invalid Huber iteration/tolerance rejection;
9. large absolute timestamp epochs with preserved local rate;
10. rejection of negative clock rate;
11. rejection above the 128-anchor resource bound.

## Next validation and comparison gates

- Rust 1.96 format, locked tests and strict Clippy on the exact PR head.
- Independent known-answer fixtures with clean, jittered and gross-outlier anchors.
- Anchor-count and placement ladder (0/1/2/3/5/9/...) and held-out anchor blocks.
- OLS / Huber / Theil–Sen comparison under identical public anchors and cost accounting.
- False-identifiable and unnecessary-abstention metrics.
- Explicit out-of-order arrival with unchanged sensor-time truth.
- External matched reproduction of MATH-SYNC only when its assumptions and data protocol are available.

The module is a candidate-side estimator baseline. It does not certify clocks, provide sensor/timebase ownership, establish physical sensor synchronization, or infer semantic event correspondence.
