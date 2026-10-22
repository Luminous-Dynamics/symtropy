# Symthaea to Symtropy Flight Transfer Benchmark v0.1

**Status:** research benchmark and simplified cross-simulator backend. No policy superiority, physical fidelity, physical safety, or SOTA claim is made.

The previous real-source smoke checks only that a real cognitive loop can feed a probe actuator in Symtropy PhysicsWorld. This tranche tests an actual Symthaea robotics-domain controller against a baseline inside Symtropy dynamics.

The public Symthaea multirotor crate abstracts plant dynamics behind its PhysicsSimulator trait. This change implements that trait using Symtropy PhysicsWorld. It first trains the Symthaea HDC-LTC controller with the existing FlightTrainer and SimplePhysicsSimulator, then transfers the learned controller into the Symtropy backend without further policy training. It also evaluates the same trained weights, with online FEP disabled, on both native and Symtropy backends under matched seeds and disturbance schedules. That produces an explicit simulator-to-simulator transfer gap rather than inferring transfer quality only from whether the controller can execute in the target backend.

## Matched policy comparison

Every trial shares initial perturbation seed, target, physics backend and per-seed force-disturbance schedule for:
1. PD baseline from Symthaea's flight domain.
2. PID baseline from Symthaea's flight domain, with integral state reset per trial.
3. Untrained Symthaea ablation with the same deterministic genesis and architecture, but no learned weights and no online FEP modulation.
4. Trained Symthaea without online FEP modulation.
5. The same trained Symthaea weights with online FEP modulation.

The trained/no-online-FEP versus trained/FEP pair isolates the incremental contribution of the FEP adaptation path more cleanly than comparing a trained controller only with an untrained one. Every controller is cloned/reset between trials so hidden temporal state does not leak across paired trials. The benchmark reports per-policy metrics and paired deltas without assuming the learned policy wins.

## Metrics

Position RMSE, mean/final/maximum position error, fraction of time inside 5 cm of the target, thrust impulse in N·s (not electrical energy), hard-ground-clamp count, invalid-input count, and in-process controller/FEP update latency p50/p95 in microseconds. Latency is load- and machine-dependent: compare policies within the same run, not as absolute cross-machine claims. Disturbance onset, axis and amplitude are deterministically varied by trial seed and held identical across every policy in that pair. Results include paired deltas against PD, PID, untrained Symthaea, and trained Symthaea without online FEP. The JSON receipt also reports the *same trained policy without online FEP* on both its native SimplePhysicsSimulator and Symtropy, including the RMSE and within-5-cm transfer gap. Deterministic seed-level bootstrap intervals are provided for paired RMSE and within-5-cm differences. These intervals only describe the sampled seed set; the two-trial CI smoke is explicitly not statistical evidence, and model/task-family uncertainty still requires held-out families and more independent trials.

## Industry / SOTA comparison discipline

Published results demonstrate what a serious future comparison must include, but their figures are not directly comparable to this small custom plant:

