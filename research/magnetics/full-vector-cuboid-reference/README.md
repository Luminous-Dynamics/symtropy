# Full-vector cuboid magnetic-field reference

A zero-dependency Rust research crate for the three-component field of a
homogeneously polarized rectangular prism.

## Model

The cuboid is centred at the origin with half-dimensions (a,b,c) in metres and
uniform prescribed polarization J=(Jx,Jy,Jz) in tesla. The API returns B in
tesla at strictly off-boundary observer positions.

Each of the six rectangular faces carries magnetic surface charge
sigma = J dot n. A face contribution is the analytical surface integral

    B_face(r) = sigma/(4*pi) * integral_face (r-r')/|r-r'|^3 dA

evaluated in a right-handed local face frame. The six face contributions are
summed; J is added strictly inside the material to convert from the H-derived
surface-charge field to B.

This implementation uses face-wise asinh and principal-arctangent expressions.
It is an independently derived representation, not a copy of the eight-corner
arctangent/logarithm kernel used by Magpylib and magpylib_jax.

## Run

    cargo fmt --manifest-path research/magnetics/full-vector-cuboid-reference/Cargo.toml -- --check
    cargo clippy --manifest-path research/magnetics/full-vector-cuboid-reference/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path research/magnetics/full-vector-cuboid-reference/Cargo.toml

The nested Cargo workspace is excluded from the production Symtropy workspace.

## Boundary and scope

v0.1 rejects observers on the cuboid coordinate planes. It does not silently
choose an interior/exterior face limit and does not evaluate edges/corners.
Use only strictly interior and exterior points. A future boundary-aware API
must expose explicit one-sided limits.

The model assumes a homogeneous, prescribed polarization in free space. It
does not solve nonlinear B-H response, self-demagnetization, hysteresis, eddy
currents, temperature, interfaces or process microstructure. It is not a
finite-element solver or physical component qualification.

Synthetic reference fixtures are mathematical controls, not field measurements.
