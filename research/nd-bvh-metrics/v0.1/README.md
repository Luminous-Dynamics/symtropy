# N-dimensional BVH Metrics v0.1

A dependency-free Rust research reference for dimension-aware AABB diagnostics. This crate supplies geometry measures; it does not build a BVH or select a production broadphase algorithm.

## Measures

For a finite ordered D-dimensional AABB with widths w_i:

- Hypervolume: V_D = product_i(w_i).
- Generalized boundary content: B_D = 2 sum_i product_(j != i)(w_j).
- Normalized internal boundary-content ratio: sum of supplied internal-node boundary contents divided by the root boundary content.
- Pair overlap ratio: intersection hypervolume divided by the smaller input hypervolume.
- Axis-normalized extent: each node width divided by the root width on that axis.

In 2D, boundary content is perimeter. In 3D, it is surface area. Its units depend on dimension and raw values must not be compared cross-dimensionally.

## Degenerate and invalid geometry

Construction rejects zero dimensions, non-finite endpoints and inverted bounds. Hypervolume of a valid degenerate box is zero. Boundary content can remain nonzero for a lower-dimensional box (for example, a sheet in 3D). Ratios that require a zero or unavailable denominator return None, not zero.

Arithmetic overflow and non-finite intermediate results return None. A disjoint intersection has zero hypervolume and therefore zero overlap ratio when both input volumes are positive.

## Run

    cargo fmt --manifest-path research/nd-bvh-metrics/v0.1/Cargo.toml -- --check
    cargo test --manifest-path research/nd-bvh-metrics/v0.1/Cargo.toml --locked
    cargo clippy --manifest-path research/nd-bvh-metrics/v0.1/Cargo.toml --locked --all-targets -- -D warnings

## Claim boundary

These are descriptive geometry metrics. They do not predict query cost, establish broadphase recall, certify swept bounds or imply that a smaller ratio means faster traversal. Issue #7257 owns calibration against opt-in traversal counters from #7172 and held-out query workloads from #7225. No production algorithm-selection input is authorized by this crate.
