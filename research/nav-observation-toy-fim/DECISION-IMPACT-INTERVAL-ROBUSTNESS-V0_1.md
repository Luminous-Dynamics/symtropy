# Interval-robust decision-impact sensing v0.1

**Status:** draft research follow-up to decision-impact scoring PR #7187. The implementation is synthetic, offline and non-authorizing. Exact-head CI is pending for this branch.

## Motivation

A point impact-weight vector can be internally consistent and still be wrong, disputed, stale or domain-specific. Treating those values as exact would turn an external judgement into a false-precision scalar.

v0.1 therefore adds an interval contract for every unordered decision-class pair. The caller supplies positive bounds [wL_i,wU_i] and an uncertainty-model commitment. The evaluator does not infer a probability distribution, midpoint, or independence from those intervals.

## Mathematical contract

For class-pair i, let the exact separability fraction be r_i = s_i/n_i, where s_i is the count of guaranteed separated hypothesis pairs and n_i is the total count in that decision-class pair.

Each r_i is represented with outward fixed-point bounds:

- rL_i = floor(10^9 * s_i/n_i) / 10^9;
- rU_i = ceil(10^9 * s_i/n_i) / 10^9.

For weights w_i in [wL_i,wU_i], the weighted resolution is:

    R(w) = sum_i(w_i * r_i) / sum_i(w_i).

The lower endpoint solves:

    min_w sum_i w_i * (rL_i - t) >= 0.

For each term, the minimizing endpoint is wL_i when rL_i - t is nonnegative, and wU_i when it is negative. The expression is monotone in t, so integer binary search over parts-per-billion yields floor(min_w R_L(w)).

The upper endpoint solves:

    max_w sum_i w_i * (rU_i - t) <= 0.

For each term, the maximizing endpoint is wU_i when rU_i - t is nonnegative, and wL_i when it is negative. The same monotone search yields ceil(max_w R_U(w)).

All multiplication and accumulation uses checked integer arithmetic. No floating-point threshold comparisons, Monte Carlo assumptions or independent-weight probability distribution are used.

The output preserves:

- worst-case and best-case weighted resolution interval;
- complementary worst/best residual ambiguity;
- each decision-class pair's weight bounds, counts and outward resolution bounds;
- the least-resolved pair independently of the aggregate;
- exact decision-partition, impact-model and uncertainty-model commitments.

## Important assumptions and boundaries

The interval box is an outer set: every real impact weight must lie inside its declared interval. If the true joint model has correlations or shared constraints, the box includes combinations that may never co-occur. That can make the result conservative but loose. We do not infer correlations or assign a distribution.

Every lower weight bound must be strictly positive. A zero lower bound would permit a declared class pair to disappear entirely from the weighted objective; this v0.1 rejects that condition rather than silently dropping the pair.

These are symmetric relative importance multipliers, not probabilities, calibrated physical-loss values, expected harm, Bayes risk or minimax regret. The evaluator cannot authenticate the source digest or determine whether the impact model is normatively appropriate. Full decision regret requires a separately governed directed true-state/action loss matrix and a defensible uncertainty contract.

The point scorer's class-pair report is revalidated for pair counts, separated/unresolved counts, per-pair fixed-point values, total weights and aggregate consistency. The interval module does not regenerate upstream sensor outcome enclosures or authenticate model digests.

## Synthetic verification suite

The module includes nine tests:

1. Fixed weight intervals produce tight outward bounds around a non-integer rational resolution.
2. A two-class-pair adversarial interval box returns the exact endpoint extremes for a zero/one resolution case.
3. All eight corners of a three-pair weight box are enumerated and enclosed by the calculated robust interval.
4. Widening all weight intervals cannot improve the worst-case lower bound or shrink the best-case upper bound.
5. Missing, duplicate, extra, zero and reversed weight intervals reject.
6. Partition, impact-model and uncertainty-model commitment mismatches reject.
7. Pair/weight input permutation produces identical output.
8. Incomplete decision-class-pair graphs reject.
9. Tampered counts, class-pair residuals and inconsistent aggregate totals reject.

These are authored synthetic tests. They are not passing until the exact PR head's CI reports success.

## How to compare with the state of practice

Use matched frozen reports and the same candidate actions, source evidence, budgets and stopping rules. Compare:

