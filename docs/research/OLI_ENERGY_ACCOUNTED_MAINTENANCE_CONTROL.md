# OLI abiotic control: energy-accounted feedback maintenance v0

**Status:** deterministic Rust reference and unit tests added; exact-head validation pending  
**Owner:** Symtropy / `symtropy-lifesim-core`  
**Machine-readable case:** `benchmarks/oli/maintenance-thermostat-v0.json`

## Scientific question

Can an engineered, non-biological feedback controller produce a homeostasis-like recovery curve? Yes. This is a deliberate abiotic false-positive control for OLI.

The scalar compartment has an integrity state, a finite energy store and a fixed feedback rule. Damage reduces integrity. If sensing and control are enabled, repair restores integrity toward a declared setpoint, subject to efficiency, power and available energy. Energy can be harvested from an explicit external input, stored up to capacity, spilled, or consumed by idle operation and repair.

The model has:
- no reproduction, heredity or evolution;
- no task learning or adaptive parameter updates;
- no generalization beyond its fixed scalar control rule;
- no autonomous goal formation or agency mechanism;
- no biological metabolism or cellular boundary.

A system can exhibit regulated recovery because it is engineered to do so. A recovery curve, negative feedback loop or setpoint regulation is not sufficient evidence of biological life, cognition or intelligence.

## Model and accounting

Module: `symtropy_lifesim_core::maintenance`.

The integrity balance is:
[
I_{t+1}=I_t-D_t+R_t,
]
where (D_t) is the applied damage and (R_t) the repair.

The energy balance is:
[
E_t+H_t=S_t+P_t+W_t+E_{t+1},
]
where:
- (H_t): externally harvested energy;
- (S_t): energy spilled because storage is full;
- (P_t): idle/controller energy use;
- (W_t): repair energy consumed.

Repair is bounded by all of:
1. the integrity deficit to the declared setpoint;
2. repair efficiency and energy-per-integrity cost;
3. maximum repair power;
4. energy remaining after spill and idle draw;
5. controller and sensor availability.

All intervention inputs are explicit. The system never creates energy to repair a damaged state.

## Frozen controls

1. **Active feedback recovery:** integrity 0.5, energy 10, damage 0.2. The controller repairs 0.7 integrity using 0.7 energy and returns the state to 1.0.
2. **Controller disabled:** identical damage and energy, but repair is zero and integrity ends at 0.3.
3. **Sensor unavailable:** identical damage; repair is zero even though energy and controller remain available.
4. **Energy depleted:** identical damage; repair is zero and stored energy remains zero.
5. **Harvest refill:** an initially depleted controller receives explicit harvested energy and can subsequently repair; the source is recorded.
6. **Full ledger:** a test includes energy harvest, storage spill, idle draw and repair in the same step; all energy flows reconcile.

These are mechanism tests, not evidence that a natural organism follows this exact scalar model.

## Paradigm shift: causal qualification, not curve fitting

OLI should not merely observe that a system returns toward a setpoint. It should intervene on the causal dependencies that make recovery possible:

- disable the controller;
- disconnect the sensor;
- deplete the energy store;
- withhold external energy harvesting;
- change the disturbance magnitude;
- vary repair power and efficiency;
- compare against a passive or fixed-dynamics baseline.

A causal recovery claim must identify which intervention changes the outcome, and the energy ledger must show where the work came from.

The test supports a narrow statement: this declared fixed feedback mechanism restores a declared scalar state under the registered conditions. It does not support a claim of biological self-maintenance or life.

## Benchmark comparisons

Compare on identical disturbances and resource budgets:
- no-controller passive baseline;
- sensor-disabled controller;
- controller-disabled system;
- energy-depleted controller;
- finite-energy feedback controller;
- ideal bounded repair oracle (future analytical baseline).

Report separately:
- recovery fraction and latency;
- energy cost per integrity restored;
- time to reserve depletion;
- energy-balance residual;
- integrity-balance residual;
- effect of sensor/controller/energy ablations;
- false ontology promotion count.

Do not optimize a single score that rewards recovery while hiding energy use, or that calls a thermostat-like controller alive because it maintains a state.

## Qualification targets (proposed, not achieved)

- Absolute energy-balance residual ≤ (10^{-12}) for frozen f64 cases.
- Absolute integrity-balance residual ≤ (10^{-12}).
- Zero negative energy and zero integrity outside [0,1].
- Zero repair when controller is disabled, sensor is unavailable or energy is depleted.
- Every repair joule linked to explicit energy storage or harvest.
- Zero promotion from this control to biological life, reproduction, learning, agency or intelligence.

## Claim ceiling

This is a synthetic known-mechanism engineered feedback control. It demonstrates that regulation and recovery can arise in non-biological systems. It is not a validated physiological, biochemical, ecological or spacecraft model.
