# TAVIS reproduction protocol for Symtropy active-vision controllers

**Status:** experiment specification. No TAVIS rollout has yet been run from this branch. Do not report a benchmark score until the exact simulator, checkpoint and native evaluator have been executed and hashed.

## Purpose

Test whether a small evidence-aware controller/memory layer adds value on top of a capable pretrained active-vision policy, instead of merely outperforming weak hand-coded toy baselines.

TAVIS is a practical first embodied benchmark because its public project currently describes eight simulated manipulation tasks, two humanoid-torso embodiments (Fourier GR1T2 and Pollen Reachy 2), matched head-camera versus fixed-camera evaluation, procedural ID/OOD modes, and GALT (Gaze-Action Lead Time). Its public model organization exposes paired pretrained π₀ headcam/fixedcam checkpoints and datasets.

Official references:
- Project and task definitions: https://tavis-benchmark.org/
- Public datasets/checkpoints and evaluation protocol: https://huggingface.co/tavis-benchmark
- Paper: https://arxiv.org/abs/2605.07943
- Code link published by the project (anonymous-review mirror): https://anonymous.4open.science/r/tavis-F5D7

This is a benchmark adapter experiment, not a direct numeric comparison with Symtropy's synthetic loss values.

## Why this is a paradigm-shift test

The question is no longer "does a heuristic beat other heuristics in the synthetic world?" It is:

> With the same pretrained policy and task protocol, does preserving/revising task-relevant evidence, or preventing low-value camera moves, improve native task success under visual distribution shift without increasing latency or degrading anticipatory gaze?

The intervention must be controller-level. Changing policy weights, prompts, image preprocessing, action decoding, camera action space, or the task success checker at the same time would make causal attribution unclear.

## Locked benchmark facts to preserve

TAVIS project protocol specifies:
- TAVIS-Head: five tasks using global search/head pan-tilt.
- TAVIS-Hands: three tasks involving local occlusion and wrist-camera perception.
- Robots: GR1T2 and Reachy2.
- Pretrained π₀ checkpoints are listed for TAVIS-Head headcam and fixedcam modes per robot; TAVIS-Hands checkpoints are also published.
- Evaluation modes include `id`, `ood_spatial`, and `ood_init_pose`.
- 96 stochastic episodes per robot × task × evaluation-mode cell.
- Native reports include success-rate estimates with Wilson 95% intervals and per-episode GALT.

Consult the live project instructions before execution. If the current repository version differs, record the observed version and update this protocol rather than silently mixing definitions.

## Execution ladder

### Gate A: environment and artifact audit

Before installing anything, collect:
1. Local GPU model/VRAM and driver/toolchain details.
2. Exact upstream code revision/commit or archive digest.
3. SHA-256 of the selected π₀ checkpoint directory manifests or each downloaded artifact.
4. Exact simulator/assets revisions and all dependency lockfiles.
5. Checkpoint license, dataset license and robot/object asset attributions.
6. The official command used for each rollout and the resolved CLI/configuration.
7. Whether the task can be launched and its success state checked without a paid external inference service.

The published quick start states an RTX GPU is required for simulation/evaluation and that the full π₀ baseline was trained using substantial external hardware. Start with a released checkpoint; do not start by retraining a world-action model.

### Gate B: official baseline smoke test

Use the official evaluation script without edits. Begin with one named TAVIS-Head task on GR1T2, ideally `clutter_pick_cube`, at a few episodes per mode strictly as a smoke test. Verify:
- reset determinism/reproducibility as supported by the benchmark;
- the correct camera variant and checkpoint load;
- native task-success criteria;
- rollout completion and artifact output;
- GALT generation;
- no model/checkpoint fallback or silent missing episodes.

A smoke test is not a reported benchmark reproduction and must not be presented as the 96-episode official score.

### Gate C: reproduce the matched headcam/fixedcam controls

Run the official `headcam` and `fixedcam` pretrained models using the same named tasks, robot, evaluation mode, episode counts, simulator/assets and published action interface. Start with all five TAVIS-Head tasks. Preserve the official native score output.

Treat this as a paired benchmark-family reference, but document an important limitation: headcam/fixedcam checkpoints may differ in training configuration and available visual input, so their difference alone does not isolate the effect of a controller overlay.

### Gate D: pre-register the controller-layer ablation

Only after baseline execution is reproducible, test a small number of isolated controller variants. Do not alter model weights. Candidate interventions:
- baseline pretrained active headcam policy;
- exact same policy with an evidence-memory/controller shim that can veto a camera move only under a declared, observable low-value condition;
- same policy with a provenance/freshness gate that prevents stale or unexecuted observation updates;
- passive/fixed-camera reference as already supplied by the benchmark;
- random legal head action at the same camera-action budget, if supported without violating the native interface.

