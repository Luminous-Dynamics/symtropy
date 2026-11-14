# Water Mixing SOTA Benchmark Matrix v0.1

**Programme:** WATER-MIXING-FIDELITY-QUAL-001 / #5216  
**Sub-qualifications:** experiment design #5226; mass accounting #5242; physical benchmark bridge #5253  
**Evidence date:** 14 November 2026  
**Purpose:** matched, evidence-graded comparison of network solvers, local mixing models, sensors, physical experiments and qualification architecture.

## 1. Benchmark the task, not the product label

There is no single meaningful water-mixing leaderboard. At least five different tasks are often mixed together:

1. Network hydraulic state and network-scale water-quality routing.
2. Local junction outlet-wise scalar mixing.
3. Tank residence-time distribution and mixing dynamics.
4. Experimental design and sensor/sampling placement.
5. Evidence qualification: uncertainty, provenance, domain validity, mass recovery and abstention.

A model may be strong on one task and weak or untested on another. Network leak detection scores, water-demand forecasting coverage, CFD field resolution and physical junction tracer accuracy are not interchangeable metrics.

The programme should publish task-specific scorecards and an evidence-class label for every result.

## 2. Comparator matrix

| Comparator | Task / established role | What to measure fairly | Evidence / limitation |
|---|---|---|---|
| EPANET 2.2 | Network hydraulics and water-quality reference; complete instantaneous junction mixing; tank MIXED, 2COMP, FIFO, LIFO | Network parity, outlet concentration error, mass-balance residual, tank RTD error | Open reference and reproducible, but the declared local mixing abstractions are not a physical junction/RTD measurement |
| Ho (2008) bulk-mixing model | Reduced-order junction alternative / bounds | Prediction error and bound coverage on paired geometries and flow conditions | Published analytical comparator; applicability is geometry/regime dependent |
| Shao et al. (2014) junction experiments | Physical evidence on mixing across junction types | Outlet concentration profiles, flow split/momentum-ratio effects and repeatability | Published experimental evidence; obtain raw data or digitize only with transparent uncertainty and rights review |
| Wéber et al. (2026) CFD/regression | Geometry-aware incomplete cross-junction prediction | Regression vs EPANET/Ho/CFD on held-out angles, flow splits and geometry families | Online 19 September 2026; reported regression validated on literature experiments. Supporting data/models are available on reasonable request, so full reproduction requires access |
| Geometry-specific CFD | High-resolution local flow and scalar-field hypothesis | Outlet integrated flux, field profiles, mesh/time-step/turbulence/scalar-diffusivity sensitivity, wall and boundary sensitivity | CFD is a comparator, not ground truth; physical tracer observations remain the independent anchor |
| OpenFOAM scalar transport | Open-source passive-scalar transport implementation | Solver-version parity, selected discretization/limiter, scalar boundedness, mass conservation and runtime | ScalarTransport supports flux, diffusivity and sources. Version/configuration matters; v2606 function-object docs say boundedness depends on the upwind scheme, while current development API exposes MULES/subcycling options |
| WaterGEMS / WaterCAD | Commercial network-scale analysis and operational workflow | Exact engine/version compatibility, water-quality mass balance, tank options, runtime and reproducibility against EPANET | Broad network capability; engine mode and exact build matter. Bentley's current compatibility article states that WaterGEMS 2.2.0 uses the updated EPANET 2.2.0 water-quality calculations beginning with product version 24.00.02.20, while older compatibility modes retain EPANET 2.00.12 behavior. Another Bentley help page has a feature table that appears to say the improved mass-balance algorithm is not included, so the official documentation is inconsistent across pages. Treat exact build and mode as a mandatory differential test dimension; do not claim a product-wide limitation. Record engine compatibility and algorithm details, not just product name. |
| Tank CSTR / 2COMP / FIFO / LIFO | Reduced-order tank comparators | Outlet RTD curve, early breakthrough, mean/variance, tail fraction, quantiles and mass recovery | EPANET options are useful baseline models, not universal physical descriptions |
| Series/parallel compartments and empirical RTD | Reduced-order/measurement-derived tank model | Blind pulse/step response, tail prediction, repeatability and transfer across level/flow schedules | Crowther & Dandy (2012) found their series/parallel model and CFD compared well against one clear-water tank tracer study; that result is not a universal tank ranking |
| Random / greedy / D-optimal / E-optimal / discrimination-only | Experiment/sensor selection baselines | Identifiability, model separation, worst-case information and cost on identical candidate sets | Equal candidate universe, equal budget, declared parameter scales and repeated random seeds required |
| Adaptive sampling / conformal prediction | Adjacent sensor-allocation and uncertainty methodology | Coverage, interval width, sampling cost, calibration samples and regime-shift behavior | 2026 demand-forecasting work is not local mixing. Use it as a methodology comparator, not an apples-to-apples mixing accuracy score |


