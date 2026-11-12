# EV battery process-window robustness benchmark v0.1

Status: synthetic contract and read-only evidence auditor. No physical production yield, process capability, causal effect, safety, lifetime or cost improvement is claimed.

## Objective

Determine whether an exact EV battery cell configuration and manufacturing recipe produce conforming cells across independent lots, production lines and equipment under realistic process variation. A single high-performing cell or nominal recipe is not evidence of robust production.

## Canonical owners

- Symthaea MFG-PROC #5686 owns generic process identity, capability, recipes and executed-process semantics.
- Symthaea ENG-UQ #5636 owns general tolerance campaigns, variation source classes and robust-design summaries.
- Symthaea MAT-013 #4344 owns physical sample/process/characterization lineage.
- Symthaea MAT-PSPP #6224 owns process → realized structure → property → performance lineage.
- Symthaea MAT-UQ #6225 owns cross-fidelity uncertainty and calibration.
- Symtropy EV battery adapter #4791 / PR #4792 binds those upstream references to EV cell/pack/mission outcomes.
- Symthaea EV-FIELD-EVIDENCE PR #11421 owns the reusable battery dataset permission/evidence registry; consume its per-operation rights semantics rather than creating a second rights engine.

This benchmark owns only the battery-specific application question and its cell-level lot/line acceptance protocol. It creates no duplicate process ontology, tolerance engine, genealogy store, materials ontology, metrology authority, process optimizer or factory controller.

## Process scope

Depending on chemistry and manufacturing route, the declared process family may include slurry mixing or dry-electrode processing, coating, drying, calendering, slitting, electrode handling, assembly, electrolyte filling/wetting, formation, aging and end-of-line inspection. Process parameters remain namespaced, versioned and unit-bound references; do not assume one universal recipe schema across wet/dry electrodes or different chemistries.

## Paradigm shift

```text
nominal recipe / best cell
    -> fixed process-window hypothesis
    -> exact realized run records by line and lot
    -> characterized structure and measured cell quality
    -> per-lot and per-line conformance
    -> robust process-window evidence or explicit UNKNOWN
```

Nominal performance, model prediction, simulation, physical observation, first-pass yield, process capability and production qualification are separate claims.

## Benchmark rules

1. Bind exact recipe identity/revision and cell configuration on every run.
2. Bind each process parameter to a unit, declared window, exact equipment/line and an evidence reference.
3. Bind each quality metric to a unit, cell/system boundary and an explicit lower/upper specification limit.
4. Use only evaluation-admitted physical measurement / independent-laboratory evidence for physical first-pass-yield claims.
5. Keep synthetic/simulation/surrogate/vendor/literature evidence in separate evidence tiers.
6. Out-of-window runs are separately counted; they remain in the strict first-pass-yield denominator and cannot count as process-conforming passes. A separate process-window adherence rate and conditional complete-measurement conformance rate expose the reason for non-conformance.
7. Missing, incomplete or invalid inspections are not passes; they are counted separately.
8. Report first-pass conformance independently by lot and line. A large number of cells from one lot does not create many independent manufacturing replications.
9. Do not report Cp/Cpk/Pp/Ppk unless process stability, measurement-system suitability, distributional assumptions, subgrouping and specification limits are defensible.
10. Do not infer causal process improvements from observational correlation alone.

## Holdout tournament

- leave-one-lot-out;
- leave-one-line/equipment-out;
- chronological process-drift holdout;
- chemistry/process family holdout only where independent support exists;
- prospective blinded line/lot validation.

Compare frozen recipe/SPC, DOE/response-surface, physical/statistical, tree/boosting and process-aware hybrid methods under identical feature availability and group holdouts. Any learned process recommendation stays shadow-mode; this adapter does not authorize setpoint changes, equipment programs, formation waveforms or physical testing.

## Metrics

- strict process-conforming first-pass yield: passes within the declared process window divided by all declared runs (incomplete and out-of-window runs cannot count as passes);
- per-lot and per-line conforming/nonconforming/incomplete counts;
- between-lot and between-line variation and worst-group performance;
- out-of-window process-run frequency;
- defect, scrap and rework burden;
- cycle-life distribution only when linked to actual ageing endpoints or a separately qualified censored prognostics result;
- accepted kWh per process energy, water, solvent and consumables under explicit boundaries;
- cross-lot/line generalization, false acceptance/rejection and abstention;
- inspection/data/compute burden.

