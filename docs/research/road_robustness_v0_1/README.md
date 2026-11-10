# ROAD-ROBUSTNESS-EXEC-001 — deployment perturbation manifest v0.1

**Status:** protocol source audit in progress; no benchmark execution or reproduction claimed  
**Tracking issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4489  
**Protocol-assurance research:** https://github.com/Luminous-Dynamics/symthaea/issues/11393  
**External protocol:** [Bench2Drive-Robust](https://github.com/Thinklab-SJTU/Bench2Drive-Robust), [paper](https://arxiv.org/abs/2605.18059)  
**Pinned Robust source candidate:** `ae1b5867324710bf0574cba797062280d9d97105`  
**Simulator compatibility declared upstream:** CARLA 0.9.15  
**Execution owner:** Symtropy #3532 / #3387  
**Measurement consumers:** Symthaea #11291 / #11349 / #10043 / #10320 / #10012

## Purpose

Freeze the external deployment-perturbation protocol, then audit whether the code implements the published and README-level claims. The companion JSON is deliberately a template: the Bench2Drive base revision, route manifest, agent repositories, model/checkpoint identities, dependency lock and artifact hashes remain unresolved and must be pinned before reproduction.

This document does not claim that the upstream benchmark has been installed, executed, independently reproduced, or validated by SAM.

## Published perturbation families

The upstream README/paper describes:

| Family | Published setting | Important semantic distinction |
|---|---|---|
| GPS localization noise | 5 m / 15 m | Localization input perturbation |
| Inference/control latency | 100 / 200 / 500 ms | README labels this FIFO action buffering |
| Burst frame drop | 20 / 60 ticks | Cached/frozen camera-frame behavior |
| Partial observation | 50% / 80% | README labels these occlusion ratios |
| Speed estimation noise | Gaussian multiplicative settings, mean 0.5 or 0.2, standard deviation 0.2 | Ego-speed estimate perturbation |

The upstream example configuration also uses a burst trigger probability of 0.01. These are declared settings, not proof that every realized event has the label's exact duration or dose.

## Pinned source review: action-latency wrapper

The public `agent_wrapper.py` was inspected at the repository's one-commit snapshot:

- [Pinned wrapper source](https://github.com/Thinklab-SJTU/Bench2Drive-Robust/blob/ae1b5867324710bf0574cba797062280d9d97105/leaderboard/leaderboard/autoagents/agent_wrapper.py)
- [Upstream discussion #1](https://github.com/Thinklab-SJTU/Bench2Drive-Robust/issues/1)

The source-level behavior is more specific than the README's "FIFO" label:

1. `WARMUP_STEPS` defaults to 20.
2. Fixed delay is converted to integer ticks by truncating `latency_ms * SIM_RATE / 1000`.
3. Once warm-up and sensor-readiness conditions pass, the policy is invoked and the command is stored as the next command.
4. The next switch is scheduled at current step + delay ticks.
5. At that step the previous result becomes current, another policy invocation occurs, and the next switch is scheduled.
6. On intervening ticks, the currently released command is held. The code does not invoke the policy every tick and enqueue a new command every tick in this fixed-delay branch.
7. Before the first command is released, the wrapper returns a default control with zero throttle, brake and steering. This is not an explicit braking/stop command.

At 20 Hz, the published 100/200/500 ms settings convert to 2/4/10 ticks. Under the wrapper's periodic invocation + sample-and-hold schedule, command age cycles from (d) through (2d-1) ticks, where (d) is the integer delay. The respective source-derived ages are:

| Requested setting | Delay ticks | Command age range | Mean age under this schedule |
|---|---:|---:|---:|
| 100 ms | 2 | 2–3 ticks | 2.5 ticks = 125 ms |
| 200 ms | 4 | 4–7 ticks | 5.5 ticks = 275 ms |
| 500 ms | 10 | 10–19 ticks | 14.5 ticks = 725 ms |

These are analytical consequences of the inspected control flow at 20 Hz, not runtime measurements. Exact observed command age must be measured from command-generation and command-application receipts. The machine-readable schedule fixture records this distinction.

Measured mode also truncates inference runtime to integer ticks and uses an equality-triggered next-step condition. A runtime shorter than one simulation tick yields a zero-tick delay; after the simulation step increments, the equality condition may never be satisfied again. This is a source-level edge case, not a claim that a particular published run encountered it.

## Paired experiment semantics

A clean/perturbed pair begins from the same frozen initial condition and route/scenario identity. Pairing does not mean forcing downstream trajectories to remain identical after an intervention.

Use common exogenous random events only when the declared causal model says the intervention cannot alter that event channel. If a perturbation changes actor response, future visibility, collision geometry or subsequent observations, those descendants must be recomputed and separately receipted.

Keep these timestamps distinct:
- sensor acquisition;
- sensor delivery to the controller;
- policy decision;
- inference start/end;
- command ready;
- action-buffer enqueue/dequeue;
- command latch / actual execution;
- world transition;
- monitor detection;
- last admissible action deadline.

All timestamps require a qualified clock domain and transformation. Same-tick ordering without a happens-before relation is unresolved.

## Candidate protocol discrepancies requiring further primary-source checks

An independent preflight audit reports these possible code/protocol mismatches. They are explicitly **not promoted to verified findings by this PR** until the exact pinned primary files and deterministic fixtures are independently checked:

| Candidate finding | Potential effect | Current SAM status |
|---|---|---|
| Burst duration sampled from a range below the maximum | Actual stale-frame exposure differs from the named 20/60-tick condition | Third-party source-audit lead; primary verification pending |
| Occlusion scales both mask dimensions by the ratio | 0.5 and 0.8 side ratios would correspond to 25% and 64% area, not 50% and 80% | Third-party source-audit lead; primary verification pending |
| Random streams not derived from stable route/condition identities | Paired and repeated runs may not receive matching faults | Third-party source-audit lead; primary verification pending |
| Gaussian speed multiplier not clipped/censored | Negative or out-of-domain speed estimates may enter the condition | Third-party source-audit lead; primary verification pending |
| Sensor interface accepts earlier buffered frames | Controller-visible observation age may differ from simulator tick | Upstream issue #1 raises this; primary verification pending |
| Subset metric merge retains full-suite denominator | Subset aggregates may be diluted | Third-party source-audit lead; primary verification pending |

External audit reference: [Bench2Drive-Robust preflight source review](https://github.com/manfromnowhere143/sentinel/blob/master/docs/research/BENCH2DRIVE_ROBUST_PREFLIGHT_2026-07-16.md). Treat it as a lead, not a substitute for SAM's own source review.

## Rights and reuse gate

The upstream README states that its code and assets are CC-BY-NC-ND unless otherwise specified. That is incompatible with assuming this upstream implementation can be copied, adapted, vendored or used as a dependency in a commercial Luminous product. The applicable rights for Bench2Drive base code, routes, maps, model repositories and checkpoints must be checked separately.

This PR records protocol facts and independently authored measurements; it does not copy upstream source. Before acquiring, modifying, redistributing, combining or commercially relying on any restricted code or assets, obtain appropriate written permission and counsel review. A clean-room harness from public behavioral specifications may be a separate option, subject to legal review.

## Evidence contract

Each run should eventually bind:
- source repository and immutable commit IDs;
- CARLA build, Bench2Drive revision, route/scenario and checkpoint hashes;
- perturbation family, exact parameters, units, seed and injection point;
- initial state, event-keyed random manifest and paired-run relation;
- acquisition/delivery/availability receipts;
- inference invocation, command-ready, buffered, latched and executed action receipts;
- realized perturbation dose per tick/frame;
- expected branch/event manifest and completeness;
- native upstream DS/SR/RD outputs and their denominators;
- SAM option, policy-tree, timing, fallback and replay measurements.

Native benchmark outputs must remain unchanged and separately reported. SAM measurements are an additional evidence layer, not a replacement score.

## Required first slice

Start with:
1. one clean control with equivalent warm-up handling;
2. a pure analytic command-schedule fixture for 100/200/500 ms;
3. fixed-delay wrapper smoke only after its exact behavior and rights are cleared;
4. independent true-FIFO reference arm with a separate experiment ID;
5. burst-frame timing and realized stale-age receipts;
6. fresh-process replay from immutable inputs.

The upstream periodic/held-command implementation and a true per-tick FIFO reference must never share one experiment label.

## Claim boundary

A successful manifest validation establishes only that the declared experiment is structurally complete. Source inspection establishes only the inspected source behavior. A simulator run establishes only the named simulator-relative result. None establishes real-world sensor-fault frequency, physical recoverability, collision probability, roadworthiness, or SAE Level 4/5 capability.

## Source notes

- [Bench2Drive-Robust repository](https://github.com/Thinklab-SJTU/Bench2Drive-Robust)
- [Pinned agent wrapper](https://github.com/Thinklab-SJTU/Bench2Drive-Robust/blob/ae1b5867324710bf0574cba797062280d9d97105/leaderboard/leaderboard/autoagents/agent_wrapper.py)
- [Upstream implementation questions](https://github.com/Thinklab-SJTU/Bench2Drive-Robust/issues/1)
- [Bench2Drive-Robust paper](https://arxiv.org/abs/2605.18059)
- [NIST MSAV measurement programme](https://www.nist.gov/programs-projects/measurement-science-automated-vehicles-0)
