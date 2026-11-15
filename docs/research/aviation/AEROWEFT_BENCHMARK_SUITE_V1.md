# AeroWeft Benchmark Suite V1

**Programme:** AeroWeft — federated intelligence and infrastructure for shared skies  
**Status:** research benchmark contract; synthetic-first  
**Owners:** Symthaea (aircraft physical models and engineering evidence), Symtropy (independent reachability/enclosure math and simulation), Sol-Atlas (source-qualified spatial context), Mycelix (provenance/federation)  
**Related:** AIRSPACE-OPS-001 #2279, AIRSPACE-OPS-002 #2297, AEROWEFT-KINEMATICS-001 #5163, trajectory-audit PR #2285, Mycelix airspace PR #5024  
**Claim ceiling:** no operational ATC/UTM, clearance, dispatch, aircraft-control, separation-assurance, airworthiness or certification authority.

## 1. Evaluation philosophy

AeroWeft must not be judged by one weighted score. A planner that is fast but emits one false-safe result is not better. A mathematically sound outer bound that is so inflated that no route can be assessed is also not useful.

The suite therefore uses **non-compensatory gates**:

1. Model applicability and input validity.
2. Mathematical enclosure / conflict-screen soundness.
3. Uncertainty and probability calibration, when probabilities are claimed.
4. Operational-policy quality after hard constraints pass.
5. Resilience, data rights, human factors and public value.
6. Reproducibility and independent audit.

A failure at a higher gate cannot be compensated by throughput, latency, energy savings or an aggregate score.

## 2. Benchmark tracks

### Track A — physical-model and reachable-set soundness

Question: does a declared model enclose all states reachable within its declared domain?

**Metrics**
- Outer-set under-approximation count (hard gate).
- Containment against an independent analytic / validated-interval oracle.
- Enclosure width and volume inflation relative to the narrowest independently justified reference.
- Model-applicability coverage and unsupported/abstention rate.
- Refinement behavior as step size or enclosure budget changes.
- Separate initial-state, model-parameter, control, disturbance and numerical contributions.

**Initial targets**
- 100% containment on the frozen, bounded synthetic oracle domain.
- Zero under-approximation cases.
- 100% rejection of unsupported classes/regimes, unqualified frames/datums, invalid units, malformed time windows, non-finite inputs and budget exhaustion.
- Wider input sets must never produce a smaller reachable set under the declared set ordering.

These targets apply only to the frozen mathematical domain. They do not prove that the model covers real aircraft behavior.

### Deterministic / probabilistic dual-envelope rule

Where a model supports both, preserve two distinct outputs:
- deterministic outer enclosure for bounded worst-case exclusion;
- calibrated probabilistic prediction envelope for efficiency and planning.

A probabilistic 95% volume is not a worst-case guarantee. The uncontained tail remains possible; it cannot be re-labelled unreachable. Report nesting, volume ratio, calibration, tail misses and unsupported conditions separately. See [AeroWeft Kinematic Tubes §4.1](AEROWEFT_KINEMATIC_TUBES_001.md).

Existing reachability research already includes zonotopic state estimation coupled with reachable-tube prediction for aircraft, with preliminary CALC landing-challenge simulations. Backward reach-avoid tubes have also been explored for aircraft conflict management. AeroWeft must not claim to invent reachable sets; its research target is independently checked, profile-qualified, provenance-preserving composition across the aircraft model, spatial context, airspace constraints and network allocation.

### Track B — probabilistic prediction and conformance

Question: when the model reports probabilities or confidence intervals, do they mean what they say?

**Metrics**
- Empirical coverage at nominal 50%, 90%, 95% and 99% intervals.
- Coverage by prediction horizon, aircraft/model class, wind regime, sensor quality, site and traffic density.
- Calibration error, sharpness, interval width and tail miss rate.
- Drift detection delay and post-drift coverage.
- Coverage under out-of-distribution and incomplete-data conditions.

**Initial research target**
- A 95% nominal predictive interval should achieve 93–97% empirical coverage on an independently held-out, rights-cleared confirmatory corpus, with calibration error reported by subgroup rather than only in aggregate.
- No probability is emitted where model applicability or calibration evidence is absent; use explicit unknown/abstention.

The 93–97% band is an AeroWeft research target, not a regulatory requirement. It must be preregistered and revised only before confirmatory evaluation.

**Industry comparison**
- EASA U-space guidance discusses 4D authorization volumes incorporating operational performance and weather, with 95% containment as an expected level in the referenced guidance.
- EASA conformance-monitoring guidance specifies alerting within five seconds for 99% of detected non-conformance events.

These compare probabilistic containment and alert service performance. Neither is equivalent to deterministic worst-case reachable-set enclosure. Run the two tracks separately.

### Track C — four-dimensional conflict screening

Question: does the system detect all conflicts in the evaluated model and state the limits of its conclusion?

**Metrics**
- False-negative / false-safe count (hard gate).
- False-positive rate and unnecessary-alert burden.
- Detection lead time.
- Conflict-pair attribution and event grouping accuracy.
- Candidate-generation recall and candidate inflation.
- Full-time-coverage and stale-criteria rejection.
- Not-assessable / unsupported outcomes.

**Initial targets**
- Zero false-safe outcomes against the independent exhaustive oracle on the frozen synthetic corpus.
- 100% broadphase candidate recall against exhaustive pairwise enumeration on bounded test fleets.
- False positives reported separately; a conservative filter may retain extra candidates, but may never prune an oracle-positive pair.
- No-conflict is permitted only over the explicitly evaluated horizon with complete applicable criteria coverage.

### Track D — airspace-network planning and resource allocation

Question: can the network move more useful traffic without relaxing hard safety, infrastructure or community constraints?

**Policies to compare**
1. Fixed, predeclared corridors with static capacity.
2. First-come-first-served / FIFO reservation.
3. Shortest feasible route.
4. Time-expanded graph / capacity-aware shortest path.
5. Greedy conflict-free scheduling.
6. Rolling-horizon robust optimization (MILP/CP-SAT or another independently verified solver).
7. AeroWeft policy using reachable occupancy, uncertainty, dynamic restrictions, landing resources and energy constraints.

**Metrics**
- Feasible missions completed.
- Throughput per time window and infrastructure resource.
- Median and p95 mission delay.
- Route length / flight time / energy per completed mission.
- Time to first independently verified feasible plan.
- Optimality gap where a valid bound exists.
- Resource overbooking and false reservation conflict count.
- Replanning frequency, stability and unnecessary churn.
- Emergency insertion and recovery performance.
- Equity of access across locations and service classes.

Every algorithm must run on the same frozen input graph, aircraft profiles, source revisions, demand, weather, constraints and compute budget. Report Pareto fronts; do not hide safety constraints in objective weights.

### Track E — resilience and degraded operation

Inject, independently and in combinations:
- GNSS degradation, loss and spoofing indicators (synthetic only);
- communications loss, delayed messages, partition and provider disagreement;
- stale, missing, revoked, contradictory or future-effective airspace records;
- wind shifts, visibility deterioration, turbulence and weather closures;
- unavailable landing sites, charging failures and energy shortfall;
- non-cooperative traffic as an explicit unknown occupancy process;
- emergency demand and simultaneous diversion requests;
- clock skew, bad coordinate transforms, vertical-datum mismatch and corrupted records.

**Metrics**
- Safe unresolved / degraded outcomes.
- Service continuity for declared mission classes.
- Recovery and replanning time.
- Diversion feasibility and landing-resource contention.
- False-safe decisions (hard gate).
- Cost and delay of conservatism.
- Whether any provider partition or data-source loss creates contradictory authoritative state.

No software fallback may be interpreted as permission to continue a real flight. Aircraft-local lost-link and emergency behavior remains governed by the relevant certified/approved aircraft and operating procedures.

### Track F — community and lifecycle value

Evaluate the service delivered, not just aircraft utilization.

