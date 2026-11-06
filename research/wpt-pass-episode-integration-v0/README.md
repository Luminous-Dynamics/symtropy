# WPT verified pass evidence integration v0

**Status:** research specification / synthetic-only implementation target. Not a measured test report, safety assessment, standards certificate or authorization to energize hardware.

Tracking issue: [#3950](https://github.com/Luminous-Dynamics/symtropy/issues/3950)

## 1. Purpose

Compose existing WPT evidence products into one fail-closed, independently replayable vehicle-pass record. The primary object is one observed vehicle–infrastructure interaction, not an isolated coil, pavement coupon, energy ratio or controller output.

The integration evaluator proves that trajectory, EM projection, pavement observations, energy ledger, asset snapshot and controller score all refer to the same pass and exact source artifacts. It does not reimplement their domain calculations.

## 2. Reuse rather than duplicate

| Evidence role | Existing owner | Integration responsibility |
|---|---|---|
| Canonical trajectory and EM/pavement projections | [#3887](https://github.com/Luminous-Dynamics/symtropy/pull/3887) | Identify the authoritative trajectory artifact |
| Raw bytes, SHA-256 and independent transform/clock replay | [#3923](https://github.com/Luminous-Dynamics/symtropy/pull/3923) | Invoke verifier and bind its receipt |
| Alignment, wheel paths, sampled DC power and temperature | [#3908](https://github.com/Luminous-Dynamics/symtropy/pull/3908) | Bind report to the same trajectory digest |
| Segment/route energy | [#3895](https://github.com/Luminous-Dynamics/symtropy/pull/3895) | Verify contiguous pose windows and meter-plane identity |
| Exact setpoint oracle | [#3928](https://github.com/Luminous-Dynamics/symtropy/pull/3928) | Bind schedule to trajectory and envelope digests |
| Asset health | [#3726](https://github.com/Luminous-Dynamics/symtropy/issues/3726) | Bind asset identity, time and model receipt |
| Cross-lab metrology | [#3477](https://github.com/Luminous-Dynamics/symtropy/issues/3477) | Reuse calibration and acquisition evidence |
| Predictive uncertainty | [#3659](https://github.com/Luminous-Dynamics/symtropy/issues/3659), [#3692](https://github.com/Luminous-Dynamics/symtropy/issues/3692) | Bind support and calibration disposition |

A versioned adapter is required when interfaces differ. Never silently coerce schema versions, units, meter planes or evidence classes.

## 3. Core design

### 3.1 One pass as a content-addressed evidence graph

The top-level manifest contains identity and references. Large raw artifacts remain with their owning packages or storage. Every reference records role, schema version, media type, byte length, SHA-256, evidence class and provenance receipt.

Minimum identity:
- episode/pass ID and trajectory ID + exact digest;
- vehicle ID/configuration digest;
- roadway/site/lane/direction and route window;
- receiver, coil, segment and asset IDs + geometry/hardware/firmware digests;
- coordinate-frame and transform references;
- route-run and controller-run IDs;
- manifest and evaluator versions.

Required roles:
1. canonical trajectory;
2. EM projection and transform;
3. pavement projection and transform;
4. independent trajectory-verification receipt;
5. trajectory-coupling evaluation receipt;
6. segment/route energy ledger and evaluation receipt;
7. asset-state snapshot and model receipt;
8. controller envelope, schedule and score receipt;
9. calibration/acquisition/custody references for measured evidence.

Do not duplicate raw sample arrays in the manifest. Referenced bytes are authoritative.

### 3.2 Join invariants

- Exactly one canonical trajectory is authoritative per pass.
- Every dependent artifact binds both trajectory ID and exact digest.
- Projection records bind transform digest, coordinate frame and sample identity.
- Vehicle/receiver/coil/segment/asset/configuration revisions agree.
- Route segment pose windows are ordered and contiguous; endpoint times match exactly; idle intervals remain explicit.
- Controller reports bind exact trajectory and envelope digests, with support state for every interval.
- Different sensor rates require explicit versioned alignment operators, age/gap limits and uncertainty propagation. No implicit sorting, interpolation or extrapolation. v0 uses exact synthetic correspondences.
- Raw time, correction and uncertainty remain separately represented.
- Energy boundaries remain distinct: SPC→EVPC, roadway inverter DC→vehicle DC bus, grid AC→battery terminal and BESS DC output.
- Evidence classes remain role-specific. Composition cannot promote synthetic evidence to measured evidence.
- Missing, stale, conflicting or out-of-support evidence yields typed failure codes.

### 3.3 Dispositions

v0 may emit only PASS_SYNTHETIC for the known-truth fixture. Failure codes include INCOMPLETE, IDENTITY_CONFLICT, DIGEST_MISMATCH, FRAME_MISMATCH, TIME_ALIGNMENT_UNRESOLVED, POSE_WINDOW_INVALID, METER_BOUNDARY_MISMATCH, ASSET_REVISION_MISMATCH, ENVELOPE_MISMATCH, OUT_OF_DOMAIN, PROVENANCE_UNVERIFIED and INVALID.

No scalar score may override a hard gate.

## 4. v0 corpus

Compose the existing synthetic fixtures into one pass: trajectory, EM/pavement projections and transforms, two route segments, explicit idle interval, asset snapshot and exact-oracle control result.

Positive controls:
- independently recomputed artifact hashes;
- hand-calculated transform and pose membership;
- exact segment windows and meter-plane energy totals;
- controller report bound to exact envelope and trajectory digests;
- deterministic report for identical inputs.

Negative controls:
- changed bytes or digest; same ID with different bytes;
- wrong transform/frame; duplicate/missing/reordered/orphan pose;
- clock residual or sample gap outside support;
- segment skip/reversal/overlap; route gap or missing idle meter;
- swapped meter plane or partial BESS coverage;
- mismatched vehicle/coil/asset/firmware; stale asset snapshot;
- controller using another envelope; unsupported interval scored feasible;
- synthetic evidence relabelled measured; missing measured-role calibration;
- unknown fields, invalid units, non-finite values, oversized artifacts and resource exhaustion.

## 5. Benchmarks

Keep metric families independent:

| Family | Report |
|---|---|
| Identity / integrity | exact digests, provenance, missing/conflicting joins |
| Geometry / time | transform replay, clock residual, pose coverage |
| EM / circuit | matched-condition coupling/impedance/power residuals and support |
| Pavement / thermal | temperature, strain/deflection, heat flux and material/bond strata |
| Energy | plane-specific Wh, ratio of sums, residual and uncertainty |
| Service | useful battery Wh/pass and Wh/km, demand coverage, availability, handover |
| Control | shortfall, tracking/overshoot, hard-constraint margin, oracle gap, runtime |
| Asset | drift, failures, maintenance, availability and calibrated RUL |
| Calibration | 90/95% coverage, sharpness/proper score, regime metrics and cluster uncertainty |

### Industry lanes (not one leaderboard)

- Fraunhofer IISB 2026 robotic IPT bench: seven axes, 13.5 m track, up to 2.4 m/s; controlled motion/misalignment.
- Purdue–INDOT 2026: 85 embedded coils, Class-8 truck, 50–190 kW up to 65 mph, approximately 85% at roadway-inverter DC to vehicle-DC boundary; not grid-to-battery.
- Japan 2026 public road: approximately 10 kW, WPT replenishment equivalent to 63.3% of driving energy and 18-month coil-parameter stability; service/stability, not link efficiency.
- Honda October 2026: future one-million-wheel-load campaign at 49 kN per wheel, up to 150 kW and later road demonstration; plans, not achieved qualification.
- 2026 asphalt study: binder-dependent thermomechanical response near embedded pads; stratify by material and interface condition.
- Rahman et al. 2026: direct MPC method comparator. Reproduce equations, horizon, sample time, weights, actuator limits and solver budget before matched comparison.

Standards lanes: [IEC PAS 61980-5:2024](https://webstore.iec.ch/en/publication/68093) is off-board dynamic MF-WPT equipment; [ISO/PAS 5474-6:2025](https://www.iso.org/standard/84305.html) is heavy-duty vehicle-side MF-WPT. SAE J2954 is principally light-duty.

## 6. Beyond current SOTA

1. Preserve complete matrix reciprocity/passivity and frequency-domain positive-realness. Adaptively spend FEM solves where residual, eigenvalue margin or decision sensitivity is highest; reuse #3530/#3546/#3560.
2. Couple high-fidelity FEM/circuit/thermal/mechanical models to reduced-order or residual models only with calibrated residual bounds and explicit support domains.
3. Test traffic/environment → pavement/interface → coil geometry/material/electrical drift → qualified service envelope using matched controls and longitudinal evidence.
4. Hold out complete trajectories, vehicles, coil pairs, road sections, hardware revisions and material lots. Random row splits leak.
5. Report interval coverage and width/proper score by regime; quantify coverage uncertainty at trajectory/site/hardware cluster level.
6. Compare useful energy, unmet demand, fairness, hard-limit failures, asset margin, handover and runtime. Safety remains non-compensatory.
7. Use #3477 for a blind three-lab round robin and independent review.

## 7. Proposed acceptance targets (not achieved)

- 100% expected disposition on the frozen known-truth/adversarial corpus.
- Zero invalid byte/pass/frame/clock/window bundles admitted.
- 100% exact reproduction of hand-computed synthetic transforms and route windows.
- Full route interval coverage including idle and applicable BESS accounting.
- No ratio or optimality score when metrology, energy closure, envelope identity or support is unresolved.
- Deterministic report bytes for identical frozen inputs and evaluator version.
- Hard evidence failures cannot be offset by efficiency or service scores.

For predictive models, #3659's proposed 90% coverage band (88–92%) and 95% band (93–97%) are programme targets only; publish interval width, proper scores and regime-specific results. Use #3692 for dependence-aware uncertainty at trajectory/site/hardware cluster level.

## 8. Execution order

- P0: repair #3908/#3895 package-test failures and #3923/#3928 formatting without weakening gates. Exact failing assertions remain to be diagnosed from job logs.
- P1: implement bounded manifest schema and Rust evaluator.
- P2: compose existing verifier/evaluator receipts and adversarial tests.
- P3: bind asset-state, metrology and calibrated-uncertainty receipts.
- P4: publish independently replayable synthetic benchmark report.
- P5: admit lab/field evidence only with calibrated instrumentation, acquisition provenance, held-out regimes and independent review.

## 9. Safety / claim ceiling

Synthetic fixtures and offline replay only. No high-power construction, energization, roadway installation, human-exposure testing or field control is authorized. A synthetic pass certificate proves only defined software joins and byte integrity—not sensor authenticity, physical truth, safety, lifetime, standards compliance or SOTA superiority.

## References

- [Purdue–INDOT 2026 final report](https://doi.org/10.5703/1288284318633)
- [Japan public-road WPT experiment (2026)](https://doi.org/10.1541/ieejias.146.194)
- [Honda DC-distributed DWPT announcement, 5 October 2026](https://global.honda/en/topics/2026/c_2026-10-05beng.html)
- [Asphalt thermal-mechanical response, Applied Thermal Engineering (2026)](https://doi.org/10.1016/j.applthermaleng.2026.132099)
- [Rahman et al., MPC misalignment compensation (2026)](https://doi.org/10.3390/en19112640)
- [FMI 3.0 standard](https://fmi-standard.org/)


## 10. Machine-readable starting artifacts

- [manifest.schema.json](./manifest.schema.json) defines the closed v0 manifest shape, artifact roles, pass identity, route windows, projection joins, asset/controller bindings and bounded policy fields.
- [manifest.NOT_RUN.template.json](./manifest.NOT_RUN.template.json) is deliberately an empty NOT_RUN template. It contains no fabricated IDs, artifact hashes, sensor data or pass result. It is not a benchmark fixture and cannot qualify a pass.

### Fixture compatibility finding

The current component fixtures cannot simply be concatenated and called one integrated pass:
- #3887's canonical replay fixture is a three-pose synthetic-wander episode.
- #3908's coupling fixture is a different three-sample synthetic-pass episode.
- #3923's artifact verifier fixture is a two-pose known-truth trajectory.
- #3928's control fixture is a four-interval trajectory-control episode.
- #3895 has its own route ledger and pose-window fixture.

Their identities, sample counts and schemas are not presently one shared episode. P1/P2 must create one canonical master synthetic pass and versioned adapters or regenerated domain fixtures that preserve the same pose IDs, timestamps, coordinate lineage and exact trajectory digest. Until that exists, cross-package composition must fail rather than be inferred from similar-looking sample labels.
