# EHD source-atlas update — Jiang et al. 2026 sub-unit electrode cohort

**Source:** Jiang, Tsubata, Shimizu et al., *Performance enhancement of electrohydrodynamic thruster using sub unit electrode configuration*, Scientific Reports (published 7 November 2026; Article in Press at transcription time).  
**DOI:** https://doi.org/10.1038/s41598-026-78941-0  
**Atlas record:** JIANG_2026_SUB_UNIT_ELECTRODE

## Why this cohort matters

This is a recent experimental configuration study, not a numerical cohort. At 22 kV its abstract reports 50.23 mN for the conventional electrode system, 64.09 mN for the sub-unit system (+27.59%), and 68.70 mN for the sub-unit-plus system (+36.77% relative to conventional; +7.19% relative to sub-unit).

The additional structure changes the thrust/flow interaction; absolute thrust is not the only performance variable. The authors explicitly report a trade-off between maximizing thrust and maintaining thrust-to-power ratio.

## Measurement and provenance boundary

The reported experimental method uses the apparent-weight difference between discharge and non-discharge states on an A&D Fz-5000i high-resolution electronic balance. The study describes ten repeated measurements per condition, with the reported mean; the experimental setup uses vibration isolation and reports ambient temperature 21 °C and relative humidity 22.8%. The Figure 10 caption identifies its error bars as 95% confidence intervals, but this atlas transcription does not reproduce numerical interval endpoints.

The geometry table defines the sub-unit/collector distance, emitter–collector gap, emitter–guide distance, guide–emitter length, collector length and sub-unit length. Do not infer missing dimensions from schematic proportions. Only the named Figure 10 configuration sweep is transcribed here.

## Source-derived comparison: keep evidence classes distinct

### Abstract-level thrust values

| Configuration | Applied voltage | Thrust | Relative to conventional |
|---|---:|---:|---:|
| Conventional | 22 kV | 50.23 mN | Baseline |
| Sub unit | 22 kV | 64.09 mN | +27.59% |
| Sub unit plus | 22 kV | 68.70 mN | +36.77% |

### Figure 10 geometry sweep

The source text identifies the 22 kV maximum as sub-unit-plus width 50 mm and collector distance 10 mm, with a 15° inclination, 20 mm emitter–collector gap and 40 mm guide. The same figure text reports:

| Sub-unit-plus width | Distance to collector | Thrust |
|---:|---:|---:|
| 50 mm | 10 mm | 68.70 mN |
| 50 mm | 20 mm | 67.08 mN |
| 40 mm | 10 mm | 63.23 mN |
| 40 mm | 20 mm | 61.63 mN |

Figure 10's caption says the error bars are 95% confidence intervals; numerical endpoints are not copied into this record.

### Figure 9 visual estimates — not exact tabulated data

The source Figure 9 plots voltage against thrust-to-power and current. At 22 kV, a visual read gives approximately:
- conventional "No sub unit G20 Guide40": 3.0 N/kW and 0.77 mA;
- "Sub unit plus50 10mm Guide40": 2.4 N/kW and 1.34 mA.

These values are explicitly stored in the machine-readable atlas as approximate visual digitizations from source figures, not tabulated exact values. They are not raw measurements or an independently digitized data export. Do not use them for quantitative leaderboard ranking or report them with false precision.

The important engineering implication is nevertheless robust at the qualitative level and explicitly noted by the authors: the higher-thrust electrode arrangement trades against thrust-to-power. A selector maximizing absolute thrust alone would miss this Pareto trade-off.

## What the atlas deliberately does not claim

This record is not yet suitable for a cross-study EHD performance leaderboard:
- The exact figure-level electrical power and its boundary have not been independently transcribed.
- The plotted T/P and current values are approximate visual estimates, not raw time-series.
- The numerical 95% CI endpoints, trial-level dispersion and calibration certificate are not captured.
- The complete geometry dimensions/tolerances, system mass, converter/battery boundary and integrated thrust-to-weight ratio are not transcribed.
- The source-level thrust balance is not independently reconciled against all-surface control-volume momentum, pressure and viscous traction, PIV/LDV and frame/support drag.
- The current record is based on the early-access Article in Press; re-audit against the final Version of Record when it replaces the early version.

Do not rank this result against reduced-pressure CO2 DBD, grid-collector studies, wind-tunnel effective thrust or integrated microrobot systems without matching gas, pressure, geometry family, voltage/current waveform, force boundary, power boundary, mass boundary and uncertainty.

## Benchmark implications for Luminous

The most valuable follow-on is not to chase 68.70 mN as a single scalar target. Reproduce the configuration family and learn its Pareto surface:

1. **Matched-configuration reproduction:** same conventional, sub-unit and sub-unit-plus geometries, same voltage range, and a frozen force-balance protocol with randomized on/off order, before/after calibration, raw balance traces and ten-run source replication.
2. **Simultaneous power measurement:** synchronous electrode-terminal voltage/current and supply wall power; publish T, I, real P, T/P and uncertainty for every geometry. Keep terminal and wall-power boundaries separate.
3. **Flow and drag decomposition:** PIV/LDV on the complete plume and around sub-unit structures; quantify recirculation, entrainment, collector drag and frame/support forces. Reconstruct thrust from all control-volume surfaces, not one exit plane.
4. **Robust experiment design:** use nuisance-adjusted design to choose geometry/voltage runs that discriminate physical flow enhancement from force-stand offset, cable force, temperature drift, current rise and aerodynamic support effects.
5. **Exergy and species accounting:** reconcile measured electrical input, flow exergy, thermal/chemical/radiative exports and entropy generation. Do not call a static-balance thrust value useful flight propulsive power.
6. **Held-out family validation:** hold out a complete width/distance geometry family and a voltage range. Report prediction interval coverage and abstain beyond the measured envelope.

## Competitive comparison

- ANEMONE is a three-dimensional three-component plasma-fluid simulator for corona inception and energy-conversion efficiency. It is a discharge-physics reference, not evidence that an entire neutral-flow / thrust / power boundary is validated.
- PASCHEN-1D provides a strong one-dimensional transient plasma/circuit/surface-emission reference with drift-diffusion, Poisson, RLC coupling and configurable emission. It is a breakdown / discharge / circuit reference, not a three-dimensional atmospheric EHD thruster.
- COMSOL's simplified Corona Discharge interface and its fuller Electric Discharge capability must be treated as different model families: the former uses a reduced ion transport region and does not resolve ionization-layer electron dynamics; the latter can solve coupled charged-species drift-diffusion and Poisson. A label such as "COMSOL result" is not a sufficient model manifest.
- Recent experimental ML surrogates for EHD airflow report R² around 0.90 on their own dataset. Luminous should compare against Gradient Boosting, Random Forest and simple regression under identical full-family holdouts, with calibrated predictive intervals and physical closure gates; a better training fit or an uncalibrated R² is not enough.
- The 2026 analytical corona framework reports 4.13% MAPE for a 20 mm gap on its validated corona component, while its other propulsion quantities are derived from established relationships. The fair comparison is quantity-by-quantity: validate inception/current first, then flow, force, power and integrated system metrics.

The path to a defensible SOTA claim is cross-solver and cross-experiment agreement under one pinned equation/measurement boundary, independent replay, held-out transfer and uncertainty calibration—not a larger thrust number or a faster isolated surrogate.
