# Active-perception benchmark bridge: synthetic logic to public interactive tasks

**Status:** evaluation protocol and targets, not results. This document does not claim that the current synthetic harness matches any public benchmark or achieves state of the art.

## Why a benchmark bridge is needed

Active perception is not the same as processing another fixed image. A valid policy selects an action that changes subsequent evidence, executes that action, receives the resulting observation, and revises the downstream decision. Evaluation must separate (1) evidence quality, (2) task utility, and (3) the measured cost and feasibility of acquiring the evidence.

This distinction is consistent with the October 2026 review of active visual perception, which highlights the difficulty of comparing studies with different scenes, action spaces, budgets, and evaluation criteria:
- Li et al., *Active Visual Perception: Opportunities and Challenges*, Sensors (15 October 2026): https://www.mdpi.com/1424-8220/26/20/6544

## External reference points and comparability limits

| Benchmark | What it covers | Appropriate use here | What not to claim |
|---|---|---|---|
| [ActiveArena](https://github.com/leeibo/ActiveArena) | 35 language-conditioned robotic-manipulation tasks; five categories (single-object search, single-object loop, multi-object loop, multi-object decision, interactive information acquisition); controllable viewpoint; ID/OOD splits; an 18-dimensional embodiment action representation | First external test for view selection, evidence retention, multi-object search and interactive information acquisition. Keep the native task-success checks and the same robot/action backend for every policy. | Its success rates are not directly comparable with the discrete synthetic world. A reduced active-view sub-protocol is not a claim of full 18-D robot-policy competence. |
| [iWorld-Bench](https://proceedings.mlr.press/v306/fang26l.html) | Interactive world-model evaluation covering distance perception, memory and action-conditioned behavior; paper reports 330k source clips, 2.1k selected samples, and 4.9k test samples across six task types and nine metrics | Select compatible memory/interaction subsets after mapping the legal action and observation contract. Preserve its native metrics. | Do not combine its multi-task aggregate with Symtropy's synthetic loss or describe unmatched sample counts as equivalent trials. |
| [CHAIN](https://arxiv.org/abs/2602.21015) | Interactive 3-D physics tasks, including interlocking, stacking and packing, where geometric/contact reasoning affects multi-step action success | A later downstream check that active evidence actually improves physical-structure reasoning and executable plans. | It is not an active-vision-memory score unless the same policy can control evidence acquisition under the same protocol. |

Benchmark descriptions above come from their published/project materials: ActiveArena's [official repository](https://github.com/leeibo/ActiveArena), iWorld-Bench's [PMLR paper](https://proceedings.mlr.press/v306/fang26l.html), and CHAIN's [paper abstract](https://arxiv.org/abs/2602.21015).

## Evaluation ladder

### Tier 0 — causal, deterministic regression suite

Use the Rust synthetic harnesses to test causal ordering and invariants quickly. Preserve paired hidden worlds, exact exogenous seeds, independent world-seed replicates, action-conditioned observations, explicit negative controls, deterministic replay, and budget fail-closed behavior. Treat synthetic latency/resource units only as accounting units. The evaluator must not publish hidden outcomes to the policy before action commitment.

### Tier 1 — rendered, held-out simulation

Introduce real rendered pixels, camera intrinsics/extrinsics, occluder geometry, motion blur, lighting, distractors, sensor noise, camera-motion delay, failed/no-op actions, and action-dependent observation freshness. Split by scene/object family and intervention mechanism—not only by numeric severity. Ensure no repeated render/prediction is credited as a new sensor observation. Keep world generation, sensor observations, action validation and private truth in separately testable modules.

### Tier 2 — external interactive benchmark

Start with an ActiveArena-compatible adapter where the same frozen vision/VLA backbone, action decoder, motion planner and environment configuration are used for every active-memory policy. Vary only the policy components under study. Run on the published task/seed split and retain benchmark-native success checks. Then port only the compatible action/memory protocol to selected iWorld-Bench tasks. Use CHAIN later to check downstream consequences for physical reasoning.

### Tier 3 — hardware/robot validation

Only after simulation qualification, measure the actual platform. Report sensor execution failures, wall-clock latency (median and p95), GPU/CPU use, energy if instrumented, action duration, safety interventions, dropped/stale frames and recovery/abstention. Never substitute synthetic cost units for measured physical costs. A successful synthetic or simulator run does not authorize safety-critical deployment.

## Fair baseline slate

Keep the vision model and environment fixed when attributing gains to active-perception policy. Use:
- no acquisition / passive memory; always-view when legal; deterministic or randomized view under the same acquisition budget;
- confidence-threshold and surprise/novelty-triggered policies;
- no memory, last observation, bounded FIFO/sparse retention, confidence-gated overwrite, provenance-and-time-aware belief memory, and active revision after disconfirmation;
- compute escalation and a task-value/expected-value-of-information policy where that action is legal and its cost can be measured;
- an evaluator-only feasible oracle for regret, plus a clearly labeled clairvoyant ceiling that is **never** counted as an eligible policy.

Do not silently change a VLM, prompt, resolution, crop strategy, tracking model, inference precision, action decoder, or planner between baseline and proposed policy. If any component must change, run a factorial ablation to isolate its contribution. Compare under matched total budgets, not the same number of policy calls while omitting allocator, view-motion, retry or memory costs.

## Required measurements

**Primary outcome (pre-register one):** episode-level task loss or benchmark-native task success, analyzed at the episode/task/seed level. Per-frame observations are not independent samples.

Always report task success/loss and abstention separately. Stratify by persistence versus removal, successful versus failed acquisition, occlusion, blur, distractor, novelty, ID/OOD scene families and action availability. Include false persistence after a removal, incorrect entity re-association, recovery after contradictory evidence, calibration/abstention quality when defined, and how often a supposedly informative action yields no fresh evidence.

Resource reporting must separate actual from estimated costs: action/view counts, failed/no-op attempts, measured wall time, p50/p95 latency, peak memory entries/bytes, hardware memory, energy where measured, and every budget violation. Report paired seed differences with a confidence interval and show per-task/per-stratum failures, not only the aggregate leaderboard row.

## Project target gates (aspirational; not industry-standard thresholds)

Before confirmatory evaluation, freeze the primary metric, strongest eligible baseline, practical improvement threshold, confidence-interval method, held-out task families, stop rule and non-inferiority bounds.

Proposed gates:
1. At least **10% relative reduction in primary task loss** versus the strongest eligible baseline at matched measured budgets, with a paired 95% interval excluding no improvement on the preregistered external evaluation.
2. Benefit must survive at least three distinct held-out task/scene families; no critical stratum may regress by more than the preregistered non-inferiority bound (initial proposal: 3 percentage points absolute in success, or the equivalent preregistered loss bound).
3. Zero accepted hidden-truth/future-observation leaks in the adversarial test suite; zero unreported budget violations; unavailable, failed or unexecuted sensing actions receive no information-acquisition credit.
4. Every number is reproducible from exact code/model/config/task/seed identifiers, and the report includes failures, missing runs and uncertainty.

These are internal engineering goals for choosing what to optimize, not a claim that such thresholds are universal standards or that current code passes them.

## What would count as beating the state of the art

A meaningful claim requires a matched evaluation where the current best eligible baseline is actually run under the same environment, observation release protocol, action permissions, backbone, task seeds and measured resource budget. The primary result should be the paired incremental improvement from the active-perception/memory controller, alongside task success and resource cost. If only a subset can be run, name that subset and restrict the claim to it. Never infer a leaderboard ranking from a different benchmark's aggregate score.

## Current Symtropy boundary

The current dependency-free Rust pilots test evaluator arithmetic, selected causal invariants and synthetic task hypotheses. They use symbolic cues rather than rendered pixels and abstract resource units rather than measured compute/energy/robot time. They are useful for rejecting bad controllers and leaking evaluators early; they do **not** establish real-camera generalization, robot competence, physical safety or SOTA. The multi-round VIS-ACT-003 prototype must pass exact-head focused tests before its diagnostic output is treated as reproducible.
