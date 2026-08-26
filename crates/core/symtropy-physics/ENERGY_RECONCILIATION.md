# Energy Reservoir Reconciliation

Symtropy now has three complementary accounting questions:

1. **Boundary closure** — does total modeled internal-energy change match net external ledger flow?
2. **Reservoir reconciliation** — does each stable tracked reservoir change by the amount the ledger claims?
3. **Reservoir lifecycle** — did the set of represented reservoirs itself remain stable across the interval?

The stronger checks matter because total conservation can pass while compensating bookkeeping mistakes cancel, and a numerically zero reservoir can appear or disappear without changing the total at all.

## Tracked core reservoirs

`EnergyStateSnapshot` currently captures, per body:

- kinetic energy for dynamic bodies
- uniform-gravity potential energy for dynamic bodies
- sensible thermal energy for bodies with attached thermal state

The thermal reference temperature is explicit and must match between compared snapshots.

Snapshot capture and reconciliation revalidation reject:

- non-finite measured reservoir values
- overflow in the tracked total
- duplicate reservoir identity (for example duplicate body handles creating the same energy port)
- non-finite aggregated ledger deltas or state-versus-ledger residuals

This prevents invalid serialized or post-capture-mutated evidence from becoming an apparently valid audit.

## Stable-reservoir reconciliation rule

For a reservoir represented at both endpoints,

`residual = measured_state_delta - ledger_net_delta`.

A fully reconciled interval requires:

- every stable tracked reservoir residual to be within tolerance
- total first-law boundary closure to be within tolerance
- no internal ledger port to appear that the state snapshot cannot represent
- no represented reservoir to appear or disappear during the interval

The last rule is deliberate. **Absent is not zero.** A body with no thermal reservoir is not the same modeled state as a body with a thermal reservoir containing `0 J` at the chosen reference.

When a tracked port appears or disappears, `ReservoirReconciliation` leaves its numeric measured delta/residual as `None` and the audit records an explicit `ReservoirPresenceChange`. The interval cannot be `fully_reconciled` until a future lifecycle-provenance mechanism explains reservoir creation, attachment, replacement, or removal.

Likewise, if a future solver journals chemical, latent, fracture-surface, elastic, electrical, or another internal energy form before the corresponding state variable exists, the audit reports that port as untracked rather than silently assuming zero state.

## Validation

`energy_reconciliation_validation` exercises three cases:

1. **Audited 1000 J heating** through `exchange_external_heat_audited` — expected to reconcile completely.
2. **Direct unjournaled 1000 J heating** — expected to expose a 1000 J unexplained thermal residual and total closure error.
3. **Zero-energy reservoir appearance** — a body starts without thermal state, then receives a valid thermal reservoir at 0 K with an empty ledger. The numeric total still closes at exactly 0 J, but reconciliation must fail because a reservoir appeared without provenance.

The third case is intentionally adversarial: it proves the system cannot confuse a topology/lifecycle change in modeled state with numeric conservation.

Run with:

```bash
cargo run -p symtropy-physics --example energy_reconciliation_validation
```

## Scope

The snapshot intentionally represents only energy forms that exist as explicit state in the current core. As elastic strain energy, latent heat, fracture-surface energy, chemical energy, electrical energy, and other reservoirs become real state variables, they should be added to this snapshot rather than inferred from ledger traffic alone.

This PR detects reservoir appearance/disappearance but does not yet provide the provenance vocabulary that would make such transitions physically accountable. That belongs with the forthcoming authoritative body/reservoir lifecycle work: attachment, replacement, detachment, body creation/removal, and representation transitions should each carry explicit causal receipts rather than being inferred from endpoint state.

`RigidBody::kinetic_energy()` also remains the engine's current modeled kinetic quantity; it does not upgrade the known scalar-mean rotational-inertia approximation. Reconciliation can prove that the ledger agrees with the modeled reservoir, not that the reservoir model itself is already physically exact.
