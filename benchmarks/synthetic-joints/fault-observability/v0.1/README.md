# Actuator Fault Observability and Sensor-Integrity Benchmark v0.1

**Status:** M1 event-level scoring reference with synthetic fixtures. The crate evaluates supplied ground truth and deduplicated alarm episodes; it does not implement a fault detector or demonstrate physical diagnostic performance.

**Research owner:** [ACTUATOR-FAULT-OBSERVABILITY-001 #6316](https://github.com/Luminous-Dynamics/symtropy/issues/6316)  
**Synchronized actuator metrology:** [#6266](https://github.com/Luminous-Dynamics/symtropy/issues/6266) / [PR #6272](https://github.com/Luminous-Dynamics/symtropy/pull/6272)  
**Grouped actuator model calibration:** [#6262](https://github.com/Luminous-Dynamics/symtropy/issues/6262) / [PR #6263](https://github.com/Luminous-Dynamics/symtropy/pull/6263)  
**Mission-conditioned joint capability:** [#6230](https://github.com/Luminous-Dynamics/symtropy/issues/6230) / [PR #6231](https://github.com/Luminous-Dynamics/symtropy/pull/6231)

## 1. Why this benchmark is needed

Many fault-diagnosis papers report frame/window accuracy or F1 without the operational quantities needed to judge whether a detector is useful on a robot joint:
- **TTD:** time from a verified fault onset to the first valid alarm;
- **FAR:** false alarm episodes per hour of verified no-fault exposure;
- **MDFA:** the smallest tested fault amplitude that meets a preregistered detection-sensitivity requirement with uncertainty;
- **compound-fault isolation:** whether the alarm identifies the complete simultaneous fault set;
- **abstention:** whether unsupported, corrupted or out-of-domain inputs cause the detector to say “unknown” rather than confidently invent a diagnosis.

Maican et al., *Fault Detection and Diagnosis for Human-Centric Robotic Actuation in Healthcare: Methods, Failure Modes, and a Validation Framework*, Actuators 14(12), 566 (21 November 2025), DOI https://doi.org/10.3390/act14120566, reviews 127 studies and reports that accuracy/F1 are commonly reported while MDFA, TTD and FAR are rarely documented. Its fault taxonomy is directly relevant to robot joints: harmonic-drive backlash/wear/lubricant ageing; SEA compliance changes and torque-estimation drift; cable-drive slack and friction variation; and motor thermal/current signatures.

## 2. Public datasets and SOTA comparators

### GD-HDD: synchronized harmonic-drive fault data

Maintainer repository: https://github.com/GDUT-YANGGUO/Harmonic-Drive-Dataset

The project describes synchronized vibration, servo-motor and high-precision metrology channels across 12 operating conditions, seven common fault categories, three dual-compound fault cases and one triple-compound case. This is a valuable public starting point for multi-channel transmission diagnosis. Before any measured benchmark is claimed, the exact repository revision, dataset licence, acquisition timing, channel metadata, specimen/machine grouping and ground-truth definitions must be audited. The synthetic fixture in this PR does **not** contain GD-HDD measurements.

### Cross-machine diagnosis and sensor self-healing

Yao et al., *Industrial robot transmission components cross-machines fault diagnosis via fault intrinsic representation and channel self-healing under sensor failure*, Scientific Reports 16, 15921 (3 April 2026), DOI https://doi.org/10.1038/s41598-026-47066-9.

The method combines a fault-intrinsic representation with semantic channel self-healing and evaluates cross-machine transfer, including complete single-channel failure on public gearbox/bearing proxy datasets. Its conclusion identifies drift, intermittent failure, partial corruption and real industrial robot-joint validation as further work. This is a stronger baseline than clean multi-channel classification, but its proxy-dataset transfer must not be represented as a deployed robot-joint safety result.

### Multimodal sensor integrity and reporting

Pană et al., *From Signals to Remaining Useful Life: Multimodal Sensor Fusion for Fault Diagnosis and Prognostics—Methods, Pitfalls, and Reporting Standards*, Sensors 26(12), 3661 (8 June 2026), DOI https://doi.org/10.3390/s26123661.

The review discusses bias/drift, miscalibration, dropout, saturation, cross-talk, time desynchronization and domain shift. It calls for stronger reporting of synchronization, fault ground truth, missingness, leakage controls and uncertainty calibration. This benchmark treats channel integrity and timing as part of the diagnostic problem rather than preprocessing details that can be silently assumed away.

## 3. Event-level ground truth and alarm semantics

A fault episode is defined by:
- independent specimen ID, fault class and amplitude with unit;
- onset time and either a verified clear time or an observation-end time;
- source/evidence class, ground-truth reference, raw-trace digest and configuration/boundary/condition/mission digests.

A compound event is represented by multiple overlapping episodes of different fault classes on the same specimen. Same-class overlapping episodes are rejected because the event boundary would be ambiguous in this M1 contract.

An alarm is a deduplicated event with a first timestamp and a **set** of hypothesized fault classes. Repeated alarms from the same specimen at the same instant must be merged upstream. A single label on a two-fault event is a mis-isolation, not an exact compound diagnosis.

Every alarm is assigned exactly one disposition:
- **ExactIsolation:** hypothesized fault set equals the active ground-truth set;
- **MisIsolation:** a fault is active, but the alarm's set is incomplete or incorrect;
- **FalseAlarm:** no fault is active and the alarm lies in explicitly verified no-fault exposure;
- **UnscoredOutsideExposure:** no fault is active and no verified no-fault exposure covers the timestamp.

An alarm before onset cannot detect a later event. An alarm at or after the exclusive clear time cannot detect the episode. An incorrect label during an active fault is a mis-isolation; it is not counted as a no-fault false alarm.

## 4. Detection, censoring and time-to-detect

For a cleared fault episode, a matching alarm must occur in the half-open interval:

`[onset_s, clear_s)`

Then:

`TTD = first_matching_alarm_time - fault_onset_time`

If no matching alarm occurs before a verified clear time, the episode is a **Missed** event.

If observation ends while the fault is still active, the episode is **RightCensored**. It is not treated as a confirmed miss, and it is not counted as a detection. The report shows:
- detected episode count;
- cleared-but-missed episode count;
- right-censored episode count;
- evaluable detection denominator = detected + cleared-but-missed;
- event detection fraction over that evaluable denominator;
- median, nearest-rank p95 and maximum TTD among detected episodes.

The TTD percentiles are conditional on detection; they must always be read alongside misses and censoring. They are not an estimate of time-to-detection for all fault episodes.

## 5. False-alarm rate: the denominator is verified exposure

The benchmark computes:

`FAR = false_alarm_episode_count / verified_no_fault_exposure_hours`

A no-fault window must carry specimen identity, source, raw-trace digest, evidence class, configuration and condition/mission/boundary digests. It cannot overlap a fault episode or another no-fault window on the same specimen. Exposure hours are summed only after those checks.

No verified no-fault exposure means FAR is **Unknown**, not zero. An alarm outside both known fault and known no-fault exposure is unscored, not silently reclassified as true or false. A frame-level false-positive percentage is not a substitute for FAR/hour.

## 6. MDFA: specimen-grouped and uncertainty-qualified

For each fault class and tested amplitude:
- each independent specimen contributes at most one trial;
- repeated windows from one specimen are not treated as extra independent units;
- the report includes total independent specimens, evaluable specimens (detected or confirmed missed), right-censored specimens, detected count, empirical detection fraction and two-sided 95% Wilson interval. Censored specimens are excluded from the Wilson denominator and remain separately counted;
- amplitude units must be consistent within a fault class.

MDFA is the **smallest tested amplitude** for which:
1. the Wilson 95% lower bound for detection probability meets the preregistered sensitivity threshold at that amplitude and every higher tested amplitude (a monotone-tail rule);
2. every included amplitude level has at least the preregistered minimum independent specimen count;
3. no amplitude level has right-censored follow-up;
4. verified no-fault exposure exists and the campaign FAR is within its preregistered budget;
5. at least two amplitude levels were tested.

Otherwise MDFA is explicitly not qualified, with a reason such as no no-fault exposure, excessive FAR, insufficient amplitude levels, insufficient specimen count or insufficient follow-up. The M1 reference defaults its synthetic unit test plan to 16 independent specimens per level and a Wilson lower bound of 0.8; these are illustrative campaign settings, not universal requirements. Physical studies must preregister justified values and sufficient power before data are collected.

## 7. Sensor integrity and cross-machine challenge set

The M1 arithmetic evaluator consumes externally labelled alarms; it does not transform raw sensor waveforms. The full benchmark campaign should challenge detectors with:
- complete channel loss and intermittent dropout;
- partial corruption, clipping/saturation and stuck-at values;
- bias/scale drift, calibration expiry and sensor remounting;
- timestamp offset, clock drift, jitter and filter/group delay;
- cross-talk and inconsistent vibration/servo/metrology channels;
- changed speed, load, temperature, mounting and cooling;
- leave-one-machine, leave-one-model and where possible leave-one-architecture-out validation;
- single, dual and triple compound faults.

Detectors should be compared at matched input channels, label budget, compute/latency budget and held-out specimen/machine splits. A self-healing method must report whether it reconstructed a signal, abstained, or changed diagnosis; reconstructed channels must not be treated as independent physical truth.

## 8. Required reporting and comparison

Report per fault class and condition:
- event detection rate with independent specimen denominator and Wilson interval;
- TTD median/p95/max, plus missed and right-censored counts;
- tested amplitude grid, MDFA disposition and confidence threshold;
- false alarm episodes and verified no-fault exposure hours, plus FAR/hour;
- exact compound-fault isolation and mis-isolation counts;
- OOD, missing-channel and abstention rates;
- cross-machine transfer gap and selective risk;
- detector digest, channel-set digest, configuration/condition/mission/boundary identities;
- raw trace, source, ground-truth and method provenance.

Baseline families should include residual/parity observers, thresholds/change detection, conventional supervised temporal/CNN models, domain-invariant representations, cross-machine fault-intrinsic/channel-self-healing models, and uncertainty-aware multimodal fusion. Accuracy/F1 may be reported, but they must not replace event metrics.

## 9. Synthetic known-answer fixture and tests

`examples/fault-observability-known-answer.json` includes five synthetic fault episodes, four synthetic alarms and three verified synthetic no-fault windows. It demonstrates a one-false-alarm/five-hour = 0.2/hour denominator, an overlapping compound fault with exact-set alarm, an alarm outside scored exposure and a right-censored fault. Values and IDs are placeholders, not real actuator data.

Twenty-eight uniquely numbered Rust test definitions cover:
1. TTD and FAR arithmetic;
2. specimen-grouped Wilson MDFA;
3. exact compound-fault set isolation and partial mis-isolation;
4. pre-onset and post-clear alarm treatment;
5. wrong-label event treatment;
6. absent no-fault exposure and FAR abstention;
7. FAR budget suppressing MDFA;
8. overlap and duplicate-record rejection;
9. out-of-exposure alarms;
10. same-class versus compound-event overlap;
11. evidence/digest consistency;
12. amplitude-unit consistency;
13. minimum independent specimen count;
14. right-censoring separated from misses and MDFA;
15. no-alarm behaviour and Wilson reporting.

The unit tests synthesize 16 independent specimen trials per amplitude level to validate the MDFA arithmetic; the small JSON example is intentionally underpowered and is not used to claim an MDFA result.

## 10. Run

Dependency-free Rust crate, Rust 1.96:

`\`\`bash
cargo fmt --manifest-path benchmarks/synthetic-joints/fault-observability/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/fault-observability/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/fault-observability/v0.1/Cargo.toml --all-targets --locked -- -D warnings
`\`\`

Dedicated workflow: `.github/workflows/actuator-fault-observability.yml`. The authoring environment has no local Cargo/rustc/rustfmt; exact-head CI is the compile/format/Clippy gate.

## 11. M0–M5 promotion

- **M0:** audit GD-HDD version/licence, channels, time alignment, fault labels, specimens, machines and source provenance.
- **M1:** synthetic event metrics, compound isolation, censoring, FAR/MDFA arithmetic and adversarial data-contract controls.
- **M2:** reproduce public baselines on frozen grouped splits; leave-one-machine/model/condition-out tests and sensor-failure/corruption injection in replay.
- **M3:** independently instrumented, guarded actuator rig with calibrated torque, vibration, encoder, current, voltage and temperature channels; independently verified fault ground truth.
- **M4:** hardware-in-loop and integrated robot task diagnostics with detection latency, exact isolation and abstention; diagnostics are not safety actions.
- **M5:** wear, service, remounting, calibration drift and post-service requalification.

## 12. Claim ceiling and safety

M1 validates event-metric arithmetic against synthetic records only. It does not establish detector performance, reproduce GD-HDD, prove generalization, or certify a robot safety function. Physical fault/sensor injection requires a separate hazard analysis, competent operators, guarded/rated fixtures, independent safety shutdown, traceable instrumentation and preregistered stop limits. No human-contact test or safety-critical control intervention is authorized by this benchmark.
