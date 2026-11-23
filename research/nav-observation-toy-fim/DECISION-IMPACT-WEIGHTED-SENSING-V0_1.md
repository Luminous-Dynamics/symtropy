# Decision-impact-weighted active sensing v0.1

**Status:** research increment stacked on navigation evidence transaction PR #7167. This is a deterministic synthetic software component. No sensor was activated, no field data was scored, and no navigation state or authority was changed.

## Why this increment exists

The preceding decision-aware planner improves on raw information gain by ignoring hypothesis pairs that imply the same navigation decision. Its next limitation is that it counts every pair of distinct decision classes equally.

That can be irrational when consequences differ: resolving many low-consequence ambiguities can dominate one unresolved high-consequence ambiguity. It can also let a decision class with many nuisance hypotheses dominate another class merely because it generates more pair instances.

This increment introduces a separately committed, externally owned, symmetric impact-weight matrix over decision-class pairs. It normalizes discrimination within each class pair before applying the relative impact weight.

## Contract and calculation

For each unordered pair of distinct decision classes (c,d):

- n_cd is the number of hypothesis pairs in that class pair.
- s_cd is the number whose conditional outcome enclosures are guaranteed disjoint.
- w_cd is the positive, dimensionless relative impact weight supplied by the decision-profile owner.

The within-pair resolved fraction is conservatively rounded down to parts per billion:

    r_cd = floor(1e9 * s_cd / n_cd) / 1e9

The weighted conditional resolution score is:

    R = floor(sum(w_cd * floor(1e9 * s_cd / n_cd)) / sum(w_cd)) / 1e9

The evaluator requires exactly one weight for every unordered class pair in the frozen decision partition. Missing, duplicated, extra, zero or noncanonical pair entries fail closed. The impact-model digest is preserved and the partition digest must match exactly.

The score is deliberately class-pair normalized. A class pair with 20 hypothesis combinations does not receive 20 times the influence of a class pair with one combination solely because its hypothesis inventory is larger.

The existing #7082 admitted acquisition reliability interval [p_L,p_U] can be composed separately:

- expected resolved-impact fraction: [p_L * R, p_U * R];
- expected residual-impact fraction: [1 - p_U * R, 1 - p_L * R].

This is a planning proxy under the explicit assumption that failed, invalid or unqualified acquisition yields no incremental decision discrimination. It is not a measured field outcome.

## Deliberate claim boundaries

This v0.1 is **not** Bayes risk, expected decision loss, minimax regret, a posterior probability, a calibrated monetary value, a physical-harm estimate, or a safety case.

The weights are symmetric, dimensionless relative severity multipliers, not probabilities or physical units. Their authority, calibration, completeness and provenance remain external. A directed loss model needs explicit true-state/action consequences and a separately admitted probability or uncertainty model; this code does not manufacture those inputs.

The assessment rechecks assignment, class-pair and separation counters against the supplied report, but it does not recreate upstream physical outcome enclosures or authenticate the report's digests.

## Synthetic adversarial example

There are three decision classes. One class pair has a single unresolved hypothesis pair and relative weight 1000. The other two class pairs have ten hypothesis pairs each, all separated, and weight 1 each.

- Unweighted hypothesis-pair separation: 20/21 = 95.2%.
- Weighted, within-class-pair-normalized resolution: 2/1002 = 0.1996%.

The large difference is intentional: the impact model declares that the one unresolved class pair matters much more than the two resolved class pairs. This demonstrates a property of the weighting rule, not that any real navigation decision has 1000 times the consequence of another.

## Tests authored

The module contains ten focused tests covering:

- severe unresolved pair not being hidden by many low-impact resolved pairs;
- normalization against hypothesis-count imbalance within a class pair;
- complete class-pair weight coverage, including missing and extra weights;
- decision-partition and impact-model commitment failures;
- duplicate, zero and noncanonical weights;
- pair/assignment consistency and counter revalidation;
- deterministic invariance to pair, assignment and weight serialization order;
- reliability-interval composition and action-identity mismatch;
- malformed reliability and one-class/no-pair input.

These are authored tests only until exact-head Rust CI completes.

## Benchmark plan: how to outperform the current baseline

No superiority claim follows from adding a score. The next benchmark must compare these methods under identical frozen hypothesis inventories, decision partitions, action candidates, costs and acquisition-reliability contracts:

