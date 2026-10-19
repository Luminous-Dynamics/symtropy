# Energy Boundary Accounting Adapter V1

**Status:** additive research adapter; production migration not yet established  
**Reference parent:** [THERMO-LEDGER-001 reference PR #2113](https://github.com/Luminous-Dynamics/symtropy/pull/2113)  
**Issue:** [#1693](https://github.com/Luminous-Dynamics/symtropy/issues/1693)  
**Module:** crates/domains/symtropy-consciousness-physics/src/energy_boundary_accounting.rs

## Why a second, explicit contract

The current ThermodynamicLedger API aggregates action cost into energy_in and dissipation into energy_out. Its tick_balance() computes energy_in - energy_out and derives conservation_error from that flow-only difference. There is no initial/final stored-energy input in that method.

That flow-only residual can equal zero even if stored energy has changed. The frozen boundary reference in PR #2113 showed the counterexample and establishes a separate declared-boundary accounting identity. This PR makes that identity available as a pure Rust API, without silently changing the behavior or interpretation of the existing ledger.

## Contract

For a single explicitly named boundary and one common energy unit:

    R = E_initial + sum(boundary_inflows)
        - sum(boundary_outflows) - E_final

The assessment computes the scale as:

    S = E_initial + sum(boundary_inflows)
        + sum(boundary_outflows) + E_final

and accepts arithmetic balance when:

    abs(R) <= relative_tolerance * max(1, S)

The tolerance must be finite and in (0, 0.01] (maximum 1%). All energy magnitudes must be finite and non-negative. Boundary flow magnitudes are sorted and accumulated with compensated summation, making totals independent of the caller's flow-vector order and reducing loss when small and large flows coexist. Arithmetic accumulation overflow is rejected. Each event ID must be unique across boundary flows and internal transfers.

### Boundary events versus internal movement

- Boundary inflows and outflows are represented by a typed direction. Flow magnitude itself is never negative.
- An internal transfer links two different, explicitly tracked stores. It is validated but does not change the boundary-flow totals.
- One common unit is declared for the snapshot. Unit conversion and dimensional correctness belong to the caller's validated quantity layer; this API does not infer that two external values are commensurate.
- The function is pure: it consumes one immutable snapshot and returns an assessment or an explicit error.

### Existing versus additive semantics

| Existing API | New boundary accounting API |
|---|---|
| Records action cost and dissipation counters | Receives initial/final stored-energy snapshots plus directed external flows |
| tick_balance() uses energy_in - energy_out | residual includes initial storage, external flows and final storage |
| Does not distinguish internal transfer events | Validates internal transfers without counting them as boundary flow |
| Existing compatibility contract retained | New, separately named API with explicit boundary and unit |

No existing production call site is changed by this PR. The new module is exported for later integration after its inputs can be traced to authoritative stores and flow events.

## Tests added

Nine focused unit tests cover:

1. a balanced boundary with stored energy and in/out flows;
2. hidden stored-energy drift despite equal total inflows and outflows;
3. internal transfer not changing the boundary residual;
4. input event-order invariance, including a mixed-scale test with a real discrepancy that naive summation can erase;
5. duplicate event IDs across boundary and internal event classes;
6. unknown and same-store transfer endpoints;
7. negative/non-finite energy magnitudes, invalid stored energy and tolerance above the 1% cap;
8. numeric overflow during flow accumulation.

The parent frozen ledger corpus and independent oracle remain part of the stacked branch and are run separately by CI, so the additive API does not replace the reference corpus.

## Claim ceiling

Passing tests establishes implementation conformance to this finite arithmetic contract. It does **not** establish:

- that every real reservoir or boundary-crossing flow is enumerated;
- that an input event is authentic, correctly measured, or recorded exactly once upstream;
- that energy units are validly converted or that the system's physical model is correct;
- that the legacy ThermodynamicLedger already obeys the new boundary contract;
- a general thermodynamic conservation theorem or a physical validation of the consciousness/physics coupling.

The next integration task is to map authoritative storage snapshots and event producers to this contract, then compare both interpretations side-by-side on frozen simulation traces. Only after that comparison should any legacy metric or caller be migrated.

## Acceptance gates

- All nine new unit tests pass.
- The parent ledger-reference corpus remains 24/24 expected decisions, with all invalid-domain cases rejected.
- Formatting, locked metadata, crate unit tests and a clean checkout postflight pass on the exact tested SHA.
- No production correspondence or physical conservation claim is made merely from the adapter passing.
