# Mine adaptive policy-value tournament

This standalone Rust crate compares three deployable policy shapes on one small authored synthetic scenario set:
- static plan: choose the prior-best lot and do not measure;
- two-stage recourse: assay A once, then choose a route using the declared recourse rule;
- rolling horizon: assay A, and if A is low, conditionally request the B assay; replan only after the B result arrives.

A separate clairvoyant evaluator computes a perfect-knowledge upper bound. The clairvoyant state is **never passed to the policy decision function** and is not a deployable policy or a fair real-time baseline.

The simulator reveals each assay only at its declared delivery epoch. The policy function takes only the prior route and observations already delivered, not the hidden world. Every decision records the full visible event-ID snapshot and the dependencies it declares using. An independent trace verifier requires the snapshot to equal the set of observations delivered by that epoch, checks every dependency against the snapshot, and rejects future-event use. A paired-scenario audit verifies that action outputs remain identical whenever policy/epoch/visible snapshot is identical.

Run:

```sh
cargo fmt --manifest-path tools/mine-adaptive-policy-value-bench/Cargo.toml -- --check
cargo test --manifest-path tools/mine-adaptive-policy-value-bench/Cargo.toml
cargo run --quiet --manifest-path tools/mine-adaptive-policy-value-bench/Cargo.toml -- \
  docs/tech/mine-adaptive-policy-value-corpus-v0.1.json \
  --source-commit "$(git rev-parse HEAD)"
```

The four public worlds are a planted counterexample, not a blinded holdout. Utilities are dimensionless integer units; no actual assay cost, ore grade, revenue, recovery, or plant effect is represented.
