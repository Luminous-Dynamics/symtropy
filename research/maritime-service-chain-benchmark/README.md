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
