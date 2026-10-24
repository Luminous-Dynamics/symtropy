# Robust Lighting Search and Measurement Protocol v0.1

**Status:** research protocol; targets are prospective, not measured results  
**Parent track:** LIGHTING_SYSTEMS_DESIGN_BENCHMARK_V0_1.md  
**Date:** 2026-10-24

## Thesis

A lighting optimizer should not return merely the design with the best simulated score. It should return the best *evidence-supported design decision* under stated uncertainty, physical constraints, and an explicit measurement budget.

The proposed paradigm shift is from nominal best-score optimization to **robust feasibility + Pareto improvement + value-of-information-guided experiments**. This matters because a design near an illuminance, glare, flicker, thermal or electrical boundary can look excellent in a nominal model and fail after small changes in geometry, reflectance, temperature, power quality or sensor behavior.

The US DOE's lighting-quality program treats color/spectrum, temporal light modulation, glare, luminance perception and daylight/electric-light integration as distinct research topics. A 2025 systematic review of precise automated lighting and shading control reported that existing studies often emphasize energy savings while treating lighting indices secondarily, and that no single control algorithm resolves all objectives. The benchmark should therefore retain a vector of hard gates and outcomes rather than compress everything to one opaque reward.

## 1. Uncertainty is structured, not a generic percentage

Record uncertainty sources independently. Do not assign an arbitrary universal error bar to the entire model.

| Source | Examples | Recommended representation |
|---|---|---|
| Input uncertainty | Surface reflectance, fixture pose, room geometry, obstructions | Declared intervals/distributions and source/version |
| Source/device uncertainty | IES/LDT/SPD data, LED bin, channel mix, driver curve | Data source, exact revision, tolerances and operating conditions |
| Simulation discrepancy | Radiance prediction versus calibrated measurement | Held-out residuals and a domain-limited error model |
| Instrument uncertainty | Lux meter, spectroradiometer, power analyser, TLM instrument | Calibration, resolution, uncertainty budget and setup |
| Control uncertainty | Occupancy false detection, daylight-sensor bias, latency, packet loss | Fault injection and measured field distributions when available |
| Degradation | Lumen depreciation, spectral shift, dirt, diffuser aging, thermal cycling | Time-indexed models with evidence tier |
| Scenario uncertainty | Weather, occupancy, surface use, schedule, off-grid availability | Scene-family-held-out scenario sets |

Keep measurement uncertainty separate from model predictive uncertainty. They arise from different processes and should be combined only by a declared, technically appropriate method. If uncertainty is unknown, the affected constraint status is unknown; do not silently substitute a default. The initial Rust evaluator now has a conservative interval API that derives lower-bound average/minimum, upper-bound average/maximum, and lower-bound minimum/average uniformity from pointwise illuminance intervals, then evaluates upper-bound power/energy. It also requires an explicit interval basis and source ID; expanded measurement-uncertainty and calibrated-prediction intervals require a non-empty coverage statement. This is a software contract only: it does not verify the source artifact, interval coverage, calibration, or physical meaning of the inputs. Those remain the adapter/test's responsibility.

For interval-based checks, the interval must state what it means: a deterministic bound, an expanded measurement uncertainty with declared coverage factor, or a calibrated prediction interval. These are not interchangeable. Pointwise 95% intervals do not automatically provide 95% simultaneous coverage over an entire grid.

## 2. Make feasibility conservative

For an illuminance grid with defensible lower and upper bounds per point, derive conservative quantities:
- lower-bound average illuminance from all pointwise lower bounds;
- lower-bound minimum illuminance from the smallest lower bound;
- conservative minimum/average uniformity from the lower-bound minimum divided by the upper-bound average;
- upper-bound wall power from the declared electrical bound.

A candidate is *robustly feasible for this case* only if the conservative quantities meet every applicable hard gate. A design that passes on the point estimate but straddles a constraint is uncertain, not pass. If a gate requires a separate glare, color, flicker, thermal or safety measurement, no illuminance-only statistic can substitute for it.

For a stochastic scenario model, use calibrated uncertainty and a preregistered risk criterion rather than selecting an arbitrary worst-case probability. Report expected outcomes and tail risk separately; keep the scenario distribution, support, and calibration set visible.

## 3. The experiment selector should maximize decision value

Symthaea should choose the next expensive simulation or physical measurement based on the expected value of information, not just which untested design has the highest optimistic predicted score.

