# Maritime service-chain source intake

This directory is the source and rights register for MARITIME-SERVICE-CHAIN-001. It composes public, simulation and experimental reference data without pretending that all sources measure the same thing.

## Current status

The catalog contains two openly catalogued TUHH simulation datasets, one publicly described physical-model experiment whose item-level redistribution rights still require review, published optimization comparators, an industry event-exchange standard, and existing commercial / port-level performance comparators.

**No raw source files are mirrored by this commit.** The two TUHH datasets are catalogued as catalogued_not_downloaded; the Duisburg Test Case article is reference-only pending item-level rights review. No SHA-256 is invented for a source file that has not been downloaded and verified.

## Benchmark ladder

1. **Horizontal transport:** use TUHH LOGMS v2 to test quay-crane / terminal-truck allocation, productivity, waiting, utilization, and approximation quality. Preserve its simulation-only class.
2. **Closed queueing:** use the separate TUHH closed-queueing dataset to test crane productivity against vehicle fleet size and shared-queue assumptions. Keep its scenarios and schema separate from LOGMS; harmonization must be explicit and justified.
3. **Hydrodynamic validation:** use the Duisburg Test Case shallow-water model experiments only as a hydrodynamic validation reference for their specified hull, scale, and calm-water regimes. Rights must be reviewed before copying raw files.
4. **Integrated optimization:** reproduce matched-scope berth / quay-crane / yard, berth / quay-crane / AGV, and berth / yard / shore-power comparators before claiming a performance advantage.
5. **End-to-end service:** compose the best matched subsystem plans across feeder, berth, cranes, yard, AGV, rail, inland transfer, and receiver handoff. Measure accepted cargo delivered to its intended receiver within the declared service window.

## Ingestion invariants

Every downloaded source must have a source-specific record and immutable intake receipt containing:

- DOI or stable source identifier; canonical repository URL; version / revision; publication or issue date; retrieval timestamp.
- Item-level rights marker and its exact URI. A public-domain status mark is not to be rewritten as a CC BY licence; missing rights are REVIEW_REQUIRED.
- Original file name, byte length, media type, and SHA-256 digest of the exact downloaded bytes.
- Original README / schema / units / scenario identifiers retained alongside the source.
- Evidence class: SIMULATION, PHYSICAL_MODEL_EXPERIMENT, OBSERVATIONAL_OPERATIONAL, or LITERATURE_ONLY.
- Separate publication_time and valid_time fields wherever the dataset provides them; never infer one from the other.
- Raw source stored unchanged; normalized / harmonized output is a separate derived artifact with transformation code and deterministic replay.
- No pooling of independently defined scenario families unless an explicit mapping, unit check, missingness review, and negative-control test pass.

## Negative controls

At minimum, each benchmark release must include:

- A throughput-shift control where quay productivity improves but downstream cargo delivery remains unchanged or deteriorates. The end-to-end evaluator must not call this a service improvement.
- A duplicate-identity control where a shipment or container appears under multiple local aliases. The conserved cargo ledger must count physical service units once.
- A stale / revised source control where a corrected or out-of-date event cannot silently overwrite the evidence history.
- A licence-missing control where ingestion refuses to mirror or redistribute the source.

## Non-goals

This intake catalog does not create a terminal operating system, port-call standard, vessel hydrodynamics solver, independent physical oracle, operational authority, or real-port qualification. It is a provenance and benchmark-composition contract only.

## Expanded reproducible sub-benchmark families (24 November 2026)

The source register now includes the AI Port Center Port-Call Data Hub and selected linked benchmark families. This matters because the current public comparison landscape is not one dataset: it is a portfolio of narrow suites that test different components of the port-call process.

### Benchmark ladder additions

- **BAP:** MathProg OR-Lib's BAP set describes 900 dry-bulk instances (400 in CSV form), including arrival, processing, vessel length and tidal windows. It is useful for BAP solver regressions, not a direct container-terminal benchmark. Rights were not established in this intake, so it remains reference-only.
- **Integrated BAP + crane assignment:** DTU's 2025 BACAP with transshipment-handling dataset (DOI 10.11583/DTU.28870757) is a larger 213.43 MB, CC BY 4.0 benchmark. This is a stronger integrated scheduling comparison than separately running BAP and QCSP, but it remains an optimization instance suite.
- **Tandem quay cranes:** Mendeley data includes exact tandem-QC scheduling (10.17632/6d9stwzjhp.1) and joint slot-planning/truck-scheduling (10.17632/y2h864xdhr.2) research supplements, both marked CC BY 4.0.
- **Weekly berth/QC recovery:** the recoverable-robustness weekly berth and quay-crane data (10.17632/3vjsyyk3pm.1) is marked CC BY-NC 3.0. Its non-commercial terms must remain explicit; do not fold it into a commercial redistribution bundle.
- **Yard cranes:** a multi-crane scheduling set (10.17632/3nkk9dprpn.2) includes 3 cranes, 2 blocks and 6–12 tasks, CC BY 4.0. It is suitable for small exact-oracle and separation constraints, not full terminal throughput.
- **Terminal dwell process mining:** the Event Log Dwelling Time dataset (10.17632/yvp2b4rtp3.1) is marked CC BY 4.0 and describes three months of event logs from discharge/lift through gate exit. This is a potentially valuable observational sub-benchmark, but before use inspect site identity, data sensitivity, field semantics, missing/censored journeys, publication times, and whether the logged records are complete. It measures terminal dwell, not delivery to a receiver outside the terminal.

