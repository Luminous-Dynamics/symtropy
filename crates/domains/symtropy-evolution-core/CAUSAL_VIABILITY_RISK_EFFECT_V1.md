# Causal Viability Risk Effect Contract v1

## Purpose

SEL-07C2 is the first layer allowed to emit a causal-selection effect value.

It is deliberately narrower than a generic causal estimator.

The governing chain is:

```text
validated design
    -> validated analysis frame
    -> validated descriptive association
    -> validated intervention identification
    -> causal viability-risk point effect
```

Every arrow is a distinct authority boundary.

The governing theorem remains:

```text
descriptive association
    + strong design label
    != causal effect
```

C2 requires a current SEL-07C1 identification capability; it cannot consume a raw B2 result alone.

## V1 target

V1 supports one causal target:

```text
do(Comparison) - do(Reference)
```

for viability-window death risk in the exact binary Reference/Comparison population already frozen by SEL-07A and materialized by SEL-07B1.

The target is `BinaryViabilityRiskDifference`.

There is no generic lifetime-fitness scalar and no population-genetic selection coefficient in this layer.

## Required current capabilities

`execute_causal_viability_risk_effect(...)` requires all three current non-Serde capabilities:

- `ValidatedSelectionAnalysisFrame<'_>`;
- `ValidatedConsequenceAssociation<'_>`;
- `ValidatedCausalSelectionIdentification<'_>`.

A raw frame digest, raw association digest, raw C1 digest, source design class, or p-value is insufficient.

The three capabilities must agree exactly on the bound frame/design lineage.

## Intervention-identification requirement

C2 accepts only `CausalIdentificationTier::InterventionIdentified`.

Therefore the source design must have passed C1's exact criterion and qualification-authority requirements for either:

- randomized intervention; or
- controlled simulation intervention.

C2 does not independently waive or reinterpret those criteria.

## Independent frame support reconstruction

C2 does not merely copy the B2 risk-difference field.

It independently scans the current validated frame and reconstructs:

- Reference group total/deaths/survivors;
- Comparison group total/deaths/survivors.

Every frame row must have:

- an observed binary predictor; and
- an observed viability-window outcome.

Missing/nonbinary predictors or nonviability/missing outcomes fail closed.

The independently reconstructed support table must exactly equal the support table in the current validated B2 descriptive association.

Thus:

```text
B2 point estimate field alone != C2 effect input
```

## Exact point effect

The C2 point effect is:

```text
risk(do(Comparison)) - risk(do(Reference))
```

under the current C1 intervention-identification authority.

V1 represents it as a reduced `ExactSignedFraction`.

The implementation uses checked exact arithmetic and no floating point.

The stored result preserves the Reference and Comparison support counts used to compute the point effect.

## Same number, different causal evidence

A causal point effect is not identified only by its numeric value.

C2 identity binds:

- exact C1 identification digest;
- exact B1 frame digest;
- exact B2 association digest;
- source design class;
- causal target;
- original denominator;
- estimand denominator;
- exact support table;
- exact causal point effect;
- exact versioned C2 method authority.

Therefore two results may both equal `+1` numerically while remaining different causal evidence objects because their identification/qualification evidence differs.

## Local result self-consistency

A serialized C2 result cannot carry an arbitrary causal point effect.

Local validation requires:

- valid version;
- supported intervention design class;
- supported causal target;
- exact built-in method authority;
- valid denominator relation;
- internally valid Reference and Comparison support counts;
- nonempty groups;
- support totals exactly equal the estimand denominator;
- stored exact point effect equals the risk difference recomputed from the stored support counts.

A modified numerator/denominator or inconsistent support table cannot obtain a canonical C2 digest.

## Current replay

`CausalViabilityRiskEffectDigest` is representation identity only.

`ValidatedCausalSelectionEffect<'_>` is deliberately non-Serde.

To regain current authority it reruns C2 from:

- current validated frame;
- current validated B2 association;
- current validated C1 identification.

The recomputed object must exactly equal the persisted result.

Thus:

```text
locally self-consistent causal result
    != current causal-effect authority
```

## No descriptive-probability promotion

B2's fixed-margin hypergeometric/Fisher-style reference distribution is descriptive evidence.

