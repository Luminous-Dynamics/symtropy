# Maritime machinery fault observability and diagnosability benchmark v0.1

**Programme:** Symtropy / Luminous Dynamics  
**Owner:** MARITIME-OBSERVABILITY-001 #4609  
**Status:** research contract; no validated vessel instrumentation map or shipboard diagnostic qualification  
**Version:** 0.1.0  
**Date:** 10 November 2026

## 1. Objective

Qualify what the installed measurement architecture can actually detect and distinguish before scoring a diagnostic model.

The benchmark asks:

> For this exact asset configuration and operating regime, which declared physical faults are detectable, which are isolable from one another, which can only be reported as an ambiguity class, and which are unobservable with the available evidence?

The output is a configuration-bound observability map, not a scalar machinery-health score. It must preserve unsupported, unknown and ambiguous states. A high classifier score on the channels available to a model does not prove that the physical asset is adequately instrumented or that every important failure mode is diagnosable.

This is a maritime application of reusable Symtropy observability and sensor-design work. It must reuse the general mechanisms in #3993, #4006 and #4026 rather than fork their algorithms.

## 2. Programme boundaries and composition

| Owner | Responsibility | This benchmark consumes or supplies |
|---|---|---|
| PR #4444 | Source/configuration admission and capability-impact handoff | Supplies asset/configuration identity and receives bounded fault/observability evidence |
| #4510 | Cross-load, cross-engine transfer and continual adaptation | Supplies domain-qualification and transfer results |
| #4535 | Missingness, sensor integrity and reconstruction | Supplies sensor quality, missingness and lineage states |
| #4545 | RUL and maintenance decision utility | May consume observability gaps as explicit forecast limitations |
| #4548 | Narrative causal risk | Supplies source-grounded fault hypotheses; does not substitute for measured signatures |
| #3993 / #4006 | General observability and sensor-layout methods | Reusable fault-pair, layout and active-diagnosis foundations |
| #4026 | Coupled-fault interaction diagnosability | Reusable multi-fault semantics and tests |
| #3893 / Symthaea #11185 | Vessel ODD and mode authority | Retains all vessel-level disposition and mode authority |
| Mycelix #5974 | Provenance and accountable evidence handoffs | Carries exact artifacts, issuer scope and revocation evidence |

The observability evaluator is read-only. It does not issue work orders, authorize inspections, change engine settings, command actuators, clear restrictions, restore capabilities or change vessel modes.

## 3. Why this changes the optimization target

Conventional diagnostic comparisons often start after a dataset has been assembled. That hides the prior question of whether the sensors, configuration, clocking, calibration and dependency structure make a fault observable at all.

This creates several failure modes:

- Two fault families produce indistinguishable responses in the available channels. A classifier still emits one label, but the physical evidence supports only an ambiguity set.
- Several channels share a power supply, reference, impulse line, gateway, clock or transformation pipeline. Counting them as independent corroboration overstates evidence.
- A derived channel is treated as an independent sensor even though it is computed from the same input signals.
- A sensor is absent from the installed configuration but a model's expected feature vector silently fills it with a nominal value.
- Calibration, units or equipment configuration change, but a diagnosis retains the previous qualification.
- A new load or transient regime lies outside the validated envelope, but the model transfers a diagnosis without detecting the domain gap.

The central innovation is therefore **diagnosability-first engineering intelligence**:

1. Establish the installed sensor and dependency topology.
2. Establish which fault signatures can be distinguished in each supported regime.
3. Quantify the effect of sensor loss, common-cause failure and configuration change.
4. Train and compare diagnostic models only inside the resulting qualified envelope.
5. Return ambiguity or abstention where the measurement architecture cannot support an isolated conclusion.
6. Identify the least-burdened additional observation or approved inspection that would resolve the ambiguity, without autonomously acting on machinery.

## 4. Typed evidence and observability states

### 4.1 Sensor and channel identity

Every observation channel is bound to:

- physical sensor/transducer identity and installed asset/configuration revision;
- measurement principle and physical location;
- measured variable, unit, range and declared sign convention;
- calibration certificate/status, calibration date and validity interval;
- sample rate, timestamp source, clock domain and synchronization status;
- acquisition path, gateway and software transformation versions;
- common-cause dependency groups, such as shared power, reference, pressure impulse line, network, time source or processing pipeline;
- provenance role: measured, derived, reconstructed or simulated;
- current quality state and exact upstream lineage.

