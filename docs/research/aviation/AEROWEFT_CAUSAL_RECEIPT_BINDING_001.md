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

The verifier checks unique IDs, producer sequence forks, parent existence and exact digest equality, role-to-stage mappings, required/optional edge multiplicity, parent claim disposition, causal order, parent validity at child evaluation, graph acyclicity and root closure. A conflict-screen root must consume one qualified operational-intent conformance receipt; that receipt must consume the exact current-authorization, subject-position, subject-track and active-restriction parents. This prevents a valid reachability/surveillance graph from bypassing Track N's execution-conformance state. Every surveillance-assessment receipt and any source-observation receipt used as its track parent also carry the same traffic class; a `TrafficOccupancy(class)` edge must resolve to that exact class. A qualified track requires exactly one observation parent, while a `NoReportQualifiedCoverage` receipt must have none. Conflict screening must consume exactly one occupancy receipt for each profile-required class and cannot add or substitute a class. Every receipt must be an ancestor of the declared root; otherwise it is an orphan.

## Trust boundary

Byte-integrity recomputation, signature verification, issuer authority and key revocation are not evaluated by this crate. Matching a SHA-256-shaped string is not authentication. A later Mycelix-backed evidence-custody service should supply independently verified trust facets; this verifier must not invent a second trust registry.

## Fixture

`fixtures/aeroweift-causal-receipt-binding/receipt-graph-cases-v0.tsv` lists synthetic fault scenarios for subject splicing, parent substitution, role errors, cycles, orphan receipts, temporal inversion, validity expiry, stale-parent laundering, profile-class weakening and replay.

## Hard gates

- 100% agreement with an independently authored finite graph oracle.
- Zero cross-subject or cross-authorization receipt splices accepted.
- Zero parent digest/owner/role substitutions, cycles, orphans, forks, causal-time or validity violations accepted;
- Zero receipt claims accepted outside their own validity windows or with observation-arrival/evaluation inversion.
- Zero cross-class surveillance relabelling from source observation through assessment and occupancy, duplicate required-class occupancy or contradictory report/no-report evidence accepted.
- Zero permutation-induced semantic variance, including profile-owned class-set order.
- Evidence deletion, scope narrowing, validity expiry and revision supersession cannot improve admissibility.
- Positive controls must pass; reject-all is not success.
- After correctness passes, measure 10 / 100 / 1,000 / 10,000 receipts with p50/p95/p99 latency and peak memory.

## Industry positioning

The current external landscape makes the AeroWeft claim boundary especially important:

