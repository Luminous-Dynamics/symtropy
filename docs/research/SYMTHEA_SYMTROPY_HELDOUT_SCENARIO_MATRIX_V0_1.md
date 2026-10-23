# Held-Out Flight Scenario Matrix v0.1

**Status:** research scaffold. The catalogue and evaluation changes have not yet been compiled or executed. No control-performance, stability, safety or SOTA claim is made.

## Why this increment exists

A single hover/position target with two deterministic force pulses is not an adequate robustness benchmark. A policy can appear effective because the target, simulator and perturbation family are narrow. Recent quadrotor research benchmarks use varied trajectories, model uncertainty, gusts and physical flight; a public 2025 adaptive-control evaluation suite similarly emphasizes modular scenario/controller comparisons and stress tests.

This increment adds a fixed scenario catalogue with immutable train/validation/test labels, larger disturbances measured relative to vehicle weight, and per-scenario paired estimates. It is designed to expose a controller that wins the average but fails on a particular target or disturbance family.

## Catalogue

| Split | Scenario | Target (m) | Disturbance |
|---|---|---|---|
| Train | `train_hover_10cm` | (0, 0, 0.10) | None |
| Train | `train_altitude_20cm` | (0, 0, 0.20) | None |
| Train | `train_x_step_5pct` | (0.05, 0, 0.12) | None |
| Train | `train_y_step_5pct` | (0, 0.05, 0.15) | None |
| Validation | `validation_diagonal_10pct` | (0.06, -0.06, 0.18) | Alternating x/y force, 10% of vehicle weight |
| Validation | `validation_sustained_gust_10pct` | (0.05, 0.02, 0.18) | Time-varying x/y force, 10% of vehicle weight |
| Test | `test_far_diagonal_15pct` | (0.12, 0.09, 0.26) | Alternating x/y force, 15% of vehicle weight |
| Test | `test_altitude_lateral_15pct` | (-0.10, 0.07, 0.28) | Time-varying x/y force, 15% of vehicle weight |
| Test | `test_vertical_impulse_20pct` | (0, -0.06, 0.22) | Vertical force pulse, 20% of vehicle weight |

The catalogue declares the training cases as target curriculum only because the current FlightTrainer consumes setpoints but does not inject these scenario disturbance profiles into its training simulator. Disturbances are applied in the evaluation harness only. This is an explicit zero-shot disturbance-family test, not domain-randomized training. The separate labels prevent evaluation cases from silently becoming training episodes.

The learned controller is **not yet trained as a residual policy**. The existing FlightTrainer is a command-imitation trainer: with `FlightConfig.use_pid=false`, its default teacher is PD, though the FEP result can request PID targets when it detects sustained offset. The experiment uses this existing learned controller as a command source and blends its difference from PID into the nominal action. It therefore tests whether that particular imitation-policy residual helps—not whether direct residual learning, online reinforcement learning, or a learned dynamics correction has been demonstrated. A follow-up must compare this imitation baseline with a genuinely residual-target-trained policy and a direct closed-loop objective before crediting the cognitive architecture with an improvement.

The evaluation applies each same trial seed to all residual-authority arms inside a scenario. Each scenario has its own paired summary and bootstrap interval so its errors cannot be hidden by pooling all scenarios into one mean.

## Disturbance scale

The world-frame disturbance magnitude is expressed as a fraction of weight:

`F_{disturbance} = \alpha m g`

where alpha is the scenario's predeclared fraction, m is the simulator body's mass, and g is 9.81 m/s². Seeds shift pulse onset deterministically; a given scenario/seed has the same force schedule across controller arms. These stress levels are benchmark settings, not measured wind equivalents or airframe-specific safety thresholds.

## Metrics and limits

Per authority/scenario, the receipt reports position RMSE, mean/final/max position error, time within 5 cm, thrust impulse, residual-clipping steps, rejected-command count and in-process controller p95. It reports matched-seed uncertainty intervals per scenario.

Still missing by design: an independently tuned LQR/MPC baseline; randomized mass/inertia/thrust scale and motor lag; sensor noise, stale observations and dropout; actuator degradation/failure; multi-waypoint trajectories; electrical energy; a high-fidelity quadrotor model; hardware-in-loop and physical flight. The current simplified body/model cannot establish physical robustness.

