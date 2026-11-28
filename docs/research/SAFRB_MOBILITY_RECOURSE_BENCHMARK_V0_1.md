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


## 11. Evidence-bound route-capacity / mobility integration

The current integration composes the capacity_admission schedule verifier with the mobility_oracle. It adds a bounded bridge from externally marked-admitted route-capacity evidence to scenario-specific, departure-period-specific movement constraints.

The integration is intentionally not a second route optimizer. It supplies an admitted schedule to the existing physical recourse model and verifies that the resulting model was reconstructed from the original records.

### 11.1 New semantics under test

Every directed route has a distinct route-capacity subject and canonical resource reference. Directed aliases of one physical corridor share a corridor-capacity subject. A complete capacity schedule is required for every mobility scenario, with exact agreement on case, scenario, horizon, timebase, quantity unit and capacity boundary.

The compiler produces a capacity for every route and every departure period. Capacity may tighten below the static structural ceiling, including to zero; it may never exceed the declared static ceiling. Missing, overlapping, stale, revoked, dimensionally inconsistent, wrongly mapped or scenario-mismatched evidence cannot be replaced by the static capacity. The independent verifier reconstructs every per-scenario/per-period value from the original records and compares the full compiled mobility model.

A new diagnostic metric is **temporal-capacity leakage**:

    L_cap = departures accepted above the admitted scenario/route/corridor capacity
            divided by all evaluated departures

The target is zero on the frozen adversarial suite. Report arc-level and shared-corridor leakage separately; a route that is individually within its cap can still violate the shared physical corridor cap.

### 11.2 Capacity-semantics ablation

| Arm | Capacity treatment | Injected fault | Expected result |
| --- | --- | --- | --- |
| C0 | Static scalar capacity | No fault | Legacy model baseline only |
| C1 | Complete scenario-period schedule | Capacity drops before a later departure | Correctly constrained plan |
| C2 | Shared corridor subject across route aliases | Alias is split into separate subjects | Reject |
| C3 | Separate schedules per scenario | One scenario has lower capacity | Preserve that scenario; never splice maxima |
| C4 | Schedule admission and independent reconstruction | Gap, overlap, stale/revoked record, wrong unit/timebase, changed receipt | Reject or retain explicit unresolved state |
| C5 | Full composition | Forged override or altered route-to-subject map | Independent verifier rejects |

The principal metric is not how often the planner finds a feasible plan. It is how often it can avoid **false feasibility** under time-varying physical limits while retaining a useful and correctly qualified unresolved result when the evidence is incomplete.

### 11.3 Routing benchmarks: separate solver quality from assurance quality

Use established benchmark families according to the subproblem they actually test:

- **Solomon VRPTW**: standard time-window routing baseline; VROOM documents 56 instances with 100 jobs across clustered, random and mixed classes. Re-run baselines on a common, modern hardware profile and publish raw run settings; historical VROOM timing data is not a current performance target.
- **Li & Lim PDPTW**: pickup/delivery plus time-window baseline; useful when the operating case requires paired pickup/drop-off semantics.
- **Blauth et al. 2024 time-dependent VRP**: use the published real-road / time-dependent travel-time instances, not static distance matrices. The published family includes ten cities and problem sizes 10, 500, 1,000 and 2,000, with piecewise-linear travel-time functions. Its delivery-only, unlimited-identical-fleet assumptions do not cover heterogeneous assets, finite service-resource capacity or infrastructure evidence; extend it without changing the original test cases.
- **DIMACS Network Flows 2.0**: retain for pure flow/min-cost-flow kernels. It is not a substitute for vehicle routing or policy feasibility. Its organisers explicitly note that old instances can be too easy for modern implementations and call for updated, real-application instances.

The mobility reference model currently has only 2–4 sites, 1–4 homogeneous units, at most 8 directed arcs and at most 8 scenarios. Do not benchmark that exact enumerator against 500–2,000-customer production routing solvers. Its role is to be a transparent correctness oracle on bounded cases. For large cases, keep the chosen operational solver (e.g. VROOM or a CP-SAT/MIP formulation) and measure the incremental admission, proof reconstruction and decision-quality cost of SAFRB.

### 11.4 Matched end-to-end benchmark protocol

