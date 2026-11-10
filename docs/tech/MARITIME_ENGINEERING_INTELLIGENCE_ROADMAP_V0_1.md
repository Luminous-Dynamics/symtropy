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

## Benchmark matrix

| Benchmark family | Primary metrics | Critical holdout / falsification |
|---|---|---|
| Fault detection and diagnosis | Macro-F1, balanced accuracy, event recall, false alarms per engine-hour, duration, detection latency, worst-fault recall | Entire runs, loads, engines and vessels; no overlapping-window leakage |
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

The diagnostic baseline registry now contains **14 unique IDs and 14 unique source DOIs** after removal of a duplicate alias for the 2025 18-task Applied Energy paper and addition of a separate 2025 dual-transfer publication.

- The 2025 Applied Energy cross-working-condition paper reports **95% accuracy** in its abstract and describes **18 transfer tasks** across two engine case studies (8V396SE, 500 kW, and 620 V16, 2,336 kW). Its evidence mix includes experimental bench lubrication-system data and virtual intake/exhaust task data. Keep each subsystem and source type separate. The exact per-task aggregation, sample grouping and split manifest must be recovered before reproduction. This is not a common-protocol or physical fleet-transfer score.
- The 2025 Energy mean-value-model-assisted dual-transfer paper reports **more than 97.70% average accuracy** for cross-domain lubricating-oil diagnosis. It describes a simulation-to-measured transfer between a 16V396 simulation domain and an 8V396 experimental bench domain, nine transfer tasks, and 70%, 85% and 100% load points. The exact source/target assignments, run grouping, task-level scores and leakage controls need methods/table audit. It is a separate paper from the 18-task Applied Energy study and is not comparable to the Aalto MU323DGSC data.
- A 2026 adaptive multi-source study on an 8V396 testbed reports 83.64% average accuracy on pure-real measured scenarios and 82.37% on real/simulation hybrid scenarios. Do not compare this average directly with either 2025 result without aligning task taxonomy, target exposure, class balance and held-out physical groups.

### Consequence for benchmarking

The reported 95%, >97.70%, 83.64% and hybrid/simulation headline scores are **separate reproduction targets, not a single ranking**. Each study has its own task definition, engine, evidence domain, split semantics and metrics. A fair comparison needs two stages:

1. Reproduce each paper under its own documented protocol, with a frozen environment, raw-data/license audit, exact split manifest, source code commit and per-task table.
2. Re-run selected methods in a shared evaluation harness with whole-recording/run separation, load-held-out tests, source-domain holdouts, no target-label leakage, calibration and OOD controls. Report per-task and worst-domain performance, event recall at fixed false-alarm rate, false alarms per engine-hour, latency, uncertainty/calibration, compute/latency and prior-domain forgetting.

Do not treat random-window or random-row splits as evidence of cross-run or cross-engine generalization. Do not silently merge measured, simulated and hybrid source domains. A high headline accuracy cannot compensate for a hard-gate failure in provenance, split leakage, unknown-state handling or false-alarm burden.

