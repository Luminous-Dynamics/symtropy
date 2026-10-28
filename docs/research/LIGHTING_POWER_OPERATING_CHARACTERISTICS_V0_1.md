# Lighting operating-characteristic simulation v0.1

**Status:** synthetic method diagnostic; not measured lighting evidence.

## Purpose

The independent scene-family planner estimates a prospective number of new families from a plug-in variance approximation. This follow-on Monte Carlo harness asks a different question: if the confirmatory analysis is a one-sided Student-t test of family-level energy-saving effects against a frozen worthwhile-effect margin, how often does it reject under null and alternative distributions?

The benchmark's inferential unit is one independently held-out scene family. Repeated seeds or illuminance-grid points are not counted as independent confirmatory observations.

## Scenarios

For each planned variance sensitivity, the harness simulates 11 conditions: null effects at the margin under normal, left-skewed lognormal, right-skewed lognormal and standardized Student-t distributions; the declared design alternative under those four distributions; a shifted-to-midpoint alternative; an alternative only one family-effect standard deviation above the margin; and a missingness/failure sensitivity. Failed or non-compliant families remain in feasibility denominators and do not get zero-filled energy values.

Every simulated rejection proportion receives an individual 95% Wilson Monte Carlo interval. The tool flags null scenarios whose interval lies above the nominal per-comparison alpha as screening signals. These are unadjusted per-scenario intervals across multiple simulated scenarios, so they are diagnostic—not simultaneous formal inference.

The harness reports feasibility twice: (a) the observed-proportion gate, and (b) a stricter gate requiring the one-sided 95% Wilson lower confidence bound for candidate and baseline feasibility to meet the declared absolute threshold. The latter prevents a tiny study with 10/10 successful families from being mistaken for evidence that the true feasibility rate is at least 95%.

## Local run

```sh
cd tools/lighting-study-plan
python3 -m unittest -v test_plan_independent_scene_families test_simulate_power_operating_characteristics
python3 simulate_power_operating_characteristics.py fixtures/synthetic-plan-output-example.json \
  --replicates 3000 --seed 20261028 \
  --output /tmp/lighting-operating-characteristics.json
python3 -m json.tool /tmp/lighting-operating-characteristics.json >/dev/null
```

The code uses only the Python standard library. Fixed inputs and seed yield deterministic JSON output. The report includes planner and simulator-source digests. The report is always marked `synthetic_simulation_diagnostics_no_empirical_claim`.

## Synthetic screening finding

With the frozen synthetic planner fixture (3 planned comparison baselines, per-comparison one-sided alpha 0.0166667, and 10 families per sensitivity), 3,000 repetitions per scenario produced left-skewed-null rejection rates around 9.5–10.1%, with individual 95% Wilson intervals entirely above the nominal alpha. The normal-null rejection rate was about 1.7–2.0%, with intervals that included the nominal alpha. This is a constructed statistical counterexample: the ordinary one-sample Student-t test can be anti-conservative under a strongly left-skewed family-effect distribution at small n. It is not evidence that real lighting data have this distribution or error rate.

The same demonstration gives approximately 100% power at its very large declared 18% alternative versus a 5% margin, while the alternative just one SD above the margin is only about 72–76% in the example. Such saturation is why the design alternative must be set by a predeclared worthwhile effect rather than an appealing pilot estimate.

With 10 planned families and illustrative baseline/candidate failure probabilities of 2%/5%, the observed candidate feasibility point gate passes about 60% of simulations, and the one-sided 95% lower-bound feasibility gate passes 0%. The latter is expected: 10 observations are insufficient to establish a true feasibility rate of at least 95%, even if all 10 succeed. Feasibility therefore needs a separate prospective sample-size/acceptance plan, not just the energy-power result.

All figures above are outputs from parametric synthetic fixtures. They do not describe real buildings, optical solver accuracy, product compliance, physical safety or energy savings.

## Limits and next steps

- The simulator currently tests the specified Student-t decision rule; it does not compare it against robust alternatives such as studentized permutation tests, trimmed-mean procedures or prespecified bootstrap tests.
- Family-level pilot SD is treated as fixed within each declared sensitivity; uncertainty in the pilot variance estimate is not sampled yet.
- Missingness is MCAR by construction and independent of the effect. Outcome-dependent timeout/failure mechanisms need separate scenarios.
- The scenario-level Monte Carlo intervals are individual, not simultaneous. A formal calibration study should preregister a family-wise decision rule for the simulation diagnostics.
- Before an empirical lighting claim, validate on held-out building/task families and compare the frozen methods under equal solver-call, tuning, compute and measurement budgets; keep DIALux practitioner design, Radiance optical reference, optimizer search and physical metrology in separate lanes.

## Method references

- Morris, White & Crowther (2019), *Using simulation studies to evaluate statistical methods*, Statistics in Medicine. Their ADEMP structure (aims, data-generating mechanisms, estimands, methods and performance measures) motivates explicit scenario definitions and operating-characteristic outputs. https://doi.org/10.1002/sim.8086
- BMJ (2025), *Determining sample size for pilot trials: a tutorial*. Pilot point estimates are weak bases for assumed treatment effects; design alternatives should be informed by a worthwhile difference, with uncertainty in variability recognized. https://www.bmj.com/content/390/bmj-2024-083405
- Campbell et al. (2023), *Key considerations for designing, conducting and analysing a cluster randomised trial*. The design and analysis must treat clusters as sampling units and account for clustering. https://academic.oup.com/ije/article/52/5/1648/7172835