The AI Port Center Data Hub is a **directory**, not a single joined dataset or licence. Its port-call map identifies real process owners and operational source types at Rotterdam and points to BAP, QCSP, YCSP, truck appointment, tug/pilot and yard-vehicle routing benchmarks. Keep each linked dataset as an independent sources[] entry, including its own version, file digests, data class, schema and rights. The directory's external references do not grant rights to the underlying source files.

### Recommended first reproducible cross-family tournament

1. Establish independent tiny-case oracles for BAP, BACAP, tandem-QC, yard-crane, and truck/vehicle scheduling.
2. Run strongest matched methods within each subproblem: exact MILP/CP-SAT or enumeration where tractable; published heuristic or robust method reproduction; FCFS/greedy/dispatch baselines.
3. Preserve common frozen inputs and per-method time/memory budgets; report optimality gap and instance coverage rather than compare incompatible objective functions.
4. Compose outputs into one service-chain experiment only after the crosswalks for vessel visit, berth, crane task, container identity, custody and receiver acceptance are explicit.
5. Include a negative-control scenario where the quay-crane or truck-throughput metric improves while final receiver delivery does not.

**Evidence boundary:** no source data in this register has been downloaded by the current commit. Every item remains catalogued_not_downloaded or reference_only. No new result or benchmark performance claim is asserted. Raw file SHA-256 values must be computed only after the exact file bytes have been retrieved; licences and source identity are item-specific.


## Official Durban Gateway Terminal snapshots (24 November 2026)

