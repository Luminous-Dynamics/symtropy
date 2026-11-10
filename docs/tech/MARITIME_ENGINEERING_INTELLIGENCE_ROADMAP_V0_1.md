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

A 24 September 2026 simulation-only paper proposes control-aware feature fusion and multi-class maneuvering-regime discrimination for marine-engine injector diagnosis. It reports improved robustness under control-sensor dropout but explicitly lacks real-engine validation. This was the fifteenth literature baseline when first added. The registry has since been extended with an October 2026 physical WP4 digital-twin / inverse-identifiability baseline and a September 2026 few-shot combustion-diagnosis baseline; see the 17-entry registry for evidence scope and limitations.

Command-conditioned residuals must account for actuator lag, load/ambient disturbances, controller mode, sensor bias, clock skew and shared dependencies. Commands are not physical measurements or independent corroboration. If a candidate dataset lacks verified control-signal semantics, this track remains NOT_EVALUATED for that dataset.

## Benchmark matrix

| Benchmark family | Primary metrics | Critical holdout / falsification |
|---|---|---|
| Fault detection and diagnosis | Macro-F1, balanced accuracy, event recall, false alarms per engine-hour, duration, detection latency, worst-fault recall | Entire runs, loads, engines and vessels; no overlapping-window leakage |
| Control-to-response consistency | Conditional residual coverage/sharpness, response-mismatch event recall, false mismatches per engine-hour, regime-wise calibration, prediction-to-response latency residual | Command available by prediction time; response strictly after prediction; future-command and future-model controls; held-out regimes; control/feedback dropout; common-cause removal; no actuation |
| Multi-source / cross-domain transfer | Target-domain delta, per-source contribution, negative-transfer rate, calibration and OOD false-known | Hold out whole operating conditions/engines; target fault labels sealed in zero-shot |
| Continual adaptation | Average accuracy, backward transfer, forgetting, prior-domain calibration, update compute/memory | Re-evaluate every previous domain after each update |
| Data trust / reconstruction | Missingness detection, anomaly recall, reconstruction error, interval coverage, false-safe reconstruction | Dropout correlated with faults, stuck sensors, drift, missing-not-at-random, reconstruction leakage |
| RUL / prognostics | MAE/RMSE, interval coverage/sharpness, proper scores, calibration, prognostic horizon, censoring-aware survival metrics | Whole engines/runs; intervention censoring; competing risks; future-data leakage |
| Narrative causal risk | Source-span precision/recall, attribution, causal-edge fidelity, expert agreement, Brier/log score, prior sensitivity | Duplicated/copied reports, source hallucination, post-event leakage, symptom-conditioned vs unconditional risk |
| Maintenance decision utility | Downtime, unplanned intervention, inspection burden, spares, missed-fault exposure, lifecycle cost | Same event tape, explicit cost/logistics assumptions and human practice baselines |
| Cross-OEM evidence interoperability | Asset/config match, source lineage completeness, replay agreement, schema/protocol conformance, unknown retention | Different OEMs, version drift, offline/partition behavior, revoked/stale inputs |

### Hard non-compensatory gates

- Zero stale/wrong-configuration/unauthenticated/simulator-only evidence promoted as current measured state.
- Zero false independence across copied, derived or common-cause sources.
- Zero promotion of control intent or controller output into measured physical response; actuator feedback requires verified signal semantics and never proves delivered fuel or torque by itself.
- Zero use of commands, model updates or measurements that were unavailable at prediction time; unsupported regimes and configuration changes withdraw the affected qualification.
- A response mismatch alone never names a root cause or authorizes maintenance, mode transition, restriction clearance or actuation.
- Zero imputed/reconstructed-to-measured promotion.
- Zero target-label, future-data, temporal-window or intervention/censoring leakage.
- Zero false-safe restoration or mode-transition claims from partial evidence.
- Zero unauthorized maintenance/work-order/ODD/actuation effects.
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

The diagnostic baseline registry now contains **17 unique IDs and 17 unique source DOIs**. The update adds the October 2026 Weichai WP4 physical test-bench digital-twin study and the September 2026 Wärtsilä 6L20 few-shot combustion-diagnosis study. Each remains a separate protocol; neither headline result is ranked directly against Aalto, LiU-ICE or another study without protocol alignment.

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
