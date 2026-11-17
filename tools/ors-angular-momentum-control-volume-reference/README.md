# ORS fixed-origin angular-momentum control-volume reference v0.1

**Evidence class:** analytical/software reference using synthetic frozen inputs  
**Tracking:** Symtropy [ORS-ANGULAR-MOMENTUM-CONTROL-VOLUME-001 #5877](https://github.com/Luminous-Dynamics/symtropy/issues/5877)  
**Companion:** [linear momentum control-volume reference #5872](https://github.com/Luminous-Dynamics/symtropy/issues/5872)  
**Parent:** [ORS-DYNAMIC-SUPPORT-001 #5387](https://github.com/Luminous-Dynamics/symtropy/issues/5387)

## Purpose

An open stream can transfer angular momentum across a finite control-volume boundary. A scalar force times a guessed lever arm is not a sufficient torque audit. This small, standalone Rust reference evaluates each port's actual advective angular-momentum contribution from its position relative to a declared fixed inertial origin and its absolute inertial velocity.

For one fixed origin O:

~~~text
dH_O,CV/dt = d/dt(integral_CV(rho * (r_O x v) dV))

tau_required_about_O =
    dH_O,CV/dt
    + sum_out(mdot * (r_O x v))
    - sum_in(mdot * (r_O x v))

tau_support_on_stream_about_O =
    tau_required_about_O - tau_other_external_about_O

tau_stream_on_support_about_O = -tau_support_on_stream_about_O
~~~

The input and output include the companion linear momentum balance because translating the angular-momentum origin changes torque by the cross product of the origin shift and resultant force.

## Origin translation is not invariance

For a new fixed inertial origin O' = O + a:

~~~text
r_O' = r_O - a
H_O' = H_O - a x P

dH_O'/dt = dH_O/dt - a x dP/dt

tau_required_about_O' =
    tau_required_about_O - a x F_required

tau_support_about_O' =
    tau_support_about_O - a x F_support
~~~

A torque about an origin is not origin-independent. The metamorphic test transforms positions, angular-momentum accumulation and other external torque consistently, then checks the corresponding torque shift. This protects against incorrectly asserting torque invariance.

## Frozen v0.1 corpus

- 6 scenarios and 14 mass-flux ports.
- Radial-through flow with zero angular-momentum flux.
- Tangential flow with hand-calculated moment-arm torque.
- Off-axis turn with independent linear and angular momentum fluxes.
- Three-dimensional multi-inlet/multi-outlet port set.
- Unsteady control-volume storage with both linear and angular momentum accumulation.
- Other-force/other-torque isolation.
- Orthogonal rotation covariance and fixed-origin translation metamorphic tests.
- Canonical port-ID ordering, strict identity/finite-value checks, negative-flow and mass-balance negative controls.

The deterministic JSON receipt binds the exact source commit and SHA-256 digests of both CSV files. The standalone crate has no third-party Rust dependencies.

## Run

From the repository root:

~~~sh
cargo fmt --manifest-path tools/ors-angular-momentum-control-volume-reference/Cargo.toml --all -- --check
cargo clippy --manifest-path tools/ors-angular-momentum-control-volume-reference/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path tools/ors-angular-momentum-control-volume-reference/Cargo.toml --locked --all-targets

SOURCE_COMMIT="$(git rev-parse HEAD)"
CASES_SHA="$(sha256sum tools/ors-angular-momentum-control-volume-reference/cases-v0.1.csv | awk '{print $1}')"
PORTS_SHA="$(sha256sum tools/ors-angular-momentum-control-volume-reference/ports-v0.1.csv | awk '{print $1}')"
cargo run --quiet --manifest-path tools/ors-angular-momentum-control-volume-reference/Cargo.toml --locked -- \
  --source-commit "$SOURCE_COMMIT" --cases-sha256 "$CASES_SHA" --ports-sha256 "$PORTS_SHA"
~~~

## Explicit exclusions and claim ceiling

This is an advective angular-momentum reference about a fixed inertial origin. It does not implement:

- stress/pressure traction or contact couples on the control-volume surface;
- electromagnetic field angular momentum or Maxwell stress;
- flexible-body gyroscopic response, structural constitutive laws or rider dynamics;
- moving/deforming control surfaces or moving/rotating origins;
- orbital dynamics, equilibrium, stability, failure response, or total energy.

A passing reference proves only the named algebraic balance and its tests over the frozen synthetic envelope. It is not a physical skyhook torque estimate or an admitted ORS support model.
