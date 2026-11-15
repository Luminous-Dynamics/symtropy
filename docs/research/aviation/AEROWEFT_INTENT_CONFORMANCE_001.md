# AeroWeft — Operational-Intent Conformance and Contingency Oracle

**Tracking issue:** [AEROWEFT-INTENT-CONFORMANCE-001 #5284](https://github.com/Luminous-Dynamics/symtropy/issues/5284)  
**Consumes:** Symtropy #2279/#2297/#5163/#5186/#5270/#5275; Mycelix #435/#458/#5904; Sol-Atlas #189/#825; Symthaea #6249  
**Status:** synthetic-first conformance and deviation accounting contract  
**Claim ceiling:** finite authorization/observation conformance semantics only; no UTM/U-space service, flight authority, operational safety assessment or aircraft-control capability.

## 1. Why this layer is missing

AeroWeft now has:
- planned 4D conflict checks and numeric conservatism (#2279/#2297);
- bounded kinematic occupancy (#5163);
- candidate schedules/resources (#5186);
- PNT/time-qualified occupancy inputs (#5270);
- cooperative/non-cooperative observability and unknown-occupancy semantics (#5275).

But a plan and a qualified track are not yet an execution-conformance claim. We need a replayable answer to: **what was authorized, what did the evidence show, which revision was current, and how much deviation is definite versus only possible?**

This is distinct from planning conflicts and sensor coverage. It consumes their outputs and never creates or authorizes an intent.

## 2. Standards and evidence context

### EASA U-space
The EASA AMC/GM for Regulation (EU) 2021/664 describes authorizations as 4D volumes with uncertainty considered, a profile-specific deviation-threshold volume, and confirmation of non-conformance when outside the authorized volume including deviation thresholds for more than 5% of the time, validated over time. The 95%/5% language is specific U-space guidance, not a universal aircraft safety threshold or deterministic integrity guarantee. It must remain an explicit profile input with jurisdiction and version.

### Dynamic restrictions
The consolidated EU Regulation 2021/664 (22 February 2026) requires continuous checking of existing flight authorizations against new dynamic airspace restrictions/limitations and relevant manned-traffic information, with updates or withdrawal as circumstances require.

### FAA UTM field evidence
The FAA UTM Pilot Program Phase 2 report describes conformance against shared intent volumes and performance-authorization boundaries, non-conforming and contingent states, and sharing revised contingency volumes based on available position/heading/speed/wind evidence. A 30-second timeout was used in that demonstration; it is not a universal value.

### Sources
- EASA Easy Access Rules for U-space: https://www.easa.europa.eu/en/document-library/easy-access-rules/online-publications/easy-access-rules-u-space
- EU Regulation 2021/664 consolidated 22 February 2026: https://eur-lex.europa.eu/eli/reg_impl/2021/664/2026-02-22/eng
- FAA UTM Pilot Program Phase 2 Final Report: https://www.faa.gov/sites/faa.gov/files/uas/research_development/traffic_management/utm_pilot_program/FY20_UPP2_Final_Report.pdf
- FAA UTM Field Test Final Report: https://www.faa.gov/uas/advanced_operations/traffic_management/UFT-Final-Report.pdf
- ISO/TR 23310:2025: https://www.iso.org/standard/87392.html

## 3. Paradigm shift

Do not define conformance as a nominal point-to-line distance with a convenient epsilon. Compare a versioned observed 4D set with an exact authorization and deviation contract.

For each observation support window, classify its observed 4D enclosure as:
- definitely inside the authorized volume including the explicit deviation expansion;
- definitely outside that volume;
- ambiguous because the observation set overlaps both sides;
- unassessable due to stale/unqualified evidence.

Return lower and upper deviation exposure:
- definite outside duration;
- possible outside duration;
- definitely inside duration;
- unassessable duration;
- unobserved duration.

Never use a midpoint to convert an ambiguous interval into conformance. Never count unobserved time as observed-inside time.

## 4. Ownership — compose, do not duplicate

| Concern | Existing owner | This work consumes |
|---|---|---|
| Planned trajectory geometry, restrictions, 4D conflict semantics | Symtropy #2279/#2297 | Authorized 4D cells and dynamic-restriction revisions |
| Kinematic reachable occupancy | Symtropy #5163 | Qualified projections only for contingency context |
| Candidate schedules/resources | Symtropy #5186 | Intent/authorization references; no new scheduler |
| PNT/time integrity and interval lineage | Symtropy #5270, Mycelix #435/#458 | Qualified time and navigation claims |
| Surveillance/track qualification and no-report semantics | Symtropy #5275 | Qualified observations or typed missing/unknown state |
| PNT and timing degradation scenarios | Symtropy #3909/#4079 | Upstream degraded/recovery states |
| Common-mode dependence | Mycelix #5904 | Exact dependency snapshot |
| Airspace context and source applicability | Sol-Atlas #189/#825 | Versioned restriction and authority context |
| Aircraft physical applicability | Symthaea #6249 | No physics is implemented here |

This track owns only the conformance decision contract, finite deviation-time accounting, revision/withdrawal response, contingency-state classification, synthetic fixture and independent oracle.

## 5. V0 input/output contract

Inputs bind:
- immutable operational-intent ID/revision/digest;
- authorization ID, issuer reference, activation/expiry, withdrawal status and current revision;
- authorized 4D bounds and horizontal frame/vertical datum;
- explicit spatial/time deviation expansion;
- policy ID, maximum observation age, contact-loss timeout, evaluation window and maximum definite-outside exposure fraction;
- observation support window, sample-time interval, receipt time, bounded position, source identity, frame/datum, PNT qualification and surveillance qualification reference;
- current dynamic-restriction snapshot/revision and effective interval;
- optional #5163 reachable projection reference; no unqualified future volume may be invented.

All clocks are already normalized by Mycelix #435. The oracle only compares intervals in the same declared scale.

Output one of:
- CONFORMING;
- POTENTIALLY_NONCONFORMING;
- NONCONFORMING;
- CONTINGENT;
- REAUTHORIZATION_REQUIRED;
- NOT_ASSESSABLE.

Keep these states distinct. A stale observation is not conforming; loss of observation beyond a profile-specific timeout is contingent; changed authority scope requires reauthorization; an ambiguous boundary crossing is potential rather than definite non-conformance.

## 6. Deviation accounting

The fixture uses finite integer ticks and integer spatial coordinates. The production interface must retain explicit units, reference frame and datum.

For each non-overlapping observation support interval:
- sample age is calculated from observation time, not receipt/authoring time;
- stale or unqualified intervals are unassessable;
- a fully contained observed 4D set contributes to definitely-inside time;
- a wholly disjoint set contributes to definite and possible outside time;
- a set that overlaps but is not contained contributes to possible outside time only;
- uncovered intervals remain unobserved.

The profile's maximum definite-outside fraction is an explicit integer ratio/ppm field. The definite-outside fraction is a lower bound over the full evaluation window; the possible-outside fraction is an upper bound over that same window and adds all unobserved and unassessable time. The evaluator never hard-codes 5%. A deployment profile may bind EASA U-space guidance only within its legal scope and version. A missing-data interval can therefore never make the upper-bound exposure look better.

## 7. Golden fixture

File: fixtures/aeroweift-conformance/tiny-intent-conformance-v0.json.

Fourteen synthetic cases:
1. fully inside nominal authorized 4D volume;
2. outside nominal but inside explicit deviation expansion;
3. boundary-straddling observation interval;
4. definitely outside beyond the configured confirmation threshold;
5. isolated outside observation below threshold;
6. stale observation with a fresh receipt time;
7. last observation beyond explicit contact-loss timeout;
8. PNT/time integrity loss;
9. superseded authorization revision;
10. expired or withdrawn authorization;
11. effective dynamic restriction intersecting active intent;
12. dynamic restriction outside the intent (positive control);
13. exact spatial boundary and half-open time edge;
14. input-order permutation and deterministic replay control.

All geometry, authorization, source IDs, time ticks and aircraft observations are fictional.

## 8. Baselines and metrics

Matched policies on identical input:
1. point estimate / zero uncertainty (optimistic negative control);
2. interval conformance with profile-defined deviation expansion;
3. definite/possible duration accounting;
4. stale or unknown observations incorrectly accepted as conforming (must fail);
5. fixed 30-second contact timeout (demonstration-specific comparator only);
6. revision-aware authorization and dynamic-restriction revalidation;
7. complete conformance oracle.

Report:
- definite, possible, unobserved and unassessable duration separately;
- false-conforming and false-nonconforming counts separately;
- time to definite-deviation detection and possible-deviation escalation;
- stale-track and contact-loss propagation delay;
- expired/withdrawn/superseded authorization rejection;
- dynamic-restriction intersection and non-intersection outcomes;
- status transitions/reasons and receipt-lineage completeness;
- runtime and memory by observation interval count, authorization cells and active aircraft.

## 9. Falsification and acceptance

- 100% agreement with an independent integer/rational oracle on the frozen fixture.
- Zero false-conforming results for stale, unknown, revoked, expired, superseded, definitely-outside or lost-track negative controls.
- Zero definite non-conformance declarations from an ambiguous interval without satisfying configured confirmation.
- Definite outside duration is never greater than possible outside duration.
- Unobserved time is never counted as observed-inside time.
- Widening position/time uncertainty cannot shrink possible deviation exposure.
- Receipt time cannot repair stale observation time.
- New intersecting dynamic restrictions require revalidation; a non-intersecting restriction does not.
- Every outcome preserves exact intent, authorization, policy, clock, observation and restriction revisions.
- Deterministic replay and permutation invariance for semantically equivalent event order.
- No flight authorization, operational alerting service, tactical instruction, dispatch or control output.

## 10. Execution order

1. Freeze the fourteen-case synthetic fixture and explicit policy parameters.
2. Implement a finite 4D-box/deviation and duration-accounting oracle independent of the planner.
3. Consume observation qualification from #5275 and PNT/time lineage from #5270/#435/#458.
4. Consume authorization geometry and dynamic-restriction revisions from #2279/#2297/Sol-Atlas #189/#825.
5. Attach #5163 reachable projections only when independently qualified.
6. Compare candidate intents and schedule outcomes with #5186; do not generate or authorize an intent.
7. Emit exact replay receipts and add the track to the benchmark suite.

**Core invariant:** a flight plan is not proof of execution; a stale or uncertain observation is not proof of conformance; an old authorization is not permission after its scope changes.
