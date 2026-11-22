# Akribeia source-faithful reproducibility audit

**Audit date:** 22 November 2026  
**Scope:** public paper, tagged orchestration repository, and the tagged neural/spectral dependencies. This is a source/configuration audit, not a reproduced Akribeia run.

## 1. Public implementation is now available

The current Akribeia paper identifies three reproducibility repositories:

- [Akribeia v1.0.0](https://github.com/moyarzoca/Akribeia/tree/v1.0.0)
- [DiffLearning v2.1.1](https://github.com/moyarzoca/difflearning/tree/v2.1.1)
- [Deterministic Gauss–Newton Solver v1.1.1](https://github.com/moyarzoca/deterministic-gauss-newton-wl/tree/v1.1.1)

The paper is [arXiv:2607.07418v2](https://arxiv.org/abs/2607.07418), revised 19 November 2026 and accepted by Physical Review D on 12 November 2026. Its code references are [57–60 in the paper](https://borowiecki.dev/pdf/2607.07418).

Akribeia is not one monolithic solver: the application repository provides the gravity equations, boundary conditions, reconstruction, training/refinement drivers, and orchestration; DiffLearning supplies the PINN discovery; the Wolfram Language repository supplies deterministic pseudo-spectral Gauss–Newton refinement.

## 2. Version-pin discrepancy requiring resolution

The paper's reference list points to DiffLearning v2.1.1 and the deterministic solver v1.1.1. However, the installation instructions in the Akribeia v1.0.0 README clone DiffLearning v2.1.0 and the deterministic solver v1.0.0.

This is a reproducibility ambiguity, not proof of a defect. A source-faithful benchmark must record which manifest is actually run:

| Manifest | DiffLearning | Deterministic solver |
|---|---|---|
| Paper reference list | v2.1.1 | v1.1.1 |
| Akribeia v1.0.0 README | v2.1.0 | v1.0.0 |

Do not silently mix files or dependencies from the two versions. The first replication should run the paper-referenced pins; a second compatibility run should use the README pins if both tags are available.

## 3. What the released 5D drivers actually configure

The Akribeia 5D shell driver configures a parametric neural-field run with:

- horizon radius parameter fixed to (r_H=1);
- radial compact coordinate (x\in[0,1.570708]), sampled with a Chebyshev distribution;
- EGB coupling input (\alpha\in[0,3]), sampled uniformly;
- stage 1: up to 60,000 training steps and (N_1=6,000) collocation points;
- stage 2: 5,000 steps and (N_2=16,000);
- stage 2 initialized from the stage-1 best checkpoint.

The paper describes a fully connected network with three hidden layers of 20 neurons each. It reports Adam warm-up/stable/decay followed by L-BFGS, with the neural discovery residual reaching approximately (10^{-3}) before spectral refinement.

The released orchestration script `akribeia.sh` selects the 5D stage-2 neural checkpoint and refines one fixed configuration:
[
\alpha=1,\qquad \Omega_H=0.33,\qquad \text{working precision override}=310.
]
The refiner's default configuration has (\text{Pre}=300) and a multi-stage spectral grid with (N_{\rm points}=20,20,50,100,200); the wrapper overrides the precision with `--pre 310`.

**Interpretation:** the neural network represents a continuous coupling family, but the published orchestration script's 5D spectral refinement is a fixed-coupling job. Do not describe every point in (\alpha\in[0,3]) as having received the same high-precision refiner pass unless independent run artifacts show that.

The Akribeia README requires Python 3.12+, uv, and Wolfram Engine / Mathematica 14.3+. The deterministic solver README likewise names Wolfram 14.3+. Record the exact runtime, package lock, hardware, and Wolfram version in any reproduced run.

## 4. Residual and precision metrics must not be conflated

The paper reports a relative (L^2) profile error of (10^{-85}) against the exact Myers–Perry solution in the Einstein control, using 300-digit extended precision. This is not the same quantity as a per-equation field residual, a held-out-node residual, a full-domain error bound, or an interval enclosure.

The deterministic solver README describes a homogeneous-grid residual check as disabled by default unless an `Abort[]` is removed, while the tagged `gauss-newton.wl` entrypoint calls the homogeneous-grid validator. This documentation/entrypoint tension must be resolved by tracing the validator path and recording a run that proves the check actually executed. Until then, Symtropy's independent residual evaluator should remain a separate gate.

## 5. Fair Symtropy comparison

The current Symtropy branch is an (O(a^2)) slow-rotation perturbation system. Akribeia's headline 5D rotating cases are full nonlinear stationary profiles. These are not the same problem at finite spin.

The fair near-term comparison is:

1. Reproduce the Akribeia-tagged EGB profile at the exact matching conventions and parameters.
2. Obtain full nonlinear reference profiles at at least three decreasing values of (Omega_H).
3. Extract (B_2,F_2,H_2) from the small-spin coefficients, keeping (r_H), action normalization, and coupling conventions fixed.
4. Compare the extracted coefficients against Symtropy's (O(a^2)) equations, horizons, infinity data, independent unscaled equations, and Noether constraint.
5. Demonstrate the expected truncation scaling against (O(a^4)) contamination before calling it a matched perturbative result.

A direct finite-spin comparison at (Omega_H=0.33) requires a full nonlinear Symtropy solver.

## 6. Better-than-SOTA opportunity

The differentiator should be **candidate discovery plus mathematical qualification**, not a claim that a spectral refiner can simply emit more digits.

The new benchmark should report a two-dimensional frontier:

- discovery: time-to-candidate, success over frozen starts, branch/parameter coverage, time-to-residual, precision, memory, and compute;
- qualification: independent original-equation residuals, endpoint mode counts, boundary-map rank, full-domain interval residual/tail/inverse bounds, existence/local uniqueness, validated continuation length, and physical hyperbolicity.

Additional branch metrics should include missed/false folds, duplicate roots, branch-event classification error, and the fraction of discovered branch length that is actually validated. Publish all failures and all candidate rejections.

Symtropy should treat PINNs as optional seed proposers. Its exact action reduction, horizon jets, Noether checks, independent evaluator, and interval proof layer should remain authoritative. This creates a credible route to outperforming the current discovery/refinement state of the art on a different and more demanding axis: **machine-replayable, proof-qualified solution-space knowledge**.

## 7. Evidence boundary

This audit inspected public tagged files and paper-referenced repository versions. It did not run Wolfram Engine, DiffLearning, Akribeia training, the spectral refiner, or a matched black-hole benchmark. No performance or superiority result is claimed.
