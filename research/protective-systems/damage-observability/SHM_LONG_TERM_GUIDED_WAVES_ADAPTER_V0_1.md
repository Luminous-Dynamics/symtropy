# Long-Term Guided-Wave Damage Observability Benchmark Adapter v0.1

**Programme owner:** PROTECT-DAMAGE-OBSERVABILITY-001 (#5267)  
**Parent:** PROTECTIVE-SYSTEMS-001 (#5246)  
**Status:** adapter/reproduction protocol proposal; no data downloaded, no models trained, no physical qualification  
**Frozen research date:** 2026-11-14

## 1. Why this dataset matters

A public *Scientific Data* dataset describes long-term outdoor guided-wave structural-health monitoring at the University of Utah. The associated article reports approximately 4.5 years of monitoring, approximately 6.4 million guided-wave observations, environmental variation including temperature from -12.2 °C to 52.5 °C, rain/snow effects, sensor drift and installation shifts, and 13 introduced damage types/states. The article describes eight guided-wave channels / propagation paths in its data records and time/environment metadata.

Dataset DOI: https://doi.org/10.6084/m9.figshare.28112504  
Dataset loading code: https://github.com/SmartDATA-Lab/Long_Term_Guided_Waves  
Descriptive article: Yang et al., *Scientific Data* (2025), DOI: https://doi.org/10.1038/s41597-025-05300-5  
Article text: https://pmc.ncbi.nlm.nih.gov/articles/PMC12162875/

**Rights gate:** the article identifies the dataset DOI and loading code, but this adapter does not assert the dataset's redistribution licence. Before fetching, retaining or redistributing the measurements, inspect and pin the Figshare record's current licence, version, terms and file hashes. The paper's own CC BY-NC-ND article licence must not be conflated with the dataset's licence. The initial software work is metadata-only and does not mirror the dataset.

## 2. A recent published baseline—and why its number is not SOTA

Gao et al., *Structural Durability & Health Monitoring*, published online 5 November 2026, compare five supervised classifiers for adjacent damage-severity classification in this long-term guided-wave setting (DOI: https://doi.org/10.32604/sdhm.2026.088197).

Reported three-day-training mean accuracies:
- LDA: 84.2%
- CNN: 82.6%
- SVM: 82.5%
- Random Forest: 80.5%
- MLP: 78.0%

The authors explicitly caution that the small gaps among the top three are not statistically confirmed. Their experiment uses one propagation path (6-2), one aluminium plate and one damage location, fixed hyperparameters, a single random seed (42), no repeated-seed significance tests, short approximately ten-day windows around individual transitions, and pairwise classification of two known adjacent severity states rather than an operational multi-class detector.

**Interpretation:** 84.2% is a study-specific adjacent-state accuracy result, not a universal SHM SOTA, deployed alarm accuracy, false-alarm rate, or cross-asset damage detector. The paper reports that fair-weather-only training did not materially improve generalization when the test stream retained weather variability. Its own discussion identifies environmental distribution shift as a major bottleneck and recommends multi-path fusion, domain adaptation/invariant features, healthy-state self-supervision, repeated runs and formal statistical testing.

## 3. Adapter contract: metadata first, bounded streaming later

The adapter must not require a 100-GB-class eager load or duplicate the owner dataset. It should initially emit a small manifest with:
- dataset DOI, source URL, source version, retrieval date and rights status;
- source paper DOI and exact reported scope;
- file names, sizes and content digests when locally available;
- channel/path names, sample counts, waveform length, sample rate if documented, timestamp range;
- environmental columns, units and missingness;
- damage-state labels and their exact time intervals;
- processing code revision, transformations, and exclusion rationale.

Read data lazily by file / time range / propagation path. Preserve raw waveforms and raw metadata as immutable source records. Derived windows, normalized signals, features, augmentations and labels get separate derived identities and digests. Never overwrite the raw source or infer a sample rate/unit that is not documented.

The dataset article describes 56 monthly `.pickle` files and eight channels / paths. Treat that description as a discovery aid, then verify each file against the frozen Figshare manifest before decoding. Pickle is executable serialization: do not load untrusted pickle files in a privileged or network-connected process. Use an isolated, least-privilege conversion step only after inspecting the source, with resource limits and no outbound network; persist a safer, typed interchange format with provenance and hashes.

## 4. Evaluation tracks—keep them separate

### Track A: source reproduction

Reproduce the 2026 paper's adjacent-severity, single-path setup using its declared splits and preprocessing only if the authors' exact code/data path and protocol can be reconstructed. Record every discrepancy. The reproduction is a baseline check, not the main score.

### Track B: temporal distribution shift

Train on an earlier contiguous period and evaluate on later periods, preserving event boundaries. Include weather/season strata, temperature range, sensor installation shifts, data gaps and drift. No random row split across adjacent time windows.

### Track C: path generalization

Evaluate all available paths separately, then leave one propagation path out. Do not label this cross-asset generalization: all paths still belong to one monitored plate and sensor installation.

### Track D: unseen severity / damage progression

Train on declared damage states and evaluate on later/unseen severity states where the data support it. Keep binary damage detection, adjacent-state classification, severity estimation and novelty detection as distinct tasks. A classifier that assumes the two current candidate states are known is not a general damage detector.

### Track E: sensor / acquisition change

Inject only declared software-side corruption into copies of the data: dropout, saturation, timestamp shifts, amplitude scaling, channel swap and bounded drift. Keep simulated faults labelled as synthetic fault injections; never present them as measured physical sensor failures.

### Track F: open-set and abstention

Withhold one damage class or environmental regime at a time. Measure whether the system reports UNKNOWN / OUT_OF_DOMAIN instead of forcing a known class. The held-out class is evaluator-only during training and model selection.

## 5. Leakage-resistant split rules

- Group all windows from the same contiguous acquisition event and overlapping source samples; they remain in one partition.
- Fit normalization, dimensionality reduction, feature selection, imputation, threshold selection, calibration and learned denoisers on training data only.
- Keep the complete test period untouched until code, hyperparameters, thresholds and analysis are frozen.
- Publish temporal cut dates, excluded intervals, label transitions and the reason for each exclusion.
- If a method requires healthy-baseline updates, distinguish a fixed pre-damage baseline from an adaptive online baseline. The latter must not silently learn away damage.
- Use repeated, predeclared seeds and paired temporal folds where the dataset supports them. Do not pretend overlapping temporal windows are independent replicates.
- Report uncertainty intervals and asset/event-level sample counts, not only window counts.

## 6. Baselines to reproduce and improve upon

At minimum:
1. Source-paper LDA, SVM, Random Forest, MLP and 1D CNN baselines.
2. Simple signal-quality / correlation threshold baseline.
3. Environmental compensation with explicit temperature/environment covariates.
4. PCA/SVD or long-short-term PCA residual baseline where data assumptions permit.
5. Mahalanobis / one-class kNN anomaly baseline.
6. Unsupervised healthy-state autoencoder baseline.
7. A multi-path fusion baseline, with path dropout and common-cause fault controls.
8. Symthaea acquisition/uncertainty/abstention policy only after a matched data and compute protocol is frozen.

No Transformer, temporal CNN, state-space or custom architecture should be assumed superior. Add them only as versioned candidate baselines with parameter counts, training budget, tuning budget and model-selection rules matched to the alternatives.

## 7. Metrics that matter operationally

Report separately:
- adjacent-state macro-F1 / balanced accuracy and per-state recall;
- damage-detection probability by class/severity;
- false alarms per monitored asset-day (and per monitored path-day);
- detection delay from the labelled state transition and detection lead time before inspection, where labels permit;
- missed-detection intervals and time-to-detection distribution;
- false-alarm duration / alarm clustering and operator review burden;
- calibration (Brier score, log loss, reliability curves, interval coverage);
- risk-versus-coverage and abstention rate under unseen regimes;
- robustness under temperature/weather strata, drift, missing channels and installation shifts;
- cross-path consistency and failure under a shared/common-cause perturbation;
- inference latency, compute, energy and memory as separate deployment metrics.

Window accuracy is not the primary operational endpoint. A safety-monitoring comparison should include an explicitly costed false-alarm / missed-detection policy and preserve a hard, independently defined inspection/engineering authority boundary.

## 8. Proposed gates

**D0 — rights and manifest**
- current dataset version and licence verified directly at the dataset repository;
- manifest and file hashes frozen;
- parser isolated; no eager full-dataset load.

**D1 — source reproduction**
- reproduce at least the declared simple baselines, with a protocol deviation log;
- report the actual test unit and split, not just the headline paper number.

**D2 — leakage-resistant generalization**
- temporal holdout, drift/weather strata, path holdout and open-set evaluation;
- all learned preprocessing fit on training only;
- repeated seeds and paired intervals.

**D3 — decision quality**
- calibrated risk/coverage, false alarms per asset-day, detection delay and inspection burden;
- compare against conventional rules and source-paper methods at matched budgets.

**D4 — prospective physical SHM**
- separate engineering review, safe specimen and sensor installation plan, calibrated independent reference measurements and human authorization;
- no structure safety or protective-product claim from this dataset alone.

## 9. What would constitute a meaningful improvement

A meaningful result is not merely >84.2% window accuracy. A credible improvement would hold or improve damage sensitivity while reducing false alarms per asset-day and maintaining calibration under genuinely later temporal periods, held-out propagation paths and sensor/environment shift—using repeated seeds and identical information access.

The stronger target is **earlier reliable detection at lower total inspection burden, without increased missed-damage risk**. It must be evaluated on held-out temporal/event data and then prospectively confirmed on an independently instrumented specimen before any physical-performance claim.

## 10. Non-goals

No damage inference on occupied/critical assets, no automatic safety declaration, no autonomous shutdown/actuation, no certification, no physical repair authorization, no hidden modification of the source data, no claim of cross-asset transfer from a single plate, and no claim that classifier accuracy proves structural integrity.
