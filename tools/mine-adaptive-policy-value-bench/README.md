# Mine adaptive policy-value tournament

This standalone Rust crate compares three deployable policy shapes on one small authored synthetic scenario set:
- static plan: choose the best prior-expected lot and do not measure;
- two-stage recourse: assay A once, then choose a route using the declared recourse rule;
- rolling horizon: assay A, and if A is low, decide whether the new state justifies assaying B before final routing.

A separate clairvoyant evaluator computes a perfect-knowledge upper bound. The clairvoyant state is **never passed to the policy decision function** and is not an eligible deployed baseline.

The world simulator reveals each assay only at its declared delivery epoch. The policy function takes only the current observation list and an initial prior route; it does not take the underlying scenario/world. Every decision records its complete visible event IDs and declared dependencies. A separate verifier checks used observations appear in the visible snapshot and were delivered by that decision epoch.

Run:

```sh
cargo fmt --manifest-path tools/mine-adaptive-policy-value-bench/Cargo.toml -- --check
cargo test --manifest-path tools/mine-adaptive-policy-value-bench/Cargo.toml
cargo run --quiet --manifest-path tools/mine-adaptive-policy-value-bench/Cargo.toml -- \
  docs/tech/mine-adaptive-policy-value-corpus-v0.1.json \
  --source-commit "$(git rev-parse HEAD)"
```

The four public worlds are a planted counterexample, not a blinded holdout. Utilities are dimensionless integer units; no actual assay cost, ore grade, revenue, recovery, or plant effect is represented.