A channel name is not a sensor identity. Two channels are not independent merely because they have distinct names or device identifiers.

### 4.2 Signature provenance

Each fault-response signature must be labelled as one of:

- MEASURED — acquired in a declared physical test or service context;
- DERIVED — calculated from named source channels and a versioned transformation;
- SIMULATED — generated by a declared model and parameter set;
- EXPERT_AUTHORED — encoded from a cited engineering source or reviewed expert assessment.

A derived or reconstructed signature retains its original parentage. It cannot independently corroborate the measurements from which it was produced. Simulation and expert-authored signatures can guide test design and support model-based analysis, but do not become physical validation by relabelling.

### 4.3 Per-fault state vocabulary

For each fault, installed configuration and operating regime, report one of:

| State | Meaning |
|---|---|
| OBSERVABLE_AND_ISOLABLE | The available qualified evidence supports detection and separation from all declared alternatives in the stated scope |
| DETECTABLE_NOT_ISOLABLE | Abnormality is distinguishable from healthy operation, but one or more fault causes remain indistinguishable |
| AMBIGUITY_CLASS | Only a declared group of fault hypotheses can be distinguished; no member can be uniquely selected |
| UNOBSERVABLE | The installed measurement architecture cannot establish the fault in the declared scope |
| UNSUPPORTED_REGIME | The operating regime is outside the qualified domain |
| EVIDENCE_CONFLICT | Relevant sources, calibration or configuration evidence conflict |
| NOT_EVALUATED | Evidence or method is insufficient to make the assessment |

Do not collapse the last five states into "healthy", "no fault" or a low-risk score. The terms are evidence states, not vessel operating dispositions.

### 4.4 Pairwise isolation

For every declared fault pair, record whether the signatures are:

- ISOLABLE under the declared configuration/regime;
- NON_ISOLABLE within the available measurement resolution and uncertainty;
- NOT_EVALUATED because the evidence is absent or unsupported.

Use complete-run or physical-test-group holdouts for empirical estimates. Where statistical separation is used, record the feature transform, covariance/uncertainty model, operating regime, calibration set and confidence interval. A structural model can prove that a sensor path is absent or that signatures are structurally identical under stated assumptions; it cannot replace measurement validation of real noise, drift and operating variability.

## 5. Core evaluation design

### Track A — structural observability coverage

Construct a typed graph linking:

asset configuration → physical component → fault hypothesis → expected physical effect → measurement principle → installed sensor → acquired channel → source/quality evidence → diagnostic signature → affected engineering capability.

For each declared fault × regime pair, require a traceable path to one or more qualified measurements. Explicitly record missing edges and unsupported assumptions.

Structural coverage is a claim about the declared model and installed configuration. Keep it separate from empirical detection performance.

### Track B — empirical detection and isolation

Use frozen source/run splits and whole-load holdouts where supported. Compare:

- physics/rule-based residuals;
- simple statistical and signal-processing baselines;
- conventional supervised classifiers;
- domain-adaptation and digital-twin-assisted baselines;
- selective classifiers with calibrated abstention;
- model ensembles only when their evidence dependencies are genuinely distinct.

Report performance per fault, fault pair, load, operating regime, asset and source type. Never use random adjacent windows from the same physical recording across training and test.

### Track C — sensor loss and common-cause resilience

Create controlled removals and mutations for:

- individual sensor/channel outage;
- shared electrical power or reference loss;
- shared impulse line or manifold failure;
- shared gateway/network loss;
- clock/timebase or synchronization failure;
- common preprocessing/model transformation failure;
- calibration expiry, bias, drift, stuck-at, clipping and unit mismatch.

Recompute the observability map after each mutation. The report must show which fault pairs become non-isolable or unobservable and invalidate any affected prior qualification.

### Track D — coupled physical and sensor faults

Test simultaneous physical faults and sensor faults. Use #4026 semantics. Do not infer a combined response by adding singleton response vectors unless a physical superposition model has been justified and validated for that regime.

### Track E — sensor-layout Pareto optimization

Compare candidate layouts across multiple non-substitutable dimensions:

