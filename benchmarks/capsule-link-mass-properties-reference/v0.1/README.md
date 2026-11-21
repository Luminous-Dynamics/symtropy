# Capsule Link Mass-Properties Reference v0.1

**Tracking issue:** [ARTICULATED-CAPSULE-MASS-PROPERTIES-001 #6475](https://github.com/Luminous-Dynamics/symtropy/issues/6475)  
**Engine-wide inertia API:** [#6443](https://github.com/Luminous-Dynamics/symtropy/issues/6443)  
**Source baseline:** `main` at `b93beb5f68f6fa0bb0c5d4f58d3afb79a19cb68e`

**Status:** M1 independent analytic reference. No production chain changes, compilation, CI, hardware measurements or robot qualification claimed.

## Why this is needed

The production 3D chain builder receives link mass, overall length and radius in `LinkSpec`, places adjacent pivots using the link length, and then constructs a sphere of the link radius. The math crate already provides an axis-aligned capsule collider. The current builder therefore ignores declared link length in collision geometry and spherical mass distribution.

The reference interprets `length` as the overall pivot-to-pivot capsule length (including both caps). For radius r, the internal cylinder length is `Lc = length - 2r`. A future production change must keep this length convention and pivot anchors consistent.

## Analytical model

A uniform capsule is the union of a cylinder and two hemispheres. Its volume is:

`Vc = pi*r^2*Lc`, `Vs = 4*pi*r^3/3`, `rho = m/(Vc+Vs)`.

The cylinder and caps have masses `mc = rho*Vc` and `ms = rho*Vs`.

For the local COM frame with capsule axis Z, store the full second-moment tensor C (the diagonal is sufficient in this aligned frame):

`Cxx = Cyy = mc*r^2/4 + ms*r^2/5`

`Czz = mc*Lc^2/12 + ms*(19*r^2/320 + (Lc/2 + 3*r/8)^2)`.

The (3r/8) term is the hemisphere COM offset from its flat face. The (19/320) coefficient is the hemisphere second moment about its own COM, normalized by hemisphere mass. The principal moments are `Ixx=Cyy+Czz`, `Iyy=Cxx+Czz`, `Izz=Cxx+Cyy`.

## Known answers

For a 1 kg, 0.3 m overall-length, 0.03 m radius link:

- cylinder length: 0.24 m;
- cylinder mass: 6/7 kg;
- combined cap mass: 1/7 kg;
- `Cxx=Cyy=0.00021857142857142857 kg m^2`;
- `Czz=0.006582857142857142 kg m^2`;
- `Ixx=Iyy=0.006801428571428571 kg m^2`;
- `Izz=0.00043714285714285714 kg m^2`.

At the boundary `length=2r`, the cylinder vanishes and the model exactly reduces to a uniform solid sphere: `Cii=mr^2/5`, `Iii=2mr^2/5`.

A long slender capsule should strongly resist rotation about a transverse axis compared with its longitudinal axis; the fixture records a 0.5 m by 0.02 m diameter, 1 kg example.

## Test coverage

- sphere-boundary limit;
- finite-cylinder mass split and tensor known answers;
- slender-link axial/transverse inertia ratio;
- linear mass scaling and quadratic geometric scaling;
- rotated-frame covariance and off-diagonal products;
- independent midpoint slice quadrature of cap contributions;
- invalid mass, geometry and reflection rejection.

## Reproduction

```sh
cargo fmt --manifest-path benchmarks/capsule-link-mass-properties-reference/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/capsule-link-mass-properties-reference/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/capsule-link-mass-properties-reference/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

## Promotion path

M1 is this independent formula and fixture. M2 replaces the chain builder's sphere collider with an appropriately Z-aligned capsule and obtains mass properties through the frame-explicit API from #6443. M3 validates link pose, pendulum dynamics, joint torque and energy against analytical references. M4 requires measured physical article geometry, mass, COM and inertia. M5 connects qualified links to long-horizon manipulation/locomotion and task benchmarks.

## Claim boundary

An exact uniform capsule formula only qualifies that idealized geometric model. A physical robot link may contain motors, bearings, tendons, wiring, batteries, cut-outs or non-uniform density; those require measured or component-composed mass properties.
