# EV battery ultrasonic electrode inspection benchmark v0.1

Status: task protocol, synthetic fixture and read-only audit. No measured ultrasonic accuracy, process yield, complete-cell performance or production inspection claim.

## Research task

Use frequency-domain ultrasonic signals as non-destructive observations of electrode state, beginning with thickness estimation on physical electrode specimens before/after calendering. The initial public candidate is the Warwick graphite-anode / NMC622-cathode dataset, Mendeley DOI 10.17632/c62yn37d9h.4, version 4 (15 Dec 2025), CC BY 4.0: https://data.mendeley.com/datasets/c62yn37d9h/4.

The dataset record describes backwall-echo signals, FFT frequency/magnitude features, thickness measured over three regions, and process metadata including web speed, roll gap, coat weight and calendering speed/density. The first task should predict thickness only. Density/porosity, adhesion, defects and lifetime require separate compatible target evidence; do not infer them as labels from acoustic features.

## Canonical owners

- Parent EV manufacturing robustness: Symtropy #4833.
- Dataset/source operation rights: Symthaea EV-FIELD-EVIDENCE PR #11421.
- Physical sample/process/characterization lineage: Symthaea MAT-013 #4344.
- Process/structure/property semantics: Symthaea MAT-PSPP #6224.
- Metrology and calibration: Symthaea ENG-MEAS #4919 and MAT-UQ #6225.
- EV recipe/lot/line audit: Symtropy #4833 / PR #4836.

This benchmark owns only the ultrasonic task split, target and scoring contract. It does not create a new materials ontology, process database, UQ engine, rights registry, signal-processing engine or manufacturing controller.

## Critical leakage controls

1. The independent unit is the physical electrode specimen, not FFT bins, echoes, spatial regions or augmented copies.
2. All repeated observations/regions from one specimen stay in one partition.
3. Paired pre/post-calendering states from one physical specimen stay in the same partition for ordinary sample-held-out evaluation. A distinct pre-to-post prediction task may be defined later, but cannot be mislabeled as generalization to an unseen specimen.
4. Material family and process condition are grouping metadata, never hidden in an observation ID or accidentally used as target-derived predictors.
5. Preprocessing, denoising, frequency selection, normalization, feature selection and interval calibration are fit inside training/validation groups only.
6. Feature availability must precede the prediction timestamp. Post-process signals cannot be used to claim a pre-process prediction.
7. Thickness targets bind independent metrology references, units and exact specimen/region; ultrasound-derived pseudo-labels are not independent ground truth.

## Split tournament

- Source/published split reproduction, if exactly documented.
- Primary leave-one-physical-specimen-out.
- Paired-state specimen holdout: both pre/post states are absent from training when held out.
- Leave-one-material-family-out (graphite vs NMC622), if group counts support it.
- Leave-one-process-condition stratum out where the source has enough independent groups.
- Instrument/domain holdout only if instrument identity is recorded.

Random observation/FFT-row splits are anti-baselines only and must not be headline results.

## Baselines

- Echo time/delay, attenuation and simple frequency-band energy features.
- Linear/ridge/SVR on prespecified signal summaries.
- Random forest and gradient boosting.
- Frequency-domain model compared with time-domain features.
- Signal-only versus signal + process-context model.
- Model with and without calibrated intervals/abstention.

Use the same feature horizon and physical specimens for each baseline. Report the number of independent specimens and material/process groups.

## Metrics

- Thickness MAE/RMSE and signed bias in the exact physical unit (canonical audit unit: µm).
- Median, P90 and worst-specimen absolute error.
- Material-family and pre/post-state stratification.
- Prediction interval coverage, width and interval score with specimen-group calibration.
- OOD/selective risk versus coverage; false accept/reject only when a specification limit and inspection decision rule are predeclared.
- Inspection sampling density/time and throughput only if measured; offline model speed is not a line-speed or economics result.

## Proposed targets (research, not achieved)

- Reproduce any exact source-reported task/split before claiming a comparable score.
- Zero specimen or paired-state leakage across partitions.
- Target ≥10% lower sample-held-out RMSE than a preregistered simple signal baseline, with no material-family regression and calibrated intervals; only promote when specimen counts and confidence intervals support the comparison.
- Zero promotion of inferred thickness as direct measurement.
- Unsupported material/process/instrument domains must abstain or widen uncertainty instead of confident extrapolation.

## Rights and source snapshot

Before raw data are used, freeze the exact Mendeley version, file inventory, raw digests, original citation/attribution, source terms and operation-specific permissions through Symthaea EV-FIELD-EVIDENCE #11421. The dataset is CC BY 4.0, but the local audit only checks that a rights-receipt reference exists; it does not adjudicate or authenticate legal permission. No raw data are copied into this benchmark seed.

## Claim ceiling

A future PASS may establish thickness prediction on the exact admitted physical specimens, sensor setup, material family and split. It does not establish complete-cell quality, cell life, production yield, battery safety, pack capability or authority to make production accept/reject decisions.