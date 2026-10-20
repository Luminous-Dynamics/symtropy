# AIRSPACE-OPS-001 — Evidence-bounded 4D trajectory conflict benchmark

**Status:** synthetic research prototype / contract v0.1  
**Execution owner:** Symtropy  
**Evidence reasoning:** Symthaea  
**Spatial context:** Sol-Atlas  
**Provenance and access control:** Mycelix  
**Operational claim ceiling:** no ATC, UTM, clearance, flight-control, dispatch or real-world separation authority.

## 1. Paradigm shift

Move aerial planning from a nominal line drawn on a map to a time-dependent, evidence-bearing trajectory scenario. The benchmark should expose time windows, reference frames, vertical datums, uncertainty, criteria validity, vehicle-profile applicability, capacity constraints and missing evidence. A trajectory is not safe because it looks separated in a visualization; a conflict-free synthetic result is not an operational clearance.

The first isolated Rust crate lives at tools/airspace-trajectory-audit. It audits a pair of piecewise-linear, local-Cartesian trajectories against explicit scenario-supplied criteria. It does not import live rules or define universal horizontal/vertical minima.

## 2. Geometry and algorithm contract

A trajectory is an ordered list of (UTC Unix milliseconds, x metres, y metres, z metres) points. Horizontal coordinates must share one explicitly named local Cartesian reference frame; vertical coordinates must share a declared datum. Adjacent points define straight segments. Every trajectory carries constant, conservative horizontal-radius and vertical-error bounds for this prototype.

For each overlapping pair of segments:
1. Intersect both segments' time spans with the declared criteria validity interval.
2. Interpolate each path to the clipped interval endpoints.
3. Add both trajectory uncertainty bounds to the supplied horizontal and vertical criteria.
4. Determine the normalized time interval in which the absolute vertical centerline separation is within the expanded vertical bound.
5. Minimize horizontal centerline separation over that vertical-compatible interval.
6. Emit a conflict event if both expanded criteria are violated at the same time; sort/deduplicate events deterministically.

The result is based on a piecewise-linear model and simple conservative uncertainty tubes. It does not include aircraft kinematics, acceleration/climb/turn constraints, geodesic motion, wind, sensor fusion, surveillance association, or the legal semantics of live airspace. A missing or invalid criteria window cannot be treated as permission.

## 3. Status semantics

- ConflictDetected: a possible conflict was found inside the interval covered by the declared criteria. This is a screening event, not a command.
- NoTemporalOverlap: the declared trajectories have no simultaneous timestamp.
- NoConflictDetectedWithinEvaluatedWindow: no conflict was found, and the criteria cover the entire overlapping trajectory interval.
- NotAssessable: no conflict was found in the evaluated portion, but the criteria fail to cover the entire time overlap.

Malformed references, digest syntax, time ordering, frame mismatches, vertical-datum mismatches, invalid uncertainty, or numeric overflow return typed errors. The report's source-authenticity flag always remains false; digest syntax and references do not authenticate data, source authority or applicability.

## 4. Benchmark plan

### Mathematical correctness
- 100% agreement against a separately implemented exact/analytic oracle across an enumerated small synthetic grid of linear 3D paths and time windows.
- Zero false-safe results on known conflict fixtures, exact horizontal/vertical tangencies, single-instant overlaps, uncertainty-expanded conflicts, partial criteria coverage, malformed point times and frame/datum mismatches.
- Separate true conflict, no temporal overlap, full-coverage no-conflict, and not-assessable outcomes; never hide a known conflict when data coverage is incomplete.
- Property-based and mutation tests for waypoint insertion, trajectory argument reordering, boundary timestamps, non-finite coordinates, arithmetic overflow, stale/partial criteria and altered profile revisions.
- Verify result determinism under equivalent candidate ordering and stable event deduplication.
- The integration suite includes three metamorphic checks: pair-order reversal, collinear waypoint subdivision, and uniform time-origin translation. It now also enumerates 72,600 sloped-segment cases and computes expected closest approach using integer-rational squared-distance arithmetic, independent of the production floating-point minimizer. Together with the 9,225 constant-relative-position cases and exact boundaries, this is an 81,825-case finite synthetic oracle suite plus metamorphic checks. It narrows but does not close the unbounded floating-point near-tangency proof gap; high-precision/interval analysis and adversarial magnitudes remain required.

### Broadphase acceptance gate (new implementation slice)

The isolated trajectory kernel now has `broadphase_candidate_pairs`, a sweep-and-prune filter over time-overlapping, outward-expanded trajectory AABBs. Its contract is intentionally restrictive: one shared local Cartesian frame and vertical datum, scenario criteria covering the fleet's full time span, outward-rounded margins, and fail-closed behavior if extents cannot be represented finitely. The base separation threshold is applied on each trajectory box, which is conservative but may inflate the candidate set. The broadphase is only a filter; every candidate still needs the pairwise geometry audit.

