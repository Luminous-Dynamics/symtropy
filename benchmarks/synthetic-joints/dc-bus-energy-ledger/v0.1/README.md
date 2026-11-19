# DC Bus Multi-Control-Volume Energy Ledger v0.1

**Status:** M1 interval energy-balance reference. It accepts already-integrated signed energy intervals; it does not integrate raw power, infer storage state of charge, validate measurement traceability or certify a physical recovery system.

**Research owner:** [DC-BUS-ENERGY-LEDGER-001 #6276](https://github.com/Luminous-Dynamics/symtropy/issues/6276)  
**Parent regenerative-power programme:** [#6185](https://github.com/Luminous-Dynamics/symtropy/issues/6185)  
**Synchronized metrology:** [#6266](https://github.com/Luminous-Dynamics/symtropy/issues/6266) / [PR #6272](https://github.com/Luminous-Dynamics/symtropy/pull/6272)  
**Mission-conditioned capability:** [#6230](https://github.com/Luminous-Dynamics/symtropy/issues/6230) / [PR #6231](https://github.com/Luminous-Dynamics/symtropy/pull/6231)  
**Discrete passivity:** [#6200](https://github.com/Luminous-Dynamics/symtropy/issues/6200) / [PR #6203](https://github.com/Luminous-Dynamics/symtropy/pull/6203)

## 1. Why the benchmark must follow energy across system boundaries

Negative mechanical work at a joint is not proof of energy returned to a DC bus. Energy may become motor/inverter heat, gear friction, DC-link capacitor energy, braking-resistor dissipation, energy reused by another drive, energy accepted by a battery terminal, or energy actually retained by storage. These are distinct physical outcomes.

The benchmark therefore treats the robot's power system as connected control volumes:
1. actuator module;
2. shared DC bus;
3. storage system.

Each volume has its own energy balance. The physical port connecting adjacent volumes is also checked independently, so three locally balanced ledgers cannot hide a mismatch at the boundary between them.

## 2. SOTA context: optimize net mission energy, not a regeneration headline

### RAIBO2: marathon-scale system energy

Lee et al., *A quadruped robot designed to complete a marathon on a single battery charge*, Nature 658, 372–379 (23 September 2026), https://www.nature.com/articles/s41586-026-11102-5, report:
- 42.195 km completed in 4 h 19 min 52 s on one battery charge;
- total cost of transport of 0.25;
- more than three times the travel range of existing quadrupeds in the authors' comparison;
- a loss model spanning lightweight/force-transparent mechanics, low-resistance motor-drive circuitry and locomotion policy, with actuator loss components grounded in measured data.

The paper data are linked via https://doi.org/10.5281/zenodo.14825866.

This is a robot-level mission-energy comparator, not a joint regenerative-efficiency test. It raises the relevant target: lower net energy per successful mission, measured at the complete robot boundary, without sacrificing task completion, speed, thermal margin, torque reserve or reliability.

### Energy recovery in an adjacent drive domain

Wang et al., *Self-Aligning Torque Energy Recovery and Bus-Voltage Stabilization in Steer-by-Wire Systems for New Energy Vehicles*, Actuators 15(7), 397 (14 July 2026), https://doi.org/10.3390/act15070397, explicitly distinguishes available mechanical energy, gross recovered electrical energy, losses, net recovered energy and efficiency. The reported result emphasizes repeated duty, concurrent loads, converter efficiency and battery acceptance.

This is useful adjacent-domain evidence, not proof that a particular topology is optimal for robot joints.

## 3. Frozen sign conventions and control-volume equations

All energy quantities are in joules. Each term is signed according to the convention below. The control-volume boundary and included/excluded ports must be frozen before evaluation.

### A. Actuator module

- dc_terminal_in: positive when electrical energy enters the actuator drive/module; negative when energy leaves the module and is returned toward the bus.
- mechanical_output: positive when the actuator delivers mechanical energy to the robot; negative when an overhauling load supplies mechanical energy into the actuator.
- thermal_dissipated: nonnegative energy dissipated out of the actuator control volume.
- delta_internal_stored: signed increase/decrease in declared internal stored energy, such as spring or rotating-inertia energy included inside the boundary.

```text
R_actuator =
    E_dc_terminal_in
  - E_mechanical_output
  - E_thermal_dissipated
  - Delta_E_internal_stored
```

### B. Shared DC bus

- external_source_in: signed energy delivered by the external source into the bus.
- drive_draw_signed: positive when a drive draws energy from the bus; negative when a drive returns energy to it.
- storage_draw_signed: positive when the bus delivers energy to storage; negative when storage supplies the bus.
- other_loads: nonnegative energy consumed by other bus loads.
- brake_dissipated: nonnegative energy dissipated by a braking chopper/resistor.
- delta_bus_cap: signed change in energy stored in the declared DC-link capacitance.

```text
R_bus =
    E_external_source_in
  - E_drive_draw_signed
  - E_storage_draw_signed
  - E_other_loads
  - E_brake_dissipated
  - Delta_E_bus_cap
```

### C. Storage system

- storage_terminal_in: positive when energy crosses the storage terminal into the storage system; negative when it is discharged.
- delta_stored_energy: signed change in energy stored internally.
- storage_losses: nonnegative internal storage/converter losses inside the declared storage-system boundary.

```text
R_storage =
    E_storage_terminal_in
  - Delta_E_stored_energy
  - E_storage_losses
```

The storage-terminal energy is not equal to energy retained internally. Losses remain a separate term. State-of-charge change is not converted into joules without a validated storage model and uncertainty.

## 4. Inter-boundary coupling: the extra system-level gate

The full-ledger API performs two additional comparisons after each control volume is assessed:

1. Actuator-module dc_terminal_in must reconcile with the shared-bus drive_draw_signed at the same physical DC port.
2. Shared-bus storage_draw_signed must reconcile with storage-system storage_terminal_in at the same physical storage port.

For two already expanded intervals A and B, the signed coupling residual is:

```text
R_coupling = A - B
interval(R) = [A.lower - B.upper, A.upper - B.lower]
```

Zero containment means the two boundary measurements are consistent with their declared intervals. The two control volumes intentionally have different system-boundary digests, but must share mission-interval, operating-condition and time-alignment digests.

A cable, connector or converter loss between two instrument locations must either be included in one defined boundary or represented as its own measured port. It must not be hidden by widening an interval after seeing the residual.

## 5. Uncertainty and fail-closed semantics

Each input term represents an **already-integrated** signed energy interval and carries:
- joule unit;
- deterministic validated-bound representation;
- absolute integration/quadrature error bound in joules and its source/digest;
- integration method and frozen integration-contract digest;
- exact start/end time and mission-interval digest;
- time-alignment, system-boundary and operating-condition digests;
- calibration status, traceability status and evidence class;
- source, measurement method, calibration digest and raw-trace digest.

The crate expands each term by its declared integration error. For a signed term, the interval is expanded symmetrically. For a physically nonnegative dissipative/load term, the expanded interval is intersected with its known nonnegative domain.

Unknown, expired or partial calibration/traceability; missing ports; missing integration bounds; mismatched mission/time/boundary/conditions; or unknown evidence produces Unknown, not zero. A non-finite/inverted interval, duplicate/unexpected port, wrong unit, invalid integration bound, or negative dissipative term is rejected.

The report returns:
- ConsistentWithDeclaredBounds when zero is contained by the residual interval;
- InconsistentWithDeclaredBounds when zero is excluded;
- Unknown when the balance cannot be evaluated from qualified terms.

Zero containment is only a consistency check. It is not proof of a correct measurement model, correct calibration, electrical recovery, battery acceptance or system safety. A very wide residual interval may contain zero while being practically uninformative, so interval width and residual relative to the largest term are reported separately.

## 6. Synthetic known-answer fixture

The file examples/dc-bus-energy-ledger-known-answer.json contains four synthetic cases. Each input energy has a deterministic integration error bound of 0.01 J.

| Case | Signed balance | Expected residual interval |
|---|---|---:|
| Actuator motoring | 100 − 80 − 15 − 5 | [−0.04, +0.04] J |
| Actuator overhauling | −60 − (−90) − 20 − 10 | [−0.04, +0.04] J |
| Shared DC bus | 150 − 100 − 20 − 15 − 15 − 0 | [−0.06, +0.06] J |
| Storage system | 20 − 18 − 2 | [−0.03, +0.03] J |

These cases use fabricated identities, sources, calibration and raw-trace digests. They are software known answers, not physical measurements.

The Rust test corpus also constructs a complete three-volume example and checks the two inter-boundary energy couplings. Negative controls change a drive-terminal energy or storage-terminal energy while keeping the local balance(s) closed, demonstrating that an inter-boundary mismatch is still detected.

## 7. Run

Dependency-free Rust crate, Rust 1.96:

```bash
cargo fmt --manifest-path benchmarks/synthetic-joints/dc-bus-energy-ledger/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/dc-bus-energy-ledger/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/dc-bus-energy-ledger/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

Dedicated workflow: .github/workflows/dc-bus-energy-ledger.yml.

Twenty-seven uniquely numbered Rust test definitions cover motoring/overhauling, shared-bus and storage balances; interval expansion; missing/extra/duplicate ports; units; calibration and traceability; mismatch in mission/time/boundary/conditions; nonnegative dissipation; provenance receipts; residual failure; complete-ledger coupling; and deliberate actuator/bus and bus/storage boundary mismatches.

The authoring environment used for this change has no local Cargo/rustc/rustfmt. Exact-head CI is the compilation, formatting and Clippy gate; no local Rust pass is claimed.

## 8. M0–M5 promotion

- **M0:** freeze signs, control-volume boundaries, port locations, mission intervals, integrated-energy methods and source/calibration records.
- **M1:** interval balances, coupling receipts and adversarial negative controls.
- **M2:** independent reconstruction from synchronized torque/speed/voltage/current traces; integration convergence and circuit/electromechanical cross-check.
- **M3:** guarded source/sink bench with bidirectional power instrumentation, DC-link voltage/current, storage-terminal and brake-resistor measurement.
- **M4:** multi-joint shared-bus missions, including simultaneous motoring/braking and held-out trajectories; report net energy per successful task.
- **M5:** storage aging, resistor/chopper thermal cycling, converter drift, cable/connector wear and post-service requalification.

## 9. Claim ceiling and safety

M1 verifies arithmetic and provenance for supplied, already-integrated energy intervals. It does not validate the integration pipeline, battery/SOC model, BMS charge acceptance, converter behavior, DC-link protection, braking-resistor sizing or physical energy recovery. It does not authorize energized hardware tests, intentional overvoltage, battery experimentation or human-contact testing. Physical work requires qualified electrical engineering review, independently verified protection, rated guarded fixtures, calibrated instrumentation and a separate hazard analysis.
