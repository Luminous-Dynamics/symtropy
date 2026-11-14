# Wood Cut-Plan Reference Evaluator

Standalone Rust reference evaluator for bounded, one-dimensional cuts of already-squared stock. It is not a 3D log breakdown simulator.

Run from the repository root:

    cargo fmt --manifest-path tools/wood-cut-plan/Cargo.toml -- --check
    cargo test --manifest-path tools/wood-cut-plan/Cargo.toml
    cargo run --quiet --manifest-path tools/wood-cut-plan/Cargo.toml -- docs/tech/wood-cut-plan-corpus-v0.1.json --source-commit "$(git rev-parse HEAD)"

It enumerates all feasible product-count patterns under integer lengths, saw kerf, declared demand caps and structural grade/inspection gates; computes a deterministic Pareto frontier; compares it with a clearly named highest-value-per-length greedy baseline; and emits a receipt over the exact fixture bytes and tested source commit. The frozen synthetic corpus checks the baseline's outcomes and verifies that each feasible baseline result is weakly dominated or matched by the exact frontier. If the search-node budget is exceeded, it errors rather than returning a partial frontier.

All values are synthetic. A passing receipt validates only this bounded reference problem, not 3D log recovery, grade certification, real prices, sustainable harvesting or production performance. See the design and literature notes in `docs/tech/WOOD_CUT_PLAN_REFERENCE_EVALUATOR_V0_1.md`.