The new frozen 96-trajectory synthetic test enumerates every brute-force pair and asserts that every `ConflictDetected` pair survives candidate generation. It also verifies pruning versus all-pairs enumeration and checks rejection of incomplete criteria windows and mixed frame/datum inputs. This is a contract test, not a benchmark claim. A release-mode benchmark binary now emits deterministic CSV for 10/100/1k/10k paths (11 repetitions each), includes known-conflict checks, and reports candidate reduction and p50/p95 runtime. These readings will be hardware-specific; they are not a SOTA comparison by themselves. On the current validated head, the hosted Linux runner reported 10k candidate-generation p50 3.017 ms and p95 4.123 ms, retaining 5,000 candidates out of 49,995,000 possible pairs in the synthetic paired-path fixture. The broadphase test and exact-head CI passed; the host is not pinned and the timings exclude narrow-phase work. The CI run logs the measurements. For external comparison, publish compiler/CPU/memory details and a frozen workload before interpreting results. Before scale claims, publish per-size (10/100/1k/10k) candidate recall, candidate count, false-positive ratio, p50/p95 runtime, memory, and brute-force reference runtime on a pinned host. Require 100% recall against the brute-force oracle; do not average a missed conflict away in a general quality score.

### Model credibility
- Add negative controls where a nominal centerline misses but the uncertainty-expanded tube conflicts.
- Add out-of-domain cases: sharp turns, implausible velocity/acceleration, discontinuous observations, non-local coordinate frames, unknown vertical datum, stale criteria and missing segments must not be presented as physically qualified.
- Keep geometry calculation, path feasibility, data quality, legal applicability and operational authority as separate claims.

### Planning-level comparisons
- Compare pairwise screening and later fleet selection against first-come-first-served, shortest feasible path and capacity-aware time-expanded baselines only on matched synthetic cases.
- Report false positives and false negatives separately; false-safe rate is a hard gate, not an averaged objective.
- Benchmark declared sizes (10, 100, 1,000 and 10,000 trajectories) with p50/p95 latency, memory, segment-pair count and scenario coverage. Performance targets are set only after a pinned-machine baseline; do not trade away conservatism for throughput.

## 5. Cross-repository architecture

| System | Owns | Must not claim |
|---|---|---|
| Symtropy | Synthetic trajectory execution, deterministic conflict screening, resource/time-window scenarios and replay | Real-time separation assurance, legal minimums, control authority or clearance |
| Sol-Atlas | Versioned read-only airport/airfield, airspace geometry, terrain/obstacle, weather-source coverage and ground-connection context | That mapped presence proves operational status, permission, service or capacity |
| Symthaea | Evidence contradiction checks, model applicability analysis, abstention and multi-objective planning advice | That cognition/reasoning is flight control or authoritative traffic management |
| Mycelix | Access-controlled source/configuration provenance, operator credentials and review receipts | That popularity, consensus or hashes prove physical truth, approval or safety |

Cross-links:
- Symtropy [AIRSPACE-OPS-001 #2279](https://github.com/Luminous-Dynamics/symtropy/issues/2279)
- Sol-Atlas [AIRSPACE-ATLAS-001 #189](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/189)
- Symthaea [LOADMASTER-001 PR #8185](https://github.com/Luminous-Dynamics/symthaea/pull/8185) and [LOADMASTER-002 #8187](https://github.com/Luminous-Dynamics/symthaea/issues/8187)
- Mycelix [AeroCommons #3638](https://github.com/Luminous-Dynamics/mycelix/issues/3638) and [reproducibility challenge #3643](https://github.com/Luminous-Dynamics/mycelix/issues/3643)
- Symtropy's existing [multimodal whole-life transport contract #1601](https://github.com/Luminous-Dynamics/symtropy/issues/1601) and [mission-chain audit PR #1887](https://github.com/Luminous-Dynamics/symtropy/pull/1887)

## 6. Official research anchors

- FAA, [UAS Traffic Management](https://www.faa.gov/uas/advanced_operations/traffic_management): an ecosystem for qualifying unmanned operations that is complementary to, not equivalent to, traditional air traffic services.
- FAA, [Aeronautical Information Services](https://www.faa.gov/air_traffic/flight_info/aeronav/): authoritative aviation information acquisition must be jurisdiction-, product-, revision- and use-specific.
- NASA Langley, [Formal Methods Research Program](https://shemesh.larc.nasa.gov/): formal methods and assurance research relevant to high-consequence aerospace software.
- FAA, [Aircraft Certification](https://www.faa.gov/aircraft/air_cert/design_approvals/aircraft_certification): simulation/analysis does not replace the regulated certification process.

## 7. Exclusions and release gates

No live flight plans, surveillance feeds, actual NOTAM or weather updates, operational separation thresholds, cockpit/ATC interface, automatic rerouting, dispatch authorization, or flight-control commands. The first dataset is synthetic and deliberately fictional. The crate must remain unconnected to operational systems.

Before even evaluating any real-world planning application, require a separately reproduced mathematical oracle, independent aviation-domain review, authoritative and licensed data sources, verified frame transformations, qualified trajectory/uncertainty models, system-level assurance evidence and all applicable regulatory/operator approvals.