# Long-Term Guided-Wave Damage Observability Benchmark Adapter v0.1

**Programme owner:** PROTECT-DAMAGE-OBSERVABILITY-001 (#5267)  
**Parent:** PROTECTIVE-SYSTEMS-001 (#5246)  
**Status:** adapter/reproduction protocol proposal; no data downloaded, no models trained, no physical qualification  
**Research update:** 2026-11-15

## 1. Why this dataset matters

A public *Scientific Data* dataset describes long-term outdoor guided-wave structural-health monitoring at the University of Utah. The associated article reports approximately 4.5 years of monitoring, approximately 6.4 million guided-wave observations, environmental variation including temperature from -12.2 °C to 52.5 °C, rain/snow effects, sensor drift and installation shifts, and 13 introduced damage types/states. The article describes eight guided-wave channels / propagation paths in its data records and time/environment metadata.

Dataset DOI: https://doi.org/10.6084/m9.figshare.28112504  
Dataset loading code: https://github.com/SmartDATA-Lab/Long_Term_Guided_Waves  
Descriptive article: Yang et al., *Scientific Data* (2025), DOI: https://doi.org/10.1038/s41597-025-05300-5  
Article text: https://pmc.ncbi.nlm.nih.gov/articles/PMC12162875/

**Rights gate (updated 2026-11-15):** OpenAIRE's DataCite-linked metadata currently labels the dataset CC BY and surfaces the base DOI and a version-specific DOI (.v1). This is a useful licence signal, but the primary Figshare record/API could not be directly retrieved in this research session. Before fetching, retaining or redistributing measurements, confirm the exact current Figshare version, licence, terms, file list and hashes from the primary record. Do not infer the licence of individual files or later versions solely from the article's licence or an aggregator summary. The initial software work remains metadata-only and does not mirror the dataset.

Licence/version references:
- OpenAIRE metadata view: https://oamonitor.ireland.openaire.eu/national/search/dataset?pid=10.6084/m9.figshare.28112504
- Figshare versioning rules: https://info.figshare.com/user-guide/how-versioning-works/
- Version-aware citation targets: https://doi.org/10.6084/m9.figshare.28112504 and https://doi.org/10.6084/m9.figshare.28112504.v1

## 2. Source-integrity audit: cadence, data units and distribution provenance

The paper's own cadence is reconcilable: its Methods describe original acquisition about every 8.6 seconds, while the published benchmark retains one in twenty acquisitions, giving about 176 seconds between retained records. The apparent internal contradiction is explained by downsampling. A separate conflict remains between the paper/repository (56 monthly pickle files, eight guided-wave paths) and the SmartDATA Lab landing page (HDF5, 14 PZT sensors, 30-second cadence). In addition, the repository README now says the original Figshare link is unavailable and points to a Dropbox distribution. Treat these as separate provenance/version claims; do not combine schemas or cadence assumptions.

| Source | Description | Benchmark treatment |
|---|---|---|
| Yang et al. *Scientific Data* (2025), Data Records | 56 monthly `.pickle` files; 8 guided-wave paths per acquisition; example monthly matrix 14,997 × 8 × 2,000 | Primary published data-layout claim; verify each file and timestamp range |
| Yang et al. *Scientific Data* (2025), Methods | Original acquisition about every 8.6 seconds at 1 MHz sampling; 10,000 original samples (10 ms) per waveform | Original collection cadence and signal capture; not the cadence of the downsampled public benchmark |
| Same paper, Data Records | Public benchmark selects every twentieth original measurement (~176 seconds), trims each guided wave to 2,000 samples (2 ms), and stores 56 monthly `.pickle` files | Use the retained public timestamps and payload, not the original acquisition rate |
| Same paper, Data Records example | `measurements 2018_04.pickle`: 14,997 acquisition rows × 8 paths × 2,000 samples; eight paths 5-1 through 5-4 and 6-1 through 6-4 | One timestamp row is one acquisition; it contains eight path waveforms. Never call all 8 waveforms one acquisition row |
| Same paper, abstract/summary | ~6.4 million guided-wave records over ~4.5 years | The headline count is ambiguous between acquisition rows and path-wave records; independently count rows, path records and waveform samples from the payload |
| SmartDATA Lab landing page (updated 7 April 2026) | says >7.6 million signals, 30-second cadence, 14 PZT sensors and HDF5 | Conflicts with the paper/repository's eight guided-wave paths and monthly pickle layout; do not merge unless the maintainer identifies this as a new version |
| SmartDATA Lab GitHub README (retrieved 15 Nov 2026) | Says the original dataset link is no longer available and supplies a Dropbox folder as the current download link | Treat the Dropbox folder as a distribution transition. Pin its exact contents/version/rights before retrieval |
| OpenAIRE/DataCite metadata aggregator | lists DOI `10.6084/m9.figshare.28112504`, version `.v1`, and CC BY | Useful metadata lead, not a substitute for primary dataset terms or confirmation that the Dropbox payload is byte-identical to that version |

Primary sources:
- Paper and data-layout description: https://pmc.ncbi.nlm.nih.gov/articles/PMC12162875/
- DOI: https://doi.org/10.6084/m9.figshare.28112504
- Versioned DOI reported by OpenAIRE: https://doi.org/10.6084/m9.figshare.28112504.v1
- SmartDATA Lab landing page: https://smartdata.ece.ufl.edu/index.php/2025/07/23/long-term-guided-wave-dataset-under-real-world-conditions-dataset/
- Repository README (current distribution notice): https://github.com/SmartDATA-Lab/Long_Term_Guided_Waves
- Repository loader: https://github.com/SmartDATA-Lab/Long_Term_Guided_Waves/blob/main/load_dataset.py
- Dropbox folder linked from the README (do not ingest until version/rights/hash are confirmed): https://www.dropbox.com/scl/fo/cigpra8cde187g7321etz/ANJMc309bWNDdNDe9NdJCQ8?rlkey=bdv8xwkgxybo4umz4mbyc1mnr&dl=0
- OpenAIRE metadata view: https://oamonitor.ireland.openaire.eu/national/search/dataset?pid=10.6084%2Fm9.figshare.28112504

### Canonical count vocabulary

Never use a single unqualified "number of measurements". The manifest must independently count:
1. acquisition rows / timestamps;
2. guided-wave path records (acquisition rows × valid paths, allowing missing paths);
3. samples per waveform;
4. monthly source-file counts and byte sizes.

The example month has 14,997 acquisition rows and eight paths per row, or up to 119,976 path waveforms before missingness. The paper's original cadence (~8.6 s) and retained public benchmark cadence (~176 s) are different by design: the benchmark selects one record in twenty. The README's Dropbox relocation is a distribution change, not by itself a dataset-version identity. The SmartDATA Lab page's 30-second / HDF5 / 14-sensor description must not be blended into the paper's eight-path pickle schema without a maintainer-confirmed version mapping.

### Rights and distribution gate refinement

OpenAIRE's DataCite-linked record reports CC BY for two dataset versions, including DOI `.v1`. That is a useful indication, not primary proof of the licence for the current distribution. The GitHub README retrieved on 15 November 2026 states that the original Figshare download link is no longer available and redirects users to a Dropbox folder. The primary Figshare record/API returned HTTP 403 in this audit, and the README does not establish that Dropbox contents are byte-identical to a particular DOI version. Therefore the following remain **not directly verified**: current distribution version, exact version-level licence/terms, payload file inventory, per-file hashes, and equivalence to the cited Figshare version. Do not download, mirror, redistribute or adapt the Dropbox payload until the data owners or authoritative metadata confirm those items. The GitHub loader's GPLv2-or-later code licence and the paper's CC BY-NC-ND article licence are separate from the dataset licence.

### D0 acceptance additions

- Pin the dataset DOI/version, the current README-linked distribution location, the data owner, direct licence/terms evidence, retrieval date and exact payload manifest. Confirm whether the Dropbox payload is byte-identical to the cited Figshare version or a new version.
- Record the cadence chain explicitly: original 8.6 s collection → every-20th-record public sampling (~176 s). Verify actual deltas from the timestamps.
- Record the discrepancy between the SmartDATA landing page (HDF5/14 sensors/30 s) and paper/repository schema (monthly pickle/eight paths) as unresolved unless the data owner supplies a mapping.
- Count separately: source files, acquisition rows/timestamps, path waveforms per path, valid/missing paths per row, samples per waveform, and total scalar waveform samples.
- Assert container format and shape from actual file signatures and decoded metadata; never infer them from landing-page prose.
- Compute timestamp-difference distributions (min/median/p95/max) and report gaps/duplicates; do not hard-code cadence assumptions.
- Treat unexplained format/count/schema differences as a new dataset version or failed source-integrity gate, not a silent preprocessing choice.
- Inspect payload provenance and hashes before any untrusted pickle decoding. Use a no-network, least-privilege, resource-limited conversion process; preserve raw bytes and convert into a typed, content-addressed interchange format.

## 3. A recent published baseline—and why its number is not SOTA

Gao et al., *Structural Durability & Health Monitoring*, published online 5 November 2026, compare five supervised classifiers for adjacent damage-severity classification in this long-term guided-wave setting (DOI: https://doi.org/10.32604/sdhm.2026.088197).

Reported three-day-training mean accuracies:
- LDA: 84.2%
- CNN: 82.6%
- SVM: 82.5%
- Random Forest: 80.5%
- MLP: 78.0%

The authors explicitly caution that the small gaps among the top three are not statistically confirmed. Their experiment uses one propagation path (6-2), one aluminium plate and one damage location, fixed hyperparameters, a single random seed (42), no repeated-seed significance tests, short approximately ten-day windows around individual transitions, and pairwise classification of two known adjacent severity states rather than an operational multi-class detector.

**Interpretation:** 84.2% is a study-specific adjacent-state accuracy result, not a universal SHM SOTA, deployed alarm accuracy, false-alarm rate, or cross-asset damage detector. The paper reports that fair-weather-only training did not materially improve generalization when the test stream retained weather variability. Its own discussion identifies environmental distribution shift as a major bottleneck and recommends multi-path fusion, domain adaptation/invariant features, healthy-state self-supervision, repeated runs and formal statistical testing.

### Exact reproduction boundary and leakage adversary

The full paper clarifies the scope precisely: the 2026 classifier benchmark uses only propagation path 6-2 on one 53 cm × 53 cm × 3 mm aluminium plate, one central damage location, and thirteen adjacent binary severity-pair tasks. Each pair uses roughly ten days around one transition, with the last five days of the earlier state and first five days of the later state partitioned into three-day training and two-day test windows. It fixes all classifier seeds at 42, uses untuned hyperparameters, and does not report repeated-seed significance tests. It explicitly says this does not establish multi-year, multi-season, cross-structure, cross-layout or cross-damage-location generalization.

One especially important reproduction trap: the paper's fair-weather filtering autoencoder is fitted on the complete five-day segment, which includes the later test windows, even though the test examples themselves are not filtered. The authors identify this as test-window exposure and recommend fitting the filter only on training windows. Therefore the adapter must preserve two distinct baselines:
1. **Faithful paper reproduction:** replicate the published filtering pipeline exactly and label the transductive test-window exposure.
2. **Leakage-resistant variant:** fit all preprocessing and filtering models only on training windows, then apply the frozen transform to held-out test windows.

Never compare one policy using the paper-faithful transductive preprocessing against another using training-only preprocessing as though they had equivalent information access.

Dataset details from the paper: eight PZT transducers were installed on the plate, but only six (two transmitters and four receivers) were used for acquisition, yielding eight transmitter–receiver paths (5-1 through 5-4 and 6-1 through 6-4). Public waveforms contain 2,000 samples at the downsampled 1 MHz record length of 2 ms, with approximately 176-second spacing. The 2026 classifier study used only path 6-2 after qualitative path selection; it did not compare the other paths or multi-path fusion.

Source-paper evaluation is a benchmark of adjacent-pair classification accuracy. The Protective Systems adapter adds operational evaluation: false alarms per asset-day, detection delay, calibration, open-set abstention and inspection burden. Do not claim that these measures were assessed in the published 84.2% result.

## 4. Adapter contract: metadata first, bounded streaming later

The adapter must not require a 100-GB-class eager load or duplicate the owner dataset. It should initially emit a small manifest with:
- dataset DOI, source URL, source version, retrieval date and rights status;
- source paper DOI and exact reported scope;
- file names, sizes and content digests when locally available;
- channel/path names, sample counts, waveform length, sample rate if documented, timestamp range;
- environmental columns, units and missingness;
- damage-state labels and their exact time intervals;
- processing code revision, transformations, and exclusion rationale.

Read data lazily by file / time range / propagation path. Preserve raw waveforms and raw metadata as immutable source records. Derived windows, normalized signals, features, augmentations and labels get separate derived identities and digests. Never overwrite the raw source or infer a sample rate/unit that is not documented.

The public benchmark is downsampled to approximately 176-second spacing (one in twenty original acquisitions), with eight paths and 2,000 retained samples per path waveform. The 8.6-second original measurement cadence and the 1 MHz/10,000-sample original waveform description apply to the pre-downsampled collection, not the public benchmark payload. Verify this against the timestamp and file metadata before ingest. Do not assume that the entire corpus must be loaded in memory. Use bounded, file/time/path streaming. Pickle is executable serialization: do not load untrusted pickle files in a privileged or network-connected process. Use an isolated, least-privilege conversion step only after inspecting the source, with resource limits and no outbound network; persist a safer, typed interchange format with provenance and hashes.

## 5. Evaluation tracks—keep them separate

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

## 6. Leakage-resistant split rules

- Group all windows from the same contiguous acquisition event and overlapping source samples; they remain in one partition.
- Fit normalization, dimensionality reduction, feature selection, imputation, threshold selection, calibration and learned denoisers on training data only.
- Keep the complete test period untouched until code, hyperparameters, thresholds and analysis are frozen.
- Publish temporal cut dates, excluded intervals, label transitions and the reason for each exclusion.
- If a method requires healthy-baseline updates, distinguish a fixed pre-damage baseline from an adaptive online baseline. The latter must not silently learn away damage.
- Use repeated, predeclared seeds and paired temporal folds where the dataset supports them. Do not pretend the 13 adjacent-state pairs are independent physical assets or independent field campaigns.
- The classifier paper reports class-balanced pair splits (1,006 test samples per class); accuracy is therefore interpretable for its particular task, but report per-class sensitivity/recall and confusion matrices anyway.
- Report uncertainty intervals and asset/event-level sample counts, not only window counts.
- Reproduce the paper-faithful autoencoder preprocessing separately from a strict training-only preprocessing variant; disclose the difference and do not tune on the sealed test windows.

## 7. Baselines to reproduce and improve upon

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

## 8. Metrics that matter operationally

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

## 9. Proposed gates

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

## 10. What would constitute a meaningful improvement

A meaningful result is not merely >84.2% window accuracy. A credible improvement would hold or improve damage sensitivity while reducing false alarms per asset-day and maintaining calibration under genuinely later temporal periods, held-out propagation paths and sensor/environment shift—using repeated seeds and identical information access.

The stronger target is **earlier reliable detection at lower total inspection burden, without increased missed-damage risk**. It must be evaluated on held-out temporal/event data and then prospectively confirmed on an independently instrumented specimen before any physical-performance claim.

## 11. Non-goals

No damage inference on occupied/critical assets, no automatic safety declaration, no autonomous shutdown/actuation, no certification, no physical repair authorization, no hidden modification of the source data, no claim of cross-asset transfer from a single plate, and no claim that classifier accuracy proves structural integrity.
