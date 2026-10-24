# Causal Transition-Residual Benchmark for Symthaea × Symtropy (v0.1)

**Status:** research contract + target-extraction primitive; implementation and Rust tests are committed on the branch for this draft, but compilation and the benchmark have not been verified in this environment.

## Why this is the next step

The current bounded-residual experiment is a useful control-blending smoke test, but its controller is trained by imitation of Symthaea's PD/PID command targets. It is not a model of residual vehicle dynamics. The distinction matters: adding an imitation-trained command to PID cannot establish that Symthaea has learned wind, payload, actuator lag, or a physics-model discrepancy.

The next hypothesis is more testable:

> A compact causal model of the *one-step dynamics residual*, paired with calibrated predictive uncertainty, can improve a nominal controller's predictions and disturbance rejection under held-out dynamics shifts at a measured compute budget.

This is a hypothesis, not a result. The prior target-collection PR adds the causal supervised-target definition; this follow-on adds a transparent fixed-scale ridge-regression baseline and a held-out one-step smoke evaluator, but not a learned Symthaea residual head, calibrated uncertainty model, or MPC controller.

## Target definition

For consecutive states observed at times t and t+dt, using the action applied over that interval:

- Observed world-frame acceleration: a_obs = (v[t+dt] - v[t]) / dt.
- Transparent nominal prediction: a_nom = (thrust/m) R(q[t]) e_z - (c/m) v[t] + [0, 0, -g].
- Supervised residual target: r = a_obs - a_nom.

The initial implementation is in `integration/symthaea-physics-loop/src/transition_residual.rs`. It:

- derives dt from monotonically increasing state timestamps rather than a caller-supplied interval;
- validates model parameters, finite state/command values, and quaternion norm;
- uses the actuator-clamped thrust, not an unclamped policy proposal;
- records observed acceleration, nominal acceleration, and residual separately;
- does not consume simulator-only wind / force labels.

The companion collector is `integration/symthaea-physics-loop/examples/causal_transition_dataset.rs`. It uses a classical PID baseline only as a data-collection policy, applies deterministic training-only exogenous force families, and writes one-step samples containing prior state, applied command, next state, and derived target. Exact hidden force values are omitted from per-sample records. The raw-state ridge and frozen HDC/LTC reservoir evaluators consume the same training artifact and share the held-out test seed/scenario convention, enabling a first representation comparison without changing the plant or controller. Collection uses a crossed design: each of the four training setpoint scenarios is run under each of the four disturbance families. The CI smoke uses 16 episodes to exercise the full 4×4 factorial rather than permanently confounding one setpoint with one disturbance. A jq gate checks that every scenario/family cell has representation, alongside sample counts, split membership, timestamp order, vector dimensions, model/target schema versions and source receipt formats; the dataset and held-out ridge receipt are uploaded as artifacts.

The target is deliberately a model discrepancy bucket. It can include external disturbances, attitude changes during the interval, integration mismatch, actuator/model mismatch, and measurement noise. The linear/ridge model uses fixed, explicit feature scaling and a three-output regularized least-squares solve. Its synthetic unit test checks recovery on a known linear mapping; that does not measure performance on simulator data. The paired evaluator fits from the exported train-only dataset and scores only predeclared test-split transition targets, with zero-residual (physics-only) prediction as a reference. Its smoke thresholds validate provenance and finite metrics, not that ridge wins. Do not label it “wind estimate” without a separately identified disturbance-observer design and supporting tests. The currently used nominal model is simple; parameters must be frozen before test evaluation and included in the experiment receipt. The dataset collector does not train a predictor, and a successful schema smoke would demonstrate plumbing only—not residual-model accuracy or control improvement.

## Data contract for the next executable increment

Each transition record should carry:

- `episode_id`, split, scenario ID and family, seed, transition index;
- prior and next timestamps, prior state, applied action, next state;
- nominal-model parameter digest or version ID plus exact numeric parameter values, and residual-target algorithm version;
- simulator and source SHAs, Rust compiler version, and state/action schema version;
- target vector and explicit invalid-sample reason when rejected.

