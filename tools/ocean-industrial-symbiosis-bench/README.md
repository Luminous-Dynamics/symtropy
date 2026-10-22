# Ocean Industrial Symbiosis Benchmark

A synthetic-first Rust evaluator for **circular ocean/coastal industry portfolios**. It tests typed stream compatibility, conservation/capacity constraints, process quality, timing, permission, monitoring, lifecycle counterfactuals, and independent ecological-pressure limits before comparing portfolio trade-offs.

## Run

    cargo fmt --manifest-path tools/ocean-industrial-symbiosis-bench/Cargo.toml --check
    cargo test -p ocean-industrial-symbiosis-bench --locked
    cargo run --quiet -p ocean-industrial-symbiosis-bench --locked -- --corpus docs/tech/OCEAN_INDUSTRIAL_SYMBIOSIS_CORPUS_V0_1.json --source-commit "$(git rev-parse HEAD)" --receipt target/ocean-industrial-symbiosis-receipt.json

The focused workflow uploads a machine-readable receipt pinned to the exact JSON corpus bytes and Git source commit.

## What it screens

Every candidate link declares a producer port, consumer port, typed stream, quantity, time-window identity, quality constraints, route-distance bound, transfer energy, synthetic economic value, life-cycle GHG change, water saving, local value, counterfactual ID, benefit-claim ID, rights/use permission, environmental review, chemical-fate status, monitoring coverage and independent pressure channels.

The exact small-case enumerator explores all link subsets up to the frozen bound. It rejects portfolios that exceed producer availability, consumer demand, route distance, any individual pressure cap, or that reuse the same benefit claim. The Pareto frontier compares net value, lifecycle GHG change, water savings, local value, transfer energy, and sound/light/sediment/thermal/nutrient/chemical pressures separately. It never produces a single “ocean sustainability score”.

## Frozen adversarial cases

1. Heat-and-water circularity with shared producer capacities.
2. A high-value-looking heat link at the wrong temperature grade/time window and a stream-type mismatch.
3. Brine-recovery claims with unknown chemical fate and a known treatment option over the declared chemical-pressure cap.
4. Two consumers trying to claim the same grid-displacement counterfactual.
5. A passive data service contrasted with under-monitored, restricted, pending-rights, and thermal-over-budget actions; includes an explicit no-build baseline.

## Boundaries

All quantities, values, emission changes, monitoring thresholds, budgets and candidate industries are synthetic contract-test values—not costs, life-cycle assessments, carbon credits, permits, commercial recommendations or measured ecological thresholds. A green receipt means that the defined schema and adversarial checks behaved as written, not that a portfolio is economically viable or ecologically safe.

Real project screening must add primary data, endpoint-specific use rights, independently reviewed lifecycle boundaries, explicit functional units/counterfactuals, physical simulation where needed, chemical fate and ecological baselines, local/community participation, uncertainty analysis, and independent engineering review.
