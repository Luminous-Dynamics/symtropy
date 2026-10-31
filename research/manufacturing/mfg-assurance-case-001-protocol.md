# MFG-ASSURANCE-CASE-001 — Executable assurance-case graph

**Version:** 0.1  
**Date:** 2026-10-31  
**Evidence tier:** T0 structural synthetic known-truth  
**Implementation:** `tools/manufacturing-assurance/assurance_case.rs`

## 1. Research question

Can claims, argument structure, evidence, assumptions, counter-evidence, defeaters, authority, scope and validity time be represented as a deterministic graph that independently checks whether a declared manufacturing assurance argument is structurally admissible?

This is different from proving the claim's truth. The objective is to make missing, stale, contradictory, out-of-scope or under-tier evidence visible and fail closed rather than allow a polished but incomplete assurance document to appear qualified.

## 2. Prior art and standards boundary

### ISO/IEC/IEEE 15026-2:2022

This international standard defines minimum structure and terminology for assurance cases and applies to their development and maintenance. It expressly does not set requirements on the quality of the case contents. Therefore a structural parser cannot turn a syntactically valid case into proof of truth. Our policy checker is an additional project profile, not a replacement for the standard.

### OMG SACM 2.2

The Structured Assurance Case Metamodel defines structured claims and their relationships to evidence and contextual information. It supports counter-arguments and counter-evidence via the `isCounter` relationship and assertion declarations including assumed, needs-support and defeated. It also defines XMI interchange compliance points. This prototype borrows the conceptual separation of assertions, evidence and counter-evidence; it is not a SACM XMI importer/exporter and makes no SACM conformance claim.

### GSN and CAE

GSN and CAE are established structured assurance notations. The current implementation uses a small text interchange profile with claim, support, assumption, defeater and evidence edges. A later adapter should map to/from GSN/SACM while preserving node identity and semantics. It must not call the format GSN or SACM merely because similar concepts exist.

### Manufacturing digital-twin frontier

A systematic review published 2026-10-28 classified 235 manufacturing quality-assurance digital-twin studies. It reported that none of the selected studies met its L5 governed adaptive QA autonomy criteria. The review identifies a fragmented assurance chain: evidence triggering an action, responsible authority, intervention scope, execution, fallback/recovery, and post-action quality verification. It also cautions that absence of reported evidence does not prove an underlying mechanism is absent, and the classification was performed by a single reviewer. This prototype turns those evidence categories into graph nodes and gates; it does not claim to prove the review's literature-level result.

## 3. Paradigm shift: claims are executable objects

Treat an assurance claim as a query over an evidence graph and an assessment context:
- what is claimed;
- for which physical/organizational scope;
- at what assessment time;
- with what minimum evidence tier;
- under which ALL/ANY argument strategy;
- using which child claims, evidence, assumptions and rebuttals;
- with which defeaters still live;
- and, for an intervention claim, under whose authority and for which action.

The result is time- and scope-specific. A claim can be conditional at one assessment time, blocked when evidence expires, defeated by counter-evidence, or become admissible again only after explicit evidence discharges a defeater. Historical evaluations should be preserved as immutable reports rather than overwritten.

## 4. Manifest semantics

The first version provides:
- one CASE header containing scope, assessment UTC, required evidence tier and top claim;
- typed CLAIM records with validity interval, minimum tier, ALL/ANY support rule, claim kind and optional action key;
- EVIDENCE records with scope, validity, tier, admission status, SHA-256 syntax and description;
- explicit ASSUMPTION and DEFEATER records;
- AUTHORITY grants with principal and action scope;
- directed typed edges SUPPORTS, COUNTERS, ASSUMES, DEFEATS, DISCHARGES and AUTHORIZES.

All validity intervals are half-open `[from,until)`. Evidence/claim/authority scope is checked on every edge. A GLOBAL claim cannot be supported by local evidence. A local claim may use matching local or GLOBAL evidence.

An accepted assumption produces CONDITIONAL rather than unconditional qualification. An open assumption blocks. A rejected assumption defeats. Active defeaters block unless there is current verified discharge evidence at or above the claim's minimum tier. Verified counter-evidence defeats; unresolved counter-evidence blocks.

Intervention claims require a current GRANTED authority record whose action scope exactly matches the action key or uses a deliberately broad `*`. Technical model evidence is not authority. This is an explicit gate rather than an inference from model confidence.

Evidence SHA-256 strings are syntax-checked only. The prototype does not authenticate artifact bytes, check digital signatures, verify who admitted the evidence, or assess semantic quality. Those need a separate evidence-receipt verifier.

## 5. Frozen fixture

The valid fixture demonstrates a CONDITIONAL top claim with:
- a qualified physics-model subclaim;
- a qualified intervention-authorization subclaim;
- evidence at T0;
- an explicit accepted ambient-boundary assumption;
- a named thermal-overheat defeater;
- independent current evidence that discharges that defeater;
- a current authority grant for the exact action `adjust_feed_rate`.

The test suite mutates the fixture to cover stale evidence, local-to-global scope leakage, missing or unverified support, open/rejected assumptions, missing/revoked/wrong-action authority, active defeaters, counter-evidence, invalid times/digests, duplicate nodes/edges, unknown references, support cycles and evidence-tier mismatch.

## 6. Benchmarks and target thresholds

These are proposed acceptance gates, not current results.

