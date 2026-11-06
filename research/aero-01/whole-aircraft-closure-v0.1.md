# AER-01 Whole-Aircraft Closure Contract — v0.1

**Status:** Research contract / conceptual analysis only  
**Parent:** [AER-01 #3921](https://github.com/Luminous-Dynamics/symtropy/issues/3921)  
**Implementation issue:** [AERO-WHOLE-AIRCRAFT-CLOSURE-001 #3945](https://github.com/Luminous-Dynamics/symtropy/issues/3945)  
**Related owners:** #3237, #3495, #3885, #3900, #3903  
**Date:** 2026-11-06

> This document defines a software and analysis contract. It is not an aircraft design, construction drawing, performance prediction, safety finding, accepted certification plan, or authorization to manufacture or fly.

## 1. Why this layer exists

AER-01 already compares conventional fixed-wing, strut-braced, partial/full blown-lift, battery-electric, hybrid-electric and VTOL counterfactuals. The next bottleneck is not the number of configurations. It is whether all disciplines evaluate the **same aircraft, in the same state, for the same mission**, and whether changes in one discipline feed back into all affected disciplines.

The missing object is an aircraft-level closure contract.

AER-01 should not admit a candidate merely because a CFD run converged, an optimizer returned a low objective, a battery has enough nameplate energy, the propulsion system has enough peak power, or a digital evidence receipt is internally consistent. These facts have different claim ceilings. A candidate can only be considered feasible within a declared model when the coupled aircraft and mission state closes, all required disciplines return admissible results, and hard constraints are satisfied with explicit margins and uncertainty.

The closure contract composes existing work rather than replacing it:
- **#3237:** MDO interoperability, solver orchestration, independent replay and benchmark methodology.
- **#3495:** aerodynamic applicability domains, selective prediction and abstention.
- **#3900:** phase-resolved hybrid dispatch, storage state transitions and power/failure constraints.
- **#3903:** battery/capacitor evidence levels and end-of-life/adverse-condition qualification.
- **#3885:** aircraft energy architecture alternatives.
- **#3921:** AER-01 mission/configuration portfolio.

## 2. Core design principle

Replace one-way discipline handoffs with a closed-loop state:

    MISSION + OPERATING DOMAIN + PAYLOAD
                     |
                     v
           EXACT AIRCRAFT CONFIGURATION
                     |
                     v
        AERODYNAMICS / LOADS / PERFORMANCE
                     |
                     v
           STRUCTURE / PROPULSION / SYSTEMS
                     |
                     v
           MASS / CG / INERTIA / THERMAL
                     |
                     v
           TRIM / MISSION / ENERGY DISPATCH
                     |
                     v
            RESERVE / FAILURE / COST MODEL
                     |
                     v
              COUPLING + CLOSURE CHECK
                     |
              +------+------+
              |             |
           converged     not converged
              |             |
              v             v
       evidence admission  unresolved /
       and Pareto analysis non-converged

A candidate can be rejected by any discipline. Missing, stale, failed, unsupported or out-of-domain analysis cannot be silently replaced by a nominal value.

The optimization problem is multi-objective and constraint-first. Do not compress safety, payload, range, cost, noise, maintainability and emissions into one opaque scalar. First enforce hard feasibility constraints, then publish the remaining Pareto set and trade-offs.

## 3. Versioned input contracts

All canonical quantities use SI units. Source values may be retained in their original units, but every conversion must record the conversion rule, precision and source datum. Unknown values remain explicit unknowns.

### 3.1 MissionContract

A mission is a versioned object, not a route label.

Required fields:
- mission identity, version, owner, source digest and intended claim scope;
- aircraft family/configuration identity and permitted configuration envelope;
- route or mission profile, departure/arrival conditions and operating-domain boundaries;
- payload split into occupants, baggage, mission equipment and consumables;
- chronological phases with start/end conditions, duration or event boundary, target speed/altitude and demanded performance;
- takeoff/landing runway length, surface, slope, contamination, wind, obstacle and declared performance definition;
- atmosphere including pressure altitude, temperature, density and humidity where material;
- climb, cruise, descent, approach, landing, go-around, diversion and reserve cases as applicable;
- weather, icing and visibility assumptions and exclusions;
- utilization, turnaround and lifecycle cost-accounting boundary.

At minimum, the AER-01 mission set must preserve distinct Wren 2 training/local/personal missions, Wayfarer 4 touring/utility missions, hot/high missions, and short-field/obstacle missions. Values are not frozen merely because they are present in an early spreadsheet. Every requirement needs an owner, rationale, source and revision.

### 3.2 AircraftConfiguration

Every evaluated candidate has an immutable identity and a canonical digest covering:
- geometry generation, coordinate system, datum and reference dimensions;
- aerodynamic models, solver setup and applicability manifest;
- structural layout, materials, joints, load paths and structural-model identity;
- propulsion topology, engine/generator/motor/propulsor identity and installation;
- energy storage topology and source-to-load electrical architecture;
- cabling, power conversion, protection, cooling and thermal rejection;
- controls and actuation, including the authority boundary between pilot, aircraft-local control and decision-support functions;
- landing gear, furnishings, avionics, safety equipment and options;
- mass ledger, CG and inertia references;
- bill-of-material/cost revision and service/maintenance configuration.

Changing an item that affects mass, geometry, power, drag, thermal behavior, safety or maintainability creates a new candidate identity. A prior analysis does not automatically transfer.

### 3.3 MassLedger

Use itemized mass records, not one unexplained empty-weight constant.

Each record includes:
- stable item ID and parent/subassembly;
- mass and uncertainty;
- coordinate location or distributed-mass representation;
- installed/removed/operating state;
- source, source revision and applicability;
- measured / manufacturer-reported / estimated / analytical / unknown class;
- inclusion category and whether another parent record already includes it.

Explicitly distinguish basic empty mass, manufacturer-reported empty mass, operational empty mass, useful load, zero-fuel mass, ramp mass and takeoff mass. Definitions must match the relevant aircraft/reference source; do not mix manufacturer conventions.

At minimum, itemize structure, propulsion, motors, generators, propellers, energy storage, inverters, converters, cabling, cooling, mounts, containment, protection, controls, avionics, landing gear, furnishings, safety equipment, fuel, oil, fluids, occupants, baggage and mission payload.

Negative controls must catch omitted motor/cable/cooling/containment mass, double counting, unit mismatch and stale configuration generations.

### 3.4 Reference frames, CG and inertia

Declare axes, signs, datum, units and every coordinate transform.

For point-mass fixtures, independently calculate:

    m_total = sum(m_i)

    r_CG = sum(m_i * r_i) / sum(m_i)

For distributed or rigid subassemblies, retain each component's local inertia tensor and apply the parallel-axis theorem in the declared common frame. Do not substitute an inertia estimate from a different geometry or mass generation.

The CG result must be derived from the same mass-item identities as the mass ledger. A caller-provided CG that conflicts with the ledger is rejected. Whether a result lies inside a certified/approved envelope remains unknown unless the applicable envelope and its configuration scope have been supplied and qualified.

### 3.5 DisciplineResult

Every solver or analysis adapter returns:
- exact aircraft and mission digests;
- all input artifact digests and source identities;
- solver, model, executable, build and toolchain identity;
- command/configuration and environment identity where relevant;
- output artifacts and measurement/coordinate definitions;
- convergence and native termination state;
- fidelity and declared model scope;
- uncertainty and applicability decision;
- compute/resource observations with unknowns retained;
- failed, timed-out, cancelled and divergent state;
- claim ceiling and verifier/replay receipt.

A converged run is only a converged run. It is not automatically a validated physical prediction.

## 4. Closure invariants

### 4.1 Mass and balance

For a declared aircraft state:

    m_TO = m_structure + m_systems + m_propulsion
           + m_usable_energy_or_fuel + m_payload + m_consumables

The exact included terms follow the chosen mass convention. Report the mass residual, omitted/unknown items and uncertainty separately.

Compute CG and inertia from that same mass state. Fuel/energy consumption, occupants, baggage and removable modules may change the state. A takeoff CG must not be reused as a landing CG without recomputation.

### 4.2 Power and energy

Keep distinct:
- chemical/fuel energy;
- stored battery/capacitor energy;
- generator mechanical input and gross/net electrical output;
- bus electrical power;
- inverter/converter loss;
- motor electrical input and shaft output;
- propulsive power/thrust;
- auxiliary and cooling load;
- rejected heat.

At each time/mission interval, preserve the residual:

    P_sources_bus - P_propulsion_bus - P_aux
      - P_storage_charge - P_losses = P_residual

Sign convention, storage efficiency and time integration are defined by #3900. Do not force the residual to zero. Report its scale and uncertainty. Never count stored energy as new primary energy or mistake peak power for continuous power.

### 4.3 Thermal closure

For each relevant component, declare heat generation, thermal capacity, heat-transfer path, coolant/air flow, temperature limits and derating law. Cooling equipment has mass, drag, power and maintenance consequences.

If the model cannot establish an allowed temperature state over a mission or declared contingency, the result is indeterminate or infeasible. Do not assume a cooling capacity.

### 4.4 Aerodynamic and structural scope

Every aerodynamic result must carry its model family, geometry, mesh, operating condition, flow regime, convergence and applicability identity. Out-of-domain results may support exploration but not validation-backed constraint satisfaction.

Keep structural, fatigue, damage-tolerance, buckling, aeroelastic and flutter assessments distinct. CFD convergence does not prove structural adequacy. A simple aerodynamic screen cannot certify stall behavior, handling qualities or control authority.

### 4.5 Mission and reserve

The mission model consumes the exact mass/CG/propulsion/aero state and evaluates the actual chronological phase sequence. It must retain required reserve, alternate/diversion, go-around and failure recourse as explicit cases.

The evaluator returns one of: FEASIBLE_WITHIN_DECLARED_MODEL, INFEASIBLE, INDETERMINATE or NON_CONVERGED. These states are not airworthiness or operational-release decisions.

## 5. Coupled evaluation methods

Implement and compare more than one formulation:

- **Sequential/nested sizing:** disciplines are evaluated in a controlled order; simple and useful as a baseline but may hide coupling or require repeated outer loops.
- **MDF (Multidisciplinary Feasible):** an inner multidisciplinary analysis seeks consistency at each design point; optimizer sees a coupled feasible response when closure succeeds.
- **IDF (Individual Discipline Feasible):** discipline-level states are exposed with consistency constraints; useful to compare convergence behavior and optimizer coupling.
- **Fixed-point iteration:** transparent reference method for small deterministic contracts; must detect oscillation, divergence and iteration-limit failure.
- **Surrogate/multi-fidelity variants:** permitted only with applicability, calibration and uncertainty contracts; held-out evidence must remain blind to optimizer tuning.

No method is assumed universally best. The primary comparison is validated decision quality per total engineering cost and decision regret, not optimizer runtime in isolation.

## 6. Initial configuration matrix

All configurations share identical mission, payload, reserve and environmental definitions.

| ID | Architecture | Decisive question |
|---|---|---|
| C0 | Conventional direct-drive fixed-wing | What is the matched baseline after variant, options and reserve reconciliation? |
| C1 | High-aspect-ratio / strut-braced conventional propulsion | Does reduced induced drag offset structure, interference and inspection burden? |
| C2 | Partial-span blown lift + distributed propulsion | Does useful low-speed benefit survive installed mass, cruise drag, wiring and failure penalties? |
| C3 | Full-span blown lift + distributed propulsion | Is the incremental lift benefit worth the additional installed system and integration burden? |
| C4 | Battery-electric fixed-wing / DEP | Which declared missions close at adverse temperature and end-of-life? |
| C5P | Parallel hybrid: mechanically coupled primary engine plus electric boost/recuperation where supported | Does avoiding a full series conversion chain reduce installed mass/loss while retaining useful transient assistance? |
| C5S | Series hybrid: generator + battery + electric propulsion | Does dispatch flexibility and distributed propulsive control justify generator, motor, inverter, cable and cooling mass/loss? |
| C6 | Evidence-selected alternative geometry | Does any non-obvious topology win under the same constraints? |
| C7+ | VTOL alternatives | Is vertical access actually required, and does it justify hover/transition and certification penalties? |

The first decisive comparison should be C0 vs C2 vs C5P vs C5S. Parallel and series hybrid are distinct topologies; neither should be treated as a generic 'hybrid' score. For a two- or four-seat aircraft, the mass and efficiency penalty of the series electrical conversion chain may be decisive, while a parallel layout brings mechanical integration and control complexity of its own. C3 and VTOL remain in the portfolio but should not consume expensive high-fidelity resources until first-order screening shows a credible mission reason.

## 7. Benchmarks and evaluation protocol

### 7.1 Correctness

Create 12–20 hand-calculable synthetic cases for:
- mass closure and mass-convention conversions;
- CG and inertia, including moving payload/consumables;
- bus-power and energy residual;
- phase chronology and reserve;
- simple aerodynamic coefficient/force bookkeeping;
- coupled-iteration convergence and non-convergence;
- missing, stale, failed, out-of-domain and contradictory inputs.

Use a separately implemented oracle with independent expected values. Test units, sign conventions, reference frames, rounding and boundary behavior.

### 7.2 Hostile negative controls

At minimum:
- omit motor, inverter, wire, cooling, mount or containment mass;
- substitute cell-level energy density for installed pack/system evidence;
- double count fuel, storage energy, useful load or payload;
- import CG/inertia from a stale design generation;
- change datum, units, area or reference chord without updating transforms;
- mark peak power as continuous;
- ignore generator ramp/start time or storage power/thermal limits;
- skip go-around, reserve or diversion;
- carry nominal feasibility through a declared failure without a qualified model;
- use an aerodynamic result outside its admitted domain;
- omit failed/timed-out solver runs;
- turn coupling oscillation or iteration-limit failure into success;
- hide a hard constraint violation inside a weighted scalar objective.

The target is zero false-feasible outcomes on the locked bounded corpus. Report sample size and confidence bounds: zero observed synthetic errors is not proof of zero operational risk.

### 7.3 External design/solver baselines

Use domain-appropriate, independently pinned tools rather than writing replacement discipline solvers:
- OpenVSP / VSPAERO for geometry and suitable conceptual aerodynamic analyses;
- SU2 for selected CFD and adjoint cases within its supported scope;
- NASA Turbulence Modeling Resource cases for code/model verification and comparison;
- independently qualified structural/aeroelastic and propulsion analyses when available;
- OpenMDAO and comparable MDO formulations as optimization/orchestration baselines.

Separate solver verification, iterative/numerical convergence, grid/time convergence, model validation, experimental correlation and decision validity. NASA TMR provides turbulence-model and verification/validation cases; it is not an aircraft certification database. #3495 defines the applicability/abstention path.

### 7.4 Optimization baselines

On identical locked tasks and evaluation budgets, compare:
1. sequential engineering and one-factor-at-a-time;
2. deterministic grid/space-filling sweeps;
3. gradient/adjoint optimization where valid;
4. evolutionary multi-objective methods;
5. Bayesian and multi-fidelity methods;
6. MDF and IDF formulations;
7. Symthaea-directed value-of-information / experiment-selection policies.

Report feasibility and false-feasible counts with confidence bounds, feasible Pareto hypervolume and regret against an exact small-case oracle, worst-decile constraint margin, uncertainty calibration and selective coverage/abstention, number and cost of high-fidelity evaluations, failed/timeout/divergent analysis count, wall time, CPU/GPU/energy where measured, human engineering hours, cost per independently reproduced decision, and held-out geometry/mission/condition generalization.

Symthaea receives credit only if it improves the decision under the same physics, data, tool access, compute and total evaluation budget. Its suggestions do not become verified facts.

## 8. Aspirational programme gates

Targets below are internal research goals, not aviation standards or achieved results.

| Gate | Research target |
|---|---|
| Oracle correctness | 100% agreement on the frozen finite synthetic corpus |
| False-feasible | Zero on locked synthetic/hostile cases; unknown stays indeterminate |
| Configuration identity | 100% of material discipline outputs bind exact aircraft + mission digests |
| Closure | Explicit mass, energy, thermal and coupling residuals; no hidden residual or missing mass |
| Failed analyses | 100% retained and counted in the run ledger |
| Mission improvement | Seek at least 10% improvement in one preregistered primary mission-level metric on at least one frozen mission, with no hidden hard-constraint violation or material unreported regression |
| Decision efficiency | Test whether Symthaea/Symtropy reduces total cost to the same independently validated decision quality; do not claim improvement without matched-budget results |
| Generalization | Preserve unseen geometry/missions/conditions; report performance on each family rather than only pooled averages |

The 10% target is a development ambition, not an industry benchmark or prediction. If no candidate clears it, publish that result and explain why.

## 9. What could differentiate AER-01

The most promising differentiation is not a particular component. It is a set of measurable system advantages:

1. **Hot/high mission robustness:** useful payload and required climb/obstacle performance under declared high-density-altitude conditions, with margin and uncertainty.
2. **Partial-span powered lift only if it earns its mass:** a matched C0/C2 test with installed hardware, cruise drag, noise, thermal and failure effects included.
3. **Lifecycle simplicity:** measured inspection, diagnosis, replacement, configuration verification and return-to-service effort.
4. **Graceful degradation:** explicit mission outcomes for power-source, propulsor, inverter, bus and sensor failures within a declared safety-analysis scope.
5. **Evidence-native engineering:** all claims bound to the exact geometry, mass state, model domain, source, result, uncertainty and reviewer/authority status.

A credible aircraft may ultimately use conventional propulsion and a relatively simple airframe. Rejecting a fashionable idea when it fails the whole-system comparison is a programme success.

## 10. Immediate execution sequence

### Now — no runner required
- Freeze the interface and unit conventions.
- Produce a hand-worked mass/CG/inertia fixture set.
- Build one C0 conventional and one C2 partial-blown-lift synthetic case.
- Specify the #3900 and #3903 input/output adapters and shared identity digests.
- Define the closure state machine and failed-run retention.
- Write an independent oracle before production optimizer integration.

### After the bookkeeping oracle
- Integrate phase-resolved dispatch and installed-system battery evidence.
- Add first-order mission and trim coupling with transparent assumptions.
- Run a deterministic configuration sweep and compare with fixed-point/MDF/IDF variants.
- Use decision-value selection to choose which candidate deserves a higher-fidelity solve.

### Only after a useful candidate survives
- Acquire exact manufacturer POH/AFM and configuration data for baselines.
- Promote selected aerodynamic cases to pinned, independently checked solvers.
- Add structural/aeroelastic and thermal analyses with qualified experts.
- Define a separate, reviewed low-consequence component-test programme.

## 11. Certification, safety and evidence boundary

For South Africa, the SACAA Airworthiness/Engineering material is the primary official starting point for the applicable local pathway. Its published guidance includes Type Certification, Experimental Certificate of Airworthiness, Design Organisation Approval and related processes. FAA Part 23, EASA CS-23 Amendment 6 / AMC & GM Issue 5, ASTM F3264 and the SAE ARP4754B / ARP4761A development and safety-assessment practices are useful technical references only where applicable and accepted by the competent authority.

Certification basis, means of compliance, conformity, design approval, production approval, maintenance privileges and operational authorization remain authority- and applicant-specific. AER-01's analysis software cannot make those decisions.

## 12. Primary public references

- Electra EL9 FAA certification-basis milestone, 10 July 2026: https://electra.aero/news/electra-achieves-faa-certification-milestone-for-el9-ultra-short-aircraft
- DLR HYDRA, 21 August 2026: https://www.dlr.de/en/latest/news/2026/hydra-aircraft-concept-takes-first-place-in-the-dlr-design-challenge-2026
- NASA truss-braced structural wing test, 17 July 2026: https://www.nasa.gov/centers-and-facilities/armstrong/nasa-pushes-new-wing-design-to-find-structural-limits/
- NASA Turbulence Modeling Resource: https://tmbwg.github.io/turbmodels/
- GAO-26-107816, *Electric Aircraft: FAA Is Evaluating Designs for Certification and Considering Long-Term Regulatory Approaches*, 27 May 2026: https://www.gao.gov/products/gao-26-107816
- EASA CS-23 publication register, including Amendment 6 / AMC & GM Issue 5, published 15 April 2026 and corrected 5 May 2026: https://www.easa.europa.eu/en/document-library/certification-specifications/group/cs-23-normal-utility-aerobatic-and-commuter-aeroplanes
- SACAA Airworthiness / Engineering: https://www.caa.co.za/industry-information/airworthiness/
- SAE ARP4754B, revised 20 December 2023: https://saemobilus.sae.org/standards/arp4754b-guidelines-development-civil-aircraft-systems
- SAE ARP4761A, revised 20 December 2023: https://saemobilus.sae.org/standards/arp4761a-guidelines-conducting-safety-assessment-process-civil-aircraft-systems-equipment
- ASME V&V / VVUQ standards overview, including V&V 20 and VVUQ 20.1-2024: https://www.asme.org/codes-standards/publications-information/verification-validation-uncertainty

**Claim ceiling:** This contract and its synthetic tests can qualify the software bookkeeping, identity and bounded closure behavior. They do not establish physical aircraft performance, structural strength, flutter margin, battery safety, handling qualities, flightworthiness, type certification or commercial superiority.
