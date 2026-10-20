# Subterranean Corridor Benchmark v0.1

A dependency-free Rust reference kernel for fail-closed lifecycle comparison of underground corridor alternatives, including earthquake-resilience evidence gates.

## Offline commands

    cargo fmt --manifest-path tools/subterranean-corridor-benchmark/Cargo.toml -- --check
    cargo test --manifest-path tools/subterranean-corridor-benchmark/Cargo.toml --locked
    cargo run --manifest-path tools/subterranean-corridor-benchmark/Cargo.toml --locked

The crate is an isolated nested Cargo workspace and does not require private Mycelix/Symtropy workspace dependencies.

## Scope

- Common lifecycle NPV and declared externality accounting for equivalent passenger or freight service contracts.
- Embodied construction/refurbishment energy and emissions plus operating intensities.
- Capacity sufficiency, declared source references, hard evidence gates and a multi-objective Pareto frontier.
- Separate geothermal/tunnel-heat matching; unresolved source/sink qualification is never counted as useful delivered heat or a transport-cost credit.
- Explicit seismic gates for site hazard, active fault displacement, liquefaction/lateral spread/slope, lining/interfaces, portals/shafts/stations, and post-event inspection/recovery.
- Experimental functional-recovery curve metrics (`recovery` module): demand-normalized safe service over time, T50/T90/T95 only when sustained, and cumulative unmet service with right-censoring when recovery is not reached.
- Dependency-aware safe capacity (`dependencies` module): common-cause failures invalidate every path relying on the shared dependency; missing/unknown dependency states are not credited, and unresolved capacity that is needed to meet demand blocks a comparison.
- Deterministic constrained-crew repair scheduling (`repair` module): declared task precedence, multi-skill crew pools, resource bottlenecks, and explicit external re-entry authorization that withholds service-ready time unless satisfied.

Every demonstration value is synthetic. The evaluator does not fetch/authenticate sources, model seismic or continuum geotechnical physics, validate the ground, estimate real project cost, certify safety, or authorize excavation/entry/reopening. A supplied gate state is an input assertion, not an engineering conclusion.

See docs/research/SEISMICALLY_RESILIENT_CORRIDOR_BENCHMARK_V0_1.md for the design philosophy, failure-mode register, baselines, evidence ladder, benchmark metrics and research references. See docs/research/SEISMIC_FUNCTIONAL_RECOVERY_V0_2.md for the experimental service-recovery, common-cause dependency and constrained repair-scheduling contracts and limits.