A publishable comparison must freeze:
- original route network and directed-arc identity;
- travel-time functions, timebase, departure/arrival convention and service windows;
- route and shared-corridor capacity schedules by scenario and effective interval;
- fleet identity, heterogeneity, availability and in-transit semantics;
- scenario set, observation/reveal times and nonanticipativity constraints;
- objective weights, tie-breaking, hardware, solver version, time/memory limits and stopping rules.

Use the same optimization engine in each assurance arm. Do not compare an uncapacitated vehicle-routing instance with a finite-capacity service obligation and call the delta a solver improvement.

The benchmark matrix should have three tiers:

| Tier | Cases | Purpose | Comparator |
| --- | --- | --- | --- |
| A — exact correctness | Exhaustive tiny networks, 1–4 units, adversarial route closures and capacity schedules | Prove semantic correctness, policy nonanticipativity and exact certificate rejection | Independent exhaustive trajectory oracle; hand-derived known answers |
| B — realistic routing | Solomon, Li & Lim, Blauth 2024 and documented local road networks | Route quality, deadlines, fleet use, robustness to real time-dependent travel | VROOM and compatible OR-Tools / CP-SAT / MIP baselines |
| C — assurance under operations | Multi-owner capacity evidence, freshness/revocation, scenario updates, route aliases, beneficiary floors and service restoration | False assurance, decision regret, qualified service and evidence cost | Same industry/optimization planner with and without SAFRB assurance |

Tier A tests correctness; Tier B tests optimization quality; Tier C tests whether evidence/authority and independent verification change operational outcomes. Report them separately, never collapse them into one headline score.

### 11.5 Proposed acceptance targets — not current results

| Metric | Initial target | Qualification rule |
| --- | ---: | --- |
| False GUARANTEED service claims | 0 / 3,000 independent hard negatives | One-sided 95% upper bound is approximately 0.1% only if cases are genuinely independent |
| False rejection of valid positive controls | ≤1% / 300 independent positives | Publish exact confidence interval and stratify by fault family |
| Supported proof verification | 100% | Independent verifier; source/model/receipt mutations must invalidate affected proofs |
| Temporal-capacity leakage | 0 on frozen adversarial suite | Count arc and physical-corridor violations separately |
| Unsupported / indeterminate coverage | 100% correctly surfaced | Never silently turn missing evidence or solver DNF into infeasibility |
| Decision regret reduction | ≥20% vs matched planner-only arm | Same data, solver, scenario and decision budget |
| Assurance overhead | P95 ≤10% of decision-time budget | Report admission, verification and end-to-end P50/P95/P99 separately |
| Unauthorized actions/disclosure | 0 | Negative controls and permission-boundary audit |

Do not count multiple mutations of the same base case as independent samples for the confidence-bound claim. Use independently generated base cases, stratify adversarial families, publish the seed and generator version, and retain failed/DNF runs in the denominator.

### 11.6 Industry and standards positioning

- VROOM is a real routing-engine comparator for VRPTW/MDVRP/PDPTW baselines; its benchmark page publishes reproducible Solomon-family data and historical performance measurements. Use it as a replaceable planner, not as a full infrastructure assurance comparator.
- PyPSA remains a stronger comparator for energy-system optimization and stochastic recourse; SAFRB should demonstrate additional value only where an independently checked cross-owner evidence and beneficiary obligation layer is actually needed.
- AVEVA PI, Siemens Gridscale X and Schneider Microgrid Advisor remain integration environments for operational data, grid planning/operations and DER/microgrid control respectively. Public product descriptions do not prove an assurance gap in any vendor.
- NIST's SI for Manufacturing / TIDAL programme is an important measurement-science and VVUQ comparator. Align the benchmark's reproducibility, industrial testbench and digital-twin credibility methods rather than creating a disconnected second standard/testbed.
- NIST IR 8536 (2026) is the traceability meta-framework comparator. Preserve interoperability and existing standards; use Mycelix for authority, provenance and selective-disclosure bindings.

