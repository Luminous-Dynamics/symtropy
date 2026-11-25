# Physical Memory Compiler — Research and Contract v0.1

**Programme:** PHYSICAL-MEMORY-COMPILER-001  
**Tracking:** [Symtropy #7482](https://github.com/Luminous-Dynamics/symtropy/issues/7482)  
**Parent:** [PHYSICAL-STORAGE-SYSTEMS-001 #7447](https://github.com/Luminous-Dynamics/symtropy/issues/7447)  
**Connected algorithmic work:** Symthaea [#13630](https://github.com/Luminous-Dynamics/symthaea/issues/13630), [#13662](https://github.com/Luminous-Dynamics/symthaea/issues/13662)  
**Connected physics qualification:** Symtropy [glass #7477](https://github.com/Luminous-Dynamics/symtropy/issues/7477), [magnetic recording #7450](https://github.com/Luminous-Dynamics/symtropy/issues/7450), [NVM #7481](https://github.com/Luminous-Dynamics/symtropy/issues/7481)

## 1. Thesis

A physical storage design should be compiled from a service contract, not selected from one headline product metric.

Input is a typed description of what information must survive, how much is written/read, how quickly it must be restored, what physical environment it experiences, what errors are acceptable, what failure domains must be survived, and what manufacturing/economic constraints apply.

Output is a reproducible design dossier:
- qualified medium family and status;
- code family, parameters and physical layout;
- calibration, metadata and spare-region plan;
- reader/writer assumptions and qualified model adapters;
- exact recovery and restore evidence;
- process and metrology requirements;
- uncertainty, missing-data and unsupported-regime register;
- lifecycle, decoder replacement and migration plan.

The compiler is an evidence-gated engineering system, not a product recommender that guesses from a capacity table.

## 2. Files in this protocol

| File | Purpose |
|---|---|
| `service_contract_v0_1.schema.json` | Strict Draft 2020-12 JSON Schema for the storage service requirements |
| `example_1pb_archive_contract_v0_1.json` | Illustrative 1 PB deep-archive contract; not a customer requirement |
| `candidate_decision_v0_1.schema.json` | Schema for machine-readable candidate assessments |
| `example_candidate_decision_1pb_v0_1.json` | Worked example that correctly refuses to rank the candidates |
| `../PHYSICAL_STORAGE_SYSTEMS_001.md` | Technology baselines, qualification gates and physical storage benchmark programme |
| `../fixtures/industry_baselines_v0_1.json` | Source-pinned current product/research/roadmap ledger |

The schemas enforce types, enumerations, required fields and unknown-value handling. Additional semantic validation is required for cross-field constraints such as ordered environmental ranges, workload fractions summing to one, the number of independent failure domains not exceeding copies, and dependency-profile reference/status consistency. A schema-valid contract can still be physically infeasible.

## 3. Compile pipeline

1. **Parse and normalize.** Validate JSON, units, dimensions, explicit nulls, classifications and cross-field invariants. Never silently infer a missing temperature, cost, retention or throughput.
2. **Resolve service semantics.** Translate capacity, growth, access and restore constraints into a frozen workload and system boundary.
3. **Filter by evidence status.** Separate commercial products, customer-qualified products, research demonstrators and roadmaps. Roadmaps and lab demonstrations cannot become production-eligible candidates.
4. **Qualify failure-domain independence.** Consume the canonical multi-valued/transitive dependency profile and cut-set semantics from Mycelix [ASSURE-PORTFOLIO-CUT-002 #5762](https://github.com/Luminous-Dynamics/mycelix/issues/5762), through Symtropy's dependency-diversity adapter [#7341](https://github.com/Luminous-Dynamics/symtropy/issues/7341). A declared copy count or distinct domain labels are not proof of independent recovery paths. Incomplete/unknown dependency closure yields `INDETERMINATE`.
5. **Apply hard constraints.** Reject candidates that violate known hard requirements. Mark candidates *unknown / not rankable* where required evidence is missing.
6. **Generate architecture candidates.** Explore media, redundancy, coding, physical layout, calibration, reader count and recovery domains.
7. **Invoke qualified physics only.** Each external optical, micromagnetic, electrothermal, ferroelectric or device model must be pinned to a version, configuration, input identity and applicability domain.
8. **Run Symtropy process/observation twin.** Evaluate geometry, materials, process variability, spatial defect maps, read/write registration and manufacturing constraints.
9. **Run Symthaea search.** Optimize the candidate Pareto frontier under equal evaluation budgets, held-out defects and preregistered ablations.
10. **Independent oracle.** Recheck hard constraints, MDS/code algebra, layout completeness, exact recovery, payload digests and all evidence identities independently from the optimizer.
11. **Emit decision dossier.** Report each candidate's pass/fail/unknown state, measured/simulated/source evidence, Pareto metrics, exclusions and reproducibility receipts.
12. **Lifecycle plan.** Include reader/decoder replacement, key custody, format self-description, migration windows, restore drills and end-of-life media handling.

## 4. The most important rule: unknown is not pass

The candidate report has three states for each required metric:
- **pass:** an admissible value/evidence item meets the contract;
- **fail:** admissible evidence shows the hard constraint is violated;
- **unknown:** a required value is absent, incomparable, unsupported or outside the model's applicability domain.

A candidate with unknown required values cannot win a production ranking. Research candidates may still enter an explicitly separate exploration queue. Roadmaps remain roadmaps. A synthetic error model remains synthetic. A measured sample does not establish a population distribution without sample count, collection method and uncertainty.

Do not fill missing cost, retention, read throughput, yield or error rates with estimates just to produce a ranking.

## 5. Core objective: verified information-years

The conventional comparison is capacity and cost. The compiler should eventually evaluate:

- net payload that can be restored byte-exactly;
- required lifetime and environmental exposure;
- restore-time SLA and ingest rate;
- verified TB-years per rack-volume;
- cost per verified TB-year;
- energy per successfully restored TB;
- accepted-part yield and lifecycle cost;
- independent reader and decoder portability;
- resilience to correlated damage and common-cause failure.

These KPIs need full system boundaries. Public product pages rarely report enough information to calculate them. Until system-level cost, retention, restore and reliability evidence exists, the compiler must leave them unknown.

## 6. Example contract and why it is not rankable

The example `EXAMPLE-PB-ARCHIVE-01` expresses:
- 1 PB (decimal) logical archive payload;
- 50 TB/year illustrative growth;
- append-mostly workload, 99% sequential and 1% random reads;
- 10-day full-restore target;
- 100-year retention target;
- 99% target confidence for the declared retention claim;
- ambient non-condensing storage envelope from −10°C to 40°C;
- exact SHA-256 verified recovery, two independent reader implementations;
- three copies in three independent failure domains;
- zero accepted undetected corruption.

This is a **synthetic engineering example**, not a customer request, a safety standard or a vendor requirement.

The worked candidate report keeps the system *not rankable* because the public comparison sources do not establish the full combination of century-scale retention, 10-day restore for the full archive, independent failure domains, two independent readers and comparable system-level cost/energy.

Examples:
- Glass has strong density and accelerated voxel-ageing results, but is a published research demonstrator; full production cost, restore throughput, independent reader portability and whole-system lifetime are not demonstrated by the cited paper.
- LTO-10 is a commercial archive format with known native capacity/transfer rate, but the example does not include a matched 1 PB library restore test or a substantiated 100-year guarantee.
- Seagate Mozaic 4+ is customer-qualified production storage, but 44 TB capacity alone does not establish century-scale retention or the full archive restore SLA.
- Micron 6600 ION is a high-capacity commercial QLC SSD, but package capacity and interface specifications do not establish century-scale archive retention or total-system service guarantees.

The compiler should not declare one medium the winner from those incomplete data.

## 7. Paradigm shifts

### 7.1 Compile the whole service, not the cell

The unit of optimization is the end-to-end information service: media, encoding, physical layout, reading, verification, replication, lifecycle and restore.

### 7.2 Reliability-shaped physical layout

The error-correcting code and physical shard placement must be co-designed. The Symthaea spatial-MDS reference is a first small example: a spatial-stride map survives synthetic regional damage that a row-major placement cannot. Its deliberately low 58.33% net tile efficiency is not a density improvement; it shows that robustness and density form a Pareto frontier.

### 7.3 Cross-medium failure diversity

Three copies are not three independent copies if they share the same site, power, reader, software, credentials, supply chain or environmental exposure. Use common-cause cut sets and independent observation paths to qualify failure-domain diversity.

### 7.4 Reader and decoder durability

The physical medium is only half the archive. Preserve format specification, decoder source, known-answer vectors, calibration references, cryptographic hashes and migration procedures. Test independent implementations.

### 7.5 Active evidence acquisition

When a candidate is unrankable, the compiler should identify the cheapest next experiment that can resolve the decisive unknown: full-restore benchmark, reader portability, aged sample, spatial defect mapping, costed process route, or repeated-lot yield.

### 7.1 Independence is a dependency property, not a copy count

Three physical copies may still share one site, power source, network, reader, decoder, key service, operator authority or software supply chain. The service contract therefore carries a versioned dependency-profile reference and an explicit status. The compiler consumes Mycelix #5762's multi-valued/transitive dependency closure and cut-set semantics through Symtropy #7341; it must not create a second generic dependency graph.

Keep these claims separate:
- physical copy count;
- declared failure-domain labels;
- dependency-qualified path diversity under a complete identity-bound inventory;
- empirical/statistical independence under a justified dataset and model.

Distinct IDs, hashes, reader brands or site labels do not establish empirical independence. Unknown dependency closure means no numeric independence guarantee.

## 8. Compiler benchmarks

| Benchmark | Success condition |
|---|---|
| Contract validation | Strict parse; correct units; cross-field invariants; unknowns explicit |
| Evidence classification | No roadmap/research promotion to production |
| Hard constraints | No candidate marked pass if any hard requirement fails or remains unknown |
| Technology selection | Matched service boundaries and separately reported candidate tiers |
| Code/layout | Exact payload and digest recovery on frozen known-answer and fault fixtures |
| Optimizer | Equal compute/evaluation budget; held-out fault maps; ablations |
| Physics adapters | Versioned inputs, declared applicability, model convergence and measurement discrepancy |
| Manufacturing | Material/process/geometry constraints and repeatability across lots |
| Restore | Full archive restore SLA on the actual proposed system, not inferred from media transfer headline |
| Lifecycle | Independent decoder/reader exercise and migration/restore drill |
| Economics | Cost per verified TB-year with measured/quoted inputs and explicit missing values |

A benchmark pass is scoped to its evidence class. Passing a JSON schema is not a physical qualification; passing a solver convergence test is not physical validation; passing a synthetic damage fixture is not measured reliability.

## 9. First implementation plan

**C0 — contracts:** strict schema, structural checks, cross-field invariants, missingness and negative controls.

**C1 — evidence registry:** source-pinned technology rows with evidence status, claim date, comparability limits and unsupported fields.

**C2 — deterministic compiler:** given a frozen contract and registry, produce candidate eligibility and not-rankable explanations. No optimization yet.

**C3 — design-space generation:** produce code/layout/redundancy candidates with a machine-checkable layout proof.

**C4 — physical adapters:** import actual glass or magnetic defect maps and uncertainty from Symtropy.

**C5 — optimization and experiment selection:** equal-budget optimizer tournament and active value-of-information experiments.

**C6 — physical pilot:** independent read/restore, reader replacement, repeat build/lot and full cost/energy data.

## 10. Evidence and authority boundary

This v0.1 work consists of schemas, illustrative data and a decision-report example. It is not a deployed compiler, a qualified device selector, a product recommendation or a manufacturing recipe. No device-control authority is granted. Human review and independently qualified engineering remain mandatory before any physical experiment or production decision.
