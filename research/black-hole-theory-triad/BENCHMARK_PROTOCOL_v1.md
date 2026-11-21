# Black-hole theory laboratory: benchmark protocol v1

**Status:** proposed benchmark contract; thresholds are targets, not achieved results.  
**Parent:** [BH-THEORY-TRIAD-001](../README.md)  
**Date:** 2026-11-17

## 1. Objective

Measure time-to-verified-scientific-accuracy across complete model-to-observable pipelines. A fast ray tracer attached to an unverified metric is not a superior gravity solver. A model that cannot state its action, matter coupling, frame, boundary conditions, observer and uncertainty must abstain from model ranking.

The benchmark compares GR, declared GR-EFT truncations, pure Weyl-squared gravity, explicitly specified scalar-conformal gravity, and named quadratic scalar-tensor theories. Chain Theory remains a separate discrete-foundation workstream until it supplies a controlled continuum map and field equations.

## 2. Paradigm shifts

1. **Theory identity as a type, not a label.** Every run carries action hash, equations, matter action, conformal frame, units, gauge, boundary conditions, solution branch and observer tetrad. "Weyl" is not a sufficient model identifier.
2. **Observables before pictures.** Fit closure quantities / visibility-domain data where available; reconstructed images are downstream products with their own priors and uncertainty. Never score a theory on a hand-picked image alone.
3. **Derive geometry and matter motion together.** Null propagation may be conformally invariant under stated conditions; massive matter generally is not. Scalar-coupled matter must follow its own action, not an assumed metric geodesic.
4. **Numerical correctness before speed.** Independent field-equation residuals, analytic limits, convergence order and invariant drift gate every performance result.
5. **Predictive discrimination, not retrospective curve fitting.** Freeze nuisance models and priors, seal holdouts, preregister scoring, include a null/abstention option, and publish failed runs.
6. **Compute as an experimental-design resource.** Select the next calculation by expected reduction in model uncertainty per wall-clock, memory and energy cost.
7. **Failure is data.** Singular charts, nonphysical branches, unstable modes, ill-conditioned inversions and non-identifiability must be emitted as typed outcomes, not silently filtered.

## 3. Benchmark ladder and target gates

These are proposed acceptance targets for the laboratory, not claims about current code.

| Gate | Test | Initial target | Stretch target |
|---|---|---:|---:|
| B0 Identity | Run manifest completeness | 100% required fields | Signed, content-addressed run capsule |
| B1 Analytic | Schwarzschild / exact limits | Relative error <= 1e-10 on well-conditioned fixtures | <= 1e-12 |
| B2 Field equations | Independent tensor residual | Convergence demonstrated; residual below declared discretization error | Two independent implementations agree within 3 sigma |
| B3 Geometry | Curvature, horizons, extremality | Root and invariant convergence under refinement | Certified root isolation and interval bounds |
| B4 Dynamics | Geodesic / action-derived trajectories | Conserved-quantity drift <= 1e-8 on reference orbits | <= 1e-10 without hiding stiffness |
| B5 Optics | Shadow / caustic / ray-bundle | Refinement-stable observables; error budget published | Adaptive cost near-linear on matched cases |
| B6 Perturbations | QNM poles, residues, retarded response | Independent reference agreement within stated truncation error | Uniform high-spin validation to a/M=0.99 where equations permit |
| B7 Inference | Coverage / predictive score | 90% and 95% interval coverage tested on simulations | Calibration robust to nuisance and prior shifts |
| B8 Generalization | Sealed OOD cases | False-acceptance measured; abstention allowed | Pre-registered blind prediction replicated externally |
| B9 Efficiency | Time, memory, energy per qualified result | Full cost ledger | Best verified accuracy per joule and engineering hour |

### Required reporting

- Absolute and relative error, observed convergence order, and conditioning.
- Wall-clock, CPU/GPU hours, peak memory, energy if measurable.
- Solver tolerances, failed steps, rejected roots and all abstentions.
- Coverage, calibration curves, proper scoring rules (e.g. log score / CRPS where applicable).
- False-positive model selection rate on synthetic truth-known cases.
- Exact source-data digests, code revision and reproducible command.

Do not collapse these into one opaque leaderboard score. Report a Pareto frontier and a separate qualification status.

## 4. Matched benchmark suites

### Suite A — analytic and tensor controls
- Schwarzschild and Kerr limits in GR.
- Mannheim–Kazanas static solution with convention-pinned parameters.
- Independent Bach-tensor evaluation for pure Weyl-squared vacuum.
- Coordinate transformation checks for scalar invariants.
- Horizon multiplicity and extremality checks with bracketed roots.
- Dimensional analysis and conformal-frame negative controls.

