# WDN Dataset Crosswalk v0.1

**Tracking issue:** [Symtropy #4979 — WDN-DATASET-CROSSWALK-001](https://github.com/Luminous-Dynamics/symtropy/issues/4979)  
**Parent benchmark:** [Symtropy #4963 — Pressurized-water leak diagnostics](https://github.com/Luminous-Dynamics/symtropy/issues/4963)  
**Status:** metadata crosswalk only. No source dataset files have been acquired, hashed, split, or redistributed.

## Why this is a separate engineering deliverable

The 9 November 2026 systematic review by Ruiz-Diaz et al. identified 91 publications (70 primary studies and 21 reviews). Of the primary studies, 67 addressed leak detection, 32 localisation and 10 characterisation. The review also identifies weak real-world validation, generalisation and comparability as persistent gaps. Importantly, diagnostic task counts are multi-label, not mutually exclusive. ([DOI 10.3390/w18212797](https://doi.org/10.3390/w18212797))

A second review by Farah & Shahrour (September 2026) found 90 studies, with 30 classified as using public benchmarks but only 23 reusing one of three shared named resources (BattLeDIM/L-Town, LeakDB or the Aghashahi acoustic testbed). It documents different dataset definitions, split methods, metrics and scenario populations; these data cannot be pooled into one simple leaderboard. ([DOI 10.3390/w18182267](https://doi.org/10.3390/w18182267))

Our paradigm shift is from “benchmark the model” to **qualify the population, labels, evidence type, rights, splits and operational task before evaluating the model**.

## Dataset population map

| Dataset | Population | Best first use | What it cannot establish |
|---|---|---|---|
| DiTEC-WDN | 36,000 synthetic normal hydraulic scenarios across 36 networks; 228M graph states reported | P0 normal-state modelling, hydraulic residual calibration, graph forecasting | Leak-positive recall or field leak detection; normal-state source is not a leak-labelled corpus |
| LeakDB | Synthetic leak scenarios across different networks and conditions; 13.9 GB record | P1 synthetic leak detection/localization reproduction | Physical or utility field performance |
| WDSEventDB | Sensor-instrumented testbed with leak, cyberattack and sensor-failure events | P2 event diagnosis / sensor fault separation, after file audit | Utility-scale field transfer; exact labels/rights still require file audit |
| Aghashahi LeakTestbed | 280 physical lab signals; 47 m, 152.4 mm PVC loop; acoustic/vibration/dynamic pressure | P2 physical leak/no-leak and localization by sensor/location | Full municipal-network generalization |
| Slovak utility SCADA | One real distribution zone; 2022; 8,737 hourly records; 18 anomaly variables and one binary fault label | P3 single-zone utility retrospective / missingness | Sub-hour transient/acoustic claims or multi-utility generalization |
| BattLeDIM / L-Town | 2018 historical / 2019 evaluation, 5-minute SCADA-style data, EPANET model, leak events | P3 benchmark reproduction; detection/localization/economic response | Live-utility field claim; L-Town is a hypothetical model based on a real network |
| 47k+ Net3 paper corpus | Paper-reported EPANET/WNTR synthetic benchmark | P1 result reproduction, once exact artifacts are recovered | Reproduction from abstract alone or field performance |
| Pamplona dark-fibre DAS trial | Controlled field events over 17 km of telecom dark fibre; 2.6 m effective gauge length; raw traces available on request | P2 controlled-field DAS detection/localization after access and rights admission | Natural-leak validation, generic utility transfer or raw-data redistribution |

Sources: DiTEC-WDN ([Scientific Data](https://doi.org/10.1038/s41597-025-06026-0), [dataset CC BY 4.0](https://huggingface.co/datasets/rugds/ditec-wdn)); LeakDB ([Zenodo record](https://zenodo.org/records/13985057), [KIOS code](https://github.com/KIOS-Research/LeakDB)); WDSEventDB ([Zenodo](https://zenodo.org/records/17547955)); Aghashahi dataset ([Data in Brief](https://doi.org/10.1016/j.dib.2023.109148), [WaterBenchmarkHub](https://waterfutures.github.io/WaterBenchmarkHub/benchmarks/LeakTestbed.html)); Slovak SCADA ([Scientific Data](https://doi.org/10.1038/s41597-026-07203-5), [Zenodo CC BY 4.0](https://zenodo.org/records/15096167)); BattLeDIM ([competition dataset](https://zenodo.org/records/4017659), [competition data page](https://battledim.ucy.ac.cy/?page_id=33)); Net3 paper ([Journal of Water Process Engineering](https://doi.org/10.1016/j.jwpe.2026.110114)); Pamplona dark-fibre DAS trial ([Sensors 2026](https://doi.org/10.3390/s26072152)).

## Important corrections and distinctions

1. **DiTEC-WDN is a normal-state hydraulic dataset.** It has broad graph/temporal coverage and CC BY 4.0. It is useful for hydraulic state modelling, but the source describes standard operating conditions, not controlled leak labels. Do not train a leak classifier by treating all states as leak negatives without a separate event-label audit.
2. **LeakDB is synthetic.** The Zenodo page exposes a blank License field in its Rights section, while a secondary NLM catalogue lists CC BY 4.0. The archive and primary record metadata must be reconciled before redistribution.
3. **WDSEventDB and the Aghashahi LeakTestbed are not assumed identical.** WDSEventDB's Zenodo record references the Aghashahi Data in Brief DOI among related works, but that DOI describes a separate 280-signal leak testbed. File-level comparison is required before any merge.
4. **Aghashahi LeakTestbed is physical but laboratory-scale.** It includes five classes (four leak mechanisms plus no-leak), two network topologies and multiple sensor types/positions, with short high-frequency signals. It is the right place to test acoustic feature extraction and sensor placement, not prove city-scale performance.
5. **The Slovak dataset is real utility telemetry but has hourly sampling.** It covers one zone for 2022, 8,737 timestamps, 18 anomaly-score fields and a binary fault label. Its NULLs were preserved. Do not resample it into synthetic high-frequency acoustic data or treat the binary label as a verified defect mechanism/location without auditing the label construction.
6. **BattLeDIM is a benchmark population with a historical/evaluation split.** Its 2018 and 2019 files, EPANET model and evaluation rules should be preserved exactly. Avoid random window splitting across the same year/event; preserve the official held-out evaluation and economic metric.
7. **The >95% Net3 result is paper-reported.** We need the exact case generator, 47k scenario files, feature definitions, train/test split, leakage sizes, sensor budget and scoring script before an exact reproduction can be claimed.
8. **The Pamplona dark-fibre DAS source is controlled-field evidence, not natural-leak evidence.** It demonstrated a 17 km sensing fibre, 2.6 m effective gauge length, and controlled valve events from 0.14–3.75 L/s. Its Data Availability Statement says raw data are available on reasonable request. No raw traces or redistribution permission have been acquired. DOI: https://doi.org/10.3390/s26072152.

## D-A-T task contract

The 2026 review's D-A-T taxonomy is useful for keeping claims precise:

- **D — Data acquisition:** hydraulic (pressure/flow), vibroacoustic (accelerometer/hydrophone), optical/remote, geophysical/electromagnetic, or sensor-fault/cyber event channels.
- **A — Processing architecture:** physical/model-based, signal/statistical, machine learning, deep learning, or hybrid/fusion. Integration modifiers must not be ranked as standalone families unless tested as such.
- **T — Diagnostic task:** T1 detection, T2 localization, T3 characterization (magnitude/flow/severity). Every metric must identify the specific task achieved.

The literature review counted 67/70 primary studies for T1, 32/70 for T2 and 10/70 for T3; these are multi-label counts. Only six reviewed studies covered all three tasks. That makes **jointly calibrated detection + localization + magnitude characterization** a worthwhile research target, but not a claim we can make from a binary fault dataset.

## Admission states

- `catalogued_metadata_only`: public record and article metadata indexed; no files acquired.
- `rights_pending`: rights/license not clear for the raw dataset archive.
- `artifact_recovery_blocked`: paper result is documented but data/code artifacts not yet found.
- `admitted_for_reproduction`: exact source revision, rights, archive digest, file list and task-label semantics verified.
- `split_qualified`: event/network/site-level split manifest reviewed and fixed.
- `benchmark_eligible`: only after the preceding admission gates and baseline execution are complete.

No dataset is admitted or split-qualified in v0.1. The machine-readable register intentionally contains empty SHA-256 arrays and null split-manifest fields.

## Common benchmark reporting

For each run, freeze and publish:
- dataset ID, release/revision, source URL, licence/rights decision, archive and unpacked file digests;
- acquisition date, source file inventory, parser and preprocessing commit;
- site/network, topology, pipe material/diameter/age if available, pressure regime and flow conditions;
- native time step, sensor model/location/range/calibration and missingness;
- label provenance, label uncertainty, event onset and independent ground-truth method;
- group-aware splits by event and network/site; all windows from the same physical event stay in one partition;
- model and baseline versions, hyperparameters, feature/sensor budget, compute and runtime;
- T1/T2/T3 outputs and separate detection, localization and characterization metrics;
- false alarms per time/asset denominator, detection delay, P50/P95/max localization error, magnitude interval coverage, abstention and unobserved exposure;
- transfer performance by target population, without pooling the raw scores.

## First execution order

1. Request and rights-check the **Pamplona DAS** dataset and method details with the corresponding author; keep reproduction Blocked until raw traces, labels, baseline records and permissions are received.\n2. Admit **BattLeDIM** and **Slovak SCADA** first as separate P3 tracks: they provide complementary 5-minute benchmark and hourly utility populations.
3. Admit **LeakTestbed** for P2 sensor/localization work after verifying Mendeley raw-data licence and files.
4. Admit **LeakDB** for P1 after resolving licence metadata and retrieving its scoring tool revision.
5. Admit **WDSEventDB** only after inspecting its native labels, sample rates, files and rights.
6. Use **DiTEC-WDN** as P0 normal hydraulic state, not leak ground truth.
7. Recover the 47k+ Net3 paper's exact artifact set or mark reproduction blocked.
8. Only after these are in place, choose one shared baseline and run a matched, separately reported transfer experiment across two populations.

## Nonclaims

No dataset files have been acquired, hashed, redistributed or evaluated by this document. No model result, leak location, field performance, water savings, repair outcome, or SOTA claim is made. Metadata catalogue entries are not a successful dataset admission.


## Pamplona dark-fibre DAS intake note

The 31 March 2026 Sensors paper's Data Availability Statement says its data are available on reasonable request from the corresponding author. The raw DAS traces, baseline records, event labels and route geometry are not included in this repository and have no SHA-256 digests or split manifest here. Article open access is not treated as raw-data redistribution permission. See [DARK-FIBER-WDN-DIAGNOSTICS-001 #4991](https://github.com/Luminous-Dynamics/symtropy/issues/4991) and DOI https://doi.org/10.3390/s26072152.
