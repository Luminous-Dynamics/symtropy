# Claim-profile binding contract — design note v0.1

Status: proposed adapter contract; **not an implemented API and not a stable upstream interface**.

Parent: [ROBOT-CONFIG-CHANGE-QUAL-001 #5488](https://github.com/Luminous-Dynamics/symtropy/issues/5488).

## 1. Purpose

The configuration-change evaluator needs more than a list of strings naming dependencies. For each claim it must know which qualified owner profile defines the claim, which evidence dimensions matter, how configuration changes affect applicability, which re-verification procedure is competent, and what the evidence can actually support.

This note defines the smallest read-only binding layer needed to join those owner contracts. It is a reference map, not another source of truth.

Core rule:

```text
claim binding != claim registry
claim binding != dependency graph
claim binding != evidence store
claim binding != qualification authority
```

The binding carries exact references to owner-controlled records. It does not copy their substantive contents into a second local ontology.

## 2. Existing authorities to compose

| Concern | Owner | What #5488 may consume |
| --- | --- | --- |
| Generic engineering change semantics | [Symthaea #10543](https://github.com/Luminous-Dynamics/symthaea/issues/10543) | Exact source-to-target engineering delta, semantic impact paths and supported/unknown transformation scope |
| Claim dependency and common-mode lineage | [Symthaea #4918](https://github.com/Luminous-Dynamics/symthaea/issues/4918) | Claim-relative dependency closure, shared failure roots, independence/unknown assessment |
| Robot capability and task profile | [Symtropy #1834](https://github.com/Luminous-Dynamics/symtropy/issues/1834) | Versioned capability claim, physical article/configuration, ODD, units, task and capability envelope |
| Physical trial evidence | [Symtropy #3522](https://github.com/Luminous-Dynamics/symtropy/pull/3522) | Exact trial receipt, source configuration, instrument/calibration, raw artifact, denominator and measurement references |
| Evidence admissibility | [Symtropy #3531](https://github.com/Luminous-Dynamics/symtropy/pull/3531) | Claim-specific admissibility profile and its orthogonal evidence dimensions |
| Operational reconfiguration chronology | [Symtropy #5411](https://github.com/Luminous-Dynamics/symtropy/issues/5411) | Exact transition and source/target configuration references, from last accepted output in A to first accepted output in B |
| Endurance and maintenance | [Symtropy #3628](https://github.com/Luminous-Dynamics/symtropy/issues/3628) | Exposure, failure, maintenance, repair, restore and accepted-production history |
| Interlaboratory replication | [Symtropy #3591](https://github.com/Luminous-Dynamics/symtropy/issues/3591) | Test protocol, laboratory/article replication dimensions and metrology comparison |
| Component provenance and custody | [Mycelix #5149](https://github.com/Luminous-Dynamics/mycelix/issues/5149) | Component identity, provenance, custody, correction, revocation and scoped approvals |
| Decision-sufficient evidence request | [Symthaea #12046](https://github.com/Luminous-Dynamics/symthaea/issues/12046) | Read-only candidate evidence-request options, competent source, cost, latency and disclosure constraints |
| Robotics degradation/lifecycle | [Symthaea #4863](https://github.com/Luminous-Dynamics/symthaea/issues/4863) and [Symtropy #3628](https://github.com/Luminous-Dynamics/symtropy/issues/3628) | Article/configuration-generation, usage, wear, maintenance, repair and observed degradation context |
| Runtime evidence/certificate validity | [Symtropy #4858](https://github.com/Luminous-Dynamics/symtropy/issues/4858) and [#4117](https://github.com/Luminous-Dynamics/symtropy/issues/4117) | Observation drift, freshness, calibration/certificate horizon, state-dependent assurance validity and safe service response |
| Design for knowability | [Symthaea #6124](https://github.com/Luminous-Dynamics/symthaea/issues/6124) | Design-stage access, measurement, inspection, calibration and requalification affordances |
| Productive work vs terminal reserve | [Symthaea #11866](https://github.com/Luminous-Dynamics/symthaea/issues/11866) | Accepted productive work and independently evidenced terminal physical reserve as separate outcome dimensions |

These are ownership references, not claims that every owner has published or accepted the required machine-readable interface. Until an exact stable artifact is available, the field remains unbound and the adapter must return `Unsupported` or `UnknownImpact`; it must not infer a schema from issue prose or invent a compatibility layer.

## 3. Minimal binding record

A future binding record should contain references with exact revision identity. The names below are conceptual field names for discussion, **not a frozen JSON API**.

```text
ClaimProfileBinding
  binding_id
  binding_revision

  claim_ref
    owner
    claim_id
    profile_artifact_ref
    profile_content_digest

  capability_scope_ref
    owner_profile_ref
    physical_article_and_configuration_scope
    operating_domain_profile_ref
    task_or_work_package_profile_ref

  change_impact_ref
    change_contract_ref
    source_configuration_ref
    target_configuration_ref
    delta_ref

  dependency_ref
    claim_dependency_profile_ref
    dependency_snapshot_ref
    closure_status
    common_mode_assessment_ref

  admissibility_ref
    evidence_admissibility_profile_ref
    required_evidence_dimensions
    required_independence_dimensions

  verification_ref
    competent_test_or_inspection_procedure_ref
    acceptance_criterion_ref
    measurement_unit_and_frame_refs
    uncertainty_or_decision_margin_policy_ref

  equivalence_policy_ref
    permitted_witness_types
    witness_scope_requirements
    witness_admission_owner_ref

  validity_scope
    ODD_and_task_scope_ref
    configuration_and_article_scope_ref
    effective_interval
    invalidation_conditions

  authority_boundary
    allowed_adapter_outputs
    prohibited_actions
```

The binding is immutable once used in a scored run. Any changed owner artifact, dependency revision, evidence profile, procedure, acceptance criterion or validity scope creates a new binding identity. A display name, branch name, issue number, SKU or model family is not a content digest.

## 4. Exact-reference requirements

Every owner reference must identify:

1. the owning repository and issue/contract identifier;
2. the exact contract or profile revision;
3. the exact artifact path or immutable content location;
4. a digest of the referenced artifact bytes;
5. any transitive profile/schema revisions required to interpret it.

A GitHub issue URL alone is not a pinned contract. A mutable branch URL alone is not an immutable artifact. A SHA proves byte identity only; it does not authenticate the issuer, grant authority, or prove physical truth.

If a required dependency cannot be pinned, the affected result is `UnknownImpact` or `Unsupported`, never an optimistic preservation.

## 5. Dynamic condition and evidence-currentness boundary

A configuration-delta evaluator is not a lifecycle health estimator. A robot can retain the same as-built configuration digest while its physical condition and evidence currency change.

Examples include:
- calibration expiry or observed calibration drift;
- bearing wear/backlash, connector degradation or structural damage;
- battery capacity/health decline;
- sensor quality/freshness degradation or observation-model drift;
- actuator derating, thermal state or reduced safe torque envelope;
- changes in duty-cycle exposure, service history or environment;
- certificate/model validity horizon expiry.

The adapter should therefore carry exact references to the relevant lifecycle and runtime-currentness owner records, with effective time, observation time, configuration/article generation, calibration identity and validity horizon. It must not infer currentness from unchanged configuration identity, absence of a detector alarm, or missing telemetry.

Composition rules:
- #5488 owns **static change impact**: what a declared source-to-target delta could invalidate.
- #4863/#3628 own **physical degradation and lifecycle history**: what has changed through use, wear, maintenance and repair.
- #4858/#4117 own **observation drift and state-dependent assurance validity**: whether current observations/certificates still support an operating decision.
- #6124 owns **design-for-knowability affordances**: whether the design permits important quantities to be observed and requalified after realization/service.
- #11866 owns **productive work versus terminal reserve**: whether current useful output preserves required recoverability/future options.

A static change-impact pass cannot clear a currentness or lifecycle hold. Conversely, a drift alarm does not by itself identify which claims have become false or authorize a physical action. Missing or stale dynamic evidence remains explicit and is escalated to its owner.

## 6. Closure and uncertainty semantics

The adapter must preserve the distinction between:

- `Complete`: the relevant owner has declared the dependency snapshot complete for this claim and scope;
- `Incomplete`: one or more required dependency paths are known to be absent;
- `Unknown`: completeness has not been established or cannot be interpreted.

For v0.1, any evaluated claim in a case with an `Incomplete` or `Unknown` case-level dependency snapshot receives `UnknownImpact` or `Unsupported`, unless an exact upstream owner contract supplies an independently admitted boundary proving that the unresolved component cannot affect that claim. The current public development corpus has no such owner-backed exception mechanism.

Do not reason from absence of a known edge. In particular:

```text
not listed in the local claim catalog
!= independent of the changed component

no detected impact
!= evidence of no impact

separate component name
!= independent failure domain
```

Claim-relative dependence and common-mode classes remain owned by #4918. #5488 consumes its assessment and reports uncertainty; it must not build a second dependency graph.

## 7. Evidence and equivalence rules

A change may preserve a claim only when one of these conditions is met:

- the exact owner-defined dependency closure is complete and unchanged;
- a claim-specific qualified-equivalence witness is admitted for the changed dependencies and full claim scope;
- a separately qualified method has recomputed the claim for the exact target configuration and its result is admissible.

A proposed equivalence witness must be bound to the exact source and target component identities, revisions, interfaces, physical variables, relevant operating envelope, claims covered, method/procedure, validity interval and provenance. A shared product name, nominal dimensions, vendor assertion, software diff or synthetic fixture label is not enough.

Evidence dimensions remain orthogonal. Configuration identity, calibration, measurement uncertainty, metrology, physical origin, independence, provenance/custody, adjudication, ODD coverage and replication must not be collapsed into a universal evidence rank or confidence score. Reuse #3522/#3531 and #4918 rather than recreating those semantics.

A geometry-equivalence witness cannot preserve a process-quality claim unless that claim's own evidence profile says the witness covers the process variables and acceptance procedure. A battery-interface witness cannot preserve battery-health or endurance claims without the relevant measurements.

## 8. Requalification obligations are proposals

The adapter may emit a claim-specific proposed next action:

- applicability review;
- bench reverification;
- physical reverification;
- independent adjudication;
- exact recomputation;
- no preservation;
- unresolved/unsupported.

The output must identify the owner, procedure/profile reference, claim, changed dependency path, reason code, exact configuration/delta/evidence references and claim ceiling.

It does **not** acquire evidence, contact a laboratory, spend money, alter a robot, commission a module, clear a safety hold or authorize work. Where cost/source options are known, the adapter can pass a read-only request proposal to #12046. Any real acquisition/disclosure/action stays with the proper human or institutional authority.

## 9. Output contract and auditability

Every evaluated claim should return at least:

- exact claim/profile binding ID and digest;
- source and target configuration IDs/digests;
- exact change/delta ID and digest;
- dependency snapshot reference and closure status;
- prior evidence references and their applicability scope;
- disposition and deterministic reason code;
- changed dependency path(s);
- minimum competent resolving action and its procedure/profile reference;
- owner of that next action;
- evidence dimensions that remain unknown;
- claim ceiling and non-authority marker;
- evaluator version, input digest, oracle version and run identity.

Historical evidence is never deleted or rewritten because it is no longer applicable. It remains historical evidence with a new applicability assessment for the target configuration.

Results must be deterministic for identical canonical inputs and pinned owner revisions. Reordering JSON object keys must not change semantic identity; changing source bytes must still change the raw artifact digest. Conflicting, missing or unreconciled inputs remain explicit.

## 10. Evaluator implementation gate

Do not implement the scored Rust evaluator until all of the following exist:

- [ ] The 10543 delta input/output semantics are pinned to an immutable artifact.
- [ ] The 4918 dependency closure and common-mode result are pinned, including explicit incomplete/unknown states.
- [ ] The 1834 capability/claim identifiers and ODD/task scope are pinned for the claims under test.
- [ ] The 3522 receipt and 3531 admissibility profile revisions are pinned.
- [ ] Each disposition has a competent resolving procedure and acceptance profile, rather than a heuristic inferred from the changed component name.
- [ ] The evaluator and independent oracle are separate implementations with mutation and metamorphic controls.
- [ ] All unsupported owner-interface states have explicit negative fixtures.

Until these gates are satisfied, the right executable work is to improve the public corpus, oracle consistency, schema integrity and benchmark audit—not to build a plausible-looking evaluator that silently invents owner semantics.

## 11. Nonclaims

This note does not define or implement an upstream API, dependency engine, capability registry, evidence store, equivalence authority, safety function, robot controller, commissioning workflow, maintenance ledger, physical test, certification or release decision. It is a proposed adapter contract for coordinating the existing owners.

## 9. Robotics evidence-gap adapter to decision-sufficient acquisition

Symthaea #12046 owns decision-sufficient evidence acquisition, competent-source selection, value-of-information assumptions and the request-planning method. #5488 must not create a second test-selection optimizer.

The robotics adapter should translate claim-specific RCQ outputs into **read-only evidence-gap requests**. For each unresolved claim it may emit:
- exact claim/profile binding and source→target configuration pair;
- required evidence axis/dimension and the specific reason the previous evidence is not applicable;
- a reference to an owner-approved test/inspection procedure and acceptance predicate;
- minimum subject/configuration, ODD, calibration, frame, uncertainty and raw-artifact fields the procedure requires;
- prerequisite claims, isolation/safe-state conditions and admissible sequence constraints;
- competent source/laboratory/instrument class and its exact profile reference;
- candidate cost, elapsed time, crew burden, disclosure scope and operational exposure when supplied by the owning procedure/source;
- outcome semantics: which claim dimension the result can resolve, which claims it cannot resolve, and the accepted/failed/unknown/aborted dispositions.

The adapter must not invent a test procedure from a disposition label. RequiresBenchReverification or RequiresPhysicalReverification is not itself a work instruction. If no exact competent procedure, acceptance profile or authorized source is pinned, return Unsupported/UnknownImpact and request owner review.

### Minimum-competent-evidence policy

The planning problem is a constrained coverage problem, not run-everything and not unconstrained cost minimization:

1. Preserve all non-compensable safety, quality, legal, site and authority constraints. No expected-value or cost reduction can relax them.
2. Cover every decision-required claim/evidence dimension with at least one competent, applicable procedure, unless the intended decision is an explicit safe abstention/hold.
3. A single procedure may satisfy several claims only when its owner-published coverage contract explicitly lists those claims and the applicable configuration/ODD/acceptance profiles.
4. Preserve common-mode lineage, metrology independence, calibration, event-time and raw-evidence prerequisites. Two tests sharing one unqualified reference do not become independent corroboration because they have different names.
5. Keep candidate plans as a Pareto set over direct cost, elapsed time, total crew effort, disclosure exposure and physical risk. Do not collapse those axes to an opaque scalar.
6. After each admissible result, re-evaluate remaining claim gaps and choose the next request adaptively. No request that contacts a source, discloses data, incurs cost, interrupts equipment or initiates a physical test occurs without the competent human/organizational approval required by its owner.

For held-out evaluation, compare #12046's source-competence/dependency-aware planner with a fixed all-tests checklist, simple next-missing-field heuristic and expert human plan under the same source availability, time, cost and disclosure constraints. The robotics-specific loss should separately report critical false-clear, unnecessary retest, time to first accepted B output (#5411), accepted functional units (#3605), and maintenance/restoration context (#3628). It must not combine these into one score.

### Explicit ownership split

- #5488 computes only the finite fixture dispositions in v0.1 and, later, owner-integrated claim-specific applicability.
- #10543/#4918 own generic semantic change impact and claim-relative dependency/common-mode lineage.
- #3522/#3531/#3505 own physical trial receipts, metrology/ODD and evidence admissibility.
- #12046 owns candidate evidence requests, source competence, decision sufficiency and value-of-information planning.
- #5411 owns the operational A→B chronology through first accepted output.
- #3605 owns functional-unit and independent-acceptance semantics.

The adapter transports exact references and typed gaps. It is not a second evidence graph, procedure catalog, scheduling engine, safety authority or acquisition agent.