The learning API must not see evaluation-only disturbance amplitudes, hidden simulator forces, future states, target split labels as features, or post-transition values when predicting the next transition. Split by scenario family and randomized dynamics parameters, not random individual frames: neighboring time steps are correlated and random-row splitting leaks trajectories.

## Paradigm shift: keep identification separate from control

**Stage A — identify dynamics.** Predict a distribution over the next-state residual from causal history (state, applied action, elapsed time and optionally a bounded recent residual window). Start with a one-step target and add multi-step rollout loss only after one-step calibration is credible. A Symthaea HDC/LTC model is one candidate, not a presumed winner.

**Stage B — calibrate epistemic and aleatoric uncertainty.** Compare predicted variance/intervals with errors on held-out scenario families. Report calibration by stress family and shift severity; an aggregate score may conceal dangerous under-coverage. A deep ensemble or simple residual model is a stronger baseline than pretending a single point prediction is uncertainty-aware.

An initial nonparametric benchmark is now implemented in `src/episode_conformal.rs` and wired into `examples/causal_residual_ridge_benchmark.rs`: calibrate a Euclidean radius for the 3D residual-error vector using the maximum one-step prediction error from each *validation episode*. This treats episodes—not highly autocorrelated frames—as calibration units and applies the finite-sample corrected split-conformal rank. The evaluator tests whether all steps of each held-out test episode fit inside that radius, along with ordinary step coverage, interval radius/diameter, and family-specific coverage.

Important boundary: split conformal's finite-sample guarantee requires exchangeability of calibration and test episode scores. Our validation and test scenario families intentionally differ, so test coverage is reported as a **distribution-shift stress diagnostic**, not a guaranteed 90% interval. Calibration must also have enough valid episodes; if the rank exceeds the calibration sample count or any scores are invalid, the radius is unavailable and the smoke fails closed. The implementation is a scalar-radius baseline, not a calibrated heteroscedastic uncertainty model and not a safety bound.

**Stage C — use the residual in a nominal controller.** Only after prediction quality is demonstrated, integrate the mean residual into nominal dynamics. Use predictive uncertainty to tighten constraints or reduce residual authority. Start with a transparent nominal MPC / tube-MPC reference. Keep the conventional controller as the fallback. A clipped command delta is not a control-barrier function, a stability proof, or a physical safety guarantee.

## Pre-registered evaluation plan

### Prediction benchmark arms

1. Physics-only nominal model (zero residual prediction).
2. Constant-mean residual and linear/ridge predictor (a fixed-scale ridge baseline is implemented in `integration/symthaea-physics-loop/src/linear_residual_ridge.rs`; its held-out test-split evaluator is `examples/causal_residual_ridge_benchmark.rs`).
3. Frozen HDC/LTC temporal reservoir + ridge readout (implemented in `src/hdc_ltc_residual_reservoir.rs` and `examples/hdc_ltc_residual_reservoir_benchmark.rs`). The Genesis-seeded HDC encoder and small HdcLtcUnifiedNetwork remain frozen; a fixed seeded CountSketch projects the 16,384D reservoir state to 256 dimensions and only a ridge readout is learned. This is a reservoir-computing ablation, not end-to-end HDC/LTC training.
4. A compact nonlinear raw-state MLP (implemented in `src/compact_mlp_residual.rs` and `examples/compact_mlp_residual_benchmark.rs`): 18 fixed causal features → 16 tanh units → three residual outputs (355 trainable parameters), deterministic hash-seeded initialization, fixed target scaling, full-batch Adam and a predeclared 160-epoch budget. It uses the exact fixed feature transform shared with raw-state ridge, so its gain cannot be attributed to a different input normalization. This is a small nonlinear baseline with current-state inputs only.
5. An explicit finite-history MLP (implemented in `src/history_window_mlp.rs` and `examples/history_window_mlp_residual_benchmark.rs`): current state/action/dt plus the preceding four transition-input feature rows, with explicit per-lag availability masks (94 input features, 16 tanh units, 1,571 parameters). All rows use the same fixed raw-state feature transform as ridge. This controls for accessible finite history without an HDC encoder or recurrent state. Its predeclared optimizer and epoch budget are fixed; episode history resets at boundaries.
6. End-to-end trained Symthaea HDC/LTC residual head.
7. HDC/LTC with predictive scale or ensemble disagreement.
8. Residual-target shuffle / time-offset placebo as a negative control.
9. Simulator-ground-truth disturbance/oracle model as an upper-bound diagnostic only (never a deployable competitor).

