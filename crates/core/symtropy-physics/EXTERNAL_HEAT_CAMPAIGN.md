# External Heat Validation Campaign

This campaign validates deterministic sensible-heat exchange across the modeled energy boundary.

## Model

A lumped body with constant thermal mass `m` and constant specific heat `c_p` receives prescribed external power `P`.

The analytical solution is

`T(t) = T0 + P t / (m c_p)`.

Every applied heat increment is also recorded as a double-entry boundary transfer between `EnergyOwner::External(source_id)` and the body's `ThermalSensible` reservoir. Positive power is external -> body; negative power is body -> external.

The campaign obtains `m c_p` through the same finite-derived-capacity validation path as the thermal kernel rather than reintroducing unchecked raw multiplication in the reference calculation.

## Scenarios

Both signs of the accounting boundary are exercised over 80 steps of `0.125 s` (10 seconds total):

- **Heating:** 290 K, +2500 W, 25,000 J enters the body, expected rise 6.25 K.
- **Cooling:** 310 K, -1000 W, 10,000 J leaves the body, expected drop 2.5 K.

Both use thermal mass 5 kg and specific heat 800 J/(kg K).

## Acceptance criteria

The example `external_heat_validation` must satisfy all of the following for both scenarios:

1. Absolute temperature error relative to `T0 + Pt/(m c_p)` is <= `1e-12 K`.
2. Ledger net external energy matches the prescribed signed boundary energy to floating-point tolerance.
3. The strict complete-accounting first-law relative closure error is <= `1e-12`.
4. The body's measured sensible-energy change agrees with that reservoir's ledger net change to <= `1e-12` relative error.
5. Every ledger entry has the expected external/body direction and the dedicated external-heat mechanism constant.
6. Exactly one boundary ledger entry is emitted per non-zero prescribed step.
7. A repeated run produces bitwise-identical `WorldSnapshot` state.
8. A repeated run produces an identical ordered `EnergyTransferLedger`.
9. The legacy untracked replay executor refuses external-heat commands before mutating earlier commands in the same batch.
10. The audited replay executor preflights the complete batch, including staged repeated heat commands and ledger acceptance, so a rejected batch leaves world and ledger unchanged.

## Replay transaction contract

Replay command slices are validation-atomic under the current command vocabulary. Before authoritative mutation:

- every referenced body must exist;
- legacy execution rejects any energy-boundary command;
- audited execution stores staged thermal reservoirs in a deterministic map keyed by `BodyHandle` while applying heat commands in their original command order;
- repeated heat commands for one body observe prior staged heat in the same batch;
- ledger effects are applied to a cloned journal during preflight.

Only after this preflight succeeds are commands committed to the authoritative world and ledger. If future replay commands introduce a new fallible mutation, its preflight rule must be added before that command is allowed into the audited executor.

## Validity limits

This campaign does not validate convection, radiation, phase change, temperature-dependent material properties, spatial gradients, or source dynamics. The external source/sink is prescribed and has no modeled internal state of its own.

The purpose of this campaign is narrower: prove that explicit signed boundary energy changes thermal state by the analytical amount, is reconciled to the causal ledger, closes the first-law boundary audit, and remains deterministic and retry-safe under replay.

Run with:

```bash
cargo run -p symtropy-physics --example external_heat_validation
```
