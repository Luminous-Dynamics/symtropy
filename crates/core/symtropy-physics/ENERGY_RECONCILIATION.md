# Energy Reservoir Reconciliation

Symtropy now has two complementary conservation checks:

1. **Boundary closure** — does the total modeled internal-energy change match net external ledger flow?
2. **Reservoir reconciliation** — does each tracked reservoir change by the amount the ledger claims?

The second check matters because total conservation can pass while two compensating accounting errors cancel each other.

## Tracked core reservoirs

`EnergyStateSnapshot` currently captures, per body:

- kinetic energy
- uniform-gravity potential energy for dynamic bodies
- sensible thermal energy for bodies with thermal state

The thermal reference temperature is explicit and must match between compared snapshots.

## Reconciliation rule

For each tracked internal port,

`residual = measured_state_delta - ledger_net_delta`.

A fully reconciled interval requires:

- every tracked reservoir residual to be within tolerance
- total first-law boundary closure to be within tolerance
- no internal ledger port to appear that the state snapshot cannot yet represent

The last rule is deliberate. If a future solver journals chemical, latent, fracture-surface, elastic, or electrical energy before the corresponding state variable exists, the audit reports that reservoir as untracked rather than silently assuming zero state.

## Validation

`energy_reconciliation_validation` runs two otherwise equivalent heat-input cases:

1. 1000 J applied through `exchange_external_heat_audited` — expected to reconcile.
2. 1000 J applied directly to body thermal state with an empty ledger — expected to expose a 1000 J unexplained residual and total closure error.

This is a negative-control test for the accounting architecture: the audit must detect a physically plausible state change when its causal energy transfer was not recorded.

Run with:

```bash
cargo run -p symtropy-physics --example energy_reconciliation_validation
```

## Scope

The snapshot intentionally represents only energy forms that exist as explicit state in the current core. As elastic strain energy, latent heat, fracture-surface energy, chemical energy, and other reservoirs become real state variables, they should be added to this snapshot rather than being inferred from ledger traffic alone.
