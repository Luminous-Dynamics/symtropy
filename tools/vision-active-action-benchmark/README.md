# VIS-ACT-001 deterministic synthetic diagnostic pilot

This file is a small dependency-free Rust harness for checking the active visual-action evaluation contracts before introducing learned perception models, external datasets, or GPU workloads.

## Run the diagnostic report

```sh
rustc --edition=2024 tools/vision-active-action-benchmark.rs -o /tmp/vis-act-bench
/tmp/vis-act-bench
```

## Run the tests

```sh
rustc --edition=2024 --test tools/vision-active-action-benchmark.rs -o /tmp/vis-act-tests
/tmp/vis-act-tests
```

The benchmark imports `tools/vision-active-action-reference.rs` and its tests. The harness uses 11 hand-authored synthetic episodes and six policy baselines: always-cheap, always-escalate, confidence-threshold, surprise-only, deterministic-random, and sequential-evidence heuristic. The deterministic-random baseline is repeated across 16 policy seeds on the same fixed episode set and reports its min/max loss; these are policy-randomization repeats, not independent environment episodes or a confidence interval. It includes examples where surprise is misleading, a different viewpoint resolves occlusion/novelty, a later sample beats compute escalation, and abstention is preferable when no useful evidence action is available.

## Information boundary

The policy proposal function receives only:
- a trial id;
- an observation-feature record (confidence, surprise, occlusion, blur, novelty);
- feasibility flags for compute, next sample, alternate viewpoint and timed event.

It does not receive the authored candidate losses, oracle choice, future frames or hidden task labels. The evaluator outcomes are attached after the policy proposes and the action is committed. This separation is a code-structure guard, not a proof against all covert channels; future integration must preserve the orchestrator-owned causal stream in Symtropy issue #1282.

## Budgets and measurements

All resource quantities are explicitly synthetic: `synthetic_latency_unit` and `synthetic_sensor_action`. They are not milliseconds, FLOPs, watts, energy or claims about actual runtime. The budget guard applies the same shared run budgets to each baseline. Results are a diagnostic smoke test, not a powered experiment; 11 synthetic episodes are too few for significance, generalization or SOTA claims.

The combined test harness currently contains 24 authored tests (17 evaluator checks and 7 pilot checks); the focused GitHub Actions workflow compiles and runs them from an exact checkout. The evaluator includes a bounded exact run-budget-constrained counterfactual oracle. If its resource-state count exceeds its fixed safety cap or no globally feasible plan exists, it fails closed rather than switching silently to an approximation.

## Research issue and limitations

- Protocol and research plan: [VIS-ACT-001 #2239](https://github.com/Luminous-Dynamics/symtropy/issues/2239), [protocol PR #2240](https://github.com/Luminous-Dynamics/symtropy/pull/2240).
- Evaluator reference: [PR #2273](https://github.com/Luminous-Dynamics/symtropy/pull/2273).
- Causal stream prerequisite: [Symtropy #1282](https://github.com/Luminous-Dynamics/symtropy/issues/1282).

Do not report test success until the exact-head standalone command completes. This pilot implements neither a production action policy nor physical sensing, and establishes no benchmark superiority.
