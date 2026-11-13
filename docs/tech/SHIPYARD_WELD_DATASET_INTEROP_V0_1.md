# Rights-aware multimodal welding dataset interoperability v0.1

Tracking: [SHIPYARD-WELD-DATASET-INTEROP-001 #4683](https://github.com/Luminous-Dynamics/symtropy/issues/4683)

## Four registered domains

| Dataset | Independent unit | Modalities | Label/evidence ceiling | Rights |
|---|---|---|---|---|
| BAM Tandem-GMAW | 5 WIDs | robot, current/voltage, leading/trailing source, 3D scan, video, t8/5 | process observations, surface metrology, thermocouple measurement; no NDT result in seed | raw Zenodo licence blank; RIGHTS_PENDING; article CC BY 4.0 applies only to article text/metadata |
| Intel Robotic Welding Multimodal | 236 sessions | video, 16 kHz audio, sensor time series, five post-weld images | intended defect-category annotations; dataset card notes label noise/no expert post-weld validation and possible multi-defect samples | gated Intel Research Use License; research-only/non-commercial, non-transferable/non-sublicensable |
| BAM Scientific Tool Use v0.1.0 | 5 WIDs | selected NetCDF/HDF5 arrays, task prompts/schemas/reference answers; no video | 160 numerical/evidence tasks (130 answerable, 30 constructed missing-file), 480 separate introductory SFT summaries; no expert defect/NDT/quality labels | derivative claims CC BY 4.0, but original Zenodo source record currently shows blank licence and copyright; RIGHTS_RECONCILIATION_REQUIRED; no raw files admitted |
| GMAW thermography/profilometry | 4 runs | thermography/process CSV, laser profile, macrograph penetration profile, processed features | width/reinforcement metrology and destructive macrographic penetration; not NDT | article CC BY-NC; item-level Figshare v3 licence still pending independent read-back |

No raw files are downloaded or vendored in v0.1. The three data sources are not concatenated and their labels are not pooled.

## Source and rights details

- BAM Zenodo DOI 10.5281/zenodo.21824586 v2.0.0. Companion article DOI 10.1016/j.dib.2026.113329 is CC BY 4.0, but the raw data record's License field remains blank.
- Intel dataset card: https://huggingface.co/datasets/IntelLabs/Intel_Robotic_Welding_Multimodal_Dataset. The 39.9 GB archive is gated. The card describes 236 sessions, over 4,000 annotated samples, 12 intended defect categories, 16 kHz audio, nominal 30 FPS video and 5 post-weld images/sample. It warns of label noise (not expert-validated post-weld) and possible device response misalignment.
- Thermography article DOI 10.1016/j.dib.2025.111883, PMCID PMC12301811; item version DOI 10.6084/m9.figshare.28062602.v3. Four runs (I–IV) each provide time-stamped thermography/process parameters, laser width/reinforcement, macrograph-derived penetration profiles and processed features. Article CC BY-NC is not automatically the hosted file licence.

## Evidence / label authority

1. Process signals: useful for process state; not defect or acceptance truth.
2. Surface/laser metrology: geometry only, with frame/uncertainty.
3. Macrograph: destructive section-specific penetration evidence; not UT/RT/PAUT.
4. Intended/weak defect annotations: weak supervision, not expert-verified truth.
5. Independent NDT reports: separate high-authority target only after method, procedure, coverage and acceptance criteria are recorded.
6. Mechanical/structural tests: separate outcome family.

No model metric may promote a lower-authority source into a higher-authority claim.

## Group-aware validation

- BAM groups by WID (5 groups).
- Intel groups by session/weld identity (236 sessions in the source card).
- Thermography groups by complete experiment run (4 groups).
- Keep all nested layers, frames, audio windows, scans and derived features in the group partition.
- Do not report pooled accuracy across targets that do not share a measurement definition.
- Leave-one-domain-out transfer is exploratory only for semantically matched outputs, and zero-shot/calibrated/fine-tuned results must be separate.
- Preserve uncertain and uninspected records and disclose data selection. A sample with no NDT result is not a negative-quality label.

## Deterministic multimodal alignment oracle

The Rust crate freezes this correction convention:

`master_event_time_ns = local_timestamp_ns - clock_offset_local_minus_master_ns - sensor_response_delay_ns`

Clock uncertainty and sensor-latency uncertainty are combined by conservative addition, not root-sum-square, because independence is not established. Every modality retains its native sample interval. A missing sample block remains empty; the verifier never interpolates it into measured truth.

The five known-answer cases cover:
- 0.1 ms process current/voltage;
- 8 ms robot TCP;
- 100 ms power-source OPC-UA;
- 40 Hz video trigger;
- an explicit missing block with no samples and no invented interpolation.

These are synthetic nanosecond-level known answers, not measured results from BAM, Intel or the thermography dataset. They test offset sign, response-latency correction, native cadence, conservative uncertainty and fail-closed missingness.

## V0 targets (research goals)

- 100% source/version and group identity preservation.
- Zero specimen/session leakage across partitions.
- Zero rights and evidence-authority escalations in frozen controls.
- 100% detection of seeded clock/latency/unit/missingness/digest faults.
- Report per-domain calibration and uncertainty; no pooled interval-coverage claim.
- Target at least 90% empirical coverage for nominal 90% intervals on sufficiently sized independent holdouts.
- Zero false-safe classifications on a frozen authoritative-NDT negative-control set; estimate the operational rate only from a preregistered independent validation cohort.
- A 5% quality-adjusted engineering-utility improvement is aspirational, not achieved.

## Claim ceiling

A PASS establishes only that metadata rights, grouping and synthetic alignment rules are represented correctly. It does not parse the 39.9 GB or 4.5 GB datasets, establish physical quality, validate an NDT predictor, demonstrate cross-domain generalization or authorize commercial reuse.

## BAM Scientific Tool Use derivative — ceiling-limited regression candidate

Public card: https://huggingface.co/datasets/harrrshall/bam-scientific-tool-use, repository revision `51e0e9a`. The card describes 160 raw-file tasks across 30 passes from five WIDs (130 answerable + 30 constructed withheld-file cases), 480 separate introductory evidence-summary/SFT examples, and a 60-file NetCDF/HDF5 source subset of about 400,184,634 bytes. It reports three models solving all 32 test tasks in three repeats; the test set is a single held-out workpiece (WID632). This is an auditable tool-use regression harness with a ceiling effect, not a frontier-discriminative benchmark or physical quality/acceptance benchmark.

**Rights conflict:** the derivative card and its attribution file assert CC BY 4.0 for source and derivative data, while the official Zenodo v2.0.0 source record currently presents a blank License field and “Copyright (C) 2026 The Authors.” The registry therefore sets `RIGHTS_RECONCILIATION_REQUIRED`. Do not download, mirror, train on, redistribute or score the derivative's raw source subset until the source authors/BAM confirm the exact rights chain in writing. The card also states that it is independently prepared and not endorsed by BAM/source creators.

Use restrictions even after rights resolution: five WIDs are the independent physical clusters; 160 task rows are not 160 independent experiments; the 32-task test has one WID; the 30 withheld-file cases are constructed controls, not natural missingness; and the 480 introductory SFT items are not to be pooled into raw-file task scores. These boundaries are machine-checked as WDINT-042 through WDINT-046.