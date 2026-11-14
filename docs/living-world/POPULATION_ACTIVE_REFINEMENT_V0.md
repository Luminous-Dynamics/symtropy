# Living World Population Active Refinement v0

Status: companion authority contract. This document separates presentation-only individual projection from temporary canonical individual refinement and persistent organism identity.

## Why this distinction is required

A coarse population may be materialized into individual-looking members for rendering without making those members authoritative.

That is safe only while the generated microstate cannot affect canonical history.

If a synthetic tuple such as:

```text
juvenile + stressed + cell A
```

changes pursuit speed, predation outcome, collision, injury, resource consumption, reproduction, or any other authoritative event, then the tuple has already influenced the world's future.

Promoting it to authority only *after* that event is too late.

Living World therefore needs three distinct levels rather than one generic "materialized individual" concept.

## Three authority levels

### Level P — presentation projection

A `PresentationProjection` or equivalent is derived, replaceable, and non-authoritative.

It may control:

- meshes;
- animation phase;
- fur/feather/leaf presentation;
- presentation-only pose variation;
- non-authoritative particles;
- local avoidance whose result is discarded and cannot influence canonical simulation;
- debug/scientific visualization.

Destroying and reconstructing Level P cannot change ecology.

### Level A — active canonical refinement

An `ActivePopulationRefinement` or equivalent is a temporary but canonical refinement of an explicitly reserved coarse population subset.

Its microstate may affect authoritative local simulation.

Examples include:

- exact local position used by canonical interactions;
- condition used by authoritative locomotion/escape capacity;
- individual resource consumption;
- local predation;
- injury;
- local disease exposure;
- migration paths;
- interaction with terrain/destruction;
- other state whose consequences survive the current frame.

Level A is authoritative from the explicit refinement transition forward. It does **not** claim that unresolved individual relationships were historically known before that transition.

### Level I — persistent organism identity

A persistent organism has individual authority that must survive active-refinement collapse and distance/LOD changes.

Examples include:

- companions;
- named/tracked animals;
- organisms with durable wounds or memories that cannot be represented sufficiently by the coarse state;
- parent/offspring lineages requiring identity;
- landmark trees;
- quest/narrative organisms;
- individuals participating in canonical biography that must remain individually addressable.

Level I remains excluded from exchangeable coarse count/biomass authority for as long as that persistent identity is authoritative.

## Authority ladder

The intended transition structure is:

```text
coarse population authority
        |
        | reserve
        v
reserved coarse authority
        |
        +---- presentation projection (P)
        |          |
        |          +-- no canonical feedback
        |
        | explicit causal refinement
        v
active canonical microstate (A)
        |
        +---- aggregate/collapse when safe
        |
        +---- persistent crystallization
                    |
                    v
            persistent organism (I)
```

Presentation projection may be generated from either coarse/reserved or active/persistent state, but it never becomes the source of biological authority.

## Refinement is an authority transition

Creating Level A is not merely an LOD operation.

It resolves previously unrepresented degrees of freedom into a canonical active microstate that may now influence the future.

Therefore the refinement trigger and refinement seed must come from authoritative inputs.

Invalid authority inputs include:

- render frame number;
- GPU scheduling;
- nondeterministic ECS iteration order;
- camera culling timing;
- presentation LOD alone;
- wall-clock timing.

Potential valid inputs include:

- canonical simulation tick;
- population/region scope;
- authoritative interaction/activation volume;
- explicit population reservation plan;
- deterministic refinement generation/seed;
- later, canonical causal event identity once that infrastructure is qualified.

## Refinement must not be caused by rendering

A camera may request Level P presentation.

It may not silently cause Level A authority simply because something became visible.

If product policy chooses observation itself as a canonical interaction, the observation must cross an explicit simulation boundary and become an authoritative event independent of renderer timing.

This prevents changing graphics settings from changing ecology.

## Active handles are not necessarily persistent IDs

Level A needs stable references during one active refinement epoch, but those references need not become globally persistent organism identities.

A concept such as:

```rust
pub struct ActiveMemberHandle {
    refinement_generation: u64,
    ordinal: u64,
}
```

may identify a member while the refinement is active.

The exact form may differ.

Required properties:

- handles are unique within their refinement authority scope;
- stale handles from a prior refinement generation are rejected;
- a Level A handle cannot be accepted where persistent Level I identity is required;
- collapse invalidates active-only handles unless promoted/preserved explicitly.

## Canonical microstate selection

When coarse state contains sparse strata, active refinement should draw from those strata so joint properties already represented canonically are preserved.

When only marginals exist, resolving a joint active microstate is a real information-selection event.

That selection must be:

- deterministic from authoritative refinement inputs;
- compatible with every canonical marginal/reservation constraint;
- explicit about which unresolved correlations are being chosen;
- not backdated into history before refinement.

The sufficient-statistics contract still applies: if a correlation affects coarse dynamics before refinement, it should already live in coarse authority rather than being invented at Level A.

## Refinement-local canonicality

Once Level A is created, its selected microstate is canonical for the lifetime of that refinement.

It cannot be regenerated every frame from a different seed.

For example, an animal cannot alternate between healthy and stressed active tuples merely because its render entity was reconstructed.

The active refinement owns that state until:

- an explicit active transition changes it;
- it is collapsed back into coarse authority;
- or it becomes persistent Level I authority.

## Collapse

Level A may collapse back into coarse state when the target coarse representation is sufficient for everything that must survive.

Collapse must preserve all promised coarse observables exactly or within their qualified closure tolerance.

Examples:

