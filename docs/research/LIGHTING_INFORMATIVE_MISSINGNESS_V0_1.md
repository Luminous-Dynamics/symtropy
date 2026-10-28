# Lighting informative-missingness sensitivity benchmark v0.1

**Status:** synthetic statistical-method diagnostic. It does not measure lighting-system performance.

## Why this work exists

The complete-case inference in the preceding stress matrix assumed that the observed scene families represented all attempted families. That assumption is unsafe when a candidate's run is more likely to time out, fail parsing or return a usable measurement in particular scene conditions. A study can then appear to have higher savings simply because difficult or poor-performing cases are absent.

Simulation-study guidance recommends defining data-generating mechanisms, estimands, methods and performance measures before comparing statistical procedures (ADEMP). Clustered designs also require inference on independent units and sample-size calculations that reflect the number of clusters, not nested observations. See references at the end.

## Estimand and unit

The target is the arithmetic mean latent energy-saving percentage across all independently sampled scene families attempted in a comparison, relative to a fixed superiority margin. One scene family is one independent unit; repeated seeds or grid points are not independent families.

The code models a technical *non-observation* of an otherwise defined latent energy-effect value. This is distinct from a physically non-compliant design or a service failure. A true non-compliance is an outcome and must remain in the feasibility endpoint, never be encoded as a missing energy value.

## Methods compared

### A. Naive complete-case Student-t lower bound

Computes a one-sided Student-t lower bound using only observed family effects. It is an intentionally familiar failure-mode reference. Under MCAR, the observed values are a random subset; under MAR/MNAR, the observed-case mean can differ systematically from the all-family mean. It must not be interpreted as an all-family mean bound without additional assumptions or adjustment.

### B. Bounded worst-case completion + Hoeffding lower bound

Each unobserved family effect is replaced by a preregistered lower plausibility bound \(L\). All \(n\) completed values are then analyzed with a one-sided Hoeffding lower bound for the mean of independent values in \([L,U]\):

\[
\bar Z - (U-L)\sqrt{\frac{\log(1/\alpha)}{2n}}.
\]

If every unobserved latent effect really lies in \([L,U]\), the completion value satisfies \(Z_i \leq X_i\) pointwise. Therefore, a lower confidence bound for \(E[Z]\) is also a lower bound for \(E[X]\). This does not require missingness to be MCAR, MAR, or MNAR. It *does* require:
- independent scene-family units;
- valid, frozen finite outcome bounds;
- observed effects that are measured without unaccounted error;
- a missing energy outcome representing an otherwise well-defined latent energy outcome.

It is intentionally conservative. It may be unable to establish superiority even when the true simulated mean is above the margin. It is a sensitivity guard, not a universal replacement for efficient study design.

## Scenario design

The standard run covers 96 scenarios: 10, 20, or 40 independent scene families × four bounded effect distributions × four missingness mechanisms × null/target effect locations, with 2,500 repetitions per scenario.

- **Effect distributions:** clipped normal-like noise; right-skewed two-point noise; left-skewed two-point noise; rare extreme outcomes. Each family effect is checked against the declared bounds.
- **Complete:** no missing outcomes.
- **MCAR:** each effect has a constant 30% probability of non-observation.
- **MAR difficulty:** difficulty is observed for each attempted family and influences the latent effect; higher difficulty also raises non-observation probability. Complete-case analysis ignores this known covariate.
- **MNAR low outcome:** non-observation depends directly on the latent effect—65% probability below the margin and 5% at/above it.
- **Null:** population effect location equals the frozen margin.
- **Target:** effect location is two family-level SDs above the margin.

Every rejection rate includes an individual 95% Wilson Monte Carlo interval. Intervals are not simultaneous across the scenario family. Outputs include source digests, seed, scenario definitions and an explicit synthetic-only claim ceiling.

## Local execution

From the repository root:

\`\`\`sh
cd tools/lighting-study-plan
python3 -m unittest -v test_mnar_sensitivity_audit
cd ../..
python3 tools/lighting-study-plan/mnar_sensitivity_audit.py \\
  --replicates 2500 --seed 20261028 --sample-sizes 10,20,40 \\
  --output /tmp/lighting-mnar-sensitivity.json
python3 -m json.tool /tmp/lighting-mnar-sensitivity.json >/dev/null
\`\`\`

Higher sample sizes can be tested separately, for example \`--sample-sizes 80,160 --replicates 500\`, to show how the conservative bound changes with sample size. The CLI is standard-library-only and does not require Radiance.

## Local synthetic findings

### Main 96-scenario run

- 2,500 repetitions per scenario; 48 null scenarios and 48 target-effect scenarios.
- Maximum naive complete-case Student-t null rejection rate: **68.56%** across the declared grid.
- **26 of 48** naive null scenarios had an individual 95% Monte Carlo interval entirely above the 5% nominal alpha.
- The bounded worst-case method had zero observed null rejections, and no null-scenario interval was entirely above alpha.
- With 20 families and clipped-normal-like effects, complete-case rejection was 4.96% under complete data, 5.12% under MCAR, 8.68% under MAR difficulty, and **40.32% under MNAR low-outcome missingness**. In the MNAR scenario, 35.13% of outcomes were missing and the mean observed-case effect was 6.15% despite a true null location of 5%.

These are constructed distributions. The inflation demonstrates a method failure mode, not an estimate of how often lighting runs fail in practice.

### Power trade-off

With the deliberately wide declared effect interval [-15%, 25%], the Hoeffding lower bound had **zero observed rejections in all target scenarios up to 40 families**. In a separate 500-repetition scale run it averaged about 93.1% target rejection at 80 families and 100% at 160 families when no outcomes were missing. With MCAR/MAR 30%-ish missingness it remained at zero, because assigning a wide lower bound to many missing values leaves the all-family lower bound below the margin. Under the MNAR mechanism it averaged about 8.2% rejection at 80 and 72.5% at 160 families across the four effect distributions.

This is the central trade-off: **valid but uninformative bounds do not constitute a complete solution**. Outcome bounds need external justification, and a useful workflow should add a separate covariate-adjusted MAR analysis (where pre-outcome covariates and missingness model are defensible) plus tipping-point sensitivity for departures from MAR. The MNAR worst-case analysis remains the guardrail when assumptions are not credible; it must not be relaxed merely to make a result pass.

## Claim boundary

- No test outcome here says Symtropy beats any optimizer, DIALux workflow or optical solver.
- The sensitivity bounds are caller-declared assumptions, not measured physical bounds.
- The simulation does not validate photometric accuracy, field savings, product compliance or safety.
- Method selection requires a separately frozen confirmation scenario set and must account for multiplicity across the chosen comparisons.
- A physical non-compliance or quality failure is a separate endpoint, not an imputed energy effect.

## References

- Morris, White & Crowther (2019), *Using simulation studies to evaluate statistical methods*, Statistics in Medicine. ADEMP provides a structure for declaring data-generating mechanisms, estimands, methods and performance measures. https://pmc.ncbi.nlm.nih.gov/articles/PMC6492164/
- Campbell et al. (2023), *Key considerations for designing, conducting and analysing a cluster randomised trial*, International Journal of Epidemiology. Includes the requirement to account for clusters in both design and analysis. https://academic.oup.com/ije/article/52/5/1648/7172835
- Little & Rubin, *Statistical Analysis with Missing Data*, 3rd ed. Missingness assumptions are part of the identification problem, not something that can be solved by dropping incomplete observations.
