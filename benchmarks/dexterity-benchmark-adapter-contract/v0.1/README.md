# Dexterity Benchmark Adapter Contract M0 v0.1

**Tracking issue:** [BENCH-DEXTERITY-ADAPTERS-001 #6554](https://github.com/Luminous-Dynamics/symtropy/issues/6554)  
**Related benchmark contracts:** [LADDER #6478](https://github.com/Luminous-Dynamics/symtropy/pull/6478), [skill transfer #6514](https://github.com/Luminous-Dynamics/symtropy/pull/6514), [robustness scorecard #6511](https://github.com/Luminous-Dynamics/symtropy/pull/6511)

**Status:** M0 source/record contract. Both fixtures are synthetic. No DexArena or Bench2Dex policy was run.

## Why two adapters, one evidence model

DexArena and Bench2Dex ask different questions and must retain their native scoring definitions.

- **DexArena:** stage execution, frozen-policy cross-scene transfer and full long-horizon completion. The full-sequence protocol starts once and has no resets between stages.
- **Bench2Dex:** Stable Success Rate (SR), Latched Stage Completion Rate (LSCR) and success-conditioned Efficiency (Eff.), with per-task, multi-embodiment and four-channel records.

The adapter normalizes *evidence and provenance*, not the benchmark score. It must never turn a Bench2Dex LSCR value into a physical stable-success claim or compare DexArena stage accuracy directly with Bench2Dex SR.

## Required provenance

Each run binds the benchmark URL/ID and source commit, data revision/digest where data is used, exact task-contract digest, embodiment, robot article, end-effector, channel, controller/configuration, policy checkpoint digest, training-data scope, reset mode, episode budget and evaluation track.

A record is explicitly labelled:
- `synthetic_fixture`;
- `paper_baseline`;
- `official_submission`;
- `third_party_result`;
- `reproduced_run`;
- `smoke_test`.

A result from a paper or leaderboard snapshot must not be labelled as an independently reproduced run.

## Native metrics

The manifest stores each original metric unchanged with:
- native metric name and value;
- unit and exact aggregation description;
- optional numerator/denominator if defined by the benchmark;
- optional variation and its meaning (scene standard deviation, confidence interval, or none);
- task/channel/embodiment scope.

Examples:
- DexArena stage success, cross-scene success and full-sequence success remain separate.
- Bench2Dex SR, LSCR and Eff. remain separate. The upstream definition of Eff. is success-conditioned and uncapped; zero-success tasks are not silently assigned a fabricated efficiency.
- Any paper-reported mean ± standard deviation must mark whether that variation is across scenes, seeds, or episodes. The schema will not call scene standard deviation a trial-level confidence interval.

## Reset and skill semantics

Outcomes declare whether they are:
- `stage_reset`: one independently reset stage;
- `cross_scene_frozen_policy`: one frozen checkpoint across scene variants;
- `one_reset_sequence`: one initial reset and no resets between stages;
- `local_restored`: standardized state restoration for isolated skill evaluation;
- `inherited_handoff`: starts from the predecessor's actual terminal state.

Local restored skill success and inherited-state handoff success are not pooled. A later stage not reached after an upstream failure is `not_attempted`, not success and not silently excluded. An inherited handoff binds predecessor skill ID, transition ID and predecessor terminal-state digest.

## Data and licensing gates

- DexArena code repository currently declares Apache-2.0.
- DexArena's Hugging Face dataset requires accepting its access conditions; its data page currently reports approximately 699 GB of files. M0 uses public code and metadata only. M1 download/evaluation requires accepted access, data-card review, storage approval, pinned revision and integrity verification.
- Bench2Dex protocol and score snapshots must be pinned; every submission records actual episodes, execution budgets and any deviation from the nominal 50 episodes/task/channel.

No gated data is included or fetched by this M0 package.

## Reproduction

```sh
python -m pip install jsonschema==4.24.0
python scripts/validate_dexterity_benchmark_manifest.py
```

CI runs the pinned Draft 2020-12 validator on both synthetic examples and semantic negative controls.

## Claim ceiling

These manifests make records auditable. They do not establish benchmark performance, sensor truth, dataset permission, independent reproduction or SOTA. Any benchmark score claim must retain the suite's official task set, denominator, reset semantics, policy revision, and aggregation.
