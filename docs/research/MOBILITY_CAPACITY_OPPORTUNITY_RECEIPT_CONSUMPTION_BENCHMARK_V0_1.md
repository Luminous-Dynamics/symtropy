# Mobility Capacity Opportunity Receipt Consumption Benchmark v0.1

**Programme:** [SUSTAINABLE-MOBILITY-SERVICE-CAPACITY-001 #6909](https://github.com/Luminous-Dynamics/symtropy/issues/6909)  
**Consumer:** [SUSTAINABLE-MOBILITY-SERVICE-QUALITY-001 #6921](https://github.com/Luminous-Dynamics/symtropy/issues/6921)  
**Composition issue:** [MOBILITY-CAPACITY-OPPORTUNITY-RECEIPTS-001 #6999](https://github.com/Luminous-Dynamics/symtropy/issues/6999)  
**Capacity producer:** [PR #7004](https://github.com/Luminous-Dynamics/symtropy/pull/7004)  
**Consumer branch:** \`research/mobility-capacity-opportunity-consumer-v0-1\`  
**Research cut-off:** 23 November 2026  
**Status:** linked consumer source, strict schema, synthetic fixture, CLI and workflow integration committed; exact-head CI pending.

## 1. The composition failure

The first service-quality evaluator accepted a user-authored list of aerial opportunities containing aircraft IDs, start times, mission durations and turnarounds. It checked overlap and compared the number of opportunities with an aggregate capacity budget.

That was still insufficient. A fixture author could invent a plausible aircraft slot that the upstream battery/charger event simulation had never produced. Aggregate throughput and a plausible calendar do not prove that a specific pack was available at that time.

The source event engine now exports one \`CapacityServiceOpportunity\` for each mission it actually schedules and completes within the horizon. This consumer closes the missing provenance link:

\`\`\`text
capacity architecture input
    -> deterministic battery/charger event replay
    -> per-aircraft mission opportunity receipts
    -> unique receipt/configuration digest
    -> payload qualification
    -> request admission against receipt times
\`\`\`

The new linked mode does not accept a user-supplied aerial schedule. It takes only:
- the request stream;
- exact source architecture ID;
- each aerial capacity opportunity ID;
- maximum payload and a payload-qualification evidence envelope;
- separate fallback opportunities and flight/weather/permission evidence.

Aircraft ID, mission sequence, start, duration and turnaround are resolved from the capacity evaluator's own output.

## 2. Why a sequence of receipts is stronger than cycles/hour

The Dock-shaped capacity reference has a modelled steady-state charge replenishment ceiling of:

\[
2 \times \frac{3600}{1620} \approx 4.44
\]

mission battery-restoration cycles per hour under the simplified 27-minute charge assumption. This is a replenishment upper bound, not a customer-delivery rate.

The event model's finite eight-hour run produces 38 completed missions for its one-aircraft/four-pack synthetic profile. The corresponding receipts are exact start windows, not interchangeable quotas. Its early mission starts are 0, 345, 690 and 1,035 seconds; after the initial spare inventory is depleted, the sequence includes longer charge-dependent gaps such as 1,965 and 3,585 seconds. The event model does not create later slots when no pack is restored.

The new composition invariant is:

> Every aerial service assignment must reference one receipt emitted by the exact source architecture replay, and its assigned start must equal that receipt's start.

A downstream request scheduler cannot create a new mission window, shift an existing one earlier, or substitute a different aircraft.

## 3. Linked evaluator contract

The new source entry point is \`evaluate_linked_service_quality(input, capacity_input)\`.

At evaluation time it:

1. Re-runs the bounded #6909 event simulation from the supplied, versioned architecture input.
2. Requires the exact benchmark/schema/horizon and exactly one selected architecture.
3. Recomputes a SHA-256 digest of the serialized capacity report, selected architecture configuration and canonical service-opportunity array.
4. Verifies the embedded opportunity digest from the capacity result.
5. Checks receipt IDs are unique, mission sequences start at zero and are contiguous per aircraft, mission completion stays in the horizon, and the next mission begins no earlier than previous mission completion plus turnaround.
6. Resolves each qualification ID to one source receipt; unknown or duplicated IDs fail closed.
7. Requires the payload evidence to bind the exact opportunity ID, aircraft, site, site configuration and architecture-configuration digest, be non-unknown, have valid observation time, and cover mission plus turnaround.
8. Derives the aerial service slots from the receipts; only payload limits/evidence are supplied by the consumer.
9. Runs both FCFS and EDF with identical request, fallback, weather, permission and maintenance inputs.

The source capacity report is recomputed from the input rather than trusted as a user-supplied schedule. The SHA-256 values bind the deterministic serialization of those inputs and outputs; they do not authenticate who produced them or establish physical truth.

## 4. Frozen matched request fixture

The linked synthetic fixture uses one aircraft and seven of the 38 capacity receipts, with twelve requests:
- two equal-arrival requests whose deadlines differ, making FCFS-vs-EDF ordering observable;
- one later aerial request;
- a fallback-only request;
- a weather-no-go request;
- a maintenance-overlap request;
- a request served by ground fallback after a no-go interval;
- a request with unknown aerial permission and a ground fallback;
- one post-acceptance-cutoff arrival;
- one request after the selected aerial receipt set is exhausted;
- one request that starts before its deadline but completes late.

Seven receipts are payload-qualified; the remaining 31 remain real capacity opportunities but are not eligible for this synthetic request stream.

Expected policy outcomes encoded in the workflow:
- FCFS: 12 offered, 11 accepted, 1 rejected, 6 on-time, 2 late and 3 unserved.
- EDF: 12 offered, 11 accepted, 1 rejected, 7 on-time, 1 late and 3 unserved.

The same 38-opportunity source report and the same seven qualification receipts are used by both policies. The only policy difference is request ordering; FCFS orders equal-arrival requests by stable request ID and does not use deadline, while EDF uses deadline then arrival.

These are frozen synthetic model outcomes. They are not measured delivery results or claims about DJI operations.

## 5. Negative controls

The linked module includes focused tests for:
- a fabricated opportunity ID not present in the event report;
- payload evidence claiming a different aircraft;
- a stale/mismatched architecture-configuration digest;
- evidence expiring before mission plus turnaround completes;
- replaying one payload evidence reference for multiple opportunity IDs;
- requesting an architecture absent from the source input;
- mismatched request/source horizon;
- permutation invariance for requests, qualifications, fallback slots, flight windows, blocked intervals and policies;
- changing the architecture input while retaining stale qualification evidence;
- true FCFS arrival-order tie behavior versus EDF deadline ordering.

The workflow additionally checks that the linked input's aerial qualification objects contain only receipt ID, payload bound and evidence—no manually supplied \`aircraft_id\`, \`start_seconds\`, \`mission_duration_seconds\` or \`turnaround_seconds\`. It asserts the resulting start time matches the source receipt for every aerial outcome.

## 6. SOTA and industry comparisons

### Commercial automated docking

DJI Dock 4 publishes swap below 25 seconds, landing-to-next-takeoff turnaround below 45 seconds, three maximum stored batteries, two parallel charging channels and a 27-minute charge from 15% to 95% at 25°C. DJI says the swap and turnaround numbers come from controlled test environments and are reference values. The linked benchmark deliberately uses these as a profile shape, not a reproduction of DJI hardware or fleet operations.

The useful research question is not whether the receipt consumer is faster than a 25-second exchange. It is whether the complete service chain produces the correct time-bounded opportunities after pack inventory, charge queues and channel headroom are accounted for.

Source: [DJI Dock 4 official specifications](https://enterprise.dji.com/dock-4/specs).

### Direct routing and resupply research

- Zhao, Zhang and Coelho, *Transportation Research Part E* 214 (October 2026), article 105074, model dynamic request acceptance and truck-drone routing under random arrivals and changing wind, using approximate dynamic programming. Reproduce their native one-truck/one-drone benchmark before comparing the multi-pack event model. [DOI 10.1016/j.tre.2026.105074](https://doi.org/10.1016/j.tre.2026.105074).
- Liang, Chen and Du, *European Journal of Operational Research* (online 15 October 2026), formulate multi-visit pickup/delivery with release dates, time windows, lockers and drone resupply, using an MIBLP and ALNS. This motivates linking mission opportunities to inventory/replenishment rather than treating slots as independent jobs. [DOI 10.1016/j.ejor.2026.10.020](https://doi.org/10.1016/j.ejor.2026.10.020).
- Shi, Hong and Li, *Transportation Research Part E* (September 2026), study a truck-drone system with a mobile drone-station trailer and a routing-guided ALNS. Mobile station trailers are therefore an existing published architecture; novelty cannot be claimed for a trailer alone. [DOI 10.1016/j.tre.2026.104975](https://doi.org/10.1016/j.tre.2026.104975).
- Xie et al., *Drones* 2026, model multi-truck/multi-drone fleet sizing and routing with cross-vehicle retrieval. Their native results report average cost reductions around 15.4–15.5% for medium/large instances. These are paper-specific cost results, not Luminous service-capacity results. [DOI 10.3390/drones10090658](https://doi.org/10.3390/drones10090658).
- Xu et al., *Transportation Research Part D* 158 (September 2026), model mobile swapping vans returning to central stations to reload, jointly sizing vans, racks and inventory under uncertainty. This supports treating replenishment as a closed-loop logistics process. [DOI 10.1016/j.trd.2026.105458](https://doi.org/10.1016/j.trd.2026.105458).

The linked receipt gate is not an alternative to these route/optimization methods. It is an independent feasibility boundary each proposed request schedule must satisfy. A fair tournament runs FCFS, EDF, SPT, slack rules, MPC, exact DP/MILP, ADP and learned proposals on the same source receipt set and hard reserve/permission/commitment gates.

## 7. Benchmarks and target ladder

Targets, not achieved performance:

| Benchmark | Target |
| --- | --- |
| Receipt provenance | 100% of accepted aerial slots resolve to an exact source event receipt |
| Fabricated receipt acceptance | 0 |
| Duplicate/replayed IDs and evidence references | 0 accepted |
| Aircraft sequence/turnaround | 100% agreement with independent receipt enumeration |
| Input permutation | 100% deterministic linked report |
| Mission deadline integrity | 0 accepted aerial slots whose source mission window is impossible |
| Weather/permission | 0 aerial admissions on unknown, denied or no-go intervals |
| Small request stream | 100% agreement with exhaustive assignment oracle |
| Policy quality | Report on-time, late, unserved and rejected separately; no blended score |
| Runtime | Target p99 <10 ms for bounded linked admission on declared hardware, excluding capacity-event re-run time; report total end-to-end separately |
| Capacity provenance overhead | Target <10% additional work beyond the capacity event simulation, measured on the same runner |

For larger cases, target mean regret ≤1% and p95 ≤5% against exact references where tractable. A capacity report that is synthetic or unmeasured remains synthetic even when its digest is valid.

## 8. Follow-on work

- **v0.1 (this change):** re-evaluate capacity architecture input and resolve linked aerial receipt IDs; no user-supplied schedule fields.
- **v0.2:** use signed/independently authenticated producer provenance only after key ownership, revocation and evidence trust policy are specified; SHA-256 alone is not authentication.
- **v0.3:** connect multiple sites, mobile carrier relocation/setup windows and cross-site aircraft sharing via #6177/#6153 without duplicating those owners.
- **v0.4:** use a scenario tree and exact oracle to compare policies with uncertain arrivals, wind, charge availability and outage correlation.
- **v0.5:** field telemetry replay with serial-bound battery, charger, aircraft and permission evidence; calibrate predicted opportunity times against measured turnaround and charge completion.

## 9. Claim boundary

A linked-mode pass demonstrates only deterministic consistency between a supplied synthetic architecture input, the capacity event evaluator's emitted receipt sequence, payload qualification envelopes, and the request assignment model. It does not prove actual flight performance, battery health, charge completion, weather prediction, aviation permission, customer delivery, grid eligibility or commercial superiority. No physical or market authority is granted.
