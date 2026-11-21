# CMNI causal experiment-design matrix

**Snapshot:** 2026-11-20  
**Tracking:** https://github.com/Luminous-Dynamics/symtropy/issues/6453  
**Scientific hypothesis register:** https://github.com/Luminous-Dynamics/symthaea/blob/main/docs/research/cmni/HYPOTHESIS_REGISTER.md

## 1. Design objective

Design experiments that distinguish the causes of changes in low-energy D–D yield in metal/hydride targets. The target is not to maximize anomalous counts. It is to maximize expected discrimination among models under fixed experimental cost, while preserving calibrated uncertainties and negative controls.

The 2026 Pd/Ti study reports that a single effective screening parameter can lump together electronic structure, defect density, deuterium distribution, and transport. Its authors propose depth-resolved H/D profiling and positron annihilation spectroscopy as direct tests of defect/trapping hypotheses. The model must therefore represent these fields separately rather than collapsing them into one unconstrained coefficient.

## 2. Causal graph and state separation

`sample preparation/history → phase, strain, surface state, defects`

`electrochemical loading + beam flux + time + temperature → D occupancy, implantation, diffusion, desorption`

`beam species/energy distribution + stopping + local material state → reaction yield`

`reaction yield + branching + geometry + detector response + background → observed counts`

Track independent state variables:
- Material: host, isotope, phase, lattice parameter, composition, purity, layer/overlayer, thickness, grain structure, strain and sample history.
- Defects: vacancies/open-volume defect proxies, trapping sites, dislocations, grain boundaries and uncertainty.
- Fuel state: D/H occupancy vs depth/time, concentration gradients, beam implantation, replenishment and surface loss.
- Beam: species mixture, energy distribution, flux/current, integrated charge, spot/geometry and duty.
- Environment: pressure, residual gas composition, temperature and thermal history.
- Observation: detector identity, geometry, energy range, calibration, efficiency, resolution, live/dead time, trigger/selection, backgrounds and clock sync.

Unknown or unmeasured variables remain unknown. A model-inferred D profile or defect density is not a measured input; identify it as latent and propagate its uncertainty.

## 3. Study-design families

| Design family | Comparison | Main causal question | Required measurements |
|---|---|---|---|
| Beam × loading factorial | Beam on/off crossed with electrochemical D loading on/off | Is the effect conditional on beam-driven nuclear excitation, and does loading change the target response? | Beam/charge record, neutron/proton channels where available, current/potential and electrolyte records, material state |
| H/D isotope control | D-bearing electrolyte vs H-bearing electrolyte under matched apparatus conditions | Is the response isotope-specific rather than a generic electrochemical/thermal/DAQ effect? | Isotope composition, temperature, electrical input, beam state, product spectra and background |
| Host-material transfer | Pd vs Ti (and justified later materials) at measured comparable fuel/defect state | Which features generalize across different electronic/transport properties? | Composition, phase, occupancy profile, diffusion/transport indicators, defects and detector response |
| Defect-state intervention | Samples with different characterized defect states while matching D occupancy as closely as feasible | Does yield track defect/trap populations independently of fuel density? | Pre/post positron-annihilation measurements, depth-resolved D, microstructure and anneal history |
| Transport/history intervention | Different loading histories, beam dose/flux/duty and thermal histories | Do response lags/hysteresis follow D replenishment/depletion? | Time-resolved D profile proxies, beam flux, temperature, pressure/RGA and state history |
| Depth-profile intervention | Matched bulk composition but different characterized near-surface profiles/overlayers | Is the effect localized to the implantation/surface region? | SIMS/NRA/ERDA or equivalent depth-sensitive methods; thickness and surface characterization |
| Detector cross-check | Same physical condition observed with independent channels in their actual common energy range | Is the effect stable under distinct detection physics and calibration paths? | Detector-specific calibration, efficiency, live time, background, pulse-shape/energy spectra and synchronized timing |
| Beam species / energy decomposition | Mass-resolved D⁺ and D₂⁺ fractions, species-specific energy distributions, and prompt/delayed dissociation alternatives | Can a molecular-ion component create a distinct per-deuteron center-of-mass energy population that changes the apparent low-energy curve? | Mass-resolved beam composition vs operating point, energy distribution per species, fragmentation/dissociation characterization, Faraday-cup calibration, beam current and charge records |\n| Alternative excitation | New ion species or non-ion-beam stimuli only after baseline qualification | Does the effect require D+ implantation, or is it coupled to another material excitation? | Predicted signature per mechanism, appropriate diagnostics, null controls and power analysis |