Do not compare an overlay that can use extra future frames, hidden simulator state, task phase labels, or privileged object positions. The controller may use only the policy's actual legal observations and permitted robot/action state.

If action-veto changes the action sequence or violates the imitation checkpoint's expected control format, stop and use a native policy extension point instead; do not hack the evaluator. Document the adapter's exact input/output schema and retain original commands and output receipts.

### Gate E: confirmatory evaluation

Run the official 96 episodes in each declared robot × task × mode cell after the smoke path is stable. Minimum first study:
- TAVIS-Head, GR1T2, all five tasks.
- `id` and `ood_spatial`.
- Baseline headcam and fixedcam first; controller overlay only after those reproduce and only with a compatible extension point.
- If compute permits, expand to `ood_init_pose` and Reachy2 without changing the analysis after seeing results.

The full TAVIS-Head first-stage grid is 5 tasks × 2 evaluation modes × 96 episodes per checkpoint/robot condition. Keep task-cell CIs and missing/failed rollouts visible.

## Metrics to report separately

Primary:
- Native benchmark task success per episode and per task.
- Aggregate exactly as TAVIS defines it; publish individual task rows.
- Paired controller-minus-baseline delta under identical initial seeds whenever seed pairing is supported.
- Wilson intervals as the native benchmark reports them; for controller comparisons, add a task/seed-clustered paired interval if the underlying episode IDs permit it.

Secondary:
- GALT distribution in seconds, not just its mean: median, lower/upper quantiles and task-wise results.
- Task success conditioned on positive/near-zero/negative GALT where that analysis is supported.
- Camera/head motion magnitude, oscillation, invalid commands and motion smoothness where available.
- Decision/task latency and policy wall-clock time (measured, with hardware noted).
- Failed, late, redundant and fresh-evidence-producing camera actions, but only where the official output stream allows these to be measured without inferring unavailable state.
- ID-to-OOD regression by task, not just total success.

Do not combine GALT and success into an arbitrary one-number metric. High positive GALT is not sufficient if the robot fails the task; high success with unstable camera motion is also worth exposing.

## Statistical and reproducibility contract

- Keep the official task-native scoring logic.
- Use exact task, robot, mode, seed/episode ID and checkpoint/config hashes.
- Report all task-level outcomes including zero-success rows, missing episodes and run failures.
- For the controller's primary claim, use a paired analysis on matched episode IDs if the benchmark allows identical IDs across the two evaluated conditions. Bootstrap/resample at the task level when claiming task-family generalization; do not treat episodes from a few tasks as independent evidence of generalization to new tasks.
- Pre-register the primary metric, one practical-effect threshold, CI method, stop rule, and designated OOD slice before the confirmatory controller run.
- Treat "runner queued", partial rollouts, or a smoke test as unqualified, not as completed.

## Internal target gates (aspirational)

The initial goal is to demonstrate a reproducible, useful controller effect—not to claim universal SOTA:
1. Baseline results reproduce the published suite using the same model/checkpoint variants and native task predicates closely enough that all material discrepancies are documented.
2. On a pre-registered OOD task set, the controller overlay yields a positive paired task-success improvement with a 95% interval excluding no improvement, or a clearly supported resource/success Pareto improvement if success is tied.
3. No designated critical task regresses beyond a pre-registered non-inferiority margin (initial proposal: 3 absolute percentage points, subject to sample size and baseline variance).
4. GALT and camera-motion stability do not regress materially while task success improves.
5. Every gain is traceable to the controller intervention, not different model weights, prompt, vision inputs, seed list, action decoder or scoring script.

These are proposed internal gates, not results and not universal standards.

## Relationship to other benchmark targets

- **TAVIS first:** best for a paired active-head-camera versus fixed-camera reference, with native GALT to characterize anticipatory gaze.
- **ActiveArena after baseline parity:** 35 language-conditioned tasks, five categories, fixed ID/OOD seed protocol. This is the broader test for multi-object evidence acquisition and memory.
- **RoboTwin-AV / ActiveWAM:** aspirational stronger world-action-model comparator. The ActiveWAM paper reports gains on TAVIS and RoboTwin-AV, but confirm the official code/checkpoint availability and exact evaluation setup before attempting a numerical reproduction. Do not present manuscript results as Symtropy results.

The central comparison is incremental controller value on top of an eligible, capable baseline, not a leaderboard row from a different action space.

## Current status

This document is an implementation/reproduction protocol only. No TAVIS environment install, checkpoint download, simulation rollout, or native metric reproduction is claimed here. The next executable step is Gate A: resolve official environment prerequisites, available GPU capacity, asset/checkpoint revisions, and a first single-task smoke command.