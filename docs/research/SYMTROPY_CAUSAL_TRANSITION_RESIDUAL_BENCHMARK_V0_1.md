# Causal Transition-Residual Benchmark for Symthaea × Symtropy (v0.1)

**Status:** research contract + target-extraction primitive; implementation and Rust tests are committed on the branch for this draft, but compilation and the benchmark have not been verified in this environment.

## Why this is the next step

The current bounded-residual experiment is a useful control-blending smoke test, but its controller is trained by imitation of Symthaea's PD/PID command targets. It is not a model of residual vehicle dynamics. The distinction matters: adding an imitation-trained command to PID cannot establish that Symthaea has learned wind, payload, actuator lag, or a physics-model discrepancy.

The next hypothesis is more testable:

> A compact causal model of the *one-step dynamics residual*, paired with calibrated predictive uncertainty, can improve a nominal controller's predictions and disturbance rejection under held-out dynamics shifts at a measured compute budget.

This is a hypothesis, not a result. This PR adds the initial supervised-target definition, not the learned predictor or an MPC controller.

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

The companion collector is `integration/symthaea-physics-loop/examples/causal_transition_dataset.rs`. It uses a classical PID baseline only as a data-collection policy, applies deterministic training-only exogenous force families, and writes one-step samples containing prior state, applied command, next state, and derived target. Exact hidden force values are omitted from per-sample records. The smoke workflow gates sample counts, split membership, timestamp order, vector dimensions and source receipt formats, then uploads the dataset as an artifact.

The target is deliberately a model discrepancy bucket. It can include external disturbances, attitude changes during the interval, integration mismatch, actuator/model mismatch, and measurement noise. Do not label it “wind estimate” without a separately identified disturbance-observer design and supporting tests. The currently used nominal model is simple; parameters must be frozen before test evaluation and included in the experiment receipt. The dataset collector does not train a predictor, and a successful schema smoke would demonstrate plumbing only—not residual-model accuracy or control improvement.

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

**Stage C — use the residual in a nominal controller.** Only after prediction quality is demonstrated, integrate the mean residual into nominal dynamics. Use predictive uncertainty to tighten constraints or reduce residual authority. Start with a transparent nominal MPC / tube-MPC reference. Keep the conventional controller as the fallback. A clipped command delta is not a control-barrier function, a stability proof, or a physical safety guarantee.

## Pre-registered evaluation plan

### Prediction benchmark arms

1. Physics-only nominal model (zero residual prediction).
2. Constant-mean residual and linear/ridge predictor.
3. Small MLP or TCN with equivalent transition data.
4. Symthaea HDC encoder + liquid/LTC dynamics-residual head.
5. HDC/LTC with predictive scale or ensemble disagreement.
6. Residual-target shuffle / time-offset placebo as a negative control.
7. Simulator-ground-truth disturbance/oracle model as an upper-bound diagnostic only (never a deployable competitor).

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

Training disturbance randomization must actually enter the training physics loop. In the current trainer, the scenario catalogue supplies target setpoints while evaluation supplies disturbances; that is not disturbance-domain-randomized training.

The existing Symtropy flight backend is a simplified spherical rigid body with command-level thrust/moments, drag and a ground clamp. It is an integration substrate, not rotor-resolved or validated hardware physics. Do not convert a floor-clamp event into an aircraft crash probability.

## Proposed go/no-go targets (not achieved results)

These are initial engineering gates to power and refine before confirmatory runs:

| Dimension | Candidate gate before claiming improvement |
|---|---|
| Held-out one-step prediction | At least 10% lower acceleration-residual RMSE than physics-only and a strong small-model baseline on the frozen test families; paired episode-level 95% CI must exclude zero improvement. |
| Multi-step fidelity | No material worsening at 5-, 10-, and 20-step rollout horizons; publish all horizons rather than selecting the best. |
| Predictive uncertainty | Nominal 90% interval coverage should be near 90% overall and reported per stress family; investigate any family below 85% coverage. Also report interval width / proper scoring rule so trivially huge intervals do not pass. |
| Held-out control | At least 10% lower position RMSE than tuned PID on at least two predeclared held-out families, with confidence intervals and no regression hidden by aggregate averages. |
| Tail behavior | CVaR90 and worst-seed tracking error must not worsen versus nominal control; report floor-clamp events and recovery time separately. CVaR is an empirical statistic, not a guarantee. |
| Compute | Meet a declared control-loop deadline on the target hardware at p99, including sensor/state preparation, inference, guard and controller work. Report worst observed latency and deadline misses. |
| Integrity | Source/model/scenario/data/config digests, frozen splits, baseline gains, seeds, invalid samples, and negative-control results must be included in a machine-readable receipt. |

The 10% targets are proposed research gates, not an industry standard or a performance estimate. Do a power analysis using pilot variance before choosing the confirmatory episode count; two-trial CI runs are schema/plumbing smoke tests only.

## Current evidence and comparison to the field

Recent work supports investigating the direction, but its published numbers are not directly comparable to this benchmark:

- [Uncertainty-aware learning-based MPC for quadrotor trajectory tracking (Chinese Journal of Aeronautics, online 11 Aug 2026)](https://doi.org/10.1016/j.cja.2026.104424) predicts residual mean and covariance and uses chance-constrained MPC. Its abstract reports 32% lower tracking error and 44% fewer safety-boundary violations relative to its own strongest baseline in high-fidelity Gazebo experiments. This is a motivating design pattern, not a transferable target or head-to-head result.
- [Robust tracking under multimodal wind via residual-aware RL (Drones, 27 Jul 2026)](https://doi.org/10.3390/drones10080575) emphasizes residual features, regularization, out-of-distribution wind and a CVaR-style objective.
- [Disturbance-Aware Hybrid Learning for Robust and Adaptive UAV Flight in Extreme Winds (IJCAI 2026)](https://doi.org/10.24963/ijcai.2026/844) uses dynamics residuals for disturbance perception and compensation; its reported gains are specific to its evaluated trajectories and baselines.
- [Physics-informed sparse identification-based tube MPC for aerial vehicles (Engineering Applications of Artificial Intelligence, 1 Oct 2026)](https://doi.org/10.1016/j.engappai.2026.115494) motivates interpretable residuals, adaptive disturbance bounds and feasibility analysis instead of relying on prediction accuracy alone.
- [Crossing the Sim-to-Real Barrier in RL for Quadrotor Control (IEEE ICCA 2026)](https://doi.org/10.1109/ICCA69928.2026.11618113) reports physical Crazyflie experiments and emphasizes actuator dynamics, delays, attitude representation and more realistic disturbance modeling.
- [Physics-Based Residual Learning for Actuator Fault Identification and Health Monitoring in Quadrotors (Drones, 9 Oct 2026)](https://doi.org/10.3390/drones10100756) uses real-flight data and is a reminder to compare physics-residual and neural approaches against sensor/actuator bias, false alarms and high-frequency noise—not only tracking RMSE.

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
