# MARITIME-CAL-CONTINGENCY-001: minimax service-window selection from calibrated ETA intervals

**Status:** executable synthetic adapter and six authored Rust tests; exact-head CI pending.  
**Tracking:** https://github.com/Luminous-Dynamics/symtropy/issues/1977  
**Composes:** split-conformal ETA calibration and the evidence-aware decision benchmark.

## Paradigm shift

A point ETA is not the decision input. The decision input should be an uncertainty interval that was calibrated from residuals available before a strict cutoff, plus source-conflict and location-identity status. The scheduling benchmark can then ask which candidate window has the lowest worst-case cost over that interval, while treating abstention as a real alternative with a declared cost.

This adapter composes two reference modules without giving either module operational authority:
- ETA residuals and finite-sample intervals come from arrival_calibration;
- candidate-window costs come from evidence_action;
- calibrated_contingency selects or abstains and records a reason.

## Selection rule

Given a finite interval [lower, upper] and candidate window w, compute:

max(score_window(lower, w), score_window(upper, w))

The current synthetic score is monotone as the arrival time moves away from the window start on either side, including the declared lateness penalty jump, so its worst case over the interval is attained at one of the endpoints. The adapter chooses the candidate with minimum worst-case cost, breaking ties deterministically by earlier window start and then window ID.

It does not select if:
- calibration produced an infinite/unavailable radius due to insufficient sample size;
- no candidate windows were supplied;
- the minimum worst-case cost is greater than or equal to the declared abstention cost;
- the interval bounds overflow the supported timestamp representation (reported as an error; no schedule action is produced).

## Tests authored

Six synthetic Rust tests cover:
1. a finite interval choosing a window with lower worst-case cost than the point-nearest alternative;
2. an unbounded calibration interval forcing explicit abstention;
3. abstention when best worst-case cost equals the declared abstention cost;
4. no candidate windows as a distinct abstention reason;
5. candidate-window permutation invariance;
6. timestamp interval overflow failing closed.

The current fixture estimates arrival at 12:55 UTC with a nine-second radius. For the synthetic 12:00 and 13:00 windows, the 13:00 window has five cost-minutes of worst-case waiting, while the earlier window misses its grace and incurs a larger penalty. If the abstention cost is five, the same selection must abstain instead of accepting an equal-cost option.

## Research and qualification context

The calibration module uses a finite-sample split-conformal residual quantile. Its marginal coverage interpretation requires exchangeability; time drift, data selection and subgroup differences can invalidate the practical assumption. The adapter treats its interval as supplied input and does not claim to repair miscalibration.

References:
- Doubly robust prediction-set calibration under covariate shift (JRSS-B, 2024): https://academic.oup.com/jrsssb/article/86/4/943/7618755
- Gibbs & Candès, Conformal Inference for Online Prediction with Arbitrary Distribution Shifts (JMLR 25(162), 2024): https://www.jmlr.org/papers/v25/22-1218.html
- DCSA Port Call Standard, operational-event semantics and JIT timing: https://dcsa.org/standards/port-call/documentation-port-call

## Proposed benchmark gates (not measured)

- 100% selection and abstention agreement with an independent exhaustive oracle over tiny combinations of interval radius, window start, grace, penalty and abstention cost.
- Zero selections for unbounded calibration intervals.
- Zero selections when best worst-case cost is equal to or greater than abstention cost.
- 100% determinism under candidate-window permutation and exact tie conditions.
- Report calibration nominal target, residual sample count, radius, candidate worst-case costs, selected window, and abstention reason as separate fields.
- Evaluate under baseline and shifted residual regimes; do not imply that minimax selection fixes loss of calibration coverage.

## Authority boundary

This is a deterministic synthetic selection reference. It does not book a berth, change a vessel speed, post DCSA requests, update schedules, activate participant flexibility, or settle payments. A future Mycelix adapter must validate participant consent, authorization, contract terms and lifecycle state before any actual schedule effect. The result here is not a live operational recommendation.
