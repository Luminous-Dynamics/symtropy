# AeroWeft — evidence scope-join oracle

**Tracking issue:** [AEROWEFT-SCOPE-JOIN-001 #5318](https://github.com/Luminous-Dynamics/symtropy/issues/5318)  
**Parent composition layer:** [Track O #5304 / PR #5305](https://github.com/Luminous-Dynamics/symtropy/pull/5305)  
**Status:** draft synthetic scope-join contract  
**Claim ceiling:** exact scope and point-in-time joining of already-qualified owner claims only.

## 1. The missing seam

Track O checks owner disposition, receipt presence and a shared time-scale/profile/frame/datum identity. That is necessary but not sufficient to establish that the claims are about one operation.

The current Track O model can express identical profile/frame/time identities while omitting flight-instance, aircraft-instance, operation, intent, authorization ID and authorization revision. Its intent claim is reduced to a state enum plus a non-empty receipt reference. Track N already retains richer authorization lineage, but Track O does not yet join on that lineage. Similarly, Track M's surveillance claims are cell/class/window-specific, while Track O consumes class-level composition evidence.

The risk is not necessarily a bad subsystem. It is a **bad join of individually plausible subsystem outputs**:
- PNT for flight A + reachability for flight B;
- current aircraft + prior flight instance;
- same intent ID + superseded authorization revision;
- conforming result for authorization revision 7 + current candidate revision 8;
- surveillance coverage for cell A + request for cell B;
- a late-arriving earlier observation inserted into a historical decision;
- a fresh receipt timestamp mistaken for a fresh physical observation.

This crate checks the seam only. It does not claim that a receipt string authenticates the fields next to it. Production integration requires owner receipts to bind these scope fields cryptographically or through a separately verified immutable attestation.

## 2. Paradigm shift — scoped evidence graph

Replace a status tuple with an explicit join graph:

`evaluation -> operation -> flight instance -> aircraft instance -> intent -> authorization revision`

Each operation-scoped claim must bind to the same exact graph. Surveillance evidence is a different node type: it binds to an evaluation and an exact airspace/cell/traffic-class/time query, not to the identity of one aircraft.

This avoids two opposite errors:
1. under-binding: joining the wrong flight or authority revision because the profile and coordinate frame happen to match;
2. over-binding: pretending a surveillance coverage claim is intrinsically about one particular aircraft rather than the spatial/time query it actually supports.

The output `SCOPE_MATCHED_FOR_COMPOSITION` means only that the requested keys and time scopes match. Track O still evaluates upstream dispositions. It is not `SAFE`, `CLEAR`, `AUTHORIZED`, `DISPATCHABLE` or `CONFLICT_FREE`.

## 3. Operation-scoped evidence

PNT, reachability and intent owner claims must match:
- evaluation ID;
- operation ID;
- flight-instance ID;
- aircraft-instance ID;
- intent ID;
- operator and issuing-USSP identity, intent ID/revision, authorization ID/revision and exact authorization-artifact digest;
- profile revision;
- time-scale ID;
- horizontal frame;
- vertical datum.

PNT applicability is checked against the evaluation support epoch. Reachability and intent applicability are checked against the requested operation window. This crate does not calculate age limits, convert clocks or infer validity policy; the upstream owner must supply the correct applicability envelope and typed receipt.

## 4. Surveillance query-scoped evidence

Each requested surveillance query specifies:
- query ID and evaluation ID;
- airspace/partition ID;
- spatial cell/volume ID;
- traffic class;
- exact half-open query interval.

Exactly one owner-produced coverage claim must match each query. A claim must preserve its coverage receipt, dependency snapshot, identity and declared validity interval. Duplicate, missing or extra queries are rejected. Qualified no-report coverage is allowed as a correctly scoped query, but this crate does not reinterpret no-report as empty occupancy; Track M's unresolved-possible-occupancy state remains authoritative.

## 5. Bitemporal cutoff contract

The join keeps these times separate:
- **event/measurement time:** when the described observation occurred;
- **validity interval:** when the evidence is declared applicable;
- **evaluation time:** when the owner produced its claim;
- **first-received time:** when the composition service could know it;
- **decision cutoff:** the frozen as-of time.

For the finite synthetic contract:
- owner evaluation, event and receipt times must not exceed the decision cutoff;
- owner evaluation cannot occur after first receipt;
- an observed event cannot occur after first receipt;
- all time fields are already normalized into the declared time scale;
- the claim validity window must cover the owner-specific requested interval;
- receipt-after-cutoff is excluded even when its embedded event timestamp is old.

Freshness limits and clock normalization remain with Track L/Mycelix #435 and the generic point-in-time evidence owners (#4705/#5171). This crate does not decide whether an old-but-pre-cutoff observation is fresh enough; it prevents that old observation from being silently refreshed by receipt time.

## 6. Owner boundaries

| Owner | Owns | This crate checks |
|---|---|---|
| Track L #5270 | PNT, time uncertainty, source freshness and deterministic hard-bound eligibility | same operation/authority key, identity, applicability and as-of ordering |
| Reachability #5163 | aircraft model and bounded occupancy | same operation/authority key, identity and requested operation interval |
| Track N #5284 | authorization and intent conformance | exact intent/authorization/revision join and applicability interval |
| Track M #5275 | traffic class, coverage, observation and common-mode semantics | exact evaluation/query/cell/class/window join |
| #4705 / #5171 | bitemporal point-in-time evidence, cutoff leakage and evidence commitments | consume the resulting time and receipt provenance; do not build a new temporal database |
| Track O #5304 | non-compensatory evidence disposition composition | receive only scope-matched owner envelopes |
| #2279/#2297 | conflict geometry and conflict screening | unchanged; no geometry is implemented here |

## 7. Frozen synthetic benchmark

Fixture: `fixtures/aeroweift-scope-join/scope-join-v0.json`.

It contains 32 expected decisions: one fully matched positive control; cross-flight, cross-intent and cross-revision negatives; mismatched profile/frame; future, late and reversed event/receipt times; incomplete validity; wrong cell/class/window; missing/duplicate/extra query; missing lineage; qualified no-report; and input-permutation replay.

The dependency-free Rust test campaign contains direct constructor-level adversarial tests. Fixture and test inputs are fictional; no sensor, aircraft, authorization or operational result is represented.

## 8. Hard gates and benchmark targets

| Metric | Acceptance target |
|---|---:|
| Cross-operation/flight/aircraft false joins | **0** |
| Wrong intent/authorization/revision joins | **0** |
| Future-event or post-cutoff receipt leakage | **0** |
| Event-after-receipt / evaluation-after-receipt ordering errors admitted | **0** |
| Wrong cell/class/query-window substitutions | **0** |
| Missing, duplicate or extra required coverage queries admitted | **0** |
| Positive scope-match controls | **At least 1** |
| Exact input-order replay | **100%** |
| Receipt/source snapshot traceability | **100%** |
| Widening or removing evidence improves admissibility | **0** |

Scale experiments should report p50/p95/p99 runtime, memory and allocation count at 10, 100, 1,000 and 10,000 operation/query envelopes. Do not publish performance comparisons until measured on pinned hardware and with the same scope cardinality.

Do not collapse correctness, latency, false rejection and coverage into one weighted score. A fast wrong-flight join is a hard failure.

## 9. Industry comparison and competitive opportunity

- EU Regulation 2021/664 Article 10(10) requires continuous checking of active authorisations against new dynamic restrictions and relevant manned-traffic information, with updates or withdrawals as circumstances require. Article 10(11) requires a unique authorisation number capable of identifying the authorised flight, operator and issuing U-space service provider. This reinforces the value of exact authorization identity and revision-aware scope binding. [Consolidated regulation](https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX%3A02021R0664-20260222)
- EASA's May 2024 Easy Access Rules provide specific U-space AMC/GM conformance guidance, including a 5% time-outside confirmation guide and a 5-second/99% alerting performance guide. These are scoped AMC/GM benchmarks, not universal thresholds for every jurisdiction, aircraft or operation. [EASA Easy Access Rules](https://www.easa.europa.eu/en/document-library/easy-access-rules/online-publications/easy-access-rules-u-space)
- The FAA UTM Pilot Program Phase 2 report models operation intent transitioning to nonconforming and contingent states. Its demonstration timeout is not a universal operational parameter. [FAA UPP Phase 2 report](https://www.faa.gov/sites/faa.gov/files/uas/research_development/traffic_management/utm_pilot_program/FY20_UPP2_Final_Report.pdf)
- NASA's 2025 strategic path-planning flight evaluation reports that tighter corridor volumes left fewer manoeuvring degrees of freedom in some tests. That argues for preserving exact corridor/query scope and authority revision when replaying a decision, not merely matching an aircraft profile. [NASA/TP-20250003243](https://ntrs.nasa.gov/citations/20250003243)
- A 2026 SACA review identifies persistent fragmentation and weak interoperability between ATM, UTM and AAM frameworks, alongside inconsistent uncertainty modelling. AeroWeft can test one concrete interoperability boundary: whether claims are joined to the exact same operation and time-space query. It does not replace DAA, conflict resolution or separation algorithms. [2026 SACA review](https://doi.org/10.1016/j.paerosci.2026.101241)

Matched baseline comparisons:
1. profile/frame-only join (negative control);
2. operation/authorization-key join without cutoff;
3. event-time-only filter;
4. receipt-time-only filter;
5. exact operation key + owner-validity + as-known-at join;
6. exact query-scoped surveillance join.

Compare false joins, false exclusions, cutoff leakage, revision correctness, replay invariance and runtime independently. Do not claim superiority over an industry implementation without its exact source, data and matched assumptions.

## 10. Claim ceiling and next integration

This prototype only checks the declared synthetic metadata contract. The current upstream owner structs do not all yet expose every flight/authorization key at the required boundary. A positive synthetic test therefore demonstrates the contract, not that the production evidence chain already supplies those bindings.

The next integration step is to extend each owner receipt schema (or a separately verified adapter attestation) to bind the exact scope keys and validity/cutoff fields. Track O can then require a verified scope-join receipt before emitting its existing conflict-screen eligibility state.

No live airspace, live sensors, operational UTM/ATM, dispatch, DAA, flight control, separation or certification is in scope.
