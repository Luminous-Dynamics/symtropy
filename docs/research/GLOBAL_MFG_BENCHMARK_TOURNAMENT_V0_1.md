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
- wrong hierarchy level (supranational/national/subnational/local authority/customs/data-residency scope);
- authoritative-language versus working/official translation confusion, missing translation artifact, or translation silently replacing source text;
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

### B1. Qualification transfer between sites and manufacturing realizations

**Question:** When a process is proven at source site A, what exact evidence permits any part of that capability to transfer to target site B?

This is a separate benchmark from twin composition and factory reconfiguration. It composes Symthaea MFG-FOUNDATION #5763/#5765 and MFG-PROC-006 #5697; it must not create another transfer/currentness engine.

Freeze a source-site evidence package and a target-site profile. Mutate one factor at a time and in controlled combinations:

- equipment model versus exact equipment build, controller/firmware and configuration;
- tooling/fixture geometry, wear state, installation and calibration;
- process recipe, software, parameter and material-state revision;
- supplier site, material origin, lot and legally relevant transformation sequence;
- utility quality/continuity, ambient environment, contamination and site conditions;
- workforce skills, training, shift patterns and process handover;
- measurement-system, instrument, calibration, MSA and inspection method;
- product variant/revision, specification and acceptance plan;
- destination-market and responsible-operator obligations.

The transfer assessor must produce a dimension vector rather than a universal transferable=true:

- direct applicability under exact matching profile;
- bounded transfer supported by named bridge evidence;
- challenged/contradicted under target profile;
- requalification required;
- not applicable;
- unknown/insufficient evidence.

A target-site process history must remain separate from the source-site history. A common machine model, nominal recipe, common country, same product drawing or one successful target-site article does not prove production capability transfer.

**Matched baselines:** (1) manual engineering/quality review; (2) copy source process settings without structured transfer audit; (3) conventional capability-history and change-control process; (4) Luminous transfer/applicability assessment composed with exact MFG-PROC and Mycelix lineage.

**Measures:** false-transfer rate; missed requalification triggers; correct abstention; time and cost to establish a target-site qualification; first-accepted-output time; first-pass yield and drift after transfer; calibration/measurement comparability; supplier/lot sensitivity; completeness of target-site evidence.

**Hard controls:** changed tool/controller/recipe/material/lot/environment/measurement system without bridge evidence; expired calibration; changed product variant; source-site data pooled into target-site capability; a failed scale-up hidden by successful coupons; supplier site mistaken for component origin; a foreign legal approval presumed recognized by the target market.

Use held-out target sites, lines and lots for confirmation. Do not claim portable qualification from synthetic profile permutations or simulation. For continuous characteristics, justify process stability, distribution assumptions, sampling/subgrouping and measurement-system suitability before reporting Cp/Cpk/Pp/Ppk. Time-dependent processes should be compared with the applicable method in ISO 22514-2:2026 rather than forcing a stable-process assumption.

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

## 8. External benchmark and standards watch-list (checked 17 November 2026)

- **NIST SI for Manufacturing** is an active measurement-science programme for trustworthy manufacturing intelligence. Its published work plan includes integration test benches, data/contextualization architectures (including UNS/MQTT, knowledge graphs/type models, I3X and OPC UA), fault detection/diagnosis, VVUQ and agentic decision tasks. Its proposed metrics include accuracy, latency, cost and grounding, with ablations for contextualization. Use it as a benchmark-programme alignment and future interop target, not as a published leaderboard that Luminous has beaten. https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing
- **NIST Smart Manufacturing Systems Test Bed** includes CAD/CAM/CAI/PDM/V&V workflows, CNC machine tools, CMM/dimensional inspection equipment and MTConnect data publication. It is a useful reference for designing matched interoperability and digital-thread tasks. https://www.nist.gov/laboratories/tools-instruments/smart-manufacturing-systems-sms-test-bed
- **ISO 23247-5:2026** defines a manufacturing digital thread across design, planning, production and testing. **ISO 23247-6:2026** defines integrated, unified and federated digital-twin composition, reflecting that a single monolithic factory twin is often impractical. https://www.iso.org/standard/87425.html and https://www.iso.org/standard/87426.html
- **ISO 22514-2:2026** provides capability/performance methods for time-dependent process models, including processes that are not continuously in statistical control. Use it where its continuous-characteristic scope and assumptions fit; do not report capability indices without suitable stability, sampling and measurement-system evidence. https://www.iso.org/standard/88883.html
- **ISO 9001:2026** was published 16 September 2026; **ISO 14001:2026** was published 15 April 2026. Quality and environmental management-system transitions should be represented by exact edition and date in the jurisdictional obligation layer, not assumed from an old generic "ISO certified" field. https://www.iso.org/standard/9001 and https://www.iso.org/standard/14001
- **ISA/IEC 62443** remains a relevant industrial automation and control-system cybersecurity family. Security zones/conduits, system ownership, patching and supplier responsibilities should be part of plant deployment qualification, not an afterthought. https://www.isa.org/standards-and-publications/isa-standards/isa-iec-62443-series-of-standards
- **FJSP-E2E** reports 72 expert-audited scheduling-model reconfiguration tasks and structural process diagnostics beyond makespan checking. The publisher currently assigns the paper to *Journal of Manufacturing Systems*, Volume 89, December 2026, which is later than this review date. Track as an emerging benchmark watch item; confirm formal online publication date and public code/data before treating it as a released head-to-head baseline. https://doi.org/10.1016/j.jmsy.2026.08.009

These references are standards/research comparators, not blanket conformity or proof of Luminous superiority.

## 9. Release and claim ceiling

A schema pass is not a legal review. A capability offer is not process qualification. A digital-twin pass is not physical acceptance. A first-article pass is not production yield. A locally assembled product is not necessarily locally originated. A passport projection is not a registry submission. A benchmark pass is not homologation or authority to sell.

Every public comparative claim must identify the exact product, process, site, market, dataset, evidence class, baseline, metric, denominator, uncertainty, source date and limitations.
