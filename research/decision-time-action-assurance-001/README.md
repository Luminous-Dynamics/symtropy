# DECISION-TIME-ACTION-ASSURANCE-001

**Status:** executable Rust reference and CLI; no vehicle-control or benchmark run  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4961  
**Related Symthaea input bridge:** https://github.com/Luminous-Dynamics/symthaea/pull/11480  
**Related renderer/replay:** https://github.com/Luminous-Dynamics/symtropy/pull/4616 and https://github.com/Luminous-Dynamics/symtropy/pull/4647  
**Related branch evidence auditor:** https://github.com/Luminous-Dynamics/symtropy/pull/4556

## The paradigm shift

A driving model's output should not be treated as a command merely because it was reasonable when inference completed. The evidence can become stale, be superseded, become inconsistent with another sensor, or lose its validity while the command waits in a queue.

This crate introduces a short-lived **action-evidence envelope**:

`action proposal -> exact observation bundle -> declared evidence profile -> authorization envelope -> execution-time revalidation`

The proposal is authorized only against the exact evidence bundle it names. The envelope's exclusive expiry is the minimum of:
1. the proposal's own expiry;
2. the policy's maximum authorization lifetime;
3. the earliest freshness deadline among all required evidence streams.

At execution time, the gate rechecks the proposal, complete evidence-bundle digest, policy digest, envelope commitment and current time. A new frame, changed proprioception, policy change, expired proposal or altered action invalidates the previous envelope. The consuming system must make a new decision.

This is a model-independent assurance interface. Symthaea proposes and binds the action; Symtropy checks whether the evidence contract still holds. The library does not select the fallback action, drive the vehicle or call an actuator.

## What the field is doing (13 November 2026)

### Bench2Drive and Bench2Drive-Robust

The current Bench2Drive v0.0.4 published closed-loop table reports LEAD at 95.59 Driving Score and 87.28% Success Rate over the benchmark's 220-route evaluation set. This is a driving-performance reference, not a runtime assurance result.

Bench2Drive-Robust is the closest existing comparator for deployment-side evidence faults. Its public benchmark sweeps:
- 20- and 60-tick burst frame drops;
- 50% and 80% partial camera observations;
- 5 m and 15 m GPS localization noise;
- 100, 200 and 500 ms control delays;
- speed-estimation noise.

This project does **not** claim to introduce those perturbations. It adds a separate test: did the system issue an action authorization tied to the evidence that was actually available, and did it reject that authorization when the evidence changed or expired before execution?

### DriveDegrade: distinguish robustness from sensor indifference

DriveDegrade (arXiv:2608.29005, 29 August 2026) studies 16 image-degradation families at five severities and separates quality degradation from information deletion. Its central warning is relevant: a policy that barely changes when its cameras are blinded may be relying on an ego-state shortcut rather than robust visual evidence.

The next model-evaluation layer should therefore include paired tests:
- remove decision-critical evidence and verify that the action or authorization changes appropriately;
- remove irrelevant evidence and verify that the action remains stable;
- compare both with a clean baseline.

A flat score under camera deletion is not, by itself, evidence of robustness.

### Industry edge models

NVIDIA announced Alpamayo 2 Edge on 10 November 2026: a 2B-parameter adaptive-reasoning VLA targeting DRIVE AGX Thor, using surround-camera context, navigation inputs, a trajectory action expert and closed-loop RL. NVIDIA reports a 48.9% closed-loop improvement from its RL post-training over the best pre-RL checkpoint. Its deployment article describes 160–190 ms pixel-to-trajectory latency on DRIVE AGX Thor with its acceleration recipe.

Our contribution is not another large reasoning model. It is to test whether an independent, low-cost evidence gate can keep the action's validity bounded while the policy uses its available inference budget for driving.

### Operational evidence

Waymo's safety dashboard reports 271.3 million rider-only miles through June 2026 and publishes location-adjusted comparison rates with 95% confidence intervals. That is a distinct, operational evidence class. Passing the deterministic tests in this crate is not comparable to fleet mileage or crash-rate evidence.

## What this crate implements

- Typed observation samples per modality and sensor ID, including capture/ready/delivery timestamps, clock identity, sequence, payload digest, calibration digest, sensor health and fresh/delayed disposition.
- Explicit missing-evidence events. A newer missing event invalidates a prior sample until a newer delivered sample is recorded.
- Caller-versioned evidence profiles, each declaring required sensors, minimum temporal samples, maximum capture age, processing delay, delivery delay, inter-sample gap, sensor health and delayed-frame tolerance.
- Nominal and explicitly restricted profiles. A restricted profile has its own allowed action classes and evidence rules; the gate never invents a degraded action.
- Sorted reason codes and a three-way decision: `authorized`, `authorized_restricted` or `fallback_required`.
- A deterministic evidence-bundle commitment whose canonicalization is independent of input array ordering.
- An authorization envelope binding the policy, proposal, action digest, evidence digest, profile, authorization time and exclusive expiry.
- Pre-execution revalidation. Any changed policy, proposal or evidence bundle requires reassessment.
- Strict JSON CLI and request/report schemas.