- diagnosability coverage and worst safety-relevant fault-pair isolation;
- lifecycle instrumentation cost, including installation, calibration, replacement and inspection;
- sensor availability and maintainability;
- physical integration and environmental suitability;
- common-cause and cyber/dependency exposure;
- bandwidth, storage, compute and edge latency.

Return a Pareto frontier and explain the trade-offs. Do not compress everything into one opaque weighted score. Any decision preference or weighting belongs to an identified engineering owner.

### Track F — active diagnosis and information gathering

When the current evidence supports multiple hypotheses, rank passive or separately approved information-gathering options using expected value of sample information, information gain, time, cost, risk and operational constraints.

Examples include requesting a longer synchronized observation, comparing an independent measurement principle, obtaining an existing OEM diagnostic record, collecting a sample, or scheduling an engineer inspection. Any physical test, control adjustment or machinery intervention requires its own approved procedure and authority. A software recommendation is not permission to perform it.

## 6. Metrics

Every metric must be stratified by fault, fault pair, regime, configuration, asset and source evidence role where sample size allows.

| Dimension | Measures |
|---|---|
| Structural coverage | Traceable fault-to-measurement path; declared fault × regime coverage; unobservable count |
| Detection | Event recall, false alarms per engine-hour, detection latency, worst-fault recall |
| Isolation | Pairwise isolation accuracy, false-isolation rate, ambiguity-class precision/recall |
| Selective prediction | Risk-versus-coverage curve, abstention precision, false-known/OOD rate |
| Calibration | Brier/log score where probabilistic outputs are supported; reliability diagrams and grouped intervals |
| Resilience | Loss of diagnosability under individual sensor and common-cause group failures |
| Independence | Number of independent measurement principles and dependency groups, not raw channel count |
| Layout | Pareto frontier for lifecycle cost, calibration burden, availability and worst-case diagnosability |
| Active diagnosis | Information gain, expected value of sample information, observations/inspections to resolve ambiguity |
| Runtime | Compute, memory, communication volume, edge latency and update/requalification cost |

For time-series data, calculate exposure denominators from valid observation time, not row count. Exclude gaps and unusable periods according to a predeclared policy. Group uncertainty by independent physical asset, campaign and run; windows and rows are not independent engines.

No weighted aggregate may compensate for a false-healthy, false-independent, false-isolated or authority-boundary violation.

## 7. Frozen falsification controls

1. Two fault hypotheses have identical or overlapping signatures: the evaluator returns an ambiguity class, not a fabricated single cause.
2. A required sensor is absent from the installed configuration: a synthetic feature cannot fill the gap as measured evidence.
3. Two channels share a declared common-cause dependency: they cannot count as independent corroboration under that failure.
4. A derived channel is counted as independent evidence from its own measured inputs.
5. A sensor is stuck at a plausible healthy value while a physical fault is present.
6. A sensor is stale, uncalibrated, unit-mismatched or bound to the wrong configuration revision.
7. A common-cause failure invalidates an earlier observability claim, but the prior qualification is silently reused.
8. The system is evaluated outside its qualified load/speed/transient domain and reports a confident known fault.
9. Two simultaneous physical faults have a non-additive interaction, but their joint signature is fabricated from singleton signatures.
10. A simulator-only signature is presented as a measured physical result.
11. An LLM/classifier emits a fault without a source-bound measurement/signature path.
12. A diagnostic recommendation silently becomes an inspection order, maintenance authorization, restriction release or ODD/mode transition.

Hard-gate failures are reported individually and are never averaged away.

## 8. Proposed research targets

These are aspirations for experiments, not measured results, safety thresholds or operational limits.

- 100% correct treatment of planted unobservable and indistinguishable cases in the frozen synthetic contract suite.
- Zero false-healthy admissions from absent, stale, failed, uncalibrated or wrong-configuration sensors in the negative-control suite.
- Zero false-independence admissions from derived channels and declared common-cause dependencies.
- Zero isolated-cause claims when signatures are indistinguishable within the declared uncertainty.
- At matched fault coverage, at least 15% lower lifecycle instrumentation burden than a documented baseline layout, with all cost assumptions exposed.
- In approved active-diagnosis scenarios, at least 20% fewer observations/inspection steps to resolve the same ambiguity class than a matched fixed-checklist baseline, without increasing diagnostic risk.
- No regression in worst-regime event recall or calibration after sensor loss or domain shift.

