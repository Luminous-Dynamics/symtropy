# AeroWeft Benchmark Suite V1

**Programme:** AeroWeft — federated intelligence and infrastructure for shared skies  
**Status:** research benchmark contract; synthetic-first  
**Owners:** Symtropy (physics/simulation), Symthaea (advisory reasoning), Sol-Atlas (spatial context), Mycelix (provenance/federation)  
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

AeroWeft must not claim to outperform EUROCONTROL, FAA UTM, EASA U-space or NASA programmes from a toy simulator comparison. Compare only matched tasks. Treat established systems as a reference architecture and evaluation standard where executable data or baselines are not available.

## 4. Data admission and rights

Start with synthetic golden cases and independently implemented mathematical oracles.

Potential archived/research sources require separate admission:
- **OpenSky Network:** useful historical and scientific air-traffic data. Its current data page describes full historical database access for university-affiliated researchers, government organisations and aviation authorities; private/commercial entities must contact OpenSky for a licence. Public API and individual scientific datasets have their own conditions. Do not assume unrestricted commercial training or redistribution.
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

**Regulatory snapshots must be versioned.** The U.S. FAA BVLOS final rule (RIN 2120-AL82) was listed as pending EO 12866 review on the current OIRA page when this document was prepared. South Africa published 2026 Civil Aviation Regulations amendments in Government Gazette 55226; the exact current Part 101 text, commencement and applicability must be reviewed with SACAA / aviation counsel before any compliance statement.

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
