# Meal compiler V1

Status: draft software contract and synthetic fixtures. No physical cooking, food safety, inventory, appliance or sensory qualification is implied.

## Purpose

Compile a versioned Symthaea `MealProposalV1` into two linked, typed graphs:

1. **Recipe/action graph** — action kinds, explicit reviewed ontology mappings, task dependencies, scheduling fields and exact ingredient quantities, mapped into the existing `RecipeTaskGraphV1`.
2. **Food-flow graph** — explicit material edges from raw ingredient inputs through intermediate task outputs to planned edible outputs or planned losses.

The graphs are linked by proposal identity/revision and explicit task/output IDs. This avoids forcing material transformation into a task DAG that cannot represent it.

## Evidence and admission semantics

- A proposal SHA-256 is computed over `serde_json::to_vec` of the typed proposal in the pinned Rust contract. Acceptance must bind that digest and revision and have a prior, non-empty reviewer/acceptance reference.
- Source-rights resolution is a supplied reviewed declaration. The compiler checks binding and local-processing scope; it does not independently authenticate the review record.
- Each task has explicit action, duration-method, schedule/dependency, and planning-resource resolution references. Scheduling resources are not appliance module IDs.
- Action ontology bindings must be mapped and their provenance reference must equal the explicit action-resolution reference. Natural-language descriptions are never action commands.
- All quantities use positive integer coefficients, explicit dimensions/units and decimal scale. No density conversion, stock inference, or zero-filling is performed.
- Each explicitly quantified ingredient must be fully allocated across task inputs and/or explicit planned loss outputs. Under-allocation is indeterminate; over-allocation or contradictory bindings are conflicted.
- Every task output has explicit ingredient-origin lineage and exactly one outgoing edge. V1 intentionally does not infer splitting; splitting requires explicitly declared child materials in a future schema.
- Price and availability facts require market and validity bounds. Allergen declarations require a known ingredient reference and product-SKU or supplier-lot scope. This does not establish allergen safety.
- Every output quantity is a *planned* value. It is not measured yield, residue, evaporation, accepted edible mass or sensory acceptance.

## System boundary

`MealCompilationDispositionV1::Compiled` means only that the supplied proposal and explicit resolutions form internally consistent action and planned material-flow graphs.

It does **not** mean:
- Mycelix found or verified a household food lot;
- budget, availability, freshness or allergen safety are qualified;
- the Symtropy scheduler found a feasible schedule;
- any appliance/module revision was admitted;
- food-contact hygiene or physical compatibility is proven;
- pressure, thermal, blade, motor or electrical safety is qualified;
- the meal was prepared, accepted or measured;
- any device command is authorized.

The next independent stages remain: Mycelix read-only lot resolution; Symtropy scheduling and exact module admission; independent physical safety controls; measured material/energy/water accounting; food-quality assessment and household acceptance.

## Cross-repository wire conformance

The focused harness freezes a real synthetic `MealProposalV1` fixture sourced from Symthaea PR #12715. The fixture records the upstream production-source commit, schema ID, schema blob SHA and fixture-bundle blob SHA. Symtropy deserializes that proposal into its local versioned mirror without a runtime dependency on the Symthaea crate.

This is an explicit drift detector, not a claim that the two repositories are automatically synchronized. Any upstream schema or enum change requires a reviewed fixture refresh and a new exact-head conformance run. The test asserts that the unknown beans quantity and planning-blocker uncertainty remain unknown, the lot reference remains absent, and the proposal remains in the `proposed` state rather than silently becoming accepted.

## Conformance

Run the exact-head focused workflow:

```sh
cargo fmt --manifest-path tools/kitchen-meal-compiler-harness/Cargo.toml --all -- --check
cargo test --manifest-path tools/kitchen-meal-compiler-harness/Cargo.toml
cargo clippy --manifest-path tools/kitchen-meal-compiler-harness/Cargo.toml --all-targets -- -D warnings
```

The harness imports `symtropy-kitchen-planner` through a local path dependency; it does not copy the production compiler source. The workflow pins Rust 1.96.1 and asserts checkout SHA.

## Research benchmark targets (not results)

- 100% exact digest/revision binding for accepted proposals.
- 100% preservation of explicit unknowns; zero fabricated quantities, inventory, appliance IDs or action labels.
- 100% graph/reference conformance on the preregistered synthetic suite.
- Zero false `compiled` dispositions in at least 10,000 adversarial generated proposals (target; confidence intervals and mutation coverage required).
- 100% decision-affecting reference coverage in the compilation report.
- Bit-for-bit deterministic reports for identical typed inputs and compiler version.
- p95 compile latency below 50 ms for a 100-task / 500-edge synthetic graph on a declared reference host (performance hypothesis, not a deployment guarantee).
- Zero actuator, purchase, pantry-mutation or DHT-write side effects; compiler remains pure/read-only.

Report each metric separately. A high compile pass rate must never compensate for a false accept or a lost uncertainty.