## Industry comparison and matched baselines

Battery digital twins, manufacturing traceability and predictive quality are already commercial or active research areas; do not position the existence of a digital twin as the differentiator.

- **Siemens Battery Production / Xcelerator:** markets production digital twins, virtual commissioning/ramp-up, process integration, MES/QMS, traceability, predictive quality and closed-loop process adjustments. These are competitor capability claims and product descriptions, not independent matched performance measurements: https://www.siemens.com/en-us/digital-thread/smart-manufacturing/battery-production/
- **EU BatCAT project:** combines data-driven and physics-based models, interpretable industrial decision support, ontology/formal-method safeguards and pilot-line validation across Li-ion/Na-ion coin-cell and redox-flow battery technologies. This is a research-project baseline, not evidence of commercial fleet-scale performance: https://cordis.europa.eu/project/id/101137725
- **ORNL electrode-densification anomaly detection:** describes sensor/vision-based real-time detection and adaptive feedback control for electrode coating densification. It is a relevant technical competitor, but the public technology summary does not supply a matched, independently audited yield score: https://www.ornl.gov/technology/202506000
- **Roll-to-roll coating AI study (Manufacturing Letters 49, Oct 2026):** reports five continuous-production cases, maximum holdout R² of 0.966, four of five cases at or above 0.95, and RMSE ranging 20.125–52.097 for mass-loading prediction. This is a task-specific paper-reported regression baseline; do not treat it as leave-one-line-out transfer or proven yield improvement: https://doi.org/10.1016/j.mfglet.2026.07.013

### Where Luminous should compete

1. Match or reproduce the published regression task with the same response variable, units, split and raw-data grouping before comparing headline R²/RMSE.
2. Add strict leave-one-lot-out and leave-one-line/equipment-out evaluation; report the number of independent lines/lots, not just electrode strips or cell rows.
3. Predict calibrated distributions and abstain under OOD, rather than emitting only a point mass-loading estimate.
4. Connect the process prediction to characterized structure, complete-cell quality and subsequent lifetime under exact configuration lineage.
5. Measure whether recommended actions improve strict first-pass process-conforming yield, scrap, throughput and resource use on later blinded lots; use a fixed-recipe/SPC baseline and report uncertainty.

That end-to-end evidence chain—not a larger R² on an easier random split—is the proposed competitive wedge.
## Public dataset entry points and rights

The following sources offer a concrete retrospective benchmark ladder. Their licence and target boundaries must remain bound to every derived snapshot; none alone establishes multi-line industrial yield.

| Dataset | Scope and useful variables | Rights / permitted path |
|---|---|---|
| Warwick frequency-domain ultrasonic electrode dataset, DOI 10.17632/c62yn37d9h.4 (version 4, 15 Dec 2025) | Graphite anode and NMC622 cathode; before/after calendering FFT frequency/magnitude arrays, average thickness across three regions, web speed, roll gap, coat weight, calendering speed and density. Electrode-level NDT/process-to-structure task, not cell yield. | CC BY 4.0; attribution and exact version required. https://data.mendeley.com/datasets/c62yn37d9h/4 |
| Warwick NMC622 pilot-plant calendering dataset, DOI 10.17632/wwhm2frfmy.1 (version 1, 24 Jul 2023) | Coating/calendering operating conditions, electrode physical properties before/after calendering and related electrochemical cell performance. A useful pilot-scale process/structure/property task. | CC0 1.0 on the dataset record. https://data.mendeley.com/datasets/wwhm2frfmy/1 |
| Graphite electrode formulation/manufacturing dataset, DOI 10.17632/4dh2h3tsf4.1 (version 1, 10 Jan 2022) | Related paper describes slurry composition/mixing, coating gap, drying temperature, coating speed and calendering, with rheology, adhesion and electrochemical outcomes. Verify exact contents and data dictionary before defining targets. | CC BY 4.0 on the dataset record. https://data.mendeley.com/datasets/4dh2h3tsf4/1 |
| TRI-AMDD formation/aging dataset, Data.MATR 8 | 186 pouch cells and 62 formation protocols in the associated Joule study; raw formation records and structured ageing cycles. Useful for protocol → early formation signature → cycle-life reproduction, not electrode-line yield. | CC BY-NC 4.0 on the dataset record: research/non-commercial use only unless additional permission is obtained. https://data.matr.io/8/ |

