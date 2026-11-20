# Paired Actuator Load-Sharing and Thermal Asymmetry v0.1

**Status:** M1 per-unit output-torque, loaded-speed, peak-duration and temperature envelope checker with optional current-share reporting and joint-torque reconciliation. This is not a thermal-life model or a motor-current-to-torque estimator.

**Research owner:** [PAIRED-ACTUATOR-LOAD-SHARING-001 #6292](https://github.com/Luminous-Dynamics/symtropy/issues/6292)  
**Synchronized metrology:** [#6266](https://github.com/Luminous-Dynamics/symtropy/issues/6266) / [PR #6272](https://github.com/Luminous-Dynamics/symtropy/pull/6272)  
**Mission-conditioned capability:** [#6230](https://github.com/Luminous-Dynamics/symtropy/issues/6230) / [PR #6231](https://github.com/Luminous-Dynamics/symtropy/pull/6231)  
**Thermal mission envelope:** [#6155](https://github.com/Luminous-Dynamics/symtropy/issues/6155)

## 1. Research motivation

Yang, *Upgrading a Humanoid Leg Without Replacing Actuators: Torque–Speed Limits and Bench Measurements*, Applied Sciences 16(20), 10145 (13 October 2026), DOI https://doi.org/10.3390/app162010145, evaluates a paired hip-roll drive using two actuators and a 24:55 external gear pair.

The study reports that:
- at approximately twice its reference moment, measured maximum joint speed was 47.8% below the no-load upper bound;
- at 23.39 N·m, three static holds ended after 333, 351 and 394 s at the prespecified 70 °C reported-temperature limit;
- both lower-load conditions completed 600 s;
- one unit ran hotter and drew a larger current fraction than its partner.

The paper explicitly distinguishes observed current/temperature asymmetry from individual torque contribution and states that the holds do not establish an isolated continuous actuator rating. It is a source-specific paired assembly experiment, not a universal drive limit.

This makes a broader qualification issue visible: a shared output shaft or common motor command does not prove equal internal actuator effort. A sum of two individually plausible torque ratings can hide one unit exceeding its continuous, peak-duration, speed or thermal boundary.

## 2. Paradigm: distributed actuator effort envelope

For a joint with N actuators, the evaluated system is not just one aggregate torque. It is the vector of per-unit states at one common reference plane and instant:

```text
unit_i = (
  output-equivalent torque interval,
  loaded output-speed interval,
  continuous/peak torque lower bounds,
  remaining peak-duration lower bound,
  temperature interval and thermal-limit lower bound,
  evidence/calibration/provenance
)
```

The joint output interval is the conservative sum of individual torque intervals only when all contributions are expressed at the same output reference plane, physical instant, clock, system boundary and operating condition.

The M1 reference never assumes:
- equal torque sharing because commands are equal;
- 50/50 contribution because two motors are mechanically coupled;
- torque fraction equals current fraction;
- average temperature represents the hottest winding/unit;
- aggregate joint torque means every actuator is within its own envelope.

## 3. Separate electrical and mechanical sharing

Current-magnitude share is optionally reported as an electrical observation:

```text
current_share_i = I_i / sum(I_j)
```

Output-torque share is separately calculated from per-unit output-equivalent torque intervals:

```text
torque_share_i = tau_i / sum(tau_j)
```

Each share is an interval, not a scalar point estimate. If the aggregate denominator includes zero or an actuator contribution is missing, its share is unknown.

The code contains no conversion from current to torque. Such a model would need, at minimum, a calibrated torque constant/back-EMF/current model, winding-temperature correction, drive/inverter limits, direction-specific reducer efficiency/friction and matching calibration/configuration digests. Even then it would be labelled ModelDerived/Estimated rather than independent torque measurement. Synthetic values are carried as `Synthetic` and validate arithmetic only; they cannot satisfy an independent-physical evidence requirement.

## 4. Per-unit capability and thermal gates

Each unit is assessed separately:
- delivered output-torque magnitude upper bound versus continuous torque capability lower bound;
- where continuous is exceeded, delivered demand versus peak torque lower bound;
- peak-limited task segment duration versus the unit's own remaining qualified peak-duration lower bound;
- loaded output-speed upper bound versus the unit's qualified speed lower bound;
- temperature upper bound versus thermal-limit lower bound.

The joint disposition is:
- **Infeasible:** at least one unit has a known torque, duration, speed or thermal violation, or the unit-torque sum contradicts the independent joint-output transducer.
- **Unknown:** at least one required per-unit torque, capability, speed, temperature or evidence field is unsupported, and there is no known violation.
- **Feasible:** every per-unit gate passes under the submitted bounds and evidence.

This is a qualification disposition under submitted M1 records only; it is not whole-robot task success or a safety decision. The hottest unit and minimum thermal margin are reported. Average temperature is not used to pass the joint.

## 5. Joint-port cross-check

The evaluator subtracts the declared joint-output reference interval from the sum of individual unit torque intervals. The cross-check result is evidence-scoped:

- **Consistent / Contradictory:** reserved for a cross-check labelled `IndependentPhysical`.
- **SyntheticConsistent / SyntheticContradictory:** used for a synthetic known-answer reference. These statuses qualify only interval arithmetic and never represent an independent physical observation.

A synthetic contradictory fixture can still make the arithmetic disposition Infeasible for that synthetic case; it does not establish that a real actuator assembly failed.

The cross-check must use the same:
- joint configuration digest;
- output reference plane;
- reference clock and physical instant;
- system boundary and operating-condition digest.

A missing joint-output transducer is reported as NotAvailable; it does not invent a contradiction or an agreement.

## 6. Evidence contract

Each actuator unit preserves separate evidence for:
- output torque and its calibration/method/source;
- continuous/peak/speed capability and its envelope digest/source/method;
- temperature/thermal limit and its calibration/method/source;
- optional current magnitude, its calibration/method/source.

Evidence classes remain Synthetic, IndependentPhysical, SourceAuthorPhysical, VendorPhysical, ModelDerived or Unknown. Synthetic is a first-class non-physical class: synthetic known-answer inputs and their synthetic joint-output cross-checks must never be labelled IndependentPhysical. Only an IndependentPhysical joint-output cross-check is an independent physical measurement; the evaluator preserves the class in every report. A vendor current measurement cannot be promoted to independent torque; a model-derived torque envelope cannot be silently treated as independent physical capability.

## 7. Synthetic known-answer fixtures

`paired-actuator-sharing-known-answer.json` contains two fully synthetic cases:

1. **Asymmetric pair:** unit A torque [2, 3] N·m, unit B [7, 8] N·m; current intervals [4, 5] A and [8, 9] A. The torque-share interval for unit A differs from its electrical current-share interval. A synthetic joint-output reference overlaps the unit sum; it tests arithmetic consistency, not independent measurement.
2. **Hot-unit negative control:** unit D temperature interval [68, 71] °C against a 70 °C thermal-limit lower bound. The other unit is cooler. The joint is Infeasible because the hottest unit's upper temperature exceeds the limit, regardless of the pair's average.

All values, URN references and calibration IDs are synthetic placeholders. Their evidence class is `Synthetic`, not `IndependentPhysical`; they are not actuator measurements or physical performance claims.

## 8. Run

Dependency-free Rust crate, Rust 1.96:

```bash
cargo fmt --manifest-path benchmarks/synthetic-joints/paired-actuator-sharing/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/paired-actuator-sharing/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/paired-actuator-sharing/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

Dedicated workflow: `.github/workflows/paired-actuator-load-sharing.yml`.

Twenty-two Rust test definitions cover conservative torque sum, unequal torque shares, missing shares, no equal-share assumption, hottest-unit thermal gate, aggregate-pass/unit-fail, loaded speed, per-unit peak duration, joint-transducer agreement/contradiction, reference-plane/clock/condition mismatch, unknown evidence, preserved vendor/model classes, separate current share, joint configuration/time binding, per-unit evidence receipts, independent-only joint cross-check, known peak overload precedence and malformed capabilities.

## 9. M0–M5 and prospective targets

- **M0:** define per-unit output reference planes, sensor chain and thermal boundary.
- **M1:** per-unit robust envelope, load-share intervals and joint-torque reconciliation.
- **M2:** calibrate per-unit torque/current/temperature relationships and validate estimated sharing on held-out paired specimens.
- **M3:** synchronized paired-drive rig with per-unit torque/current/voltage/temperature and a common joint-output torque transducer.
- **M4:** loaded-speed and hold missions; retain all asymmetry, derating and termination events.
- **M5:** characterize load-share and thermal drift after wear, remounting, firmware changes, maintenance and component replacement.

Prospective targets (not achieved results):
- zero equal-share defaults;
- 100% of per-unit contributions measured/bounded at one declared output plane or explicitly Unknown;
- zero torque-share claims from current alone without a calibrated uncertainty-bearing model;
- the worst unit governs the joint capability/thermal disposition;
- every torque-sum residual is retained against the independent joint transducer;
- terminated holds remain finite-duration observations and are never promoted to continuous ratings.

## 10. Claim ceiling and safety

M1 is interval arithmetic and evidence gating only. It does not infer torque from current, forecast thermal life, qualify a paired drive or authorize physical tests. M3+ requires separate hazard analysis, competent operators, guarded rated fixtures, calibrated independent channels and preregistered stop limits.
