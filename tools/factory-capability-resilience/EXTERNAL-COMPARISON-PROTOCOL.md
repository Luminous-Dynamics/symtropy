# External comparison and SOTA protocol — Evidence-Gated Industrial Intelligence

**Scope:** research positioning and benchmark protocol as of 2026-11-17.  
**Status:** literature/vendor/standards review; not a claim of comparative performance.

## 1. The field is converging on context, stability, and closed-loop operation

Three complementary directions are now visible:

### SLR2 — incumbent-aware selective repair

Lin and Yang, *An incumbent-aware selective-repair framework for dynamic flexible job shop scheduling*, Computers & Industrial Engineering, November 2026, DOI: https://doi.org/10.1016/j.cie.2026.112299

The paper uses a heterogeneous schedule graph, a dual-head GNN that partitions active-window operations into keep/release sets, and a reduced CP-SAT repair model. Its abstract reports:
- held-out Brandimarte Mk6–Mk10: lowest mean makespan and total tardiness among seven methods using ten common random seeds; 14.6% lower changed-operation ratio; half the changed-machine ratio; 1.5× faster than full re-optimization;
- full Mk1–Mk10: 20.4% lower repair scope and 2.06× solver speedup;
- zero-shot transfer of a frozen policy to synthetic instances five times larger while retaining the stability advantage.

These are the authors' results. They are a strong, direct comparator for our selective repair work; do not quote only one headline number or treat a different synthetic corpus as a reproduction.

**Required Luminous comparison:** reproduce the same Brandimarte instance semantics, disruption model, active-window definition, seven comparator arms (or document deviations), ten shared random seeds, stopping budget, and all incumbent-quality/stability/runtime measures. Add our distinct outcomes only in a separately reported extension: qualified route validity, evidence reacquisition, false-preserve / false-release, first-article disposition, and time to first independently accepted output. If the authors' implementation, training protocol, or full data are unavailable, report “paper-abstract comparison only”; do not invent a local SLR2 reproduction.

Source: https://www.sciencedirect.com/science/article/pii/S0360835226005000

### Document-driven masked scheduling / twin closed loop

*Document-driven masked scheduling for flexible job shops: From standardized semantics to twin-closed-loop deployment*, Computers & Electrical Engineering, Vol. 139, Part A, November 2026, Article 111371, DOI: https://doi.org/10.1016/j.compeleceng.2026.111371

The paper compiles STEP-NC, OPC UA, ISA-95 and ISO 22400 artefacts into capability/compliance and maintenance constraints. It applies triple masks over precedence, capability/compliance and availability; couples a masked Transformer PPO policy to a genetic algorithm; and uses an OPC-UA-enabled twin for event-triggered schedule updates with an explicit stability cost. Its abstract reports 3–6% reductions in makespan/tardiness, about 99% one-shot feasibility, 70% lower repair rate, and 25–35% faster disturbance recovery on its listed FJSP/online benchmark experiments, with five-seed mean/std and significance testing.

These are paper-specific claims; they are not directly comparable to #5520's pump/valve synthetic episodes.

**Required Luminous comparison:** implement precedence, capability/compliance and availability masks as a transparent comparator; implement schedule-stability cost explicitly; use the same benchmark instances and disturbance/event traces; report the same objective, seed, hardware, and compute budgets. Then add separate evidence and qualification outputs. A schedule with 99% one-shot feasibility is not proof of material conformity, valid inspection, correct genealogy, or accepted product.

Source: https://www.sciencedirect.com/science/article/pii/S0045790626004416

## 2.5 Process-model reconfiguration is a separate correctness problem

An additional high-value comparator has emerged: **FJSP-E2E: A process-verified framework for automatic scheduling-model reconfiguration in smart manufacturing**, DOI https://doi.org/10.1016/j.jmsy.2026.08.009.