## Prospective acceptance gates

- All scenario IDs unique and split membership stable.
- Zero-authority arm produces exactly zero delta versus its PID baseline by construction; this is a wiring check, not scientific evidence that the cognitive arm helps.
- Record which teacher target was active during training (PD default vs FEP-requested PID); keep this distinction in every receipt.
- Do not call the model a trained residual policy until a residual-specific training objective and an ablation against the current imitation teacher are implemented.
- No non-finite command, rejected command or silently missing metric in a declared valid run.
- Before performance claims: compare against separately tuned PID, LQR and constrained/robust MPC, with gains selected without peeking at the test split.
- A prospective target is at least 10% lower *held-out* RMSE than tuned PID with a paired uncertainty interval below zero, without worse max/p95 error, ground-contact rate or safety-filter intervention count. Treat this as a goal, not an achieved result.
- Do power analysis for each scenario family. Two-trial CI runs validate plumbing only; they do not provide useful evidence of superiority.
- Keep the test set frozen. If the test outcomes alter controller design, introduce a new sealed test set before the next confirmatory claim.

## Paradigm shifts and literature

1. **Generalization before aggregate score:** explicitly freeze test families and report per-scenario failures.
2. **Model-conditional cognition:** estimate dynamics context (mass, actuator lag, thrust scale, delay) and calibrate it; do not assume one policy is invariant to every plant.
3. **Residual authority as a dose-response study:** retain per-channel authority sweeps and report performance, tail error and compute cost together.
4. **From command clipping to state-aware assurance:** command residual clipping is not a barrier certificate. The next safety work should add a validated constraint-aware filter, explicit assumptions and falsification tests.
5. **Reproducibility over paper-number comparisons:** use the same model, initial conditions, disturbance traces, tuning budgets and metrics for every baseline.

Research references:
- Zhang et al., *A Simulation Evaluation Suite for Robust Adaptive Quadcopter Control* (2025): https://hiperlab.berkeley.edu/wp-content/uploads/2025/10/Zhang_2025_AdaptQuadBenchmarkSuite.pdf
- *Uncertainty-aware model predictive control with learning-based residual dynamics compensation for agile quadrotor trajectory tracking*, online 11 August 2026. Its reported improvement is specific to its own high-fidelity Gazebo model/task and must not be treated as comparable to this simplified plant: https://www.sciencedirect.com/science/article/pii/S1000936126003626
- *Extended state observer enhanced constrained LPV-MPC for robust quadrotor trajectory tracking under wind disturbances*, 2026. The paper reports robustness tests with motor lag, MEMS noise and Dryden turbulence; those are useful future scenario families: https://www.sciencedirect.com/science/article/pii/S1270963826010503
- *Robust Optimal Agile Flight of Quadrotors: An Internal Model-Based Nonlinear Model Predictive Optimization Approach*, IEEE Transactions on Robotics, 6 August 2026. It reports physical flight under payload/wind variation and formal robustness analysis, a substantially higher validation bar than this simulation scaffold: https://doi.org/10.1109/TRO.2026.3721737
- *The Reality Gap in Robotics: Challenges, Solutions, and Best Practices*, Annual Reviews: https://doi.org/10.1146/annurev-control-031924-100130
- Survey: *A survey of world models for physical AI with uncertainty representation and control*, 2026. Recommends explicit transfer regime, parameter-randomization ranges, perturbation stress tests, constraint metrics and repeated trials: https://link.springer.com/article/10.1007/s44163-026-02122-1

## Reproduction

The new scenario catalogue lives in `integration/symthaea-physics-loop/src/scenario_matrix.rs`. The residual benchmark emits `symthaea-symtropy-bounded-residual-matrix-v0.2` JSON, with split IDs, per-scenario summaries and per-trial metrics. Preserve both repository SHAs with any receipt. The dual-checkout CI uses a two-trial, short-episode configuration for wiring checks only; run a powered multi-seed experiment and locked test evaluation separately.
