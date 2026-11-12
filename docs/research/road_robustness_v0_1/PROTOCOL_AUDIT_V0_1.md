# Bench2Drive-Robust protocol audit — source-level findings v0.1

**Status:** source review in progress; no benchmark execution  
**Primary source pin:** Thinklab-SJTU/Bench2Drive-Robust @ `ae1b5867324710bf0574cba797062280d9d97105`  
**Tracking:** [Symtropy #4489](https://github.com/Luminous-Dynamics/symtropy/issues/4489), [Symthaea #11393](https://github.com/Luminous-Dynamics/symthaea/issues/11393)

## Confirmed from the pinned public README

The README says the Robust repository is a four-file replacement/overlay on an existing Bench2Drive installation, requires CARLA 0.9.15, advertises 100/200/500 ms inference/control latency, 20/60-tick burst frame drops, 50%/80% partial occlusion and multiplicative speed noise, and states that code/assets are CC-BY-NC-ND unless otherwise specified.

The README alone does not provide a complete reproducibility identity for the benchmark composition.

## Confirmed from the pinned agent wrapper source

The source file inspected is the exact pinned agent wrapper linked below. Its default warm-up is 20 steps; fixed latency is converted to integer ticks. The fixed-delay branch computes a policy command, schedules its release at current tick plus delay, releases it at that tick while computing the next command, and returns the current command on intervening ticks.

This is periodic invocation plus sample-and-hold. It is not a per-tick FIFO of every policy output. The wrapper's initial default control sets steering, throttle and brake to zero; that is not an explicit brake command.

At a 20 Hz setting, the source's integer delay ticks are 2, 4 and 10 for 100, 200 and 500 ms. Under the wrapper schedule, after first release, applied-command age cycles from d through 2d-1 ticks. The corresponding mean ages are 125, 275 and 725 ms. These are source-derived analytic consequences, not measured runtime latencies.

Measured mode converts inference seconds to integer ticks and uses an equality check on the scheduled step. For inference shorter than one simulation tick (under 50 ms at 20 Hz), the delay truncates to zero. The first result is not released on the next tick because the step advances beyond the equality target. This is a source-level edge case only; it has not been observed in CARLA.

## Source claims and evidence classes

| Claim | Evidence class | Current disposition |
|---|---|---|
| README advertises FIFO | Published documentation | Conflicts with inspected fixed-mode source semantics; author/runtime clarification needed |
| Fixed-delay cadence is periodic and holds last released command | Pinned source | Confirmed by source inspection |
| WARMUP_STEPS default is 20 | Pinned source | Confirmed default; launch-script overrides not yet checked |
| Measured mode can schedule zero ticks | Pinned source logic | Confirmed edge case by control-flow analysis; runtime not observed |
| Exact command-age sequence in CARLA | Runtime | Not observed |
| Table 3 used this exact wrapper and configuration | Experimental provenance | Not verified |
| Clean baseline used equivalent warm-up/sensor interface | Experimental provenance | Not verified |
| Burst duration, occlusion area, RNG derivation, speed clipping, sensor freshness, subset denominator discrepancies | Independent preflight leads | Not yet independently verified against primary source |

## Required follow-up

1. Inspect all upstream run scripts and freeze every environment-variable override.
2. Retrieve the exact Bench2Drive base/evaluator revision and route XML identity.
3. Inspect the perturbation processor, frame buffer, seed derivation, speed-noise generation and metric merge utility at the exact source commit.
4. Build deterministic analytic fixtures for every intervention.
5. Use separate experiment IDs for upstream periodic/held-command behavior and a true per-tick FIFO reference.
6. Measure actual tick-level event receipts in a legally cleared environment before making runtime claims.
7. Verify licensing across Robust, Bench2Drive, Bench2DriveZoo, route/map assets and model checkpoints before acquisition, modification or commercial use.

## Rights boundary

The README declares CC-BY-NC-ND for the upstream code/assets unless otherwise specified. This project must not vendor, copy, adapt, redistribute or commercially depend on those materials without suitable written permission and counsel review. Any independent test harness must be clean-room and based on public behavior descriptions, with provenance and rights review.

## Primary references

- [Pinned agent wrapper](https://github.com/Thinklab-SJTU/Bench2Drive-Robust/blob/ae1b5867324710bf0574cba797062280d9d97105/leaderboard/leaderboard/autoagents/agent_wrapper.py)
- [Upstream README](https://github.com/Thinklab-SJTU/Bench2Drive-Robust/blob/ae1b5867324710bf0574cba797062280d9d97105/README.md)
- [Upstream issue #1](https://github.com/Thinklab-SJTU/Bench2Drive-Robust/issues/1)
- [Independent preflight source-audit lead](https://github.com/manfromnowhere143/sentinel/blob/master/docs/research/BENCH2DRIVE_ROBUST_PREFLIGHT_2026-07-16.md)
