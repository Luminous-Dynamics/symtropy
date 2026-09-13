# SEL-08A Model-Specific Viability Selection V1

## Governing theorem

`causal viability effect != population-genetic selection quantity != adaptation`

SEL-08A introduces the first population-genetic interpretation layer above an endpoint-specific causal viability effect. It does not reinterpret every causal contrast as fitness or as a universal selection coefficient.

V1 is deliberately narrow: a binary genotype-or-lineage comparison, one viability consequence window, Reference-relative survival, and one exact relative-viability quantity.

## Required current authorities

The executable translation consumes:

- current `ValidatedSelectionAnalysisFrame<'_>`;
- current `ValidatedCausalSelectionEffect<'_>` from SEL-07C2;
- current `ValidatedSelectionTranslationModel<'_>`.

A B2 descriptive association, C1 identification alone, D2 p-value, raw risk difference, or serialized digest is insufficient.

D2 uncertainty is not required to define the V1 point estimand. A small p-value is not a prerequisite for defining a model-specific selection quantity, and a small p-value cannot select the biological model.

## Heritable predictor gate

V1 requires the exact B1 design predictor source to be:

`PredictorSourceKind::GenotypeOrLineage`.

The predictor representation must be binary `Reference` / `Comparison`, and the selected estimand must be the viability-window risk contrast.

A drug/placebo, policy, exposure, phenotype-only, or arbitrary external grouping does not become a population-genetic selection class merely because C2 identified a causal viability effect.

`causal intervention contrast != heritable selection contrast`.

The B1 rows retain exact hereditary-manifest provenance for genotype/lineage predictor materialization.

## Model authority is separate from the observed effect

`ViabilitySelectionTranslationModel` is declared from the current frame and external model authorities, not from the observed C2 effect magnitude.

It binds:

- semantic model ID;
- revision;
- built-in V1 model-spec digest;
- exact population identity;
- exact evolutionary context digest;
- exact predictor-definition binding digest;
- exact generation/life-cycle mapping authority;
- exact heritable-class mapping authority;
- exact selection-model qualification authority;
- Reference-survival baseline convention;
- viability consequence-window stage;
- other-fitness-component policy;
- frequency-dependence policy;
- density-dependence policy;
- migration-contribution policy.

This separation is intentional:

`model preregistration != observed effect result`.

The same model may be applied to different current causal-effect realizations only when the current population/context/predictor authority remains compatible. The resulting evidence objects remain distinct because the estimate binds the exact C2 and frame digests.

## External mapping and qualification authorities

### Generation mapping

`GenerationMappingRef` binds the claim that the exact viability consequence window corresponds to the modeled population-genetic life-cycle/generation stage for the bound population and context.

A digest is an auditable authority identity, not independent proof that the mapping is scientifically correct.

### Heritable class mapping

`HeritableClassMappingRef` binds the rule that maps the exact genotype/lineage predictor definition into the Reference and Comparison heritable classes under the bound context.

The authority is bound to the exact predictor-definition digest and context. It does not let a human-readable class label replace B1 hereditary-manifest provenance.

### Model qualification

`SelectionTranslationQualificationRef` separately qualifies the V1 biological assumptions for the exact population, predictor and context.

V1's qualified assumptions are intentionally restrictive:

- other fitness components are held equal or excluded for this translation;
- frequency dependence is excluded within the validity domain;
- density dependence is excluded within the validity domain;
- migration contribution is excluded within the validity domain.

Changing only this qualification authority changes the model and result evidence identities even if the numerical quantity is unchanged.

`same number != same qualified biological claim`.

## Exact transform

C2 reports group-specific viability-window death support. V1 first converts death risk into survival probability:

`survival = survived_window / total`.

It then defines exact Reference-relative viability:

`w_rel = survival_comparison / survival_reference`.

Finally V1 reports the explicitly named model quantity:

`q = w_rel - 1`.

The result type calls this:

`RelativeViabilityRatioMinusOne`.

It is intentionally not serialized as a naked generic `s` or a generic `fitness` scalar.

