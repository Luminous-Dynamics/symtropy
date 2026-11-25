# AERO Change-Impact Adapter to BASALT and Mycelix v0.1

**Tracker:** Symtropy #7352  
**Composes:** Symtropy #7319 / #7342 / PR #7383; Symthaea BASALT PR #9203 and ASSURE #2618; Mycelix AeroCommons #6345 / PR #6347  
**Status:** interface specification and synthetic mapping only  
**Claim ceiling:** schema/identity mapping contract; no production integration, observer qualification, physical qualification or airworthiness claim

## 1. Objective

Map aerospace resource and evidence identities into the existing generic BASALT state timeline and Mycelix manufacturing/build packet without creating another dependency graph, event ledger, artifact store, invalidation resolver or qualification state machine.

The aviation adapter is responsible for domain meaning: material lots, process batches, panels, specimens and serialized physical units; product configurations, layup/geometry, process recipes, machine/tool/site; instruments, calibration records, test methods, inspection/NDE methods; analysis models, solvers, meshes, boundary conditions and mission/environment profiles; typed aviation claims and verification actions.

The generic owners remain responsible for their existing primitives. This document does not assert that the referenced open research PRs are merged or production-qualified.

## 2. Exact BASALT v1 timeline mapping

The inspected BASALT timeline schema is benchmarks/basalt/schema/state-timeline.schema.json on Symthaea branch research/basalt-evidence-freshness-2026-10-30. Its version-1 contract has:

- schema_version: 1, program: "BASALT", run_id;
- initial_resources[] with resource_id and sha256:<64 lowercase hex> digest;
- ordered entries[] of resource_changed or evidence_recorded;
- resource pins containing resource ID, digest and BASALT-owned generation;
- evidence dependencies that point only to earlier evidence IDs;
- explicit dependency_coverage and unresolved_inputs.

### 2.1 Resource identity

Use the stable resource templates already listed in schemas/aero_basalt_resource_map_v0_1.json, for example:

| Aerospace resource | BASALT resource ID |
|---|---|
| Material lot | aero:material-lot:{immutable_id} |
| Process batch | aero:process-batch:{immutable_id} |
| Specimen | aero:specimen:{immutable_id} |
| Product configuration | aero:product-config:{immutable_id} |
| Calibration/metrology record | aero:metrology:{immutable_id} |
| Inspection/NDE procedure/equipment | aero:inspection:{immutable_id} |
| Analysis model/solver | aero:analysis-model:{immutable_id} |
| Analysis mesh/boundary conditions | aero:analysis-input:{immutable_id} |
| Serialized component/build packet | aero:component-packet:{immutable_id} |
| Mission/environment case | aero:environment:{immutable_id} |

The immutable ID is a stable domain identity, not a mutable display name. Keep logical resource_id separate from its content digest and BASALT generation.

### 2.2 Digest binding

BASALT requires digests in sha256:<hex> form. Mycelix ContentArtifactV1.sha256 stores the lowercase SHA-256 hex value with no prefix; the adapter may add the BASALT sha256: prefix when mapping that exact artifact commitment. It must not re-hash a label, invent a content hash, or treat a digest as evidence of physical identity or signer authority.

BASALT event_digest is recomputed according to its basalt.resource-change.v1 canonical commitment over schema version, run ID, sequence, event ID, change kind, resource ID and before/after digests. The digest binds event payload identity; it is not an issuer signature.

BASALT owns resource generation. A digest reverting from A → B → A must still have a higher generation; old evidence cannot be resurrected by matching content bytes.

### 2.3 Domain change-kind mapping

| Aerospace change | BASALT change kind |
|---|---|
| Material source/lot/grade, storage/out-time, process recipe/window, batch, machine, tooling, site, specimen, instrument/calibration, test method, NDE method/equipment, evidence correction/revocation | dependency_change |
| Product definition, geometry, layup, joint configuration, mesh or boundary-condition configuration | configuration_change |
| Analysis source or model implementation | source_change |
| Compiler, solver implementation/runtime, external analysis toolchain | toolchain_change |
| Mission/environment/thermal boundary state | environment_change |

A revocation or correction is an authority/status change even if physical measurement bytes do not change. Record it as an event against the affected calibration/evidence resource.

### 2.4 Evidence records

Use the existing check_id names in the aviation resource map:

| Evidence class | BASALT check ID | BASALT evidence kind |
|---|---|---|
| Material measurement | aero.material.measurement.v1 | other |
| Coupon/element test | aero.structure.test.v1 | test |
| Model validation | aero.analysis.validation.v1 | other |
| Inspection/NDE result | aero.inspection.result.v1 | other |
| Structural closure | aero.aircraft.structural-closure.v1 | other |
| Mission/energy/performance closure | aero.aircraft.mission-closure.v1 | other |

Each evidence_recorded entry pins every relevant resource's exact digest and generation and points to prior supporting evidence through evidence_dependencies. A test result may depend on material measurement and calibration evidence; an aircraft structural closure may depend on the test, model-validation, inspection and component/build evidence. Keep typed claim-support meaning in the existing CEB/claim-support owner, not inside a parallel aviation graph.

### 2.5 Dependency-coverage rule

The BASALT schema's dependency_coverage=complete is a producer declaration. It does not prove that every real dependency was observed. Until the Symthaea trusted observation and coverage-qualification boundaries (#9210 and #9239) are actually qualified for the relevant aerospace inputs:

- emit incomplete when any required input channel is unobserved or unresolved;
- name every known unresolved input;
- do not preserve unrelated claims merely because no edge was found;
- do not treat a signed/self-declared complete flag as proof of observer completeness.

The inspected BASALT implementation hard-codes confirmatory_eligible = false. This adapter must preserve that boundary. eligible_under_declared_coverage is not independent confirmation.

## 3. Exact Mycelix AeroCommons calibration mapping

The inspected Mycelix CalibrationValidityV1 is:

{ evidence, valid_from_unix_ms, valid_until_unix_ms, used_at_unix_ms }.

Its current validator requires the referenced calibration evidence to match the exact evidence record, requires valid_from < valid_until, and enforces:

valid_from_unix_ms <= used_at_unix_ms < valid_until_unix_ms.

The v0.2 aviation evaluator now uses the same unsigned Unix-millisecond fields and half-open interval. For each observation:

| AERO v0.2 field | Mycelix / AeroCommons field |
|---|---|
| calibration_certificates[].calibration_evidence_id | CalibrationValidityV1.evidence.id |
| fixed reference kind evidence in the mapper | CalibrationValidityV1.evidence.kind |
| valid_from_unix_ms | valid_from_unix_ms |
| valid_until_unix_ms | valid_until_unix_ms |
| observations[].acquired_at_unix_ms | used_at_unix_ms |
| certificate's instrument identity | instrument reference bound by the AeroEvidence qualification record |
| certificate_artifact_ref | Mycelix content-artifact reference/digest/byte length, not the validity record's evidence reference |

Keep the calibration evidence-record identity distinct from the certificate's file/artifact identity. The v0.2 evaluator checks time, instrument and method consistency on its synthetic contract, but it does not replace Mycelix's packet verifier or establish certificate authenticity. The imported authenticity_status remains an assertion requiring the responsible authority's validation policy.

The Mycelix PR #6347 currently binds calibration evidence to a specific inspection/test instrument and checks its measurement time. Its own stated claim ceiling is structural completeness and byte-integrity only; it does not authenticate the producer, measurement truth or engineering acceptance.

## 4. Qualification lifecycle composition

Use the existing division:

- Mycelix: manufactured-unit/build/inspection/test lineage, packet references and artifact SHA-256/byte-length integrity.
- Symtropy: aerospace identity, product configuration, material/process/analysis semantics, claim mapping and verification action descriptions.
- Symthaea BASALT: resource digest/generation freshness, direct/transitive stale-evidence closure and re-verification queue.
- Symthaea ASSURE-004: append-only Initial → Invalidated → Requalified qualification lifecycle in a supplied view.
- CEB claim-support closure: typed support, counter-evidence, completeness and non-circularity.
- Symthaea #7412: measurement uncertainty and sampling/dependence provenance.
- Symthaea #13112: decision-value/evidence-action selection, consuming qualified owner-native change-impact receipts rather than implementing another invalidation graph.

ASSURE supplied-view terminality is not global currentness. BASALT freshness is not physical truth. Mycelix artifact integrity is not publisher authentication. A signed event is not proof of observer completeness. None of these systems can grant airworthiness or authorise an aircraft release.

## 5. Benchmark: change-impact adapter tournament

Freeze the same aerospace resource/claim graph, same changes, same evidence corpus and same compute/action budget for all arms:

1. Blanket product requalification.
2. Manual engineering impact matrix.
3. Direct resource reachability without evidence descendants/typed claim closure.
4. BASALT + ASSURE generic machinery without aviation mapping.
5. BASALT + ASSURE + CEB with this aviation adapter.
6. The previous arm plus the evidence-action selector, with identical action catalogue and hard obligations.

