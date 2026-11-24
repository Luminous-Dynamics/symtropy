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
