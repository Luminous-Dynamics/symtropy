# MATH-SYNC v1.1 reproduction delta for ErgoStream

**Status:** protocol addendum; no exact reproduction claim.  
**Pinned source:** Hechavarría-Hernández et al., *MATH-SYNC: Robust and Identifiable Affine-Clock Alignment of Heterogeneous Multirate Sensor Streams*, Mathematics 14(20), 3720 (2026), https://doi.org/10.3390/math14203720.  
**Archive:** Zenodo DOI https://doi.org/10.5281/zenodo.23106783, version 1.1, published 2026-10-02.  
**Release:** https://github.com/jrhechavarriah/MATH_SYNC_v1.0/releases/tag/v1.1  
**Archive SHA-256 as published:** \`7f029bc9b7f892e65dcfa29fd4241eb362a8eb223356ab0037b21a7d852ea3a1\`  
**Owner:** Symtropy #7518. Active acquisition: #7529. Decision-sufficient event disambiguation: #7555.

The v1.1 release adds frozen G12 experiment specifications and supplementary analyses beyond the v1.0 baseline. This addendum updates the reproduction contract; it does not state that the archive has been downloaded, byte-verified, or executed by ErgoStream. The checksum above is transcribed from the release record and must be checked against the downloaded artifact before a reproduction run.

## 1. Scientific boundary

MATH-SYNC addresses joint affine-clock registration when synchronization-event correspondences are already known. It estimates latent common event times and inverse affine clock parameters after fixing a reference-stream temporal gauge.

For stream \(i\) and event \(e\):

\[
t_{i,e}=a_i\tau_e+b_i+\epsilon_{i,e},
\qquad
\tau_e=\gamma_i t_{i,e}+\kappa_i.
\]

For a reference stream \(r\), fix \(\gamma_r=1\) and \(\kappa_r=0\). With \(M\) events and \(S\) streams, the gauge-fixed parameter count is \(p=M+2(S-1)\). Full column rank of the design is the mathematical identifiability condition. A low residual does not establish uniqueness.

This estimator does **not** solve semantic event identity, arbitrary cross-modal association, causal direction, or physical hardware synchronization. ErgoStream must keep those outcomes separate:

- clock identifiability and clock parameter recovery;
- event correspondence and set-valued ambiguity;
- structural transfer validity;
- downstream decision quality.

A wrong event correspondence must be reported as an upstream data-association failure, not absorbed into clock-estimator quality.

## 2. Frozen v1.1 experiments to reproduce

### G12-A — robust estimator comparison

Reproduce the published clean, nominal and stress strata with the published estimator set: OLS, Huber, LAD, Tukey and RANSAC. Use paired worlds and preserve each method's failures, convergence status and runtime.

The article reports 50 paired replicates per stratum. In its reported implementation, OLS and Huber are near parity in clean timing; under nominal and stress contamination Huber materially reduces error. LAD is a useful robust comparator. Tukey's failures and RANSAC's substantially higher runtime are part of the result, not rows to omit.

ErgoStream must pin the exact source routine, data-generation distributions, seed handling, scale calculation, initialization, stopping rule and score aggregation before trying to reproduce the table. A similarly named estimator is not a faithful reproduction.

### G12-B — anchor count and temporal span

Reproduce the crossed design:

- anchor counts: 4, 6, 8, 12, 16 and 24;
- temporal spans: 25%, 50% and 100% of the acquisition interval;
- 50 paired replicates per configuration.

Preserve and report rank, smallest singular value, condition diagnostic and reconstruction error. Do not reduce this experiment to residual RMS.

The published analysis reports rank failures concentrated at low anchor counts and substantial improvements from wider temporal spread. This is an explicit warning that anchor count alone is not a sufficient design descriptor.

### G12-C — reference-stream sensitivity

Reproduce the six reference choices: event/reference, EEG, PPG, EDA, IMU and HMD. Include clean and distorted-reference cases, preserving the study's canonical-coordinate transformation when comparing different gauges.

The published distorted-reference experiment applies a sinusoidal perturbation of approximately 20 ms to selected reference anchors:

\[
\delta(t)=0.020\sin\left(\frac{2\pi(t-3)}{114}\right).
\]

The evaluator may use hidden truth to express estimates in a common canonical coordinate; the candidate estimator must not receive that truth. Keep the distortion mechanism and all anchor membership details pinned to the source release.

A reference stream is a coordinate gauge, not a privileged truth source. If its measurements are corrupted, more calibration observations can make the result more confidently wrong. The required ErgoStream outcome is a quantified sensitivity result and, where the available evidence cannot distinguish reference corruption from model mismatch, an explicit unresolved/abstain disposition.

### G12-D — convergence tolerance and compute scaling

Reproduce the published Huber relative-update tolerance sweep:

- \(10^{-4}\), \(10^{-6}\), \(10^{-8}\);
- canonical protocol \(10^{-10}\);
- 50 paired trials per tolerance configuration.

Record convergence rate, iteration count, final parameter error and held-out event-time error. A tolerance that makes the loop stop earlier is not better if it worsens recovery or hides non-convergence.

Reproduce the scaling matrix:

- stream counts: 5, 10, 20 and 40;
- anchor/event counts: 8, 16, 32, 64 and 128;
- 50 trials per configuration.

Preserve the full observation count and parameter count for every run. The publication reports a largest configuration of 40 streams and 128 anchors, with 206 unknowns and 4,622 observations. Its wall-clock number is a study-machine result, not a portable real-time guarantee.

## 3. Required numerical differential validation

The current ErgoStream joint estimator uses column-scaled, pivoted Householder QR with an explicit floating-point rank tolerance. Its pivot ratio is not an SVD condition number.

This branch adds an independent small-matrix audit:
- reconstruct the gauge-fixed design independently from the public problem contract;
- column-scale it independently;
- compute singular values with a one-sided Jacobi SVD that does not form \(A^\mathsf{T}A\);
- compare QR rank and full-column-rank disposition against the independent SVD on all non-empty anchor subsets of a small two-stream/three-event design;
- include a full three-stream fixture and a known rank-deficient fixture.

This is a numerical cross-check for bounded fixtures, not an exact symbolic-rank proof or a substitute for validating the floating-point tolerance against the upstream implementation. A same-threshold QR/SVD agreement does not establish that the chosen threshold is scientifically appropriate for every timestamp scale or noise regime.

Before claiming broader numerical qualification, add:
1. adversarial near-rank-deficient designs at multiple timestamp epochs and column scales;
2. independently derived exact-rank fixtures for small integer matrices;
3. differential tests against a pinned mature SVD/LAPACK implementation;
4. smallest-singular-value and condition-spectrum outputs with declared parameter scaling and units;
5. perturbation tests where timestamp contamination changes the design matrix itself, not merely the response vector.

Do not substitute QR pivot ratio for \(\sigma_{\min}\) or \(\kappa(A)\).

## 4. Update to the ErgoStream implementation envelope

The current v0.1 Rust module is bounded to:
- 8 streams;
- 128 declared events;
- 2,048 observations.

The v1.1 scale study reaches 40 streams, 128 anchors, 206 unknowns and 4,622 observations. Therefore the current module is a bounded reference, not a scaling reproduction.

A separate scale qualification should extend the implementation and allocation bounds only after the small-instance numerical audit passes. The target is at least the complete published envelope (40 streams, 128 events/anchors, 4,622 observations, 206 unknowns), followed by larger stress configurations. Report memory, factorization work, runtime distribution, allocations, and failure/timeout denominators. Do not advertise the current 8-stream/2,048-observation limit as reproducing the paper's scaling result.

## 5. Active calibration must follow—not replace—the static reproduction

Issue #7529 proposes selecting the next known-correspondence calibration observation. The correct order is:

1. Reproduce G12-B's passive anchor-count/span matrix.
2. Reproduce G12-C reference-stream sensitivity and add reference-corruption detection/abstention controls.
3. Reproduce G12-D tolerance/scaling behavior.
4. Validate rank and singular-value diagnostics independently.
5. Only then compare passive, FIFO, first-two, random, uniform-coverage, rank-greedy, E-optimal, D-optimal and cost-aware active policies.

Each policy must receive the same public candidate set, response latency distribution, available-observation budget and compute budget. The evaluator owns hidden future observations and true clock parameters. Keep failed, unavailable, late, duplicate and rejected requests in the denominator.

Measure at least:
- requests to first full-rank design;
- full-rank rate at a fixed request and cost budget;
- \(\sigma_{\min}\), condition spectrum and numerical rank under a declared scaling;
- held-out event-time and inverse-clock error;
- coverage/calibration of uncertainty intervals when available;
- false-identifiable and unnecessary-abstention rates;
- request cost, wait, energy, runtime and memory;
- tail performance by generator family.

The active policy may not improve query count by increasing false-identifiable claims or accepting a corrupted reference. A model-invalidity negative control must test whether more observations are misleading.

## 6. Dataset and inferential-unit rules

Harvard and WESAD schedule replays remain semi-synthetic. They can test published replay protocols and robustness against sensor sampling schedules, but they are not ground-truth physical clock measurements.

For participant-level results:
- pin exact public dataset versions, access route, license and preprocessing;
- reproduce released participant and schedule manifests;
- preserve inclusion/exclusion and missing-data accounting;
- treat participants—not windows, timestamps, or Monte Carlo repetitions—as the inferential unit;
- use participant-clustered uncertainty intervals;
- report every participant-level outcome and missing/failed replay.

Do not claim an exact external reproduction until archived code, dependency/environment lock, data manifests, random seeds, table generation and score semantics are all pinned and differentially checked.

## 7. AV-SyncBench remains a separate external lane

AV-SyncBench measures pairwise consistency on real audio/video under temporal and semantic perturbations. Its reported Synchformer and SparseSync overall numbers are pairwise accuracies, not clock-offset classification scores. ErgoStream synthetic F1, active-query efficiency and MATH-SYNC clock RMSE must not be compared numerically against those scores.

Use #7516 to reproduce the official media/model/preprocessing/score pipeline before comparing candidates. Keep the official temporal and semantic tracks separate. Respect model, code, media and checkpoint licenses; do not copy or redistribute upstream media or weights into this Rust research crate.

## 8. Claim ladder

| Evidence obtained | Permitted claim |
|---|---|
| Tiny exact known-answer fixtures | Correct for declared finite fixtures |
| Independent QR/SVD differential | Rank implementation agrees on tested designs and tolerance |
| Pinned v1.1 G12 reproduction | Reproduces the specified MATH-SYNC experiments within preregistered tolerance |
| Harvard/WESAD participant replay | Reproduces the specified semi-synthetic schedule replay |
| Active policy beats matched passive policies | Lower measured acquisition cost at matched identifiability and held-out error on tested families |
| External sensor experiment with independent timing instrumentation | Physical synchronization accuracy for the tested hardware/protocol |
| Sealed-family improvements with matched compute/data budgets | Evidence of generalization beyond the development generator families |

No row implies the next row. In particular, successful CI, low in-sample residual, full rank, or synthetic query efficiency alone does not establish SOTA, real-time performance, physical synchronization, semantic identity, causality, or general intelligence.

## 9. Immediate sequence

- Validate the independent Jacobi-SVD integration tests on the exact branch head.
- Resolve every numerical disagreement before expanding the rank policy.
- Pin and inspect the full v1.1 archive, source revision, manifests and dependency lock; compare its actual implementation to the Rust equations line by line.
- Extend the differential fixture suite to near-rank-deficient and reference-corrupted designs.
- Add the full G12-B/C/D static reproduction harness.
- Execute G12-A estimator comparison, then participant schedule replays.
- Connect the qualified evaluator to a real Symthaea child acquisition policy only after evaluator/candidate data separation and output-freeze tests pass.

Until those gates are met, describe the joint Rust module as a MATH-SYNC-style reference formulation, not an exact MATH-SYNC reproduction.
