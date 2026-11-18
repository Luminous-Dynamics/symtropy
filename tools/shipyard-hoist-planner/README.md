# Shipyard hoist sequence and path planner (experimental V0.1)

This standalone Rust crate is the first executable slice of [SHIPYARD-HOIST-PLANNING-001 #6110](https://github.com/Luminous-Dynamics/symtropy/issues/6110), a child of [SHIPBUILD-CO-DESIGN-001 #2048](https://github.com/Luminous-Dynamics/symtropy/issues/2048).

It jointly searches:
1. the order in which a single gantry crane performs a set of block hoist jobs; and
2. the shortest discrete grid route from the current hook position to each pick cell (empty crane) and from pick to drop (loaded block).

For each route, V0.1 uses deterministic A* with Manhattan heuristic. For the order, it exhaustively enumerates all topological permutations, bounded to six jobs. The objective is total empty-hook plus loaded-hook travel distance in millimetres. Ties are broken lexicographically by job IDs, and the A* queue uses a deterministic cell tie-break.

## Exact owner composition

- Each job must carry a matching `ReadyForHumanReview` input/report receipt from the per-lift preflight owner [#6077 / PR #6107](https://github.com/Luminous-Dynamics/symtropy/pull/6107). This means only that the supplied preflight arithmetic and references are internally consistent under that bounded contract; it is not a lift permit or permission to operate.
- Static ship-block geometry, staging and temporal occupancy remain with [#2050](https://github.com/Luminous-Dynamics/symtropy/issues/2050) and the canonical experimental polygon kernel [#2717](https://github.com/Luminous-Dynamics/symtropy/pull/2717).
- A route from this crate is a nominal discrete candidate. It is **not** a swept volume or a clearance certificate. The existing [motion-envelope producer #5345](https://github.com/Luminous-Dynamics/symtropy/issues/5345), [independent motion oracle #5363](https://github.com/Luminous-Dynamics/symtropy/issues/5363), and [occupancy adapter #5369](https://github.com/Luminous-Dynamics/symtropy/issues/5369) must expand and project actual bounded envelopes before a spatial/temporal conflict result exists.
- Non-anticipative disturbance evaluation remains with [#2734](https://github.com/Luminous-Dynamics/symtropy/issues/2734).

This crate does not add a general scheduler, resource ledger, spatial index, collision engine, trajectory-envelope compiler, kinematics or crane controller.

## Input and conservative cell semantics

The input pins:
- crane ID and exact configuration revision;
- site epoch, frame, monotonic clock domain, workspace revision and clearance-profile revision;
- grid dimensions and integer millimetres per cell;
- separate admitted-free sets for empty and loaded crane travel;
- initial hook cell;
- each block/job identity and revision, pick/drop cells, predecessor job IDs;
- exact preflight block/revision/crane/configuration/site/frame/clock identity and preflight input/report SHA-256 references.

Cells omitted from a free set are **unknown or not admitted**. A* never treats an omitted cell as free. The loaded free set must be a subset of the empty crane free set, and pick/drop cells must be admitted for both profiles.

The planner checks SHA-256 lexical shape and exact preflight subject/context equality. It does not authenticate source signatures, digests, physical measurements or approval authorities. Preflight status is consumed as a typed upstream claim, not recomputed here.

## V0.1 algorithm

For every currently precedence-ready job in lexicographic ID order:
1. run A* through the unloaded-free set from the current hook cell to the job's pick cell;
2. run A* through the loaded-free set from pick to drop;
3. add both route distances and recurse with the new hook position at the drop cell.

The exact search stores the lowest total-distance candidate and uses lexicographic job-order tie-breaking. It rejects cycles and does not silently discard a failed preflight.

A* is optimal for this V0.1 four-neighbour, unit-edge-cost grid because Manhattan distance is admissible and consistent. The production implementation is tested against a separately written breadth-first-search (BFS) reference for every ordered start/goal pair and every obstacle mask on a 3×3 grid: 9,216 route cases. A second independent BFS plus permutation oracle checks the joint sequence/path objective on a tiny multi-job fixture.

## Run

```sh
cargo fmt --manifest-path tools/shipyard-hoist-planner/Cargo.toml -- --check
cargo test --locked --offline --manifest-path tools/shipyard-hoist-planner/Cargo.toml
cargo check --locked --offline --manifest-path tools/shipyard-hoist-planner/Cargo.toml --all-targets
cargo clippy --locked --offline --manifest-path tools/shipyard-hoist-planner/Cargo.toml --all-targets -- -D warnings
```

The authoring environment currently has no Rust/Cargo binary. Tests are authored but are not claimed to have compiled or passed; the exact-head workflow is the execution gate.

## Current limits

V0.1 is a **discrete top-down route and sequence model**, not a three-dimensional crane simulation. It does not model:
- hook height, hoist/reeving geometry, gantry bridge height or vertical lift duration;
- block dimensions beyond the caller's loaded-free-cell profile;
- load swing, pendulation, rope dynamics, acceleration, braking, emergency stop or suspended-load MPC;
- crane structural limits, chart capacity, rigging, load sharing, pick-point strength, ground bearing or weather (these remain preflight/source-owner concerns);
- crane/trolley calendars, time windows, multiple cranes, transfer/settling time, route-time occupancy or dynamic obstacles;
- continuous geometry, polygon swept volumes or 3D clearance.

No path generated here may be sent to a crane. It must first be converted to a conservative time-indexed motion envelope by the existing owner, checked against occupancy, reviewed by the responsible engineers/operators and admitted through the real site authority process.

## Industry and SOTA comparison

Qi et al., *Application of Digital Twin-Based Improved A*HNSA Algorithm to Ship Block Hoisting Tasks*, Journal of Shanghai Jiao Tong University 60(8), 1266–1278 (2026), DOI 10.16183/j.cnki.jsjtu.2025.353, jointly optimises ship-block hoist sequence and gantry-crane path using real Shanghai shipyard scheduling data. The abstract reports 3.51/5.33/6.79% travel-distance and 3.82/5.35/7.07% schedule-time reductions over its selected comparators for small/medium/large cases.

Those figures are literature-only. The complete instance data, comparator configurations, raw per-run outputs and exact constraint protocol must be rights-reviewed and obtained before claiming reproduction or superiority. V0.1 is a correctness-first exact tiny-instance baseline, not yet a competitor to A*HNSA.

Commercial products such as 3D Lift Plan already describe 3D lift simulation, crane selection, configuration-dependent capacity monitoring, rigging design/tension calculations and ground-bearing checks. This planner does not replace those functions. It tests a compositional, evidence-gated sequence/path research layer while delegating preflight, geometry and physical-envelope checks to their existing owners.

## Next gates

1. Exact-head Rust 1.96 fmt/test/check/Clippy.
2. Extend the independent joint oracle with precedence, asymmetrical loaded/empty free sets and route infeasibility cases.
3. Bind generated path segments to #5345 motion-envelope inputs and compare the #5369 occupancy projection without reapplying margins.
4. Add disturbance replay through #2734; past execution events stay immutable and only future jobs are replanned.
5. Reproduce A*HNSA only if source/data rights and matching instance definitions are obtained.

A green synthetic result means only that the discrete planner agrees with its declared tiny oracle and hard input gates. It does not establish physical crane clearance, suspended-load safety, lift authorisation, regulatory compliance or yard productivity.