1. **B0 — uniform hypothesis-pair count:** current unweighted cross-decision pair count.
2. **B1 — uniform class-pair normalized:** equal weight for every decision-class pair, normalized within each pair.
3. **B2 — impact weighted:** owner-supplied relative impact weights over the same pair outcomes.
4. **B3 — worst-class-pair:** maximize the minimum resolved fraction across class pairs.
5. **B4 — minimax ambiguity:** minimize the maximum unresolved impact-weighted class-pair burden.
6. **B5 — exact finite oracle:** exhaustive selection on small finite cases using the declared objective.
7. **B6 — full decision regret:** future track only, after a source-backed directed state/action loss matrix and admitted uncertainty over states exist.

Report separate metrics:

- high-impact unresolved-pair recall and false-reassurance rate;
- weighted resolution and residual ambiguity;
- minimum per-class-pair resolution and worst-pair residual;
- action-selection regret and Pareto-frontier quality against the exact oracle;
- acquisition-probability interval, independent campaign count, latency, energy and compute;
- impact-model calibration and inter-reviewer stability against independently reviewed outcomes;
- abstention rate and reasons when consequence evidence or weights are missing.

**Proposed gates (aspirational, not achieved):**

- 100% rejection of missing, duplicate, extra, zero and wrong-partition impact weights;
- 100% invariance to equivalent input permutations;
- exact oracle parity for all bounded toy cases;
- no aggregate score may be presented as calibrated harm, expected loss or regret;
- adversarial no-masking tests must show that low-consequence pair counts cannot conceal a declared high-impact unresolved class pair;
- held-out physical campaigns must show a predeclared improvement in high-impact ambiguity or decision regret without worsening protection-level coverage, false-safe exposure, availability or resource budgets.

A weighted average is never a sufficient gate by itself: preserve the per-class-pair vector and the worst-pair statistic so one severe residual cannot disappear inside a high mean.

## Industry and benchmark positioning

ISO/IEC 21134:2026 is a current benchmark-framework anchor for integrated indoor localization and tracking with dead reckoning. It gives the work a standards-aligned track for master data sets, test environments, metrics, benchmarking processes and conformance. It is not an end-to-end integrity or safety certificate; those claims require separate fault, temporal, protection-level and authority evidence.

MILUV is a 2026 multi-UAV indoor localization dataset with UWB and vision, motion-capture ground truth and complementary sensors. It can support a separate multimodal track only after actual payload access, rights, hashes, timestamp semantics, calibration and source coverage are admitted. The paper's publication does not establish that this repository ingested or scored the payload.

The 2025 outdoor UWB LOS/NLOS dataset offers a separate ranging/environment generalization track. Keep UWB-only, UWB+IMU and UWB+vision tracks separate; do not pool sensor privilege into one leaderboard.

Published UWB learning results such as delay-compensated tracking and cross-environment NLOS/ranging models are relevant tracking/ranging comparators, not direct competitors for end-to-end trajectory integrity. Compare them on matched sensor inputs, training/test splits, hardware and epochs, then report integrity coverage, false-safe exposure, time validity and decision quality as additional axes. Product accuracy figures and protocol conformance are not substitutes for tail-error or integrity benchmarks.

## Next execution sequence

1. Integrate the weighted assessment into the planner's Pareto vector without deleting raw pair counts, conditional PL delta, latency, energy or reliability provenance.
2. Add an independent exact selector for tiny candidate sets and compare B0-B5.
3. Add asymmetric loss only through a separate state/action consequence contract; do not overload the symmetric class-pair matrix.
4. Implement out-of-sequence measurement replay/fixed-lag repropagation as a separate temporal track, preserving Mycelix stream and clock ownership.
5. Admit MILUV and Bregar payloads only when exact bytes, rights, timestamps, calibration, common-cause and dependency-group evidence are complete.
6. Run held-out physical campaigns before any field-accuracy, integrity or SOTA claim.

## References

- ISO/IEC 21134:2026, official scope: https://www.iso.org/standard/86651.html
- ISO/IEC 21134:2026, official preview: https://www.iso.org/obp/ui?_escaped_fragment_=iso%3Astd%3Aiso-iec%3A21134%3Aed-1%3Av1%3Aen
- MILUV: A Multi-UAV Indoor Localization dataset with UWB and Vision (2026): https://doi.org/10.1177/02783649251405898
- Comprehensive outdoor UWB dataset: Static and dynamic measurements in LOS/NLOS environments (2025): https://doi.org/10.1038/s41597-025-05887-9
- Delay-compensated UWB localization (2026): https://doi.org/10.1016/j.eswa.2026.132248
- Cross-environment UWB NLOS Transformer (2026): https://doi.org/10.1186/s13634-026-01377-1