Do not promote these internal targets to OEM limits, class requirements, safe operating thresholds or claims of superiority before reproducing a matched baseline.

## 9. Dataset plan and claim ceilings

### Aalto Marine Engine Fault dataset

The Aalto MU323DGSC single-engine test-bench corpus can support measured signal-response and held-out-run/load experiments for its declared induced faults. It cannot establish cross-engine generalization, fleet-level observability or vessel-level sensor-layout adequacy.

The archive's channel dictionary must be reconciled against actual transducer identity, measurement principle, calibration and derived-channel lineage. If those records are not available, report physical sensor independence and layout-level coverage as NOT_EVALUATED. Do not infer physical sensor count or independence from approximately 70 channel columns.

### Additional physical data required

A partner dataset should provide:

- physical engine/vessel and installed configuration identity;
- sensor/instrument register and measurement principles;
- wiring, power, reference, gateway and clock/acquisition topology;
- units, ranges, calibration records and validity intervals;
- load/speed/ambient/transient regime context;
- fault-injection or independently adjudicated inspection records;
- maintenance/intervention history and data rights;
- enough independent engines and campaigns for the intended transfer claim.

Report synthetic, simulated, bench and shipboard results in separate evidence strata.

## 10. Industry comparison and intended differentiation

| Existing capability | Publicly described scope | Our intended complementary qualification |
|---|---|---|
| Wärtsilä Expert Insight | Vessel-data anomaly detection using AI and rule-based diagnostics with Wärtsilä expert analysis and crew collaboration | Vendor-neutral sensor/dependency graph and explicit detectability/isolability limits across mixed equipment |
| Everllence PrimeServ Assist / CEON | Connected engine telemetry, AI/algorithms and continuous expert monitoring; Bureau Veritas issued SMART(MH3) Smartship Software Type Approval in March 2026 | Preserve the OEM result as an evidence input and independently assess source/configuration/calibration and cross-vendor evidence consistency |
| DNV Machinery Maintenance Connect | Machinery maintenance survey workflows and fleet benchmarks; DNV describes at least one year of maintenance history for an initial digital MPMS survey | Make asset/sensor observability and evidence quality portable across maintenance systems, not dependent on a single PMS history |
| General sensor-placement and fault-diagnosis research | Structural observability, fault-signature matrices, Pareto sensor selection and coupled-fault analysis | Reuse those foundations and apply them to real maritime component/function maps, OEM data and vessel configuration revisions |

These providers have substantial operational products and domain expertise. We do not currently outperform them in deployed diagnostic coverage, fleet validation, expert network, class acceptance or customer evidence. The credible differentiator is an independent, portable qualification layer: it can explain not only what a diagnostic service reports, but what the installed evidence can and cannot prove, under which configuration and common-cause assumptions.

The competitive benchmark must be matched on actual data access, engines, faults, operating regimes, intervention budgets, false-alarm exposure and human review—not vendor marketing claims or unrelated paper accuracy.

## 11. Delivery sequence

1. **Contract:** approve the fault/signature vocabulary and strict report schema; define the first engine component and its configuration.
2. **Structural model:** create one reviewed component → fault → measurement-principle → sensor → channel → capability map.
3. **Synthetic falsification:** add ambiguity, missing sensor, shared power/clock, derived-channel and configuration-revision cases.
4. **Aalto measured subset:** after the archive payload audit, evaluate only claims supported by the physical test-bench evidence; do not claim sensor-layout independence without the instrument register.
5. **Independent physical review:** have a marine engineer review the topology, signatures, fault pairs and ambiguity semantics.
6. **Partner validation:** add a second physical engine and vessel instrumentation map before cross-engine/vessel claims.
7. **Decision integration:** supply read-only outputs to #4444 and the vessel ODD/mode owner; keep all operational authority outside this evaluator.

## 12. References

