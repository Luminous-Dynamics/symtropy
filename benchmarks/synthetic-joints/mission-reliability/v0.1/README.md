# Mission Reliability, Censoring and Lifecycle Degradation v0.1

**Status:** M1 statistical reference and synthetic schema fixture. This directory contains no physical lifetime campaign and no empirical component failure-rate estimate.

**Research owner:** [MISSION-RELIABILITY-001 #6195](https://github.com/Luminous-Dynamics/symtropy/issues/6195)  
**Common result/evidence contract:** [#6140](https://github.com/Luminous-Dynamics/symtropy/issues/6140)  
**Thermal mission profile:** [#6155](https://github.com/Luminous-Dynamics/symtropy/issues/6155)  
**Paired actuator observability:** [#6171](https://github.com/Luminous-Dynamics/symtropy/issues/6171)  
**Task-space capability:** [#6189](https://github.com/Luminous-Dynamics/symtropy/issues/6189)

## 1. Paradigm shift: reliability is a specimen-level mission history

A joint that completes 10,000 cycles is not "10,000 independent successful samples." It is one specimen with 10,000 cycles of exposure. If that assembly is later remounted or repaired, its identity/history changes but it does not magically become a new independent specimen.

Likewise:
- a thermal derating or protection stop can be a valid event without mechanical fracture;
- a test ending before failure is right-censored, not a pass at infinite life;
- an actuator replaced in one assembly creates a parent/child specimen history;
- functional capability can degrade long before a binary failure;
- competing failure modes (thermal, mechanical, electrical, transmission, sensor/controller) cannot be collapsed into one label without losing information.

The qualification object therefore joins **pre/post functional capability**, **mission exposure**, **specimen-level event/censor records**, and **service history**.

## 2. Standards and research context

### ISO 9283 — matched functional robot performance

ISO 9283 defines industrial robot tests including pose/path accuracy and repeatability, drift, stabilization time, path velocity, static compliance and related performance characteristics. Its comparison principle is directly relevant: robot-to-robot comparison needs the same test cube, loads, velocities, paths, cycle definitions and environmental conditions. https://www.iso.org/standard/22244.html

Our contribution is to retain those matched task-performance measurements before and after mission/lifecycle exposure and bind them to the same physical specimen identity.

### IEC 60068-2-64:2026 — broadband random vibration and degradation

The third edition of IEC 60068-2-64 was published 8 October 2026. It describes broadband random vibration methods for components/equipment and an initial low-level response investigation, the stress test, and a final response investigation to identify changes in dynamic behavior. https://webstore.iec.ch/en/publication/104354

For joint qualification this supports an important principle: measure the dynamic response before and after the environmental stress, not merely whether the specimen survived the exposure. The amplifier, vibrator, fixture, specimen, control and mounting points form part of the test boundary.

### IEC 60068-2-86:2024 — multi-axis vibration and transient methods

IEC 60068-2-86:2024 covers multi-exciter/multi-axis vibration methods and certain shock/transient applications. https://webstore.iec.ch/en/publication/66483

### Robot performance evaluation review

A 2026 systematic review summarizes ISO 9283-derived accuracy, repeatability, orientation and path metrics and common metrology approaches including laser trackers, CMMs and vision systems. https://www.mdpi.com/2227-7080/14/5/297

These references do not prescribe our joint-specific survival estimator or lifecycle data model. They supply matched functional and environmental-test context.

## 3. Record contract

`mission_reliability_profile.schema.json` is a JSON Schema Draft 2020-12 record for:

- campaign protocol/preregistration and frozen analysis plan;
- exact specimen identity, component/assembly hierarchy, revision, lot and prior service;
- mission and environmental exposure expressed in explicit units;
- first-event status: observed, right-censored, left-censored, interval-censored, or competing event;
- event definition, failure mode, detection method, exposure and raw-trace evidence;
- initial/intermediate/final/post-service functional measurements;
- statistical estimator choice, censoring assumptions and independent-unit rule.

Unknown data remain null or explicitly unknown with a reason. The schema does not certify instrument quality, event truth or statistical assumptions.

The fixture `examples/competing-risk-known-answer.json` is entirely synthetic and exists only for schema/known-answer checking. Its five specimen rows must not be cited as actuator reliability evidence.

## 4. Statistical reference implemented in Rust

The dependency-free crate in `src/lib.rs` currently supports:
- Kaplan–Meier product-limit survival for **time to first event**, with right censoring and a risk table;
- Greenwood standard error at each event/censor time (undefined when the final risk set is exhausted);
- Aalen–Johansen cumulative incidence for mutually competing first-event causes, preserving event-free survival and cause-specific incidence;
- restricted mean event-free time up to a prespecified horizon, refusing to extrapolate past maximum observed follow-up;
- specimen ID uniqueness, finite/nonnegative exposure, cause and common-unit validation.

Nine synthetic test definitions cover known product-limit values, risk-set arithmetic, Greenwood SE, competing-risk probability-mass balance, tied event/censor handling, restricted mean, out-of-follow-up rejection, duplicate specimen IDs, mixed units, bad event causes and exhausted risk sets.

### Deliberate statistical boundaries

This is a compact M1 reference, not a full survival-analysis package:
- one first-event or right-censor record per independent specimen only;
- left/interval censoring is represented in the schema but the estimator rejects/does not implement it;
- recurrent-event processes are not estimated;
- cluster frailty, covariates, time-varying exposures and repair/re-entry are not implemented;
- no Weibull/parametric fleet-life extrapolation or accelerated-life transform;
- no confidence interval for the Aalen–Johansen estimator is implemented;
- no inference is valid unless the censoring/independence assumptions are justified by the campaign.

Repeated cycles are exposure units within a specimen. They do not become separate independent specimen records.

## 5. Run the analytic reference

The crate uses Rust 1.96 and no external dependencies:

```bash
cargo fmt --manifest-path benchmarks/synthetic-joints/mission-reliability/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/mission-reliability/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/mission-reliability/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

The dedicated workflow is `.github/workflows/mission-reliability-reference.yml`. The authoring environment has no local Cargo/rustc/rustfmt; exact-head CI is required before reporting the Rust checks as passing.

## 6. Required pre/post scorecard

At each preregistered exposure block, measure the same specimen under matched conditions:
- task-space wrench region and required-force coverage;
- pose/path accuracy, repeatability and stabilization time;
- backlash, friction, compliance/stiffness matrix and cross-axis coupling;
- torque-speed and bandwidth under the same load/temperature;
- thermal mission envelope and first-to-derate state;
- bus/mechanical energy ports and accepted/dissipated energy;
- calibration offset and instrument uncertainty.

Report continuous drift and paired pre/post deltas, not only threshold crossings. Keep metrology drift separate from physical degradation using calibration checks and reference artifacts.

## 7. Prospective benchmarks

Targets, not current results:
- every event/stop has a specimen ID, event/censor status, exposure, cause/status and evidence reference;
- zero repeated cycles counted as independent specimens;
- matched test cube/load/speed/path/cycle/environment boundaries for comparisons;
- every specimen has initial/final functional characterization or a missingness reason;
- time-to-event estimates include risk tables and uncertainty; if assumptions or sample size are insufficient, report unknown;
- competing causes are reported with cumulative incidence rather than being silently folded into one pass/fail fraction;
- post-service changes have an explicit parent/child specimen relationship and requalification decision.

Do not set fleet-reliability percentages, B10 life, confidence targets or acceleration factors after seeing the results. Preregister the analysis and sample-size rationale against the intended application.

## 8. Claim ceiling and safety

The schema, fixture and Rust code are synthetic software/analysis references. They do not constitute a reliability study, life test, vibration qualification, field-life prediction or safety case.

No destructive/life test, vibration/shock test, energized test, intentional thermal trip, human-contact experiment or field deployment is authorized by this directory. Physical campaigns require a separate approved plan, competent operators, rated and independently verified fixtures/protections, applicable standard access, and hazard analysis.
