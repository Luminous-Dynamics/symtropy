# Consequence Association Contract v1

## Purpose

SEL-07B2 is the first statistical execution layer in the selection-evidence chain.

It consumes an already validated SEL-07B1 analysis frame and emits **descriptive association evidence only**.

The governing distinction remains:

```text
preregistered design
    != executable analysis frame
    != descriptive association
    != causal selection effect
    != model-specific selection coefficient
    != adaptation
```

No result in this layer may be treated as a causal-selection capability merely because its source design was interventional, controlled, matched, or otherwise stronger than an ordinary observational design.

## Exact current-authority prerequisite

Execution accepts only `ValidatedSelectionAnalysisFrame<'_>`.

The estimator does not reopen:

- raw consequence observations;
- raw phenotype/exposure evidence;
- population/census reconstruction;
- predictor materialization;
- calibration translation;
- denominator/exclusion selection.

Those decisions are already frozen and replay-qualified in SEL-07A/SEL-07B1.

Changing any of them changes the validated frame digest and therefore changes association evidence identity.

## One built-in method in V1

V1 intentionally supports one statistical lane only:

- binary Reference/Comparison predictor;
- `ViabilityWindowRiskContrast` outcome;
- comparison-minus-reference risk difference;
- comparison/reference risk-ratio status;
- fixed-margin hypergeometric reference support;
- two-sided Fisher-style probability ordering;
- exact integer/rational arithmetic;
- no stochastic resampling;
- no multiplicity adjustment;
- maximum 64 estimand rows.

There is no estimator parameter at execution time.

The frozen SEL-07A `UncertaintyPlan.method` must exactly equal `binary_viability_exact_reference_method_v1()` and its `minimum_information` must exactly equal `binary_viability_exact_reference_minimum_information_v1()`.

Method-ID equality alone is insufficient: the authority includes exact revision and content digest generated from the versioned built-in specification.

This prevents a caller from inspecting outcomes and then selecting another estimator while retaining the same design identity.

## Exact support table

Every estimand row must contribute exactly once to one of four cells:

```text
                    survived       died
Reference               a            b
Comparison              c            d
```

The result retains the two group totals and their survived/death counts explicitly.

The comparison direction is fixed:

```text
Comparison - Reference
```

Reversing predictor group labels therefore reverses the signed risk-difference direction and changes frame/result identity.

## Exact risk difference

V1 reports:

```text
risk(Comparison) - risk(Reference)
```

as a reduced signed rational number.

No floating-point approximation is required to represent the point estimate.

## Exact risk-ratio status

V1 reports:

```text
risk(Comparison) / risk(Reference)
```

as one of:

- a reduced finite nonnegative rational;
- positive infinity when Reference risk is zero and Comparison risk is nonzero;
- `BothRisksZero` when both death risks are zero.

The result does not invent a continuity correction merely to make an infinite or zero/zero ratio finite.

## Fixed-margin hypergeometric reference

For the observed 2x2 table, V1 conditions on:

- total estimand size;
- total deaths;
- Comparison group size.

It enumerates every feasible Comparison-death count and stores the exact integer hypergeometric weight:

```text
C(total_deaths, comparison_deaths)
×
C(total_survivors, comparison_survivors)
```

The complete support is retained in the result rather than only a final scalar probability.

The total weight is:

```text
C(total_rows, comparison_group_size)
```

and the implementation requires the enumerated support weights to sum exactly to that quantity.

## Two-sided probability ordering

The V1 two-sided reference probability follows Fisher-style probability ordering:

- determine the observed table's exact hypergeometric weight;
- include every feasible table whose weight is less than or equal to the observed weight;
- divide the summed included weight by the total fixed-margin weight;
- reduce the exact rational.

The field name is deliberately `two_sided_probability_ordered_p` rather than a generic proof-of-effect flag.

## Statistical-reference boundary

The hypergeometric/Fisher calculation is a **descriptive statistical reference distribution**.

Its existence does not establish that group assignment was randomized or exchangeable.

It does not independently prove:

- random assignment;
- valid treatment assignment;
- exchangeability;
- absence of confounding;
- absence of interference;
- positivity;
- consistency;
- a valid causal null;
- causal selection.

Those requirements belong to later causal-identification evidence.

In particular:

```text
small descriptive reference probability
    != causal selection evidence
```

## Exact arithmetic and bounded domain

V1 supports at most 64 estimand rows.

This is a scientific/computational contract, not an arbitrary UI limit.

For this domain:

```text
max C(n, floor(n/2)) = C(64, 32)
```

fits in `u64`.

Intermediate binomial construction uses checked `u128` arithmetic and converts to `u64` only after exact computation.

All denominator additions/multiplications and reference-weight sums fail closed on arithmetic overflow or invariant violation.

