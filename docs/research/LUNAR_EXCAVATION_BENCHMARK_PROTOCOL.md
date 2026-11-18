# Lunar excavation: from simulator accuracy to evidence-qualified delivered service

**Research update — 18 November 2026**

The programme's next competitive edge is a shared benchmark spanning material state, tool interaction, model generalization, and end-to-end material delivery. A cutting-force fit is necessary but insufficient: the useful output is accepted, delivered material or accepted underground asset, not a nominal cut.

## New evidence that changes the benchmark

- **DEM calibration:** Sun et al. (15 March 2026) report below-5% relative error for a dynamically/static-calibrated Hertz–Mindlin-with-bonding DEM on matched JLU-H excavation and cone-penetration tests. Treat this as a reproduction target for those conditions, not universal lunar-soil SOTA. https://doi.org/10.1016/j.powtec.2025.122070
- **CEL limitations:** Long-Fox et al. (2026) reproduce broad JSC-1A force trends in static/percussive excavation but identify mean-force overprediction and constitutive/numerical limitations. https://doi.org/10.1016/j.actaastro.2026.06.050
- **Simulant reproducibility:** Long-Fox, Dotson & Britt (31 August 2026) emphasize composition, particle size, preparation, compaction, temperature, pressure and environment, and recommend common protocols, public property databases and larger test facilities. https://doi.org/10.1007/s00603-026-05898-x
- **Integrated chain performance:** ESA's 2026 MoLES3 analogue campaign separated excavation, transfer and beneficiation and found transfer loss to be the dominant recoverable inefficiency for its tested architecture. The work uses a 2.5-hour field window, 60-kg system-mass limit and 300-Wh target; these are challenge-specific constraints, not universal machine benchmarks. https://www.nature.com/articles/s44453-026-00051-y
- **Lunar construction readiness:** NASA's 2024 technology status described an IPEx pilot profile of 10 tonnes over 15 days with limited autonomy and listed repairability, dust, survivability, autonomy and scale-up as gaps. It stated that extraterrestrial excavation for construction had not yet been attempted. Its stated surface-site KPPs included 10-mm topology resolution (5-mm goal) and 1-m trench depth (3-m goal). These are programme thresholds/goals in a 2024 presentation, not verified current flight performance. https://ntrs.nasa.gov/citations/20240003669
- **Rock-tunnelling comparator:** USACE EM 1110-2-2901 defines utilization as mining time divided by shift time, stresses its averaging basis, and gives typical whole-project utilization of 35–50% (historical database average 38%). Compare lunar systems with the same time boundary and downtime taxonomy; do not compare raw cutting rate with calendar advance. https://www.publications.usace.army.mil/Portals/76/Publications/EngineerManuals/EM_1110-2-2901.pdf

## Paradigm shifts

1. **Material identity is part of the model.** No universal “lunar soil” parameter set. Material batch, PSD, angularity, preparation, compaction, density, overburden, confining pressure and environment travel with every result.
2. **Generalization outranks calibration fit.** Split by material batch, preparation, tool geometry and operating regime. Hold out entire groups, not random repeats from the same specimen/campaign.
3. **Uncertainty is a first-class output.** Score prediction-interval coverage and sharpness beside point error; report worst-regime behavior and non-convergence.
4. **Benchmark the whole material chain.** Survey → cut → collect → transfer → process → store → deliver. Measure loss and energy at each boundary.
5. **Optimize accepted service, not machine motion.** Report accepted delivered tonnes/m³ per mission-hour and mission-kWh, while preserving quality, uptime, wear and safety.
6. **Keep model authorities independent.** Symtropy orchestrates immutable data and replay; specialist solvers own physics; Symthaea proposes informative experiments but cannot certify its own outputs.

## Proposed benchmark targets (research hypotheses)

- Reproduce source-reported results under matched protocols before making an SOTA comparison.
- Reduce held-out force-curve error by at least 20% against a preregistered analytical/empirical baseline while maintaining the preregistered 90–95% prediction-interval coverage band.
- Achieve the same held-out accuracy and coverage with at least 25% fewer physical calibration experiments.
- Measure 100% of declared mass-transfer boundaries or return explicit unknown/indeterminate states.
- Improve accepted delivered tonnes per mission-hour and per mission-kWh without trading away particle-size quality, tool life, availability or safety.

These are proposed falsifiable targets. Absolute tolerances must be set after a matched terrestrial analogue campaign and tied to instrument uncertainty and intended use. They are not achieved results.

## Implementation

The run-level contract, dataset-level split manifest, synthetic examples and fixture CI guard are in tools/lunar-excavation-benchmark/. The schemas make the intended grouping and holdout boundaries explicit; CI currently verifies only the synthetic examples. A general independent evaluator must still enforce every imported record against the manifest, artifact digest and reference resolution, calibration validity, interval ordering, unit consistency, repeat leakage, blind-set custody and licence admission. JSON Schema conformance alone is not evidence qualification.

Detailed protocol: tools/lunar-excavation-benchmark/README.md.

Related programme: [#6012 model validation](https://github.com/Luminous-Dynamics/symtropy/issues/6012), [#6006 as-built closure](https://github.com/Luminous-Dynamics/symtropy/issues/6006), [#5988 terrestrial analogue](https://github.com/Luminous-Dynamics/symtropy/issues/5988).

## New 2026 comparators — 18 November

The SOTA comparison now needs an explicit ML baseline. Srinivasan et al. (Acta Astronautica, online 18 August 2026; DOI 10.1016/j.actaastro.2026.08.051) report XGBoost RMSE of 11.305 N horizontal and 6.824 N vertical, 64% and 85.5% lower than their Lockheed–Martin analytical baseline on their study dataset. Those numbers must be reproduced with the original feature set, split and test protocol; they are not evidence of new-batch, new-tool, percussive or lunar-gravity generalization.

A separate returned-sample reference is now available: Bao et al. (Acta Astronautica, online 3 November 2026; DOI 10.1016/j.actaastro.2026.10.071) report rotating-drum slumping tests on authentic Chang'e-6 farside sample CE60100 (1.85 g available mass). Their dynamic angle of repose fell from 50.4° ± 3.1° to 41.1° ± 1.8° over Fr = 3.3×10^-4 to 3.3×10^-3, with a force-balance model reproducing the trend without fitting those DAoR values. This supports a distinct micro-sample flow/handling benchmark, not excavation-force qualification.

The model tournament now includes analytical equations, published MLR/SVR/RF/XGBR baselines, DEM, CEL/FEM, and an experimentally qualified physics-informed residual model candidate. Score interpolation and genuine group-level transfer separately; report abstention/OOD detection and uncertainty coverage alongside RMSE. For the material-handling leg, measure the complete excavation-to-beneficiation transfer chain and isolate retained material, bridging and discharge losses.

NASA's public Lunar Surface Technology page currently describes IPEx's 10-tonne / 100-m / 11-day profile, while other programme text describes an approximately 20-tonne single-mission goal. Preserve each source's mission boundary and goal/status label rather than merge them into an achieved throughput metric. The 2026 ISAM State of Play classifies surface infrastructure across excavation/site preparation, processing, logistics and construction.

