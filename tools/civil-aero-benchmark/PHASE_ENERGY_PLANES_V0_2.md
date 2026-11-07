# AER-01 Carrier-Aware Phase Energy Planes v0.2

**Parent:** [AERO-PHASE-ENERGY-PLANES-001 #4036](https://github.com/Luminous-Dynamics/symtropy/issues/4036)  
**Mission chronology:** [#4029](https://github.com/Luminous-Dynamics/symtropy/issues/4029) / [PR #4030](https://github.com/Luminous-Dynamics/symtropy/pull/4030)  
**Hybrid dispatch owner:** [#3900](https://github.com/Luminous-Dynamics/symtropy/issues/3900)  
**Installed battery/energy-store evidence owner:** [#3903](https://github.com/Luminous-Dynamics/symtropy/issues/3903)  
**Status:** synthetic arithmetic/contract qualification; all outputs screening-only

## 1. Why v0.2 exists

Mission chronology v0.1 was intentionally a restricted battery-only arithmetic pilot. It used a single storage state decrement equal to a single bus source-energy number. That is not a general hybrid-energy relationship: the bus can receive both generator output and store discharge, and internal store energy changes by more than or less than bus energy because of charge/discharge conversion losses.

v0.2 adds a separate carrier-aware contract. It does not silently reinterpret v0.1 data and does not replace #3900's hybrid dispatch or #3903's installed energy-store evidence.

## 2. Energy planes and conservation boundaries

The model keeps these planes separate:

1. **Fuel chemical energy:** chemical energy removed from a named fuel source. Its source digest and declared reserve are kept separate from electrical energy.
2. **Prime-mover mechanical output:** fuel input becomes mechanical output plus declared thermal/exhaust loss. Mechanical output is then allocated to generator input, direct propulsor shaft output, accessories and mechanical distribution loss.
3. **Generator electrical output:** generator mechanical input becomes electrical bus output plus generator conversion loss.
4. **Electrical bus:** generator electrical output plus all store discharge delivered to bus must equal motor-drive input, auxiliary loads, thermal-management electrical loads, storage charging drawn from bus and distribution losses.
5. **Energy-store internal state:** each battery/supercapacitor has its own start/end usable energy, bus discharge/charge, efficiency, reserve floor, capacity, state of health, temperature and charge/discharge power limits.
6. **Motor/drive:** bus input equals inverter loss, motor loss and shaft output.
7. **Propulsor shaft/useful work:** shaft input equals useful propulsive energy plus propulsor loss.

For each store in a lumped phase model:

`E_{end}=E_{start}-E_{discharge,bus}/\eta_{discharge}+E_{charge,bus}\eta_{charge}`

The store's internal energy change is not interchangeable with its bus-terminal energy. Fuel chemical energy and battery usable energy are not summed into an unqualified “remaining energy” number.

Thermal electrical demand (fans, pumps, controls) is recorded as a bus load. Rejected heat is a different physical quantity and is not added as a second electrical load.

## 3. Rust implementation

`src/phase_energy_planes.rs` exposes `PhaseEnergyBalance::evaluate`. It recomputes residuals rather than accepting submitted residual numbers.

It validates:
- exact mission, configuration, start-state and end-state identities;
- source/model evidence digests for fuel, prime mover, generator, motor and propulsor;
- installed-system and operating-envelope digests for every energy store;
- unique component/store identities and valid component-to-source relationships;
- fuel state and prime-mover conversion closure;
- prime-mover shaft allocation into generator(s), direct shaft, accessories and mechanical losses;
- generator mechanical-to-electrical conversion and component average/peak power limits;
- per-store charge/discharge efficiency, internal energy transition, capacity, reserve floor, SOH/temperature envelope, average/peak charge and discharge limits;
- bus supply/demand conservation including storage charging as a bus load;
- motor, shaft-combiner and propulsor conservation;
- aggregate bus average/peak power margins and trace identity.

The aggregate peak bus supply is an upstream dispatch/measurement result, not derived from a phase-average energy total. It carries a SHA-256 trace identity. This crate does not fetch or authenticate that trace; source admission and exact #3900 dispatch trace generation remain separate.

## 4. Power boundaries

Do not call every power number “source power”.

- Prime mover average/peak mechanical output has its own limits.
- Generator average/peak electrical bus output has its own limits.
- Each storage unit has independent average/peak charge and discharge limits.
- The aggregate electrical bus has its own continuous/peak supply limit and peak trace digest.

A short-duration source peak is not treated as a continuous rating. Energy divided by phase duration provides an average; it is not enough to reconstruct a true peak waveform.

## 5. Synthetic fixture corpus

`data/mission/phase_energy_planes_fixtures_v0_2.json` contains six positive known-answer configurations:

1. Battery-only discharge.
2. Generator-only electric propulsion.
3. Series hybrid with a battery peak buffer and concurrent charge/discharge accounting.
4. Parallel hybrid with direct engine shaft contribution plus a distinct electric bus path.
5. Generator charging a battery while supplying the traction motor.
6. Generator + battery + supercapacitor with distinct store states, efficiencies, reserve floors and power limits.

The Python standard-library oracle in `scripts/verify-phase-energy-planes.py` independently recomputes fuel, engine, generator, store, bus, motor, shaft and propulsor balances. Its hostile controls include internal storage-energy mismatch, bus/generator double count, reserve-floor violation, temperature out of domain, storage peak-power violation, generator peak-power violation and prime-mover peak-power violation.

The schema `schemas/phase-energy-balance-v0.2.schema.json` declares the v0.2 data shape. The workflow parses the schema and fixture JSON and runs the independent oracle. JSON parsing alone is not a full JSON Schema validation; the Python oracle checks the finite v0.2 fixture semantics. A production validator and wider schema corpus remain future work.

## 6. Explicit limitations

This module is an energy-accounting contract, not a component physics model.

- Generator, prime-mover, inverter, motor and propulsor efficiencies are represented by supplied component input/output energy and evidence identities; maps are not computed here.
- The upstream #3900 owner remains responsible for fuel flow, generator dispatch, start/ramp, source scheduling and time-resolved power allocation.
- The #3903 owner remains responsible for installed battery/capacitor capacity, SOC/SOH/temperature derating, charge acceptance and component-specific limits. Supplied usable capacity must already have a declared interpretation; do not apply SOH/temperature derating twice.
- Store temperature and SOH are phase-level values, not a transient electrochemical thermal model.
- Fuel energy is already expressed as a source-qualified chemical-energy value. This module does not calculate fuel mass × lower heating value or authenticate fuel properties.
- The current implementation requires at least one electric motor-drive and propulsor record. It covers electric and hybrid-electric power paths, not a standalone conventional direct-drive aircraft; the conventional aircraft remains a separate reference path.
- The aggregate peak bus supply number and per-component peak numbers require their supplied model/trace evidence; the Rust module does not reconstruct a power waveform.

## 7. Acceptance targets

Internal software targets, not aviation standards or achieved aircraft performance:
- 100% agreement with the independent oracle for the locked synthetic corpus.
- Zero false-feasible outcomes in the synthetic hostile controls.
- Separate reported fuel, prime-mover, generator, storage, bus, motor, shaft and propulsor residuals.
- Exact mission/configuration/state/component evidence identities carried through.
- No aggregate reserve that treats fuel chemical energy and electrical storage as interchangeable.
- Component and bus average/peak ratings kept at their correct physical boundary.
- At least three independent mission families and real installed-system source evidence before making generalized performance claims.

## 8. Claim ceiling

All v0.2 values and evidence identities are synthetic. A passing test qualifies arithmetic and finite software semantics only. It does not validate an engine, generator, battery, motor, inverter, propulsor, thermal system, aircraft energy reserve, aircraft performance, safety, airworthiness, certification or flight permission.