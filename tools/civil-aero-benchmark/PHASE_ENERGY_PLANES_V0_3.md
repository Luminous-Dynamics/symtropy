# AER-01 Carrier-Aware Phase Energy Planes v0.3

**Parent:** [AERO-POWERTRAIN-TOPOLOGY-COVERAGE-001 #4047](https://github.com/Luminous-Dynamics/symtropy/issues/4047)  
**Energy-plane contract:** [#4036](https://github.com/Luminous-Dynamics/symtropy/issues/4036)  
**v0.2 implementation:** [PR #4040](https://github.com/Luminous-Dynamics/symtropy/pull/4040)  
**Mission chronology:** [#4029 / PR #4030](https://github.com/Luminous-Dynamics/symtropy/pull/4030)  
**Status:** synthetic arithmetic/contract qualification; no aircraft or component performance is validated.

## Why v0.3 exists

The v0.2 contract was correctly scoped around battery-electric and hybrid-electric bus/shaft accounting. Its initial evaluator required an electric motor record, so a conventional piston/turboprop direct-drive path could not use the exact same energy accounting boundary. A fuel-cell data type was also insufficient until fuel use, electrical bus output, component power limits and fuel-tank closure were all wired into the same independent accounting path.

v0.3 adds:
- conventional direct mechanical drive from a fuel-consuming prime mover to the propulsor shaft, without a fictitious electrical bus or motor;
- fuel-cell chemical input -> electrical bus output + declared thermal loss;
- fuel-cell + battery-buffer hybrid path;
- shared fuel-tank closure across all prime movers and fuel-cell converters;
- uniqueness and hostile controls for fuel-cell identities and path accounting.

The v0.2 schema, six-case fixture and v0.2 oracle remain separately versioned and unchanged.

## Typed paths

### Conventional direct drive

`fuel chemical energy -> prime mover mechanical output -> direct propulsor shaft -> useful propulsive work`

The prime-mover shaft balance includes generator mechanical inputs (if any), direct propulsor shaft contribution, accessory mechanical load and mechanical distribution loss. A conventional direct-drive fixture has no motor or generator; electrical bus supply/demand remains zero. A shaft output with no declared path is infeasible.

### Fuel-cell electric

`fuel chemical energy -> fuel cell -> electrical bus -> motor/drive -> shaft -> propulsor`

Fuel-cell chemical input is not bus energy. The conversion residual is recomputed as fuel chemical input minus electrical output to the bus minus declared thermal loss. Fuel-cell power ratings and exact model evidence digest are independently checked.

### Fuel-cell + battery buffer

Fuel-cell electrical output and battery discharge are separate bus sources. Battery internal energy changes using its own charge/discharge efficiencies; it does not equal bus energy. The sum of all converter fuel inputs sharing a fuel-tank ID must reconcile to that fuel source's chemical-energy consumption.

## Independent fixture set

Nine synthetic positive topologies:
1. battery-only;
2. generator-only electric propulsion;
3. series hybrid with battery buffering;
4. parallel hybrid with mechanical and electric shaft inputs;
5. generator charging a battery;
6. generator + battery + supercapacitor;
7. conventional direct-drive;
8. fuel-cell-only electric propulsion;
9. fuel-cell + battery buffer.

The Python standard-library oracle independently recomputes fuel, prime-mover, generator, fuel-cell, energy-store, electrical-bus, motor, shaft-combiner and propulsor balances. Negative controls include residual errors, bus double counting, prime-mover/generator/fuel-cell power limits, storage reserve/temperature/power limits, fuel-cell shared-tank closure, direct-drive orphaned shaft output, peak/continuous bus limits and stale identity.

## Claim ceiling

All values, model/source digests and operating envelopes in these fixtures are synthetic. They qualify arithmetic and fail-closed software semantics only. They do not validate an engine, generator, hydrogen/fuel-cell system, battery, motor, inverter, propulsor, thermal system, aircraft-level mission performance, reserve policy, safety, airworthiness, certification or flight permission.

The real fuel-cell, storage, prime-mover and propulsor model maps remain owned by their respective domain work packages. Hybrid dispatch and start/ramp scheduling remain owned by #3900; installed storage evidence remains owned by #3903. The energy-plane contract is an accounting seam, not a replacement for those models.
