# DEGRADATION-AND-RECOVERY-ENVELOPE-001

**Status:** deterministic seven-family schedule, typed paired-trace auditor, causal fault relationships, campaign/report schemas, and independent plus compound fixtures; no trained policy or vehicle-dynamics result  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4657  
**Related capability frontier:** #4639  
**Related camera receipt boundary:** #4595 / #4616  
**Related renderer-to-policy bridge replay:** #4618  
**Related counterfactual branch auditor:** #4556

## The paradigm shift: measure the whole recovery envelope

A driving stack is not robust merely because it completes a nominal route or because it continues moving under a fault. It must identify corrupted or unavailable evidence, contain the fault, retain useful capability where justified, and only return to normal after the observation stream has recovered.

The state trajectory is:

`healthy -> fault onset -> detection -> containment -> degraded/fallback operation -> fault clearance -> healthy-observation quorum -> normal operation`

We therefore report event-specific latency, rejection, false-positive, recovery, deadline and resource axes separately. There is deliberately no weighted composite "robustness score".

## What the field is measuring (snapshot: 11 November 2026)

The current industry landscape makes a nominal-score-only comparison inadequate:

| Reference | Publicly reported result or capability | Correct comparison use |
|---|---|---|
| Bench2Drive v0.0.4 | LEAD: 95.59 Driving Score, 87.28% Success Rate; separate efficiency and comfort values | Closed-loop route benchmark. Pin exact tag, route/sensor profile and evaluator; do not infer fault recovery from the nominal aggregate |
| NVIDIA Alpamayo 2 Edge | Model card: 2.45B-parameter backbone plus 0.2B action expert; 115,000 hours of multi-camera driving video and about 3.7M Chain-of-Causation traces; 15.15 ± 0.04 km per at-fault incident on 875 AlpaSim scenarios with three rollout sets; separate minADE_6 0.52m at 5s and Lingo-Judge 75.2 | Strong edge-oriented VLA reference. These are separate closed-loop, open-loop and reasoning measures, not interchangeable metrics |
| Alpamayo 2 Edge deployment recipe | NVIDIA reports 160–190ms pixel-to-trajectory latency on DRIVE AGX Thor-X with DRIVE OS 7.2.7.0, CUDA 13.4 and TensorRT 11.3.1.9 | Reference for a precisely declared target-hardware latency experiment; not comparable to laptop CPU timing |
| WOD-E2E, CVPR 2026 | 4,021 segments, approximately 12 hours, curated for challenging long-tail scenarios with frequency below 0.03% | Useful rare-event dataset reference. It is not itself a closed-loop safe-stop or recovery score |
| Waymo Open Dataset | The 2026 formal challenge programme is not running, but its public leaderboards remain active | Candidate public datasets and evaluator family; distinguish public leaderboard metrics from operational fleet evidence |
| CARLA Leaderboard 2.0 | Historical sensor/map tracks and a multi-metric route evaluator; the official site states submissions and evaluation servers are permanently closed | Protocol reference only. Do not present it as a currently open submission track |

Sources:
- Bench2Drive v0.0.4: https://github.com/Thinklab-SJTU/Bench2Drive/blob/0.0.4/README.md
- Alpamayo 2 Edge model card and evaluation: https://huggingface.co/nvidia/Alpamayo2-Edge
- NVIDIA edge deployment and latency recipe: https://developer.nvidia.com/blog/running-av-reasoning-models-within-onboard-compute-and-latency-limits/
- WOD-E2E CVPR 2026: https://openaccess.thecvf.com/content/CVPR2026/html/Xu_WOD-E2E_Waymo_Open_Dataset_for_End-to-End_Driving_in_Challenging_Long-tail_CVPR_2026_paper.html
- Waymo Open Dataset challenge status: https://www.waymo.com/open/challenges/
- CARLA Leaderboard 2.0 evaluation notice: https://leaderboard.carla.org/evaluation_v2_0/

The Alpamayo model card reports a 2.45B backbone plus 0.2B action expert, while NVIDIA's blog headline describes the model as a 2B-parameter edge model. We preserve the model-card breakdown rather than silently equating the two counts. Reported model-card results are vendor-published and are not independently reproduced by this repository.

## Fault families in v0.1

The deterministic schedule generator supports:
- camera drop;
- delayed camera frame;
- stale-frame replay;
- pixel corruption;
- calibration shift/mismatch;
- clock drift/violation;
- compute overrun.

The typed protocol permits concurrent events, provided each tick explicitly identifies every active condition and each detection/containment record references the exact fault-event ID. A generic `fault_detected=true` flag is intentionally insufficient: it could otherwise credit one fault's detection to another.

Each event also declares its causal relationship:
- `independent`: no common-cause group and no parent event;
- `common_cause`: a non-empty `causal_group_id` shared by at least two fault events;
- `cascaded`: a `parent_event_id` that identifies an earlier or same-tick declared event.

