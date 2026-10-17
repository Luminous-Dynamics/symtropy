# Forest Terrain Grid Simulator v0.1

A standalone, offline-only Rust simulator for a deterministic 9×9 terrain grid. It tests route feasibility, return-energy reserve, unknown/blocked terrain, fault-triggered degraded/blocked transitions, and a conservative sensor-visibility coverage model. It has no ROS, network, hardware, motor, actuator, dispatch, or route-execution adapter.

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
- **Traversed survey targets** count only target cells physically reached in the simulation before a fault transition.
- **Verified survey targets** count a reached target only if it appears in a modeled sensor footprint and the target cell is known clear.
- **Planned area** is the full 9×9 map (81 cells). **Observed area** is the union of cells within the configured three-cell range and 180° forward field of view from traversed poses, after conservative grid ray casting.
- Obstacles and unknown cells are observable as targets when directly visible, but they block rays to cells behind them. Diagonal rays conservatively check adjacent side cells to avoid visibility leaking through corners. Unobserved area is never credited merely because it lies inside a Euclidean-radius disk.
- Heading follows the incoming route step. The simplified model has no continuous yaw, 3D canopy geometry, range noise, reflectivity, weather, or calibrated sensor characteristics.
- The energy value is an illustrative integer model, not a calibrated battery or drivetrain model.
- Fault-response latency is measured in simulator steps, never wall-clock time. The zero-step response verifies only a synchronous software state transition, not physical stopping time.

More details and benchmark proposals: `docs/tech/FOREST_SENSOR_FOOTPRINT_COVERAGE_V0_1.md`.

## Claim and safety boundary

All maps and faults are synthetic. A corpus pass is evidence only that the deterministic software contract matches these declared fixtures. It does **not** establish field navigation accuracy, sensor detection performance, physical stopping distance, machinery safety, forestry-standard conformity, or fitness for real autonomous operation. A physical prototype needs separately engineered safety controls, validated emergency stop, machine-specific risk assessment, qualified tests, and independent field datasets.
