# Bounded Cognitive Residual Flight Benchmark v0.1

**Status:** experiment scaffold. It has not yet produced verified benchmark results. Nothing in this document establishes stability, physical safety, or superiority over PID/MPC.

## Research question

Can a trained Symthaea HDC-LTC policy improve disturbance rejection when it is permitted to add only a bounded residual to a classical PID command, and how does performance change as residual authority increases?

This tests a different deployment thesis from replacing the nominal controller with a learned policy. A classical controller continues to supply the nominal action; the cognitive controller only influences the command within an explicit per-channel envelope.

## Implementation

The Rust module at integration/symthaea-physics-loop/src/bounded_residual.rs defines ResidualEnvelope:

- rejects non-finite nominal or cognitive command channels, authority outside [0, 1], and negative/non-finite limits;
- clips cognitive-minus-nominal delta independently for thrust, roll moment, pitch moment, and yaw moment;
- scales the clipped delta by a configured authority in [0, 1];
- applies actuator-range clamping after blending;
- returns None on invalid input so the caller can count the event and fall back to nominal.

**Important:** this is a *command-envelope guard*, not a control-barrier function, reachable-set safety filter, stability certificate, collision-avoidance proof, or guarantee that the resulting trajectory is safe. The per-channel limits in the experiment are knobs selected for sensitivity analysis; they are not certified limits for any airframe.

## Paired experiment

The example at integration/symthaea-physics-loop/examples/bounded_residual_transfer.rs:

1. Trains one real Symthaea FlightController on Symthaea's native SimplePhysicsSimulator.
2. Freezes the learned weights and evaluates in Symtropy PhysicsWorld.
3. Repeats each seed across residual authorities 0, 0.1, 0.25, 0.5, 0.75, and 1.0.
4. Uses the same PID gains, initial-condition seed, and deterministic disturbance pulses across authority arms within each trial.
5. Disables online FEP for this experiment so residual authority is the isolated treatment variable.
6. Emits JSON with per-trial RMSE, mean/final/max position error, time within 5 cm, thrust impulse, residual clipping, guard rejections, controller compute p95, paired deltas from the zero-authority PID baseline, and deterministic bootstrap intervals.

The zero-authority arm skips the cognitive forward pass and must reproduce the PID baseline. This is an important negative control. A guard rejection falls back to nominal PID and is reported rather than silently hidden.

The current smoke lane runs two matched seeds to verify the receipt/contract only. Bootstrap intervals at this sample size are plumbing evidence, not inferential evidence. The full research default is 30 trials, but a prospective power analysis and held-out scenario families are still required. The current one-setpoint task and simplified sphere/command-level physics remain severe limits.

## Prospective gates (not yet achieved)

- **Correctness:** tests prove the residual is zero at authority 0, the clipped delta respects its configured envelope before actuator clamping, invalid values fail closed, and output commands respect actuator ranges.
- **Baseline integrity:** per-seed authority-0 metrics exactly match the baseline PID reference; no training/evaluation state leaks across arms.
- **No hidden safety claim:** report every rejected command and every residual-clipped step. Any nonzero rejection count blocks a superiority claim until explained.
- **Performance:** report paired RMSE and within-5-cm deltas at each authority. Only claim benefit at an authority selected before held-out evaluation, with uncertainty intervals, no worse tail error or limit-event rates, and a predeclared practical-effect margin.
- **Latency/cost:** measure nominal+residual path p95, not just cognitive forward time; later add complete sensor-to-actuator latency, peak memory and electrical energy.
- **Generalization:** separate training and held-out trajectory families, target positions, pulse axes/onsets/durations, sensor noise/staleness, actuator lag and model parameters. At least 100 matched seeds per scenario family is an initial floor, not a substitute for power analysis.

## Paradigm-shift path

1. **Residual cognition:** add cognitive capability without transferring all authority away from well-understood control.
2. **Uncertainty-gated authority:** eventually permit authority to vary only from a calibrated uncertainty estimate and validated operating envelope, rather than a hand-selected constant. Calibration and failure modes must be independently measured.
3. **Runtime assurance:** replace simple delta clipping with an actual state/constraint-aware shield (for example a control-barrier / reachable-set filter) once dynamics and safety constraints are explicit. That requires formal assumptions and tests; command clipping alone does not provide it.
4. **Transfer-aware learning:** train over a plant ensemble and evaluate on combinations held out from training. Include motor lag, thrust scale, inertia, delay and sensor errors rather than only changing the pulse seed.
5. **Cost-aware comparison:** compare on a Pareto frontier of tracking error, tail error, limit events, completion, p95 latency, compute/memory and eventually electrical Wh per task.

## Industry and literature comparison

A recent uncertainty-aware learning-based MPC study reports large improvements on its own high-fidelity Gazebo benchmark, including lower tracking error and fewer safety-boundary violations. Those numbers are **not directly comparable** to this one-setpoint simplified plant. Its valuable lesson is methodological: uncertainty must affect both prediction and constraint handling, and baseline comparisons must share dynamics, disturbances and metrics.

- Uncertainty-aware learning-based MPC, *Chinese Journal of Aeronautics* (online 11 August 2026): https://www.sciencedirect.com/science/article/pii/S1000936126003626
- PhyFilter physics-filtered residual learning across multiple robot classes, *npj Robotics* (2026): https://www.nature.com/articles/s44182-026-00114-y
- 2026 survey of uncertainty-aware world models and sim-to-real evaluation: https://link.springer.com/article/10.1007/s44163-026-02122-1
- The reality-gap survey's guidance on model fidelity, randomization and target-environment validation: https://doi.org/10.1146/annurev-control-031924-100130

## Reproduction

Under the dual-checkout environment established by scripts/test-real-symthaea-in-symtropy.sh:

```sh
STFR_TRAIN_EPISODES=4 \
STFR_TRAIN_STEPS=256 \
STFR_EVAL_STEPS=320 \
STFR_TRIALS=100 \
cargo run --manifest-path integration/symthaea-physics-loop/Cargo.toml \
  --example bounded_residual_transfer
```

The emitted benchmark_id is symthaea-symtropy-bounded-residual-v0.1. Preserve the exact Symtropy and Symthaea SHAs with every saved receipt. Hosted CI is responsible for compiling/running the exact PR head; local Rust tooling was unavailable during authoring.
