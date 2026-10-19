# MINE-CASCADE-001: simulation contract v0.1

## Purpose

This benchmark tests the composition of service availability over time in a synthetic underground-mine dependency graph. It complements MINE-SYSTEM-001: the campaign evaluator accounts for material, energy, product and constraints; this module checks whether local service states compose into a viable system as faults and evidence updates arrive.

It does not simulate detailed mine physics. Ventilation, geotechnics, water inflow, radio propagation, machine behavior and safety procedures remain separate models and authorities. The simulator consumes explicit service capacities, local states, evidence freshness, dependencies and failure domains.

## Paradigm shift

A component marked available is not sufficient to claim that its dependent service is viable. Service viability requires a known available local state, current evidence, capacity at or above the declared minimum envelope, satisfied dependency rules, and no active common-cause failure in its declared failure domain.

Dependencies support AllOf (every prerequisite) and AtLeast (explicit redundancy/quorum). This avoids assuming either that any upstream service suffices or that every redundant supply must be healthy.

A stale or unknown service is Unknown, not Viable. For an AtLeast rule, unknown candidates prevent a definitive failure classification only when they could still satisfy the threshold; they never count as viable.

## Deterministic simulation semantics

- Time is represented by integer ticks; callers declare the real-world duration per tick.
- Events are sorted by (at_tick, event_id). IDs are unique and define same-tick ordering.
- All services are reevaluated after every event; restoration of one component does not bypass downstream prerequisites.
- Service availability/unknown burden is measured as service-ticks over [0, horizon_tick). Concurrent affected services count separately; this is not wall-clock mine downtime.
- Cyclic graphs, unknown service references, duplicate IDs, invalid thresholds, non-finite/negative capacities, events past the horizon and exceeded service/event caps are rejected.
- Invalid input or a cap breach returns an explicit error; no partial run is reported as complete.
- Reports preserve post-event dispositions, canonical event order, per-service burden and critical-service burden.

## Frozen v0.1 cases

1. prerequisite loss and later restoration;
2. two redundant pumps with a shared power failure domain;
3. two redundant pumps with independent failure domains;
4. stale evidence propagation;
5. deterministic same-tick replay independent of input vector order;
6. cyclic graph rejection and explicit service-count cap;
7. unknown redundancy candidate not counted as viable.

Positive and negative controls are both required. The target is not to fail everything closed: independently powered redundancy should remain viable in the control case.

## Proposed benchmark gates

All are proposed targets, not observed results:

- 100% repeatability for identical scenario/policy/build inputs.
- Zero false-viable outcomes in frozen unknown/stale/common-cause controls.
- 100% agreement with an independently implemented reference on tiny graphs.
- Explicit failure for malformed references, cycles and cap overflow.
- Policy comparisons use identical event traces, evidence delivery times and resource envelopes.
- Adaptive recovery is not presumed better; it must improve a declared held-out metric without violating hard invariants.
- No weighted aggregate can hide unknown or critical-service loss.
- Synthetic results remain synthetic until calibrated models and held-out physical observations support stronger evidence claims.

## Next model layer

Add repair crews, spares, access-constrained repair precedence, restoration delays and policy decisions limited to observations delivered by the current tick. Integrate by references/adapters to existing communication-timeline and flood-egress benchmarks, not by duplicating their physics or receipt schemas.

## Research anchors

- Danko et al., Applications and verification of a computational energy dynamics model for mine climate simulations (2020): https://doi.org/10.1016/j.ijmst.2020.03.019
- A digital engineering framework for information-centric modelling of drill-and-blast tunnel excavation processes (2026): https://doi.org/10.1016/j.dte.2026.100096
- A framework to define, design and construct digital twins in the mining industry (2025): https://doi.org/10.1016/j.cie.2024.110805
- MineLib benchmark data and descriptions: https://mansci-web.uai.cl/minelib/Datasets.xhtml

These sources motivate model verification, data dependencies and benchmark design; none validates this synthetic service simulator as a mine-safety model.
