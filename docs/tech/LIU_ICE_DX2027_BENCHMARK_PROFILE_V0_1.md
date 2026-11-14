# LiU-ICE 2027 edition profile and admission contract

**Status:** source-metadata profile audited; archive bytes, file-level schema, and model performance not audited.  
**Owner:** Symtropy / LiU-ICE external diagnosis lane.  
**Parent:** `research/maritime-dx2026-clean-integration-v0-1` (PR #5038).  
**Profile:** `liu-ice-dx2027-profile-v0.1.json`.

## Why this is a separate edition

Linköping University now publishes a distinct **LiU-ICE Industrial Fault Diagnosis Benchmark 2027** page for the DX/Safeprocess 2027 cycle:

- 2027 official page: https://vehsys.gitlab-pages.liu.se/dx_safeprocess_benchmarks/liuice/
- 2026 official page: https://vehsys.gitlab-pages.liu.se/dx_benchmarks/liuice/liuice_index

The 2027 page reports **41 training datasets**, whereas the 2026 page reports **26**. Both describe 20 Hz engine test-bench recordings, two actuator/control-intent signals, eight sensor signals, WLTP-only training and test data that may use other driving cycles. Both define the same general five-way sample-level isolation interface. The count is edition-specific: do not mutate the 2026 constant or reuse a 2026 split, digest, result or score as a 2027 result.

## Source metadata captured

The profile records the 2027 page's stated contract:

- 20 Hz CSV, time column first, one sample per row.
- Sensors: `ypic`, `yTic`, `ypim`, `ywaf`, `yxpos`, `yomega`, `ypamb`, `yTamb`.
- Actuator requests: `umf` (injected-fuel-mass request) and `uwg` (wastegate-position request). These are **control intent**, not proof of realized actuator motion.
- `yxpos` is described as throttle actuator position; retain it as candidate feedback until the instrument/I/O dictionary and calibration establish its measurement semantics.
- The page reports 41 training datasets, two to seven recordings per known fault family, WLTP training and possible other test cycles.
- For faulty recordings, fault onset is stated to occur at approximately 120 s and remain to the end.
- Sensor faults are multiplicative: (y=(1+f)x), and the page notes closed-loop feedback can make the fault affect plant behaviour as well as the sensor output.
- The supplied analytical model has unknown parameters; the page explicitly says the available signals are insufficient to determine them uniquely.
- The sample-level interface calls `Input(sample)` and requires a new output after each sample, with no retrospective alteration of prior outputs. Detection is Boolean; isolation has five ordered slots `(f_pic, f_pim, f_waf, f_iml, f_x)`. A non-detection returns a zero-sum isolation vector; a detection returns a unit-sum vector.
- The official total score is `((1-FAR)+TDR+FIA)/3`, using secret test data and training-only calibration.

## Source ambiguities preserved rather than guessed

The page has three details that must be resolved before data ingestion or an official adapter is declared conformant:

1. It first describes one nominal dataset plus seven fault-scenario datasets, then explicitly states that training contains 41 datasets and says there are two to seven datasets from each known fault. Those statements do not unambiguously specify how the 41 recordings are partitioned. The profile uses 41 as the explicit total and records the per-fault range, but the recording manifest and organizer clarification must settle the partition before splits are frozen.
2. Its fault/filename labels include `y_pim` and `y_waf`, while the output vector uses `f_pim` and `f_waf`. The profile preserves the source spellings and marks both crosswalks for verification against the official evaluator; it does not silently normalize them.
3. The interface section permits MATLAB or Python, while the evaluation section says the submitted code should be executable in Python and cannot include other licensed software. Confirm the precise runtime/package requirement with the organizers before implementing the submission adapter.

The official page exposes a downloadable training link, but a link is not proof that the archive has been retrieved, that its bytes are correct, or that redistribution is permitted. Current status is therefore `linked_not_downloaded`; archive and recording-manifest hashes are null; file inventory is empty; licence/redistribution status is unverified. No dataset result or score is claimed.

## Calibration adequacy: a hard sample-size limit

The official page reports only two to seven datasets per known fault family. The current finite-sample split-conformal protocol requires at least 9 independent calibration episodes for 90% marginal coverage, 19 for 95%, and 99 for 99%. Since the maximum stated population per family is seven—and the available recordings must also serve training/evaluation design—**a per-fault 90% split-conformal guarantee cannot be claimed from the stated training population**.

Pooling fault families may yield a pooled marginal coverage statement only under the relevant exchangeability assumptions. It does not establish per-fault coverage, and it can conceal poor coverage on a rare or difficult fault. The profile and Rust validator now hard-code this limit. For per-fault calibration, obtain additional independent recordings or use a different, explicitly justified uncertainty method with its own finite-sample/assumption statement; do not relabel pooled coverage as class-conditional coverage.

## Why this advances the research programme

The 2027 dataset expansion is not simply "more rows." It creates an edition-to-edition robustness question. Freeze and test separately:

- **DX2026 replication:** recover the exact 26-recording edition, benchmark code, splits and baseline procedure.
- **DX2027 replication:** independently acquire and audit the 41-recording edition; create edition-specific recording/split/digest manifests.
- **Cross-edition transfer:** train on one edition and evaluate on another only where fault labels, signal semantics, preprocessing and operating conditions can be matched. Report transfer in both directions and keep it distinct from the official score.
- **Protocol generalization:** preserve WLTP training restrictions and use held-out cycles as actual OOD/operating-regime tests, not random row splits.
- **Identifiability:** keep the admissible hypothesis set when model parameters are not uniquely identifiable. A low residual or optimizer-selected parameter vector is not proof of a unique physical cause.

### Metric nomenclature in the 2026 paper

The 2026 paper's abstract refers to its isolation component as TIR, while Table 7 and the official DX2026 score definition label the component FIA (fault-isolation accuracy). The benchmark profile uses FIA for the official score calculation and preserves the TIR wording as a paper-level nomenclature difference. Do not substitute historical 2024 TIR values for the 2026 FIA component.

The 2026 published grey-box solution is a comparator, not a measured Symtropy baseline. Keep its two reported evaluation levels separate:

- Its shared-dataset validation table reports 92.29% global score over 41 files (FAR 0.68%, TDR 78.75%, FIA 98.97%).
- The competition organizers' official hidden-test table reports 88.01% overall (FAR 7.86%, TDR 92.26%, FIA 79.62%).

The two tables are not the same evaluation set and must not be averaged or quoted as interchangeable. The hidden-test outcome is a literature reference; reproducing it requires the exact hidden evaluation inputs or an official rerun. A public reproduction on the released data must be labelled as a separate result.

The published grey-box system is a serious comparator: it uses an MVEM-derived structural model, MSO decoupling, four virtual sensors and recurrent grey-box estimators, with residual/diagnosis logic. On the paper's reference CPU it reports 0.82 ms average inference per sample and a 49.5 ms observed maximum. It also uses a 0.75 s (15-sample) debounce and a 90 s startup calibration/warm-up; the authors explicitly discuss the resulting detection delay and the risk of startup fault masking.

The official hidden-test table exposes two highly informative weak cells: score 45.64% for FTP75 Standard `f_pim -10%` (TDR 21.12%) and 44.87% for WLTP `f_pic +8%` (TDR 100%, 1-FAR 34.61%, FIA 0%). The first is primarily a missed-detection boundary; the second detects the anomaly but fails to control false alarms and isolate it. These require different remedies and should be preregistered separately, not folded into one average target.

## Next benchmark sequence

1. Obtain the 2027 archive through the official route; verify bytes, checksum, archive structure, licence/redistribution terms and data dictionary.
2. Run the profile audit, then validate every CSV's columns, units, time monotonicity, sample rate, missingness, recording identity and onset semantics.
3. Freeze 2027 data and split digests before training. Split by entire recording; hold out driving cycles and fault magnitudes. Keep calibration recordings independent of model fitting and test.
4. Reproduce the official analytical model / Fault Diagnosis Toolbox baseline first, then add SPC/residual baselines, feature classifiers, grey-box methods and deep models.
5. Implement the competition adapter in the organizer-confirmed runtime. Differentially test its every-sample output against the Rust contract using shared golden vectors.
6. Compare 2026↔2027 cross-edition transfer without pooling incompatible labels or pretending separate engine/test-bench evidence establishes fleet transfer.
7. Publish full score vector, event metrics, calibration, abstention/risk-coverage, per-fault/per-cycle results, latency/resource use and digest-bound run receipts.

## Verification and claim ceiling

The Rust profile-audit binary contains negative controls for edition-count substitution, fabricated archive hashes, fault-label normalization, isolation-order mutation, deleted source ambiguity records and cross-edition score promotion.

No Rust compile/test or hosted workflow pass is claimed at authoring time. This is a source-metadata contract, not an archive audit, model, competition adapter, physical test or diagnostic result. It does not authorize controller writes, actuation, maintenance or restriction clearance.
