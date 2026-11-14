# Autonomous Construction — Industry and Benchmark Alignment

**Research snapshot:** 14 November 2026  
**Scope:** autonomous construction production, high-rise climbing workcells, construction planning, mixed-fleet interoperability and accepted-output assurance.

- Accepted-floor accounting integration: [#4957](https://github.com/Luminous-Dynamics/symtropy/issues/4957) / draft [#4960](https://github.com/Luminous-Dynamics/symtropy/pull/4960).
- Vertical-logistics and climbing-workcell co-design: [#4964](https://github.com/Luminous-Dynamics/symtropy/issues/4964).

This note distinguishes publicly advertised industry capabilities, peer-reviewed project-specific results, research targets and Luminous implementation status. Product pages describe vendor-stated features; they do not establish a complete capability inventory or prove that an unlisted capability is absent.

## 1. Current industry position

### Planning and digital construction

| Reference | What is already established | Implication for Luminous |
|---|---|---|
| Oracle Primavera P6 | CPM planning/scheduling, resource and role optimization, demand/capacity planning, what-if analysis, integrated cost/schedule management and forecasts | Do not rebuild enterprise CPM, resource levelling or project-cost management |
| Bentley SYNCHRO 4D | Full CPM schedule support, BIM/model-linked tasks, assignment of human/material/equipment/space resources, resource status/actual use, actual costs/earned value and dynamic clashes between objects/equipment/work zones | Do not claim ordinary 4D simulation, space assignment or cost roll-up as a differentiator |
| Cat Command / autonomy | Remote operation and fleet integration across construction equipment; Caterpillar has also announced expanded autonomous product lines | Do not compete first on machine manufacture, controls, dealers, parts or global service |
| Doka SKE plus | Modular, fully hydraulic, crane-independent climbing formwork, with multiple parallel climbing units and integrated work platforms/access | “Self-climbing” or “crane-independent” alone is not a new category |
| VDA 5050 v3.0.0 | Open communication interface between central fleet control and mobile robots | Use it as an adapter boundary where applicable; build construction workflow semantics above it |

Sources: [Oracle Primavera P6](https://www.oracle.com/construction-engineering/primavera-p6/); [SYNCHRO 4D feature list](https://bentleysystems.service-now.com/community?id=kb_article&sysparm_article=KB0017688); [Cat Command](https://www.cat.com/en_US/by-industry/construction-industry-resources/technology/command.html); [Doka SKE plus](https://www.doka.com/en/system-groups/doka-climbing-systems/automatic-climbing-formwork/automatic-climbing-formwork-ske-plus/index); [VDA 5050 v3.0.0 official specification](https://www.vda.de/dam/jcr%3A09f03b91-13e2-4db3-bf30-4f221710071b/VDA5050-V3.0.0-2025-03.pdf).

**Important VDA boundary — distinguish interface primitives from traffic-management authority:** VDA 5050 v3.0.0 is a vendor-neutral interface for information exchange between central fleet control and mobile robots. Version 3.0 adds meaningful primitives for freer navigation: a zone concept (including restricted areas, one-way routes and zones requiring explicit master-controller authorisation), and robot-to-controller planned-path sharing that lets the controller influence where/how robots move while detailed route planning can remain onboard. These are real capabilities and should be represented in our adapter/conformance tests, not ignored. However, the specification's Scope §2 explicitly says it does not define safety requirements or traffic-management decision logic such as routing, prioritisation, congestion handling or deadlock resolution. It also excludes peripheral/infrastructure interfaces, project coordination and acceptance procedures, allocation of operational responsibilities, and cybersecurity measures. The accurate opportunity is therefore not “VDA has no zones or traffic hooks”; it is to implement the construction-specific coordination, risk/permit authority, cross-resource scheduling, acceptance and evidence semantics above the communication primitives. This boundary is not proof that any particular fleet product lacks those functions; vendor adapters and operator trials are needed. [VDA v3.0 press release (20 April 2026)](https://www.vda.de/en/press/press-releases/2026/260421_PM_VDA_5050_EN) · [Official VDA 5050 v3.0 specification](https://www.vda.de/en/news/publications/publication/vda-5050) · [Official specification source repository](https://github.com/VDA5050/VDA5050).

### Climbing systems and physical workcells

A strong current reference is the 20 August 2026 *Scientific Reports* paper, “Key technologies and application of a lightweight intelligent aerial construction machine for sustainable super-high-rise building.” Its reported engineering validation includes real applications at the Meituan Shanghai Technology Center and Wuhan Taikang Financial Center. The authors report a reduction in support-column mass from 30 t to 5 t, platform steel consumption from 0.5 t/m² to 0.2 t/m², total platform equipment mass from about 900 t to 230 t, more than 50% lower jacking duration, installation reduced from 45 to 25 days and component turnover increased from about 30% to over 80%.

Source: [Scientific Reports (2026), DOI 10.1038/s41598-026-67566-y](https://www.nature.com/articles/s41598-026-67566-y).

These are study-specific results, not universal benchmarks and not a baseline transferable without matched geometry, load paths, floor-cycle definitions and equipment scope. They demonstrate that platform mass, support strategy, modularity, synchronized lifting and component reuse are already active engineering frontiers. Luminous should not claim to have invented an intelligent climbing platform. The opportunity is to make the structure, temporary works, robot cells, material logistics, inspection and lifecycle data cohere as one qualified production system.

Doka's SKE plus also advertises crane-independent hydraulic repositioning, modular adaptation, multiple parallel climbing units and payload movement while climbing. This is another reason not to equate “remove the tower crane” with a defensible innovation claim.

### Measurement frameworks and deployment maturity

- The **Cambridge Framework for Construction Productivity**, published 30 September 2026, was developed using data from more than 25 UK projects and frames measurement at project, work-package and activity levels. We should align the project/work-package/activity reporting hierarchy rather than invent a competing generic productivity taxonomy. [Cambridge announcement](https://www.construction.cam.ac.uk/news/new-cambridge-framework-aims-transform-how-construction-productivity-measured-and-improved).
- The **NIST Performance Assessment Framework for Robotic Systems** composes task-driven perception, mobility, dexterity and safety measures into system-level assembly performance models. It is a manufacturing assembly framework, not a whole-construction-project productivity metric; use it for relevant robot component/task tests and keep accepted work-package productivity separate. [NIST framework](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems).
- A 2026 ISARC review assessed 134 peer-reviewed intelligent construction robot papers using a construction-oriented technology-readiness framework and reported only two at TRL 9 field validation. This is a review of its identified literature corpus, not a census of all commercial deployments. It supports a research emphasis on multi-condition field validation and workflow integration, not a claim that the industry has no deployed autonomous equipment. [ISARC 2026 paper](https://www.iaarc.org/publications/2026_proceedings_of_the_43rd_isarc_singapore/evaluation_of_technical_readiness_level_for_intelligent_construction_robots.html).
- **PAS 4010:2026**: BSI's press release and ICE both state that the specification was published on 9 November 2026, while the BSI Knowledge product page still shows a 30 November 2026 publication date. Treat the 9 November publication announcement as the current release claim, but record the catalogue-date discrepancy and use the authoritative purchased/downloaded final text for detailed clause mapping. The companion ICE guide says the PAS is intentionally non-prescriptive about the exact project metrics: teams must define context-specific measures, baselines and targets, and manage productivity across the full lifecycle/value chain. This reinforces our approach to report accepted output and whole-workflow economics without imposing one misleading universal scalar. [BSI publication announcement](https://www.bsigroup.com/en-GB/insights-and-media/media-centre/press-releases/2026/november/new-standard-to-help-improve-uk-infrastructure-productivity/), [ICE guidance (9 November 2026)](https://knowledgehub.ice.org.uk/guidance/pas-4010/), [BSI project timeline](https://standardsdevelopment.bsigroup.com/projects/2024-03460), [BSI product listing](https://knowledge.bsigroup.com/products/infrastructure-improving-productivity-in-infrastructure-development-and-delivery-specification).

## 2. The defensible competitive wedge

The opportunity is not another 4D planning UI and not a proprietary robot transport protocol. It is an **evidence-carrying transaction layer for physical production**:

`planning system -> normalized work contract -> capability/configuration admission -> current resource reservation -> authorized release -> machine/worker execution -> observation -> independent acceptance -> total workflow reconciliation`

The layer should preserve exact revisions, coordinate frames, site epochs, task identity, capability constraints, resource leases, material identity, required observations, decision-time knowledge and acceptance references across vendor boundaries.

A schedule is a proposal. A resource reservation is not a safety permit. A transport acknowledgement is not a completed task. Executor completion is not physical acceptance. Inspection completion is not necessarily a pass. A floor is not accepted until every required functional unit has its final accepted disposition.

The externally testable differentiator is whether this contract survives real systems and disturbances without silent semantic loss, stale-state reuse, unauthorized transitions, resource overbooking or false accepted-output credit—and whether that produces better whole-workflow economics in a matched trial.

Public product descriptions do not prove that no incumbent does this end-to-end. We need an adapter plugfest and matched trial to demonstrate the difference, not an unsupported uniqueness claim.

## 3. Paradigm shifts to pursue

### Apply PAS 4010's Avoid → Switch → Improve hierarchy before optimizing the robot

The 9 November 2026 ICE companion guidance to PAS 4010 explicitly orders productivity work as:
1. **Avoid:** test whether the need can be met without the proposed asset, operation or project scope at all.
2. **Switch:** where the need is real, find a fundamentally different design, material, method or delivery model that achieves the required outcome more productively.
3. **Improve:** only then optimize the selected method, reduce waste and improve resource efficiency.

This changes the Luminous optimization problem. A planner must not assume the conventional construction sequence, a climbing platform, a crane lift or a robot task is inherently necessary. It should first ask whether the required functional outcome can be delivered through reuse, design simplification, offsite manufacture, alternative assembly interfaces, reduced temporary works or a different sequence. Robot scheduling is the third-level lever, not the first.

The same guidance says there is deliberately no universal PAS productivity metric set: each project/value chain identifies context-specific indicators, defines baselines and targets, and measures effectiveness as well as efficiency over the lifecycle. The Luminous benchmark should therefore publish a small, owned metric set for each task/work-package class rather than create a universal score or compare heterogeneous units. This complements the accepted-output ledger: outcome adequacy and independently accepted quality come before output-per-input ratios.

Source: [ICE Guidance to PAS 4010:2026, published 9 November 2026](https://knowledgehub.ice.org.uk/guidance/pas-4010/). The PAS publication announcement is dated 9 November 2026, although BSI's product catalogue has a discrepant 30 November listing; use the authoritative final text for clause-level mapping.


### A. Compile the design into executable work contracts

Move from BIM as a visualization/coordination model to a versioned production contract. A work package should carry its geometry and frame, source revision, functional unit, material identity, prerequisites, capability envelope, resource needs, inspection method, postconditions and acceptance contract. The planner compiles candidate task graphs from those contracts; the physical execution layer checks each admission against current context.

This is not a replacement for IFC, BIM authoring, P6 or SYNCHRO. It is an assurance-bearing link between those systems and physical work.

### B. Treat the site as a space-time resource network

A crane, hoist, pump, robot, inspection team, material buffer, workface, access corridor, platform zone and survey-control area are all capacity-bearing resources with different semantics.

Represent:
- exclusive and capacity-limited time reservations;
- spatial occupancy and swept envelopes;
- setup, changeover, delivery, charging and inspection windows;
- resource availability, calibration, qualification and expiry;
- resource dependencies and shared failure domains.

The next research step beyond interval booking is **coupled spatial-temporal feasibility**: whether two tasks can coexist in the actual workface geometry and time window, not merely whether their resource IDs differ. This remains a planning/assurance model; it never replaces site safety controls or human clearance authority.

### C. Make information acquisition a scheduled production activity

A survey, material-lot check, tool calibration, inspection or as-built scan consumes time and resources but can prevent downstream rework or unsafe release. Schedule these as decision actions.

For each observation policy, compare the same hidden-fault scenarios and information available at decision time. Measure:
- direct observation and crew cost;
- downstream work preserved or invalidated;
- rework and waste avoided;
- delay and recovery time;
- residual uncertainty and false-negative rate.

Later, add explicit scenario probabilities and calibration. The current paired synthetic experiment assigns no probability and must not be called an expected-value estimate. Early inspection can be net-negative when the information does not alter the decision or when its cost exceeds the avoided loss.

### D. Optimize accepted production, not machine activity

Use task-specific functional units and independent acceptance predicates. Report first-pass yield, final acceptance, rework, rejected output, unresolved work, material/energy usage, crew time and whole-floor cost separately.

A robot that moves quickly but creates rework may lower project productivity. A slower process that produces accepted, reusable output may be economically superior. Robot utilization and cycle time remain diagnostic measures.

### E. Co-design the building, the workcell and the logistics system

Do not design the building, then bolt on a climbing platform and robots. Co-optimize:
- panel/module dimensions and connection topology;
- transport envelope, lift points, payload and staging;
- temporary works and support path;
- platform segmentation, access and reconfiguration;
- robot reach/tool changes and human access;
- inspection visibility and measurement datums;
- disassembly, repair, replacement and reuse.

The second-hoist fixture is a useful counterexample: increasing abstract hoist capacity saves nominal ticks, but the upper-duration case still misses the 17-tick deadline. The correct experiment compares capacity expansion against kitting, preassembly, buffer placement, alternative task routing, revised work-package sequence and geometric/workcell changes—then includes actual resource acquisition, installation, support, staffing, maintenance and lifecycle assumptions when field data exist.

### F. Treat recovery as evidence-bounded state reconciliation

Unknown machine, delivery, survey or workface state must remain unknown until reconciled. A communications outage is not permission to assume the last command completed or failed. A design revision invalidates affected dependent admissions. A search timeout is incomplete, not infeasible. A worker or machine may recover only through the applicable qualified process.

This is where the existing task state machine, site epochs, reservations and recovery owner should compose.

## 4. Benchmark suite

Use one frozen starting state, identical disturbance traces and the same acceptance contracts for every strategy. Report each metric separately; do not hide hard failures inside a weighted score.

| Layer | Benchmark | Required evidence |
|---|---|---|
| Interoperability | Two or more independently implemented adapters; semantic round-trip and differential corpus | Exact field/unit/frame/revision preservation or typed rejection; zero silent conversion |
| Task lifecycle | Duplicate, replay, stale sequence/session, failure, rejection, rework and reboot cases | Exact agreement with an independent finite-state oracle; zero invalid transitions |
| Scheduling | Tiny exact cases, increasing resource/task scale, hard gates, uncertainty and deadlines | Independently recomputed schedules, optimality gap only when bounded, p50/p95 solve time, explicit incomplete status |
| Resources | Crane/hoist/pump/workface/inspection/staging reservations and spatial occupancy | Zero admitted hard-capacity conflicts in frozen tests; false-positive/negative conflict rates against ground truth |
| Recovery | Hoist outage, late/wrong lot, blocked workface, design epoch, calibration invalidation, comms loss, weather/ODD exit, inspection reject | Time to safe disposition, replan time, accepted output lost, recovery labour, stale-work prevention |
| Quality | Independent inspection, measurement uncertainty, defects, corrections and reinspection | First-pass and final acceptance with exact denominators; zero false acceptance |
| Economics | Total workflow labour, material, equipment, logistics, inspection, energy, maintenance and financing | Fully burdened cost per accepted functional unit and accepted floor; all scope assumptions disclosed |
| Robustness | Upper-duration scenarios, common-cause faults, missing observations, second site | Held-out disturbances, site-transfer degradation, uncertainty calibration and unresolved state accounting |
| Robot components | Task-specific perception, mobility, dexterity, tool use and safe human interaction | NIST-aligned component/task test methods, reported separately from project productivity |

### Strategy baselines

- **B0 — fixed schedule:** no adaptive resequencing; declared safe hold on a hard-gate or resource failure.
- **B1 — deterministic rule recovery:** frozen priority and recovery rules; no future information.
- **B2 — evidence-bounded adaptive replan:** only admitted observations known at decision time; bounded search with explicit incumbent/bound/completeness status.
- **B3 — exact micro-oracle:** exhaustive or otherwise proved optimum for tiny instances only; a reference bound, not a field planner.
- **B4 — conventional/manual practice:** only after matched project time studies, task scope and independent acceptance data are collected.

Compare P6/SYNCHRO/ALICE/nPlan as planning and optimization ecosystems through supported exports, adapters and operator workflow. Do not claim a full product bake-off from public descriptions or from synthetic simulations.

## 4A. Competitive reality: what existing products and research already do

### November 2026 additions to the SOTA baseline

**Multi-robot task allocation:** Kaitha and Yu's 2026 ISARC paper, *Phase-Adaptive Large Language Models for Multi-Robot Task Allocation in Dynamic Construction Environments*, compares an LLM allocation strategy with DP/RL on a benchmark dataset. Its phase-adaptive objective shifts from task completion early in a project toward workload balance later, as remaining adjustment opportunities shrink. It reports performance comparable to RL with workload-balance benefits and emphasises interpretability/flexibility. This is an important matched baseline for our Symthaea allocation/planning policy; it is not evidence that LLM task allocation is superior on safety, accepted output or whole-floor economics. We should reproduce its workload-balance and task-completion metrics, then add the construction-specific hard gates and accepted-output denominators. [43rd ISARC Proceedings (2026), DOI 10.22260/ISARC2026/0066](https://doi.org/10.22260/ISARC2026/0066).

**Construction robotics deployment evidence:** a June 2026 PRISMA-based systematic review analyses 375 construction-robotics studies from 2023–2025. It finds research concentrated in structural construction, with planning, maintenance and demolition less explored; operator-led workflows remain common, and it identifies dynamic task assignment, real-time conflict resolution and multi-agent coordination as needs for future deployment. Treat this as a research-gap map, not a claim that all commercial fleets lack these features. [Results in Engineering (2026), DOI 10.1016/j.rineng.2026.110406](https://doi.org/10.1016/j.rineng.2026.110406).

**Agentic project-delivery systems:** a 7 October 2026 systematic review of 50 journal articles finds most agentic-AI construction work still at prototype/simulation stage with inconsistent evaluation, while highlighting reliability, interoperability, explainability, human oversight and data integration challenges. Its H-CAIL proposal connects context, multimodal perception, planning, reasoning, tool selection, human-governed action, evaluation and adaptive learning. This is another architecture comparator, but our differentiator should be evidence-qualified execution and reproducible acceptance rather than claiming a new agentic loop. [AI 2026, 7(10), 410, DOI 10.3390/ai7100410](https://doi.org/10.3390/ai7100410).


Do not claim that Luminous invented 4D construction planning, dynamic clash detection, resource calendars, or crane-space checking.

**Commercial capability baseline:**
- Bentley SYNCHRO 4D publicly documents model-based scheduling and 4D simulation, human/material/equipment/**space** resource assignments, dynamic spatial coordination tests for objects/equipment/work zones, baseline-vs-actual comparison, cost tracking and earned-value reporting. [Official SYNCHRO feature list](https://bentleysystems.service-now.com/community?id=kb_article&sysparm_article=KB0017688) · [Product overview](https://www.bentley.com/products/synchro-4d/).
- Autodesk Navisworks publicly documents federated-model review, clash/interference resolution, 4D TimeLiner simulation and cost-item import. [Official Navisworks overview](https://www.autodesk.com/products/navisworks/overview).
- These capabilities are meaningful industry baselines, not straw men. We should test interoperability and complementary workflow value rather than claim that Luminous replaces those products.

**Research state of the art to reproduce on matched inputs:**
- Dashti et al. (2021) already integrated 4D BIM, discrete-event simulation and RRT path planning to detect, resolve and reschedule time-space conflicts, with a real case study. This predates our work; time-space planning itself is not novel. [Automation in Construction, DOI 10.1016/j.autcon.2021.103957](https://doi.org/10.1016/j.autcon.2021.103957).
- Agrawal et al. (November 2025) combined linked data, construction-site point clouds, 4D BIM and a crane-space ontology for mobile-crane placement/rotation constraints, reporting mean F1 scores of 0.94 and 0.91 on two modular-construction sites. These are study-specific dataset results, not a universal target or score transferable to our AABB model. [Advanced Engineering Informatics, DOI 10.1016/j.aei.2025.103778](https://doi.org/10.1016/j.aei.2025.103778).
- Hosny et al. (2020) quantified uncertainty effects in construction workspace planning; their experiment observed collision-volume increases up to 220% for 40% input-parameter deviations. This supports uncertainty stress testing, not a universal multiplier. [Automation in Construction, DOI 10.1016/j.autcon.2020.103222](https://doi.org/10.1016/j.autcon.2020.103222).
- A 2026 systematic review of 375 construction-robotics studies highlights multi-agent coordination, dynamic task assignment, real-time conflict resolution and integration with construction safety zoning/scheduling as deployment research needs. It is a research agenda, not evidence that every commercial fleet lacks those features. [Results in Engineering, DOI 10.1016/j.rineng.2026.110406](https://doi.org/10.1016/j.rineng.2026.110406).

**2026 dynamic-scheduling baseline — an important new comparator:**
- Chen et al. (November 2026), *Adaptive job scheduling and resource allocation for industrialized construction processes using digital twin and reinforcement learning*, Advanced Engineering Informatics 76, 105166. Its disturbance-triggered DT-JSRA framework uses computer-vision-derived workstation/workforce/machine/material state and a DRL policy for adaptive job dispatch, worker allocation and shared-machine allocation; it was validated on a real wooden floor-panel production case. This is direct prior art for disturbance-triggered, multi-resource construction scheduling. Luminous must not claim to invent adaptive dispatch or disturbance-aware DT/DRL scheduling. [DOI 10.1016/j.aei.2026.105166](https://doi.org/10.1016/j.aei.2026.105166).
- Wang et al. (available online 14 August 2026), *Digital twin-enabled real-time heavy equipment scheduling with rolling-horizon optimization*, Construction Innovation. The study reports a 38.4% reduction in construction delay on a large hydropower project. Treat this as a study-specific reported result, not a transferable benchmark score or a matched comparison to our synthetic reference floor. [DOI 10.1108/CI-05-2025-0221](https://doi.org/10.1108/CI-05-2025-0221).
- Artigues, Hartmann and Vanhoucke's 2026 review of 50 years of RCPSP research highlights exact/heuristic methods, complexity and the importance of project data collection/calibration. A 2026 review of RCPSP variants identifies material arrival times, transfer times and lead times as less-studied uncertainty dimensions compared with activity duration/resource availability. [EJOR DOI 10.1016/j.ejor.2025.03.024](https://doi.org/10.1016/j.ejor.2025.03.024) · [IJPR review DOI 10.1080/00207543.2026.2683116](https://doi.org/10.1080/00207543.2026.2683116).
- AACE RP 70R-12 (18 August 2026) is a current practice reference for schedule contingency in EPC/capital construction projects. Contingency buffers and schedule risk management are established practice, so Luminous should benchmark them rather than present buffers as novel. [AACE RP 70R-12](https://web.aacei.org/docs/default-source/toc/toc_70r-12.pdf).

**Updated competitive differentiation hypothesis:** the contested space is not whether a system can replan after an outage. It is whether prequalified recovery alternatives can be bound to current evidence, exact site/design/frame/configuration epochs, spatial/temporal reservations, capability and material qualifications, approval roles, and whole-floor accepted-output/cost consequences—and whether the system abstains when the trigger is unknown. Test that hypothesis against C0 fixed schedule, C1 deterministic recovery rules, C2 rolling-horizon optimisation, C3 disturbance-triggered DT/DRL (when reproducible on common inputs), and C4 a tiny exhaustive contingent-policy oracle. An unreplicated paper result is a literature comparator, not an exact-head bake-off.

**The defensible Luminous differentiation hypothesis is the assurance composition, not 4D geometry alone:**
- bind occupancy to current site epoch, design digest, coordinate frame and instrument/calibration context;
- use declared geometry and time uncertainty with fail-closed unknown/stale semantics;
- join spatial conflicts to existing resource reservations, task lifecycle, independent acceptance receipts and accepted-floor economics;
- expose complete conflict sets and reject partial reservation/occupancy mutations in the local reference model;
- plan information acquisition through scenario-wise value of information without invented probabilities;
- retain accountable human/site authority for permits, lifts, safety and professional disposition.

Public feature pages do not establish whether SYNCHRO, Navisworks or other enterprise deployments can or cannot supply all of those evidence semantics. That question requires a real adapter/conformance test and vendor/operator review. Do not turn missing public documentation into a claim of missing product capability.

**Head-to-head protocol before any “better than SOTA” claim:**
1. Freeze the same IFC/4D model, task graph, work-zone geometry, resource capacities, uncertainty envelope, timebase and hidden-state challenge for every implementation.
2. Compare SYNCHRO/Navisworks-supported workflows, B1 nominal geometry, B2 uncertainty-expanded geometry, B3 our indexed broad phase plus exact AABB oracle, and B4 an independent narrow-phase geometry implementation where available.
3. Report precision/recall/F1 against independent conflict labels, false-clear and false-conflict counts, stale-context rejection, candidate-pair reduction, memory, p50/p95/p99 latency, schedule disruption and conflict-resolution time.
4. Separately report independently accepted output, first-pass yield, rework, total workflow labour, unallocated shared cost, task-attributed cost, energy/material, and time-to-accepted-floor.
5. Publish versioned adapters, raw fixtures, exact input/output digests, vendor/product version, configuration and assessor protocol. Missing data or incompatible interfaces are a disclosed non-comparison, never a guessed win.

A valid first competitive milestone is **matched semantic parity plus stronger evidence traceability**. A performance win comes only after independent replication on at least two site datasets and an equal-scope vendor baseline.

## 4B. Updated competitive baseline — 13 November 2026

A stricter review changes the novelty claim: **time-space clash detection, 4D schedule/resource linking, and resource-cost tracking are already established commercial capabilities.** Luminous should integrate with them, not claim to originate them.

### Product capabilities that must be treated as parity baselines

- **Autodesk Navisworks:** TimeLiner links external schedules to model objects, compares planned and actual dates, assigns task costs and updates simulations when models/schedules change. Autodesk documents linking TimeLiner, Object Animation and Clash Detective to run clash tests on fully animated schedules, including moving cranes and adjacent workgroups. This is already a time-dependent workspace-conflict capability. [TimeLiner overview](https://help.autodesk.com/cloudhelp/2025/ENU/Navisworks-Timeliner/files/GUID-D0D36E3D-F1D0-43B6-AB4E-2E7799B340A3.htm) · [Clash Detective overview](https://help.autodesk.com/cloudhelp/2024/ENU/Navisworks-Clash-Detective/files/GUID-36D9904E-12F3-4F82-8DD3-C2103DB0BC29.htm).
- **Bentley SYNCHRO 4D Pro:** vendor documentation lists a CPM engine, schedule import/synchronization, resource assignment for people/material/equipment/space, cost and earned-value tracking, plus 4D spatial-coordination tests for dynamic clashes between objects, equipment and work zones. This is a very close product baseline for our initial scheduling/occupancy thesis. [SYNCHRO feature list](https://bentleysystems.service-now.com/community?id=kb_article&sysparm_article=KB0017688) · [SYNCHRO 4D product](https://www.bentley.com/products/synchro-4d/).
- **Oracle Primavera P6 / ALICE / nPlan:** retain these as schedule, optimization and forecasting comparators. They are not to be represented as inferior merely because our synthetic prototype is easier to inspect.

Accordingly, do not position a 3D AABB grid, 4D timeline, named resource reservation, dynamic clash report, or cost roll-up as a field-level paradigm breakthrough. Those are implementation primitives or existing product categories.

### Current research baselines that narrow the gap further

- **Sensing-to-scheduling with explicit acceptance evidence:** Isaac and Lucko's 2026 ISARC paper separates production-facing work measurement from time-based schedule control and translates measured work, backlog/forelog and QA/inspection/approval evidence into schedule forecasts and recovery targets. This reinforces the importance of keeping work quantities, time forecasts and formal acceptance distinct. [ISARC 2026, DOI 10.22260/ISARC2026/0106](https://doi.org/10.22260/ISARC2026/0106).
- **Frequent robotic site digitalization:** Haage, Erikshammar and Stehn's 2026 field-grounded roadmap shifts attention from robot capture to the complete capture-to-consumption chain. It proposes a persistent production-phase evidence layer and explicitly cautions against direct overwrite of the digital twin/Site Utilization Plan from raw robot observations. The study is a longitudinal single-case roadmap hypothesis, not a validated universal operating model. This is a strong reason to make evidence admission and controlled revision proposals central to our product. [Buildings 2026, 16(15), 2988](https://doi.org/10.3390/buildings16152988).
- **Construction AMR logistics:** a 2026 study of resource scheduling and delivery combined a construction-constrained VRP, adaptive optimization and 6-DoF grasping, reporting a 39.2% reduction in total waiting time in its experimental setup. Treat this as a study-specific result; reproduce the waiting-time/task/robot conditions before comparison. It means that material delivery, resource reuse, pick reliability and idle time must be part of our competitive benchmark, not just schedule makespan. [Automation in Construction, S0926580526002098](https://www.sciencedirect.com/science/article/pii/S0926580526002098).
- **Robot component measurement:** NIST's Performance Assessment Framework composes task-driven perception, mobility, dexterity and safety measurements into system-level performance models. We should map these component measures to construction tasks while retaining accepted work and project economics as separate system outcomes. [NIST Performance Assessment Framework](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems).
- **Productivity as effectiveness plus efficiency:** ICE's 9 November 2026 guidance to PAS 4010:2026 puts Avoid → Switch → Improve in order and emphasizes lifecycle outcomes, cross-value-chain collaboration and project-specific baselines/indicators. We should not assume the correct intervention is always more automation or another crane; avoiding or redesigning a movement may dominate optimizing it. [ICE PAS 4010:2026 guidance](https://knowledgehub.ice.org.uk/guidance/pas-4010/) · [ICE launch note](https://www.ice.org.uk/news-views-insights/latest-news/pas-4010-productivity-standard-and-guidance-launched).

### What Luminous should actually attempt to make better

Our differentiator hypothesis is the evidence/authority composition across an existing industrial toolchain:

1. **Evidence freshness as a hard execution dependency:** every work proposal binds the exact design revision, site epoch, frame/control evidence, machine/tool configuration and relevant calibration. A changed dependency invalidates affected work eligibility and triggers reconciliation.
2. **Resource + geometry admission with explicit uncertainty:** keep the existing resource-calendar semantics and geometry/time conflict semantics distinct, but compose them with all blockers visible. A reservation is not a physical clearance or lift permit.
3. **Independent acceptance as the production unit:** a robot completion, schedule percent-complete, scan observation or installation event is not accepted output. Only a task-bound independent acceptance record earns output credit; rejection, corrective work and reinspection stay in the cost denominator.
4. **Active metrology with decision value:** choose surveys/inspections only where they change a downstream decision; unknown, occluded, stale or unavailable evidence remains unknown. Report observation burden and scenario-wise VOI separately.
5. **Whole-floor economics:** reconcile task-attributed and shared labour, equipment, materials, logistics, inspection, energy, rework, maintenance and unallocated cost against finally accepted functional units. Do not compare a robot's cycle rate with a project's delivered value.
6. **Lifecycle reuse:** connect accepted component identity, approved substitutions, custody, inspection, warranty, repair, replacement and disassembly—not merely installation.

This is not yet demonstrated as superior to SYNCHRO/Navisworks. The current public work is a synthetic research prototype, and product pages do not prove what a particular customer's configured deployment can or cannot do. The missing proof is a real adapter/conformance experiment and independent field comparison.

### Required head-to-head benchmark

Freeze one common data pack for every comparator: identical IFC/model export, task graph, schedule and calendar, work-zone/temporary-works geometry, equipment paths/envelopes, capacity, site epoch/revision, uncertainty inputs, observation/inspection records, hidden faults and acceptance contract.

Compare at least:
- **C0:** Navisworks TimeLiner + animated Clash Detective workflow;
- **C1:** SYNCHRO 4D resources + 4D spatial-coordination workflow;
- **C2:** current reference full-scan resource calendar and exact AABB conflict oracle;
- **C3:** indexed temporal + spatial broad phases with exact final predicates;
- **C4:** resource-constrained schedule/adaptive-recovery baseline (e.g. P6/ALICE-supported export or an independent exact micro-oracle on small cases);
- **C5:** Luminous evidence-qualified execution and accepted-floor accounting, using the same upstream schedule/geometry outputs.

Report separate metric families—no single weighted score may hide a hard failure:

| Metric family | Measures |
|---|---|
| Geometry/time conflicts | Precision, recall, F1, false-clear, false-conflict, conflict localization and time-to-resolution |
| Context/evidence | Stale revision/frame/epoch detection, evidence provenance completeness, calibration/coverage validity, invalidation latency |
| Scheduling | Feasibility, makespan, tardiness, critical-resource utilization, solve time p50/p95/p99, incumbent/optimality gap and incomplete-search rate |
| Logistics | Material on-time/kitted rate, hoist/crane queue and wait, rehandling, delivery reliability, pick/grasp success and recovery time |
| Robot task capability | NIST-aligned perception, mobility, dexterity and safety tests for the specific task family |
| Quality/output | First-pass yield, final acceptance, defect/rework/reinspection, accepted functional units per workflow crew-hour |
| Project economics | Fully burdened cost per accepted unit and floor; direct/shared/unallocated costs, labour, energy, equipment, materials and lifecycle maintenance |
| Observation value | Observations per accepted unit, time/crew/energy per qualified observation, decision-changing rate, scenario-wise VOI/regret and false-clear controls |
| Transferability | Held-out disruptions, second-site reproduction, independent adapter implementation and assessor agreement |

For the first synthetic milestone, require exact agreement with independent small-case oracles, zero false-clear negative controls, zero stale-context admission, 100% attempt/acceptance/cost reconciliation and deterministic replay. For physical trials, use matched scopes, measured baselines, independent acceptance, raw evidence and confidence intervals. The existing target of >=20% crew-hour reduction remains a hypothesis until a matched field study supports it.

## 5. Target ladder

The following are proposed research targets, not achieved results:

### P0 — executable synthetic semantics

- 100% agreement with exact small-instance schedules and an independently implemented lifecycle oracle.
- Zero false acceptance, stale-revision release, duplicate-cost inflation, overlapping-person-time inflation or hard resource/gate violation in frozen positive/negative controls.
- 100% of required units, attempts, failures, rework and unresolved states reconciled.
- Deterministic replays of the same frozen inputs produce identical structured reports.
- Search exhaustion is always reported as incomplete rather than infeasible or optimal.

### P1 — matched production experiment

- At least 20% reduction in total workflow crew-hours per finally accepted matched work package, without reducing required scope or inspection stringency.
- p90 time to finally accepted floor no worse than the matched conventional baseline.
- Report first-pass yield, rework, accepted units/crew-hour, hoist/crane occupancy and wait, material loss, energy and fully burdened cost separately.
- Measure disruption recovery against B0 and B1; publish effect sizes and uncertainty instead of declaring a universal threshold from synthetic ticks.

### P2 — transfer and real execution

- Two independently implemented vendor adapters, hardware-in-the-loop and fault injection.
- Second-site replication with exact revision/epoch, measurement method, raw evidence, human interventions and lifecycle cost.
- No increase in safety exposure or reduction in independent acceptance rigor to achieve a productivity gain.

The existing programme target of >=20% crew-hour reduction is a sensible hypothesis to retain. It becomes a result only after matched, independently accepted work and complete workflow accounting support it.

## 6. Immediate next experiments

1. **Accepted-floor ledger (now in PR #4960):** compose the existing task lifecycle and resource-reservation owners with required work units, attempts, person-time and costs.
2. **Reference-floor adapter integration:** run #4939's four frozen cases and ten disturbances against #1902 and #1843 without duplicating either owner.
3. **Vertical-logistics co-design:** [#4964](https://github.com/Luminous-Dynamics/symtropy/issues/4964) compares abstract extra capacity, delivery/kitting windows, temporary buffers, alternative routing, preassembly and sequence changes. Optimize an explicit total-cost function; do not infer that another hoist is the solution.
4. **Inspection value-of-information:** pair early and deferred detection on identical hidden-fault cases; later add calibrated probability distributions and decision-regret measurement.
5. **Benign panel-cell trial:** start with non-load-bearing panel fabrication/installation, metrology, rejection, repair, reinspection and accepted-output economics before any high-rise actuation.
6. **Lifecycle thread:** keep material identity, approved substitution, custody, inspection, warranty, repair and replacement linked to the accepted component, not merely its installation event.

## 7. Source register

- VDA 5050 v3.0.0, March 2026: [official PDF](https://www.vda.de/dam/jcr%3A09f03b91-13e2-4db3-bf30-4f221710071b/VDA5050-V3.0.0-2025-03.pdf).
- Oracle Primavera P6: [official product overview](https://www.oracle.com/construction-engineering/primavera-p6/).
- Bentley SYNCHRO 4D Pro: [official feature list](https://bentleysystems.service-now.com/community?id=kb_article&sysparm_article=KB0017688).
- Doka SKE plus: [official product overview](https://www.doka.com/en/system-groups/doka-climbing-systems/automatic-climbing-formwork/automatic-climbing-formwork-ske-plus/index).
- Cat Command: [official product overview](https://www.cat.com/en_US/by-industry/construction-industry-resources/technology/command.html).
- Lightweight aerial construction machine: [Scientific Reports (20 August 2026)](https://www.nature.com/articles/s41598-026-67566-y).
- Cambridge Framework: [Cambridge Laing O’Rourke Centre (30 September 2026)](https://www.construction.cam.ac.uk/news/new-cambridge-framework-aims-transform-how-construction-productivity-measured-and-improved).
- NIST Performance Assessment Framework: [NIST](https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems).
- TRL review: [ISARC 2026](https://www.iaarc.org/publications/2026_proceedings_of_the_43rd_isarc_singapore/evaluation_of_technical_readiness_level_for_intelligent_construction_robots.html).
- PAS 4010:2026 publication and guidance: [BSI press release (9 November 2026)](https://www.bsigroup.com/en-GB/insights-and-media/media-centre/press-releases/2026/november/new-standard-to-help-improve-uk-infrastructure-productivity/), [ICE implementation guide (9 November 2026)](https://knowledgehub.ice.org.uk/guidance/pas-4010/), [BSI project timeline](https://standardsdevelopment.bsigroup.com/projects/2024-03460), [BSI product listing](https://knowledge.bsigroup.com/products/infrastructure-improving-productivity-in-infrastructure-development-and-delivery-specification).

## 8. Claim ceiling

This is a competitive research plan and synthetic software prototype. No Luminous physical construction robot, climbing system, safety function, productivity gain, construction cost reduction, standards conformity or field deployment is established by this note. All physical work requires project-specific engineering, qualified human authority, suitable inspection/metrology and independently approved safety arrangements.
