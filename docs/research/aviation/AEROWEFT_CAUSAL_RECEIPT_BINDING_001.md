# AeroWeft — Subject-Scoped Causal Receipt Binding

**Tracking issue:** #5337  
**Parent work:** Track O / #5304 / PR #5305  
**Status:** synthetic-first structural verifier  
**Claim ceiling:** receipt-scope and causal-graph integrity only; no cryptographic attestation, flight authority or operational safety claim.

## Research question

Can AeroWeft prove that every downstream claim was derived from the exact upstream evidence, aircraft, flight, intent revision, authorization revision, profile, scenario and evaluation window it names?

Track O composes already-qualified owner dispositions. Its V0 identity tuple includes time scale, profile, frame and vertical datum. That is necessary but not sufficient to bind a complete flight decision. Two receipts may share those generic values while referring to different aircraft, flights, intent revisions or authorizations. Opaque receipt IDs also do not prove that a child consumed the exact parent payload presented later to composition.

Track P adds a structural subject-and-causality layer. It does not repeat PNT qualification, surveillance observability, aircraft dynamics, reachability propagation, conflict geometry, conformance logic or scheduling.

## Paradigm shift: evidence as a typed causal graph

Evidence is not a bag of IDs attached after a decision. It is a directed acyclic graph of scoped claims and exact consumed-parent edges. Each edge binds the exact parent receipt ID, parent payload-digest string, semantic role and expected owner/stage.

The graph is structurally bound only when every relevant node shares the exact subject scope, each parent edge resolves to the exact payload, required roles exist, the graph is acyclic, and causal/validity ordering is coherent. The positive result is `CAUSALLY_BOUND_ONLY`, not cryptographic trust or operational readiness.

## Mandatory subject scope

Every receipt must match the exact scenario and manifest digest, operation, pseudonymous aircraft, flight, intent ID/revision/digest, authorization ID/revision/digest, profile ID/revision, time scale, horizontal frame, vertical datum, and evaluation window. Matching generic profile metadata is not enough to join flights or authority revisions.

## Typed receipt graph

Receipts bind stage, schema/revision, producer/revision, producer stream and sequence, payload digest string, subject scope, typed disposition, evaluated/produced times, validity interval, and consumed-parent edges. Observation receipts additionally retain observed-at and received-at separately. Profile policy receipts bind the required traffic-class set.

The verifier checks unique IDs, producer sequence forks, parent existence and exact digest equality, role-to-stage mappings, required/optional edge multiplicity, parent claim disposition, causal order, parent validity at child evaluation, graph acyclicity and root closure. Every receipt must be an ancestor of the declared root; otherwise it is an orphan.

## Trust boundary

Byte-integrity recomputation, signature verification, issuer authority and key revocation are not evaluated by this crate. Matching a SHA-256-shaped string is not authentication. A later Mycelix-backed evidence-custody service should supply independently verified trust facets; this verifier must not invent a second trust registry.

## Fixture

`fixtures/aeroweift-causal-receipt-binding/receipt-graph-cases-v0.tsv` lists synthetic fault scenarios for subject splicing, parent substitution, role errors, cycles, orphan receipts, temporal inversion, validity expiry, stale-parent laundering, profile-class weakening and replay.

## Hard gates

- 100% agreement with an independently authored finite graph oracle.
- Zero cross-subject or cross-authorization receipt splices accepted.
- Zero parent digest/owner/role substitutions, cycles, orphans, forks, causal-time or validity violations accepted.
- Zero permutation-induced semantic variance.
- Evidence deletion, scope narrowing, validity expiry and revision supersession cannot improve admissibility.
- Positive controls must pass; reject-all is not success.
- After correctness passes, measure 10 / 100 / 1,000 / 10,000 receipts with p50/p95/p99 latency and peak memory.

## Industry positioning

EASA's Easy Access Rules for U-space remain listed as the May 2024 edition; EASA separately published the Easy Access Rules for Unmanned Aircraft Systems revision of 30 June 2026. The consolidated Regulation (EU) 2021/664 text available on EUR-Lex is dated 22 February 2026. Article 10(10) requires continuous checking of active authorizations against new dynamic restrictions and relevant manned-traffic information, with update or withdrawal as circumstances require. EASA's U-space AMC/GM includes service-specific conformance thresholds and alert targets; those must not be generalized to all aviation operations.

ASTM F3548-21 explicitly addresses strategic conflict detection, aggregate conformance to operational intents, constraint awareness and situational awareness for nonconforming/contingent operations, with UTM service interoperability and APIs. NASA has also published a Monte-Carlo-based safety assessment of conformance monitoring and strategic deconfliction for mixed conforming/contingent UTM traffic. Track P does not replace these systems or standards. Its narrower contribution is independently replayable binding of the exact subject, authority revision and consumed-parent evidence across layers.

References:
- EASA Easy Access Rules for U-space: https://www.easa.europa.eu/en/document-library/easy-access-rules/online-publications/easy-access-rules-u-space
- EASA drones and air mobility rule editions (30 June 2026 UAS revision): https://www.easa.europa.eu/en/domains/civil-drones/easy-access-rules
- EUR-Lex Regulation (EU) 2021/664 consolidated to 22 February 2026: https://eur-lex.europa.eu/eli/reg_impl/2021/664/2026-02-22/eng
- ASTM F3548-21 scope: https://store.astm.org/f3548-21.html
- FAA UTM Concept of Operations v2.0: https://www.faa.gov/researchdevelopment/trafficmanagement/utm-concept-operations-version-20-utm-conops-v20
- NASA, Safety Assessment of Conformance Monitoring for Situational Awareness in UTM Operations (2024): https://ntrs.nasa.gov/citations/20240015613

## Claim ceiling

A passing result proves only that supplied synthetic receipt headers form a structurally coherent subject-matched causal graph under the declared edge policy. It does not prove source-byte integrity, signer authenticity, issuer authority, revocation state, aircraft physical truth, flight safety, compliance, separation, clearance or certification.