These are abstract study designs. They do not specify high-voltage, ion-source, radiation, vacuum or electrochemical apparatus operating procedures. Physical work belongs in qualified institutions with approved safety and radiation controls.

## 4. Factorial structure and randomization

Do not attempt an enormous full-factorial matrix before the basic states are characterized. Use a staged design:

1. **Baseline characterization:** establish material and instrument repeatability; quantify sample-to-sample variation and the complete nuisance budget.
2. **Core factorial:** cross beam condition with deuterium-loading condition, with blanks and isotope controls.
3. **Material transfer:** replicate the core comparisons across Pd and Ti, while recording occupancy and defect differences.
4. **Mechanism interventions:** only after core effects reproduce, target defects, transport, depth, or electronic structure one at a time or using a statistically efficient fractional-factorial design.
5. **Prospective confirmation:** randomize condition order, blind sample labels and analysis where feasible, freeze thresholds and models before revealing results.

If a factor cannot be randomized (e.g. material batch), block by batch and model it explicitly. Do not confuse repeated time samples within one specimen with independent experimental replicates.

## 5. Multi-fidelity computational plan

| Fidelity | Tool/model role | Output contract |
|---|---|---|
| F0 — analytic | Bare-nucleus and homogeneous ALR-style reference yields | Units, parameter validity and numerical error |
| F1 — transport | Evaluated stopping/implantation and diffusion/replenishment/desorption models | Depth/time fields and uncertainty |
| F2 — materials | DFT/MD/phase-field or validated material characterization | Electronic/defect/phase descriptors and solver provenance |
| F3 — local reaction | Depth-weighted local screening/occupancy hypotheses | Reaction-yield distribution, parameter identifiability |
| F4 — observation | Detector response, branching, geometry, background and DAQ model | Expected spectra/counts with channel-specific uncertainty |
| F5 — evidence twin | Symtropy composition and replay across all adapters | Conservation residuals, data lineage, replay receipt and sensitivity |

Use specialist tools for the physics they are validated to solve. Symtropy orchestrates and qualifies model coupling; it does not replace evaluated nuclear data, electronic-structure solvers, stopping codes or detector Monte Carlo.

## 6. Symtropy's experiment-selection objective

For candidate experiment (e), compare the predicted observation distributions (p(ymid H_i,e)) for the competing hypotheses. Select experiments that maximize expected discrimination, for example expected information gain / expected reduction in model entropy, subject to:

- experiment and sample cost;
- instrument time and availability;
- expected statistical power and background;
- safety and institutional feasibility;
- uncertainty in the proposed intervention itself;
- ability to produce a clear null result.

Benchmark active learning against uniform sweeps, one-factor-at-a-time, random designs and a predeclared expert design under equal cost. The score is discrimination gained per unit cost, not the number of positive-looking outcomes.

## 7. Required outputs for each proposed design

- A machine-readable design record linked to source and hypothesis IDs.
- Predicted distributions for all measured channels, not just scalar yield.
- The design's ability to separate H0 from each alternative, with power/uncertainty.
- Required calibrations, blanks, positive references and negative controls.
- Minimum detector sensitivity and energy/time resolution.
- Material characterization before/after.
- Expected run duration/cost, instrument time, compute and solver limits.
- Preregistered stopping, exclusion and analysis rules.
- A no-go condition when key latent variables cannot be measured tightly enough.

## 8. Acceptance criteria

A virtual design is not accepted because the simulation looks plausible. It must:
- produce reproducible output from frozen inputs;
- pass dimensional, conservation, manufactured-solution and limiting-case tests;
- show parameter identifiability or explicitly report non-identifiability;
- retain correct null behavior under synthetic negative controls;
- pass sensitivity analysis for nuisance and uncertainty assumptions;
- demonstrate superior expected discrimination at matched cost against the benchmark designs;
- be independently reviewed before any laboratory protocol is considered.

