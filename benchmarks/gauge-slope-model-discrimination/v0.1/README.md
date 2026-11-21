# GAUGE slope model discrimination v0.1

**Parent:** [GAUGE rigid-body qualification #6446](https://github.com/Luminous-Dynamics/symtropy/issues/6446)  
**Model-identifiability programme:** [#6584](https://github.com/Luminous-Dynamics/symtropy/issues/6584)  
**This increment:** [#6633](https://github.com/Luminous-Dynamics/symtropy/issues/6633)  
**Generic calibration owner:** [#3784](https://github.com/Luminous-Dynamics/symtropy/issues/3784)

## Research question

Can a constant-acceleration response be rejected when measured acceleration changes systematically with sliding speed, rather than allowing a solver to report a precise but structurally wrong constant-friction fit? A detected trend is evidence of speed-linked model discrepancy, not proof that friction is its unique cause.

This small GAUGE-specific reference compares two local model families at one fixed incline/article/material:

- M0 — constant friction, represented by a constant acceleration intercept: a(v) = b0.
- M1 — locally speed-dependent friction: a(v) = b0 + b1 v.

For a fixed, admitted effective incline angle phi, the latter corresponds locally to mu(v) = mu0 + k_v v, with b1 = -g k_v cos(phi). The intercept is deliberately free: it absorbs the constant acceleration level, including the effect of a constant friction level or constant angle-registration offset. The discriminator therefore tests whether speed explains additional variation after the best constant model has been fitted.

This is a local linear alternative, not a complete Stribeck/contact law. A non-zero slope is not uniquely attributable to friction: aerodynamic drag, speed-correlated geometry, changing contact patch, material heating/wear, camera-derived speed bias, release-condition confounding or an incorrect acceleration window can produce similar trends. Friction attribution requires additional independently varied interventions and appropriate competing models.

## Why this is a useful paradigm shift

Parameter identification asks: "What parameter values best fit the model I chose?"

Model discrimination asks: "Could a different physical mechanism explain the measurements materially better, even after both candidate models get to fit their own nuisance parameters?"

A low residual from one selected model cannot answer the second question. Symtropy should keep parameter uncertainty and model inadequacy as separate dispositions. A narrow interval around a parameter is not evidence that the model family is adequate.

The reference is intentionally narrow and composes with #3784. It does not introduce a second calibration engine, Bayesian inference library, general covariance toolkit, or active-policy optimizer.

## Executable reference

The dependency-free Rust library provides:

1. Inverse-variance weighted least squares for M0 and M1, using the supplied per-trial standard deviations as known measurement sigmas.
2. Null and alternative weighted chi-square residuals.
3. DeltaChi2 = Chi2(M0) - Chi2(M1). A positive result shows that the fitted linear-speed response explains more of these data under the declared likelihood; model complexity is penalized separately by BIC.

The v0.1 regression treats speed as an error-free predictor and the supplied acceleration sigmas as known; it ignores speed measurement uncertainty and speed/acceleration covariance. That is acceptable only for the frozen synthetic algebra reference. In a physical mocap pipeline, speed and acceleration are usually derived from the same position stream and their errors are correlated. Physical use therefore requires an errors-in-variables/latent-trajectory likelihood or independently measured speed, with the shared trajectory covariance propagated; the v0.1 fit must not be presented as a physically calibrated parameter estimator.
4. DeltaBIC = BIC(M1) - BIC(M0) = ln(n) - DeltaChi2, for the declared known-sigma Gaussian likelihood.
5. Residual degrees of freedom for the two-parameter alternative.
6. A profiled design-separation score after eliminating the unknown constant intercept.
7. Optional conversion from the measured acceleration-speed slope to k_v, but only when a caller supplies an explicitly qualified geometry record and its evidence digest.

### Profiled experiment-design score

For a candidate design with speeds v_i, acceleration uncertainties sigma_i, weights w_i = 1/sigma_i^2, and a hypothesized alternative slope beta:

lambda = beta^2 * sum_i w_i * (v_i - weighted_mean(v))^2

This is the squared standardized separation between the alternative's predicted response and its best-fitting constant response. It is a deterministic noncentrality/separation proxy under the declared diagonal Gaussian error model. It is not Bayesian expected information gain, a p-value, a posterior, or an assurance of physical detectability.

An equal-speed design has zero speed-discrimination information regardless of trial count: after profiling the intercept, its design matrix has no speed direction. At equal trial count and equal uncertainty, a wider safe speed range produces a larger separation score for a non-zero hypothesized slope.

### Observation independence

Each input row must be one independent, preregistered physical-trial aggregate with a unique trial ID. The crate rejects duplicate trial IDs. It does not accept adjacent 30-Hz frames as independent observations, estimate within-trial autocorrelation, or cluster by article/session. Physical evaluations must split by physical recording, article/material and session before fitting; resampling must follow those independent units.

### Geometry and unresolved source semantics

The raw GAUGE JSON profile and the parser do not by themselves resolve quaternion algebra, transform direction, handedness, frame IDs, Euler units, absolute clock origin, or the exact 180-to-30 Hz export filter/phase/timestamp map. This benchmark therefore estimates the acceleration-speed trend without deriving an absolute friction-speed coefficient by default.

The conversion helper returns no coefficient when qualified geometry is absent. Supplying QualifiedSlopeGeometry is a caller-side admission claim; the crate checks that the evidence digest is non-zero but does not itself authenticate or fetch that artifact. Do not instantiate it until the angle units, frame and gravity evidence have passed their owning gates.

The 30-Hz released trajectory cadence is export cadence, not the original 180-Hz camera rate. Do not infer sub-frame impact dynamics or reconstruct lost high-frequency information. See [temporal observability #6587](https://github.com/Luminous-Dynamics/symtropy/pull/6587).

## Prospective physical experiment

A prospective fixed-article campaign could vary release height while retaining the same incline, article, material, orientation and measurement region. A block released from rest at different heights can pass through a common downstream region at different local speeds without an externally applied push. Freeze the region and the acceleration/speed summary before collecting trials; retain one summary per independent physical trial.

This is a proposed protocol, not a claim that the released GAUGE records already contain the necessary speed spread or initial-condition metadata. It must first pass rights, raw-record, sampling, geometry, measurement-noise and repeatability gates.

The physical campaign should use:

- a friction-attribution factorial arm that varies surface/material or controlled normal load independently of release speed, where physically feasible;
- an air-drag / speed-measurement negative-control analysis where geometry and object scale make it relevant;

- repeated-speed controls to verify the null design's lack of model-discrimination power;
- low/mid/high speed strata selected within a safe and observable regime;
- equal physical-trial, measurement-channel, duration and compute budgets across designs;
- independent article/material/session holdouts, with the holdout unsealed only after thresholds and analysis are frozen;
- explicit accounting for failed, incomplete and unsafe trials.

## Synthetic tests

The Rust unit tests cover:

- a synthetic speed-dependent response with signed slope recovery and M1 preference;
- an exactly constant response where the added slope is not rewarded by BIC;
- repeated-speed design rejection;
- wider-versus-narrower speed spread at equal trial count;
- the inverse-square uncertainty scaling of the design separation;
- duplicate physical-trial rejection to prevent pseudoreplication;
- invalid negative speed;
- withholding physical coefficient conversion when geometry evidence is absent or invalid;
- input-order invariance.

These are software and algebra controls only. They are not GAUGE measurement results or physical validation.

## Industry / state-of-the-art comparison

Symtropy should compete on model-family falsification and evidence validity, not on the claim that it has invented system identification:

- **NVIDIA Isaac Sim SysID** already provides a gated Robot/Data/Parameters/Check/Solve/Results workflow, telemetry and identifiability checks, held-out validation chunks, and a known-truth sim-to-sim recovery tutorial. The documentation explicitly distinguishes matched-model recovery from physical fidelity. Use this as an implementation/protocol baseline: [SysID documentation](https://docs.isaacsim.omniverse.nvidia.com/latest/robot_setup/ext_isaacsim_robot_setup_sysid.html), [known-truth recovery tutorial](https://docs.isaacsim.omniverse.nvidia.com/latest/robot_setup_tutorials/tutorial_system_identification_simsim.html).
- **MuJoCo SysID** already supplies bounded nonlinear least squares, Gauss-Newton with finite-difference sensitivities, batched rollouts, parameter bounds and confidence-interval reporting: [official toolbox README](https://github.com/google-deepmind/mujoco/blob/main/python/mujoco/sysid/README.md).
- **IDTD** (arXiv:2610.10905 v2, revised 2026-10-27) reports a 39.6% average reduction against its strongest prior active-exploration baseline across the paper's evaluated settings; this is an author-reported aggregate, not a universal per-platform win. The detailed comparisons include a Crazyflie case where E-optimal design reports lower parameter error than IDTD (1.59% versus 2.11%). On the physical Booster K1 humanoid, IDTD improves held-out open-loop prediction over Passive and SPI-Active, but the authors report that downstream control improvements remain limited. These qualifications matter: trajectory design must be evaluated separately for parameter recovery, held-out prediction and downstream task/control transfer. This reference does not reproduce IDTD's full policy search, CMA-ES, simulator stack or robot trials: [paper](https://arxiv.org/abs/2610.10905), [detailed paper rendering](https://www.alphaxiv.org/abs/2610.10905).
- **GAUGE** supplies measurement-grounded trajectories, calibrated physical metadata, uncertainty annotations and task-specific observables across eight rigid-body families and additional deformable regimes. Its paper reports that no evaluated engine is uniformly faithful and that impulsive contact remains especially difficult: [paper](https://arxiv.org/abs/2608.05948), [dataset card](https://huggingface.co/datasets/InternRobotics/GAUGE-Dataset).

The competitive path is to reproduce the established baselines under matched constraints, then demonstrate that the model-discrimination gate catches false confidence on blind physical trials without harming held-out predictions or downstream task performance.

## Run

From the repository root:

```bash
cargo fmt --manifest-path benchmarks/gauge-slope-model-discrimination/v0.1/Cargo.toml -- --check
cargo test --offline --locked --manifest-path benchmarks/gauge-slope-model-discrimination/v0.1/Cargo.toml
cargo clippy --offline --manifest-path benchmarks/gauge-slope-model-discrimination/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

The crate has no third-party dependencies and is kept in its own Cargo workspace.

## Claim ceiling

This v0.1 is a synthetic fixed-angle, diagonal-Gaussian reference for detecting a local acceleration-speed trend. It does not prove that real GAUGE slope-slider friction is speed-dependent; establish a physical friction coefficient from unresolved metadata; deliver a posterior or calibrated confidence coverage; identify full Stribeck/static/kinetic/directional/temperature behaviour; replace #3784; qualify Symtropy's solver; or demonstrate physical or industry superiority.
