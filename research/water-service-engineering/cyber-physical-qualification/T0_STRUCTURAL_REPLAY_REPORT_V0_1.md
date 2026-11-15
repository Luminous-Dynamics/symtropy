# T0 structural replay report — water cyber-physical causal v0.1

**Status:** structural read-back only; no native Rust, full JSON Schema engine, CI or physical execution claim.

## Tested source revision

- Branch: `research/water-cyber-physical-causal-v0-1`
- Exact artifact revision read back: `327ae5b14a195d2e9777c7579ad05acadf3c728e`
- Protocol blob: `a60a36da0ddbf23581ae7aad2587c70246beb1be`
- Scenario fixture blob: `9e6491498afca62514382323de0bc4ea9ed189dd`
- Schema blob: `699ee26abc34c593ca8e99cdd8d3e5abd2d840ba`

## Method

Each of the three committed files was fetched back from the exact branch revision and parsed as JSON where applicable. A separate structural replay checked the protocol-to-artifact crosslinks and the fixture's registered invariants. This was a bounded JavaScript structural check executed during repository authoring; it is not a committed validator and does not replace the schema engine or Rust evaluator.

## Results

**17/17 structural checks passed; 0 failed.**

1. Fixture JSON parses.
2. Schema JSON parses.
3. Protocol names the event, integrity, restoration and matched-experiment owners.
4. Schema declares Draft 2020-12 and a closed root object.
5. Exactly twelve synthetic cases are present and cardinality is fixed in the schema.
6. Exactly five separately typed dataset lanes are present and cardinality is fixed in the schema.
7. Scenario IDs are unique.
8. Matched-pair IDs are unique.
9. The four-arm nominal/disturbed × baseline/evidence-gated design is declared.
10. Each declared contrast changes exactly one registered factor.
11. Every case uses the same registered four-arm matrix.
12. Future covariates are unavailable in every case.
13. Evaluator truth is explicitly marked evaluator-only.
14. Scenario seeds are unique.
15. All frozen input digest references have the required SHA-256 string syntax.
16. Every dataset lane states exclusions and an admission state.
17. Restoration dispositions use the registered finite vocabulary.

## Not performed / not claimed

- Full Draft 2020-12 meta-schema or instance validation by a dedicated JSON Schema implementation.
- Rust compilation, unit tests, formatting or strict Clippy.
- Exact-head GitHub Actions qualification.
- EPANET/WNTR solver replay, cyber range, HIL or physical testing.
- Measurement, hydraulic, service-restoration, potability or utility performance.
- Cryptographic authentication or recomputation of the placeholder digests; they are synthetic identifiers used to exercise contract shape.

The result qualifies only fixture structure and a limited set of preregistered semantic invariants. The claim ceiling remains **synthetic contract design**.
