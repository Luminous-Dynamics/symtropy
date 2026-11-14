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
| 2026 Dubai integrated UAM MIP | Joint site, fleet, route, schedule and partial-recharge decisions; multi-trip and speed-dependent battery discharge | Paper reports 4–30% improvement over its studied heuristics; vertiport simultaneous landing/parking capacity is omitted and charging-plug availability is simplified; data/implementation/assumptions must be pinned before reproduction |
| 2026 demand-uncertain vertiport planning | Risk-based two-stage site selection and fleet optimization; route-extension sensitivity under airspace restrictions; existing heliport use | Paper-level comparator until exact formulation, uncertainty set, data and code are reviewed |
| 2026 TU Delft / Île-de-France integrated planning | Land-use, aeronautical feasibility and community-acceptance screening plus bidirectional demand-supply, infrastructure, fleet, dispatch and repositioning | Large-scale integrated reference; do not claim reproduction without matched source, data, assumptions and solver setup |
| 2026 data-driven vertiport siting (Shenzhen) | K-means candidate generation, revealed/stated preference surveys, binomial-logit adoption model and three-phase multi-objective integer programming; reports 88.21% demand coverage in its case study | Match the demand-coverage definition and data, not just the percentage; do not transfer its reported uptake/coverage to another city |
| 2026 urban grid-based UAV airspace planning (Dubai) | Road-aligned low-altitude grid, building-height map, two mathematical models and a genetic algorithm for high-rise urban traffic/service-performance evaluation | Urban grid/route baseline, not aircraft-specific reachable-set assurance or an operational UTM comparison |
| 2026 demand-uncertain vertiport siting | Risk-based two-stage site selection and eVTOL-fleet optimization; includes route-extension sensitivity under airspace restrictions and use of existing heliports | Match the uncertainty set, stages, recourse and risk objective before numerical comparison |

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

- FAA UTM is already described as a collaborative ecosystem for flight planning, authorization, surveillance and conflict management, separate from but complementary to air traffic services. FAA reports Letters of Acceptance for providers offering strategic deconfliction services.
- ICAO's UTM framework is specifically intended to support global harmonization and interaction with existing ATM systems.
- EASA U-space includes 4D flight-authorisation volumes, dynamic airspace reconfiguration, traffic information, geo-awareness and conformance services.
- EUROCONTROL's Trajectory Prediction Specification separates aircraft intent from the trajectory engine/performance model and distinguishes planned from tactical trajectories.
- NASA's 2026 ATMS project explicitly researches a shift from tactical to strategic traffic-flow and trajectory management while safely integrating new air vehicles with traditional operations.
- Commercial vendors also advertise low-latency vehicle-to-vehicle and multi-domain tactical deconfliction. Such vendor performance claims should be treated as claims until independently measured on a matched workload.

**Regulatory snapshots must be versioned.** The U.S. FAA BVLOS final rule (RIN 2120-AL82) was listed as Pending Review on OIRA's 14 November 2026 page (received 10 July 2026); this is a snapshot, not an indication that the rule is in force. South Africa's official Government Gazette 55226 (21 August 2026), Notice 7844, contains the Thirty-Third Amendment of the Civil Aviation Regulations 2026. It substitutes Part 101 regulation 101.01.2: the private-operation carve-out is limited to R-VLOS with a Class 1A or 1B RPA and prohibits dangerous goods aboard a UA. This is a narrow private-operations provision, not the complete rules for commercial/BVLOS operations or all medical cargo. Payload classification, exact consolidated Part 101 text, commencement and operation-specific permissions must be reviewed with SACAA / aviation counsel before any real deployment or compliance statement.

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
