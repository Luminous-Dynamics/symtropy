# AER-01 virtual-sensing joint health-state oracle (Tier 0)

This crate is a deliberately small, exact synthetic benchmark for the joint structural-health / sensor-health problem. It is a child implementation for #4176 and composes with placement #4259, structural observability #4244 and measurement-operator contract #4215 / #4236.

**Claim ceiling:** synthetic joint-state hypotheses, witness gating, raw-data preservation and abstention only. It is not an SHM detector, NDE system, aircraft model, POD assessment, airworthiness analysis or flight-safety function.

## Paradigm shift: reconstruction is not diagnosis

A measured anomaly can arise from a structural change, a sensor fault, an environmental/operational change, an invalid measurement chain, or multiple causes. A residual on one channel does not establish which cause occurred.

This oracle applies four hard rules:

1. Evaluate structure and sensor state jointly: HH, HF, DH and DF (healthy/damaged structure × healthy/faulty sensors).
2. A single-channel residual is only a candidate explanation. It does not prove sensor failure. A proposed sensor fault requires an independent, channel-matched health witness with an explicit evidence digest.
3. Multiple plausible explanations, unqualified operators, expired calibration, insufficient observations and out-of-model responses abstain.
4. Raw observations are immutable. An optional model-imputed value is emitted in a separate field with status CONDITIONAL_MODEL_ESTIMATE_NOT_MEASUREMENT. It is never written back as a measurement and is not independent evidence.

This is intentionally more conservative than “reconstruct first, classify later.” A localized defect or unmodeled boundary condition can generate the same one-channel residual pattern as a faulty sensor. Without independent sensor-health evidence, the correct outcome is uncertainty, not automatic reconstruction.

## Frozen synthetic corpus

Three registered channels, two structural templates, a normalized static response and a one-channel residual fault model. The eight cases cover:
- HH: healthy structure / healthy sensors;
- HF: healthy structure / faulty sensor, with independent synthetic built-in-self-test witness;
- DH: damaged structure / healthy sensors;
- DF: damaged structure / faulty sensor, with independent synthetic witness;
- ambiguous structural-damage versus sensor-fault explanations;
- out-of-model environmental/unknown shift;
- insufficient observations;
- a single-channel localized anomaly with no independent sensor-fault witness.

The HF and DF witness digests are synthetic fixture values. They are not records from actual hardware. The operator digests, channel health, structural templates, thresholds and signal values are also synthetic.

## Output contract

Every case report preserves:
- original observed_values exactly as supplied;
- candidate structural state and disposition, or an explicit abstention;
- faulty_sensor_ids only when the channel-matched independent witness is present;
- model_imputed_values separately from observed_values;
- reconstructed_sensor_ids and the literal status CONDITIONAL_MODEL_ESTIMATE_NOT_MEASUREMENT;
- post-reconstruction structural state, which is required to remain the pre-reconstruction structural hypothesis;
- damage_preserved as a synthetic invariant, not physical damage-detection evidence.

For a case with no sensor fault, model_imputed_values is null. For an abstained case, no model imputation is emitted. Unwitnessed single-channel residuals return ABSTAIN_SENSOR_FAULT_UNCONFIRMED. If the healthy/damaged template hypotheses are both consistent, the result is ABSTAIN_AMBIGUOUS_HYPOTHESES.

## Implementation limits

This v0.1 model is deliberately tiny:
- three channels, two fixed structural templates and a static normalized response;
- a single arbitrary channel residual as the only injected sensor-fault pattern;
- a deterministic absolute tolerance, not a learned or calibrated probability;
- synthetic built-in self-test witnesses, not a real diagnostic architecture;
- no temporal response, dynamic transfer functions, covariance estimation, uncertainty propagation, environmental causal model, damage morphology, fatigue, fracture, POD or field calibration.

It does not reproduce either current research frontier:
- Di Fiore et al., *Real-time sensing digital twin for aerospace structural health monitoring* (8 June 2026), combines physics-informed sensing twin, joint sensor/structure classifier, healthy-manifold reconstruction and probabilistic sensor reliability; its demonstration is a numerical composite panel with simulated fault/damage conditions. https://doi.org/10.1007/s00158-026-04324-8
- Chen & Liu, *Causal Bayesian optimization for sensor placement and deconfounding in structural health monitoring* (1 October 2026), uses a structural causal model, do-calculus, counterfactual uncertainty and multi-objective sensor placement, with numerical, experimental and full-scale bridge validation. https://doi.org/10.1016/j.ymssp.2026.114895

