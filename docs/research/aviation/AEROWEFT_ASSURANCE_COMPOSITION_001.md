# AeroWeft — Cross-Layer Assurance Composition Benchmark

**Tracking issue:** [AEROWEFT-ASSURANCE-COMPOSITION-001 #5304](https://github.com/Luminous-Dynamics/symtropy/issues/5304)  
**Consumes:** Symtropy #5163/#5270/#5275/#5284 and #2279/#2297/#5186; Mycelix #435/#458/#5904; Symthaea #6249/#10775/#10826  
**Status:** synthetic-first composition contract and finite benchmark  
**Claim ceiling:** offline composition of already-qualified owner claims. It is not a UTM/ATM service, flight authorization, dispatch system, DAA system, separation service, aircraft controller, airworthiness determination or safety certification.

## 1. Research question

Can qualified evidence from navigation/time, surveillance observability, aircraft reachability and operational-intent conformance be composed without allowing one layer to turn another layer's unknown, stale, invalid, revoked or unqualified state into a false-safe planning input?

AeroWeft already has separate work for these domains. The missing experiment is at their seam. A planner may receive a valid reachability envelope while the clock basis is stale; a surveillance layer may report no aircraft while only cooperative traffic is observable; an intent may have been superseded while its last received message is fresh. Independent components can all be locally plausible while their composition is invalid.

This benchmark tests the *composition contract*, not the internal algorithms of those owners.

## 2. Paradigm shift — an evidence lattice, not a safety score

Do not collapse safety-relevant outputs into one confidence score, weighted utility, or "AI says safe" label.

Every layer contributes a scoped claim with:
- owner and immutable qualification receipt;
- source and dependency snapshot;
- time-scale, profile, coordinate-frame and datum identity;
- validity interval and revision;
- a typed positive, negative, unknown, degraded or revoked disposition.

Composition follows four rules:

1. **Non-compensation.** A positive output from one layer cannot compensate for an invalid or unknown prerequisite in another layer.
2. **Monotone degradation.** Removing evidence, worsening service state, widening uncertainty or discovering a common-mode dependency cannot improve admissibility.
3. **Authority before utility.** A revoked, expired, superseded or nonconforming intent cannot be rescued by a fresh track or a high-quality navigation fix.
4. **Unknown remains occupied/unknown.** No report under qualified coverage means unresolved possible occupancy. No qualified coverage means not assessable. Neither means an empty sky.

The output is a typed evidence disposition, not a probability of safety. The strongest positive result is `READY_FOR_CONFLICT_SCREEN`. It must be passed to the existing conflict-screening owner (#2279/#2297), with any unresolved occupancy preserved. Every result, including a rejected one, retains the complete submitted owner-input snapshot and all receipt references; a rejected result returns no partially validated occupancy list. It is not a conflict-free result, clearance, route authorization, dispatch decision or proof of separation.

## 3. Owner boundary

| Domain | Owner | Composition responsibility |
|---|---|---|
| 4D trajectory conflict and planning | Symtropy #2279/#2297 | Receives eligible, explicitly qualified occupancy; retains conflict math |
| Aircraft reachable occupancy | Symtropy #5163 | Supplies a qualified bounded-set receipt |
| PNT, sample time and evaluation time | Symtropy #5270; Mycelix #435/#458/#5904 | Supplies accepted hard-bound/fallback disposition and exact lineage |
| Cooperative/non-cooperative observability | Symtropy #5275 | Supplies class-specific coverage, track or unresolved-occupancy disposition |
| Operational intent/conformance | Symtropy #5284 | Supplies current authorization, conformance, contingency and reauthorization state |
| Candidate schedules/resources | Symtropy #5186 | Retains schedule/resource ownership; receives no implied clearance |
| Aircraft physics/applicability | Symthaea #6249 | Retains physical-model provenance and applicability |
| Receive-side PNT assessment/oracle provenance | Symthaea #10775/#10826 | Retains detector/hypothesis and truth-source ownership |

This oracle must not independently convert clocks, count receiver independence, infer sensor range, derive aircraft physics, propagate a second reachability model, generate schedules, evaluate 4D conflict geometry, create an authorization, issue a pilot alert or control a vehicle.

## 4. V0 composition contract

A candidate is eligible for conflict screening only when:
- the intent/conformance result has a present, identity-matched receipt;
- intent state is `CONFORMING`;
- PNT state is an accepted deterministic hard bound or an accepted qualified fallback, with its receipt;
- reachability is qualified and bounded, with its receipt;
- time-scale, profile, frame and datum identities match across the evidence chain;
- every required traffic class has exactly one class-specific surveillance assessment;
- every class assessment carries its coverage qualification receipt and dependency snapshot;
- the upstream independence assessment is qualified for the declared profile;
- a `QUALIFIED_TRACK` also carries an observation receipt; or
- a `NO_REPORT_QUALIFIED_COVERAGE` is retained as `UNRESOLVED_POSSIBLE_OCCUPANCY`.

The following intent states are terminal and must be propagated:
- `REAUTHORIZATION_REQUIRED` → `REAUTHORIZATION_REQUIRED`;
- `NONCONFORMING` → `NONCONFORMING`;
- `CONTINGENT` → `CONTINGENCY_HANDOFF`.

`POTENTIALLY_NONCONFORMING` and `NOT_ASSESSABLE` cannot be promoted to conforming.

The V0 traffic classes include cooperative crewed and non-cooperative crewed traffic. The required class list is profile-owned. Cooperative coverage cannot satisfy the non-cooperative obligation.

A positive disposition preserves each class's coverage receipt, optional observation receipt, and dependency snapshot. A failed composition returns no partial occupancy list; downstream callers cannot mistake a partially validated result for a complete one.

## 5. Adversarial fixture

Golden fixture: `fixtures/aeroweift-assurance-composition/evidence-composition-v0.json`.

The 28-case fixture contains positive controls and fault injections for:
1. fully qualified conforming path;
2. qualified fallback PNT path;
3. no report under qualified coverage;
4. stale PNT with a fresh delivery receipt;
5. statistical-only navigation;
6. PNT clock-scale mismatch;
7. overlapping/unqualified dual-clock interval;
8. nominal receiver multiplicity with unqualified common-mode independence;
9. cooperative-only coverage while non-cooperative traffic is required;
10. unknown TDoA clock dependency;
11. stale track delivered with a fresh receipt;
12. missing surveillance qualification receipt;
13. non-assessable reachability;
14. expired/superseded authorization;
15. intersecting dynamic restriction;
16. non-intersecting restriction positive control;
17. definite nonconformance despite unrelated upstream degradation;
18. contingency after profile-owned contact-loss timeout;
19. potentially nonconforming intent;
20. missing PNT/reachability/intent lineage;
21. frame/profile/time-scale identity mismatch;
22. duplicate class evidence;
23. class-order permutation and replay determinism;
24. horizontal-frame mismatch;
25. vertical-datum mismatch and absent vertical-datum identity;
26. duplicate required traffic-class declarations;
27. complete source-snapshot retention on accepted and rejected decisions;
28. repeated-evaluation determinism.

All values, identities, receipts, aircraft, traffic, restrictions and timelines are fictional. Synthetic fixture success is not a sensor, navigation, airworthiness, operational or safety validation.

## 6. Hard-gate metrics and targets

These are targets for this declared benchmark, not achieved field results.

| Metric | V0 target | Why |
|---|---:|---|
| False-safe / false-empty occupancy promotion | **0** | No unknown may become empty or cleared |
| Unqualified positive-evidence admission | **0** | Positive labels require exact receipts |
| Required traffic-class substitution | **0** | Cooperative does not imply non-cooperative |
| Reauthorization/nonconformance/contingency suppression | **0** | Authority and execution state dominate utility |
| Positive-control acceptance | **≥ 1** | Avoid reject-all implementations |
| Exact replay determinism | **100%** | Same input and revision, same disposition |
| Input-order semantic variance | **0** | Permutation must not change the decision |
| Evidence-degradation monotonicity violations | **0** | Worsening input cannot improve admissibility |
| Accepted/rejected receipt traceability | **100%** | Every decision must be auditable |

For a later benchmark release, expand to at least 10,000 deterministically seeded combinations, pairwise and selected higher-order fault interactions, mutation testing of every admission predicate, adversarial time-boundary sweeps, and independent implementation comparison. Report p50/p95/p99 latency and memory only after measuring them on declared hardware and fixture scale. Never trade away hard-gate correctness for throughput.

## 7. Industry and SOTA comparison

AeroWeft should compare against the work that exists, not claim to replace it.

- **Integrated vertiport and vertipad optimization (Transportation Research Part A, July 2026)** jointly models siting, vertipad capacity, multinomial-logit demand and multi-server queueing, using Lagrangian relaxation and Benders decomposition. It reports that service quality and waiting time materially affect adoption. AeroWeft should consume the resulting candidate plans and attach evidence admissibility; it should not rebuild that optimizer. [Article](https://doi.org/10.1016/j.tra.2026.105027)
- **Dubai integrated planning (Transportation Research Interdisciplinary Perspectives, May 2026)** jointly optimizes vertiport locations, fleet size, routes and charging, including partial recharging and speed-dependent discharge; the study reports 10–30% higher profit than its heuristic or peak-demand planning baselines. This is an economic/planning comparator, not a safety-assurance benchmark. [Article](https://doi.org/10.1016/j.trip.2026.101945)
- **Vertiport airside capacity via discrete-event simulation (Journal of the Air Transport Research Society, June 2026)** evaluates topology, taxi operations, throughput, practical capacity and queuing policies. It identifies taxi processes and limited surface resources as material capacity factors. AeroWeft's relevant challenge is to preserve qualified occupancy and evidence constraints when these simulations are coupled to candidate schedules. [Article](https://doi.org/10.1016/j.jatrs.2026.100109)
- **Advanced-air-mobility separation assurance and collision avoidance review (Progress in Aerospace Sciences, 2026)** surveys geometric, probabilistic, intent-aware and graph-based methods for heterogeneous traffic and sensing. AeroWeft must be evaluated as an assurance/composition layer around such methods, not represented as a new DAA algorithm. [Review](https://doi.org/10.1016/j.paerosci.2026.101241)
- **FAA UTM** describes a collaborative ecosystem for low-altitude operations, with flight planning, authorization, surveillance and conflict-management services; the FAA notes that standards, interoperability and service-recognition work continue. [FAA UTM overview](https://www.faa.gov/uas/advanced_operations/traffic_management)
- **NASA/FAA UTM ConOps v2.0** keeps operators responsible for separation and unsafe-condition avoidance, supported by shared intent, shared awareness, strategic deconfliction, tracking/conformance and tactical capabilities as applicable. AeroWeft's claim ceiling must remain below operational separation/authorization authority. [ConOps](https://www.nasa.gov/wp-content/uploads/2024/04/2020-03-faa-nextgen-utm-conops-v2-508-1.pdf)

The defensible differentiation to test is *cross-owner evidence integrity*: typed abstention, immutable qualification receipts, common-mode-aware input admission, revision-aware authority propagation, explicit possible occupancy, and deterministic replay. Industry performance comparisons must be matched on aircraft profile, traffic mix, sensor availability, time horizon, demand, restrictions, uncertainty, compute and output semantics. A cheaper schedule or faster conflict query is not an assurance win if it was evaluated with weaker inputs.

## 8. Falsification plan

A benchmark result is invalidated if any of these can occur:
- remove or corrupt one qualification receipt and still get `READY_FOR_CONFLICT_SCREEN`;
- stale or statistical-only PNT is accepted as a hard bound;
- a fresh message receipt masks stale observation time;
- three correlated receivers are counted as independent by composition;
- no-report, unqualified coverage or an uncovered required class becomes an empty cell;
- cooperative-only coverage satisfies a non-cooperative class requirement;
- dynamic restriction, authorization revision or definite nonconformance is suppressed;
- widening uncertainty or degrading a service makes the decision more permissive;
- input permutation changes the semantic result;
- a positive result is exposed as safe, clear, authorized or dispatchable.

## 9. Implementation and evidence status

The isolated Rust oracle is dependency-free. It checks only owner status, identity and receipt composition. The JSON fixture is a separately inspectable golden corpus. The composition layer never claims to verify the truth of upstream claims; each upstream owner remains responsible for qualification.

All checks must be reported by exact commit SHA. A queued workflow is not a pass; a format pass is not a Clippy or test pass; fixture evaluation is not physical validation.
