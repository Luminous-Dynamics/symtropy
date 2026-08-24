# External Heat Validation Campaign

This campaign validates deterministic sensible-heat exchange across the modeled energy boundary.

## Model

A lumped body with constant thermal mass `m` and constant specific heat `c_p` receives prescribed external power `P`.

The analytical solution is

`T(t) = T0 + P t / (m c_p)`.

Every applied heat increment is also recorded as a double-entry boundary transfer from `EnergyOwner::External(source_id)` to the body's `ThermalSensible` reservoir.

## Scenario

- Initial temperature: 290 K
- Thermal mass: 5 kg
- Specific heat: 800 J/(kg K)
- Prescribed power: 2500 W
- Timestep: 0.125 s
- Steps: 80
- Elapsed time: 10 s
- Expected heat input: 25,000 J
- Expected temperature rise: 6.25 K

## Acceptance criteria

The example `external_heat_validation` must satisfy all of the following:

1. Absolute temperature error relative to `T0 + Pt/(m c_p)` is <= `1e-12 K`.
2. Ledger net external energy matches prescribed heat input to floating-point tolerance.
3. First-law relative closure error is <= `1e-12`.
4. A repeated run produces bitwise-identical `WorldSnapshot` state.
5. A repeated run produces an identical ordered `EnergyTransferLedger`.
6. The legacy untracked replay executor refuses external-heat commands.

## Validity limits

This campaign does not validate convection, radiation, phase change, temperature-dependent material properties, spatial gradients, or source dynamics. The external source is prescribed and has no modeled internal state of its own.

The purpose of this campaign is narrower: prove that an explicit boundary energy intervention changes thermal state by the analytical amount, closes the first-law ledger, and remains deterministic under replay.

Run with:

```bash
cargo run -p symtropy-physics --example external_heat_validation
```
