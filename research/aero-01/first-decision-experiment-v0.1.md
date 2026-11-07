# AER-01 First Decision Experiment — Wren 2 and Wayfarer 4

**Status:** Research protocol / mission cards not yet frozen  
**Parent:** [AERO-AIRFRAME-001 #3921](https://github.com/Luminous-Dynamics/symtropy/issues/3921)  
**Closure contract:** [AERO-WHOLE-AIRCRAFT-CLOSURE-001 #3945](https://github.com/Luminous-Dynamics/symtropy/issues/3945)  
**Underlying civil-aero implementation:** [AERO-CIVIL-001 #2878 / draft PR #2880](https://github.com/Luminous-Dynamics/symtropy/pull/2880)  
**Mass-properties implementation:** [draft PR #3977](https://github.com/Luminous-Dynamics/symtropy/pull/3977), stacked on #2880  
**Date:** 2026-11-07

> This protocol defines what to measure and how to compare candidates. It is not a selected aircraft design, approved mission requirement, performance prediction, safety finding, or authorization to construct or fly an aircraft.

## 1. The first product decision

Compose—not replace—Symthaea [AIRFAMILY-001 #7745](https://github.com/Luminous-Dynamics/symthaea/issues/7745):

- **Wren 2:** first product-family candidate; two-seat training, local flying and personal transport.
- **Wayfarer 4:** separate four-seat touring/utility mission; do not force Wren 2 geometry, propulsion or certification assumptions onto it.
- **Wren-E:** gated electric-trainer demonstrator. It advances only if the actual sortie duration, reserve, charging turnaround, battery replacement, adverse-temperature operation and ownership economics close.
- **Ranger 4:** conditional utility/backcountry derivative only if a distinct mission justifies its own loads, gear, structure and operating envelope.

Reuse the existing Symthaea maintenance-task-card validator (#7750) and transparent ownership-cost model (#7754). Reuse the AER-01 / civil-aero analysis and evidence infrastructure. Do not create duplicate maintenance authority, ownership economics, CFD or battery-dispatch systems.

**The first engineering priority is Wren 2**, not because it is necessarily the final best product, but because it is the recommended first product candidate in the existing aircraft-family work and provides a bounded place to validate the process.

## 2. Freeze missions before optimizing

The six initial machine-readable mission templates are in [mission-cards-v0.1.json](./mission-cards-v0.1.json). They intentionally leave unverified numbers null and record evidence status; they are input templates, not frozen requirements.

Do not populate these cards from marketing-page headlines. Each value must be recovered from a controlling source, independently checked or marked Unknown.

### Wren 2 mission card

| Field | Required contract |
|---|---|
| Mission classes | Training sortie; local/personal flight; two-seat cross-country |
| Payload | Pilot, second occupant, baggage and mission equipment separately |
| Performance | Takeoff/landing ground roll and obstacle distances; climb; approach/stall speeds; cruise; ceiling |
| Energy/fuel | Exact fuel or installed battery state; phase-resolved demand; reserve; diversion; end-of-life/temperature derating |
| Environment | ISA baseline plus declared hot/high temperature and pressure-altitude scenarios; wind/gust/crosswind; runway surface/slope/contamination |
| Operation | Sortie duration, annual utilization, turnaround, dispatch reliability and maintenance availability |
| Economics | Cost per completed training sortie and per flight hour, with capital, owner/maintainer time, energy, maintenance, replacement and downtime shown separately |

### Wayfarer 4 mission card

| Field | Required contract |
|---|---|
| Mission classes | Four-seat touring; practical utility; short-field destination access |
| Payload | Four occupant positions, baggage and usable load distribution |
| Performance | Payload-range-reserve; obstacle-clear takeoff/landing; climb at declared mass and density altitude; cruise and diversion |
| Energy/fuel | Exact mission chronology, source dispatch, reserves, fuel/battery states and contingencies |
| Environment | Hot/high, crosswind, surface condition, obstacle, temperature and alternate/diversion cases |
| Operation | Typical leg duration, annual utilization, turnaround, maintenance and parts availability |
| Economics | Cost per completed mission and passenger-kilometre, plus dispatch availability and downtime |

### Mandatory rules

1. A missing number remains Unknown; it is not a hidden nominal default.
2. A ground-roll number cannot be substituted for an obstacle-clear distance.
3. Manufacturer maximum range cannot be compared with another aircraft's range unless payload, power setting, altitude, fuel/energy, weather and reserve are matched.
4. Occupant capacity is not the same as usable payload.
5. Takeoff and landing CG/inertia must be recomputed for their own fuel, occupant and baggage states.
6. A successful nominal scenario cannot stand in for the worst declared hot/high, reserve or failure case.
7. Freeze one primary metric per mission before revealing optimizer results. Publish all other metrics and constraints; never use one weighted scalar to hide a safety or payload violation.

## 3. Baseline portfolio

These public values are source-acquisition leads only. They are not a normalized aircraft comparison and are not approved manual data.

### Wren 2 — two-seat lane

- **Sling 2:** locally relevant two-seat reference. Manufacturer page: https://slingaircraft.com/sling-2/
- **Evektor SportStar RTC:** conventional two-seat certified-trainer comparator. Manufacturer page: https://www.evektor.com/sportstar-rtc
- **Pipistrel Velis Electro:** type-certified battery-electric trainer; manufacturer publishes 428 kg empty mass with batteries, 600 kg MTOW, 172 kg useful load, 57.6 kW maximum takeoff power, approximately 50 minutes typical endurance plus VFR reserve, and a nominal 1 h 20 min charge time. These are published values; current manual revision, environmental limits and operating experience must be recovered before a matched comparison. Sources: https://www.pipistrel-aircraft.com/products/velis-electro/ and https://www.pipistrel.eu/products/general-aviation/velis-electro/

The Velis is a local-training energy-service comparator, not a long-range touring peer. Measure completed sorties per day, turnaround, battery degradation/replacement, charging availability and dispatch rate—not just Wh/kg or range.

### Wayfarer 4 — four-seat lane

- Cessna Skyhawk: conventional four-seat owner/trainer baseline.
- Diamond DA40 NG: modern fixed-wing efficiency/payload comparator.
- Tecnam P2010: four-seat touring comparator; reconcile its page headline and detailed speed values before freezing any baseline.
- **Sling TSi:** manufacturer publishes 430 kg useful load, 155 KTAS typical cruise, 1,000 nm maximum range, 330 m takeoff and 350 m landing over a 50 ft obstacle. Source: https://slingaircraft.com/aircraft/sling-tsi/
- **Sling High Wing:** manufacturer publishes 490 kg useful load, 145 KTAS typical cruise, 900 nm maximum range, 810 m takeoff and 550 m landing over a 50 ft obstacle. Source: https://slingaircraft.com/aircraft/sling-hw/

Sling TSi and Sling High Wing are particularly relevant South African market and engineering comparators. They are not proof that their configurations are optimal or that AER-01 can outperform them. Recover exact manuals, aircraft configuration, approval/registration basis, option weights, performance definitions and environmental correction methods.

The baseline acquisition file remains the source of truth for the preliminary table and its unresolved mismatches: [baselines-v0.1.md](./baselines-v0.1.md).

## 4. First architecture experiment

The first down-selection is deliberately small:

| Candidate | Architecture | What would falsify it? |
|---|---|---|
| C0 | Conventional fixed-wing, direct-drive reference | Does not meet the frozen mission/ownership requirement, or a challenger demonstrates a robust whole-mission gain |
| C1 | Higher-aspect-ratio / strut-braced wing with conventional propulsion | Structural mass, drag, gust/flutter/inspection burden or manufacturability negates cruise benefit |
| C2 | Partial-span blown lift / distributed propulsion | Installed motors, inverters, cabling, cooling, mounts, failure groups and cruise drag erase the short-field mission benefit |
| C3 | Full-span blown lift / DEP | Incremental powered-lift benefit fails to repay its larger integration burden |
| C4 | Battery-electric fixed-wing / DEP | End-of-life energy/power, adverse temperature, reserve, payload or turnaround fails the declared sortie |
| C5P | Parallel hybrid | Mechanical coupling, controls, integration and maintenance costs outweigh the mission benefit |
| C5S | Series hybrid | Generator, motor, inverter, cable, cooling and conversion losses/mass outweigh the dispatch flexibility benefit |
| C6 | Evidence-selected alternative geometry | No held-out robust benefit over simpler architectures |
| C7+ | VTOL alternatives | Evaluate only against a mission with an actual vertical-access requirement; never import hover-system burden into the eSTOL baseline |

First compare **C0, C1, C2, C5P and C5S** on one frozen Wren 2 mission and one frozen Wayfarer 4 mission. Keep C3/C4/C6/C7 in the portfolio, but promote them to expensive analysis only when a first-order mission screen shows a credible reason.

Do not assume that a generator sized for cruise plus battery peak-buffering is optimal. Compare parallel and series hybrids against direct drive and battery-electric under the same installed-mass, reserve, thermal, service and failure boundaries. #3900 owns phase-resolved dispatch; #3903 owns battery/capacitor evidence normalization.

## 5. Coupled analysis ladder

1. **Source acquisition:** approved manual/variant data, mass conventions, engine/propeller maps, energy-system evidence, operating environment and maintenance schedules.
2. **Mass properties:** itemized installed mass, CG and inertia at takeoff, landing and relevant mission/failure states. Unknown items block complete closure. Current initial code is #3977.
3. **Mission screen:** low-order aerodynamic and energy calculations with explicit omissions and a screening-only claim ceiling. Reuse the existing civil-aero-benchmark crate.
4. **Hybrid/energy closure:** #3900 phase dispatch and failure states; #3903 installed battery data and end-of-life/adverse-temperature envelopes.
5. **Aerodynamic qualification:** TMR NACA 0012 narrow reference, then selected 3D DPW-8/AePW-4 tasks. A 2D airfoil fit is not a wing or aircraft validation.
6. **Structural/aeroelastic/thermal:** independently qualified analyses on candidates whose aerodynamic and mission benefits justify their cost.
7. **MDO:** compare sequential, MDF, IDF, dynamic workflow selection and multi-fidelity strategies using exact same candidate and mission identities.
8. **Manufacturing/serviceability:** only after a promising configuration survives model screening; first measure non-flight coupons and serviceability mock-ups, then use separately reviewed low-consequence component tests.

Do not run expensive CFD simply because it is available. Use value-of-information/EVSI to identify which test or analysis is most likely to change the architecture decision.

## 6. What would count as better than the state of the art?

The target is not one record-setting L/D or ground-roll number.

### Aircraft-level outcomes

- Payload-range-reserve feasibility under identical conditions.
- Ground roll and obstacle-clear distance, with exact runway/environment basis.
- Climb/ceiling/go-around/diversion margins at declared hot/high conditions.
- Energy/fuel per completed mission and passenger-kilometre.
- Installed-system mass and power density, not cell or motor headline values.
- Thermal headroom, end-of-life performance, power-source failures and reserve survival.
- Scheduled/unscheduled labor, diagnosis and replacement time, parts fill rate, downtime and first-time-fix.
- Lifecycle cost per completed mission, with capital/owner time/maintenance/replacement/infrastructure assumptions separated.
- Noise, emissions and operating constraints as independent axes.

### Engineering-method outcomes

Compare on identical locked tasks, information and total evaluation budgets:
- sequential sizing and deterministic sweeps;
- gradient/adjoint methods where differentiability and solver support are real;
- evolutionary and Bayesian/multi-fidelity optimization;
- OpenMDAO reference MDO formulations;
- ShapeBench and BOCoDe task subsets with compatible licences and exact adapter versions;
- Symthaea-directed experiment selection and architecture proposals.

Record high-fidelity and surrogate calls, failed/aborted runs, human engineering hours, compute/energy where actually measured, tuning cost, uncertainty calibration, out-of-domain abstention, worst-decile constraint margin, Pareto hypervolume, decision regret and cost per independently reproduced decision.

Do not claim a Symthaea gain unless physics, data access, solver fidelity, information and total budgets are matched. The current ShapeBench preprint reports 103 aerodynamic optimization tasks across eight categories and a mean pairwise optimizer-ranking Spearman correlation of approximately 0.013, demonstrating that a win on one task does not establish cross-domain generality. Source: https://arxiv.org/abs/2605.20763

The BOCoDe 2026 preprint describes 307 optimization tasks, including 159 engineering tasks, and evaluates 31 Bayesian/evolutionary methods. Its core warning is also relevant: rankings do not reliably transfer from synthetic/HPO tasks to engineering design. Source: https://arxiv.org/abs/2608.15073

OpenMDAO is a serious comparator, not an implementation target to dismiss: its project emphasizes analytic derivatives, gradient-based MDO, parallel analysis and design-space exploration. The current documentation lists version 3.45.2 released 27 October 2026. Sources: https://openmdao.org/docs/ and https://openmdao.org/newdocs/versions/latest/

## 7. Paradigm shifts to test

1. **Architecture is a design variable.** Do not select series hybrid, distributed propulsion or a wing topology before optimization begins. Search architecture families and geometry jointly; charge every added component and failure dependency.
2. **The mission is the unit of truth.** Optimize useful completed missions and lifecycle economics, not isolated lift, drag, battery or motor metrics.
3. **Evidence has a domain.** Every prediction carries its solver/model identity, operating regime, uncertainty and applicability. Unknown/out-of-domain results abstain.
4. **Active learning is a design instrument.** Select experiments by expected decision value, not by convenience or the largest uncertainty number alone.
5. **Passive safety before active complexity.** Use geometry and stable load paths to avoid needing actuators/software to rescue a poor physical architecture.
6. **Serviceability is part of the product.** Inspection access, standard parts and replaceability count only when measured and when the approval/configuration case permits them.
7. **Commonality is optimized, not maximized.** Reuse interfaces and parts only when lifecycle cost and mission fit improve. Wren 2 and Wayfarer 4 can legitimately have different wings, engines and certification bases.
8. **The product includes the ground system.** Charging, spares, qualified maintenance, field support and energy availability can decide whether an electric aircraft completes more useful missions.

A 2026 study on dynamic MDAO workflows explores changing design variables, disciplines, tools and constraints during the workflow itself. This is a useful direction for configuration/architecture search, not proof that a dynamic workflow is universally superior. Source: https://doi.org/10.1016/j.ast.2026.111748

## 8. Preregistered decision and falsification rules

Before optimizer results are inspected:
- freeze the mission card, primary metric, reserve policy and hard constraints;
- freeze the baseline variant and evidence class;
- freeze the design-space bounds and categorical architecture options;
- freeze total optimizer/analysis budget and stop conditions;
- freeze held-out geometry, mission and operating-condition splits;
- freeze the physical/model validation gates relevant to the claim.

Internal development target: seek at least 10% improvement in one preregistered mission-level primary metric on at least one frozen mission, without hard-constraint violation or material unreported regression in payload, reserve, safety, serviceability or cost. This is not an aviation standard or current result.

**Falsification is a deliverable.** Publish when C0 wins, when C2's added propulsion is not worth its mass, when parallel is better than series, when the battery-electric sortie does not close, and when an optimizer's apparent gain disappears at higher fidelity.

## 9. South African certification decision

SACAA publishes separate Technical Guidance Materials for Type Certification, Type Acceptance, Design Organisation Approval, Experimental Certificate of Airworthiness and Amateur-Built Aircraft. The applicable product/applicant pathway must be discussed with the Engineering Unit before physical design freeze; an amateur-built route must not be presumed to support a future commercial product. Official guidance: https://www.caa.co.za/industry-information/airworthiness/

## 10. Current delivery status

- AER-01 architecture and baseline brief: draft PR #3922.
- Whole-aircraft closure contract and 12 synthetic fixtures: included in #3922.
- Whole-aircraft mass-properties Rust module/CLI and independent oracle: draft stacked PR #3977; exact-head Rust tests, strict Clippy, CLI oracle and inherited civil-aero workflows passed.
- Existing civil-aero screen and optimizer/CFD evidence infrastructure: draft PR #2880; its exact head had successful reported CI workflows on 7 November 2026.
- No Wren 2/Wayfarer 4 mission card is frozen yet because controlling POH/AFM and aircraft-specific inputs have not been acquired.
- Takeoff loading state now couples into the steady-cruise screen in draft PR #4025, with exact-head tests; phase chronology and fuel-burn transitions are explicitly deferred to #4029.
- No complete aircraft design has been selected; no real-aircraft CFD, structural, thermal, wind-tunnel or flight result is claimed.

**Claim ceiling:** This is a controlled research and decision protocol. It is not evidence that AER-01 is superior to any existing aircraft, safe, flightworthy, certifiable, commercially viable or ready to build.