Two Bentley references must be read together: the WaterGEMS 2026 Help engine compatibility page https://docs.bentley.com/LiveContent/web/OpenFlows%20WaterGEMS-v2026/Help/en/topics/1481500/GUID-CFCC6D24-2729-4DB8-BB13-70A642ADECB1.html and the current OpenFlows compatibility article https://bentleysystems.service-now.com/community?id=kb_article_view&sysparm_article=KB0014547. The latter states that WaterGEMS 2.2.0 uses updated EPANET 2.2.0 water-quality calculations from version 24.00.02.20 onward, addressing EPANET water-quality mass-imbalance behavior; an alternate feature-table help page appears inconsistent. This creates a better benchmark: test exact version/build and engine mode on matched WQ cases, rather than assuming a blanket incompatibility. Include legacy WaterGEMS 2.00.12 and EPANET 2.2.0 modes, plus WaterGEMS 2.2.0 before/after 24.00.02.20 if those builds can be obtained. Record water-quality time step, report version and any unsupported options.

## 3. What the latest literature implies

### 3.1 Junction models

The Wéber et al. paper published online on 19 September 2026 is an important direct comparator. It varies cross-junction connection angle and inflow/outflow distribution using parametric CFD, then develops a lower-cost predictive regression model. The abstract reports that angle and relative flow rates are dominant in the studied cases, while molecular diffusivity and turbulent Schmidt number had more limited influence there. The paper says the model was validated against literature experiments and can integrate with network solvers.

Our benchmark must not assume that the new local transfer matrix is automatically better than this regression. Implement the published comparator if the authors' data/model become available, or explicitly label a reconstruction as approximate. Test whole-angle and whole-flow-regime holdouts.

Source: https://doi.org/10.1061/JHEND8.HYENG-14715

### 3.2 Tank residence-time behavior

A single CSTR is a useful baseline, not a default truth. The 2012 clear-water tank tracer comparison by Crowther & Dandy found that the CFD and their series/parallel compartment model compared well with the study's measured concentrations, with the single-CSTR comparator performing worst in that experiment. We should use this as motivation to test RTD families, not as a universal assertion that series/parallel models always win.

Source: https://doi.org/10.7158/W10-835.2012

### 3.3 Sensor placement and uncertainty transfer

The 2026 conformal-prediction adaptive-sampling paper reports empirical coverage of approximately 89.4–90.2% on water-network demand forecasting tasks with a modest computational overhead. A separate 2026 multi-domain demand-forecasting UQ study reports 95.1% coverage and reduced calibration burden across six villages. These are adjacent results, not physical mixing benchmarks. Their value is methodological: regime-aware calibration, sample-efficient adaptation, uncertainty coverage and adaptive sampling need matched controls.

Sources:
- https://doi.org/10.1016/j.icte.2026.01.010
- https://doi.org/10.1016/j.mlwa.2025.100812

### 3.4 Public network benchmarks

The University of Exeter Centre for Water Systems benchmark collection provides EPANET network inputs and associated problem metadata across expansion, layout, operation, design and resilience. DiTEC-WDN contains 36,000 simulated scenarios over multiple networks and 228 million graph states. These are valuable for network-level integration, scenario generation and held-out network tests. They do not establish physical junction mixing or tank tracer truth.

Sources:
- https://www.exeter.ac.uk/research/centres/cws/resources/benchmarks/
- https://doi.org/10.1038/s41597-025-06026-0

### 3.5 Physical benchmark opportunity

Politecnico di Milano FluidLab describes E-NET as an experimental water distribution network based on the Jowitt–Xu benchmark. The page lists three inverter-driven pumps, 25–100 mm diameters, 0–7 L/s demand, approximately 1.1 km total length and an EPANET input file available for download/contact. The 2026 publication presents a numerical scaled counterpart and explicitly treats subsequent controlled physical experiments as future validation.

E-NET is a strong candidate host, but no source reviewed here confirms installed tracer injection hardware, concentration sensors on every outlet, a tank RTD apparatus or mass-recovery equipment. Confirm those capabilities and permission before scheduling any T1 trial.

Sources:
- https://www.fluidlab.polimi.it/exp/
- https://doi.org/10.1007/s11269-026-04785-y

## 4. Fair benchmark protocol

Every pairwise comparison must freeze:

