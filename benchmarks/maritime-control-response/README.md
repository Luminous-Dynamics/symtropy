# LiU-ICE command-response benchmark lane v0.1

**Parent owner:** Symtropy issue #4628, MARITIME-CONTROL-AWARE-DIAGNOSIS-001  
**External benchmark:** [LiU-ICE Industrial Fault Diagnosis Benchmark 2026](https://vehsys.gitlab-pages.liu.se/dx_benchmarks/liuice/liuice_index), DX37  
**Status:** Rust evaluator/diagnostic contracts have passed formatting, locked tests and strict Clippy on a normalized prior revision (112 unit tests in the latest archive-audit attempt); archive retrieval, publisher size/MD5 verification and safe extraction passed. A real-file baseline-header defect was identified and corrected in source; the updated exact-head tests and raw CSV audit are pending  
**Claim ceiling:** source contracts, synthetic fixtures and dataset metadata protocols only; the DX2026 figures are published references, not Symtropy results; no trained Symtropy model, public-data score, competition submission, or physical validation claim

## External benchmark fit

The public LiU-ICE page describes an engine test-bench dataset at 20 Hz with two requested commands and eight measured signals. The commands are \`umf\` (requested injected fuel mass) and \`uwg\` (requested wastegate actuator position). The signal list includes intercooler pressure and temperature, intake manifold pressure, air-filter mass flow, throttle actuator position, engine speed, ambient pressure and ambient temperature.

The evaluator deliberately maps \`umf\` and \`uwg\` to \`CONTROL_INTENT\`. It maps \`yxpos\` only to \`CANDIDATE_ACTUATOR_FEEDBACK\`, pending review of the instrument/I-O dictionary. It is not a wastegate-position signal and is not promoted to verified feedback. The other listed measurements are classified as physical response or disturbance context according to their public descriptions.

The benchmark lists four known fault classes: intake-manifold leakage and multiplicative faults affecting \`ypic\`, \`ypim\` and \`ywaf\`. Its official isolation interface has a fifth slot for unknown/unclassified faults. The public training set is reported as 26 recordings, with only 2–7 recordings per known fault family; evaluation may include driving cycles other than WLTP. See the [official benchmark description](https://vehsys.gitlab-pages.liu.se/dx_benchmarks/liuice/liuice_index).

## Second marine-engine dataset: separate validation track

A second useful public data asset is [Marine Engine Fault, Zenodo record 10.5281/zenodo.19857425](https://zenodo.org/records/19857425), released under CC BY 4.0. Its source metadata identifies a Matsui Iron Works MU323DGSC test-bench engine and describes 16 CSV files: one baseline reference recording and 15 anomaly-scenario recordings, about 115,000 time-stamped samples and roughly 70 measured/derived channels. The CSVs have a three-row header, with separate 70-column baseline and 73-column scenario schemas. The raw archive header inspection confirms the baseline first row starts with `Time`, symbol `T`, unit `sec`; scenario files start with `Time_abs`, `Time_rel`, `Anomaly State`, with both time fields in seconds. The variable dictionary marks `Time_abs`/`Time_rel` as scenario-only. A file index, variable dictionary and normalization script are included.

The five reported anomaly families are compressor air-filter clogging, air-cooler fouling, injection-valve nozzle clogging, cooling-water pump cavitation and turbine degradation. Loads vary by scenario (40–85% in the listed tests). The metadata notes that two channels—Compressor Filter Loss and Turbine Back Pressure—are empty in five runs. Two injection-nozzle recordings contain anomalous operation only, so they cannot establish onset latency without an independently recorded injection time.

**Source identity needs care.** The opened Zenodo record uses DOI `10.5281/zenodo.19857425`, while Aalto University's research portal cites `10.5281/zenodo.19857424` for the same dataset. The repository records both and uses the Zenodo landing page as the file locator; resolve the DOI/version relationship before citing one identifier as exclusive. The Zenodo page lists a 26,538,633-byte ZIP and publisher MD5 `f4d246c1bb46e05b26e56221acc2606c`. A GitHub Actions runner successfully downloaded the exact-size archive, independently matched the publisher MD5, and computed SHA-256. The archive extracts to 18 CSV entries while the metadata describes 16 recording CSVs. The audit contract explicitly separates recording files from metadata exports instead of silently dropping two files. The independent auditor now parses this inventory; the raw file-level receipt has not yet completed, so no exact 16+2 classification, time-series count or raw-payload SHA-256 result is claimed.

The associated data-descriptor preprint is [arXiv:2607.19444](https://arxiv.org/abs/2607.19444), posted 21 July 2026. Its abstract describes the engine, load programme and five induced anomaly scenarios. A secondary review flags possible proxy interventions and largely qualitative technical validation; that is an audit lead, not an authoritative correction. Until the archive README and full manuscript have been checked, treat the labels as identities of controlled experimental scenarios—not as proof that each intervention reproduces the full causal physics, severity progression or failure envelope of an in-service marine fault.

This is **not** an additional engine/fleet validation. The source identifies one MU323DGSC test-bench identity. Nor is it a label-compatible extension of LiU-ICE: the fault families, channel set, headers, label semantics and submission protocol differ. The [external-marine-engine-dataset-v0.1.json](external-marine-engine-dataset-v0.1.json) freezes a separate audit and evaluation track. The per-file input contract is defined by [external-dataset-file-manifest.schema.json](external-dataset-file-manifest.schema.json) and implemented in Rust at `src/external_dataset.rs`:

- Archive/file audit: verify the ZIP bytes, file inventory, three-row headers, variable dictionary, units, per-file signal presence and normalization script.
- Held-out-load evaluation: hold out entire recording files/load points, not random time rows or overlapping windows.
- Leave-one-fault-family-out: use a whole family as an exploratory unknown-fault class, with the known-class/unknown-label semantics declared before training.
- Missing-channel track: preserve the five runs with blank fields; do not turn empty cells into zero or evidence of a physical sensor fault.
- Event-latency track: use only files with independently supported onset/offset labels.
- Cross-benchmark transfer: evaluate LiU-ICE and this dataset separately, then attempt a reviewed component/function-level signal map. Never pool their sample labels or report a single shared score.

Every run must bind separate dataset archive, file-manifest, index, variable-dictionary, normalization-script, signal-map, fault-map and split digests. The Rust manifest validator checks expected file/family counts, schema/header shape, allowed missing channels, anomaly-only eligibility and exact whole-file split assignment; it does not parse or authenticate the CSV payload. A separate raw-data auditor now exists in `src/csv_audit.rs`, exposed by the `marine-dataset-audit` CLI. It verifies each CSV's SHA-256, 3-row header, 70/73-column width, role-specific monotonic time (`Time` in baseline reference, `Time_rel` in scenarios), binary `Anomaly State` for scenarios only, role-specific time units, fully blank channels and per-file missingness, without running the publisher's normalization script or inferring fault classes from filenames. It also hashes the dataset index, variable dictionary and normalization script. The exact-head workflow `.github/workflows/marine-dataset-archive-audit.yml` downloads the publisher archive, verifies the reported size/MD5, computes SHA-256, extracts it without executing scripts, runs Rust synthetic tests and the real archive audit, and retains a machine-readable receipt. The first audit reached the real payload: all 112 unit tests passed, but the parser rejected `Reference_Data.csv` because it had incorrectly required scenario columns `Time_abs`/`Time_rel` on the distinct baseline file. The source has now been corrected to bind the baseline `Time`/`sec` column at index 0 and scenario `Time_rel`/`sec` at index 1; a new test rejects role mixing and non-second units. The audit report schema is versioned v0.2 and records the chosen time-column name, zero-based index and unit. The next exact-head run must still pass before this is described as a successful raw-payload audit. Similar signal names or the same engine model designation do not establish identical semantics, measurement provenance or independent engine transfer.

The Zenodo record lists CC BY 4.0; the ZIP has been downloaded and its publisher-reported size and MD5 independently matched on a GitHub runner. The archive contents, exact recording/metadata file inventory, embedded notices, README, variable dictionary, normalization script and physical run identities remain under audit. No model score or validation result is claimed for this dataset.

## Implemented evaluator primitives

- Typed signal-role registry, including the distinction between requested commands and measured plant response.
- Exact public competition score calculation: \`((1 - FAR) + TDR + FIA) / 3\`, with five isolation probabilities in the published order.
- FIA uses the mean probability assigned to the true diagnosis among correctly detected fault samples; it is not top-1 accuracy. Undefined FIA denominators are explicitly reported.
- Exact row-key matching for labels and predictions; probabilities must satisfy the official detection-dependent sum contract.
- Whole-recording split manifests and optional held-out-driving-cycle split separation.
- Monotonic recording-row checks and missing-channel preservation without zero or nominal-command imputation.
- Command-conditioned response residual evaluation with distinct command-availability, prediction and physical-observation timestamps. It requires command availability at or before prediction, response strictly after prediction, a bounded response horizon, and command freshness measured at prediction time. Missing, stale, future, unsupported and uncalibrated states are not normal. A residual mismatch never independently claims an actuator fault.
- Split-conformal calibration using one maximum standardized residual per independent recording/episode, not 20 Hz samples or overlapping windows.
- Calibration receipts retain their sorted episode identities and maximum residuals. Their digests and finite-sample order statistic are recomputed; the threshold must match the frozen model, regime, training manifest and split manifest.

## Read-only value of information: choose the next evidence, not the next action

The exposed `active_probe` module ranks additional candidate evidence by expected information gain over an explicit competing-hypothesis prior. For each candidate, it computes the entropy before acquisition, expected entropy after each possible observation, and expected information gain. A deterministic utility adjusts that gain by declared cost, delay and risk scales after hard eligibility checks. The candidate manifest binds its outcome likelihood table, model/calibration references, stable hypothesis-space digest, cost, delay, risk-assessment digest and authority-evidence reference. Each selection portfolio separately binds the exact prior-distribution digest used in that decision.

Three candidate classes are explicit:

- **Passive measurement:** eligible only with an upstream approved-read-only authority state and a digest reference to the authority evidence.
- **Operator inspection:** eligible only when the policy enables it and the caller supplies a human-authorized state plus an authority-evidence reference.
- **Controlled perturbation:** always excluded. It requires a separate safety case, hazard analysis, asset-owner review and explicit control authority; this selector cannot authorize or execute it.

The selector records rejected candidates and reasons, rejects zero-information probes by default, uses deterministic tie-breaking and returns `execution_performed=false` and `actuation_authorized=false`. The authority digest is only a content reference: signer identity, permission scope, expiry, revocation and issuer authority must be verified upstream. The risk score is a caller-supplied policy input, not a calibrated probability; its assessment method, reviewer, scope, expiry and revocation must be independently verified upstream. Likelihood-model, calibration and risk-assessment digests bind evidence but do not prove statistical calibration or safety.

The active evidence comparator protocol is frozen in [active-evidence-selection-v0.1.json](active-evidence-selection-v0.1.json), schema version v0.4. It compares no additional evidence, cheapest eligible, maximum information gain, information gain per cost, seeded-random eligible, and the risk-adjusted worst-case information-gain selector, and maximum worst-case pairwise Chernoff information over the *same* eligible candidate set.

### Sequential identity: fixed model, changing belief

The selector separates two identities that must not be conflated:

- **Hypothesis-space digest:** sorted, unique hypothesis IDs only. Candidate likelihood models bind this immutable label-space identity, so the same candidate can be reused across later evidence rounds.
- **Prior-distribution digest:** sorted hypothesis IDs plus exact current probabilities. Every decision portfolio binds this changing digest; it must change when an observation changes the posterior.

After an authorized external acquisition, `realize_probe_observation` verifies that the candidate was selected from the exact replayed portfolio, validates the current posterior ledger, applies the receipt once, and returns a new posterior plus a chained ledger. A subsequent round recomputes its portfolio using the updated posterior and the same stable hypothesis-space digest. The two-observation synthetic test specifically checks this reuse and a surprising observation whose realized entropy reduction is negative.

This closes the coupling between model identity and belief state: learning which hypothesis is more likely must not invalidate a likelihood model defined over the same hypothesis set. It does not prove that the likelihood model remains calibrated after regime or distribution shift; calibration review and scenario coverage remain separate mandatory gates.

A single point estimate of probe likelihood can be dangerously optimistic. The selector accepts multiple likelihood scenarios over the same hypothesis and outcome spaces, computes scenario-weighted expected information gain, and uses the **minimum scenario-specific information gain** for eligibility and utility. A candidate that is informative only under the nominal model but uninformative under a plausible degraded/noisy scenario is rejected. The result reports `multi_scenario_assessed`; that is deliberately separate from calibration review and is not, by itself, an epistemic qualification.

The active evidence tournament now contains seven distinct methods: no additional evidence, cheapest eligible, maximum EIG, EIG per cost, seeded random, risk-adjusted worst-case EIG and maximum worst-case pairwise Chernoff information. The Chernoff comparator addresses a different objective. For each pair of positive-prior fault hypotheses it computes the discretized Chernoff information, then retains the weakest pair and the least-separable calibrated scenario. The 100-interval (s\in[0,1]) grid is frozen across candidates. This can guide selection when the objective is to separate the hardest competing modes over repeated observations, whereas EIG measures average expected entropy reduction after one observation. The classical Chernoff error-exponent interpretation requires repeated conditionally independent observations from the same likelihood regime: it is **not** a finite-sample classification guarantee or a one-shot confidence score. A 2026 paper on Bayesian active diagnosis derives a minimum-probability-of-error input-design objective using expected Chernoff distance for multi-mode stochastic systems ([DOI 10.1002/rnc.70544](https://doi.org/10.1002/rnc.70544)); Symtropy's comparator is a passive/authorized-evidence ranking adaptation, not a reproduction of that paper's controlled-input optimization.

A candidate must also carry an independent likelihood-calibration review receipt and review state. Without that receipt/state it remains visible in the rejection ledger but cannot be selected, even when its likelihood table is normalized. The candidate manifest and calibration catalogue bind the model SHA, calibration SHA, review receipt, hypothesis manifest and scenario set. The reviewer’s identity, competence, independence, signature, scope and revocation are still verified upstream; the source code checks a receipt digest and review flag, not the reviewer’s cryptographic identity or the truth of the calibration.

This is a robust lower-bound decision rule, not a proof that the declared scenarios cover reality. Scenario weights must be positive and sum to one; each scenario must normalize for every hypothesis; outcome spaces must match; and the scenario set and calibration references are frozen before confirmatory outcomes. The selector records rejected candidates, reasons, expected and worst-case gains, review state and evidence digests. It does not acquire observations, conduct inspections, operate actuators or grant authority. Synthetic contract tests establish only mathematical behavior and fail-closed gates—not physical diagnostic utility or safety.

## Set-valued behavioral diagnosis: identify what the evidence cannot distinguish

Most fault classifiers return one class even when two physical explanations produce overlapping output trajectories. The [`behavioral-set-diagnosis-v0.1.json`](behavioral-set-diagnosis-v0.1.json) protocol adds a different comparator: a hypothesis is admissible when its independently calibrated, regime-specific multivariate response envelope intersects the observed measurement-uncertainty intervals within the preregistered margin. The evaluator returns **all** admissible hypotheses.

- Two or more compatible envelopes produce `ambiguous_hypothesis_set`; no softmax/argmax tie-break is allowed.
- No compatible envelope produces `unknown_or_out_of_support`, not the nearest known fault by force.
- Unsupported regime, missing signal, stale or conflicted observation produces `not_evaluable` rather than normal.
- Every envelope binds model, regime, train/calibration manifests, coverage target, channel set, independent calibration count and review receipt. Finite-sample minima are 9 independent episodes for 90%, 19 for 95%, and 99 for 99% marginal coverage.
- A singleton is unique only within the frozen declared hypothesis catalogue. If the true physical mechanism is omitted from that catalogue, the singleton can still be wrong; global physical uniqueness is always false in this contract.

The Rust implementation is `src/behavioral_sets.rs`. Synthetic tests cover overlap, singleton, unknown, missing/stale data, unsupported regimes, calibration under-powering, digest tampering and catalogue ordering. These tests passed in the 105-test normalized crate run. No trained model or physical evaluation is present.

The scientific motivation is the move from point classification to behavioural consistency and identifiability. A recent review of dynamical-system diagnosis discusses behavioural-set methods, robust filtering and active input design; the current LiU-ICE structural model also has unknown parameters that are not uniquely determined by all supplied signals. The method therefore targets *honest identifiability*, not artificially confident root-cause labels. It does not itself guarantee the hypothesis catalogue is complete or that independent episode calibration remains exchangeable under shift.

### Behavioural-set benchmark against simpler alternatives

Compare on the same frozen model, signals and recording split against (1) argmax classifier, (2) selective classifier with abstention, (3) independent residual-threshold union and (4) joint Mahalanobis ellipsoid. Report set size, coverage, ambiguity, unknown/out-of-support, not-evaluable rate, worst-regime results and false-singleton rate. Measure event recall and false alarms at matched ambiguity/coverage; a method that simply returns every hypothesis is not useful. Research targets remain aspirational, not achieved results.
## Append-only streaming diagnosis contract

The official interface is streaming: it supplies one sample at a time and expects a boolean detection output plus five isolation values in the order `f_pic`, `f_pim`, `f_waf`, `f_iml`, `f_x`. A later sample may update the current diagnosis but may not rewrite an output already returned for a previous timestamp. The official submission implementation is required to be Python or MATLAB.

The Rust module `streaming_contract` provides the equivalent model-agnostic core contract:

- A session binds one recording identity and one frozen model SHA-256.
- Each accepted input produces exactly one output and a content-addressed receipt. The five-slot probability contract is checked at the boundary.
- Sample indices must be contiguous; timestamps must strictly increase. Invalid input is rejected before the model is invoked.
- If the stateful model errors or returns an invalid output after invocation, the session is poisoned. It cannot continue with uncertain internal state until an external reconciliation and new session.
- A receipt chain binds model identity, input digest, output payload/digest and previous receipt. Historical outputs are returned as immutable values; the API has no retrospective update method.

The receipt chain provides tamper-evident ordering, not signer authentication, trusted time, secure isolation or proof that sensor bytes are truthful. The selector and streaming wrapper are separate: the active-evidence module never executes observations or interventions.

The protocol is frozen in [streaming-contract-v0.1.json](streaming-contract-v0.1.json). The Rust core is **not** a direct LiU-ICE competition submission. A Python or MATLAB adapter still needs exact output parity on golden vectors, streaming/no-rewrite tests and dependency/license review. Adapter compliance is a separate acceptance gate, not something inferred from the Rust tests.
## Orthogonal out-of-distribution (OOD) experiments

The benchmark now distinguishes condition shift (C), fault-semantic shift (F), and their combination (C+F). This is not a cosmetic taxonomy: the system must demonstrate both known-class retention under new conditions and rejection of faults outside the training label space. The 2026 mechanical OOD survey explicitly separates these deployment requirements, while LiU-ICE provides a concrete condition-shift opportunity: the public training recordings use WLTP, evaluation may include other cycles, and ambient pressure/temperature vary between recordings for non-fault reasons.

The preregistered [shift-matrix-v0.1.json](shift-matrix-v0.1.json) contains six experiments:

1. **C-shift / driving cycle:** whole-cycle holdout; all known fault classes remain in the label space.
2. **C-shift / ambient envelope:** test whether ambient-only variation triggers false fault alarms. The ambient strata must be frozen from metadata before evaluation; if confounded with cycle or fault labels, the result is non-identifiable.
3. **F-shift / held-out fault family:** exclude every recording of one family from fitting, feature selection and calibration; map it to the unknown slot during inference. This is a *surrogate* open-set test using a known family, not proof that a novel physical mechanism has been discovered.
4. **C+F:** jointly hold out a fault family and a driving cycle. Separate C-only and F-only passes do not imply a combined-shift pass.
5. **Magnitude shift:** withhold complete recordings from predeclared fault-magnitude bands while preserving the known family identity.
6. **Sensor-chain shift:** distinguish documented sensor drift/dropout/calibration changes from synthetic corruption stress tests and from real plant faults.

The public 26-recording training set (2–7 recordings per known family) is too small to support sweeping per-family/per-cycle 95% or 99% episode-level calibration. The finite-sample split-conformal requirements remain 19 independent episodes for 95% and 99 for 99% coverage; no amount of additional 20 Hz samples from the same recording fixes that independent-unit shortage. Underpowered strata are descriptive, not qualified. Cross-engine/fleet generalization remains unsupported until separate engine/fleet identities are available.

Every run manifest binds the shift matrix digest, declared shift axis, holdout manifest digest and count of independent test recordings. A C-shift result cannot be presented as evidence of F-shift or C+F robustness.

## Event-level metrics: time-to-diagnosis, not row count

The sample-level competition score can reward a detector that raises a late alarm, or conceal repeated alerts inside one fault episode. The exposed Rust module `event_metrics` adds a separate episode-level view:

- Debounced positive samples become alarm episodes. The protocol requires three positives, merges positive observations no more than 0.15 seconds apart, and timestamps confirmation at the third positive.
- Each alarm can match at most one annotated fault event. An alarm must start at or after the true onset and confirm by both the configured latency deadline and the event end. A pre-onset alarm remains a false alarm; it cannot be relabelled as early detection.
- Reports include event recall, mean/median/p95/max confirmation latency, false alarms per observed normal hour, duplicate alarms during detected events, late alarms during missed events, short transient bursts, and event-level isolation quality.
- Normal exposure is computed only from adjacent observed intervals no larger than 0.10 seconds, less the time covered by fault events. Long missing-data gaps are excluded rather than treated as healthy operating time.
- Event onset/offset must come from a source-bound annotation or independent injection record. The benchmark page says faults begin at approximately 120 seconds; that prose is not exact ground truth and is never hard-coded as an event label.

The initial LiU-ICE configuration is a frozen protocol choice, not a result or an optimized operating point. Sensitivity analyses belong on validation data; changing thresholds after confirmatory results invalidates the run. Report uncertainty with recording-level or engine-run-level resampling, not sample-row bootstrap. The training archive and event annotations have not yet been audited, so all event targets remain aspirational.

## Statistical constraint

For the finite-sample split-conformal order statistic, the minimum number of independent calibration episodes is 9 for 90% marginal coverage, 19 for 95%, and 99 for 99%. A regime with fewer independent recordings is not calibrated to that target and must remain not-evaluated. Exchangeability of independent episode maxima is still an assumption; this library does not prove it across different cycles, fault modes or regimes.

The small 26-recording training corpus cannot substantiate 99% calibration per fault and per driving cycle. More data from independent runs/engines and an explicit instrument dictionary are required.

## Required benchmark tournament

The machine-readable protocol is frozen in [`tournament-v0.1.json`](tournament-v0.1.json). It enumerates sixteen comparators, split protocols, metric definitions, non-compensatory evidence gates and research targets. In addition to the Fault Diagnosis Toolbox analytical reference and matched state-only/control-aware baselines, it includes all six LiU-ICE solutions from the 2025 Control Engineering Practice benchmark as separate reproduction targets. It is marked `protocol_only_not_executed`; the targets are aspirations, not measured performance.

Evaluate on common frozen recordings/splits, same channel availability, equivalent model capacity and disclosed compute:

1. Official Fault Diagnosis Toolbox analytical/structural model, where its public configuration and outputs can be reproduced.
2. Calibrated physics-informed inverse/digital-twin model, only where a model, parameter map and calibration data are available; preserve non-unique admissible hypotheses.
3. State-only residual / statistical process-control baselines.
4. Command-only baseline.
5. Naive command + state concatenation.
6. Command-conditioned response model with regime and disturbance context.
7. Gated/control-aware fusion and multi-class regime discrimination.
8. Calibrated selective / unknown-fault rejection baseline.
9. CAREngine residual/QDA/LSVC.
10. RLS local models with open-set residual signatures.
11. Gaussian/KL distribution matching.
12. Healthy-residual/ANFIS with cumulative alarm confirmation.
13. CCDNN multi-view representation.
14. Dual-VAE detection/isolation with one-class SVM unknown scoring.

The six published methods and their historical results are specified separately in [published-six-solution-baselines-v0.1.json](published-six-solution-baselines-v0.1.json) and must be individually reproduced or explicitly marked unavailable. [Paper: Control Engineering Practice 164, 106427](https://doi.org/10.1016/j.conengprac.2025.106427).

The official page requires a MATLAB or Python competition interface and an input call for every sample; an earlier output cannot be changed after it is returned. The Rust streaming contract enforces this temporal shape, but a Python/MATLAB adapter and golden-vector parity gate are still missing. This Rust crate is an independent evaluator and evidence-verification component, not a direct submission.

## Industry comparison and the route beyond current practice

Commercial marine and industrial predictive-maintenance services already combine real-time asset data, anomaly detection, trend analysis, engineering expertise and maintenance workflows. Examples include [Wärtsilä Expert Insight](https://www.wartsila.com/marine/services/lifecycle-agreements/expert-insight-service), [Siemens Senseye](https://www.siemens.com/en-us/products/industrial-digitalization-services/senseye-predictive-maintenance/), and [ABB Ability Digital Powertrain Insights](https://www.abb.com/global/en/areas/motion/services/asset-health-and-monitoring/abb-ability-digital-powertrain-insights). These are capability descriptions from vendors, not independent accuracy benchmarks.

| Industry capability | Publicly described practice | Symtropy research differentiator to prove |
|---|---|---|
| Wärtsilä Expert Insight | Vessel data, AI and rule-based diagnostics combined with expert analysis; recommendations and crew/shore collaboration | Explicit command-intent / actuator-feedback / plant-response separation; competing-cause ambiguity; independently replayable event and diagnosis evidence |
| Siemens Senseye | Scalable cloud ML condition monitoring, broad asset/data integration, risk insights and maintenance prioritization | Deterministic sample-level output receipts, fail-closed unknowns, calibrated per-regime response envelopes and explicit OOD-shift claims |
| ABB Ability Digital Powertrain Insights | Real-time asset-health dashboards, anomaly detection, trend analysis and multi-site/fleet visibility | Worst-case calibrated value-of-information ranking, no-retroactive-output guarantees and independent measurement of diagnostic delay/false-alarm burden |

Do **not** claim Symtropy beats these offerings on uptime, accuracy, avoided cost or fleet performance: public product pages do not supply matched LiU-ICE scores or comparable independently audited metrics. Nor should this workstream be positioned as a replacement for OEM support. The competitive thesis is narrower and testable: make the diagnosis process itself more falsifiable, uncertainty-aware, temporally accountable and evidence-replayable, then demonstrate that these properties improve outcomes on matched datasets and testbeds.

The standards context is supportive, not a certification claim. [ISO 13379-1:2025](https://www.iso.org/standard/88027.html) provides general guidance for machine-system diagnostic approaches where state is described by operational inputs and responses. The 2026 [mechanical OOD survey](https://doi.org/10.1007/s10462-026-11722-3) distinguishes condition shift, fault-semantic shift and combined shift; the 2026 [Annual Review of fault diagnosis in dynamical systems](https://doi.org/10.1146/annurev-control-030123-015422) reviews behavioral-set views, robust filters and active input design. These establish important research directions—not measured superiority of this implementation.

### Measurable ways to exceed a leaderboard baseline

1. **Same score, earlier alarm:** raise event recall and reduce p95 confirmation latency at the same or lower false alarms per observed normal hour. Preserve the official sample score for comparability, but do not optimize it alone.
2. **Known-class retention plus unknown-fault restraint:** evaluate C-shift, F-shift and joint C+F separately. A model that labels every anomaly as a known class is not open-world capable; a model that rejects every difficult case is not useful either. Report risk-coverage and false-known rate together.
3. **Diagnose only what is identifiable:** report the admissible hypothesis set, sensitivity rank and conditioning. The benchmark explicitly says its provided signals do not uniquely determine all engine-model parameters; a single optimizer minimum cannot override that limitation.
4. **Ask for the most valuable next evidence:** compare worst-case information-gain selection with cheapest-only, EIG-only, EIG-per-cost and seeded-random choices on the identical authorized candidate set. Measure realized uncertainty reduction and cost-to-identification.
5. **Make streaming correctness measurable:** zero sample gaps, zero cardinality mismatches, zero retrospective output changes, and a replay-valid receipt chain. Report p50/p95/p99 per-sample latency and memory at a declared reference machine.

The current numeric targets remain aspirational: +0.05 official score over the best faithfully reproduced matched baseline with FAR no worse; +5 percentage points event recall at the same false-alarm budget; p95 event confirmation latency no more than 5 seconds or 20% below the best matched baseline; ≤5% false-known rate on independent unknown-fault episodes; and zero false-unique-root-cause claims when multiple hypotheses remain admissible. No result is claimed yet.
### Literature and deployment-readiness context

The nearest published reference is now decomposed into six named reproduction targets from Jung et al. (2025), rather than one generic aggregate comparator. Each implementation must be matched to the original 2024 challenge protocol before it is re-run under the current DX2026 protocol. The 2026 marine control-aware domain-adversarial paper is an adjacent direction, but its published abstract describes a thermodynamic simulation study and explicitly says that real-engine validation is absent; it is not a directly matched LiU-ICE baseline.

A 2026 survey of 26 industrial CPS/ICS anomaly-detection studies defines nine deployability criteria and reports that detection accuracy is evaluated far more consistently than other real-world deployment criteria. This supports a wider scorecard here: streaming latency and bounded memory, missing-data behaviour, OOD and calibration, failure recovery, auditability, data/asset integration, authority boundaries and maintenance workflow—not accuracy alone. [Deployability survey](https://doi.org/10.1007/s10207-026-01330-1).

## Published same-benchmark reference: six 2024 solutions (paper published 2025)

The baseline tournament now contains six named methods from Jung et al., [A fault diagnosis benchmark of technical systems with incomplete data — six solutions](https://doi.org/10.1016/j.conengprac.2025.106427), Control Engineering Practice 164 (2025), article 106427. These were presented at the IFAC Safeprocess 2024 LiU-ICE challenge. The machine-readable source is [published-six-solution-baselines-v0.1.json](published-six-solution-baselines-v0.1.json).

| Published solution | Method family | Historical mean 1−FAR | Historical TDR | Historical TIR |
|---|---|---:|---:|---:|
| 1 — CAREngine | Healthy-model residuals, adaptive bounds, EWMA, QDA detection, Nystroem/LSVC isolation | 92.2% | 87.3% | 50.1% |
| 2 — RLS/open-set residuals | Four local RLS models, coefficient residual signatures, MSO/fault-sensitivity and open-set detectors | 100.0% | 83.1% | 85.5% |
| 3 — distribution/KL matching | Gaussian window descriptors, KL similarity, historical distributions and unknown thresholds | 58.4% | 84.5% | 48.8% |
| 4 — residual/ANFIS | Healthy model, residual limits, cumulative alarm-density confirmation, four ANFIS isolators | 99.9% | 97.2% | 68.0% |
| 5 — CCDNN | Deep multi-view feature learning with canonical-correlation constraint and redundancy filtering | 30.5% | 87.4% | 37.2% |
| 6 — VAE/SVM ensemble | VAE fault detection, VAE known-fault isolation, one-class SVM unknown signal | 36.2% | 78.6% | 42.3% |

These published means show why optimizing accuracy alone is inadequate: solution 4 had the highest historical mean detection (97.2%) and 99.9% one-minus-FAR, while solution 2 had the highest historical top-ranked isolation (85.5%) but lower detection (83.1%). Solution 5's mean one-minus-FAR was only 30.5% despite an 87.4% TDR. They are reference points for reproduction, not performance claims for this codebase.

**Metric and protocol warning.** The 2025 paper's historical TIR is the fraction of detected faults where the true class ranked highest. Current DX2026 FIA is the average probability assigned to the true diagnosis over correctly detected faulty samples. TIR is not current FIA and the published values cannot be converted to the current official total score. The 2025 comparison used its own challenge data release and evaluation protocol; the current DX2026 task has a defined Python streaming interface and current official metric. Reproduce the old paper protocol separately, then evaluate each faithful method again on the same current DX2026 data, split, sample interface and official scorer as new methods.

The six baselines have separate IDs in the machine-readable tournament and separate family values in the run schema. The acceptance record must say `faithful_reproduction`, `reimplementation`, `unavailable_source_artifacts` or `not_started`; do not describe an approximation as a reproduction. Recovery is required for each method's feature selection, channel use, preprocessing, model architecture, threshold/window settings, training/calibration split, online update order and compute. The paper's own observations are useful falsification leads: dynamic end-of-cycle residual false alarms, residual sensitivity overlap caused by feedback, poor generalization to held-out fault magnitudes, sensitivity to environmental ambient conditions, and VAE detector-to-isolator error propagation.

The current DX2026 comparison uses the official sample-wise FAR/TDR/FIA/total score as its primary cross-team metric, plus event-level latency/false alarms, magnitude/cycle robustness, unknown-fault false-known and abstention, calibration, resource usage and streaming correctness. Only matched, current-protocol results can establish a SOTA comparison.

## Current DX2026 official reference: grey-box physics-informed LSTM

The current-year comparator is [Rogowski and Sztyber-Betley, *Physics-Informed ML Diagnostic Engine: DXC’26 LiU-ICE Track Solution*](https://doi.org/10.4230/OASIcs.DX.2026.15), published in the DX 2026 proceedings. Table 7 reports the official current metrics: **TDR 92.26%, 1−FAR 92.14% (FAR 7.86%), FIA 79.62%, official score 88.01%**. The 12 per-condition rows are transcribed in [published-dx2026-graybox-baseline-v0.1.json](published-dx2026-graybox-baseline-v0.1.json). This is a literature reference, not a Symtropy reproduction or result.

The method is a physics-informed grey-box recurrent network with physical hidden states and derivative/direction features, four virtual sensors built around MSO-derived residual structure, a diagnosis matrix and fixed alarm thresholds, plus adaptive covariate-shift compensation. Its published limitations are unusually useful for falsification:

- A **0.75-second / 15-sample** high-inertia debounce reduces false alarms but creates detection delay.
- A **90-second / 1800-sample warm-up** adapts residual thresholds to operating conditions, but the paper explicitly warns that a fault present from startup can be assimilated into the healthy baseline.
- Under FTP75 Standard, the `f_pim -10%` cell had only 21.12% TDR; under WLTP, `f_pic +8%` had FIA 0.00% and 1−FAR 34.61%, despite a high detection rate.
- Air-mass-flow sensor faults were a stated weakness.

These are better than aggregate SOTA labels: they are exact test cells where a new method can be challenged. A fair comparison must recover the same split, preprocessing, sensor set, warm-up, debounce, virtual-sensor implementation and scorer. Do not optimize the new method on the hidden test outcomes.

### What would actually beat this reference?

The preregistered targets are: at least **+2.00 absolute official-score points over 88.01%** on a matched protocol, while keeping FAR at or below 7.86%; p95 event confirmation latency at or below 0.50 seconds at the same false-alarm budget; and zero startup-fault episodes absorbed into the healthy baseline. These are ambitions, not measured results. All 12 per-condition cells must be reported so that an aggregate score cannot conceal failures like the published `f_pim -10%` and `f_pic +8%` cases.

## ## Published DX2026 boundary red-team suite

The publication's overall 88.01% score hides condition-specific failures. The new [dx2026-boundary-falsification-v0.1.json](dx2026-boundary-falsification-v0.1.json) turns the three most informative published cells into source-bound *synthetic envelope counterexamples*:

| Published cell | Reported result | Required Symtropy disposition to test |
|---|---:|---|
| FTP75 Standard, `f_pim -10%` | TDR 21.12%, score 45.64% | Overlapping normal/fault envelopes must return `ambiguous_hypothesis_set` and retain the true fault; no forced singleton |
| FTP75 Highway, `f_pim -10%` | TDR 98.79%, score 97.05% | A separate highway-regime catalogue may return a singleton, but that confidence must not transfer to FTP75 Standard |
| WLTP, `f_pic +8%` | TDR 100%, FIA 0%, score 44.87% | Detection may fire, but when no known envelope matches, isolation must return `unknown_or_out_of_support` rather than inventing a known cause |

The tests in `src/behavioral_sets.rs` exercise these three dispositions with synthetic response envelopes. They validate the *decision semantics* only; no published sample waveform or model has been reproduced. The official metrics and reported cell values are copied into the boundary fixture from Table 7, and the fixture remains explicitly `protocol_only_not_executed` until a matched raw-data experiment is run.

This makes the benchmark more discriminating than a simple average score: the method must know when to preserve ambiguity, when a different operating regime genuinely separates hypotheses, and when anomaly detection must be separated from fault isolation.

Paradigm shift: a frozen live baseline with a quarantined shadow candidate

The 90-second adaptation warm-up exposes a structural conflict: a system needs to adapt to changing conditions, but it must not learn a fault as normal. The new [startup-adaptation-guard-v0.1.json](startup-adaptation-guard-v0.1.json) and Rust module `src/startup_adaptation.rs` enforce a two-track architecture:

1. **Live reference:** keep the last independently qualified baseline and model digest frozen. It continues producing diagnosis from sample zero; shadow collection is not a reason to suppress current detection.
2. **Shadow candidate:** collect new samples in an append-only, content-addressed lane. The live inference path never consumes these candidate statistics.
3. **Quarantine on doubt:** any alarm, unknown/ambiguous hypothesis set, out-of-support regime, stale/missing sensor, invalid command timing, residual excursion, sample gap or unverified dependency-health state quarantines the whole candidate.
4. **Independent review:** after the predeclared sample count, the candidate may produce a review package bound to the old baseline, model and receipt chain. At least two distinct issuers from independently verified dependency domains must attest to the exact candidate digest. The digest and caller-supplied review flags still require upstream signature, authority, competence, expiry and revocation verification.
5. **Separate activation authority:** this module has no baseline-commit or activation function. Replacing the live reference requires a separate owner decision, rollback evidence and full prior-regime regression tests.

The default stress protocol uses 1,800 samples (90 seconds at 20 Hz) but unlike a blind warm-up, the old detector remains live for the entire period. This is a shadow evidence-acquisition window, not a diagnosis blackout. The v2 shadow receipt digest now covers baseline/model/regime identity, sample and input receipt, residual, regime/signal/command/timing decisions, dependency-health state and digest, all known/unknown/ambiguous alarms, and the previous receipt link. `validate_shadow_receipt_chain` replays the complete content and ordering; field changes are reported as digest failures before semantic admission checks. The digest still does not authenticate upstream issuers or prove physical health. The current code and synthetic tests are authored but have not been compiled or physically validated.
## Report beyond the leaderboard

Every run should be serialized against [`benchmark-run-manifest.schema.json`](benchmark-run-manifest.schema.json). It binds dataset bytes and recording membership, split membership, comparator configuration, model artifact, evaluator revision, dependency lock, protocol/seed manifest, metric outputs and the independent-unit uncertainty method. A score without these bindings is not a qualified result.

Always report FAR, TDR, FIA and official total score, but do not stop there. Add event recall, false alarms per observed normal hour, detection latency, per-fault/per-cycle performance, unknown-fault false-known rate, ambiguity/abstention, risk-coverage, interval coverage and sharpness, worst-regime performance, model size, inference latency and resource use. Group uncertainty by independent recording/engine campaign, never by rows.

A test bench with one engine supports within-engine drive-cycle/magnitude generalization. It does not establish transfer to another engine, vessel or fleet.

## Claim boundaries and next steps

- The LiU-ICE training ZIP has not been retrieved or audited here. The Marine Engine Fault ZIP was downloaded by GitHub Actions, its publisher-reported size and MD5 were matched, and its 18 CSV entries were safely extracted. The exact file audit and machine-readable receipt are still pending. No trained model score, competition score or external-dataset validation score is claimed.
- The LiU-ICE public landing page does not establish an archive redistribution licence. The Marine Engine Fault Zenodo record lists CC BY 4.0, but archive notices and bytes remain unaudited. Do not redistribute either archive until the applicable source terms are checked.
- Inspect the archive, variable dictionary, sampling/time semantics, data completeness and fault-label rules before freezing feature and split manifests.
- Compare published baselines only under a recovered, matched protocol. Mark unreproduced paper scores as literature references.
- Physical validation requires permissioned synchronized controller intent, independently measured actuator feedback and physical plant response from an engine testbed, with an independently reviewed I/O map.

This benchmark lane does not write controllers, actuate equipment, authorize maintenance, establish safe-life limits, or clear operating restrictions.


### Prediction-time causality control

The response evaluator records three distinct times: when the command became available to the diagnostic process, when the frozen model issued its expected response, and when the independent physical response was observed. It rejects a command that arrived after prediction, a response that predates or coincides with prediction, a response beyond the declared horizon, and a command that was already stale at prediction time. This is stricter than merely checking that the command precedes the measurement; it blocks an after-the-fact command from being used to explain an earlier prediction.

The inverse-model comparator must report the cardinality of the admissible solution set and a false-unique-root-cause rate. A low residual at one optimizer-selected parameter vector is not enough: where multiple fault/degradation hypotheses fit inside the calibrated acceptance region, the diagnostic output must remain an ambiguity set. This comparator is conditional on obtaining a calibrated model and validated parameter map; it is not an assumed LiU-ICE dataset capability.

### Identifiability protocol for inverse-model comparisons

For an inverse/digital-twin comparator, use a dimensionless, uncertainty-whitened sensitivity Jacobian over the declared configuration and operating regime. Report its singular spectrum, numerical rank, rank tolerance, condition number, null-space dimension, and the size/components of the admissible hypothesis set. Explore declared parameter bounds with multi-start, profile or interval methods; calibrate the residual acceptance region independently and freeze it before confirmatory testing.

A unique diagnosis is eligible only when one admissible component remains and the local sensitivity matrix is full-rank and sufficiently conditioned under the preregistered rule. Multiple admissible components or deficient/ill-conditioned sensitivity means an explicit ambiguity class. Missing model calibration, parameter map, uncertainty region or supported regime means NOT_EVALUATED. Full local rank is not proof of global uniqueness.
