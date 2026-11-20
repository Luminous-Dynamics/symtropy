# Task-Constrained Active Diagnosability Reference v0.1

**Status:** M1 deterministic feature-interval separability and hard-margin filter with synthetic fixtures. This is not an active trajectory optimizer, physical safety proof, fault detector or hardware qualification.

**Research owner:** [TASK-CONSTRAINED-ACTIVE-DIAGNOSABILITY-001 #6327](https://github.com/Luminous-Dynamics/symtropy/issues/6327)  
**Fault-observability event metrics:** [#6316](https://github.com/Luminous-Dynamics/symtropy/issues/6316) / [PR #6325](https://github.com/Luminous-Dynamics/symtropy/pull/6325)  
**Synchronized actuator metrology:** [#6266](https://github.com/Luminous-Dynamics/symtropy/issues/6266) / [PR #6272](https://github.com/Luminous-Dynamics/symtropy/pull/6272)  
**Mission-conditioned joint capability:** [#6230](https://github.com/Luminous-Dynamics/symtropy/issues/6230) / [PR #6231](https://github.com/Luminous-Dynamics/symtropy/pull/6231)  
**Discrete passivity:** [#6200](https://github.com/Luminous-Dynamics/symtropy/issues/6200)

## 1. Research baseline: active diagnosis exists already

This programme does **not** claim novelty for actively exciting a system to reveal faults. The relevant research already includes input design, diagnosis geometry and co-optimization with control:

- Sheikhi et al., *Fault Diagnosis in Dynamical Systems: Geometric Interpretation and Tractable Algorithms*, Annual Review of Control, Robotics, and Autonomous Systems 9:147–179 (2026), DOI https://doi.org/10.1146/annurev-control-030123-015422. This review covers passive/active detection and estimation, behavioural-set geometry, robust residual filters and optimization-based input design.
- *Active Fault Diagnosis for Stochastic Systems With Incipient Actuator and Sensor Faults*, International Journal of Robust and Nonlinear Control 36(11), 5702–5715 (first published 7 April 2026), DOI https://doi.org/10.1002/rnc.70544. The method designs inputs for incipient actuator, sensor and mixed faults using a Bayesian minimum-error/Chernoff-distance objective under stochastic assumptions.
- *Active Fault Diagnosis for Stochastic Systems: A Unified Design With Optimal Control*, IEEE Transactions on Industrial Informatics 21(9), 6626–6637 (September 2025), DOI https://doi.org/10.1109/TII.2025.3563541. It formulates active diagnosis and task-control input design together under finite-horizon constraints and demonstrates on a four-tank system.
- *A three-level hierarchical fault diagnosis framework for APS mechatronic systems with adaptive Bayesian root-cause analysis*, Results in Engineering 30, 111163 (June 2026), DOI https://doi.org/10.1016/j.rineng.2026.111163. It separates fault type from subsystem location, uses a validated digital model for labelled data, and reports a UR5e case study under unseen fault and trajectory shifts.
- Khan, Nasir and Mekid, *Fault-tolerant control strategies for industrial robots: state of the art and future perspective on AI-based fault management*, Artificial Intelligence Review 58, 362 (30 August 2025), DOI https://doi.org/10.1007/s10462-025-11327-2. It surveys analytical/physical redundancy, residual generation, isolation and reconfiguration.

The opportunity for Symtropy is therefore a **robot-joint qualification layer**: diagnostic separability must carry measurement provenance and be evaluated only for excitation profiles with independently supported task, torque/speed, peak-duration, thermal, energy, passivity, workspace and exposure margins.

## 2. M1 question and mathematical meaning

Each fault hypothesis (including healthy/nominal) has a set of feature intervals. Features may represent calibrated torque residuals, encoder/servo residuals, vibration-band quantities, current signatures, backlash indicators, stiffness estimates or other predeclared diagnostics.

For one feature dimension, two interval signatures are separated only when they do not overlap:

`gap(A,B) = max(0, lower_B - upper_A, lower_A - upper_B)`

The dimensionless gap is:

`normalized_gap = gap / preregistered_feature_scale`

For an axis-aligned feature box, a single valid feature dimension with positive gap is sufficient to prove the two boxes disjoint. If every available validated feature interval overlaps, that pair is NotSeparable. If no feature is available or available evidence is unqualified, the result is Unknown.

This is a deterministic sufficient test for disjoint interval boxes. It does **not** model feature correlation, temporal dependencies, nonrectangular behavioural sets or probability of misclassification. M2 must compare against geometric residual-set, likelihood-ratio, Chernoff and stochastic active-input methods.

## 3. Sensor availability is part of the qualification

A profile contains a frozen list of required sensor-availability scenarios, for example:
- nominal vibration + servo + torque-metrology channels;
- vibration dropout;
- servo dropout;
- metrology dropout;
- compound vibration + servo dropout.

For each scenario, every pair of fault hypotheses is re-evaluated using only the features whose sensor channels remain available. The final profile is:
- **FeasibleFullyDiagnosable:** all hard margins are known/nonnegative and every hypothesis pair is separable in every required scenario;
- **FeasibleButNotFullyDiagnosable:** hard margins pass and at least one pair/scenario is demonstrably not separable;
- **Unknown:** a required margin, feature/scenario evidence or pair/scenario evaluation is unknown;
- **Infeasible:** at least one known, evidence-qualified hard margin is negative.

A profile may be diagnosable with the full sensor set but not after a sensor combination fails. The result must retain that weakness rather than averaging it away.

## 4. Hard constraints: diagnostic information cannot buy an unsafe excitation

The profile consumes externally supplied conservative lower-bound margins for eight independent constraints:
1. task-performance tolerance;
2. torque/speed envelope;
3. peak-duration capability;
4. thermal headroom;
5. energy budget;
6. passivity;
7. workspace/contact/pose;
8. exposure-time budget.

A known negative margin makes the profile Infeasible. Missing value or Unknown provenance makes it Unknown. A large signature gap cannot compensate for a failed hard constraint.

This M1 crate only checks the supplied margins and their evidence/digest binding. It does not calculate the physical margins, create excitation waveforms or establish a safe-to-execute condition. The values must come from independently validated models/measurements in a later qualification phase.

## 5. Evidence and configuration binding

Every feature signature and constraint margin carries:
- source/evidence class, method and raw/calibration reference;
- sensor channel, modality, unit and interval/scale definition;
- configuration, system-boundary, operating-condition, mission-set and profile digests.

Every sensor-availability scenario carries the same profile/configuration/boundary/condition/mission identities and its own source/method. A stale feature, calibration, system boundary or scenario is rejected. Unknown evidence cannot establish separation.

Vendor or model-derived signatures remain in their own evidence classes; they are not promoted to independent physical evidence by the geometry calculation.

## 6. Synthetic known-answer example

`examples/active-diagnosability-known-answer.json` contains:
- three synthetic hypotheses: healthy, gearbox backlash, bearing wear;
- three synthetic feature modalities: vibration, motor-servo and torque-metrology residuals;
- five sensor scenarios;
- eight positive synthetic hard-constraint margins;
- a two-second, 12-joule placeholder diagnostic profile.

The nominal sensor set separates every pair. Vibration-only, servo-only and metrology-only dropout each have specific retained feature signatures; the combined vibration+servo dropout leaves only torque-metrology, where healthy and bearing-wear intervals overlap. The expected profile classification is therefore **FeasibleButNotFullyDiagnosable**, not full diagnosability.

All IDs, intervals, energy and hard margins are invented software fixtures. They do not come from GD-HDD, a vendor or a physical robot test.

## 7. Current evaluator outputs

For every required profile/scenario/hypothesis-pair combination the evaluator reports:
- Separable / NotSeparable / Unknown;
- available separating feature IDs;
- maximum normalized gap among separating features;
- reason for Unknown/NotSeparable.

At profile level it reports:
- disposition and known hard-margin violations / unknown constraints;
- hypothesis-pair and sensor-scenario counts;
- separable / nonseparable / unknown pair-scenario counts;
- pair-scenario separation coverage;
- worst-case normalized separation (zero if a known pair/scenario is not separable; null if unknown prevents a bound);
- diagnostic duration and electrical energy as separate Pareto-ready axes.

It does not collapse diagnosability, task deviation, energy, thermal margin, passivity or evidence quality into one weighted score.

## 8. Run

Dependency-free Rust crate, Rust 1.96:

```bash
cargo fmt --manifest-path benchmarks/synthetic-joints/active-diagnosability/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/active-diagnosability/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/active-diagnosability/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

Dedicated workflow: `.github/workflows/task-constrained-active-diagnosability.yml`. The authoring environment has no local Cargo/rustc/rustfmt; exact-head CI is the compile/format/Clippy gate.

Twenty Rust test definitions cover nominal/dropout separability, nonseparable and unknown scenarios, hard-margin failure and missingness, provenance requirements, box-disjointness logic, feature normalization, healthy hypothesis/taxonomy, duplicate IDs/constraints, digest mismatch, units/modalities and malformed intervals.

## 9. M0–M5 promotion

- **M0:** freeze fault taxonomy, feature definitions, sensor inventory, operating domain, dropout masks, task tolerances and diagnostic-profile constraints.
- **M1:** interval-box separation, required sensor scenarios, evidence binding and hard-margin filter with synthetic fixtures.
- **M2:** generate fault signature distributions from validated APS models; compare geometric/behavioural-set, likelihood, Chernoff and stochastic active-input baselines; quantify correlation and model uncertainty.
- **M3:** guarded and independently instrumented joint experiments with calibrated torque, vibration, encoder, current, voltage and temperature. Verify fault signatures and margins.
- **M4:** matched whole-robot task tests with diagnostic excitation, task deviation, energy, TTD/FAR from #6316 and sensor-dropout scenarios.
- **M5:** remeasure after wear, lubrication/service, remounting, sensor calibration drift and component replacement; revoke stale signature validity.

## 10. Prospective benchmarks

Targets, not achieved Symtropy results:
- every required hypothesis pair is labelled Separable / NotSeparable / Unknown under every preregistered availability scenario;
- zero feasible profile with any known negative hard margin;
- zero Unknown sensor/calibration evidence promoted to separation;
- nominal full-sensor coverage and each declared single/compound dropout reported separately;
- held-out physical signatures must fall inside preregistered signature intervals before a measured diagnosability claim;
- compare against active/stochastic input-design baselines at matched task-deviation, energy, latency and sensor budgets;
- report diagnosis margin and task/energy/thermal/passivity margins as separate Pareto axes.

## 11. Claim ceiling and safety

This package is M1 deterministic signature-interval checking only. It does not synthesize or execute an excitation, validate its margins, guarantee real fault separation, certify a safe state or authorize physical testing. Any active diagnostic excitation requires a separate hazard analysis, guarded/rated equipment, independent shutdown, calibrated sensors, competent operators and preregistered amplitude/energy/time stop limits.
