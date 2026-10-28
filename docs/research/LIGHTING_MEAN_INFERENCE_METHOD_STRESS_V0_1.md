# Mean-inference method stress benchmark v0.1

**Status:** exploratory synthetic statistical diagnostic. This is not measured lighting evidence and does not select a winning inference method.

## Aim and estimand

Compare one-sided lower-confidence-bound procedures for the arithmetic mean of percentage energy savings across independent scene families:

`H0: mean_family_savings <= margin` versus `H1: mean_family_savings > margin`.

The sample is the set of independent scene-family outcomes. Repeated seeds, points on one workplane grid and technical replicates are not additional independent families.

## Procedures compared

1. **One-sample Student-t lower bound.** Reference parametric method for the arithmetic mean.
2. **Percentile bootstrap lower bound.** Nonparametric bootstrap of the arithmetic mean, with a predeclared alpha quantile.
3. **Studentized bootstrap-t lower bound.** Bootstrap the t-statistic relative to the observed mean; use its upper critical quantile to form a lower bound for the mean. Degenerate bootstrap resamples are handled conservatively by assigning an infinite upper t-statistic.

All three target the arithmetic mean. The exact sign-test helper is implemented separately for the **median** and is excluded from the mean-method ranking. A median test is not evidence about a mean-saving estimand, especially under skew.

## Frozen development stress grid

- Independent family counts: 10 and 20.
- Distribution shapes: normal, standardized right-skewed lognormal, its reflected left-skewed counterpart, and standardized Student-t with 3 degrees of freedom.
- True means: exactly at the superiority margin (null/type-I error), half an assumed family SD above the margin (near-boundary power), and 1.5 family SD above the margin (a larger design alternative).
- Primary report: rejection fraction and one-sided lower-bound coverage for each method, each with an individual 95% Wilson Monte Carlo interval.
- Fixed random seed, JSON output and source digest.

This is a stress matrix for exploring failure modes, not an empirical model of building populations. The bootstrap procedures are approximations and are not guaranteed to control type-I error in small samples under skew.

## Exploratory local prototype result

A locally executed 500-repetition / 199-bootstrap comparison at alpha 0.05, using a family-effect SD of 1 percentage point, produced the following rejection fractions at the null mean. Monte Carlo intervals are required because 500 repetitions leave meaningful simulation noise.

| Independent families | Distribution | Student-t | Percentile bootstrap | Studentized bootstrap-t |
|---:|---|---:|---:|---:|
| 10 | Normal | 5.2% | 8.0% | 5.2% |
| 10 | Left-skewed | 15.0% | 18.8% | 9.4% |
| 10 | Right-skewed | 0.8% | 2.6% | 2.6% |
| 10 | Student-t(df=3) | 5.0% | 9.4% | 8.0% |
| 20 | Normal | 5.0% | 6.6% | 4.6% |
| 20 | Left-skewed | 15.6% | 16.4% | 11.2% |
| 20 | Right-skewed | 1.4% | 3.8% | 4.6% |
| 20 | Student-t(df=3) | 4.6% | 7.2% | 7.2% |

These numbers are exploratory results from a local prototype with the same declared procedures, not physical data and not a claim that this exact PR head passed local tests. The repository workflow has a test suite and CLI smoke test; its exact-head result must be inspected separately. The results already indicate there is no universal winner in this small, hand-declared grid. The bootstrap-t procedure was less anti-conservative than the other two for the left-skewed cases, but remained materially above 5% in this 500-repetition sample. Its performance needs independent validation before deployment.

The larger alternative is defined by construction and should never be picked after seeing the best pilot result. The null-at-margin cases are the calibration priority; method choice based only on power can reward an invalid test.

## Usage

From the repository root:

```sh
cd tools/lighting-study-plan
python3 -m unittest -v test_plan_independent_scene_families \
  test_simulate_power_operating_characteristics test_compare_mean_inference_methods
cd ../..
python3 tools/lighting-study-plan/compare_mean_inference_methods.py \
  --replicates 500 --bootstrap-replicates 199 --alpha 0.05 \
  --seed 20261028 --output /tmp/lighting-inference-methods.json
python3 -m json.tool /tmp/lighting-inference-methods.json >/dev/null
```

Increase both replication counts when refining estimates. The per-scenario Monte Carlo intervals are individual, not simultaneous across the entire method/scenario family.

## Claim ceiling and next steps

- No inference method is currently declared the winner.
- Freeze a separate confirmatory distribution/scenario suite before selecting a method; use the development grid for exploration only.
- Add pilot-SD uncertainty, heteroscedasticity, outcome-dependent timeouts/missingness, and distributions calibrated to real but held-out pilot data.
- For randomized comparisons, use the actual assignment mechanism for randomization inference; do not assume sign-flip exchangeability when the design does not support it.
- Evaluate multiplicity and feasibility separately. A mean-saving test does not establish a true feasibility probability or verify photometric quality.
- Physical lighting, calibrated Radiance measurements, product compliance and safety remain separate evidence gates.

## Methodology reference

Morris, White & Crowther (2019), *Using simulation studies to evaluate statistical methods*, Statistics in Medicine 38:2074–2102, DOI 10.1002/sim.8086. It recommends defining simulation aims, data-generating mechanisms, estimands, methods and performance measures (ADEMP) and reporting Monte Carlo uncertainty. https://pmc.ncbi.nlm.nih.gov/articles/PMC6492164/
