# Global Manufacturing Benchmark Tournament v0.1

**Programme:** GLOBAL-MFG-DEPLOY-001  
**Companion:** GLOBAL-MFG-DEPLOYMENT-PROFILE-V0_1.md  
**Status:** protocol proposal; no comparative performance results  
**Date:** 17 November 2026

## 1. Competitive thesis

Luminous should not claim to beat industrial manufacturers at their strongest isolated capabilities: high-volume automotive takt, battery-cell yield, precision-machine dynamics, established supplier qualification or the accumulated features of mature MES/ERP/PLM suites.

The proposed competitive advantage is the **verified composition of global engineering, local manufacturing capability, regional market obligations, independent inspection, lifecycle provenance and reconfiguration**.

The benchmark asks whether a product can be designed once, realized through different qualified manufacturing routes, independently accepted in the relevant contexts, and then repaired or reconfigured without losing its engineering identity or evidence.

This is not a single composite score. Safety, law, product acceptance, worker protection, environmental permits, evidence provenance and traceability are hard gates. A weighted performance score cannot compensate for failure.

## 2. Competitor classes and matched baselines

Compare against actual deployed workflows and matched test scope, not marketing feature lists.

| Comparator | What it is strong at | What Luminous must prove |
|---|---|---|
| Spreadsheet/email and manual engineering release | Familiar, flexible, low initial software cost | Lower duplicate entry, fewer stale references, fewer missed obligations, reproducible review |
| Traditional PLM + ERP + MES stack | Mature product BOM, work orders, inventory, planning and plant execution | Cross-system semantic preservation, verifiable joins, federated suppliers/sites, source-aware evidence |
| Industrial quality/SPC/metrology suites | Established measurement, SPC, gauge and inspection workflows | Independent ground-truth mapping, measurement-system uncertainty, exact product/process genealogy |
| Vendor-specific digital-twin platforms | Integrated simulation, visualization and vendor ecosystem | Cross-vendor composition, independent VVUQ, scope-safe federation, open evidence export |
| OEM/Tier-1 manufacturing systems | Scale, process expertise, mature CoP, production engineering and supplier networks | Comparable product/volume/route results; never infer advantage from an architecture diagram |
| Distributed supplier marketplaces | Discovery, procurement, commercial capacity | Capability qualification, origin and quality evidence, custody, exact availability and execution authority |
| Luminous proposed system | Common engineering core + regional realization + evidence execution + lifecycle record | Independent matched results on the tracks below |

Potential reference frameworks and datasets include ISO 23247-5:2026 (digital thread), ISO 23247-6:2026 (integrated/unified/federated composition), MTConnect, OPC UA companion specifications, QIF/ISO 23952, NIST Smart Manufacturing Systems Test Bed and task-specific NIST AM-Bench data. These are reference/interop inputs, not automatic proof of conformance or a single competitor baseline.

## 2A. External research alignment: NIST trustworthy manufacturing intelligence

NIST's 2026 **Super Intelligence (SI) for Manufacturing** initiative is directly relevant as an external research programme. Its stated threads include trustworthy manufacturing-intelligence fundamentals, human-machine teaming, contextualized data, an integration test bench and benchmark studies, and digital-twin-assured manufacturing intelligence with VVUQ. NIST's July 2026 Digital Twins Workshops report (NISTIR 8620) separately identifies interoperability, VVUQ, cybersecurity and workforce readiness as persistent challenges.

Sources:
- NIST SI for Manufacturing: https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing
- NISTIR 8620 (published 21 July 2026): https://doi.org/10.6028/NIST.IR.8620

This is a research and measurement-science alignment, not a published NIST performance leaderboard or evidence that Luminous is ahead of the field. We should track emerging NIST benchmark datasets, test-bed interfaces and measurement methods as they become available, and compare on common tasks rather than inventing a proprietary score with no external baseline.

## 3. Evidence classes and comparison discipline

Every result must carry one of these classes:

- **Synthetic fixture** — authored data for software contract and negative-control execution.
- **Simulation output** — output from a named simulator/model and declared context of use.
- **Public source claim** — manufacturer, authority, standard or published research statement, with source date, edition, scope and limitation.
- **Instrument observed** — raw observation bound to instrument identity, calibration, method, units, timestamp/clock basis and raw-data digest.
- **Independently verified** — separate reviewer/reimplementation/measurement confirms the exact claim within scope.
- **Production accepted** — exact product, process, site, material/lot and inspection disposition accepted by the responsible independent authority.

Do not pool unlike evidence classes. Do not count echoes, pixels, layers, multiple measurements from one physical article or synthetic replications as independent physical samples. Keep training/calibration and held-out confirmation groups separate at the level where leakage could occur (article, part, lot, line, supplier or site).

