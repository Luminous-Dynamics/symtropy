# Service-First Washing Machine Reference Architecture v0.1

**Status:** architectural study and test plan; no CAD freeze, prototype or physical result exists.  
**Tracker:** [APPLIANCE-WASHER-001 #1828](https://github.com/Luminous-Dynamics/symtropy/issues/1828)  
**Portfolio:** [APPLIANCE-PORTFOLIO-001 #1825](https://github.com/Luminous-Dynamics/symtropy/issues/1825)  
**Parent substrate:** [APPLIANCE-QUAL-001 #1805](https://github.com/Luminous-Dynamics/symtropy/issues/1805)  
**Evidence destination:** [Mycelix APPLIANCE-EVID-001 #5156](https://github.com/Luminous-Dynamics/mycelix/issues/5156)

## 1. Hypothesis

A washer can be made easier and cheaper to maintain by treating serviceability as an explicit product requirement: independently accessible functions, repairable wear interfaces, offline diagnostics and verified restoration—with each module boundary justified against the cost of fasteners, connectors, seals, tolerances, inventory and discarded material.

The project must not assume that every module improves economics. A large replaceable tub/wet-core may save labour but discard expensive, still-good parts; a separate bearing cartridge can save material but add precision assembly, seal failure and vibration risk. Those are falsifiable architecture choices.

**First target:** a front-load, cold-fill household washer of a capacity class chosen after buying at least three comparable retail controls. Target market and intended-use profile remain open until the control set is selected.

## 2. Paradigm shifts

### 2.1 Welded/hidden assembly -> measured service interfaces

Give the technician and approved service task access to the filter/pump, water valves, door lock, heater, sensor/harness, controller/drive, suspension and drum support without destructive cosmetic disassembly. Compare the existing product's architecture against the proposed access path using actual disassembly/reassembly time and failure-mode evidence.

### 2.2 Replace a large assembly -> replace the smallest justified repair unit

For the bearing/tub interface, compare:
- replace a complete tub/drum assembly;
- replace a bounded bearing/seal/hub cartridge where the outer tub supports the required load and sealing;
- repair or renew only a defined wear part if tooling and precision checks are repeatable.

The target is not “always replace the smallest part.” Select the smallest unit for which safe diagnosis, repeatable installation, available parts, leak integrity, vibration, lifetime cost and post-repair verification all close.

### 2.3 Commanded programme -> measured cleaning result

Compare every variant under the same load/soil/water hardness/detergent/profile and evaluate cleaning, rinsing, spin-drying, cycle time, noise, water and electricity. Lower electricity use is not a win if clothes are less clean, residues increase, or the cycle silently becomes longer than the accepted profile.

### 2.4 Service code -> useful test sequence

Symthaea may suggest the next test under the existing shared finite-hypothesis EIG/EVSI kernel. A diagnostic path may include:
1. safe/non-invasive symptom and sensor-quality checks;
2. distinguish inlet, drain, imbalance, heating and control hypotheses;
3. recommend only tests eligible for the current operator and protection state;
4. report what evidence would distinguish hypotheses and when it is insufficient;
5. after repair, require exact-unit, exact-protocol function verification.

A code or model hypothesis does not establish a root cause. A spin command does not establish the drum spun correctly. A completed work order does not establish restored washing function. Diagnostics never override door locks, over-temperature, leakage protection or other independent protective devices.

### 2.5 Scheduled maintenance -> transparent, task-specific service

Routine user-maintenance guidance should make the filter, dispenser, door seal and permitted inlet screens obvious to inspect and clean. It should state water-capture, electrical isolation, stop conditions and reassembly checks where relevant. Internal electrical, heater, heavy rotating assembly and protective-electrical testing remain qualified tasks unless a separate product-specific safety assessment explicitly justifies another service tier.

## 3. Proposed functional decomposition

This is a logical map, not a frozen BOM.

| Function | Candidate module/boundary | Serviceability question | Required verification |
|---|---|---|---|
| Structural support | Cabinet/frame, feet, suspension mounts | Can the cabinet be opened/reassembled without damaged clips, crushed cables or lost alignment? | Stability, vibration, torque/fastener checks |
| Water intake | Inlet valve(s), hose connection, filter, water-level/flow observation | Can a leak, restricted inlet and faulty valve be distinguished? | Water shutoff, leak and fill/time profile |
| Detergent delivery | Dispenser and inlet passages | Can residue/blockage be cleaned without opening hazardous internals? | Correct delivery, leak and cycle profile |
| Wash vessel | Outer tub, drum, seals and bearing support | Is bearing/seal replacement possible without a whole tub swap? | Static leak, loaded spin, vibration/noise and fatigue checks |
| Drain | User-accessible filter, sump path, drain pump and outlet hose | Can blockage be cleared and pump failure distinguished without moving or dismantling most of the machine? | Drain time/remaining water, leak and pump test |
| Heat | Heater, thermal sensors and independent thermal protection | Can a heating fault be diagnosed and repaired without undermining independent limits? | Qualified electrical/thermal verification and controlled cycle |
| Drive | Motor, belt or direct-drive coupling, inverter/drive electronics | Which topology minimizes whole-life cost at matched cleaning/spin outcomes? | Speed, load, vibration, acoustic and efficiency measurements |
| Suspension/imbalance | Dampers, springs, load/imbalance observations | Can ordinary wear parts be changed and balance correctly restored? | Guarded loaded spin and stability checks |
| Control | Controller, user interface, harness, sensors | Does it operate offline and preserve exact configuration/service records? | Offline operation, sensor fault injection and safe fault response |
| Service evidence | Manuals, part/revision identities, task/test protocol | Can an independent repairer identify parts and qualify a substitute? | Part fit/rating checks and post-repair capability report |

### Interface contract

Every serviceable module needs:
- stable part/module identifier and revision;
- mechanical envelope, mounting/datums, fastener/tool list and allowed substitution rules;
- electrical ratings, connector/pinout, polarity, temperature, current, insulation/protective-earth and relevant environmental requirements;
- water/hose/material/pressure/temperature limits and seals where applicable;
- harness routing, strain relief, splash isolation, service/disconnection sequence and residual-energy warnings;
- fault modes it can/cannot diagnose, compatible firmware/configuration if relevant, and exact post-installation verification;
- original/third-party source and licence information in the open design package.

A common document schema is reusable across appliance families; a common physical connector or generic voltage is not assumed. Unknown interface compatibility blocks substitution.

## 4. Architecture variants to compare

Machine-readable variant identities and status are maintained in [`fixtures/appliances/large_appliance_repairable_portfolio_v0_1.json`](https://github.com/Luminous-Dynamics/symtropy/blob/research/large-appliance-repairable-architecture-v0-1/fixtures/appliances/large_appliance_repairable_portfolio_v0_1.json): `W0-RETAIL-CONTROL`, `W1-ACCESSIBLE-CONVENTIONAL`, `W2-SERVICE-SPINE`, `W3-BEARING-TUB-TRADE`, and `W4-DRIVE-TOPOLOGY`. They are hypotheses with required measurements and named failure modes—not measured scores or a ranking of completed designs.

### W0 — retail control

At least three comparable commercial units where practical. Capture model/revision, year/condition, rated capacity, programme settings, public manuals/parts documentation, price/date/market and data gaps. Do not silently treat a well-documented premium model as equivalent to a low-cost model without a declared comparison question.

### W1 — accessible conventional layout

Preserve conventional function architecture but improve reversible service access, fastener/clip visibility, service documentation, removable filter access, labelled connectors and post-repair test procedures. This tests whether serviceability improves mostly through layout/documentation rather than high-modularity design.

### W2 — service-spine layout

A dedicated front/side service path for filter/pump, inlet valve, dispenser, door lock, heater/sensor harness, controller/drive assemblies and test access. Wet paths and mains/control boundaries are documented and segregated. Every module replacement is measured against component-level repair and against W1.

### W3 — tub/bearing architecture variants

Keep external size/capacity and functional test profile matched while comparing conventional sealed/welded tub replacement with bolted/serviceable outer-tub and separate bearing/seal support. Measure manufacturing cost, part inventory, joint count, seal integrity, noise, vibration, repeat repair probability, assembly time and material discarded. A bolted design is not selected if field-like loaded spin/leak reliability suffers.

### W4 — drive variants

Compare belt-driven and direct-drive candidates only if both can be sourced and tested at matched useful function. Include controller/inverter cost and repairability, belt/coupling wear, bearing load, balancing, acoustics, vibration and part availability. No topology is assumed to be superior before data.

### W5 — optional resource innovations (later, not first prototype)

- load/soil sensing to reduce water/heat only when validated against cleaning/rinsing outcomes;
- adaptive water temperature and spin profiles;
- external hot-water input only for specifically approved installation profiles with verified temperature control, mixing/scald and backflow safeguards;
- drain-water heat recovery or water reuse only after hygiene, fouling, energy, plumbing, contamination, service and economic analyses.

Do not add water recycling, elaborate sensors, network connectivity or proprietary app features to the first design unless they demonstrate net value. Each subsystem can add faults, validation cost and service burden.

## 5. Service role tiers

These tiers are a project vocabulary, not a legal classification or blanket authorization.

**U0 — ordinary user maintenance.** Clean accessible filter and dispenser, inspect/clean door seal, inspect external hoses, follow indicated descaling/maintenance (where intended), read user diagnostics and perform documented non-invasive checks.

**U1 — specific competent-owner tasks only if individually reviewed.** A task may be promoted here only after an explicit task-level safety assessment, safe isolation method, tool and competence requirements, likely failure modes, reassembly validation and clear stop conditions. Internal pump, valve or harness work is not automatically U1 merely because the panel can be removed.

**P — qualified repair.** Internal mains/drive/heater/controller work; heavy tub/drum/bearing/suspension repair; electrical protective checks; controlled load spin; leak verification after internal work; any work requiring specialist tools/measurement or where the residual risk is not suitable for user maintenance.

User experience goals must be measured only against U0 and specifically qualified U1 tasks. Do not advertise internal repair as DIY-safe based on appearance.

## 6. Fault and repair benchmark curriculum

Start with controlled non-hazardous or simulated traces to qualify the contract; physical fault introduction is a separately risk-reviewed lab activity. Never run an energised machine with guards removed, intentionally introduce unsafe leakage/mains exposure, bypass a door lock, or create a dangerous imbalance for a casual test.

Candidate fault families:
- inlet restriction / inlet valve fault / no-fill symptom;
- drain filter blockage / drain path restriction / pump fault;
- dispenser blockage or delivery discrepancy;
- door lock/closure sensor discrepancy;
- temperature sensor vs heater fault (qualified service only);
- motor/drive/speed feedback discrepancy;
- suspension/imbalance/vibration discrepancy under protected, controlled procedures;
- leakage detection / seal or hose observation;
- sensor stuck, missing, stale, conflicting units or impossible value;
- fault-code-only evidence with physical output still unobserved;
- recurrence after apparently successful repair;
- incompatible substitute part or wrong firmware/configuration identity.

For each trace define hypothesis set and priors (clearly labelled synthetic until calibrated), available test likelihoods, operator eligibility, protection state, measurement units/freshness, expected abstention or next test, test cost units and exact input/model digest. For physical trials, the likelihood/cost model must be estimated or validated from traceable held-out evidence; repair-shop records alone do not necessarily represent random field failures.

## 7. Baseline and preregistration protocol

### Phase A — discovery, no CAD freeze

1. Select target market, capacity and installation profile.
2. Obtain at least three matched commercial controls and exact documentation/parts records where feasible.
3. Interview competent repairers and users using a fixed prompt; record themes, not claims of statistically representative prevalence.
4. Record observed failure/repair barriers and the source/sample scope of every record.
5. Freeze candidate variants and identify hazards, lab capability and a product-specific safety review path.

### Phase B — common-condition function measurements

Freeze detergent, dry textile type/mass, soil challenge, load size, water hardness, supply water temperature, programme/settings, ambient conditions and repeat count. Calibrate instruments and record uncertainty. Measure:
- cleaning and rinse outcomes under a declared method;
- water per cycle and per successfully cleaned kg;
- electricity per cycle and per load;
- cycle duration and spin-drying outcome;
- sound/vibration, leaks, temperature and any relevant safety observation;
- missing data, rejected runs, deviations and raw input digest.

A performance metric is not admissible if the candidate and control workloads, outputs or exclusion rules differ materially.

### Phase C — repair trials

Preregister a fault-task set and task script. Record:
- time for access, diagnosis, parts sourcing, waiting, repair, reassembly, post-test and cleanup;
- every tool, special jig, destructive action, damaged fastener and access obstacle;
- planned versus actually consumed/replaced parts, part price/date/source and delivery time;
- repair success by fault class, failed attempts, rework, residual faults, total outage, discarded mass;
- exact unit, protocol revision, instrument calibration and test result;
- recurrence during a bounded follow-up window with pending/complete/failed/inconclusive state.

Do not count only hands-on wrench time. Report median and P90 *total restoration time*, including part and service delay.

### Phase D — architecture decision

Compare W1/W2/W3/W4 results against W0. Keep function, energy/water, repair success, safety and lifecycle cost separate. Choose a more modular design only if the net advantage persists after realistic module prices, connector/seal failure, inventory, availability and rework sensitivity. Use Pareto frontiers when one architecture trades lower resources for higher cost or longer cycle time; do not hide the trade in an undocumented scalar.

### Machine-readable measurement plan

The exact measurement inventory and preregistration gates are also represented in [`fixtures/appliances/large_appliance_repairable_portfolio_v0_1.json`](https://github.com/Luminous-Dynamics/symtropy/blob/research/large-appliance-repairable-architecture-v0-1/fixtures/appliances/large_appliance_repairable_portfolio_v0_1.json), under `washer_benchmark_protocol`, checked against the portfolio schema and shared audit utility. It currently remains `draft_pending_control_selection`: controls, workload variables and instruments are not yet selected, and service trials are not scheduled.

The protocol uses three gates: (A) freeze control selection and measurement method before any measurements; (B) run a control-only pilot to estimate repeatability and instrument uncertainty; (C) freeze numeric functional non-inferiority margins, exclusions and analysis before collecting candidate-unit measurements. Unknown values remain unknown, not zero. Cleaning, rinsing and spin-drying quality are gates on whether a load counts as a successful output; electricity/water per cycle alone is not a valid superiority claim.

The machine-readable metric set keeps separate 11 dimensions: cleaning, rinsing, spin-drying, water per successful load, energy per successful load, cycle duration, total restore time, repair cost, recurrence, critical safety findings, and discarded material. These are methods-to-freeze, not already measured outcomes.

## 8. Proposed acceptance targets (hypotheses, not results)

- At least 25% lower median and P90 total restoration time on a preregistered common-fault set versus W0, without unsafe outcomes or functional regression.
- At least 90% successful completion of approved U0 maintenance tasks in a representative usability test.
- At least 90% first-visit repair success for the separately scoped qualified-service task set; report each critical fault class separately.
- Matched cleaning/rinse/spin quality within a preregistered non-inferiority margin before claiming energy or water superiority.
- A positive lifecycle advantage after including module/connector cost, spare parts, freight, labour, failed attempts, rework, water/energy, downtime and discarded material; report sensitivity ranges.
- Zero hazardous next-test/repair recommendations in the finite adversarial diagnostic suite; tripped/unknown protection and unreliable observation state must cause abstention/referral.
- At least three prototype units for initial repeatability estimates; at least one independent builder/test team before a stronger reproducibility claim.
- A 10-year parts-availability goal for the chosen open design, presented as a project commitment hypothesis and checked against supply-chain cost/market law before publication.

These thresholds require preregistration and review once the control set and instrument capability are known. The first measurement can reveal that a numeric goal is infeasible; record that as evidence rather than moving the target after seeing results.

## 9. Repair data and regulatory context

The [Open Repair Alliance reported 619,875 repair attempts across 34 countries in its 16 October 2026 release](https://openrepair.org/announcements/over-half-a-million-records-of-open-data-on-repair/). The data are gathered through community repair events; the established ORDS dataset focuses on small electrical/electronic repairs. Use it to improve terminology and to learn how to record repair barriers, part unavailability and repair outcome. **Do not infer a representative washer failure distribution or calibrated fault priors from that sample without validating product coverage, sampling, missingness and selection bias.** See the [download page](https://openrepair.org/open-data/downloads/) and [Open Repair Data Standard](https://openrepair.org/open-data/open-standard/).

The European Commission's [washing-machine guidance](https://energy-efficient-products.ec.europa.eu/product-list/washing-machines_en) summarizes ecodesign, resource-use and listed spare-parts/repair information requirements under Regulation (EU) 2019/2023. The consolidated [Right to Repair Directive (EU) 2024/1799, text dated 31 July 2026](https://eur-lex.europa.eu/eli/dir/2024/1799/2026-07-31) scopes repair obligations through Annex II and applicable Union acts/national implementation. Review the current requirements for the exact product and market; compliance is not inferred from open design or a good benchmark score.

## 10. Software and provenance composition

- **Symtropy:** design variants, task graph, benchmark protocol, lifecycle model and architecture decision.
- **Symthaea:** advisory hypotheses and test rankings through the existing finite-hypothesis EIG/EVSI kernel, with hard feasibility/protection gates and evidence-chained posterior updates.
- **Mycelix:** existing identities for design/artifact set, BOM/substitute evidence, exact assembled unit, actual repair/material events, post-repair functional receipt and bounded recurrence. No second work-order or repair ledger.
- **Appliance:** local physical controls and independent protection have the authority; model output never actuates a safety-critical function.

## 11. Open design decisions

Do not settle these in advance:
- front-load model/capacity/market and control-unit selection;
- welded versus bolted tub and bearing service boundary;
- drive topology and sourced motor/drive interface;
- optional hot-water input or resource-recovery features;
- service task role tiers and required competence;
- local cost assumptions, repair-shop access, part availability and independent test team;
- applicable safety/electrical/plumbing/EMC requirements and qualified lab route;
- licensing and the third-party part/source manifest.

## 12. Claim ceiling

This is a versioned architecture and test plan. It does not establish that a washer has been built, that any variant is less expensive, more efficient, reliable or repairable, or that it is safe or conforming. Claims must bind the exact physical unit/configuration, matched protocol, raw evidence, analysis plan and reviewer.