The publisher's indexed record currently assigns it to *Journal of Manufacturing Systems*, Volume 89, December 2026, which is later than this review date (2026-11-17). The article page is discoverable, but its final online-publication date should be verified before treating it as an already-published November baseline. The current record describes:
- 72 expert-audited FJSP reconfiguration tasks across three levels;
- translation of shop-floor updates into executable model edits and CP-SAT structural extensions;
- process diagnostics for omissions involving routes, tools, workers, AGVs, and calendars;
- diagnostic fields explicitly described by the authors as structural proxies, not complete formal verification of the CP-SAT constraint graph;
- a reported 93.1% few-shot pass rate for Claude Sonnet 4.5 and only 4.2% for a Qwen3-8B model fine-tuned on 4,500 synthetic samples.

Treat these as author-reported experimental results, and do not interpret the pass rate as physical production readiness or formal model proof.

**Why it matters:** an AI-generated scheduling model can return a plausible objective and a feasible schedule while silently omitting a required worker, tool, transport, maintenance, material, process-envelope or quality constraint. Objective-value agreement alone does not establish that the model expresses the intended manufacturing problem.

**New Luminous opportunity — process-model compiler assurance:**
1. Compile a typed change request into an explicit change set over the canonical process/resource/qualification model; keep natural-language generation outside the authoritative validator.
2. Require a trace from each source requirement and process artifact to the executable constraint or an explicit unsupported/unknown disposition.
3. Mutate one obligation at a time (remove a worker constraint, change a machine eligibility, drop a setup transition, omit a transport edge, stale a calibration, erase a quality hold, or change a product revision) and check that the independent verifier detects the semantic violation.
4. Include **objective-preserving mutants**: deliberately omit a constraint that happens not to change the fixture's optimum. These are the strongest controls against a system that only checks makespan.
5. Compare generated models against canonical small-instance semantics and independent finite oracles; do not let the model generator write its own expected answers.
6. Report semantic obligation coverage, mutation detection, false acceptance, unsupported handling, exact schedule correctness, and recovery cost separately.

