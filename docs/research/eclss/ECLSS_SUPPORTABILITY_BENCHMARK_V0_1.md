# ECLSS Supportability Benchmark v0.1

**Status:** research protocol / synthetic-first  
**Owner:** Symtropy; domain adapter under [ECLSS-SUPPORTABILITY-001 #4298](https://github.com/Luminous-Dynamics/symtropy/issues/4298)  
**Claim ceiling:** synthetic supportability contract and published-reference comparison only

## 1. Research thesis

A life-support subsystem is not successful merely because it reaches a high recovery fraction at nominal conditions. The engineering object is a time-dependent service system whose usable outputs depend on process quality, storage, distribution, energy, heat rejection, consumables, component life, inspection, maintenance, spares, crew/robot time and the ability to requalify after repair.

The proposed benchmark evaluates **qualified service delivered over a declared mission horizon**, subject to hard service floors and resource limits, while accounting for the full sustainment burden needed to keep that service available.

This reframes the objective from isolated subsystem efficiency to sustainable, evidence-backed service. It does not replace specialized ECLSS, water chemistry, atmosphere, crop, thermal, power, biological or reliability models.

## 2. Current state of the field and comparison boundary

### 2.1 ISS water recovery

NASA reported in June 2023 that the ISS ECLSS water-recovery system demonstrated 98% total water recovery with the Brine Processor Assembly. The same report describes wastewater collection, urine distillation, brine treatment, humidity recovery, filtration, catalytic oxidation, purity sensing, reprocessing of unacceptable water and iodine dosing. Thus the 98% figure is not a standalone potable-service guarantee; output acceptance and the residual/brine path matter.

Primary source: https://www.nasa.gov/missions/station/iss-research/nasa-achieves-water-recovery-milestone-on-international-space-station/

### 2.2 CO2-to-oxygen recovery

NASA's SpaceCraft Oxygen Recovery page gives the ISS Sabatier carbon-dioxide reduction assembly an estimated average oxygen recovery of about 47% (described elsewhere on the same page as approximately 50%). NASA's SCOR programme seeks greater than 75%, with a 100% stretch goal. These are development goals, not achieved flight performance. This oxygen-from-CO2 boundary must remain distinct from water recovery and from oxygen generated through water electrolysis.

Primary source: https://www.nasa.gov/spacecraft-oxygen-recovery-scor/

### 2.3 Reliability and supportability

The full 2025 ICES paper is materially more useful than its abstract alone: it presents a **preliminary component-level ISS ECLSS reference dataset** in its appendix, derived from the Maintenance & Analysis Data Set (MADS). Published fields include ORU mass, operational K-factor, MTBF, operational MTTR, preventive-maintenance remove/replace interval and duration, and operational-/throughput-limited life where applicable. It also gives 41 major-ORU crew R&R task-time estimates. The branch now manually transcribes 142 component entries across Appendix Tables 2–10 and all 41 Table 1 task-time entries across two versioned source registers. This is a public comparison surface, not a complete validated mission-reliability database: the authors state that the reliability dataset is not yet complete, analogies may be used when data are missing, and a larger integrated simulation framework is still in development. The registers preserve printed blank cells as null and retain MTBF, operational MTTR, Table 5 component R&R hours, Table 1 crew-task time, preventive maintenance, throughput limits and calendar/shelf-life limits as distinct fields. The selected register stores nine component rows and two Table 1 task records in separate arrays; its UPA Distillation Assembly component row retains 7 h operational MTTR and no duplicated Table 1 crew time (the separate Table 1 task record is 180 min). Table 5 ACTEX's 0.5 h R&R remains component R&R, not a Table 1 task.

The supplement is `docs/research/eclss/fixtures/eclss-iss-appendix-supplement-v0_1.json` with Draft 2020-12 schema `docs/research/eclss/eclss_iss_appendix_supplement.schema.json`. It adds 133 component rows and 39 crew-task rows to the selected register's 9 component rows and 2 crew-task rows: 142 component entries and all 41 crew R&R estimates in total. These are manual transcriptions from the published tables—not raw MADS event records.

Examples transcribed into the versioned source registers in this PR include:
- Water Processor Assembly: catalytic reactor 57.43 kg / 27,077 h MTBF / 6 h MTTR / K=1.05; ion-exchange bed 12.02 kg / 442,478 h / 0.65 h / K=1.38; pump/separator 27.58 kg / 39,429 h / 1.15 h / K=3.26.
- Urine Processor Assembly: distillation assembly 75.57 kg / 41,376 h / 7 h / K=1.84.
- Maintenance-limited items: brine filter 60 days between preventive R&R, with 0.17 h preventive R&R time; the purge and distillate filters are listed at 545 days and 0.17 h.
- Water-storage ACTEX filter: 1.46 kg, 469,469 h MTBF, K=1.15, 0.5 h R&R, plus 6,750 lb throughput or four years wetted-life limits.
- Brine Processor Assembly: 10.8-year processor life; each bladder is to be disposed of within 35 weeks of brine fill. The paper also reports a 24 L BPA batch every 26 days; this is a batch/cadence reference, not a throughput-limited lifetime.

These figures are **published component reference values**, not guaranteed mission lifetimes, current fleet predictions or calibrated failure probabilities. The paper's annual failure-rate expression includes duty cycle, quantity and K-factor as well as MTBF; do not derive a mission probability from MTBF alone. Keep calendar-limited, throughput-limited, preventive-maintenance and stochastic failure mechanisms as separate competing life limits. The paper's Table 1 crew times explicitly exclude gathering and stowing; the authors say they depend on installation location, accessibility and task complexity and are not exhaustive. For example, Table 1 gives 14 h for OGA hydrogen ORU R&R and 19 h 15 min for the combined WPA gas-separator/water-delivery/ion-exchange/AAA task grouping. Treat these as task-time reference points, not complete EVA/IVA or mission timeline durations.

Source: https://ntrs.nasa.gov/citations/20250003955

### 2.3.1 Industry baseline and the research gap

The same paper identifies two relevant established comparison families:
- **NASA Langley's EMAT**: a mission-timeline simulation that stochastically triggers component failures, executes repair actions and tracks spare parts. The paper characterizes it as primarily focused on component reliability and maintainability.
- **RELISSA**: an ECLSS reliability simulator using stochastic failures and backup activation, with failure/reliability trend outputs; the authors describe its integration with ESM mass/volume/power/heat for early system optimization, but without crew-time accounting.

The 2025 paper's own direction is to extend component evidence with replacement rate, shelf/throughput limits and crew R&R time. It says the larger integrated simulation is still future work. Therefore the fair opportunity for Symtropy is **not** to claim a more accurate NASA reliability model. It is to build a reproducible service-and-sustainment evaluation layer around public component data and existing reliability methods:

1. Preserve component-level source values and life-limit semantics.
2. Add task scheduling, crew/robot availability, precedence, access and maintenance contention; distinguish R&R duration from logistics/gather/stow/diagnosis time.
3. Couple component state to accepted product quality and time-resolved service floors.
4. Track finite consumables, inventory, qualified interchangeability, shelf-life and resupply windows.
5. Compose power, thermal rejection and resource ledgers without collapsing temperature grade or mass/quality grade.
6. Require exact-configuration post-repair tests before service is requalified.
7. Evaluate model uncertainty, missing data, and alternate hazard models without fabricating priors or censoring rules.

Start with deterministic known-answer scheduling/resource cases and reproduce only the paper's public reference records. Compare against EMAT/RELISSA or equivalent tools only when executable implementations, inputs and identical boundaries are available. Otherwise report a feature/assumption comparison, not a performance ranking.

NASA's 2026 ICES paper describes using the ISS as a testbed to mature exploration-class ECLSS hardware and integration, with performance and reliability evidence intended to support deep-space missions.

Source: https://ntrs.nasa.gov/citations/20260003280

### 2.4 Regenerative life support — MELiSSA is an active comparator, not just a concept

ESA's MELiSSA programme is a mature, multi-decade regenerative-life-support research programme, and it must be treated as a serious comparator rather than as a diagram-only architecture:

- The MELiSSA Pilot Plant at UAB/Barcelona runs connected biological and cultivation compartments continuously: C3 photobioreactors, C4a/C4b higher-plant cultivation, and a C5 small-mammal crew-compartment model. The MELiSSA Foundation describes it as continuous integrated full-loop operation, not isolated single-process trials. This is ground pilot evidence at its tested scale, not an operational human space habitat.
- The Concordia Antarctic station has a greywater recycling system derived from MELiSSA research. The Foundation reports >90% greywater recovery and 11 years of autonomous operation with almost no maintenance. This is a strong remote-site greywater reference; it is **not** total ECLSS closure or an eleven-year human life-support-loop demonstration.
- The programme reports nine flight experiments. ARTEMISS (2024) demonstrated CO2-to-O2 conversion kinetics; WAPS (plants) and URINIS (urine) are described as next experiments in preparation. The Foundation describes four technology-demonstrator lines targeting 2030. These are programme statements and staged technology goals, not evidence that the complete regenerative habitat has flown.

Sources:
- MELiSSA ground demonstration: https://melissafoundation.org/the-melissa-program/ground-demonstration/
- MELiSSA flight experiments: https://melissafoundation.org/the-melissa-program/flight-experiments/
- ESA MELiSSA concept: https://www.esa.int/Enabling_Support/Space_Engineering_Technology/Melissa/Closed_Loop_Concept

**Competitive implication:** integrated biological-loop modelling, predictive process control and long-duration ground integration are not novel contributions that Symtropy can claim over MELiSSA. The Symtropy research opportunity is complementary: interoperable evidence contracts across biological/physicochemical subsystems, exact component/configuration/maintenance lineage, finite mission spares and crew-time accounting, consumer-quality service floors, source-bounded model applicability, independently replayable balances and adversarial qualification. Any performance claim must still be tested on a matched MELiSSA-relevant boundary and must not substitute an accounting layer for MELiSSA's biological process expertise.

### 2.5 Integrated lunar infrastructure

NASA's current Moon-base description treats enduring presence as a coupled architecture: dependable power, habitation/life support, cargo transportation, communications/navigation, mobility, and reliable logistics must operate together under radiation, temperature and lunar-dust hazards. This supports a system-level benchmark, but does not mean those future systems are already deployed or qualified.

Source: https://www.nasa.gov/moon-base-frequently-asked-questions/

### 2.6 Direct operational cases from NASA's 2026 ISS exploration-ECLSS testbed report

The 2026 status paper adds source-bounded operational evidence that should become part of a fair benchmark, not just background literature:

- **Configuration mismatch in a replacement part:** the Four-Bed CO2 Scrubber's Air Save Pump failed in June 2024. The on-orbit spare omitted an internal valve present in the original because of a design-data transfer error; without it, the system could not maintain vacuum on the adsorbent beds. Workarounds were used and an external COTS valve assembly was flown and installed in February 2025. This is a real configuration/functional-equivalence negative control, not merely a stock-count failure.
- **Material compatibility and repeat failure:** during an early-2023 14-day, four-crew UWMS demo, the pretreat dose pump failed; NASA attributes the issue to PEEK check-valve material incompatibility with pretreat over an indeterminate duration. A legacy-design spare pump flown in March 2024 failed to dispense pretreat again on May 2024 reactivation; a controller failure in July 2024 then prevented further troubleshooting. This supports compatibility-aging, recurrence and fault-isolation cases without inventing an exact time-to-failure.
- **Fouling with an incomplete causal model:** ISS water separators accumulated FOD, with evidence linking some material to the hydrophilic heat-exchanger coating and dried condensate; the paper says other contributors remained unclear. Dry-outs changed from monthly to every 90 days to reduce precipitate accumulation, with six months stated as a future goal. The benchmark must preserve the unresolved causal contributors and distinguish a changed operating procedure from demonstrated lifetime extension.
- **Condition-based conservative life:** TCCS charcoal filters were removed after water data showed run-to-failure at about 4.25 years. Ground assessment suggested operation to about four years, but the ISS programme accepted a conservative three-year operational life. This is an explicit comparator for fixed conservative limits versus evidence-backed trending; it is not permission to extend life beyond a qualified limit.
- **Sensor drift and maintainability:** the H2 Sensor Technology demonstration had one of four sensors fail after three months and another fail in January 2025; two remained stable. Legacy H2 sensors were held to a 201-day installed life based on qualification/on-orbit experience. Returned-unit drift analysis suggested some might support longer use (up to 348 days was discussed with R&R in May 2026), subject to review and testing. Treat any extension as conditional, sensor-/ORU-specific and authority-controlled.
- **Condition monitoring has its own failure modes:** COTS CO2/O2/H2O monitors were deployed across ISS modules and were being used for atmosphere trending when the legacy MCA was unavailable; an RFID interaction affected ppCO2, although one-hour averages remained accurate. Airflow sensors enabled longer-period trends rather than spot checks, but unfiltered ISS FOD stuck to and blocked sensor heads. Sensor cleaning, fouling and interference therefore belong in the benchmark, not only sensor accuracy.
- **Finite sustainment plan:** the Thermal Amine Scrubber (TAS) had multiple component failures and replacements; the paper says it was no longer being actively sustained with a logistics plan and would continue as long as existing spares lasted. This is a direct case for distinguishing operational runtime from an active, replenishable support plan.
- **Design-for-maintenance in the next generation:** the 2026 paper describes AOGA design changes intended to enable component-level replacement (e.g. individual valves), reduce spares mass/volume and add purge/flush capability for contamination control and dormancy; delivery to ISS was planned for mid-2027. These are design intentions and schedule, not demonstrated AOGA flight results as of the paper.
- **Evidence-domain boundary:** the authors state the ISS microgravity demonstration is most relevant to microgravity transit/orbit. Lunar/Martian partial-gravity surface stays may need different requirements; pressure differences also require ground testing. Do not transfer ISS pass results to surface conditions without an explicit applicability case.
- **Demonstration versus critical configuration:** NASA states standalone technology demonstrations were treated as non-critical demos with limited reliability requirements; most upgraded subassemblies were not part of the critical system configuration until proven, and the nominal critical-system spares fleet was retained. A successful tech demo must not be relabelled as a qualified critical life-support service.

Machine-readable narratives and source-page lineage are stored in `docs/research/eclss/fixtures/eclss-2026-iss-operational-cases-v0_1.json`. The file intentionally distinguishes observed operational events, operating adaptations, planned design features and applicability limits.

Source: Brown et al., *International Space Station as a Testbed for Exploration ECLSS: 2026 Status*, ICES-2026-379: https://ntrs.nasa.gov/citations/20260003280

### 2.7 AOGA schedule evidence: preserve milestone and forecast variance

The AOGA schedule must be treated as a versioned set of dated programme statements, not as one timeless target:

| Source snapshot | Published / updated | Stated milestone |
|---|---|---|
| NASA TechPort project 157887 | Updated 18 Dec 2025 | Mod-kit delivery planned early calendar 2026; launch to ISS in fall |
| ICES-2026-423, *Status of the Advanced Oxygen Generation Assembly in 2026* | 12 Jul 2026 | AOGA hardware described as undergoing build; launch to ISS slated late 2026 |
| ICES-2026-379, *ISS as a Testbed for Exploration ECLSS: 2026 Status* | 12–16 Jul 2026 | AOGA delivery to ISS described as planned for mid-2027, with three new manifolds and pressure-sensor qualification on the critical path |
| NASA FY 2026/2027 Agency Performance Plan | Apr 2026 | Deliver AOGA for testing on ISS is a FY 2027 critical milestone |

These are not perfectly equivalent milestone verbs: "mod-kit delivery", "delivery for testing" and "launch" may refer to different integration steps. However, the publication snapshots contain a material schedule variance. As of this benchmark's 9 November 2026 research cutoff, the sources collected here do not independently establish that AOGA has launched or entered operational service. Preserve each assertion, date, source and milestone term; do not silently select the most optimistic date, infer a slip cause, or treat a plan as an achieved milestone.

Machine-readable records are stored in `docs/research/eclss/fixtures/eclss-aoga-schedule-snapshots-v0_1.json`. This is a schedule-provenance fixture, not a programme-control schedule.

Sources:
- NASA TechPort project 157887: https://techport.nasa.gov/projects/157887
- ICES-2026-423: https://doi.org/10.32865/2346/109056
- ICES-2026-379: https://ntrs.nasa.gov/citations/20260003280
- NASA FY 2026/2027 Agency Performance Plan, page 7: https://www.nasa.gov/wp-content/uploads/2026/04/nasa-fy-27-volume-of-integrated-performancevf.pdf


## Industry comparison and matched SOTA evaluation

See [ECLSS Industry / SOTA Comparison Protocol v0.1](ECLSS_INDUSTRY_SOTA_COMPARISON_V0_1.md). It explicitly compares EMAT, RELISSA, SmartHab stochastic MILP and the 2026 distributionally robust maintenance/spares MILP; defines common-input fairness rules, ablations, metrics and target hypotheses. It does not claim an unmeasured performance win.

## 3. Paradigm shift

### From recovery fraction to accepted service

For every stream, distinguish:
1. feed entering a declared process boundary;
2. product leaving the process;
3. quality acceptance by the downstream consumer;
4. storage/distribution losses and access;
5. residual, waste, brine, purge and contaminant flows;
6. make-up required from stored or resupplied material.

An output outside the consumer's quality envelope contributes zero accepted service until reprocessed or independently accepted. A high gross recovery percentage cannot compensate for failed quality.

### From nominal efficiency to sustainment economics

Overlay a second, coupled ledger:

fault → detection → diagnosis → degraded mode → isolation → maintenance → parts/consumables → repair → requalification → restored service.

Every task consumes time, qualified labor or robot capability, tools, energy, consumables and potentially service reserve. The part must match the installed configuration, and a repair report is not evidence of restored service.

### From closed-loop rhetoric to finite-horizon closure

A loop is not closed merely because a product is recycled once. Evaluate residual losses, contaminant accumulation, quality decay, batch rejection, biological instability, maintenance-induced downtime, and every required make-up stream across repeated cycles. Keep each element/material balance separate.

### From one aggregate score to constrained Pareto evidence

Report a vector of service, quality, resupply, energy, heat, maintenance, crew time, reliability evidence and uncertainty. A critical floor breach is a gate—not a cost that can be offset by better efficiency elsewhere.

### From maintenance schedules to proof-carrying maintenance plans

A schedule is not yet an admissible maintenance operation. Each proposed task should carry an independently checkable plan envelope before it is offered to an operator or sent to any execution authority:

- **Asset scope:** immutable asset/component identity, installed revision, configuration digest, affected service(s), and current evidence/freshness state.
- **Procedure scope:** approved procedure revision, hazard/isolation steps, prerequisites, task dependencies, abort criteria and a declared completion test.
- **People and tools:** named qualified worker/robot capability, current training/authorization evidence, required tools, access and operating environment.
- **Parts and consumables:** exact spare/consumable identities, revision and compatibility evidence; stock reservation; shelf-life, storage and throughput constraints; no implicit substitution.
- **System state:** isolation and bypass evidence, affected dependency graph, remaining service floor/reserve, common-cause exposure and safe degraded configuration.
- **Resource budget:** task duration, crew/robot time, tool occupancy, energy, consumables, spare mass and expected service interruption as separate dimensions.
- **Requalification:** post-repair inspection / test method, calibration and acceptance criteria bound to the exact repaired configuration; downstream availability is recomputed only after acceptance.
- **Authority and expiry:** responsible decision authority, accepted/rejected/deferred status, scope, validity window and the exact evidence digest set used to admit the task.

The independent verifier should reject or return NEEDS_EVIDENCE for a plan with a missing prerequisite, overlapping use of a non-shareable worker/tool, an unqualified or wrong-revision spare, expired credentials, unsupported maintenance window, missing isolation, a predicted hard service-floor breach, or a missing post-repair acceptance result.

A plan receipt is **not execution authorization**. A scheduler proposes; a verifier checks; a responsible authority approves; the operator executes; measurements and a post-repair test determine whether the service can be requalified. Planned duration is not actual duration, and a completed work order is not proof of restored service.

This creates a distinct benchmark target: compare not only which maintenance sequence is cheapest, but which plans are admissible under exact evidence and constraints, how often they are safely deferred/abstained, whether any invalid plan is falsely admitted, and what service/sustainment cost results after accepted execution. Keep this as a consumer of #1625 evidence and repair receipts and #1752 paired policy evaluation; do not add a second safety/authority engine.

## 4. System boundaries and composition

This adapter consumes existing project owners:

- #3952 owns the planetary habitat / regenerative food loop and preserve constraints.
- #3957 owns full-facility energy, thermal, water and accepted-food accounting.
- #3254 owns finite logistics and irrecoverable/no-resupply disruption semantics.
- #1625 owns evidence-qualified, time-scoped availability and repair/requalification receipts.
- #1752 owns paired policy evaluation under common exogenous events and information budgets.
- #1803, #1840 and #1637 own their respective resource-product, energy-service and nutrient/resource accounting semantics.

The adapter binds these records into an ECLSS sustainment campaign. It must not add another crop model, thermal solver, water-treatment solver, resource ledger, reliability engine or actuation authority.

### Subsystem profile

A profile identifies:
- subsystem, component, installed configuration and software revision;
- function and critical-service consumer;
- input, output and byproduct streams with units, quality grades and process boundaries;
- throughput, turndown and environmental validity envelopes;
- electrical and thermal demand, including temperature-grade limits and heat rejection;
- consumables, batch identities, quantities, shelf life and storage conditions;
- maintainable items, inspection/cleaning/calibration tasks and replacement rules;
- spare identity/revision, interchangeability evidence, stock and lead time;
- required tools, crew/robot skills, access, task duration and resource demand;
- fault modes, detection coverage, isolation and degraded-mode limits;
- repair attempt, post-repair measurements and exact-configuration requalification;
- sensors/assays, calibration, uncertainty, timebase and raw-data digest;
- source, evidence tier, validity domain, known limitations and claim ceiling.

Unknown lifetime, replacement rate, failure rate or maintenance duration remains unknown. Do not substitute a convenient distribution or zero.

### Maintenance-induced risk and paired condition-policy falsification

Maintenance is not a free state transition. Opening a subsystem, disconnecting a line, changing seals/valves, handling a filter, disturbing contamination or reinstalling an ORU can introduce leaks, damage, contamination or configuration errors. The benchmark must therefore measure the *net* effect of maintenance, not assume that every completed intervention reduces risk.

For paired policies under the #1752 owner, compare:
1. conservative approved interval-based maintenance;
2. condition-based maintenance using qualified, in-domain sensing and predeclared thresholds;
3. hybrid condition-based maintenance with independent confirmation, sensor-health monitoring and conservative approved life limits.

All policies receive the same scenario/event trace, sensor channels, calibration and observation-latency contract, crew/tool/spare budgets and authority limits. Include:
- true degradation crossing the threshold;
- healthy component with false-positive alarm and unnecessary removal;
- sensor drift / false-normal;
- blocked or fouled sensor head;
- common-mode bias across apparently independent channels;
- stale or delayed telemetry;
- missing calibration or out-of-domain sensor;
- maintenance-induced damage / contamination / leak during a task;
- repair that restores nominal flow but fails the output-quality or post-repair acceptance test;
- condition-based proposal that would exceed the approved life limit.

Report false deferral, unnecessary maintenance, missed/late maintenance, diagnosis latency, maintenance-induced fault/damage, service-floor exposure, spare use, worker/tool time, and verified post-task service. A predictive model may recommend a measurement or task; it cannot promote a sensor observation to qualified evidence or extend an approved component life limit without separate authority and evidence.

### Proof-carrying maintenance plan fixture (synthetic only)

The executable reference now includes a second frozen corpus:
- Schema: `docs/research/eclss/eclss_maintenance_plan_admissibility.schema.json`
- Fixture: `docs/research/eclss/fixtures/eclss-maintenance-plan-admissibility-v0_1.json`
- Rust validator: `tools/eclss-supportability-bench/src/main.rs`

The 13 cases include one admissible plan and 12 negative controls. The validator independently resolves exact asset revision, approved procedure, qualified worker/skill, tool availability and exclusive time windows, spare compatibility/expiry/quantity, verified isolation, service-floor margin, authority scope/freshness, post-test revision/status, task-window and dependency order. It checks the computed result against the frozen known answer rather than trusting the expected disposition field.

The receipt must report `execution_authorized=false`. This is a synthetic evidence-contract check only: an admissible plan is not a maintenance authorization, command, safety case or hardware-control capability. Approval and execution remain with the responsible authority/operator; measured post-repair evidence must independently requalify service.

### Condition monitoring is not automatically safer than scheduled maintenance

NASA maintenance-management guidance supports a shift away from blanket interval-based tasks where justified condition monitoring can reduce unnecessary work. But the monitoring channel can fail too: the 2026 ISS status paper documents airflow sensor heads blocked by unfiltered debris and H2 sensor failures/drift. A condition-based policy must therefore be evaluated against at least three arms:

1. conservative approved interval-based baseline;
2. condition-based policy with qualified sensors and validated thresholds;
3. hybrid policy with independent cross-checks, conservative life limits, sensor-health monitoring and explicit escalation.

Seed sensor blockage, drift, bias, common-mode dependence, missing telemetry, false-normal and delayed observation. Measure false maintenance deferral, unnecessary PM, maintenance-induced damage, detection/diagnosis delay and service-floor exposure. Do not use condition monitoring to exceed an approved life limit without separate authority and qualification.

NASA's guidance also calls for traceable maintenance procedures/status, critical-step completion, restored-function evidence, replacement-unit history and procedure execution records. These requirements reinforce the proof-carrying maintenance plan contract; they are existing industry expectations, not a novelty claim.

Sources:
- NASA hardware/condition-monitoring and maintenance management guidance: https://sma.nasa.gov/sma-disciplines/hardware-and-equipment
- NASA ISS ECLSS testbed status with sensor fouling and drift cases: https://ntrs.nasa.gov/citations/20260003280

## 5. Benchmark design

### 5.1 Frozen scenario registry

The initial registry contains 30 typed synthetic cases, ECLSS-001 through ECLSS-030. The machine-readable scenario file is at:

docs/research/eclss/fixtures/eclss-supportability-synthetic-registry-v0_1.json

Schema:

docs/research/eclss/eclss_supportability_case.schema.json

The cases cover water recovery/quality, residual streams, oxygen recovery, CO2 removal, contaminants, common-cause dependencies, throughput, power/thermal limits, spare compatibility and shelf life, maintenance contention, repair/requalification, biological loop interruption, communication delay, out-of-domain evidence and tamper/metamorphic controls.

These cases are synthetic and deterministic. Their purpose is to test accounting, evidence and decision semantics before any empirical calibration.

### 5.2 Known-answer ledger check (synthetic only)

A deliberately small arithmetic fixture makes the distinction between recovery and resupply explicit:

- demand: 100 abstract water units per day;
- accepted recovery: 98% of the process-boundary input;
- daily make-up: 2 units;
- horizon: 30 days;
- initial qualified make-up inventory: 40 units;
- no resupply during the horizon.

The ledger therefore requires 60 units of make-up, has 40 units available, and ends with a 20-unit unserved deficit if no other qualified source exists. Recovery remains 98% in this toy model while the service floor eventually fails. This is an arithmetic counterexample, not an ISS reproduction, potable-water model or claim about an actual habitat.

### 5.3 Common event and information budgets

Every candidate architecture/policy pair receives the same:
- initial inventory and subsystem condition;
- crew demand and service-floor version;
- exogenous fault and environmental event trace;
- power, thermal and logistics envelope;
- observation availability, delay and sensor quality;
- mission horizon and resource accounting basis.

A policy does not receive future fault information. Exogenous events cannot be rewritten by a repair or by an intervention. If a policy receives additional measurements, report this as a different information arm and charge the full resource/time cost.

## 6. Baselines

Use matched cases and clearly separate empirical, simulated and conceptual evidence:

1. Open-loop supply with explicit consumable mass and delivery windows.
2. ISS-heritage subsystem references, only for public values with reproducible process boundaries.
3. Reactive maintenance after a qualified observed threshold.
4. Scheduled preventive maintenance.
5. Service-floor/reserve-first maintenance scheduling.
6. Adaptive maintenance or measurement selection only where likelihoods, decision utility and observation access are justified.
7. No-failure and perfect-information runs as diagnostic bounds, never peer competitors.

### 6.1 Industry comparison matrix — compare capabilities, not brand names

| Reference | What is actually demonstrated / modelled | Important boundary | Symtropy's fair target |
|---|---|---|---|
| NASA Langley EMAT | Mission-timeline, stochastic component failures, repair actions and spare tracking, as characterised by the 2025 ICES paper | Component reliability / maintainability focus; do not assume public source code or identical assumptions | Consume its component/event semantics if executable comparator access is available; add quality-qualified service and interoperable evidence receipts without rebuilding its reliability model |
| RELISSA | ECLSS stochastic failure and backup activation; integration with ESM mass, volume, power and heat described in the 2025 paper | The paper characterises crew-time accounting as absent; executable parity and current version must be established before performance ranking | Add crew/robot task scheduling, task precedence, service-floor consequences and exact repair/requalification evidence; no claimed numerical superiority yet |
| ISS operational ECLSS | Long-duration flight operation and real repair, spare, sensor, fouling and configuration lessons | ISS microgravity, pressure, crew, logistics and certified-configuration scope differ from a partial-gravity surface mission; public event histories are incomplete | Reproduce named operational cases, retain current critical/demo configuration, quantify only measured or disclosed performance |
| ESA MELiSSA | Continuous integrated ground pilot plant; nine reported flight experiments, with ARTEMISS 2024 CO2-to-O2 kinetics; Concordia greywater >90% recovery and 11-year autonomous system reported by Foundation | Different process boundaries and evidence tiers; pilot plant/animal-model and remote greywater references do not equal a full human-rated flight loop | Complement, do not replace: harmonize service and material ledgers, evidence lineage, sustainment and adversarial cross-boundary qualification |
| Symtropy target | A domain adapter that composes existing physics, resource, evidence, logistics and policy owners | Current PR is documentation + schemas + source-bounded and synthetic registries; no integrated executable engine has been demonstrated in this PR | Demonstrate exact replay, source traceability, independent balance checks, zero seeded false acceptance and fair paired policy outcomes before any SOTA claim |

### 6.2 Where a paradigm shift could be real

The research hypothesis is **proof-carrying operations across a heterogeneous life-support system**. Each consequential service-state change should carry a replayable evidence packet tying together:

1. Exact installed component, configuration, software, spare revision and compatibility evidence.
2. Context-of-use vector: gravity, pressure, fluid state, temperature, duty cycle, crew load and operating envelope.
3. Calibrated observations, sensor health/drift/fouling, sampling window and contradictory evidence.
4. Process boundary, input/product/residual streams, units, quality grade and conservation checks.
5. Service demand, hard floor, delivery, reserves and explicit unmet service.
6. Forecast assumptions, alternative model families, validity and unknowns.
7. Maintenance/intervention authority, crew/robot skills, task precedence, tools, consumables, spare and energy/thermal budgets.
8. Repair receipt and independent post-repair acceptance tied to the exact new configuration.
9. Evidence expiry, triggers for reinspection, and a machine-readable reason to abstain.

The packet is not a new simulator or digital-twin platform. It is the auditable bridge between domain engines and a decision that affects service. No subsystem is allowed to certify its own output.

### 6.3 Candidate targets — staged and falsifiable

**Before executable comparison:**
- 100% of the 30 synthetic known-answer cases match an independent evaluator.
- 100% of seeded wrong-configuration, stale, out-of-domain, failed-quality, false-sensor and repair-without-requalification controls are rejected or explicitly marked INDETERMINATE.
- Zero unexplained inventory, mass/element balance or unit conversions.
- Byte-identical deterministic replay for identical frozen inputs and software identity.

**Against established component-maintenance baselines:**
- Same component dataset, failure/event semantics, mission horizon, task availability and spare assumptions for EMAT/RELISSA or an equivalent reproducible implementation.
- Report service-floor breaches, downtime, spare mass, maintenance crew-hours, energy/heat and run-time/memory separately.
- Candidate target after baseline parity: at least 20% lower sustainment burden (predeclared Pareto measure, e.g. crew-hours plus spare mass with explicit conversion only if approved) or service-loss regret, while showing no worse critical service outcome in any held-out scenario family. Do not declare the percentage target achieved from synthetic-only data.
- Zero false admission of an incompatible replacement, an expired spare or a demo-only assembly as critical qualified service in the frozen adversarial corpus.

**Against biological-loop references:**
- Do not compete with MELiSSA's biological expertise on conceptual completeness. First establish comparable mass/element balances, quality contracts, operating duration, intervention burden and evidence tier.
- Treat Concordia's greywater result as a greywater reference only; treat pilot-plant loop and ARTEMISS as their actual system/experiment boundaries.
- Target independent reproduction of disclosed measurements and improved cross-source traceability, not unsupported higher biological recovery.

All outcomes remain a vector and a Pareto frontier. A weighted average cannot offset a critical service-floor failure.

## 7. Metrics

Report independently by stream, service and interval:

- demand, accepted delivery, deficit and time below each authorized service floor;
- product-quality accept/reject/reprocess totals and evidence;
- gross recovery, net make-up, purge, residual and waste by material/element boundary;
- resupply mass and volume per crew-day and crew-year by grade;
- electrical energy, thermal energy by temperature grade, heat rejection and parasitics;
- throughput, availability, degraded-mode interval and storage reserve;
- time to detection, diagnosis, isolation, repair, requalification and stable recovery;
- maintenance count, crew-hours, robot-hours, tools, consumables and spare mass;
- stockout, expiry, storage excursion and interchangeability;
- contaminant/residual concentration trajectory and accumulation;
- first service loss, recurrence, and service-loss regret;
- fraction of intervals with current, calibrated and in-domain evidence.

MTBF/MTTR, probability of survival and Monte Carlo reliability are admissible only with justified exposure/event data, censoring treatment, dependence assumptions, uncertainty and model validation.

## 8. Proposed benchmark gates

P0 — schema/fixture validity, deterministic replay and independent arithmetic oracle.

P1 — every valid synthetic case matches an independent balance/decision evaluator; all negative controls fail closed.

P2 — reproduce public ISS water and oxygen reference statements without mixing process boundaries or converting targets into results.

P3 — admit the 2025 reliability paper's component replacement, shelf-life and throughput records only to the degree that methods/data are accessible and their assumptions can be reconstructed.

P4 — conduct a metered uncrewed analogue with independent quality assays, seeded component faults, maintenance logs, parts/shelf-life tracking and repeated cycles.

P5 — separately qualify integrated hardware and environmental/flight use. No promotion is automatic.

Hard gates:
- no critical service credit from an output that fails its consumer quality contract;
- no negative inventory or unexplained mass/element disappearance;
- no unqualified substitute, repair or post-repair service;
- no automatic recovery when resupply, parts, staff or tools are absent;
- no scalar score may mask a critical floor violation;
- unknown or out-of-domain evidence remains unresolved.

A candidate research goal is ≥20% lower service-loss regret or sustainment burden than the strongest transparent baseline on a frozen held-out suite, with no service-floor regression. This target is deferred until public baselines and analogue measurements establish a stable comparison. It is not an achieved result.

## 9. Promotion ladder and claim ceiling

| Evidence tier | Admissible claim |
|---|---|
| Synthetic | Exact accounting, state transition, negative-control and decision-semantics behavior on frozen known-answer cases |
| Published reference | Reproduction of disclosed source values with boundary, date, method and uncertainty retained |
| Ground analogue | Measured performance for the exact tested apparatus, environment, sensors, calibration and operating envelope |
| Integrated test | Coupled-system behavior only for the integrated configuration and event envelope actually tested |
| Flight | Flight performance only for the configuration, duty cycle and environmental conditions directly evidenced |

V0.1 is synthetic-only. It does not qualify ECLSS hardware, crew exposure, potable water, oxygen supply, medical safety, biological stability, a lunar/Martian site or a flight system. NASA/ESA sources are technical references, not endorsements or certifications.

## 10. Source register

- NASA, *NASA Achieves Water Recovery Milestone on International Space Station*, 20 June 2023; page last updated 2 March 2026: https://www.nasa.gov/missions/station/iss-research/nasa-achieves-water-recovery-milestone-on-international-space-station/
- NASA, *SpaceCraft Oxygen Recovery (SCOR)*: https://www.nasa.gov/spacecraft-oxygen-recovery-scor/
- Chen, Rivas and Rangel, *Advancing ECLSS Reliability Modeling: Integrating ISS Data for Sustainable Long-Duration Mission Planning*, ICES-2025-127: https://ntrs.nasa.gov/citations/20250003955
- Brown et al., *International Space Station as a Testbed for Exploration ECLSS: 2026 Status*, ICES-2026-379: https://ntrs.nasa.gov/citations/20260003280
- ESA, *MELiSSA Closed Loop Concept*: https://www.esa.int/Enabling_Support/Space_Engineering_Technology/Melissa/Closed_Loop_Concept
- NASA, *Moon Base: Frequently Asked Questions*: https://www.nasa.gov/moon-base-frequently-asked-questions/
- NASA, *Reliability and Maintainability*: https://sma.nasa.gov/sma-disciplines/reliability-and-maintainability