- **ASTM F3548-21** covers strategic conflict detection, aggregate conformance to operational intents, constraint awareness, and situational awareness for nonconforming or contingent UTM operations. It explicitly does not establish the complete safety case for air collision risk or tactical conflict detection. AeroWeft should use it as a service/interoperability baseline, not claim to replace it.
- **ASTM F3442-26** is now listed by ASTM as the active Detect-and-Avoid System Performance Requirements specification. The preceding F3442-25 scope was focused on smaller uncrewed aircraft encountering crewed traffic in lower-risk airspace, including mixed cooperative/non-cooperative traffic; its architecture-neutral performance approach is a baseline for Track M and any later DAA-facing work. It does not establish every UA-to-UA, terrain, bird, or all-airspace problem as solved.
- **ASTM F3705-25** is the guide for testing DAA systems against F3442/F3442M; it is a test-method comparator for any later physical surveillance/DAA qualification programme, not something the current synthetic Track M oracle can claim to meet.
- **ASTM F3201-26** now covers software development, verification and validation practices for UAS and third-party UTM/data-service software. Its scope includes configuration management, lifecycle dependability, and safety/security; it explicitly recognizes Monte Carlo, software-in-the-loop, hardware-in-the-loop and flight testing as possible V&V techniques. This is a useful process baseline for AeroWeft's future evidence programme, but passing the current Rust workflow is not compliance with F3201-26.
- **FAA UTM Field Test (2023)** examined operational-intent priority, data correlation, identity/access management, Remote ID correlation and message security. Its final report observed that research message-signing approaches still need review for operational deployment. That reinforces the need to keep structural parent-binding separate from cryptographic custody and operational issuer authority.
- **NASA's 2024 CMSA safety assessment** used Monte Carlo encounters with conforming and contingent aircraft and evaluated modeled mid-air-collision probability per flight hour. It is a stronger safety-analysis comparator than simple alert-accuracy metrics; AeroWeft should eventually reproduce the *methodological rigor* with independent scenario distributions, not copy a headline rate without matching assumptions.
- **NASA's 2024 UAM DCB work** compares vertiport-only, waypoint and speed-adjusted airborne-delay strategies. It shows how sequential resource balancing can leave delay and capacity inefficiency; AeroWeft's Track D should compare transparent implementations under identical demand and hard constraints.
- **NASA's 2025 in-flight strategic path-planning flight evaluation** used two helicopters with up to 330 virtual aircraft. It found tighter flow-corridor volumes reduced available maneuvering degrees of freedom in some maneuvers. AeroWeft should therefore benchmark retained maneuverability/recoverability and future options, not simply maximize corridor utilization.
- **EU Regulation 2021/664**, Article 10(10), requires continuous re-checking of active U-space authorizations against new dynamic restrictions and relevant manned-traffic information, with updates or withdrawal as circumstances require. The EASA U-space AMC conformance rules, including the 5% time-outside threshold and 5-second/99% alerting target, are scoped U-space guidance—not universal aviation thresholds.

References:
- ASTM F3548-21: https://store.astm.org/f3548-21.html
- ASTM F3442-26: https://store.astm.org/f3442-26.html
- ASTM F3705-25: https://store.astm.org/f3705-25.html
- ASTM F3201-26: https://store.astm.org/f3201-26.html
- FAA UTM Field Test: https://www.faa.gov/uas/research_development/traffic_management/field_test
- FAA UTM Field Test Final Report: https://www.faa.gov/uas/advanced_operations/traffic_management/UFT-Final-Report.pdf
- NASA CMSA safety assessment (2024): https://ntrs.nasa.gov/citations/20240015613
- NASA UAM DCB algorithms (2024): https://ntrs.nasa.gov/citations/20240007944
- NASA in-flight strategic path planning evaluation (2025): https://ntrs.nasa.gov/citations/20250003243
- EUR-Lex Regulation (EU) 2021/664 consolidated 22 February 2026: https://eur-lex.europa.eu/eli/reg_impl/2021/664/2026-02-22/eng
- EASA Easy Access Rules for U-space: https://www.easa.europa.eu/en/document-library/easy-access-rules/online-publications/easy-access-rules-u-space


## Follow-on workstreams

- **Track Q — trusted evidence custody:** [#5351](https://github.com/Luminous-Dynamics/symtropy/issues/5351). Reuses Mycelix's sensor provenance, pinned trust roots, signed evidence, revocation and EvidenceLease owners; no parallel trust stack.
- **Track R — conflict-management performance evidence envelope:** [#5354](https://github.com/Luminous-Dynamics/symtropy/issues/5354). Defines domain-scoped performance evidence and statistical reporting inspired by NASA's RCMP research; it does not set separation minima or authorize operations.

Track P itself remains a structural receipt-graph verifier, not cryptographic custody or conflict-management performance qualification.

## Claim ceiling

A passing result proves only that the supplied synthetic receipt headers form a structurally coherent subject-matched causal graph under the declared edge policy. It does not prove source-byte integrity, signer authenticity, issuer authority, revocation state, aircraft physical truth, sensor calibration, traffic observability beyond the supplied owner claims, flight safety, regulatory compliance, separation, clearance, dispatch authority or certification. The digest fields are syntax/value matched by this crate; the actual bytes and cryptographic envelopes are not evaluated.
