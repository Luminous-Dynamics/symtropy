# Typed Rock-Extraction Resource and Energy Ledger v0.1

**Parent issue:** #3825  
**Composes with:** #3812 / draft PR #3814; #3823/#3824 mechanism response; #3305 TBM ring records; #1933 mine campaigns; #3631 accepted lifecycle cost; #3591 metrology reproducibility  
**Status:** resource accounting contract and synthetic conformance fixture  
**Claim ceiling:** typed resource lineage, declared-boundary completeness and accounting eligibility only. This contract does not establish physical calibration, conversion efficiency, lifecycle cost, carbon intensity, exergy, machine performance or safe operation.

## 1. Why a resource vector is necessary

A single `total_energy_kwh` cannot describe the energy and consumable burdens of hybrid extraction. It can conceal electricity, fuel, hydraulic and pneumatic energy, optical delivery, mechanical work, cooling, ventilation, water, abrasive, gas, cryogen and downstream crushing.

Every resource observation therefore retains its native carrier, quantity, unit, uncertainty, time window, subsystem, physical boundary, source and evidence role. A normalized energy total is optional and is never a substitute for the native vector.

## 2. The accounting distinction

The ledger separates five different concepts:

1. **External boundary input:** energy or material entering the declared system boundary from outside it.
2. **Internal transfer:** energy/carrier passed between subsystems within the declared boundary.
3. **Process delivery:** energy delivered to the physical process (for example, optical energy at the workpiece or mechanical work at the cutter).
4. **Conversion loss:** measured or derived energy not delivered as useful output by a conversion stage.
5. **Recovery and waste:** returned water, recovered material, spent abrasive, exhaust, heat rejected or other outputs.

Only explicitly designated external energy inputs may contribute to a common-boundary energy total. Do not add source electricity and its optical, hydraulic, thermal or mechanical output a second time. A process-delivery or loss observation is diagnostic, not another external input.

A material resource (water, abrasive, cryogen, gas, coolant, chemical, filter media) remains in its native mass/volume/count basis. Do not silently turn litres of fuel or water into kWh, or kilogrammes of abrasive into energy. Any conversion to an energy-equivalent or lifecycle basis requires a versioned, sourced model with an explicit boundary and lineage.

## 3. Resource observations

Every observation carries:

- unique observation ID and physical-flow ID;
- resource class and type (energy carrier or material resource);
- physical dimension and native unit;
- positive quantity and standard uncertainty;
- flow role and accounting treatment;
- declared boundary and subsystem;
- start/end of its measurement window;
- energy basis when its dimension is energy;
- instrument/meter, calibration, evidence role and source artifact;
- derived lineage and conversion-model reference where relevant.

The `physical_flow_ref` joins different representations of the same physical flow. For example, 15 kWh grid electricity and a later optical-output observation can be linked through a conversion stage. If a fuel volume and an energy-equivalent derived from that same fuel flow both appear, they share the same physical-flow reference. The evaluator must never count both in the same aggregate.

### Energy basis

Energy observations declare one of:
- `FinalEnergyAtBoundary`;
- `PrimaryEnergy`;
- `Exergy`;
- `ToolInterfaceMechanicalWork`;
- `DeliveredProcessEnergy`;
- `EstimatedAbsorbedEnergy`.

These bases are not interchangeable. A common energy total can only be produced when the declared policy and all included observations use one compatible basis. A tool-interface-work total is not a facility-input total; optical delivery is not source electricity; estimated absorbed energy is not independently measured input.

### Accounting treatment

- `CountTowardCommonEnergyTotal`: only an external energy input with an explicit common basis.
- `NativeVectorOnly`: preserve the resource in the vector, but do not add it to the kWh total.
- `ReconciliationOnly`: check against other meters or balances; never add it to the total.
- `DoNotAggregate`: process delivery, internal transfer, conversion loss, recovered output or waste.

Two counted observations with the same `physical_flow_ref` are a double-counting error. Any observation marked as an internal transfer/process delivery/loss/recovery/waste must not count as an external input.

## 4. Conversion stages

A stage connects one or more input observations to measured/derived outputs and losses. The graph preserves the chain:

`external carrier -> conversion stage -> internal carrier/process delivery -> useful process / loss`

Examples:

- Electricity -> laser source -> optical output -> workpiece-plane incident optical energy.
- Electricity/fuel -> motor/pump -> hydraulic energy -> percussion tool interface.
- Grid electricity -> cutter drive -> measured mechanical work at cutter.
- Grid electricity -> crusher -> measured downstream comminution input.
- Water/abrasive -> jetting subsystem -> recovered or disposed water/abrasive streams.

A stage can document measured inputs and outputs without assuming a fixed efficiency or forcing energy-balance closure where measurements are incomplete. A missing loss term must remain missing/unknown; it must not be invented to make the balance close.

