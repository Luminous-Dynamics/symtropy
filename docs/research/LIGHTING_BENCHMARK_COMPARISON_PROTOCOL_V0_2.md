# Lighting benchmark comparison protocol v0.2

**Status:** preregistration template; no benchmark result or superiority claim is included.

## Three claims, three evidence tiers

1. **Process integrity:** a pinned executable ran under a recorded environment and produced complete, hash-bound output.
2. **Predictive/optical validity:** the solver and source model agree with analytical reference cases where valid and with held-out calibrated measurements under a declared uncertainty budget.
3. **Design improvement:** the proposed search policy produces better compliant service-energy outcomes than preregistered baselines on blind scene families under matched compute budgets.

Passing a lower tier never proves the next tier. A parser test is not optical validation; agreement with a simulation reference is not physical calibration; a lower simulated energy score is not measured service-energy savings.

## Baselines and equal budgets

Pre-register at least a conventional engineering layout, seeded random or Sobol search, the strongest tuned non-Symthaea optimizer available, and commercial design workflows such as DIALux evo where the same case can be represented fairly. Pin tool versions, inputs, source photometry, scene settings, scoring code and each method's budget. Charge scene compilation, solver calls, retries, invalid runs and measurement selection to declared budgets. No method receives per-candidate solver-setting tuning unless the same tuning budget is available to every method.

## Avoid benchmark leakage

Split by entire scene families, not grid points or random seeds from the same room: development, validation and a blind holdout set. Keep near-duplicate rooms, luminaire/source families, material/reflectance variants and topology/fault variants within one split. Select solver settings on a separate convergence set. Freeze optimization policies, primary endpoint, analysis methods and exclusion rules before revealing blind results.

## Primary endpoint and hard gates

For designs that pass every required photometric and application-specific gate, compare measured or conservatively bounded AC energy per compliant task-hour. Define the energy measurement boundary and denominator before the experiment. A candidate with a required failed, unknown, unsupported or unmeasured gate cannot be ranked as compliant. Missing measurements and timeouts remain explicit; they do not become zero energy, be imputed as passes, or disappear from the attempt denominator.

Report separately: energy per compliant task-hour, feasible candidate rate, average/minimum illuminance, uniformity, glare, color/spectral metrics, temporal light modulation when applicable, thermal/electrical constraints, service hours, runtime, CPU/GPU use, solver calls, high-fidelity query count and uncertainty. Use a preregistered Pareto or application-specific policy rather than an after-the-fact scalar that hides failed gates.

## Statistical comparison

The independent unit is a scene family; repeated runs and grid points are nested measurements. Use paired comparisons where the experimental design supports them and plan sample size from independent-family pilot variability. Report the point estimate, interval, number of attempted families, paired coverage, failure reasons, feasibility rates and multiplicity across all baselines tried. A 95% interval only has its nominal interpretation if the underlying procedure's assumptions and operating characteristics are supported. The current mean-inference calibration study has not selected a confirmatory procedure; do not suppress that failure.

## Candidate hypotheses—not results

The following are research targets that must be frozen and powered before a blind test; they are not claimed achievements:

- Lower energy per compliant task-hour by at least 15% versus the strongest preregistered feasible baseline, with interval excluding zero improvement.
- At least 2x fewer high-fidelity solver calls to a predeclared quality threshold, without reducing the final feasible Pareto quality.
- No false-feasible candidate in a fixed negative-control suite, with an exact uncertainty bound on the residual false-feasibility rate.
- Calibrated prediction intervals with coverage and interval width reported across held-out scene/source families.

These targets are not universal standards and may be unrealistic for some use cases. They may not be changed after the blind results are visible.

## Physical validation gate

Record luminaire IES/LDT or spectral data and provenance, meter model and calibration record, meter positioning, measurement grid, power meter calibration, warm-up/environment, daylight state, supply/driver behavior and repeats. Validate solver prediction separately from design optimizer value. Product-level photometric/electrical testing, flicker/TLM, glare, color quality, photobiological safety and certification require their own applicable methods; do not infer them from a scalar illuminance grid.

## Claim rule

Only claim superiority when the candidate passes predeclared optical/application gates, all attempted blind families are accounted for, the statistical calibration gate passes, the strongest relevant baseline is beaten under a matched budget, and measured energy evidence supports the outcome. Otherwise report a narrower claim or a no-go. No synthetic or software-contract result establishes a SOTA win.