**Metrics**
- Cost per successful delivery / mission.
- Median and p95 medical-supply delivery time versus matched ground logistics.
- Essential-service coverage by travel-time band.
- Energy and emissions per delivered kilogram / passenger-kilometre.
- Noise exposure, time of day and population affected.
- Ground-risk exposure and consequence weighting.
- Wildlife / sensitive-area impact.
- Landing-site lifecycle cost, repairability and local serviceability.
- Rural/urban service gap and affordability.

Safety, privacy, legal constraints, noise limits and ground-risk limits are hard constraints where applicable. Do not let a weighted economic objective buy its way through them.

### Track G — end-to-end assurance and performance

**Evidence record**
- Frozen scenario and dataset manifests.
- Exact source revisions, rights and byte digests.
- Aircraft/model profile identity and applicability scope.
- Coordinate frame, vertical datum, units, time basis and transformation lineage.
- Algorithm, code, compiler, dependency and toolchain identities.
- Seeds and deterministic ordering.
- Complete rejected/unknown/unresolved case census.
- Independent verifier identity and output receipt.

**Performance**
- p50 / p95 / p99 end-to-end latency.
- Peak and allocated memory.
- Total trajectory and segment counts.
- Broadphase candidates and narrow-phase audits.
- Number of subdivisions/branches and budget exhaustion.
- End-to-end source-ingest, validation, model, conflict, planning and alert latency.

Benchmark 10 / 100 / 1,000 / 10,000 trajectories first. Only extend beyond that after recall and soundness gates pass. Candidate-generation latency alone is not full traffic-management latency.


### Track H — infrastructure recovery portfolios and common-cause exposure

Question: does a selected portfolio preserve complete, aircraft-applicable landing + energy + ground-transfer chains under declared single and compound failures, rather than merely providing nominal geographic coverage?

