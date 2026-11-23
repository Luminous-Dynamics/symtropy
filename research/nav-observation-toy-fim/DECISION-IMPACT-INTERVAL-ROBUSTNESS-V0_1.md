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
