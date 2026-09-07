# Living World Conditional Microstate Refinement v0

Status: companion authority contract.

## Purpose

A sparse canonical stratum can preserve important joint ecological facts without containing a complete microscopic organism state.

For example, a stratum may canonically establish:

```text
age_band = Juvenile
condition_band = Stressed
cell = A
count = 17
exact living biomass = ...
```

while leaving unresolved:

- exact continuous age within the juvenile band;
- subcell position;
- orientation;
- continuous hydration/energy state;
- modeled body size;
- phenotype details;
- social offset within a herd/cluster;
- locomotor phase;
- many species-specific latent variables.

When one member becomes Level A, Symtropy needs a principled way to refine only what the coarse source did not already know.

This contract defines that boundary.

## Core invariant

> Level-A realization preserves every canonical fact established by the source authority and derives only unresolved degrees of freedom from an explicitly qualified conditional model.

Refinement is not permission to overwrite known coarse truth with a prettier random individual.

## Three provenance classes

Every future-bearing Level-A microstate field should be attributable conceptually to one of three provenance classes:

### K — canonical known

The source authority already establishes the fact exactly or at its declared canonical resolution.

Examples:

- occupied stratum age band;
- condition band;
- occupancy cell;
- exact authority-share ownership;
- persistent measured wound carried from Level I.

A refinement sampler cannot change K-state.

### D — derived/refined

The source authority leaves the variable unresolved, and a versioned qualified model derives one deterministic microstate value compatible with the K-state.

Examples:

- continuous age within an age band;
- subcell point within a qualified spatial patch model;
- modeled body-size percentile within a stratum;
- phenotype trait from species/genome-like seed information when no measured value exists.

Once D-state is allowed to influence canonical outcomes, it becomes stable Level-A state for that refinement. It is not rerolled merely because the renderer despawns or another system asks for it later.

### M — measured/assimilated

A later authoritative observation/evidence path establishes a stronger individual fact.

Examples:

- measured body mass;
- diagnosed disease stage;
- persistent injury location;
- observed genotype marker.

An M-state update may supersede a D-state estimate only through an explicit assimilation/state-transition rule. It must not silently rewrite history.

## Conditional, not independent, derivation

Unresolved variables must be derived **conditioned on the canonical source facts that constrain them**.

Illegal pattern:

```text
sample age independently
sample condition independently
sample cell independently
```

when canonical strata already establish their joint association.

Preferred pattern:

```text
source stratum K-state
    -> qualified conditional refinement model
    -> unresolved D-state only
```

A strata-aware realization therefore starts from an occupied canonical joint stratum and does not flatten it back into independent marginals merely to reuse a simpler sampler.

## Keyed deterministic derivation

Sequential RNG is a poor authority primitive because adding a new sampled variable can shift every later draw.

Derived Level-A variables should preferentially use keyed deterministic derivation bound to stable authority context, conceptually:

```text
refined_value = derive(
    population_scope,
    stratum_key,
    partition_epoch,
    authority_slot_ordinal,
    refinement_scheme_version,
    variable_key,
)
```

Examples of variable keys:

```text
"age.continuous"
"position.subcell.x"
"position.subcell.y"
"body.size_percentile"
"phenotype.branching_bias"
"behavior.baseline_boldness"
```

Adding a new unrelated variable key must not perturb previously frozen derived values.

## Refinement scheme versioning

The conditional model is part of canonical semantics once D-state can affect future history.

Therefore a refinement scheme/version must freeze at least:

- derivation/hash family;
- source authority schema/capabilities assumed;
- variable-key namespace;
- conditional distributions/response curves;
- spatial model version where relevant;
- species/phenotype model version where relevant.

Changing canonical D-state semantics requires an explicit version transition.

## Representation-bound validity

A D-state value is valid under the source facts/model assumptions that generated it.

If the canonical source changes before realization, the prospective derived candidate may become stale.

After successful Level-A realization, however, its D-state has become part of the active canonical microstate and is not automatically regenerated because the coarse source later rebases.

This parallels exact authority-share ownership:

```text
before realization: proposal can stale

after realization: committed active state persists until explicit canonical transition
```

## Spatial refinement

Subcell placement must respect the strongest canonical spatial information available.

If source authority contains only occupancy S0, a presentation-only point may use a qualified coarse placement rule.

If source authority establishes clusters/territories/stands/contact structure, Level-A subcell position must condition on those structures.

If a requested canonical process needs exact S4 contact geometry and no qualified refinement exists, realization at that process fidelity must fail or promote the representation rather than invent unsupported geometry.

## Continuous age/development

A categorical age/development band does not imply exact continuous age.