A first deterministic selector primitive now lives in tools/lighting-benchmark/src/experiment_selection.rs. It accepts externally estimated expected decision-loss reduction, cost in a declared common value unit, resource budget cost, and an external safety-clearance flag. It validates all option rows before selecting, fails closed on malformed estimates or duplicate IDs, deterministically breaks ties, and refuses options that are unsafe, over budget or have non-positive net expected value. It does **not** estimate value of information or validate the calibration/provenance of supplied estimates. That remains a research-model and evidence-contract responsibility. A practical acquisition policy can combine:
1. **Boundary value:** prioritize candidates where improved evidence could change a hard-gate decision.
2. **Pareto value:** prioritize candidates plausibly improving a currently uncovered section of the feasible Pareto frontier.
3. **Uncertainty reduction:** measure the variable or scene with the largest expected reduction in decision-relevant uncertainty.
4. **Model-disagreement value:** compare independent methods or surrogate models where predictions differ.
5. **Cost and feasibility:** account for measurement cost, tool availability, prototype cost, expected information, and whether the experiment can be performed safely.

A candidate with uncertain safety/quality properties should never be chosen for live deployment solely because it has high expected reward. Active experiments must remain offline or under qualified laboratory controls until appropriate evidence is available.

### Baselines for experiment choice

Compare value-of-information-guided search with:
- fixed design-of-experiments plans;
- seeded random/Sobol exploration;
- a standard Bayesian or derivative-free optimizer;
- uncertainty-blind best predicted score;
- measurement focused only on constraint-boundary cases;
- a no-active-selection baseline.

All arms receive the same design space, data available at decision time, high-fidelity solver-call budget, physical test budget and compute budget. Tune each method on development scenes only.

## 4. Stress families for the held-out matrix

Keep these as *declared synthetic perturbations* until replaced with actual distributions from measurements or source evidence.

- **Geometry:** fixture position/height/orientation, task surface size, obstructions and room aspect ratio.
- **Reflectance/material:** darker, brighter, asymmetric and spatially nonuniform surfaces.
- **Electrical/thermal:** supply variation, channel/current changes, heat-sink and enclosure variation, warm-up and dimming.
- **Optical aging:** diffuser contamination or material changes only when supported by a specified physical model; otherwise treat them as qualitative future cases.
- **Control faults:** ambient sensor bias, occupancy false positives/negatives, missing samples, stale state, actuator saturation and delayed commands.
- **Daylight/schedule:** daylight changes, seasonal/weather files, occupancy variability and schedule disruption.
- **Outdoor environments:** spill/uplight, nearby reflective surfaces, dark-sky constraints and wildlife-sensitive contexts.
- **Resilience:** mains loss, DC/solar/battery supply constraints, low-state-of-charge fallback and restart after outage.

Partition by *scenario family*, not random rows, to prevent closely related variants from appearing in both tuning and held-out splits. Keep a frozen public development matrix and a separately controlled final evaluation matrix. Report both nominal and stress performance. An initial 27-case typed registry is now implemented in tools/lighting-benchmark/src/stress_matrix.rs and emitted by the lighting-stress-matrix binary. It covers synthetic fixture translations/heights, reflectance, power input, held-out asymmetric reflectance/occlusion/coupled geometry, control sensor bias/dropout/latency/stale state, and battery/outage interventions. It validates unique IDs, parameter ranges and family-level split isolation within each tier. These are software benchmark interventions only; they are not calibrated site distributions and are not yet executed against an optical/control solver.

For the South African application track, add Southern Hemisphere seasonal/weather cases only with pinned local weather files and explicit site/climate scope. Do not use a Johannesburg or South African label as a substitute for actual local data.

## 5. Metrics for robust search

Report these separately:

| Metric | Definition or rule |
|---|---|
| Robust feasibility rate | Fraction of held-out cases meeting every conservative hard gate; show the denominator and missing-evidence count |
| False-feasible rate | Fraction accepted as compliant that fail the independent reference check; target zero in the declared test corpus |
| Constraint margin | Distance to each limit, with uncertainty bounds, not just pass/fail |
| Service regret | Difference from the best verified feasible comparator on the preregistered service-energy objective |
| Pareto quality | Hypervolume or another fixed metric over normalized feasible outcomes; freeze normalization and reference point before final evaluation |
| Calibration | Empirical coverage and sharpness by scene family; report calibration data separately from held-out evaluation |
| Simulation-to-measurement error | Per-point and aggregate illuminance/electrical residuals with uncertainty and geometry/setup evidence |
| Measurement efficiency | Expensive simulations and physical measurements required to reach a fixed feasible Pareto quality |
| Tail behavior | Per-family worst case and a declared CVaR/quantile measure when sample count supports it |
| Evidence completeness | Raw inputs, outputs, calibration, source hashes and verifier receipt present versus missing |

