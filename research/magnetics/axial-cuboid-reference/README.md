# Axial cuboid magnetic-field reference

A zero-dependency Rust research crate implementing the exact axial Bz field
of a uniformly polarized rectangular prism (cuboid) in free space.

## Geometry and assumptions

- Cuboid is centred at the origin, with half-dimensions (a,b,c) in metres.
- Uniform polarization is (0,0,Jz) in tesla, where Jz = mu0*M.
- Observer is restricted to (0,0,z) on the cuboid symmetry axis.
- Result is magnetic flux density Bz in tesla.
- The material polarization term is included strictly inside the cuboid.
- Signed principal arctangents and explicit one-sided limits handle the upper
  and lower end-face centres.

This is an exact finite-body axial reference, not a full 3D field solver.
It does not calculate off-axis components, nonlinear B-H response, iterative
demagnetization, hysteresis, eddy currents, interfaces, temperature or process
microstructure.

## Run

    cargo test --manifest-path research/magnetics/axial-cuboid-reference/Cargo.toml
    cargo fmt --manifest-path research/magnetics/axial-cuboid-reference/Cargo.toml -- --check
    cargo clippy --manifest-path research/magnetics/axial-cuboid-reference/Cargo.toml --all-targets -- -D warnings

The crate declares its own Cargo workspace, keeping it outside the production
Symtropy workspace and lockfile.

## Independent comparison

Magpylib and magpylib-jax provide independently maintained closed-form
cuboid-field implementations. This Rust reference intentionally covers only
the symmetry axis, where the formula follows directly from the two charged end
faces and can be independently checked without copying a general cuboid kernel.

Synthetic known answers validate the formula and code arithmetic; they are not
physical field measurements or magnet qualification.
