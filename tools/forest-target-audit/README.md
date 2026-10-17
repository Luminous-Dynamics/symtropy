# Forest Target-Completeness Audit v0.1

Standalone Rust auditor that re-scores planner receipts against deterministic, seeded target-feature tasks. No ROS, hardware, network, motor, or actuation interface.

## Run

Generate the exact-head planner receipt first:

```sh
source_commit="$(git rev-parse HEAD)"
cargo test --manifest-path tools/forest-next-best-view/Cargo.toml
cargo run --quiet --manifest-path tools/forest-next-best-view/Cargo.toml -- \
  docs/tech/forest-next-best-view-corpus-v0.1.json --source-commit "$source_commit" > planner-receipt.json
cargo test --manifest-path tools/forest-target-audit/Cargo.toml
cargo run --quiet --manifest-path tools/forest-target-audit/Cargo.toml -- \
  planner-receipt.json \
  --target-corpus docs/tech/forest-target-completeness-corpus-v0.1.json \
  --source-commit "$source_commit" > target-completeness-receipt.json
```

The workflow does these same steps on the exact checked-out PR head and uploads both receipts.

## Method boundaries

The audit recomputes visibility separately from the planner crate and validates scenario IDs, map hashes, configuration hashes, plan feasibility and disjoint development/held-out seeds. Target features are synthetic grid samples on frozen map templates. `coverage_oracle` is optimal only for the existing unique-cell objective; it is **not** an optimum for target completeness. Read `docs/tech/FOREST_TARGET_COMPLETENESS_V0_1.md` for the claim ceiling and benchmark ladder.
