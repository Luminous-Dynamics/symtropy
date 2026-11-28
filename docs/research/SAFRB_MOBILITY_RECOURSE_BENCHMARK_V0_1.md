# SAFRB Mobility-Constrained Recourse Benchmark v0.1

**Status:** research protocol / design specification  
**Parent:** [SAFRB-MOBILITY-RECOURSE-001 #7876](https://github.com/Luminous-Dynamics/symtropy/issues/7876)  
**Composes with:** #7796, #7795, #7831, #7761, #7837, #7764

## 1. Research objective

Measure whether an observation-contingent infrastructure policy remains feasible when mobile resources must physically travel to the site where service is required.

The current two-site reference in [PR #7831](https://github.com/Luminous-Dynamics/symtropy/pull/7831) deliberately uses zero-time, zero-cost, unconstrained relocation for its full-recourse information bound. Keep that bound; do not reinterpret it as a physical movement policy.

This benchmark asks three distinct questions:

1. Can one common pre-observation deployment serve every supplied scenario?
2. Can an adaptive policy serve every supplied scenario while respecting what is known at each decision time and all physical movement constraints?
3. Would a clairvoyant or zero-movement relaxation say "feasible" when the physical, non-anticipative problem is not?

The third question is a diagnostic for optimistic model error, not a candidate operational policy.

## 2. Bounded v0.1 model class

Start with one homogeneous mobile-resource class, a finite directed transport graph, integer time, integer resource units and a finite scenario tree. Suggested initial ceilings:

| Dimension | v0.1 ceiling |
| --- | ---: |
| Service sites | 2–4 |
| Directed movement arcs | 8 |
| Discrete periods | 4 |
| Scenario leaves | 8 |
| Homogeneous mobile units | 8 |
| Arc travel duration | 1–4 periods |

These are oracle-size limits, not claimed deployment-scale capabilities. Any resource or instance limit exceeded must produce an explicit unsupported/DNF disposition, never a relaxed model.

### 2.1 Movement network

Each directed arc binds:
- stable arc ID and directed tail/head site IDs;
- positive integer travel duration in the shared timebase;
- per-departure-period capacity and availability;
- canonical physical corridor/resource identity;
- optional non-negative movement cost, reported separately from feasibility.

Reverse travel requires a separately declared reverse arc. A movement begun at period (t) with duration (	au) arrives no earlier than (t+	au). A resource in transit is unavailable at both endpoints until arrival. Corridor capacity applies to the declared departure interval and is not multiplied by route aliases.

### 2.2 Resource-state accounting

At each period boundary, every unit is in exactly one state:
- located at one site;
- in transit on one directed arc with one defined arrival period;
- unavailable under an explicit, separately modeled service/maintenance state.

A resource cannot be counted at its origin and destination while travelling, depart twice in one period, use a route before it is available, arrive before its travel duration elapses, or serve a site while in transit.

For each site and period, the evaluator reconciles:
`opening units + arrivals - departures - explicit removals = closing units`.

For each arc and departure period, total departures must respect the capacity of its physical corridor. Any shared corridor represented by several logical arcs must use one common capacity identity/gate or be rejected as unsupported.

### 2.3 Service

Each service floor binds beneficiary, site, service window, unit, quantity boundary and minimum required quantity. The initial profile should use a fixed, explicit service rate per deployed unit and period. Service is supported only by resources physically present at the required site during the relevant period; do not infer service from a resource that is merely planned to arrive.

Do not conflate mobile-unit count, handling throughput, storage capacity, route throughput and delivered service quantity. Their boundaries remain separate.

### 2.4 Information and policy

Each decision point binds:
- period and decision stage;
- the observable history available at that instant;
- its canonical information-set identity;
- actions that become binding at that time.

Decisions sharing the same observable history must be identical. Scenario IDs, future branch outcomes and later measurements cannot appear in pre-observation decision inputs. A route-closure or demand observation becomes usable only at its declared reveal time. Probabilities are omitted unless separately sourced and authorized; robust service floors are never silently converted into expected-value or chance constraints.

## 3. Mathematical reference semantics

Let (G=(V,A)) be the directed movement graph, (T={0,ldots,H}) the discrete horizon, and (mathcal{L}) the supplied scenario leaves.

For movement arc (a=(i,j)) with duration (	au_a>0), departure variable (x_{a,t,ell}) moves units from (i) at (t) to (j) at (t+	au_a), subject to the information available at the decision node and the arc's availability/capacity. The state compiler must represent in-transit occupancy explicitly; a normal one-period edge cannot be substituted when (	au_a>1).

At a decision node (n), let (I(n)) denote the validated information set. For every pair (n,n') sharing the same observable history and decision time:
`action[n] = action[n']`.

Service (s_{b,t,ell}) is bounded by the physically present resource count and the declared per-unit service rate. Every beneficiary floor must be met on every required scenario leaf and service period. No resource transition may depend on a branch before its corresponding observation.

The producer may build a time-expanded network for the movement-only subproblem and a deterministic equivalent for policy coupling. The chosen formulation must declare its model class. Do not force non-network couplings into a single-commodity flow certificate.

## 4. Result dispositions

Keep these outcomes separate:

- **HereAndNowFeasible:** a common deployment/action plan serves every supplied leaf without using hidden scenario information.
- **MobilityFeasiblePolicy:** a non-anticipative policy serves every supplied leaf under the declared travel, occupancy, route and service constraints.
- **MobilityFeasibleOnlyWithRecourse:** a common here-and-now plan fails, but an observation-contingent, travel-feasible policy succeeds.
- **InformationRelaxationOnly:** an explicitly optimistic zero-movement or clairvoyant relaxation is feasible, but no physically feasible policy witness has been established.
- **NotFeasibleWithinSuppliedTree:** exhaustive evaluation proves no policy in the admitted finite model class satisfies all floors.
- **Indeterminate / Unsupported / DNF / Timeout:** evidence, scenario closure or model-class support is incomplete, or the resource/time budget ends before a conclusion.

A timeout is not infeasibility. A failed heuristic search is not an infeasibility proof. A valid proof is scoped to the supplied tree and model assumptions.

## 5. Independent oracle and verification

Use three distinct paths on tiny cases:

1. **Producer:** time-expanded movement graph / deterministic-equivalent policy evaluator.
2. **Physical-state oracle:** exhaustive enumeration or dynamic programming over unit locations, in-transit states, remaining arc capacity and observable histories. It must not call the producer's graph compiler.
3. **Independent verifier:** reconstruct movement and service accounting from the source instance and submitted policy; verify digest, route legality, arrival epochs, in-transit occupancy, information-set equality and all per-leaf service floors.

A separate CP-SAT or MILP encoding may be used as a fourth differential comparator. Solver agreement is useful but does not replace the independently coded state-transition oracle.

For the pure single-commodity network-flow subcase, compare with Google OR-Tools `SimpleMinCostFlow` and LEMON `NetworkSimplex` / `CostScaling` where the model's constraints are mathematically equivalent. Do not benchmark a constrained policy model against a solver on a relaxed or different problem.

## 6. Frozen adversarial cases

At minimum, preregister positive/negative pairs for:

1. **Delayed observation:** departure must occur before a branch is revealed; root actions remain identical.
2. **Teleportation:** zero-time relaxation is feasible but every legal arrival misses the service deadline.
3. **Transit accounting:** reject a policy that counts a travelling unit at both endpoints.
4. **Multi-arc timing:** two sequential arcs require the sum of their travel durations.
5. **Direction:** a route in one direction does not imply a reverse route.
6. **Route closure:** distinguish a closure before departure from one after a legal departure.
7. **Shared corridor:** two logical route aliases cannot double a unit-capacity corridor.
8. **Congestion:** concurrent departures cannot exceed the shared corridor capacity.
9. **Late arrival:** an otherwise valid resource arrives after a half-open service window closes.
10. **Look-ahead:** reject policies using future scenario IDs, observations or outages.
11. **Information-set mutation:** equal histories cannot be split to authorize different actions.
12. **Boundary/overflow:** test half-open intervals, maximum integer periods and checked arrival arithmetic.

Add positive controls where an alternate path, earlier observation, additional mobile unit or longer service window changes only the declared feasibility boundary.

## 7. Benchmark scoreboards

Do not collapse results into one weighted score.

| Scoreboard | Report |
| --- | --- |
| Assurance correctness | False mobility-feasible, false infeasible, unsupported, indeterminate, DNF and timeout counts |
| Service | Per-beneficiary floor attainment, shortfall, missed periods and restoration time |
| Policy quality | Regret against exhaustive optimum, extra units required, pre-positioning burden and recourse dependence |
| Movement | Total travel time, distance/cost when defined, route utilization, transit occupancy and idle time |
| Runtime | P50/P95/P99 model build, solve and independent verification; peak memory |
| Robustness | Results under observation delay, route loss, stale capacities and common-cause corridor failure |
| Portability | Exact replay after serialization, digest stability and verifier agreement on a separate process |

Prospective targets (not achieved results):
- exact producer / physical-state-oracle parity on all bounded admitted cases;
- 100% rejection of frozen teleportation, look-ahead, transit double-counting and shared-corridor mutations;
- zero false mobility-feasible claims in a preregistered hard-negative corpus;
- false rejects reported on independent positive controls, with a proposed ceiling of 1%;
- every supported certificate accepted by the independent verifier and every mutated certificate rejected;
- complete DNF/timeout accounting and matched P50/P95/P99 latency and memory.

For a future 3,000-case hard-negative lane, report the independence design and confidence bound; do not infer an approximately 0.1% one-sided 95% failure-rate upper bound from repeated mutations of a small number of correlated base cases.

## 8. Competitive protocol

The 13th DIMACS Network Flows 2.0 challenge is the external algorithm-engineering reference for the pure MCF/flow track. Its MCF definition supports lower/upper arc bounds and signed node supplies; its benchmark page asks for core-instance reporting and says DNF should be reported rather than omitted. The challenge also notes that some historical instances are too easy for modern implementations, so include both official families and infrastructure-derived, preregistered cases.

Use:
- LEMON `NetworkSimplex` and `CostScaling` as strong pure-MCF references;
- OR-Tools `SimpleMinCostFlow` for the same network-flow subproblem;
- OR-Tools CP-SAT or a matched MILP formulation for movement/policy couplings;
- the exhaustive physical-state oracle for bounded correctness.

Every performance comparison must freeze the same graph, scenario tree, objective, timebase, integer precision, hardware, thread count, solver options and wall-time/memory budgets. Publish solver versions and configurations. Preserve raw results, DNF and all exclusions.

Reference material:
- DIMACS Network Flows 2.0: https://coral.ise.lehigh.edu/flow-challenge-2-0/
- DIMACS MCF definition/format: https://coral.ise.lehigh.edu/flow-challenge-2-0/mcf-definition-and-format/
- DIMACS MCF instances/generators: https://coral.ise.lehigh.edu/flow-challenge-2-0/mcf-instances-and-generators/
- OR-Tools minimum-cost flow: https://developers.google.com/optimization/flow/mincostflow
- LEMON NetworkSimplex: https://lemon.cs.elte.hu/pub/doc/latest-svn/a00404.html
- LEMON CostScaling: https://github.com/seqan/lemon/blob/master/include/lemon/cost_scaling.h

## 9. Claim ceiling and integration

The benchmark specification is paired with the bounded reference implementation recorded in Section 10. Until its exact-head Rust gates pass, treat the code as unqualified draft research code. A future passing implementation would establish only the declared finite synthetic mobility model and supplied scenario tree. It would not establish real route conditions, empirical travel-time distributions, asset condition, scenario completeness, source authority, legal access, dispatch authority or actual delivered service.

Compose rather than duplicate:
- #7796 owns general scenario-tree / non-anticipativity semantics.
- #7831 owns the tiny two-site, one-resource pre-positioning reference and its zero-time information-relaxation boundary.
- #7795 owns robust beneficiary floors under explicit recourse.
- #7761 owns coherent joint capacity scenarios.
- #7837 owns structural admission of evidence-bound dynamic capacity schedules; [#7894](https://github.com/Luminous-Dynamics/symtropy/issues/7894) owns the mobility consumer integration.
- #7764 owns benchmark preregistration and run identity.
- Mycelix remains the owner of issuer identity, rights, provenance and source-independence.

The intended advancement is not a faster route planner. It is a policy result that proves exactly what can be moved, when the decision-maker could know the relevant event, whether the resource arrives before service is due, and which assumption separates physical feasibility from an optimistic upper bound.

## 11. SOTA, industry and assurance-methodology comparators

The competitive position must be divided into four different questions. Do not describe all established products as flow solvers, and do not compare the mobility policy oracle to a relaxed flow model.

### 11.1 Pure network-flow algorithm engineering

Use the 13th DIMACS Network Flows 2.0 challenge and its official/core instance families for the pure MCF/max-flow lane. The challenge itself notes that many historical instances are now too easy for modern solvers and calls for new realistic benchmark families. Report official instances, infrastructure-derived instances, timeouts and DNF.

Comparators:
- LEMON `NetworkSimplex` and `CostScaling`;
- OR-Tools `SimpleMinCostFlow`;
- NetworkX capacity scaling / network simplex where graph class and multigraph semantics match;
- a separate exhaustive integer-flow oracle for bounded cases.

### 11.2 Coupled movement and decision policies

Use a matched CP-SAT or MILP formulation for integer movement, route, time and policy coupling. OR-Tools CP-SAT exposes distinct OPTIMAL, FEASIBLE, INFEASIBLE, MODEL_INVALID and UNKNOWN outcomes; preserve these distinctions and do not treat UNKNOWN as infeasible. HiGHS is a relevant independent LP/MILP comparator for compatible formulations.

The comparison must freeze the information sets, scenario leaves, route capacities, travel durations, service floors, integer precision, objective, hardware, wall-time and memory budget. The exhaustive physical trajectory oracle remains the correctness reference at tiny scale.

### 11.3 Domain-specific energy and industrial platforms

PyPSA is an important open-source energy-system comparator, not a SAFRB replacement target. Its current documentation describes economic dispatch, linear and security-constrained optimal power flow, capacity expansion, stochastic two-stage investment/recourse and modelling-to-generate-alternatives. For energy applications, compare against the strongest applicable PyPSA formulation and preserve its native optimization metrics. SAFRB's proposed incremental test is whether the same source/model revisions and candidate decisions also pass independent evidence, resource-identity, obligation and beneficiary-service qualification.

Commercial platform roles also differ:
- AVEVA PI System: industrial operational-data collection, contextualization, storage and visualization.
- Siemens Gridscale X: grid planning/operations, metering and digital-twin workflow.
- Schneider EcoStruxure Microgrid Advisor: DER monitoring, forecasting and microgrid optimization/MPC.

Treat these as potential integration environments and matched read-only replay sources, not interchangeable algorithm baselines. Public descriptions do not establish that any vendor lacks assurance features. The research question is whether an independently verified, portable cross-owner assurance receipt adds measurable value to their existing workflows.

### 11.4 NIST methodology and evidence federation

Use NIST's 2026 Super Intelligence for Manufacturing / TIDAL programme as a methodology comparator. Its research plan covers a shared test bench spanning real equipment and simulators, planning/scheduling/monitoring/root-cause benchmark studies, human-machine teaming and digital-twin verification/validation/uncertainty quantification. Compose with its measurement-science and VVUQ direction; do not create a second generic manufacturing testbed.

Use NIST IR 8536 (final, September 2026) as a traceability and provenance architecture comparator. It provides a meta-framework for interoperable traceability across manufacturing ecosystems while allowing organizations to retain existing standards and proprietary processes. Use its linking/selective-disclosure direction through Mycelix; do not duplicate a generic traceability repository.

### 11.5 Matched assurance ablation

A publishable SAFRB study should compare the same base planner and frozen input data under these arms:

| Arm | Added layer | Question |
| --- | --- | --- |
| A0 | Planner only | What does the native plan claim? |
| A1 | Semantic/model admission | Do the source records compile to a supported physical model? |
| A2 | Independent proof/replay | Does an independent verifier accept the result or expose a model/certificate error? |
| A3 | Authority/provenance closure | Are the evidence issuers, rights, freshness and common-cause dependencies admitted? |
| A4 | Decision-aware acquisition | Which admissible new observation changes an unresolved decision at lowest acquisition cost? |

Freeze planner version, source revisions, scenario set, compute budget, and operator action policy across arms. Report separately:
- false guaranteed-service and false infeasible findings;
- true/false positive and negative control outcomes;
- unresolved/unsupported/DNF/timeout coverage and risk-coverage curves;
- per-beneficiary service shortfall, missed deadlines and restoration delay;
- decision regret and acquisition cost;
- percentage of reports accepted/rejected by a separate verifier;
- P50/P95/P99 model-admission, solve, proof-build, verification and end-to-end latency;
- operator time and the quality of the identified next evidence action.

The key experiment is not whether A4 makes a prettier plan. It is whether A1–A4 reduce false assurance and decision regret enough to justify the added evidence and verification cost, without hiding difficult or unresolved cases.

### Reference links

- DIMACS Network Flows 2.0: https://coral.ise.lehigh.edu/flow-challenge-2-0/
- DIMACS instances and generators: https://coral.ise.lehigh.edu/flow-challenge-2-0/mcf-instances-and-generators/
- OR-Tools CP-SAT: https://developers.google.com/optimization/cp/cp_solver
- OR-Tools SimpleMinCostFlow: https://developers.google.com/optimization/flow/mincostflow
- HiGHS: https://ergo-code.github.io/HiGHS/stable/
- PyPSA optimization overview: https://docs.pypsa.org/latest/user-guide/optimization/overview/
- NIST SI for Manufacturing / TIDAL: https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing
- NIST IR 8536: https://csrc.nist.gov/pubs/ir/8536/final
- AVEVA PI System: https://www.aveva.com/en/products/aveva-pi-system/
- Siemens Gridscale X: https://www.siemens.com/en-us/products/gridscale-x/
- Schneider Microgrid Advisor: https://www.se.com/us/en/product-range/65896-ecostruxure-microgrid-advisor/

## 10. Reference implementation status — 28 November 2026

The draft implementation is now present on the stacked branch:

- `tools/safrb-time-expanded-adapter/src/mobility_oracle.rs` contains the bounded Rust producer, policy report, independent admission/replay path and per-scenario trajectory enumerator.
- `src/lib.rs` exports `mobility_oracle`.
- `schemas/mobility-instance-v0.1.schema.json` and `schemas/mobility-report-v0.1.schema.json` define strict portable input/report shapes.
- `tools/safrb-time-expanded-adapter/README.md` records the model ceiling and claim boundary.
- The Rust exact-head workflow has been added to the mobility branch's push qualification list and is triggered by the PR's source changes.

The test suite includes frozen cases for (a) zero-time feasibility with physically late arrival, (b) observation available before departure, (c) observation available only after the departure decision, (d) in-transit resource unavailability, (e) parallel logical routes sharing a physical corridor, (f) route closure with late observation, (g) insufficient total mobile units, and (h) mutated policy/history digests, (i) arrival exactly on a service boundary, and (j) directed-route non-reciprocity. It also contains 250 deterministic one-period tiny instances and 100 two-leg, multi-period cases verified through a separate scenario-trajectory enumeration and non-anticipativity join.

**Qualification remains pending.** The latest commit is not yet reported as passing exact-head formatting, Clippy, Rust tests or independent portable certificate conformance. No runtime, scale, SOTA, field or operational result is claimed. A failed or queued runner is not a pass.

