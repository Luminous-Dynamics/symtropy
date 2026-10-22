# Seafood Chain Integrity Benchmark

A synthetic-first Rust benchmark for seafood lot lineage, source evidence, transformation mass balance, custody events, temperature-log coverage, raw-ready-to-eat control evidence, species-label consistency, and declared sustainability-claim scope.

This is **evidence-integrity triage**, not a food-safety certification, a replacement for HACCP, a legal-compliance determination, or an automated release-to-consumer system. `eligible_for_human_review` means only that the synthetic fixture passes the listed evidence gates. An authorized operator must still make the actual disposition under applicable law and the facility's validated procedures.

## Run

From the repository root:

    cargo test --manifest-path tools/seafood-chain-integrity-bench/Cargo.toml
    cargo run --manifest-path tools/seafood-chain-integrity-bench/Cargo.toml -- --corpus docs/tech/SEAFOOD_CHAIN_INTEGRITY_CORPUS_V0_1.json --source-commit "$(git rev-parse HEAD)" --receipt target/seafood-chain-integrity-receipt.json

The receipt binds to the exact corpus bytes with SHA-256 and records the supplied source commit. The frozen scenarios are synthetic and deliberately small; their expected results are regression controls, not empirical safety or ecological-performance claims.

## Fail-closed rules in V0.1

- The target lot must trace through a non-cyclic parent graph to a wild-capture or aquaculture source with a non-empty source reference and verified source evidence.
- Parent-input quantities are explicit. Transformation output cannot exceed declared inputs, and source material cannot be allocated to child lots beyond the source lot's declared mass. Fish-material grams exclude rice, sauces, glazing, and other non-fish additions; those would require separately modeled inputs.
- Lot and event identifiers must be unique; event references, per-lot event chronology, source/transform events, and a final serving event are checked.
- Temperature limits and monitoring gaps are supplied as a scenario's declared, product-specific control profile, not asserted as universal legal thresholds. The logger must have verified calibration, continuous-enough samples across the entire target lineage's event window, and no sample beyond the declared maximum.
- Hazard-review evidence must be verified. Raw-ready-to-eat scenarios require verified parasite-control evidence or a documented not-applicable basis. The tool does not decide which controls apply to a species or jurisdiction.
- The declared species must match verified identity and the target lot. Missing or conflicting identity evidence holds the lot for review.
- Sustainability claims are assessed separately from food disposition. A claim is only supported for the declared scope in this fixture when its evidence status is verified, its reference is present, its scope covers the target lineage, and it is not expired. This does not independently authenticate a certifier or prove an ecological outcome.

## Benchmark targets

- Zero eligible-for-human-review scenarios with missing legal-source evidence, broken or cyclic lineage, overallocated source mass, unverified species identity, absent required raw-RTE control evidence, incomplete temperature coverage, or temperature excursions beyond the declared limit.
- Zero unsupported sustainability claims reported as substantiated.
- No non-detection, missing data, or unverified record is silently converted to a positive assertion.
- Deterministic receipt for identical corpus bytes and the same source-commit string.
- Explicit synthetic negative controls for legal source evidence, cold-chain excursion, species substitution, missing parasite-control evidence, and unsupported sustainability wording.

## Research and interoperability anchors

- FAO, *Advancing end-to-end traceability along capture fisheries and aquaculture value chains* (2023): https://www.fao.org/family-farming/detail/en/c/1640146/
- GS1, *Global Traceability Standard*: Critical Tracking Events and Key Data Elements: https://www.gs1.org/standards/gs1-global-traceability-standard/current-standard
- FAO/WHO, *Code of Practice for Fish and Fishery Products* (Codex CXC 52-2003): https://www.fao.org/fao-who-codexalimentarius/codex-texts/codes-of-practice/en/
- U.S. FDA, *Fish and Fishery Products Hazards and Controls Guidance* (June 2022): https://www.fda.gov/food/seafood-guidance-documents-regulatory-information/fish-and-fishery-products-hazards-and-controls
- South Africa DFFE, fisheries forms and permit resources: https://www.dffe.gov.za/fisheries-forms-and-documents

These sources inform the concepts, not a single jurisdiction-independent rulebook. The schema must be extended with jurisdiction, species, product form, intended use, and validated local control profiles before any real deployment.

## Planned system connections

- **Symtropy:** supply-chain simulation, cold-chain disruptions, alternative logistics, inventory loss and recall-reach scenarios.
- **Symthaea:** evidence contradiction detection, uncertainty-preserving supplier and ecological assessments, and sensor/anomaly analysis; model output is advisory and source-linked.
- **Mycelix:** consented, permissioned provenance attestations; producer-held data; auditable dispute, correction, and recall workflows.
- **Sol-Atlas:** source regions, landing sites, farms, ports, protected areas and route context with explicit uncertainty and provenance.

These are integration directions, not implemented claims. V0.1 does not ingest live vessel tracks, marine-stock data, laboratory results, certification registries, or jurisdictional permits.
