# Adaptive Robust Transport Policies with Non-Anticipativity v0.1

**Status:** implementation and synthetic tests authored; no compilation or execution pass is claimed.
**Implementation:** `crates/domains/symtropy-transport-optimization/src/adaptive_policy.rs`
**Stacked dependency:** [PR #1721 — joint robust route portfolios](https://github.com/Luminous-Dynamics/symtropy/pull/1721), [PR #1718 — physical-resource feasibility](https://github.com/Luminous-Dynamics/symtropy/pull/1718), [PR #1714 — finite-scenario minimax paths](https://github.com/Luminous-Dynamics/symtropy/pull/1714).

## 1. Paradigm shift: choose a policy, not necessarily one route forever

A static minimax selector chooses a path before the scenario is known and makes that same route survive every scenario. This can be overly conservative when the operator can observe a closure, delay, or capacity change before committing to the trip: take route A when the corridor is healthy and route C after the disruption is observed.

The new layer represents that decision with explicit scenario information sets. Scenarios in the same information set are observationally indistinguishable at the decision point and must get the same path choice. Different information sets can make different choices. The route alternatives may be available in only a subset of scenarios; the route can be selected in an information set only if it is qualified in **every** scenario in that information set.

This is non-anticipativity for one recourse stage. It is not full multistage stochastic programming: there is no general scenario tree, sequential state transition, probability model, queueing simulation, or endogenous revelation process yet.

## 2. Input contract

A frozen problem declares:
- a scenario-set ID and SHA-256-shaped manifest digest;
- a shared time profile, departure instant, knowledge cutoff, evaluation instant, and capacity-freshness policy;
- scenario IDs and kinds; and
- each scenario's information-set ID and information-available timestamp.

Each demand supplies route alternatives. An alternative gives one or more scenario-specific route estimates plus explicit edge-to-resource crosswalk bindings and capacity evidence. Missing route/scenario records mean unavailable, not zero travel time or unlimited capacity.

The selector validates that the path estimate fits the frozen time/profile/knowledge contract; each path leg has an explicit crosswalk; the crosswalk is stable across the alternative's supplied scenarios; assigned intervals match the route leg's evaluated departure/arrival interval; and the existing scheduled-capacity validator accepts the evidence. It then checks shared capacity jointly across all simultaneously selected demands in each scenario.

At a policy information set, the same route ID is selected for every scenario in that set. Capacity feasibility is checked scenario-by-scenario after the policy choices are assembled. The function never assumes that individual route feasibility implies joint portfolio feasibility.

## 3. Optimization objective and baseline comparison

The bounded exact objective is lexicographic:
1. minimize the slowest individual demand's elapsed time over all scenarios;
2. minimize the maximum scenario's weighted aggregate elapsed time;
3. minimize the nominal weighted aggregate elapsed time; and
4. choose stable demand, information-set, and path IDs for deterministic ties.

Objective weights are explicit input policy and do not imply probabilities or a universal welfare/fairness definition.

The report also finds the best **static baseline** within the same supplied options: for each demand, the path must remain the same across every information set. It reports the static baseline's worst individual elapsed time and the adaptive policy's improvement on that metric. This is a paired, synthetic objective comparison—not evidence of real-world benefit. It compares only supplied options and does not generate routes.

## 4. Synthetic examples authored

### Route adaptation after observation

One demand has:
- route A available only in the nominal/pre-alert case: 100 seconds;
- route B available in both cases: 130 seconds nominal, 150 seconds after the closure;
- route C available only after the closure: 120 seconds.

The expected policy is A before the alert and C after the alert. Its worst elapsed time is 120 seconds versus the best supplied static policy's 150 seconds, for a **synthetic 30-second improvement**. These are fixtures, not measured performance data.

### No anticipatory choice

If nominal and delayed scenarios share the same information-set ID, a route available only in the nominal scenario cannot be selected for that information set. The route must be qualified in both scenarios; the same choice then applies to both.

### Shared capacity remains binding

Two demands can choose different information-set-dependent paths, but the route policy is rejected if their combined use overbooks the same resource in any scenario. Scenario adaptivity never overrides resource feasibility.

The authored Rust tests cover these cases, jointly shared-resource selection, and explicit refusal when the full policy cartesian product exceeds its maximum combination budget. They have not been run.

## 5. Benchmark ladder and target thresholds

| Gate | Benchmark | Target |
|---|---|---|
| AP0 — information correctness | Place two or more scenarios in the same information set and make a fast path unavailable in one | Zero anticipatory choices; identical path choice within every information set |
| AP1 — adaptive value oracle | Enumerate small finite problems and compare all information-set policies with an independent policy-table reference | 100% optimal-policy agreement on every bounded fixture |
| AP2 — static baseline | Compare best adaptive policy with best feasible supplied static policy on the same data/scenarios | Report per-scenario scores and static gap; never claim a positive gain unless it is observed |
| AP3 — route availability | Remove a path from one scenario in an information set | Zero policies that choose it for that group |
| AP4 — joint resource capacity | Make scenario-adaptive choices collide on shared capacity in one scenario | Zero policy portfolios selected if any scenario overbooks a resource |
| AP5 — evidence isolation | Future, stale, expired, conflicted, wrong-profile, wrong-unit or unqualified resource evidence | Zero invalid records accepted; failure witnesses preserved |
| AP6 — determinism | Permute scenario, demand, route, evidence and resource-binding input order | Identical decisions, score vectors, and ordered failure witnesses |
| AP7 — search bounds | Exact combination product at and just above the configured cap, plus checked arithmetic limits | Exact result only within the cap; explicit error outside it; overflow fails closed |
| AP8 — out-of-sample validation | Tune on pre-event data, evaluate on separate event holdouts, and compare adaptive/static policies with identical evidence cutoffs | Report outcomes per event; no leak from future evidence; disclose failures and static comparisons |
| AP9 — operational scale | Vary demands × routes × information sets × scenarios × capacity intervals | Publish pinned-hardware throughput, p50/p95/max latency, memory, and exhaustive-search limits before setting budgets |

Negative controls must include scenario information sets that are identical despite differing hidden ground truth, a path only available after a later reveal, a deliberately inconsistent resource crosswalk, evidence published or ingested after cutoff, overlapping operations exceeding capacity, a no-feasible-policy case, duplicate scenario and route IDs, and product size exceeding the search cap. Include successful cases too so an always-rejecting solver cannot pass.

## 6. Research context and what is not yet solved

- The time-dependent routing paper [Vehicle routing with time-dependent travel times: Theory, practice, and benchmarks](https://doi.org/10.1016/j.disopt.2024.100848) develops algorithms for piecewise-linear arrival-time functions and publishes time-dependent benchmark instances. It is a suitable baseline for arrival-function correctness and route computation; it does not by itself certify adaptive recourse or shared-resource feasibility.
- [Partially adaptive multistage stochastic programming](https://doi.org/10.1016/j.ejor.2024.09.034), published in 2025, studies decisions that are revised at selected stages and emphasizes non-anticipativity constraints and decomposition. It supports the principle behind explicit information partitions, but this implementation is a smaller single-stage deterministic finite-scenario selector rather than a reproduction of that method.
- [Robust Data-Driven Vehicle Routing with Time Windows](https://doi.org/10.1287/opre.2020.2043), published online in 2021, uses empirical travel-time distributions and a Wasserstein ambiguity set to mitigate out-of-sample lateness risk. Symtropy's current selector does not estimate probability distributions or ambiguity sets; stress scenarios remain unweighted unless a separately specified future model justifies otherwise.
- [Delay-resistant robust vehicle routing with heterogeneous time windows](https://doi.org/10.1016/j.cor.2024.106553) studies robust routing that explicitly hedges against travel/service delays. It offers a useful objective comparison beyond minimum arrival time, but the current policy objective does not yet model appointment-window tightness or service fulfillment risk.

The practical validation sequence is: establish the independent tiny policy oracle; pin one permitted time-dependent routing benchmark family and checker; pair static and adaptive policies on identical route options/scenario/evidence inputs; then use event-held-out replay. Benchmark file rights must be reviewed per family, and no data license should be inferred from the benchmark code license.

## 7. Important limits and next work

This version makes one route choice per demand at a declared information set. It does not allow a route decision after every en-route event, optimize a full scenario tree, model current route progress, estimate disruption probabilities, account for queue-induced waiting, schedule existing external bookings, or authenticate the physical-world truth of source records.

Next:
1. Generalize information sets to a validated nested scenario tree with explicit observation times and state transitions.
2. Add a limited recourse model for queueing, scheduled departures, service windows and route-switch costs—never inventing waits for unavailable resources.
3. Add a separate calibrated-risk mode only when permitted historical samples support a documented probabilistic or distributionally robust interpretation.
4. Compare adaptive policy value, worst-case service and operational switching costs against static robust, nominal, and published baselines using the same held-out evidence.
5. Replace exhaustive search with an exact decomposition/branch-and-bound solver only after it can be cross-checked against this bounded reference implementation.
