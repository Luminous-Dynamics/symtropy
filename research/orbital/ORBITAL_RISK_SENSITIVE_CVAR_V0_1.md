# ORBITAL risk-sensitive incumbent extension v0.1

**Status:** research/CI implementation; synthetic probability receipt only.  
**Related work:** #7374 — ORBITAL-RISK-SENSITIVE-INCUMBENT-001.  
**Implementation:** `src/cvar_regret.rs`, `src/bin/cvar-regret.rs`; independent oracle `tests/cvar_regret_oracle.py`.  
**Claim ceiling:** `probability-conditioned-synthetic-tail-risk-algebra-only`.

## 1. Why this is separate from the current incumbent gate

The current incumbent adapter evaluates an unweighted finite joint inventory of compatible physical worlds and loss models. Its distribution-free worst-case result remains authoritative within its declared finite claim ceiling.

A scenario inventory is not a probability distribution. Scenario frequency, row order, model count or convenient weights must never be used to manufacture one.

This extension therefore has a separate input contract. It requires an explicit, scope-bound probability-model receipt with exact scenario weights, a current calibration/validation receipt, coverage qualification and declared dependence assumptions. Without that receipt, the CVaR track returns `ABSTAIN`; it does not change the existing worst-case result.

## 2. Exact estimand

For a policy `pi`, let `R(pi, s)` denote a specific per-scenario decision-regret estimand and let `p(s)` be the supplied joint probability mass. The upper-tail CVaR at confidence level `alpha` is:

`CVaR_alpha(R) = min_eta [ eta + (1 / (1-alpha)) * sum_s p(s) * max(R(pi,s)-eta, 0) ]`

The implementation reports three distinct quantities for each regret estimand:
- expected regret under the supplied joint distribution;
- worst-case regret over the declared support;
- CVaR at each preregistered alpha.

It keeps **wait-and-see regret** and **outcome-class regret** separate. Terminal loss, expected loss, worst-case loss and regret are not interchangeable.

For finite support, the Rockafellar–Uryasev objective is piecewise-linear convex in `eta`; evaluating its exact rational value at every unique support loss gives the finite-support minimum. The report includes the fraction of the VaR atom needed to fill the upper tail. This makes discrete ties and the case where the tail probability is smaller than one scenario atom explicit rather than approximating or dropping the partial atom.

## 3. Receipt contract

The probability receipt binds:
- model and receipt IDs, source commit, case, scope ID/digest and decision cutoff;
- compatible-world receipt, joint loss-model receipt and full joint scenario manifest digest;
- exact expected scenario IDs and per-scenario nonnegative rational weights;
- calibration receipt ID/digest and validation split ID/digest;
- model currentness, calibration qualification, support coverage and model-family state;
- explicit joint-distribution and dependence assumptions.

The weights must cover the exact joint scenario ID inventory once and sum exactly to one. Their payload digest is recomputed. The joint scenario manifest is also recomputed from the full (scenario ID, physical world ID, loss-model ID) records.

Calibration is a supplied source claim. The evaluator verifies structural pins and the declared qualification state; it does **not** infer that real-world probabilities are correct from a calibration label or ECE-like statistic, and it does not independently establish source authenticity.

## 4. Exact arithmetic and fail-closed behavior

- Python uses the standard-library `fractions.Fraction` oracle.
- Rust uses bounded reduced rational arithmetic with checked intermediate operations. Unsupported/overflowing exact results fail closed; no floating-point fallback exists.
- Rational inputs must be canonical (reduced) and bounded.
- Weights may be zero, but the complete declared support must still match. Zero-mass rows do not contribute to expected or tail risk.
- CVaR alpha must be a unique exact rational strictly between zero and one.
- A stale model, unqualified calibration, inadequate coverage or inadequate model family yields `ABSTAIN`.
- Missing, altered, non-normalized or support-mismatched weights yield `ABSTAIN` or `INVALID_INPUT` as appropriate.
- No policy is ranked for execution, selected, or authorized.

## 5. Frozen synthetic known answer

The fixture uses four joint scenarios with masses:
- 0.70, 0.20, 0.08 and 0.02.

It tests two policies at alpha 0.90, 0.95 and 0.99. For wait-and-see regret:
- incumbent expected regret = 16/25; worst regret = 10; CVaR = 22/5, 29/5, 10;
- candidate expected regret = 19/50; worst regret = 6; CVaR = 14/5, 18/5, 6.

For outcome-class regret:
- incumbent expected regret = 2/5; worst regret = 7; CVaR = 3, 4, 7;
- candidate expected regret = 6/25; worst regret = 4; CVaR = 2, 5/2, 4.

