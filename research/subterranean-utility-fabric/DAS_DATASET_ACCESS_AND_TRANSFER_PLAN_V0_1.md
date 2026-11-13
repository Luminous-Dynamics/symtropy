# DAS Dataset Access and Transfer Plan v0.1

**Owner:** Symtropy #4972  
**Snapshot:** 2026-11-13  
**Status:** desk research; access and processing not yet performed

## 1. Purpose

Identify legally usable research data for signal-pipeline development while preventing a public dataset from being misrepresented as a buried-utility benchmark. Dataset rights, provenance, physical population, raw/processed status and split design must be checked before model training or publication.

## 2. Candidate dataset inventory

| Resource | Population and contents | Access/rights status | Suitable work | Explicit limit |
|---|---|---|---|---|
| Kamaruzaman et al. (2026), operational SCADA communication fibre fault study | Field study on a 5.4 km MV underground cable corridor; reports a fault/event at about 2.8 km; direct-burial vs conduit coupling comparison | Article states data are available on request. Author/owner permission and terms are required. | Principal fault/event-screening reproduction target | Not yet an acquired dataset; one corridor and one reported field event do not establish general event sensitivity |
| Cohen et al. (2026), traffic-induced buried-fibre geolocalization | DAS plus traffic GPS/video trajectories and physics-based strain-map inversion; reports sub-metre route geolocation in investigated conditions | Request paper supplements, raw data, code, metric definitions and reuse terms from authors; do not assume access | Principal route-geolocalization reproduction target | Route mapping, not fault detection; exact experimental metric and population must be reproduced |
| Power-cable intrusion DAS, Mendeley Data DOI 10.17632/fp77d4223z.1 (2025) | Field-collected cable signals at reported burial depths 1.2–2.0 m; hammering, background, vehicle vibration, rainfall and fan interference; 5 kHz sampling, 2,500 points/window, 500 ms windows and 40% overlap | Dataset page describes CC BY-NC 4.0 and author application, while its separate licence metadata says CC BY 4.0. This is a rights/metadata conflict. Obtain written clarification and authorization before download, reuse or redistribution. No commercial use assumed. | Candidate field-signal transfer and leakage-audit study after rights are resolved | Windowed, min-max-normalized, single-channel samples are not equivalent to raw interrogator traces; reported split construction must be audited |
| DAS-10, Zenodo DOI 10.5281/zenodo.23043546 (2026 v2) | 2,000 labelled 2-second samples, ten event classes, ten spatial channels, effective 50 kHz; environmental audio replayed by loudspeaker onto fibre | Public Zenodo dataset record; verify the exact version licence/terms before use | Signal ingestion, signal-processing smoke tests, controlled classifier and OOD experiments | Laboratory loudspeaker replay; not a buried cable, field route or utility fault benchmark |
| OOI Regional Cabled Array DAS 2025–2026, Zenodo DOI 10.5281/zenodo.19930457 | HDF5 DAS over the first southern cable span to about 95 km; low-rate 8 Hz data; multi-month deployment; 1.2 TB remote data | Public Zenodo record and linked data endpoint; check data citation/terms and availability before bulk retrieval | HDF5/chunk ingestion, long-duration continuity, clock/configuration and degraded-calibration handling | Subsea ocean observatory population; dataset notes calibration issues and says 40 m gauge-length data appear highest quality |
| OOI DAS24 (2024 campaign) | Public long-duration DAS data, approximately 2.4 TB, with a public processing repository | Public data endpoint; verify the exact current terms and source revision | Large-scale ingestion and long-horizon signal-quality controls | Subsea observation population; not a terrestrial utility fault or theft dataset |

## 3. Dataset-specific cautions

### 3.1 Mendeley power-cable intrusion dataset

The Mendeley page states that the data are divided into training/validation/test sets and that the test set includes composite noise. It also reports sliding windows with 40% overlap and Min-Max normalization. The page contains conflicting licence descriptions: CC BY-NC 4.0 in the description and CC BY 4.0 in the licence field, plus author application for access.

Required actions before use:

1. Ask the corresponding author to confirm the controlling licence, whether commercial research/use is permitted, whether derived model weights may be published, and whether the location-protection application imposes additional conditions.
2. Obtain the dataset's original acquisition metadata, the exact split-generation code/order of operations, physical event/session identifiers and any raw traces available.
3. Determine whether overlapping windows from the same physical recording or event cross train/validation/test boundaries.
4. Group splits by physical recording, event session, corridor/site, acquisition day and fibre profile where identifiers permit. Do not use a random window split as the headline generalization result.
5. Preserve the published split as a reproduction baseline, but report a leakage-resistant grouped split separately.
6. Treat normalized values as processed unitless samples unless the author supplies the calibration mapping back to physical strain/strain-rate units.

