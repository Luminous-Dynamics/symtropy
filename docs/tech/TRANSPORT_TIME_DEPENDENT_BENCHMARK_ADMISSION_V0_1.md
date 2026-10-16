# Time-Dependent Transport Benchmark Admission and Qualification v0.1

**Status:** protocol and Rust arrival-function primitive drafted; no third-party benchmark bytes are vendored here and no performance result is claimed.  
**Implementation:** `crates/domains/symtropy-transport-optimization/src/time_dependent.rs`  
**Related work:** [TRANSPORT-OPT-001 #1601](https://github.com/Luminous-Dynamics/symtropy/issues/1601), [TRANSPORT-ALLOC-002 #1639](https://github.com/Luminous-Dynamics/symtropy/issues/1639), [TRANSPORT-RES-005 #1671](https://github.com/Luminous-Dynamics/symtropy/issues/1671)

## 1. Paradigm shift: benchmark behavior, not only static shortest paths

A static edge cost cannot express congestion by time-of-day, service schedules, delayed departures, operating windows or the fact that leaving later may change which arrival is feasible. A time-dependent routing benchmark must evaluate an **arrival-time function** for each departure, preserve the information that was available at that time, and distinguish route-search error from model/evidence error.

The first Rust primitive implements piecewise-linear **FIFO arrival-time functions**:

- Each breakpoint is `departure_epoch_seconds -> arrival_epoch_seconds`.
- Arrival must not precede departure; departure knots must be strictly increasing; arrival values must be non-decreasing (FIFO).
- Interpolation uses integer arithmetic and rounds arrival upward.
- No extrapolation is allowed outside the declared profile/evidence horizon.
- Publication time, source observation time and repository-ingestion time remain distinct; the function is available only if it was public and present in the replay corpus at the declared cutoff.
- Query time-profile identity must match exactly. UTC Unix seconds and local wall-clock schedules are not silently mixed.
- Unqualified evidence produces an explicit error, not a usable travel time.
- `evaluate_fifo_path` composes legs by carrying each arrival forward and adding explicit per-leg handling/dwell delays. It checks node continuity, repeated-edge use, time profile and cutoff at every leg.

This is deliberately **not** a complete time-dependent VRP solver. It does not handle non-FIFO profiles, scheduled departures/connection waiting, probabilistic travel times, or capacity-coupled path scheduling yet. Explicit `pre_leg_delay_seconds` is a caller-supplied deterministic delay; it is not permission to invent a timetable connection or wait for an unavailable service. These unsupported cases fail or remain out of scope rather than being coerced into the FIFO scalar function.

## 2. External benchmark source inventory

Treat this as a discovery registry. Do not download, copy or redistribute benchmark bytes just because a repository is public. Rights and lineage vary by dataset family and sometimes by component inside a single repository.

| Source | What it contributes | Admission posture |
|---|---|---|
| [MAMUT-routing benchmark catalogue](https://github.com/ANR-MAMUT/MAMUT-routing) | Curated CVRP/VRPTW plus time-dependent TDVRP/TDVRPTW families; arrival-time-function sidecars, pinned digests, and checker-validated best-known solutions. Current public catalogue reports 5,048 instances and 5,048 checked BKS entries. | **First adapter target**, pending per-family rights and format audit. Its project README explicitly describes family-specific source/derivative repositories and licenses; do not infer one license for every family. |
| [BonnTour / Blauth et al. 2024](https://bonndata.uni-bonn.de/catalog/datasets/10.60507/FK2/X22BKR/) | Time-dependent road travel with time windows, real-world-derived road networks and speed profiles, with published solutions. | **Evaluation-only unless terms authorize the intended use.** Catalogue says CC BY-NC 4.0 and records OSM + Uber Movement lineage. Do not include those bytes or derivatives in commercial/distributable corpora without a separate rights review. |
| [VRP-REP](https://www.vrp-rep.org/) | Broad community vehicle-routing instance/solution repository, format specifications and best-known solution sharing. | **Supplementary reference**, not a replacement for time-dependent sidecars, provenance, or per-instance licenses. Qualify each selected family and solution independently. |
| [Public operator/regulator event data](https://pancanal.com/en/maritime-services/advisory-to-shipping/) | Policy and observed operating summaries for prospective/historical transport evaluation. | Keep separate from synthetic VRP instances; require source artifacts, hash, valid/known-at time, clause locator, units and permitted-use status before empirical scoring. |

Research pointers:

- Blauth et al., *Vehicle routing with time-dependent travel times: Theory, practice, and benchmarks* (2024), develops piecewise-linear arrival functions, efficient tour scheduling and benchmark cases where time dependence matters especially with tight windows. [Paper](https://www.sciencedirect.com/science/article/pii/S1572528624000276) · [upstream benchmark repository](https://gitlab.com/muelleratorunibonnde/vrptdt-benchmark).
- The MAMUT-routing project has added a curated catalogue with SHA-256-pinned sidecars and independent BKS checks. Use its pinned snapshot and instance-family metadata rather than treating a changing website page as an immutable benchmark.
- A 2025 transport-network study evaluates resilience through alternative path redundancy under random and targeted edge attacks. Symtropy should report served demand/cut-off demand and recovery curves, not only average path length. [Study](https://doi.org/10.1016/j.seps.2025.102228).

## 3. Canonical local manifest — fields required before an adapter can run

One row per immutable problem instance and every required sidecar:

- dataset ID, family ID, version/release and upstream commit or snapshot ID;
- exact source URL and exact file path/name;
- SHA-256 of every admitted file, byte size, acquired-at time and acquisition method;
- license identifier plus explicit allow/deny/unknown for commercial use, redistribution, modification, and derived artifacts;
- provenance lineage for raw network, speeds/traffic overlays, travel-time profiles, demand, time windows, vehicle/capacity limits and best-known solution;
- time unit, time profile/time-zone convention, departure horizon, interpolation semantics and whether the arrival function is FIFO;
- objective definition, unit, constraints, expected parser schema/version and independent checker version;
- best-known solution value plus the checker artifact/digest, when available;
- qualification status and reviewer evidence; unresolved rights/format/semantic fields must be explicit.

A source URL, README statement, or source digest alone does not prove that a file is authorized, authentic, parsed correctly, or semantically suitable. Do not vendor or transform a family until its specific terms have been reviewed. Benchmark readers must preserve upstream identities and produce local normalized artifacts as separate, hash-pinned derivatives only when permitted.

## 4. Benchmark ladder and target metrics

| Gate | Measurement | Target |
|---|---|---|
| TD0 — arrival-function correctness | Exact endpoints, interpolation, FIFO check, no negative travel duration, profile matching, evidence cutoff, validity horizon, input-order invariance | 100% agreement with hand-computed reference cases; zero silent extrapolation or time-profile coercions |
| TD1 — function checker differential | Compare Rust interpolation against an independent arbitrary-precision/rational reference over generated breakpoint sets and every integer departure in small domains | Zero arrival-time disagreement after the declared ceiling-rounding rule |
| TD2 — published instances | Compare parser results, schedule feasibility and objective calculations against upstream contract and independently checked BKS | 100% parser/schema conformance; 100% best-known solution checker agreement for the admitted subset |
| TD3 — baseline solver | Run published best-known solutions, simple FIFO earliest-arrival/cheapest-feasible baselines and the Symtropy solver on identical instances | Report optimality gap where a defensible optimum/BKS exists, feasible-solution rate, missed windows and runtime distribution |
| TD4 — time-dependence ablation | Same topology/demand with true arrival functions versus an explicitly declared static approximation | Quantify route regret, missed-window changes and objective gap; do not assume time dependence always improves objective |
| TD5 — adversarial replay | Future information, late-ingested traffic record, expired validity, non-FIFO function, missing knot, wrong units/profile, stale/superseded sidecar, checksum mismatch | Zero leakage, implicit extrapolation, fail-open fallback or unreported infeasibility |
| TD6 — robustness and resilience | Paired baseline/strategy runs under time-dependent congestion, closures, delayed transfers, weather and capacity reductions; independent seeds and held-out scenarios | Zero hard constraint violations; report demand served, lateness distribution, p50/p95 end-to-end duration, recovery curve and worst-cohort service |
| TD7 — scale | Instance size, breakpoint count, route-search steps, cache memory, interpolation calls and p50/p95/max wall time on pinned hardware | Establish baseline first; then freeze machine-specific SLOs with confidence intervals, not invented universal thresholds |

For external instance families, record the exact admitted subset, not the headline size of the upstream catalogue. The 5,048-instance headline is source discovery context, **not** a claim that Symtropy has downloaded, licensed, parsed, or run 5,048 cases.

## 5. Required negative controls

1. **Future-data leak:** a traffic profile published after the simulated cutoff cannot affect the route chosen at that cutoff.
2. **Late-ingestion leak:** a public profile absent from the declared corpus until after the cutoff cannot be used in that replay.
3. **Outside horizon:** queries before the first or after the final departure knot fail; no constant-time or nearest-knot fallback.
4. **Non-FIFO input:** a later departure with an earlier arrival is rejected by the FIFO type. A distinct non-FIFO scheduler must be implemented before this data class is supported.
5. **Profile mismatch:** UTC Unix seconds cannot be evaluated as local wall time or as a different calendar/time-zone profile.
6. **Rights failure:** an unqualified data source produces no valid travel time, even when syntactically well-formed.
7. **Hash mutation:** changed source bytes invalidate the manifest/sidecar relation; a digest string cannot replace recomputation over admitted bytes.
8. **BKS mismatch:** published score alone is never accepted without passing the instance-family's feasibility/objective checker.
9. **Static-ablation negative control:** keep network topology, demand and costs fixed while only travel-time dynamics vary, so observed differences are attributable to the time model.
10. **Disruption leakage:** do not use post-disruption speeds/outcomes in a model claiming to act before the disruption was knowable.

## 6. Current implementation limits and next coding gates

Current PR branch adds `FifoArrivalTimeFunctionV1`, with typed evidence metadata, a declared time-profile ID, piecewise-linear interpolation, conservative integer rounding, knowledge-cutoff checks, and Rust unit tests for malformed functions and leakage.

It does **not** yet:
- ingest MAMUT/BonnTour/VRP-REP files;
- authenticate source digests or decide source rights;
- compose multiple edge functions into a path schedule;
- model explicit service departure schedules, waiting, transfer windows or non-FIFO networks;
- solve TDVRP/TDVRPTW instances or report performance.

Next slices: (1) validate this primitive against an independent rational-arithmetic reference; (2) implement a Rust-only, license-gated adapter for one small admitted MAMUT TD instance; (3) add path-level arrival composition + waiting/transfer semantics; (4) compare static and time-dependent baselines with fixed snapshots; (5) combine the time profile with the existing operation-window capacity gate, without merging distinct flow-rate and concurrent-occupancy units.
