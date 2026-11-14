# Net additionality, substitution uncertainty, and circular rebound benchmark

Standalone Rust benchmark for [Symtropy issue #1898](https://github.com/Luminous-Dynamics/symtropy/issues/1898). This is a **synthetic method test**, not a real lifecycle assessment or a claim that any actual product is preferable.

## Run

```sh
cargo fmt --manifest-path tools/net-additionality-benchmark/Cargo.toml -- --check
cargo test --manifest-path tools/net-additionality-benchmark/Cargo.toml
SOURCE_REVISION="$(git rev-parse HEAD)" cargo run --manifest-path tools/net-additionality-benchmark/Cargo.toml
```

## What changes

The benchmark refuses to treat gross recovered content as equivalent to avoided virgin production. For an eligible option, it calculates yield-adjusted recovered mass, caps effective substitution at the scenario's market-displacement limit, then models scenario-specific rebound as additional service volume. The headline loop score is capped at virgin-material demand per service unit. Virgin-material and action-process burdens are both scaled, with integer arithmetic and conservative rounding. An option whose evidence is unknown, revoked, partial, claim-mismatched, or out of scope is excluded; it receives no policy score.

Three policies are compared on the same fixed functional unit:

- **Gross-circularity policy:** highest eligible yield-adjusted recovery.
- **Expected-impact policy:** lowest probability-weighted total synthetic GHG burden over the declared scenarios.
- **Minimax-regret policy:** lowest maximum GHG regret versus the per-scenario hindsight oracle.

The oracle is only a comparator; it cannot be used as a deployable policy because it chooses with knowledge of each scenario. The separate expected GHG/cost Pareto frontier shows the declared trade-off without mixing mass, price, and emissions into an arbitrary single score.

## Interpretation limits

Scenario probabilities are illustrative inputs, not learned probabilities. Substitution caps, rebound response, embodied-GHG factors, and process burdens are synthetic. All values are in integer grams, cents, and basis points, with explicit floor/ceiling rules. Before applying the method to a real intervention, replace the synthetic fixture with traceable primary data, representative system boundaries, measured service delivery, and a documented consequential-LCA counterfactual. Do not promote this fixture's rankings into environmental claims.