This is a natural composition with FJSPLib import and exact scheduling (#5459/#5446), the canonical process owner (#5686), and Symthaea's engineering knowledge/inspectability work (#10702). It must not create a second process ontology or solver.

Source record: https://www.sciencedirect.com/science/article/pii/S0278612526002219

## 2.6 Factory resilience must be measured across the disruption lifecycle

Bushagour et al., *An analysis of indicators for assessing manufacturing resilience at the factory level*, *Production & Manufacturing Research* 14(1), article 2738298, published online 29 September 2026, DOI https://doi.org/10.1080/21693277.2026.2738298, reviews 93 factory-level resilience indicators.

Its important conclusion for this programme is methodological: resilience is time-dependent. Before a disruption, topological and attributional indicators characterize properties such as flexibility, modularity, redundancy, connectivity and reconfigurability. During and after a disruption, performance indicators become necessary to quantify degradation, robustness and recovery. The review also highlights gaps in validation of topology/attributional indicators against performance, human performance and safety measures, and multi-scale facility/factory/supply-chain assessment.

This supports a three-stage scoreboard rather than a static redundancy score:
- **Before disruption:** qualified path topology, independence evidence, capacity/options, capability coverage, human/authority coverage, and inventory completeness.
- **During disruption:** detection/diagnosis delay, service degradation, impacted claims, valid alternatives, quarantine and decision latency.
- **Recovery/after:** time to first independently accepted output, lost/late output, requalification effort, human workload, safety/quality disposition, and retained capability.

Do not claim the number of independent synthetic paths predicts real factory resilience until it has been validated against dynamic recovery performance. A topology metric is a hypothesis about future performance, not a substitute for it.

Source: https://doi.org/10.1080/21693277.2026.2738298

## 2.6.1 Execution truth versus execution registration: add a factory reality-gap benchmark

Smolenaars, Akcay and Adan, *Ramp-up performance measurement in job–shop systems using real-time location data: a case study in semiconductor manufacturing*, *Production & Manufacturing Research* 14(1), article 2738937, published online **2 October 2026**, DOI https://doi.org/10.1080/21693277.2026.2738937, report a real RTLS deployment alongside MES at SMART Photonics during a photonic integrated-circuit foundry ramp-up.

The study distinguishes:
- manufacturing performance indicators (how the system performs); and
- registration-accuracy indicators (how accurately the information system records what is happening).

The authors report that real-time location data reveals actual product pathways and traceability information that conventional WIP/MES records may not capture, including unregistered production steps and differences between assumed and actual product pathways. The article also discusses practical limits such as sensor coverage, position flipping between adjacent stations, and the need to combine RTLS with MES rather than treating RTLS as the only source.

**Benchmark consequence:** the factory intelligence corpus needs three independent views and one derived reconciliation:
1. intended route / work order;
2. recorded MES/ERP/QMS/scan events;
3. bounded observations (RTLS, equipment, metrology, operator or other authorized evidence);
4. reconciled execution state with explicit uncertainty, provenance and affected-claim disposition.

Do not make RTLS the physical ground truth. Position evidence establishes location only within a validated coverage, calibration, identity, map-version and uncertainty envelope. A machine cycle does not prove which product was processed unless product identity is independently bound. Absence of a signal during an observation gap is UNKNOWN, not proof that an operation did not happen. Event/effective time and ingest/observation time must remain distinct.

For comparison, preserve the paper's registration-accuracy measures and the factory's existing performance metrics, then add:
- route/event reconstruction under a declared observation-coverage denominator;
- identity/serial/lot join correctness;
- missing/duplicated/unregistered event detection;
- route sequence and rework-loop discrepancy;
- coverage-aware UNKNOWN vs NOT_OBSERVED_WITH_COVERAGE correctness;
- discrepancy-to-correct-disposition latency;
- affected genealogy/claim closure and time to first independently accepted output.

This is an important complement to the SLR2 and document-driven scheduling papers: an excellent schedule or a successful rescheduling loop cannot establish that the physical product followed that schedule. The plan, recorded history and observed history must be reconciled.

Source: https://doi.org/10.1080/21693277.2026.2738937

## 2.7 Industrial policy-governed AI is already moving beyond anomaly prediction

Lin and Chen, *An integrated framework featuring policy-governed agentic AI for closed-loop manufacturing control with multi-source sensor–MES–ERP*, *The International Journal of Advanced Manufacturing Technology*, published 17 March 2026, DOI https://doi.org/10.1007/s00170-026-17806-2, report evaluation on 25,275 manufacturing records and author-reported improvements in anomaly accuracy, false alarms, monitoring robustness and operational efficiency. Their architecture combines sensor/MES/ERP context, temporal alignment, edge anomaly models, capability-aware limits, policy-governed action constraints, digital-twin checks and human review.

This is a relevant industrial comparator, not a directly matched result. The abstract's aggregate percentages do not expose a common benchmark protocol with our synthetic scenarios, and the publisher page exposes the abstract while the full article is subscription-gated.

Our distinction must therefore be measurable, not rhetorical: compare the systems on matched events and decision budgets, then report whether each policy correctly preserves human authority, evidence independence, claim-specific qualification and accepted-output criteria. Use their reported improvements as context only; do not compare percentages from different datasets.

Source: https://doi.org/10.1007/s00170-026-17806-2

## 2. FJSPLib: the solver tournament must be an honest engine comparison

Official benchmark: https://scheduleopt.github.io/benchmarks/fjsplib

The public inventory consists of 452 logical instances: 138 classic FJSP, 264 machine-independent FJSP, and 50 flexible assembly FJSP. The upstream text representations comprise 402 FJSP and 452 FAJSP files (854 files), with 402 paired encodings. The official protocol compares CPLEX, CP Optimizer, OR-Tools CP-SAT and OptalCP, with a 600-second/10-minute reference budget and published best-known lower/upper bounds.

**Protocol for #5528 / #5459:**
1. Treat 452 logical instances as the statistical unit; never report 854 encodings as 854 independent instances.
2. First prove byte/source provenance and parse equivalence for paired representations; preserve FAJSP assembly DAG semantics.
3. Reproduce the official engine/version/parameter and time-budget details to the extent licenses and software availability permit. Record every deviation.
4. Use the same pinned hardware, core count, memory, OS, solver versions, random seeds, wall-clock budget, and parallelism. Keep a separate CI correctness lane from the performance lane.
5. For each case retain source revision/blob identity, normalized input digest, seed, solver configuration, incumbent, lower bound, status, elapsed wall/CPU time, memory, termination, and verifier receipt.
6. Separate closed instances (proof/defensible bound match) from open instances (primal/dual gap). Never call a best-known solution optimal without a defensible proof.
7. Compare geometric lower-bound ratio, geometric upper-bound ratio and shifted geometric gap using the benchmark's definitions; report per-family and per-instance distribution, not just an aggregate.
8. Train/choose the portfolio on one partition and assess once on held-out logical instance IDs. Include selector inference cost, failed/timeout solver starts and runtime overhead. The portfolio must beat the best single engine on held-out instances—not just the mean of engines chosen after seeing the test set.

A portfolio that cannot beat the best individual engine after selection cost is not a useful portfolio. The value proposition may still be robustness or anytime quality, but those must be measured independently.

### Proposed portfolio research

Features must be computed from each instance without access to BKS/held-out performance: operation count, machine count, flexibility distribution, operation-alternative entropy, precedence DAG width/depth, critical-path lower bound, machine-load bounds, assembly joins, and structural density. Avoid hand-picking algorithm based on known answer.

Compare at least:
- CP-SAT;
- exact Rust oracle only within its declared finite envelope;
- one documented constructive heuristic;
- one local-search / large-neighbourhood policy;
- the learned/feature-based portfolio selector;
- best-single-engine oracle (evaluator-only, to quantify selector regret).

Report PAR-2/PAR-10 or another explicitly defined timeout-penalized score alongside gap and quality metrics; do not allow timeouts to disappear from averages. Keep hardware normalization and energy reporting.

## 3. NIST: align with the measurement programme, do not claim to outrun it

NIST's Super Intelligence for Manufacturing programme describes TIDAL (Teamed, Integrated, DT-assured and Layered Intelligence), with asset/site/enterprise layers, human decision owners, industrial data and digital-twin foundations, a shared test bench spanning discrete/additive/continuous processes, and benchmarks for planning, scheduling, monitoring and root-cause analysis. Its integration-test-bench work describes UNS over MQTT, knowledge graphs/type models, I3X and OPC UA; its initial process is Tennessee Eastman, with planned extensions to NIST DT and the Additive Manufacturing Data Integration Lab. It names accuracy, latency, cost and grounding metrics and context ablations; benchmark software/toolkit is planned for FY28.

Source: https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing

NISTIR 8620 (July 21, 2026) records workshop priorities including interoperability, VVUQ, cybersecurity and workforce readiness:
https://www.nist.gov/publications/digital-twins-workshops-summary-report

**Luminous positioning:**
- Align asset/site/enterprise layers and human decision ownership.
- Provide adapters and test profiles for OPC UA, ISA-95, UNS/MQTT and relevant knowledge-graph/type-model interfaces; do not build a proprietary replacement for every plant data system.
- Treat evidence qualification, claim-specific requalification, selective repair and first accepted output as a complementary benchmark dimension.
- Publish an open synthetic-first corpus, schema, deterministic replay receipts, negative controls and independently checkable tiny oracles now.
- Where possible, engage NIST/MEP/industry/standards stakeholders as benchmark participants or reviewers. Our goal should be interoperability and reproducibility, not a premature “better than NIST” claim.

NIST's programme is active and developing its test bench. It is a reference direction, not an already-frozen external leaderboard against which this PR can claim a win.

## 4. Commercial industrial platforms: complement the plant system of record

Siemens describes Module Type Package (MTP) as plant-tested modular automation with plug-and-produce integration, and advertises up to 50% lower time-to-market and 70% lower engineering effort. These are vendor-reported maxima, not independently verified results and not a matched comparison with this work.

Source: https://www.siemens.com/en-gb/company/insights/modular-production/

Siemens Opcenter describes a MOM portfolio linking PLM, production automation, production planning, quality operations, real-time visibility and closed-loop improvement:
https://www.siemens.com/en-us/products/opcenter/

### Competitive distinction

Do not position Symtropy/Symthaea as another MES/MOM, finite scheduler, or plug-and-produce interface. Those are established product categories with mature operational integrations.

Position the additional layer as cross-system **capability and evidence assurance**:
- Is this exact operation/resource/process/material/person/inspection combination qualified for this product revision and operating envelope?
- Which evidence is current, applicable, independent and authoritative?
- When a module, process, material, software generation or partner changes, exactly which claims and commitments become invalid?
- What minimum work is needed to requalify while preserving proven unaffected operations?
- Can the new module or route produce an independently accepted first article?
- Can an auditor replay why the decision was admissible?

The integration benchmark should consume vendor/MES/PLM/SCADA evidence and send bounded recommendations/receipts back through authorized interfaces. It should not assume authority to command plant equipment or release product.

To compare commercial systems fairly, use a matched A→B module/configuration-change episode and measure engineering time, schedule disturbance, qualification scope, time to first accepted output, genealogy coverage and recovery under common-cause events. Record baseline workflows and vendor assistance. Do not compare against a vendor's “up to” claim unless its denominator, scope, plant, baseline, and measurement method are available.

## 4.1 Compare against full industrial APS workflows, not a weak scheduler baseline

The vendor comparison must start from what mature APS platforms already claim to do.

**Siemens Opcenter APS** publicly describes:
- multi-horizon production planning and detailed scheduling;
- resource availability, materials, capacity, work orders, labor and tooling;
- order-based multi-constraint sequencing;
- concurrent resource-use rules and inter-operation time constraints;
- sequence-dependent changeovers, operation-specific run speeds, overlap and slack;
- priorities, due-date negotiation, split batches and interactive schedule views;
- ERP-connected planning and rescheduling around production disruptions.

Sources:
- https://www.siemens.com/en-us/products/opcenter/production-planning-scheduling-capabilities/
- https://www.siemens.com/en-us/products/opcenter/advanced-planning-scheduling-aps/advanced-scheduling-software/
- https://www.siemens.com/en-us/technology/manufacturing-scheduling-software/

Siemens' Opcenter APS page also displays vendor-reported figures such as +25% productivity, -50% inventory and +50% on-time delivery. These are marketing/product-page claims with no matched denominator or common benchmark protocol in the retrieved page; do not use them as directly comparable measured performance.

**Dassault Systèmes DELMIA Ortems** publicly describes:
- finite-capacity optimization over machines, materials, workforce and process constraints;
- long-, mid- and short-term planning;
- ERP/MES/operational-data integration;
- rescheduling after late deliveries, breakdowns, labor shortages and quality issues;
- virtual-twin scenario simulation and KPI analysis;
- multi-resource planning, including tools, operators and subcontractors.

Sources:
- https://www.3ds.com/products/delmia/ortems
- https://www.3ds.com/products/delmia/ortems/production-scheduler
- https://www.3ds.com/products/delmia/ortems/manufacturing-planner

These products are serious competitors in conventional APS scope. We should not claim they lack finite-capacity modelling, resource constraints, scenario simulation, MES integration or disruption rescheduling.

### Fair matched industrial comparison protocol

Build a matched evaluation in which all candidate systems receive:
- the same product and engineering revision;
- the same process/resource/capability/qualification requirements;
- the same order set, WIP, material lots, worker/tool calendars and utilities;
- the same disruption and late-information event trace;
- the same permitted action/authority boundary;
- equivalent wall-clock, compute and human-planning/review budgets.

Compare at least:
- current customer/plant workflow (MES/ERP + planner/manual recovery);
- Siemens Opcenter APS or a documented Opcenter workflow;
- DELMIA Ortems or a documented Ortems workflow;
- open CP-SAT / exact-oracle reference where available;
- evidence-gated Luminous policy with the same scheduling engine budget;
- evidence-gated Luminous policy plus selective inspection and dependency-closure repair.

When a vendor implementation, license or test environment is unavailable, label the result **feature-level comparison only**. Do not synthesize a numerical vendor baseline from brochures. Record which functions were available, which data could be exported, and which interventions were performed by vendor specialists.

Keep separate scoreboards for:
1. schedule quality (makespan, tardiness, on-time delivery, WIP, setup, utilization);
2. schedule stability (changed operations/machines, preserved commitments, repair time);
3. model correctness (obligation coverage, constraint omissions, objective-preserving mutation detection);
4. capability/evidence correctness (qualification currentness, identity, independence, material and inspection scope);
5. accepted production (first-article result, complete genealogy, time to first independently accepted output);
6. resilience (surviving independent qualified routes, common-cause miss, worst-case recovery);
7. human/economic burden (planner effort, review/escalation, engineering change effort, requalification labour, downtime).

**Our differentiation hypothesis:** retain the mature APS strengths of constraint modelling, finite-capacity planning and rapid rescheduling while adding an independently checkable layer that answers whether the model was complete, the route remained qualified, the evidence is admissible, and the resulting product was actually accepted. This has to be demonstrated on matched plant workflows; architecture diagrams or synthetic tests alone do not prove vendor superiority.

## 5. Standards and interoperability mapping

- **ISO 23247-5:2026** is published (June 2026) and describes the digital thread for connecting and maintaining manufacturing digital twins across the product lifecycle: https://www.iso.org/standard/87425.html
- **ISO 23247-6:2026** is published (July 2026) and addresses digital-twin composition, including integrated, unified and federated compositions and interoperation among separately developed twins: https://www.iso.org/standard/87426.html
- The NIST SI for Manufacturing programme is advancing VVUQ for manufacturing digital twins and agentic intelligence and working in relation to ISO 23247-8; confirm the status of any draft at the time of each release.

Map the architecture and test artefacts to the relevant standard clauses after acquiring and reviewing the normative texts. Do not claim ISO compliance, conformity, certification or accreditation on the basis of a high-level summary or an issue checklist. Standards references guide interface and assurance design; independently assessed conformity is a separate activity.

## 6. The actual scoreboard: independent acceptance under change

Maintain distinct scoreboards, with hard safety/quality gates outside any weighted optimization function:

| Scoreboard | Main outcome | External comparator |
|---|---|---|
| Static scheduling | bound gap, makespan, tardiness, proof rate | FJSPLib engines |
| Dynamic repair | changed-operation/machine ratio, runtime, schedule quality | SLR2 and document-driven masked scheduling |
| Decision-valued inspection | expected decision loss/regret, value of tests, false-preserve/false-release | fixed diagnostics, information gain, exact oracle |
| Capability qualification | invalid admissions, evidence freshness/applicability, disposition completeness | documented industrial quality/qualification workflow |
| Reconfiguration | time A-last-good to B-first-accepted, requalification scope, commitment preservation | MTP / matched current plant changeover workflow |
| Resilience | independent qualified route count, common-cause misses, worst-case accepted-output recovery | nominal redundancy and full reconfiguration |
| Context grounding | identity/version/time/unit/source grounding, negative transfer, authority errors | NIST context-ablation direction |
| Twin credibility | VVUQ coverage, validity domains, residual error and extrapolation control | NIST/ISO VVUQ direction |
| Resource productivity | energy, water, material, waste per accepted output | matched process/resource baseline |

Primary programme KPI: **time from disruption/configuration change to first independently accepted output**, accompanied by the full quality/qualification and evidence record.

## 7. Gate sequence for a defensible SOTA claim

1. Freeze input semantics, provenance, seeds and held-out split before tuning.
2. Reproduce the strongest public baselines or explicitly document unavailable components.
3. Run independent exact oracles on bounded instances and verify every emitted schedule/route certificate.
4. Match hardware, compute budgets, seed count and disturbance traces.
5. Report mean, median, dispersion, confidence intervals, per-instance wins/losses and failures; preserve raw results.
6. Separate model-selection and final held-out evaluation; prohibit tuning against held-out answers.
7. Report hard-gate violations as disqualifying outcomes, not a small penalty hidden by makespan gains.
8. Require third-party or independent owner review of acceptance/evidence receipts before physical claims.
9. Publish negative results, incompatibilities, incomplete cases, timeouts and re-run instructions.

A genuine win is not “better than the paper” because a different objective was optimized. It is either (a) a matched reproduction with a better result under the same evaluation contract, or (b) a clearly labeled expanded benchmark showing a new outcome the original benchmark did not measure.

## Claim boundary

This document captures published paper abstracts, vendor product descriptions, standards summaries, and NIST programme descriptions as of 2026-11-17. The FJSP-E2E issue assignment is currently future-dated relative to this review and is marked as an emerging watch item, not an established published comparator. It does not independently validate the papers' experiments, vendor claims, industrial standards conformity, or project performance. Any missing implementation, code, data, license, hardware or baseline configuration must be reported rather than imputed.
