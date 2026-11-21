# Maritime diagnostic explanation evidence benchmark v0.1

**Programme:** MARITIME-DIAGNOSTIC-EXPLANATION-001 (#4906)  
**Status:** P0 contract, schemas and synthetic known-truth corpus; no trained model, SHAP, IG or LIME output is qualified  
**Version:** 0.1.0  
**Date:** 12 November 2026

## 1. Objective

Evaluate whether a diagnostic explanation is faithful to the model, stable under meaning-preserving changes, responsive to evidence withdrawal, physically grounded within declared scope, and honest about causality and ambiguity.

An explanation is not evidence merely because it is plausible prose, a SHAP bar chart or an attention heat map. A model attribution describes dependence of a model output on its inputs under an explainer/background distribution. It does not, by itself, establish a physical mechanism or causal root cause.

## 2. Research basis

- Tang et al. (25 August 2026), *Sensors* 26(17), 5379, [Explainable Artificial Intelligence in Rotating Machinery Fault Diagnosis: A Comprehensive Review and Emerging Trends](https://doi.org/10.3390/s26175379). The review identifies fidelity, stability, robustness, localization and physical consistency as quantitative explanation criteria. It distinguishes model-relative attribution from causal root-cause analysis and highlights sensor degradation, EMI, variable conditions and scarce fault data as deployment gaps.
- Altosole et al. (22 October 2026), *Journal of Marine Science and Engineering* 14(21), 1988, [Literature Review Analysis: Fault Diagnostics of Marine Engines](https://doi.org/10.3390/jmse14211988). Its >200-paper review identifies a persistent gap between algorithm development and operational validation, and calls for reusable benchmark datasets with engine/load/mode/sensor/fault/severity/validation metadata and interpretable hybrid methods.
- Park et al. (July 2026), *Computers & Industrial Engineering* 217, 112028, [Explainable AI-based root-cause diagnosis of maritime engine anomalies with external factors](https://doi.org/10.1016/j.cie.2026.112028). This paper combines internal engine measurements and external conditions and uses SHAP/RCAP to characterize predictive association. Its attribution must not be treated as causal proof without a separate causal identification design.
- Shen et al. (online 9 January 2026), [Explainable artificial intelligence in fault detection and diagnosis: a review of methods, applications, and implementation challenges](https://doi.org/10.1080/21681015.2026.2613018). It surveys model-agnostic, model-specific and hybrid rule-based explanation methods; widespread use of feature importance is not itself a measure of explanation quality.

## 3. Paradigm: explanation as a falsifiable evidence claim

Every explanation must keep five layers distinct:

1. **Model attribution:** which input features influence the model output under the declared explainer and background set.
2. **Physical observation:** which installed, calibrated and time-qualified sensors actually measured the underlying signals.
3. **Mechanistic support:** the declared component → fault → physical effect → measurement path and its regime scope.
4. **Causal claim:** what has been identified by a qualified causal design/intervention; SHAP, attention, feature importance and correlation are not causal evidence.
5. **Counterevidence and ambiguity:** competing faults, removed sensors, conflicting measurements, non-identifiable pairs and unsupported regimes.

A derived signal cannot independently corroborate its own parents. A controller command is not evidence of delivered fuel, torque or actuator motion. Simulated, reconstructed and operator-reported evidence retains its source role. Explanations may state model association while declining to assert mechanism or cause.

The report is an evidence graph with exact model, explainer, background set, asset/configuration, operating regime, signal, calibration, clock, source and split identities. Missing artifact digests remain null rather than being fabricated.

## 4. Benchmark tracks

### A. Faithfulness to model behavior
Compare SHAP, Integrated Gradients, LIME, attention/feature weights, perturbation methods, ante-hoc interpretable models, physics residuals and hybrid mechanisms on the same model/task/split.

Report:
- deletion and insertion curves/AUC under valid conditional or physically admissible masking;
- sufficiency and comprehensiveness;
- local fidelity to the model over a preregistered perturbation distribution;
- top-k overlap/rank agreement between repeat explanations and compatible explainers;
- runtime, memory and edge cost.

Do not zero arbitrary channels where this creates an impossible engine state. Mask using explicit missingness, matched windows or a declared conditional generator, and disclose which method is used.

### B. Stability and invariance
Meaning-preserving transforms: channel-order permutation, unit conversion with correct scale/sign, deterministic replay, harmless serialization/preprocessing reordering and uncertainty-bounded measurement perturbation.

Evidence-changing transforms: remove a uniquely supporting independent sensor, expire calibration, alter configuration, introduce clock uncertainty, remove a common gateway or move into an unsupported load/transient regime.

A faithful explanation should remain stable under meaning-preserving changes and change or abstain when its evidence basis is withdrawn.

### C. Physical consistency and localization
Map attribution segments/features to the asset/component/fault/effect/sensor graph. Report component/fault localization precision/recall, agreement with independently reviewed mechanism/sign constraints, contradictions and unsupported mechanism assertions.

Physics constraints are versioned, scoped and reviewed. A synthetic fixture's mechanism link is not physical validation. Directional constraints only apply within declared operating regimes and assumptions.

### D. Counterfactual and causal honesty
Falsify:
- same command / different load context;
- different commands / same physical response;
- sensor bias mimicking actuator under-response;
- true external load/weather influence without an internal engine fault;
- observationally equivalent faults;
- removal of a unique supporting sensor;
- inverse-model multi-solution ambiguity;
- retrospective external features unavailable at prediction time.

Counterfactuals must remain inside the declared physical/operating envelope. An impossible feature vector is not an admissible causal counterfactual.

### E. Evidence lineage and independence
Non-compensatory hard gates:
- no control-intent → physical-response promotion;
- no derived feature → independent-source promotion;
- no simulation/reconstruction → measured-truth promotion;
- no absent/uninstalled, uncalibrated or unqualified signal cited as an observation;
- no post-prediction information;
- no unique root cause when upstream observability says AMBIGUITY_CLASS or NOT_EVALUATED.

### F. Engineer usefulness
Future independently reviewed study: compare engineer diagnosis quality, time-to-review, error detection, appropriate disagreement/escalation, confidence calibration and cognitive load. Synthetic fixtures or preference surveys do not establish operational usefulness.

## 5. Frozen synthetic corpus

The 14-case fixture includes:
- supported measured association with explicit claim ceiling;
- controller command promoted to physical delivery;
- derived feature double counted with its parents;
- simulation output promoted to measured evidence;
- non-isolable fault pair forced into a unique diagnosis;
- ambiguity correctly preserved;
- uninstalled sensor cited as measured;
- post-prediction external context leakage;
- explanation unchanged after supporting evidence is withdrawn;
- SHAP association mislabeled as causal;
- unsupported regime with a detailed unsupported narrative;
- unit-scaling invariance pair;
- uncertainty-bounded attribution-stability case.

All fixture values and mechanism mappings are synthetic. There are no model predictions or explainer outputs from a trained engine model in P0.

## 6. Metrics

Stratify by fault, pair, regime, asset, model, explainer, sensor-availability pattern and evidence class:
- local fidelity, deletion/insertion AUC, sufficiency and comprehensiveness;
- top-k explanation stability and unit/order invariance;
- explanation response after unique-source withdrawal;
- localization precision/recall and physical-constraint violation rate;
- unsupported causal claim rate and false-unique-cause rate;
- evidence lineage completeness and independent principle count;
- OOD abstention quality;
- runtime, memory and explanation artifact reproducibility.

Research aspirations, not achieved results or safety thresholds:
- zero accepted evidence-role promotion or future-information cases;
- zero unique-cause explanations when upstream diagnosability is non-isolable/not evaluated;
- ≥95% top-k retention under preregistered uncertainty-bounded perturbations that preserve the decision and physical state;
- 100% detection of planted evidence-withdrawal failures in the synthetic conformance suite;
- meet a declared edge compute/memory budget without weakening evidence gates.

The 95% stability aspiration applies only to the defined perturbation stratum. It is not a universal trust score.

## 7. P0 acceptance and next implementation

This documentation/schema/corpus increment is P0 contract work. Next implementation:
1. Rust-first strict input/report validator and deterministic receipt.
2. Claim-level evidence-role, timestamp, calibration, dependency and diagnosability admission.
3. Metamorphic runner for unit/order invariance and evidence withdrawal.
4. Golden vectors for SHAP, IG, LIME, attention, rules and hybrid explainers with frozen model/background artifacts.
5. Human review study protocol after the executable metrics are stable.

Compose with Symtropy #4444, #4628/#4629, #4609/#4685, #4510, #4535, #4545 and reusable kernel #4005. No second observability or fault-isolation algorithm is introduced.

## 8. Authority and claim ceiling

The benchmark is read-only. It cannot command controllers, actuators, maintenance, work orders, restriction clearances or vessel modes.

A pass would qualify explanation semantics for the exact model/explainer/background/evidence/configuration/regime/split and evaluator. It does not establish causal truth, unique root cause, shipboard machinery safety, seaworthiness, class approval, maintenance authority, ODD clearance or permission to operate.