### Suite B — trajectories and optics
- Circular photon orbit and critical impact parameter.
- Finite-radius observer tetrads; no implicit observer at infinity for non-asymptotically-flat metrics.
- Null geodesic, lens map, caustic and photon-ring convergence.
- Timelike trajectories derived from each matter action.
- Plasma/radiative-transfer nuisance models held fixed across theory comparisons.

### Suite C — perturbations and response
- Test-scalar spectrum is labelled test-scalar, never gravitational ringdown.
- Operator-basis null tests: adding a redundant operator or applying a field redefinition that leaves on-shell observables unchanged must not create a physical first-order QNM shift.
- Theory-identity null tests: where two actions reduce to the same equations on the chosen background (for example via a Ricci-flat identity), independently computed spectra must agree within truncation and numerical uncertainty.
- Compute the same pole shifts with two independent algorithms (e.g. eigenvalue perturbation and continued-fraction methods) and require agreement across multipoles and overtones.
- For fourth-order gravity, identify the actual perturbation variables, constraints, extra modes and boundary conditions.
- Compare complex poles and residues, not frequencies alone.
- Check retarded response, stability and mode completeness where feasible.

### Suite D — inference and observational discrimination
- Synthetic injection/recovery with known truth.
- Blind theory recovery with identical data and nuisance assumptions.
- Visibility / closure-domain scoring before image-domain scoring.
- Prior predictive checks and prior sensitivity.
- Sealed epochs / sources; no post-hoc tuning against holdout.
- Model-class misspecification test: the system must be able to say "none of the candidates".

### Suite E — foundations bridge
- Chain Theory finite arithmetic reproduction with exact source and version pins.
- Independent reproduction of every finite claim before continuum interpretation.
- Positivity, normalization, unitarity / channel consistency, entropy inequalities and composition checks as applicable.
- Controlled continuum scaling with finite-size effects and universality analysis.
- Only after a continuum geometry and dynamics are derived: compare curvature, causal structure and black-hole observables.

## 5. Current external comparator map

| Capability | Relevant comparator | What we must match before claiming improvement |
|---|---|---|
| Adaptive stationary-metric shadow inference | Liu et al., PRD 114 (2026), DOI 10.1103/66x9-zj16 | Accuracy, scaling, posterior calibration, nuisance handling |
| Rotating solution discovery | Akribeia, Agurto-Sepúlveda et al., PRD accepted 12 Nov 2026, DOI 10.1103/3kz2-6dvv | Branch discovery rate, spectral refinement residual, high-precision convergence, parameter continuation, independent residual verification |
| Calibration-robust M87* morphology | Lai et al., Nature 659 (2026), DOI 10.1038/s41586-026-11241-9 | Closure-invariant likelihood, joint covariance, nuisance uncertainty, and direct forward-model comparison |
| Kerr ray tracing | AART v1.0.0 (released 16 Nov 2026) | Matched Kerr geodesics, lens maps, photon-ring sampling, visibility outputs, performance and convergence |
| Perturbations / self-force | BHPT Toolkit (26 packages) | Teukolsky, MST, QNM and flux reference fixtures |
| Numerical relativity | SXS / SpEC, Einstein Toolkit, SpECTRE | Matched initial data, convergence and waveform error |
| GW inference | Bilby / pyRing and LVK public analyses | Injection recovery, calibration and posterior coverage |
| Rotating Weyl geometry | Published Kerr-Weyl solution and thermodynamic analyses | Independent horizon, ergoregion and extremality reproduction |

Comparisons must be capability-specific. No single package is the baseline for every layer.

### Discovery-method caveat: Akribeia

Akribeia pairs a physics-informed neural-field search with pseudo-spectral Chebyshev refinement and extended precision. Its accepted 2026 paper demonstrates cohomogeneity-one, equal-angular-momenta rotating solutions in higher-dimensional Lovelock settings, validates against known five-dimensional solutions, and reports an extreme-precision Myers–Perry limit. The reported relative L2 figure of 10⁻¹⁶⁰ uses 300-digit arithmetic and should be treated as an author-reported result until independently reproduced. This is not a direct benchmark for four-dimensional pure Weyl gravity, scalar-conformal gravity, or generic axisymmetric PDEs. The reusable idea is the discovery/refinement split; each theory requires its own action, equations, regularity conditions, gauge, and boundary-value problem.

