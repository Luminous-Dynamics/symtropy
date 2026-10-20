# VIS-ACT-002 parameterized causal synthetic world

This file creates a deterministic, dependency-free synthetic world to evaluate active visual-action selection separately from model benchmarks.

## Run

```sh
rustc --edition=2024 tools/vision-active-action-world.rs -o /tmp/vis-act-world
/tmp/vis-act-world
```

Tests (includes the reference evaluator tests):

```sh
rustc --edition=2024 --test tools/vision-active-action-world.rs -o /tmp/vis-act-world-tests
/tmp/vis-act-world-tests
```

The focused workflow `.github/workflows/vision-active-action-bench.yml` runs both the earlier hand-authored diagnostic harness and this generated-world harness, then uploads both TSV reports.

## How this differs from VIS-ACT-001's first pilot

The earlier 11-case diagnostic deliberately hand-authors action losses to test evaluation plumbing and recognizable failure modes. This world harness instead generates 96 worlds from a deterministic cross of occlusion, blur, distractor salience and novelty. Target identity, sensor noise and action availability use domain-separated deterministic seed draws. Task loss is computed from hidden target identity and an action-conditioned sensor decision; it is not a hand-authored per-action loss table.

A paired-world test constructs two worlds with the same exogenous observation seed but opposite hidden target identities. When both targets are fully occluded, their entire policy-visible initial observation is identical, while a noiseless legal alternate view yields different identity evidence. High occlusion cannot be cured by spending more compute on the same frame.

The policy interface receives only a public observation and a policy seed. It receives no world object, target identity, episode ID, world seed, future observation or evaluator outcome. The evaluator attaches candidate outcomes only after the action has been selected and budget-guarded. This is a code-structure boundary for synthetic research—not a formal proof against covert channels or a replacement for the orchestrator-owned stream required by Symtropy #1282.

## Resources and baseline interpretation

Resource axes are named `synthetic_latency_unit` and `synthetic_sensor_action`. They are abstract units, not milliseconds, FLOPs, watts, energy or claims about real hardware. All policies use the same trial/run budgets and canonical presentation order. The random baseline is repeated across 16 policy seeds on the same generated worlds; those repeats are not independent world samples or a confidence interval.

The oracle uses the bounded exact dynamic program in `tools/vision-active-action-reference.rs`. It optimizes counterfactual loss using declared action-cost units and must be interpreted separately from actual cost compliance.

This is still a small deterministic synthetic pilot. It does not simulate photorealistic pixels, physical camera calibration, actuator dynamics, action failures/delays, or a learned policy. It establishes no physical safety, real-camera generalization, statistical significance or SOTA claim. VIS-ACT-002 #2294 defines the next requirements for richer causal world generation, interventions, held-out domain splits and anti-shortcut controls.

Related:
- [VIS-ACT-001 issue #2239](https://github.com/Luminous-Dynamics/symtropy/issues/2239)
- [VIS-ACT-001 protocol PR #2240](https://github.com/Luminous-Dynamics/symtropy/pull/2240)
- [Active evaluator and pilot PR #2273](https://github.com/Luminous-Dynamics/symtropy/pull/2273)
- [VIS-ACT-002 issue #2294](https://github.com/Luminous-Dynamics/symtropy/issues/2294)
