# ROAD-ROBUSTNESS-EXEC-001 — deployment perturbation manifest v0.1

**Status:** protocol template; not reproduced  
**Tracking issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4489  
**External protocol:** [Bench2Drive-Robust](https://github.com/Thinklab-SJTU/Bench2Drive-Robust), [paper](https://arxiv.org/abs/2605.18059)  
**Simulator compatibility declared upstream:** CARLA 0.9.15  
**Execution owner:** Symtropy #3532 / #3387  
**Measurement consumers:** Symthaea #11291 / #11349 / #10043 / #10320 / #10012

## Purpose

Freeze the upstream deployment-perturbation semantics before implementing an adapter or running a comparison. The companion JSON is deliberately a template: the exact Bench2Drive-Robust commit, Bench2Drive revision, route manifest, model/checkpoint identities, dependency lock and artifact hashes remain unresolved and must be pinned before reproduction.

This document does not claim that the upstream benchmark has been installed, executed, independently reproduced, or validated by SAM.

## Protocol families to preserve

The upstream README/paper describes these perturbation families:

| Family | Published setting | Important semantic distinction |
|---|---|---|
| GPS localization noise | 5 m / 15 m | Localization input perturbation |
| Inference/control latency | 100 / 200 / 500 ms | FIFO action-buffer delay; not wall-clock model latency |
| Burst frame drop | 20 / 60 ticks | Cached/frozen camera-frame behavior |
| Partial observation | 50% / 80% occlusion | Camera input corruption |
| Speed estimation noise | Gaussian multiplicative settings, mean 0.5 or 0.2, standard deviation 0.2 | Ego-speed estimate perturbation |

The upstream example configuration also uses a burst trigger probability of 0.01. The exact setting, seed, mode and injection point must be read from the pinned upstream revision rather than assumed from this summary.

Do not silently substitute blur for occlusion, resample a cached frame as a new observation, reinterpret FIFO delay as measured inference time, or merge sensor-side latency with actuator-side latency.

## Paired experiment semantics

A clean/perturbed pair begins from the same frozen initial condition and route/scenario identity. Pairing does not mean forcing downstream trajectories to remain identical after an intervention.

Use common exogenous random events only when the declared causal model says the intervention cannot alter that event channel. If a perturbation changes actor response, future visibility, collision geometry or subsequent observations, those descendants must be recomputed and separately receipted.

Keep these timestamps distinct:
- sensor acquisition;
- sensor delivery to the controller;
- policy decision;
- action proposal;
- action-buffer enqueue/dequeue;
- command latch / actual execution;
- world transition;
- monitor detection;
- last admissible action deadline.

All timestamps require a qualified clock domain and transformation. Same-tick ordering without a happens-before relation is unresolved.

## Evidence contract

Each run should eventually bind:
- source repository and immutable commit IDs;
- CARLA build, Bench2Drive revision, route/scenario and checkpoint hashes;
- perturbation family, exact parameters, units, seed and injection point;
- initial state, event-keyed random manifest and paired-run relation;
- acquisition/delivery/availability receipts;
- proposed, buffered, latched and executed action receipts;
- expected branch/event manifest and completeness;
- native upstream DS/SR/RD outputs and their denominators;
- SAM option, policy-tree, timing, fallback and replay measurements.

Native benchmark outputs must remain unchanged and separately reported. SAM measurements are an additional evidence layer, not a replacement score.

## Required first slice

Start with:
1. one clean control;
2. fixed FIFO action delay at 100 ms;
3. burst frame drop at 20 ticks;
4. exact action-buffer and observation-delivery receipts;
5. a fresh-process replay from immutable inputs.

These are selected first because timing and stale-observation semantics are central to SAM's decision-boundary work. Expand to other published severities only after the first slice is reproducible.

## Claim boundary

A successful manifest validation establishes only that the declared experiment is structurally complete. A successful simulator run establishes only the named simulator-relative result. Neither establishes real-world sensor-fault frequency, physical recoverability, collision probability, roadworthiness, or SAE Level 4/5 capability.

## Source notes

- Bench2Drive-Robust repository: https://github.com/Thinklab-SJTU/Bench2Drive-Robust
- Bench2Drive-Robust paper: https://arxiv.org/abs/2605.18059
- Bench2Drive CARLA setup: https://github.com/autonomousvision/carla_garage
- NIST MSAV measurement programme: https://www.nist.gov/programs-projects/measurement-science-automated-vehicles-0
