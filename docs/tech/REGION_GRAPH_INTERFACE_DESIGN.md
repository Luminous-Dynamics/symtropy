# Region Graphs and Explicit Interface Maps

**Status:** Phase C research contract only; not integrated into physics.  
**Dependency:** [PHYS-TOPOLOGY-001 #2876](https://github.com/Luminous-Dynamics/symtropy/issues/2876).  
**Tracking:** [PHYS-REGION-001 #2887](https://github.com/Luminous-Dynamics/symtropy/issues/2887).

## Executive principle

Model region identity as explicit simulation state, not as another coordinate and not as an implicit teleport. A region graph can represent application-defined local spaces and transitions. It does not establish that pocket universes or exotic spacetime topologies physically exist.

Phase C remains blocked until Phase A's domain primitives and Phase B's periodic image/collision work pass independent exact-head verification. This document prepares the interface contract; it authorizes no integration into PhysicsWorld today.

## 1. Data model

```text
RegionGraph
  RegionId -> RegionSpec
  InterfaceId -> RegionInterface(source_region, target_region)

RegionSpec
  local coordinate domain and metric profile
  coordinate units and physical-clock convention
  supported object/state profiles
  model version and state revision

RegionInterface
  source boundary predicate
  target boundary predicate
  position map and declared domain
  optional inverse map and validity domain
  derivative/Jacobian or explicit velocity policy
  orientation and field-state policy
  conservation and interface-exchange contract
```

Stable IDs are non-reused within a replay lineage. A mutable interface must have a monotonically advancing revision so existing receipts cannot accidentally authorize a new map.

### Region specification

Each region must declare:

- coordinate dimension and coordinate units;
- local domain, metric profile, and any periodic-domain identity;
- physical clock identity or a separate local-time coordinate and time-map policy;
- supported object/field profiles and the equations/solver versions that govern them;
- authority scope, state revision, and serialization/replay identity.

Do not permit bodies with incompatible dimensions, units, or solver profiles to cross merely because a numeric map returns a vector of the expected length.

### Interface map

An interface is a directed map between explicit source and destination boundary charts. It declares its domain of validity and must return a typed failure when the source state is outside that domain.

For a differentiable position map f, the velocity push-forward is J_f(x) v when the coordinate and time conventions justify that expression. If the interface maps time as well as position, velocity transformation must account for that time map. If the map is non-differentiable, singular, or undefined at the crossing, the interface must provide a separately justified state-transform rule or reject the transition. Do not guess a velocity from positions alone.

Affine maps compose in traversal order. If f maps A→B and g maps B→C, g∘f must transform position, velocity, local time scale and local time offset consistently; composition must be associative within numeric tolerance, and inverse(composed) must agree with inverse order reversed. The current reference code provides a same-dimension affine composition primitive with position/velocity/clock tests.

An inverse map is optional in general but required for any interface profile that promises reversibility. Test round trips only on the inverse's declared domain. Position invertibility does not establish momentum, energy, or information preservation. A pure time-origin shift does not change the clock rate; conservation checks should gate on compatible clock/unit/metric identities and the time scale, not reject a harmless offset by itself.

## 2. Crossing transaction

Recommended transition lifecycle:

1. Freeze body-state, source-region, target-region, and interface revisions. The transition request must state the expected source/target model revisions as preconditions, not only record them after a commit.
2. Determine the observed crossing direction and choose an interface using deterministic tie-breaking. The source-chart normal velocity must agree with the stated direction in the current affine reference model; tangent/zero velocity or ambiguous crossing history is unresolved rather than guessed.
3. Check source boundary and mapping-domain membership.
4. Evaluate candidate target state into temporary storage; do not mutate authoritative source state yet.
5. Validate finite values, target-domain admission, supported object/profile, orientation/field transforms, optional inverse/round-trip claims, and conservation/exchange residuals.
6. Create a receipt binding crossing ID, body ID, source/target regions, expected and actual source/target model revisions, source state revision, interface revision, before/after state digests, and per-quantity balance.
7. Commit target state and retire source authority atomically. Failure leaves the source authoritative and produces no target body.
8. Replay of the same request/receipt is idempotent; the same crossing ID with conflicting request data, revisions, or digests is rejected.

At any point a failed validation must not duplicate or lose a body. Crossing is a state transition coordinated outside narrow-phase collision detection; GJK should not perform teleportation.

## 3. Explicit conservation contract

Every transported quantity needs a declared policy:

- **Preserve:** the quantity is expected to remain invariant within a stated tolerance. The affine map must be an isometry with unchanged clock rate; coordinate/time units, clock identity, and metric profile must match. The current executable reference ledger supports only the Euclidean metric profile.
- **Transform:** a named transform is required; the reference prototype records its identity but does not claim a conservation law for that quantity.
- **Exchange:** the interface declares a momentum/energy ledger delta, and the before/after receipt must close it. A non-isometric affine map is allowed under Exchange, but coordinate/time units, clock identity, and metric profile must still match because the prototype evaluates both sides with the same Euclidean quantity definitions.
- **Unsupported:** the transition is rejected.

Position-map success cannot stand in for conservation proof. Momentum and energy depend on metric, mass/units, time map, and constitutive model. If a map scales distance by two, for example, the implementation must not silently leave energy and velocity under an unstated convention.

## 4. Failure and concurrency semantics

- One body has exactly one authoritative region at each committed revision.
- Unknown region/interface, stale revision, non-finite mapped state, invalid Jacobian, target rejection, unsupported field/profile, invalid conservation residual, or receipt conflict fails closed.
- A failed transition leaves source state untouched.
- Region IDs and interface IDs participate in deterministic replay and contact identity.
- Bodies in separate regions cannot collide just because local coordinates are numerically equal; cross-region collision requires an explicitly modeled interaction.
- Simultaneous transitions have deterministic ordering or an explicit unresolved disposition.
- Boundary jitter uses an explicit crossing tolerance and, where appropriate, hysteresis/cooldown; these parameters are part of the replay digest.
- Chained transitions have a bounded per-step budget and deterministic cycle policy.
- Local-time coordinates remain separate from physical time unless a common clock or time transform is declared.

## 5. Independent validation

Start with maps whose expected behavior is known:

1. Identity map: no state changes, no conservation residual.
2. Translation and orthogonal rotation: analytic inverse and Jacobian.
3. Affine scaling: known position round trip and deliberately non-preserved Euclidean energy unless explicit state transforms compensate.
4. Two-interface composition: compare composite map with direct analytic composition.
5. No-inverse interface: must not receive a reversible label.
6. Crossing exactly on a shared boundary and simultaneous candidates: deterministic tie or explicit unresolved result.
7. NaN/infinite output, stale interface version, unsupported region, and deliberately bad conservation map: no partial commit.
8. Duplicate replay: one authoritative transition; conflicting replay: rejected.
9. Separate regions with coincident coordinates: no collision without a cross-region interaction rule.
10. Boundary jitter, cycles, long chains, budget exhaustion, graph insertion-order permutations, and randomized interface graphs.

Measure absolute and scale-relative round-trip errors for position and velocity. Publish per-quantity residuals in declared units; don't combine them into a scalar that can hide a conservation failure.

## 6. Benchmark and comparison programme

Metrics:

- transitions per second and p50/p95/p99 transition latency;
- time and memory by number of regions, interfaces, and body crossings;
- per-quantity conservation/exchange residuals;
- position/velocity round-trip error, Jacobian evaluation cost, and rejected/ambiguous outcomes by reason;
- duplicate transition rate, source-body loss/duplication rate, cycle-detection time, and budget-exhaustion frequency;
- replay hash agreement under repeated execution and graph insertion-order perturbation.

Compare against a conventional explicit state machine/graph-transition engine. The point is to demonstrate that a typed region interface offers stronger numerical and conservation contracts, not to claim a new mathematical topology engine solely because the data structure has nodes and edges.

Hard gates: exactly-one authority, state finiteness, failure atomicity, idempotent replay, and the declared conservation/rejection policy. Performance only counts after these pass.

## 7. Prerequisites and claim ceiling

Before future implementation:

- [ ] Phase A domain primitives pass exact-head tests and an independent property corpus.
- [ ] Phase B image candidates match brute-force translated-image enumeration; seam CCD and duplicate-contact handling are independently verified.
- [ ] Every prerequisite result is bound to a commit SHA, scenario digest, source/toolchain identity, and preserved failure corpus.
- [ ] Region/interface schemas state units, time conventions, map domains, derivative policy, conservation policy, and versioning.
- [ ] Transition tests prove atomicity, idempotency, and deterministic failure behavior.

This design represents finite, explicit region graphs and declared interface maps. It does not model arbitrary curved manifolds, establish general-relativistic solutions, or show that pocket universes exist.