### Recommended reproduction order

1. **Ultrasonic NDT:** compare the published feature pipeline with simple frequency-domain baselines; hold out physical electrode sample IDs and pre/post-calendering pairs together where needed. Predict thickness first, then examine density/porosity transfer only where the measurements support those labels.
2. **Pilot NMC622 calendering:** reproduce a process → electrode property → half-cell/cell response chain; separate each response boundary and do not pretend half-cell performance is complete-cell EV performance.
3. **Graphite formulation:** verify the actual file inventory, exact process factors and sample identities before admitting it into a common benchmark.
4. **Formation:** reproduce the 186-cell/62-protocol study as a distinct non-commercial track; use group-aware splits by protocol and cell, and keep its NC licence out of commercial model training or deployment until separately authorized.

The CC BY and CC0 entries are candidates for admission with their stated attribution/rights, source hashes and exact dataset versions. The CC BY-NC formation data must be separately tagged COMMERCIAL_USE_BLOCKED unless new permission is obtained. Use the existing Symthaea EV-FIELD-EVIDENCE PR #11421 permission registry for each intended operation (local processing, model training, derived-data publication, model-weight release, commercial use and redistribution); a dataset's general licence label must not be treated as blanket permission for every operation. No raw dataset has been copied into this repository by this PR.

Do not merge these four datasets into one pooled score: electrode ultrasonic regression, pilot-plant NMC process/property data, graphite process/formulation records, and formation-to-cycle-life are different tasks, physical units and evidence boundaries. They can contribute to a multi-fidelity evidence graph, not a fake common leaderboard.

## Candidate research targets

- zero recipe/configuration/line/lot/evidence-class pooling without explicit compatibility;
- zero synthetic or model-predicted outputs counted as observed physical yield;
- zero incomplete/invalid inspection records silently counted as conforming;
- zero cell/lot/line holdout leakage;
- after physical baseline and power analysis, investigate ≥5 percentage-point first-pass-yield increase or ≥20% reduction in between-lot variation as prospective targets. These are not present results or universal acceptance thresholds.

## Current software

The Rust reference library audits exact identity, process-parameter windows, cell quality specs, canonical evidence references and per-lot/per-line conformance. It reports descriptive finite-sample rates stratified by evidence tier. The synthetic fixture intentionally contains a failing cell, an out-of-window run and an incomplete inspection.

It does not calculate statistical process capability or confidence intervals. It does not prove raw-byte digests, instrument calibration authenticity, process causality, product safety or actual production capability.

## Research anchors

- Applied Energy 402B (2026), electrode production machinery review: https://doi.org/10.1016/j.apenergy.2025.126968
- Next Energy 13 (Oct 2026), formation protocol design review: https://doi.org/10.1016/j.nxener.2026.100785
- Real-time process control of electrode properties (2026): https://advanced.onlinelibrary.wiley.com/doi/10.1002/aesr.70267
- Chemical Reviews (27 Feb 2026), synchrotron characterization for battery manufacturing: https://doi.org/10.1021/acs.chemrev.5c00772
- Multimodal metrology for sustainable battery formation (2026): https://www.sciencedirect.com/science/article/pii/S1364032126006945

These motivate the benchmark only. No paper-reported outcome is inherited as a Luminous result.
The following sources offer a concrete retrospective benchmark ladder. Their licence and target boundaries must remain bound to every derived snapshot; none alone establishes multi-line industrial yield.

