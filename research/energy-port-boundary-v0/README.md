# Typed Energy-Port Boundary Contract V0

This is the first integration-oriented tranche for PHYS-ENERGY-PORTS-001
(#3615). It does not yet retrofit the legacy coupling counters. It establishes
a typed, auditable event model and a pure aggregation rule that can be tested
before production pathways are migrated.

## Paradigm shift: account across a boundary cut

The system is represented as named energy reservoirs connected by directed
transfer events:

```text
source reservoir --[channel, joules, event ID]--> destination reservoir
```

A selected system boundary is a set of included reservoirs. An event contributes
to the boundary balance only if its source and destination lie on opposite
sides of that cut:

- outside -> inside: boundary inflow;
- inside -> outside: boundary outflow;
- inside -> inside: internal transfer; it cancels at the aggregate boundary;
- outside -> outside: irrelevant to this boundary.

The same event may therefore be an outflow for an entity boundary and an
internal transfer for a world boundary that contains both the source and
destination. This is the correct semantics for internal heating: mechanical
energy converted into stored thermal energy is not automatically heat leaving
the whole simulation.

## Types and safety

`EnergyPortEvent` uses:
- a stable event ID;
- a channel: Work, Heat, or Matter;
- stable source and destination reservoir IDs;
- a non-negative `Joules` amount.

The event constructor rejects negative/non-finite amounts and self-transfers.
An impulse, force, or power scalar cannot be passed directly: callers must make
an explicit conversion to `Joules`, which is an intentional review boundary.
A collision impulse alone is insufficient to infer energy. Valid options include
a mass-and-velocity kinetic-energy delta, or an independently defined
work/energy quantity from the solver.

`EnergyPortBoundary` names its included reservoirs and preserves an explicit
`BoundaryCoverage` assertion. Completeness is never inferred from the number
of events. Unknown pathways must remain Incomplete.

## Deterministic aggregation

`aggregate_boundary_ports` sorts by stable event ID before summing. This makes
the floating-point summation order independent of producer scheduling. Duplicate
IDs are rejected, as are arithmetic overflow and invalid transfer values.

The result carries:
- boundary ID and caller-declared coverage;
- total event count and number of crossing events;
- typed first-law boundary flows (work/heat/matter, in/out).

`EnergyPortSummary::to_balance_input` bridges those flows to the pure
`EnergyBalanceInput` evaluator. Incomplete coverage remains Inconclusive even
when the arithmetic closes exactly.

## Current evidence and known limits

The tests cover:
1. Internal mechanical-to-thermal transfer not counted as whole-world heat out.
2. The same transfer counted as outflow for a smaller subsystem boundary.
3. Directional separation of work, heat and matter-energy flows.
4. Stable aggregation independent of input/event completion order.
5. Duplicate event IDs rejected.
6. Incomplete coverage propagated into the first-law evaluator.
7. Negative/non-finite amounts, self-transfers, empty boundaries and overflow
   rejected.

This is a pure accounting contract, not yet production instrumentation. It does
not prove that all existing coupling paths emit events, that every reservoir
has a physically calibrated stored-energy model, or that caller completeness
assertions are true.

## P0 dimensional audit findings to preserve

The current coupling implementation still has several legacy issues which this
contract must not silently validate:

- `CollisionEvent.impulse` is impulse/momentum, not joules. The current
  `process_collisions` path records a fraction of impulse as heat; that must be
  removed or replaced with a physical kinetic-energy delta.
- Sanctuary impulse modulation currently has a simplified impulse-to-energy
  calculation. It must remain impulse telemetry until mass/velocity or a
  solver-derived energy quantity supports conversion.
- Dimensional leakage source gains are currently encoded as negative
  dissipation; source inflow and dissipation must be distinct.
- Regenerative braking currently restores a fixed fraction and writes directly
  to lifetime telemetry. Recovery, loss and stored-thermal change need separate
  records.
- `ThermodynamicLedger::record_dissipation` is currently a legacy scalar
  telemetry path. It is not proof of heat leaving a declared system boundary.

## Next integration order

1. Map every energy quantity and producer to dimensions and a physical reservoir.
2. Add stable event sequence identity and a single recording owner for collision
   and actuator events.
3. Migrate dimensional leakage source/sink events without assuming leakage is
   work, heat or matter until the mechanism has a physical definition.
4. Replace impulse-to-joule heuristics with solver-derived kinetic-energy
   differences; keep the raw impulse as a separate momentum observable.
5. Model thermal reservoirs and temperature/entropy state separately from
   boundary heat transfer.
6. Build interval snapshots from all in-boundary stored-energy reservoirs, then
   evaluate first-law residuals. Any uninstrumented producer/consumer keeps the
   result Inconclusive.

## Benchmark targets

The qualification matrix should include:
- isolated closed boundary with zero crossing ports;
- work and heat inflow with analytically known stored-energy increase;
- work/heat/matter outflow;
- internal reservoir-to-reservoir conversion (zero net whole-boundary flow);
- nested entity and whole-world boundaries over the same events;
- duplicate/missing event IDs and unknown pathway coverage;
- energy residual vs independent analytic reference;
- event-throughput and aggregation latency at 10^2, 10^4 and 10^6 events.

For published performance, report event count, boundary reservoir count, channel
mix, p50/p95 aggregation latency, allocations, source/config digest and run
repetitions. Performance must never be purchased by dropping events or weakening
coverage validation.
