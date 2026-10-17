# Forest Terrain Grid Simulator v0.1

A standalone, offline-only Rust simulator for a deterministic terrain grid. It tests route feasibility, return-energy reserve, unknown/blocked terrain, and fault-triggered degraded/blocked transitions. It has no ROS, network, hardware, motor, actuator, dispatch, or route-execution adapter.

## Run

```sh
cargo fmt --manifest-path tools/forest-terrain-sim/Cargo.toml -- --check
cargo test --manifest-path tools/forest-terrain-sim/Cargo.toml
cargo run --quiet --manifest-path tools/forest-terrain-sim/Cargo.toml -- \
  docs/tech/forest-terrain-grid-corpus-v0.1.json \
  --source-commit "$(git rev-parse HEAD)"
```

The CLI validates the frozen corpus and emits a receipt binding the exact corpus bytes, deterministic map/config JSON digests, simulator version, and supplied source commit. Scenario order and BFS neighbor order are fixed.

## Coverage semantics

- **Planned survey targets** are requested target cells, not evidence of completion.
- **Traversed survey targets** count only targets reached before a simulated fault transition.
- **Verified survey targets** count only reached targets whose synthetic map cell is observed and clear.
- Failed routes, unknown targets, insufficient return reserve, and injected faults receive no planned-only coverage credit.
- The energy value is an illustrative integer model, not a calibrated battery or drivetrain model.
- Fault-response latency is measured in simulator steps, never wall-clock time. The zero-step response verifies only that the code changes state synchronously in the model; it is not a physical stopping-time result.

## Claim and safety boundary

All maps and faults are synthetic. A corpus pass is evidence only that the deterministic software contract matches these declared fixtures. It does **not** establish field navigation accuracy, physical stopping distance, machinery safety, forestry-standard conformity, or fitness for real autonomous operation. A physical prototype needs separately engineered safety controls, validated emergency stop, machine-specific risk assessment, qualified tests, and independent field datasets.