## Run

The crate is a member of the public Symtropy workspace and uses the repository's Rust 1.96.0 pin.

```sh
cargo fmt --manifest-path research/decision-time-action-assurance-001/Cargo.toml -- --check
cargo test --locked --manifest-path research/decision-time-action-assurance-001/Cargo.toml
cargo clippy --locked --manifest-path research/decision-time-action-assurance-001/Cargo.toml --all-targets --all-features -- -D warnings
```

Evaluate a request:

```sh
cargo run --release --locked \
  --manifest-path research/decision-time-action-assurance-001/Cargo.toml -- \
  request.json report.json
```

Exit codes: 0 for nominal/restricted authorization, 1 for fallback required (report is still written), 2 for malformed input or invocation/I/O failure. The CLI never executes the action.

The request schema is `schemas/gate-request-v1.schema.json`; the output schema is `schemas/gate-report-v1.schema.json`.

## Benchmark: Evidence-to-Action Validity

The benchmark manifest is `benchmark_manifest_v0_1.json`. It preserves existing benchmark protocols and adds a separate decision-time layer.

### Core fault families

- missing required camera, localization or proprioception;
- delayed, stale, future-dated or out-of-order observations;
- sensor clock identity mismatch and excessive inter-sensor skew;
- invalid or degraded sensor health;
- sequence duplication and timestamp regression;
- missing-event record after the last usable sample;
- changed payload/calibration digest after proposal;
- action mutation, proposal expiry and policy revision change;
- a new observation arriving after authorization but before execution (time-of-check/time-of-use race).

### Metrics

- **Invalid authorization rate:** invalid proposals authorized / invalid proposals submitted.
- **Stale execution acceptance rate:** expired or superseded envelopes accepted / invalidated envelopes submitted.
- **Evidence-bundle mismatch acceptance:** executions accepted against a bundle other than the proposal's exact source bundle.
- **False restriction rate:** nominally valid proposals unnecessarily sent to restricted mode or fallback.
- **Recovery opportunity:** time between first evidence invalidation and the last legal/declared opportunity for reassessment.
- **Gate latency:** p50/p95/p99 evaluation and pre-execution revalidation latency on a pinned CPU and software profile.
- **Capability retention:** task progress retained under an explicitly restricted action profile, reported separately from nominal driving score.
- **Evidence dependence (future policy stage):** sensitivity to removal of decision-critical evidence versus irrelevant-input ablation.

### Prospective targets (not results)

- 100% rejection of known-invalid deterministic fixtures.
- Zero authorization after the exclusive envelope expiry.
- Zero acceptance after proposal, policy or evidence-bundle mutation.
- 100% expected/observed event denominator reconciliation.
- Research performance target: p99 gate evaluation below 1 ms on a pinned reference CPU for a bounded fixed-size bundle. This is not measured here.
- False restriction and capability-retention thresholds will be set only after reproducing the baselines on the same benchmark split and hardware.

## Integration path

1. Feed the Symthaea #11480 bridge receipts and Symtropy #4616 renderer receipts into an `EvidenceBundle`.
2. Have Symthaea emit an `ActionProposal` with the exact bundle digest and intended action digest.
3. Evaluate the proposal through this gate.
4. At command handoff, call `revalidate_at_execution` against the latest bundle, policy and monotonic time.
5. If any binding changed, discard the prior envelope and require a new proposal. The gate itself does not decide the safe vehicle response.

The #4647 replay harness is an appropriate first integration fixture, followed by fault injection using Bench2Drive-Robust settings.

## Claim ceiling

- This establishes consistency of finite typed records and time-window contracts only.
- SHA-256 commitments are unkeyed; they detect accidental or unauthorized mutation only when the surrounding trust boundary is sound. They do not authenticate a publisher or prove source truth.
- Fresh, calibrated sensor data can still be semantically wrong. The gate does not prove that a pedestrian, lane boundary or traffic light was correctly understood.
- A valid envelope does not prove the action is safe. The caller owns the evidence profiles and the independent fallback mechanism.
- No vehicle dynamics, controller, actuator, public-road experiment, certification, roadworthiness or SAE automation-level claim is included.
