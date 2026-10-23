# Tail-Risk Metrics and Source-Bound Robotics Evidence v0.1

**Status:** evaluation scaffold. New metrics and receipt schema must pass exact-head CI and a powered experiment before any performance inference. A JSON artifact that passes schema validation is not itself a scientific result.

## Why this increment

Average tracking error is not enough for robust control. A policy can improve mean RMSE while creating rare, severe excursions or more floor contacts. 2026 quadrotor residual-learning work explicitly evaluates held-out wind conditions and tail-error robustness; uncertainty-aware MPC work also incorporates predictive covariance into constraint tightening. These motivate reporting the risk distribution, not only its mean.

References:
- Residual-aware deep RL under multimodal winds, held-out OOD wind and measured real-wind disturbances (Drones, 2026): https://www.mdpi.com/2504-446X/10/8/575
- Uncertainty-aware MPC with probabilistic residual dynamics and chance constraints (Chinese Journal of Aeronautics, online 11 August 2026): https://www.sciencedirect.com/science/article/pii/S1000936126003626
- Zero-shot object-centric residual RL improving manipulation success on five real-robot tasks (Microsoft Research / CoRL 2026): https://www.microsoft.com/en-us/research/articles/object-centric-residual-rl/
- Review of reality-gap causes, metrics, and sim-to-real best practices (Annual Reviews): https://doi.org/10.1146/annurev-control-031924-100130

These systems are not directly score-comparable to the current test plant. They motivate experiment design, not a transfer of headline percentages.

## New metrics

For every residual-authority arm, the benchmark now aggregates the scenario-trial RMSE values and reports:

- mean RMSE;
- empirical 95th-percentile RMSE;
- empirical upper-tail mean at 90% confidence (mean of the largest ceil(10% × n) observations);
- worst observed RMSE;
- rate of scenario-trial episodes with any simulated floor-clamp contact;
- total floor-contact steps;
- total invalid-input count.

The empirical upper-tail mean is implemented in a small Rust module with unit tests for the tail-size rule, invalid quantiles, empty samples and non-finite input. It is a descriptive sample statistic, not a formal distribution-free risk bound or safety guarantee. In the two-trial-per-scenario smoke, it is extremely coarse—roughly the worst case per authority—so CI only tests receipt plumbing. Use hundreds or power-sized paired scenario trials and a frozen held-out suite for meaningful risk comparison.

The model's floor-contact counter records its hard ground clamp. It is not equivalent to a crash probability, rotor strike, or physical safety metric; the plant is simplified.

## Source-bound evidence receipt

The benchmark JSON now includes:

- Symtropy Git commit SHA;
- Symthaea Git commit SHA;
- Rust compiler version;
- SHA-256 digest of the exact scenario catalogue source;
- scenario split IDs, trial seeds and controller configuration;
- per-scenario estimates and aggregate risk metrics.

The dual-checkout script obtains these fields from the actual checkouts immediately before compiling/running and the receipt gate verifies formatting, the pinned Symthaea SHA, a well-formed source digest, all six authority arms, required risk fields and zero invalid-input/rejected-command events in the smoke trial. The JSON artifact is linked to the exact PR head by the workflow artifact name as well.

This creates traceable inputs for the experiment, but does not constitute independent verification of compiler reproducibility, random-seed independence or simulator fidelity. For publication-grade evidence, additionally preserve dependency lockfiles, full toolchain/OS image digest, command line/environment, all raw per-trajectory traces, and the analysis script revision.

## Prospective gates

- CI smoke: schema completeness; exact input provenance; stable scenario count/splits; no non-finite values or invalid-input events; zero-authority control matches nominal PID by construction.
- Confirmatory evaluation: predeclare target metric, practical-effect size and authority-selection protocol before opening test results; use paired seeds within scenarios and account for multiplicity across multiple authority/scenario comparisons.
- Robustness: no worse worst-case/p95 tracking, floor-contact episode rate, or constraint-intervention rate than tuned baseline. Any rare catastrophic event must be reported, not averaged away.
- Strong baselines: tuned PID plus independently tuned LQR and constrained/robust MPC on the same simulator, initial conditions, disturbance traces, constraints, and tuning budget.
- Simulation credibility: randomized mass/inertia/thrust scale, motor lag, sensing errors and actuator degradation; calibrate against the target airframe before hardware-in-loop or physical trials.
- Deployment credibility: complete sensor-to-actuator latency, deadlines, memory, electrical energy and hardware results. The present in-process timing and thrust impulse are not enough.

## Research direction

The next high-value algorithmic comparison is not another authority level. Compare (1) the existing PD/PID command-imitation model; (2) a residual-specific objective or object/physical-state-conditioned residual learner; (3) a fixed classical controller; and (4) a model-based uncertainty-aware MPC baseline. Evaluate adaptation/uncertainty gating as a separately randomized ablation. This separates value from imitation, residual capacity, uncertainty modeling and model-predictive control.

The current implementation still uses one simplified command-level plant. No SOTA, stability, formal risk or physical safety claim is made.
