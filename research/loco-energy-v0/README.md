# LOCO Energy Ledger V0

Dependency-free Rust reference for source-qualified locomotion energy accounting and cost-of-transport normalization, tracked by [LOCO-ENERGY-001 / #4044](https://github.com/Luminous-Dynamics/symtropy/issues/4044).

## Paradigm shift: energy is a boundary-resolved ledger, not one reward scalar

A single CoT number is not comparable unless the energy boundary, mass, gravity and measured path distance are identical. V0 therefore retains separate ledgers for:

1. Battery/source terminal energy (discharge, energy returned, net).
2. DC-bus energy delivered downstream.
3. Actuator electrical input.
4. Joint mechanical work (positive and absorbed/negative work).
5. Auxiliary loads.
6. Standby/control-on baseline.

There is deliberately no API that adds these boundaries into one "total": battery energy contains downstream consumption, so adding battery + bus + actuator + joint + auxiliary energy would double-count nested flows. Multiple independent streams can be aggregated only within the same declared boundary. Mechanical negative work is not evidence of electrical regeneration.

## Exact units and integration

Input power is signed integer picowatts (pW). Timestamps are integer nanoseconds (ns). The product pW × ns is exactly zeptojoules (zJ), so no floating-point conversion is needed during integration.

Each stream is integrated as a piecewise-linear signed-power signal using exact trapezoids. If a segment crosses zero, positive and negative triangular lobes are analytically split. Both are reported; negative power is never silently clipped. All rational arithmetic and accumulators are checked and fail closed on overflow.

Convenience constructors convert:
- battery voltage in microvolts and current in microamps to signed pW (positive current means battery discharge; negative means energy returned);
- joint torque in micro-newton-metres and angular velocity in microradians/s to signed mechanical pW.

Battery discharge, returned energy and net energy are preserved as exact rational zJ values. Joint mechanical positive and negative work remain in their own ledger.

## Cost of transport

For a source-terminal energy boundary:

    CoT = E / (m * g * d)

V0 computes the dimensionless ratios exactly using:
- total robot mass in milligrams (including battery, payload, sensing and compute);
- gravity in micrometres/s²;
- measured path distance in micrometres;
- energy in zJ.

It reports separate discharge-CoT, returned-CoT and net-CoT. The primary source-terminal discharge metric is not silently idle-subtracted. Zero/near-zero distance yields the NoProgress status and no finite CoT, while the consumed energy remains in the report. Mechanical-work normalization is not mislabeled as battery CoT.

## Provenance and integrity controls

Every run binds run, robot, controller and mass-configuration digests. Every input stream binds:
- unique stream and sensor IDs;
- energy boundary;
- source artifact digest;
- calibration digest;
- clock-basis digest;
- configured maximum sample gap;
- exact sample IDs, timestamps and sample count.

Streams reject empty identities, duplicate sample IDs, duplicate stream IDs, non-monotonic timestamps, excessive sample gaps and run-identity mismatch. The report preserves sorted stream receipts, observed maximum sample gap and start/end timestamps.

This crate does not align asynchronous signals. Battery V/I must be paired at a common sample timestamp; torque and angular velocity must be qualified and time-aligned upstream. A clock digest is provenance, not proof that a clock mapping is correct. A configured maximum-gap limit rejects large gaps, but cannot prove that no sample was dropped inside that bound. Missing samples must never be inserted as zero.

## What this V0 does not claim

- It does not prove sensor calibration, current polarity, torque/velocity alignment or source-artifact authenticity.
- It does not estimate unmeasured driver/motor losses or reconstruct battery energy from mechanical work.
- It does not automatically subtract standby energy.
- It does not estimate confidence intervals for integration error; uncertainty propagation and source-boundary closure are subsequent work.
- It does not establish real-world energy efficiency or outperform a physical robot.

## Known-answer and adversarial tests

Eighteen deterministic tests cover:
- exact constant positive and negative power;
- sign-crossing positive/negative triangle integration;
- nonuniform time intervals;
- exact battery V/I and joint torque/velocity unit conversion;
- independent battery/DC-bus/mechanical boundary preservation;
- exact CoT for 1 kg traversing 10 m with 98.0665 J input;
- zero-progress and missing-battery behavior;
- duplicate sample/stream identity;
- non-monotonic timestamps and excessive sample gaps;
- run identity, mass, gravity and ratio validation;
- rational reduction, overflow and input-order invariance.

## Commands

    cargo fmt --manifest-path research/loco-energy-v0/Cargo.toml -- --check
    cargo test --manifest-path research/loco-energy-v0/Cargo.toml --locked
    cargo clippy --manifest-path research/loco-energy-v0/Cargo.toml --locked --all-targets -- -D warnings

Dedicated exact-head workflow: .github/workflows/loco-energy.yml.

## Industry comparators and fair-comparison boundary

- RAIBO2, *A quadruped robot designed to complete a marathon on a single battery charge*, Nature (23 September 2026): https://doi.org/10.1038/s41586-026-11102-5. The reported marathon/CoT result is a full-system hardware reference, not a directly comparable FlyGym or hexapod simulation result.
- Bjelonic, Tischhauser & Hutter (IJRR, first online 31 July 2026), *Towards bridging the gap: Systematic sim-to-real transfer for diverse legged robots*: https://doi.org/10.1177/02783649261459628. Reports a physics-grounded actuator energy model and a 32% full-CoT reduction for ANYmal, with transfer across additional platforms.
- ISO 18646-5:2026 defines legged-robot locomotion performance tests, but explicitly is not intended to verify or validate safety requirements: https://committee.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/08/68/86850.html

Do not compare RAIBO2 hardware battery-terminal CoT with FlyGym mechanical work, or with Symtropy's simulator output, as though they were the same boundary. Use #3357 for matched physics/controller/body baselines and #4044 for the energy boundary and receipt protocol. The next major energy milestone is measured source-terminal and actuator/driver closure on a hardware platform, with uncertainty bounds and matched task/speed/success conditions.

## Claim ceiling

Synthetic tests validate exact unit conversion, integration and metric semantics. Simulated energy is conditional on actuator, drive, battery and contact models. Hardware battery CoT requires calibrated battery-terminal voltage/current, total mass and measured route distance. No Symtropy energy-performance improvement is claimed by this reference.
