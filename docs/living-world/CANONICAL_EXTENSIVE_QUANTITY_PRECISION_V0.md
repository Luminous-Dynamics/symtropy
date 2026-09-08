# Living World Canonical Extensive Quantity Precision v0

Status: companion accounting contract. This document governs cross-model authority for conserved/extensive quantities where exact population fixed-point state meets floating-point field/reference models.

## Problem

Current Living World foundations intentionally use different numeric representations for different jobs:

- population living biomass is exact fixed-point integer milligrams;
- environmental/physiology fields use floating point where continuous approximation is appropriate;
- the current `EcologicalLedger` uses `f64` pools and tolerance-based balance checks.

Those choices are individually reasonable, but a cross-model settlement cannot allow two different numeric representations to become independently authoritative for the same conserved quantity.

A death event must not remove `123_457 mg` from canonical living population state and add a rounded floating amount to detritus, then repeat that conversion until conservation drifts.

## Core rule

Canonical conserved/extensive authority uses an exact discrete representation at commit boundaries.

Floating-point values may model, estimate, diffuse, optimize, or report conserved quantities, but crossing into canonical accounting requires an explicit quantization/reconciliation rule.

Conceptually:

```text
continuous/approximate model
        |
        | explicit quantization + residual handling
        v
exact canonical extensive account
        |
        | exact settlement
        v
exact canonical destination account
```

## Numeric authority classes

### Exact extensive state

Examples:

- living biomass owned by a coarse population/stratum;
- detrital biomass after mortality;
- canonical stored water/carbon/mineral mass when that mass participates in exact settlement;
- explicit external input/output counters;
- harvested/transferred material.

Exact extensive values should use integer/fixed-point units with checked arithmetic.

### Approximate intensive/field state

Examples:

- local moisture fraction;
- toxin concentration;
- temperature;
- normalized physiological stress;
- continuous diffusion fields;
- renderer material parameters.

These may remain `f32`/`f64` subject to finite-value validation and process-specific numerical tolerances.

### Measurement/reference accounting

A floating ledger can remain useful for diagnostics, experiments, and comparison while it is not the sole mutation authority for exact population biomass.

If a floating ledger mirrors an exact canonical account, disagreement must be observable and the exact account wins authority disputes.

## Preferred canonical amount shape

The exact type is an implementation decision, but a low-level form may resemble:

```rust
pub struct ExactAmount(pub u128);
```

with the base unit determined by `ConservedQuantity` or by a typed amount wrapper.

For mass quantities, milligrams provide immediate compatibility with current population biomass. `u128` provides enormous headroom for regional/global aggregation while preserving checked exact arithmetic.

A later design may choose finer units or separate scales per quantity, but scale must be explicit and stable in persistence/evidence.

## No implicit float-to-exact cast

Canonical settlement APIs must not accept arbitrary `f64` and silently round.

Conversion requires an explicit policy, for example:

```text
quantized = floor/nearest/exact-rational conversion
residual  = continuous_value - represented_value
```

The chosen rule is domain-specific and must state where residual quantity goes.

Valid residual strategies include:

- retain residual in an authoritative accumulator until it reaches one exact unit;
- keep it in the continuous source field until later transfer;
- use a higher-resolution exact unit;
- classify it as bounded numerical error only for a non-authoritative measurement model.

Dropping residual silently is not canonical conservation.

## Exact internal transfer

For an exact transfer:

```text
source_after      = source_before - amount
destination_after = destination_before + amount
```

all preconditions and overflow/underflow checks occur before commit.

The amount is bit-for-bit identical on both sides of one-quantity transfer.

## Cross-quantity reactions

Biological chemistry often changes one tracked quantity into another (for example nutrient uptake contributing to biomass).

This is **not** represented as pretending unlike quantities are interchangeable.

A reaction must define explicit stoichiometric/efficiency semantics and account for all modeled products/byproducts/external fluxes.

Where coefficients are fractional, exact implementation may use:

- rational integer ratios;
- fixed-point coefficients plus residual accumulators;
- a qualified reaction integrator that periodically reconciles exact totals.

The reaction boundary, not a generic ledger transfer, owns that conversion policy.

## Field-to-account coupling

A diffusion field can model continuous density while exact accounts model conserved totals.

A safe coupling pattern is:

```text
field process proposes flux
        -> integrate proposed extensive amount
        -> quantize with explicit residual
        -> exact settlement
        -> update/reference field consistently
```

The exact account must not be reconstructed by repeatedly summing floating field cells and treating tiny numerical differences as creation/destruction of canonical matter.

## Hierarchical ownership

Exact conserved authority should support hierarchical reconciliation such as:

```text
Living total
  = sum(exchangeable population strata)
  + sum(active refinements)
  + sum(persistent organisms)
```

and:

```text
Biomass total
  = Living + Detritus + ... + boundary adjustments
```

A cached aggregate may use another representation for speed/telemetry, but canonical children and parent reconciliation must have one explicit authority direction.

## Current `EcologicalLedger` implication

The current floating `EcologicalLedger` should be treated as a useful v0/reference conservation substrate until exact cross-model population settlement is introduced.

This contract does **not** require rewriting that qualified work immediately.

It does require that future mortality/growth/reproduction code avoid declaring both:

```text
PopulationState::biomass_milligrams
```

and an independently mutable floating `Living` biomass pool as simultaneous canonical truth.

Before that integration becomes normative, either:

1. an exact canonical ledger/account layer is added; or
2. one representation is explicitly derived from the other with qualified quantization/reconciliation.

## Deterministic evidence

Exact extensive settlement should be replay-comparable without floating tolerance.

For exact accounts, qualification should prefer equality:

```text
before + inputs - outputs == after
```

Tolerance remains appropriate for approximate numerical fields and diagnostic mirrors, not for integer authority that is capable of exact accounting.

## Qualification requirements

Evidence must establish at least:

1. canonical exact amounts reject overflow/underflow atomically;
2. one-quantity internal transfer preserves exact raw amount;
3. floating-to-exact conversion requires an explicit policy;
4. conversion residual is retained/accounted rather than silently dropped;
5. population/stratum biomass reconciles exactly with the canonical Living ownership hierarchy;
6. mortality transfers the identical exact biomass amount from Living ownership to detrital/carrion ownership;
7. failed settlement leaves both biological and accounting state unchanged;
8. repeated round trips do not accumulate hidden quantization loss;
9. persistence records unit/scale unambiguously;
10. floating reference/field balance may use tolerance but cannot override exact canonical account state.

## Design principle

**Use floating point where biology is genuinely continuous and approximate. Use exact integers where the world makes a discrete claim that conserved matter moved from one authority owner to another. Never hide the conversion between those two regimes.**
