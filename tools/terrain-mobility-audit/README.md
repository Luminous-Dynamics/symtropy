# Terrain Mobility Contract Audit

Standalone Rust audit for Symtropy's frozen terrain-mobility corpus. It implements the executable contract tranche planned in the v0.1 benchmark.

## Run

To format a checkout, run `cargo fmt --manifest-path tools/terrain-mobility-audit/Cargo.toml`. Then run `cargo test --manifest-path tools/terrain-mobility-audit/Cargo.toml`.

Generate a receipt with `cargo run --quiet --manifest-path tools/terrain-mobility-audit/Cargo.toml -- fixtures/terrain_mobility/terrain_mobility_scenarios_v0_1.json --source-commit $(git rev-parse HEAD) > terrain-mobility-audit-receipt.json`.

The command emits a deterministic JSON receipt and exits non-zero on any contract error. It checks corpus metadata, unique scenario identifiers, retained negative controls, declared dispositions, explicit return-energy inputs, and an independent disposition oracle.

## Energy contract

For a mission requiring a return, each scenario separately declares `energy_budget_wh`, `estimated_return_energy_wh`, and `return_margin_requirement_pct`. The gate computes the required amount as `estimated_return_energy_wh * (1 + return_margin_requirement_pct / 100)`.

A budget below the required amount selects safe abort/retreat. Missing, invalid, or non-positive return evidence is unresolved, never zero. A mission that does not require return must set the estimate to null.

These are hand-authored synthetic inputs, not a robot-specific energy model, validated operating limit, or field result.

## Independent dispositions

The oracle derives dispositions from terrain and hazard declarations, localization and communications state, named fault signals, capability/calibration evidence, profile conflicts, repair/parts uncertainty, and return-energy feasibility. It does not use `expected_behavior` to derive its answer; it compares the independently derived result with that field afterward.

The canonical corpus SHA-256 sorts scenarios by `scenario_id` before hashing, making the digest invariant to scenario order. The exact source commit is recorded separately.

## Claim ceiling

A passing receipt establishes only that this synthetic corpus conforms to the declared contract and checker at the supplied revision. It does not establish physical locomotion performance, simulator fidelity, roadworthiness, reliability, or robot safety.
