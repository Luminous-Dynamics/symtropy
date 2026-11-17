# Living World Projection Source Authority v0

Status: companion projection/realization authority contract.

## Problem

A Level-P candidate is only as meaningful as the canonical information from which it was derived.

A marginal population may know exactly:

```text
age counts
condition counts
occupancy counts
```

while not knowing the joint association among those axes.

A sampled marginal projection can therefore show a tuple such as:

```text
juvenile + stable + cell A
```

that is fully consistent with each marginal independently.

But a richer stratified canonical population may prove that no occupied joint stratum with that tuple exists.

Therefore a projection candidate cannot be treated as source-schema independent.

## Normative rule

**Every projection scheme is bound to the canonical source authority schema/capability from which its candidate semantics were derived.**

A projection handle used at a future realization boundary must make that binding recoverable through either:

- an explicit source-authority schema/version field; or
- a projection-scheme version whose frozen semantics unambiguously include the source authority schema.

A generic projection version that silently changes from marginal-derived to stratum-derived semantics is forbidden.

## Marginal-source projection

For a canonical marginal source, the engine knows each marginal exactly but does not know the microscopic covariance.

A marginal-derived Level-P candidate is therefore best interpreted as:

> one deterministic admissible microstate proposal consistent with the currently retained marginal facts.

It is **not** evidence that the displayed age × condition × location association historically existed before refinement.

If a future Level-A realization is permitted while the same marginal authority is still current, the realization transaction may crystallize such a microstate only if it:

1. validates the exact source scope/revision/scheme;
2. transfers one unit consistently out of every affected marginal authority;
3. preserves all exact aggregate quantities;
4. records the new active joint association as canonical Level-A state;
5. does not claim that the pre-refinement world had previously measured that association.

This is an epistemic refinement, not historical discovery.

## Stratified-source projection

If canonical authority retains occupied joint strata, a targetable prospective candidate should normally be derived **from an occupied compatible stratum first**.

Conceptually:

```text
canonical occupied stratum
    -> choose/reserve-compatible stratum-local candidate proposal
    -> derive only still-unresolved microstate degrees of freedom
    -> Level-P candidate
```

This guarantees that every displayed canonical-axis tuple is compatible with a currently known joint class.

A strata-aware projection must not flatten to marginals and then independently resample axes merely for convenience when targetable realizability matters.

## Decorative projection may be weaker

A purely decorative, non-targetable visualization may be permitted to use a lower-information projection if product policy explicitly marks it non-realizable and no machine-recognized canonical action can target the synthetic entity.

The distinction must be explicit:

```text
DecorativeProjection
ProspectiveRealizableProjection
```

or equivalent typed/capability semantics.

A renderer-facing object that can be targeted by canonical gameplay must not rely on an impossible synthetic tuple without a valid realization path.

## Representation promotion invalidates incompatible projections

Suppose a population was projected while marginal authority was canonical, then before interaction the population is promoted to a richer stratified representation.

The old marginal-source candidate cannot simply be realized against the new strata.

The realization boundary must either:

- reject the candidate as stale/incompatible and request a new projection; or
- prove through the frozen compatibility contract that the shown tuple maps to a currently occupied compatible stratum without violating source authority.

Default behavior is fail-closed.

A representation/schema transition therefore participates in projection revision/generation invalidation.

## Realize what was shown, within what was known

The existing principle:

> realize what was shown

is refined to:

> realize what was shown only when the current canonical source still supports the source-schema assumptions under which it was shown.

If current authority contradicts the shown tuple, the engine must not silently substitute a different organism merely to preserve interaction continuity.

Prefer:

```text
reject stale candidate
-> reproject current truth
```

over:

```text
show A
-> secretly realize B
```

## Source schema and scheme versioning

For the current Living World work, the bounded sparse Fisher-Yates projection in PR #182 is a **marginal-source sampled Level-P scheme**.

Its projection scheme version should be understood as freezing all of:

- source authority model: canonical independent age/condition/occupancy marginals;
- deterministic marginal sampling grammar;
- tuple construction semantics;
- candidate-handle context semantics;
- non-authoritative/no-biomass boundary.

A future stratified projection is a different semantic scheme and must receive a distinct version/name even if it reuses the same bounded integer sampler internally.

## Multi-observer semantics

Two observers may project the same canonical source under different presentation seeds.

Those projections can propose different unresolved microstates without changing canonical truth.

Once one candidate crosses into Level A:

- source revision/generation must advance or ownership state must otherwise make the transfer observable;
- stale competing candidates must fail closed if they depend on authority no longer available;
- observers may reproject the updated source.

No observer-owned presentation seed can reserve authority ahead of realization.

## Interaction with sparse strata

Sparse strata solve precisely the class of ambiguity that independent marginals cannot retain.

If a process requires age × condition × cell covariance, then:

- marginal projection can remain useful for non-authoritative presentation where allowed;
- canonical process execution requires stratified authority or a qualified closure;
- targetable realization should consume compatible stratified authority rather than invent a contradictory tuple.

Future additional axes such as disease, genotype, development, territory, or social group follow the same rule.

## Qualification requirements

Evidence should establish at least:

1. a projection scheme unambiguously identifies its source authority semantics;
2. frozen marginal-source projection vectors remain stable within their scheme version;
3. a stratified-source targetable projection never emits a canonical-axis tuple absent from its occupied source strata;
4. two stratified populations with identical marginals but different covariance produce source-compatible targetable candidates rather than the same blindly resampled tuple set;
5. a marginal-source candidate becomes stale/fails closed after an incompatible source-schema promotion;
6. realization never silently substitutes a different current stratum for an unsupported shown tuple;
7. decorative non-realizable projections cannot enter canonical mutation APIs;
8. multi-observer projection cannot duplicate Level-A authority;
9. source-revision/schema mismatch leaves canonical state unchanged;
10. scheme/source changes require explicit version evolution rather than silent semantic drift.

## Design principle

**A projected organism is a proposal relative to what the world currently knows. Richer canonical knowledge must constrain what can be shown as realizable, not be flattened away to preserve a convenient presentation algorithm.**