The present contribution is not novelty in sensor-fault detection or reconstruction. It is a small, inspectable fail-closed contract that tests independent fault-witness gating, ambiguity abstention, immutability of observations and explicit separation of imputation from measurement.

## Reproducible execution

~~~sh
cargo fmt --manifest-path tools/civil-aero-benchmark/virtual-sensing-health-state/Cargo.toml -- --check
cargo test --locked --manifest-path tools/civil-aero-benchmark/virtual-sensing-health-state/Cargo.toml
cargo run --locked --quiet --manifest-path tools/civil-aero-benchmark/virtual-sensing-health-state/Cargo.toml > /tmp/virtual-sensing-health-state.json
python3 tools/civil-aero-benchmark/virtual-sensing-health-state/verify_independent.py --rust-output /tmp/virtual-sensing-health-state.json
~~~

The independent Python verifier uses only the standard library and does not import the Rust implementation. It recomputes every case and compares the complete JSON report, then runs hostile controls including wrong-channel witnesses, missing/expired channel admission, ambiguous hypotheses, out-of-model signals, insufficient observations and post-reconstruction damage preservation. The exact-head GitHub Actions workflow asserts the PR subject SHA and retains source, fixture, toolchain/environment, output and SHA-256 receipts.


## Public benchmark path: Z24 environmental variability and damage campaign

The external reference register data/external-benchmark-register-v0.1.json records a realistic next comparison target without pretending the files have been acquired or admitted.

KU Leuven's official Z24 portal describes two distinct acquisition programmes:
- a year-long continuous monitoring campaign, with 16 acceleration measurements, 48 environmental measurements, detailed temperature sensors, hourly scans/averages, and reported drift, sensor loss and accelerometer failure;
- a short-term progressive-damage campaign, with 291 measured DOFs collected across nine setups and five reference channels, two shakers, 3–30 Hz force content, and 65,536 samples at 100 Hz with a 30 Hz anti-alias cutoff.

The official portal states that data are available for non-commercial research only, require registration/acknowledgement/citation, and may not be transferred to third parties. Luminous must obtain written permission for commercial use or transfer before any data retrieval, integration or publication. The dataset has not been downloaded, hashed, terms-reviewed or admitted by this PR. Source: https://bwk.kuleuven.be/bwm/z24

A second, recent comparison target is Pham Hong et al., *SpectralTCN* (PLOS ONE, 21 September 2026). The paper reports 92.2% accuracy on its Z24 protocol, against 10 baseline models in stratified five-fold cross-validation. Its reported preprocessing uses a fourth-order zero-phase 0.5–40 Hz Butterworth bandpass, per-channel z-score normalization and 1024-sample Hann windows. Those details become a reproduction target, not a claim of aircraft-level SOTA. Source: https://doi.org/10.1371/journal.pone.0358224

There is a channel-inventory discrepancy to resolve before reproduction: the SpectralTCN paper reports 27 accelerometer channels, while KU Leuven's official long-term overview describes 16 acceleration measurements. These may refer to different acquisition subsets or derived channel configurations. Do not silently assume the same channel map; reconcile the registered raw files, configuration and preprocessing first.

### Two evaluation tracks—do not mix their scores

**Track A: paper replication.** After permission and exact-byte admission, reproduce the published 0.5–40 Hz preprocessing, z-score normalization, windows, splits, baselines and metrics. This is needed for a fair comparison with the 92.2% published result.

**Track B: fault- and domain-preserving evaluation.** Separately hold out contiguous time/environment blocks, sensor installations, damage regimes and entire campaign segments. Compare preprocessing variants that preserve low-frequency bias/drift, gain changes and channel health rather than removing those signatures before sensor-fault diagnosis. Measure HH/HF/DH/DF, fault isolation, false-clear/false-damage, out-of-domain abstention, uncertainty, decision preservation and total cost—not just damage-class accuracy.

