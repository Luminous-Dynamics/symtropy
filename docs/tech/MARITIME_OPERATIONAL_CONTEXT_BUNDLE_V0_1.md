# Maritime Operational Context Bundle (MOCB) v0.1

**Parent owner:** [MARITIME-COMMONS-FEASIBILITY-HANDOFF-001 #4276](https://github.com/Luminous-Dynamics/symtropy/issues/4276)  
**Related profiles:** S-131 #4336, S-127 #4370, S-123 #4377, S-124 #4387, S-129 #4388, DCSA Port Call #4374, UKC physical validation #4390.

## Purpose

MOCB is a time-bounded, vessel-/route-/port-call-specific, source-reversible context projection. It assembles source-native maritime information for a particular decision cutoff without copying ownership, flattening meaning or granting operational authority.

It is not a new GIS, event store, port community system, ship operator system, booking engine, navigation display, VTS or UKC solver. It composes existing owner-native records and emits a reproducible context manifest for a deterministic Symtropy run.

## Paradigm shift: compose claims, not dashboards

A conventional dashboard often makes proximity and visual aggregation look like operational truth. MOCB instead binds every claim to source-native references, revisions, timestamps, authority and a claim ceiling.

The critical distinctions are structural:

| Published context | Does not establish |
|---|---|
| S-131 berth/terminal/shore-power feature | Current berth availability, live inventory, OPS capacity or a provider commitment |
| S-127 pilotage/VTS/service-area feature | Pilot assignment, VTS clearance or traffic authorization |
| S-123 radio-service-area feature | Measured radio link, on-board reception, warning delivery or SAR acceptance |
| S-124 warning and in-force bulletin | Receipt/display/acknowledgement by this vessel or bridge-team awareness |
| S-129 Actual Plan / Actual Update | Independent physical UKC accuracy, alarm, pilot approval or permission to transit |
| DCSA Port Call API conformance | Event truth, ETA calibration, port-call optimization or physical feasibility |
| Mycelix provider commitment reference | A new booking or authority created by this bundle |
| Symtropy deterministic replay | Physical qualification or operational permission |

The bundle must preserve a vector of statuses, not a single green/red readiness badge. `Unknown`, `Conflict`, `Stale`, `NotProven`, `SupportedContextOnly` and `ProvenInDeclaredScope` are different outcomes.

## Source and dependency pinning

The synthetic profile pins S-123, S-124, S-127, S-129 and S-131 to S-100 Edition 5.2.0, rather than silently upgrading them to the general S-100 5.2.1 publication. DCSA Port Call is pinned separately at 2.0.0. S-158 Edition 1.0.0 remains implementation/testing only. Product-specific source artifacts, catalogues, DCEG and schemas each require exact-byte retrieval, SHA-256, rights/terms and trust-state capture before native adapter claims.

The MOCB schema does not replace any native product schema. It stores owner references, versioned profile identifiers, evidence digests and source time/retrieval/validity metadata.

## Temporal and authority semantics

Keep source observation, issue/publication, transaction, retrieval, valid-from/to, event time, evaluation cutoff and provider-commitment validity distinct. A fresh retrieval cannot make stale source content current.

Every source reference carries its own owner, revision, artifact digest, terms status and authenticity status. A digest proves byte identity only; it does not prove publisher identity or legal permission.

Provider offers, holds, commitments, revocations and expiry remain Mycelix/provider-owned. Simulation is effects-disabled and cannot mutate a provider resource, berth, booking, schedule, payment or vessel state.

## v0.1 synthetic corpus

The corpus contains 30 cases (one positive context-only composition and 29 adversarial controls) covering:
- S-131 berth context versus availability;
- S-127 pilotage area versus pilot assignment / VTS clearance;
- S-123 declared coverage versus measured connectivity and delivery;
- S-124 warning lifecycle, bulletin freshness/completeness and vessel receipt;
- S-129 plan purpose, vessel/route revision, validity, full replacement and physical qualification;
- DCSA API direction, semantic validity, event/retrieval time and ETA calibration;
- provider offer/commitment/revocation/expiry and booking authority;
- synthetic run, nondeterminism, source writeback and claim promotion;
- exact product dependency and source artifact digest integrity.

Cases are specification fixtures only. The follow-on independent evaluator must compute outcomes from source inputs and mutations rather than trust the expected label. The corpus is not a native S-100 parser, DCSA sandbox pass, live provider integration or physical benchmark.

## Benchmark and comparison protocol

Compare the MOCB composition against:
1. a manual document/email hand-off;
2. a dashboard aggregation with no typed authority/time provenance;
3. point-to-point adapters without a shared run-evidence manifest;
4. an integrated central mirror;
5. a federated owner-native composition with reversible provenance and independent verification.

Report separately:
- source coverage and exact-version completeness;
- source age, retrieval lag and decision-time freshness by product;
- identity/revision/route/vessel binding errors;
- product semantic false acceptance and false rejection;
- event-time/retrieval-time confusion and out-of-order recovery;
- stale/incomplete bulletin and Actual Update replacement failures;
- unsupported/missing authority promotion;
- claim traceability and reverse-navigation to source bytes;
- p50/p95/p99 bundle assembly and convergence latency;
- recovery after feed partitions and provider revocation;
- false-feasible rate and abstention rate.

Hard gates: zero false promotion from descriptive to operational claims; zero mutation of source-owned state; zero partial S-129 replacement; zero warning-receipt inference without a device receipt; zero provider authority created by simulation; 100% claim-to-input reference traceability on the frozen corpus.

Compare performance only on matched source counts, payload sizes, update frequencies, failure schedules and integration budgets. Report composition latency and maintenance cost; do not assume federation is always faster.

## Claim ceiling

A passing v0.1 evaluator would establish only the declared synthetic composition and non-promotion invariants. It would not establish live service availability, warning truth/completeness, vessel receipt, physical UKC, navigation safety, port-call optimization superiority, provider commitment truth, legal permission or authority to operate.