Interpretation rule: only a HDC/LTC improvement over both instantaneous raw-state and explicit-history baselines supports a representation/memory advantage. If explicit delay embedding matches or beats it at lower compute, prefer the simpler model unless another preregistered criterion justifies the additional complexity. A learned temporal representation must earn its complexity; it is not presumed superior.

Keep train data, total transition count, tuning budget, and inference hardware comparable. Report parameter count, memory, training time, p50/p95/p99 inference latency, and CPU/GPU energy where measurable. Preserve a frozen test split that is not used for authority or hyperparameter selection.

### Control benchmark arms (after Stage A)

- Tuned PID or PD with the exact same actuator limits.
- LQR/INDI baseline where its model and assumptions are valid.
- Nominal MPC / tube MPC with no learned residual.
- Point-residual model + nominal MPC.
- Uncertainty-aware residual + constraint tightening.
- HDC/LTC variants above, all using the same controller and safety/fallback interface.
- Oracle residual/MPC strictly as a simulation-only ceiling.

Use matched seeds, scenario-paired results, preselected validation, and held-out test families. Confidence intervals must resample independent episodes/seeds, not individual timesteps. Report both absolute results and paired changes versus each baseline; do not use one easy baseline as the sole comparator.

### Scenario expansion

Retain the existing static setpoint catalogue as a smoke set, then add separately versioned training/validation/test families for:

- correlated and multimodal wind, sudden reversals, heavy-tailed gusts and wind direction changes;
- unseen payload/mass/inertia, center-of-mass offsets and thrust-scale errors;
- motor/thrust response lag, command delay, rate limits, saturation and missing/stale observations;
- sensor bias/noise/dropouts and timestamp jitter;
- coupled shifts (e.g. payload + crosswind + delay), not only one-factor-at-a-time;
- long trajectories and recovery after a disturbance, not only static targets.

Training disturbance randomization must actually enter the training physics loop. In the current Symthaea imitation trainer, the scenario catalogue supplies target setpoints and the evaluator supplies disturbances; that is not disturbance-domain-randomized controller training. The separate transition-data collector now crosses target setpoints with disturbance families for system-identification data, but it still uses a simplified simulator and does not establish that the controller itself has been trained robustly.

The existing Symtropy flight backend is a simplified spherical rigid body with command-level thrust/moments, drag and a ground clamp. It is an integration substrate, not rotor-resolved or validated hardware physics. Do not convert a floor-clamp event into an aircraft crash probability.

## Proposed go/no-go targets (not achieved results)

These are initial engineering gates to power and refine before confirmatory runs:

| Dimension | Candidate gate before claiming improvement |
|---|---|
| Held-out one-step prediction | For a temporal-representation claim, at least 10% lower residual RMSE than both current-state MLP and explicit-history MLP on frozen test families, with powered paired episode-level uncertainty excluding zero improvement; report comparison to ridge and physics-only too. |
| Memory vs representation | Explicit-history MLP and HDC/LTC must use matched causal inputs, episode boundaries, training transitions and test trajectories. A HDC/LTC win over instantaneous models alone does not isolate representation value. |
| Multi-step fidelity | No material worsening at 5-, 10-, and 20-step rollout horizons; publish all horizons rather than selecting the best. |
| Predictive uncertainty | For episode-block scalar-radius conformal, report calibration episode count, radius/diameter, whole-episode coverage, step coverage, and coverage by held-out family. Use a larger confirmatory calibration set and proper scoring/interval-size criteria; investigate any family below 85% empirical coverage. Do not claim formal coverage under scenario-family shift. |
| Held-out control | At least 10% lower position RMSE than tuned PID on at least two predeclared held-out families, with confidence intervals and no regression hidden by aggregate averages. |
| Tail behavior | CVaR90 and worst-seed tracking error must not worsen versus nominal control; report floor-clamp events and recovery time separately. CVaR is an empirical statistic, not a guarantee. |
| Compute | Meet a declared control-loop deadline on the target hardware at p99, including sensor/state preparation, inference, guard and controller work. Report worst observed latency and deadline misses. |
| Integrity | Source/model/scenario/data/config digests, frozen splits, baseline gains, seeds, invalid samples, and negative-control results must be included in a machine-readable receipt. |

