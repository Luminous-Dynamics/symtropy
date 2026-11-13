# Membrane Fouling Dataset Crosswalk v0.1

**Programme parent:** [#4977 MEMBRANE-FOULING-QUAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/4977)  
**Dataset workstream:** [#4996 MEMBRANE-FOULING-DATASET-CROSSWALK-001](https://github.com/Luminous-Dynamics/symtropy/issues/4996)  
**Status:** source metadata and published rights catalogued; raw archive bytes have not been acquired or hashed in this environment.

## 1. The benchmark problem

Membrane fouling is not one scalar label. A model that predicts TMP, flux or normalized permeability may be useful for state estimation, but that alone does not establish the foulant mechanism, membrane integrity or solute rejection, water quality, whether cleaning is due/effective, or safe/economically optimal control.

Separate UF, NF and RO populations. Their governing equations, pressure/solute effects, cleaning regimes, sensor channels and operating ranges are not interchangeable.

The 2026 mechanism-aligned review by Yang et al. recommends selecting model families based on process and signal structure: GRU for sufficiently dense UF hydraulic signals; regularized XGBoost for multicollinear NF process variables; and CNN–GRU approaches for heterogeneous RO signals. It highlights SCADA noise, sparse data, fallback control, edge deployment, interpretability and the need for plant-scale validation. The implication is not “use the largest model”; it is “prove matched physical and decision baselines before increasing model complexity.” DOI: [10.1016/j.jwpe.2026.110543](https://doi.org/10.1016/j.jwpe.2026.110543).

## 2. Dataset populations

| ID | Process / evidence | Coverage / resolution | Best initial use | Critical limit |
|---|---|---|---|---|
| BRISTOL_POU_UF_34M | Community-scale POU drinking-water system, operational telemetry | 2019-11-01 to 2022-09-27; 34 months; 5-minute rolling averages | Long-horizon UF/POU operational state and permeability/flux forecasting | One installation; metadata describes ORP, free chlorine, flow, discharge pressure, filtration flux and UF permeability; no cleaning/fouling labels or temperature field confirmed by metadata |
| BRISTOL_POU_WQ_12W | 12-week POU trial with telemetry plus accredited-lab samples | 2019-11-01 to 2020-02-29; 5-minute operational series; water-quality samples at 9 timepoints and 3 sample locations | Sample-time-only operating context and quality audit | Sparse lab samples; never interpolate into continuous quality labels |
| MOROCCO_RO_SCADA_6Y | Full-scale industrial RO demineralization SCADA; paper-reported field evidence | Six years reported; raw series available on request | External RO early-warning reproduction if authorized data arrive | No raw data or access permission acquired; paper-reported 15–20-day warning is not independently reproduced |
| ARMSTRONG_RO_CROSSFLOW_2022 | Controlled RO transport-property experiment | Two membranes, replicate cells, 8 applied pressures from 4.14 to 33 bar; multiple solutes | M0 transport calculation / concentration-polarization / rejection simplification baseline | Lab transport experiment, not a long-run fouling or CIP event series; raw-data file licence still needs repository-level confirmation |

### Bristol long-term telemetry

Clayton, Thorn, Fox & Reynolds (2024), *Operational telemetry data of a community-scale decentralized point-of-use drinking water treatment system, Bristol, UK, November 2019–September 2022*, DOI [10.5285/d3daab67-dc02-43a0-82c0-afff709c43f4](https://doi.org/10.5285/d3daab67-dc02-43a0-82c0-afff709c43f4).

EIDC metadata says the system was at the University of the West of England Frenchay campus; the 34-month CSV series has 5-minute rolling averages from in-line sensors/probes. It lists ORP, free available chlorine, flow rate, discharge pressure, filtration flux and UF membrane permeability. The resource is explicitly under Open Government Licence v3 (OGL). EIDC requires this attribution: **“Contains data supplied by UK Centre for Ecology & Hydrology.”** Its metadata also requires citation of the dataset and key publications. The EIDC record was updated 5 November 2026.

Primary record: https://catalogue.ceh.ac.uk/documents/d3daab67-dc02-43a0-82c0-afff709c43f4  
Metadata JSON: https://catalogue.ceh.ac.uk/documents/d3daab67-dc02-43a0-82c0-afff709c43f4?format=json  
Supporting information: https://data-package.ceh.ac.uk/sd/d3daab67-dc02-43a0-82c0-afff709c43f4.zip  
Data package: https://data-package.ceh.ac.uk/data/d3daab67-dc02-43a0-82c0-afff709c43f4.zip

### Bristol sample-linked water-quality trial

Fox, Thorn, Clayton & Reynolds (2022), *Operational and water quality data from a decentralised, low-maintenance, drinking water treatment system trial, UK, November 2019 to February 2020*, DOI [10.5285/98148025-e8ef-40f9-907e-dae8b28106d2](https://doi.org/10.5285/98148025-e8ef-40f9-907e-dae8b28106d2).

EIDC metadata describes 12 weeks of operation, 5-minute telemetry with average values and min/max where available (flow, UF TMP, ORP and free chlorine), plus sample summaries from three treatment-chain locations at nine actual timepoints. Water samples were analysed by an accredited UK laboratory; UKAS sampling protocols and method details are referenced in supporting documentation. The data include biological, physicochemical, chemical, metals, hydrocarbons and disinfection byproducts. The resource is explicitly OGL v3. Required EIDC attribution: **“Contains data supplied by UK Centre for Ecology & Hydrology.”**

Primary record: https://catalogue.ceh.ac.uk/documents/98148025-e8ef-40f9-907e-dae8b28106d2  
Metadata JSON: https://catalogue.ceh.ac.uk/documents/98148025-e8ef-40f9-907e-dae8b28106d2?format=json  
Supporting information: https://data-package.ceh.ac.uk/sd/98148025-e8ef-40f9-907e-dae8b28106d2.zip  
Data package: https://data-package.ceh.ac.uk/data/98148025-e8ef-40f9-907e-dae8b28106d2.zip

The nine laboratory timepoints are sparse ground truth. Only actual sample timestamps, locations, analytes, methods, detection limits and lab report values can become quality targets. Do not forward-fill or interpolate laboratory outcomes into 5-minute quality labels.

### Morocco industrial RO SCADA

Elyaakouby, El-Kordy & Tilioua (June 2026), *Analysis and diagnostic of reverse osmosis membrane fouling using industrial plant data*, Cleaner Water 6, 100247, DOI [10.1016/j.clwat.2026.100247](https://doi.org/10.1016/j.clwat.2026.100247).

The paper reports six years of SCADA from a Moroccan industrial demineralization facility. It describes selected deviations in differential pressure, conductivity and pretreatment pressure differentials as precursors to performance degradation, with a reported 15–20-day warning window and SEM–EDS autopsy evidence consistent with organic fouling. Its Data Availability Statement says data are available on request. Raw data, tags, sampling cadence, event dates, cleaning records, train/test protocol and access terms remain unverified. This is the most valuable external RO replication target if the authors grant access, not a public file admitted by this register.

### Armstrong RO cross-flow transport dataset

Armstrong, Vickers & Coronell (2022), *Dataset of reverse osmosis membrane transport properties calculated with and without assumptions about concentration polarization and solute rejection and the errors associated with each assumption*, Data in Brief 44, 108538, DOI [10.1016/j.dib.2022.108538](https://doi.org/10.1016/j.dib.2022.108538); OSF project [10.17605/OSF.IO/PU5AJ](https://doi.org/10.17605/OSF.IO/PU5AJ).

The paper describes cross-flow tests on ESPA3 and SWC4+ membranes, replicate samples, eight pressures (approximately 4.14–33 bar), NaCl, As(V), As(III) and boron feed solutes, permeate flux, rejection, concentration polarization, water permeance, solute permeance and water/solute selectivity. It explicitly quantifies errors from common simplifying assumptions, including negligible concentration polarization and near-complete rejection. This is a transport-equation / calculation-validation corpus, not a multi-month fouling or cleaning-event population. The paper licence is CC BY-NC-ND 4.0; the OSF project's dataset-specific terms must be confirmed before raw file reuse or redistribution.

Primary article: https://pmc.ncbi.nlm.nih.gov/articles/PMC9436753/  
Dataset record: https://osf.io/pu5aj/

## 3. Admission ledger

All four records remain catalogued-metadata-only in v0.1. The two EIDC records have verified public record-level OGL v3 rights; the Morocco SCADA is request-only; the Armstrong paper is CC BY-NC-ND but OSF file/project licence terms are not confirmed. None has acquired bytes, an archive SHA-256 or a split manifest in this branch.

The EIDC ZIPs could not be retrieved in this execution environment because its outbound DNS resolution failed. This is a tooling/access failure, not a reason to fabricate an archive digest or claim file inspection. The public download links remain recorded for reproducible acquisition from an environment with network access.

## 4. Paradigm shift: fouling as a latent, mechanism-conditioned state

Do not train one universal binary “fouled/not fouled” classifier.

Represent the observed process as a combination of distinct channels:
1. **Hydraulic state:** TMP or stage differential pressure, flux, flow, feed/concentrate/permeate pressure where available.
2. **Transport/chemistry state:** osmotic pressure, feed/permeate conductivity, temperature/viscosity, pH, turbidity/organics where actually measured.
3. **Membrane state:** membrane area, age, model, module/serial, integrity/rejection, accumulated volume and cleaning history.
4. **Sensor/measurement state:** units, calibration, uncertainty, timebase, missingness, probe fouling, tag changes and historian quality.
5. **Maintenance/intervention state:** backwash/CIP/flush, chemical, concentration, temperature, duration, pressure, operator and post-cleaning response.
6. **Service outcome:** conforming permeate volume, quality samples, energy, reject/concentrate, downtime and cost.

The latent state should distinguish reversible concentration polarization, reversible cake/gel, pore blocking, inorganic scaling, organic fouling, biological fouling, membrane compaction/aging, membrane damage and sensor drift—only to the extent evidence makes them identifiable. Otherwise output INDETERMINATE_MECHANISM.

## 5. Benchmark tracks

- **M0 — transport calculation and measurement validity:** Armstrong RO cross-flow data; equation and unit tests for net driving pressure, concentration polarization, permeance, selectivity, rejection and uncertainty.
- **M1 — long-horizon UF telemetry forecasting:** Bristol 34-month series. Start with next-step and registered-horizon flux/permeability forecasting; do not claim labelled fouling-event detection unless cleaning/fouling event truth is found in the package or supporting records.
- **M2 — sample-time water-quality audit:** Bristol 12-week telemetry plus nine lab sample timepoints across three locations. Align actual sample metadata only. Quality targets do not become dense time-series labels.
- **M3 — industrial RO external replication:** Moroccan six-year SCADA. Blocked until raw data and event/cleaning tags are received under permission. Reproduce the paper's 15–20-day warning result only with its original population and protocol.
- **M4 — cross-site/module transfer:** future installations only; hold out complete sites, membrane modules, feed regimes and maintenance cycles.

Do not pool UF and RO metrics into a single accuracy number.

## 6. Matched baseline ladder

All methods use the same feature availability, causal data cutoff, chronological splits, prediction horizon, missingness policy and held-out events.

1. Scheduled maintenance / fixed cleaning interval.
2. Operator-set fixed TMP/pressure/flux threshold where site's SOP is available.
3. Robust local trend / EWMA / CUSUM and persistence baseline.
4. Mechanistic transport/resistance baseline with feed/temperature/viscosity correction where required inputs are measured.
5. Linear or autoregressive forecasting.
6. Regularized XGBoost for tabular, multicollinear process data.
7. GRU for sufficiently dense UF sequences.
8. Hybrid physics-informed state model with residual learner and calibrated prediction intervals.
9. CNN–GRU or graph/time-series fusion only when spatial/module topology and dataset size justify it.

A model does not beat industry because it has higher random-window accuracy or a larger architecture. It must outperform the best matched operational baseline at an equal quality/integrity and intervention-risk constraint.

## 7. Validation and splits

- Use strict chronological splits. No random row or overlapping-window shuffle across train/test.
- Purge a time gap at least as long as the largest feature lookback/target horizon.
- Group complete cleaning and maintenance cycles in one partition when their IDs and times exist.
- Use a final untouched temporal holdout for the Bristol long-run series.
- Use leave-one-module/site/feed-regime-out evaluation only when those group labels genuinely exist.
- Fit all scalers, imputers, feature selection and threshold calibration on training/calibration partitions only.
- Missing or offline sensors remain missing; never convert them into normal values.
- If a dataset has no verified fouling/cleaning events, report forecasting performance only and mark event detection / cleaning recommendation NOT_EVALUABLE.

## 8. Metrics

**Forecasting:** MAE, RMSE, MASE, horizon-specific error, prediction-interval coverage and width, calibration error and worst-regime error.

**Event warning (only with audited event labels):** event precision/recall/PR-AUC, false alerts per module-day or module-week, warning lead-time distribution, missed-event rate and abstention. Report at fixed false-alert burden.

**Transport/quality:** temperature-normalized permeability only when temperature and correction basis are measured; RO water permeance on net driving pressure; solute rejection by analyte and method; membrane integrity; sample-level quality pass/fail only when actual lab reference limits and uncertainty are available.

**Cleaning and lifecycle:** post-CIP permeability/flux recovery; irreversible resistance trend; chemical consumption, cleaning water, downtime, labour, membrane replacement and cleaning interval; lifecycle cost per m³ of profile-conforming permeate.

**Energy/materials:** kWh/m³ of conforming permeate, water recovery, concentrate/reject volume, chemical intensity and service availability. Raw production volume is not a substitute for delivered conforming water.

## 9. Proposed improvement goals (not achieved)

- Reproduce the six-year industrial RO paper's 15–20-day warning only after acquiring the original signal population and event protocol; then use a chronologically separate holdout and report site-specific calibration.
- On Bristol UF forecasting, beat persistence, trend and mechanistic baselines at every registered horizon, not merely at one aggregate average.
- Seek at least 10% reduction in false cleaning alarms at equal event recall and equal quality/integrity guardrails on an audited event population.
- Seek at least 10% lower cleaning energy/chemical burden or at least 10% additional profile-conforming permeate per cleaning cycle in a matched scenario, without worsening integrity, quality, service availability or membrane life.
- Demonstrate calibrated uncertainty and abstention on unseen operating regimes; target 90% empirical coverage for nominal 90% prediction intervals on the untouched holdout.
- Publish a complete Pareto frontier (energy, permeate volume, quality, recovery, chemicals, cleaning frequency, downtime, cost) rather than a single accuracy claim.

These are experimental goals, not achieved results or guarantees.

## 10. Data admission and next execution order

1. Acquire each EIDC ZIP from the official links, preserve original bytes, SHA-256 archive, licence notice and complete package tree.
2. Parse README/supporting docs first; inventory CSVs, columns, units, timestamps, quality codes and missingness.
3. Reconcile long-run and trial records: they refer to a related treatment system/site, but remain distinct releases, time ranges and evidence products until file-level record linkage proves overlap.
4. Audit the nine lab timepoints and three sample locations; capture actual sample dates, methods, units, LOQ/LOD, lab accreditation scope and treatment stage.
5. For Armstrong, inspect OSF project metadata/file licence and workbook sheets before admission; preserve source workbook unchanged.
6. Request the six-year Moroccan SCADA with tag dictionary, raw cadence, SOPs, CIP/maintenance events, feedwater regime, membrane train/module, instrument calibration and paper split/protocol.
7. Freeze one M0 transport baseline and one M1 UF trend forecast before any deep model.
8. Only after those steps, create a reproducible benchmark report with independent evidence receipts.

## 11. Nonclaims

No raw archive from any of these datasets has been acquired or hashed in this branch. No model has been trained, no CIP action evaluated, no fouling mechanism identified, no water-quality or potability result established, and no industrial performance improvement claimed. A paper-reported metric is not a reproduced benchmark result.


## M0 methodology caveat and next increment

Armstrong et al. calculate their reported reference water permeance as the slope of permeate flux versus net driving pressure across the full pressure series, with the intercept fixed at zero. The current five synthetic M0 cases verify pointwise J/NDP arithmetic only; they do not reproduce the multi-pressure fitted reference-A procedure. Exact reproduction remains blocked until the authorized OSF workbook is acquired and the replicate series are grouped. The next M0B task is a grouped through-origin regression with replicate-level uncertainty. Source: https://doi.org/10.1016/j.dib.2022.108538.
