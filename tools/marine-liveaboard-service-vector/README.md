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

The ten initial axes are arrival delay, energy reserve, freshwater reserve, food reserve, waste capacity, critical-spares coverage, essential-service outage, comfort-breach time, crew workload and purchased energy per occupied berth-day. Each axis has a physical unit, vessel/mission-specific non-compensatory floor, measurement-coverage threshold, evidence digest, measurement method/class, measurement time window, and lower/upper uncertainty bounds. The exact floor contract is canonicalized and bound to the mission snapshot; the validator does not impose generic vessel thresholds. A `gte` floor is tested against the lower bound; an `lte` floor is tested against the upper bound, never just the point estimate. Each mission also pins an explicit constraint registry: hard-gate IDs and the only soft-preference IDs eligible for a separately authorized relaxation. A hard-gate ID cannot be relabelled as soft, and a relaxation's prior value/unit must match the registered baseline.

The validator requires identical mission snapshot identities, craft configuration, occupancy, initial resource state, plan, forecast information set, hard-gate roster and base operating envelope across comparable policy arms. Each policy must carry a hard-gate result reference and digest; a bare pass/fail string is insufficient. It reports each axis with its point estimate, conservative interval, measurement window, coverage, method/class and evidence digest. It requires three distinct arms: weather-blind, weather-informed/resource-blind and integrated mission candidate.

No weighted average or scalar aggregate is calculated. Reports contain per-axis differences and Pareto dominance/incomparability. The synthetic fixture deliberately makes the candidate incomparable to its baseline: better on several axes, worse on arrival delay, food reserve and crew workload.

Any candidate hard-gate violation, essential service/reserve floor breach or inadequate measurement coverage blocks the comparison. Unknown/missing resource dimensions are invalid. Silent constraint relaxation is invalid; a separately authorized soft relaxation requires a nonempty authority receipt and its own new operating-envelope digest and still becomes a distinct, separately evaluated policy—not the original compliant candidate.

## Evidence limits

- The synthetic fixture must remain `SYNTHETIC_ONLY`.
- The validator checks declared evidence digests and shared identity; it does not authenticate sources, verify physical sensor quality or define medically/legally safe values.
- Floor values in the synthetic example are demonstration parameters only. A real contract requires owner/crew agreement, qualified system owners, applicable law, measured vessel configuration and documented uncertainty.
- No seaworthiness, stability, water safety, food sufficiency, waste legality, marine-safety or navigation-authority claim is made.


## Uncertainty and measurement time

The service-vector auditor requires every metric to declare a measurement window and a finite uncertainty interval containing its point estimate. Essential resource floors are evaluated conservatively: lower confidence/uncertainty bound for minimum reserves, upper bound for maximum outage, delay, comfort breach, workload and purchased-energy quantities. A point estimate that passes while its conservative bound breaches the floor is not accepted.

Measurement classes are explicit (`measured`, `modeled`, `derived`, `synthetic`). Synthetic records cannot be relabelled as measured campaigns. Coverage below the service contract minimum blocks the candidate. The fixture uses 24-hour synthetic projection windows and illustrative ±0.1 intervals only; these bounds are not calibrated confidence intervals.

Pareto dominance is also conservative: candidate worst-case bounds must be at least as good as baseline best-case bounds on every axis to claim dominance. Otherwise report incomparability and preserve all axis deltas. The engine never computes a scalar utility.


## Test corpus

Twenty-three authored semantic/negative-control tests cover: non-compensatory floors; conservative uncertainty-bound gating; interval validity; time-window ordering; evidence coverage; unit mismatches; metric-roster completeness; snapshot identity; canonical mission-specific floor-contract digest; hard/soft constraint registry; hard-gate relabelling denial; registered soft-limit identity; hard-gate violation; weather-blind forecast isolation; synthetic-to-measured non-promotion; unauthorized and service-floor relaxations; and explicit, separately named soft-relaxation policy arms.
