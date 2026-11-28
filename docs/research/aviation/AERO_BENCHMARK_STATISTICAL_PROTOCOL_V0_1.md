# AERO Benchmark Statistical Protocol v0.1

**Status:** prospective preregistration; not executed  
**Scope:** critical metrics in AERO IAB v0.3  
**Confidence:** one-sided 95% exact binomial bounds unless a future version preregisters another level

## Zero-adverse-event confidence

For \(n\) independent Bernoulli units and zero observed adverse events, the exact one-sided Clopper-Pearson upper confidence bound is:

\[
U = 1 - \alpha^{1/n}
\]

With \(\alpha=0.05\), the minimum sample sizes required to place the upper bound at or below a target adverse-event probability are:

| Target upper adverse-event probability | Minimum independent units with zero adverse events |
|---:|---:|
| 1% | 299 |
| 0.1% | 2,995 |
| 0.01% | 29,956 |

The machine-readable registry stores the formulas and values; CI recomputes them with the standard-library logarithm and ceiling functions. The rule of three (\(3/n\)) is an approximation, not the normative calculation.

## Independence and clustering

One independent unit is one independently sourced engineering episode/root-cause family. Generated mutations, alternative prompts, evidence variants, graph permutations and replay modes derived from one root cause remain one cluster. They provide useful robustness coverage but cannot inflate the independent \(n\).

Partition by root-cause family before generating variants. Do not place sibling mutations in train, development, visible challenge and blind-test partitions. Report:

1. independently sourced episodes;
2. unique root-cause families;
3. generated variants and metamorphic transformations;
4. critical strata with sufficient sample size;
5. maximum cluster size and dependence assumptions.

Where multiple observations per cluster are unavoidable, use a preregistered cluster-level analysis or hierarchical model; do not substitute raw row count for independent \(n\).

## Critical strata and multiplicity

For every critical metric, publish numerator, denominator, point estimate, exact one-sided confidence bound, independent-unit count and worst stratum. Every critical stratum must meet its own preregistered sample requirement. An aggregate zero does not offset one stratum with an observed failure.

For each metric, show a one-sided 95% bound. Because several critical gates are evaluated together, also show a simultaneous family-wise bound using a preregistered Bonferroni adjustment \(\alpha/M\), where \(M\) is the number of critical hypotheses. Do not choose \(M\) after inspecting outcomes.

## Zero denominators and nonzero events

If a denominator is zero, report \`NOT_ESTIMABLE\`. Never report zero errors, 100% success or pass. For nonzero adverse-event counts, report exact one-sided Clopper-Pearson upper bounds. For success metrics, report complementary exact lower bounds. Avoid Wald/normal approximations for rare critical events.

## Interpretation and release gates

The sample sizes above are not evidence of performance. They are the minimum independent sample sizes needed to make a particular upper-bound statement **if zero adverse events are observed**. Synthetic tests may establish code-path coverage; they do not establish population risk, physical-model credibility or airworthiness.

The statistical gate does not replace independent adjudication, domain expertise, source authority or the R0-R5 release gates. AERO remains research-only until the relevant independent reviewers and authorities accept the evidence and intended use.
