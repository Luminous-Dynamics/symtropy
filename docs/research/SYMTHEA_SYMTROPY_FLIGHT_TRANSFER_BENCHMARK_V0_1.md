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

## Simplified backend limitations

The backend is a research plant, not a high-fidelity quadrotor model: spherical rigid body; Symtropy's mean-inertia approximation; command-level thrust and body moments without four independent rotor dynamics; exponential drag; hard ground clamp instead of a contact-complete landing model; no prop-wash, blade flapping, battery sag or validated sensor model.

It is useful for testing controller/physics integration and diagnosing sim-to-sim transfer. It must not be equated to MuJoCo or a physical Crazyflie. Sim-to-real claims require calibration, hardware-in-loop and separately approved physical trials.

## Reproduction

The dual-checkout workflow runs a small, two-trial integration smoke and uploads machine-readable JSON evidence; these smoke results are for plumbing/contract checks, not performance claims. The research default is 30 matched seeds; training and evaluation budget can be changed through STFB_TRAIN_EPISODES, STFB_TRAIN_STEPS, STFB_EVAL_STEPS and STFB_TRIALS. STFB_FORCE_DISTURBANCE=0 disables the pulses for a diagnostic run. Use more independent seeds and held-out disturbance families before drawing conclusions.

## Claim ceiling

Passing integration tests proves a code path and its test invariants, not that the trained policy outperforms PD, that Symtropy's plant is physically faithful, or that any result transfers to deployed robots. Superiority requires completed runs, held-out scenarios, paired uncertainty estimates and separate hardware evidence.
