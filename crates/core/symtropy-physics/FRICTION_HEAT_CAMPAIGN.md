# Friction Dissipation to Heat Validation Campaign

This campaign validates a closed, centered mechanical-plus-thermal conversion with no external energy source.

## Why this exists

The current world contact solver historically reported a heuristic friction-dissipation quantity proportional to impulse magnitude. Impulse has units of momentum, not energy, so that quantity must not be treated as joules.

The coupling in this PR instead measures the pair's modeled kinetic energy immediately before and after a supplied centered friction impulse. Only a positive pair kinetic-energy loss is eligible for thermal conversion.

## Current validity boundary

This campaign deliberately validates **centered impulses between two dynamic bodies only**.

Off-center contact is rejected by the primitive until the engine's P0 angular migration is complete. The current `RigidBody::kinetic_energy()` rotational term uses scalar-mean inertia, and the Rotor/Bivector contact-kinematics convention is separately known to require coordinated migration. Allowing off-center friction-to-heat now would risk converting uncertainty in rotational dynamics into apparently physical thermal energy.

Static and kinematic partners are also rejected by this closed-pair primitive. Their contact response exchanges work/momentum with an omitted mechanical reservoir; treating the resulting dynamic-body kinetic loss as entirely local frictional heat would not be a closed accounting model.

## Analytical centered case

Two equal 1 kg dynamic bodies begin at the same modeled contact point with velocities

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

With an equal heat partition and each body having validated thermal capacity `m c_p = 1000 J/K`, each body receives 0.125 J and warms by 0.000125 K.

## Ledger decomposition

The causal ledger distinguishes pure kinetic transfer from dissipation:

- 0.125 J: A kinetic -> B kinetic
- 0.125 J: A kinetic -> A sensible heat
- 0.125 J: A kinetic -> B sensible heat

Thus A loses 0.375 J of kinetic energy, B gains 0.125 J of kinetic energy, and the pair gains 0.25 J of sensible heat. Every positive modeled state transfer receives a ledger entry; tiny positive transfers are not silently dropped.

Before commit, the primitive reconciles the staged ledger against all four modeled reservoir deltas: A kinetic, B kinetic, A sensible heat, and B sensible heat.

## Transaction and validity contract

Before any mechanical mutation, the primitive requires:

- distinct body handles;
- finite impulse and contact point;
- a valid heat partition;
- two dynamic bodies;
- finite modeled mechanical state;
- present and revalidated thermal reservoirs;
- finite non-negative pre-impulse modeled kinetic energy;
- centered contact within the declared tolerance.

After applying the staged impulse it additionally requires finite post-impulse kinetic energy, a strictly positive measured pair loss above the tiny numerical deadband, representable heat partition, consistent kinetic-transfer/dissipation decomposition, successful thermal updates, and per-reservoir ledger reconciliation.

Any failure after staging restores both bodies and the entire pre-existing ledger. Finite accounting inconsistency is reported as `LedgerStateMismatch`, not misclassified as non-finite arithmetic.

## Acceptance criteria

`friction_heat_validation` must show:

1. post-impulse kinetic energy matches the closed-form centered result within `1e-12 J`;
2. measured dissipation matches 0.25 J within `1e-12 J`;
3. the reported per-body kinetic changes equal measured state changes;
4. each body reaches the analytical temperature within `1e-12 K`;
5. strict complete-accounting total mechanical + thermal relative closure is <= `1e-12`;
6. net external ledger flow is exactly zero;
7. measured changes of all four reservoirs agree with ledger net changes to <= `1e-12` relative error;
8. the ledger contains the expected three typed `Friction` transfers;
9. an energy-injecting impulse rolls back bodies and pre-existing ledger history;
10. invalid public thermal mutation is rejected before mechanical mutation;
11. off-center impulses are explicitly rejected rather than entering an uncertified rotational path.

## Scope

This validates the centered coupling primitive, not the full world contact loop. Full solver integration must first replace the existing heuristic dissipation estimate with measured pre/post kinetic-energy accounting around the actual friction impulse **and** complete the angular convention/full-inertia work needed for off-center contacts.

The separation is deliberate: no heuristic impulse quantity, solver stabilization loss, static-boundary work, or uncertified rotational error should become heat simply because a thermal subsystem exists.

Run with:

```bash
cargo run -p symtropy-physics --example friction_heat_validation
```