Challenge at least:

- second source, wrong lot, storage/out-time and material-grade change;
- recipe/process-window, machine, tooling and site drift;
- geometry, layup, bond/joint and product-configuration change;
- specimen traveler correction, sampling-event replay, instrument mismatch, calibration expiry/revocation/correction;
- NDE method, instrument, coverage and probability-of-detection domain change;
- model/solver/mesh/boundary-condition and validation-domain shift;
- delayed, missing, contradictory and out-of-order source observations;
- interacting changes with a shared lot, batch, campaign, calibration or common-cause root;
- irrelevant changes where retention is allowed only with qualified complete observation coverage;
- digest A → B → A, stale evidence reuse, missing dependencies and forked qualification successors.

Report a vector, never one composite score:

- missed impacted claims and false-retained claims;
- stale-evidence acceptance;
- false invalidation;
- observer-coverage completeness and unresolved-impact rate;
- claim-support precision/recall;
- successor/fork correctness;
- unaffected evidence reuse when coverage is complete;
- test/analysis/inspection actions requested;
- lab cost, engineering hours, calendar time and compute;
- abstention rate;
- explanation completeness, exact dependency paths and deterministic replay.

Prospective research gates, not achieved results or standards:

- zero missed dependencies and zero stale-evidence acceptance on an independently authored bounded oracle corpus;
- zero positive retention if a required dependency or observer channel is missing, stale, contradictory or outside its qualified profile;
- 100% deterministic outputs for semantically equivalent input permutations;
- every retention/invalidation disposition names its dependency path and completeness basis;
- at least 20% lower verification effort only after a physical independent holdout demonstrates matched false-accept risk and identical hard-requirement coverage.

## 6. Decision-directed physical experiments

Use the existing Symthaea structural evidence-action selector (#13112) as a policy comparator and eventual consumer. It already has a bounded one-step selector, finite-horizon policy tree, hard admissibility gates, explicit outcome types and component-wise resource vectors. Its own PR states that its scenarios are synthetic and that no matched-budget tournament against all competing test-selection policies has yet been completed.

The aerospace tournament should compare:

- random selection;
- space-filling design;
- highest model uncertainty;
- prediction-disagreement;
- sensitivity-only;
- calibrated EVSI / expected loss reduction;
- distribution-free minimax regret;
- finite-horizon adaptive policy;
- experienced human engineering impact/testing plan.

The action set should include coupon tests, representative joints/elements, thermal/propulsion rig tests, calibration/inspection, NDE procedure validation and model/mesh verification. Each action has a target claim, exact subject/configuration, prerequisites, admissibility gate and resource vector (engineering hours, lab hours, facility, consumables, compute, schedule). A policy cannot spend less by dropping a mandatory test or exchanging a failed safety margin for cost.

Measure decision regret, missed unsafe states, coverage, test effort, calendar time, resource-vector Pareto quality and abstention. EVSI is permitted only where the probability model is justified and calibrated; otherwise use distribution-free bounds/minimax regret or request human review.

## 7. Implementation and release gates

1. v0.1 hierarchy evaluator remains the baseline; v0.2 closes the independent-source, intended-configuration and calibration-time representation gaps.
2. Keep the v0.2 CI-only Draft 2020-12 schema validation independent of the production Rust evaluator.
3. Obtain independent review of the expected-disposition corpus; it is currently marked NOT_INDEPENDENTLY_REVIEWED.
4. Inspect and pin the exact BASALT and Mycelix schema/field versions before generating an adapter. Current BASALT #9203 and Mycelix #6347 are open research PRs on non-main base branches, not production dependencies.
5. Implement a mapping-only adapter and fixture tests after generic owners have stable integration contracts. Do not copy BASALT generation, invalidation, ASSURE lifecycle or Mycelix artifact storage into Symtropy.
6. Keep confirmatory use disabled until trusted event capture, observer coverage, signer/issuer policy and independent evidence receipts are qualified.
7. Require physical, rights-cleared coupon/metrology evidence and independent lot/batch holdouts before any materials or structural claim is promoted.

## 8. Explicit non-claims

This mapping does not establish physical identity, measurement truth, calibration certificate authenticity, dependency observer completeness, model validity, material allowables, process capability, NDE probability of detection, structural substantiation, aircraft performance, part acceptance, airworthiness, release or certification. It is an interface and benchmark contract only.
