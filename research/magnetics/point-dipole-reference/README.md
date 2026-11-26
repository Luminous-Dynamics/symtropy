# Point-dipole analytic reference

Dependency-free Rust reference for ideal point dipoles in free space.

Commands from the repository root:
- cargo test --manifest-path research/magnetics/point-dipole-reference/Cargo.toml
- cargo fmt --manifest-path research/magnetics/point-dipole-reference/Cargo.toml -- --check

The crate declares its own workspace so it is not pulled into the production Symtropy workspace or lockfile.

The tests cover axial/equatorial signs and magnitudes, symmetric and off-axis known answers, superposition, moment scaling, inverse-cube geometric scaling, global moment reversal, rotational covariance, singularity rejection, non-finite input rejection and the empty-set zero result.

This is a mathematical convention oracle only. It is not a finite-magnet solver and does not model nonlinear B-H curves, finite-body demagnetization, hysteresis, eddy currents, temperature, mechanics, particle alignment or manufacturing. No physical field or component performance is inferred.
