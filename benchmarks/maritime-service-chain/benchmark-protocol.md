# Maritime service-chain benchmark protocol v0.1

Status: research protocol; no operational dataset or measured performance claim is present.

This protocol belongs to MARITIME-SERVICE-CHAIN-001 (#3683). It specifies how to
compare independent/silo planning with coordinated chain planning without
duplicating the underlying vessel, hydraulic, lock, port-call, logistics or
spatial engines.

## 1. Research question and claim ceiling

For one frozen demand set and one frozen geography, does coordination across
vessel, channel, lock, port, energy, and onward transfer improve **delivered
service** while respecting all non-compensable safety, navigability, water,
environmental, legal, data-rights and participant-authority gates?

A benchmark PASS means the implementation reproduced its declared inputs and
outputs and satisfied the declared contract. It does not prove that the physical
system is safe, that a vessel can navigate a route, that a terminal can handle
cargo, that a forecast is calibrated, or that an operator has authorized action.

Do not use "better than SOTA" until a published problem is reproduced with
matched scope, constraints, data assumptions, baselines and validation splits.

## 2. Frozen run manifest

Every run must persist a machine-readable manifest containing:

- benchmark schema version and case ID;
- synthetic/real classification and geography scope;
- demand snapshot digest, service units, participant/service-group labels,
  delivery windows and demand cutoff;
- route graph digest, CRS, horizontal/vertical datum, segment IDs and topology
  source revision;
- evidence inventory: IDs, owners, roles, source revisions, snapshot references,
  observation/validity times, state, claim ceilings and excluded evidence;
- vessel configuration/performance model revisions and applicability regime;
- lock, channel, berth, terminal, shore-power, water and inland-transfer resource
  revisions and declared units;
- environmental and water-window constraints;
- participant-authorized commitment/custody references where present;
- policy and algorithm version, executable commit SHA, toolchain and build flags;
- deterministic seed, run count, wall-clock budget, memory budget, candidate cap;
- stop reason: exact-complete, optimality-gap-reached, time-budget-exhausted,
  memory-budget-exhausted, infeasible, or input-needs-evidence;
- full objective vector, feasibility witness IDs, rejected alternatives and
  schedule/decision digest.

A missing manifest field is not filled from the machine clock, an ambient default,
or a later observation. A run that sees future information is invalid.

## 3. Matched information rule

All methods in a comparison receive byte-identical normalized inputs and the
same information cutoff. A method may not receive a fresher ETA, a revised berth
plan, an updated water-level observation, or a private commitment unavailable to
the other baselines.

When two standards describe similar but non-identical events, preserve their
semantics. Do not silently map PCO planned arrival at Pilot Boarding Place to
DCSA estimated berth arrival, or planned departure to an actual departure.
Use the existing adapters (#1930/#1955).

All resource and service units remain typed. No hidden conversion between tonnes,
TEU, passengers, vessels, lockages, slots, Wh, m3 and time ticks.

## 4. Baseline families

Freeze these as distinct methods; do not compare a new optimizer only against a
weak hand-written baseline.

| ID | Baseline | Required controls |
|---|---|---|
| B0 | Existing/no-build operating plan | Preserve the actual operating policy and observed service boundary; do not invent a schedule if records are absent |
| B1 | Silo composition | Independent vessel/voyage, lock, berth/handling and energy plans composed after optimization |
| B2 | Lock rules | FCFS, directional batching and least-wait-slack on the same requests |
| B3 | Fairness/efficiency rules | Reproduce published FSR/ESR rule definitions and parameter-selection procedure; do not relabel a locally invented heuristic as FSR |
| B4 | Multi-objective evolutionary | Matched NSGA-II, NSGA-III, MOEA/D and SPEA-II formulations where the paper's decision variables and constraints can be recovered |
| B5 | Multi-chamber/route MILP | Switching max-plus-to-MILP reference with identical vessel geometry, chamber, waypoint and route constraints |
| B6 | Simple operational heuristics | Earliest feasible arrival, minimum-energy feasible alternative, JIT, oldest-first and greedy resource fit |
| B7 | Exact oracle | Exhaustive or solver-proven tiny finite instances; independently implemented constraints and metrics |

Record baseline implementation source, version, solver, parameter-selection
process and known deviations. An unavailable baseline is marked NotReproduced,
not replaced by a straw man.

### Literature anchors

- Liao et al., "Scheduling rules of a waterway lock: fairness and efficiency,"
  *Ocean Engineering* (1 May 2026), DOI:
  https://doi.org/10.1016/j.oceaneng.2026.125004 .
  The Huai'an/Grand Canal case compares FSR and ESR using average waiting time,
  chamber-area load factor and inversion rate. Its reported one-way arrival-rate
  threshold of 7.6 vessels/hour is case-specific and must not be transplanted.
- Xu et al., "Intelligent Multi-Objective Optimization on Ship Lock Scheduling
  Considering Energy Consumption and Resource Constraints," *Systems* (3 May
  2026), DOI: https://doi.org/10.3390/systems14050507 . It compares NSGA-II,
  NSGA-III, MOEA/D and SPEA-II on a historical lock-hub instance with waiting,
  utilization, energy, rescheduling and berth-overcapacity objectives.
- Segovia et al., "Multi-vessel placement in multi-chamber inland waterway
  transport locks using switching max-plus algebra," *Ocean Engineering*
  (15 January 2026), DOI: https://doi.org/10.1016/j.oceaneng.2025.123414 .
  The model covers route/waypoint timing, lock chamber assignment and vessel
  placement and is translated to MILP.
- Du et al., appointment scheduling under vessel-arrival uncertainty,
  *Applied Sciences* (31 March 2026), DOI:
  https://doi.org/10.3390/app16073436 . Its 143-vessel, 15-day instance and
  reported improvements are paper-specific, not portable benchmark targets.
- Ji et al., serial lock scheduling with vessel sailing-speed optimization,
  *European Journal of Operational Research* (16 September 2026), DOI:
  https://doi.org/10.1016/j.ejor.2026.01.038 . This is a relevant future
  comparator for the joint serial-lock/voyage-speed service chain.

## 5. Scenario corpus

Freeze at least 30 end-to-end synthetic cases, each with a stable ID and
expected admission/infeasibility/NeedsEvidence result. Include, at minimum:

1. Valid low-demand baseline and exact replay.
2. High one-way lock demand and directional batching.
3. Alternating-direction queue with switch delay.
4. Short-deadline vessel versus long-service vessel.
5. Equal arrival but unequal arrival uncertainty.
6. Out-of-distribution arrival model regime.
7. Expired, future, disputed, revoked and unknown ETA evidence.
8. Arrival interval changed without a new evidence revision.
9. Channel depth/under-keel hard restriction.
10. Water-level/tide window closes before arrival.
11. Sediment/shoaling evidence changes the channel envelope.
12. Dredging/maintenance outage.
13. Lock outage before and during a planned interval.
14. Lock compatibility evidence revision mismatch.
15. Drought quota or environmental-flow gate.
16. Water-saving basin unavailable; no assumed recycled-water benefit.
17. Pump or grid outage and shore-power capacity reduction.
18. Berth capacity shortage.
19. Terminal handling capacity shortage.
20. Rail/road onward-transfer bottleneck.
21. Vessel payload is below demand.
22. Incompatible service units (tonnes vs TEU/passengers).
23. Participant flexibility offered but not authorized.
24. Participant authorization is revoked after plan publication.
25. Cargo custody evidence absent or disputed.
26. Demand shift changes the best feasible alternative.
27. Forecast regime shift causes calibration/coverage failure.
28. Common-mode failure affects two corridor assets.
29. Duplicate/stale/out-of-order port-call event.
30. No feasible schedule under hard constraints.
31. Multiple participants with unequal eligible demand and different service fulfilment.
32. Search budget exhausted before an optimum is proven.
33. Equivalent input permutation yields byte-identical normalized output.
34. Conflicting capacity calendars bind different source revisions.
35. Data-rights claim absent: external/real data remain unavailable to the run.

The first ten tiny cases should have hand-checkable or independent exhaustive
expected results. Larger cases must declare whether the algorithm is exact,
approximate, or incomplete.

## 6. Metrics — never collapse to one score

### Delivered service
- requested, evidence-eligible, scheduled and delivered tonnes/TEU/passengers;
- fulfilled share of eligible demand by participant and service group;
- missed deadlines, unserved amount and incomplete custody/hand-off;
- door-to-door elapsed time, with stage-level time decomposition.

### Time and throughput
- mean, median, p90, p95 and max waiting/delay by participant/service group;
- vessel time in port (waiting + berth), anchorage wait, berth occupancy and
  handling duration;
- lock service time, direction-switch time, total occupied time, chamber load
  factor, queue inversion rate, and convoy size;
- schedule churn and stability after each new observation.

### Physical/resource ledgers
- propulsion, auxiliary, shore-power and onward-transfer Wh separately;
- lock water draw, return, reuse, pump energy and loss by cycle;
- channel/navigability availability and the evidence time/datum behind it;
- dredged dry-solid mass and channel service availability when qualified inputs
  exist;
- well-to-tank, tank-to-wake and well-to-wake GHG separated; include CO2, CH4 and
  N2O only where source factors justify it.

### Fairness/distribution
- participant delivered/eligible ratio, wait tails and service displacement;
- service-group maximum wait and max-min fulfilment, where units are comparable;
- who receives delay, cost, risk, water use and flexibility benefit.

### Algorithm quality
- exact-oracle agreement on all tiny cases;
- feasibility / false-eligible / false-safe count (target zero);
- Pareto precision/recall, hypervolume only with a declared fixed reference point,
  epsilon indicator and IGD+ where a trusted reference front exists;
- best-known objective gap, proven optimality gap, candidates explored, runtime,
  memory and explicit incomplete/search-exhausted status.

Never interpret increased utilization as improvement if it increases environmental
violations, unserved service or distributional harm. Hard safety, navigability,
environment, water, consent, legal and data-rights constraints are
non-compensable.

## 7. Statistical protocol

- Use at least 30 deterministic seeds for stochastic methods and report the seed
  list. Exact deterministic methods need one run per frozen case plus replay.
- Use paired comparisons on identical case/seed pairs.
- Report medians and uncertainty intervals, plus effect sizes, not only best-run
  values or mean improvements.
- Hold out entire scenario families, forecast regimes, seasons and eventually
  geography/time periods. Do not randomly split rows from one route into train
  and test sets.
- Keep the exact small oracle disjoint from production implementation helpers.
- Fix hypervolume reference points before evaluating methods.
- Record tuning budget, solver version, hardware, timeout, and every method's
  infeasible/time-out count.
- Report dominated policies and failure cases; do not cherry-pick one objective.

## 8. Promotion gates

1. 100% agreement with the independent oracle on frozen tiny instances.
2. Zero hard-gate violations or false-eligible admissions.
3. Zero future-information leakage and zero use of expired/revoked authority.
4. 100/100 canonical identical replays for each deterministic case.
5. Every request has exactly one outcome and all request/eligible/delivered
   denominators reconcile.
6. All comparisons use matched inputs, time cutoffs, operating boundaries and
   reproduced baselines.
7. Only after independent data-rights, provenance, coverage and model-calibration
   review may source-permitted LPMS/AIS/port-call/terminal data be introduced.
8. A candidate research improvement is a repeatable >=5% change on one
   predeclared service/resource axis on held-out cases, with all hard gates at
   zero violations and other predeclared service/fairness axes within
   non-inferiority margins. This is an aspiration, not an achieved result or
   universal industry standard.

## 9. Architectural differentiation

The proposed paradigm is **evidence-qualified service-chain orchestration**:

1. Subsystems publish bounded feasible alternatives with evidence lineage, not
   opaque point predictions.
2. The chain evaluator composes only alternatives that satisfy typed contracts
   and resource/unit compatibility.
3. The optimizer returns a Pareto set and a decision certificate: input manifest,
   evidence references, hard-constraint witnesses, objective vector, rejected
   alternatives and optimality/search status.
4. New observations trigger bounded contingent re-evaluation, not uncontrolled
   replanning or silent mutation of an existing commitment.
5. Mycelix authorizes participant commitments/custody; Symthaea owns engineering
   validity; Symtropy owns optimization/scenario evaluation; Sol Atlas owns
   read-only spatial provenance/coverage. A recommendation does not grant
   operational authority.

The commercial frontier already links voyage-speed optimization to berth/terminal
plans. StormGeo announced its acquisition of Portchain on 30 October 2026,
bringing Portchain's real-time carrier/terminal berth alignment into StormGeo's
voyage-intelligence portfolio:
https://stormgeo.com/insights/stormgeo-acquires-portchain-to-advance-just-in-time-voyage-optimization-at-scale

Therefore, just-in-time arrival alone is not a credible differentiation claim.
The research target is the more complete chain: navigability and time-varying
waterway evidence, lock-cycle water/energy, sediment/environment constraints,
port/shore energy, inland hand-off, participant fairness/consent, and
uncertainty-aware recovery.

The World Bank/S&P Global CPPI is a valuable port-call diagnostic because it
benchmarks vessel time in port (waiting plus berth), but it is not a door-to-door
inland freight metric:
https://www.worldbank.org/en/topic/transport/publication/cppi

## 10. Existing project seams

- Symthaea: vessel/engineering/performance validity (#7444, #8171, #8102).
- Symtropy: deterministic scenario evaluation, JIT/recovery (#1858/#1867), lock
  engineering/scheduling (#3347/#3362), sediment/environment windows
  (#3279/#3302), port-call evidence semantics (#1930/#1955).
- Mycelix: participant authorization, cargo custody and lifecycle transitions
  (#5185).
- Sol Atlas: spatial provenance, source coverage and Africa data-rights audit
  (#115/#381).

No second optimizer, hydrodynamics solver, VTS, canonical port-call event model,
trust/identity service or GIS is introduced by this protocol.