The exact-head smoke configuration uses 18 validation episodes and 18 test episodes (64 steps each) for episode-block calibration and runs four prediction arms—raw-state ridge, current-state MLP, explicit-history MLP, and frozen HDC/LTC—on matched held-out test episodes. The machine-readable gates check shared training-data digest, exact source/scenario digests, scenario catalogue, fixed-width episode-seed sequence, test episode count and horizon across arms; no model is required to win. The smoke is pipeline validation only. Per-episode scores feed a deterministic 10,000-replicate, scenario-family-stratified bootstrap that estimates six paired RMSE deltas/intervals and reports family-level error, empirical episode-tail CVaR90 and worst-episode RMSE. The bootstrap conditions on the observed test-family mix; with 18 episodes and three scenario families, it is an engineering diagnostic, not confirmatory evidence of generalization to unseen families. Use a powered, independently seeded calibration/evaluation design for confirmatory claims.

The 10% targets are proposed research gates, not an industry standard or a performance estimate. Do a power analysis using pilot variance before choosing the confirmatory episode count; two-trial CI runs are schema/plumbing smoke tests only.

## Current evidence and comparison to the field

Recent work supports investigating the direction, but its published numbers are not directly comparable to this benchmark:

- [Uncertainty-aware learning-based MPC for quadrotor trajectory tracking (Chinese Journal of Aeronautics, online 11 Aug 2026)](https://doi.org/10.1016/j.cja.2026.104424) predicts residual mean and covariance and uses chance-constrained MPC. Its abstract reports 32% lower tracking error and 44% fewer safety-boundary violations relative to its own strongest baseline in high-fidelity Gazebo experiments. This is a motivating design pattern, not a transferable target or head-to-head result.
- [Robust tracking under multimodal wind via residual-aware RL (Drones, 27 Jul 2026)](https://doi.org/10.3390/drones10080575) emphasizes residual features, regularization, out-of-distribution wind and a CVaR-style objective.
- [Disturbance-Aware Hybrid Learning for Robust and Adaptive UAV Flight in Extreme Winds (IJCAI 2026)](https://doi.org/10.24963/ijcai.2026/844) uses dynamics residuals for disturbance perception and compensation; its reported gains are specific to its evaluated trajectories and baselines.
- [Physics-informed sparse identification-based tube MPC for aerial vehicles (Engineering Applications of Artificial Intelligence, 1 Oct 2026)](https://doi.org/10.1016/j.engappai.2026.115494) motivates interpretable residuals, adaptive disturbance bounds and feasibility analysis instead of relying on prediction accuracy alone.
- [Crossing the Sim-to-Real Barrier in RL for Quadrotor Control (IEEE ICCA 2026)](https://doi.org/10.1109/ICCA69928.2026.11618113) reports physical Crazyflie experiments and emphasizes actuator dynamics, delays, attitude representation and more realistic disturbance modeling.
- [Physics-Based Residual Learning for Actuator Fault Identification and Health Monitoring in Quadrotors (Drones, 9 Oct 2026)](https://doi.org/10.3390/drones10100756) uses real-flight data and is a reminder to compare physics-residual and neural approaches against sensor/actuator bias, false alarms and high-frequency noise—not only tracking RMSE.
- [Valid and Efficient Split Conformal Regression for Time Series (Zhai, Cheng & Wu, 27 Sep 2026)](https://arxiv.org/abs/2609.33866) studies calibration on an adjacent time block and develops guarantees/interval-length results under dependence assumptions. It motivates keeping time dependence explicit; our current episode-block calibration is a simple diagnostic and makes no formal claim for cross-family distribution shifts.
- The public [IDSIA nano-drone benchmark repository](https://github.com/idsia-robotics/nanodrone-sysid-benchmark) makes the external comparison concrete: it provides physics, neural, hybrid physics-plus-residual, and LSTM baselines with a trajectory-level split and multi-step evaluation. Reproduce their protocol and add an HDC/LTC arm only after matching actuator inputs, dynamics fidelity, data split, and inference budget.

A particularly valuable external system-identification target is the open [Nano-Quadrotor System Identification Benchmark](https://github.com/idsia-robotics/nanodrone-sysid-benchmark), accompanying the 2026 paper [Nonlinear System Identification for a Nano-drone Benchmark](https://doi.org/10.1016/j.conengprac.2026.106871). It reports roughly 75,000 real Crazyflie 2.1 Brushless flight samples; four motor angular velocity inputs; 13-dimensional state outputs; physics, neural, physics-plus-residual and LSTM reference models; and standardized multi-step prediction out to 0.5 s (50 steps at 100 Hz). The internal explicit-history ablation should be validated here only after matching actuator inputs and state conventions. This is a much more meaningful eventual external comparison than the current internal smoke suite. Complementary [NanoBench research](https://arxiv.org/abs/2603.09908) describes over 170 trajectories with actuator, IMU, estimator and controller telemetry, potentially enabling later joint system-identification, state-estimation and control benchmarks. The implementation contract, including motor-channel names, reference state representation, train-only scaling, Melon trajectory isolation, horizon metrics, and the aggregate-command compatibility gate, is specified in [SYMTROPY_NANODRONE_SYSTEM_IDENTIFICATION_ADAPTER_V0_1.md](SYMTROPY_NANODRONE_SYSTEM_IDENTIFICATION_ADAPTER_V0_1.md).

There is a nontrivial comparability gap: the public benchmark inputs are four motor angular velocities, while the current Symtropy adapter receives aggregate thrust and three body moments and uses a simplified spherical-body model. A fair external comparison requires a validated actuator mixer / motor model, matching state conventions and sampling, and consistent train/test processing. Do not directly compare the current RMSE numbers with their leaderboard.

A credible industry comparison must eventually include a well-tuned conventional controller and a reproducible PX4/ArduPilot or hardware-in-the-loop path at matching dynamics fidelity. The [official PX4 simulation documentation](https://docs.px4.io/main/en/simulation/) distinguishes SITL from HITL and documents Gazebo and a lightweight Simulation-in-Hardware path; its [HITL guide](https://docs.px4.io/main/en/simulation/hitl.html) explains how real flight-controller firmware can run against a simulated vehicle. The fair sequence is: (1) match dynamics, sensor, rate and actuator limits in a higher-fidelity simulator; (2) compare PID/LQR/MPC and the learned residual under paired disturbance seeds; (3) run the PX4 autopilot baseline in SITL against the same scenarios; (4) use HIL before any controlled physical test. A current Symtropy point-body adapter is not a fair proxy for a PX4 autopilot.

We cannot claim to outperform commercial autopilots from the current simplified simulator or from paper abstracts. The near-term competitive advantage should be reproducibility, calibrated uncertainty, clear ablations, held-out shifts, measured compute costs, fault detection behavior and transparent failure handling.

## Falsification conditions

Reject or narrow the hypothesis if any of the following occur:

- residual learning does not beat the physics-only / linear / compact-model baselines on frozen families;
- uncertainty is under-calibrated on shift cases or becomes uselessly broad;
- gains disappear when seed pairing, gains, data volume, inference budget or train/test split is controlled;
- improvement in mean RMSE hides worse CVaR90, worst-case recovery, contacts or deadline misses;
- shuffled/offset residuals perform similarly to causal residuals;
- the controller only improves on the simplified simulation and fails HIL or calibrated physical tests.

## Implementation boundary for this PR

This draft contributes a tested-by-design Rust target extractor with unit-test source, but no Rust compilation has been run here and test results are not claimed. It does not yet train a residual dynamics network, estimate calibrated uncertainty, implement MPC, or establish physical safety. Those should be separate commits with their own exact-head evidence receipts.
