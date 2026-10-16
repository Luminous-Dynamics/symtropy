# Robust Transport with Physical-Resource Feasibility v0.1

**Status:** implementation and synthetic Rust tests authored; compile, format, and test execution remain unverified.
**Implementation:** `crates/domains/symtropy-transport-optimization/src/robust_capacity.rs`
**Stacked dependency:** [scenario-robust path selection PR #1714](https://github.com/Luminous-Dynamics/symtropy/pull/1714), which depends on [FIFO arrival functions PR #1701](https://github.com/Luminous-Dynamics/symtropy/pull/1701).
**Research context:** [transport disruption replay issue #1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671).

## 1. Paradigm shift: a route is a resource-qualified plan, not just a path score

Travel-time robustness does not imply operational feasibility. A route can win a minimax comparison and still be impossible if a lock, terminal, crane, rail segment, aircraft position, or other capacity-bearing resource is missing, stale, overcommitted, or misidentified.

This layer composes the existing scenario path selector with the existing half-open, time-windowed capacity validator. It filters out any route that does not pass capacity qualification in every declared scenario, then computes nominal and minimax picks again over the remaining candidates.

This is deliberately **not** a reservation engine, dispatch controller, global multi-demand scheduler, or queueing model. If a capacity limit is exceeded, the candidate is rejected; this version does not invent a wait time or claim that a different departure slot is available.

## 2. Explicit edge-to-resource binding

Every path leg must have at least one resource use for every scenario. A use records:

- the route's logical edge ID;
- a named crosswalk record and a 64-character SHA-256-shaped crosswalk-manifest digest;
- a separately named capacity-bearing resource ID and kind;
- a capacity assignment linked to an exact evidence identity; and
- an operation window that must exactly match the evaluated leg's departure and arrival times.

The function never constructs a physical-resource ID from an edge ID. Matching strings are not prohibited, but the caller must still provide the explicit crosswalk record and manifest. The code validates digest shape and consistency only; the caller's admission boundary must verify manifest bytes, authenticity, scope, provenance and rights.

For one candidate path, the logical-edge → physical-resource mapping, crosswalk identity/digest, and assigned quantity/unit must be invariant across scenarios. A scenario may change capacity evidence or qualification; it may not silently remap the route to a more convenient resource. Missing mapping is a rejection rather than an unlimited-capacity fallback.

The capacity validator uses concurrent assigned units over half-open intervals `[start, end)`. The mapped interval currently covers the evaluated leg's movement/operation interval. Pre-leg delay, terminal queueing, timetable waiting, charging dwell, and any separate handling sub-operation need their own explicit modeled operations before they can be capacity-qualified.

## 3. Qualification and failure semantics

A path is capacity-qualified only if **every evaluated scenario** has:
1. a capacity plan;
2. an explicit resource binding for every path leg;
3. a valid and stable crosswalk mapping;
4. operation windows equal to the evaluated leg intervals; and
5. capacity evidence accepted by the existing freshness, time-profile, validity-window, lineage, unit, resource-identity and overlap checks.

Failed path/scenario pairs remain in the report. Extra plans for an unqualified path or an undeclared scenario, and duplicate path/scenario plans, are input errors. If no travel-qualified candidate passes capacity under all scenarios, the function returns an error; it does not relax constraints or select a least-bad infeasible route.

The output's nominal and minimax scores remain travel-arrival scores over the capacity-qualified candidate set. Capacity qualification itself does not estimate delays caused by competition, predict scenario probabilities, or compare simultaneous use across alternative routes. Separate concurrent shipments require a future joint allocation model.

## 4. Synthetic test coverage authored

The new unit tests cover:
- a travel-time minimax winner that fails capacity in one stress scenario, causing the selector to fall back to the only capacity-qualified route;
- a missing edge-to-resource mapping;
- an operation window that differs from the evaluated leg;
- a physical-resource remapping between nominal and stress scenarios;
- stale capacity observations;
- plan, evidence, and resource-use input-order invariance; and
- duplicate and undeclared scenario plans.

These are synthetic contract checks. No build, test, format, Clippy, runtime, or external-dataset pass is claimed until it is observed at the exact commit head.

## 5. Benchmark ladder

| Gate | Measurement | Acceptance target |
|---|---|---|
| RF0 — binding completeness | Fraction of route legs with explicit resource use in every scenario | 100%; zero implicit edge/resource joins |
| RF1 — crosswalk integrity | Mutate resource mapping or digest between scenarios | Zero inconsistent mappings accepted |
| RF2 — capacity oracle | Compare event-sweep decisions to an independent integer-time reference over exhaustive small windows and demands | 100% exact outcome agreement; zero over-capacity plans accepted |
| RF3 — evidence isolation | Missing, future, stale, expired, conflicted, superseded, wrong-unit and wrong-resource evidence | Zero invalid evidence accepted; rejection reasons retained |
| RF4 — paired-scenario consistency | Permute route candidates, scenario plans, resource uses and evidence records | Identical canonical result |
| RF5 — route selection | Exhaustively enumerate tiny path/scenario/resource combinations | 100% agreement with independent nominal/minimax ranking after capacity filtering |
| RF6 — operational generalization | Compare nominal and robust strategies on pre-registered, permitted historical events | Hold-out event set untouched during tuning; report every event and adverse result |
| RF7 — scale | Vary qualified paths × scenarios × legs × resource uses; record evaluation calls, p50/p95/max latency and memory | Publish a pinned-hardware baseline first; set budgets from measured data rather than guessing |

Mandatory negative controls include: a faster but over-capacity route, two logical edges aliasing one physical bottleneck, a missing crosswalk row, a capacity record arriving after the knowledge cutoff, evidence expiring before the operation ends, units that do not match, adjacent half-open windows, one-second overlap, and a stress scenario that would remap an edge. Rejection controls should be evaluated together with accepted cases so the test set cannot reward a system that rejects everything.

## 6. External benchmarks and research plan

- [MAMUT-routing](https://github.com/ANR-MAMUT/MAMUT-routing) curates classical and time-dependent VRP/VRPTW families, including arrival-time-function sidecars and checker-validated best-known solutions. The catalogue showed a snapshot dated 2026-09-23 with 5,048 instances and 19 benchmark families when inspected for this work. Family-level terms differ; source-code licensing must not be assumed to license every dataset file. Pin a release/commit and record file digests and rights for the chosen family. MAMUT is an appropriate external **time-dependent route** benchmark, but it does not by itself supply Symtropy's physical-resource capacity model.
- Han, Yan, Chi and Park (2025), [The electric vehicle routing problem with travel time and energy consumption uncertainty](https://doi.org/10.1016/j.tre.2025.104211), studies adaptive robust routing under joint travel-time and energy uncertainty. It suggests a future next step: evaluate resource-feasible paths against route-dependent energy uncertainty and charging constraints, without claiming this v0.1 reproduces that method.
- Blauth et al. (2024), [Vehicle routing with time-dependent travel times: Theory, practice, and benchmarks](https://doi.org/10.1016/j.disopt.2024.100848), presents time-dependent routing foundations and benchmark instances that emphasize travel-time functions and scheduling. It is a useful comparison for arrival-time handling, not proof that the current route/capacity composition is globally optimal.

Recommended sequence: finish the independent capacity oracle; pin and rights-review one time-dependent benchmark family; run a transparent baseline plus robust strategy; then add resource/capacity evidence from a separately verified source or declared synthetic scenario model. Keep synthetic stress performance, historical replay performance, and prospective live performance as three distinct claims.

## 7. Limits and next research steps

This layer validates mapped capacity-bearing resources against the given candidate path, scenario plans and evidence. It does not prove the crosswalk is complete for the real transport system, authenticate source issuers, recompute supplied source digests, or verify real-world capacity. A complete-looking input from an untrusted or incomplete adapter is still not proof of physical truth.

Next research steps:
1. Independent exhaustive interval/capacity reference and mutation/negative-control audit.
2. Explicit operation sub-events for transfer handling, charging, queueing and dwell, with typed resource identities and intervals.
3. Joint multi-demand allocation to detect competition across simultaneously proposed consignments.
4. Adaptive robust optimization across travel time, energy, safety constraints, time windows and capacity; separate probabilistic reliability only where held-out data supports calibration.
5. Pareto reporting for arrival time, user/operator cost, energy, emissions and worst-case delay, with lexicographic/fairness constraints declared rather than hidden inside a single opaque scalar.
6. External time-dependent benchmark adapter with license-qualified files, frozen manifests, independent checker outputs and a pinned machine/runtime profile.