- count;
- fixed-point biomass;
- sparse strata/correlation budget;
- disease state;
- occupancy;
- latent/hysteresis variables;
- genetic observables;
- ecological account ownership.

Active-only ordering, gait phase, exact transient path, and other intentionally disposable degrees of freedom may be discarded.

## Persistent promotion

Before Level A collapses, any member whose individual history must remain addressable must be promoted to Level I and removed from the exchangeable population settlement.

Promotion is not needed merely because an active individual experienced an event.

It is needed when the consequence cannot be represented sufficiently after aggregation or when product semantics require the exact individual to remain identifiable.

Examples:

- an anonymous grazer dies and immediately becomes aggregate carrion: Level A may settle mortality directly;
- an animal receives a durable scar that affects future behavior but the coarse state has no adequate representation: promote to Level I;
- a tracked animal crosses out of active range: promote/retain Level I;
- an anonymous animal moves between two cells and the occupancy/condition strata capture the result: Level A may collapse without persistent identity.

This avoids making every causal contact permanently expensive.

## Active mutation API

Level A should not expose unrestricted mutation of raw member vectors as the canonical world interface.

Prefer typed operations that make their effect on canonical observables explicit, for example:

```rust
pub enum ActiveRefinementCommand {
    Relocate { /* active handle + canonical destination */ },
    ChangeCondition { /* active handle + transition evidence */ },
    ConsumeResource { /* typed amount/source */ },
    ApplyMortality { /* active handle or aggregate selection */ },
    PromotePersistent { /* active handle + promotion reason */ },
}
```

The exact API may differ.

Commands must validate active-generation handles and produce settlement deltas suitable for atomic ecological accounting.

## Refinement generation

Repeated refine/collapse cycles create a continuity question.

If a population collapses completely and later refines again, exact anonymous individual identity need not be preserved unless policy says otherwise.

However the process must not introduce arbitrary nondeterminism.

The higher-level ecology authority should maintain a deterministic refinement-generation/seed policy based on canonical population/world state.

Possible policies include:

- stable population microstate seed plus monotonic refinement generation;
- explicit refinement events with canonical IDs;
- cached encounter continuity for recently observed cohorts;
- persistent promotion for individuals whose continuity matters.

The v0 core requirement is simply that the seed/generation is authoritative and reproducible, not renderer-derived.

## Refinement hysteresis

Simulation activation should avoid thrashing at a distance threshold.

A higher-level policy may use separate canonical enter/exit thresholds or minimum active duration.

Example:

```text
refine when interaction relevance <= 80 m
collapse only after relevance > 120 m
```

The numbers are product policy, not part of this contract.

The invariant is that hysteresis controls compute representation without changing biological truth by itself.

## Multiple observers

Two players/agents approaching the same exchangeable population must not independently refine overlapping copies of the same authority.

The world authority layer must coordinate reservation/refinement ownership so overlapping requests produce one consistent active authority or disjoint reservations.

Required invariant:

```text
one ecological quantity -> one active/coarse/persistent authority owner
```

regardless of observer count.

## Network/federation implication

Future multiplayer/federated simulation must not let two peers each treat the same reserved population subset as canonical active authority.

The eventual networking layer will need an authority lease/partition protocol or deterministic ownership rule around Level A.

This document does not choose that protocol, but the single-owner invariant is normative.

## Relationship to physics

Canonical local collision/contact need not use the full presentation mesh.

Level A may own deterministic coarse collision/body state while Level P renders higher-frequency soft tissue, fur, vegetation motion, or secondary dynamics.

If a physical deformation changes biology (limb damage, branch breakage, crushing), the authoritative trigger must come from Level A/canonical physics evidence, not presentation shader deformation.

## Qualification requirements

The refinement boundary is not qualified until evidence demonstrates:

1. Level P construction/destruction cannot alter canonical population/active/persistent state;
2. Level P cannot invoke Level A mutation through presentation-only APIs;
3. Level A can only be created from explicitly reserved authority;
4. Level A refinement conserves count, biomass, and every promised coarse observable;
5. identical authoritative refinement inputs produce identical active microstate;
6. frame rate, renderer configuration, camera culling, and GPU timing do not change active refinement results;
7. active handles from old refinement generations are rejected;
8. active mutation uses typed validated commands/settlements;
9. unchanged Level A collapse reconstructs coarse authority exactly;
10. changed Level A collapse preserves every target sufficient statistic and settlement balance;
11. Level I promotion removes the promoted member's exact contribution before coarse recombination;
12. overlapping observer requests cannot duplicate authority;
13. fine-vs-coarse observatory scenarios bound the error introduced by permitted active collapse;
14. save/reload while Level A is active either preserves that active authority exactly or performs an explicit qualified collapse before persistence.

## Evidence scenarios

### Renderer independence

Run the same authoritative movement/interaction trace with:

- no renderer;
- low visual fidelity;
- cinematic visual fidelity.

Level A state and resulting ecological settlements must match exactly.

### Refinement repeatability

Given the same coarse state, reservation plan, authoritative refinement generation, and simulation tick, produce the same Level A microstate on repeated runs.

### Active collapse

Refine a cohort, execute a typed migration/condition transition, collapse it, and verify exact target strata/count/biomass.

### Observer overlap

Issue two overlapping refinement requests in opposite orders. Authority ownership and final active/coarse state must be equivalent.

### Stale-handle rejection

Collapse generation `g`, refine generation `g+1`, and prove that handles from `g` cannot mutate the new active state.

## Design principle

**Rendering may reveal an organism. Active refinement may choose a causal microstate. Persistent identity preserves an individual biography. These are three different authority operations and must never be conflated.**