## 6. First blind challenge: M87* closure-domain discrimination

**Question:** Can any candidate model produce a statistically distinguishable, predictive improvement over GR once source, plasma, calibration and instrument uncertainties are treated consistently?

Protocol:
1. Freeze candidate actions, parameter priors, matter/radiation model and instrument likelihood before unsealing the evaluation data.
2. Use closure invariants / robust closure quantities where available. The research reference in [closure_invariants.rs](reference/closure_invariants.rs) implements the scalar co-polar reference-pinned advariant and common L2 normalization; independently verify it against the authors' implementation and real EHT data before using it in inference.
3. For theory comparison, generate predicted visibilities from ray-traced emission models and evaluate the measured closure-invariant likelihood directly. Do not insert a diffusion-model image prior into the gravity likelihood.
4. Carry the joint covariance of normalized advariants and reference-station choices. A diagonal chi-square over all normalized components is not automatically valid.
5. Generate matched synthetic datasets from each candidate plus out-of-family controls.
6. Measure injection recovery, parameter bias, interval coverage, false selection and abstention.
7. Evaluate held-out epochs or sources without retuning.
8. Publish likelihood code, synthetic truth, digests and a machine-readable result capsule.

**No model wins by matching the 42.2 ± 1.5 microarcsecond ring diameter alone.** Lai et al. (published 11 November 2026) report an independent closure-invariant reconstruction from public 2017, 2018 and 2021 EHT epochs; their nominal 6.5 ± 0.5 billion-solar-mass estimate depends on the adopted scaling calibration. The paper explicitly notes that near-unity closure-invariant reduced chi-square establishes self-consistency with measured invariants but does not prove the recovered morphology is absolutely correct. Their GenDIReCT reconstruction also uses a learned generative image prior. Our theory test should therefore forward-model the sky brightness for each candidate spacetime and compare the predicted visibilities/closure invariants directly with data, marginalizing emission and instrument nuisance parameters, rather than scoring a reconstructed image as if it were a direct measurement. Closure invariants remove selected station-based calibration degrees of freedom and absolute position/flux-scale information; they do not remove plasma, source-variability, finite-sampling or covariance uncertainty. The ring size by itself constrains a combination of mass, distance, spin, inclination, emission and gravity assumptions; it does not uniquely identify a theory.

## 7. Chain Theory firewall

Chain Theory is not yet a metric theory in this comparison. Maintain a separate staged bridge:
- C0: exact finite claims and arithmetic independently reproduced.
- C1: finite quantum consistency and composition.
- C2: derive a coarse-grained state space and observables without assuming spacetime.
- C3: demonstrate a controlled continuum limit and identify emergent causal structure.
- C4: derive an effective metric / field equations with quantified corrections.
- C5: only then enter the black-hole suites.

Failure at any stage is a useful result. Do not map graph distance to spacetime distance by analogy.

## 8. Immediate implementation sequence

1. Extend the existing static Mannheim–Kazanas reference kernel with bracketed horizon-root classification and dimensional/convention fixtures.
2. Cross-check the symmetry-reduced Bach residual in [BACH_REDUCTION.md](reference/BACH_REDUCTION.md) against a second independent full-tensor implementation; do not call the metric fully field-equation-qualified before this independent check.
3. Add finite-observer optical comparisons and cross-check the analytic photon-sphere fixtures.
4. Reproduce the published rotating Weyl horizons / ergoregions before attempting a full rotating ray tracer.
5. Build the closure-domain synthetic inference harness only by composing existing Symtropy observation and inference infrastructure; do not create a parallel imaging stack.
6. Prototype a discovery/certification split for rotating branches: neural or other global search may propose candidates, but only an independent high-precision refiner and off-grid residual suite can qualify them. Start on known Kerr / Myers–Perry / static controls before applying it to a new theory.
7. Add perturbation null tests and dual-algorithm pole comparisons before publishing modified-gravity QNM shifts.
8. Keep Chain Theory's finite-to-continuum bridge in Symthaea and link only qualified artifacts.

## 9. Claim discipline

- "Implemented" means code is present.
- "Structurally checked" means source and invariants were inspected.
- "Arithmetic checked" is not a numerical solver test.
- "Test authored" is not "test executed".
- "CI pending" is not pass.
- "Physical / observational validation" requires independent data and controls.
- Better-than-SOTA requires a matched baseline, same hardware/data/tolerances, uncertainty and full cost ledger.

The project's north-star metric is **verified scientific information gained per unit compute and human effort**, subject to hard correctness and calibration gates.