The validator rejects singleton common-cause groups, missing parents, parents that start after their children, and causal cycles. These fields record the campaign's declared causal model; they do not prove that the physical common cause or cascade is true.

## Metrics (never collapse these axes)

Per fault event:
1. exposure ticks and severity;
2. detection latency from fault onset;
3. containment latency from fault onset;
4. whether each predeclared detection and containment deadline was met;
5. stale/corrupt policy-input acceptances;
6. tick of return to normal;
7. recovery latency after fault clearance;
8. number of consecutive healthy observations required before recovery.

Per paired run:
- clean-control false-positive degradation ticks;
- full expected/observed tick denominators;
- missed policy deadlines under a predeclared campaign deadline;
- decision latency p50/p95/p99/max;
- inference energy p50/p95/p99/max;
- input-integrity rejection and event-reference validity;
- failed, incomplete and censored run state.

A camera drop may reduce usable observations while still permitting a safe degraded mode; a stale or corrupt frame accepted as valid is a different and more severe integrity failure. A system that never returns to normal is also not a recovery success. We report these outcomes separately.

## Fair comparison protocol

- Pair clean-control and faulted runs with the same scenario seed, exogenous environment-randomness digest, expected tick count, evaluator revision and clock identity.
- Freeze the fault schedule, severity, duration, detection/containment deadlines, max acceptable delayed-frame age and recovery quorum before model evaluation.
- Keep the fault schedule evaluator-side; the policy must not receive the future injection plan. Campaigns declare `fault_schedule_visible_to_policy=false`; this is a recorded control, not cryptographic proof of the policy's actual inputs.
- Require every expected tick exactly once. Missing, duplicate, reordered and out-of-domain tick IDs are findings; never delete failed/censored outcomes from the denominator.
- Detection and containment traces must reference the exact event ID. Unknown or duplicate event references are rejected. A containment reference is accepted only on a tick that records degraded mode, fallback or completed safe stop.
- Do not call an exogenous random process "common random numbers" unless its shared key and intervention invariance have independent provenance; use #4556 for branch records.
- Measure the whole stack and declared target hardware. Compare HDC/LTC with compact GRU/LSTM and transformer baselines using identical input privileges, fault schedule, scenario, training exposure and system budget.
- Keep native closed-loop route metrics, fault recovery, reasoning scores, open-loop trajectory error and energy/latency in their own metric families.

## Candidate and resource provenance

Every campaign must identify:
- model ID, model status and model revision;
- checkpoint SHA-256 for a trained policy (optional only for explicitly marked untrained-input-bridge or synthetic-reference fixtures);
- hardware ID, inference precision, power profile and energy measurement method;
- exact policy-visible input set.

This prevents a result card from describing only a policy name while hiding a different checkpoint, precision, hardware target or privileged input. The current positive fixture deliberately declares:
- `model_status = untrained_input_bridge`;
- `checkpoint_sha256 = null`;
- `hardware_id = synthetic-fixture-host-not-measured`;
- `power_profile = not-measured-fixture`;
- `energy_measurement_method = synthetic-fixture-values-not-hardware-measurement`.

Its numeric latency and energy fields are synthetic fixture values used to test the auditor's arithmetic and completeness rules. They are **not hardware measurements** and must not enter a capability frontier as measured results. A real trained-policy campaign must supply its exact checkpoint digest and measured host/power/energy methodology.

The declared `policy_visible_inputs` list and `fault_schedule_visible_to_policy=false` flag make intended privileges explicit; the auditor cannot authenticate the actual runtime wiring. Raw frame/receipt artifacts and independent runtime attestation remain necessary for empirical claims.

## Reusable Rust tooling

Pinned to Rust 1.96.0. The crate lives at `research/degradation-recovery-envelope-001/`.

### Generate a deterministic seven-family schedule

```sh
cargo run --locked \
  --manifest-path research/degradation-recovery-envelope-001/Cargo.toml -- \
  generate-schedule 10 5 3 1 1 2 \
  research/degradation-recovery-envelope-001/fixtures/seven-family-schedule.json
```

Arguments are first start tick, spacing ticks, exposure duration, detection deadline, containment deadline and healthy-recovery quorum. The generator rejects a spacing that cannot accommodate the full exposure plus the recovery quorum. It returns an event schedule only; a simulator/injector must still execute each intervention and record the actual trace.

### Generate a deterministic severity sweep

Exercise every fault family at several predeclared severities. The generator emits independent, non-overlapping event windows and reserves the full exposure plus the healthy-recovery quorum before the next event:

```sh
cargo run --locked \
  --manifest-path research/degradation-recovery-envelope-001/Cargo.toml -- \
  generate-severity-sweep 10 5 3 1 1 2 100000,250000,500000,750000,1000000 \
  research/degradation-recovery-envelope-001/fixtures/severity-sweep.json
```

