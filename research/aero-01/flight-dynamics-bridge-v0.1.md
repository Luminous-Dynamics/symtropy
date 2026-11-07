# AER-01 Flight-Dynamics Evidence Bridge — v0.1

**Status:** Interface research contract; no aircraft flight-dynamics result is qualified  
**Tracking:** [AERO-FLIGHT-DYNAMICS-BRIDGE-001 / Symtropy #4076](https://github.com/Luminous-Dynamics/symtropy/issues/4076)  
**Parent programme:** [AER-01 / #3921](https://github.com/Luminous-Dynamics/symtropy/issues/3921)  
**Flight-dynamics model owner:** [Symthaea #6249](https://github.com/Luminous-Dynamics/symthaea/issues/6249)  
**Whole-aircraft closure:** [#3945](https://github.com/Luminous-Dynamics/symtropy/issues/3945)  
**Mission chronology:** [#4029](https://github.com/Luminous-Dynamics/symtropy/issues/4029)  
**STOL performance:** [#4055](https://github.com/Luminous-Dynamics/symtropy/issues/4055)  
**Date:** 2026-11-07

> This document defines an interoperability and evidence contract. It is not an aircraft design, verified flight model, handling-qualities result, flight-envelope approval, certification finding, or authorization to manufacture, test, or fly.

## 1. Why this bridge is needed

AER-01 now has separate owners for aircraft configuration, source-bound mass/loading states, mission chronology, carrier-aware energy accounting, runway/climb performance, aerodynamic applicability, and fixed-wing flight dynamics. The remaining integration risk is that a valid result from one owner can be consumed as though it answered a different engineering question.

In particular:

- a converged trim point is not proof of static or dynamic stability;
- a stable local linearization is not proof of nonlinear manoeuvre or envelope safety;
- a controller accepting a command is not proof that the aircraft has aerodynamic or control authority to execute it;
- a nominal control-effectiveness estimate is not valid after a propulsor failure, CG shift, mass change, or change in blown-lift state;
- a correct calculation bound to the wrong aircraft/loading/mission state is still the wrong engineering result.

The bridge therefore owns **identity, semantics, applicability, exchange, and admission**. It does not own a second fixed-wing dynamics solver.

## 2. Ownership boundaries

| Concern | Owning work | Symtropy bridge responsibility |
|---|---|---|
| Fixed-wing aerodynamic and flight-dynamics model, derivatives, local dynamics and qualification profile | Symthaea #6249 | Consume exact model/result references; preserve scope and claim ceiling |
| Configuration, mission and whole-aircraft coupling | Symtropy #3921 / #3945 | Require matching immutable identities |
| Mass, CG, inertia and loading-state transitions | Symtropy #3984 / #4000 | Verify state digests and frame/datum consistency |
| Phase chronology and reserves | Symtropy #4029 / #4030 | Bind each result to a specific phase and state |
| Propulsion and carrier-aware energy planes | Symtropy #3900 / #3903 / #4036 | Carry the exact propulsion, dispatch, store and thermal state references |
| Takeoff, landing, obstacle and hot/high performance | Symtropy #4055 | Consume qualified trim/control evidence where that work package requires it; do not replace its performance model |
| Aerodynamic model applicability | Symtropy #3495 | Reject or downgrade out-of-domain references |
| Certification-basis / means-of-compliance applicability | Symtropy #4042 | Preserve compliance status as separate from research admission |

No bridge output may be promoted by implication from another owner's successful workflow.

## 3. Three distinct evidence products

The bridge must preserve three separate products. They must not collapse into a single `stable=true` or `controllable=true` field.

### 3.1 Trim equilibrium

A trim record answers: *for this exact configuration and operating state, does the declared model have an equilibrium solution, and what controls/power are needed to hold it?*

Required result content:

- target flight condition and phase;
- aircraft configuration, geometry, mission and loading-state digests;
- mass, CG, inertia, datum and reference-frame identifiers;
- atmosphere and aerodynamic-model applicability decision;
- propulsion/propulsor states and available power/thermal limits;
- solved state/control variables and any alternate solution branches;
- dimensional force and moment residuals in all declared axes;
- reference force/moment scales and tolerances used to classify residuals;
- active constraints, control-surface position/rate limits and actuator saturation;
- engine, motor, generator, inverter, battery and thermal feasibility references as applicable;
- convergence/termination status, solver identity, uncertainty and claim ceiling.

A trim solution requiring a saturated or unavailable effector is not silently accepted as a feasible trim. Multiple roots or branch ambiguity must be reported.

### 3.2 Local dynamics and stability

A local-dynamics record answers: *what does the linearized model say about small perturbations around this particular trim point?*

Required result content:

- exact trim-result digest and linearization-point state;
- complete state and input ordering;
- coordinate axes, units, sign conventions, nondimensionalization and reference quantities;
- dimensional/nondimensional stability and control derivatives with source/model identity;
- state-space matrices or a lossless, versioned representation of them;
- eigenvalues, mode classification and the numerical method used;
- perturbation neighbourhood and operating-domain validity;
- uncertainty and comparison/reference evidence;
- explicit statement that local results do not establish nonlinear, stall, spin, upset-recovery or full-envelope behaviour.

A derivative table without conventions and reference quantities is not an interoperable result. A linearization at one CG or propulsor state cannot be reused at another state without an explicit, qualified transformation or a new solve.

### 3.3 Control authority and degraded controllability

A control-authority record answers: *under the declared normal or degraded configuration, which forces and moments remain achievable, with what margins and assumptions?*

Required result content:

- control-effectors and propulsors available in this state;
- failure identity, detection assumption, isolation state and any reconfiguration;
- actuator position/rate limits and remaining authority;
- asymmetric thrust/slipstream effects where applicable;
- available force/moment set or a clearly bounded approximation;
- demanded trim/maneuver moments and margin by axis;
- electrical bus, source, storage and thermal states that constrain actuation;
- pilot/control-system authority boundary and recourse assumptions;
- uncertainty, applicability and explicit unresolved conditions.

Propulsor count is not a redundancy metric. A failed propulsor can change yawing moment, local dynamic pressure, wing lift distribution, thermal/electrical loading and available control authority simultaneously.

## 4. Immutable identity and applicability

Every exchanged record must bind at least:

1. aircraft family/model/variant and exact configuration digest;
2. geometry generation and reference-frame/datum manifest;
3. loading-state digest, including ordered installed/removed/unknown mass items;
4. mass/CG/inertia report digest;
5. mission-card and mission-timeline digests;
6. phase ID and predecessor state digest;
7. aerodynamic database/model revision and applicability envelope;
8. propulsion/propulsor map, dispatch, storage and thermal-state references;
9. control-system/actuator configuration and declared failure state;
10. solver/model build, input/output artifact digests and execution status.

The evidence must distinguish **identity** from **truth**. A SHA-256 digest establishes byte identity; it does not authenticate a manufacturer source, prove aircraft effectivity, establish measurement quality, or show that a model is physically valid.

Use explicit result states:

- `ADMITTED_FOR_DECLARED_SCREEN`
- `PENDING_REVIEW`
- `OUT_OF_DOMAIN`
- `REJECTED_STALE_IDENTITY`
- `NON_CONVERGED`
- `NOT_ASSESSED`
- `INDETERMINATE`

No missing derivative, unavailable effector, absent phase result or unresolved model boundary may be replaced by zero.

## 5. Coordinate and exchange conventions

Use AIAA S-119 as the primary interoperability reference for variable definitions, frame conventions, units, sign conventions and model provenance. The bridge may define a Rust/JSON internal representation, but it must preserve equivalent semantics and offer a documented lossless exchange mapping where the relevant S-119 concepts apply.

Every exchanged variable must state:

- symbol and stable machine identifier;
- physical meaning;
- dimensional unit or nondimensional definition;
- reference frame and axis directions;
- sign convention;
- reference origin/datum;
- reference area, length, mass, speed or other normalization where relevant;
- source and model version;
- uncertainty and evidence class.

Do not claim AIAA S-119 conformance until the full applicable standard has been obtained, mapped and independently checked. The current goal is to avoid ambiguous exchanges and preserve provenance—not to advertise compliance.

## 6. Minimum bounded synthetic corpus

The first independent corpus should be deliberately small and hand-checkable. It is for contract and arithmetic verification, not aircraft performance validation.

| Case | Positive control | Required hostile control |
|---|---|---|
| Symmetric level trim | Analytic force/moment equilibrium | Wrong axis or sign convention |
| Speed change | Two operating points with distinct trim | Reuse stale trim digest |
| Loading change | Off-CG fuel burn shifts mass/CG/inertia | Reuse pre-burn trim/derivatives |
| Propulsion change | Declared power-limited trim | Treat peak power as continuous |
| Control authority | Known actuator moment limit | Accept a saturated effector as unconstrained |
| Asymmetric thrust | Known yawing-moment balance | Ignore failed-propulsor asymmetry |
| Model applicability | Query inside declared model envelope | Query outside envelope without abstention |
| Linear dynamics | Independently computed small-model eigenvalues | Claim nonlinear envelope from local eigenvalues |
| Actuator degradation | Recompute remaining authority | Retain the failed actuator's authority |
| Identity chain | All digests match | Stale geometry/mass/mission/model digest |
| Solver status | Converged bounded solution | Treat iteration limit/non-convergence as pass |
| Evidence class | Synthetic result remains synthetic | Relabel synthetic fixture as measured evidence |

The independent oracle must be separately implemented. It must not import Symthaea's production model, the Symtropy bridge, or production trim functions. Store raw inputs, expected outputs, conventions and fixture hashes.

## 7. External comparison ladder

Use references by the claim they can support; do not rank unrelated models as if they were interchangeable.

### A. Exchange and model-description practice

- **AIAA S-119 Flight Dynamics Model Exchange Standard:** coordinate systems, units, sign conventions, simulation variable representation, component provenance and model-quality descriptions. NASA's standards catalogue lists it as active and publicly accessible.
- **NASA-STD-7009B Change 1:** model and simulation credibility practice. Programme-specific acceptance criteria must still be defined and approved by the relevant technical authority.

### B. Fixed-wing dynamics and control references

- **NASA RAVEN-SWFT:** a subscale Bede BD-6-derived subject intended for wind-tunnel and flight-test research in flight dynamics and controls. It is a promising external reference subject; obtain and pin the exact model/data revision, test conditions and reuse terms before numerical admission.
- **NASA AirSTAR / Generic Transport Model:** published subscale flight-research work and a NASA software-catalogued flight-dynamics simulation with rigid-body equations, aerodynamic coefficient table lookups, actuator servos and sensor bandwidth. The historical GTM DesignSim material also documents trim-point and linearization utilities. Confirm exact release artifacts, execution dependencies and license before using it as a reproducibility benchmark.

### C. Aerodynamic and aeroelastic context

- **NASA TMR:** use its cases for the specific turbulence-model verification/validation claims represented by those cases; do not infer fixed-wing handling qualities from a 2D airfoil polar.
- **DPW-8 / AePW-4:** use steady drag, static deformation, unsteady buffet and FSI work packages as separate benchmark tasks. They do not replace subscale flight-dynamics validation.
- **FAA AC 23-8C / EASA CS-23 Amendment 6 and AMC/GM Issue 5:** technical/certification references only. AER-01's actual South African certification basis and accepted means must be established with SACAA and appropriately qualified specialists under #4042.

## 8. Benchmark scorecard

Report each metric independently and stratify by model family, operating condition and evidence class.

| Category | Metrics |
|---|---|
| Trim | Force/moment residuals, convergence, repeatability, branch count, active constraints |
| Local dynamics | Derivative discrepancy, eigenvalue/mode discrepancy, linearization validity region |
| Control authority | Available force/moment margin, actuator saturation/rate margin, failure-state authority |
| State sensitivity | Change caused by CG/inertia, fuel, payload, blown-lift and propulsion-state transitions |
| Applicability | In-domain coverage, abstention precision/recall, extrapolation errors |
| Evidence quality | Source completeness, artifact replay, uncertainty coverage, independent-oracle agreement |
| Decision value | Cost to resolve a control/stability uncertainty that changes the architecture or mission decision |

Initial internal targets (not aviation standards or achieved results):

- 100% configuration/mission/loading/phase/model identity propagation;
- 100% agreement on locked analytic synthetic cases;
- zero false-feasible results in the frozen synthetic hostile corpus;
- 100% rejection or abstention for stale, unsupported or out-of-domain states;
- no local-stability, trim or control-authority result represented as full-envelope or airworthiness evidence;
- at least three distinct model/operating-condition families before making generalization claims.

Do not optimize aggregate metrics at the expense of worst-case control margins. Publish the task-level results and all failed runs.

## 9. No-runner execution sequence

1. Freeze the versioned trim, local-dynamics and control-authority result schemas.
2. Build the independent analytic corpus and mutation controls.
3. Implement only the identity/admission/exchange adapter in Rust, consuming owner outputs from Symthaea #6249.
4. Replay the same cases through direct/manual exchange and the S-119-aligned representation; record conversion loss, ambiguity and audit effort.
5. Admit one external reference subject only after source bytes, revision, conditions, license and model applicability are reviewed.
6. Join accepted per-operating-point results to #4029 mission phases and #3945 closure; unsupported phases remain `NOT_ASSESSED`.
7. Let #4055 consume only the specific trim/control results required for its runway and climb model.
8. Keep Symthaea as decision support. Any eventual flight-control implementation requires a separate aircraft-local control, verification, runtime-assurance and certification pathway.

## 10. Claim ceiling

A passing synthetic corpus qualifies identity handling, exchange semantics, arithmetic and fail-closed behaviour for the bounded test cases. It does not establish real-aircraft aerodynamic coefficients, trim capability, stability, handling qualities, stall/spin behaviour, flutter safety, pilot acceptability, flight envelope, airworthiness, certification, or permission to manufacture, test or fly.
