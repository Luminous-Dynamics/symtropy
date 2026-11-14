# AeroWeft — PNT / Clock Degradation to Reachable Occupancy Adapter

**Tracking issue:** [AEROWEFT-PNT-REACHABILITY-001 #5270](https://github.com/Luminous-Dynamics/symtropy/issues/5270)  
**Consumes:** Symtropy #3909/#4079/#5163/#2279/#2297; Symthaea #10775/#10826; Mycelix #435/#458/#5904  
**Status:** synthetic-first adapter contract  
**Claim ceiling:** exact transformation semantics for caller-supplied bounded position, velocity and time claims only; no receiver, aircraft, operational navigation, surveillance or flight-safety qualification.

## 1. Scope

This adapter connects existing navigation/time assurance to AeroWeft's aircraft occupancy model. It does not detect GNSS interference, fuse sensors, convert clock domains, infer independence, derive aircraft dynamics or schedule flights.

- Symtropy #3909 supplies deterministic PNT/communications degradation and recovery state.
- Symtropy #4079 supplies timing-service consequence state.
- Mycelix #435 supplies clock-domain identity and qualified clock-correlation intervals.
- Mycelix #458 supplies navigation-health/freshness/failover assessment.
- Mycelix #5904 supplies common-mode dependency and evidence-independence assessment.
- Symthaea #10775/#10826 supplies receive-side hypotheses and ground-truth/oracle provenance.
- Symtropy #5163 supplies the existing bounded-velocity reachability primitive.

The adapter owns only the aviation-specific gate and transformation from an already-qualified position-time claim to a bounded initial occupancy set.

## 2. Core rule

A position is admissible to the hard-bound reachability layer only when:
- its position claim is a deterministic hard enclosure, not a statistical prediction;
- the horizontal frame and vertical datum match the active reachability profile;
- both its sample time and evaluation epoch are intervals normalized by the qualified clock-correlation owner;
- the full age interval is non-negative and within the operation-profile maximum;
- clock-correlation lineage is current and qualified;
- the source/dependency snapshot is complete and meets the profile's independent-evidence requirements;
- the velocity envelope is applicable and supplied by the existing aircraft-model owner;
- service-state policy allows the specific primary or fallback source to be used.

This adapter does not verify the truth of the claim, digest-to-source association, signature authenticity, or the correctness of the upstream dependency graph. It preserves those identities for independent upstream qualification.

## 3. Time uncertainty propagation

The adapter receives a sample-time interval [s_min, s_max] and an evaluation-time interval [a_min, a_max], both already normalized into the same comparison time scale by Mycelix #435.

The admissible elapsed interval is:

T = [a_min - s_max, a_max - s_min]

The oldest possible observation age is a_max - s_min. Admission requires s_max <= a_min and the oldest possible age to remain within the profile limit. The position at the evaluation epoch is enclosed by:

P_now = P_sample + V × T

Both clock-correlation intervals must be qualified, current and tied to explicit scale/correlation identities. All operations use the existing reachability Interval type and outward-rounded arithmetic. The millisecond-to-second conversion must itself be expanded outward before multiplying by a velocity interval.

A missing or incompatible clock correlation, future/overlapping sample-time interval, excessive age, invalid time-scale mapping, or overflow produces a typed NOT_ASSESSABLE result. Do not clamp a negative elapsed interval to zero or substitute receipt time for observation time.

## 4. Hard-bound versus statistical claims

The adapter accepts only DeterministicHardBound claims into the conservative occupancy path.

Statistical confidence ellipses, 95%/99% predicted regions, RMS, CEP and model confidence can be useful in a separate probabilistic layer; they are not a deterministic physical enclosure and cannot shrink the hard outer occupancy set.

A statistical-only input returns NOT_ASSESSABLE_FOR_HARD_BOUND, even when the reported confidence is high.

## 5. State-to-source policy

The adapter consumes the upstream state; it does not reproduce the PNT state machine.

| Upstream state | V0 adapter behavior |
|---|---|
| NORMAL | Consider the primary deterministic hard-bound claim |
| DEGRADED | Consider only an independently qualified fallback hard-bound claim under the declared profile |
| INTEGRITY_UNCERTAIN | Do not use the primary as a hard-bound position; consider an independently qualified fallback |
| UNAVAILABLE | Consider only an independently qualified fallback |
| RECOVERING | Reject until the upstream service owner emits its domain-specific requalification and returns an admissible service state |
| INDETERMINATE | Reject; do not infer position absence or an empty occupancy region |

The fallback must have its own source, hard-bound, frame/datum, clock, freshness and independence evidence. A second message from the same GNSS/clock/receiver failure domain does not constitute an independent fallback.

The adapter's output is a mathematical initial set for #5163. It does not authorize a route, aircraft action, conflict resolution or re-entry into an operating service.

## 6. Fixture

Golden fixture: fixtures/aeroweift-pnt-reachability/tiny-pnt-time-envelope-v0.json.

All values, sources, frames, time scales, claims and velocity intervals are fictional. The ten cases cover:
1. fresh NORMAL hard bound with a non-zero clock-correlation interval;
2. exact-time positive control whose enclosure is no wider;
3. DEGRADED primary with an independently qualified fallback;
4. INTEGRITY_UNCERTAIN primary with only correlated GNSS-derived GNSS/ADS-B channels;
5. statistical-only 95% envelope;
6. stale sample interval;
7. time interval overlapping or extending beyond the assessment epoch;
8. RECOVERING state without requalification;
9. incomplete/unknown clock or source-dependency state;
10. contradictory independent witnesses.

The fixture binds expected admission/rejection reasons and outward position bounds. It is a semantic oracle, not a spoof detector or receiver benchmark.

## 7. Metrics

Report separately:
- primary/fallback admission and reason codes;
- exact input sample-time interval and derived elapsed interval;
- position enclosure width before and after time uncertainty;
- age-limit margin;
- independent-evidence-family count and dependency-snapshot status;
- hard-bound versus statistical-only rejection count;
- NOT_ASSESSABLE rate by reason;
- time from upstream degradation event to removal of the affected primary claim from the reachability input;
- stale/unknown inputs incorrectly accepted (target zero);
- deterministic replay and exact fixture agreement.

Do not report only aggregate admission rate: rejecting a statistically useful but unqualified input is preferable to falsely admitting it as a hard bound. Do not turn a large or unknown envelope into a "clear" corridor.

## 8. Baselines and falsification

Matched baselines:
1. nominal point estimate with zero time uncertainty (optimistic negative control);
2. deterministic position bound with exact sample time;
3. deterministic position bound with clock-correlation interval;
4. statistical envelope incorrectly used as hard bound (negative control; must reject);
5. shared-domain GNSS/ADS-B message count treated as two independent sources (negative control; must reject);
6. independently qualified fallback under DEGRADED/INTEGRITY_UNCERTAIN state;
7. stale, future or unqualified time input incorrectly treated as current (negative control; must reject).

Metamorphic properties:
- widening the qualified sample-time interval cannot shrink the output set;
- widening velocity bounds cannot shrink the output set;
- increasing sample age cannot shrink the output set for signed bounded velocity;
- changing only receipt time cannot make an old observation fresh;
- a shared failure-domain duplicate cannot increase independent-family count;
- an unknown dependency graph cannot be treated as independent;
- changing a statistical envelope to a hard-bound label without new evidence cannot gain admission;
- RECOVERING cannot map to NORMAL merely because the position value looks stable;
- frame/datum mismatch cannot be repaired by numeric coincidence;
- input order and non-semantic ID changes preserve semantic results;
- NOT_ASSESSABLE never becomes an empty occupancy tube or a clear path.

## 9. Acceptance targets

- 100% exact agreement with the independent rational/integer fixture oracle.
- Zero statistical-only, stale, future-time, invalid-frame, incomplete-independence or unknown-clock inputs admitted as hard bounds.
- Widening time/velocity uncertainty never reduces the output enclosure.
- Every accepted output retains source, profile, clock-correlation and dependency-snapshot identities.
- Every rejected output has a typed reason; no implicit default-to-normal.
- 100% deterministic replay on the frozen corpus.
- No actor/attack attribution, flight clearance, separation authority or operational safety claim.

## 10. Execution sequence

1. Freeze the synthetic fixture and expected transformations.
2. Implement a small adapter using the existing aeroweift-reachability Interval type.
3. Bind upstream service-state receipts from #3909/#4079.
4. Bind clock intervals and navigation-health profile from Mycelix #435/#458.
5. Consume common-mode independence status from Mycelix #5904 and oracle provenance from Symthaea #10826.
6. Pass only accepted hard-bound initial position sets into #5163.
7. Replay occupancy consequences in #2279/#2297 and candidate schedules in #5186.

**Invariant:** an unqualified position/time claim is not a small uncertainty; it is a missing or unresolved basis for a hard occupancy claim.
