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

The run-level data contract and synthetic example are in tools/lunar-excavation-benchmark/. The schema is the first increment only. A later independent evaluator must still enforce cross-record split disjointness, artifact digest and reference resolution, calibration validity, interval ordering, unit consistency, repeat leakage and licence admission. JSON Schema conformance alone is not evidence qualification.

Detailed protocol: tools/lunar-excavation-benchmark/README.md.

Related programme: [#6012 model validation](https://github.com/Luminous-Dynamics/symtropy/issues/6012), [#6006 as-built closure](https://github.com/Luminous-Dynamics/symtropy/issues/6006), [#5988 terrestrial analogue](https://github.com/Luminous-Dynamics/symtropy/issues/5988).