| Reference | What it evaluates | How we should use it |
|---|---|---|
| [Robust pitch tracking with H-infinity / MRAC / RBFNN (IEEE Access, 2026)](https://doi.org/10.1109/ACCESS.2026.3701802) | A reduced-order pitch-axis model versus PID, LQR and MPC under wind and inertia uncertainty | Require LQR/MPC alongside PD/PID; do not compare its pitch RMSE to our 3D position RMSE |
| [Learning uncertainty online for quadrotor flight control (2025)](https://link.springer.com/article/10.1007/s10846-025-02305-5) | Uncertainty estimators across simulation and hardware, including estimation quality, tracking and real-time cost | Measure model-uncertainty calibration and compute cost, not tracking error alone |
| [LSTM wind prediction + learning-enhanced NMPC (2026)](https://www.nature.com/articles/s41598-026-71675-z) | Predictive disturbance compensation; reports 0.024 m nominal RMSE and 0.0343 m 95th-percentile error in its own test setup | A high-performance reference for methodology and tail-error reporting, not a score to compare directly |
| [Sim-to-real RL on Crazyflie (ICCA 2026)](https://doi.org/10.1109/ICCA69928.2026.11618113) | RL control validated on physical Crazyflie trajectories unseen in training | Defines a later hardware/held-out-transfer bar that this simplified backend cannot meet yet |
| [Dynamic event-triggered robust tube MPC (2026)](https://onlinelibrary.wiley.com/doi/10.1002/rnc.70351) | Constrained full-state tracking with bounded disturbances | Candidate strong-baseline family for the next comparison tranche |

The current baseline set (PD, PID, untrained Symthaea, trained Symthaea without online FEP, trained Symthaea with online FEP) is a starting point, **not state-of-the-art coverage**. A credible SOTA comparison must add at least a linearized LQR controller and a carefully tuned MPC / robust-MPC baseline on the same target task, plant, disturbance schedule and evaluation budget. We must report whether the classical baselines were tuned on training scenarios only, and freeze parameters before evaluating held-out scenarios.

## Prospective gates before any superiority claim

These are engineering targets to test, not standards limits or achieved results:

- **Integration integrity:** zero non-finite commands/states and zero invalid-input resets. The CI receipt checks this for every Symtropy-backed policy; backend-only diagnostics are represented as null, not misreported as zero.
- **Like-for-like task win:** aim for at least 10% lower held-out position RMSE than tuned PID, with a paired interval excluding zero, while not increasing ground-clamp rate, maximum error or 95th-percentile error. Also report within-5-cm occupancy and thrust impulse; do not hide trade-offs in one aggregate score.
- **FEP incremental value:** count online FEP as useful only if the trained-with-FEP policy beats the exact same trained weights without online FEP on preregistered held-out seeds, with uncertainty reported and compute overhead included. If it does not help reliably, remove or narrow the adaptation path rather than claiming a benefit.
- **Transfer gap:** after calibrating both dynamics models, target no more than a 10% relative RMSE increase when the same frozen policy moves from the native training plant to Symtropy on matched initial states and disturbances. The current uncalibrated spherical-body backend cannot yet qualify against this target.
- **Control budget:** target controller-forward p95 below 2 ms for a 500 Hz command loop, and FEP-update p95 below 20 ms for a 50 Hz cognitive update. The current telemetry measures those operations separately; it does not yet measure end-to-end sensor-to-actuator latency.
- **Generalization:** move beyond the two-trial CI smoke to a power-sized evaluation across held-out trajectory shapes, force directions, disturbance onsets, sensor dropout/staleness and actuator-lag mismatch. Use at least 100 matched seeds per scenario family as an initial research floor, or increase it when power analysis requires more. Keep the CI smoke as wiring verification only.
- **Energy accounting:** thrust impulse in N·s is not battery energy. Before comparing energy efficiency with industry, add rotor/motor efficiency and battery-voltage/current models and report electrical Wh per completed task.

## Paradigm shifts worth pursuing next

1. **Cognitive residual control, not cognitive replacement.** Keep tuned PID/LQR/MPC as a stabilizing nominal controller and let Symthaea contribute a bounded residual, learned model correction or mode selection. Gate residual authority by calibrated uncertainty and a runtime assurance shield. Compare the hybrid, classical-only and Symthaea-only policies under the same faults.
2. **Transfer-aware training.** Train across an ensemble of plants covering inertia, drag, thrust scale, actuator delay and sensor bias, then hold out combinations that were never sampled. The goal is robust rank retention across backend changes, not overfitting one simulator's dynamics.
3. **Causal mechanism tests.** The trained-with-FEP vs trained-without-online-FEP pair is a first ablation. Next randomize FEP activation by trial, freeze the remaining policy/configuration, and report matched per-seed deltas. Assigned FEP activation alone is not proof of a causal mechanism unless action realization and outcome controls are also logged.
4. **Capability–cost frontier.** Measure success, tail error, hard-limit events, p95 latency, compute/memory and later electrical Wh together. Symthaea should win by producing a better feasible frontier—not merely by making more motor activity or consuming more compute.

## Simplified backend limitations

The backend is a research plant, not a high-fidelity quadrotor model: spherical rigid body; Symtropy's mean-inertia approximation; command-level thrust and body moments without four independent rotor dynamics; exponential drag; hard ground clamp instead of a contact-complete landing model; no prop-wash, blade flapping, battery sag or validated sensor model.

It is useful for testing controller/physics integration and diagnosing sim-to-sim transfer. It must not be equated to MuJoCo or a physical Crazyflie. Sim-to-real claims require calibration, hardware-in-loop and separately approved physical trials.

## Reproduction

The dual-checkout workflow runs a small, two-trial integration smoke and uploads machine-readable JSON evidence; these smoke results are for plumbing/contract checks, not performance claims. The research default is 30 matched seeds; training and evaluation budget can be changed through STFB_TRAIN_EPISODES, STFB_TRAIN_STEPS, STFB_EVAL_STEPS and STFB_TRIALS. STFB_FORCE_DISTURBANCE=0 disables the pulses for a diagnostic run. Use more independent seeds and held-out disturbance families before drawing conclusions.

## Claim ceiling

Passing integration tests proves a code path and its test invariants, not that the trained policy outperforms PD, that Symtropy's plant is physically faithful, or that any result transfers to deployed robots. Superiority requires completed runs, held-out scenarios, paired uncertainty estimates and separate hardware evidence.
