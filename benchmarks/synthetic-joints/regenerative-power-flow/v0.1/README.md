# Regenerative Power Flow and DC-Bus Qualification v0.1

**Status:** M0/M1 evidence contract and source-log transcription. No new electrical experiment, regenerative-energy measurement or hardware qualification is claimed.

**Research owner:** [REGENERATIVE-POWER-FLOW-001 #6185](https://github.com/Luminous-Dynamics/symtropy/issues/6185)  
**Paired actuator observability:** [#6171](https://github.com/Luminous-Dynamics/symtropy/issues/6171)  
**Thermal mission envelope:** [#6155](https://github.com/Luminous-Dynamics/symtropy/issues/6155)  
**Common result contract:** [#6140](https://github.com/Luminous-Dynamics/symtropy/issues/6140)

## 1. Why this belongs in joint qualification

A joint can produce negative mechanical work without returning electrical energy to a battery or DC bus. Returned mechanical energy may become winding/inverter/gear heat, raise bus voltage, enter a braking resistor, charge storage, or be consumed by other joints.

The four-quadrant capability is therefore a property of the **joint + drive + DC link + source/sink + storage + other loads + protection + control schedule**. It is not solely a motor property.

This work adds an explicit signed energy ledger and system-boundary contract.

## 2. Source anchors

### Paired MX-106 hip-roll bench: actual overvoltage evidence

Yang, *Upgrading a Humanoid Leg Without Replacing Actuators: Torque–Speed Limits and Bench Measurements*, Applied Sciences 16(20), 10145 (13 October 2026): https://doi.org/10.3390/app162010145

Public code, run ledger and logs: https://github.com/yjh-mcut/redesign-methodology

Three downhill/overhauling-load logs were marked invalid by the author because the source-only switched-mode bench supply could not sink returned energy:
- Drive 1 / drive 2 maximum voltage: 21.8/21.6 V, 24.1/23.8 V, and 21.5/21.3 V.
- Prespecified actuator upper-voltage limit: 16.0 V.
- Raw log counts above 16.0 V: 116/116, 58/58 and 117/116 samples.
- The logs contain 800 rows each.

The example record preserves those actual raw file references and Git blob SHAs. It marks the voltage channel as **drive-reported**, not as an external calibrated bus measurement. It does not infer excursion duration from sample count, because sample timing/threshold dwell was not independently qualified here.

Crucially, the raw records do not expose independently calibrated DC current at each drive, DC-link capacitance/ESR, source sink current, battery acceptance or braking-resistor energy. Consequently, **regenerated electrical energy remains unknown**. The data prove a bus-voltage/protection problem, not recovered energy.

### Related research

Wang et al., *Self-Aligning Torque Energy Recovery and Bus-Voltage Stabilization in Steer-by-Wire Systems for New Energy Vehicles*, Actuators 15(7), 397 (14 July 2026), https://doi.org/10.3390/act15070397, explicitly models the mechanical-to-electrical-to-DC-bus path and distinguishes available mechanical energy, gross returned electrical energy, losses and net recovered energy. It also evaluates DC-bus voltage stabilization and notes that system benefit depends on duty cycle, concurrent loads, battery acceptance and converter efficiency.

Muru and Rassõlkin, *A Scoping Review of Energy Consumption in Industrial Robotics*, Machines 13(7), 542 (23 June 2025), https://doi.org/10.3390/machines13070542, discusses DC-bus buffering and energy management for industrial robots.

WO2025259918A1, *Distributed robot controller*, https://patents.google.com/patent/WO2025259918A1/en, describes shared-DC-bus architectures where one joint's regenerated energy may power other motors, with centralized braking dissipation and optional storage. This is an architecture disclosure, not independent proof of a particular implementation's performance.

## 3. Paradigm shift: account at explicit energy ports

The sign conventions must be fixed before a run. Record at least:

- Joint mechanical port: `P_joint = tau_joint * omega_joint`.
- Motor shaft mechanical port: `P_shaft = tau_motor * omega_motor`.
- Drive electrical port: `P_dc = V_dc * I_dc`, with current sign convention.
- DC-link stored energy: `E_cap = 0.5 * C_dc * V_dc^2`, including measured distributed capacitance where relevant.
- Battery or storage port: synchronized voltage/current and measured accepted energy.
- Shared-bus peer loads: energy transferred to other drives.
- Dissipation: braking resistor/chopper, inverter, winding, gear and mechanical-friction sinks.

For a chosen control volume, report:

```text
E_in - E_out - delta_E_stored - E_dissipated = residual
```

The residual is not a free balancing term. It must be compared with calibrated measurement uncertainty and declared unmetered paths. Unknown losses and storage remain unknown rather than being set to zero.

Do not claim energy recovery from:
- negative joint mechanical work alone;
- voltage rise alone;
- a drive current register that has not been calibrated as DC current;
- a negative power sample without integrating the correct boundary and sign convention.

## 4. Architecture families to benchmark

| Architecture | What it can do | Main qualification questions |
|---|---|---|
| Source-only supply + passive DC link | Baseline motoring, transient buffer only | Overvoltage envelope, safe derating/abort, no sustained energy sink |
| Braking chopper/resistor | Absorbs surplus energy as heat | Clamp threshold, pulse energy, duty, resistor and enclosure temperature |
| Battery-accepting regenerative bus | Stores energy in battery | BMS acceptance vs SOC/temperature/voltage, converter limits and faults |
| Supercapacitor/hybrid store | Handles high-power transients | Usable voltage window, ESR, converter losses, aging, mass and thermal burden |
| Shared multi-joint bus | Braking joint supplies a motoring joint | Concurrent demand, cable/converter loss, stability and worst-case simultaneous braking |
| Trajectory/elastic co-design | Reduces peaks or reuses energy mechanically | Task success, impact and torque constraints, storage/elastic losses |

No architecture is universally best. Report the complete installed-system Pareto frontier.

## 5. Required test matrix

1. Forward/reverse motoring and braking at matched torque-speed points.
2. Controlled overhauling loads and deceleration trajectories, with pulse and repeated-mission cases.
3. Characterized source/sink modes: source-only, bidirectional supply, battery acceptance and storage.
4. Measured DC-link capacitance, ESR, cable resistance/inductance and bus topology.
5. Chopper clamp voltage, pulse energy, duty and resistor thermal state.
6. Battery/BMS acceptance across its declared SOC and temperature envelope.
7. Supercapacitor charge/discharge current, voltage, ESR, temperature and converter efficiency.
8. Multiple joints: one regenerating while another motors; simultaneous braking; all-joints regenerative worst case.
9. Controller synchronization, bus voltage stability, communication loss, current limits, over/undervoltage protection.
10. Hot-start and repeated mission profiles; held-out trajectories and storage states.

Any physical test is a separate guarded electrical-engineering activity. This protocol is not authorization to intentionally overvolt hardware or modify protection.

## 6. Metrics — keep gross and net quantities separate

Report:
- Positive/negative mechanical work at joint and motor shaft.
- Gross energy returned to the DC bus.
- Energy accepted by battery/supercapacitor.
- Energy transferred to other joints.
- Energy dissipated by chopper/resistor and other thermal losses.
- DC-link/storage energy change.
- Peak/minimum DC voltage, ripple, slew rate, threshold dwell and protection events.
- Peak regenerative power and repeated-duty thermal exposure.
- Round-trip efficiency by path and operating quadrant.
- Net electrical energy per successful task including controllers, converters, storage and cooling.
- Added mass, volume, cost, wiring, fault modes and service burden.
- Task success/failure and derating under held-out missions.

Gross bus-return energy is not net battery-stored energy. Both are useful; they are different metrics.

## 7. M0–M5 promotion

- **M0:** frozen port/sign definitions, system boundaries, sensor positions, storage/source/sink details, raw traces and calibration.
- **M1:** known-answer RC/DC-link energy cases, integration convergence, passive sign checks, energy residual accounting and missing-channel negative controls.
- **M2:** independent circuit/electromechanical reconstruction and held-out waveform predictions.
- **M3:** guarded component rig with calibrated mechanical and bidirectional electrical power measurement, independent bus protection and thermal channels.
- **M4:** multi-joint mission, concurrent motoring/braking and actual energy reuse.
- **M5:** storage aging, resistor thermal cycling, cables/connectors, actuator replacement and requalification.


## 7. Dependency-free analytic Rust reference

The crate in this directory implements M1 accounting primitives in `src/lib.rs`:

- Four-quadrant classification from the signs of motor torque and speed.
- Signed trapezoidal integration of sampled port power into joules.
- Ideal DC-link capacitor stored energy and signed energy change.
- DC-bus threshold excursion counts without falsely inferring energy or dwell time.
- Declared-control-volume energy-balance residual.
- Fail-closed checks for non-finite values, negative capacitance/voltage, duplicate/non-monotonic timestamps and undersampled data.

Twelve Rust tests provide known-answer and negative-control coverage. They are analytic software tests, not tests on energized hardware or a validated inverter/battery model.

Run the package checks with Rust 1.96:

```bash
cargo fmt --manifest-path benchmarks/synthetic-joints/regenerative-power-flow/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/regenerative-power-flow/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/regenerative-power-flow/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

The dedicated workflow is `.github/workflows/regenerative-power-flow-reference.yml`. The current environment does not include a Rust toolchain, so execution status is determined only by exact-head CI; no local test pass is claimed.

## 8. Prospective targets

- Every result names the control volume and included/excluded energy ports.
- Zero cases where negative mechanical work is labelled regenerated battery energy without a measured electrical/storage path.
- Energy-balance residual threshold preregistered from calibration and instrument uncertainty.
- Held-out bus-voltage and energy-flow predictions report interval coverage.
- Lower net energy per successful task versus a matched non-regenerative or resistor-only baseline without violating voltage, current, thermal, torque or task requirements.
- Report energy reused by peer joints, accepted by storage and dissipated as separate outcomes.
- Preserve all overvoltage, undervoltage, current-limit, abort, derating and censored runs.

These are targets, not achievements.

## 9. Claim ceiling and safety

The schema and example are source-log transcription and protocol artifacts. They are not an energy-recovery result, independent physical replication or hardware qualification.

No energized test, intentional overvoltage, battery experimentation, real braking-resistor design, fabrication, hardware modification or human-contact use is authorized. Physical work requires qualified electrical engineering review, rated protective hardware, guarded fixtures, interlocks, manufacturer limits and a separate hazard analysis.
