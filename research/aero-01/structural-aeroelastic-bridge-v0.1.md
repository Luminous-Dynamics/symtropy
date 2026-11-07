# AER-01 Structural / Aeroelastic Evidence Bridge

**Status:** Research contract and benchmark protocol; not a structural solver or aircraft validation result.  
**Tracking:** [AERO-STRUCTURAL-AEROELASTIC-BRIDGE-001, Symtropy #4093](https://github.com/Luminous-Dynamics/symtropy/issues/4093)  
**Version:** 0.1  
**Date:** 2026-11-07  
**Parent programme:** [AER-01 architecture brief](README.md)

## 1. Purpose

Define how structural loads, deformation, aeroelasticity and durability evidence enter and leave AER-01 whole-aircraft closure. Each result must be reproducible for the exact aircraft configuration, geometry, loading state, mission phase, propulsion/control state and environment that produced it.

The research question is:

> Can the same evidence-bound aircraft state be used to calculate aerodynamic loads, structural response and aeroelastic feedback, then independently replay the resulting effect on trim, power, mission capability and architecture choice?

This work does not create a second FEM, CFD, aeroelastic or certification engine. Existing specialist solvers remain independently versioned backends. Symtropy owns the state/evidence contract, coupling orchestration, budgets, admission policy and independent replay.

## 2. Ownership and integration boundaries

| Existing owner | Responsibility | Structural bridge integration |
|---|---|---|
| [#3921 AER-01](https://github.com/Luminous-Dynamics/symtropy/issues/3921) | Aircraft families, configurations, mission cards | Candidate and mission identity |
| [#3945 whole-aircraft closure](https://github.com/Luminous-Dynamics/symtropy/issues/3945) | Coupled aircraft-level feasibility | Structural result inputs, outputs and residuals |
| [#4000 mass evidence](https://github.com/Luminous-Dynamics/symtropy/pull/4000) | Source-bound mass and loading states | Exact distributed mass and inertia |
| [#4029 mission chronology](https://github.com/Luminous-Dynamics/symtropy/issues/4029) | Phase/state transitions | Load-case operating point and state digest |
| [#4055 runway performance](https://github.com/Luminous-Dynamics/symtropy/issues/4055) | Takeoff, landing, hot/high and blown-lift performance | Ground/flight loads and deformation feedback |
| [#4076 flight-dynamics bridge](https://github.com/Luminous-Dynamics/symtropy/issues/4076) | Trim, local stability and degraded control | Force/moment, control and propulsor-state consistency |
| [#3237 MDO interoperability](https://github.com/Luminous-Dynamics/symtropy/issues/3237) | External solver execution and MDO benchmark | Structural/aeroelastic adapters and full-cost accounting |
| [Symthaea #6249](https://github.com/Luminous-Dynamics/symthaea/issues/6249) | Fixed-wing aero, flight dynamics, loads and qualification semantics | Source of flight-dynamics/load/aeroelastic references |

No owner may replace a missing structural result with zero mass, zero deflection, zero load or an implicit pass. A missing or inapplicable discipline result is unresolved.

## 3. Paradigm shift: structural response is part of the aircraft state

The analysis must not stop at a one-way aerodynamic-load export:

`aerodynamics -> structural check -> pass/fail`

Use an explicit coupled state:

`mission + atmosphere + controls + propulsion
 -> aerodynamic pressure / forces / moments
 -> distributed structural loads
 -> deformation / stress / strain / modes
 -> updated aerodynamic shape and control effectiveness
 -> updated trim / power / mission state
`

The loop terminates only when declared coupling residuals meet pre-registered tolerances, or it reports `NON_CONVERGED`. Structural convergence and CFD convergence are separate. A rigid-wing converged CFD solution is not a structural result; static strength is not flutter clearance; flutter analysis is not fatigue-life substantiation.

Model discrepancy must remain distinct from:
- measurement and instrumentation uncertainty;
- numerical and mesh uncertainty;
- material-property and manufacturing variability;
- boundary-condition and load-transfer uncertainty;
- model-form uncertainty and extrapolation beyond the admitted domain.

Do not compress these into a single undocumented confidence score or safety factor.

## 4. Versioned result products

All records use canonical SI units, explicit frames/datum, schema version, source/model revision, immutable input digests and a declared claim ceiling.

### 4.1 StructuralLoadCase

Required identity:
- aircraft configuration and geometry digests;
- mission and phase IDs/digests;
- loading-state, mass-ledger and inertia digests;
- atmosphere, wind, runway/ground context and operating point;
- propulsion dispatch, propulsor state, control configuration and failure-state identity;
- coordinate frames, datum, axes, sign conventions and reference scales.

Required content:
- distributed aerodynamic pressure/force/moment fields or traceable reduced representations;
- inertial, gust, manoeuvre, landing-gear, taxi/ground-handling and propulsor thrust/torque loads;
- attachment and interface loads;
- load-transfer method, mapping identity and force/moment conservation residuals;
- source reference, fidelity, applicability and uncertainty.

A resultant-only load may be used for a bounded analytical screen, but must not be represented as a distributed pressure field.

### 4.2 StructuralModelRef

Record solver name/version/build digest, model/element class, mesh digest and refinement lineage, geometry mapping, material-card source/revision, joints/attachments, constraints/boundary conditions, mass representation, damping assumptions, reduced-order model identity, numerical method and applicability domain.

Unknown material allowables, missing load paths, unsupported boundary conditions and unqualified model reduction remain unresolved.

### 4.3 Separate analysis results

| Result | Required outputs | What it does not prove |
|---|---|---|
| Static structural | Deflection/twist, strain/stress, declared margins, buckling/local-instability outputs where modeled, convergence and uncertainty | Aeroelastic stability or fatigue life |
| Static aeroelastic | Coupled load/deformation history, final geometry identity, residuals, convergence and sensitivity | Dynamic flutter clearance |
| Dynamic aeroelastic | Modal basis, frequencies, damping, dynamic response and method/domain metadata | Full-envelope clearance absent required cases and validation |
| Durability | Load spectrum, cycle counting, material/joint model, environmental/inspection assumptions, damage and uncertainty | Life without qualified spectrum/allowables |
| Structural closure | Case-by-case status, residuals, worst margins, uncertainty decomposition, failures and replay receipt | Airworthiness or certification |

Never emit one unqualified `structurally_safe=true`. Separate strength, stiffness, buckling, aeroelastic stability, fatigue/durability and maintainability. Each is assessable only when its own evidence is admitted.

### 4.4 Coupling identity

Every result binds the same exact configuration, geometry, mission, phase, loading state, propulsion/control state and environment. A change in fuel location, payload, mass/CG/inertia, propulsor dispatch, blowing state, control setting, geometry, structural model, mesh or material card invalidates dependent results unless a documented applicability/transform proof exists.

Each analysis receipt includes:
- exact input and output byte digests;
- solver executable/source/build identity and arguments;
- mesh/material/model input digests;
- convergence and termination state;
- numerical tolerances and residuals;
- resource and full cost record, including failed or aborted attempts;
- uncertainty and applicability decision;
- independent replay/admission status.

## 5. Required AER-01 load-state coverage

The candidate study must declare and preserve separate load cases for:

1. Dispatch, taxi, takeoff, rotation, initial climb, climb, cruise, descent, approach, landing and go-around.
2. Gust and symmetric/asymmetric manoeuvre conditions within a declared structural design envelope.
3. Landing gear, ground handling, parking and applicable runway/surface conditions.
4. Propulsor thrust, torque, gyroscopic/inertial and mounting loads.
5. Distributed slipstream/blowing, wing/flap interference, high-lift and control-surface loads.
6. Propulsor-out/asymmetric thrust and declared degraded-control cases.
7. Fuel burn, occupant/baggage relocation and removable-module states affecting CG, inertia or load paths.
8. Battery, generator, cooling, wiring, containment and other installed equipment support loads.
9. Relevant thermal and material-property states.

A cruise-only aerodynamic coefficient cannot be used to infer the complete design-load envelope. An unimplemented case is `NOT_ASSESSED`.

## 6. Verification and validation ladder

| Tier | Work | Evidence admitted |
|---|---|---|
| 0 | Units, frames, load transfer and hand calculations | Contract/arithmetic verification |
| 1 | Analytic beam, plate and simple wing-box cases | Known-answer numerical verification |
| 2 | Mesh refinement, solver cross-check and coupling convergence | Numerical verification with scoped tolerances |
| 3 | DPW-8/AePW-4 cases and experimental references | External method/solver reproduction for exact cases |
| 4 | Candidate-specific material, geometry, load-path and propulsor-interaction models | Candidate-specific analytical validation |
| 5 | Independently reviewed coupon/component/wind-tunnel measurements | Physical correlation for the tested article and conditions |
| 6 | Approved aircraft-level test and certification programme | Only the authority-governed scope actually substantiated |

### 6.1 Synthetic independent oracle

Start with hand-calculable cantilever beam and plate solutions, simple wing-box bending/torsion, and conserved load-transfer fixtures. Add a small coupled static-aeroelastic fixed-point example with a known solution plus a divergent/non-convergent control.

The independent verifier must not import production coupling or solver code. It must recompute known-answer results from frozen raw inputs.

Hostile controls include stale geometry/mass/mesh/material digests, axis/sign/unit mutation, double-applied or non-conserved loads, missing structural mass, unsupported materials, out-of-domain aerodynamics, unresolved allowables, false-feasible status and iteration-limit termination reported as convergence.

### 6.2 External references: DPW-8/AePW-4

The joint 2026 Drag Prediction and Aeroelastic Prediction Workshop is the preferred open method-qualification destination. Its working groups include static deformation, buffet, high-angle, high-speed, large-deflection, sources of scatter and test environments. Public workshop resources include NASA Common Research Model and ONERA OAT15A experimental data.

Sources:
- https://www.aiaa-dpw.org/
- https://nescacademy.nasa.gov/workshops/AePW4/public
- https://dpw.larc.nasa.gov/DPW8/
- https://www.nasa.gov/common_research_model/

**Applicability limitation:** the transonic CRM cases are not low-speed validation of Wren 2 or Wayfarer 4. Use them to qualify data exchange, coupling, solver reproduction and discrepancy accounting. Candidate-specific validation requires relevant low-speed Reynolds/Mach conditions, propulsor interaction, materials, load paths and independent measurements.

### 6.3 Industry solver strategy

- SU2 remains the pinned CFD/adjoint backend. Its v8.6.0 release (30 October 2026) lists experimental FSI+CHT/aerothermoelasticity support; experimental capability is not assumed to be production-qualified for our cases. https://github.com/su2code/SU2/releases/tag/v8.6.0
- NASTRAN structural/aeroelastic workflows are a reference backend where legitimately available. NASA's public NASTRAN/OpenMDAO integration report covers static, aeroelastic, flutter and buckling analysis integration. https://ntrs.nasa.gov/citations/20250007060
- OpenMDAO remains a coupled MDO/derivative baseline, not a replacement for Symtropy's Rust evidence/orchestration layer.
- Do not claim support for any solver feature until its exact build, case, convergence and output have been independently replayed.

## 7. Competitive benchmark protocol

Compare sequential aero-then-structure, rigid-geometry CFD plus independent structure, one-way load transfer, coupled static aeroelastic iteration and supported MDF/IDF methods. Add Symthaea experiment selection only as a policy over the same physics backends.

Freeze inputs, cases, solver versions, seeds and total budgets before comparison. Charge mesh generation, setup, tuning, surrogate training, failed solves, coupling retries, independent audits, compute and human review.

Primary metrics:
- force/moment conservation error during load transfer;
- deformation, modal frequency and damping error against admitted references;
- mesh/solver/model sensitivity and uncertainty decomposition;
- coupled-iteration success, failure and cost;
- false-feasible rate with denominator and uncertainty interval;
- out-of-domain abstention and stale-identity rejection;
- worst-decile constraint margin and robust feasible-Pareto quality;
- decision regret and cost per independently reproduced design decision.

A coordinator is not better than SOTA because it uses fewer CFD calls or selects a novel configuration. It must preserve or improve independently validated decision quality at matched total cost across held-out geometry and mission families.

## 8. Proposed internal gates

Targets below are research acceptance goals, not aviation standards or achieved results:

- 100% agreement with the locked analytical oracle.
- Zero false-feasible results in the frozen synthetic hostile corpus.
- 100% traceability of load-, configuration- and margin-driving inputs.
- 100% stale-identity rejection and deliberate out-of-domain abstention.
- No missing load case silently treated as zero or as passed.
- Structural, CFD and coupling convergence reported separately.
- DPW-8/AePW-4 reproduction results reported with actual numerical/experimental discrepancies; no post-hoc tuning on held-out measurements.
- Candidate-specific physical correlation required before candidate performance or structural-superiority claims.

A zero-failure finite test corpus does not establish zero real-world risk.

## 9. Immediate sequence

1. Freeze this contract and add machine-readable schemas for load cases, structural model references, static results, aeroelastic results and closure reports.
2. Build the independent beam/plate/load-transfer oracle and negative controls.
3. Add the exact-state interface matrix to #3237 and #3945.
4. Admit DPW-8/AePW-4 sources by exact file hash, revision, conditions, terms and geometry identity.
5. Acquire controlling Wren 2 / Wayfarer 4 geometry, material, joint, mass-distribution, propulsor-mount and control-surface evidence; leave unavailable values null.
6. Promote only selected candidates to appropriate structural/aeroelastic solver runs and independently reviewed physical correlation.

## 10. Claim boundary

This document is a research contract and benchmark plan. It does not establish structural adequacy, flutter clearance, fatigue life, aeroelastic safety, aircraft performance, airworthiness, certification, suitability for manufacture or permission to test/fly. Physical tests require separate qualified engineering review, risk assessment, calibrated metrology, facility approval and applicable authority permissions.
