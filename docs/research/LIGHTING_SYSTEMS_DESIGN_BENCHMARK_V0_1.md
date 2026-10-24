# Lighting Systems Design Benchmark v0.1

**Status:** research protocol proposal; not an implementation or performance result  
**Owner:** Symtropy × Symthaea research track  
**Version:** 0.1.0  
**Last literature check:** 2026-10-24

## Executive thesis

Optimize the *lighting service* rather than maximizing a lamp's headline lumens per watt. A design is useful only when it delivers the required visual task across the required space and time while satisfying color, glare, temporal-modulation, thermal, electrical, environmental, repairability, and cost constraints.

Symthaea may propose candidate designs, causal experiments, and next measurements. Symtropy may represent the coupled scene, controls, resources, and operating context. Neither should silently become its own independent optical oracle. Use established photometric tools such as Radiance for lighting predictions and EnergyPlus/OpenStudio for building-energy/daylight interactions, then check selected designs against physical measurements.

This document defines a research protocol, not an assertion that Symtropy already contains a validated photometric solver or that Symthaea currently beats commercial design methods.

## Why this is a timely research direction

The US Department of Energy's solid-state-lighting program identifies opportunity beyond source efficacy: suitable amount, spectrum and distribution of light, delivered to the right place and at the right time. Its lighting-quality work explicitly covers spectrum/color, flicker, glare, luminance perception, and integration of daylight, electric light and controls. DesignLights Consortium (DLC) SSL V6.0 / LUNA V2.0 requirements also expand the focus on efficacy, lighting quality, controllability and outdoor light-pollution mitigation; the published transition says V6.0 took effect for applications on 2026-01-05. Check the current category-specific requirements before using any threshold as a compliance gate.

## Paradigm shifts to test

1. **From lamp efficacy to service efficacy.** Optimize electrical energy per *compliant task-hour*, not lumens per watt alone. A candidate that misses required illuminance, distribution or quality is not a winner, regardless of efficiency.
2. **Joint optical–electrical–thermal–control co-design.** Search LED package/channel selection, current/driver behavior, optics, fixture geometry, heatsinking, sensors, dimming policy and placement as one system. Keep manufacturing and repair cost in the objective vector.
3. **Spectral-spatial-temporal lighting.** Treat spectrum, beam distribution, and time-varying controls as coupled choices. Report spectrum and color metrics; do not claim health benefits merely because a spectrum or CCT changes. Any human-response or circadian track needs its own predeclared exposure metric and relevant human-subject evidence.
4. **Counterfactual digital twin.** Use a validated external lighting engine as an optical reference. Symtropy represents context and constraints; the design optimizer asks what changes cause what effects. Every prediction is paired with an explicit model domain and uncertainty.
5. **Multi-fidelity, experiment-efficient search.** Screen cheaply with analytical bounds and low-cost scenes, then spend high-quality Radiance and physical-prototype budget only on candidates that might improve the Pareto frontier. Compare against strong non-AI search algorithms under the same expensive-evaluation budget.
6. **Evidence-carrying design.** A design carries source hashes, photometry, spectrum, driver curves, environmental assumptions, solver settings, calibration, uncertainty and verification outcomes. A candidate without required evidence is unknown or not evaluated, never a pass.
7. **Repairability and resilience as first-class design goals.** Report replaceable subassemblies, service time, spare-part dependence, idle power, safe failure behavior, DC/off-grid compatibility and lifecycle costs. Avoid optimizing only first cost or laboratory performance.

## Initial scope: fixed task-light scene

Start with a deterministic, no-daylight task-light scene so optical and energy effects can be isolated before adding occupancy, daylight and whole-building simulation. The synthetic reference scene is in fixtures/task-light-room-v0.1.json.

The fixture is a software test scene, **not** a product test or a regulatory definition of an office. Its 500 lx average, 300 lx minimum and 0.60 minimum-to-average ratio are proposed research targets for this fixture only. Replace or supplement them with the correct maintained-illuminance and quality requirements for each real application, with source and edition recorded.

The first physical follow-up should measure a commercial task light and then a candidate prototype at the same geometry, power-measurement boundary, thermal stabilization condition, sensor grid, ambient condition, and task-service thresholds.

## System boundary and responsibilities

