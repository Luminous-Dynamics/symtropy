# Symtropy Thermodynamic Balance Lab

A standalone, standard-library-only research crate for testing dimensionally valid energy accounting independently of Symtropy's runtime engine.

This is intentionally not yet wired into symtropy-consciousness-physics. It is an oracle/prototype to make the balance contract executable before changing production APIs.

## Scope

- Explicit energy stores: kinetic, thermal and internal reservoir.
- Typed Joule values rejecting negative and non-finite quantities.
- Internal transfers with source/destination store identities.
- Explicit external boundary inflow and outflow.
- Full event sequence and balance residual: current = initial + boundary_in - boundary_out.
- One-dimensional two-body impact using the coefficient-of-restitution formula.
- Conversion from lost kinetic energy to thermal energy using pre/post kinetic energy, not impulse times an arbitrary constant.
- Unit tests for elastic/inelastic collision, conservation, dimensional inputs, invalid values and atomic rejection.

All quantities are SI joules for this specific experiment. This is not a complete thermodynamic model: it does not model temperature, entropy, heat capacity, chemical energy or field energy.

## Run

cargo test --manifest-path research/thermo-balance-lab/Cargo.toml
cargo test --manifest-path research/thermo-balance-lab/Cargo.toml --release

## Claim ceiling

Passing these tests establishes only that this small accounting model balances according to its declared stores and collision equations. It does not validate the full Symtropy energy model, Landauer accounting, Phi as physical energy, a robot, or a physical digital twin. Do not use this crate to authorize actuators or safety decisions.