- ISO, [ISO 13379-1:2025 — Condition monitoring and diagnostics of machine systems: Data interpretation and diagnostics techniques, Part 1: General guidelines](https://www.iso.org/standard/88027.html), published October 2025.
- ISO, [ISO 13374-1:2003 — Condition monitoring and diagnostics of machines: Data processing, communication and presentation, Part 1: General guidelines](https://www.iso.org/standard/21832.html), reviewed and confirmed in 2025.
- Daigle, M., Roychoudhury, I. & Bregon, A., [Diagnosability-Based Sensor Placement through Structural Model Decomposition](https://ntrs.nasa.gov/citations/20190001644), NASA NTRS.
- Fernandes, A. A. & Kimball, J. W., [Structural Analysis of Multi-Domain Dynamic Systems Modeled Via Bond Graphs](https://doi.org/10.1109/ACCESS.2025.3638672), IEEE Access, 2025.
- Wärtsilä, [Expert Insight service](https://www.wartsila.com/marine/services/lifecycle-agreements/expert-insight-service).
- Everllence, [PrimeServ Assist](https://www.everllence.com/services/offerings/marine-power/digital-solutions/primeserv-assist/).
- Bureau Veritas, [PrimeServ Assist SMART(MH3) type approval announcement, 17 March 2026](https://marine-offshore.bureauveritas.com/newsroom/bureau-veritas-certifies-everllences-digital-monitoring-solution-primeserv-assist).
- DNV, [Machinery Maintenance Connect](https://www.dnv.com/services/machinery-maintenance-connect-mmc--168870/).

---
A passing report qualifies only the exact asset/configuration, fault set, operating domain, measurement architecture, evidence, split, model and evaluator. It does not establish seaworthiness, class/flag approval, safe remaining life, maintenance authority, ODD clearance or permission to operate.


## Temporal qualification and common timebase

Clock-domain labels alone do not establish event-time comparability. Every channel record now carries a common `timebase_id`, a non-negative `timestamp_uncertainty_ms` bound (or null when unknown), the source of that bound, explicit timebase qualification state and a SHA-256 qualification artifact. The local `clock_domain` remains separately identified.

A sensor with unknown/unqualified timebase, missing uncertainty, or no qualification digest cannot supply physically time-aligned evidence for a cross-channel isolability claim. Matching clock strings are not proof of synchronization. Qualification must include measured offset/drift and timestamp-path uncertainty over the stated validity interval; a PTP profile or vendor specification alone is not an installed-system measurement.

Relevant current references: IEEE 1588 defines PTP for heterogeneous networked measurement/control clocks; a 2026 IEEE industrial-facilities review compares IRIG-B, SNTP and PTP; ITU-T G.8275 (01/2026) specifies common aspects of PTP timing profiles. These inform clock architecture but do not qualify a vessel installation.
- IEEE 1588: https://standards.ieee.org/ieee/1588/6825/
- IEEE industrial timing review (2026): https://doi.org/10.1109/MIAS.2026.3682180
- ITU-T G.8275 (01/2026): https://www.itu.int/epublications/publication/itu-t-g-8275-2026-01-common-aspects-of-ptp-profiles-for-phase-time-synchronization

## Evidence-admission preflight

The maritime evidence validator performs identity, reference, derivation-DAG, calibration, source-role, common-timebase and authority-boundary checks before any downstream pair-separation algorithm is called. It emits pair-specific evidence eligibility; it does not compute pairwise separability or reinterpret the declared pair state.

- `MEASURED_INPUTS_ELIGIBLE_FOR_INDEPENDENT_REVIEW`: all referenced channels are installed, directly measured, current, calibrated and timebase-qualified.
- `NOT_PHYSICALLY_ELIGIBLE`: at least one evidence reference is derived, simulated, uninstalled, stale, uncalibrated, timebase-unknown or otherwise unsupported.

Declared common dependencies and measurement principles are surfaced for independent review; the adapter does not assert independence from identifiers alone. Derived-channel lineage must resolve to known channels and form an acyclic graph. The synthetic six-channel fixture is fabricated software-conformance data only.


### Measured support for declared isolability

The evidence-admission receipt now audits a report's declared isolability state without recomputing physical separability. A pair declared isolable is labelled ELIGIBLE_FOR_INDEPENDENT_REVIEW_ONLY only when its referenced inputs are all eligible measured channels, share exactly one qualified timebase, use at least two declared measurement principles, and contain no shared dependency group among the referenced channels.

If these conditions fail, the receipt labels the declared isolability as unsupported by the referenced measured evidence. Non-isolable and not-evaluated pair states do not create an isolability claim. Every pair audit and the top-level receipt explicitly record physical_pair_separability_recomputed=false. This prevents an evidence-admission pass from being mistaken for a fault-separation proof.