- exact geometry and connection revisions;
- inlet/outlet flow rates and mass-flux histories;
- tracer, temperature, viscosity, molecular diffusivity and scalar boundary conditions;
- initial tank state, level and inlet/outlet schedule;
- injection waveform and recovery window;
- sensor placement, calibration, uncertainty, response time, sample-line delay, sample cadence and clock mapping;
- solver version, configuration, numerical scheme, mesh and time step;
- train/calibration/holdout identity and raw-data digests.

Do not compare different geometries, flow splits, tracer diffusivities, sensor bandwidths or boundary conditions and report the difference as model superiority.

For stochastic optimizers, use repeated seeds and publish dispersion. For physical experiments, use randomized run order where possible, blinded holdout conditions, repeat trials and a documented measurement-uncertainty budget.

## 5. Metrics

### Junction

- outlet-wise concentration MAE, RMSE and normalized error;
- outlet peak error and peak-time/arrival-time error;
- integrated tracer mass-flux error per outlet;
- outlet cross-contamination matrix error;
- residual mass and recovery fraction;
- uncertainty coverage, width, CRPS or weighted interval score;
- out-of-domain detection and selective risk/coverage curve.

### Tank

- full RTD-curve distance, including Wasserstein distance where curves are normalized only after recovery is separately reported;
- first-arrival and early-breakthrough error;
- mean residence time and variance error;
- tail-mass fraction and late-quantile error;
- independently measured inventory and unrecovered mass;
- pulse/step consistency and repeatability.

### Experiment design

- singular spectrum, numerical rank and threshold sensitivity;
- condition number and weakest information direction;
- nuisance-adjusted target information;
- model-separation score;
- D-/E-optimal criteria;
- Pareto frontier across information, discrimination, instrument cost, sensor count and duration;
- performance vs random, greedy and adaptive-sampling baselines under equal budgets.

### Evidence and efficiency

- per-stratum empirical coverage and interval width;
- false-safe count and risk/coverage curve;
- abstention rate and reason codes;
- raw-data/provenance completeness;
- calibration effort and number of physical runs;
- runtime, peak memory, solver cost and instrumentation cost.

Do not publish only a pooled average. Stratify by geometry family, flow split/momentum ratio, Reynolds/Péclet regime, tank configuration, sensor layout and facility.

## 6. Proposed target ladder (research targets, not achieved)

| Stage | Target |
|---|---|
| T0 arithmetic | 100% of frozen known-answer/refusal cases matched; zero false-identifiable and false-closure outputs |
| T0 numerical | Deterministic ranking and Pareto replay; unit/scale and row-permutation invariants |
| T1 physical | Nominal 90% predictive interval coverage on preregistered blind trials, with interval width and per-stratum coverage published |
| T1 safety corpus | Zero false-safe decisions in the frozen high-consequence control corpus |
| T1 mass balance | Residual within combined measurement uncertainty, or explicit open/partial/indeterminate status |
| T2 geometry transfer | Leave-one-junction-family-out and leave-one-operating-regime-out evaluation |
| T2 facility/scale transfer | Hold out at least one facility/scale when a physically comparable dataset exists |
| Efficiency | Reduce sensor/calibration/test cost without degrading blind physical prediction or uncertainty calibration |
| Compute | Publish accuracy–uncertainty–runtime–instrumentation Pareto frontier against reduced-order and CFD comparators |

A nominal 90% coverage result is not enough by itself. Report sample count, interval width, abstention, calibration-set size and whether the holdout is genuinely independent.

## 7. How to outperform the field

Do not set the objective as “beat CFD” or “replace EPANET.” Build a composable qualification layer around the established tools:

1. EPANET or WaterGEMS remains the network-scale model and baseline.
2. Ho, Wéber-style regression, compartment models and geometry-specific CFD are local competing hypotheses.
3. Tracer data independently constrain the local operator and the RTD.
4. The experiment planner chooses tests that distinguish the hypotheses, not merely add more sensors.
5. The evidence layer checks calibration, provenance, mass recovery, uncertainty and validity-domain membership.
6. The network model consumes the local operator only inside its qualified geometry/flow envelope; otherwise it expands uncertainty or abstains.

The competitive claim we can eventually test is not necessarily lower raw CFD error. It is lower total cost to reach a defensible prediction, at equivalent or better blind physical accuracy and calibrated uncertainty, with reproducible evidence and honest abstention.

## 8. Current evidence boundary

The 18-case mass-accounting corpus, 6-candidate design corpus, 15 junction/tank mixing-fidelity cases and 10 identifiability cases are synthetic T0. They do not constitute physical T1 evidence. No T0 pass establishes a real junction mixing operator, tank RTD, tracer compatibility, sensor calibration, chemical fate, potability, contamination or release authority.