No dataset files are copied into this repository.

### 3.2 DAS-10

The dataset is useful for checking signal decoding, signal-processing smoke tests, tensor orientation, multi-channel processing, event-window accounting, class imbalance, calibration of classifier confidence and open-set abstention. Because the signals are generated through an acoustic loudspeaker replay, all results must retain the laboratory-audio-replay population label. A model trained here must not be described as qualified for buried utility cables.

### 3.3 OOI DAS campaigns

The OOI campaign is valuable because it exposes data volume, long-duration acquisition, configuration changes, interruptions and calibration problems. The 2025–2026 record reports an outage from 2025-11-27 to 2025-12-05 and warns that the interrogator experienced significant calibration problems; it identifies 40 m gauge length as apparently highest quality. These properties are useful adversarial cases for continuity and quality logic, not evidence of utility-event sensitivity.

## 4. Leakage-resistant evaluation protocol

Before any model comparison:

- Freeze exact dataset version, record DOI, file list, file digests, rights record and acquisition population.
- Preserve raw source bytes; create immutable processed derivatives with explicit transformations and digests.
- Split by the highest available independent grouping: site/corridor, acquisition campaign/day, physical event session, cable/fibre route and interrogator/configuration.
- Keep overlapping windows, repeated traces and augmentations from one source recording in one split.
- Report both within-population evaluation and cross-population transfer; do not pool metrics across datasets.
- Use a held-out site/corridor or acquisition campaign for any generalization claim. If not available, state that site-level generalization could not be assessed.
- Keep a no-event/background exposure denominator in monitored fibre-km-hours/days, not just the number of windows.
- Include sensor dropout, calibration shift, coupling change, traffic/weather confounders and adversarially ambiguous conditions.
- Report per-class precision/recall, critical miss rate, false alarms per monitored fibre-km-day, detection latency, calibration (Brier score/reliability curve), risk-versus-abstention coverage and localization error where independent position truth exists.
- Never convert a classifier confidence into a geographic location or asset diagnosis without the independent route and identity evidence.

## 5. Ordered execution

**D0 — Rights and manifest:** make a source inventory with terms, contact, access state and allowed operations. No files are downloaded until rights permit.

**D1 — Format smoke tests:** use only clearly permitted small samples to parse metadata, identify sample axes/units, verify byte-digest binding and create a read-only normalized metadata envelope.

**D2 — Processed-signal baselines:** establish simple deterministic baselines (energy/threshold, band-energy, matched filter where a reference waveform exists) before deep models. Preserve the original published split and a separately grouped split where source identifiers permit.

**D3 — Cross-dataset stress tests:** train on one permissible population and test on a distinct one. A failed transfer is a result and must not be hidden by retraining on the test population.

**D4 — Paper reproductions:** acquire authorized data from Kamaruzaman et al. and Cohen et al. and reproduce their exact reported metrics and conditions before proposing a superior method.

**D5 — Non-live fixture:** characterize coupling, known events, clock/configuration and route-calibration failure on a non-energized rig with independent trigger and survey truth.

## 6. References

- Kamaruzaman et al. (2026), *Field-validated distributed acoustic sensing for real-time fault detection and localization in medium-voltage underground cables using operational SCADA communication Fibres*: https://doi.org/10.1016/j.rio.2026.101093
- Cohen et al. (2026), *Buried Fiber-Optic Geolocalization With Distributed Acoustic Sensing*: https://doi.org/10.1109/TGRS.2026.3729504 and https://arxiv.org/abs/2604.10331
- Mendeley power-cable intrusion dataset: https://data.mendeley.com/datasets/fp77d4223z/1
- DAS-10 Zenodo dataset: https://zenodo.org/records/23043546
- OOI 2025–2026 DAS dataset: https://zenodo.org/records/19930457
- OOI DAS24 repository: https://github.com/uwfiberlab/OOI_DAS_2024

## 7. Claim ceiling

This is a desk-research inventory. The project has not acquired these datasets, resolved outstanding access/rights questions, trained a classifier, reproduced either 2026 paper, validated an interrogator or demonstrated field detection. Public availability does not imply a right to use data commercially or to redistribute it.
