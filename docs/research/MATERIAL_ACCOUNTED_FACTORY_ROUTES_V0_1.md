# Material-Accounted Factory Routes — Benchmark Contract v0.1

**Issue:** [Symtropy #5875 — MATERIAL-ACCOUNTED-ROUTES-001](https://github.com/Luminous-Dynamics/symtropy/issues/5875)  
**Status:** Research specification; synthetic-first. No physical process or site is qualified.  
**Created:** 2026-11-17  
**Scope:** Pump/valve subassembly and returned-pump repair/refurbishment across matched factory architectures.

## 1. Purpose

The current matched-route benchmark evaluates whether required operations can be assigned to eligible modules under capability, qualification, calibration, workforce, utility, segregation and quality gates. It intentionally uses synthetic ticks and relative cost/energy units.

This extension asks a stronger question:

> Did the route produce an independently accepted conforming output, and can every material and utility crossing the process boundary be accounted for?

The benchmark must not infer material yield, grade, purity, conformity, industrial economics or physical capability from route feasibility.

## 2. Composition: use existing owners

| Concern | Canonical owner | This benchmark's role |
|---|---|---|
| Process definition, recipe, material-state transformation and capability semantics | Symthaea [MFG-PROC-000 #5686](https://github.com/Luminous-Dynamics/symthaea/issues/5686) | Bind exact refs; do not redefine process meaning |
| Process mass closure and uncertainty-aware material streams | Symthaea [PIE-001 #1606](https://github.com/Luminous-Dynamics/symthaea/issues/1606) | Consume the canonical closure oracle |
| Energy, power, thermal quality, time and utilities | Symthaea [PIE-002 #1610](https://github.com/Luminous-Dynamics/symthaea/issues/1610) | Consume typed utility envelopes |
| Primary resources, feedstock and refining dependencies | Symthaea [CIV-UPSTREAM #5958](https://github.com/Luminous-Dynamics/symthaea/issues/5958), [CIV-MIDSTREAM #5948](https://github.com/Luminous-Dynamics/symthaea/issues/5948) | Bind evidence and preserve unresolved stages |
| Matched route assignment across four factory architectures | Symtropy [FACTORY-MATCHED-ROUTES #5432](https://github.com/Luminous-Dynamics/symtropy/issues/5432) | Extend its route corpus; do not replace its evaluator |
| Composed twin verification/validation/uncertainty | Symtropy [MFG-TWIN-VVUQ #3121](https://github.com/Luminous-Dynamics/symtropy/issues/3121) | Bind component and composition qualification receipts |
| Material lots, material-flow event accounting, lifecycle, custody and circular re-entry | Mycelix [MFG-EVID #3084](https://github.com/Luminous-Dynamics/mycelix/issues/3084), [CIRC-MFG #3095](https://github.com/Luminous-Dynamics/mycelix/issues/3095), and [accounting work PR #3107](https://github.com/Luminous-Dynamics/mycelix/pull/3107) | Consume exact subjects/events/receipts; do not create a second ledger |
| Spatial/site and regional evidence | Sol-Atlas [Subterra adapter #922](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/922), Symtropy [global manufacturing #5782](https://github.com/Luminous-Dynamics/symtropy/issues/5782) | Carry site/frame/source refs when relevant |

The composition is read-only with respect to other owners' authority. A hash/digest establishes byte identity, not truth, qualification or signer authority.

## 3. Benchmark layers

### 3.1 Frozen route and architecture layer

Retain the existing matched case:
- Product routes: pump/valve subassembly; returned-pump inspection, repair/refurbishment and requalification.
- Architectures: dedicated line; functional job shop; multipurpose microfactory; hybrid local-plus-qualified-partner network.
- Hard route gates: exact process/material envelope, qualification currentness, calibration, workforce, utilities, segregation and inspection requirements.
- Disturbances: machine/module outage, partner loss, stale metrology, unavailable skill, utility interruption and common-cause dependency loss.

All architectures receive the same product requirements, process operations, incoming material lots, acceptance criteria, demand and scenario realization.

### 3.2 Operation material-flow projection

Every operation references, rather than redefines, canonical material subjects and quantities.

Minimum logical fields:

| Field | Meaning |
|---|---|
| `operation_ref` | Exact operation in the frozen process plan |
| `process_definition_ref` / `recipe_ref` | Exact versioned process and recipe/profile |
| `input_subject_refs[]` | Exact lot/article/state subjects consumed |
| `input_quantity_refs[]` | Canonical quantities, units, basis, intervals and evidence |
| `output_subject_refs[]` | Exact output subjects created or advanced |
| `output_quantity_refs[]` | Canonical quantities and uncertainty |
| `disposition_refs[]` | Product, intermediate, coproduct, recovered residual, waste/effluent, inventory delta or unresolved remainder |
| `material_state_transition_ref` | Evidence-backed input-to-output state/grade transformation |
| `measurement_refs[]` | Sample, assay, inspection or metrology refs where required |
| `evidence_class` | Observed, public-source, model-derived, simulated, synthetic, assumed or unknown |
| `source_digest` | Exact source/artifact identity where supported |

These are a projection of canonical identities, not a new material ontology or ledger. If a canonical field or type already exists, reuse it. Do not add duplicate quantity, material, grade or custody semantics.

### 3.3 Process-boundary mass accounting

For each boundary, account for all inputs, including added feedstock, consumables/reagents that become material outputs, and explicitly modelled inventory changes.

The intended balance is:

`total admitted material input = all accounted material outputs + declared inventory change`

The right-hand side must include conforming product/intermediate, coproducts, recoverable residuals, waste/effluent and any other process-boundary output. Process loss is not a free balancing term: it needs a defined, evidence-bearing disposition or remains unresolved.

Use the canonical PIE-001 interval-aware oracle and Mycelix material-flow accounting semantics. Do not create another balance algorithm. Do not silently:
- treat missing or below-detection composition as zero;
- conflate dry and as-received mass;
- convert between oxide-equivalent, elemental and mineral-phase quantities without a sourced conversion;
- allocate the same physical mass to both harvested parts and recycled material;
- upgrade a recovered constituent to specification-grade feedstock;
- infer a physical atom-level chain of custody after mixing.

Return the canonical status and the full residual/interval context. A numerically small residual is not an automatic acceptance if uncertainty or missing streams are mishandled.

### 3.4 Utility and thermal account

Material quantities must remain separate from utilities. Bind the canonical PIE-002 quantities/profiles for whichever are required by the operation:
- electrical and other energy;
- peak and continuous power;
- process heat and rejected heat, including temperature/quality compatibility;
- process water and water-quality requirements;
- gases, vacuum, refrigeration and other consumables;
- process time, batch/continuous basis and duty cycle;
- waste, emissions and treatment capacity where in scope.

Average energy over a shift does not prove instantaneous power sufficiency. A heat quantity is not interchangeable across temperature grades. Recovered heat cannot be allocated to multiple consumers simultaneously. Common upstream faults must remain common-cause dependencies.

### 3.5 Independent acceptance layer

The evaluator must distinguish:
- planned operation;
- process started;
- process completed;
- observation acquired;
- result measured against frozen criteria;
- result accepted or rejected.

Only valid inspection and functional evidence bound to the exact output article/lot, design/BOM revision, process/recipe, measurement method and calibration may produce an accepted output disposition. A scheduler result, self-reported yield, model estimate or completed work order is not product acceptance.

## 4. Evaluation outputs

Do not collapse outcomes into one weighted score. Preserve an outcome vector and all hard blockers.

### Quality and service
- accepted conforming units;
- first-pass yield and final yield, with denominators;
- rework, scrap and defect escape counts;
- false acceptance and false rejection;
- repair return-to-service and repeat-failure measures where real data exist.

### Material and circularity
- total input and virgin input per accepted unit;
- recovered/secondary input, with grade and custody admission evidence;
- product, coproduct, recoverable residual, waste and unresolved remainder;
- mass-balance status and interval residual;
- fraction of recovered material actually re-qualified and re-entered;
- no double-counted mass.

### Utility and production
- energy, peak power, thermal service, water and consumables per accepted unit;
- elapsed time, queue/lead time, throughput and bottleneck wait where modeled;
- downtime, recovery time and reconfiguration-to-first-accepted-output;
- fully burdened cost per accepted unit, with currency date, scope, labor, energy, amortization and partner assumptions explicit.

### Assurance
- source/lot/recipe/resource/inspection traceability completeness;
- uncertainty coverage and calibration status;
- out-of-domain detection and abstention;
- schema/identity/time/unit mismatch rejection;
- deterministic replay of the declared deterministic portion;
- model-to-measurement discrepancy on held-out physical evidence.

Each outcome reports its evidence class, source, version, uncertainty, applicability domain and denominator.

## 5. Baselines and fair comparison

1. **Existing route evaluator:** the frozen baseline for route feasibility under #5432.
2. **Simple transparent dispatch:** deterministic greedy selection, explicitly defined and reproducible.
3. **Exact small-instance enumeration:** oracle for bounded route assignment and Pareto correctness.
4. **CP-SAT/MILP:** admit as a scheduling comparator only when multi-order, capacity, precedence and non-overlap constraints are introduced. Google's public job-shop formulation is a useful reference for precedence and machine non-overlap constraints: https://developers.google.com/optimization/scheduling/job_shop.
5. **Conventional industrial workflows:** job shop, OEM/authorized repair, dedicated line, flexible cell and qualified hybrid/partner route, using matched measured data only.

No apples-to-oranges claims. Synthetic relative units cannot be compared to vendor quotes, published factory throughput or measured kWh. Publish task, version, data, boundary, objective and acceptance equivalence before comparing results.

## 6. Dataset strategy

- **AM-Bench 2025 (NIST):** useful for a separate additive-manufacturing model/measurement sub-benchmark. NIST reports eight metal measurement/challenge sets and one polymer set; the challenge series includes blind submissions. It is not pump/valve CNC or repair data and must not be presented as such. https://www.nist.gov/ambench
- **NIST Measurement Science for Additive Manufacturing:** use for metrology, machine/material/process/part qualification and process-monitoring measurement challenges. https://www.nist.gov/programs-projects/measurement-science-additive-manufacturing-program
- **Recorded factory data:** only use lawful, source-pinned, quality-controlled records with exact part/lot/process/inspection identity. Separate training, calibration and held-out test data by relevant source, lot, machine, time or site; do not random-split correlated observations into leakage.
- **South African localization:** use official dtic / government / supplier sources with publication date and scope. The 2026 Industrial Development Strategy prioritizes decarbonisation, digitalisation and diversification; the 2025 Critical Minerals and Metals Strategy includes value addition/localisation, research/innovation, infrastructure/energy security and circular economy. These are strategic context, not evidence of local reserves, demand, site viability or product eligibility. https://www.thedtic.gov.za/south-africas-industrial-development-strategy-2026/ https://www.gov.za/documents/other/critical-minerals-and-metals-strategy-south-africa-2025-12-may-2025

## 7. Adversarial and positive controls

The first frozen synthetic corpus must include:
1. positive closed process boundary with all output streams accounted;
2. interval overlap that is consistent but not exact;
3. intervals that are disjoint, so closure is impossible;
4. input lot name collision with distinct exact subjects;
5. missing added material/reagent;
6. missing coproduct/residual output;
7. below-detection result incorrectly treated as zero;
8. wet/dry or unit basis mismatch;
9. unknown constituent remainder;
10. negative mass or impossible quantity;
11. same recovered mass double-allocated to component harvest and recycling;
12. recycled material promoted without reprocessing/assay;
13. process completed without accepted inspection;
14. inspection bound to wrong article/revision/lot;
15. stale or invalid calibration;
16. average energy sufficient but peak power unavailable;
17. heat available at incompatible temperature grade;
18. water or waste-treatment capability unavailable;
19. shared feeder/partner/metrology root presented as independent redundancy;
20. source, recipe, material, machine or site revision changed without downstream impact assessment;
21. stale, duplicated, reordered or replayed evidence;
22. unknown/withheld evidence converted into a pass;
23. cheaper/faster route that fails a hard acceptance or material-accounting gate;
24. accepted synthetic output correctly labelled synthetic and not physical.

The evaluator and hand-calculated oracle must agree on every positive and negative case.

## 8. Promotion gates

| Stage | What it may establish | Required before promotion |
|---|---|---|
| 0: synthetic | Contract/evaluator behavior on frozen synthetic corpus | Oracle agreement, hostile controls, reproducible digests, no authority escalation |
| 1: public measured data | Model accuracy for a specifically bounded public dataset/process | Dataset license/identity, exact measurements, method and uncertainty, valid train/holdout split, reproducible evaluation |
| 2: recorded-data replay | Retrospective fit on a real, bounded site/process | Lawful source, quality system, calibration/identity lineage, independent holdout, discrepancy and OOD reporting |
| 3: prospective pilot | Local performance of a bounded workflow | Risk review, approved test plan, human authority, matched baseline, independent inspection, incident/stop criteria |
| 4: operational deployment | Only the exact scope qualified | Product/process/site/jurisdiction qualification, cybersecurity, worker safety, permits and responsible operator approval |

A stage cannot be skipped because an earlier-stage score is high. A draft, queued or skipped CI run is not a passing check; physical qualification is not replaced by software tests.

## 9. External benchmark landscape and differentiation

The field is moving toward composed manufacturing digital twins, VVUQ, high-quality process measurements, trustworthy AI and product-lifecycle traceability:
- ISO 23247-6:2026 defines integrated, unified and federated manufacturing digital-twin composition: https://www.iso.org/obp/ui?_escaped_fragment_=iso%3Astd%3Aiso%3A23247%3A-6%3Aed-1%3Av1%3Aen
- NISTIR 8620 (21 July 2026) identifies interoperability, VVUQ, cybersecurity and workforce readiness as manufacturing digital-twin challenges: https://www.nist.gov/publications/digital-twins-workshops-summary-report
- NIST's 2026 Smart Manufacturing AI/ML Roadmap covers industrial data, perception, autonomy, AM, twins, robotics, supply chain and sustainable manufacturing: https://www.nist.gov/publications/2026-roadmap-artificial-intelligence-and-machine-learning-smart-manufacturing
- EU's DPP Registry became operational on 20 July 2026; applicable battery passport obligations are scheduled for 18 February 2027: https://single-market-economy.ec.europa.eu/single-market/digital-product-passport_en

The differentiator is not claiming to replace all industrial MES/ERP, CAE, CAM or digital-twin suites. The measurable research advantage to pursue is a **cross-owner, evidence-carrying route decision** that binds exact input material and process identity to utility constraints, independent inspection, mass balance, lifecycle recovery and reproducible trade-offs.

## 10. Immediate implementation sequence

1. Audit exact current heads and canonical schemas for #5434, PIE-001/#1606, PIE-002/#1610, MFG-PROC/#5686 and Mycelix #3095/#3107 before coding.
2. Freeze one small material-accounted pump/valve route with explicit primary input, added material, accepted product, coproduct and residual streams.
3. Add a separate independent evaluator that consumes route transcripts and canonical accounting results; it must not call the candidate planner or hidden scenario oracle.
4. Add the adversarial corpus and exact manifest/digest.
5. Emit a machine-readable per-route outcome vector and a human-readable report.
6. Run local format, lint and tests when a local checkout is available; report exact-head CI independently.
7. Do not expand to batch/queue stochastic scheduling until the single-unit boundary is correct. Do not promote a synthetic result to physical evidence.

## Non-claims

This benchmark does not establish actual recoveries, material grades, machine capability, production yield, energy intensity, local-content eligibility, lifecycle impact, commercial viability, regulatory compliance, worker safety or industrial SOTA. Those require separate, scope-matched, source-backed and, where relevant, physical evidence.
