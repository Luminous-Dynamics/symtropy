# AeroWeft — Evidence-Linked Operational Assurance Case

**Tracking issue:** #5647  
**Composes:** Tracks P–V and Mycelix evidence-custody/currentness owners  
**Status:** synthetic-first structural assurance-argument oracle  
**Claim ceiling:** review readiness only; never an operational permission or safety determination.

## Research question

Can an independent reviewer reconstruct why a bounded operational claim is supported by the supplied evidence, identify every relevant hazard and mitigation, see which assumptions and defeaters remain open, and distinguish an external authority decision from an engineering tool's output?

This is distinct from evidence custody. Tracks P–V determine the identity, qualification, applicability, performance and provenance of specific evidence. Track W checks how an assurance argument uses that evidence to support a claim in a named operational context.

## Paradigm shift: from green checks to a defeasible assurance argument

Replace the claim "All tests passed, therefore the operation is safe" with an explicit, bounded argument: the top-level claim is supported by scoped subclaims and evidence; assumptions and context are visible; hazards are addressed by named mitigations; evidence dependencies that remain uncertain or contradicted are exposed; known defeaters are open or discharged; and the responsible authority's independent decisions remain distinct.

A green CI workflow is evidence about a checked software artifact. It is not an aviation safety argument. A complete-looking argument graph is not proof that its claims are true. A review-ready case is not an accepted safety case.

## Industry/process comparators

- **ISO/IEC/IEEE 15026-2:2022** specifies assurance-case structure and terminology and applies to development and maintenance of assurance cases. It expressly does not prescribe the quality of the substantive argument content. Track W provides a testable aviation profile and blocker logic; it does not claim the ISO standard's requirements have been satisfied.
- **SAE ARP4754B**, revised 20 December 2023, provides civil aircraft/system development guidance, including requirements validation, design verification, development assurance and configuration management. It points to ARP4761A/ED-135 for safety-assessment processes and separate software, hardware and security lifecycle guidance.
- **SAE ARP4761A**, revised 20 December 2023, provides systematic safety-assessment process guidance for civil aircraft, systems and equipment. Track W can preserve traceability to owner-produced hazard/safety-assessment evidence; it cannot replace those analyses.
- **NASA-STD-7009B** establishes model-and-simulation practices and says acceptance criteria are defined by the project/program and approved by delegated technical authority. **NASA-HDBK-7009B**, dated 3 February 2026 and listed active, is implementation guidance for that standard. These qualify model credibility and intended use, not operational permission.
- **FAA safety-case material for SWIM** defines the safety/assurance case as a structured argument linking specific safety/dependability claims to evidence and explicit reasoning. It is a useful aviation-domain conceptual comparator, not a current certification checklist.

## Ownership and composition

Track W owns:
- aviation assurance-case node/edge vocabulary and frozen case profile;
- claim-to-hazard/mitigation/evidence traceability;
- explicit assumption and defeater lifecycle;
- operation-window and effective/knowledge cutoff consistency;
- non-compensatory blocker reporting;
- declared common-cause/corroboration accounting;
- external authority-decision presence/scope/chronology checks;
- deterministic graph evaluation and synthetic adversarial corpus.

It composes:
- Track P #5337 causal receipt binding;
- Track Q #5351 authenticated custody via Mycelix provenance/trust owners;
- Track R #5354 performance envelopes;
- Track S #5359 multi-subject traffic scope;
- Track T #5362 model credibility and validation envelope;
- Track U #5371 bitemporal change impact and requalification;
- Track V #5394 experiment selection (planning output is never promoted to realized operational evidence);
- Symtropy #4487 multi-fidelity admission, #4366 semantic equivalence, #3749 analytic numerical oracles, #4446 differential fuzzing;
- Mycelix #5981 common-cause discovery.

Track W does not duplicate those algorithms, trust roots, lease algebra, event stores, currentness services, aircraft dynamics, conflict screening or authority systems.

## V0 scope and graph model

The frozen case manifest binds:
- case/campaign and exact policy digest;
- operation, pseudonymous aircraft, flight and authorization identity/revision;
- intent, intended-use/risk profile and operating-domain digests;
- time scale, horizontal frame and vertical datum;
- proposed operation interval;
- effective-time and knowledge-time assessment cutoffs;
- frozen required-hazard manifest;
- required authority role/identity pairs;
- exact verifier build.

Node kinds are explicit: top-level claim, subclaim, strategy, context, assumption, hazard, mitigation, evidence, defeater and external authority decision.

Typed edges are explicit:
- claim/strategy decomposition;
- evidence support;
- context qualification;
- assumption use;
- mitigation-to-hazard association;
- defeater discharge/challenge;
- required external authority decision.