A read-only source-discovery pass verified that the official [DGT Terminal Updates index](https://dgt.ictsi.com/terminalupdates) currently exposes distinct source artifacts for vessel updates, export stack dates, import storage, firm berth plans, customer notices and NAVIS/TOS communications. The 24 November page lists both AM and PM export stack snapshots, the 24 November import storage file, and a 24 November firm berth plan. These are separate source classes, not one generic event timeline.

Two source PDFs were inspected:
- The official vessel update carries an internal report time of **24 November 2026, 11H00** and distinguishes NOT YET IN, IN NOT WORKING, ON WAY IN and WORKING states; it includes vessel visit, berth, ETA, started-working, remaining discharge/load, completion and notes. [Official PDF](https://cdnweb-dgt.ictsi.com/users/user20/DGT%20VESSELS%20UPDATE%20SPREADSHEET%20%2853%29.pdf)
- The official firm berth plan carries an internal live-update timestamp of **24 November 2026, 11:48:45** and lays out berth/bollard occupancy and planned vessel windows through December. This remains a plan; it must not be promoted to observed alongside or actual berth occupancy. [Official PDF](https://cdnweb-dgt.ictsi.com/users/user20/DGT%20FIRM%20BERTH%20PLAN%2024.11.2026.pdf)

The linked AM/PM export-stack and import-storage XLSX files were discovered by URL but were **not downloaded** in this intake (the web viewer did not render XLSX). Their schema, rights and file digests are therefore still unverified. The source register marks them catalogued-not-downloaded and rights-review-required. No raw data or third-party-derived status has been promoted to an oracle receipt.

The third-party Dues DGT status page can be useful as a **diagnostic comparator** for source freshness, status normalization and discrepancies, but its own page says it republishes/refreshes terminal files. It is not an independent physical witness and cannot replace the DGT source artifact. [Dues DGT view](https://dues.co.za/ports/durban-gateway-terminal)

This evidence suggests a valuable first real-source engineering task: a strictly read-only, permission-aware, append-only artifact indexer that preserves source-native class, publication time, retrieval time, valid-time scope, revisions and raw digests. The benchmark should measure source revision linkage, false plan-to-actual promotion, as-of replay correctness and disagreements between primary and derived sources. It should not start by trying to schedule vessels or claim improved terminal performance.


## Competitive boundary and next-generation benchmark strategy (24 November 2026)

### Do not rebuild the industry's TOS

The public industry baseline is already substantial. Kaleris describes N4 as coordinating vessel, yard, rail, gate and control-room workflows, and its product portfolio includes RTG/RMG optimization, terminal truck dispatch/scheduling and yard intelligence. Those are credible incumbents to compare against, not features we should claim are absent from the market. Vendor capability pages are vendor-published claims, not independent performance evidence.

The DCSA Port Call 2.0.0 specification and conformance framework also establish an existing interoperability contract. Its publisher/subscriber scenarios test GET/POST event exchange, timestamps, move forecasts and schema/business-field validations. Our interface should reuse that contract and conformance tooling rather than invent a new port-call event API. DCSA conformance proves the tested message contract, not that the source sensor captured every physical event or that a delivered container reached its receiver.

The defensible differentiation is an **independent evidence-and-decision assurance layer** that can sit across existing TOSs, ports, carriers, terminals, rail, inland operators and receivers. It should qualify source claims, preserve as-of history, reconcile identities and custody, expose dependence and missingness, run independently checked counterfactuals, and keep unsafe/unauthorized actions blocked. It must not need to own dispatch or replace the TOS.

### Do not confuse the CPPI with terminal or receiver outcomes

The June 2026 World Bank/S&P Global CPPI methodology is a strong external comparator, but it reports at **port level**, not individual terminal level. It uses six vessel timestamps: arrival at anchorage/pilot station, movement to berth, start of cargo operations, end of cargo operations, departure from berth and exit from port limits. The interval between berth departure and port-limit exit is excluded from its vessel-time metric.

Therefore report a CPPI-adjacent port-call result for comparability, but measure DGT or another individual terminal separately, and measure terminal-to-receiver delivery separately again. A better port-level vessel-time score cannot compensate for missed receiver deadlines, unverified custody, service inequity or hidden cargo inventory loss.

### Public benchmark ladder to target

| Layer | Benchmark / comparator | Proposed acceptance target |
|---|---|---|
| API interoperability | DCSA Port Call 2.0.0 official conformance scenarios | 100% pass of relevant publisher/subscriber GET/POST, timestamp and move-forecast scenarios; no claims beyond the tested API contract |
| Source semantics | New source-claim admission CI fixture | Zero plan/status/stack/storage/DCSA/third-party promotions into physical movement or receiver acceptance without the required distinct evidence class |
| Historical replay | Immutable raw artifact ledger and as-of reconstruction | 100% byte-digest replay; zero use of artifacts published or retrieved after cutoff; every correction retained as a new revision |
| Identity and custody | Existing container identity, capture-completeness and paired handoff owners | Zero unresolved identity/custody joins presented as complete deliveries; unknown remains unknown |
| Port-call service | CPPI-adjacent six-timestamp methodology | Like-for-like stratification by vessel size/call size; report port and terminal scopes separately |
| Container dwell | Current Mendeley Event Log Dwelling Time Dataset v2 | Reproduce published process-mining baseline; explicitly report missing/censored traces and stop metric at terminal gate exit |
| Optimization | MathProg BAP, DTU BACAP, tandem-QC and yard-crane public suites | Exact agreement on all tractable tiny cases; matched compute budget and optimality gap on larger cases; include simple FCFS/greedy and best published baselines |
| Decision-risk | #7353/#7359 repeated-sampling and dependence-aware tournaments; #7360 cluster-aware oracle adapter | Expected-risk and high-probability claims remain distinct; calibration uses justified independent units; report abstention and confidence bounds |
| Physical/authority safety | #7231 exact schedule/authority oracle and typed receipt projection | Zero false-safe and unauthorized-action claims on the frozen synthetic oracle set; unknown physical or authority outcome blocks composition |
| End-to-end outcome | #3683 coastal-to-inland / terminal-to-receiver composition | Measure receiver-accepted on-time deliveries, tail dwell, total cost/energy/emissions, rehandling, cargo loss, participant service floors and resilience—not throughput alone |

All numeric acceptance goals in this table are proposed qualification targets, not achieved results.

### The paradigm shift: optimize the evidence path as well as the logistics path

The conventional optimization stack is often evaluated on a clean state estimate and its own objective function. We should add an *evidence path* to the comparison:

1. **Observe:** source artifacts carry native owner, semantic class, raw digest, publication time, retrieval time and valid-time scope.
2. **Reconcile:** identities, corrections, duplicate observations, censored/missing events and common-cause clusters are resolved without silently manufacturing truth.
3. **Qualify:** an independent oracle assesses physical feasibility, authority, participant/service floors and terminal-to-receiver custody.
4. **Counterfactually decide:** the TOS/solver alternatives are replayed against frozen inputs and policy revisions, with uncertainty and dependence preserved.
5. **Execute only within authority:** the decision layer may recommend, constrain or abstain; operational execution remains with the authorized TOS/port/participant.
6. **Close the loop:** independent observations and receiver acceptance determine realized outcomes; plan, action, reported status and physical realization stay distinct.

This is a stronger and more defensible product proposition than "a better port optimizer": it tests whether an improvement is real, attributable, safe, fair and preserved through final delivery.

### Benchmark datasets and rights corrections

The Mendeley Event Log Dwelling Time Dataset has a current **version 2** record (DOI 10.17632/yvp2b4rtp3.2, published 26 June 2024, CC BY 4.0). It measures container discharge/lift until exit through the terminal main gate, not receiver delivery. Its companion Data in Brief article describes multi-perspective process-mining fields and anonymized data. Do not compare a model trained on this event log directly with current DGT operations without a rights-cleared, regime-aware external validation design.

The AI Port Center Port-Call Data Hub remains a directory and research-problem map for Rotterdam, not one common dataset or blanket licence. Keep every underlying dataset at its own version, provenance, schema and licence. In particular, retain non-commercial limits on the weekly recoverable-robustness dataset.

### Immediate execution order

- First, qualify the DGT artifact semantics and immutable as-of replay before parsing/exporting any operational values.
- Second, retrieve the CC BY 4.0 dwell-time v2 artifact, inspect the exact files and reproduce event-log process discovery/conformance diagnostics; preserve its 2021 source regime.
- Third, implement DCSA 2.0.0 publisher/subscriber conformance without replacing source truth or authority.
- Fourth, add exact small-instance oracles and matched solver baselines across BAP/BACAP/QCSP/YCSP/vehicle routing.
- Fifth, link operational schedules to independently adjudicated cargo/receiver outcomes and then evaluate the full service-chain objective vector.

Sources:
- [DCSA Port Call 2.0.0 conformance scenarios](https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-conformance-scenarios)
- [DCSA Port Call 2.0 implementation guide](https://reference.dcsa.org/content/standards/releases/port-call/v2/pc-v2-implementation-guide)
- [Kaleris N4 TOS](https://kaleris.com/terminal-operating-system/kaleris-n4-tos/)
- [Kaleris terminal optimization portfolio](https://kaleris.com/solutions/terminal-operating-system/)
- [World Bank/S&P Global CPPI methodology, 10 June 2026](https://thedocs.worldbank.org/en/doc/aac122f6df85534428d66a7b9af4b7f6-0400012026/original/CPPI-Methodology-Note.pdf)
- [Mendeley Event Log Dwelling Time Dataset v2](https://data.mendeley.com/datasets/yvp2b4rtp3/2)
- [Data in Brief dataset paper](https://doi.org/10.1016/j.dib.2024.110906)


## Standards-aligned event, correction and regulatory exchange architecture (24 November 2026)

A robust maritime evidence layer should not define a competing global event ontology. Use the standards at their proper layers:

| Standard / system | What it standardizes | What it does **not** prove |
|---|---|---|
| DCSA Port Call 2.0.0 | Port-call event exchange and publisher/subscriber API conformance | That the physical event occurred or every event was captured |
| DCSA Track & Trace 2.2.0 | Container/shipment visibility event definitions and API mapping | Complete physical custody, source independence or receiver acceptance |
| GS1 EPCIS 2.0.1 | Visibility-event data model: subject/object IDs, eventTime, timezone, recordTime, readPoint, bizLocation, business step/disposition, sensor/source/destination context | That a sensor or source actually captured every event or that a business claim is independently true |
| GS1 CBV 2.0.0 | Standard business-step, disposition and event vocabulary alongside EPCIS | Physical proof or authority |
| IMO FAL Maritime Single Window | Regulatory declarations and one-time electronic exchange for ship arrival, stay and departure across public authorities | Terminal operational event capture, cargo movement, custody or receiver delivery |
| South African SARS Customs Single Window | Phased national customs / multi-agency information exchange, including inspection, permits and declarations | A complete maritime terminal event ledger or physical cargo outcome |

GS1 EPCIS is especially relevant because it already separates the core evidence dimensions: **what** object/subject, **when** eventTime plus timezone and repository recordTime, **where** readPoint and business location, and **why** business step/disposition/transaction context. Its ErrorDeclaration semantics are also important: a later declaration says that a prior event's assertions were erroneous as of declarationTime; the prior event is not modified and subsequent queries return both original and declaration. Use this to represent corrections instead of overwriting earlier event rows.

The new append-only EPCIS correction fixture in [draft PR #7389](https://github.com/Luminous-Dynamics/symtropy/pull/7389) implements this narrow rule: the ErrorDeclaration shares the prior eventID and is bound to the same subject IDs, eventTime, timezone and assertion-payload digest; correctiveEventIDs remain references until the corresponding corrected events have been recorded. The test preserves the original event, time-of-knowledge and correction availability separately. This is not a full EPCIS implementation or proof that the event source is truthful.

### South African interoperability opportunity

The IMO FAL amendments requiring a Maritime Single Window for public authorities entered into force on **1 January 2024**. IMO reported that a Durban workshop on 10–12 September 2024 brought together the Department of Transport, TNPA, SAMSA, SARS, BMA and shipping stakeholders to plan national implementation. SARS publishes a phased Customs Single Window programme; a Freight News report on 8 September 2026 stated that no overall completion date had yet been supplied. Treat that as a time-bounded secondary status report and re-check official government updates before making a current implementation claim.

This creates a valuable local integration requirement:
- Port-call and ship-clearance declarations belong to the authority/Maritime Single Window layer.
- DCSA Port Call and T&T carry standardized operational shipping events.
- EPCIS/CBV can represent item/container-level visibility and event corrections.
- TOS/terminal sources report operational statuses, plans, movements and inventory.
- An independent receiver record closes physical custody and delivery.
- Mycelix/authority systems must preserve who may submit, read, correct or act on each record, including purpose and participant permission.

Keep these systems federated. A successful API POST, FAL declaration, customs release, TOS status or EPCIS capture operation is not interchangeable with proof that a container physically moved or reached a consignee.

### Proposed conformance campaign

1. **DCSA Port Call 2.0.0**: run the official publisher/subscriber conformance scenarios and pin the schema/test version.
2. **DCSA Track & Trace 2.2.0**: run official API reference/conformance scenarios and map the event families into the source-claim adapter.
3. **GS1 EPCIS 2.0.1 / CBV 2.0.0**: run JSON/JSON-LD schema and semantic validation, including eventTime, eventTimeZoneOffset, repository recordTime, EPC/eventID, readPoint/bizLocation, bizStep/disposition, source/destination, and ErrorDeclaration/correctiveEventIDs.
4. **FAL / national single window**: only test authorized sandbox or partner interfaces; keep customs/ship-clearance decisions and public-authority records under the relevant government authority. Do not simulate regulatory permission by merely passing a JSON schema.
5. **Cross-standard bridge**: preserve the original native message, standard version, mapping revision, source record ID, canonical subject, publication/retrieval/valid/event/record times, rights and claim ceiling. Mapping must be reversible or explicitly lossy.
6. **Physical/receiver closeout**: link the event chain to qualified independent observations, custody handoffs and receiver acceptance. Unresolved identity, source coverage, custody or authority remains UNKNOWN/NOT_EVALUABLE.

Proposed acceptance targets (not achieved): 100% applicable standard conformance scenarios; 100% replay of raw message digests; zero lossy cross-standard mappings silently marked reversible; zero event corrections that delete the original; zero planned/status/declaration records promoted to physical events; zero custody closure without an independently qualified receiver record.

Sources:
- [GS1 EPCIS 2.0.1](https://ref.gs1.org/standards/epcis/2.0.0/)
- [GS1 CBV 2.0.0](https://ref.gs1.org/standards/cbv/2.0.0/)
- [DCSA Track & Trace implementation guide](https://reference.dcsa.org/content/standards/guidelines/implementation-guides/implementing-track-and-trace)
- [IMO FAL Convention](https://www.imo.org/en/about/conventions/pages/convention-on-facilitation-of-international-maritime-traffic-(fal).aspx)
- [IMO report on South Africa Maritime Single Window planning, 17 September 2024](https://www.imo.org/en/mediacentre/pages/whatsnew-2137.aspx)
- [SARS Customs Single Window](https://www.sars.gov.za/customs-and-excise/customs-single-window-solution/)
- [Freight News report, 8 September 2026](https://www.freightnews.co.za/article/no-completion-date-for-sas-single-window)


### DCSA milestone maturity adapter

The new DCSA maturity adapter preserves the shared classifier vocabulary as four distinct values:
- EST = estimated;
- REQ = requested;
- PLN = planned;
- ACT = source-reported actual.

The Information Model's shared classifier entity lists all four, but field-level event schemas can be narrower: the Shipment Event field lists PLN/ACT/EST, while the Operations Event field lists PLN/REQ/ACT/EST. Therefore this adapter is a classifier-preservation fixture, not the event-family-specific T&T schema validator. A live adapter must first validate the exact event-family OpenAPI schema and pinned release. DCSA T&T 3.0.0 was announced in September 2026 but remains restricted to members/DCSA+ partners until its planned public release in H2 2027; do not claim public 3.0.0 conformance from this 2.2.0 adapter.

The adapter does not promote ACT to an independent physical witness. The exact source record, canonical subject, standard version, event time/timezone, record time, publication and retrieval times all remain attached. The receipt fails closed on unknown classifiers, version drift, missing event time for ACT, unknown publication, or an as-of cutoff that predates publication/retrieval. Estimated/requested/planned future event times remain valid forecasts/plans and are not treated as events already realized.

This is particularly important for ETA, pilot, berth and cargo milestones: ACT is the publisher's maturity classification, not independent verification that an event physically occurred. The claim still requires separate capture completeness and physical/receiver evidence.


### Cross-standard mapping and semantic loss

DCSA and GS1 EPCIS/CBV are complementary standards, not a common vocabulary with one-to-one renames. The crosswalk contract requires source release, DCSA event family, target EPCIS release, target CBV vocabulary release, an explicit field/value rule, mapping-bundle identity and a declared review scope:
- **EXACT** mappings preserve the literal value and are reversible;
- **CONTEXTUAL** and **LOSSY** mappings are explicitly irreversible and explain the semantic gap.
- A rule scoped to EquipmentEvent cannot be reused against ShipmentEvent merely because a field name matches.
- The target profile separately pins EPCIS 2.0.1 and CBV 2.0.0.

A hard negative control prevents DCSA `eventClassifierCode` values (EST/REQ/PLN/ACT maturity) from mapping directly to EPCIS `bizStep` or `disposition`. A DCSA `equipmentEventTypeCode=LOAD` to CBV `loading` crosswalk is only CONTEXTUAL / IRREVERSIBLE; it cannot imply that the source message proves physical loading. The target validator covers a pinned logistics subset of CBV 2.0.0 terms and URI forms, not the complete vocabulary or EPCIS JSON-LD serializer.

The executable contract is in draft [PR #7391](https://github.com/Luminous-Dynamics/symtropy/pull/7391). It carries the source record/event/subject, source-payload SHA-256 and source-store reference, as well as mapping-bundle revision/digest/ref and target vocabulary version. The checked-in 1.0.0 bundle is marked SYNTHETIC_FIXTURE_ONLY and its exact bytes are verified by SHA-256 in CI. Such a rule is rejected on any source message marked non-synthetic. EXTERNALLY_APPROVED rules require an approval-record reference, but this mapper only verifies that the reference is present; governance must resolve and verify it independently. The synthetic receipt has 25 known-answer cases (26 lines including header, 13 columns). Proposed acceptance: 100% deterministic known-answer mapping for pinned rules; zero unreviewed or fixture-only rules used on real sources; zero non-exact mappings labelled reversible; zero classifier-to-business-step/disposition promotion; 100% source digest/reference and mapping-bundle manifest verification.


## Terminal dwell process-mining benchmark: version and baseline discipline

The current Mendeley record is **Event Log Dwelling Time Dataset v2** (DOI 10.17632/yvp2b4rtp3.2, 26 June 2024, CC BY 4.0). Its page describes container dwell as discharge/lift from ship until exit through the terminal main gate. The companion Data in Brief article (DOI 10.1016/j.dib.2024.110906) links the earlier **version 1** DOI (10.17632/yvp2b4rtp3.1) and describes July–September 2021 event logs, anonymization and multi-perspective attributes including worker roles and costs. The exact v2 file manifest was not retrieved in this pass; do not assume v2 is byte/schema-identical to the article's v1. Before reproducing the paper, fetch the exact v2 manifest/files through an authorized public API session, record each filename/size/hash and compare schema, event counts, case/trace coverage, labels and licence metadata with v1/paper descriptions.

### A current sound-discovery comparator: OptIMIIst (2026)

The 25 June 2026 *Process Science* paper "Locally optimized inductive process discovery" introduces OptIMIIst, an Inductive Miner extension that uses local ILP decisions to preserve soundness while handling infrequent/incomplete behaviour with less heuristic filtering. Its reported Process Discovery Contest 2024 score is **88.24%**; the paper reports Pyrrhus at 90.9% and a prior Brons et al. result of 89.46% but notes the latter was not independently reproduced because its implementation was unavailable. In the authors' separate comparison across real-life logs, OptIMIIst is compared with IMf, AIM, ILP-Miner, Split Miner and Heuristics Miner. The reported evaluation is not a blanket leaderboard: some competitors trade away soundness or activity coverage, and OptIMIIst itself reports weaknesses for duplicate activity labels and long-term dependencies.

This means our benchmark should use OptIMIIst as a major comparator, but not claim SOTA merely by beating default Inductive Miner. We should:
- reproduce the PDC24 reference protocol and published score before adapting it to the maritime dwell log;
- run OptIMIIst and matched sound baselines on the same pinned v2 log, identical activity abstraction, train/test splits, compute/time limits and solver/version configuration;
- include duplicate labels, long-term dependencies, incomplete/out-of-order traces and infrequent branches as separate challenge cohorts;
- report fitness, precision, generalization, model size, soundness, activity coverage, runtime/memory and stability—not only a composite score;
- preserve the process-tree representational limitation; compare against non-tree models where long-range dependencies or duplicate activity labels make the bias material.

Paper: https://doi.org/10.1007/s44311-026-00051-x
Open implementation: https://github.com/Nik314/OptIMIIstJournal
PDC24 dataset: https://doi.org/10.4121/3cfcdbb7-c909-4f60-8bec-62c780598047

### Required discovery and conformance baselines

For each frozen version, compare:
- Directly-Follows Graph (DFG) as a transparent, low-complexity operational baseline.
- Inductive Miner / infrequent Inductive Miner (sound, block-structured baseline).
- Split Miner (fitness/precision and complexity trade-off).
- Heuristics Miner / Flexible Heuristics Miner (noise and frequency baseline).
- ILP Miner / exact small-log oracle where tractable.
- Current research comparator: OptIMIIst (2026), which extends Inductive Miner with ILP-based local decisions and evaluates against IMf, AIM, ILP-Miner, Split Miner and Heuristics Miner on Process Discovery Contest synthetic logs. Reproduce its published protocol before claiming an improvement.

Report more than a single F-score. Use held-out alignment-based fitness and precision, generalization, model simplicity/size, soundness/deadlock freedom where applicable, runtime, memory, activity coverage, and stability under repeated seeds. Use fixed input logs, identical activity abstraction, same train/test cases, same compute budget and pinned implementation versions. Keep Pareto frontiers; do not hide low precision behind a high-fitness spaghetti model or high precision behind a model that rejects ordinary behavior. Use a chronological holdout in addition to grouped case holdout to expose process drift and leakage.

### Maritime-specific falsification

1. **Censored cargo:** cases that have not exited the gate by the dataset cutoff remain right-censored/ongoing, not silently deleted or assigned a zero dwell time.
2. **Missing capture:** preserve unexplained gaps and compare the model's output against capture-completeness diagnostics; an event log of recorded messages is not proof that every physical event was logged.
3. **Duplicate/reordered events:** adversarially inject duplicate events, late publication, corrected timestamps and out-of-order recordTime; test whether replay preserves both event and knowledge time.
4. **Route/clearance heterogeneity:** retain the source's separate route/activity/worker/cost dimensions where rights and anonymization permit; do not merge process branches into one generic "customs" step unless a versioned abstraction says so.
5. **Operational-regime drift:** isolate time periods, terminal/TOS/operator changes and source-schema revisions; test chronological conformance decay rather than training and testing on randomly interleaved events only.
6. **Receiver boundary:** stop the dwell-time metric at terminal main-gate exit. It says nothing about inland transport, custody handoff or receiver acceptance.

### Proposed success criteria (targets, not achieved)

- 100% raw v2 file digest and manifest reproducibility for the retrieved corpus.
- 100% exact agreement on deterministic synthetic event-order/correction/censoring fixtures.
- 0 dropped censored cases without a declared censoring rule.
- 0 future-published/retrieved events in historical as-of replay.
- 0 planned/source-reported events promoted to physical outcomes.
- Discovery model must improve the held-out fitness/precision/complexity Pareto frontier over the strongest reproducible baseline at the same compute budget, with no loss of soundness where claimed.
- Dwell-time evaluation reports median, p90/p95, tail/conditional summaries and censoring-aware estimates separately from throughput and schedule KPIs.

References:
- [Mendeley Event Log Dwelling Time Dataset v2](https://data.mendeley.com/datasets/yvp2b4rtp3/2)
- [Data in Brief article and v1 dataset description](https://doi.org/10.1016/j.dib.2024.110906)
- [OptIMIIst: Locally optimized inductive process discovery (2026)](https://link.springer.com/article/10.1007/s44311-026-00051-x)
- [On Process Discovery Experimentation (ACM TOSEM, 2024)](https://doi.org/10.1145/3672447)
- [South African berth allocation and quay-crane scheduling review (2025)](https://doi.org/10.3390/jmse13071339)


## Current standard releases and a deliberate migration boundary (25 November 2026)

The public interoperability baseline has two distinct version states that should not be conflated:

- DCSA Track & Trace 2.2.0 remains the public implementation/conformance baseline used here. DCSA announced Track & Trace 3.0.0 on 30 September 2026 with IoT/reefer events and enhanced correction/withdrawal semantics, but says the new release is initially available to DCSA members and DCSA+ partners, with general public release planned for the second half of 2027. Until the exact 3.0.0 event-family schemas and conformance artefacts are publicly available or legitimately obtained by an authorized partner, do not claim a public 3.0.0 implementation or conformance result. Keep the adapter versioned and stage a separate migration campaign rather than silently changing 2.2.0 mappings.
- GS1's current artefact page exposes EPCIS 2.0.1 with CBV 2.0.0, plus the JSON-LD context, OpenAPI, JSON Schema, SHACL and XML schemas. The crosswalk pins EPCIS and CBV separately; a target field with CBV semantics must also pin CBV version, not just EPCIS version.

DCSA T&T 3.0.0 is a forward-looking challenge set: add IoT/reefer data, measurement provenance, calibration, device identity, custody, corrections/withdrawals and sensor common-cause failure to the evidence layer once its normative schemas can be validated. Do not implement from a press release alone.

## Paradigm shift: optimize a service-chain outcome with an evidence constraint

The conventional port optimizer solves a scheduling objective assuming its input state is correct. A more defensible system solves two linked problems:

1. **Operations problem:** produce a feasible berth/quay-crane/yard/gate/inland plan under time, capacity, energy and resource constraints.
2. **Evidence problem:** qualify whether the state and claimed result are supported by timely, versioned, source-attributed, correction-aware, identity-consistent and sufficiently complete evidence.

A plan is only eligible for operational recommendation when the relevant physical/authority oracles can establish feasibility. An observed result is only scored as achieved service when the event/custody chain closes to an independently qualified receiver record. The assurance layer does not replace TOS dispatch; it compares, constrains, or abstains on top of multiple systems.

### Industry and SOTA comparators

| Comparator | What it already addresses | What Symtropy should measure beyond it |
|---|---|---|
| DCSA Port Call / Track & Trace | Standardized port-call and container event exchange, schemas, event definitions and conformance | Source-family schema validity plus capture completeness, bitemporal as-of replay, semantic claim ceilings, correction history and independent physical/receiver qualification |
| GS1 EPCIS / CBV | Interoperable item/event visibility, business-step and disposition vocabularies, event corrections | Reversible/contextual/lossy mapping with explicit version and bundle provenance; no status-to-physical-truth promotion |
| Kaleris N4 / incumbent TOS suites | Vessel, yard, rail, gate and control-room operating workflows and optimization products | Do not claim a TOS replacement; compare on independently adjudicated outcomes, safe abstention, source traceability and inter-system auditability |
| Integrated continuous-time BAP + time-variant QC + yard assignment research (2025) | Coupled berth, quay-crane and yard-space decisions with exact solution methods on tractable instances | Matched benchmark quality plus source uncertainty, capture loss, schedule perturbations, energy/emissions, receiver deadlines and evidence-qualified execution |
| Robust integrated BQCYAP research (2026) | Two-stage robust integrated berth/quay-crane/yard planning, exact column-and-constraint generation on smaller cases and heuristics for large cases | Compare robust baselines under identical uncertainty sets and compute budgets; add evidence missingness/correction, common-cause source dependence, service floors and receiver outcomes |
| Integrated berth/QC/shore-power/energy-dispatch research (2026) | Coupled berth, crane, yard and power/energy decisions with carbon accounting | Multi-objective Pareto fronts for turnaround, energy, emissions, cost, service distribution and independently verified end delivery |

Do not claim to beat these papers by presenting a synthetic toy fixture. Reproduce their public instances/protocols where rights allow; freeze code, objective, feasibility checker, input versions, compute budget and stopping rule. Report exact optimality gaps on small instances, anytime primal/dual bounds on larger instances, and a Pareto frontier on multi-objective cases. Compare with FCFS, greedy, deterministic MILP, robust/distributionally robust, ALNS, RL+ALNS and the strongest reproducible published exact/heuristic baseline.

### New benchmark suite and acceptance targets

All targets below are proposed, not achieved.

| Benchmark dimension | Metric | Initial target |
|---|---|---|
| Standard conformance | Relevant DCSA/EPCIS/CBV normative schema + conformance test pass rate | 100% of the pinned applicable suite; version/family explicitly recorded |
| Semantic preservation | Exact mappings preserve source value; non-exact mappings state irreversible loss | 100% known-answer correctness; zero invalid source-family or vocabulary acceptance |
| Evidence integrity | Source raw-payload digest/reference and mapping-bundle manifest hashes | 100% replayable and verified against exact bytes in CI |
| Synthetic/production separation | Fixture-only rule attempts on non-synthetic input | Zero accepted |
| Correction history | Original event, declaration/correction and repository knowledge time | Zero destructive rewrites; deterministic as-of replay |
| Physical truth | Plan/status/API/ACT source claims promoted to physical proof without independent witness | Zero false promotions |
| Capture completeness | False-complete rate under omissions, duplicates, clock drift and common-cause failures | Zero false-complete in frozen adversarial controls; field confidence bounds only after representative independent episodes |
| Logistics quality | Berth turnaround, QC/yard productivity, gate queue, inland reliability | Match published objective definitions; exact optimum on small instances; report median/p90/p95 and tail outcomes |
| Service-chain outcome | Receiver-accepted on-time delivery, end-to-end delay, damage/loss, energy, carbon and total cost | Improve the held-out Pareto frontier without violating safety/authority or participant service floors |
| Risk assurance | False-safe rate, coverage-risk curve, abstention, calibration by independent cluster | Zero false-safe in frozen oracle controls; cluster-aware field estimates with effective sample size and confidence intervals |

### Falsification protocol

The benchmark should actively try to make the system fail:
- Reuse an EquipmentEvent mapping against a ShipmentEvent with the same property name.
- Replay old source data after later publication, schema correction or event withdrawal; historical decisions must not use future knowledge.
- Present an ACT message with no event time, contradictory timestamp, or missing source payload bytes.
- Remove an EPCIS context mapping, change the CBV version, or supply a valid-looking but wrong vocabulary URI.
- Replay a synthetic fixture crosswalk against a record marked non-synthetic.
- Duplicate sensors under one common power/network/vendor dependency and test whether nominal witness count is mistaken for independent replication.
- Inject out-of-order and duplicate cargo moves; require the original event plus error/corrective records to remain queryable.
- Stress vessel bunching, crane outages, yard block congestion, customs holds, reefer alarms and inland rail/truck disruptions at once.

The decisive advantage is not that a planner returns a schedule faster. It is that the system can say what it knows, what it does not know, which source rule produced a claim, how later corrections change the view, whether the action is authorized, and whether the receiver actually got the goods—then compare its operational decisions fairly against existing solvers.

References:
- [DCSA Track & Trace 3.0.0 announcement, 30 September 2026](https://dcsa.org/newsroom/track-and-trace-3-0-0-iot-reefer-visibility)
- [DCSA Track & Trace standard](https://dcsa.org/standards/track-and-trace)
- [GS1 EPCIS / CBV 2.0.1 artefacts](https://ref.gs1.org/standards/epcis/artefacts)
- [Integrated continuous-time berth allocation, time-variant quay crane and yard assignment (Transportation Research Part B, 2025)](https://doi.org/10.1016/j.trb.2025.103317)
- [Robust integrated berth, quay-crane and yard assignment (Computers & Industrial Engineering, 2026)](https://doi.org/10.1016/j.cie.2026.112092)
- [Integrated berth allocation, quay-crane and onshore power/energy dispatch (Journal of Cleaner Production, 2026)](https://doi.org/10.1016/j.jclepro.2026.149105)