| Dataset | Scope and useful variables | Rights / permitted path |
|---|---|---|
| Warwick frequency-domain ultrasonic electrode dataset, DOI 10.17632/c62yn37d9h.4 (version 4, 15 Dec 2025) | Graphite anode and NMC622 cathode; before/after calendering FFT frequency/magnitude arrays, average thickness across three regions, web speed, roll gap, coat weight, calendering speed and density. Electrode-level NDT/process-to-structure task, not cell yield. | CC BY 4.0; attribution and exact version required. https://data.mendeley.com/datasets/c62yn37d9h/4 |
| Warwick NMC622 pilot-plant calendering dataset, DOI 10.17632/wwhm2frfmy.1 (version 1, 24 Jul 2023) | Coating/calendering operating conditions, electrode physical properties before/after calendering and related electrochemical cell performance. A useful pilot-scale process/structure/property task. | CC0 1.0 on the dataset record. https://data.mendeley.com/datasets/wwhm2frfmy/1 |
| Graphite electrode formulation/manufacturing dataset, DOI 10.17632/4dh2h3tsf4.1 (version 1, 10 Jan 2022) | Related paper describes slurry composition/mixing, coating gap, drying temperature, coating speed and calendering, with rheology, adhesion and electrochemical outcomes. Verify exact contents and data dictionary before defining targets. | CC BY 4.0 on the dataset record. https://data.mendeley.com/datasets/4dh2h3tsf4/1 |
| TRI-AMDD formation/aging dataset, Data.MATR 8 | 186 pouch cells and 62 formation protocols in the associated Joule study; raw formation records and structured ageing cycles. Useful for protocol → early formation signature → cycle-life reproduction, not electrode-line yield. | CC BY-NC 4.0 on the dataset record: research/non-commercial use only unless additional permission is obtained. https://data.matr.io/8/ |

### Recommended reproduction order

1. **Ultrasonic NDT:** compare the published feature pipeline with simple frequency-domain baselines; hold out physical electrode sample IDs and pre/post-calendering pairs together where needed. Predict thickness first, then examine density/porosity transfer only where the measurements support those labels.
2. **Pilot NMC622 calendering:** reproduce a process → electrode property → half-cell/cell response chain; separate each response boundary and do not pretend half-cell performance is complete-cell EV performance.
3. **Graphite formulation:** verify the actual file inventory, exact process factors and sample identities before admitting it into a common benchmark.
4. **Formation:** reproduce the 186-cell/62-protocol study as a distinct non-commercial track; use group-aware splits by protocol and cell, and keep its NC licence out of commercial model training or deployment until separately authorized.

The CC BY and CC0 entries are candidates for admission with their stated attribution/rights, source hashes and exact dataset versions. The CC BY-NC formation data must be separately tagged COMMERCIAL_USE_BLOCKED unless new permission is obtained. Use the existing Symthaea EV-FIELD-EVIDENCE PR #11421 permission registry for each intended operation (local processing, model training, derived-data publication, model-weight release, commercial use and redistribution); a dataset's general licence label must not be treated as blanket permission for every operation. No raw dataset has been copied into this repository by this PR.

Do not merge these four datasets into one pooled score: electrode ultrasonic regression, pilot-plant NMC process/property data, graphite process/formulation records, and formation-to-cycle-life are different tasks, physical units and evidence boundaries. They can contribute to a multi-fidelity evidence graph, not a fake common leaderboard.

## Candidate research targets

- zero recipe/configuration/line/lot/evidence-class pooling without explicit compatibility;
- zero synthetic or model-predicted outputs counted as observed physical yield;
- zero incomplete/invalid inspection records silently counted as conforming;
- zero cell/lot/line holdout leakage;
- after physical baseline and power analysis, investigate ≥5 percentage-point first-pass-yield increase or ≥20% reduction in between-lot variation as prospective targets. These are not present results or universal acceptance thresholds.

## Current software

The Rust reference library audits exact identity, process-parameter windows, cell quality specs, canonical evidence references and per-lot/per-line conformance. It reports descriptive finite-sample rates stratified by evidence tier. The synthetic fixture intentionally contains a failing cell, an out-of-window run and an incomplete inspection.

It does not calculate statistical process capability or confidence intervals. It does not prove raw-byte digests, instrument calibration authenticity, process causality, product safety or actual production capability.

## Research anchors

- Applied Energy 402B (2026), electrode production machinery review: https://doi.org/10.1016/j.apenergy.2025.126968
- Next Energy 13 (Oct 2026), formation protocol design review: https://doi.org/10.1016/j.nxener.2026.100785
- Real-time process control of electrode properties (2026): https://advanced.onlinelibrary.wiley.com/doi/10.1002/aesr.70267
- Chemical Reviews (27 Feb 2026), synchrotron characterization for battery manufacturing: https://doi.org/10.1021/acs.chemrev.5c00772
- Multimodal metrology for sustainable battery formation (2026): https://www.sciencedirect.com/science/article/pii/S1364032126006945

These motivate the benchmark only. No paper-reported outcome is inherited as a Luminous result.