Every node has a unique identity, exact case scope, owner role, revision/digest and a validity interval. Every evidence node points to an exact source receipt and source-assessment digest. Node validity and accepted authority-decision validity must cover the entire proposed operation interval, not merely the review timestamp.

## Hard structural rules

1. The root must be a top-level claim decomposed through at least one named strategy.
2. Each strategy resolves into at least one subclaim; each claim has direct qualified evidence or an explicit decomposition path.
3. Evidence scope and effective/knowledge cutoffs match the exact case.
4. Required hazards must all be present; extra/unregistered hazards are rejected rather than silently omitted.
5. Every required hazard is explicitly linked from a claim or strategy using an AddressesHazard relation and has at least one mitigation; every mitigation must attach to a registered hazard and have qualified evidence.
6. Contradicted, unsupported or indeterminate claims/mitigations remain negative/unknown; no aggregate score can conceal them.
7. Every assumption is explicit and owned. It is supported by qualified evidence, accepted through a current policy-pinned authority decision, open, expired or unknown. Unknown is indeterminate; open is an unresolved argument.
8. Every defeater explicitly challenges a claim or mitigation. It is open, unknown, discharged by qualified evidence, or accepted/not-applicable under a current matching authority decision.
9. Evidence with declared common-cause groups is not double-counted. Missing correlation identity gives no independence credit.
10. Generic software CI and Track V experiment plans are not realized operational evidence.
11. Each required authority role/identity has exactly one supplied external decision record. Pending, rejected, revoked, expired, late-recorded or insufficiently long-lived decisions block review readiness.
12. Effective time and knowledge time remain separate. A later-recorded decision cannot be used in an earlier as-known view.
13. The argument graph is acyclic, duplicate-free and root-connected. Orphan nodes are rejected.
14. A Track U stale, contradicted or indeterminate dependency cannot be silently represented as qualified current evidence.
15. A Track V plan or expected information-gain estimate is planning evidence only; it cannot be promoted to a realized validation result.
16. The result's permission_granted field is always false.

## Non-compensatory result model

The primary status distinguishes:
- REVIEW_READY_ONLY
- INCOMPLETE_ARGUMENT
- BLOCKED_BY_OPEN_DEFEATER
- BLOCKED_BY_UNSUPPORTED_OR_STALE_EVIDENCE
- BLOCKED_BY_SCOPE_MISMATCH
- INDETERMINATE

The result also returns all stable per-node blockers, outstanding authority roles and independent evidence-group counts per claim. There is no scalar safety score, weighted average or implied authorization.

## Frozen synthetic adversarial corpus

The suite covers positive review-ready control, missing root strategy, strategy without subclaim, claim without support/decomposition, missing and unmitigated hazards, unregistered hazards, hazards not linked to claims/strategies, mitigation without evidence/without hazard attachment, unsupported/indeterminate mitigation, open or unchallenged defeaters, missing discharge evidence, unknown/open assumptions, stale/requalification-required/contradicted evidence, generic CI and Track V planning misuse, wrong aircraft scope, wrong cutoffs, common-cause duplicates, missing/pending/rejected/late authority decisions, authority validity shorter than the operation, node validity shorter than the operation, duplicate identities/edges, missing endpoints, cycles, disconnected nodes and node/edge permutation invariance.

All test IDs, digests, times, authority names and evidence references are fictional.

## Hard targets

Prospective acceptance targets:
- 100% agreement with an independently authored finite argument oracle;
- zero open or unknown mandatory defeaters in REVIEW_READY_ONLY;
- zero required hazards without mitigation;
- zero extra hazards outside the frozen hazard manifest silently accepted;
- zero mitigation nodes not attached to registered hazards;
- zero assumptions with unknown/open/expired status silently accepted;
- zero authority decisions used outside their effective, knowledge or operation-validity interval;
- zero wrong-case/aircraft/flight/authorization/profile evidence accepted;
- zero correlated or unknown-dependence observations counted as independent;
- zero Track V planning or generic CI output promoted as operational evidence;
- 100% deterministic replay under node/edge permutation;
- 100% explicit separation between review readiness and operational permission;
- 10 / 100 / 1,000 / 10,000 nodes and edges with p50/p95/p99 runtime, memory, blocker count and path depth after correctness gates pass.

## Claim ceiling

A passing result establishes only that the supplied synthetic assurance argument is structurally complete enough for independent review under the frozen profile and supplied owner states. It does not calculate or accept risk, establish physical truth, demonstrate aircraft safety/separation, establish airworthiness, satisfy certification, grant clearance, or authorize any operation. The applicant and competent authority retain their respective responsibilities.
