# Natural-hazard benchmark protocol v0.1

Status: **protocol proposal; no physical-model result is implied**.

Parent work: [NATURAL-HAZARDS-000](https://github.com/Luminous-Dynamics/symtropy/issues/2232), [HAZARD-BENCH-001](https://github.com/Luminous-Dynamics/symtropy/issues/2234), [HAZARD-IMPACT-VAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/2264), [HAZARD-PILOT-JHB-001](https://github.com/Luminous-Dynamics/symtropy/issues/2267), and [qualification contract PR #2236](https://github.com/Luminous-Dynamics/symtropy/pull/2236).

This document defines how a benchmark is specified and how much a passing result is allowed to claim. It is not a benchmark dataset, a solver implementation, an operational forecast, or evidence that any hazard model currently passes.

## 1. Design shift: benchmark claims, not screenshots

Symtropy should be a **federation of specialist physical models with a common evidence, uncertainty, provenance, and decision interface**. It should not replace domain physics with one universal solver or compress every hazard into a single realism score.

The evaluation target is a chain of independent claims:

`forcing -> physical hazard -> exposure -> vulnerability/fragility -> impact distribution -> candidate action -> observed outcome`

Each arrow needs its own declared inputs, validity envelope, evidence and failure states. A passing upstream stage does not automatically pass a downstream stage. Spatial overlap, timing correlation, or a plausible animation is not proof of causal transfer. Missing inputs mean `unavailable` or an explicit qualified failure, never zero impact.

The intended advantage over fragmented tooling is compositional: reproduce a specialist model's results where it is the appropriate reference, make model-to-model coupling auditable, and test whether the combined chain improves decisions at a disclosed resource cost.

## 2. Benchmark case contract

Each case must have a stable ID and a versioned machine-readable manifest containing at least:

- **Question and target:** the exact claim under test; target quantity; units; event definition; required output fields; spatial support; valid-time interval; lead time; aggregation rule.
- **Regime and validity:** terrain/geometry, physical regime, boundary conditions, temporal resolution, spatial resolution, solver assumptions and declared out-of-domain boundaries.
- **Inputs:** source/version/license, retrieval or archive date, checksums, transformations, coordinate reference system, units, temporal alignment, missingness, and quality limitations.
- **Reference:** reference dataset or model version, reference uncertainty, known limitations, and why it is suitable for this specific target. Reference is not automatically ground truth.
- **Split:** event IDs and a leakage-resistant calibration/development/held-out split. Closely related time steps from the same event must not be randomly distributed across train and test.
- **Execution:** source commit, build/toolchain, configuration digest, dependencies, hardware, precision, deterministic seed where relevant, runtime, memory, accelerator/CPU selection and output artifact digests.
- **Scoring:** metric definitions, units, baseline, masks, thresholds, aggregation weights fixed before testing, sample count, missing-observation treatment and uncertainty method.
- **Claim ceiling:** the strongest claim this evidence class can support, along with explicit claims it cannot support.

Case IDs and manifest versions are immutable after the held-out set is opened. Fixing an error requires a new manifest version and a record of whether the change was made after seeing the held-out result. Preserve failed runs and negative controls in the ledger.

## 3. Evidence ladder (no automatic promotion)

| Tier | Evidence | Minimum gate | Permitted claim |
| --- | --- | --- | --- |
| E0 | Provenance and execution | Input/source digests, configuration, environment and output receipt are complete; run can be replayed or differences are explained | Reproducible execution only |
| E1 | Structural invariants | Predeclared conservation, bounds, dimensional, topology and state-transition checks pass within documented numerical tolerance; adversarial negative controls fail closed | The tested invariant holds for the tested cases |
| E2 | Analytical/manufactured solution | Error and convergence are measured against a solution with known answer; expected convergence is checked over at least three meaningful refinements where the numerical method supports it | Numerical behavior for that analytic regime |
| E3 | Published or independent reference | Paired comparison uses matching geometry, forcing, boundaries, units, observation operator, and time window; discrepancies and reference uncertainty are reported | Agreement with the named reference under declared conditions |
| E4 | Held-out observation | Event/region/time split was frozen; comparison includes uncertainty, calibration and a relevant naive/reference baseline | Empirical skill for the declared held-out population |
| E5 | Decision/impact evaluation | Upstream predictions and the downstream impact/action model are evaluated separately; cost/loss or utility assumptions are explicit; interventions and observation bias are addressed | Decision value for the named context and assumptions |
| E6 | Operational evaluation | Prospective monitoring, data latency, failure handling, availability, human factors and responsible-authority workflow are measured in the intended deployment context | Only the operational scope actually evaluated |

A higher tier requires the relevant lower-tier evidence but does not replace it. An E1 invariant pass must never be presented as forecast accuracy; E3 reference agreement is not automatically E4 observational skill; and no synthetic-only run supports E5 or E6.

## 4. Fair baselines and state-of-the-art comparison

Every result must compare against baselines chosen before opening the holdout. Use the lowest-complexity baseline that is meaningful, then a domain-appropriate specialist reference:

1. **Null / persistence baseline:** no event, climatology, persistence, or another declared simple baseline appropriate to the target.
2. **Established specialist reference:** e.g. a named hydrodynamic, seismic hazard/risk, fire-atmosphere, atmospheric-dispersion, or planetary-entry reference appropriate to that phenomenon.
3. **Best applicable operational or published baseline:** only when its forecast initialization, data access, valid time, target definition and evaluation protocol can be matched or the mismatch is clearly reported.
4. **Symtropy composition ablation:** run the same case with and without the coupling, new parameterization, active-learning choice, or additional data being claimed as useful.

Fairness requirements:

- Same case, input information cutoff, target/observation mask, valid time, and metric implementation for paired comparisons.
- Report any difference in grid, time step, ensemble size, hardware, numerical precision, and available compute. Do not describe a compute-unmatched result as an unqualified efficiency win.
- Include a cost-quality frontier (error/skill, runtime, peak memory, energy when measured, and data/HPC cost), not just the fastest or most accurate point.
- Report both absolute metrics and paired differences from the baseline. Use event-clustered bootstrap or another justified dependence-aware interval; publish sample size and interval. Do not treat adjacent pixels or time steps as independent events.
- Preserve the strongest applicable external baseline even when it outperforms Symtropy. A negative result is a benchmark result, not a reason to change the held-out set.
- If no defensible SOTA reference or licensing-compatible observation exists, say so and qualify against analytical/reference evidence only; do not claim SOTA superiority.

For operational weather comparison, use homogeneous verification, suitable simple guidance baselines and enough cases to characterize natural variation. NOAA's National Hurricane Center documents both homogeneous baseline comparisons and substantial annual variability; a small or cherry-picked sample is not a reliable improvement claim: https://www.nhc.noaa.gov/verification/ .

## 5. Metric families

Choose metrics for the target and retain the complete metric profile. Do not rank unlike hazards by one unqualified scalar.

- **Conservation / numerical integrity:** mass, water, energy or species budget residual; sign/bounds; positivity; dimensional checks; stability; resolution/timestep convergence. State the scale against which residuals are normalized and why the tolerance is physically justified.
- **Spatial fields and footprints:** MAE/RMSE and bias alongside suitable spatial metrics; intersection-over-union or critical success index at predeclared thresholds; boundary/extent distance where meaningful; sensitivity to resolution and observation operator. Pixel overlap alone can reward overly smooth or oversized predictions.
- **Timing and trajectories:** arrival/onset-time error, track/position error, along/cross-track error, duration error and phase error as appropriate; score censored and missing events explicitly.
- **Probabilistic events:** Brier score and reliability/calibration; probability of detection, false-alarm ratio and misses at fixed decision thresholds; include uncertainty and base rate.
- **Continuous distributions / ensembles:** CRPS or another proper scoring rule, interval coverage, interval width/sharpness and calibration diagnostics. Coverage alone is not enough: arbitrarily broad intervals can cover observations while being uninformative.
- **Rare extremes:** event-stratified scores and uncertainty intervals; disclose denominator counts and test missed extremes separately. Do not let many easy non-events hide severe misses.
- **Impact and service disruption:** define the target (e.g. assets inundated above depth, expected service interruption-hours, smoke exposure-hours) and separate physical hazard error from exposure completeness, fragility assumptions, adaptation and reporting bias.
- **Decision value:** predeclared cost-loss, regret, utility or value-of-information comparison for a named decision-maker and specified costs. Report the assumptions, sensitivity and cases where the preferred action changes.
- **Systems performance:** wall time, peak memory, hardware/software, energy if instrumented, setup/data ingestion costs, throughput, failure rate and quality versus resource budget.

Use the Rust score primitives in `symtropy-natural-hazards` as implementation components, not as a declaration that the chosen case or model is valid. Independently test score code with hand-calculated examples, edge cases and property-based or metamorphic tests before trusting comparative reports.

## 6. Required adversarial and negative controls

Every benchmark pack should include negative controls appropriate to its claim. Required cross-cutting controls:

- Remove a required input or boundary condition: result cannot silently become a zero-effect forecast.
- Supply stale, incompatible or invalid units/coordinate reference; check explicit rejection or a traceable declared conversion.
- Change the hazard/event kind, source field, field digest, time lag, or causal-edge evidence; the downstream transfer must be rejected unless the changed case independently satisfies the contract.
- Perturb resolution and timestep; measure convergence/stability instead of selecting the prettiest output.
- Shift the event outside the declared validity envelope; qualification must be revoked or result marked out-of-domain.
- Break or alter one required causal prerequisite; no cascade is admitted simply because its source and target overlap in space or time.
- Introduce missing/censored observations, reporting gaps and changed exposure vintage; do not recode unknown as none.
- Compare a deliberately weak model, persistence and shuffled/misaligned inputs; the score should expose failure rather than report impressive-looking skill.
- Re-run a fixed seed and re-run across supported environments; verify determinism where promised and characterize numerical tolerance where bitwise identity is not promised.

Keep negative-control results visible in the report. A failed control blocks the relevant claim; it must not be averaged away by successful cases.

## 7. Data splitting, uncertainty and statistical discipline

1. Freeze the metric, baseline, threshold policy, holdout membership and stopping rules before examining the holdout.
2. Split by independent event, region or period as the scientific question requires. Spatial neighboring cells and adjacent time samples from one event are correlated and are not independent holdout units.
3. Maintain a separate tuning/calibration set for probability calibration and hyperparameter selection. Do not calibrate on the final evaluation set.
4. Report sample counts at event level and observation level; quantify uncertainty in both the score and the paired difference from each baseline.
5. Report domain shift and subgroup performance (for example terrain, severity, lead time, day/night, urban/rural or data-completeness strata when relevant).
6. For multi-model, multi-metric studies, predeclare the primary comparison or control multiplicity; publish all predeclared metrics rather than only winners.
7. Do not turn missing reference data, a failed solver run or an unsupported regime into a zero error. Label the result unavailable, failed or excluded with a reason and keep it in the accounting.

## 8. Qualification gates and proposed advancement targets

These are **program gates to adopt and test**, not claims that any current model meets them:

- **G0 — Traceability:** 100% of scored runs have versioned case/config/input/output identities, source attribution and stated claim ceilings.
- **G1 — Negative controls:** 100% of required contract-level negative controls are rejected or explicitly unavailable as expected. Any false admission blocks that claim.
- **G2 — Numerical:** all declared budget/invariant checks meet scientifically justified tolerances; convergence results are published rather than summarized as a single pass.
- **G3 — Reference:** every claimed physical capability has a named, compatible reference and published paired error profile. No named reference means no reference-superiority claim.
- **G4 — Held-out skill:** report score and dependence-aware uncertainty against the declared simple and specialist baselines. Promote a superiority claim only when the predeclared paired comparison supports improvement and no critical subgroup regresses beyond its guardrail.
- **G5 — Calibration:** report calibration/reliability and nominal interval coverage with uncertainty; a model cannot pass solely on sharpness or point error.
- **G6 — Coupling:** compare the coupled model against independent component runs and a no-coupling ablation; quantify added skill, compute cost and cascade false positives/negatives.
- **G7 — Decision value:** demonstrate value under prespecified cost/loss assumptions and sensitivity analysis on held-out cases before claiming better decisions.
- **G8 — Efficiency:** show Pareto improvement or clearly state the accuracy/resource trade. No universal percentage improvement is stipulated until a baseline, regime and acceptable risk are specified.

Advancement is **claim-specific**. One hazard, one regime and one output can be qualified while adjacent regimes remain unqualified. Do not average a critical failure into an overall pass. For life-safety-related claims, benchmark results are supporting evidence, not autonomous authorization for public warnings, evacuation or infrastructure control.

## 9. First execution order

Prefer a small complete vertical slice over a broad but untested catalogue:

1. Run existing synthetic/analytical hydrology controls for a bounded flood case; verify budget, domain, resolution/timestep behavior and fail-closed conditions.
2. Select one historical event only after confirming rainfall/forcing, terrain, boundary, observation, timestamp, CRS, license and reuse compatibility. Produce a source manifest and split before calibration.
3. Compare against a named specialist model or published case under matched inputs. If the inputs cannot be matched, document the mismatch and do not label the result an SOTA comparison.
4. Evaluate hazard field first; then separately add exposure, fragility and impact quantities where suitable evidence exists.
5. Add one cascade edge only if source output, target prerequisite, causal evidence, unit mapping and lag have been qualified. Ablate the coupling.
6. Add a cost-aware experiment-selection run only after the uncertainty is decision-relevant; compare against random-feasible, cheapest-first, entropy-only, maximum-disagreement and expert/rule-based policies under the same acquisition budget.
7. Freeze the report with digests, exact command/configuration, baseline, metrics, uncertainty, failure inventory and an explicit claim ceiling.

The Johannesburg pilot may eventually use flood, heat, drought, storm and fire datasets, but it must not proceed to a real-event score until dataset rights, observation coverage and compatible temporal/spatial definitions are verified. A missing data license is a blocker, not an invitation to scrape around it.

## 10. External reference anchors

- WMO, [Early Warnings for All in Focus: Hazard Monitoring and Forecasting](https://wmo.int/resources/publication-series/global-status-of-multi-hazard-early-warning-systems/early-warnings-all-focus-hazard-monitoring-and-forecasting).
- WMO, [Global Status of Multi-Hazard Early Warning Systems 2025](https://wmo.int/resources/publication-series/global-status-of-multi-hazard-early-warning-systems/global-status-of-multi-hazard-early-warning-systems-2025).
- NOAA/NHC, [forecast verification methods, homogeneous baselines and historical errors](https://www.nhc.noaa.gov/verification/).
- GEM Foundation, [OpenQuake Engine documentation](https://docs.openquake.org/oq-engine/manual/latest/).
- NASA Ames, [Asteroid Threat Assessment Project](https://nas.nasa.gov/areas/atap.html), for multi-fidelity asteroid consequence assessment.
- NASA NTRS, [Uncertainty effects from an asteroid risk assessment perspective for the PDC25 impact scenario](https://ntrs.nasa.gov/citations/20250004315), for value-of-information motivation.

**Claim ceiling for this protocol:** it makes benchmark requirements explicit. It does not indicate that data packs have been assembled, any solver has been validated, any baseline has been beaten, or any warning system is operational.