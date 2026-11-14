# Maritime Engineering Intelligence — architecture and benchmark roadmap v0.1

**Programme:** Luminous Dynamics / Symtropy / Symthaea / Mycelix  
**Status:** research and synthetic contract programme; no vessel deployment or physical qualification claimed  
**Date:** 10 November 2026

## Executive direction

Build a vendor-neutral **engineering evidence and decision-support layer** for maritime assets, not another isolated fault classifier.

The common evidence spine should bind:

asset and installed configuration → source and issuer → timestamp/clock/freshness → calibration and units → raw/derived/reconstructed state → dependency/common-cause lineage → fault/risk hypotheses → affected engineering capability → uncertainty → human review and recovery evidence.

The architecture has independent task owners. It does not create a second ODD evaluator, machinery controller, OEM diagnostics suite, maintenance authority or certification process.

## Programme ownership

| Area | Owner / artifact | Responsibility |
|---|---|---|
| Engineering evidence contract | Symtropy PR #4444 | Source/configuration admission, synthetic corpus, capability-impact projection |
| Vessel ODD and mode state | Symtropy #3893 | Vessel-specific operating envelope and ODD dispositions |
| Mode, authority and fallback | Symthaea #11185 | Human/remote/autonomous mode authority and contingencies |
| Cross-stack provenance | Mycelix #5974 | Issuer scope, exact artifacts, revocation and owner-to-owner handoffs |
| Cross-engine transfer and continual learning | Symtropy #4510 | Domain generalization, negative transfer and forgetting |
| Control-aware diagnosis | Symtropy #4628 | Typed control intent, actuator/controller state and physical response; maneuvering-regime conditional diagnosis and dropout tests |
| Data trust and reconstruction | Symtropy #4535 | Missingness, drift, reconstructed values and sensor integrity |
| RUL and maintenance utility | Symtropy #4545 | Censored event-time forecasts, calibration and maintenance-policy replay |
| Narrative causal risk | Symtropy #4548 | Source-grounded incident/maintenance extraction and time-varying causal risk |

An output from one owner is an input artifact for another owner, not a replacement for that owner's decision contract. Every handoff must carry exact source/evaluator/model digests and preserve unknown/unsupported/conflict states.

## Architectural planes

### 1. Evidence and identity plane

- Exact vessel, asset, component, installed configuration revision and functional capability.
- Source role and issuer; measured, native alarm, operator report, maintenance record, model residual, external diagnostic and simulator-derived data remain distinct.
- Unit, calibration, validity profile, clock domain, timestamp, age and uncertainty lineage.
- Declared dependency/common-cause groups; derived or copied evidence is not independent corroboration.
- Raw, derived and reconstructed values are separate typed states.

### 2. Engineering inference plane

- Condition detection and diagnosis.
- Data-quality/anomaly qualification and mask-aware reconstruction.
- Cross-load/cross-engine/domain transfer and continual adaptation.
- Degradation/prognostics and survival/RUL distributions.
- Incident/maintenance narrative extraction, causal graph construction and time-varying Bayesian risk.

Each inference family has task-specific metrics and independent calibration. Do not average accuracy, RUL error and causal extraction into a single score.

### 3. Capability and decision-support plane

- Map evidence to an explicit asset/component/capability and function-allocation relation.
- Provide a bounded evidence contribution to the ODD/mode owner.
- Never convert a machinery status directly to a vessel-level ODD disposition.
- Keep recommended inspection/maintenance options distinct from issued work orders, provider commitments and human authorization.
- Restoration evidence may be review-ready; it never clears a restriction or restores a mode automatically.

### 4. Evidence execution and replay plane

- Deterministic effects-disabled scenario replay.
- Frozen inputs, exact code/model/corpus digests, independent evaluator and immutable reports.
- Negative controls and mutation campaigns for every hard gate.
- Unknown outcome or unavailable evidence means no promotion to a permissive state.

### Control-aware inference

