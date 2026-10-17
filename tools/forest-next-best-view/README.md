# Forest Next-Best-View Benchmark v0.1

Standalone, offline Rust benchmark comparing a deterministic marginal-gain greedy planner with an exhaustive oracle on tiny synthetic forest scenes. It has no ROS, network, hardware, motor, actuator, dispatch, or route-execution adapter.

## Run

    cargo test --manifest-path tools/forest-next-best-view/Cargo.toml
    cargo run --quiet --manifest-path tools/forest-next-best-view/Cargo.toml -- docs/tech/forest-next-best-view-corpus-v0.1.json --source-commit "$(git rev-parse HEAD)"

The receipt binds source revision, exact corpus bytes, map digest, and planner-configuration digest. Scene order, candidate order, heading order, tie-breaking and oracle objective are deterministic.

## Objective and constraints

- Candidate viewpoints are traversable cells paired with cardinal headings.
- Sensor visibility uses a fixed 180-degree forward field of view, radius 3 cells, and conservative grid ray casting. Unknown/obstacle cells may themselves be visible, but block visibility to cells behind them.
- The greedy baseline chooses the reachable view with the largest marginal newly-visible-cell gain divided by max(incremental travel + heading-change cost, 1).
- The exhaustive oracle enumerates unique viewpoint sequences up to the same viewpoint limit and energy budget. It maximizes unique visible cells, then minimizes total budgeted steps.
- Every sequence pays travel, a simplified one-step heading-change cost, shortest-distance return to the start, and an explicit return reserve. The return leg is not credited with extra sensing.
- Equal objectives use stable candidate order. The tiny-grid exhaustive search is intentionally capped at four viewpoints.

## Claim ceiling

All results are synthetic. The oracle is exact only for the enumerated 5×5 scene, discrete headings, view count, simplified cost model, candidate set and coverage model. It does not establish a global approximation guarantee, calibrated sensing performance, field robotics performance or machine safety. Do not extrapolate these scores to a real forest or deploy the planner on hardware.
