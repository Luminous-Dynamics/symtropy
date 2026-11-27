# MCSC corridor evidence corpus and adapter protocol v0.1

Companion to [MCSC-CORRIDOR-EVIDENCE-001 #7818](https://github.com/Luminous-Dynamics/symtropy/issues/7818), under [MCSC #7686](https://github.com/Luminous-Dynamics/symtropy/issues/7686).

## Purpose

Build a rights-aware, source-qualified public evidence corpus for historical port and hinterland-corridor disruption replay. The corpus must distinguish event occurrence from when an authorized decision-maker could know the event, and must connect port-level events to receiver-accepted service without inventing missing corridor segments.

This is a source-admission and adapter contract. It is not an AIS ingestion platform, event ledger, dependency graph, port optimizer, or receiver-service evaluator.

## Evidence class and claim ceiling

Every record is assigned exactly one evidence class:

| Class | Description | Permitted claim |
|---|---|---|
| PRIMARY_EVENT | Direct official/owner event record with stable identity and permitted reuse | The source recorded the scoped event; physical truth remains bounded by source method |
| INDEPENDENT_CORROBORATION | Separate provenance family that independently supports a claim | Corroboration only for overlapping scope/time |
| OBSERVED_MOVEMENT | AIS-derived or other observed movement data with known coverage | Observed movement within stated coverage; not cargo delivery or receiver acceptance |
| PUBLISHED_AGGREGATE | Official port, corridor or sector aggregate | Aggregate context; not individual event chronology |
| MODELLED_INFERENCE | Reconstructed or estimated event, delay, capacity or route state | Modelled estimate only; never observed fact |
| SYNTHETIC_CONTROL | Generated test case | Evaluator semantics only; excluded from empirical denominators |

Do not promote a record between classes merely because multiple derivative dashboards repeat it. Preserve provenance-family relationships and source derivation.

## Required source envelope

Each empirical record must carry:

- stable source identifier and publisher/owner;
- original URL, DOI or catalog identifier;
- publication/version date and retrieval timestamp;
- license, terms and redistribution restrictions;
- raw-byte digest only when lawful and technically available;
- evidence class, provenance family and derivation/mirror references;
- subject scope (port, terminal, vessel, route, corridor, commodity, receiver);
- measurement unit, boundary, method and aggregation level;
- observation start/end, sampling cadence, known coverage and missingness;
- uncertainty, censoring, exclusions and source limitations;
- event-time fields and their precision/clock basis;
- first-available-to-authority evidence, or explicit UNKNOWN;
- exact extraction/parser version and deterministic projection digest.

Restricted AIS or commercial source payloads must not be copied into the repository. Use permitted metadata, source references, lawful aggregates and reproducible acquisition instructions. A SHA-256 digest proves byte identity, not source authenticity or truth.

## Knowledge-time semantics

Keep separate:
1. event start/end or event time;
2. source observation/record time;
3. publication time;
4. first availability to the named decision authority;
5. ingestion time;
6. validation, rights and authority completion time;
7. decision cutoff;
8. plan publication, authorization and action start;
9. port/corridor/receiver milestones.

A publication date is not an event time. An old event timestamp in a later backfill does not make the event visible to an earlier decision. If the first-available time cannot be established, historical admissibility is UNKNOWN; the record may be used for a present-day reconstruction only when its evidence class permits that use.

Time intervals must preserve timezone/offset, precision, half-open/closed boundary semantics, clock uncertainty and correction lineage. Do not silently convert date-only reports into minute-level events.

## Corridor service path

A path is a typed projection of owner references:

origin capability → supplier/material → port call/terminal/handling → inland transport → border/customs → storage → receiver inspection → receiver acceptance.

Every segment carries its own evidence and admissibility state. A path is not FEASIBLE when a mandatory segment is UNKNOWN, expired, revoked, incompatible, unreserved or unauthorized. The benchmark may compare explicitly labelled optimistic/pessimistic bounds, but must not silently fill missing segments.

Reuse Mycelix #5762 for exact common-cause dependency closure and Symtropy #7554 for receiver-service adjudication. This protocol does not define replacement schemas for either owner.

All candidate paths share the same expected-service denominator, quality specification, receiver deadline, decision cutoff and scenario realization. Preserve accepted, on-time accepted, late, rejected, unresolved, censored and unknown quantities separately.

## Source admission sequence

1. Register source and publisher before data ingestion.
2. Record license/terms and allowed storage/redistribution.
3. Determine primary vs derivative provenance and identify common upstream feeds.
4. Record coverage, sampling cadence, missingness and aggregation.
5. Bind original source identity/version and lawful byte digest where possible.
6. Extract only fields supported by the source.
7. Preserve original units and boundaries; do not combine MVA/MW or unlike measurement windows without source-supported conversion.
8. Assign evidence class and claim ceiling.
9. Independently review event-time and first-availability semantics.
10. Freeze a deterministic projection and source manifest.

## Initial public source register

| Source | Initial use | Evidence class | Explicit limitation |
|---|---|---|---|
| UNCTAD, Review of Maritime Transport 2026, published 26 Nov 2026 | Sector-level route disruption, port performance and corridor continuity context | PUBLISHED_AGGREGATE | Does not provide event-level operational timestamps for individual South African port calls |
| Existing MCSC South African port-energy evidence snapshot from CSIR/GIZ Sept 2025 reports | Bounded energy stress-test context already captured by MCSC | PUBLISHED_AGGREGATE / MODELLED_INFERENCE per source row | Pre-feasibility and reference-power estimates; not coincident metered capacity/demand or current connection capacity |
| Port/terminal/utility/corridor official incident sources | Candidate event-level corpus, subject to rights and source availability review | PRIMARY_EVENT only after source admission | No source is admitted merely because it is official; scope, timestamps, authority and reuse must be checked |
| AIS/voyage movement sources | Movement/port-call reconstruction where licensed | OBSERVED_MOVEMENT | Coverage gaps, sampling and identity resolution prevent treating absence as no event or movement as receiver delivery |

The source register is intentionally conservative. Aggregate UNCTAD figures must not be used to synthesize individual event records. Candidate South African case studies should be selected only after a rights and coverage audit; do not choose a port because its data produces a favourable result.

## Adversarial controls

The first executable corpus should include:
- late publication and backfill with old event time;
- correction/supersession preserving prior evidence vintage;
- event time changed while first-available time remains fixed;
- derivative dashboards sharing one provenance root;
- AIS coverage gap vs true absence;
- date-only vs precise timestamp conflict;
- timezone and clock-boundary ambiguity;
- overlapping outage reports with conflicting intervals;
- alternate port with unavailable inland corridor;
- feasible port/corridor but unavailable receiver storage;
- unknown cargo identity or receiver acceptance;
- license withdrawal or restricted source payload;
- source coverage change that must not look like performance change;
- incomplete mandatory path segment yielding UNKNOWN.

## Benchmark and reporting

Report:
- number of source records by evidence class and provenance family;
- lawful raw snapshot coverage and deterministic projection digests;
- percentage of rows with defensible event-time and first-available-time semantics;
- timestamp precision and uncertainty distributions;
- source coverage/missingness by port, corridor, period and event family;
- historical replay eligibility and UNKNOWN exclusions;
- alternative-path admission precision/recall against independently adjudicated cases;
- service denominator and receiver outcomes;
- event-to-admissible-plan, authorization and action-start latency;
- held-out port/corridor/event-family results.

Do not publish a single aggregate score that hides unknown evidence, coverage changes, receiver failures or hard service-floor violations. Do not set numeric SOTA targets until the empirical corpus and adjudicated denominator are frozen.

## Evaluation order

1. Rights and source register.
2. Evidence-class and provenance-family audit.
3. Event-time / knowledge-time audit.
4. Deterministic source projection and digest.
5. KITE no-lookahead replay controls.
6. Corridor-path completeness and common-cause owner composition.
7. Receiver-service outcomes.
8. Matched baseline reproduction.
9. Held-out case evaluation.

A synthetic fixture pass qualifies evaluator semantics only. A source-admission pass qualifies the declared source envelope only. Neither establishes physical truth, causal benefit, current capacity, receiver delivery or operational superiority.
