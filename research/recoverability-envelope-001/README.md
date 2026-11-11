# RECOVERABILITY-ENVELOPE-001

**Status:** protocol and reference evaluator; no driving-policy result  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4747  
**Related renderer:** https://github.com/Luminous-Dynamics/symtropy/pull/4616  
**Related replay harness:** https://github.com/Luminous-Dynamics/symtropy/pull/4647  
**Related branch auditor:** https://github.com/Luminous-Dynamics/symtropy/pull/4556  
**Resource-normalized frontier:** https://github.com/Luminous-Dynamics/symtropy/pull/4642  
**Symthaea visual/proprioception bridge:** https://github.com/Luminous-Dynamics/symthaea/pull/11480

## Research question

Can an embodied controller preserve a recoverable operating envelope when faults are persistent, bursty, correlated, and arrive during high-demand manoeuvres—and can the system detect degradation, enter fallback, and recover within a declared temporal budget?

This track consumes the renderer, observation replay, branch evidence and resource accounting owned by the related work. It is not another simulator, camera renderer, controller, or safety authority.

## What is new relative to Bench2Drive-Robust

Bench2Drive-Robust (May 18, 2026) studies closed-loop deployment perturbations including camera frame drops/partial observations, ego-state estimation errors and inference delay. We adopt it as an external comparator.

This protocol adds a distinct temporal layer:
- multiple faults in a sequence, with persistent and burst intervals;
- explicit concurrency and common-cause grouping;
- fault onset relative to a manoeuvre phase;
- first detection and detection delay;
- first fallback and fallback delay;
- fault-envelope clearance and stable nominal recovery;
- interrupted recovery when a second fault arrives;
- per-event and per-episode denominator accounting;
- a multi-axis recoverability frontier, not one aggregate score.

The core distinction is that a single perturbation's degraded score is not enough. A system must show it recognized the fault, changed its operating mode in time, remained inside an independently declared constraint envelope, and returned to nominal operation only after the entire scheduled fault set had cleared and sensor freshness/compute deadlines were restored.

## Reference evaluator

A standalone Rust 2024 crate pinned to Rust 1.96.0 lives in `research/recoverability-envelope-001/`.

Run against the example:

```sh
cargo run --release --locked \
  --manifest-path research/recoverability-envelope-001/Cargo.toml -- \
  research/recoverability-envelope-001/fixtures/compound-merge-recovery-v1.json \
  ./recoverability-report.json
```

Exit codes:
- `0`: structurally valid finite trace; a multi-axis report is written;
- `1`: trace rejected; a report with deterministic validation findings is written;
- `2`: file, size, invocation or typed-JSON error.

The input is bounded to 64 MiB. Unknown JSON fields are rejected. SHA-256 fields must be lowercase 64-character hex; this checks syntax, not authenticity.

## Episode contract

Every event has:
- stable fault ID and kind;
- channel;
- half-open interval `[onset_tick, end_tick_exclusive)`;
- severity in parts per million;
- optional correlation group and critical manoeuvre phase.

Every tick has:
- camera and vehicle-state observation ages (null means not observed, never silently fresh);
- inference duration and control deadline;
- detected fault IDs;
- explicit fault IDs that caused fallback (required on fallback ticks; forbidden otherwise);
- operating mode (nominal / degraded / fallback);
- independently supplied oracle outcome (within envelope / degraded within envelope / constraint violation / unscored).

The contract fixes maximum detection delay, fallback delay, recovery latency, stable-nominal interval, freshness limits and fault concurrency.

### Stable recovery

The evaluator credits recovery only after `min_stable_nominal_ticks` consecutive ticks that satisfy all of:
- no scheduled fault remains active anywhere in the episode;
- mode is nominal;
- oracle outcome is `within_envelope`;
- inference duration does not exceed the control deadline;
- camera age and vehicle-state age are both present and within their limits.

This conservative rule intentionally prevents an event from being declared recovered while another scheduled fault is still active. A separate future extension can report per-channel partial recovery, but that must not be confused with full-envelope recovery.

## Reported axes

Per fault:
- detected, detection tick and detection delay;
- fallback observed, attributed fallback tick and fallback delay;
- deadline misses during the event versus an explicit per-fault miss budget;
- recovery tick and recovery latency;
- fallback ticks until recovery;
- degraded ticks during the event;
- constraint-violation ticks and first violation tick;
- contract-met status and explicit unmet conditions.

Per episode:
- expected/observed tick denominator;
- fault event count and correlation-group count;
- maximum concurrent faults;
- detected/fallback/recovered/contract-met event counts;
- total constraint-violation ticks;
- deadline-miss ticks;
- unscored ticks.

These fields are intentionally not reduced to a weighted “safety score”. Compare native task metrics, capability retention, fault-specific recovery distributions, constraint violations, latency, energy and human interventions as separate axes. Pair the faulted run with a clean baseline using the same world seed, scenario, simulator and evaluator identity. Use the #4642 resource evaluator for capability-versus-resource frontiers.

## Comparison arms

At minimum:
1. existing state-only HDC-LTC;
2. compact GRU/LSTM;
3. compact transformer;
4. learned vision + structured HDC scene binding + HDC-LTC/CfC;
5. the hybrid plus action-conditioned latent prediction;
6. decision-sensitive acquisition using #4550/#4639 owners;
7. Bench2Drive-Robust-compatible methods wherever the external evaluator and checkpoints are available.

Match policy-visible inputs, training transitions, human labels, scenario seeds, fault schedule, hardware and checkpoint privileges. Report mismatches. HDC/LTC earns no presumption of superiority.

## Prospective targets (not results)

- 100% tick and event denominator accounting.
- Zero unknown or duplicate fault IDs in accepted evidence.
- 100% explicit per-fault detection/fallback/recovery reporting.
- At least 20% fewer recovery ticks or human-intervention minutes than a matched compact baseline, with no increase in independently scored constraint violations or worst-family regressions.
- A statistically supported HDC-LTC advantage in at least one predeclared dimension: burst memory, asynchronous observation timing, correlated fault handling, recovery latency, inference energy or deadline robustness.
- No claims of roadworthiness, collision probability or real-world safety from these finite trace evaluations.

All thresholds are prospective research goals, not achieved results or certification requirements.

## Files

- `schemas/episode-v1.schema.json`: versioned typed input contract.
- `fixtures/compound-merge-recovery-v1.json`: ten-tick correlated camera-drop/inference-delay fixture.
- `src/lib.rs`: typed validator, recoverability metrics and adversarial tests.
- `src/main.rs`: bounded JSON CLI.
- `Cargo.lock`: dependency checksums copied from the repository's canonical lockfile.

## Claim ceiling

A valid report establishes only internal consistency and metrics over the exact declared trace and evaluator contract. It does not authenticate injected faults, execute vehicle/actor dynamics, verify a real fallback controller, prove a physical safety envelope, estimate real-world collision risk, validate a learned model, or authorize vehicle actuation.
