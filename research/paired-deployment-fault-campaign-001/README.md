# PAIRED-DEPLOYMENT-FAULT-CAMPAIGN-001

**Status:** typed paired-evidence contract and evaluator; no driving-policy result  
**Issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4681  
**Parent benchmark:** https://github.com/Luminous-Dynamics/symtropy/issues/4639  
**Renderer integration:** https://github.com/Luminous-Dynamics/symtropy/pull/4616  
**Episode replay integration:** https://github.com/Luminous-Dynamics/symtropy/pull/4647  
**Symthaea input bridge:** https://github.com/Luminous-Dynamics/symthaea/pull/11480

## Paradigm shift

A clean-route score is only one coordinate of a deployment capability envelope. A policy can score highly with fresh cameras and no action delay yet fail when sensor streams stall, localization drifts or controls arrive late.

This contract treats each fault result as a **paired experiment**:
- same model/checkpoint, code, initial scenario, route, map, environment revision, actor policies, random streams, action/observation schemas, evaluator, sensor permissions, hardware, precision, power profile and training exposure;
- one clean run and one perturbed run;
- the declared fault is the only permitted intervention;
- every episode, event, metric and recovery disposition is accounted for.

The validator refuses a pair if the clean and perturbed MatchedContext objects differ. It does not hide unmatched comparisons behind a normalized score.

## Fault families

The protocol incorporates the published Bench2Drive-Robust severity levels:

| Family | Severity | Required evidence |
|---|---|---|
| Camera burst drop | 20 or 60 consecutive ticks | one explicit observed_missing_frame event per missing tick; no frozen image relabelled as fresh |
| Partial camera occlusion | 50% or 80% occlusion | deterministic mask schedule, mode (fixed per episode or resampled per frame), event trace |
| GPS localization noise | 5m or 15m sigma | noise seed, onset/duration, measured perturbation event |
| Multiplicative speed-estimation noise | mean 0.5 or 0.2; standard deviation 0.2 | noise seed and sampled-value evidence |
| Inference/control delay | 100ms, 200ms or 500ms | FIFO delay declaration, action-delay events and control timestamps |

Sources: [Bench2Drive-Robust repository](https://github.com/Thinklab-SJTU/Bench2Drive-Robust) and [paper](https://arxiv.org/abs/2605.18059).

These settings define a reproducible protocol severity matrix. They are not estimates of real-world failure prevalence.

## Contract rules

1. Every pair_id has exactly one clean run and one perturbed run.
2. Both runs must have byte-for-byte equal matched context declarations, including checkpoint, seed manifests, actor policies, dataset split, input and teacher privileges, hardware, precision, power profile, resource budgets and runtime config.
3. The clean run must contain no fault spec and no fault events.
4. The perturbed run must have exactly one typed fault spec and matching fault-event evidence within its declared time window.
5. A 20/60-tick camera burst must preserve at least 20/60 distinct missing-frame tick records.
6. Each run's completed, failed, incomplete, censored and timed-out counts must sum exactly to its expected episode denominator.
7. Every metric is reported on its native scale and with its direction (higher_is_better or lower_is_better). Relative degradation is calculated per metric; no composite robustness scalar is generated.
8. Relative degradation is null with an explicit reason when the clean baseline is zero or either measurement is missing.
9. Recovery must be assessed against a fixed, versioned stability envelope. It may be recovered, not_recovered or not_applicable; changing the envelope after seeing the results is invalid experimental practice.
10. CIs, sample counts and episode dispositions remain explicit. Failed or incomplete runs are not silently removed.

## Executable evaluator

The standalone Rust 2024 crate is pinned to Rust 1.96.0 and uses Serde/Serde JSON.

From this directory:

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Run the CLI:

```sh
cargo run --release --locked -- campaign.json report.json
```

Exit codes:
- 0: structurally valid paired campaign;
- 1: structurally rejected campaign (report is still written);
- 2: invocation, input-size, file or typed-JSON error.

The validator checks pair denominator and unique IDs; exact matched context between clean and perturbed records; campaign/evaluator identity and sorted, duplicate-free privilege manifests; every fault family's supported severity values, schedule digest and event family/timing; clean-run fault contamination and perturbed-run missing fault evidence; complete episode disposition accounting; metric definitions, point estimates, confidence intervals and sample counts; recovery assessment consistency; and camera burst missing-frame event denominators.

The report includes metric-specific clean value, perturbed value and relative degradation. It does not weight or aggregate different fault families, benchmark scores, latency, energy and recovery into one headline score.

## Included fixtures

- examples/paired-camera-burst-valid.json: valid clean/perturbed pair, 20 explicit missing-frame events and a recovery assessment.
- examples/mismatched-checkpoint-negative-control.json: same campaign except for a changed perturbed checkpoint; the validator must reject it.
- schemas/fault-campaign-v1.schema.json: machine-readable Draft 2020-12 input schema.

## Next experiments

- Reproduce Bench2Drive v0.0.4 clean baseline on a pinned evaluator and route manifest.
- Run the exact paired perturbation matrix, preserving each severity independently.
- Add fresh-process deterministic fault-schedule replay and event-ledger checks.
- Compare compact GRU/LSTM, compact transformer and HDC-LTC/CfC with the same learned visual features and matched privileges.
- Measure full-pipeline energy, p99 latency, control age, deadline misses, fallback and time-to-recovery under the same hardware rig.
- Only then claim a robustness/efficiency frontier improvement. A passing record validator is not a driving result.

## Claim ceiling

A valid report establishes internal consistency of finite paired records and the arithmetic of declared metrics. It does not prove that a benchmark fault distribution represents real roads, authenticate the sensors, show the system safely responds to faults, establish sim-to-real transfer, provide a safety case, certify a vehicle or imply road readiness.
