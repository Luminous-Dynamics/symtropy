# Human thermophysiology model tournament and first integration slice v0.1

**Status:** source-audited research protocol; no thermophysiology model is implemented by this document.  
**Issue:** [HUMAN-THERMOPHYSIOLOGY-BENCH-001 / #5776](https://github.com/Luminous-Dynamics/symtropy/issues/5776)  
**Architecture:** [human multiscale systems atlas v0.1](HUMAN_MULTISCALE_SYSTEMS_ATLAS_V0_1.md)  
**Register:** [machine-readable coupling register v0.1](HUMAN_MULTISCALE_COUPLING_REGISTER_V0_1.json)

## 1. Why thermophysiology first?

Of the unimplemented human biological areas, whole-body thermoregulation offers a particularly useful first integration challenge:
- it couples the external physical environment to a living system;
- it includes spatial differences across body regions;
- it includes active control (skin blood flow, sweating, shivering and other mechanisms);
- it connects workload, metabolism, cardiovascular transport, hydration and heat exchange;
- it has multiple published model fidelities and external comparison literature.

Younes & Khovalyg's 2025 review catalogues 96 whole-body thermoregulation models. It identifies a need for stronger standardized validation and notes that active-system control and personalization have not always advanced as quickly as anatomical and spatial detail. This directly supports a benchmark-first approach, not simply adding anatomical nodes.

**Key source:** https://doi.org/10.1016/j.compbiomed.2025.111218

## 2. Candidate model tournament

| Candidate | Model class / coverage | Role in our comparison | Important boundary |
|---|---|---|---|
| Gagge two-node | Lumped core + skin; low computational cost | Low-fidelity baseline and ablation | Recent matched comparison found underprediction/error under selected high-workload and hot conditions |
| ISO 7933:2023 PHS | Standards-based heat-strain calculation for an average person; predicts water loss and rectal temperature and exposure limits | Industry / standards comparator | Not a whole-body mechanistic digital twin; standard access and implementation rights must be reviewed |
| Stolwijk-1971 / Stolwijk-2024 | Multi-segment thermal physiology; 2024 variant updates coefficients | Historical and updated multi-node comparator | Exact implementation and reference data rights need pinning |
| JOS-3 | 85-node / 17-body-region multi-node thermophysiology | Leading open-source reference model candidate | Version, model equations, parameters, tests and documented limitations must remain pinned |
| UTCI-Fiala | Higher-resolution multi-node thermophysiology | High-fidelity research comparator | The exact implementation used by a paper may be commercial or access-limited; do not assume redistributable code |
| aJOS-3 | JOS-3-derived extension adding dehydration and heart-rate / cardiovascular strain | Coupled heat–hydration–cardiovascular comparator | An extension with its own assumptions and validation domain, not a drop-in replacement for original JOS-3 |

### 2.1 JOS-3 source audit

The upstream [TanabeLab/JOS-3 repository](https://github.com/TanabeLab/JOS-3) currently reports version 0.5.0 in its Python package metadata. The reviewed upstream master commit at this research pass is:

3c74ee2af2f79aa360093cc517e38bf465ec8c5b

The upstream LICENSE file is MIT. The code requires Python 3 and NumPy. This means:
- We can use it as an external reference implementation under its MIT terms, while retaining copyright and attribution.
- Do not introduce Python into the production Symtropy physiology stack merely to call the reference implementation. It can be run as a separately pinned evaluation oracle, with outputs exported to a rights-compatible immutable fixture/receipt; later a Rust implementation can be compared against those outputs.
- The exact upstream source commit, not a floating master, must be used for every reproduced result.

The upstream README documents body parameters such as height, weight, fat percentage, age, sex, cardiac index, BMR equation and BSA equation; environmental inputs include air and mean-radiant temperature, air velocity, relative humidity, clothing insulation, activity ratio and posture. It supports regional conditions for 17 body regions and transient inputs. The code's own README records model-specific changes: non-shivering thermogenesis is not used in the code path described for Equation 41; version 0.5.0 corrects AVA blood-flow equations; version 0.4.0 changes pelvis capacity. These are important provenance details, not reasons to treat every upstream result as biological truth.

### 2.2 Published comparative evidence

Joshi et al. (2025) compared Gagge two-node, Stolwijk-1971, JOS-3, UTCI-Fiala and Stolwijk-2024 under heat, humidity, workload and clothing variations. The paper reports that multi-node models had average RMSD below 0.3 °C for core temperature and below 0.6 °C for skin temperature in its evaluation, while the two-node model had an average core-temperature RMSD around 1.2 °C in selected conditions above 3.75 met and above 35 °C. These are results for that paper's trials, model versions, conditions and aggregation; they are not universal tolerances or guarantees.

- PubMed record: https://pubmed.ncbi.nlm.nih.gov/39740271/
- DOI: https://doi.org/10.1016/j.jtherbio.2024.104035

The aJOS-3 paper proposes a JOS-3-derived model extended to represent dehydration and heart rate. It is a strong candidate for the next comparison after original JOS-3 is reproduced, especially because it spans the coupling graph we want to exercise. It still needs independent reproduction and careful audit of datasets, assumptions and individualization.

- DOI: https://doi.org/10.1016/j.scs.2025.106436

### 2.3 Standards comparator

[ISO 7933:2023](https://www.iso.org/standard/78240.html) describes the Predicted Heat Strain (PHS) model. The official summary states that it calculates heat stress in terms of water loss and rectal temperature and determines maximum allowable exposure time for an average person against specified strain limits. It is valuable as an industry/standards comparator, but not a mechanistic multi-organ simulator and not a replacement for an individual-specific validation study.

## 3. First integration target: heat–workload–fluid–cardiovascular coupling

Do not attempt full metabolism, immunology, microbiome or brain physiology in the first integrated run.

The minimum coupled demonstrator should be:

1. **Environment / workload (Symtropy):** air temperature, radiant temperature, humidity, air speed, clothing, posture, mechanical work and work/rest schedule.
2. **Thermoregulation (reference model initially):** regional skin/core thermal states, heat transfer, active heat-loss/control mechanisms.
3. **Hydration / fluid bookkeeping:** sweat/water loss and fluid replacement as explicit source/sink events. Do not add renal physiology until it has its own model and validated ports.
4. **Cardiovascular response:** heart rate / cardiac index only through a source model such as aJOS-3 or another separately justified model. Do not infer a cardiovascular state from temperature alone.
5. **Observation process (HOBS):** sensor location, capture/arrival times, noise, missingness, bias and calibration.
6. **Symthaea candidate:** state estimation, uncertainty, value-of-information, and a research policy over non-clinical task/schedule decisions.

The simulated plant advances once per physical interval. Symthaea receives only the measurement events actually available at the decision cutoff.

### 3.1 Port-level exchanges

| From | To | Port | Unit / meaning | Balance / semantic rule |
|---|---|---|---|---|
| Environment | Thermoregulation | air temperature | °C or K, conversion pinned | Boundary condition |
| Environment | Thermoregulation | mean radiant temperature | °C or K | Boundary condition; not equivalent to air temperature |
| Environment | Thermoregulation | relative humidity | fraction or percent, representation declared | Vapor-pressure input |
| Environment | Thermoregulation | air velocity | m/s | Convective heat transfer input |
| Workload | Thermoregulation | metabolic/work rate | W or W/m², exact model definition | Energy input; avoid mixing gross and net metabolism |
| Workload | Cardiovascular extension | workload intensity | declared dimensionless or physical workload mapping | Model-specific response mapping |
| Thermoregulation | Hydration ledger | sweat / water loss | kg or kg/s; model-specific | Water mass balance |
| Hydration ledger | Cardiovascular extension | fluid-volume state / deficit | kg, L or declared model variable | Requires explicit mapping and model validity |
| Thermoregulation | Observation | regional skin/core thermal state | °C/K | Observation is noisy and time-stamped; not raw ground truth |
| Cardiovascular extension | Observation | heart rate | beats/min | Only if model explicitly predicts this output |
| Observation | Symthaea | measurement event | value + unit + mask + precision + capture/availability time | Reuse Symthaea #12407 semantics |

The register remains a catalogue at v0.1. These rows are a proposed interface contract; they are not yet executable typed ports. A follow-up port-schema issue must give every implemented edge an exact source variable, destination variable, unit conversion, time contract, uncertainty and validation receipt.

## 4. Matched benchmark conditions

Freeze the following condition families and ensure every candidate gets the same boundary conditions:

| Family | Controlled variation | Main outputs |
|---|---|---|
| Neutral baseline | Thermoneutral environment, low workload, stable clothing and posture | Initialization convergence, steady-state core/skin temperatures |
| Air-temperature step | Step up/down in air temperature | Core and regional skin response, delay, overshoot, recovery |
| Radiant asymmetry | Change mean radiant temperature independent of air temperature | Segment-specific heat gain/loss |
| Humidity sweep | Dry to humid at matched temperature and air speed | Evaporative capacity, sweating, heat storage |
| Air-speed sweep | Natural/low to forced convection | Convective transfer and regional response |
| Clothing changes | Matched ensemble and regional insulation | Sensible/evaporative resistance sensitivity |
| Workload ramp | Low → moderate → high activity and recovery | Heat generation, core/skin trajectories, sweat, heart rate if supported |
| Work/rest schedule | Intermittent work with matched total work vs continuous work | Thermal accumulation and recovery timing |
| Hydration accounting | Water replacement versus no replacement as a model-only bookkeeping condition | Cumulative fluid balance; no clinical recommendation |
| Delayed/noisy sensors | Same plant, changed sensor process | State-estimation error, calibration, abstention, leakage |
| Parameter perturbation | Body-parameter changes inside and outside declared range | Sensitivity, parameter identifiability, OOD behavior |

No person should be deliberately exposed to heat or dehydration to create this benchmark. Start with reference-model and published data only.

## 5. What to measure

Report per-output, per-condition distributions. Do not aggregate °C error, water-loss error and heart-rate error into one unqualified scalar.

### Model and numerical verification
- steady-state residual and initialization convergence;
- time-step and solver convergence;
- heat-balance residuals with all boundary terms explicit;
- water-balance residuals with input/replacement/sweat/other loss terms explicit;
- positivity and physiological range violations;
- regional-to-whole-body aggregation consistency.

### External prediction
- core-temperature MAE/RMSE and bias in °C;
- regional skin-temperature MAE/RMSE and bias by site;
- sweat/water-loss error in kg or kg/h;
- heart-rate error in beats/min, only when a candidate model supports that output;
- empirical interval coverage, interval width and calibration;
- error distributions by environmental/workload/clothing condition and held-out person;
- worst-condition and lower-quantile performance.

### Coupling and systems
- effect of disabling hydration and cardiovascular coupling;
- parameter and model-form sensitivity;
- update-order and coupling-interval sensitivity;
- p50/p95/p99 runtime, peak memory and energy per simulated hour;
- cost to initialize, personalize and update a model;
- decision regret/value-of-information under an explicitly declared, non-clinical decision loss.

## 6. Competitive strategy

The goal is not to claim that Symtropy is already more physiologically accurate than JOS-3 or aJOS-3. It is to build a reproducible environment and evidence layer that can:
- run several models under identical inputs;
- expose model-form disagreement rather than hide it in averages;
- test missing/delayed/noisy sensor effects separately from plant-model error;
- quantify when extra biological coupling materially changes the result;
- identify when individual parameters are not identifiable;
- compare fidelity versus compute cost and select the lowest-cost model that is adequate for a declared task.

A meaningful advantage would be a better **accuracy–uncertainty–cost–reproducibility frontier**, not a larger number of internal state variables.

## 7. First gate

Before implementing a custom Rust thermophysiology solver:

1. pin and execute JOS-3 0.5.0 at the upstream commit above;
2. capture its neutral, step-response and workload-response outputs from the supplied examples and matched scenarios;
3. independently reproduce the selected JOS-3 reference paper cases;
4. ingest available source-authorized human trial data with participant-level holdouts;
5. compare at least the Gagge and JOS-3 families under identical conditions;
6. document every disagreement and unsupported condition;
7. only then decide whether Symtropy needs a native Rust implementation, an adapter, or a multi-fidelity external-model interface.

## 8. References

- Younes & Khovalyg (2025), 96-model methodological review: https://doi.org/10.1016/j.compbiomed.2025.111218
- Takahashi et al. (2020), JOS-3: https://doi.org/10.1016/j.enbuild.2020.110575
- TanabeLab/JOS-3 source and MIT licence: https://github.com/TanabeLab/JOS-3
- Joshi et al. (2025), comparative model evaluation: https://doi.org/10.1016/j.jtherbio.2024.104035
- aJOS-3 (2025), thermal/hydric/cardiovascular model: https://doi.org/10.1016/j.scs.2025.106436
- ISO 7933:2023: https://www.iso.org/standard/78240.html
- pythermalcomfort model documentation: https://pythermalcomfort.readthedocs.io/en/stable/documentation/models.html