- **P0 point-weight baseline:** the current decision-impact scorer with fixed point weights.
- **P1 midpoint baseline:** point weights set to interval midpoints; comparator only, never the robust guarantee.
- **P2 robust interval scorer:** proposed worst/best ratio bounds over the entire interval box.
- **P3 worst-class-pair:** maximize the minimum per-class-pair resolution.
- **P4 minimax residual:** minimize the largest unresolved class-pair burden under declared impact bounds.
- **P5 exact endpoint oracle:** enumerate all weight-box corners for small pair inventories; compare the robust integer interval with the exhaustive rational extremum.
- **P6 directed decision-loss oracle:** future track requiring independently governed state/action loss and uncertainty over true states.

Required metrics are robust lower score, robust upper score, interval width, least-resolved class pair, worst residual, ranking reversals across admissible weights, abstentions due to absent bounds, action selection regret against the exact oracle, and compute/latency cost.

Do not reward a narrow interval by itself: an unsupported tight interval is worse than a wide honest one. Measure calibration/containment of impact intervals on separately reviewed held-out decision profiles and report coverage, width, inter-reviewer disagreement and domain-shift failures.

## Proposed targets

- Exact parity with endpoint enumeration for every bounded toy case.
- 100% rejection of incomplete, duplicated, reversed, zero and wrong-commitment intervals.
- 100% deterministic permutation invariance.
- Widening the admissible weight set must never improve the worst-case lower score or shrink the best-case upper score.
- No decision ranking may be described as robust when the impact interval contract is absent or not bound to the exact decision partition and model.
- On held-out, source-reviewed profiles, robust ranking should reduce worst-case decision regret versus midpoint ranking without worsening false-safe exposure, protection-region coverage, availability or resource budgets.

All numeric targets above are research gates, not achieved field results.

## Industry and dataset benchmark plan

Align the position, position-series, orientation, relative-relationship, integrated-localization and practicability reporting with ISO/IEC 21134:2026. That standard defines a benchmark framework for indoor localization/tracking with dead reckoning; it is not an integrity certificate. Use ISO/IEC 18305's successor work only as a separately tracked test/evaluation alignment item while it remains under formal approval.

Use MILUV as a dataset family, but distinguish the Python devkit repository from the actual downloadable data package. The public GitHub devkit declares an MIT license. The dataset publication and catalog declare CC BY 4.0, with the dataset hosted separately on Figshare. This implementation must still bind the exact acquired payload, release, file manifest, hashes, experiment/robot/sensor identity, timestamp semantics, calibration and rights before admission. A passing parser test is not payload-ingestion evidence.

Keep three sensor-privilege tracks distinct: UWB-only; UWB+IMU; UWB+vision. Report the full multimodal reference result separately. Use the 2026 MILUV dataset's 217 minutes over 36 experiments and three quadcopters as a finite indoor benchmark, not as a rare-event integrity proof or universal environment sample.

## Next work

1. Integrate interval-robust values into the active planner's Pareto vector without deleting point score, raw pair counts, protection-level effect, latency, energy or reliability provenance.
2. Add a tiny exact action selector; enumerate policy trees and compare point, midpoint, robust, minimax and exact methods.
3. Extend intervals to directed state/action loss only in a separate schema and authority boundary.
4. Implement out-of-sequence measurement replay/fixed-lag smoothing and repropagation as an orthogonal temporal track.
5. Admit actual MILUV payload artifacts only after rights, hashes, calibration, clock and frame evidence are complete.

## References

- ISO/IEC 21134:2026 official scope: https://www.iso.org/standard/86651.html
- ISO/IEC 21134:2026 official benchmark categories: https://www.iso.org/obp/ui?_escaped_fragment_=iso%3Astd%3Aiso-iec%3A21134%3Aed-1%3Av1%3Aen
- ISO/IEC PRF 18305 edition 2 status: https://www.iso.org/standard/88536.html
- MILUV 2026 publication and dataset description: https://pmc.ncbi.nlm.nih.gov/articles/PMC13552925/
- MILUV Figshare dataset record and CC BY 4.0 rights: https://datasetcatalog.nlm.nih.gov/dataset?q=0002392468
- MILUV devkit source and MIT license: https://github.com/decargroup/miluv
- MILUV dataset access DOI: https://doi.org/10.25452/figshare.plus.28386041.v1


## Extension: risk-aware goal-oriented sensing — 24 November 2026

