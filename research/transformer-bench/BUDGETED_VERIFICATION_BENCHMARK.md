# PWR-TX-ACQ-001 — Budgeted candidate verification policy benchmark

**Status:** executable benchmark scaffold; the checked-in pool is synthetic, and no FEA was run.

## Why this benchmark exists

The core question is not whether uncertainty sampling sounds sensible. It is whether a policy spends a fixed expensive-evaluation budget on candidates that yield better engineering decisions than credible alternatives. This is especially important because recent reviews of active learning in materials science warn that acquisition policies can underperform random sampling when their assumptions are wrong, and their relative behavior depends on the objective and dataset. See [A critical examination of active learning workflows in materials science](https://www.sciencedirect.com/science/article/pii/S2635098X26001269) (2026), [NIST: Benchmarking Active Learning Strategies for Materials Optimization and Discovery](https://www.nist.gov/publications/benchmarking-active-learning-strategies-materials-optimization-and-discovery), and [Benchmarking the acceleration of materials discovery by sequential learning](https://pubs.rsc.org/en/content/articlehtml/2020/sc/c9sc05999g).

This is motivated by the transformer surrogate work, but the implementation is a *budgeted verification-policy replay* against a frozen candidate pool. It is **not** a full active-learning system: it does not retrain a model or call FEA after each query. The ground-truth reference column is present for offline scoring, and the policy selection function uses prediction, uncertainty, design features, and (for expected improvement) only the best reference value revealed by prior selected candidates. This fixture is not physical evidence and cannot establish real query savings.

## Run

```sh
cargo run --locked --manifest-path research/transformer-bench/Cargo.toml --bin acquisition-bench -- \
  research/transformer-bench/fixtures/acquisition-pool-synthetic.csv \
  /tmp/acquisition-report.csv 6 30 8
```

Arguments are input pool, report path, query budget per policy/replicate, replicate count, and optional oracle top-k (default 8). Budgets must be in `1..=candidate_count`. The input must contain at least two candidates and a *single* frequency/excitation domain; evaluate different frequency or excitation classes in separate runs. The same seed is paired across policies within each replicate. Randomized policies use a deterministic SplitMix64 generator.

## Input contract

Exact header:

`candidate_id,geometry_family,material_id,frequency_hz,excitation_class,feature_1,feature_2,feature_3,reference_core_loss_w,predicted_core_loss_w,prediction_lower_w,prediction_upper_w`

All values must be finite; candidate IDs must be unique; reference/predicted loss and interval limits must be positive; predictions must be inside their intervals. The three numeric features are normalized by observed pool ranges for the diversity policy. Geometry-family and material-ID mismatches add categorical distance. Because normalization uses the whole candidate pool, this is suitable for offline benchmarking but is not a streaming deployment protocol.

## Policies compared under identical budgets

- `RANDOM`: seeded random selection.
- `MIN_PREDICTED`: exploit the lowest predicted core loss.
- `MAX_UNCERTAINTY`: select the widest prediction interval.
- `LOWER_CONFIDENCE_BOUND`: minimize predicted loss minus 1.96 estimated standard deviations, treating the interval as a nominal 95% normal interval.
- `EXPECTED_IMPROVEMENT`: minimize predicted loss with expected improvement relative to the best already queried reference value. Predictions and intervals remain frozen, so this is sequential selection with a static surrogate, not iterative model retraining.
- `DIVERSE`: farthest-point selection over normalized numeric features plus categorical geometry/material dissimilarity.

Expected-improvement Gaussian CDF uses the Abramowitz–Stegun 7.1.26 approximation. The algorithm uses it only to rank acquisition choices; the approximation is not a statistical calibration claim.

## Output and interpretation

Each row is one policy/replicate trial. It reports the same query budget, oracle best available reference loss, best reference loss found within budget, percentage decision regret relative to the pool's oracle best, fraction of the oracle top-k candidates found, ordered queried IDs, and the mean prediction relative error among queried candidates.

The most important outcome metrics are best-found regret and top-k recall, not query count reduction: all six policies get exactly the same budget, and this program does not execute or price FEA calls. Mean error among queried candidates helps detect a policy that selects unusually difficult points but is not itself a measure of decision quality. Results should be aggregated across paired replicates and datasets with confidence intervals; this program emits raw trial rows and does not yet compute statistical significance.

## Research gates to aim for

These are aspirational preregistered gates, not standards or current results:

1. Evaluate across real, traceable reference pools with group-held-out splits; use `split-audit` and bind pool/split/model/report with `evidence-receipt`.
2. Compare every learned acquisition policy to RANDOM, MIN_PREDICTED and DIVERSE under equal budgets—not just to random.
3. At equal budget, aim for at least 20% lower mean best-found regret than the best simple baseline, with a paired 95% confidence interval excluding no improvement across multiple distinct datasets. Report failures and negative transfer; do not select datasets post hoc.
4. Report the full regret-vs-budget curve and top-k recall-vs-budget curve, not only one budget. Include at least 10, 25, 50, and 100 percent pool-budget operating points when pool size permits.
5. Require deterministic replay from the same seed and input digest; bind the exact CLI arguments, software revision, pool digest and output digest in the evidence record.
6. Add a true sequential active-learning phase only after an interface can execute a pinned reference solver, update the surrogate, and charge all training, validation, failed/retried jobs, final verification, and solver refinement calls to the budget.

A policy that does not beat strong simple baselines should not be retained merely because it is described as Bayesian, uncertainty-aware, or AI-driven.

## Limitations

- No FEA / CFD / laboratory reference was evaluated by this tool.
- The candidate pool contains oracle labels so offline metrics can be scored; this does not mean a deployment policy is allowed to read truth before choosing.
- The frozen predictor does not update after new observations; do not claim this benchmark demonstrates full active learning or active surrogate retraining.
- Candidate space is represented by three caller-declared numeric features plus geometry/material categories; it does not encode transformer physics.
- Intervals are caller-supplied and are not calibrated by this binary.
- No significance test or cross-dataset aggregation is emitted yet.
- Synthetic results are plumbing checks only, not industry comparisons or SOTA claims.
