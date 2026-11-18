# Luminous AER-01 — Aircraft Architecture Research Brief

**Status:** Research hypothesis / concept architecture only  
**Tracking:** [AERO-AIRFRAME-001, Symtropy #3921](https://github.com/Luminous-Dynamics/symtropy/issues/3921)  
**Version:** 0.1  
**Date:** 2026-11-06

## 1. Purpose

Develop a demonstrably better owner-operable aircraft architecture by co-designing the airframe, propulsion, energy, controls, maintainability, manufacturing and operating environment. The first design space is the Wren 2 two-seat and Wayfarer 4 four-seat family.

This is not a construction drawing, approved design, airworthiness finding or performance claim. AER-01 is a candidate to be falsified against conventional aircraft and other advanced configurations.

The programme's actual success condition is:

> More useful, reliable missions per unit of lifecycle cost and operating burden, with equal or stronger safety margins and independently reviewable evidence.

A new wing shape alone does not meet that condition.

## 2. What the 2026 field already demonstrates

We must not claim that the following ingredients are novel:

| Reference | What it contributes | Evidence boundary |
|---|---|---|
| Electra EL9 | Hybrid-electric distributed propulsion, blown lift, eight wing propulsors and fly-by-wire; manufacturer-published targets include 150 ft ground roll, 175 KTAS and 1,100 nmi ferry range | Company targets and certification-basis progress are not independently demonstrated EL9 service performance or a type certificate |
| DLR HYDRA | Modular test-aircraft concept with interchangeable fuel-cell and turbogenerator powertrain containers, battery support and DEP | 2026 research/design concept, not a certified production aircraft |
| DLR PomoΔoro | High-aspect-ratio, structurally supported wing; battery/fuel-cell energy box; DEP, boundary-layer and morphing concepts | Student design concept; not physical aircraft validation |
| NASA truss-braced wing experiments | Structural and aerodynamic work toward longer, thinner wings | Test article evidence applies to that article and protocol, not automatically to AER-01 |
| NASA X-57 | Lessons on electric propulsion integration, wind-tunnel work, systems testing and airworthiness process | Project closed without first flight; not a successful flight demonstration |

Primary references:
- Electra technology and published targets: https://electra.aero/technology
- Electra FAA certification-basis announcement (10 July 2026): https://electra.aero/news/electra-achieves-faa-certification-milestone-for-el9-ultra-short-aircraft
- AOPA's 2026 report on the blown-wing architecture: https://www.aopa.org/news-and-media/all-news/2026/july/flight-training/electras-blown-wing
- DLR HYDRA (21 August 2026): https://www.dlr.de/en/latest/news/2026/hydra-aircraft-concept-takes-first-place-in-the-dlr-design-challenge-2026
- DLR PomoΔoro: https://www.dlr.de/en/latest/news/2026/hydra-aircraft-concept-takes-first-place-in-the-dlr-design-challenge-2026/pomodoro-tu-munchen
- NASA structural wing experiment (17 July 2026): https://www.nasa.gov/centers-and-facilities/armstrong/nasa-pushes-new-wing-design-to-find-structural-limits/
- NASA X-57 programme status: https://www.nasa.gov/x-57-maxwell/

**Implication:** AER-01 must differentiate through the quality of the whole-system solution—especially owner economics, maintainability, hot/high operations, configuration control, fault containment and measured performance—not by rebranding DEP, blown lift, modular power or a truss.

## 3. The leading design hypothesis

### AER-01: maintainable short-field fixed-wing aircraft

Start with **eSTOL rather than VTOL**. A fixed-wing aircraft avoids the continuous hover-power burden and extra lift-system hardware of VTOL, while still investigating access to shorter and less-developed landing surfaces. This is a mission hypothesis, not a universal conclusion: where vertical access is essential, evaluate VTOL separately.

Candidate features, each subject to comparative testing:

1. **High-wing, high-aspect-ratio lifting system.** Compare cantilever, strut-braced and alternative load paths. A strut's structural benefit must be weighed against interference drag, junction effects, mass, inspection and fatigue.
2. **Partial-span distributed blown lift.** Investigate propulsors integrated with the wing and high-lift surfaces. Compare partial-span blowing, full-span blowing, and conventional propulsion. Optimize propulsor number, location, diameter, disk loading, failure groups, flap interaction and cruise state. Do not copy another aircraft's propulsor count.
3. **Phase-aware hybrid power as one option.** Test whether a generator sized for sustained cruise plus battery buffering for takeoff/climb/go-around improves the installed system. The model must include generators, motors, inverters, cabling, converters, thermal management, battery degradation, protection, structure and maintenance. Battery-electric and conventional propulsion remain required comparators.
4. **Load-aware wing structure.** Begin with passive aeroelastic tailoring and structural load paths. Active gust-load alleviation and morphing are later options only when the mass, sensors, actuators, flutter, failure and maintenance penalties are modelled.
5. **Conventional stability and authority boundary.** Preserve a conventional empennage and pilot authority in the initial reference configuration. Any advanced fly-by-wire or autonomy must be evaluated as a separate system behind an independent runtime-assurance layer.
6. **Service-first modularity.** Design replaceable energy and propulsion modules, harness access, standard interfaces, inspectable structure and a serial/configuration-bound maintenance history. Modularity includes joints, connectors, fire/thermal containment and post-replacement verification; it does not remove the need for approved configuration control.
7. **Ground charging.** Conductive charging is the default comparison. Static inductive charging is optional pending measured efficiency, alignment, foreign-object, thermal, electromagnetic, weather and lifecycle evidence. In-flight power beaming is excluded.

The distinguishing paradigm is **designing the aircraft around complete mission and service life**, rather than independently optimizing wing, battery, motor or cockpit.

## 4. Configuration search — retain competing hypotheses

| ID | Candidate | Question being tested |
|---|---|---|
| C0 | Conventional Wren / Wayfarer baseline | What is the best reference for cost, payload, range, reliability and serviceability? |
| C1 | High-aspect-ratio or strut-braced wing with conventional propeller | Does lower induced drag outweigh structural and interference penalties? |
| C2 | High-aspect-ratio wing + partial-span blown lift | Can we capture useful low-speed lift without the full distributed-propulsion mass and drag? |
| C3 | Full blown wing + distributed propulsion | Does the larger lift effect survive complete installed-system and failure accounting? |
| C4 | Battery-electric DEP | Which short training/local missions close at end-of-life and adverse temperature? |
| C5P | Parallel hybrid | Does mechanical drive plus electric boost/recuperation outperform direct drive and series-electric conversion on the matched mission? |
| C5S | Series-hybrid DEP | Does phase-aware power sharing improve the installed system after generator, motor, inverter, cable and cooling penalties? |
| C6 | Other topology (joined/box wing, tandem/canard, lifting body, etc.) | Does any alternative produce a robust, validated advantage rather than visual novelty? |
| C7 | Lift+cruise / tiltrotor / tiltwing / multicopter | Is VTOL access worth its hover energy, mass, transition, noise and certification cost for a mission that needs it? |

Do not force one architecture across Wren 2 and Wayfarer 4 if their payload, range, speed, handling and economics differ.

## 5. Mission contracts to freeze

Before optimization, create versioned mission cards for:

- **Wren 2:** training, local flying and two-seat personal travel.
- **Wayfarer 4:** four-seat touring and practical utility.
- **Hot/high:** South African plateau operations as a relevant design case, with explicit temperature, pressure altitude, wind and runway conditions.
- **Short-field:** surface quality, slope, obstacles, runway length, crosswind and go-around.
- **Diversion and reserve:** alternate, weather deterioration, energy degradation and forced-landing cases.

Each mission card must contain occupant/baggage payload, useful load, range, cruise and approach speeds, climb, takeoff/landing limits, reserve/diversion, weather/icing boundary, density altitude, turnaround, annual utilization and cost-accounting boundary. Initial numbers are hypotheses to review, not silently adopted requirements.

## 6. Whole-aircraft architecture and evidence flow

```text
MISSION + ODD + PAYLOAD + RESERVE
                |
                v
   CONFIGURATION / GEOMETRY GENOME
                |
        +-------+-------+
        |       |       |
        v       v       v
       AERO   STRUCT.  PROPULSION
        |       |       |
        +-------+-------+
                |
                v
       MASS / CG / POWER / THERMAL
                |
                v
       MISSION + COST + FAILURE MODEL
                |
                v
     PARETO / UNCERTAINTY / DECISION
                |
                v
     INDEPENDENT ORACLE + REPLAY
                |
        [evidence acquisition gate]
                |
                v
       BENCH / WIND-TUNNEL / HIL
```

- **Symtropy:** configuration space, sizing, solver adapters, MDO, uncertainty, Pareto analysis and independent replay. It is not a universal CFD, FEM, propeller, electrochemistry or certification solver.
- **Symthaea:** mission reasoning, candidate/experiment proposal, uncertainty and abstention. It is not the flight controller or the safety monitor.
- **Symthaea aircraft family / serviceability:** reuse AIRFAMILY-001 (#7745) for the Wren 2 / Wayfarer 4 / conditional Ranger 4 / gated Wren-E family decisions; reuse the evidence-bound maintenance task-card validator (#7750) and transparent ownership-cost model (#7754). Do not duplicate maintenance authorization or lifecycle-cost implementations. Commonality is a hypothesis, not a mandate to force one airframe or certification basis across all variants.
- **Mycelix:** source, component, test, inspection and approval provenance. A signed record is not proof that the underlying assertion is true.
- **Sol-Atlas:** versioned airfield, obstacle, terrain, airspace, weather-source and ground-energy context. Context is not clearance.

Use the existing aerospace MDO interoperability contract (#3237), aerodynamic applicability work (#3495), fixed-wing physics owner (Symthaea #6249), energy owner (#7646), assurance boundary (#10747), aircraft-family and maintenance work (#7745/#7750/#7754). Do not create a second orchestration, solver, maintenance-task or ownership-cost stack.

## 7. Benchmark scorecard

All candidates must use identical payload, reserve, atmosphere, surface, obstacle, noise and lifecycle-accounting boundaries.

| Dimension | Metrics |
|---|---|
| Mission capability | Payload–range–reserve feasibility; ground roll and obstacle-clear distance; climb; cruise; ceiling; approach/stall speed; go-around and diversion margin |
| Aerodynamics | L/D; CLmax; lift/drag/moment polars; powered/unpowered wing; propeller–wing–flap interaction; stall progression; asymmetric-thrust control |
| Structure | Structural mass fraction; stiffness/deflection/twist; buckling; fatigue/damage tolerance; flutter; gust response; load-path and strut-junction evidence |
| Energy and thermal | Installed system Wh/kg and W/kg; phase-resolved bus demand; generator response; battery SoC/SOH/current/temperature; converter/drive losses; cooling mass and drag; end-of-life reserves |
| Safety | Single and common-cause propulsor, inverter, bus, battery, generator, sensor and actuator faults; fire/smoke; electrical isolation; forced landing and diversion outcomes |
| Human factors | Pilot workload; visibility at high pitch; handling qualities; cabin and baggage ergonomics; emergency egress; noise and vibration |
| Maintainability | Scheduled/unscheduled labor per flight hour; median/p90 diagnosis and replacement time; tool count; access/inspection time; first-time-fix; spare availability; configuration error rate |
| Manufacturing | First-pass yield; rework; scrap; inspection hours; production cycle time; supplier concentration |
| Economics | Acquisition and financing; fuel/electricity; overhaul; battery/engine replacement; storage; insurance; training; downtime; infrastructure; residual value; cost per completed mission and passenger-km |
| Design optimisation | Feasible Pareto hypervolume; regret; total engineering cost; solver failure accounting; uncertainty calibration; out-of-domain abstention; experiments to decision |

Never hide safety or hard constraint failure inside a weighted average. Publish a Pareto set, constraint margins, uncertainties and sensitivity drivers.

### Comparison methods

Compare conventional sequential engineering, deterministic sweeps, gradient/adjoint MDO, evolutionary optimization, Bayesian/multi-fidelity search and Symthaea decision-value-based experiment selection under equal total budgets. Count human hours, compute, failed runs, tuning, experiments and independent verification.

### Aspirational gates (not achieved results)

- Zero false-feasible outcomes on locked known-truth and hostile cases.
- 100% traceability for material inputs, assumptions, solver identity, outputs and claims.
- Independent replay agrees exactly for deterministic canonical calculations, otherwise within a declared numerical tolerance.
- No promotion with unresolved structural, thermal, controllability or safety-analysis failures.
- On at least one frozen mission, seek at least 10% improvement in one primary mission-level measure against the best properly matched baseline, with no hidden constraint violation or unreported material regression in other measures.
- Show whether value-of-information experiment selection reaches an equivalent validated decision at materially lower total engineering cost than fixed sweeps or random/space-filling selection.
- Publish losing configurations and their failure regimes.

The 10% figure is a research target, not a universal aviation standard or an existing performance result.

## 8. Work packages

**P0 — Mission and baseline cards**
- Source-qualified C0 references, variant identity, payload/range/reserve and conditions.
- Wren 2 / Wayfarer 4 mission cards, including hot/high and short-field cases.
- Units, coordinate frames, mass/CG/inertia conventions and uncertainty register.

**P1 — Parametric concept model**
- Wing planform, aspect ratio, sweep/taper, airfoil, strut geometry, propulsor arrangement, flap geometry, tail volume, gear and powertrain mass.
- First-order mass/CG, trim, energy/power and aerodynamic screens.
- Hand-calculable independent fixtures.

**P2 — Coupled aero/structure/propulsion**
- Propeller–wing–flap interaction; selected high-fidelity aero and structural/aeroelastic runs.
- Explicit domain-of-applicability and uncertainty. No global "validated" flag.

**P3 — Energy and thermal closure**
- Reuse Symtropy #3900, #3903 and #3902.
- Close installed mass, phase dispatch, cooling, protection, reserve and CG across normal and failure cases.

**P4 — Independent benchmark**
- 10–20 small, hand-calculable synthetic cases.
- Negative controls: missing cable/cooling/motor mass; cell/pack confusion; peak treated as continuous; propulsor drag omitted; unsupported lift after motor failure; reserve removed; stale geometry/solver inputs; failed runs omitted; out-of-domain claims promoted; infeasibility hidden by scalar scoring.

**P5 — Low-consequence component test**
- Only after P0–P4: guarded instrumented wing/propulsor/flap rig, remote shutdown, independent metrology and approved test plan.
- No passenger aircraft, unrestrained free flight or live aircraft installation follows from this document.

## 9. Certification boundary

Map the selected aircraft to its actual applicable certification and operating path with the competent authority and qualified specialists. Official starting points:

- FAA Part 23 / Amendment 23-64: https://www.faa.gov/sites/faa.gov/files/2022-12/Amd_23-64.pdf
- EASA SC-VTOL Issue 2 (for VTOL counterfactuals only): https://www.easa.europa.eu/en/document-library/product-certification-consultations/special-condition-vtol
- SACAA Airworthiness and Engineering guidance, including Type Certification and Experimental Certificate of Airworthiness: https://www.caa.co.za/industry-information/airworthiness/

A simulation result, benchmark pass, prototype or provenance record does not grant design approval, type certification, production approval, maintenance privilege or permission to operate.

## 10. Immediate next outputs

1. [First decision experiment v0.1](./first-decision-experiment-v0.1.md): Wren 2 / Wayfarer 4 mission contracts, baseline families, C0/C1/C2/C5P/C5S down-selection, preregistered benchmarks and falsification rules.
2. [Machine-readable mission cards v0.1](./mission-cards-v0.1.json): six Wren 2 / Wayfarer 4 mission templates, with unknown values and source status preserved.
3. [Certification applicability register v0.1](./certification-applicability-v0.1.json) and [schema](./schemas/certification-applicability.schema.json): source-bound SACAA/EASA leads, explicit acceptance status and unresolved authority questions.
4. [Architecture discovery benchmark v0.1](./architecture-discovery-benchmark-v0.1.md) and JSON: separates architecture choice, parameter optimization, analysis-graph activation and fidelity acquisition; includes matched-budget policies and hostile controls.
5. [External benchmark reference manifest v0.1](./external-benchmark-reference-manifest-v0.1.json): pins OpenMDAO 3.45.2 and SU2 8.6.0, records build/licensing caveats and distinguishes reference candidates from actual run receipts.
6. Baseline cards with exact variants and evidence classes.
7. C0–C7 decision matrix with unknowns preserved.
8. Initial mass/CG/energy/power bookkeeping with transparent assumptions.
9. [Mass-properties implementation PR #3977](https://github.com/Luminous-Dynamics/symtropy/pull/3977): configuration-bound itemized mass, CG and inertia; draft, exact-head software checks passed.
10. [Mass-to-mission coupling PR #3979](https://github.com/Luminous-Dynamics/symtropy/pull/3979): exact-configuration ledger-to-cruise-screen adapter; draft, exact-head software checks passed.
11. [Mass provenance/loading states PR #4000](https://github.com/Luminous-Dynamics/symtropy/pull/4000): source-bound item evidence, source-capture/metrology metadata and immutable state manifests; draft, exact-head Rust/oracle/Clippy and inherited civil-aero workflows passed.
12. [Takeoff-state mission adapter PR #4025](https://github.com/Luminous-Dynamics/symtropy/pull/4025): evidence-qualified takeoff state into steady-cruise screen; draft, both exact-head workflows passed. Landing/intermediate states are withheld pending phase chronology.
13. [Phase-resolved mission issue #4029](https://github.com/Luminous-Dynamics/symtropy/issues/4029): immutable phase transitions, fuel/energy/CG updates, reserve/diversion/go-around and independent per-phase oracles.
14. [Mission-chronology pilot PR #4030](https://github.com/Luminous-Dynamics/symtropy/pull/4030): machine-readable phase/state/energy contracts and an independent synthetic oracle; overall sample mission explicitly INDETERMINATE because non-cruise phases remain unassessed.
15. [Carrier-aware phase energy PR #4040](https://github.com/Luminous-Dynamics/symtropy/pull/4040): v0.2 fuel/generator/store/bus/motor/propulsor energy planes; draft, exact-head CI pending.
16. [Certification evidence bridge #4042](https://github.com/Luminous-Dynamics/symtropy/issues/4042): applicability matrix and authority pathway research.
17. Only then, down-select candidate geometries for OpenVSP/VSPAERO and selected SU2/structural analysis.

**Claim ceiling:** This brief is a source-bound concept-search plan. It cannot establish optimality, safety, flightworthiness, certification, commercial readiness, better-than-SOTA performance or readiness to build.