C2 V1 does **not** copy into the causal result:

- B2 p-value;
- hypergeometric support;
- observed reference weight;
- total reference weight;
- any field named causal significance.

This is deliberate.

A valid causal uncertainty/randomization distribution requires evidence that the executed reference mechanism matches the actual qualified assignment mechanism and causal estimand.

C1 V1 preserves assignment-mechanism evidence, but C2 V1 does not yet define and qualify that mapping theorem.

Therefore C2 V1 provides an identified causal **point effect only**.

A future uncertainty tranche must explicitly prove the assignment/reference relationship instead of relabeling the descriptive B2 probability.

## Denominator preservation

The C2 result retains both:

- original denominator; and
- estimand denominator.

The exact support table must sum to the estimand denominator.

The C2 executor cannot silently construct a new complete-case subset.

## Group-label direction

Effect direction is fixed:

```text
Comparison - Reference
```

Reversing intervention labels reverses the signed point effect and changes frame, B2, C1 and C2 evidence identity.

The public fixture demonstrates exact `+1` versus `-1` under label reversal.

## Method authority

V1 has one built-in method authority:

`intervention-identified-binary-viability-risk-difference-v1`

Its exact method digest binds the rule that:

- C1 must be current and `InterventionIdentified`;
- the frame is exact binary/viability evidence;
- frame support must equal B2 support;
- the point effect is exact `do(Comparison)-do(Reference)` risk difference;
- no causal p-value or uncertainty interval is emitted.

There is no runtime causal-estimator selector in V1.

## B2 remains valuable but descriptive

C2 binds the exact B2 association digest because the descriptive result is part of the evidence lineage.

However, causal interpretation comes from the independent C1 capability, not from B2's numerical extremeness.

The same B2 support table combined with different valid C1 qualification evidence yields different C2 identity even when the point effect number is unchanged.

## Public adversarial corpus

The initial C2 corpus requires:

- hand-checkable randomized fixture yields exact `+1` causal risk difference;
- result binds exact frame, B2 and C1 digests;
- Serde-restored result requires fresh B1/B2/C1 replay;
- intervention-label reversal yields exact `-1` and changes result identity;
- identical numeric point effect under different C1 evidence/qualification identities produces different causal evidence identity;
- altered serialized causal point effect fails local canonical validation;
- C1 authority from another frame cannot authorize the effect;
- C2 wire shape contains no B2 probability/reference distribution, fitness, selection coefficient, beneficial/deleterious classification, adaptation, or speciation field.

## Wire-shape boundary

C2 V1 contains a causal point effect but contains no field for:

- p-value;
- causal significance decision;
- hypergeometric/Fisher reference distribution;
- generic fitness;
- relative fitness in general;
- population-genetic selection coefficient;
- beneficial/deleterious mutation status;
- adaptation;
- reproductive isolation;
- speciation.

## Explicit non-claims

C2 establishes only the intervention-identified causal viability-risk point effect for the exact bound intervention/context/window/population lineage.

It does not by itself establish:

- a valid causal uncertainty interval;
- causal statistical significance;
- lifetime fitness;
- a population-genetic selection coefficient;
- mutation-level beneficial/deleterious status;
- adaptation;
- reproductive isolation;
- speciation.

It also does not upgrade the external scientific validity of the C1 criterion/qualification authorities beyond C1's explicit trust boundary.

## Successor boundary

The next scientifically clean layers are separate:

1. **causal uncertainty/reference qualification** — prove that an assignment-derived or other causal uncertainty procedure is valid for the exact C1 mechanism and C2 target;
2. **model-specific selection translation** — convert qualified endpoint-specific causal effects into a selection coefficient only under an explicit population-genetic model;
3. **multi-generation heritable response** — require repeated heritable population response before adaptation claims;
4. **reproductive-isolation/speciation evidence** — remain later still.

No successor may infer adaptation directly from a single-window C2 causal effect.

## Evidence status

Source implementation, contract text, static review, and test design are not executable qualification.

A CI-only helper must freeze and execute the exact final C2 product head before any executable PASS is claimed.

Queued/unassigned CI is unexecuted evidence and must not be interpreted as success or product-code failure.
