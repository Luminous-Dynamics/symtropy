# Marine Liveaboard Service Vector

Standalone Rust 1.96-compatible validator for Symtropy #6238. It treats a voyage as an occupied, resource-constrained home rather than a route-only optimization problem.

## Command

```sh
cargo fmt --manifest-path tools/marine-liveaboard-service-vector/Cargo.toml -- --check
cargo clippy --manifest-path tools/marine-liveaboard-service-vector/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/marine-liveaboard-service-vector/Cargo.toml
cargo run --manifest-path tools/marine-liveaboard-service-vector/Cargo.toml -- audit-service-vector tools/marine-liveaboard-service-vector/fixtures/synthetic-liveaboard-service-vector-0001.json
```

## Contract

The ten initial axes are arrival delay, energy reserve, freshwater reserve, food reserve, waste capacity, critical-spares coverage, essential-service outage, comfort-breach time, crew workload and purchased energy per occupied berth-day. Each axis has a physical unit, non-compensatory floor, measurement-coverage threshold and evidence digest.

The validator requires identical mission snapshot identities, craft configuration, occupancy, initial resource state, plan, forecast information set, hard-gate roster and operating envelope across policy arms. It requires three distinct arms: weather-blind, weather-informed/resource-blind and integrated mission candidate.

No weighted average or scalar aggregate is calculated. Reports contain per-axis differences and Pareto dominance/incomparability. The synthetic fixture deliberately makes the candidate incomparable to its baseline: better on several axes, worse on arrival delay, food reserve and crew workload.

Any candidate hard-gate violation, essential service/reserve floor breach or inadequate measurement coverage blocks the comparison. Unknown/missing resource dimensions are invalid. Silent constraint relaxation is invalid; a separately authorized soft relaxation requires a nonempty authority receipt and its own new operating-envelope digest and still becomes a distinct, separately evaluated policy—not the original compliant candidate.

## Evidence limits

- The synthetic fixture must remain `SYNTHETIC_ONLY`.
- The validator checks declared evidence digests and shared identity; it does not authenticate sources, verify physical sensor quality or define medically/legally safe values.
- Floor values in the synthetic example are demonstration parameters only. A real contract requires owner/crew agreement, qualified system owners, applicable law, measured vessel configuration and documented uncertainty.
- No seaworthiness, stability, water safety, food sufficiency, waste legality, marine-safety or navigation-authority claim is made.
