# Lighting power and scene-family planning v0.1

**Status:** research protocol / implementation draft. No physical result or SOTA claim.

## Research question

How many *new, independent* building/room/task scene families are required to detect a declared reduction in energy per compliant task-hour without mistaking seeds, grid points or repeated variants of one room for independent evidence?

## Paradigm changes

1. **Scene families are the unit of generalization.** Multiple seeds test stochastic optimizer/solver behavior within a family; they do not replace independent buildings, room geometries, daylight conditions or task classes.
2. **Plan from a declared worthwhile effect.** The assumed alternative and superiority margin are explicit CLI inputs. Do not set the target to the largest effect discovered after running many optimizers.
3. **Budget for uncertainty, not just the point estimate.** Report sensitivity at 1×, 1.5× and 2× the pilot between-family standard deviation. A single sample-size number is not robust to misspecified variance.
4. **Failures remain outcomes.** Timeouts, invalid outputs, infeasibility and unknown status stay in the denominators. Conditional energy savings do not compensate for worse feasibility or service quality.
5. **Keep pilot and confirmatory families separate.** Reusing the pilot as a blind test set overstates generalization. Freeze the held-out family IDs before candidate tuning.

## Implemented gates

- Unique scene-family × seed rows, balanced seed schedules, legal status values and status/energy consistency.
- Minimum pilot-family count, overall paired-energy coverage, per-family paired coverage, at least one paired result per family, and baseline/candidate feasibility-rate gates.
- Family-level median of paired relative energy savings; sample-size calculation counts independent scene families, not seeds.
- Declared one-sided alpha, target power, superiority margin, assumed true effect and maximum family count.
- Input CSV and source-code SHA-256 digests; deterministic JSON output; synthetic-tier claim suppression.
- Unresolved outcomes for low pilot quality, effect at/below margin, zero/unavailable family variance or scenarios that exceed the configured maximum.

## Planning approximation and remaining work

The utility uses a normal-effect plug-in-SD approximation with the Student-t critical value calculated from the Student-t CDF. It treats the pilot standard deviation as known and does not calculate exact noncentral-t power. This is an inexpensive planning screen, not a guarantee. Next statistical increments should add simulation-based operating-characteristic checks under heavy tails, skew, clustered missingness and explicit family-distribution shifts, plus a separate family-level feasibility design and multi-endpoint multiplicity policy.

Do not interpret the pilot's observed energy savings as causal savings unless design assignment, solver configuration, equal compute/tuning budgets, data provenance and any required physical measurement process support that interpretation. A high-fidelity reference run or optimizer oracle is not an industry baseline by itself.

## Industry comparison protocol

Use parallel, separately reported benchmark lanes instead of a single blended lighting score:

- **Design/optical baseline:** practitioner-designed cases and a current commercial workflow such as DIALux evo, with exact version, inputs, luminaire catalog, defaults, operator time and manual tuning budget recorded. Separately compare a reproducible Radiance reference workflow; solver/reference accuracy and layout-optimizer quality are distinct questions.
- **Optimizer baselines:** fixed templates, random search, space-filling/DOE, a strong multi-objective optimizer (e.g. NSGA-II), and cost-aware Bayesian/multi-fidelity optimization. Match expensive-solver calls, wall-clock, hardware, candidate evaluations, tuning and information available at decision time.
- **Lighting service:** required task-specific illuminance/uniformity, energy per compliant task-hour, spatial distribution and task coverage; glare/discomfort and quality constraints remain hard gates, not terms that can be traded away for energy.
- **Spectrum and temporal behavior:** spectral quantities and temporal-light-modulation metrics should use their own traceable input/measurement methods. DLC SSL V6.0/LUNA V2.0 adds efficacy, quality and controllability criteria. The 2026 IEA 4E TLM interlaboratory report documents disagreement between labs and uncertainty-budget issues as real qualification risks.
- **Robustness:** freeze scenario-family train/pilot/confirmatory partitions; hold out complete room/building/task families. Include reflectance, geometry, mounting height, daylight, occupancy, occlusion, sensor bias, control faults and outages. Report worst-family/tail results alongside aggregate results.
- **Evidence tier:** synthetic contract → simulation-only → calibrated simulation → physical prototype → independently tested product. Never pool tiers into one evidence claim.

## Proposed targets (hypotheses, not current results)

- At least **15% lower energy per compliant task-hour** versus each named, preregistered baseline under equal high-fidelity evaluation/tuning budgets, with an adjusted interval that clears the predeclared superiority margin.
- At least **2× fewer high-fidelity solves** to reach the same preregistered service/quality gates, counting failures and all tuning calls.
- No worse than **5 percentage points** candidate feasibility regression relative to each baseline, plus an independent absolute feasibility gate.
- Report family-level effect distributions, interval coverage/calibration where predictions are used, Pareto hypervolume/regret for multi-objective search, p50/p95/p99 runtime, wall-clock, memory, solver calls, failure/timeout rates, and all negative controls.
- Zero false-compliant results on the declared finite negative-control suite; then separately establish calibrated physical accuracy with traceable workplane measurements and uncertainty budgets.

These are proposed acceptance hypotheses, not standards thresholds and not measured claims. Revisit their feasibility with pilot variance and a frozen budget before confirmatory data collection.

## Current external anchors

- DesignLights Consortium, *Technical Requirements for LED Lighting: SSL V6.0 and LUNA V2.0*, effective 5 January 2026: https://designlights.org/our-work/solid-state-lighting/ssl-v6-luna-v2/
- IEA 4E SSL Annex, *Measurement of Temporal Light Modulation Interlaboratory Comparison 2023 (IC 2023) Final Report*, 9 June 2026: https://www.iea-4e.org/ssl/publications/measurement-of-temporal-light-modulation-interlaboratory-comparison-2023-ic-2023-final-report/
- Michael A. Lones, *Avoiding common machine learning pitfalls*, Patterns 5 (2024), 101046, including multiple comparisons and benchmark leakage: https://doi.org/10.1016/j.patter.2024.101046
- Radiance \`rtrace\` manual, for explicit boolean switch semantics and output-field selection: https://radsite.lbl.gov/radiance/man_html/rtrace.1.html
- DIALux evo 14 Patch 6 release entry (19 October 2026): https://www.dialux.com/en-GB/download?cmsPreview=1

## Claim ceiling

A clean plan only helps design a credible future experiment. It does not prove that Symthaea/Symtropy beats DIALux, Radiance, any optimizer, or industry. No real measured dataset is included in the synthetic fixture. Physical measurement, product quality, code compliance and safety claims require their own methods, evidence and qualified review.
