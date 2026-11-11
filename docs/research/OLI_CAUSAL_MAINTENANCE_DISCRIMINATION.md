# OLI causal maintenance discrimination control v0

**Status:** Rust reference and frozen cases added; exact-head validation pending  
**Owner:** Symtropy / `symtropy-lifesim-core`  
**Related:** [OLI experiment-selection issue #11446 in Symthaea](https://github.com/Luminous-Dynamics/symthaea/issues/11446) and [Symtropy abiotic tournament #4572](https://github.com/Luminous-Dynamics/symtropy/issues/4572)  
**Manifest:** `benchmarks/oli/causal-maintenance-control-v0.json`

## Why this is a paradigm shift

A static pattern classifier can mistake persistence, oscillation or local order for life-like behavior. This benchmark instead asks a causal question:

> After the same controlled perturbation, does a system restore a defined organizational variable, by what mechanism, at what energy cost, and does the effect disappear when the relevant controller or energy source is removed?

This follows the OLI classify-last / discriminate-first principle. It turns self-maintenance into an operational property with rival mechanisms and interventions, rather than a visual impression or model-generated score.

## Three known mechanisms

The implementation is deliberately small and fully specified:

1. **Passive relaxation:** integrity loses a fraction through an exact first-order decay law; there is no repair actuator.
2. **Fixed-rate repair:** a constant repair effort is applied independently of the observed integrity error, subject to target headroom, actuator limit and available energy.
3. **Proportional feedback:** repair request scales with the measured error from the target, again subject to actuator and energy limits.

All three share the same state variable, disturbance sequence, energy accounting and environmental input. This creates a matched causal comparison.

The model is not a biophysical organism model. It does not implement reproduction, heredity, evolution, memory, learning, agency or consciousness.

## Implementation

File: `crates/domains/symtropy-lifesim-core/src/maintenance.rs`

The bounded state is integrity (x\in[0,1]). Passive decay is exact for the declared first-order law:

[
x_{decay}=x_t e^{-k\Delta t}.
]

A disturbance applies an instantaneous, explicitly supplied damage value. The energy store receives an environment-dependent harvest, subject to capacity. The controller requests repair according to its declared mode; actual repair is bounded by:
- target headroom;
- maximum repair rate;
- energy available divided by energy required per unit repair.

The update report preserves:
- controller mode and environmental energy availability for that exact tick;
- integrity before and after;
- passive decay loss and applied disturbance;
- requested, applied and unmet repair;
- gross harvested energy, spill, energy used and remaining energy.

The ledger invariant is:

[
E_{before}+E_{harvest}=E_{after}+E_{used}+E_{spilled}.
]

Update order is fixed in source and part of the benchmark replay contract.

## Pre-registered interventions

The manifest includes:
- **Sham:** no disturbance; feedback should not expend repair energy.
- **Matched disturbance:** passive and feedback modes receive the same damage and energy availability.
- **Dose response:** fixed-rate and proportional controllers receive small and large perturbations; fixed-rate request remains constant while feedback request scales with error until common clipping.
- **Controller knockout:** switch feedback to passive after damage; repair must cease.
- **Energy cutoff:** empty energy store plus zero intake; repair request can remain positive but applied repair must be zero.
- **Finite reserve depletion:** initial stored energy can support temporary repair, but not indefinite restoration without intake.
- **Unseen perturbation sequence:** assess robustness of the declared controller under a pre-registered varied sequence without calling it learning.

## What this distinguishes—and what it does not

| Observation | Supports | Does not establish |
|---|---|---|
| State persists without input | Persistence under tested conditions | Active self-maintenance |
| State recovers after damage, consumes energy | Energy-coupled repair mechanism | Biological life |
| Recovery disappears after controller knockout | Causal dependence on controller | Agency or intelligence |
| Recovery stops when energy is exhausted | Resource-bounded maintenance | Metabolism in a biological sense |
| Feedback scales effort to error | Error-responsive control | Learning or adaptation |
| Generalizes to a fixed unseen perturbation sequence | Robustness within the declared control law | Open-ended learning |

This separation is essential: the benchmark must not silently promote persistence to self-maintenance, repair to life, or feedback to intelligence.

## Industry/SOTA comparison

The benchmark is not intended to beat a generic classification leaderboard. Its comparators are control-theoretic and causal:
- passive relaxation/no-control baseline;
- fixed-rate actuator baseline;
- proportional feedback baseline;
- controller-knockout intervention;
- energy-starvation intervention;
- exact model equations as a bounded reference oracle.

The novel contribution is the combined, machine-readable evidence contract: matched perturbation, energy supply, work, state trajectory, controller interventions, and a separate claim ceiling. A future, more realistic physical model can replace the scalar state without changing the causal question or evidence structure.

## Qualification targets (proposed, not achieved)

- Absolute energy-ledger residual (leq10^{-12}) for the f64 reference.
- Exactly zero applied repair in passive mode.
- Exactly zero feedback repair at zero integrity deficit.
- Exactly zero applied repair when the energy store is empty and energy intake is zero.
- Zero repair after controller knockout in the passive mode.
- Exact deterministic replay on the same supported platform.
- Every comparison uses matched exogenous disturbance and energy-input sequences.
- No train/test split across time steps from one trajectory.
- Zero ontology promotions from persistence, repair or feedback alone to life, learning, agency, intelligence or consciousness.

## Next scientific step

After this synthetic reference passes exact-head validation, connect it to a more realistic physical model in Symtropy: damage and repair of a spatially distributed network or membrane-like boundary, with mass/energy throughput, transport delays, sensor noise, actuator failure and spatially localized perturbations. That model should be compared with the same passive, fixed-control and feedback interventions.

The scalar control is the executable ground-truth kernel; it is not the endpoint.