A control-aware diagnosis track is separately owned by [MARITIME-CONTROL-AWARE-DIAGNOSIS-001 #4628](https://github.com/Luminous-Dynamics/symtropy/issues/4628) and contributes typed signals and matched experiments to #4510. It distinguishes:
- control intent (setpoints and requested commands);
- controller/actuator state (outputs, feedback, saturation, mode and interlocks);
- measured physical plant response.

A 24 September 2026 simulation-only paper proposes control-aware feature fusion and multi-class maneuvering-regime discrimination for marine-engine injector diagnosis. It reports improved robustness under control-sensor dropout but explicitly lacks real-engine validation. This was the fifteenth literature baseline when first added. The registry now contains 29 unique IDs and 29 unique DOIs, including the October 2026 physical WP4 digital-twin / inverse-identifiability baseline, September 2026 few-shot combustion diagnosis, July 2026 external-context maritime anomaly analysis, the December 2025 operational-vessel induced-fault study and additional transfer/OOD baselines. See the machine-readable registry for each study's separate evidence scope and limitations.

Command-conditioned residuals must account for actuator lag, load/ambient disturbances, controller mode, sensor bias, clock skew and shared dependencies. Commands are not physical measurements or independent corroboration. If a candidate dataset lacks verified control-signal semantics, this track remains NOT_EVALUATED for that dataset.

## Benchmark matrix

| Benchmark family | Primary metrics | Critical holdout / falsification |
|---|---|---|
| Fault detection and diagnosis | Macro-F1, balanced accuracy, event recall, false alarms per engine-hour, duration, detection latency, worst-fault recall | Entire runs, loads, engines and vessels; no overlapping-window leakage |
| Reliability-aware diagnostic status | Risk–coverage, selective risk at matched coverage, false-safe rate, severe-case underwarning, status coverage, evidence-completeness sensitivity | Scenario-disjoint train/calibration/validation/test; compare forced class, confidence, margin and entropy rejection; pressure/torsion feature-group loss; severe-class recall and positive controls |
| Control-to-response consistency | Conditional residual coverage/sharpness, response-mismatch event recall, false mismatches per engine-hour, regime-wise calibration, prediction-to-response latency residual | Command available by prediction time; response strictly after prediction; future-command and future-model controls; held-out regimes; control/feedback dropout; common-cause removal; no actuation |
| Operational data quality and reconstruction | Missingness precision/recall by mechanism; localized anomaly precision/recall; region-wise reconstruction RMSE/MAE and interval coverage; false-safe rate; downstream fault-event recall retained after repair | Whole recording/campaign/vessel holdouts; fit GMM-EM regions, SVD bases, normalization and imputation on training only; include MNAR/block dropout/stuck-at cases; retain raw/mask/reconstruction roles; reconstructed outputs never count as independent measured evidence |
| Cross-individual engine transfer | Zero-shot per-engine macro-F1/event recall; worst-engine recall; target-twin parameter error; false-known/OOD rate; calibration | Hold out complete physical engine identities; separate target healthy-reference/twin calibration from sealed target-fault evaluation; no target-fault labels in tuning/normalization/thresholding; grouped uncertainty by engine; simulation and physical transfer reported separately |
| External-context robustness | False-context-excusal rate, false-internal-attribution rate, negative-control sensitivity, context-conditioned residual coverage/sharpness, context OOD abstention, worst-context recall | Same-internal-state context swaps; same-context internal-fault swaps; no-path negative controls; post-prediction/weather leakage; context-source independence; C-shift/F-shift/C+F splits |
| Multi-source / cross-domain transfer | Target-domain delta, per-source contribution, negative-transfer rate, calibration and OOD false-known | Hold out whole operating conditions/engines; target fault labels sealed in zero-shot |
| Continual adaptation | Average accuracy, backward transfer, forgetting, prior-domain calibration, update compute/memory | Re-evaluate every previous domain after each update |
| Data trust / reconstruction | Missingness detection, anomaly recall, reconstruction error, interval coverage, false-safe reconstruction | Dropout correlated with faults, stuck sensors, drift, missing-not-at-random, reconstruction leakage |
| RUL / prognostics | MAE/RMSE, interval coverage/sharpness, proper scores, calibration, prognostic horizon, censoring-aware survival metrics | Whole engines/runs; intervention censoring; competing risks; future-data leakage |
| Narrative causal risk | Source-span precision/recall, attribution, causal-edge fidelity, expert agreement, Brier/log score, prior sensitivity | Duplicated/copied reports, source hallucination, post-event leakage, symptom-conditioned vs unconditional risk |
| Maintenance decision utility | Downtime, unplanned intervention, inspection burden, spares, missed-fault exposure, lifecycle cost | Same event tape, explicit cost/logistics assumptions and human practice baselines |
| Passive value of information | EVSI/net EVSI, posterior decision-risk reduction, ambiguity resolution per observation, observation/review burden, regret at matched risk | Already-available records only; calibrated likelihood models; priors/loss frozen before confirmatory evaluation; no derived/shared/stale evidence, active excitation or new measurements |
| Cross-OEM evidence interoperability | Asset/config match, source lineage completeness, replay agreement, schema/protocol conformance, unknown retention | Different OEMs, version drift, offline/partition behavior, revoked/stale inputs |

### Hard non-compensatory gates

- Zero stale/wrong-configuration/unauthenticated/simulator-only evidence promoted as current measured state.
- Zero false independence across copied, derived or common-cause sources.
- Zero promotion of control intent or controller output into measured physical response; actuator feedback requires verified signal semantics and never proves delivered fuel or torque by itself.
- Zero use of commands, model updates or measurements that were unavailable at prediction time; unsupported regimes and configuration changes withdraw the affected qualification.
- A response mismatch alone never names a root cause or authorizes maintenance, mode transition, restriction clearance or actuation.
- Zero environmental-context use to excuse an independently supported internal-fault residual; zero context effect from a declared negative-control variable with no qualified physical path.
- Zero observational association, SHAP attribution or weather/anomaly correlation promoted to causal mechanism without an independently qualified physical path and appropriate intervention/identification evidence.
- Zero post-prediction, stale, out-of-support or internally derived external-context observations promoted as independent predictive evidence.
- Zero imputed/reconstructed-to-measured promotion.
- Zero target-label, future-data, temporal-window or intervention/censoring leakage.
- Zero false-safe restoration or mode-transition claims from partial evidence.
- Zero D0/reliable-normal or D3/D4/high-severity status promotion when required evidence is missing, stale, uncalibrated or outside the qualified feature-group envelope; diagnostic status is not maintenance or vessel operating authorization.
- Zero unauthorized maintenance/work-order/ODD/actuation effects.
- Zero positive EVSI claims from unqualified likelihood models, derived/shared/stale observations or evidence already included in the prior; score retrieval/review burden in the same declared decision-loss units.
- Passive EVSI ranking never initiates a measurement, control excitation, machinery test or inspection; it is an information-priority estimate, not authorization.
- No benchmark score can compensate for a hard-gate failure.

A model that abstains on everything also fails: positive controls, coverage, capability retention and useful workload must be measured.

## Competitive positioning

Public vendor descriptions show strong incumbent capabilities:
- Wärtsilä Expert Insight: vessel-specific monitoring, AI/rule-based anomaly analysis, expert review and crew/shore collaboration.
- Everllence CEON / PrimeServ Assist: OEM-connected real-time engine/turbine/compressor monitoring, 24/7 expert response and CEON TechBot grounded in OEM documentation; PrimeServ Assist received Bureau Veritas SMART(MH3) software/service approval in March 2026, subject to vessel-specific approval elements.
- Kongsberg K-CMS: type-approved condition monitoring and decision support, with DNV certificate conditions for specified versions, topology, vessel asset inventory and IACS UR E27 Rev.1 cyber requirements.
- DNV Machinery Maintenance Connect: structured machinery-maintenance history, digital surveys and fleet benchmarking.

Do not claim superiority over proprietary services without a matched authorized evaluation. The opportunity is to compete **across the OEM boundaries**:
1. preserve evidence provenance and configuration identity when combining different vendor outputs;
2. make unknown, stale, reconstructed and conflicting data explicit;
3. provide calibration and out-of-domain qualification across different engine families;
4. reproduce evidence and model decisions independently across edge/cloud and offline operation;
5. translate multiple evidence sources into function-specific capability impacts without usurping human/OEM/class authority.

## SOTA strategy and research targets

The goal is not to beat every paper's headline accuracy. It is to establish a stricter, operationally meaningful frontier:

- Reproduce reported scores on exact published splits before comparing.
- Then demonstrate fewer false alarms at matched event recall on held-out runs/loads.
- Demonstrate transfer to unseen engines and vessels only when independently identified physical assets exist.
- Improve worst-fault and unknown/OOD behavior, not only average performance.
- Show calibrated RUL intervals under explicit duty/censoring assumptions.
- Show decision-policy utility under matched costs and operational constraints.
- Demonstrate vendor-neutral evidence portability and independent replay.

Current numerical targets in child issues are explicitly aspirational; no target is an operational threshold or accepted safety limit. The Aalto MU323DGSC dataset is a single controlled four-stroke engine testbench. It supports physical fault and load-transfer experiments after payload audit, but it cannot validate fleet transfer or RUL without run-to-failure/intervention labels.

## Validation ladder and claims

- **V0 — contract/synthetic:** schema, identity, source roles, clocks, negative controls and deterministic evaluation.
- **V1 — payload audit:** licensed source files, archive integrity, member digests, channel/units and timestamp/label audit.
- **V2 — paper reproduction:** exact published methods/splits, code/data rights and matched metrics.
- **V3 — held-out physical benchmark:** full runs, loads, engines and independent data partitions.
- **V4 — operational evidence:** permissioned shipboard data, human maintenance labels, multi-vessel exposure and independent review.
- **V5 — vessel-specific engineering/class/operator acceptance:** applicable authority and safety processes.

A pass at one level does not imply any higher level. A synthetic pass is not physical accuracy; a held-out testbed pass is not shipboard generalization; a model result is not an operating permission.

## Immediate execution order

1. Complete the Aalto archive integrity and channel/label audit; create only reviewed file-level split assignments.
2. Reproduce the 2026 multi-source transfer and 2025 18-task cross-condition studies under their exact protocols.
3. Run held-out recording/load Aalto tests with common baselines and uncertainty intervals.
4. Build the cross-engine dataset partnership path; do not claim multi-engine transfer until a second independent engine is available.
5. Develop missingness/reconstruction mutation suite before adding imputation to any diagnostic model.
6. Reproduce the two-stroke RUL and LLM/Bayesian causal-risk papers on their own data/protocols.
7. Compose bounded owner-native artifacts through ODD/mode and Mycelix provenance contracts.

## Claim ceiling

This programme currently establishes research artifacts, synthetic evidence-contract semantics, literature and vendor analysis, and an authored dataset-audit tool. It does not establish physical diagnostic accuracy, fleet reliability, safe remaining life, shipboard effectiveness, class approval, MASS Code compliance, maintenance authority, ODD clearance or permission to operate.

## Literature update — transfer baselines (10 November 2026)

The diagnostic baseline registry now contains **29 unique IDs and 29 unique source DOIs**. Every result remains bound to its data domain and split protocol; simulation, test-bench, one-vessel and multi-engine evidence are not merged into a single leaderboard. The December 2025 operational-vessel SVM result is a particularly strong reproduction target, but not a cross-engine/fleet benchmark.

- The 2025 Applied Energy cross-working-condition paper reports **95% accuracy** in its abstract and describes **18 transfer tasks** across two engine case studies (8V396SE, 500 kW, and 620 V16, 2,336 kW). Its evidence mix includes experimental bench lubrication-system data and virtual intake/exhaust task data. Keep each subsystem and source type separate. The exact per-task aggregation, sample grouping and split manifest must be recovered before reproduction. This is not a common-protocol or physical fleet-transfer score.
- The 2025 Energy mean-value-model-assisted dual-transfer paper reports **more than 97.70% average accuracy** for cross-domain lubricating-oil diagnosis. It describes a simulation-to-measured transfer between a 16V396 simulation domain and an 8V396 experimental bench domain, nine transfer tasks, and 70%, 85% and 100% load points. The exact source/target assignments, run grouping, task-level scores and leakage controls need methods/table audit. It is a separate paper from the 18-task Applied Energy study and is not comparable to the Aalto MU323DGSC data.
- A 2026 adaptive multi-source study on an 8V396 testbed reports 83.64% average accuracy on pure-real measured scenarios and 82.37% on real/simulation hybrid scenarios. Do not compare this average directly with either 2025 result without aligning task taxonomy, target exposure, class balance and held-out physical groups.

### Consequence for benchmarking

The reported 95%, >97.70%, 83.64% and hybrid/simulation headline scores are **separate reproduction targets, not a single ranking**. Each study has its own task definition, engine, evidence domain, split semantics and metrics. A fair comparison needs two stages:

1. Reproduce each paper under its own documented protocol, with a frozen environment, raw-data/license audit, exact split manifest, source code commit and per-task table.
2. Re-run selected methods in a shared evaluation harness with whole-recording/run separation, load-held-out tests, source-domain holdouts, no target-label leakage, calibration and OOD controls. Report per-task and worst-domain performance, event recall at fixed false-alarm rate, false alarms per engine-hour, latency, uncertainty/calibration, compute/latency and prior-domain forgetting.

Do not treat random-window or random-row splits as evidence of cross-run or cross-engine generalization. Do not silently merge measured, simulated and hybrid source domains. A high headline accuracy cannot compensate for a hard-gate failure in provenance, split leakage, unknown-state handling or false-alarm burden.



### Additional October/September 2026 baselines

- **Minchev et al., 10 October 2026 — Weichai WP4 digital twin.** A calibrated 0D–1D model is compared with physical test-bench measurements, including a controlled intake restriction. The paper is especially valuable because it explicitly reports non-unique inverse solutions: different physical-factor combinations can produce similar observable responses. It is not a continuously synchronized real-time twin; model runs are initiated through the diagnostic workflow. Our benchmark should therefore measure inverse-problem identifiability and ambiguity, not just whether an optimizer returns its preferred parameter vector.
- **Luo et al., 29 September 2026 — few-shot Wärtsilä 6L20 combustion diagnosis.** Its cycle-level leakage-free, twin-generated benchmark reports 87.05% ± 5.14 pp for a joint triplet–center method at K=20, but a triplet-only ablation reports 90.91% in the same setting. Accuracy under combined noise/speed fluctuation is reported at 84.43% across 25–100% MCR. This is a useful matched few-shot baseline, but it is simulation-based rather than real-vessel fault validation, and the stronger ablation must be reproduced rather than ignored.


### Control-response benchmark principle

For each response model, preserve three distinct temporal records: command availability to the diagnostic process, model prediction issue time, and independently observed plant-response time. Require command availability no later than prediction, response strictly later than prediction and inside a predeclared horizon, and command freshness measured at prediction time. A post-hoc command/measurement correlation is not a causal prediction test.

The scored output is a calibrated residual/evidence status, not a fault name. Out-of-envelope response means only that the conditioned expectation was not met. Fault isolation requires independently discriminating evidence; when fault hypotheses remain observationally equivalent, report the equivalence class and the missing measurement needed to separate them. Any observation-selection proposal must be read-only and must not imply permission to change operating conditions.


## Literature update — external context and operating-environment confounding (11 November 2026)

**Park et al., July 2026 — Explainable AI-based root-cause diagnosis of maritime engine anomalies with external factors (DOI 10.1016/j.cie.2026.112028).** The paper reports 18 months of operational data from one container ship, 335 internally detected anomaly events and four clusters. A VAE supplies internal-engine anomaly scores; a LightGBM Root Cause Anomaly Predictor (RCAP) predicts those scores from external operational/environmental variables including wind-wave height, draft/trim and sea-surface temperature; SHAP is used to explain predictive associations. The registry records the study as reported-not-reproduced and not directly comparable to Aalto.

The study opens a new benchmark axis: **external-context qualification**. Engine-internal anomaly manifestation and external operating context must remain separate evidence classes. A context record must carry source/issuer and artifact digest; observation/forecast/reanalysis/reported/derived role; **available time** (when the diagnostic process could know it), including clock uncertainty; **valid time** (when it applies), separately from available time; vessel/route/spatial scope and alignment uncertainty; units, source quality, derivation lineage and common dependencies.

For predictive use, the latest possible available time must be no later than the earliest prediction time. Forecast valid time must overlap the declared response horizon and its route/asset scope. A later reanalysis or retrospective observation may support post-hoc analysis, but must be labelled `RETROSPECTIVE_ATTRIBUTION_ONLY` and excluded from predictive features. A forecast issued before prediction can be valid for a future response interval; available-time and valid-time cannot be collapsed into one timestamp.

### External-context benchmark tournament

Compare, on the same chronological/route-held-out evidence:
1. Internal-only physical residual / VAE anomaly baseline.
2. External-only RCAP-style anomaly-score predictor.
3. Naive internal + external fusion.
4. Context-conditioned physical-response residual.
5. Regime/route-stratified baseline.
6. Causal-graph / structural-equation hypothesis where identification assumptions are explicit.
7. Selective predictor that abstains on missing, stale, spatially misaligned or post-hoc context.

Report event recall at a matched false-alarm-per-engine-hour budget; context-conditioned residual calibration; anomaly-score prediction skill; worst-weather/route/load strata; context-source dropout and spatial/time-misalignment degradation; calibration/selective risk; and matched-context permutation controls. SHAP or other attribution is a model explanation, not causal evidence.

### External-context falsification gates

- No forecast issued after prediction may enter its feature set.
- No retrospective reanalysis may be silently promoted to real-time available context.
- A context record outside the vessel/route valid-time/spatial envelope cannot support a prediction.
- Two weather products sharing an upstream feed/dependency do not count as independent corroboration.
- SHAP attribution or RCAP score prediction alone cannot assert a causal engine fault.
- Context effects must be evaluated within matched load, speed, route and sea-state strata to avoid confusing operating policy with physical degradation.


## Literature update — operational-vessel fault diagnosis comparator (12 November 2026)

**Upadrashta & Wijaya, first published 24 December 2025 — AI/ML Based Anomaly Detection and Fault Diagnosis of Turbocharged Marine Diesel Engines: Experimental Study on Engine of an Operational Vessel (DOI 10.3390/info17010016).** The study uses a 1.89 MW, 900 RPM auxiliary diesel generator aboard a cargo/container vessel. Two fault mechanisms—turbocharger nozzle-ring clogging and intercooler/charge-air restriction—were physically induced on one auxiliary engine at three steady loads (825/900/975 kW), each with three severity levels. The authors report a seven-class SVM balanced accuracy of **99.72%** under an 80/20 training/test split and 10-fold cross-validation. Their combined-fault stacked-autoencoder anomaly result is **87.57% balanced accuracy / 83.84% F1**; intercooler-only SAE is **75.0% balanced accuracy / 63.1% F1**, while UAE is **51.55% / 6.04%**. The paper also reports only about **1.3% fault observations** in a 96,897-observation dataset.

This is the strongest operational-vessel classification comparator currently in this registry, but its claim ceiling must remain narrow: only one fault-injection engine, two mechanisms, three steady loads, and no independent-vessel holdout are reported. The 80/20 split is not described as intervention/run/chronology grouped. The dataset is available only under institutional/partner data-sharing conditions, not as an open payload. The abstract calls the SAE metric 87.6% “accuracy”, while the detailed results distinguish 98.15% ordinary accuracy from 87.57% balanced accuracy; our registry records the detailed table metrics.

### Reproduction and confirmatory tournament

1. Reproduce the paper-reported 80/20 + 10-fold protocol, SVM, SAE and UAE with exact preprocessing, threshold and feature definitions.
2. Obtain the source split semantics and run/intervention identifiers. If unavailable, mark exact reproduction as NOT_EVALUATED and do not reconstruct a random-window split from summary figures.
3. Run chronological block holdout and intervention/run-grouped holdout; perform leave-one-steady-load-out across 825/900/975 kW as a separate within-engine test.
4. Report the seven-class balanced accuracy and macro-F1, per-severity recall, calibration and selective risk; report anomaly event recall, false alarms per valid engine-hour, detection latency and threshold sensitivity.
5. Reproduce the difficult intercooler-only results and the UAE baseline, then test low-severity generalization, missing channels, sensor bias, clock/gateway loss, and held-out configuration.
6. Compare thermodynamic-only, vibration-only and fused inputs, while preserving the differing acquisition pathways and measurement uncertainty.

The 99.72% result is a **paper-reported reproduction target, not a fleet SOTA ranking**. The programme should aim to match it under the original protocol and materially outperform it under stronger unseen-run, unseen-load, weak-fault, uncertainty and evidence-integrity evaluations.


## 12. Deployment-shift and adaptive-sensing benchmark extension (12 November 2026)

Recent reviews indicate that the field needs scenario-aligned evaluation, not another pooled classifier leaderboard. The September 2026 mechanical OOD survey (Lin et al., DOI 10.1007/s10462-026-11722-3) codes 161 primary OOD-related mechanical diagnosis studies and separates condition shift (C-shift), fault-semantic shift (F-shift) and combined C+F settings. It argues that known-class generalization and unknown-fault detection are different requirements; reliable deployment combines both with calibrated OOD detection and an escalation/human-referral route.

The June 2026 zero-shot survey (DOI 10.1049/gtd2.70361) distinguishes four target-semantic/domain-relation scenarios (SD-UFC, SD-CFA, CD-UFC, CD-CFA) and two special data-access settings (zero-target-shot and zero-fault-shot). It explicitly identifies scenario-aligned evaluation/reporting standards as a gap. Therefore, every Symtropy transfer result must declare whether it uses labelled target examples, unlabelled target data, a target fault catalogue/semantic prior, or no target examples at all. “Zero-shot” must never be used as a synonym for ordinary unsupervised domain adaptation.

An October 2026 review of adaptive sensor placement for mechanical digital twins (Li et al., DOI 10.1016/j.jmsy.2026.07.027) describes a lifecycle loop of virtual pre-placement, data/model fusion and calibration, performance evaluation, triggered re-optimization and software-defined adjustment. It also names model generalization, real-time computation and complete-loop integration as open challenges. For maritime engineering, the safe translation is **qualification-aware redesign proposals**: reevaluate diagnosability when configuration, sensor calibration, operating envelope, fault priorities or onboard maintenance constraints change; do not autonomously install, move, enable or disable physical sensors.

### Required benchmark axes

| Axis | Frozen split / access rule | Primary outputs | Must not be conflated with |
|---|---|---|---|
| Within-engine condition generalization | Hold out entire loads/cycles/transient regimes within one physical engine | Per-fault macro recall, event recall, calibration, latency | Cross-engine transfer |
| Run/intervention generalization | Hold out entire fault injections, campaigns or recordings | Event recall, false alarms per valid engine-hour, severity recall | Random window split |
| Cross-engine transfer | Leave one physical engine/configuration out; disclose target labels and target calibration | Worst-engine performance, negative transfer, calibration | Cross-vessel transfer |
| Cross-vessel transfer | Leave one vessel out; independent instrumentation/configuration | Worst-vessel performance, failure/unknown rates | Another run on the same vessel |
| Fault-semantic OOD | Hold out complete fault families or severity mechanisms | False-known rate, unknown-fault recall, risk-coverage, referral precision | Condition-only OOD |
| Combined C+F shift | Hold out both condition and fault semantics | Joint selective risk and calibration under unseen combinations | A single held-out load |
| Zero-shot | Freeze catalogue/semantic priors; target data access must be exactly specified | Per-scenario accuracy, false-known, prior sensitivity, abstention | Few-shot or unlabeled-target adaptation |
| Adaptive sensor layout | Freeze baseline configuration; trigger on a declared change; compare against no-reoptimization and static Pareto layouts | Retained diagnosability, worst-case pair loss, requalification latency, lifecycle burden | Classifier retraining alone |

### SOTA reference scores — separate evidence strata

These are reported values from different papers and must not be ranked as one table:

- **99.72% balanced accuracy:** a seven-class SVM severity classifier on one operational-vessel auxiliary engine, two induced fault mechanisms and three steady load points; the reported 80/20 split and 10-fold CV are not established as intervention- or vessel-grouped. This is the strongest operational-vessel comparator in this registry, not a fleet transfer result.
- **83.64% average accuracy:** adaptive multi-source transfer on pure real measured scenarios for an instrumented 8V396 testbed and lubrication faults; exact split groupings still need source audit.
- **82.37% average accuracy:** the same study's real/simulation hybrid scenario; separate from its real-only result.
- **70.18% accuracy:** zero-shot virtual-to-real transfer in the Xie et al. multi-source thermal dataset; simulation, industrial-noise bench, real-vessel telemetry and virtual-real fused data are distinct evidence subsets.
- **99.45% accuracy:** a separate simulation-only 2,567-sample GT-Power study with randomized stratified 70/30 split and five-fold CV. It is not physical-vessel accuracy.

Our first target is to reproduce each result on its own protocol and evidence source, including the strongest simple ablation and per-fault results. Our second target is to show which scores survive grouped chronological/intervention splits, leave-one-load-out, leave-one-engine/vessel-out, OOD faults and sensor/common-cause removal. Do not claim “better than SOTA” by comparing 99.45% simulation accuracy with 83.64% measured transfer or 99.72% balanced accuracy on a single vessel.

### Proposed qualification metrics and research aims

Report separate known-class and unknown-class results: macro/balanced recall, per-fault/severity recall, event recall, false alarms per valid engine-hour, detection latency, false-known/OOD rate, unknown-fault recall, Brier/log score or interval coverage where probabilistic semantics are valid, risk-versus-coverage, referral precision, worst-engine/worst-vessel subgroup, and degradation under sensor/timebase/common-cause loss.

Proposed research aims (not achieved results or operational thresholds):
- +5 percentage points macro/event recall at a matched false-alarm-per-engine-hour budget on an independent run/intervention holdout.
- ≤5% false-known rate on frozen unknown-fault episodes at a declared coverage level.
- Prior-domain macro-recall degradation ≤2 percentage points after each adaptation, with calibration independently rechecked.
- 100% correct abstention/referral on the frozen synthetic C-shift/F-shift/C+F falsification matrix.
- No accepted future, post-hoc or target-label leakage in the confirmatory split.
- Re-optimize the sensor-layout Pareto set only after a declared configuration/mission change; report the number of prior diagnoses whose qualification remains valid, the number requiring requalification, and the cost/time of that process.

The benchmark result should therefore be a scenario-by-scenario evidence passport, not one composite score. A hard-gate violation (future leakage, false-known on an unobservable fault, false-independent channel promotion or unauthorized operating action) cannot be averaged away by a high classification score.


### Operational-vessel fault-diagnosis baseline — 99.72% reported balanced accuracy

The registry entry `operational-vessel-turbocharger-diagnosis-2025` records a December 2025 *Information* paper (DOI 10.3390/info17010016) using an in-service 1.89 MW Hyundai HiMSEN auxiliary generator. It reports 99.72% balanced accuracy for a linear SVM over seven classes: healthy plus three severity levels each for turbocharger nozzle-ring clogging and intercooler/charge-air restriction.

Important limits preserved in the registry:
- Physical fault interventions were on one auxiliary engine and two fault mechanisms, across three steady load points (825, 900 and 975 kW); dynamic transients were excluded.
- The 96,897-observation / 42-feature dataset is highly imbalanced (about 1.3% fault observations).
- The paper reports an 80/20 split and ten-fold CV, but the accessible protocol does not establish grouping by intervention, recording/run, chronology, engine or vessel. No independent-vessel fault holdout is reported.
- Thermodynamic values were captured non-intrusively from engine-room displays rather than direct machine interfaces; the paper identifies detailed measurement-uncertainty analysis as future work.
- The intercooler-only anomaly result is substantially weaker (75% balanced accuracy, 63.1% F1), making early/low-severity detection a useful falsification target.

Do not claim to beat this result until it is reproduced on the paper's own protocol. Then evaluate whole-run/intervention and chronological holdouts, leave-one-load-point-out, severity-wise event recall, false alarms per valid engine-hour, calibration and unknown-fault rejection. Cross-engine/vessel performance remains NOT_EVALUATED unless independent fault-labeled engines/vessels are actually available. This dedicated reproduction and grouped-split audit is tracked by [MARITIME-OPERATIONAL-BASELINE-001 #4909](https://github.com/Luminous-Dynamics/symtropy/issues/4909).


## External-context robustness and adaptive sensing

The external-context benchmark is now an executable companion to the control-response contract: [#4629](https://github.com/Luminous-Dynamics/symtropy/pull/4629), benchmark ID `MARITIME-EXTERNAL-CONTEXT-ROBUSTNESS-001`. It separates the internal physical state, external environment, a configuration/regime-bound context-effect path, the expected physical response and the measured response. The synthetic suite checks that high sea state can explain an otherwise healthy response without creating an internal-fault label, while an independently evident internal degradation remains a mismatch under the same high sea state.

This distinction is supported by current research, but the evidence levels matter:

- Park et al. (2026) report an internal-VAE / external-only RCAP-SHAP method on 18 months of one container ship's data, with 335 anomalies in four clusters and external associations involving wave height, draft/trim and sea-surface temperature. This is a useful operational-vessel observational comparator, not proof of causal intervention effects or multi-vessel transfer: https://doi.org/10.1016/j.cie.2026.112028.
- Lin et al. (2026) classify mechanical diagnosis shifts as condition shift (C-shift), fault-semantic shift (F-shift) and combined C+F, and distinguish known-fault generalization from calibrated unknown-fault detection and human referral: https://doi.org/10.1007/s10462-026-11722-3.
- Li et al. (October 2026) review adaptive closed-loop sensor placement for mechanical digital twins: virtual pre-placement, data/model calibration, online performance evaluation, trigger-based re-optimization and software-defined sensing adjustments. This suggests moving beyond a one-time static sensor layout, while keeping physical sensor changes and operational actions behind independent engineering authorization: https://doi.org/10.1016/j.jmsy.2026.07.027.

### Empirical protocol

Keep four evidence strata separate: observational vessel context associations, simulation-generated context interventions, controlled test-bench interventions, and independently replicated vessel/engine interventions. Use chronological and whole-run/whole-vessel holdouts. Match on operating regime and internal state where possible. Include negative-control exposures and context-swap counterfactuals. Report the context support envelope and abstention rate rather than extrapolating outside it.

### New scorecard

- **False-context-excusal:** internal faults that are accepted as environmentally explained when their independently supported internal residual remains outside the qualified conditioned envelope.
- **False-internal-attribution:** healthy internal state labelled faulty when a qualified external context path explains the observed response.
- **Negative-control sensitivity:** rate at which a context variable with no qualified physical path changes the expected-response or diagnostic result.
- **Context-conditioned residual coverage and sharpness:** conditional on environment/load strata, including worst-context calibration and event recall.
- **C/F/C+F shift matrix:** known fault under unseen environmental conditions; unseen fault semantics under familiar conditions; and both shifts simultaneously, with explicit risk/coverage and referral precision.

Hard gates: zero false-context-excusal and false-internal-attribution on the frozen synthetic corpus; zero negative-control envelope changes; zero future-context leakage; 100% synthetic abstention outside the declared context-support region. These are software-contract aims, not measured vessel performance or operating limits.

### Lifecycle adaptive sensing

Treat sensor selection as a versioned engineering policy, not a one-off install-time optimum. Trigger a virtual layout re-evaluation when any of the following changes: asset configuration, mission/operating envelope, sensor availability or calibration, timebase quality, model scope, fault catalogue, or context support envelope. Report Pareto trade-offs across fault-pair diagnosability, context coverage, calibration burden, independence/common-cause exposure, latency, bandwidth and lifecycle cost.

A software-defined measurement policy may recommend selecting among already-installed qualified channels or changing analysis weights. It must not silently activate/deactivate physical instruments, alter controller inputs or change vessel operations. Route/load changes, test maneuvers and hardware modifications require separate approved authority. The qualification-impact engine in #4611 supplies the dependency-expiry mechanism for determining which prior claims must be repeated after a sensing-policy or configuration change.

### Literature update — evidence-aware diagnostic status (13 November 2026)

A 4 October 2026 study, “Reliability-Aware Diagnostic Status Formation for Marine Diesel Engine Fault Diagnosis Under Noisy and Incomplete Evidence,” adds a useful comparator. It separates forced class prediction from post-prediction diagnostic status and explicitly evaluates calibrated confidence, severity, pressure/torsion feature-group consistency and evidence completeness.

On the simulated 3500-DEFault benchmark at fixed 2500 rpm and four SNR levels, it reports a status-layer coverage of 0.9791 ± 0.0015, four-class selective risk of 0.0342 ± 0.0039, and false-safe rate of 0.00208 ± 0.00165 versus 0.0094 ± 0.0038 for forced classification. Its approximately coverage-matched entropy-rejection baseline has slightly lower selective risk (0.0329 ± 0.0037), and the authors do not claim superior rejection efficiency. Partial torsional-feature loss also exposes severe-case underwarning. Five split seeds measure split sensitivity, not independent engine replication. The results do not establish shipboard maintenance validity.

**Our reproduction target:** compare forced classification, calibrated confidence, probability margin, entropy rejection and a structured diagnostic-status layer at matched coverage. Report false-safe rate, severe-case underwarning, risk–coverage, and feature-group dropout effects separately. Do not optimize away refusal by accepting every case, and do not treat D0–D5 as maintenance authority or a vessel-level operating disposition.


### Literature update — control-aware affine maneuvering diagnosis (13 November 2026)

Luo et al., published online 21 October 2026, propose a Control-Aware Affine Network (CAAN) for marine diesel injector diagnosis in dynamic maneuvers. Unlike simple feature concatenation, CAAN uses control signals to dynamically calibrate fault features, then separates severity-bearing and maneuver-invariant representations using knowledge distillation and adversarial training.

This is an important comparator for #4628: compare (1) command/context-conditioned physical residuals, (2) CAAN-style control-conditioned feature calibration, (3) severity retention, and (4) selective rejection under unsupported maneuvers. The accessible abstract does not report numeric performance metrics. The authors state that the simulation data and self-developed model are unavailable for public sharing under confidentiality/IP restrictions. It is therefore a reported-but-not-reproduced architecture baseline, not a score we can claim to beat.


### Passive information prioritization

The maritime observability work now supplies a synthetic EVSI evaluator for already-available independent records. The same methodology should be applied to #4444: rank existing telemetry, archived OEM records, calibration evidence and human-reviewed records by expected reduction in diagnostic decision loss. A likelihood model must be independently qualified, and a candidate that shares dependencies with current evidence must not be counted as new information. The experiment should compare EVSI ranking with a fixed-checklist baseline at matched decision risk and report the number of observations/reviews needed to resolve the same ambiguity class. Active diagnostic excitation remains outside this read-only programme.


## Open-world generalization and multiview corroboration — 13 November 2026

The 2026 open-set fault-diagnosis literature makes it necessary to separate two independent shift axes:

- **Condition shift (C-shift):** known fault semantics, but unseen operating conditions, loads, speeds, environment, sensor mounting or asset configuration.
- **Fault-semantic shift (F-shift):** known operating domain, but previously unseen fault families or fault combinations.
- **Combined C+F shift:** both the operating domain and fault semantics are unseen. This is the most difficult open-world diagnostic setting and must not be hidden inside an aggregate cross-validation score.

The benchmark should publish four separate strata: closed known-domain/known-fault reference, C-shift, F-shift and C+F shift. Every stratum must use whole physical recordings, campaigns and assets as holdout units, with fault-family holdout for F-shift and entire operating-domain holdout for C-shift.

### New 2026 open-set comparators

The baseline registry adds four current general-machinery methods:

- **TFEAT** reports 94.74% average H-score across 16 open-set domain-generalization tasks and three public bearing datasets. This is the strongest numeric open-set comparator added in this iteration, but it is not a marine-engine score.
- **SEE–GROW** uses pseudo-unknown synthesis and boundary-aware representation learning for unseen-condition open-set diagnosis; its accessible abstract reports qualitative H-score superiority without a directly transcribed numeric result.
- **MVAPN** combines native multiview signals, adversarial purification, cross-view consistency and prototype-driven OOD synthesis across three public gearbox/bearing datasets. Its key lesson for our work is to use physically meaningful views while explicitly checking shared dependencies and common-mode disturbances.
- **Domain-augmented OSDG** reports 92.3% accuracy and 89.5% unknown-fault rejection on its own rotating-machinery protocol. These metrics are not directly comparable to TFEAT's H-score.

All four are recorded as reported-not-reproduced. They are methodological comparators, not evidence of marine-engine performance.

### Required open-world metrics

- Known-fault macro-F1 / balanced accuracy and event recall.
- Unknown-fault AUROC and AUPR, false-known rate at a preregistered known-fault acceptance coverage, and unknown-rejection recall.
- H-score with its known/unknown component scores separately reported.
- Risk-versus-coverage and abstention precision.
- Worst-domain and worst-fault performance, not only aggregate averages.
- Calibration error and uncertainty by C-shift, F-shift and C+F stratum.
- False unique-cause rate when multiple fault hypotheses remain admissible.

A multiview model receives no independence credit from raw sensor count. The observability adapter must qualify physical measurement principles and common-cause dependencies before cross-view agreement contributes to a fault conclusion.

### Competitive target

The research target is to reproduce the published H-score baselines under their exact splits, then improve the **worst-stratum** C+F H-score and false-known rate without sacrificing known-fault event recall or calibration. A marine-engine superiority claim requires a matched marine dataset and independently audited physical fault labels; the three generic rotating-machinery datasets do not satisfy that requirement.


### Geometric behavioural-set diagnosis review (May 2026)

Sheikhi et al., *Fault Diagnosis in Dynamical Systems: Geometric Interpretation and Tractable Algorithms*, Annual Review of Control, Robotics, and Autonomous Systems 9:147–179, DOI 10.1146/annurev-control-030123-015422. The publisher lists publication in May 2026 and first online review publication on 10 December 2025. It surveys diagnosis for linear and nonlinear systems, model-based and data-driven filters, behavioural-set geometry, robust residual generation and passive/active diagnosis. It is a general dynamics review, not marine-engine validation and not a common-protocol accuracy score.

The publisher lists an erratum published 5 June 2026 (DOI 10.1146/annurev-as-09-060526-200001). The erratum landing page has no abstract; detailed reproduction must inspect the corrected article rather than infer the erratum's scope.

This review motivates #5040's set-valued diagnosability work. The benchmark must distinguish a certified disjointness result from an actual intersection witness and from mere overlap of conservative outer approximations. Do not promote geometric theory coverage into physical marine-engine performance.


### Operational-vessel data quality and reconstruction — Taghavi & Perera (2026)

[Localized anomaly detection and recovery of marine engine data to support digital twin development in shipping](https://doi.org/10.1016/j.oceaneng.2026.124356), *Ocean Engineering*, Article 124356. The work uses GMM-EM to identify seven operating regions, then local SVD-based anomaly detection/recovery and separate steady/transient operating-mode identification on an ocean-going-vessel dataset. The repository registry records paper-reported SVD fuel-consumption RMSE of 2.27–2.66% of mean consumption in its main regions, hyperplane RMSE of 2.24–2.60% in main regions and 1.42–4.85% in transient regions, with localized processing reported under five seconds on the authors' standard laptop.

These are within-paper, selected-vessel results. The authors caution that some external RMSE values in the paper use different vessels/datasets and are contextual, not matched SOTA comparisons. Symtropy should reproduce the regime split and recovery methods on the exact source data before comparing. The key differentiator is not simply lower reconstruction RMSE: we must measure whether repair hides faults, creates false healthy evidence or falsely increases diagnosability. [Publisher record](https://www.sciencedirect.com/science/article/pii/S0029801826001903) · [SINTEF record](https://www.sintef.no/en/publications/publication/019dcdef47c1-61082462-cc2d-4a44-9a02-17d126993bcd).

### Cross-individual marine main-engine transfer — Sun et al. (2026)

[Research on cross-individual fault diagnosis of marine main engines based on adaptive mechanisms](https://doi.org/10.19693/j.issn.1673-3185.04917), *Chinese Journal of Ship Research*, 21(4), 432–441. Its abstract reports 0.052% MAPE for PSO-tuned individual-engine performance parameter factors and 100% DT-DANN diagnostic accuracy using healthy-state anchors, a multi-scale 1D-CNN and an improved domain-adversarial network. The paper explicitly describes the results as simulation results; this is not physical cross-engine validation. [Official English journal record](https://english.ship-research.com/en/article/doi/10.19693/j.issn.1673-3185.04917).

The fair reproduction target is a sealed engine-instance split with target healthy-reference calibration isolated from target fault evaluation. Report twin-parameter error separately from diagnostic accuracy; preserve per-engine and worst-engine metrics; disclose number of simulated engine instances, target-data access, label budget and uncertainty. Aalto's single MU323DGSC physical engine cannot validate a cross-individual result.

### New hard gates for the two tracks

- A reconstructed value must keep its original raw/missing state, reconstruction method, input-parent lineage, model/data digests and uncertainty. It cannot become a measured value or count as an independent sensor.
- Every data-repair method must be evaluated for false-safe errors and downstream fault-recall loss, not only pointwise RMSE.
- Regime segmentation, SVD bases, scaling, imputation and thresholding must be fit on training partitions only; test periods may not influence their own preprocessing.
- Cross-engine claims require whole physical-engine holdouts. With one physical engine, report within-engine load/regime transfer only.
- Simulation-only cross-individual accuracy must be reported separately from real test-bench and vessel evidence; never rank the 100% simulation result against a physical measured result without protocol alignment.


## Adjacent marine-equipment digital-twin comparators (14 November 2026)

These studies are strategically relevant to the marine engineering intelligence architecture, but they are **not marine-diesel-engine leaderboard rows**. Keep them in an adjacent evidence stratum and do not combine their accuracy/F1 values with engine-fault benchmarks.

| Study | Evidence domain and reported result | What it changes in our strategy | Comparison boundary |
|---|---|---|---|
| Wang et al., *Research progress and prospects of digital twin technology for marine monitoring equipment*, online 10 Nov 2026, DOI 10.1016/j.joes.2026.100791 | Review screens 49 core digital-twin studies and 45 supporting studies across marine monitoring equipment. It identifies fragmented data layers and limited long-term field validation, with priorities including sensing/communications integration, physics-data models with physical feedback and safety-aware decisions. | Treat the twin as a closed evidence chain: sensing → acquisition/time → asset/configuration identity → model/physical feedback → diagnostic task → bounded decision. Measure lifecycle and task-specific validation, not twin-dashboard completeness. | Marine monitoring equipment is broader than marine engines; this is a research-maturity review, not a fault-diagnosis performance score. |
| Kim et al., *A framework and application for dynamic assessment of ship equipment based on multi-source data fusion and digital twins*, IJNAOE 18 (2026), Article 100746, DOI 10.1016/j.ijnaoe.2026.100746 | Marine centrifugal-fan application synchronizes vibration, visual and speed data with a lightweight digital twin; six fault modes are reported with weighted F1 0.9965. Visual detection reports AP 0.861 for cracks and 0.827 for corrosion. | A strong adjacent multimodal-fusion comparator. We should match the task on the fan domain only if the data/methods are accessible, then add source-role lineage, physical measurement-principle independence, calibration, OOD/unknown faults, event-level false alarms and configuration-bound qualification. | Six fan fault modes and visual crack/corrosion detection are not diesel-engine fault classes. The weighted F1 must not be ranked against Aalto, LiU-ICE or engine papers. |
| Du et al., *A review of digital twin methods and applications for bearings under marine operating constraints*, online 16 Jun 2026, DOI 10.1080/17445302.2026.2684687 | Review distinguishes direct marine, marine-adjacent and transferable bearing-DT evidence, and physics-, data- and hybrid-driven methods. It identifies insufficient long-term online monitoring, shipboard field testing and full-chain verification for water-lubricated stern bearings. | Make evidence maturity an explicit benchmark dimension: direct physical marine observation, marine-adjacent test rig, simulation-only and transferable non-marine methods must stay distinct. Require the full sensor-to-maintenance evidence chain before claiming a qualified shipboard twin. | Review of marine bearing twins, not a diesel-engine diagnostic benchmark or an independently measured classifier result. |

### Implications for our competitive frontier

A task-specific weighted F1 of 0.9965 on a marine centrifugal fan is a strong adjacent result, but it does not establish that a system can distinguish engine faults under unknown loads, identify unobservable fault pairs, survive common-cause sensor failures or prove that two views are independent. Our shared evaluation harness should therefore expose separate scorecards for:

1. **Task reproduction:** reproduce the paper’s own asset, classes, sample units, split, metric and strongest simple baseline.
2. **Evidence integrity:** source/clock/configuration/calibration lineage, measured-versus-derived distinction, dependency review, future-information tests and independently replayable receipts.
3. **Diagnostic validity:** event recall at a matched false-alarm-per-valid-engine-hour budget, worst-fault/worst-regime recall, false-known/OOD rate, selective risk and calibration.
4. **Engineering diagnosability:** fault-pair isolation, ambiguity-set correctness, common-cause cuts, sensor-layout burden, and qualification withdrawal after physical/configuration change.

This is the more credible way to outperform the field: match each adjacent system on its own task first, then show which conclusions remain valid under independent evidence, unknown conditions, changing configurations and failure of the measurement architecture. No cross-domain performance superiority is claimed here.