A highly relevant external comparator is Jakeman et al., **Risk-Aware Goal-Oriented Bayesian Optimal Experimental Design** (arXiv:2609.08023, 7 September 2026; https://arxiv.org/abs/2609.08023).

The paper separates three design-risk levels:
1. posterior push-forward deviation for each quantity of interest (QoI);
2. risk aggregation across components of a vector/field QoI;
3. risk aggregation over the as-yet-unobserved dataset.

It uses a risk-quadrangle construction to connect regret, deviation and risk, and studies safety-margin, average value-at-risk (AVaR/CVaR) and entropic risk. In its advection–diffusion sensor-placement experiment it used 50 candidate sites, 20 downstream QoIs, 4,000 inner and 4,000 outer samples, and 30 bootstrap resamples. Its risk-aware goal-oriented designs differed materially from parameter-focused KL placement; the KL design was comparable to random under the goal-oriented objectives in that specific test problem. Some objective-mismatch losses reached 20–25% between the tested goal-oriented designs. The result is a strong design-method comparator, not a localization benchmark or evidence that one risk measure is universally best.

The paper is especially relevant because it identifies three distinct objectives that navigation research should not collapse:

- **Level 1 — conditional prediction / outcome uncertainty:** how much residual navigation-decision ambiguity remains for each hypothetical observation result. In this project, these per-scenario residual upper bounds must come from an explicit scenario/posterior audit; the current kernel consumes them rather than fabricating them.
- **Level 2 — decision consequence:** how residual ambiguity across different decision classes is aggregated. PR #7187 adds a committed relative-impact model; PR #7211 adds interval-robust weights and retains the worst class pair.
- **Level 3 — future observation/data risk:** whether the action will deliver useful evidence, produce an uninformative or invalid result, fail, or remain unreconciled. The new `navigation_outcome_risk` kernel computes mean residual, VaR and upper-tail AVaR over a supplied finite scenario law, while checking that the total qualified-observation probability lies in the exact #7165 admitted acquisition interval.

This is a *structural alignment*, not a claim of reproducing the paper. The paper's estimator assumes Gaussian observation likelihoods for its main optimized design parameterization and notes poor scaling beyond roughly 100 candidate sensors under very large sensor noise; it identifies sequential OED and non-Gaussian likelihoods as future work. UWB NLOS/CIR and delayed asynchronous sensor outcomes are not Gaussian by default, so the Symtropy/Symtropy research path must not assume that the paper's estimator transfers unchanged.

### Outcome-risk contract

For one frozen action, the caller supplies a finite mutually exclusive scenario set. Each scenario carries an exact probability mass in parts per billion, an outcome category and an upper residual-impact score derived from the exact impact/decision model. The masses must sum exactly to (10^9).

The qualified-observation mass must lie inside the action's #7165 acquisition probability interval and must be bound to the same action, domain, protocol, decision partition, impact model and impact-uncertainty model. Failed, timed-out, acquired-but-invalid and unknown/unreconciled branches retain the pre-action residual score. An unknown event remains a runtime stop-and-reconcile condition under #7138; assigning a scenario mass does not authorize another attempt.

For upper-tail residual impact (L), the kernel reports:
- conservative mean residual impact;
- upper (mathrm{VaR}_{\alpha}(L));
- upper (mathrm{AVaR}_{\alpha}(L)), calculated by consuming exactly the worst (1-\alpha) probability mass, including a fractional boundary scenario;
- worst scenario residual impact;
- separate qualified, failed/timed-out, invalid and unknown probability masses.

The scenario probabilities and per-scenario residuals remain externally supplied. The kernel checks probability arithmetic, action/reliability interval compatibility, digest equality and no-improvement-on-nonqualified-outcome invariants. It does not estimate the distribution, authenticate its source, update a posterior, execute a sensing request or authorize a navigation transition.

### Benchmark implications

Add matched baselines:
1. expected information gain (parameter-focused);
2. goal-oriented mean posterior-predictive deviation;
3. goal-oriented upper-tail AVaR over decision-relevant QoIs;
4. point impact-weighted decision ambiguity;
5. interval-robust impact-weighted ambiguity;
6. outcome-law expected residual;
7. outcome-law AVaR / entropic residual;
8. worst-class-pair and minimax residual;
9. exhaustive exact outcome-tree oracle on tiny finite models.

Report the risk preference explicitly. For each profile compare action rankings, regret against the exact oracle, the change in high-impact tail ambiguity, AVaR/mean ratio, sensitivity to scenario-law bootstrap resampling, sensitivity to impact-weight intervals, abstention, false-safe exposure, calibration, availability, cost and compute. Do not cherry-pick alpha: preregister a profile-relevant range such as 0.90, 0.95, 0.99 and publish the full risk-preference frontier.

For sequential sensing, the next step is a non-anticipative decision tree in which only controller-visible, qualified outcomes select the next observation. A fixed scenario law can test one-step outcome-risk assessment; it cannot alone establish sequential policy superiority. The tiny exhaustive oracle must compare passive, random, EIG, goal-oriented mean, risk-aware goal-oriented, current decision-aware and interval-robust AVaR policies under identical action/resource budgets.

### New kernel status

`src/navigation_outcome_risk.rs` adds:
- action-bound finite outcome scenarios;
- exact sum-to-one probability masses;
- #7165 reliability interval cross-check for qualified-result mass;
- baseline retention for failed/invalid/unknown branches;
- mean, VaR, AVaR and maximum residual outputs;
- tests for a rare high-residual outcome hidden by a low mean, exact fractional tail mass, alpha monotonicity, unknown outcome retention, reliability/mass mismatch, frontier-action and outcome-model binding, reliability-action binding, commitment mismatch, permutation invariance and malformed scenarios.

The action being assessed may be any action that appears exactly once in the reliability-admitted #7165 Pareto frontier; it need not be the plan's single recommended action. This makes it possible to evaluate every admitted alternative with the same finite-outcome contract before comparing actions. Its outcome-model digest and embedded reliability-record action ID must match the same frontier entry.

The outcome-risk module has eleven focused tests:
1. A rare high-residual outcome is visible in AVaR despite a low mean.
2. AVaR consumes a fractional boundary scenario exactly.
3. AVaR is monotone as the confidence level increases.
4. Failed/invalid outcomes cannot claim improvement.
5. Unknown outcomes retain baseline residual impact and remain separately visible.
6. Scenario mass and qualified acquisition probability must both satisfy their contracts.
7. Action and impact commitments must match.
8. The scenario outcome-model digest and reliability action ID bind to the matching planner entry.
9. A non-recommended action on the admitted Pareto frontier can still be assessed.
10. Scenario permutation does not change the result.
11. Duplicate IDs, zero mass and missing commitments reject.

These tests are authored; exact-head Rust CI is the qualification gate.


## Risk-aware Pareto action selection

The module `src/navigation_outcome_risk_frontier.rs` compares the outcome-risk assessment of every eligible action without collapsing the risk and resource dimensions into one weighted scalar.

It requires all compared actions to share the same:
- deployment-domain fingerprint;
- acquisition-protocol digest;
- decision-partition digest;
- impact-model digest;
- impact-uncertainty-model digest.

It also requires comparable risk contracts: one fixed AVaR confidence level, a minimum valid-acquisition lower bound, a minimum count of independent campaign blocks and a maximum confidence error. A scenario law remains action-specific but is carried with its outcome-model and scenario-law digests.

The selection has three deliberately separate stages:

1. **Hard eligibility:** reject any candidate exceeding latency, energy, mean residual, VaR, AVaR or worst-scenario ceilings, or failing the reliability/calibration floor.
2. **Pareto construction:** retain every nondominated candidate over mean residual, VaR, AVaR, worst residual, latency and energy.
3. **Declared preference:** choose a deterministic recommendation from that frontier using one explicit profile policy: AVaR-first, worst-case-first or VaR-first, with latency/energy and stable action ID tie-breaks.

No scalar score hides trade-offs. If no action satisfies every hard gate, the result is abstention with no recommendation. The result never restores navigation authority.

Eight synthetic tests cover:
1. preservation of tail-risk/latency trade-offs;
2. removal of a strictly dominated action;
3. hard resource and risk-ceiling rejection;
4. abstention when every action violates the profile;
5. duplicate IDs, malformed policy and commitment mismatch;
6. invalid risk-metric ordering and scenario-mass totals;
7. rejection of incomparable AVaR confidence levels or insufficient acquisition reliability;
8. deterministic lexicographic preference behavior.

The module does not execute actions or independently authenticate the reliability/scenario evidence. Its candidates must be assessments produced by the preceding evidence transaction and outcome-risk contract.

## Research references

- Jakeman, J. D., White, R., van Bloemen Waanders, B., Kouri, D. P., and Alexanderian, A. (2026), *Risk-Aware Goal-Oriented Bayesian Optimal Experimental Design*, arXiv:2609.08023: https://arxiv.org/abs/2609.08023
