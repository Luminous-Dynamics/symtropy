# External marine engine fault benchmark v0.1

**Dataset:** [Marine Engine Fault Dataset, Zenodo 19857425](https://doi.org/10.5281/zenodo.19857425)  
**Related preprint:** [arXiv:2607.19444](https://arxiv.org/abs/2607.19444)  
**License:** CC BY 4.0; attribution is required  
**Integration status:** metadata verified; archive payload and channel dictionary not yet audited

## Why this matters

The public record describes controlled experiments on a Matsui Iron Works MU323DGSC turbocharged/intercooled three-cylinder marine diesel engine. It contains one baseline reference recording and 15 fault-scenario CSVs, approximately 115,000 timestamped samples and about 70 measured/derived channels. Five fault families were physically induced: compressor air-filter clogging, air-cooler fouling, injection-valve nozzle clogging, cooling-water pump cavitation and turbine degradation. The Zenodo record describes 16 CSV files, 73 columns for scenario files (including three leading time/label columns), a distinct 70-column baseline schema, a three-row header, dataset index and variable dictionary.

This is substantially stronger than synthetic-only examples for testing machinery fault detection and transfer across load conditions. It is still one controlled test bench and one engine model—not a shipboard fleet validation set. The associated preprint is submitted to a journal and shown as under review; it is not a final peer-reviewed benchmark standard.

## Payload audit status

This manifest records the publisher's public metadata only. The 26.5 MB archive could not be retrieved by this execution environment because the web content fetch rejected the payload size and the local runtime could not resolve the external host. Therefore:
- raw CSVs have not been read;
- the dataset index and variable dictionary have not been inspected;
- the publisher's MD5 value has not been checked against downloaded bytes;
- no SHA-256 is claimed;
- no split assignments or performance results are claimed.

Do not treat the published file description as an actual local payload audit. Once retrieved in a connected environment, verify archive bytes, inspect archive paths, parse all three header rows, reconcile baseline/scenario schemas, check timestamps, missingness and exact per-file sample counts, then update the manifest with SHA-256 and audited metadata.

## Evaluation protocol

### 1. Freeze the data before modeling

Keep original CSV files as indivisible recording groups for primary evaluation. A random row/window split would leak adjacent time samples and recording-specific operating conditions across training and test sets. Preserve source run, fault intervention, load, engine configuration, timestamp range, label availability and channel-presence mask.

Do not merge the 70-column baseline file and 73-column scenario files by positional column index. Align channels only through the inspected variable dictionary, normalized name, unit and physical meaning. Retain source names and unit metadata. Any conversion must be explicit and reproducible.

### 2. Use multiple complementary evaluation regimes

- **Run-held-out fault diagnosis:** keep every window from an original scenario recording in one fold. Report fault family and load coverage; do not claim generalization beyond held-out runs from this test bench.
- **Load transfer:** for fault families with sufficient distinct load runs, train on some loads and hold out entire load-level recordings. A held-out load is a distinct test, not a random segment split.
- **Baseline nuisance-alert assessment:** because the record has one baseline recording, use chronological blocked windows with a purge/guard interval to quantify within-recording false alarms. Explicitly label this as a single-campaign estimate, not independent baseline replication.
- **Open-set severity/unknown analysis:** hold the two-hole injection-nozzle restriction out as a separate severity/generalization experiment. Do not score event-onset detection on the two injection-nozzle files documented as anomaly-only; they lack an observed pre-anomaly segment.
- **Missing-channel robustness:** evaluate the five runs with the two documented absent channels separately. Preserve masks and compare mask-aware inference with any imputation baseline; fit imputation only on the training partition.
- **Pre-anomaly versus anomaly detection:** use only files whose label timeline actually includes both states. Confirm the label transition and annotation semantics from each CSV before scoring latency.

Split assignments remain pending archive audit; do not hardcode inferred filenames or load labels until the index is inspected.

### 3. Matched model and workflow baselines

Run all methods over the same file groups, windows, channel mapping, label scope, train-only normalization and compute budget:

1. documented equipment alarms/engineer rules, only where source limits are explicitly available;
2. PCA / multivariate statistical process control;
3. Isolation Forest or One-Class SVM for anomaly detection;
4. Random Forest / gradient-boosted trees for labelled fault classification;
5. temporal convolutional / 1-D CNN baseline;
6. a calibrated open-set or abstaining classifier;
7. Symthaea evidence synthesis, on identical input channels and labels;
8. physics/model residuals only if an engine model and independent validation evidence exist.

Separate anomaly detection from fault isolation, severity classification, root-cause inference and remaining-useful-life estimation. The archive description alone does not establish labels suitable for every task.

### 4. Metrics

Report by fault family, load, fault realization, channel-availability pattern and evaluation regime:
- event-level recall and missed-fault rate;
- false alarms per engine-hour and false-alarm duration;
- detection latency from documented anomaly onset;
- fault-family macro precision/recall/F1 and balanced accuracy;
- calibration and uncertainty (Brier score, reliability curve, interval coverage where applicable);
- unknown/OOD rejection and false-known rate;
- confusion between physically different fault mechanisms;
- load-transfer performance delta;
- degradation under channel dropout and sensor-quality perturbations;
- inference latency and compute/memory footprint.

Use grouped bootstrap confidence intervals by recording, not by individual row. Report per-file results; the number of independent physical runs is small. Avoid an overall accuracy headline dominated by repeated, autocorrelated samples.

### 5. Proposed competitive criteria (targets, not claims)

- Zero cross-recording leakage in the primary held-out protocol.
- Every result binds dataset DOI/version, file digests, channel map, unit conversions, split manifest, model/configuration, code commit and evaluator digest.
- At matched fault recall, reduce false alarms per engine-hour versus the strongest reproducible baseline; alternatively, at matched false-alarm burden, improve event recall. Report the Pareto frontier, not one weighted score.
- Improve held-out-load performance without degrading the worst-performing fault family.
- Preserve an explicit unknown/abstain path; do not trade false-known confidence for nominal accuracy.
- No numerical "beats SOTA" target until the archive is independently audited and baseline results are reproduced on identical file-level splits.

## Industry/SOTA positioning

This public test bench enables a fair comparison against conventional condition-monitoring and ML workflows, but it does not constitute a universal marine-engine SOTA leaderboard. ISO 17359 and ISO 13374-1 provide condition-monitoring programme and diagnostic-data process guidance, not a matched fault-detection score. DNV AROS separately recognizes engineering functions alongside navigation, operational and safety functions; the benchmark therefore feeds a bounded engineering-capability status rather than redefining vessel autonomy or mode authority.

Our potential differentiation is the assurance layer around diagnostics: configuration-bound evidence, source and calibration lineage, common-cause handling, clock/freshness qualification, calibrated unknown states, capability impact and review-gated recovery. We must still prove diagnostic quality on this public measured dataset and later on independent engine models and shipboard data.

## Attribution and claim boundary

Cite the dataset DOI and the associated preprint. The dataset is CC BY 4.0; retain author attribution and license notices if redistributing derived material. This benchmark can establish performance on the released MU323DGSC controlled test-bench recordings only. It does not establish shipboard performance, fleet reliability, seaworthiness, safe operating limits, class/statutory compliance, MASS Code compliance, maintenance authorization or permission to operate.

See `benchmarks/maritime/marine-engine-fault-dataset-v1.manifest.json` for the machine-readable source record and current audit status.