The five severity levels are 10%, 25%, 50%, 75% and 100% of the declared fault-family-specific severity scale. These values are control parameters, not calibrated physical equivalences across fault families. With five levels, the schedule contains 35 events (seven fault families × five severities). It does not inject the faults; an environment adapter must execute the interventions and record their observed effects.

### Audit a paired campaign

```sh
cargo run --locked \
  --manifest-path research/degradation-recovery-envelope-001/Cargo.toml -- \
  research/degradation-recovery-envelope-001/fixtures/camera-drop-paired-v1.json \
  ./out/camera-drop-audit.json
```

Exit codes: 0 structurally valid; 1 rejected campaign (report still written); 2 invalid input/invocation.

### Authored adversarial tests

The current branch contains **41 library tests and 9 CLI tests**. Coverage includes:
- event-specific detection/containment attribution and the negative control where one fault attempts to claim another fault's detection;
- missing, stale, corrupt, delayed, calibration, clock and compute-fault condition accounting;
- clean-control false positives, missing latency/energy evidence and exact tick denominators;
- deadline misses versus undetected faults, healthy recovery quorum and premature return to normal;
- independent, cascaded and common-cause fault declarations, including parent-cycle rejection and overlapping exposure requirements for common-cause groups;
- deterministic seven-family schedules and deterministic multi-severity schedules, including overflow, duplicate severity and overlapping-recovery-window rejection;
- checked-in camera-drop, compound common-cause and 35-event severity-sweep fixtures;
- typed CLI parsing, unknown-field rejection and native-metric separation without a composite score.

These are authored test cases, not evidence that the Rust toolchain has executed them. The exact-head GitHub Actions workflow remains the compiler/test authority. Queued or pending is not a pass.

### Check and test

```sh
cargo fmt --manifest-path research/degradation-recovery-envelope-001/Cargo.toml -- --check
cargo test --locked --manifest-path research/degradation-recovery-envelope-001/Cargo.toml
cargo clippy --locked --manifest-path research/degradation-recovery-envelope-001/Cargo.toml --all-targets --all-features -- -D warnings
```

Schemas:
- `schemas/campaign-v1.schema.json`
- `schemas/report-v1.schema.json`

Fixture:
- `fixtures/camera-drop-paired-v1.json` — eight paired ticks, camera drop on ticks 3–4, event-attributed detection and containment, two healthy observations before normal mode resumes on tick 7.
- `fixtures/compound-common-cause-camera-clock-v1.json` — the same paired eight-tick episode with simultaneous camera-drop and clock-drift faults attributed to one declared power/clock-domain common cause; the auditor reports both event latencies independently.
- `fixtures/severity-sweep.json` — generated schedule only; do not audit it as an observed trace or present it as a fault-injection result.

## Prospective target ladder (not measured results)

- **Tier 0 — protocol:** exact tick/event denominator; deterministic schedule and report; exact event-ID attribution.
- **Tier 1 — input integrity:** zero stale/corrupt/calibration-mismatch/clock-invalid policy inputs accepted.
- **Tier 2 — detection/containment:** every fault detected and contained within a predeclared, platform-calibrated deadline, or counted as a miss.
- **Tier 3 — recovery:** no normal-mode restoration before the healthy-observation quorum; report recovery latency per event.
- **Tier 4 — comparative improvement:** statistically supported improvement in recovery latency or usable-capability retention versus matched compact recurrent and transformer controls, with no worst-fault-family regression and no increase in false-positive degradation.
- **Tier 5 — external evaluation:** exact pinned closed-loop evaluation on Bench2Drive/AlpaSim or a comparable simulator, then HIL/closed-course only under independent safety governance.

Do not impose universal millisecond deadlines before the sensor cadence, target hardware and safety requirements are known. Freeze each deadline before the model sees the test schedule.

## Next implementation step

Wire this schedule into Symtropy #4616/#4618 and inject the first faults at the actual observation boundary: a missing frame, a delivered-late frame, a repeated/stale sequence, pixel bytes that no longer match the upstream receipt, camera-calibration digest mismatch and a deliberate compute-deadline overrun. Then run the same episode through the real Symthaea bridge and record whether it rejects, degrades and recovers as specified. Only after this works should we add vehicle dynamics and closed-loop policy comparisons.

## Claim ceiling

The current code validates paired trace consistency, exact event-attributed detection/containment evidence, state-backed containment, deadline-compliant recovery quorum and declared software-fault accounting. It does not yet inject faults into the Symthaea bridge, execute a traffic simulator, evaluate a trained driver, measure real recovery performance, prove physical fault fidelity, establish safe-stop capability, transfer to public roads or provide a vehicle safety case.