## 9. Source anchors

- 2026 Pd/Ti study discussion on local defects, deuterium transport, and recommended profiling: https://doi.org/10.1038/s41467-026-74421-1
- 2025 Pd beam-driven/electrochemical loading study: https://doi.org/10.1038/s41586-025-09042-7
- ARPA-E LENR evidence and control criteria: https://arpa-e-foa.energy.gov/FileContent.aspx?FileID=68c066ad-2aa8-4bbe-bfbf-fcb73762fdda


## 10. Beam species and per-deuteron energy distribution

The 2026 paper reports a typical source composition of ~85% D⁺ and ~15% D₂⁺, with deuteron flux calculated by counting two D atoms per D₂⁺ ion. Treat these as reported source fractions, not as a guaranteed constant for every energy point.

For a singly charged ion accelerated through (V) kV:
- D⁺ has approximately (V) keV total kinetic energy and (E_{cm}approx V/2) keV for D on stationary D.
- If D₂⁺ promptly dissociates into two D fragments that retain the molecular velocity, each D has approximately (V/2) keV and (E_{cm}approx V/4) keV.

For 85/15 ion-current fractions, this prompt-dissociation scenario gives approximately 26.1% of the deuteron flux in the lower-energy fragment component. It is a sensitivity case, not an observed fragment distribution.

Symtropy must compare at least (a) mono-species D⁺; (b) prompt D₂⁺ dissociation; and (c) an adjustable/delayed-dissociation model. Preserve ion-current fractions separately from deuteron-flux fractions. A retarding-potential analyzer's scalar energy-spread measurement does not by itself establish a mass-resolved per-deuteron spectrum; request the source's mass-resolution and fragmentation characterization.

Do not infer that D₂⁺ explains the reported plateau. The work item is to quantify whether the species mixture changes predicted yields materially at matched total deuteron flux and to define the measurements needed to discriminate it.


## 11. Source-backed 2025 control matrix

| Control | Published observation | Tests | Does not establish |
|---|---|---|---|
| Pre-run background | 0.21(9) n/s; described as stable over 24 h | Baseline neutron background under reported conditions | Background invariance in all apparatus states |
| Seven-target beam repeatability | ~69.3(2) n/s after 120 min at -30 kV / 0.2 mA | Repeatability across different Pd targets | Replication of the separate 0.5 mA electrochemical Phase I/II gain |
| Sheath voltage OFF | Signal returns to ~0.2 n/s | Dependence on D+ beam excitation | Electrochemical-cell-off persistence |
| H2O instead of D2O | 88.3(3)% reduction | Isotope/fuel-loading control | Direct D/Pd measurement during fusion or exclusion of every artifact |
| Electrochemical cell OFF with beam active | At most 2% variation; reactivation increase about 6% vs initial ~20% | Persistence of electrochemically loaded D | Beam-off behavior |
| XRD characterization | Separate in-situ XRD gives H/Pd around 0.7 | Independent evidence of high hydride loading in characterization setup | Direct D/Pd measurement during each fusion run |

Build separate source-summary controls and synthetic challenge cases. Never use the seven-target 0.2 mA result as if it were a replicate of the three 0.5 mA Phase I/II gains.

The reported 2025 setup uses one EJ-309 detector. Reconstruct the source response/PSD method first; future independent validation should use a separately calibrated detector or a different detection modality.


## 12. Spatial-fluence and irradiation-history interventions

The 2026 Methods report that beam focusing changes with target bias above 5 kV, while the reported spot is approximately 1 cm². A total-current-normalized yield can still conceal local changes in current density, defect production, temperature and D replenishment.

Add a matched-dose spatial counterfactual suite:
- Same integrated charge, different 2-D spot profiles.
- Same peak current density, different total charge.
- Same total charge and spot, different beam raster/drift.
- Same average D loading, different near-surface depth profile.
- Same D+/D2+ fractions, different species-resolved energy histogram.
- Same sample average, different local defect/temperature gradients.

Do not claim these effects explain the reported plateau. Their value is to find whether yield is identifiable from total charge and nominal energy alone, or whether spatial/temporal fluence must be measured and modelled.
