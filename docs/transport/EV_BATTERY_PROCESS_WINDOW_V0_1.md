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
- Mycelix manufacturing owns cross-organization lot/batch/line/resource events and work-order routing.

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
6. Out-of-window runs are separately counted and are not used as in-window yield samples.
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

- first-pass yield with exact in-window denominator and evidence tier;
- per-lot and per-line conforming/nonconforming/incomplete counts;
- between-lot and between-line variation and worst-group performance;
- out-of-window process-run frequency;
- defect, scrap and rework burden;
- cycle-life distribution only when linked to actual ageing endpoints or a separately qualified censored prognostics result;
- accepted kWh per process energy, water, solvent and consumables under explicit boundaries;
- cross-lot/line generalization, false acceptance/rejection and abstention;
- inspection/data/compute burden.

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