A high-pass filter and per-channel z-score can be legitimate for a particular damage-classification task, but they can also erase information needed to detect drift or gain faults. Keep the paper-reproduction track unchanged, then add a separately preregistered fault-preserving track. Do not claim it outperforms SpectralTCN unless both methods use matched, disclosed data splits and costs.

Z24 is a civil-infrastructure transfer benchmark, not an aerospace validation article. The My Thuan comparison in the register is an FEM-derived simulation dataset, not physical bridge measurements. Neither replaces an admitted composite aerospace article, a validated damage morphology or an NDE/POD study.



## MuViS-C: faithful corruption protocol, synthetic adapter only

The additional executable \`src/bin/muvis_c_corruption.rs\` implements the ten single-channel corruption equations and three principal robustness measures from Brandt et al., *Reliable Virtual Sensing: A Multi-Domain Benchmark for Robustness Under Sensor Failures* (arXiv:2609.18396, 16 September 2026). The original benchmark evaluates nine datasets across six domains and six model families; it finds model rankings vary by dataset/failure and no single architecture is robust across all settings. It also probes input-sensor dropout, adversarial training and self-supervised fault pretraining. The protocol is the comparator; this tiny synthetic fixture does not reproduce those datasets, training runs or rankings.

**Main protocol copied into the adapter:** one corrupted channel at a time; 20 severities from 0.05 to 1.00; \(k=3\), \(p_{\max}=0.3\), \(\alpha=0.4\); 200 fixed bootstrap resamples per RMSE. The ten modes are bias, additive Gaussian noise, scaling, time-varying scaling, linear drift, quadratic drift, outliers, variable trimming/soft clipping, constant trimming/hard clipping, and stuck-at hard fault. \(mPC\) averages RMSE across mode × channel × severity; \(rPC=mPC/RMSE_{clean}\); \(s_{cross}\) is the earliest severity at which any mode/channel RMSE reaches the naïve training-mean predictor. The implementation retains the full 600-entry severity grid, not just aggregate metrics. Source: https://arxiv.org/abs/2609.18396. The upstream artifact and exact source commit were not admitted into this repository; this is a locally implemented protocol adapter, not a claim of bit-for-bit upstream code parity.

**Frozen synthetic fixture:** 6 windows × 8 time steps × 3 channels; per-channel population z-score statistics and raw pre-normalized values are retained; stochastic normal and uniform inputs are precomputed from declared seeds and verified from those seeds; bootstrap indices are also regenerated and checked from the declared seed/method. The linear virtual sensor is fixed and all targets are derived from a declared synthetic equation. This is a deterministic conformance fixture, not a trained model evaluation.

Frozen adapter answers:
- clean RMSE ≈ 0.0931334; naïve mean-predictor RMSE ≈ 0.3198549;
- \(mPC\) ≈ 0.2056479; \(rPC\) ≈ 2.2081014; \(nPC\) ≈ 0.6429413;
- \(s_{cross}=0.05\);
- worst individual setting: hard fault on wing-root strain proxy at severity 1.0, RMSE ≈ 1.3473686;
- 10 × 3 × 20 = 600 corruption-grid entries.

These scores say only that the frozen linear toy model degrades according to the declared corruption protocol. They do not compare with any paper model and are not a statement of AER-01 performance.

**Important boundary:** the published MuViS-C main grid excludes severity \(s=0\), although \(D_0\) is identity for most corruptions and zero/stuck-at for hard fault; the paper also reports a separate hard-fault-at-zero practical scenario. Our severity-zero unit test checks the corruption operator's boundary, but the \(mPC/rPC/s_{cross}\) aggregate uses only \(s=0.05,\ldots,1.00\). The aircraft-specific HH/HF/DH/DF joint-state corpus is a separate evaluation and must not be folded into the atomic single-channel MuViS-C score.

### What the reference benchmark says about beating SOTA

The paper's reported best model varies by failure family: F2F-led TST variants lead some drift/bias cases; TST-ISensD leads trimming; TST-PGD leads scaling; XGBoost leads noise and outliers. The paper explicitly warns that single-domain model rankings are incomplete. Therefore the target is not “beat one architecture with one aggregate score.” The AER programme should report:
1. a faithful single-channel atomic-corruption leaderboard (mPC, rPC, \(s_{cross}\), per-mode curves, clean RMSE);
2. a separate multi-channel/common-cause robustness track (bus, power, reference-clock, thermal, data-acquisition and shared-calibration groups);
3. joint structural/environment/sensor-state diagnosis (HH/HF/DH/DF and unknown conditions);
4. damage-decision preservation, false-clear risk and abstention under missingness/model mismatch;
5. total instrumentation + calibration + maintenance + compute + independent audit cost.

For fair architecture comparison, reproduce the paper's own baseline families (XGBoost, CatBoost, xLSTM-M, Transformer/TST, PatchTSMixer, ModernTCN) on the admitted dataset/split. Then add physics-informed observer/ROM baselines, wavelet/TCN models, graph/multisensor fusion, F2F-style pretraining, input sensor dropout and adversarial training. Evaluate every model on identical data splits and total budgets. Averages are insufficient: keep per-mode and per-channel severity curves plus earliest baseline crossing.



## Environment/damage confounding: causal identifiability contract

The additional Rust executable \`src/bin/environment_damage_confounding.rs\` separates structural and environmental explanations before assigning a structural state. A three-channel synthetic structural signature is combined with independently measured mean-temperature and span-gradient effects:

- nominal environment contributes no offset;
- uniform heating adds a common-mode response;
- span gradient adds a localized wing response that is intentionally identical to the synthetic damage signature;
- combined heating and gradient adds both effects.

This creates a deliberate identifiability test. A measured vector of [1, 0, 0] is consistent with either healthy structure + thermal gradient or damaged structure + nominal environment. With no independent environmental evidence, both hypotheses are returned and the oracle abstains. With an admitted, calibrated, time-aligned environmental witness, it may resolve the pair. The environmental witness is checked for both measurement and topology-evidence digests, uncertainty, typed failure domains (power, DAQ, clock, calibration chain and wiring harness), independence from the structural-sensor topology, independence among environmental probes/witnesses, acquisition window, spatial support and redundant gradient-probe agreement. A distinct sensor name or manually different common-cause label is not enough when power, DAQ, clock or wiring resources overlap. Expired, contradictory, out-of-profile or unaligned witnesses cannot authorize a damage call.

The 17-case synthetic corpus includes healthy/nominal, healthy/heated, damaged/nominal, damage plus heating, damage plus combined environment, the damage/gradient collision both with and without a witness, conflicting redundant gradient probes, expired calibration, time-window mismatch, excessive uncertainty, out-of-profile temperature, structural/environment model mismatch, incomplete environmental observations, two separately packaged witness sets sharing a typed clock domain, and an environmental probe sharing a DAQ domain with a structural sensor.

This is an identifiability and abstention contract, not a causal effect estimator, temperature compensation model or physical SHM result. Its environment-response coefficients and all probes are synthetic. A physical version must derive the causal graph and transfer functions from validated structural/thermal evidence, test sensor placement and common-cause independence, account for spatially varying temperature/strain fields, and validate across held-out specimens and excitation regimes.

### Why this matters against current research

The October 2026 causal MO-CBO paper by Chen & Liu explicitly models environment, structural properties, damage, sensor configuration and measured response in a structural causal model, with do-calculus and counterfactual uncertainty; it also reports numerical, experimental and full-scale bridge evaluation. The June 2026 aerospace sensing-digital-twin paper separately models structural damage and sensor faults, with ROM-based signal reconstruction and sensor reliability. These are stronger systems than this small oracle; we do not claim to reproduce or beat them. The contribution of this child is a small executable contract that forces the environment/damage ambiguity to remain visible and makes independent environmental evidence a precondition for disambiguation. References: https://doi.org/10.1016/j.ymssp.2026.114895 and https://doi.org/10.1007/s00158-026-04324-8.



## Aerospace dataset ladder: stop treating civil-bridge data as the only route

The external register now identifies five aerospace/composite sources that are more directly relevant than Z24. None has been acquired or admitted by this PR; all remain reference-only pending exact bytes, data rights, channel maps, units, and provenance.

1. **Controlled EOV before damage:** the Airbus/USES2 2026 Lamb-wave panel record (Zenodo 19209079) reports an undamaged composite panel in a climatic chamber from 20–70 °C, constant humidity and five-minute interrogations, with raw guided-wave files and climate logs. This is an environment-only benchmark: measure thermal response and calibrate the environmental witness before any damage classifier sees it.
2. **Damage progression with independent imaging:** NASA PCoE's CFRP composite dataset reports 16 PZT Lamb-wave sensors, multiple triaxial strain gauges, three laminate layups, tension–tension fatigue to failure and periodic X-rays as damage ground truth. It is suitable for damage progression and sensor-fusion evaluation, with X-ray inspection timing treated as interval-censored truth rather than continuous labels.
3. **Full-scale aeronautical article + sensor integrity:** the SARISTU outer-wing demonstrator paper reports a roughly 4.5 m composite outer-wing structure with 160 secondarily bonded PZT transducers. Its SHM system measures electromechanical impedance at each transducer to assess reliability and bond strength, in addition to active guided-wave screening; four stages span before loading, before fatigue, before impact and after impact. This is an unusually useful joint structural/sensing reference.
4. **Long-term environmental drift transfer:** the University of Utah paper describes ~4.5 years and ~6.4 million guided-wave measurements, eight propagation paths per measurement (two actuators × four receivers), an approximately 170-second interval, thirteen introduced damage types, daily temperature variation from 260.95 K to 325.65 K, rain/snow, sensor drift and installation shifts. A separate SmartDATA Lab landing page reports over 7.6 million signals, a 30-second interval and 14 piezoelectric sensors, while the repository notes that its original dataset link is no longer available. These metadata are not reconciled; the register keeps the paper values and discrepancy separate until exact files and acquisition metadata are admitted. This is civil/infrastructure, not aircraft evidence.
5. **Damage morphology and ultrasound verification:** the Open Guided Waves omega-stringer CFRP dataset reports intact, local-debond and large-debond cases with full guided-wave fields and conventional ultrasound verification, under constant-temperature conditions.

The intended experimental sequence is therefore not one monolithic leaderboard. Use environment-only data to assess environmental response and probe integrity; use fatigue/X-ray and guided-wave/ultrasound data for damage localization and progression; use full-scale outer-wing data to examine sensor/bond health alongside damage; then use long-term outdoor data as a cross-domain EOV/drift challenge. Do not transfer one dataset's labels, unit scales or calibration to another without an admitted mapping.

### Admission and split rules

The register records each source URL, reported metadata, dataset role, claim ceiling, rights uncertainty and admission state. Before bytes are admitted:
- record the data owner, source-version/record identifier, licence and intended commercial/non-commercial use;
- obtain any required permission in writing, especially where the catalogue only requests acknowledgement or where licence terms are unstated;
- download the original archive and record byte count and SHA-256 before unpacking; retain the archive and a complete extracted-file inventory;
- validate channel IDs, physical units, sensor locations, time bases, acquisition windows, calibration, damage state, environment and ground-truth timing;
- freeze the partition by specimen, layup/configuration, damage campaign, acquisition day, temperature regime and sensor installation—not random overlapping windows;
- treat every bandpass, normalization, temperature compensation, resampling and model reconstruction as a versioned measurement operator; preserve raw data unchanged;
- keep X-ray/ultrasound/NDE truth separate from model predictions. Report uncertainty in damage timing when ground truth is periodic.

This ladder creates a defensible way to compare the industry baselines on comparable real signals before attempting Wren 2 / Wayfarer 4 candidate validation. It does not turn a bridge, coupon or demonstrator into an aircraft certification article.



## Benchmark partition integrity: specimen, environment, installation and time

The executable \`src/bin/benchmark_split_contract.rs\` and independent verifier \`verify_benchmark_split_contract.py\` implement a split-integrity contract. The fixture is synthetic metadata only. Five distinct evaluation tracks are validated:

- **Specimen holdout:** validation and test specimen IDs cannot appear in training.
- **Thermal extrapolation:** train, validation and test occupy disjoint thermal-regime groups.
- **Installation shift:** each split uses a distinct sensor installation/re-bond generation.
- **Compound article/campaign holdout:** specimen, campaign and configuration/layup must all be disjoint across train, validation and test. A leak in any one axis rejects the entire track.
- **Forward time:** the physical article, campaign, configuration and installation remain fixed while acquisition time advances; the purge gap must be at least the feature-window length plus preprocessing/filter memory.

All tracks reject duplicate sample IDs, source-record IDs and acquisition-window IDs. Negative controls deliberately introduce a cross-split specimen leak, a compound configuration leak, a duplicate acquisition window and an insufficient temporal purge. The Rust result is independently recomputed in Python and retained with the exact subject commit and source digests.

This is a partition-integrity check, not a model-performance benchmark. Real datasets must first be admitted and mapped to these fields. Use specimen/campaign-level uncertainty intervals rather than treating adjacent signal windows as independent experimental replicates. Do not report a random-window split as evidence of specimen, campaign, temperature or installation transfer.

## SOTA comparison: what is already published, and where to compete

Three relevant 2026 comparators materially raise the bar:

- **MuViS-C (September 2026)** benchmarks ten sensor-failure corruptions over nine datasets and six domains, across six model families. The authors report that all models degrade under at least one corruption setting, gradient-boosted trees are robust baselines, and model rankings change across datasets. Our local MuViS-C adapter currently checks its equations on a tiny synthetic fixed linear predictor; it does not reproduce the nine datasets, model families, training protocols or leaderboard. Compare to the upstream benchmark only after exact upstream code/data/versions and split semantics are admitted.
- **Causal Bayesian optimization for SHM sensor placement (1 October 2026)** uses a structural causal model for environment, structural properties, damage, sensor placement and measured responses; it reports numerical, experimental and full-scale bridge evaluations and counterfactual sensor-layout reasoning. That work is a substantive comparator. Our environment/damage oracle is a transparent identifiability/abstention contract, not a competing causal optimizer or a reproduction of its results.
- **Real-time sensing digital twin for aerospace SHM (8 June 2026)** combines reduced-order structural models, optimal sensor placement, sensor-fault mechanisms, physics-aware classification, signal correction and probabilistic sensor reliability on a progressively damaged composite wing panel. This is close to the target application. Our distinction is the explicit refusal to promote model imputation to measurement, independent-witness preconditions, common-cause checks and preservation of damage hypotheses through correction. We still need matched physical comparisons before claiming any advantage.

References: MuViS-C, https://arxiv.org/abs/2609.18396; causal Bayesian optimisation, https://doi.org/10.1016/j.ymssp.2026.114895; aerospace sensing digital twin, https://doi.org/10.1007/s00158-026-04324-8.

### Fair comparison protocol

Use identical raw source bytes, admission records, channel mappings, feature windows, train/validation/test partitions, target definitions and compute budgets for every baseline. Compare at least:
1. MuViS-C model families (gradient-boosted trees, convolutional, recurrent, attention, and MLP-mixing sequence models) under the same corruption grid;
2. guided-wave signal-processing baselines (baseline signal stretch, optimal baseline selection, PCA/SVD/ICA, wavelet/feature pipelines and delay-and-sum imaging where appropriate);
3. structural model/ROM observers and physics-informed hybrid predictors;
4. causal EOV deconfounding and Pareto sensor-placement methods;
5. the aerospace sensing-digital-twin architecture with separate damage, sensor-fault and reliability outputs.

No result is comparable if its data split, feature/normalization pipeline, target quantity, or compute budget differs without explicit stratification. Report every model by dataset, specimen, layup, temperature range, installation generation and fault family; do not collapse these into one headline number.

### Metrics that matter

| Evaluation axis | Required report |
|---|---|
| Virtual sensing | RMSE/MAE and normalized error on clean data; \(mPC\), \(rPC\), \(nPC\), \(s_{cross}\), full mode × channel × severity curves, worst-case degradation |
| Damage detection | POD versus damage size/severity, missed-detection rate, false alarms per interrogation or operating hour, localization error, sizing error and confidence intervals |
| Sensor health | per-fault precision/recall, detection latency, false isolation, sensor/bond-health calibration, common-cause fault detection |
| EOV robustness | false alarm rate by temperature/humidity/load regime, residual after compensation, unseen-regime error and OOD/abstention coverage |
| Decision quality | damage-clearance false-negative count, damage preservation after sensor correction, selective risk versus coverage, uncertainty calibration |
| Instrumentation economics | sensor count, mass, power, cabling, DAQ, installation, calibration, replacement, maintenance and audit cost; Pareto frontier rather than information score alone |

Bootstrap at the specimen or independent campaign level. A million adjacent windows from one panel are not a million independent specimens. For periodically inspected fatigue data, report interval-censored damage onset/size uncertainty rather than assigning a fictitious exact timestamp between X-rays.

### Research targets (aspirational, not achieved)

- 100% exact replay and digest verification on frozen synthetic and admitted real-data tasks.
- Zero split leakage under all declared holdout axes and mutation/negative controls.
- Zero damage-state downgrade in the declared joint sensor/structure synthetic suite, including simultaneous damage plus sensor faults; this is a software invariant, not a physical probability-of-detection claim.
- Publish false alarms, missed detections, localization/sizing error and uncertainty coverage with specimen-level confidence intervals across every held-out domain.
- Beat the strongest matched baseline on a preregistered Pareto frontier: equal or better detection and localization at lower total lifecycle instrumentation cost, while not worsening false alarms or missed-detection risk.
- Require an independent physical campaign and external reproduction before claiming a field/SOTA advantage.

The first four targets are contract-level goals; the last two are research objectives. None is yet demonstrated on Wren 2 or Wayfarer 4.


## Advancement ladder

**Tier 0 — this crate:** frozen synthetic HH/HF/DH/DF + abstention; independent fault witness; immutable raw data; independent Rust/Python replay.

**Tier 1 — temporal synthetic:** bias, drift, gain, saturation/clipping, dropout, stuck-at, detachment, timing skew, noise and correlated/common-cause failures. Preserve complex response, phase, transient peaks and spectral features. Add unknown/overlapping faults and missingness during the event of interest.

**Tier 2 — model mismatch and environment:** held-out stiffness, mass, boundary conditions, temperature, humidity, excitation and geometry; environmental-only cases; multiple damage morphologies; counterfactual tests with hidden confounding. Hold out complete specimens/configurations/excitation regimes, not random windows.

**Tier 3 — matched literature baselines:** nearest-neighbour interpolation; modal interpolation; POD/ROM projection; observers/Kalman methods; physics-informed ROM updating; sensing-digital-twin-style joint classification/reliability; causal MO-CBO deconfounding where reproducible. Same data, hidden truth, fault scenarios and total evaluation budget.

**Tier 4 — physical article:** selected representative article, controlled sensor injection and independent NDE/ground-truth damage records; calibrated environmental/operational campaign; blinded operators; independent specimen/configuration holdouts. No POD statement without an appropriate inspection reliability study.

## Metrics that matter

Do not optimize aggregate reconstruction RMSE alone. Report:
- joint HH/HF/DH/DF confusion, per-fault isolation precision/recall and unknown-fault abstention;
- false-clear and false-damage decisions, decision preservation and downstream structural decision regret;
- complex amplitude/phase error, resonance/damping/mode identity preservation and local peak-strain/load preservation;
- uncertainty calibration/coverage and out-of-domain abstention on grouped holdouts;
- rate of unconfirmed fault hypotheses, additional-measurement requests and witness integrity failures;
- compute/update latency and total cost, including instrumentation, calibration, data review, maintenance, extra tests and independent audit.

Suggested research targets after the dataset and independent specimen count are frozen: zero synthetic false-clear; 100% raw measurement immutability; 100% witness-to-channel identity enforcement; 100% independent replay agreement; no silent out-of-domain promotion; and report calibrated false-clear confidence bounds on physical holdouts. These are gates to measure, not claims already achieved.

## Safety and NDE boundary

SHM anomaly scores and virtual reconstructions do not replace qualified inspections. If the result affects structural acceptance or maintenance, the benchmark must retain NDE method/procedure, flaw truth, sample population, operator/equipment variability, hit/miss data, false calls and confidence bounds. Relevant references include MIL-HDBK-1823A (NDE system reliability guidance; guidance, not a contractual requirement) and ASTM E2862-23 (hit/miss POD statistical analysis). They do not certify this implementation.

- MIL-HDBK-1823 status: https://quicksearch.dla.mil/qsDocDetails.aspx?ident_number=206122
- ASTM E2862-23: https://store.astm.org/e2862-23.html