Freeze:
- product/variant and product revision;
- process/recipe and material/lot identities;
- equipment/site/configuration and tool/fixture identity;
- demand, route, volume and work content;
- environmental/utility scenario and shift/calendar;
- inspection plan, instruments, calibration and measurement uncertainty;
- baseline implementation/version and solver/toolchain;
- random seed set, scenario family and held-out split;
- raw inputs, outputs, manifests and digests.

Publish exclusions, failed cases, missingness and uncertainty. A failed test must not disappear from the denominator.

## 4. Benchmark tracks

### A. Global source and obligation correctness

**Question:** Does a global deployment profile distinguish real legal obligations from proposals, guidance, voluntary standards, contracts and unknowns?

Measures:
- wrong jurisdiction/actor/product/operation/date applicability;
- proposed or adopted-not-effective source treated as in-force;
- stale/superseded source treated as current;
- foreign approval incorrectly treated as recognized;
- wrong issuer or missing authority;
- missing effective/application/transition date;
- false satisfied/not-applicable disposition;
- source currency, source digest and review lineage.

Hard targets on frozen corpus:
- 0 false-ready / false-approval results;
- 100% rejection or abstention on every authored wrong-scope, stale-source, proposal-as-law, false-recognition and unknown-to-pass control;
- 100% obligation records have a source/status/scope/date disposition or explicit unknown;
- 100/100 canonical-order permutations produce identical reports.

External legal review remains necessary. This track measures the tool's handling of a frozen obligation corpus, not whether it has captured every law worldwide.

### B. Common engineering core and regional realization

**Question:** Can the same product/process identity be realized at different sites and in different destination markets without silent semantic drift?

Benchmark panel:
- dense urban / constrained private charging and compact production site;
- hot/dusty / water constrained;
- cold/high-altitude / weak-grid;
- humid/coastal/corrosive;
- island/import-dependent and sparse specialist service;
- remote industrial/mining operation;
- low-connectivity / multilingual;
- high-volume / highly automated;
- disrupted-supplier / disaster-recovery.

First use synthetic archetypes. Later add source-backed site/market profiles; never treat an archetype as a factual country model.

Measures:
- common-core identity preservation;
- independent join correctness for manufacturing site, destination market, supplier site, material origin, transformation location, data processing and service/lifecycle;
- obligation-to-jurisdiction-dimension link correctness;
- supplier-site versus legal-origin versus transformation-location confusion detection;
- explicit incompatibility and requalification detection;
- time to create/update a local realization;
- duplicated engineering-data entry;
- regional exception and waiver traceability;
- deployment lead time and rework;
- offline/manual operation and local-language procedure access.

Initial hard target: every variant preserves the shared engineering-core identity or emits an explicit, reviewed product-variant/requalification path. No invisible local override.

### C. Process capability and product quality

**Question:** Does a named process, machine, material and environment repeatedly produce the required features?

Measures:
- critical-feature error and repeatability by feature, axis, orientation, region and environmental condition;
- capability indices (Cp/Cpk/Pp/Ppk) only after process stability, distributional assumptions, subgroup design and measurement-system suitability are justified;
- first-pass yield, final acceptance, scrap, rework and quarantine with denominators;
- false accept / false reject / indeterminate rates;
- inspection time and detection delay;
- calibration and requalification burden;
- time from design release to first independently accepted article.

Baselines:
- fixed engineering limits;
- conventional SPC/control-chart workflow;
- calibrated metrology / QIF record;
- process-specific qualified incumbent procedure;
- proposed Luminous capability profile and evidence audit.

No single pooled average may hide a weak site, feature, lot, orientation or product family.

### D. Factory flow, capacity and economics

**Question:** Does the proposed factory route deliver the same accepted service at a better fully loaded cost and disruption profile?

Measures:
- takt, throughput, bottleneck capacity, WIP, queue and delivery lead time;
- first-pass yield and saleable yield;
- cost per finally accepted product, including material, energy, labor, equipment, inspection, consumables, scrap, rework, maintenance and logistics;
- capex, fixed/variable cost, working capital and utilization;
- energy, water and thermal use per accepted output;
- cost and service sensitivity under demand and supplier scenarios.

Baselines:
- manual/FCFS scheduling;
- conventional finite-capacity planning;
- constrained optimization;
- published dataset/benchmark where task and objective match;
- candidate Luminous scheduler/route proposal.

Report median, P90/P95 and scenario-family worst case where data support them. A synthetic plant model is not a supplier quote, a bankable CAPEX estimate or a measured factory result.

### E. Reconfiguration and resilience

**Question:** Can the production network change product, process, site or module while preserving accepted output and evidence?

Primary measure:
**last independently accepted output in configuration A → first independently accepted output in configuration B**.

Include:
- WIP and material-lot disposition;
- safe stop and isolation;
- module/tool/fixture movement and connection;
- utility, environmental and contamination changes;
- recipe/software/revision changes;
- worker skill/authorization;
- metrology and calibration;
- commissioning, first-article inspection and stable restart;
- suspect lot quarantine, rollback, recovery and service backlog.