All finite arithmetic is reduced exact rational arithmetic; no floating point or continuity correction is used.

## Sign convention

V1 freezes:

`PositiveMeansHigherComparisonRelativeViability`.

Therefore:

- `q > 0` means Comparison has higher viability than Reference under the exact model;
- `q = 0` means equal relative viability under the exact model;
- `q < 0` means Comparison has lower viability than Reference under the exact model.

This direction is local to the exact Reference/Comparison class mapping, context, viability window and model assumptions. It is not a universal beneficial/deleterious classification.

## Zero-baseline boundaries

If Reference survival is zero and Comparison survival is positive, V1 returns typed:

- `RelativeViabilityRatio::PositiveInfinity`;
- `RelativeViabilitySelectionQuantity::PositiveInfinity`.

If both Reference and Comparison survival are zero, V1 returns typed:

- `UndefinedBothSurvivalsZero`.

No pseudocount, continuity correction or epsilon is inserted.

`undefined/infinite biological boundary != arbitrary finite estimate`.

## Persisted model versus current model authority

`ViabilitySelectionTranslationModel` is serializable and deterministically hashed.

A serialized model does not regain authority from its own bytes. `ValidatedSelectionTranslationModel<'_>` is non-Serde and requires current replay against:

- current analysis frame;
- fresh generation mapping;
- fresh heritable-class mapping;
- fresh model qualification.

The reconstructed model must exactly equal the persisted representation.

## Persisted estimate versus current selection authority

`ModelSpecificSelectionEstimate` binds:

- exact C2 causal-effect digest;
- exact selection-model digest;
- exact B1 frame digest;
- exact population/context;
- exact predictor-definition digest;
- exact Reference/Comparison viability support;
- exact survival fractions;
- exact relative-viability ratio;
- exact model-specific quantity;
- explicit sign convention.

Local validation recomputes survival, relative viability and the final quantity from the stored support counts. Internally inconsistent serialized arithmetic cannot obtain a canonical digest.

`ValidatedModelSpecificSelectionEstimate<'_>` is non-Serde and requires exact replay against the current frame, C2 effect and validated translation model.

`locally consistent selection bytes != current model-specific selection authority`.

## Relationship to causal randomization uncertainty

SEL-07D2 may establish an exact sharp-null causal randomization probability for a compatible randomized-intervention lineage.

SEL-08A does not copy that probability, does not require it to be below a threshold, and does not convert statistical significance into biological significance.

Future uncertainty propagation for the model-specific quantity must bind an explicit causal uncertainty theorem and translation rule. V1 is a point translation only.

## Adversarial corpus

`model_specific_selection_v1` establishes at least:

1. the hereditary forward fixture translates Reference survival `1`, Comparison survival `0` into relative viability `0` and exact model quantity `-1`;
2. reversed group labels produce the typed positive-infinity zero-baseline boundary rather than a continuity-corrected finite value;
3. model/result representations survive Serde only as representations and require current replay;
4. changing only model qualification preserves the numerical quantity but changes model and result evidence identity;
5. stale fresh qualification cannot reauthorize a persisted model;
6. tampering a serialized survival/selection quantity fails local arithmetic validation;
7. the result wire shape contains no generic fitness, generic selection coefficient, beneficial/deleterious, adaptation, reproductive-isolation or speciation field.

The source module also unit-tests the both-zero-survival undefined boundary.

## Explicit non-claims

SEL-08A does not establish:

- lifetime fitness;
- total reproductive fitness;
- a universal or context-free selection coefficient;
- beneficial/deleterious mutation status;
- heritability beyond the bound mapping authority;
- realized multi-generation response;
- adaptation;
- ecological dominance;
- reproductive isolation;
- speciation.

The model applies only within its exact population, context, class mapping, viability window and qualified assumptions.

## Successor

SEL-08B / #861 must independently establish complete multi-generation heritable response and replication before any adaptation status can be supported.

A favorable SEL-08A quantity—even a causal, statistically well-supported one—must not be promoted directly to adaptation.
