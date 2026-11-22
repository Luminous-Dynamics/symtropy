# Decision Identifiability Benchmark v0.1

**Owner:** [#6804 — decision identifiability](https://github.com/Luminous-Dynamics/symtropy/issues/6804)  
**Composes:** [#6659](https://github.com/Luminous-Dynamics/symtropy/issues/6659), [#6752](https://github.com/Luminous-Dynamics/symtropy/issues/6752), [#4251](https://github.com/Luminous-Dynamics/symtropy/issues/4251), [#6715](https://github.com/Luminous-Dynamics/symtropy/pull/6715)  
**Implementation:** `tools/orbital-observation-surface-bench/src/decision_identifiability.rs`  
**Claim ceiling:** exact finite probability-law algebra over frozen synthetic worlds only.

## 1. Research change

The evaluator must not require exact physical-parameter recovery when every physical/observation world compatible with the admitted evidence implies the same domain-owned decision.

Let (W(E)) be the finite set of declared worlds whose public observation laws are exactly equal to the admitted evidence law. Let (d(w)) be the predeclared decision class supplied by the domain owner:

[
D(E)={d(w):win W(E)}.
]

A decision is identified exactly when (D(E)) is a singleton. Parameter identification is reported separately.

This supports five materially different outcomes:

- `DECISION_IDENTIFIED`: one compatible parameter signature and one decision class.
- `PARAMETERS_UNIDENTIFIED_DECISION_IDENTIFIED`: multiple parameter signatures remain, but every compatible world implies the same decision.
- `DECISION_NOT_IDENTIFIED`: compatible worlds imply different decision classes; no unique decision may be emitted.
- `IDENTIFIED_SET_EMPTY_MODEL_INCOMPATIBLE`: no declared world matches the evidence law; do not select the nearest world or normalize an empty set.
- `EVIDENCE_INDETERMINATE`: evidence admission is not established; the evaluator does not perform identification.

## 2. Exact-law semantics and deliberate boundary

v0.1 uses finite categorical probability laws with rational masses. Two laws are equal by exact cross-multiplication, not floating-point tolerance. Fractions may use different encodings (1/2 and 3/6) and still compare equal. Every law must sum exactly to one.

This is an **analytical equivalence oracle**, not a finite-sample estimator. The input evidence law is the exact synthetic law used by the fixture. It is not a claim that a finite observed event history reveals its generating distribution. Approximate likelihood compatibility, confidence regions, Bayesian posteriors, finite-sample coverage, and asymptotic tests belong to a later separately qualified statistical layer.

Integer numerators, denominators and probe-cost fields are limited to the JavaScript safe-integer envelope so the independent oracle can reproduce all arithmetic exactly with `BigInt`. The Rust evaluator additionally uses checked rational addition and rejects overflow rather than silently approximating.

## 3. Probe selection

A probe supplies a conditional public-outcome law for every candidate world. For each possible outcome, the evaluator finds the worlds that could produce it and counts the remaining decision classes. The worst outcome determines the probe's residual ambiguity.

The current selector uses this lexicographic order:

1. Exclude every probe not marked admissible or lacking an upstream admissibility-receipt identity.
2. Minimize worst-case remaining decision-class count.
3. Minimize reference trials.
4. Minimize exposure in the fixture's declared micro-units.
5. Minimize elapsed milliseconds.
6. Use the probe ID as a deterministic final tie-break.

This is a research ranking over synthetic, predeclared admissibility labels. The implementation does not verify a receipt signature, assess physical hazard, or authorize an experiment. A real probe must be governed by the upstream safety, metrology and campaign owners. **A probe is selected only if every possible public outcome leaves at most one decision class.** If every admissible probe has at least one outcome compatible with opposite decision classes, no probe is selected; the candidates remain visible in the ranked diagnostics and the result is `no_resolving_admissible_probe`. This is immediate one-observation partition semantics, not a claim that a stochastic probe has no sequential information. Overlapping-support evidence accumulation belongs to the finite-sample sequential follow-on [#6887](https://github.com/Luminous-Dynamics/symtropy/issues/6887).

The selector currently does not use expected information gain. That is intentional: generic entropy reduction is not allowed to overrule safety/admissibility or worst-case decision separation. A later tournament should compare this policy against entropy gain, random calibration, uniform grids, maximin discrimination, and oracle-only upper bounds.

## 4. Frozen v0.1 controls

The corpus contains twelve cases:

1. Constant thinning with different physical rates and capture probabilities but the same decision.
2. The same exact observation law with opposite decision classes, resolved by an admitted independent-reference probe.
3. Multi-sensor common-cause loss versus apparently independent low capture.
4. Mark-dependent physical/detection alias.
5. Latent-rate versus dead-time alias.
6. False-negative and false-positive compensation.
7. A permitted probe that does not resolve the decision ambiguity.
8. A statistically discriminating but inadmissible probe that must be excluded.
9. Commanded-versus-realized stimulus confounding without independent readback.
10. Out-of-domain evidence compatible with no declared world.
11. Evidence-admission indeterminacy.
12. A single compatible world with an identified parameter signature and decision.

These are finite conformance fixtures, not 12 independent statistical trials and not a population performance estimate. The hidden causal story encoded in each parameter signature is fixture documentation; the evaluator uses only candidate public laws, signatures and domain-supplied decision labels. No fixture has a privileged true-world flag.

## 5. Reproducibility and independent implementation

The Rust evaluator emits a source-pinned receipt binding the corpus SHA-256. A separate JavaScript implementation uses `BigInt` rational arithmetic and independently computes compatible-world sets, decision sets and probe ranking. CI compares the semantic outputs field-by-field and validates the receipt schema.

The Rust evaluator and JS oracle must not import shared decision logic. Both implementations consume the same frozen corpus, but neither can read simulator audit fields or a hidden selected-world label. This is finite-domain cross-implementation agreement, not formal verification of either implementation.

## 6. Benchmark comparison

Use matched evidence, candidate-world set, permitted probe set, trial/cost budgets and decision map when comparing:

- Plug-in point estimate plus threshold decision.
- Maximum-likelihood / MAP world decision.
- Conventional POD confidence-bound decisions when assumptions match.
- Parameter-set identification followed by worst-case decision.
- Decision-set identification with explicit abstention.
- Entropy / expected-information-gain probe selection.
- Maximin decision-class discrimination per unit native cost.
- Safety-filtered lexicographic decision-focused selection.
- Exhaustive exact finite oracle as an upper bound only.

External methods solve different subproblems and must not be presented as end-to-end equivalents:

- NIST TN 2119 provides established confidence-interval and confidence-bound procedures for POD, probability of identification and false-alarm estimates from instrument tests: https://doi.org/10.6028/NIST.TN.2119
- ASTM E2862-23 covers hit/miss POD regression for a defined NDE examination configuration, including model validation and interpretation. It assumes the examination procedure and data-collection process are sufficiently controlled; it does not model every upstream loss mechanism: https://doi.org/10.1520/E2862-23
- NASA-STD-5009C is a domain-specific NDE standard for fracture-critical metallic components, not a general software certification target: https://standards.nasa.gov/node/249
- Mottier et al. (21 October 2026) study structural and practical identifiability in thinned Poisson point-process models, including spatial detection variation, temporal replication and partial pooling. This is a direct observation-process baseline to reproduce under matched assumptions: https://arxiv.org/abs/2610.29365
- Timans et al. (UAI 2025) provide a comparator for sequential risk-violation monitoring under unknown distribution shift using testing-by-betting: https://proceedings.mlr.press/v286/timans25a.html
- Bao et al. (JMLR 2025) study online selective conformal prediction with false coverage-statement-rate control; this is relevant to a future sequential uncertainty/reporting layer, not exact physical-world equivalence: https://jmlr.org/beta/papers/v26/24-0452.html
- Set-identification and moment-restriction methods provide the broader statistical foundation for identified sets and inference when point identification fails: https://doi.org/10.1146/annurev-economics-063016-103658

## 7. Prospective research gates

Targets, not achieved results:

- 100% agreement with an independent exhaustive oracle on all bounded finite fixtures.
- Zero false singleton decisions on exact observational-equivalence controls.
- 100% correct distinction between parameter ambiguity and decision ambiguity on the frozen corpus.
- Zero probe selections without the required admissibility-receipt identity.
- Zero nearest-world commitments on empty-compatible-set controls.
- For a later randomized confirmatory challenge, zero false decision commits in at least 299 independent episodes gives an unadjusted one-sided 95% exact upper bound below 1% for that sampled challenge distribution. Hand-authored fixtures do not qualify for that population claim.
- Evaluate 90% and 95% set coverage separately by every admitted stratum, using preregistered exact intervals and multiplicity allocation. Roughly 1,825 independent simulations per stratum is an initial planning approximation for ±1 percentage-point Monte Carlo precision near 95% coverage; finalize sample sizes before confirmatory runs.
- At matched decision error and coverage, target at least 25% fewer independent reference trials than uniform calibration on held-out configurations. Report all strata and the complete cost/coverage frontier.

Do not combine these into a single weighted leaderboard score. Report false commitment, set coverage, correct abstention, unnecessary abstention, worst-case regret, decision ambiguity after probes, reference-trial cost, runtime, memory and reproducibility separately.

## 8. What passing means

A passing v0.1 receipt demonstrates the exact finite set algebra and probe-ranking behavior for these synthetic fixtures, plus agreement between two implementations. It does not establish finite-sample statistical coverage, real-world identifiability, sensor completeness, reference independence, safe physical intervention, physical reliability, hardware qualification, flight readiness or mission authority. Those claims require the separately scoped evidence and physical campaign work owned by #6659, #6715 and the canonical metrology/safety workstreams.
