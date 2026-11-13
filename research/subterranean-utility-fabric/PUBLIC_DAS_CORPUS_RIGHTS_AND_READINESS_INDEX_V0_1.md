# Public DAS Benchmark Corpus — Rights and Readiness Index v0.1

**Workstream:** DAS-PUBLIC-CORPUS-QUAL-001 (#5023)  
**Status:** metadata-level audit; no bulk dataset acquired  
**Date:** 2026-11-13

## 1. Purpose

Build a reproducible baseline ladder without pretending public research datasets are representative of terrestrial utility faults. Every candidate corpus is assigned a task boundary, rights state, known quality caveats and permitted next step.

## 2. Audited candidate corpora

### C1 — Campus DAS event-classification corpus (Tomasov et al., Scientific Data 2025)

- Paper: *Comprehensive Dataset for Event Classification Using Distributed Acoustic Sensing (DAS) Systems*, Scientific Data 12, 793 (14 May 2025), DOI 10.1038/s41597-025-05088-4.
- Data: HDF5 waveform data, JSON annotation curves and NumPy event-window masks. The paper lists nine classes: car, construction, fence, longboard, manipulation, open/close manhole, regular/background, running and walking.
- The published table totals 324,465 labeled event/window samples across 14 HDF5 files. The regular/background class contributes 210,023 samples (~64.7%), so accuracy alone can conceal class imbalance. This is a campus installation, not a utility corridor.
- The paper reports a CNN technical-validation accuracy of up to 91%; that is the authors' task-specific baseline, not a transferable utility-fault score.
- Licence: the article and dataset reference CC BY-NC-ND 4.0. Non-commercial and no-derivatives restrictions apply. This is **not commercially cleared for Luminous LLC**. Do not use in a commercial pilot, product training, derivative redistribution or paid benchmark without explicit written permission or an alternative licence from the rights holder. Public metadata may be used for literature comparison with attribution.
- Best use after rights clearance: reproduce the published nine-class classification task, reproduce their split exactly, then separately test site/session-held-out generalization and class-balanced/operational metrics. Keep it out of commercial training/evaluation until rights are resolved.

### C2 — Trondheim ship-arrival and localization DAS corpus

- Zenodo DOI 10.5281/zenodo.18851481; published 17 March 2026.
- Record describes 63.5 GB of ocean-bottom DAS recordings acquired 3 March 2020 on a seafloor fibre in Trondheimsfjord, Norway.
- The dataset is associated with ship-arrival/event-localization research and a persistent-homology arrival-time detection code example.
- Licence field was not visible in the Zenodo record during this audit. Rights state is **unknown / permission check required**. Confirm the dataset licence, third-party/cable-owner permissions and redistribution restrictions before download or derivative use.
- Best use after rights clearance: reproduce the ship-arrival temporal detection/localization task and assess replay, time alignment, event association and uncertainty. It is subsea/marine, not a terrestrial utility fault benchmark.

### C3 — OOI Regional Cabled Array 2025–2026 OptoDAS

- Zenodo DOI 10.5281/zenodo.19930457; published 30 April 2026.
- The record describes 1.2 TB of HDF5 data at approximately 8 Hz from the south cable, collected 12 November 2025–28 January 2026, with an outage 27 November–5 December 2025.
- Spatial and temporal sampling rates vary. Dataset authors explicitly report significant calibration problems and say the 40 m gauge-length data appear highest quality.
- Raw data are hosted on external OOI infrastructure; the Zenodo record itself contains documentation/readme files. Licence was not visible in the record during this audit. Confirm access and use rights before acquisition.
- Best use: streaming/range retrieval, missingness, data-volume, calibration-state, parameter-profile and replay tests. Do not treat as a clean event-localization truth corpus or claim the calibration issue has been solved.

### C4 — OOI Regional Cabled Array 2025–2026 Nokia multi-span DAS

- Zenodo DOI 10.5281/zenodo.19930402; published 30 April 2026.
- The record describes approximately 3.9 TB at 8 Hz using multi-span binaries with embedded arrays and metadata across north and south subsea cables.
- Sampling rates/spatial intervals vary; the record reports channel spacing from 10–200 m, multiple outages, nonlinearities and strong cross-phase modulation caused by observatory-related synchronization signals. Spatial masking retained selected 165 km north-cable and 275 km south-cable data, while residual non-physical noise remains. The authors also report several-second absolute timestamp delay due to processing-chain latency.
- Licence was not visible in the record during this audit. Confirm access and use rights before acquisition.
- Best use: multi-span identity, metadata parsing, clock-delay/gap handling, spatial masking, data provenance and scale-aware replay. It is not terrestrial asset-fault truth.

## 3. Priority ladder

| Priority | Work | Data requirement | Claim ceiling |
|---|---|---|---|
| P0 | Parse metadata, README and file indexes; record rights and data-readiness | Public records only | Metadata audit |
| P1 | Small sample / read-range proof with permission and digests | Sample file only | Ingestion/replay contract |
| P2 | Reproduce one published baseline on its exact task/split | Rights-cleared task subset | Task-specific reproduction |
| P3 | OOD/session/site-held-out and uncertainty/abstention tests | Complete lawful evaluation splits | Generalization evidence for that population |
| P4 | Terrestrial fibre/utility field corpus with independent truth | Owner-authorized data and survey | Utility-specific qualification |

No multi-terabyte corpus should be bulk-downloaded to create a dashboard. Begin with a small authorized sample and streaming/range access.

## 4. Reproduction protocol

1. Record DOI, publisher, data owner/custodian, licence, permitted purpose, redistribution terms, file sizes, external URLs and collection dates.
2. Record the exact source artifact hash/byte length after acquisition. The public DOI or mirror MD5 is not a substitute for the SHA-256 of bytes actually acquired.
3. Record raw format, interrogator model/configuration, gauge length, channel pitch, declared spatial resolution, sample rate, timebase and units exactly as supplied.
4. Reproduce the authors' preprocessing and split before creating a leakage-resistant alternate evaluation.
5. Keep split grouping at site/corridor/day/session/event level where possible. Overlapping windows and augmentations from one physical event must remain in one split.
6. Publish metrics with class counts, denominators, confidence intervals, event/no-event exposure, missingness and abstention.
7. Mark any adaptation (code, preprocessing, model, threshold or split) as a separate experiment.
8. Never claim terrestrial utility fault localization from marine ship data or campus security classes.

## 5. Unmet dataset requirements

The public candidates do not replace:
- a terrestrial traffic-DAS corpus with independent video/GNSS trajectories and route survey truth;
- a utility-cable event corpus with a declared fault/event taxonomy, negative exposure and independently confirmed event truth;
- fibre-to-utility offset and as-built route geometry with datum/control provenance;
- a dataset whose rights explicitly allow commercial evaluation and, where applicable, product use.

A first field pilot should collect these under a utility-owner-approved protocol, not repurpose public data beyond their licence.

## 6. References

- Tomasov et al. (2025), DOI 10.1038/s41597-025-05088-4: https://doi.org/10.1038/s41597-025-05088-4
- Trondheim DAS dataset (2026), DOI 10.5281/zenodo.18851481: https://doi.org/10.5281/zenodo.18851481
- OOI OptoDAS (2026), DOI 10.5281/zenodo.19930457: https://doi.org/10.5281/zenodo.19930457
- OOI Nokia multi-span DAS (2026), DOI 10.5281/zenodo.19930402: https://doi.org/10.5281/zenodo.19930402
- OOI 2025 DAS documentation: https://github.com/uwfiberlab/OOI_DAS_2025
- Trondheim persistent-homology code example: https://github.com/ltDAS/Arrival_Time_Detection_using_Persistent_Homology

## 7. Claim ceiling

This index is a metadata/right-readiness audit. No corpus bytes were downloaded, no baseline was reproduced, no licence was independently adjudicated by counsel and no terrestrial utility event performance is claimed.
