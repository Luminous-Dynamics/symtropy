# ORS exact-vector momentum control-volume reference v0.1

**Evidence class:** analytical/software reference using synthetic frozen inputs  
**Tracking:** Symtropy [ORS-MOMENTUM-CONTROL-VOLUME-001 #5872](https://github.com/Luminous-Dynamics/symtropy/issues/5872)  
**Parent:** [ORS-DYNAMIC-SUPPORT-001 #5387](https://github.com/Luminous-Dynamics/symtropy/issues/5387)

## Purpose

A moving material stream is an open system when mass crosses a finite control-volume boundary. Its support interaction cannot be determined from a scalar curvature formula alone. This small, standalone Rust reference implements the signed three-dimensional advective momentum balance with explicit mass accumulation and multiple inlet/outlet ports.

In one declared inertial frame:

~~~text
dM_CV/dt = sum_in(mdot) - sum_out(mdot)

F_required_on_stream =
    d/dt(integral_CV(rho * v dV))
    + sum_out(mdot * v) - sum_in(mdot * v)

F_support_on_stream =
    F_required_on_stream - F_other_external_on_stream

F_stream_on_support = -F_support_on_stream
~~~

The non-support external-force vector is an explicit input, not a hidden residual. The result is the support interaction required by this *declared mathematical boundary* after the stated other forces are accounted for.

## Exact finite-angle skyhook limit

For equal steady inlet/outlet mass flow, equal speed V and a direction change theta:

~~~text
mdot = lambda * V

|F_turn| = mdot * V * 2 * sin(theta / 2)
         = lambda * V^2 * 2 * sin(theta / 2)

as theta -> 0:
|F_turn| ~= lambda * V^2 * theta
~~~

This recovers the small-angle form used by Birch (1982) while retaining the exact vector chord change at finite angle. It does not validate Birch's conic approximation, support mechanism or orbital-ring architecture.

## Frozen v0.1 corpus

- 6 named scenarios and 14 mass-flux ports.
- An explicit inertial-frame identifier and evaluation epoch per scenario; every port in a scenario uses that same frame and epoch.
- Straight-through control and 90-degree turn.
- Unsteady storage case with nonzero mass and momentum accumulation.
- Three-dimensional multiple-inlet/multiple-outlet case.
- Explicit non-support-force isolation.
- 179-degree near-reversal case.
- Negative tests for duplicate IDs, invalid directions, non-finite values, negative flow and mass-balance mismatch.
- Metamorphic tests for orthogonal rotation, Galilean frame changes and port-order permutations.
- Small-angle error convergence against Birch's approximation.

The JSON receipt binds the source commit and the SHA-256 of each input CSV. It contains no wall-clock timestamp and the crate has no third-party Rust dependencies.

## Run

From the repository root:

~~~sh
cargo fmt --manifest-path tools/ors-momentum-control-volume-reference/Cargo.toml --all -- --check
cargo clippy --manifest-path tools/ors-momentum-control-volume-reference/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path tools/ors-momentum-control-volume-reference/Cargo.toml --locked --all-targets

SOURCE_COMMIT="$(git rev-parse HEAD)"
CASES_SHA="$(sha256sum tools/ors-momentum-control-volume-reference/cases-v0.1.csv | awk '{print $1}')"
PORTS_SHA="$(sha256sum tools/ors-momentum-control-volume-reference/ports-v0.1.csv | awk '{print $1}')"
cargo run --quiet --manifest-path tools/ors-momentum-control-volume-reference/Cargo.toml --locked -- \
  --source-commit "$SOURCE_COMMIT" --cases-sha256 "$CASES_SHA" --ports-sha256 "$PORTS_SHA"
~~~

## Explicit exclusions and claim ceiling

The reference accounts only for advective mass momentum at the ports and the declared momentum accumulation. It does not implement:

- pressure or stress traction at a control-volume surface;
- internal material constitutive response, bending stiffness or tension;
- gravitational, electromagnetic or other force laws (the declared other-force vector must be independently supplied);
- electromagnetic field momentum, Poynting flux or Maxwell stress;
- angular-momentum flux, energy flux or a total energy equation;
- inertial/local-frame transformations beyond the Newtonian covariance metamorphic tests;
- orbital propagation, ring equilibrium, stability, thermal design, failure response or physical hardware.

A passing result demonstrates only the implementation and analytical tests of this pressureless advective momentum reference over its frozen synthetic envelope. It is not a physical skyhook load estimate or an admitted orbital-ring force law.