The corpus also has three non-pass controls: no probability receipt, stale model receipt, and unqualified calibration. They must all abstain; the report has one pass, three abstentions, zero invalid cases and zero authority grants.

Unit tests also cover:
- a tail mass smaller than one scenario atom;
- exact fractional consumption of the VaR boundary atom;
- tied losses and zero-probability support rows;
- non-normalized probability weights;
- alpha endpoints and support tampering.

These values demonstrate the exact finite-distribution calculation only. They do not validate the probability model or establish any empirical improvement in deployed outcomes.

## 6. Research comparison and what would constitute a real contribution

- **PEAR, ICML 2026:** tangent-space projection of prediction error characterizes regret gradients under regularity and locally stable active constraints; it is a training/gradient method, not a probability-receipt or tail-risk assurance layer. https://proceedings.mlr.press/v306/lee26q.html
- **Robust DFL, UAI 2026:** addresses measurement-error uncertainty sets and Wasserstein ambiguity around conditional empirical distributions via worst-case regret. That is a complementary robust-distribution track; it must not be collapsed into plain CVaR. https://proceedings.mlr.press/v337/yamao26a.html
- **Risk-Averse DFL / CVaR of regret, arXiv preprint dated 14 November 2026:** uses entropy-regularized instance reweighting to train tail-sensitive predictors, with experiments on knapsack, portfolio, shortest path and energy-aware scheduling. It is a recent preprint, not treated here as a peer-reviewed SOTA result. https://arxiv.org/abs/2611.19575
- **IJCAI 2026 active feature acquisition survey:** frames sequential feature acquisition using MDP/POMDP formulations and identifies robustness, logging, computational and deployment challenges. This is relevant to the later adaptive-query track, not evidence that the current passive finite model supports adaptive stopping. https://ijcai.org/proceedings/2026/860

Industry safety/ALM tools establish a strong baseline for structured safety analysis, requirements traceability and change-impact governance. Ansys medini documents FMEA/FMEDA, HAZOP, FTA and related safety/reliability methods; Siemens Polarion describes traceability, test management and impact analysis; IBM ELM/DOORS Next documents requirements traceability and change impact. These are adjacent enterprise capabilities, not a measured head-to-head CVaR benchmark:
- https://ansyshelp.ansys.com/public/Views/Secured/medini/v261/en/pdf/Ansys_medini_analyze_User_Guide.pdf
- https://www.siemens.com/en-gb/products/polarion/application-lifecycle-management-alm/
- https://www.ibm.com/products/engineering-lifecycle-management

The Symtropy contribution to test is the composition of: current source evidence, exact world × loss-model support, non-anticipative policies, a probability receipt with explicit assumptions, independently replayable tail metrics, and selective abstention. That is a research hypothesis, not a claim of superiority over those tools or papers.

## 7. Next benchmark gates

1. **Distribution receipt integrity:** mutate scenario IDs, joint mapping, scope, cutoff, weights, calibration receipt, validation split and source digests; zero invalid receipt escapes.
2. **Exact arithmetic parity:** generated bounded rational distributions compared across Rust and Python Fraction; zero mismatches, including partial VaR atoms and ties.
3. **Statistical adequacy:** evaluate calibration on a held-out split, report reliability diagrams, Brier/log scores and finite-sample uncertainty. These diagnose calibration; none alone proves support completeness.
4. **Shift robustness:** add a separate Wasserstein/ambiguity-set model binding ground metric, units, support and radius; report worst-case expected regret separately from plain CVaR.
5. **Decision-level outcomes:** compare static incumbent, common-action baseline, observable policy frontier, expected-regret policy, CVaR-regret policy and robust ambiguity-set policy on matched domains.
6. **Adaptive acquisition:** only after the upstream sequential/optional-stopping contract is actually bound and qualified.
7. **Scale:** measure p50/p95/p99 runtime, peak memory, rational-overflow rates and deterministic replay on increasing scenario/policy counts.

Targets are prospective: zero missing-support/provenance escapes, zero arithmetic mismatch, zero unauthorized actions, and explicit abstention on every missing or unqualified probability model. Tail-risk confidence levels are preregistered; no post-hoc alpha selection.

## 8. Non-negotiable claim boundary

CVaR is conditional on the supplied probability model. This module cannot show that the weights describe reality, prove calibration, establish the candidate support is complete, validate stochastic dependence assumptions, or replace the existing worst-case comparator. The output is an evidence-qualified analytical metric only, not a recommendation or operational decision.
