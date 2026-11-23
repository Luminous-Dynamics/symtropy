# CIVDYN closed-loop policy-value reference v0.1

This isolated, dependency-free Rust crate implements the first executable slice of [CIVDYN-CLOSED-LOOP-UTILITY-001 #4688](https://github.com/Luminous-Dynamics/symtropy/issues/4688).

It asks a more consequential question than whether a model predicts the next state accurately:

**Does using the model to choose and repeatedly revise actions produce better feasible service outcomes in the true world?**

## Frozen scenario

The benchmark has two structurally competing deterministic worlds and a four-tick horizon. Both begin from the same observed state:

| State component | Initial value |
|---|---:|
| Essential water service | 8 |
| Essential energy service | 8 |
| Water maintenance debt | 0 |
| Energy maintenance debt | 0 |
| Water reserve | 20 |
| Energy reserve | 20 |
| Maintenance credits | 2 |

The worlds differ in degradation and repair response:

| World | Water decay | Energy decay | Water repair | Energy repair |
|---|---:|---:|---:|---:|
| Water degradation | 2 | 1 | 4 | 2 |
| Energy degradation | 1 | 2 | 2 | 4 |

All quantities are integer reference units. They are deliberately small so the four-step action sequence can be exhaustively enumerated and independently hand-checked.

## Transition contract

For each tick:

1. Select NoOp, RepairWater or RepairEnergy.
2. A repair increases the chosen service by the world-specific repair amount, capped at 10; reduces that asset's maintenance debt by 3 (floor 0); consumes one maintenance credit and two energy-reserve units.
3. Water and energy service degrade by the corresponding base decay plus one quarter of current maintenance debt, rounded down.
4. Maintenance debt increases by the base decay.
5. Water reserve decreases by 2 and energy reserve decreases by 1.

The following are hard constraints, never weighted objectives:

- Water service is at least 4 at every transition.
- Energy service is at least 4 at every transition.
- Neither resource reserve becomes negative.
- Neither maintenance debt exceeds 15.
- At the horizon, both reserves are at least 10.

The objective is cumulative service in native service-point-ticks. Hard infeasibility remains disqualifying even when an infeasible method has a larger raw service total.

## Comparison arms

Each true world is evaluated using five separate arms:

1. **Reactive no-model:** repair whichever service is currently lower; ties use the declared debt then water tie-break.
2. **Fixed open-loop:** repair water at ticks 0 and 1, then take no action; no replanning.
3. **Correct action-conditioned model:** receding-horizon exhaustive planning using the candidate model matching that world's registered transition mechanism.
4. **Misspecified action-conditioned model:** the opposing world's candidate model, used without access to the actual world identity.
5. **Evaluator-only oracle:** exhaustive finite-horizon plan using the true world. This is available only to the evaluator and is not a deployable policy.

The model policy receives only the current observable state, its candidate model, the remaining horizon and the fixed action set. The candidate chooser does not accept the true-world object. The evaluator computes the oracle action separately for comparison and never passes that result to model or baseline policy selection.

A candidate model that cannot find a full-horizon feasible plan records an explicit abstention and does not silently disappear from the denominator.

## Known-answer results

The independently hand-enumerated reference optimum in both worlds is 50 service-point-ticks.

| True world | Correct model sequence | Minimum water | Minimum energy | Oracle service |
|---|---|---:|---:|---:|
| Water degradation | noop → repair water → noop → repair water | 6 | 4 | 50 |
| Energy degradation | noop → repair energy → noop → repair energy | 4 | 6 | 50 |

The correct action-conditioned model must match the evaluator-only oracle's action sequence, cumulative service, and all four per-tick action rankings in each world. Its one-step prediction error is zero only because this is a deterministic known-answer fixture using identical transition equations. This does not establish empirical model accuracy.

The reactive and open-loop arms violate at least one hard service floor. The misspecified action-conditioned model also violates a service floor in the opposing world. These controls show why policy value cannot be inferred from one-step prediction, current weakest-service heuristics, or raw cumulative averages alone.

## Receipt and replay

The closed-loop-policy-sweep binary emits ten deterministic rows (five policy arms × two true worlds) with:

- true-world and candidate-model digests;
- explicit evaluator-oracle-access flag;
- complete action traces;
- cumulative service, oracle service and raw regret;
- per-service hard-floor violations and worst shortfalls;
- ending reserves, maintenance debts and remaining credits;
- action-ranking agreement, replan count and explicit abstentions;
- component-wise one-step MAE for service, debt and reserve state;
- scenario, split and independent-unit identities.

Run:

    cargo fmt --manifest-path research/civdyn-closed-loop-policy/Cargo.toml --all -- --check
    cargo clippy --locked --manifest-path research/civdyn-closed-loop-policy/Cargo.toml --all-targets -- -D warnings
    cargo check --locked --manifest-path research/civdyn-closed-loop-policy/Cargo.toml --all-targets
    cargo test --locked --manifest-path research/civdyn-closed-loop-policy/Cargo.toml --all-targets
    cargo run --locked --manifest-path research/civdyn-closed-loop-policy/Cargo.toml --bin closed-loop-policy-sweep

The dedicated GitHub Actions workflow runs format, strict Clippy, check, tests, two-run CSV replay and a machine-checked known-answer contract.

## Claim ceiling

This v0.1 is deterministic synthetic software/protocol evidence only. It does not establish real-world water or energy policy effectiveness, external validity, causal identification, stochastic calibration, statistical coverage, sampling uncertainty, preference legitimacy, or superiority to an external world-model benchmark.

The correct-model arm's equality with the oracle is expected in a finite known-truth environment; it is not a SOTA claim. A superiority claim requires preregistered held-out mechanism families, paired multi-seed inference, independent outcome evaluation, source-qualified empirical scenarios, and a matched policy-visible compute budget.

Follow-on work in #4688 should add stochastic dependent shocks, observation noise, heterogeneous households, distributional service objectives, networks and spillovers, delayed intervention effects, robust multi-model control, and source-qualified South African water-energy-municipal evaluation.