| Benchmark | T0 target | Advanced target |
|---|---|---|
| Structural negative-control detection | 100% of frozen cases | property-based/adversarial mutations with independent oracle |
| False qualification | 0 on frozen mutation panel | report confidence-bounded false-qualification rate on pre-registered random mutations |
| Determinism | identical bytes for identical manifest/build | reproducible reports across supported architectures/toolchains |
| Explainability | reason code + evidence/claim witness path for every result | minimal unsatisfied cut set and provenance trail |
| Time/scope correctness | exact boundaries and GLOBAL/local rules in all tests | bitemporal validity and multi-tenant scopes |
| Defeater semantics | active defeater blocks until explicit discharge | defeater prioritization, counterargument dependency and evidence freshness |
| Authority | reject missing/revoked/wrong-action grants | delegated authority, separation of duties, revocation receipts and recovery |
| Scale | correctness at 10k nodes | report runtime, memory and tail latency at 100k and 1M nodes |
| Interchange | document the project profile | loss-aware mapping to SACM XMI / GSN / CAE with round-trip differential tests |
| Evidence authenticity | explicit limitation in v0.1 | signed evidence receipts, content-addressed bytes and independent key/status verification |

Do not use precision/recall alone: false qualification is the dangerous error. Report false qualification, false rejection, abstention, defect localization time, and size/cost of the minimal missing-evidence cut set.

## 7. Industry comparison and differentiation

The first research rule is to acknowledge existing tools honestly. A graph of claims/evidence, defeater nodes, or structural validation is not a novel capability by itself.

### Adelard ASCE

Adelard currently documents support for GSN v3, Modular GSN, CAE schemas including ASCAD 2.0, defeater/comment management, CAE block structure validation, evidence linking, lifecycle change control, stakeholder views and reporting. This is a strong direct assurance-case authoring and management baseline.

Therefore the research should not attempt to beat ASCE at graphical notation, narrative authoring, modular GSN authoring, stakeholder views or case-report generation. The proposed difference is the *executable engineering admission profile*: bind case nodes to exact Git commit, fixture, test-run, physical dataset, calibration and artifact digests; enforce cross-repository temporal validity, scope, evidence maturity, intervention action authority and downstream requalification; then independently reproduce the disposition from a stable manifest. These must be tested empirically; do not assume ASCE cannot support a custom workflow without a product evaluation.

### NASA AdvoCATE

The CAE Tools resource describes AdvoCATE as pursuing formal foundations and automation for argument creation/assembly, integration of formal methods, pattern instantiation, hierarchical abstraction, queries/views and argument verification. Treat it as an important academic/automation comparator. Compare graph semantics and verifier output, not superficial UI features.

### Ansys medini Analyze

Ansys medini Analyze documents HAZOP/HARA/FHA/FTA/FMEA/FMEDA workflows, consistency checking, architecture/requirements traceability and links to design and requirements tools. It is a strong safety analysis and traceability comparator, though it should not be assumed to be a general GSN/SACM assurance-case graph engine without testing the relevant release and integrations.

### Fair seeded-defect comparison

Evaluate version-pinned product releases, configurations and native data formats. Apply the *same* frozen defect panel to each workflow:
- missing support edge;
- cycle in argument support;
- invalid or expired evidence;
- mismatched scope;
- incorrect evidence tier;
- counter-evidence/defeater left unresolved;
- stale source revision;
- revoked or action-mismatched authority;
- evidence digest changed without requalification;
- unsupported export/import relationship.

Measure:
1. defect detection rate and false qualification rate;
2. false rejection and abstention;
3. localization quality (which claim, edge and artifact caused the failure);
4. amount of manual reviewer intervention;
5. update latency after a dependency changes;
6. round-trip loss for native GSN/CAE/SACM representations;
7. deterministic replay and integrity of exported reports;
8. runtime/memory at 10k, 100k and 1M graph nodes.

The comparison must preserve each tool's native strengths and configuration. A custom verifier may sit beside an authoring tool; it does not need to replace it. No vendor capability should be marked absent from documentation alone—test it in a reproducible evaluation or report “not evaluated.”

### What would constitute a measurable advantage

The target is not “more features” or “more rigorous-looking diagrams.” Demonstrate lower false-qualification rates, exact detection of stale/out-of-scope/under-tier evidence, reproducible report bytes, shorter change-to-requalification latency, and better witness localization at equal or lower total reviewer/compute cost. Publish raw panel manifests and tool configurations so another lab can reproduce the comparison.


## 8. Relationship to the existing assurance modules

The executable graph is intended to become the claim layer above:
- composition structure and VVUQ;
- causal validity and intervention evaluation;
- lifecycle change impact;
- drift diagnosis;
- composition energy conservation and synchronization;
- correlation-preserving uncertainty propagation.

Future integration should bind evidence nodes to exact commit/fixture/run/artifact digests, and change-impact outputs should invalidate the graph nodes that consume revised artifacts. A graph must not accept a PR number or mutable URL as sufficient evidence identity.

## 9. Roadmap

1. T0: this dependency-free graph checker and deterministic fixture.
2. T1: add real evidence receipt verification (content hash, signer identity, trust policy, revocation, reproducible run descriptor); independent checker; property-based mutation suite.
3. T1: add loss-aware GSN/CAE/SACM XMI import/export, with round-trip testing.
4. T2: bind to public measured manufacturing datasets and traceable metrology evidence.
5. T3: independent lab assurance case review and negative-control replication.
6. T4: industrial shadow-mode assurance case for bounded human-approved action, then evaluate autonomous action authority only after evidence and governance gates.

## 10. Claim ceiling

Version 0.1 can establish structural and declared-policy consistency for this project format. It cannot prove premises, evidence authenticity, physical validity, semantic relevance, intervention safety, or GSN/SACM/ISO conformance. The graph is a machine-auditable argument skeleton, not an autonomous certification authority.
