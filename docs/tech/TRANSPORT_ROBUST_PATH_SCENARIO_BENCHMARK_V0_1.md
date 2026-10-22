# Scenario-Robust Time-Dependent Path Selection v0.1

**Status:** deterministic minimax path selector and synthetic unit tests authored; not yet compiler/test verified.  
**Implementation:** `crates/domains/symtropy-transport-optimization/src/robust_path.rs`  
**Dependency:** [time-dependent arrival functions PR #1701](https://github.com/Luminous-Dynamics/symtropy/pull/1701)  
**Research issue:** [TRANSPORT-RES-005 #1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671)

## 1. Paradigm shift: compare strategies across the same future conditions

A nominally quickest route is not necessarily the most resilient route. A path that uses a fast but fragile corridor may perform much worse when that corridor is disrupted. Instead of publishing just one route score, this prototype evaluates each supplied candidate path across the **same frozen scenario set**, then returns two explicit decisions:

- **Nominal best:** smallest final arrival time in the scenario explicitly marked `Nominal`.
- **Minimax best:** smallest worst-case final arrival time across every declared scenario.

All scenario paths for one candidate must preserve the same ordered edge/node topology. Travel-time functions and explicit delays can vary by scenario. Each scenario result retains per-leg source identity, source digest, time profile, departure/arrival and elapsed time.

The selector is exact only over its caller-supplied paths and declared finite scenarios. It does not prove that route generation found every alternative, and it does not infer real-world scenario probabilities.

## 2. Scenario semantics

Every scenario has a stable ID and one kind:

- `Nominal` — exactly one reference condition.
- `Stress` — a declared deterministic adversarial/stress case; it carries no probability or observed frequency.
- `ObservedReplay` — a specific historical case with separately qualified source provenance.

The scenario set includes an immutable manifest SHA-256 and time-profile identity. The digest is an asserted link to an external manifest; this module checks digest shape but does not recompute source bytes or qualify rights.

Every candidate must include exactly one path for every scenario in the set. Missing or undeclared scenarios are rejected rather than silently removed. A candidate that cannot be evaluated with qualified, in-horizon evidence in *any* scenario is marked as rejected for that scenario and cannot win the robust comparison. If no candidate is qualified over the full set, selection fails closed.

The finite set is **not a statistical sample by default**. Do not label its maximum as an extreme quantile, assign likelihood to hand-crafted failures, or claim population-level reliability. Percentiles require a separately justified sampling protocol, calibrated weights/probabilities and a held-out outcome set.

## 3. Synthetic check

The current unit fixture creates two routes and three paired scenarios:

| Candidate | Nominal travel time | Worst scenario travel time | Nominal rank | Minimax rank |
|---|---:|---:|---:|---:|
| Route A | 100 s | 500 s | 1 | 2 |
| Route B | 130 s | 200 s | 2 | 1 |

With the departure at second 100, Route A arrives at 200 in nominal conditions but at 600 in the disruption scenario. Route B arrives at 230 nominally and no later than 300 across its three declared scenarios. The minimax selector therefore chooses B. This demonstrates a mechanism only; the times are synthetic and not a performance or benefit claim.

The test suite also checks scenario-order and candidate-order invariance, preservation of path rejection when a future-observed profile leaks into a scenario, missing/unknown scenario rejection, topology and common origin/destination consistency, and manifest/nominal requirements. Nine synthetic robust-selector tests are authored, including an independent exhaustive reference over 64 toy candidate paths and all declared scenarios; they have not yet been run.

The 64-path reference test enumerates nominal and two disruption travel times in a tiny integer domain, computes nominal and worst-case scores with a separate slow score table, and checks the production selector against both chosen IDs and every reported score. This validates the finite minimax selection contract for that fixture space only; it is not a runtime benchmark or evidence of real-world route benefit.

## 4. Benchmark ladder

| Gate | Benchmark requirement | Target |
|---|---|---|
| SR0 — determinism | Permute candidate and scenario insertion order; pin all inputs and manifest | Identical canonical report and selections |
| SR1 — finite-domain minimax oracle | Exhaustively score all supplied candidate paths under all supplied scenarios using an independent slow reference | 100% selected-path agreement; exact worst-case score and scenario tie-break |
| SR2 — evidence isolation | Future-published, future-observed, late-ingested, expired, wrong-time-profile or unqualified evidence | Zero invalid scenarios accepted; rejected path/scenario pairs remain inspectable |
| SR3 — topology/coverage | Missing scenario, undeclared scenario, duplicate identity, route topology changes | Zero partial-scenario comparisons; no result represented as complete when a scenario is missing |
| SR4 — stress-set sensitivity | Sweep the same candidate set through increasing congestion, corridor losses, transfer delays and resource-capacity cuts | Report per-scenario outcomes and worst-case service; no probability or causal claim from hand-picked cases |
| SR5 — empirical event replay | Use time-frozen inputs and observed, permitted pre-event/through-event records | Paired comparisons against declared baselines; held-out events reserved before tuning |
| SR6 — scale | Candidate-path count × scenario count × breakpoints/leg; record evaluation calls, p50/p95/max runtime and memory where measured | Capture pinned-hardware baseline first; derive budgets after repeatable measurements |

## 5. Mandatory disruption scenario families

1. **Corridor degradation:** time-dependent travel increases on one or several physical links, without changing the declared graph identity.
2. **Corridor closure:** a path is explicitly infeasible and rejected; no invented constant-time fallback.
3. **Transfer delay:** add an explicit source-qualified or scenario-declared deterministic handling/dwell delay; do not invent a scheduled departure.
4. **Capacity interaction:** paired scenario evaluation must later be composed with the existing operation-window capacity checker; faster arrival does not imply resource availability.
5. **Source freshness:** evidence validity ends before a scenario's planned departure.
6. **Knowledge cutoff:** profile publication, observation or repository-ingestion time occurs after the simulated planning cutoff.
7. **Non-FIFO input:** rejected by the FIFO type unless a separate schedule/waiting model is implemented and independently tested.
8. **Alternative corridor failure:** multiple paths share an upstream/downstream chokepoint. This current path selector alone does not establish physical resource identity; use a canonical resource crosswalk and separate capacity validator.

## 6. Research context

- Han et al. (2025), [*The electric vehicle routing problem with travel time and energy consumption uncertainty*](https://www.sciencedirect.com/science/article/pii/S1366554525002522), examines uncertainty in both travel time and energy consumption with adaptive robust optimization and route-dependent uncertainty sets. Symtropy's current slice isolates deterministic travel-time stress-set comparison; it is not a reproduction of that algorithm and does not yet model EV energy uncertainty.
- The [MAMUT-routing benchmark catalogue](https://github.com/ANR-MAMUT/MAMUT-routing) lists time-dependent VRP/VRPTW families and source-specific provenance/licensing. Its displayed snapshot is dated 2026-09-23. Select a permitted family, pin a release/commit, verify per-file digests and compare with the dataset's checker before reporting external benchmark results. The Blauth2024 family has non-commercial/Uber lineage; other OSM-derived families have family-specific terms. Do not assume the code and every benchmark artifact share a license.
- Historical-disruption replay should follow [TRANSPORT-RES-005 #1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671): freeze pre-event knowledge, preserve evidence arrival chronology, compare strategies on paired inputs and retain adverse results.

## 7. Current limits and next gates

The current Rust slice selects a nominal and minimax candidate over caller-supplied route topologies. It does **not** generate routes, prove candidate completeness, couple paths to time-windowed resources, estimate scenario probabilities, calculate EV energy, or establish operational benefit.

Next:
1. Test minimax choice and worst-scenario reporting against a separately implemented exhaustive oracle over generated tiny scenario sets.
2. Add explicit feasibility outcomes for closed edges/resources, rather than encoding closures as arbitrarily large travel times.
3. Join selected path-leg intervals to physical-resource IDs, evidence validity and capacity; fail closed when the crosswalk or capacity is unknown.
4. Admit one rights-qualified, hash-pinned benchmark family; report nominal-vs-minimax results side by side against a published checker and simple baselines.
5. Only after enough independently observed events exist, assess generalization and calibrated reliability. Do not use stress-set success as proof of likely real-world performance.