| Component | Responsibility | Must not claim |
|---|---|---|
| Symthaea | Propose designs; select informative experiments; track candidate trade-offs and model uncertainty | That a generated design is physically feasible or safe without independent checks |
| Symtropy | Represent scene geometry/state, controls, resources, operating schedules and coupled outcomes; orchestrate adapters | That a visually plausible Bevy scene is a validated photometric solution |
| Radiance | Reference ray-traced illuminance/luminance/daylight computation using pinned geometry, materials and settings | That simulation alone validates a product, optical material model, or installation |
| EnergyPlus / OpenStudio | Annual building energy, daylight/control interaction and whole-building operating context when in scope | That a synthetic annual profile establishes field savings |
| Independent evaluator | Enforce constraints, calculate metrics and compare results from immutable inputs | That a candidate's own reported score is proof |
| Physical test | Measure electrical/optical output and thermal behavior using appropriate equipment and methods | That a short test proves lifetime or human-health effects |

Do not add a bespoke renderer or reimplement a production-grade optical solver before showing that adapters to validated tools cannot satisfy the need. Keep optical, building-energy and system-state responsibilities separately versioned.

## Benchmark tiers

### T0 — analytical and software-contract tests

- Unit and dimensional checks for area, distance, angles, radiant/photometric quantities and electrical energy.
- Finite-value validation, invalid-input rejection, deterministic replay, stable units, duplicate-ID rejection and explicit unknown states.
- Metamorphic cases: a controlled point-source fixture follows inverse-square behavior within stated assumptions; scaling electrical power with a fixed spectrum and linear operating regime scales output as declared; rotating a symmetric scene preserves expected symmetric results.
- Energy ledger: controller, driver and source power are included at a declared meter boundary. Never use LED-package efficacy as if it were wall-plug luminaire efficacy.
- A deliberately invalid or unsupported optical/material input must fail closed and must not return a ranked candidate.

### T1 — fixed synthetic task-light scene

- Generate a reproducible illuminance grid across the task plane and report average, minimum, maximum, uniformity (minimum/average), and grid-level residuals.
- Compare commercial measured baseline data, an engineering-rule/IES-file layout baseline, random or Sobol search, and at least one tuned non-AI optimizer.
- Run Symthaea's proposal/search path with the same candidate space, information access, expensive-solver budget and fixed seeds.
- Keep the initial scene and all evaluation criteria frozen before scoring candidates.

### T2 — held-out scene generalization

- At least 20 held-out variants across task-plane size/position, fixture height, room geometry, surface reflectance, optical input, and bounded sensor/model perturbations.
- Split by *scene family*, not randomly by nearly identical rows. Keep a public development set and a sealed or separately maintained evaluation set.
- Include reflective, dark, asymmetric and partially occluded cases; do not tune only on a single empty rectangle.
- Use matched random seeds and paired deltas. Bootstrap uncertainty at the scene level (and hardware-specimen level for physical tests), not by pretending correlated grid points are independent samples.

### T3 — daylight, adaptive control and full-system operation

- Integrate weather/daylight and occupancy only after the fixed-scene optical contract is stable.
- Run annual or representative-period evaluation with pinned climate/weather file, time zone, schedules, sensor placement, daylight control logic, standby power, dimming behavior and control constraints.
- Baselines include static full-on, standard daylight dimming, occupancy-only control, daylight-plus-occupancy control, and Symthaea-guided policy. All arms receive the same observations and actuator limits.
- Report energy, compliant-service hours, under-lighting duration, over-lighting, sensor faults, transition behavior, comfort constraints and failure recovery separately.

### T4 — prototype and independent measurement