Measures:
- elapsed time and cost by phase;
- first-article acceptance;
- backlog and lost accepted output;
- traceability completeness;
- quality escapes, rework and scrap;
- rollback success and time;
- worst-family recovery and maximum regret under disruption.

Compare against conventional changeover reports and the same factory using manual planning. Do not compare only robot travel or equipment relocation time.

### F. Digital twin and evidence composition assurance

**Question:** Does composing independently valid twins preserve meaning and uncertainty end-to-end?

Three levels:
1. Component model/twin validity in its declared context.
2. Interface, identity, unit, coordinate, timestamp and uncertainty preservation.
3. End-to-end decision and post-action evidence.

Measures:
- independent model error and convergence;
- physical held-out validation and OOD abstention;
- calibration/coverage of predictive uncertainty;
- shared nuisance/covariance propagation;
- unit/time/coordinate mapping correctness;
- exact identity join completeness;
- replay determinism;
- partial federation, stale/missing/reordered observation and schema-skew handling;
- unauthorized/replayed/out-of-envelope action rejection;
- postcondition verification.

This track should compose Symtropy MFG-TWIN-VVUQ-001 #3121, not duplicate its model or uncertainty owners.

### G. Supply-chain and network capability

**Question:** Can a product route be fulfilled across independently operated sites with qualified capability, not just advertised capacity?

Measures:
- capability coverage against the product's process requirements;
- offer-to-qualification conversion;
- available capacity and freshness;
- exact component/material origin and transformation lineage;
- supplier concentration and common-cause dependencies;
- transport lead time and volatility;
- alternative route coverage and time to switch;
- partner handoff custody and genealogy;
- disruption recovery and service continuity.

A supplier offer is not a capability certificate. A qualified capability is not necessarily available capacity. Capacity is not execution authorization.

### H. Lifecycle, repair and circularity

**Question:** Can the manufactured product be inspected, repaired, remanufactured, repurposed and recycled with evidence continuity?

Measures:
- diagnosis, isolation, repair, reassembly and verification time distributions;
- serviceability and parts availability;
- number of repairable/reusable subassemblies;
- recovered material by verified stream and quality;
- lifecycle cost per delivered service;
- product/passport predecessor lineage after remanufacture or repurposing;
- end-of-life route and mass-balance completeness.

Keep cell, pack, vehicle, installed-system and second-life stationary-storage boundaries distinct.

## 5. SOTA strategy: where Luminous should aim to lead

The IEA's 2026 analysis reports nearly 22 million electric cars produced globally in 2025 and nearly 75% of output in China. It reports China produced over 80% of battery cells and more than 90% of anode active material. Global lithium-ion nameplate capacity exceeded 4 TWh by end-2025; the IEA notes many new facilities take more than five years to approach nominal output. These figures show the scale and accumulated industrial advantages Luminous should not assume it can reproduce immediately.

The first competitive strategy should therefore be:
- use established qualified cells and critical components;
- develop local tooling, fixtures, pack integration, assembly, inspection and repair where evidence and economics support it;
- qualify regional partner resources rather than build every capability in-house;
- specialize in low-capital, reconfigurable product-family cells;
- preserve exact product/material/process/site/inspection genealogy;
- reduce the cost and lead time of adding a site or market without changing engineering meaning;
- make repair, remanufacture, second-life and local service part of original product requirements.

The longer-term research ambition is **capability portability with qualification continuity**: a common engineering product can be produced by different qualified site networks with explicit deltas, requalification and local market evidence. This is a research target, not an achieved SOTA result.

## 6. Target-setting policy

Set targets in three tiers:

### Software and evidence invariants
- zero false accepts on a frozen and independently adjudicated hard-gate corpus;
- 100% source/configuration/manifest binding for every report claim;
- deterministic replay where the algorithm declares deterministic behavior;
- explicit abstention for unknown, stale, out-of-scope or unsupported input;
- no weighted score compensates for a failed safety, law, quality, worker or environment gate.

### Physical engineering targets
First measure matched baselines and uncertainty. Then preregister effect size, sample size, holdout, practical margin and decision loss. Do not select percentage targets after observing results.

### Commercial/global deployment targets
Measure actual time, cost, rework, uptime, energy, water, lead time, repair and market deployment against a frozen route and comparable incumbents. Keep quoted, catalog, modeled, source-backed and observed prices separate.

## 7. Release and claim ceiling

A schema pass is not a legal review. A capability offer is not process qualification. A digital-twin pass is not physical acceptance. A first-article pass is not production yield. A locally assembled product is not necessarily locally originated. A passport projection is not a registry submission. A benchmark pass is not homologation or authority to sell.

Every public comparative claim must identify the exact product, process, site, market, dataset, evidence class, baseline, metric, denominator, uncertainty, source date and limitations.
