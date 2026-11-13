# Membrane Evidence Twin — Reference Architecture v0.1

**Status:** architecture blueprint for staged implementation; no live digital twin or plant integration is claimed.

## Design thesis

Do not build a single model that emits one fouling probability and then drives a cleaning action. Build an evidence-preserving process intelligence system that maintains separate, versioned state estimates and never substitutes one evidence class for another.

Every output is an evidence object: claim, scope, time horizon, process/site/module, source observation IDs, calibration/normalization provenance, model/config revision, uncertainty, domain status, alternatives considered and reason for abstention.

## State vector

Return at least six orthogonal outputs:

| State | Question | Required evidence | Forbidden shortcut |
|---|---|---|---|
| Process state | What is the hydraulic/transport process doing? | Synchronized, unit-qualified flow/pressure/flux/temperature/chemistry as available | Missing sensor treated as normal |
| Measurement state | Can the instruments and historian be trusted? | Calibration, freshness, timestamp, uncertainty, missingness, cross-channel checks | A sensor fault called fouling |
| Transport state | Are inferred NDP/permeance/CP quantities valid in this operating domain? | Measured correction inputs, versioned formulas and uncertainty | Raw flux/TMP renamed normalized permeability |
| Mechanism state | Which foulant mechanism is supported? | Independent chemistry, autopsy, process history or a qualified mechanistic signature | Mechanism inferred from one pressure/flux trend |
| Integrity/quality state | Is the barrier and product-water evidence acceptable? | Direct/indirect integrity evidence and actual lab/online quality channels with defined scope | Hydraulic performance treated as proof of potability/integrity |
| Intervention value | Would a cleaning/mitigation action improve service and lifecycle outcomes? | Causal/counterfactual evidence with confounding, overlap, treatment-version and cost accounting | Post-CIP pressure drop treated as causal benefit |

Never collapse these into one score. Keep a Pareto view across service, integrity, energy, water, chemicals, membrane life and cost.

## Layered architecture

1. **Source/measurement plane.** Immutable raw archives, source rights, dataset versions, instrument IDs, calibration, timebase, units and observation aggregation semantics. Uses the observation envelope; preserve original bytes and digests.
2. **Data integrity plane.** Detect missing/stale/stuck/bias/timestamp/historian anomalies. Output suspect channels, coverage and uncertainty; do not infer process or fouling state when evidence is insufficient.
3. **Normalization/transport plane.** Versioned physical equations and reference-condition corrections. Keep pointwise transport arithmetic separate from grouped pressure-series fitting. Expose every correction input and its provenance.
4. **Process state estimator.** Mechanism-aligned statistical/ML models trained only on admissible populations: GRU for sufficiently dense UF sequences, regularized XGBoost for multicollinear NF process records, CNN-GRU or graph models for heterogeneous RO signals only when sample size and topology justify them. Baselines always include persistence, operator thresholds, EWMA/CUSUM and mechanistic models.
5. **Mechanism evidence plane.** Multi-hypothesis diagnosis with an explicit INDETERMINATE state. Require orthogonal evidence before naming organic, biological, scaling, colloidal or pore-blocking mechanisms.
6. **Integrity/quality plane.** Separate online indirect integrity signals, periodic direct integrity tests and actual laboratory results. Lab evidence is sample-time/location/analyte/method only; no interpolation to unobserved times.
7. **Counterfactual decision plane.** Emulate the cleaning/mitigation decision as a target trial where possible. Estimate policy value only within treatment support and under defensible confounding assumptions. Include full lifecycle costs.
8. **Human supervisory plane.** Operators and existing plant interlocks retain authority. Model outputs may recommend Monitor, Investigate, Mitigate, Prepare CIP or Request Review; no autonomous CIP/chemical/pressure/valve/bypass/release command is authorized.

## Evidence-carrying receipt

Each evaluation/decision receipt should bind:
- immutable input archive/file digests and licence/attribution record;
- schema and ontology versions; source observation IDs;
- sensor/calibration/timebase/aggregation and unit metadata;
- formula/normalization versions and input lineage;
- model code/config/weights, random seeds, exact revision and dependency lock;
- chronological/site/module split manifest and data cutoff;
- uncertainty, calibration, missingness and domain-shift results;
- matched baseline results and negative-control outcomes;
- state outputs, abstentions, human review and any subsequent action/outcome.

An unknown action outcome must not be represented as success. Reconcile before new mutation or downstream causal update.

## Evaluation and SOTA comparison

Use separate scorecards:
- Forecast: MAE/RMSE/MASE per horizon, prediction-interval coverage/width and worst-regime performance.
- Detection: LOD, alarm lead-time distribution, false-alarm rate per sensor-day/module-week, missed events and stability under temperature/salinity shifts.
- Measurement integrity: fault-family precision/recall, stale/missing exposure, false sensor-fault alerts, timestamp integrity and correct abstention.
- Mechanism: accuracy only where independent mechanism labels exist; otherwise report indeterminate coverage and false mechanism claims.
- Integrity/quality: method-specific tests and actual sample or online monitoring windows; never surrogate these with flux accuracy.
- Decision: policy value/regret, causal uncertainty, CIP frequency, recovery retention, conforming permeate, energy/chemicals/waste/downtime and lifecycle cost.

Compare against supplier normalization tools and operating thresholds, not just other ML papers. DuPont FTNORM-style reference-condition normalization is a credible RO/NF baseline; it does not replace measurement-provenance, integrity/quality or causal policy evaluation.

## Maturity ladder

- L0: metadata catalogued.
- L1: synthetic contract/arithmetic tests and independent receipts.
- L2: authorized raw source bytes acquired, digests/inventory and label/column/unit audit.
- L3: held-out chronological/site/module retrospective validation.
- L4: prospective shadow operation with no actuation.
- L5: operator-approved advisory study with predeclared safety and service criteria.
- L6: controlled intervention trial with independently measured outcomes.

Current work is L1. The Bristol archives are not acquired in this environment; the Morocco industrial SCADA is request-only; the Armstrong OSF workbook still requires file-level rights and byte verification. No real plant performance or operational superiority is claimed.

## Why this could change the field

The proposed contribution is not a larger neural architecture. It is a common benchmark and runtime contract that makes it impossible to silently confuse sensor reliability, process change, membrane mechanism, integrity/quality and intervention value. That directly addresses recurring gaps in 2026 reviews: data comparability, measurement drift/missingness, temporal/external validation, uncertainty, deployment readiness and the disconnect between monitoring and safe operational decisions.