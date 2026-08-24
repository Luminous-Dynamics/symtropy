# Friction Dissipation to Heat Validation Campaign

This campaign validates a closed mechanical-plus-thermal conversion with no external energy source.

## Why this exists

The current world contact solver historically reported a heuristic friction-dissipation quantity proportional to impulse magnitude. Impulse has units of momentum, not energy, so that quantity must not be treated as joules.

The new coupling instead measures translational plus modeled rotational kinetic energy immediately before and after a supplied friction impulse. Only a positive pair kinetic-energy loss is eligible for thermal conversion.

## Analytical centered case

Two equal 1 kg bodies begin with velocities

- A: 1 m/s
- B: 0 m/s

A centered 0.5 N s impulse is applied to B and the opposite impulse to A.

The exact post-impulse velocities are

- A: 0.5 m/s
- B: 0.5 m/s

Therefore

- initial kinetic energy = 0.5 J
- final kinetic energy = 0.25 J
- dissipated kinetic energy = 0.25 J

With an equal heat partition and each body having thermal capacity `m c_p = 1000 J/K`, each body receives 0.125 J and warms by 0.000125 K.

## Ledger decomposition

The ledger distinguishes kinetic transfer from dissipation:

- 0.125 J: A kinetic -> B kinetic
- 0.125 J: A kinetic -> A sensible heat
- 0.125 J: A kinetic -> B sensible heat

Thus A loses 0.375 J of kinetic energy, B gains 0.125 J of kinetic energy, and the pair gains 0.25 J of sensible heat.

## Acceptance criteria

`friction_heat_validation` must show:

1. post-impulse kinetic energy matches the closed-form result within `1e-12 J`
2. measured dissipation matches 0.25 J within `1e-12 J`
3. each body reaches the analytical temperature within `1e-12 K`
4. total mechanical + thermal energy closes within `1e-9 J`
5. net external ledger flow is exactly zero
6. per-reservoir ledger changes match the measured state changes
7. an energy-injecting impulse is rejected and all state is rolled back

## Scope

This validates the coupling primitive, not yet the full world contact loop. Integrating it into the solver requires replacing the existing heuristic dissipation estimate with measured pre/post kinetic-energy accounting around the actual friction impulse. The separation is deliberate: no heuristic value should become heat simply because a thermal subsystem now exists.

Run with:

```bash
cargo run -p symtropy-physics --example friction_heat_validation
```
