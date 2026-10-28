# Lighting study planner v0.1

Pilot-based planning for the number of **independent scene families** needed by a confirmatory lighting comparison. This is a research-design utility, not a photometric solver, optimizer, certification tool, or confirmatory effect analyzer.

## Why this exists

A scene with 10 seeds is not 10 independent building types. Seeds and grid points are nested within a scene family; treating them as independent can make uncertainty look much smaller than it is. This planner compresses each family's repeated paired results to a within-family median energy-saving effect, then estimates the number of **new, held-out scene families** needed for a declared energy-superiority margin. Pilot families must not be reused as confirmatory test families.

The tool preserves failed and non-compliant attempts in coverage and feasibility denominators. Missing or non-rankable energy never becomes zero. A pilot with inadequate paired coverage, too few independent families, low feasibility, or unestimable variance returns an unresolved status rather than fabricating a sample-size recommendation.

## Run

From the repository root:

\`\`\`sh
cd tools/lighting-study-plan
python3 -m unittest -v test_plan_independent_scene_families
cd ../..
python3 tools/lighting-study-plan/plan_independent_scene_families.py \\
  tools/lighting-study-plan/fixtures/synthetic-pilot.csv \\
  --evidence-tier synthetic_contract \\
  --assumed-true-effect-pct 18 \\
  --superiority-margin-pct 5 \\
  --target-power 0.80 --alpha 0.05 --comparison-family-size 3 \\
  --sd-multipliers 1,1.5,2 \\
  --output /tmp/lighting-study-plan.json
python3 -m json.tool /tmp/lighting-study-plan.json >/dev/null
\`\`\`

The unit-test command deliberately runs from the script directory: Python's unittest discovery may reject a directory with a hyphen in its name as an importable top-level package. The GitHub Actions workflow uses the same tested command.

The assumed effect is a **predeclared design alternative**, not the most flattering pilot estimate. `--alpha` is the one-sided family-wise error budget across all planned superiority comparisons; `--comparison-family-size` applies a Bonferroni adjustment to each comparison. The sensitivity multipliers show what happens if family-level variability is 1×, 1.5×, or 2× the pilot standard deviation.

## Input contract

CSV columns:

- \`scene_family\`, \`seed\`: unique pair ID; every scene family must use the same seed schedule.
- \`baseline_status\`, \`candidate_status\`: one of \`compliant\`, \`non_compliant\`, \`unknown\`, \`unsupported\`, \`not_evaluated\`, \`timeout\`, \`invalid_output\`, \`invalid_input\`, \`over_budget\`.
- \`baseline_energy_wh_per_compliant_task_hour\`, \`candidate_energy_wh_per_compliant_task_hour\`: finite positive values only when the corresponding status is \`compliant\`; otherwise the cell must be blank.

The input digest and planner-source digest are included in the JSON report. The caller-supplied evidence tier is not authenticated by the program. Synthetic data always produce \`synthetic_planning_demo_no_empirical_claim\`.

## Interpretation limits

The planner uses a transparent normal-effect, plug-in-SD approximation. It numerically inverts the one-sided Student-t CDF for a critical value, but treats pilot standard deviation as known and does not calculate exact noncentral-t power. Variance-estimation uncertainty, heavy-tailed family effects, clustered missingness, and shifts to unseen building populations are not fully modeled. Do not present its result as an exact guarantee. Preregister the test, use independent holdout families, and verify operating characteristics by simulation under plausible skew, tails, heterogeneity, and scene-family shift before a confirmatory study.

This plans only the conditional energy-superiority endpoint. Feasibility, light quality, glare, temporal light modulation, service continuity, and other safety/quality endpoints need their own prospective designs and gates. Passing this planner cannot show superiority or certify a lighting design.


## Simulation-based operating-characteristic audit

To check whether the declared sample-size recommendations behave under different distributions, run:

```sh
python3 -m unittest -v test_plan_independent_scene_families test_simulate_power_operating_characteristics
python3 simulate_power_operating_characteristics.py fixtures/synthetic-plan-output-example.json \
  --replicates 3000 --seed 20261028 --output /tmp/lighting-power-diagnostics.json
python3 -m json.tool /tmp/lighting-power-diagnostics.json >/dev/null
```

The simulator uses 33 scenarios across three pilot-SD sensitivity settings, including nulls under normal, left/right-skewed lognormal and Student-t effects, the declared alternative, an alternative near the worthwhile-effect margin, a shifted-family alternative, and failure/feasibility sensitivity. It reports per-scenario Monte Carlo Wilson intervals and hashes the planner/simulator inputs. Missing/non-compliant families are excluded from conditional energy effect calculations but remain in feasibility denominators, never zero-filled.

It separately reports point-estimate feasibility gates and one-sided Wilson lower-bound feasibility gates. The latter asks for evidence that the true feasible fraction meets the specified threshold. Ten out of ten pass events cannot establish a 95% true pass rate at 95% confidence.

On the included synthetic planner output, the small-sample Student-t test shows an anti-conservative left-skew counterexample: approximately 9.5–10.1% null rejection versus one-sided per-comparison alpha 1.667%, in the specified standardized lognormal family. This is a statistical stress-test finding on constructed data, not a statement about real buildings. See [the operating-characteristic protocol](../../docs/research/LIGHTING_POWER_OPERATING_CHARACTERISTICS_V0_1.md) for full assumptions and limits.


## Mean-inference method stress benchmark

This exploratory harness compares the one-sample Student-t, percentile-bootstrap, and studentized-bootstrap-t **lower confidence bounds for the arithmetic mean** across independent scene families. It exercises 24 fixed combinations of 10/20 families, normal/left-skew/right-skew/heavy-tail effects, and null/near-margin/target alternatives. Each rejection-rate and lower-bound-coverage fraction has an individual 95% Wilson Monte Carlo interval.

```sh
python3 -m unittest -v test_plan_independent_scene_families \
  test_simulate_power_operating_characteristics test_compare_mean_inference_methods
python3 compare_mean_inference_methods.py \
  --replicates 500 --bootstrap-replicates 199 --alpha 0.05 \
  --seed 20261028 --output /tmp/mean-inference-stress.json
python3 -m json.tool /tmp/mean-inference-stress.json >/dev/null
```

The local prototype exposed skew sensitivity in all three procedures: at 10 independent families, null rejection in the left-skewed case was 15.0% for Student-t, 18.8% for percentile bootstrap, and 9.4% for studentized bootstrap-t in 500 repetitions. These are synthetic stress results, not an empirical finding about building outcomes. The bootstrap-t result is less anti-conservative in that example but still not qualified as a reliable method. The harness deliberately does not select a winner from its development grid. See [mean-inference method protocol](../../docs/research/LIGHTING_MEAN_INFERENCE_METHOD_STRESS_V0_1.md).

A separate exact sign-test helper targets the median and is not included in the mean-estimand rankings. Any method-selection claim requires a separately frozen confirmation suite plus pilot-variance, missingness and family-shift sensitivity analyses.
