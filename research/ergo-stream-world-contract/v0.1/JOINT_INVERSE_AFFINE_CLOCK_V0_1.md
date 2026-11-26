# Joint inverse-affine clock reconstruction v0.1

**Status:** executable reference module staged; exact-head Rust qualification and differential validation pending.  
**Owner:** Symtropy / ERGO-STREAM-MATHSYNC-REPRO-001 #7518.  
**Composes with:** paired-anchor estimators #7495 / PR #7498, multimodal microcosm #7442 / PR #7493, AV-SyncBench adapter #7516, and AV-STREAM #12837 sensor/timebase ownership.

## Why this is a separate estimator

The paired-anchor module estimates an affine map from observed timestamp pairs. MATH-SYNC solves a different problem: given **known event correspondences** across heterogeneous sensor streams, jointly estimate latent common event times and each stream's inverse affine clock.

For stream i and event e:

`t[i,e] = a[i] * tau[e] + b[i] + epsilon[i,e]`

The inverse parameterization is:

`tau[e] = gamma[i] * t[i,e] + kappa[i]`

Choose one reference stream r and fix the temporal gauge:

`gamma[r] = 1; kappa[r] = 0`

The unknown vector is the event times plus two inverse-clock parameters for each non-reference stream, so `p = M + 2(S - 1)`. A reference observation contributes the row `tau[e] = t[r,e]`; a non-reference observation contributes `tau[e] - gamma[i] * t[i,e] - kappa[i] = 0`. Missing event markers simply omit rows.

The unique common-time reconstruction is structurally identifiable only when the gauge-fixed design matrix has full column rank. A connected-looking event/stream graph is not by itself a uniqueness proof.

## Implemented module

Exported as `joint_clock` from the microcosm crate in `src/joint_clock.rs`.

- Explicit versioned problem type with declared stream IDs, event IDs, reference stream and sparse timestamp observations.
- Deterministic event/stream parameter ordering and row-to-observation construction.
- Duplicate IDs, unknown stream/event references, duplicate stream/event observations, and bounded v0 resource limits are checked before matrix construction.
- Column-scaled, pivoted Householder QR supplies a numerical rank decision and least-squares solution. The rank tolerance is explicit in the design report; it is not advertised as exact symbolic rank.
- A deficient matrix returns `RankDeficient { rank, parameters }`; the solver does not regularize an underdetermined design into a false unique clock.
- OLS joint fit and Huber IRLS joint fit. Huber uses c=1.345, MAD scale floor 1e-6, at most 30 iterations, and relative parameter update tolerance 1e-10.
- Outputs latent event times, inverse affine clock maps, observation/parameter/rank diagnostics, in-sample residual metrics, robust scale and iteration/convergence status.
- No arrival tick, delivery order, transport-delay field, hidden timestamp truth, or outlier label enters the estimator.

## Tests staged

Eight unit tests currently cover:

1. exact two-stream joint OLS recovery of latent event times and inverse clock map;
2. Huber recovery under one gross timestamp outlier compared with OLS;
3. rank-deficiency detection for one shared event across two affine clocks;
4. sparse missing event observations without interpolation;
5. three streams with different native rates and a missing marker;
6. invariance to input row order;
7. duplicate stream/event observation rejection;
8. reference-stream identity gauge.

These are tiny known-answer synthetic fixtures. They are not the published MATH-SYNC cohort or Monte Carlo reproduction.

## Fidelity boundary

The implementation follows the published inverse-affine joint problem and the headline Huber settings, but does **not yet claim an exact MATH-SYNC reproduction**. In particular:

- The rank decision currently uses a column-scaled pivoted QR with a declared floating-point tolerance; the paper's rank theorem is an exact mathematical condition, and numerical tolerance must be separately validated.
- The reference release's exact design-row ordering, scale update details, condition-number computation, stopping metric and random data-generation implementation still require differential verification against the archived source at Zenodo DOI 10.5281/zenodo.23106783.
- The current resource cap (8 streams, 128 events, 2,048 observations) is a bounded Rust reference setting, not the paper's scalability claim.
- Harvard/WESAD replay, Monte Carlo factors, and external model/participant statistics are not implemented.
- Correct event correspondence is an input precondition. Wrong semantic event identities are not recovered by this module.
- Semi-synthetic schedule replay is not independently instrumented hardware synchronization.

## Next validation gates

1. Rust 1.96 format, locked tests, doctests and strict Clippy on the exact PR head.
2. Independent known-answer matrix fixtures and differential rank tests against a separate LAPACK/SVD or verified reference implementation.
3. Differential comparisons with the exact archived MATH-SYNC version, including row/column layout, Huber scale and stopping rule.
4. Reproduce the paper's controlled six-stream multirate simulation, seed 20260821, 24 irregular anchors over 120 seconds, 50 paired replicates per scenario, and clean/nominal/stress factors only after the exact code and manifests are pinned.
5. Reproduce participant-level Harvard and WESAD schedule-replay tables only from their released manifests, scripts and licenses. Participant is the inferential unit, not Monte Carlo replication.
6. Separate rank/identifiability, latent-event reconstruction, inverse-clock recovery, robustness, cost and native-rate preservation.
7. Factor clock recovery across aligned, shuffled and ambiguous event correspondences using #7493; keep MATH-SYNC's known-correspondence estimator score separate from event association.

No physical synchronization, semantic matching, causal inference, structural transfer, or SOTA-superiority claim follows from this synthetic module.
