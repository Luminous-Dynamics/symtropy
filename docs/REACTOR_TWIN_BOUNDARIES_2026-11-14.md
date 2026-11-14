# Reactor Twin: Solver, Energy and Evidence Boundaries — 2026-11-14

**Status:** Integration contract and capability boundary; no reactor solver or physical twin claimed

## Executive direction

Symtropy should become the deterministic scenario, energy-boundary and evidence-replay substrate for reactor research—not a pretend fusion, MHD, CFD or neutron-transport solver.

The existing public code already provides useful foundations, but its validity envelope must remain explicit.

## Existing capability and limit

### 1. Rigid-body physics

symtropy-physics provides N-dimensional rigid-body mechanics, collision handling, replay and invariant snapshots. Its RESEARCH_VALIDATION.md explicitly lists production fluid, deformable, fracture, FEM and MPM simulation as not certified.

It is not a reactor thermal-hydraulics, plasma, structural stress or neutronics solver.

### 2. Fluid module

symtropy-fluid contains an SPH particle representation, but its source contains placeholder gradient/Laplacian calculations. It must remain experimental until it has:
- analytical/manufactured-solution tests;
- conservation and convergence studies;
- independent reference comparisons;
- adversarial boundary and stability cases;
- explicit validity envelope and calibrated uncertainty.

It must not currently provide design-grade coolant-flow, heat-exchanger or plasma-flow results.

### 3. Energy-system bookkeeping

symtropy-energy-systems is a deterministic 0D fuel-to-engine-to-generator bookkeeping model. It has typed quantities and requires explicitly supplied inputs rather than inventing unknown physical values. That is a useful pattern, but the model is not a fusion plant balance.

For fusion, reuse the first-law/energy-port work in [PHYS-ENERGY-PORTS-001 #3615](https://github.com/Luminous-Dynamics/symtropy/issues/3615). Extend the domain-specific adapter only; do not create a second generic ledger.

### 4. Plasma and external solver integration

[PLASMA-TWIN-001 #1685](https://github.com/Luminous-Dynamics/symtropy/issues/1685) already defines a solver-neutral plasma scenario adapter. It includes a distinct fusion regime profile and correctly keeps specialist solvers responsible for their supported equations/results.

The scenario adapter should bind frozen inputs, solver capability, version, boundary conditions, raw artifacts, required observables, warnings, convergence status, uncertainty and output disposition. An integrity PASS is not a scientific-validity PASS.

## Proposed reactor run contract

A versioned ReactorScenario/RunReceipt should include:
- reactor architecture and physical regime identifier;
- fuel/reaction class without implying an unproven reaction mechanism;
- exact scenario/configuration and geometry hashes;
- specialist solver identifier, version/commit, model options, capability scope and known omissions;
- source datasets, nuclear/material data libraries, licence/rights state and content hashes;
- initial/boundary conditions, units, reference frames and sampling support;
- convergence, residuals, conservation diagnostics and solver warnings;
- raw result artifact references, canonical output digest and environment identity;
- model discrepancy and uncertainty records;
- status: measured, externally reported, modelled, extrapolated, unsupported, non-converged, incomplete, failed or unknown;
- claim ceiling: which decisions the receipt is and is not allowed to support.

Do not create a plasma PDE branch inside symtropy-physics merely because the scenario is rendered in Bevy. Do not use an internal rigid-body or placeholder SPH result as a substitute for a specialist solver.

## Fusion energy boundaries

For a D–D, D–T, inertial or magnetic-confinement system, declare the system boundary and report each metric independently:

1. driver or source wall-plug input;
2. energy delivered to the target or plasma;
3. fusion/reaction energy released;
4. energy deposited as heat or carried by escaping particles/radiation;
5. gross thermal energy recovered;
6. gross electrical generation;
7. auxiliary and recirculating loads;
8. net electricity exported;
9. change in stored energy within the selected boundary.

Nested boundaries must be explicit so internal energy transfers are not double-counted. Heat remaining in a modeled reservoir is not already external heat loss. A device's reaction yield cannot be used as electrical output.

Maintain separate names for target gain, plasma gain, driver wall-plug gain and net plant electric output. A net-power claim needs a complete facility boundary, calibrated measurements and uncertainty bounds that leave net export positive.

Use the first-law residual with the declared sign convention:
- all energy crossing into the selected boundary;
- all energy crossing out;
- change in stored energy over the observation interval.

Unknown or unmetered flows prevent a closed energy claim. Do not silently fill missing parasitic loads with zero.

## Material and manufacturing interface

The reactor twin should consume, not duplicate, qualified material/joining evidence:
- Symthaea's atmospheric-plasma surface activation and bondability work (#11780, #11795, #11907) is an adjacent manufacturing process programme, not fusion plasma physics.
- [SHIPYARD-WELD-PHYSICS-VVUQ-001 #4614](https://github.com/Luminous-Dynamics/symtropy/issues/4614) is a useful pattern for thermo-mechanical model verification, validation and uncertainty qualification.
- Material/sample lineage, heat treatment, joining state, inspection and repair should be explicit evidence inputs.

No manufacturing release follows from a digital-twin projection. Joining/structural models must be qualified for their actual material, geometry, process and measurement envelope.

## Proposed benchmark suite

| Benchmark | Required comparison | Acceptance target |
|---|---|---|
| Receipt integrity | Frozen inputs, solver version, output hashes | All altered/stale/missing artifacts rejected |
| Unit and frame checks | Deliberate dimension/frame mismatches | 100% of invalid inputs rejected |
| Conservation | Analytical known-answer energy/mass fixtures | Residual within preregistered tolerance |
| Energy boundary closure | Complete nested input/output/store ledger | No unclassified crossing; unknown input blocks closure |
| Solver capability | In-profile vs out-of-profile cases | Unsupported cases cannot produce a success disposition |
| Reproducibility | Frozen repeat runs and declared nondeterminism | All attempted runs receive explicit status |
| Model validation | External reference and held-out conditions | Error, bias, uncertainty coverage and failure counts published |
| Model transfer | Changed material, geometry, solver or operating envelope | Transfer rejected absent bridge evidence |
| Claim boundary | Model output supplied as measurement/qualification | Type and status checks reject claim promotion |

Numerical tolerances must be set per physical problem and reference dataset, not copied as a universal percentage.

## Immediate implementation sequence

1. Reuse #1685 for the solver-neutral plasma/fusion scenario envelope.
2. Reuse #3615 for typed energy ports and instrumented first-law accounting.
3. Add the smallest fusion-specific energy adapter and exact system-boundary definitions.
4. Implement receipt/schema and refusal tests before connecting a solver.
5. Select specialist solvers only after the target regime, equations, available reference datasets and licensing are identified.
6. Do not promote symtropy-fluid or rigid-body capabilities to reactor engineering until their own validation suites pass.

## Non-goals and safety boundary

This work does not provide reactor operating procedures, neutron-source construction instructions, radiation-shielding designs, high-voltage/laser/power-supply operating settings, or hardware control authority. Physical reactor work requires qualified specialists, licensed facilities, independent safety review and applicable authorization.

**Claim boundary:** A deterministic replay or complete receipt proves only that the recorded scenario and artifacts are internally consistent. It does not prove physical correctness, net energy, safety, manufacturability, or operational readiness.