References:
- Blauth et al., *Vehicle Routing with Time-Dependent Travel Times: Theory, Practice, and Benchmarks*, Discrete Optimization 53 (2024), 100848, DOI 10.1016/j.disopt.2024.100848; [benchmark repository](https://gitlab.com/muelleratorunibonnde/vrptdt-benchmark); [MAMUT conversion and instance details](https://mamut-routing.univ-ubs.fr/docs/benchmarks/families/tdvrptw/blauth2024/).
- Almadhaani & Ji (2026), *Time-Dependent Vehicle Routing Problem with Time Windows: A Systematic Review of Modeling Approaches, Solution Methods and Sustainability Implications*, [Sustainability 18(18), 9557](https://doi.org/10.3390/su18189557).
- [VROOM benchmark protocol](https://github.com/VROOM-Project/vroom/wiki/Benchmarks).
- [DIMACS Network Flows 2.0](https://coral.ise.lehigh.edu/flow-challenge-2-0/) and [instance/generator catalogue](https://coral.ise.lehigh.edu/flow-challenge-2-0/mcf-instances-and-generators/).
- [NIST SI for Manufacturing / TIDAL](https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing); [NIST IR 8536](https://csrc.nist.gov/pubs/ir/8536/final).

## 12. Implementation status — 28 November 2026

The new stacked branch research/safrb-capacity-mobility-v0 now contains:
- the bounded mobility policy oracle from the mobility-recourse research branch;
- complete scenario-specific route and shared-corridor departure capacity overrides, with legacy static inputs retained when no overrides are present;
- a mobility_capacity compiler that binds original admitted Route-role schedules to per-arc and shared-corridor subjects;
- an independent verifier that re-admits the source model and schedules and rebuilds every compiled capacity value from the original input;
- four portable JSON schema documents and README contract notes;
- frozen tests for complete per-period compilation, low-vs-high scenario separation, zero capacity as an outage, schedule gaps/stale/dimensional mismatches, subject/resource mismatches, alias splitting, forged compiled capacity, receipt mutation and schema JSON parsing.

**Qualification is pending.** These source files have not yet been confirmed by exact-head Rust formatting, Clippy or test runs. The tests and schema parse checks are authored, not recorded as passing CI evidence. No SOTA, runtime, field-service or operational dispatch result is claimed.

## 13. Dual world-state / knowledge-state timing

The next assurance step is not merely adding another scenario field. Model two synchronized state machines:

1. **World state** — when a capacity, closure, asset state or service obligation physically applies.
2. **Knowledge/authority state** — when a specific actor observes a fact, when it is admitted, when that actor becomes eligible to use it, when the source revokes it, and when the actor receives an admissible revocation notice.

For a capacity fact, the actor-visible start is:

visible_from = max(observed_at, admitted_at, actor_eligible_at)

The observation history must not contain the fact before this boundary. The world-state capacity, however, constrains physical movement from its effective period in every scenario, whether or not the actor knows the fact.

The actor-visible interval is half-open. A source fact can be withdrawn from the actor's observation history when its effective interval or freshness interval ends, or when an admissible revocation notice becomes eligible to that actor. The revocation notice retains the authority's distinct effective revocation period. Record IDs and scenario IDs must not split otherwise equivalent canonical facts into different information sets.

**A crucial unresolved distinction remains:** the source's revocation can become effective before the actor receives an admissible notice. The observation-history adapter must not pretend the actor knew early; but it also must not authorize use of now-revoked evidence. During that gap the current v0.1 information adapter is not a permission gate. A future policy/certificate layer must either prove a current independent authority status at decision time or return UNSUPPORTED / INDETERMINATE and prevent the affected action from being called qualified. The evidence-time report now emits a typed revocation_notice_gaps array, independently reconstructed from the source timing rows, so this interval is machine-readable and mutation-tested. It must not infer a physical outage or zero capacity from missing authorization.

### 13.1 Evidence-time acceptance suite

| Case | Physical effective time | Actor observation | Admission | Actor eligible | Required result |
| --- | ---: | ---: | ---: | ---: | --- |
| Delayed observation | 4 | 6 | 6 | 6 | Physical capacity applies at 4; no policy split before 6 |
| Delayed admission | 4 | 4 | 5 | 5 | No evidence-based policy split before 5 |
| Delayed actor permission | 4 | 4 | 4 | 6 | No policy split before 6 |
| Future schedule known early | 4 | 2 | 2 | 2 | Policy may plan for the future capacity at 2 |
| Revocation notice delay | 4 | 4 | 4 | 4; notice eligible 6 | Actor learns revocation at 6; effective revocation remains 4 |
| Equivalent facts | same | same | same | same | Different record IDs do not split policy histories |
| Backdated metadata | 4 | 6 | 6 | 6 | Reordering or backdating cannot reveal at 4 |
| Forged lifecycle receipt | any | tampered | tampered | tampered | Independent reconstruction rejects |

Measure **pre-reveal leakage** (must be zero), **pre-admission leakage** (zero), **pre-eligibility leakage** (zero), **stale-evidence reliance after effective revocation** (zero once a permission/basis verifier is implemented), plus physical-capacity violations (zero). Do not combine these into one metric; they test different state machines.

### 13.2 Value-of-information benchmark

For each candidate observation/verification acquisition, measure:
- expected service loss or decision regret without the evidence;
- expected loss/regret with the evidence at the earliest legitimate availability period;
- acquisition and validation cost;
- change in feasible policy space;
- delay introduced by admission and actor-eligibility gates;
- independent verifier overhead.

Report EVSI / acquisition_cost, absolute expected regret avoided, and service-floor failures avoided. Do not assume information is valuable just because it reduces uncertainty; only count the value of information when it changes an admissible decision with positive expected net benefit. The world-state oracle with perfect early information remains a non-operational upper bound, not a deployable policy.

### 13.3 Current implementation ceiling

Branch research/safrb-evidence-time-mobility-v0 adds half-open observation visibility windows and an actor-scoped compiler/verifier for externally supplied observation, admission, actor-eligibility and revocation-notice times. It does not yet enforce an evidence-basis/permission proof for a policy decision during the interval between effective revocation and actor-eligible notice. That missing permission layer remains an explicit blocker to live operational claims.

## 14. Comparator discipline and data licensing (28 November 2026)

### 14.1 Do not claim to beat routing SOTA with the tiny exact oracle

The mobility reference oracle is deliberately small (2–4 sites, 1–4 homogeneous units, at most 8 arcs and 8 scenarios). It is a correctness instrument for exhaustive small instances, not a competitor to high-performance vehicle-routing heuristics. The Blauth et al. TDVRPTW family is a substantially more realistic routing benchmark: 10 cities and four size levels (10, 500, 1,000 and 2,000 customers), using OpenStreetMap road networks and time-dependent piecewise-linear arrival functions based on hourly Uber Movement speeds. Its current delivery-only instances assume an unlimited homogeneous uncapacitated fleet, so it still does not test the finite shared-corridor and evidence-authority semantics SAFRB is trying to qualify. Retain the unmodified original instances as routing baselines, then publish clearly versioned SAFRB extensions rather than silently changing the baseline. See the 2024 research paper and MAMUT benchmark documentation:
- https://www.sciencedirect.com/science/article/pii/S1572528624000276
- https://mamut-routing.univ-ubs.fr/docs/benchmarks/families/tdvrptw/blauth2024/

**Licensing and reproducibility:** the MAMUT satellite describes this benchmark family as CC BY-NC 4.0 / non-commercial use, with distinct third-party data attribution. Check the actual dataset and source repository licenses before redistributing or incorporating it into any commercial offering. The Uber Movement source was discontinued in 2023; preserve the published data as a fixed historical benchmark and do not claim that its traffic matrix can be regenerated from the current public source. The new SAFRB benchmark tier should therefore have both (a) a licensed, frozen historical set and (b) a separately versioned, reproducible public-data generation recipe.

### 14.2 PyPSA is a real recourse comparator, not a straw man

PyPSA's current documentation describes two-stage stochastic optimization with common first-stage investment decisions, scenario-specific second-stage dispatch, scenario probabilities and CVaR risk aversion. Its documented current stage structure is fixed: investment is first-stage and dispatch is second-stage; it does not present arbitrary multi-stage actor-observation and permission timelines as a universal feature. Thus SAFRB must not claim stochastic recourse or nonanticipativity as novel. The fair experiment is to export equivalent physical cases into PyPSA where the energy-system formulation applies, preserve its first-stage/second-stage semantics and compare objective/feasibility under identical scenario information. Then isolate any incremental assurance contribution: source-bound temporal capacity records, independent reconstruction, cross-owner provenance, actor-specific evidence eligibility, decision-specific authority and beneficiary claim receipts.
- https://docs.pypsa.org/latest/user-guide/optimization/stochastic/
- https://docs.pypsa.org/latest/examples/stochastic-optimization/

### 14.3 Industry systems should be integration baselines

Public product descriptions position:
- AVEVA PI System as operational data management: collecting, storing, normalizing, enriching and visualizing real-time operations data;
- Siemens Gridscale X as an interoperable platform connecting grid planning, operations, asset management and metering through a common digital twin;
- Schneider EcoStruxure Microgrid Advisor as microgrid/DER monitoring, forecasting and optimization, including model-predictive control for generation, storage and EV charging.

These are substantial capabilities. The SAFRB experiment should connect to them as data/decision environments and test cross-system evidence continuity, independent claim verification and authority boundaries. It must not infer a vendor capability gap from marketing pages; ask vendors for matched technical evidence and operator-approved deployment access.
- https://www.aveva.com/en/products/aveva-pi-system/
- https://www.siemens.com/en-us/products/gridscale-x/
- https://www.se.com/us/en/product-range/65896-ecostruxure-microgrid-advisor/

### 14.4 NIST alignment is preferable to a competing assurance vocabulary

NIST's 2026 SI for Manufacturing / TIDAL programme describes a layered, human-teamed, digital-twin-assured architecture, a shared integration test bench covering real equipment and simulators, VVUQ, context, latency, cost and grounding metrics, and benchmark-study/tooling work planned in FY28. This is a close metrology and interoperability neighbour, not evidence that a complete public benchmark suite is already available. Map SAFRB receipts and ablations to its VVUQ, context, integration and decision-assurance work; seek a compatible contribution to the shared testbench rather than asserting a parallel standard. NIST's 2026 IR 8536 traceability meta-framework is also a relevant manufacturing provenance comparator.
- https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing
- https://csrc.nist.gov/pubs/ir/8536/final

### 14.5 What would constitute a credible "better than SOTA" result?

There are three distinct claims and they need three distinct scoreboards:

1. **Optimizer performance:** on an unchanged routing/flow benchmark, match the best published feasible objective and report optimality gap, runtime, memory, hardware and solution quality. Be prepared for an established solver to win.
2. **Model/decision correctness:** on the exact bounded suite, outperform baselines on false-feasibility, false-infeasibility, nonanticipativity, temporal-capacity leakage and correct indeterminate/unsupported handling. A zero false-guarantee result is more important than a small runtime gain.
3. **Assurance under operations:** on matched cross-owner cases, show that evidence admission, independent proof reconstruction and current-authority checks reduce decision regret/service failures without unacceptable latency or loss of usable coverage. This is the plausible SAFRB differentiation.

A legitimate research claim must report negative controls, all failed/timeout outcomes, external-oracle dependencies, confidence intervals, and the exact data/model/solver hashes. No headline "beats SOTA" result should combine performance on one benchmark with assurance on a different one.

## 15. FSRE decision-grade monitoring alignment and cross-layer benchmark

The mobility-recourse programme should reuse the existing Mycelix FSRE decision-grade monitoring framework rather than inventing a second, incompatible evidence-quality taxonomy. FSRE already treats the objective as decisions enabled by evidence, with source freshness, actor-availability time, sequential EVSI, service outcomes, equity, operations and independent as-of replay. SAFRB contributes a complementary physical service/policy model: it asks whether the proposed mobility/infrastructure action can physically satisfy declared service floors and whether its evidence basis is valid for the actor and decision cutoff.

### 15.1 Shared metric vocabulary

| FSRE metric family | SAFRB mobility extension | Required denominator / split |
| --- | --- | --- |
| Observation-to-availability latency | Source observation → admission/verification → actor-eligibility → decision cutoff | Report each latency separately; actor-eligible before deadline |
| Freshness and data quality | Capacity/route evidence valid for the entire service decision interval | Evidence records by role, resource and scenario |
| Sequential EVSI | Evidence acquisition that changes the admissible feasible-policy set or decision regret | Total acquisition, laboratory, travel, analyst and verification cost |
| Decision regret | Difference between chosen admissible policy and matched information/decision oracle | Same world model, budget, authority and operator action constraints |
| Service outcome | Safe service-hours preserved, floor-deficit hours, cumulative deficit, recovery and relapse | Each beneficiary/service cohort and geography; preserve unresolved denominators |
| Independence/common cause | Shared issuer, calibration, upstream dataset, sensor, or derived source lineage | Count independent source roots, not record IDs |
| Operations | Decision latency, dispatch workload, route feasibility, evidence turnaround, maintenance | P50/P95/P99, failures and timeouts included |
| Reproducibility | World-state, knowledge-state, evidence-basis and authority-basis replay | Exact hashes, as-known-at-cutoff evidence, independent verifier |

A record arriving before a decision cutoff is not necessarily usable: it may fail admission, be outside the actor's permission or have an effective revocation. A valid observation is not equivalent to a valid evidence basis. Keep the four timing facts (observed, admitted, actor eligible, decision cutoff) and source effective/freshness intervals distinct.

### 15.2 Matched layered ablation

Evaluate the same operator/case distribution under these arms:

| Arm | System | What it isolates |
| --- | --- | --- |
| F0 | Existing operator practice and owner-native monitors | Real-world baseline |
| F1 | Strong route/flow/microgrid planner only | Optimization contribution |
| F2 | Planner + structurally admitted time-varying capacity | Model/evidence-window contribution |
| F3 | F2 + independent physical model/certificate replay | False-feasibility reduction from independent verification |
| F4 | F3 + actor-specific reveal/expiry history | Pre-reveal, pre-admission and pre-eligibility leakage |
| F5 | F4 + current issuer authority/revocation checkpoint | Decision-specific evidence-basis validity |
| F6 | F5 + sequential EVSI acquisition policy | Net information value and evidence-acquisition efficiency |
| F7 | Perfect-information world-state oracle | Non-operational upper bound only |

Use the same physical planner, scenarios, service floors, solver budget, evidence-acquisition budget and action authority across arms. Only add the stated assurance layer. Do not attribute optimizer improvements to the assurance layer or call the oracle deployable.

### 15.3 Operational hard gates and proposed targets

Hard gates:
- Zero service guarantees based on evidence not actor-eligible at the decision cutoff.
- Zero decisions relying on evidence whose authority was already effectively revoked.
- Zero physical-capacity violations, even when the outage is not yet observed.
- Zero unauthorized purpose/resource/scope/actor uses.
- Zero false beneficiary-floor satisfaction claims.
- 100% independent rejection of altered model, plan, source receipt, actor history, evidence basis, permission, revocation or provenance checkpoint.
- 100% retention of UNSUPPORTED / INDETERMINATE where current authority/completeness cannot be established, without translating that uncertainty into physical zero capacity.

Comparative hypotheses (not current results):
- At least 10% lower decision regret than the strongest matched operator/planner baseline at equal lifecycle spend and action authority; 20% is a stretch target for controlled, fully frozen synthetic suites.
- Or at least 10% lower lifecycle cost per independently verified safe service-hour preserved, without worsening any protected cohort floor.
- No critical-event detection sensitivity regression beyond a pre-registered non-inferiority margin.
- P95 evidence-basis and proof-verification overhead remains within 10% of the relevant decision-time budget, with admission, authority lookup and replay costs measured separately.

Use held-out catchments/utilities, event families and time periods where available. Do not use random row splits as the only generalization test. Publish confidence intervals, base-case independence, failed/timeout outcomes and exact source/model/solver hashes.

### 15.4 Mycelix checkpoint contract is a dependency, not a local shortcut

Mycelix PR #6834 fixes local effective-time interpretation and differentiates RecordObserved from NoRecordObserved. It does not establish DHT-wide completeness or that a record-backed query is the newest globally committed issuer state. The follow-on [Mycelix #6840](https://github.com/Luminous-Dynamics/mycelix/issues/6840) specifies versioned issuer revocation checkpoints with scope, monotonic sequence, event digest/root, supersession and freshness.

Until a current checkpoint covering the evidence issuer/scope and decision cutoff can be independently verified, SAFRB must not report that evidence basis as QUALIFIED. Missing, stale, out-of-scope, forked or incomplete checkpoint state yields UNSUPPORTED or INDETERMINATE; it does not imply revoked physical capacity, a route closure or mathematical infeasibility.

### 15.5 Integration boundary and evaluation order

- Mycelix: actor/issuer identity, signed authority, revocation checkpoints, provenance dependency closure and permission/purpose basis.
- Symtropy: physical state, temporal capacity, mobility policy, service obligations, independent reconstruction and decision-proof binding.
- Symthaea: hypotheses, policy proposals and evidence-acquisition candidates under EVSI; it cannot self-certify its observations or permissions.
- Sol-Atlas: spatial/resource/beneficiary identity and access boundaries.
- FSRE: monitoring deployment, operational workflows, service outcomes and equity measurement.
- Operators/lawful authorities: retain procurement, sampling, dispatch and actuation authority.

The order for a decision is: establish a bounded physical model; verify physical capacity and service policy; reconstruct actor-eligible knowledge at the decision cutoff; verify current authority/checkpoint coverage; verify the action and claim scope; then compare expected service outcome, regret and lifecycle cost. A failure at a later evidence/authority stage must not rewrite the earlier physical model.
