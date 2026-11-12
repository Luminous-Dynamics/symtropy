# EV battery thermal-propagation evidence qualification v0.1

Status: evidence-contract and synthetic audit scaffold. Not a safety case, test protocol, fire model, certification or product approval.

## Purpose

Audit whether thermal-runaway initiation/propagation evidence actually supports a claim for the exact battery product, level and application stated. The audit is read-only. It does not prescribe or authorize destructive tests.

## Regulatory and standards boundary

- UN Regulation No. 100 as consolidated in EU Regulation 2026/1695 (17 August 2026) includes EV REESS thermal-propagation provisions in 6.15 and Annex 9K. The regulation describes a physical-test route and a systematic risk-management route where physical testing is not suitable. The physical-test outcomes, trigger-method selection and REESS-versus-vehicle test-level decision are specific to its scope and approval process. The software does not determine legal applicability or type approval.
  - Official text: https://eur-lex.europa.eu/eli/reg/2026/1695/oj/eng/pdf
- IEC 62660-3:2022 addresses safety of EV propulsion cells and cell blocks; cell-level evidence is not automatically REESS/vehicle evidence.
  - Official IEC catalogue: https://webstore.iec.ch/en/publication/65084
- SAE J2464_202108 describes RESS abuse-test methods across cell/module/pack levels but does not establish pass/fail criteria.
  - Official SAE catalogue: https://saemobilus.sae.org/standards/j2464_202108-electric-hybrid-electric-vehicle-rechargeable-energy-storage-system-ress-safety-abuse-testing
- UL 9540A Edition 6 is a stationary battery energy storage system test method, including installation-level testing. It is not an EV traction REESS approval test.
  - Official listing: https://www.shopulstandards.com/ProductDetail.aspx?productId=UL9540A_6_S_20260313

The applicable licensed normative text, jurisdiction, effective date, vehicle category and technical-service decisions remain under the competent authority. This contract stores those references; it is not a standards interpreter.

## Research gap

The 2026 review “Evaluation of Lithium-Ion Battery Thermal Runaway Initiation and Propagation: A Cross-Standard, Multilevel Evidence-Chain and Equivalence-Assessment Framework” (Batteries 12(9), 375) describes non-equivalence among test applications, initiation methods, boundary conditions and acceptance criteria. It motivates an evidence chain from cell trigger/source terms through module/RPU propagation, pack/system consequences and product-change review:
https://www.mdpi.com/2313-0105/12/9/375

The research thesis is that test reports must be linked through physical identities and bounded equivalence, not merely grouped by chemistry name or a vendor's product label.

## Contract

### Test record

Each record binds:
- application domain: EV traction, stationary storage or industrial;
- test level: cell, module/RPU, REESS/pack, vehicle or stationary installation;
- exact tested article, revision, chemistry, format, capacity, SOC and relevant condition;
- six domain fingerprints: cell source terms, topology, thermal pathways, protection, enclosure, and exposure;
- standard/protocol/revision and trigger method;
- trigger-achieved state, observation window, propagation observation and reported result;
- evidence source digest, class, admission, laboratory/authority and metrology reference.

Result states are deliberately separate: PASS, FAIL, TRIGGER_NOT_ACHIEVED, NO_PROPAGATION_WITHIN_WINDOW, RISK_ANALYSIS_SUBMITTED, INVALID, INCOMPLETE, NOT_MEASURED and UNKNOWN. The audit never converts a failed trigger into a successful propagation-suppression result.

### Claim and evidence transfer

A claim names its target application and target level and identifies its candidate product/article. In v0.1 a direct claim can only be considered when the source evidence has the same application and test level. A cell test cannot directly support a pack/vehicle claim; stationary-storage evidence cannot directly support EV traction evidence.

For reuse across product revisions, the claimant must provide six typed comparisons:
1. cell source terms;
2. topology;
3. thermal pathways;
4. protection functions;
5. enclosure and vent boundary;
6. occupant/personnel exposure.

Each comparison is SAME_WITH_EVIDENCE, DIFFERENT, UNKNOWN or NOT_APPLICABLE_WITH_RATIONALE and binds both the exact source and candidate fingerprint for that domain. Every SAME_WITH_EVIDENCE record references admitted evidence and a rationale; stale fingerprint pairs are rejected. DIFFERENT or UNKNOWN blocks automatic inheritance. NOT_APPLICABLE requires a non-empty rationale.

The strongest positive disposition is CANDIDATE_FOR_HUMAN_REVIEW. It is never a test PASS, safe/unsafe judgment, compliance decision or approval. Even a source-reported PASS remains scoped to that exact test, article, protocol, result and authority.

## Benchmark families

The synthetic known-answer corpus must cover:
- exact article and same-level/same-application evidence;
- cell-to-vehicle or module-to-REESS level promotion;
- stationary UL 9540A-style evidence proposed for EV traction;
- changed cell source-term fingerprints;
- changed module/pack topology, cooling, venting, protection or exposure;
- unknown or missing equivalence domains;
- failed or unachieved trigger, invalid/incomplete test and short observation windows;
- risk-analysis evidence distinguished from a physical test;
- missing admission, calibration/metrology and source digests;
- input-order permutations and stale evidence references.

## Baselines

Compare the typed audit with:
1. an unstructured document-binder checklist;
2. exact-identity-only matching;
3. a single-axis configuration match;
4. six-domain evidence review;
5. an independently adjudicated synthetic reference set.

Metrics:
- zero false-green evidence transfers on hard negative controls;
- zero direct cross-level or cross-application promotion;
- 100% detection of injected DIFFERENT/UNKNOWN domains;
- 100% preservation of trigger-not-achieved, no-propagation, invalid, incomplete and unknown outcomes;
- 100% deterministic reports under 100 input permutations;
- complete identity, source digest, protocol and evidence-admission closure for every candidate review disposition.

These are software-contract targets, not physical safety-performance targets.

## Ownership boundaries

Composes Symtropy #2114, #4833, #4840, #4848 and Symthaea #11271, #11485, #11421. It does not duplicate their models or databases. The benchmark never performs hazard analysis, runs laboratory experiments, buys or handles cells, operates equipment, or takes BMS/vehicle action.

## Claim ceiling

A passing software audit demonstrates only evidence identity, scope, completeness, result semantics and transfer-gate correctness on the declared corpus. It does not establish that any battery will or will not enter thermal runaway, that propagation is contained, that occupants/personnel are protected, or that any regulation or standard is satisfied.
