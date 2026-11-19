# Synchronized Actuator Port-Power Metrology v0.1

**Status:** M1 deterministic bounded arithmetic and provenance gate. This reference does not calibrate sensors, establish traceability or qualify a physical bench.

**Research owner:** [ACTUATOR-METROLOGY-SYNCHRONIZATION-001 #6266](https://github.com/Luminous-Dynamics/symtropy/issues/6266)  
**Mission-conditioned capability:** [#6230](https://github.com/Luminous-Dynamics/symtropy/issues/6230) / [PR #6231](https://github.com/Luminous-Dynamics/symtropy/pull/6231)  
**Grouped actuator model calibration:** [#6262](https://github.com/Luminous-Dynamics/symtropy/issues/6262) / [PR #6263](https://github.com/Luminous-Dynamics/symtropy/pull/6263)  
**Robust evidence-lane Pareto:** [#6234](https://github.com/Luminous-Dynamics/symtropy/issues/6234) / [PR #6260](https://github.com/Luminous-Dynamics/symtropy/pull/6260)

## 1. The measurement chain is part of the actuator result

An actuator torque-speed map is only as trustworthy as its measurement channels. Torque and encoder speed can be logged in the same file yet refer to different physical instants because of:
- independent sensor clocks and clock mapping error;
- acquisition timestamp jitter;
- digital filtering and group delay;
- unknown or uncompensated filter phase delay;
- dynamics between the sample instants;
- calibration range, validity and drift;
- inconsistent mechanical system boundaries.

Because mechanical power is \(P(t)=\tau(t)\omega(t)\), time misalignment can distort its sign and magnitude, especially during rapid transients. The error then propagates into energy, efficiency, regeneration, impact and passivity claims.

## 2. Research and metrology anchors

### BIPM/JCGM: uncertainty models and nonlinearity

The BIPM/JCGM current Guides in Metrology catalogue lists:
- JCGM 100:2008, *Guide to the Expression of Uncertainty in Measurement* (GUM);
- JCGM 100:2008/Amd.1:2026, *Nonlinearity in measurement models*;
- JCGM 101:2008, propagation of distributions using Monte Carlo;
- JCGM 102:2011, extension to multiple output quantities;
- JCGM 106:2012, measurement uncertainty in conformity assessment;
- JCGM GUM-5:2026 and GUM-6:2020.

Source: https://www.bipm.org/en/web/guest/publications/guides

This reference intentionally begins with deterministic intervals. It does not turn a standard uncertainty into a hard error bound and does not report a probability or confidence level. Nonlinear/correlated statistical uncertainty propagation belongs in the later GUM/Monte Carlo track with documented distributions, covariance and model adequacy.

### NIST: traceability belongs to measurement results

NIST's *Metrological Traceability: Frequently Asked Questions and NIST Policy* says only measurement results—not a laboratory or organisation—are traceable. A traceability claim needs a defined measurand, described measurement system, stated result with evaluated uncertainty, specified calibration reference and measurement-assurance evidence.

Source: https://www.nist.gov/metrology/metrological-traceability

This crate checks that those fields are present and internally consistent. It does not independently verify a certificate, reference standard or traceability chain.

### Dynamic timing and synchronization

Gorman et al., *Quantifying the Performance of Distributed Large-Volume Metrology Systems for Dynamic Measurements: Methodology Development*, Metrology 6(1), 7 (30 January 2026), DOI https://doi.org/10.3390/metrology6010007, discusses synchronization strategies and treats latency, jitter, drift, temporal sampling and traceable timing as factors that must be quantified in dynamic measurement. The authors identify temporal sampling and traceable timing as limitations for deconvolving latency from measurement error.

### 2026 actuator characterization framework

Torres Rodríguez et al., *Toward Reproducible Actuator Characterization: A Comprehensive Framework for Robotics Applications*, Actuators 15(6), 301 (30 May 2026), DOI https://doi.org/10.3390/act15060301, describes eleven no-load, locked-rotor and dynamic-load protocols. It uses an output torque transducer and records DC-bus voltage/current to compute power and energy. Its idealized position-bandwidth estimate assumes instantaneous acceleration and explicitly notes omitted physical nonlinearities such as magnetic saturation, Stribeck friction and inverter limits. Its data availability statement says no new data were created or analyzed. This programme complements its test protocol with stricter timing and metrology evidence; it does not replace it.

## 3. M1 channel contract

A channel record must include:
- typed quantity: output torque (N·m) or angular velocity (rad/s);
- conservative bounded measurement interval;
- physical sensor range and calibration-covered range;
- bound representation (ValidatedBound, not standard/expanded uncertainty);
- calibration status, sensor serial, certificate, reference standard and chain digest;
- traceability status and evidence class;
- measurement source, method, raw-trace digest, system-boundary and operating-condition digests;
- common reference-clock identity and clock-mapping digest;
- reference instant, estimated sample time offset and clock-offset uncertainty;
- timestamp-jitter bound;
- measured filter group delay, applied compensation and residual delay uncertainty;
- validated absolute signal-slew bound, with source and provenance digest.

Calibration must cover the actual operating interval and the full interval after timing expansion. The calibration certificate need not cover unused portions of the sensor's entire hardware range.

## 4. Timing-aware bounded propagation

For channel c, the M1 implementation forms a conservative maximum time skew:

\`\`\`text
max_skew_c =
    abs(sample_time_offset_c)
  + clock_offset_uncertainty_c
  + timestamp_jitter_bound_c
  + abs(filter_group_delay_c - filter_delay_compensation_c)
  + filter_delay_uncertainty_c
\`\`\`

Given a validated absolute signal-slew bound \(L_c\) over that interval, expand its input interval:

\`\`\`text
aligned_lower_c = measured_lower_c - L_c * max_skew_c
aligned_upper_c = measured_upper_c + L_c * max_skew_c
\`\`\`

Both torque and angular velocity must:
- map to the same declared reference clock;
- target the same reference instant;
- use matching system-boundary and operating-condition digests;
- remain inside the calibration-covered operating range after expansion.

The power interval is the exact rectangular interval product. All four combinations are evaluated:

\`\`\`text
tau_lower * omega_lower
tau_lower * omega_upper
tau_upper * omega_lower
tau_upper * omega_upper
\`\`\`

The minimum and maximum of those four values give the conservative product interval over the Cartesian product of the two declared component intervals. This is deterministic interval arithmetic, not a probabilistic independence assumption. Known dependence could support a tighter joint set, but the rectangular interval remains an enclosure when only marginal hard bounds are available. It handles negative values and sign crossings without relying on nominal centres.

The result reports:
- pre-time-alignment power interval;
- time-aligned power interval;
- maximum additional outward power bound contributed by timing;
- per-channel time-skew and value expansion;
- calibration certificate/reference standard/chain, clock mapping, raw-trace, source/method, slew-bound, system-boundary and evidence provenance.

This is a deterministic bounded calculation. It does not claim a Gaussian distribution, a 95% confidence interval, or a GUM Monte Carlo result. Unknown dependence/common-mode timing error is not automatically handled; it must be represented in the supplied bounds or a later covariance model.

## 5. Fail-closed semantics

The result is Unknown (no numerical power claim) when:
- calibration is expired, out of tolerance, missing or unknown;
- traceability is partial/not established;
- calibration certificate serial does not match the sensor;
- the observed or time-expanded interval is outside the calibration-covered range;
- standard or expanded uncertainty is supplied where a validated deterministic bound is required;
- evidence, source, method, clock mapping, slew provenance or required digests are missing;
- channels use different reference clocks or physical reference instants;
- system boundaries or operating conditions do not match.

Malformed/non-finite/inverted numerical intervals, invalid negative timing bounds, wrong channel kinds and interval-product overflow are rejected as invalid inputs.

**Negative mechanical power is not electrical regeneration.** A negative \(\tau\omega\) result only establishes mechanical-port power direction under the declared sign convention. The DC-bus, converter, storage sink and accepted returned energy require separately measured electrical evidence.

No energy integral is emitted by this M1 crate. A future energy result must include a separate, validated sampling/interpolation/quadrature error bound and preserve time-varying power intervals.

## 6. Synthetic known-answer fixture

The examples/synchronized-port-power-known-answer.json fixture contains two wholly synthetic channels. The torque interval is [9, 11] N·m and speed is [4, 6] rad/s. Nonzero clock/sample/filter offsets and slew bounds expand the intervals before multiplication.

The fixture records:
- torque maximum skew 0.054 s and interval expansion 0.108 N·m;
- speed maximum skew 0.1025 s and interval expansion 0.3075 rad/s;
- pre-timing power interval [36, 66] W;
- aligned interval approximately [32.83371, 70.06371] W.

These values are arithmetic test data, not sensor or actuator measurements.

## 7. Run

Dependency-free crate, Rust 1.96:

\`\`\`bash
cargo fmt --manifest-path benchmarks/synthetic-joints/synchronized-metrology/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/synchronized-metrology/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/synchronized-metrology/v0.1/Cargo.toml --all-targets --locked -- -D warnings
\`\`\`

Dedicated workflow: .github/workflows/actuator-synchronized-metrology.yml. The local authoring environment has no Cargo/rustc/rustfmt; exact-head CI is the compilation/format/Clippy gate.

Twenty-three Rust tests cover rectangular interval multiplication, negative and zero-crossing power, clock/slew expansion, filter-delay compensation, expired calibration, serial mismatch, incomplete traceability, different clocks/instants, uncertainty-vs-bound semantics, boundary/condition mismatch, missing slew provenance, calibration range limits, preserved vendor evidence, wrong channel kinds, invalid intervals, traceability/clock-map abstention, raw-trace receipt preservation and timing overflow.

## 8. M0–M5 promotion

- **M0:** inventory measurands, sensor chains, clocks, system boundaries, calibration and uncertainty sources.
- **M1:** bounded synchronized-port arithmetic and fail-closed provenance semantics.
- **M2:** nonlinear/correlated uncertainty propagation, Monte Carlo references and timing-skew adversarial replay.
- **M3:** common-clock instrumented bench with independent torque/encoder/DC-bus/temperature channels, calibration before/after and intermediate checks.
- **M4:** synchronized versus deliberately skewed dynamic task/impact traces with independently measured timing error.
- **M5:** calibration drift, transducer overload, remounting and post-service requalification.

## 9. Claim ceiling

M1 computes a bounded instantaneous mechanical-port power interval from externally supplied bounds. It does not calibrate sensors, establish traceability, validate slew or clock bounds, infer electrical regeneration, integrate energy, qualify a dynamometer or establish human-robot safety. Physical evaluation needs a separate metrology/test plan, competent operators, rated guarded fixtures, traceable calibrations and predeclared stop limits.
