# ANIMA Stochasticity V0

Status: normative Living World integration contract. Documentation only.

## Purpose

Define how ANIMA may use bounded stochastic variation without making canonical behavior depend on mutable global RNG consumption, ECS iteration order, thread scheduling, renderer cadence, or debug instrumentation.

## Parent contracts

This contract refines the determinism requirements in `ANIMA_DECISION_ACTION_V0.md`, the canonical tick model in ANIMA scheduling work, and the profile/version identity rules of ANIMA species/policy configuration.

It does not require randomness. Deterministic causal behavior remains the default.

## Core invariant

> Any stochastic outcome that can change future canonical behavior has a stable causal draw identity.

A random sample is part of authoritative semantics only when its complete key/policy/version are defined.

## Deterministic-first rule

Use stochasticity only where the model intentionally represents unresolved variability, bounded exploration, procedural diversity, or a declared stochastic process.

Prefer:

`qualified evidence + body state + history + policy -> explainable action`

over:

`random chance -> action`

when an affordable causal model exists.

Randomness must not substitute for fear, trust, fatigue, affordance, uncertainty, or perception mechanics.

## Keyed canonical draws

Canonical draws should prefer a stateless keyed construction equivalent to:

```text
scenario/world seed
+ stable agent/process identity
+ policy/profile identity
+ canonical tick or causal event identity
+ purpose tag
+ ordinal only when semantically required
        -> versioned deterministic draw
```

The key must not depend on incidental container position or task execution order.

The exact implementation may use a deterministic PRF/hash-to-sample/RNG scheme, but the algorithm and mapping are versioned semantics.

## Purpose domains

Different canonical purposes use separate domains, for example:

- temperament realization;
- exploratory choice;
- declared tie break;
- sensor noise model;
- ecological sampling;
- coarse population closure.

Presentation-only randomization uses a separate domain and must not feed canonical state.

## Distribution identity

A distribution is defined by more than a seed. Canonical policy freezes:

1. algorithm/version;
2. purpose/domain tag;
3. key composition/order;
4. raw sample mapping;
5. distribution transform;
6. parameter/profile identity;
7. bounds;
8. rounding rules;
9. failure/range semantics;
10. schema version when any of these change.

Changing these semantics changes evidence identity.

## Ordering independence

Adding an unrelated animal, changing ECS iteration order, enabling trace output, or moving work between threads cannot perturb another agent's draw.

A mutable global or per-world stream whose next value depends on unrelated consumption is not an admissible canonical default.

## Streams and counters

If a subsystem genuinely requires a sequential stream, the stream identity and counter become future-bearing state.

Such state must participate in persistence, fidelity, replay, and authority handoff.

Prefer stateless keyed draws when the same semantics can be represented without a mutable cursor.

## Numeric output

Bounded normalized outputs follow the canonical ANIMA quantitative semantics.

A stochastic confidence/risk/strength sample remains semantically typed; sharing a numeric representation does not make those concepts interchangeable.

## Multiplayer authority

Only the active canonical authority for an agent/process may commit a stochastic outcome that changes authoritative biography or action.

Other peers may recompute the same result from a sealed key for validation where appropriate, but do not independently commit competing stochastic futures.

## Persistence

A snapshot immediately before a canonical stochastic decision must continue with the same outcome as an uninterrupted run under equivalent subsequent inputs.

Stateless keyed draws naturally satisfy this when their key material is preserved. Stateful streams require exact counter/state persistence.

## Fidelity

A coarse representation may replace fine-grained stochastic processes only when Living World information-sufficiency qualification proves the coarse closure preserves required semantics.

Representation demotion cannot silently reseed an individual.

## Qualification direction

At minimum test:

1. same seed/agent/policy/tick/purpose -> exact replay-identical draw;
2. permuting entity/container order cannot change draws;
3. adding/removing an unrelated agent cannot change another agent's draws;
4. trace/Observatory level cannot change canonical draws;
5. render FPS cannot change canonical draws;
6. distinct purpose domains remain separated;
7. save/reload immediately before a draw matches uninterrupted execution;
8. authority handoff preserves the next stochastic outcome;
9. profile/distribution semantic changes alter explicit policy identity;
10. cosmetic random seed changes cannot alter canonical ledgers.

## Non-goals

This contract does not claim cryptographic randomness, require randomness for animal behavior, authorize arbitrary online-RL exploration, permit random refusal as a substitute for causal modeling, or allow renderer/debug RNG state to become simulation authority.

Relates to #1191, #1193, #1195, #1197, #1198, #1199 and #170.