A Level-A process may derive a continuous phase inside the band only when a qualified conditional distribution/model exists.

The resulting D-state should then age forward deterministically while active. It must not be resampled from the band on every reload/tick.

On collapse, if exact continuous age no longer needs to persist, the process-information contract determines whether it can be reduced back to a band or whether sufficient phase/history information must survive.

## Body size and mass

A modeled body-size or body-mass value may be D-state.

It is distinct from the exact extensive authority share defined by `ACTIVE_EXTENSIVE_SHARE_V0.md`.

Thus a Level-A organism can legitimately have:

```text
exact_biomass_authority = K/accounting state
modeled_body_mass = D/biophysical estimate
```

provided every process knows which one it is authorized to use.

A later measured individual mass becomes M-state and may require explicit reconciliation with exact conservation authority rather than silent replacement.

## Behavior/personality latents

If an unresolved behavioral latent such as boldness or territoriality affects canonical decisions, then after derivation it is Level-A D-state and must remain stable for the active refinement.

It cannot be presentation randomness.

If such a latent materially affects long-term future after collapse, the coarse representation must preserve its required distribution/correlation, promote the organism to Level I, or use a qualified closure.

## Flora morphology

The same rule applies to plants.

A stratum may know species/stage/condition/location while unresolved structural phenotype is derived conditionally.

Once a particular branch architecture, wound, crown asymmetry, root response, or growth decision affects canonical interactions, it becomes active/persistent biological state rather than a shader-only variation.

Plant realization should therefore distinguish:

- canonical environmental/biographical constraints;
- deterministic derived structural phenotype;
- presentation-only microdetail.

## Presentation-only microdetail remains Level P

Not every visible variable needs to become canonical.

Examples such as sub-pixel leaf shimmer, fur noise, tiny bark roughness, and high-frequency cloth/feather motion may remain Level P if they cannot influence canonical outcomes.

The boundary is causal:

> if changing the value can change canonical future state, it cannot remain disposable presentation randomness.

## Assimilation of stronger evidence

When new evidence establishes a stronger fact than D-state, the transition should be explicit:

```text
old derived estimate
+ authoritative observation/evidence
    -> validated assimilation
    -> new measured/assimilated state
```

The system should record enough provenance to explain the change and should not rewrite prior event history as though the new measurement had always been known.

## Collapse and information debt

When Level A collapses back to coarse state, every D/M field falls into one of four categories:

1. exactly represented by the target coarse state;
2. summarized into sufficient statistics/covariance;
3. retained as a persistent imprint/Level-I fact;
4. discarded only because every enabled future process is qualified to be insensitive to it.

If none apply, collapse is illegal.

This is the **information-debt test** for active refinement.

## Counterfactual stability

A useful qualification property is that unrelated schema/model additions should not perturb already-defined derived variables.

For example, adding:

```text
"fur.microcurl"
```

must not change an existing organism's derived:

```text
"age.continuous"
"position.subcell.x"
"behavior.baseline_boldness"
```

under the same frozen refinement scheme.

This is why keyed derivation is preferred over sequential draws.

## Fail-closed cases

Realization/refinement must fail before canonical mutation when:

- source K-state is incompatible with the candidate;
- required conditional model/version is unavailable;
- required spatial information is below the process requirement;
- exact extensive resolution is insufficient for requested causal granularity;
- a variable needed by the process has no qualified K/D/M provenance path;
- keyed derivation produces invalid/out-of-domain output;
- model/source version mismatch cannot be reconciled;
- collapse would discard future-relevant D/M information without a qualified sufficient summary.

## Qualification requirements

The Living World Observatory should establish at least:

1. K-state source axes are never changed by refinement;
2. same authority slot + scheme + variable key yields identical D-state;
3. unrelated new variable keys do not perturb existing D-state values;
4. different authority slots produce bounded/diverse conditional values where the model expects diversity;
5. derived values remain inside source/biology-conditioned domains;
6. strata-aware refinement never produces a joint tuple absent from the selected canonical stratum;
7. active D-state remains stable across rendering despawn/recreation and unrelated coarse source revision changes;
8. stronger M-state assimilation is explicit and deterministic;
9. presentation-only values cannot enter canonical APIs without promotion/refinement;
10. collapse either preserves/summarizes every future-relevant D/M field or fails closed;
11. continuous development state advances rather than rerolls while active;
12. source/model/refinement-version mismatch fails before mutation;
13. replay under the same exact source/authority inputs reconstructs identical Level-A microstate.

## Design principle

**Refinement should reveal one deterministic possibility inside what the world already knows—not overwrite known truth, reroll history, or pretend an unresolved estimate was measured fact.**