- Select at least one Pareto candidate plus the strongest eligible commercial baseline for physical comparison.
- Measure optical/electrical properties using the current applicable IES method (LM-79-24 is the current edition identified by the IES publisher at this protocol's last literature check), plus temperature-related characterization as required for the design.
- Compare predicted and measured illuminance maps, distribution, spectrum, input power and thermal behavior. Record instrument model, calibration date, uncertainty, ambient conditions, stabilization, sample count and raw data.
- No lifecycle, lifetime, photobiological-safety, reliability, field-energy or health claim is established by a short prototype session; each needs a separately appropriate test plan.

## Metrics: report a vector, not an opaque composite score

### Hard gates (each is pass / fail / unknown / unsupported / not evaluated)

- Application-defined maintained illuminance and task-plane coverage.
- Minimum/average uniformity and glare/visual-comfort limits appropriate to the application.
- Color and spectrum constraints, including CCT/Duv and ANSI/IES TM-30-24 measures where relevant.
- Temporal light modulation at full output and across the supported dimming range, using appropriate instruments and metrics such as PstLM and SVM where applicable.
- Electrical, thermal, photobiological, ingress, fire and other product-safety requirements as applicable, verified by the competent party.
- Outdoor spill/uplight, trespass and dark-sky constraints when outdoors.
- Driver/control fail-safe behavior and sensor-loss fallback when adaptive control is in scope.

A candidate with an unknown or unsupported hard gate is not rankable as compliant.

### Optimization and diagnostic metrics

- Wall-plug luminaire efficacy (lm/W) and total electrical power.
- Wh per compliant task-hour and energy per unit of compliant task-plane service.
- Illuminance-distribution error, uniformity, glare, spectral/color metrics and flicker metrics.
- Thermal steady state and the most relevant junction/board/driver temperature estimates, including method and uncertainty.
- Costed bill of materials, installation cost, maintenance cost and lifecycle cost under explicit assumptions.
- Replaceable-part count, replacement/service time, common-tool requirement, repair success, idle/standby watts and embodied-material disclosures.
- Annual site-energy impact, daylight contribution and demand/controllability impacts when T3 is active.
- Search cost: expensive solver calls, total wall-clock, CPU/GPU energy if practical, candidate count, time to first feasible candidate, time to a fixed Pareto quality and failure rates.
- Model validity: prediction bias, absolute error, calibration quality, uncertainty intervals, unsupported-domain detection and physical-validation coverage.

### Guarding against metric gaming

- Report performance only for candidates that pass all applicable hard gates; separately publish the full failure distribution.
- Never replace a missed illuminance target with a weighted-score credit for excellent efficacy.
- Do not treat extra light as useful output after the required task service is met unless the case declares a reason for it.
- Count driver, control, sensor and standby power inside a consistently defined boundary.
- Report all dimming states in which the product is claimed to operate, not only nominal output.
- Keep color, flicker, glare and light-pollution results separate; these dimensions are not interchangeable.
- Report cost/repairability assumptions, and mark missing inputs rather than manufacturing estimates.

## Baselines and fair comparisons

1. **Commercial measured baseline:** at least one product in the same use class, with exact SKU, datasheet revision, IES/LDT file and/or measured outputs, price/date/region, dimming options and measurement provenance. Compare service-equivalent settings, not nameplate lumens alone.
2. **Expert engineering baseline:** a documented layout/fixture selection from an appropriate photometric file and application requirements. When feasible, obtain independent review from a lighting professional.
3. **Simple search baselines:** deterministic engineering rules, fixed-grid/coordinate search, and seeded random or Sobol sampling.
4. **Strong optimizer baseline:** a tuned, well-established multi-objective or Bayesian/derivative-free optimizer; fix its hyperparameter-selection budget before the held-out run.
5. **Symthaea-guided search:** same design variables and hard constraints; same observation access, solver budget and held-out cases. Include ablations removing memory, uncertainty gating, multi-fidelity choice, or cognitive proposals to establish which component has causal value.
6. **Oracle bound:** only for small synthetic cases with a verifiable exact solution. An oracle is a ceiling for that case, not a deployable competitor.

Pin code/model versions and licenses. An upstream benchmark is comparable only when task semantics, inputs, product boundary, constraints and metrics match. Otherwise label it not comparable rather than importing a leaderboard number.

## Pre-registered target hypotheses (not current results)

These are proposed internal engineering targets to test, not industry standards or claims already attained.

- **H1 — service-equivalent efficiency:** on held-out scenes, achieve at least 15% median reduction in energy per compliant task-hour against the strongest eligible service-equivalent baseline, while all hard quality/safety gates pass. Require the scene-clustered 95% confidence interval to exclude zero savings before claiming evidence of a positive effect.
- **H2 — search efficiency:** under an equal expensive-simulation budget, reach a predeclared reference Pareto hypervolume with at least 2× fewer high-fidelity solver calls than the strongest search baseline. Freeze normalization and reference point before evaluation.
- **H3 — prediction validity:** after calibration on disjoint cases, meet a predeclared error/uncertainty tolerance against held-out physical illuminance and electrical measurements. A proposed initial gate is median absolute relative illuminance error ≤10% for sensor points above 50 lx and a reported 95th-percentile error, with uncertainty floors based on instrument accuracy. Revisit this threshold if the measurement rig cannot support it.
- **H4 — robust feasibility:** zero *accepted-as-compliant* candidates violate explicit hard constraints in the test corpus, including injected malformed inputs and sensor/actuator faults. This finite result is not a proof of universal safety.
- **H5 — lifecycle/repair trade-off:** identify at least one Pareto candidate that reduces lifecycle cost or repair burden without worsening compliant-service energy or quality gates. A result counts only with a declared cost model and sensitivity analysis.

A hypothesis may fail. Publish negative results, uncertainty and trade-offs. Do not silently relax a gate after inspecting the result.

## Run identity and evidence contract

Every run should preserve:

- Exact repository commit(s), clean/dirty state, and relevant source-tree hashes.
- Scenario and geometry hashes; material reflectances and their source; complete IES/LDT/SPD and electrical-curve hashes; product SKU/revision if real.
- Radiance, EnergyPlus/OpenStudio, adapter and optimizer versions; executable/container digests; full solver settings and random seeds.
- Design variables, hard-constraint outcomes, per-case metrics, solver-call counts, timing and resource use.
- Raw simulator outputs and instrument data, calibration/uncertainty records, measurement protocol and exclusions.
- Independent-verifier source identity, per-check result and report digest.
- Explicit evidence tier: synthetic-contract, simulation-only, physical-prototype, or independently-tested-product.

Queued CI, a valid JSON file, a plausible render, a simulated pass, or successful candidate generation is not proof of optical accuracy or real-world superiority. Unknown, not evaluated, unsupported, timeout, and invalid output must stay distinct. Never convert a missing result into a zero, pass or estimated score.

## Implementation sequence

1. **Protocol and fixture** — drafted in this increment, including a versioned case schema and a synthetic no-daylight task-light scenario.
2. **Independent known-answer evaluator** — an initial dependency-free Rust kernel is included at tools/lighting-benchmark/. It derives average, minimum, maximum and minimum/average uniformity from raw grid samples, validates grid length/finite/nonnegative values, applies numeric service/power gates, preserves unresolved evidence states, and withholds the energy/service metric unless rankable. A separate robust-evaluation API consumes pointwise lower/upper illuminance bounds, uses conservative grid metrics, and requires upper-bound wall power/energy. Seventeen evaluator unit tests cover known-answer summaries, inverted or missing bounds, and negative controls; a separate deterministic experiment-selector module adds six tests for budget, safety, duplicate-ID, malformed-estimate and tie-breaking behavior. It does not yet parse the JSON fixture or calculate photometry. The focused CI workflow is added, but source/test status must be read from the exact-head run rather than inferred from the workflow file.
3. **Tool adapters** — pinned Radiance invocation and parsed outputs first; EnergyPlus/OpenStudio only for T3. The adapter must fail closed when the executable, input file, output or version identity is missing.
4. **Baselines and candidate store** — import versioned public IES/LDT and measured product data with rights/source metadata; never invent unavailable optical properties.
5. **Budget-matched search** — connect Symthaea through a narrow typed candidate interface. Symtropy orchestrates the scenario and resource constraints; the verifier judges the output.
6. **Held-out validation** — lock test scene families and analysis plan before tuning.
7. **Physical prototype** — purchase/test a baseline, measure a proposed design, quantify error, and update the model only on calibration data.
8. **Outdoor/off-grid and advanced spectra** — independent tracks with additional glare, skyglow, resilience, photobiological and power-system gates.

## Out of scope for v0.1

This protocol does not claim a new LED semiconductor, phosphor, metamaterial, optic, driver circuit, certified luminaire, therapeutic light, lifetime result, or energy saving. It does not make Symtropy a validated ray tracer or replace accredited product-safety assessment. Novel hardware/materials research follows only after a reproducible system-level win or a separate physical mechanism hypothesis is defined.

## References

- US DOE, Solid-State Lighting R&D: https://www.energy.gov/cmei/ssl/research-development
- US DOE, Lighting Quality: https://www.energy.gov/cmei/ssl/lighting-quality
- US DOE, Technology Roadmaps (2022 SSL R&D Opportunities): https://www.energy.gov/cmei/ssl/technology-roadmaps
- DesignLights Consortium, SSL V6.0 / LUNA V2.0 technical requirements and transition: https://designlights.org/our-work/solid-state-lighting/technical-requirements/
- IES, ANSI/IES LM-79-24, Optical and Electrical Measurements of Solid-State Lighting Products: https://store.ies.org/product/optical-and-electrical-measurements-of-solid-state-lighting-products/?v=eb65bcceaa5f
- IES, ANSI/IES TM-30-24, Method for Evaluating Light Source Color Rendition: https://store.ies.org/product/technical-memorandum-ies-method-for-evaluating-light-source-color-rendition/?v=eb65bcceaa5f
- Radiance, validated lighting simulation tool: https://www.radiance-online.org/ and https://github.com/LBNL-ETA/Radiance
- EnergyPlus documentation: https://energyplus.readthedocs.io/en/latest/
- OpenStudio (EnergyPlus + Radiance workflow): https://openstudio.net/
- IEA 4E SSL Annex, 2023 temporal-light-modulation interlaboratory comparison final report (2026-06-09): https://www.iea-4e.org/ssl/publications/measurement-of-temporal-light-modulation-interlaboratory-comparison-2023-ic-2023-final-report/

These references motivate the protocol and identify external methods; they do not establish that this proposed implementation passes them.