This track is owned by [AEROWEFT-INFRA-RECOVERY-001 #5224](https://github.com/Luminous-Dynamics/symtropy/issues/5224). It supplies aviation-specific constraints and outage evidence to the existing multimodal investment optimizer #1601; it does not implement a second site-selection or location-routing solver.

**Metrics**
- Complete recovery/service-chain availability by mission and community class.
- All inclusion-minimal failure sets and exact common-cause witnesses.
- Shared weather, power, ground-access, source and provider dependencies across nominal alternatives.
- Service-floor pass/fail under every declared singleton and compound outage.
- Landing-only recovery versus repeat-service chains that additionally require energy and ground transfer.
- Lifecycle investment, cost per protected service class and public-access distribution.
- Incomplete failure-domain inventory, unknown site status, stale/rights-pending source and unsupported aircraft applicability.

**Initial targets**
- 100% agreement with an independently authored exhaustive oracle on the frozen tiny portfolio.
- 100% exact minimal failure-set identification on that bounded corpus.
- Zero unknown/unqualified sites, energy resources or transfer links admitted as available.
- Zero asset-ID differences treated as proof of independent failure domains.
- All named service floors reported separately; no aggregate coverage score can conceal a failed group.

All numeric failure sets in this track are conditional on the fixture's declared dependency inventory. No empirical failure probability or real-world reliability rate is inferred.


### Track I — reciprocal recoverability and option externalities

Question: does an allocation preserve each aircraft's non-anticipative future options, and does one aircraft's reservation remove robust continuations from another aircraft under the same delivered observations and declared shared-resource branches?

This track is aviation-specific measurement over candidate schedules from #5186. It composes Symtropy #5163/#5207 and Symthaea #11291/#11349/#11394; it does not create a second generic scheduler, viability solver or policy-tree engine.

**Metrics**
- Per-aircraft robust lower option set and possible upper option set, with raw set identities.
- Joint continuation-policy existence under coupled shared resources.
- Reciprocal option-externality matrix using within-aircraft frozen option domains.
- Worst affected aircraft, minimum per-aircraft retention and distribution of option loss.
- Observation-equivalence / non-anticipativity failures.
- Emergency insertion and recovery-option loss.
- Unknown/incomplete branch coverage and exact rejected-successor witnesses.

**Initial targets**
- 100% agreement with an independently authored exhaustive oracle on the frozen tiny corpus.
- Zero clairvoyant policy choices when hidden branches share the same delivered observation history.
- Zero incomplete, stale or unsupported branches promoted into robust option sets.
- Exact deterministic results under input-order and identifier relabelling.
- No comparison of raw option counts across heterogeneous aircraft action domains.
- No joint recoverability claim solely because each aircraft has a non-empty individual option set.

These targets apply only to finite synthetic option/action domains. They do not establish continuous-time aircraft viability, real separation or an operational safety margin.


### Track J — endogenous demand, queueing and service-quality feedback

Question: when vertiport/pad capacity, access, fare, reliability and waiting time change, how does passenger mode choice feed back into the demand that the service must then handle?

This track is owned by [AEROWEFT-SERVICE-QUALITY-001 #5248](https://github.com/Luminous-Dynamics/symtropy/issues/5248). It consumes exact candidate schedules and resource-event traces from #5186 and provides qualified user-service outcomes to the existing multimodal optimizer #1601. It does not create another MNL estimator, general queueing library, bilevel solver, capacity planner or optimizer.

**Metrics**
- Potential, chosen, served, switched, deferred, denied and unresolved trips by anonymous cohort.
- Door-to-door generalized cost, affordability and access floors by cohort.
- p50/p90/p95/p99 wait, queue length, missed connection and unused capacity.
- Pad/stand/charger utilization and resource bottleneck attribution.
- Fixed-point count, convergence residual, cycle detection and initialization dependence.
- Operator private benefit, user generalized cost and declared public/network externality kept separate.

**Initial targets**
- 100% exact agreement with an independent finite choice/event-queue oracle on the frozen tiny corpus.
- Every fixed point rechecked by one exact application of the update operator.
- Every repeated state reported as a cycle, not silently treated as convergence.
- Adding a server cannot increase wait for an otherwise identical fixed arrival/service trace.
- Zero dropped demand cohorts and no hard #5186 capacity/airspace constraint relaxed by demand reduction.
- No aggregate score can conceal a cohort affordability/access-floor breach.

Synthetic fixed points and cycles are correctness controls, not adoption forecasts. Empirical preference coefficients require rights-cleared, representative, group-split data and calibration.


### Track K — finite-buffer airside blocking and queue spillback

Question: does an aircraft that completes nominal landing service actually have an available downstream taxi/stand/parking/charging resource, or does blocking-after-service keep it on the TLOF and push the delay back into terminal holding, energy reserve and later schedules?

This track is owned by [AEROWEFT-AIRSIDE-BLOCKING-001 #5261](https://github.com/Luminous-Dynamics/symtropy/issues/5261). It consumes candidate schedule/resource receipts from #5186, provides airside wait/practical-capacity outcomes to #5248, and may provide option-loss evidence to #5207. It does not create another scheduler, passenger queue, aircraft model or power solver.

**Metrics**
- TLOF service occupancy versus blocking-after-service occupancy.
- Finite stand/taxi/charger/holding occupancy and queue length.
- Exact blocked resource, aircraft, duration and downstream blocker witness.
- Actual versus nominal landing/release times, terminal holding and synthetic/profile-qualified energy delta.
- Charging/turnaround extension and next-mission eligibility.
- Nominal throughput versus practical capacity under a preregistered delay envelope.
- p50/p95/p99/max delay, movements displaced/deferred and resource spillback attribution.

**Initial targets**
- 100% exact agreement with an independent tiny event-trace oracle.
- Zero overlapping capacity-one resource claims or unmatched releases.
- Zero implicit infinite buffers or missing downstream resources treated as available.
- 100% explicit accounting of blocking/holding/delay events and causes.
- Exact replay and input-order/ID permutation invariance.
- A queueing approximation that drops blocking-after-service must be labelled as optimistic and rejected as a finite-buffer feasibility result.

This is event/fixture semantics, not a real vertiport capacity estimate or aircraft energy claim.


### Track L — PNT / clock evidence to hard aircraft occupancy bounds

Question: when position/navigation/time evidence becomes stale, integrity-uncertain, contradictory, correlated or recovering, does the aircraft occupancy set widen conservatively—or does the system incorrectly preserve a precise nominal point?

This track is owned by [AEROWEFT-PNT-REACHABILITY-001 #5270](https://github.com/Luminous-Dynamics/symtropy/issues/5270) and [draft adapter PR #5272](https://github.com/Luminous-Dynamics/symtropy/pull/5272). It consumes:
- PNT/communications service states from Symtropy #3909 and timing consequence states from #4079;
- clock-correlation/time-scale intervals from Mycelix #435;
- profile-specific navigation-health/failover from Mycelix #458;
- exact observation-set common-mode assessment from Mycelix #5904;
- receive-side hypothesis and oracle provenance from Symthaea #10775/#10826;
- bounded aircraft reachability from #5163 and aircraft-model applicability from Symthaea #6249.

It owns only an aviation-specific adapter and measurement protocol. It does not create a GNSS detector, clock conversion engine, sensor-fusion engine, common-mode graph, aircraft model or second reachability integrator.

**Metrics**
- Primary/fallback admissibility and typed NOT_ASSESSABLE reasons.
- Position-enclosure growth from sample-time and evaluation-time uncertainty.
- Maximum admissible observation age and profile-specific freshness margin.
- Hard-bound versus statistical-only rejection; no 95%/99% prediction is a deterministic enclosure.
- Independent evidence-family count and shared/unknown dependency cases.
- Time from upstream degradation receipt to removal of an invalid primary claim from the hard-reachability input.
- False continuity in the synthetic consequence adapter, stale/unknown acceptance and premature recovery admission.
- Exact receipt lineage: source claim, clock-correlation identities, profile/frame/datum, service-state receipt, dependency snapshot, and both time intervals.

**Initial targets**
- 100% exact agreement with a separate rational/integer interval oracle on the frozen adapter fixture.
- Zero statistical-only, stale, future-time, unknown-clock, correlated-only, contradictory or recovering inputs admitted as hard bounds.
- Widening either sample-time or evaluation-time uncertainty, or the supplied velocity set, never shrinks the resulting occupancy enclosure.
- Zero stale primary claims retained after a qualifying upstream degradation event in the deterministic suite.
- Every accepted envelope retains exact source/time/profile/dependency lineage; every rejected input has a typed reason.
- A NOT_ASSESSABLE result never becomes an empty aircraft state or a clear corridor.

The adapter's accepted inputs are mathematical claims under synthetic profiles, not certified navigation integrity, detector accuracy, aircraft physical truth or operational separation assurance.


### Track M — cooperative / non-cooperative surveillance observability

Question: when a traffic report is absent, what evidence supports the claim that a particular traffic class is observable in that volume/time—and what unresolved possible occupancy must remain when a sensor, class, environment, clock or common-mode dependency is not qualified?

This track is owned by [AEROWEFT-SURVEILLANCE-OBSERVABILITY-001 #5275](https://github.com/Luminous-Dynamics/symtropy/issues/5275). It supplies coverage-claim and unresolved-occupancy evidence to #2279/#2297/#5163 and candidate policy scoring in #5186. It does not implement ADS-B/radar/TDoA/EO-IR detection, a DAA system, a sensor-fusion engine or an avoidance controller.

**Metrics**
- Qualified/partial/degraded/unknown coverage by traffic class, cell, environment and validity interval.
- Cooperative and non-cooperative coverage measured separately.
- Receiver/sensor independent failure-domain families; never count raw receiver IDs as independent sources.
- Current qualified tracks, no-observation, stale/invalid and unresolved states.
- Coverage-volume/time fraction, false alarms, detection/track latency and continuity only where truth and sampling support such metrics.
- Blind/unobserved cell-time volume and possible-occupancy intervals delivered downstream.
- False-empty outcome count, with target zero.

**Initial targets**
- 100% exact agreement with an independent finite claim oracle on the tiny corpus.
- Zero no-report, not-qualified, stale or indeterminate cases converted to empty occupancy.
- Zero cooperative-only claims promoted to non-cooperative traffic coverage.
- Zero shared/unknown common-mode dependencies treated as independent.
- Every qualified claim binds traffic class, cell/volume, time, sensor/profile, environment and source lineage.
- A qualified coverage claim plus no report remains NO_OBSERVATION; it is not proof of zero traffic.

This is an observability/evidence adapter, not a DAA sensor-performance qualification or operational airspace-safety result.


### Track N — operational-intent conformance and contingency transitions

Question: does a qualified, time-stamped observation remain within the exact, current, activated 4D authorization and profile-specific deviation volume—and can the system distinguish definite deviation, possible deviation, missing evidence, contact loss, authorization supersession and dynamic-restriction invalidation?

This track is owned by [AEROWEFT-INTENT-CONFORMANCE-001 #5284](https://github.com/Luminous-Dynamics/symtropy/issues/5284). It composes:
- #2279/#2297 for 4D authorization geometry, restriction versions and numerical semantics;
- #5163 for optional qualified reachable projections;
- #5186 for candidate intent/schedule identities, without generating a new schedule;
- #5270/#5275 for PNT/time and surveillance/track qualification;
- Mycelix #435/#458/#5904 for clock, navigation-health and common-mode lineage;
- Sol-Atlas #189/#825 for source-qualified airspace constraints.

It owns only a finite conformance oracle, deviation-duration accounting, authorization revision/withdrawal and dynamic restriction checks, and contingency-state classification. It is not a UTM/U-space service, operator alerting system, flight authorization, or controller.

**Metrics**
- Definite-inside, definite-outside, possible-outside, unassessable and unobserved duration.
- Definite-outside lower-bound fraction and possible-exposure upper-bound fraction over the full evaluation window. Unobserved/unassessable time increases the upper bound; it never becomes observed-inside time.
- False-conforming and false-nonconforming outcomes reported separately.
- Time to definite-deviation detection, possible-deviation escalation and contact-loss/contingency transition.
- Expired, withdrawn, superseded authorization and dynamic-restriction revalidation outcomes.
- Exact authorization, intent, policy, observation, clock and restriction lineage.
- Every admitted track carries exact PNT/time and surveillance qualification receipt IDs; each dynamic restriction binds issuer, snapshot revision, digest and time-scale identity.
- Deviation expansion is clipped to the authorization activation/expiry interval; it cannot make a pre-activation observation conforming.

**Initial targets**
- 100% exact agreement with an independent integer/rational oracle on the frozen fixture.
- Zero false-conforming results for stale, unknown, revoked, expired, superseded, definitely-outside or lost-track cases.
- Zero definite non-conformance declarations from a boundary-straddling observation without satisfying the profile confirmation rule.
- Definite outside duration never exceeds possible outside duration; unobserved time is never observed-inside.
- Confirmed breaches cannot be hidden by unrelated unassessable intervals; unknown intervals cannot improve the upper exposure bound.
- Deterministic replay and canonical input-order invariance.

**Industry comparison:** EASA U-space AMC/GM describes 4D authorization volumes, uncertainty-aware deviation thresholds and a 5% time-outside confirmation rule in its specific U-space context. The EU Regulation requires continuous re-checking against dynamic restrictions. FAA UPP2 demonstrates non-conforming/contingent states and revised contingency volumes; its 30-second timeout was a pilot-demo parameter, not a universal threshold. NASA has probabilistic/simulation work on tracking performance and conformance safety. AeroWeft should treat these as matched references, not claim that this finite oracle reproduces a UTM service or certifies operation.

This is a synthetic conformance-semantic result, not an operational compliance finding.

## 3. Matched baselines

| Baseline | What it tests | Limitation |
|---|---|---|
| Symtropy #2285 piecewise-linear pairwise audit + sweep-and-prune | Current bounded geometric baseline and candidate recall | Supplied centreline paths; no aircraft dynamics |
| Mycelix PR #5024 constant-velocity coordination simulator | Lifecycle, reservation, dynamic-constraint and provider-view replay | Synthetic toy model; current PR validation must be checked at exact head |
| Point-estimate / constant-velocity propagator | Incremental value of uncertainty-aware sets | Not a physical performance model |
| Interval boxes, validated Taylor models, zonotopes / support functions | Reachability soundness versus conservatism and runtime | Compare on identical equations, input sets and horizons |
| Static corridor reservations | Baseline utilization and starvation | Rigid space-time allocation |
| FIFO, shortest-feasible, greedy and rolling-horizon planners | Traffic scheduling quality | Must use the same hard constraints and compute budget |
| EUROCONTROL trajectory-prediction requirements and NASA formal-methods / ATM research | Scope, modelling, verification and operational-system expectations | Reference specifications / research, not directly comparable executable products |
| [2026 Dubai integrated UAM MIP](https://doi.org/10.1016/j.trip.2026.101945) | Joint site, fleet, route, schedule and partial-recharge decisions; multi-trip and speed-dependent battery discharge | Paper reports 4–30% improvement over its studied heuristics; vertiport simultaneous landing/parking capacity is omitted and charging-plug availability is simplified; data/implementation/assumptions must be pinned before reproduction |
| [2026 joint backup-site planning](https://doi.org/10.1016/j.tre.2026.105075) | Joint vertiport and backup-landing-site selection, flight routes and contingency assignments; diversion-time, low-altitude conflict and passenger-disruption safety costs | Direct baseline for backup-site planning; AeroWeft must demonstrate additional value from source-qualified common-cause dependencies, complete recovery chains and fleet-level non-anticipative recoverability |




| [EASA U-space conformance monitoring guidance](https://www.easa.europa.eu/en/document-library/easy-access-rules/online-publications/easy-access-rules-u-space) | Match UA to accepted/activated authorization, compare with deviation thresholds, and confirm non-conformance beyond 5% of time outside the authorized 4D volume including deviation thresholds (validated over time) | Strong scoped policy reference. 5% is not hard-coded as a universal aviation limit; uncertainty/profile applicability must be explicit |
| [EU Regulation 2021/664 consolidated 22 February 2026](https://eur-lex.europa.eu/eli/reg_impl/2021/664/2026-02-22/eng) | Continuously re-check existing authorizations against new dynamic restrictions/limitations and relevant manned traffic information; update/withdraw as required | Revision-aware invalidation baseline; not a geometry/oracle implementation |
| [FAA UTM Pilot Program Phase 2 Final Report](https://www.faa.gov/sites/faa.gov/files/uas/research_development/traffic_management/utm_pilot_program/FY20_UPP2_Final_Report.pdf) | Non-conforming and contingent operational-intent states; share updated contingency volumes from available position/heading/speed/wind evidence | State-transition and contingency-sharing architecture comparator. Its 30-second timeout was used in the 2021 demonstration only |
| [NASA Safety Assessment of Conformance Monitoring for Situational Awareness in UTM Operations](https://ntrs.nasa.gov/citations/20240015613) | Safety analysis of operational-intent volumes, conformance and contingent transitions | Probabilistic/safety-analysis comparator; AeroWeft's tiny oracle is a deterministic semantic reference, not a replacement |
| [UTM tracking-performance Monte Carlo conformance study](https://www.mdpi.com/2504-446X/7/10/597) | Hybrid SITL/Monte Carlo study of latency, update intervals, tracking and navigation uncertainty; position extrapolation can accelerate alerts but increase nuisance/missed events under latency and velocity error | Key matched sensitivity baseline for observation freshness and false-alert tradeoffs; don't optimize alert delay in isolation |
| [ISO/TR 23310:2025 UTM system requirements study](https://www.iso.org/standard/87392.html) | Cross-jurisdictional catalogue of UTM actors, functions, data, quality, safety and security expectations | Interoperability/quality vocabulary, not a numerical benchmark |
| [ASTM F3442-25 DAA performance specification](https://doi.org/10.1520/F3442-25) | Smaller UA/crewed-aircraft encounters in stated lower-risk scope; expects mixed cooperative and non-cooperative traffic; no tactical ATC separation; scope excludes UA-to-UA and terrain/birds | Bounded system-performance reference only; this track stops at qualified observability and does not implement the standard's end-to-end DAA system |
| [FAA Part 107 DAA evaluation information](https://www.faa.gov/sites/faa.gov/files/uas/commercial_operators/part_107_waivers/Part-107-Waiver-Section-Specific-Evaluation-Information.pdf) | Evaluates the system end state after avoidance and requires cooperative/non-cooperative performance measured independently | Evaluation principle; AeroWeft supplies the evidence/occupancy interface and does not implement avoidance manoeuvres |
| [EUROCAE ED-267 OSED for VLL DAA](https://www.eurocae.net/product/ed-267-osed-for-detect-and-avoid-in-very-low-level-operations/) | Operational services/environment definition for VLL DAA and external supporting-service relationships | Operational-design baseline, not a sensor-level accuracy score |
| [2026 measurement-limited GNSS spoofing benchmark](https://doi.org/10.1016/j.nexres.2026.102604) | 22 real Baghdad FIR flight geometries with **synthetic** spoofing injection and modelled signal-power features; derives a model-specific detectability bound and compares D3QN with threshold/residual/supervised methods | Important measurement-limited comparator; 99.74% bound is specific to the injected 4,000 m / 12 dB reference attack and link-budget model; power-matched attack evades all tested methods; not field spoofing evidence or a universal detector ceiling |
| [2025 ADS-B spoofing TDoA multilateration study](https://doi.org/10.1016/j.trpro.2025.10.010) | ADS-B TDoA multilateration and global receiver-network redundancy analysis using OpenSky | Surveillance cross-check comparator; its own paper identifies receiver-network redundancy as a prerequisite; independent clock/site/power/network domains still require separate evidence |
| [EUROCONTROL GUID-203 (2026)](https://www.eurocontrol.int/publication/eurocontrol-guidelines-guid-203-operational-response-gnss-degradations) | Operational process reference for preparedness, detection, coordination, response and recovery, including AIS/ATS/ATFCM procedures | Operational guidance, not a detector score or a matched executable benchmark |
| [2026 vertiport airside DES](https://doi.org/10.1016/j.jatrs.2026.100109) | Practical versus throughput capacity; topology, stand/TLOF ratios, taxi operations and queueing policies beyond FIFO | Required finite-buffer DES baseline class; exact topology, service distributions, code and operating assumptions must be pinned before faithful reproduction |
| [2026 taxi/parking-level vertiport throughput](https://doi.org/10.3390/aerospace13010109) | Separate taxi and parking resources, charging duration, hourly demand and scheduling | Layout/capacity model-class comparison; the two-level architecture and synthetic case assumptions must be matched |
| [2026 Lyu & Feng vertiport/vertipad capacity](https://doi.org/10.1016/j.tra.2026.105027) | Joint location + pad-capacity planning with MNL demand, multi-server queueing and Lagrangian relaxation/Benders decomposition | Required service-quality baseline; Shanghai case and on-request data are not a direct reproducer; match wait definition, choice coefficients and server assumptions |
| [2026 Qin et al. vertiport capacity/commuter behaviour](https://doi.org/10.1016/j.tre.2026.105103) | Bilevel capacity/commuter-choice linkage; market underinvestment and policy-backfire analysis | Required model-class comparator; full-text assumptions and welfare estimand must be audited before numerical comparison |
| [2026 demand-uncertain vertiport planning](https://doi.org/10.1016/j.tra.2026.104957) | Risk-based two-stage site selection and fleet optimization; route-extension sensitivity under airspace restrictions; existing heliport use | Paper-level comparator until exact formulation, uncertainty set, data and code are reviewed |
| [2026 TU Delft / Île-de-France integrated planning](https://doi.org/10.1016/j.jtrangeo.2026.104611) | Land-use, aeronautical feasibility and community-acceptance screening plus bidirectional demand-supply, infrastructure, fleet, dispatch and repositioning | Large-scale integrated reference; do not claim reproduction without matched source, data, assumptions and solver setup |
| [2026 data-driven vertiport siting (Shenzhen)](https://doi.org/10.1007/s42524-026-5302-4) | K-means candidate generation, revealed/stated preference surveys, binomial-logit adoption model and three-phase multi-objective integer programming; reports 88.21% demand coverage in its case study | Match the demand-coverage definition and data, not just the percentage; do not transfer its reported uptake/coverage to another city |
| [2026 urban grid-based UAV airspace planning (Dubai)](https://doi.org/10.1007/s10479-025-06983-0) | Road-aligned low-altitude grid, building-height map, two mathematical models and a genetic algorithm for high-rise urban traffic/service-performance evaluation | Urban grid/route baseline, not aircraft-specific reachable-set assurance or an operational UTM comparison |
| [2026 Chicago mobile-location-demand siting](https://doi.org/10.1016/j.jairtraman.2026.103064) | Revealed mobile-location demand, MNL adoption, budget/capacity-constrained MILP and probabilistic siting heatmaps; identifies intermodal interceptor sites near surface bottlenecks | Demand/economic baseline, not aircraft safety; adoption probabilities are precomputed and the model is partial-equilibrium |
| [2026 DRAG-Net cross-city candidate-site transfer](https://doi.org/10.1111/exsy.70423) | Multi-source urban-data graph representation learns candidate sites in Beijing and transfers to Shanghai, then NSGA-II balances demand coverage and noise | Cross-city ML generalization baseline; underlying datasets are available from corresponding author on request, not an automatically admitted public dataset |
| [2026 joint vertiport and backup landing-site planning (Beijing)](https://doi.org/10.1016/j.tre.2026.105075) | Joint vertiport + backup-site locations, flight routes and contingency assignments; generalized safety costs for diversion time, low-altitude conflicts and passenger disruption | Direct backup-site/contingency baseline; compare the extra value of source-qualified common-cause dependencies, complete energy/ground chains and non-anticipative fleet recoverability |
| [2026 Chicago mobile-location-demand siting](https://doi.org/10.1016/j.jairtraman.2026.103064) | Revealed mobile-location demand, MNL adoption, budget/capacity-constrained MILP and probabilistic siting heatmaps; finds intermodal interceptor locations near surface bottlenecks | Economic/demand baseline, not aircraft safety; demand/adoption is partly precomputed/exogenous and the study states partial-equilibrium limitations |
| [2026 DRAG-Net cross-city candidate-site transfer](https://doi.org/10.1111/exsy.70423) | Multi-source urban-data graph representation learns candidate sites in Beijing and transfers to Shanghai, then NSGA-II balances demand coverage and noise | Cross-city ML generalization baseline; supporting datasets are available from authors on request, not automatically admitted |
| [2026 Shenzhen four-objective vertiport siting](https://doi.org/10.1007/s44212-026-00104-3) | Pareto optimization of population service coverage, network connectivity, noise exposure and infrastructure cost using high-resolution demand/spatial data | Multi-objective siting baseline; reports improvements over random placement, not aviation-safety or operational evidence |


AeroWeft must not claim to outperform EUROCONTROL, FAA UTM, EASA U-space, NASA or published 2026 UAM optimizers from a toy simulator comparison. For published optimization papers, distinguish (a) paper-level feature comparison, (b) transparent local baseline implementing the published algorithm class, and (c) faithful reproduction with source, data, parameters and compute budget pinned. Only (c) supports a direct numerical comparison to the published result.

## 4. Data admission and rights

Start with synthetic golden cases and independently implemented mathematical oracles.

Potential archived/research sources require separate admission:
- **OpenSky Network:** useful historical and scientific air-traffic data. Its current terms state that any use by a commercial/for-profit entity requires written permission and a licence; operational REST API use in a live product, service or automated system also requires a prior written agreement, including for non-profits and government entities. Full historical Trino access is described for university-affiliated researchers, government organisations and aviation authorities. Public API and individual scientific datasets have their own conditions. Do not assume unrestricted commercial training, live use or redistribution.
- **EUROCONTROL Demand Data Repository:** restricted stakeholder access and licence conditions; some API endpoints are ANSP-only. Do not describe it as an open public dataset.
- **FAA UTM Field Test publications:** useful evidence about real-world test scope, data correlation, planning/coordination, identity/access and cybersecurity. Public reports do not automatically grant access to underlying operational data.
- **South African civil aviation / aerodrome / airspace products:** require source, exact edition, legal applicability, data rights, freshness, coverage, transformations and vertical-datum review.

Each dataset must have a rights state such as CANDIDATE, RIGHTS_PENDING, RIGHTS_CLEARED, ADMITTED_DEVELOPMENT, ADMITTED_CONFIRMATORY or REJECTED.

**Leakage controls:** split by whole flight, aircraft/session, receiver/site, day, region and weather regime as applicable. A row-level random split is not confirmatory when correlated samples from one flight or sensor can occur in both train and test.

## 5. Industry and regulatory reference points

Current 2026 UAM research that should remain in the comparison matrix:

- [Dubai integrated site/fleet/routing/recharging optimization](https://doi.org/10.1016/j.trip.2026.101945): mixed-integer planning for sites, fleet, multi-trip routes and partial charging; the published 4–30% gain is relative to the study's own heuristics and scenario assumptions.
- [Lyu & Feng vertiport and vertipad capacity planning](https://doi.org/10.1016/j.tra.2026.105027): choice-based demand and multi-server queue waiting feed strategic site and pad-capacity decisions; this is the reference for Track J.
- [Wille et al. (2026), vertiport airside capacity by discrete-event simulation](https://doi.org/10.1016/j.jatrs.2026.100109): distinguishes throughput from practical capacity and includes taxi effects plus priority/non-FIFO queuing; this is the primary Track K model-class baseline.
- [EUROCONTROL GUID-203 (5 November 2026)](https://www.eurocontrol.int/publication/eurocontrol-guidelines-guid-203-operational-response-gnss-degradations): harmonized ATM/ANS approach covering preparedness, detection, coordination, response and recovery, with AIS/ATS/ATFCM procedures. This informs Track L service-state and consequence replay, not algorithmic detector scoring.
- [EASA SIB 2022-02 Revision 4](https://www.easa.europa.eu/en/newsroom-and-events/news/easa-updates-safety-information-bulletin-gnss-interference): current operational safety guidance on GNSS outage/alteration and navigation/surveillance degradation. Revision 4 was announced on 3 July 2026 and republished with editorial corrections on 22 July 2026.
- [EUROCONTROL Performance Review Report 2025](https://www.eurocontrol.int/sites/default/files/2026-03/eurocontrol-performance-review-report-2025.pdf): reports an 80% rise in reported GNSS interference cases in 2025 versus 2024 and describes impacts extending to terrain awareness, altimetry, ADS-B, CPDLC, clocks and display systems. This is occurrence reporting, not a universal risk rate.
- [Albaidhani (2026), measurement-limited civil-aviation spoofing benchmark](https://doi.org/10.1016/j.nexres.2026.102604): synthetic spoofing was injected into 22 real-flight geometries, while received-power features came from a link-budget measurement model. The paper derives a scenario-specific detectability bound; simpler residual/supervised approaches reached it, D3QN did not, and power-matched attacks defeated the tested detectors. Use as a measurement-limit and honest-abstention lesson, not a transferable accuracy target.
- [2025 ADS-B TDoA multilateration spoofing countermeasure](https://doi.org/10.1016/j.trpro.2025.10.010): studies receiver-network coverage/redundancy as a condition for multilateration-based spoofing detection. Receiver count alone does not establish independent clocks, sites, power, networks or data pipelines.
- [ASTM F3442-25](https://doi.org/10.1520/F3442-25): active DAA performance specification, with mixed cooperative/non-cooperative crewed traffic in its defined smaller-UA/lower-risk scope; excludes UA-to-UA and birds/terrain.
- [FAA DAA system-level evaluation guidance](https://www.faa.gov/sites/faa.gov/files/uas/commercial_operators/part_107_waivers/Part-107-Waiver-Section-Specific-Evaluation-Information.pdf): cooperative and non-cooperative aircraft performance must be measured separately; sensor detection alone is not the end-state safety metric.
- [EUROCAE ED-267](https://www.eurocae.net/product/ed-267-osed-for-detect-and-avoid-in-very-low-level-operations/): operational-services/environment reference for VLL DAA and supporting-service interoperability.
- [NASA EO/IR DAA surveillance range research](https://ntrs.nasa.gov/citations/20210025061): sensor-specific surveillance range and coverage assumptions for cooperative/non-cooperative traffic; not a transferable detection-range guarantee.
- [EASA Easy Access Rules for U-space](https://www.easa.europa.eu/en/document-library/easy-access-rules/online-publications/easy-access-rules-u-space): conformance determination requires matching a UA to an activated authorization and evaluating deviation thresholds; GM describes the scoped 5%-of-time confirmation threshold and 4D deviation volumes.
- [EU Regulation 2021/664, consolidated 22 February 2026](https://eur-lex.europa.eu/eli/reg_impl/2021/664/2026-02-22/eng): continuous checking of existing authorizations against new dynamic restrictions and relevant manned-traffic information, with updates/withdrawals as circumstances require.
- [FAA UTM Pilot Program Phase 2 Final Report](https://www.faa.gov/sites/faa.gov/files/uas/research_development/traffic_management/utm_pilot_program/FY20_UPP2_Final_Report.pdf): operational intent non-conforming/contingent state transitions and updated contingency volume sharing; the 30-second transition timeout is demonstration-specific.
- [NASA Safety Assessment of Conformance Monitoring for Situational Awareness in UTM Operations](https://ntrs.nasa.gov/citations/20240015613): operational intent volumes, nonconforming/contingent state semantics, and the effect of deviation on strategic deconfliction.
- [UTM tracking-performance Monte Carlo conformance study](https://www.mdpi.com/2504-446X/7/10/597): tracking latency/update rates/navigation uncertainty can alter missed and nuisance alerts; position extrapolation trades detection speed against alert variability.
- [ISO/TR 23310:2025](https://www.iso.org/standard/87392.html): cross-jurisdictional UTM function, actor, data and safety/security/quality requirements study.



- [Nagrare & Lieb (2026), vertiport throughput with taxiing and parking levels](https://doi.org/10.3390/aerospace13010109): charging duration and parking capacity affect turnaround, schedule flexibility and throughput.
- [Sengupta et al. (2025), UAM research challenges and opportunities](https://doi.org/10.1146/annurev-control-022823-031353): identifies finite parking/blocking and the interaction of airborne holding, battery state, charging and parking occupancy as cross-layer capacity issues.

- [Qin et al. vertiport capacity and commuter behaviour](https://doi.org/10.1016/j.tre.2026.105103): bilevel service-capacity/commuter-choice interaction; market and policy results remain model- and context-specific.
- [Gottschalg et al. demand-uncertain airport-shuttle planning](https://doi.org/10.1016/j.tra.2026.104957): risk-averse two-stage site/fleet planning with recourse under realized demand.
- [Salt Lake City parcel-level vertiport suitability](https://doi.org/10.1016/j.cities.2026.107255): almost 60 spatial datasets and fewer than 1% highly suitable parcels in the studied geography.

- [Demand-uncertain vertiport location planning](https://doi.org/10.1016/j.tra.2026.104957): risk-based two-stage site selection and eVTOL fleet planning, with findings on airspace route extensions and use of existing heliports.
- [TU Delft / Île-de-France integrated design and operation](https://doi.org/10.1016/j.jtrangeo.2026.104611): candidate-site knock-out criteria, land-use/aeronautical/community acceptance, bidirectional demand-supply, fleet and dispatch/repositioning interactions.
- [Shenzhen data-driven vertiport location optimization](https://doi.org/10.1007/s42524-026-5302-4): RP/SP choice modeling, adoption probability, K-means demand candidates and three-phase multi-objective location optimization; the paper reports 88.21% demand coverage in its case study.
- [Dubai road-aligned high-rise UAV grid planning](https://doi.org/10.1007/s10479-025-06983-0): urban grid and building-height-aware traffic design solved with a genetic algorithm; a planning/service-performance reference rather than an aircraft-specific reachability assurance result.
- [2026 vertiport scoping review](https://www.sciencedirect.com/science/article/pii/S2667091726000828): review of 81 publications highlighting the remaining fragmentation between network siting, physical layout and ground integration.

These studies have different objective functions, geography, aircraft assumptions, demand data and reported metrics. No percentage or coverage figure may be transferred directly to AeroWeft.

- FAA UTM is already described as a collaborative ecosystem for flight planning, authorization, surveillance and conflict management, separate from but complementary to air traffic services. FAA reports Letters of Acceptance for providers offering strategic deconfliction services.
- ICAO's UTM framework is specifically intended to support global harmonization and interaction with existing ATM systems.
- EASA U-space includes 4D flight-authorisation volumes, dynamic airspace reconfiguration, traffic information, geo-awareness and conformance services.
- EUROCONTROL's Trajectory Prediction Specification separates aircraft intent from the trajectory engine/performance model and distinguishes planned from tactical trajectories.
- NASA's 2026 ATMS project explicitly researches a shift from tactical to strategic traffic-flow and trajectory management while safely integrating new air vehicles with traditional operations.
- Commercial vendors also advertise low-latency vehicle-to-vehicle and multi-domain tactical deconfliction. Such vendor performance claims should be treated as claims until independently measured on a matched workload.

Operator-described operational architectures add important engineering baselines:
- **Amazon Prime Air:** its public operations description reports pre-planned routes reviewed for airspace conflicts, ADS-B ingestion, onboard FAA-approved detect-and-avoid, trained flight monitors, contingency return/safe-landing behaviour, and coordination with FAA-accepted UTM service providers. Amazon also claims thousands of deliveries per day during designated operating hours and plans broad 2026 expansion. These are operator-published descriptions/claims, not independently matched AeroWeft benchmark results: https://www.primeair.amazon/operations and https://www.aboutamazon.com/news/transportation/amazon-prime-air-drone-delivery-expansion
- **FAA UTM:** FAA describes UTM as a collaborative ecosystem separate from but complementary to air traffic services, supporting flight planning, authorization, surveillance and conflict management. The FAA says it has begun issuing Letters of Acceptance to providers for strategic deconfliction services. This establishes an interoperability and service-recognition benchmark; it is not a published numerical head-to-head performance score: https://www.faa.gov/uas/advanced_operations/traffic_management
- **Zipline proposed Arizona operations:** the FAA's November 2026 draft environmental assessment describes pre-planned routes generated with weather, wind, population density and obstacle context; cooperative aircraft-to-aircraft position/velocity/intent exchange over radio/cellular; strategic and tactical deconfliction; onboard ADS-B DAA; FAA-approved UTM service use; NOTAM and operational-area notifications; and route-boundary containment/fallback behaviour. It is a *draft environmental assessment of a proposed action*, not evidence that the described expansion has been approved or that the architecture has independently passed a matched safety/performance evaluation: https://www.faa.gov/uas/advanced_operations/nepa_and_drones/20261104_Zipline_Arizona_Metros_Draft_EA_508.pdf

AeroWeft should therefore not compete by reimplementing an operator's aircraft, delivery app, onboard DAA or UTM network. Its comparison opportunity is the **independent infrastructure and evidence layer**: model-applicable reachable occupancy, auditable resource constraints, reciprocal future options, common-cause recovery networks and whole-journey public value. Any comparison against Prime Air, Zipline or FAA UTM must be task-matched and must respect operator/jurisdiction-specific authority boundaries.

**Regulatory snapshots must be versioned.** The U.S. FAA BVLOS final rule (RIN 2120-AL82) was listed as Pending Review on OIRA's 14 November 2026 page (received 10 July 2026); this is a snapshot, not an indication that the rule is in force. South Africa's official Government Gazette 55226 (21 August 2026), Notice 7844, contains the Thirty-Third Amendment of the Civil Aviation Regulations 2026. It substitutes Part 101 regulation 101.01.2: the private-operation carve-out is limited to R-VLOS with a Class 1A or 1B RPA and prohibits dangerous goods aboard a UA. This is a narrow private-operations provision, not the complete rules for commercial/BVLOS operations or all medical cargo. Payload classification, exact consolidated Part 101 text, commencement and operation-specific permissions must be reviewed with SACAA / aviation counsel before any real deployment or compliance statement. For South Africa, preserve the additional distinction between **BVLOS** and **BRLOS**: SACAA's UAS page lists BVLOS under RLOS and says BRLOS is reserved for future use; the official 26th Amendment / Gazette 48228, Part 101.05.11, describes conditional Director approval for B-VLOS under a UASOC operations manual and SA-CATS 101, with VMC / below-400-ft conditions unless otherwise approved. BVLOS is not BRLOS, and neither source grants blanket permission. Store visual flight rule, radio-link scope, operator category and approval status as separate fields. See https://www.caa.co.za/industry-information/uas/ and https://www.gov.za/sites/default/files/gcis_document/202303/48228rg11556gon3170.pdf.

## 6. Confirmatory protocol

Before a benchmark can support a superiority statement:

1. Preregister workload, model profiles, hard constraints, metrics and acceptance thresholds.
2. Freeze data rights, exact source versions, group splits and the confirmatory holdout.
3. Freeze baseline implementations, toolchain, hardware and time budgets.
4. Separate exploratory results from confirmatory results.
5. Use independent implementations / verifiers for mathematical claims.
6. Publish all misses, false alarms, unsupported cases and unresolved dependencies.
7. Repeat across held-out time, region, traffic density and environmental regimes.
8. Publish the complete evidence manifest, not just a summary score.

No benchmark result from synthetic data should be described as a real-world safety rate, commercial throughput, regulatory compliance or operational SOTA result.

## 7. First demonstrator

Use a **synthetic regional medical-logistics network**:

- one district hospital;
- several clinics;
- one pathology laboratory;
- one medical-supply depot;
- multiple candidate landing / recovery sites;
- synthetic fixed-wing and multirotor profiles introduced in separate stages;
- time-varying weather and wind envelopes;
- charging / energy constraints;
- matched ground-logistics alternatives.

The first question is whether a model-qualified, uncertainty-aware network can deliver more reliable essential-service logistics than FIFO/static-corridor and ground-only baselines under identical constraints. This begins in offline simulation; it does not authorize or recommend live aircraft operations.
\n\n## 8. Medical air-ground benchmark composition\n\nThe first application-specific scenario is tracked in [AEROWEFT-MEDCHAIN-001 #5181](https://github.com/Luminous-Dynamics/symtropy/issues/5181). It composes aviation-specific resource/occupancy constraints with existing multimodal optimization (#1601), mission-chain and handoff continuity (#1887), physical cargo qualification (#1636), and source-admitted transport data (#1667). It explicitly does not create another graph optimizer or mission-chain schema. The initial corpus is synthetic regional medical cargo only, not patient carriage or clinical decision support.\n

The initial fictional regional medical-cargo scenario is specified in
[AEROWEFT_MEDCHAIN_001.md](AEROWEFT_MEDCHAIN_001.md) and tracked by
[AEROWEFT-MEDCHAIN-001 #5181](https://github.com/Luminous-Dynamics/symtropy/issues/5181).
It composes the existing multimodal optimizer and mission-chain/cargo contracts
rather than introducing a parallel graph or mission engine.


### Regulatory source snapshots checked 2026-11-14

- U.S. OIRA, RIN 2120-AL82 status: https://www.reginfo.gov/public/do/eoReviewSearch?rin=2120-AL82
- South African Government, Gazette 55226 / Notice 7844 (Thirty-Third Amendment, 21 August 2026): https://www.gov.za/sites/default/files/gcis_document/202608/55226gon7844.pdf
- The cited Part 101 amendment appears at the Gazette's printed pages 121–122 (PDF pages 67–68). It changes regulation 101.01.2's private-operation provision. The benchmark does not interpret the legal applicability of any particular real mission.


### Aircraft-model ownership alignment — 2026-11-14

The fixed-wing physical model is owned by Symthaea
[TRANSPORT-FIXED-WING-001 #6249](https://github.com/Luminous-Dynamics/symthaea/issues/6249),
composed with candidate-specific mass-coupled mission inputs (#7729) and
adaptive engineering evidence orchestration (#10902). Symtropy
[AEROWEFT-KINEMATICS-001 #5163](https://github.com/Luminous-Dynamics/symtropy/issues/5163)
owns independent set-enclosure/reachability mathematics and trajectory-tube
verification. Do not duplicate fixed-wing equations or physical model
parameters in Symtropy. The model/profile adapter must preserve exact profile
revision, operating regime, units, frames, vertical datum, state/control/
disturbance domain, dependency lineage and evidence scope.


### Current research comparator — NASA corridor flexibility and demand/capacity balancing

NASA's 2025 flight evaluation (NASA/TP-20250003243) tested strategic path-planning automation with two helicopter surrogate aircraft and up to 330 cooperating virtual traffic aircraft. It reports successful core intent-based conflict functions but notes that corridor-constrained maneuvers were less successful in part because tighter volumes reduced maneuvering degrees of freedom. The benchmark should therefore measure **retained maneuverability / contingency reserve**, not only corridor occupancy or throughput.

NASA's 2024 DCB-Waypoint work resolves capacity at vertiports and en-route waypoints sequentially, one flight and waypoint at a time; DCB-Airborne-Delays additionally uses speed changes within aircraft speed ranges. These are useful matched baseline classes for the new [AEROWEFT-DCB-001 #5186](https://github.com/Luminous-Dynamics/symtropy/issues/5186).

NASA's 2026 Digital Trajectory Rerouting Capability is a separate commercial-airline/ATM reference: it has been operationally evaluated with FAA and airline partners and uses a shared digital picture and end-to-end reroute coordination. It is not a UAM or drone benchmark; AeroWeft should compare coordination/evidence architecture, not lift its performance numbers into a different workload.

References:
- NASA/TP-20250003243: https://ntrs.nasa.gov/citations/20250003243
- NASA DCB: https://ntrs.nasa.gov/citations/20240011341
- NASA Digital Trajectory Rerouting Capability: https://www.nasa.gov/directorates/armd/aosp/atms/digital-trajectory-rerouting-capability-tech-transfer/
- Zonotopic reachable flight regions: https://doi.org/10.1016/j.ifacol.2017.08.716
- ETH Zurich reach-avoid research: https://control.ee.ethz.ch/research/applications/air-traffic-control.html


The concrete maneuverability-aware resource scheduling benchmark is
[AEROWEFT-DCB-001 #5186](https://github.com/Luminous-Dynamics/symtropy/issues/5186),
specified in [AEROWEFT_DCB_001.md](AEROWEFT_DCB_001.md). It uses sequential
DCB-Vertiport / DCB-Waypoint / speed-delay policy classes as reference baselines,
and tests joint batch space-time allocation while measuring retained
maneuverability and recovery options. It composes existing optimizer and
aircraft-model owners; it does not create another generic optimizer or flight
dynamics model.


## 10. Cross-project model / intent / authority contract

The machine-interface design is specified in
[AEROWEFT_MODEL_BOUNDARY_V1.md](AEROWEFT_MODEL_BOUNDARY_V1.md). Benchmark
records must keep mission intent, physical model/configuration, environment,
predicted trajectory, deterministic reachable enclosure, probabilistic
prediction envelope, resource reservation and external authority decision as
separate typed claims. A checksum or reservation receipt is not authorization.


### Mature operational coordination comparator — NASA Digital Trajectory Rerouting

NASA's 1 July 2026 technology-transfer page reports that its Digital Trajectory Rerouting Capability was operationally validated over four years of live operations across the North Texas Metroplex (DFW/DAL) and Houston (IAH), with FAA and three major airlines. NASA reports 3,870 candidate reroutes at Houston I90 TRACON in FY25, 62% airline approval, runway prediction accuracy above 90%, and close to one hour of aggregate metroplex delay saved per rerouted flight. These are NASA-reported results for a commercial-airline/FAA rerouting workload, not AeroWeft results and not a directly matched UAS/UAM comparator.

AeroWeft should learn from its shared digital picture, candidate identification and end-to-end coordination workflow. The correct comparison is architectural: event freshness, provider/authority boundaries, candidate-generation quality, human approval, end-to-end delay, evidence and reproducibility. Do not copy its numerical performance targets into a different aircraft class or workload.

Source: NASA Digital Trajectory Rerouting Capability, page updated 1 July 2026:
https://www.nasa.gov/directorates/armd/aosp/atms/digital-trajectory-rerouting-capability-tech-transfer/


### South African source-admission work

Sol-Atlas now tracks
[AEROWEFT-SA-DATA-001 #825](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/825)
with draft [PR #826](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/826).
Its source inventory distinguishes SACAA AIP editions, public NOTAM summaries,
authenticated current briefings, regulatory amendments and data rights. The
medical benchmark remains synthetic until every required source is admitted for
its declared purpose; a public PDF or NOTAM summary is not a live operational
feed or flight-preparation service.


### Track O — cross-layer assurance composition

**Question:** can PNT/time, bounded reachability, cooperative/non-cooperative surveillance and operational-intent conformance be composed without converting unknown, stale, revoked or unqualified owner outputs into false-safe planning inputs?

**Consumes:** #5163, #5270, #5275, #5284 and existing #2279/#2297/#5186, with Mycelix #435/#458/#5904 and Symthaea #6249/#10775/#10826.

**Owned output:** a deterministic, receipt-preserving composition disposition. The strongest positive state is `READY_FOR_CONFLICT_SCREEN`, never "safe", "clear", "authorized" or "dispatchable".

**Hard gates:** zero false-empty occupancy; zero unqualified positive admissions; zero traffic-class substitution; zero suppression of reauthorization, nonconformance or contingency; at least one positive control; exact replay and permutation invariance; zero evidence-degradation monotonicity violations; complete receipt traceability.

**Paradigm shift:** non-compensatory evidence composition rather than a weighted safety score. Each domain keeps its own authority and algorithm. Cross-layer composition checks identity, receipt, status, class scope and authority transitions only. A no-report under qualified surveillance becomes unresolved possible occupancy; a class with unqualified or missing coverage remains not assessable. No partial validated occupancy list is returned on composition failure.

**Fixture:** `fixtures/aeroweift-assurance-composition/evidence-composition-v0.json`, synthetic-only, 32 curated cases. The isolated oracle is in `tools/aeroweift-assurance-composition-oracle/`. The profile-owned traffic-class requirement is separately receipted and cannot be weakened by the composition request. The Rust test campaign adds 44 single-fault mutations, 990 unordered two-mutator combinations, and 10,000 deterministic seeded two-fault trials; every mutated case must fail closed, while a separate positive control must pass. The fixture and tests are finite contract checks, not physical, operational, certification or safety evidence.

**Research comparator:** 2026 integrated vertiport/vertipad optimization, demand/queueing and airside discrete-event simulation remain the planning/capacity comparators; AAM DAA/separation literature remains the surveillance/conflict comparator. Track O evaluates the evidence interface to such systems rather than reimplementing their optimizers or DAA algorithms.


### Track P — subject-scoped causal receipt binding (#5337)

Verifies that each composed result was derived from the exact aircraft, flight, scenario, operational-intent revision, authorization revision, model profile, coordinate/time basis and parent evidence it claims to consume. Uses typed parent roles, exact parent digest references, validity and causal-time checks, producer-sequence fork detection, graph-cycle and orphan rejection, and profile-owned traffic-class obligations. The positive result is `CAUSALLY_BOUND_ONLY`; byte integrity, signature identity, issuer authority and revocation remain explicitly not evaluated. Synthetic structural benchmark only; no flight authority or operational safety claim. Implementation: `tools/aeroweift-causal-receipt-binding/` and `docs/research/aviation/AEROWEFT_CAUSAL_RECEIPT_BINDING_001.md`.


### Track Q — trusted evidence custody and historical/current reliance (#5351)

AeroWeft-facing adapter to Mycelix's role-separated signed evidence, pinned trust roots, sensor provenance, revocation and evidence-lease owners. Verifies canonical bytes, signatures, signer identity, issuer role/predicate/subject scope, trust-policy epoch/root, key lifecycle and (where required) SCITT transparency proofs. Historical authenticity, current reliance, physical truth, source independence and operational authority remain separate facets. Reuse Mycelix #5899/#5895/#5863/#2143/#181/#2688/#1215; do not create a parallel DID registry, trust-root store, transparency service or generic currentness/lease algebra. No trust result mints flight/dispatch/control authority. Synthetic verifier only until cryptographic known-answer fixtures and independent review pass.


### Track R — conflict-management performance evidence envelope (#5354)

Builds a declared-domain performance evidence profile above planning and composition. It measures missed/late/nuisance alerts, latency tails, resolution feasibility, contingency/recovery, unresolved occupancy, exposure-normalized rates with confidence bounds, and subgroup coverage by traffic/encounter/environment/failure class. It reports a non-compensatory vector rather than a single safety score and keeps synthetic, simulation, SIL, HIL, flight-test and accepted operational evidence levels separate. NASA's 2025 Required Conflict Management Performance (RCMP) research is a motivating comparator; AeroWeft does not set separation minima, authorize operators or implement authority acceptance. See #5354.


### Track S — multi-subject evidence scope (#5359)

Separates the focal flight/operation being assessed from the external target an observation describes and from the environment/coverage cell to which a surveillance claim applies. Requires typed scope relations so an external or unknown target may contribute unresolved occupancy without becoming the focal aircraft or receiving a fabricated identity. Extends Track P's single-focal-flight causal receipt graph to multi-aircraft and unidentified-target cases; it does not implement detection, sensor fusion or target association itself.



### Track T — model credibility and simulation-to-flight transfer (#5362)

Qualifies the exact aircraft/model/control/sensor/solver/time/frame/datum configuration and operating-domain envelope for which modelled results may receive evidence credit. Separates calibration, development and confirmatory data by flight/scenario groups; binds datasets to exact aircraft/control/sensor and reference-measurement profiles; and requires per-stratum metrics with preregistered sample floors and conservative confidence bounds. Error gates use the upper confidence bound; coverage/recall gates use the lower bound. The structural evaluator pins the exact evaluator build and validation-policy manifest. Positive state is `ADMISSIBLE_FOR_DECLARED_DOMAIN_REVIEW_ONLY`, not physical validation, certification, safety or regulatory compliance. It does not run the simulator, authenticate data or prove calibration. Research comparators: ASTM F3705-25/F3442-26/F3201-26, NASA's 2025 flight evaluation of strategic path planning and NASA's 2024 CMSA safety assessment.



### Track U — model-evidence change impact and targeted requalification (#5371)

Consumes Track T's exact model-validation claims and Mycelix #6219/#1087/#2631/#1207/#181 to determine which claim dimensions remain applicable after a bitemporal change event. It distinguishes still-applicable, historical-only, requalification-required, contradicted, indeterminate and unsupported-change outcomes per claim. A sensor calibration change should invalidate only sensor-dependent claims; an aircraft/control/model/solver/reference/domain change follows explicit versioned dependency rules. Effective time and recorded/knowledge time remain separate, and incomplete change-stream coverage never means “no change.” Track U does not implement a generic event store, lease algebra or universal currentness service. Hard gates: zero stale dependent claims remain applicable, zero unrelated claims invalidated under complete dependency evidence, zero coverage gaps treated as no-change, exact bitemporal replay and deterministic event permutation.



### Track U — model-evidence change impact and targeted requalification (#5371)

Aviation-specific consumer of Mycelix #6219/#1087/#2631/#1207/#181. Evaluates exact Track T model-evidence claims against bitemporal owner change events and change-feed coverage. Effective time and recorded/knowledge time are separate; late-recorded events change later reconstructions without rewriting earlier "as-known" results. Statuses are per claim, with explicit per-dimension affected, requalification, contradiction, retirement, unsupported and indeterminate sets. Known contradictions are not hidden by unrelated feed gaps; missing/partial coverage never becomes evidence of no change. Track U owns only aviation dependency and requalification semantics; it does not create a generic event store, lease algebra or universal currentness service. Implementation prototype: tools/aeroweift-model-requalification/.