Do not compute a single robustness score that lets an excellent nominal mean hide catastrophic constraint violations. Keep failed and unknown cases in the denominator; no silent imputation.

## 6. Falsifiable hypotheses

These are research hypotheses for later preregistration, not claims or standards.

- **R1 — robustness:** uncertainty-aware selection reduces the false-feasible rate against a nominal optimizer on held-out perturbation families without sacrificing feasible-case service-energy performance beyond a prespecified margin.
- **R2 — measurement efficiency:** value-of-information selection reaches a fixed feasible Pareto quality with fewer high-fidelity simulations/physical tests than the strongest equal-budget baseline.
- **R3 — uncertainty calibration:** prediction intervals achieve coverage appropriate to their declared definition on held-out physical cases; report scene-family failure and interval width, not only pooled coverage.
- **R4 — control resilience:** adaptive control retains compliant-light service within declared bounds under sensor bias/dropout and power disturbances, with safe fallback and no unqualified live action.
- **R5 — human and environmental quality:** any spectral, circadian, glare, flicker or outdoor-light claim is supported by its own metric and relevant measurement/evaluation protocol; efficacy cannot compensate for failure.

Any hypothesis can be rejected. If uncertainty is poorly calibrated or does not improve decisions, retain simpler deterministic engineering rules rather than adding a more complex model.

## 7. Evidence and provenance contract

Each candidate report should bind:
- case ID and immutable scenario/input digest;
- raw grid samples and their position/order mapping;
- explicit source of each optical/electrical property;
- simulator and adapter revision, command/configuration, seed and environment;
- measurement instrument ID/model, calibration status, method, ambient conditions, stabilization, uncertainty and raw output digest;
- separate point-estimate, bound and gate results;
- the independent evaluator revision and receipt digest;
- rejected/missing/unsupported evidence and all exclusions.

Use evidence tiers: synthetic_contract, simulation_only, calibrated_simulation, physical_prototype, and independently_tested_product. Never promote an evidence tier just because CI passes.

## 8. Measurement practice and external anchors

Use the current applicable ANSI/IES measurement method when testing products. ANSI/IES LM-79-24 addresses reproducible measurement of total luminous/radiant/photon flux, electrical power, system efficacy, intensity distribution, color quantities and spectrum. The IEA 4E SSL Annex's June 2026 IC 2023 final report highlights issues with uncertainty estimation, the influence of AC power supplies on PstLM, SVM/TLM instrument issues, and variability/conceptual issues with Mp. This is a reason to keep instrument/setup uncertainty in the evidence contract—not to claim the industry cannot measure flicker.

Research anchors:
- US DOE, Lighting Quality: https://www.energy.gov/cmei/ssl/lighting-quality
- US DOE, SSL R&D: https://www.energy.gov/cmei/ssl/research-development
- DesignLights Consortium, SSL V6.0 / LUNA V2.0: https://designlights.org/our-work/solid-state-lighting/ssl-v6-luna-v2/
- IES ANSI/IES LM-79-24: https://store.ies.org/product/optical-and-electrical-measurements-of-solid-state-lighting-products/?v=eb65bcceaa5f
- IEA 4E SSL Annex, IC 2023 final report: https://www.iea-4e.org/ssl/our-work/testing-standards/interlaboratory-comparison-2023/
- Review: precise lighting control via automated shading and artificial lighting, Building and Environment (2025), DOI 10.1016/j.buildenv.2025.113607: https://doi.org/10.1016/j.buildenv.2025.113607
- Review: multi-objective optimization of daylighting systems for energy efficiency and thermal-visual comfort, Building and Environment (2026), DOI 10.1016/j.buildenv.2025.113921: https://doi.org/10.1016/j.buildenv.2025.113921

These references motivate this protocol; they do not validate the present evaluator, any predicted design, or a commercial performance claim.
