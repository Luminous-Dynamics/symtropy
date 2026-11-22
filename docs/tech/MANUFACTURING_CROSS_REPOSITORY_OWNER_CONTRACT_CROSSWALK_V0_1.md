# Manufacturing Cross-Repository Owner and Contract Crosswalk v0.1

**Status:** source audit and integration plan, checked 22 November 2026.
**Owner:** [MACHINE-CAPABILITY-COMPILER-001 #6786](https://github.com/Luminous-Dynamics/symtropy/issues/6786)
**Parent:** [PRODUCTIVE-MACHINE-PLATFORM-001 #6762](https://github.com/Luminous-Dynamics/symtropy/issues/6762)
**Purpose:** prevent duplicate manufacturing authorities and make exact inter-repository adapters possible.

## 1. Core architectural decision

Luminous should operate one manufacturing evidence chain across distinct owner domains, not one universal manufacturing ontology or one universal receipt.

The canonical boundary is:

`Product intent / part revision → process meaning → resource offer and route → exact machine/session authority → execution → independent measurement and acceptance → product/unit qualification → custody and lifecycle → spatial/service projection`

The layers are deliberately non-equivalent. A declared capability is not qualified capability; an offer is not availability; a work order is not an authorized machine job; a validated toolpath is not a physical print; a completed operation is not an accepted part; a serialized unit is not a qualification receipt; a map location is not local production capacity.

### Owner map

| Concern | Canonical owner | What it owns | What it does not prove |
|---|---|---|---|
| Product/engineering intent, process meaning and process capability requirements | Symthaea MFG-PROC-000 #5686 | Process identity/family, transformation semantics, capability requirements, recipes, plans, model/qualification/discrepancy semantics | Physical machine operation or commercial resource scheduling |
| Specific Luminous FDM reference machine | Symthaea LD-PRINT-PILOT-001 #12533 | LD-Print v0 design, build dossier, staged D0–D6 physical qualification | A qualified household production route for every part |
| FDM quality and spatial measurement | Symthaea #7960/#7961/#7998 | Benchmark protocol/evaluator, artifact and observation binding, spatial build-volume capability | Machine design authority, live control, automatic quality compensation |
| Matched production-value benchmark | Symthaea MFG-VALUE-001 #7608 | Fair conventional-vs-candidate production outcome comparison, metrics/ablations/guardrails | Generic process registry or physical safety authority |
| Product-to-factory vertical integration | Symthaea HABITAT-VERTICAL-001 #8051 | Exact user/product requirements through upstream supply, conversion, workcells, quality, delivery, repair and recovery | A second MRP, process ontology, factory scheduler, inventory ledger or safety authority |
| Reconfigurable capability-network architecture | Symthaea MULTIPURPOSE-FACTORY-001 #12355 | Capability graph of process modules, machines, skills, utilities, material flows, partner plants and qualified reconfiguration | One universal factory design or replacement for specialized equipment/process owners |
| Apparel microfactory reference architecture | Symthaea ATELIER-MICROFACTORY-001 #12288 | Apparel-specific product mix, workcells, skills, material flow, facilities and distributed production | Generic process/capability semantics or cross-industry scheduler |
| Matched microfactory layout/flow/resilience benchmark | Symthaea ATELIER-FACTORYBENCH-001 #12289 | Independent layout/capacity/flow and disruption benchmark profiles for the Atelier use case | Generic factory benchmark authority for every product family |
| Accepted conforming-output economics | Symtropy FACTORY-ACCEPTED-OUTPUT-ECONOMICS-001 #5966 | Cost per released conforming pump/valve output using a matched task and fair route/baseline comparison | Universal production-value benchmark or process qualification |
| Material-accounted accepted-output routes | Symtropy MATERIAL-ACCOUNTED-ROUTES-001 #5875 | Mass/grade/utility/accepted-output accounting across matched factory routes | Canonical material-property, unit, assay or lifecycle authority |
| Common-cause production-path resilience | Symtropy FACTORY-CAPABILITY-RESILIENCE-001 #5704 | Qualified route diversity after common dependency/utility/material/authority disruptions | Nominal machine redundancy or proof of an untested alternate route |
| Intended/recorded/observed execution reconciliation | Symtropy FACTORY-EXECUTION-REALITY-001 #5758 | Discrepancy between plan, MES records, sensor observations and evaluator-hidden physical event traces | Physical observation coverage completeness or output acceptance by itself |
| NIST manufacturing traceability profile | Mycelix NIST-IR-8536-PROFILE-001 #6098 | Interoperability profile mapping NIST IR 8536 patterns to Mycelix identity, selective disclosure, custody, provenance and existing event owners | NIST certification, replacement event ledger, proof a physical event occurred merely because it is signed |
| City service-to-product coverage | Sol-Atlas CITY-PRODUCT-COVERAGE-001 #602 | Essential services mapped to buildable/maintainable product families and owner-native evidence gaps | Local manufacturing supply, provider capacity or permission to manufacture |
| Governed printer observation/session/execution boundary | Symthaea INTX-FAB-ADAPTER-001 #7706 and fabrication kernel | MachineProfile, negotiated capabilities/session, validated G-code, timed session, operator and execution guard path | Live OctoPrint/Moonraker transport where backends remain unavailable; no automatic physical control |
| FPM process monitoring and anomaly evidence | Symthaea FPM baseline correction #12531; Mycelix FPM #4281/#4282/#4283 | Observation, detector inference, proposed/actual intervention and quality-assessment layers | Detector score is not proof of a physical defect; no-alarm is not proof of an accepted part |
| Execution / process-run evidence | Mycelix MFG-001 #4685 and strict process-run receipt PR #5228 | Run identity, source/build/config/input/report digests and evidence-origin distinction | A structurally valid run receipt is not calibrated measurement, product acceptance or qualification |
| Productive machine tools, tooling, workholding and capability renewal | Symthaea PROD-EQP-000 #5964 and CIV-BOOT #5774/#5781/#5782 | Equipment specialization and general AND/OR closure / multi-generation capability-envelope semantics | A machine's local reproducibility without source, tooling and metrology evidence |
| Distributed process registry, machine/resource offers, routings and MRP | Mycelix MFG-PROC-001 #3085 | Parent ownership and migration governance | Process physics or machine execution authority |
| Typed process/capability refs and migration-safe bindings | Mycelix MFG-PROC-001A #6531 / draft [PR #6532](https://github.com/Luminous-Dynamics/mycelix/pull/6532) | Exact pinned pointers to Symthaea process definitions, capability requirements/profiles and recipes; optional typed operation/routing/machine refs | Exact-head focused formatting, Rust tests, 16-case migration corpus, strict Clippy and schema checks passed; PR remains unmerged and owner-object resolution/qualification is absent |
| Provider/site/resource offer and time-bounded capacity claims | Mycelix MFG-PROC-001B #6533 / draft [PR #6541](https://github.com/Luminous-Dynamics/mycelix/pull/6541) | `ManufacturingResourceOfferV1`, stable offer ID/revision, owner-native provider/resource/site/configuration and prerequisite refs, declared-capacity profile, optional calendar and lifecycle-event refs | 17-case Rust corpus, strict reference checks, JSON Schema Draft 2020-12 conformance and negative controls are being added; latest corrected exact head `ac1255ccdc660ac55a8102cb1754bb712a68a5b9` is pending focused CI. Publication enum is Published/Withdrawn only; Expired is time-derived; Disputed/Superseded require external lifecycle/cross-record resolution. No provider authority, qualified process, real availability, booking or execution is proven |
| Product revision, manufactured unit, component/lot links and qualification receipt | Mycelix MFG-EVID-001 #3084 | Immutable design/BOM revision, unit identity, material lots/substitutions, calibration and qualification evidence references, repair/replacement lineage | Independent test correctness merely because a receipt exists |
| Manufacturing lifecycle and circularity provenance | Mycelix CIRC-MFG #3095 / pilot #3101 | Manufacture/install/use/service/repair/recovery/remanufacture event lineage, custody strategy and material accounting | Recovered material specification or environmental benefit without assay and accounting evidence |
| Physical reference manufacturing cell | Mycelix MFG-003 Node 0 #4689 | Metrology/inspection, 3-axis CNC, polymer additive, electronics/harness assembly, local edge, machine/data adapters and controlled safety layer | A second node is reproducible before BOM, wiring, calibration, maintenance and acceptance package is demonstrated |
| Appliance product requirements, functional/repair qualification and measured outcomes | Symtropy APPLIANCE-QUAL #1805 | First guarded low-voltage DC fan product, control-unit baseline, functional tests, repair and endurance evidence | Printer qualification, process routing or Node 0 authority |
| Appliance manufacturing process and product pilot | Symtropy APPLIANCE-MICROFACTORY #6603 | First appliance process/cell, make/buy/partner decision, DC fan pilot and product manufacturing flow | General printer architecture or local service qualification for every appliance |
| Household/neighbourhood part admission | Symtropy DISTRIBUTED-APPLIANCE-FAB #6735 | Part risk/tier admission and household/community capability requirements | A generic route compiler, machine controller or legal certification |
| Machine-family mechanical contribution | Symtropy PRODUCTIVE-MACHINE-PLATFORM #6762 / LD-Print contribution #6787 / PR #6791 | Machine-family architecture, mechanical trade study, BOM/sourceability and local supplier discovery | Competing LD-Print canonical specification or physical qualification |
| Synthetic machine capability route evaluator | Symtropy MACHINE-CAPABILITY-COMPILER #6786 / PR #6790 | Deterministic known-answer reference evaluation of synthetic requirements/routes, hard blockers, dependency diagnostics and route economics | Owner-native process/resource registry, machine-control authority, accepted manufacturing or real supplier capacity |
| City product coverage and household-service requirements | Sol-Atlas CITY-PRODUCT-COVERAGE #602 / HOME-APPLIANCE-INTEGRATION #837 / HIGHVELD-HOUSEHOLD-SERVICE #694 | Resident/service needs, product-family coverage, interface requirements and city/household projections | Manufacturing permission or machine qualification |
| Spatial materials, delivery and source-owned record projection | Sol-Atlas CITY-MATERIAL-LOOPS #569 / CITY-CONSTRUCTION-DELIVERY #573 / CITY-OWNER-RECEIPT-ADAPTER #606 | Read-only spatial material projection, time/work-package delivery feasibility and exact owner-schema adapter registry | Inventory authority, supplier qualification, material-lot custody or physical acceptance |

## 2. Public implementation audit (22 November 2026)

This is an inspection of accessible public main-branch paths, not a claim about private workspaces or unpublished code.

### Symthaea machine authority is deliberately narrower than process qualification

The current fabrication-kernel source is:
- `crates/domains/symthaea-fabrication-kernel/src/machine.rs`

Its `MachineProfile` binds:
- machine profile name;
- build minimum/maximum in millimetres;
- maximum feed rate;
- maximum nozzle and bed temperatures;
- maximum retraction;
- homing requirement.

Its `MachineCapabilities` binds a machine identity and those advertised limits. `MachineSession` / `TimedMachineSession` bind a nonce, capability snapshot, sequence and time window. This is a valuable execution/toolpath authority boundary, but it is not a complete manufacturing process profile: it does not alone describe process transformation physics, polymer batch identity/conditioning, operator/site competence, emissions control, independent metrology, product hazard classification or final product qualification.

The live printer transport audit in Symthaea #7706 says OctoPrint and Moonraker clients currently return BackendUnavailable. No live production transport should be assumed from the existence of a client type or a validated G-code artifact.

### Monitoring and execution evidence are separate from acceptance

- Symthaea #12531 owns the prequential FPM baseline-contamination correction: an anomalous observation is retained but must not update the baseline used to judge itself.
- Mycelix #4281 standardizes Fabrication Process Monitoring terminology; #4282 separates observation, inference, intervention and quality assessment; #4283 describes a deterministic FPM engine and adversarial trace corpus.
- Mycelix #4685 owns the first-class execution-evidence layer for the manufacturing node; draft PR #5228 defines a strict origin-aware ProcessRunReceipt.
- Mycelix #3084 remains the manufactured-product/unit and qualification-receipt owner. A run receipt, sensor observation, detector output or monitoring report is not a product qualification or an accepted part.

### Mycelix public main shows the exact type-resolution work #3085 owns

Inspected paths under `mycelix-workspace/mycelix-manufacturing`:
- `zomes/machines/integrity/src/lib.rs`: `MachineEntry.capabilities` remains `Vec<String>`; `machine_type` is a bounded MachineType.
- `zomes/operations/integrity/src/lib.rs`: `OperationEntry.machine_type` and `RoutingStepEntry.machine_type` remain strings.
- `zomes/planning/integrity/src/lib.rs`: MRP run, planned order and shortage records.
- `zomes/workorders/integrity/src/lib.rs`: work-order product, quantity, due date, BOM/routing hashes and status.
- `crates/manufacturing_common/src/lib.rs`: shared BOM, operation, routing and work-order types.

This validates the existing #3085 gap: replace string comparison as an authority with versioned references to Symthaea process/capability semantics, while preserving a migration period. Do not implement a competing machine/resource registry in Symtropy.

### Sol-Atlas already owns the adapter boundary

Sol-Atlas #606 is explicit that synthetic chain receipts are not canonical cross-repo schemas. Every adapter must pin the exact owner contract/schema ID, version, source revision, content digest, subject mapping, evidence and revocation states, and fields not inferred by the adapter. Keep it read-only and claim-preserving.



### Mycelix MFG-PROC migration and offer sequence

- MFG-PROC-001A #6531 / draft PR #6532 pins references to Symthaea PR #5701 at `0b3feb49a3c2e9c1c1748ca2a0c0ff40ed320ecd`. Symthaea #5901 remains the current-main replay owner. Until source objects are resolved by a future exact resolver, these are syntactically present, not verified.
- MFG-PROC-001B #6533 / draft PR #6534 now provides the v0.1 resource-offer schema, positive declared-only example and 16 synthetic adversarial cases. It distinguishes provider signer declaration, exact owner refs, declared capacity, calendar source, bookable capacity, reservation and observed output. The exact-head push schema/fixture workflow passed; the PR-event rerun was in progress at last inspection.
- Symtropy #6786 consumes only the raw, owner-native disposition after schema/version/digest pinning. It must not promote offer publication into resource availability or qualified capability.

## 3. Required adapter contract

A future adapter from Symtropy's synthetic compiler into owner-native records must be version-pinned and loss-aware. At minimum it should carry:
- owner repository and contract identifier;
- exact schema ID/version and schema digest;
- source commit / contract revision;
- source object or receipt ID and exact subject;
- exact product, design, BOM, machine/configuration, material lot or process profile identity as relevant;
- raw owner evidence state and disposition;
- adapter revision and transformation digest;
- source locator and rights/disclosure constraints;
- explicit unsupported / unavailable field list;
- correction, revocation, dispute and supersession semantics.

An adapter may weaken a claim to Unknown/Unavailable when information is missing. It must never strengthen a vendor assertion, synthetic result or mapped location into a qualified physical claim.

Do not define a single universal receipt with copied fields from all domains. Use small typed projections over exact owner-native payloads.

## 4. Evidence and decision boundaries

The integrated system must keep at least these states distinct:
- **Design / intent:** an exact editable design and intended function exists.
- **Process defined:** a canonical process family and transformation are identified.
- **Process modelled:** a model predicts outcome under declared assumptions.
- **Machine offer:** a provider claims a machine capability.
- **Machine qualified:** a physical machine/configuration has passed the applicable tests.
- **Job admissible:** exact part, rights, process, machine, material, site/operator, safety and inspection prerequisites pass.
- **Job authorized:** the responsible execution authority grants the exact job/session.
- **Executed:** a bounded execution receipt exists.
- **Observed:** independent sensor/metrology evidence is linked.
- **Accepted:** the product-specific acceptance predicate passes.
- **Qualified product/unit:** exact design/BOM/unit/protocol/measurement evidence is bound by the responsible owner.
- **Maintainable / repairable / reproducible:** the associated maintenance, source, skill, tooling, calibration and next-generation evidence exists.
- **Spatially projected:** Sol-Atlas can display the owner-native capability/evidence without upgrading its claim.

A state transition requires its own evidence. Do not infer a downstream state from an upstream one. A sensor observation is not a detector finding; a detector finding is not an executed intervention; an intervention is not a quality assessment; a process-run receipt is not an accepted manufactured unit.

## 5. Product-driven initial workload

Symtropy APPLIANCE-QUAL-001 #1805 owns the first guarded low-voltage DC fan product and its functional/repair benchmark. It calls for commercial control units, common-condition airflow/power/sound/vibration/thermal measurements, repair-task timing, and later endurance testing. APPLIANCE-MICROFACTORY #6603 owns the manufacturing process/cell. Symtropy #6791's part-demand corpus now includes a candidate fan production workholding jig derived from that explicit fixture/jig/gauge/test-adapter workstream; its geometry, rights, material, tolerance, demand and acceptance remain unknown and the candidate is not admitted.

Use these as separate qualification subjects:
- the printer/machine is qualified under Symthaea #12533 and FDM benchmark/spatial owners;
- the manufacturing fixture has its own fit, stiffness, repeatability and assembly acceptance;
- the fan product has its own guarded powered, airflow, electrical, acoustic, thermal, repair and endurance acceptance under #1805;
- the production cell has its own reproducibility and safety evidence under Mycelix #4689.

A successful fixture print does not qualify the fixture's measurement or assembly performance; a successful fixture does not qualify the fan; a qualified fan does not prove a manufacturing node is reproducible.

## 6. Benchmarks and industry comparison

Compose, do not duplicate:
- Symthaea FDM #7960/#7961/#7998 for dimensional/spatial print capability.
- Symthaea MFG-VALUE #7608 for independent conventional baseline, throughput, yield, cost, downtime, repair and lifecycle outcome comparisons.
- Symthaea HABITAT-VERTICAL #8051 and MULTIPURPOSE-FACTORY #12355 for product–process–factory co-design and capability-network reconfiguration.
- Symthaea ATELIER-MICROFACTORY #12288 / ATELIER-FACTORYBENCH #12289 for a product-specific reference cell and matched facility benchmark, not generic authority.
- Symtropy FACTORY-ACCEPTED-OUTPUT-ECONOMICS #5966, MATERIAL-ACCOUNTED-ROUTES #5875, FACTORY-CAPABILITY-RESILIENCE #5704 and FACTORY-EXECUTION-REALITY #5758 for distinct accepted-output economics, material accounting, common-cause robustness and intended/recorded/observed reconciliation.
- Mycelix #4689 for physical Node 0 reproducibility; MFG-PROC-001A #6531 / PR #6532 for typed process refs; MFG-PROC-001B #6533 / PR #6541 for distributed resource offers; NIST-IR-8536-PROFILE #6098 for traceability interoperability.
- Symtropy #6735 for household/neighbourhood admission and tier rules.
- Sol-Atlas #569/#573/#602/#606 for qualified material geography, delivery, city product coverage and read-only owner-native projections.

No one owner proves the whole factory. A feasible process route, a configured workcell, an issued resource offer, a schedule, a signed execution event and a qualified product are separate facts.

The integrated benchmark should compare the same exact product/task, design/BOM revision, material/lot, machine/process profile, operators/skills, site/utilities, inspection protocol, quantity/time window and acceptance predicates. Use:
1. OEM replacement route;
2. local CNC/job-shop route;
3. existing bureau/contract FDM route;
4. qualified Node 0 route;
5. household/neighbourhood route only when admitted.

Report each objective separately: accepted output per hour; first-pass/final yield; total cost per accepted part; P50/P90 request-to-part and request-to-restoration; measurement uncertainty and spatial worst-region error; energy/material/waste; downtime/MTTR/spares; second-source coverage; local service vs local manufacture; and safety/claim blockers. Do not use one weighted “smart factory” score or count failed jobs outside the denominator.

### External benchmark and standards positioning — 23 November 2026

This work should compete on **evidence-qualified production outcomes and decision quality**, not pretend to replace industrial interoperability or additive-manufacturing physics standards.

- **NIST AM-Bench** provides controlled additive-manufacturing measurement challenges and blind model-validation datasets. AM-Bench 2025 includes eight metal-AM sets and one polymer vat-photopolymerization challenge; it is not a desktop material-extrusion leaderboard. NIST's AM-Bench 2028 planning points toward higher-TRL industrial components. Our complementary benchmark is a spatially resolved desktop MEX / local-production campaign with independently inspected accepted parts, route feasibility, repair and cost per conforming output. Links: [NIST AM-Bench](https://www.nist.gov/ambench), [2026 NIST AM-Bench publication](https://www.nist.gov/publications/ambench-2025-additive-manufacturing-benchmark-test-series-overview).
- **ISO 23247** should be treated as an interoperability envelope: [Part 1: general principles](https://www.iso.org/standard/75066.html), [Part 5: manufacturing digital thread (2026)](https://www.iso.org/standard/87425.html), and [Part 6: composing independently developed digital twins (2026)](https://www.iso.org/standard/87426.html). This reinforces the current owner model: semantic and evidence adapters across independent twins, not a single universal database/twin.
- **MTConnect and OPC UA** remain the preferred equipment observation/interoperability pathways where supported: [MTConnect documentation](https://mtconnect.squarespace.com/documentation) and [OPC Foundation / MTConnect collaboration](https://opcfoundation.org/markets-collaboration/mtconnect/). Do not create a replacement device protocol or infer control authority from an observation adapter.
- **Symthaea FDM #7960/#7961/#7998 and MFG-VALUE #7608** remain the benchmark owners. Symtropy consumes their exact artifact/profile/metrology/outcome references; it does not create another FDM evaluator or an AI-industry-wide benchmark.

### Targeted benchmark tournament

Compare on the same task, exact revision, material lot, machine configuration, staffing, utilities, inspection system and time/cost accounting:

1. OEM supply and warranty route.
2. Conventional regional job shop / bureau route.
3. Fixed recipe plus conventional inspection and repair policy.
4. Standard SPC / threshold-monitoring policy where valid.
5. Conventional rules-based resource/route planner.
6. Calibrated statistical/ML predictor with abstention and uncertainty.
7. Symthaea process prediction/decision recommendation alone.
8. Symtropy deterministic route/simulation evaluation alone.
9. Integrated Symthaea–Symtropy workflow, with Mycelix provenance/coordination measured as a separate contribution.

Mandatory independent outcomes (do not aggregate into a weighted score):
- **Safety and route admissibility:** false-feasible count on rights/revision/material/safety/calibration/operator/inspection negative controls; report false-unknown and false-infeasible separately.
- **Conformity:** first-pass yield, final yield, rework/scrap, critical escapes, feature-level dimensional error by location/orientation, measurement uncertainty and inspection coverage.
- **Economics:** fully burdened cost per independently accepted/conforming part; include inspection, setup, labour, energy, transport, maintenance, rework, scrap and warranty reserve.
- **Time:** request-to-first-conforming-part P50/P90, request-to-restoration P50/P90, queue/inspection delays and schedule adherence.
- **Resilience:** MTTR, second-source coverage, critical single-source dependencies, local-service versus local-manufacture status, replacement-generation requirements.
- **Material/energy:** accepted-part normalized material, energy, water/consumable and waste metrics; recovered feedstock counts only with grade/custody/requalification evidence.
- **Traceability:** percentage of accepted units bound to exact design/BOM, process/recipe, machine/configuration, material lot, calibration, independent inspection, execution and post-repair verification records.
- **Digital-twin credibility:** prediction calibration, uncertainty coverage, domain-of-validity abstention and simulation-to-measurement discrepancy on held-out campaigns.

Preregister the primary metric and hard guardrails, group holdouts by independent machine/lot/campaign (not windows from one part), and report paired uncertainty at the lot/campaign level. The MFG-VALUE #7608 hypothesis is at least 10% lower cost per conforming part than the strongest fair conventional baseline at matched quality-escape risk, with a paired 95% interval excluding zero and no guardrail regression. This is a target, not an achieved result or universal industry benchmark. A single physical article can support only a bounded demonstration, not a yield or SOTA claim.

## 7. Immediate implementation order

1. Finish Symthaea #12533 architecture dossier using Symtropy #6791 as contribution material.
2. Freeze the existing FDM quality/spatial test protocol and use it for LD-Print design/qualification; no second FDM evaluator.
3. Treat Mycelix #4689 as Node 0; map printer and M0 workbench as cell assets/stations in that reference cell.
4. Let Mycelix #3085 own typed machine/resource/process/routing references and preserve legacy compatibility fields until migration.
5. Let Mycelix #3084/#3095 bind the actual product/unit/qualification and repair/recovery lifecycle.
6. Use Sol-Atlas #606 to build exact owner-native adapter registrations; use #602/#569/#573/#837 for downstream service, material, delivery and resident projections.
7. Keep Symtropy #6786 synthetic-only until exact external contracts and at least one physical low-risk article are available.

## 8. Claim ceiling

This crosswalk records publicly visible owner scopes and accessible main-branch implementation details. It is not a complete private-workspace audit, schema-conformance result, physical machine qualification, supplier approval, city service-capacity finding or NIST/ISO certification. Public main may change; adapters must pin exact commits/schema revisions and rerun conformance when source contracts change.


### Typed process and resource-offer software qualification — 23 November 2026

- Mycelix [MFG-PROC-001A #6531 / PR #6532](https://github.com/Luminous-Dynamics/mycelix/pull/6532): exact head `8e83efbc50fedad2c75f204d371a1bb7e4088ead`; focused workflows #45077583703/#45077579607 passed committed formatting, Rust compile/tests (including the 16-case fixture), strict Clippy and schema parsing.
- Mycelix [MFG-PROC-001B #6533 / PR #6541](https://github.com/Luminous-Dynamics/mycelix/pull/6541): exact head `1600f961776d1b1d2a486f83368af705611737ab`; workflow #45078279445 passed committed formatting, 16-case resource-offer tests, strict Clippy and schema/corpus JSON parsing.

Both PRs are draft/unmerged. Their success qualifies the local software contracts only; source-owner payloads are unresolved, provider/resource/site authority is not verified, and no actual machine capability, calendar availability, reserved capacity or accepted output is established.