No floating-point tolerance is needed to decide whether two executions agree.

## Local statistical self-consistency

A serialized `ConsequenceAssociationEstimate` may not contain an arbitrary point estimate merely because it has a valid frame digest.

Local validation recomputes from the stored Reference/Comparison support table:

- exact risk difference;
- exact risk ratio/status;
- complete hypergeometric support;
- observed hypergeometric weight;
- total hypergeometric weight;
- exact two-sided probability-ordered fraction.

All recomputed values must exactly equal the stored values.

Therefore a payload with a modified numerator, ratio, support weight, support point, or p-value is locally invalid and cannot obtain a canonical result digest.

This local self-consistency is still weaker than current scientific authority.

## Representation identity vs current authority

`ConsequenceAssociationEstimateDigest` is deterministic identity for a locally valid statistical result representation.

It does not prove that the source analysis frame is still current.

`ValidatedConsequenceAssociation<'_>` is deliberately non-Serde.

To regain current authority, it reruns the entire V1 association method against the current `ValidatedSelectionAnalysisFrame<'_>` and requires exact equality with the persisted result.

Thus:

```text
locally self-consistent result
    != current descriptive evidence authority
```

and:

```text
current descriptive evidence authority
    != causal selection authority
```

## Missing predictor/outcome behavior

V1 never converts missing predictor or missing viability outcome to zero.

A missing required predictor or viability outcome triggers the frozen insufficient-support behavior.

Supported V1 policies are:

- `FailClosed`: return an execution error carrying the typed insufficiency reason;
- `ReportInsufficientSupport`: emit a result whose status is `InsufficientSupport(...)` and contains no point estimate.

`DescriptiveOnlyFallback` is explicitly rejected in V1.

There is no second fallback estimator in this tranche, so treating that policy as an alias for another behavior would create an undeclared post-hoc analysis path.

## Unsupported configuration fails closed

V1 rejects:

- any non-viability estimand;
- any non-binary predictor representation;
- predictor rows that are not observed binary values when estimation is attempted;
- non-viability outcome variants;
- method/minimum-information authority drift;
- replicate-count configuration;
- seed-lineage configuration;
- multiplicity configuration;
- `DescriptiveOnlyFallback`;
- estimand populations above 64 rows.

Unsupported cases are not silently coerced into the supported lane.

## Source design class is evidence identity, not causal certification

The result retains `source_design_class` because the same numerical table under different frozen designs is not the same evidence object.

However, B2 does not expose a `causal` boolean or a conversion from a stronger design class into causal-selection truth.

A `RandomizedInterventional` source design may later help satisfy SEL-07C requirements, but this B2 object remains a descriptive association result.

## Denominator visibility

The result retains both:

- original denominator from SEL-07A; and
- estimand denominator from SEL-07A/SEL-07B1.

The point estimate uses only the exact estimand rows already frozen into the analysis frame.

The original population cannot disappear from result evidence merely because exclusions were predeclared.

## Hand-checkable reference fixture

The public V1 corpus contains a simple four-individual case:

```text
Reference:   0 deaths / 2
Comparison:  2 deaths / 2
```

Expected exact results:

```text
risk difference = +1
risk ratio      = +infinity

comparison deaths : hypergeometric weight
0                 : 1
1                 : 4
2                 : 1

total weight      = 6
observed weight   = 1
two-sided p       = 1/3
```

This fixture is intentionally small enough for independent manual verification.

## Wire-shape boundary

SEL-07B2 contains no field that certifies:

- causal selection;
- fitness;
- model-specific selection coefficient;
- beneficial/deleterious mutation status;
- adaptation;
- reproductive isolation;
- speciation.

The probability result is not named or typed as a causal-selection decision.

## Explicit non-claims

SEL-07B2 does not establish:

- predictor scientific validity beyond the already bound SEL-07B1 provenance;
- randomization;
- exchangeability;
- no unmeasured confounding;
- genotype -> phenotype causation;
- exposure -> consequence causation;
- causal selection;
- scalar or relative fitness in general;
- a population-genetic selection coefficient;
- beneficial/deleterious mutation classification;
- adaptation;
- reproductive isolation;
- speciation.

## Successor boundary

SEL-07C may introduce a distinct `CausalSelectionEffectEstimate` only after independently satisfying the causal-identification requirements associated with the frozen source design.

It must consume descriptive evidence without promoting it by field-copy, enum toggle, or type conversion alone.

A later model-specific selection coefficient remains a separate conversion requiring an explicit population-genetic model and estimand.

## Evidence status

Source implementation, static review, contract text, and test design are not executable qualification.

A frozen CI-only helper must execute the exact final SEL-07B2 product head before any PASS is claimed.

Queued/unassigned CI remains unexecuted evidence and must not be interpreted as product-code failure or success.
