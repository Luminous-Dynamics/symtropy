# Symthaea to Symtropy Flight Transfer Benchmark v0.1

**Status:** research benchmark and simplified cross-simulator backend. No policy superiority, physical fidelity, physical safety, or SOTA claim is made.

The previous real-source smoke checks only that a real cognitive loop can feed a probe actuator in Symtropy PhysicsWorld. This tranche tests an actual Symthaea robotics-domain controller against a baseline inside Symtropy dynamics.

The public Symthaea multirotor crate abstracts plant dynamics behind its PhysicsSimulator trait. This change implements that trait using Symtropy PhysicsWorld. It first trains the Symthaea HDC-LTC controller with the existing FlightTrainer and SimplePhysicsSimulator, then transfers the learned controller into the Symtropy backend without further policy training.

## Matched policy comparison

Every trial shares initial perturbation seed, target, physics backend and force-disturbance schedule for:
1. PD baseline from Symthaea's flight domain.
2. Untrained Symthaea ablation with the same deterministic genesis and architecture, but no learned weights and no online FEP modulation.
3. Trained Symthaea plus online FEP adaptation.

The controller is cloned/reset between trials so hidden temporal state does not leak across paired trials. The benchmark reports per-policy metrics and paired deltas without assuming the learned policy wins.

## Metrics

Position RMSE, mean/final/maximum position error, fraction of time inside 5 cm of the target, thrust impulse in N·s (not electrical energy), hard-ground-clamp count, and invalid-input count. Results include paired deltas against PD and the untrained ablation.

## Simplified backend limitations

The backend is a research plant, not a high-fidelity quadrotor model: spherical rigid body; Symtropy's mean-inertia approximation; command-level thrust and body moments without four independent rotor dynamics; exponential drag; hard ground clamp instead of a contact-complete landing model; no prop-wash, blade flapping, battery sag or validated sensor model.

It is useful for testing controller/physics integration and diagnosing sim-to-sim transfer. It must not be equated to MuJoCo or a physical Crazyflie. Sim-to-real claims require calibration, hardware-in-loop and separately approved physical trials.

## Reproduction

The dual-checkout workflow runs a small paired benchmark and uploads machine-readable JSON evidence. The full trial budget is configurable through STFB_TRAIN_EPISODES, STFB_TRAIN_STEPS, STFB_EVAL_STEPS and STFB_TRIALS. The force pulses can be disabled with STFB_FORCE_DISTURBANCE=0.

## Claim ceiling

Passing integration tests proves a code path and its test invariants, not that the trained policy outperforms PD, that Symtropy's plant is physically faithful, or that any result transfers to deployed robots. Superiority requires completed runs, held-out scenarios, paired uncertainty estimates and separate hardware evidence.