Each conversion stage retains its method/evidence role and optional versioned conversion-model reference. Estimated, simulated and operator-reported conversions are never relabelled as independent measurements.

## 5. Boundary, subsystem inventory and time

The ledger binds to an existing full-cycle trial and comparison profile; it does not create a new accepted-output authority. Its declared boundary has:
- boundary ID and scope (`MachineInput`, `FacilityInput`, `DeclaredSystem`, `FullCycle`);
- exact start/end time for the trial window;
- included subsystem inventory;
- for each subsystem, whether an energy input is expected and which native resource categories are expected;
- an explicit reason when an expected subsystem has zero input or is intentionally inactive.

Every observation belongs to an included subsystem and lies within the declared time window. The external energy observations used for a common total must cover the same full-cycle interval as the comparison. A pump, cooling loop, laser, ventilation fan, crusher or extraction conveyor cannot disappear merely because its meter was omitted.

A zero-input subsystem needs an explicit reason. “No meter available” is not a zero-input reason; it makes the comparison incomplete and should prevent a complete total.

## 6. Aggregation policy

The default is `NativeVectorOnly`: retain the vector and do not compute one total.

`CommonEnergyInputBasis` permits an energy total only if:
- at least one eligible external energy input exists;
- all counted observations have energy dimension;
- all counted observations use the declared common energy basis;
- their units are convertible by exact unit scaling (J, kJ, MJ, kWh);
- every expected subsystem is accounted for by an input observation or explicit zero-input reason;
- no physical-flow reference is counted twice;
- all observations share the same comparison window and boundary.

This unit conversion is not a primary-energy, carbon, lifecycle or exergy conversion. Such normalizations require separate versioned factors and are outside v0.1. If any condition fails, the evaluator must abstain from emitting a single total while preserving the native vector and reasons.

## 7. Method-specific accounting examples

| Method | External inputs to retain | Internal/process observations not added again |
|---|---|---|
| ATCP | site electricity/fuel, pump and fluid inputs, cooling, ventilation, extraction | hydraulic/fluid power, actual axial/torsional tool work |
| Laser-PDC | facility/source electricity, cooling, ventilation, extraction | optical output, workpiece-plane incident energy, absorbed-energy estimate, cutter work |
| ODC | machine drive, auxiliary energy, extraction, measured downstream crusher input | spindle work and tool-interface work when drive input already covers that stage |
| Waterjet | pump electricity/fuel, water, abrasive, treatment inputs | delivered jet energy and recovered/disposed streams |
| Plasma/pulse | source electricity, pump/fluid/gas/cooling/ventilation | delivered discharge/plasma energy and process uptake |
| Cutting-and-splitting | saw electricity/fuel, wedge/hydraulic power, handling and dust/water | wedge work, kerf/cut work, split energy |

## 8. B0 semantic gates

The first implementation must reject:
- duplicate observation or conversion-stage IDs;
- dangling references or cyclic conversion graphs;
- a process delivery, internal transfer or loss marked as an external aggregate input;
- a counted external input without energy dimension, unit or energy basis;
- multiple counted records for the same physical flow;
- a common total with incompatible bases or dimensions;
- incompatible time windows or observations outside the declared full-cycle window;
- missing expected subsystem energy input without an explicit zero-input reason;
- a counted facility meter and its counted subsystem submeters for the same physical flow;
- unaccounted subsystem/resource inventory silently presented as complete.

Synthetic conformance records prove only the validator contract, never physical performance or energy efficiency.

## 9. Acceptance and next validation levels

- 100% frozen synthetic valid/invalid outcomes.
- Zero false acceptance for double-counting, omitted subsystem, incompatible unit/basis and time-window faults.
- Deterministic ordering and exact replay for frozen inputs/evaluator revision.
- Preserve all native resources if a normalized total is unavailable.
- B1 physical metrology: calibrated meters and independent measurement of energy/resource quantities.
- B2 integrated equipment: all auxiliaries and material recovery instrumented.
- B3 independent site campaign: full-cycle input/output and lifecycle-cost evidence.

Lifecycle cost remains owned by #3631; TBM-ring and mine-campaign accounts remain owned by #3305/#1933. This ledger is the typed input/evidence layer, not a replacement for those owners.

## 10. Safety and claim boundary

This is an accounting and data contract. It does not specify how to operate lasers, high-voltage pulses, percussion equipment, pressure systems, crushers, cryogenic equipment or underground machinery. No physical equipment control or safety authority is created.

## 11. References and integration

- Full-cycle comparison contract: #3812 / PR #3814.
- Mechanism-response records: #3823 / PR #3824.
- Resource ledger implementation issue: #3825.
- Setpoint versus measured operating point: #3839.
- Cost per accepted work: #3631.
- Metrology round-robin: #3591.
- TBM ring performance: #3305.
- Mine campaign material/energy accounting: #1933.
