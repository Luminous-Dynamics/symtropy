# Symthaea Temporal Perception Witness Admission v0.1

Issue: [#7843 — Symthaea temporal observer witness admission](https://github.com/Luminous-Dynamics/symtropy/issues/7843)

Producer specification: [Symthaea #14063 / #14064](https://github.com/Luminous-Dynamics/symthaea/pull/14064)

Benchmark owner: [CFI-Space #7828 / #7829](https://github.com/Luminous-Dynamics/symtropy/pull/7829)

## Decision

Admit the Symthaea temporal observer as a new, typed evidence source only. It is not a pose estimator, confidence multiplier, covariance source, calibration replacement, sensor-independence proof or maneuver authority.

The producer's HDC patch novelty, CfC prediction residual and delayed-horizon skill remain distinct observation classes. They can expose temporal anomalies and justify conservative scrutiny, but low novelty does not validate pose correctness.

## Repository fact: do not confuse the current stub with upstream Symthaea

At the current public Symtropy `main`:
- `stubs/symthaea/Cargo.toml` explicitly describes a minimal, independently maintained, type-compatible stand-in.
- `stubs/symthaea/Cargo.toml` defines `vision-manifold = []`; the feature does not pull in the real upstream manifold.
- `stubs/symthaea/src/lib.rs` implements a trivial cognitive loop that returns zeroed output and telemetry.
- `crates/bridges/symthaea-bevy-brain` depends on that stub and contains `motor_output`. Its own README documents an NPC/game-engine integration. This is not an aerospace assurance boundary.

Therefore this contract must not be described as live Symthaea integration until a real upstream build/artifact is pinned and its producer invariants are verified.

## Evidence vocabulary

| Type | Meaning | Does not mean |
|---|---|---|
| `TEMPORAL_NOVELTY` | Patch representation differs from prior observations | Physical defect probability |
| `TEMPORAL_PREDICTION_RESIDUAL` | Frozen temporal model prediction differs from observation | Pose error, covariance or calibrated probability |
| `TEMPORAL_HORIZON_SKILL` | Delayed forecast error/skill with independent sample counts and baseline | Guaranteed future accuracy |
| `TEMPORAL_OBSERVER_STATUS` | Warm-up, input rejected, missing history, out of scope, internal error | A valid zero-signal observation |

Never combine these fields into a single score for pose admission.

## Admission envelope

Each input/witness must bind exact:
- schema version, producer crate/version, build/toolchain and binary digest;
- observer profile, model/checkpoint, HDC seed/dimension, encoder config and horizon schedule digests;
- episode, target, session, trajectory, frame and monotonic sequence IDs;
- raw frame byte digest and source evidence reference;
- sensor/modality, image dimensions/channels, ROI/crop digest and preprocessing revision;
- acquisition timestamp, time unit, clock profile and synchronization evidence;
- calibration / extrinsics / rig configuration digest;
- fixed episode budget and expected frame count.

The receiver must validate field lengths, enum values, digest syntax, numeric bounds, quantization and units before state mutation. Use cryptographic hashes on exact byte strings; never treat an HDC latent or similarity as a provenance digest.

## Mandatory receiver rejection rules

Reject or mark unavailable (without state mutation):
- unknown schema or unsupported producer build/profile;
- missing digest or mismatch with the actual received payload;
- duplicate, out-of-order, stale or missing frame identity;
- target/session/trajectory/calibration/clock/ROI mismatch;
- unexpected modality switch or incomplete temporal horizon;
- warm-up or insufficient history presented as steady-state telemetry;
- missing/oversized patch array, grid/shape mismatch, malformed fixed-point values;
- non-finite / out-of-range numeric values;
- producer flags not showing observer-only, learning frozen, raw-frame non-retention, no actuation authority and no probability claim;
- any profile/model/encoder change mid-episode.

No fallback to the last known telemetry. Missing witness means `TEMPORAL_EVIDENCE_UNAVAILABLE`, not zero anomaly.

## Decision projection

A minimal first policy projection:

| Condition | Projection |
|---|---|
| Invalid/missing witness | `TEMPORAL_EVIDENCE_UNAVAILABLE` |
| Warm-up / insufficient forecast count | `INSUFFICIENT_HISTORY` |
| Explicit observer scope/build mismatch | `OUT_OF_SCOPE` |
| Predeclared threshold exceeded | `TEMPORAL_ANOMALY_SUSPECTED` |
| Valid witness below threshold | `TEMPORAL_SIGNAL_WITHIN_DECLARED_ENVELOPE` |

Thresholds must be selected on development/calibration units and frozen before confirmatory evaluation. The receiver must never treat the last row as proof of pose validity, physical truth or sensor independence.

Permitted downstream effects:
- increase scrutiny and record a reasoned receipt;
- require another independent observation or operator review;
- suppress a candidate if existing policy declares temporal anomaly a blocker;
- suggest an evidence-gathering action, to be admitted by the existing sensing/action authority path.

Forbidden effects:
- alter pose, covariance, conformal region or sensor estimate;
- erase RGB/event `DEPENDENCE_UNKNOWN`, `DISAGREEMENT`, `OUT_OF_SCOPE` or abstention;
- assert camera/clock/label/source authenticity;
- authorize a maneuver, actuator command or irreversible mutation.

## Benchmark integration

Use CFI-Space episodes, counterfactual siblings and independent-unit grouping. Compare:
1. no temporal observer;
2. previous-frame persistence / robust pixel residual;
3. HDC patch novelty only;
4. frozen CfC prediction residual only;
5. HDC+CfC;
6. optional multi-scale predictive hierarchy / temporal binding;
7. complete evidence-policy projection.

For every arm report matched compute, preprocessing, target/session/trajectory split, fault-family denominator, recall at predeclared 0.1%, 1%, 5% false alarm rates, detection latency, sequence silent-miss, false-abstention and recovery. Report p50/p95/p99 latency, peak RAM and energy only when measured. The observer does not get credit for a detection if the same fault label leaked into its input or its model adapted on the holdout.

## Integration gates

### P0 — contract
- Schema, Rust types, canonical serialization, strict validator, rejection cases.
- No dependency on the local Symthaea stub for numerical telemetry.

### P1 — synthetic parity
- Positive / negative / malformed receipts.
- Independent challenger validates every field and exact byte digest.
- Replay determinism and mutation controls.
- Tests prove invalid input does not mutate the episode state.

### P2 — policy projection
- Typed evidence projection with no pose fusion.
- Counterfactual test where temporal anomaly can block/suppress but can never promote an unqualified pose.
- Missing evidence fails closed.

### P3 — CFI-Space evaluation
- Matched observer ablations and all baseline arms.
- Independent target/session/trajectory group splits.
- Results on public-real only after license and annotation audit; blind holdout requires evaluator custody.

### P4 — HIL
- Clock drift, skew, dropped frames, stale calibration, extrinsic perturbations and compute overload on exact hardware.
- This remains HIL evidence, not flight or physical qualification.

## Dependency and build options

Option A: pin a public, published Symthaea observer crate or artifact by immutable version and commit, review transitive dependencies and license, commit the lockfile and provenance, and prove reproducible builds.

Option B: run the observer as a separately isolated process and exchange the strict schema over a bounded, authenticated local IPC channel. This avoids linking a 1M+ LOC cognitive workspace into the Symtropy process, but demands process lifecycle, memory, CPU, timeout and payload custody controls.

No decision is made here to add a direct root dependency. A local `vision-manifold=[]` stub is not an acceptable stand-in.

## Claim ceiling

This specification and its eventual synthetic contract tests do not prove sensor independence, pose validity, physical truth, calibration coverage, source authenticity, flight readiness, HIL performance or SOTA superiority. The temporal witness remains supplementary evidence under existing Symtropy geometry, calibration, common-cause and action-admission owners.
