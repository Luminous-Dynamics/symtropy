# WATER-MATCHED-EXPERIMENT-001 — Paired Evaluation Fairness Protocol v0.1

**Tracking:** [#5075](https://github.com/Luminous-Dynamics/symtropy/issues/5075)  
**Composes:** pump control #5018, leak diagnostics #4963, split guard #5053, service floors #5027, observability #5045, recovery #5069, sensor placement #4006.

## Purpose

This contract prevents a baseline from appearing superior because it saw more information, received easier operating conditions, had more permissive actuator authority, used a weaker service floor or was scored over a shorter period.

The central comparison unit is a **paired trial**. Every baseline within the pair receives exactly the same deployment information and decision envelope. Development costs are recorded separately.

## Deployment parity vs development cost

### Strict deployment-information parity

Within every pair, all baseline runs must receive an identical deployment-input object binding:
- scenario and network identity;
- topology, geometry and source digests;
- initial conditions, demand trace, disturbance/fault trace and seed;
- sensor set, sampling period, time grid, measurement missingness and units;
- service-floor profile and required service-area set;
- actuator envelope, independent local-protection contract and decision deadline;
- scoring horizon and explicit future-covariate availability.

Different deployment inputs mean a different trial stratum, not a valid paired comparison.

### Development-resource disclosure

Development costs may differ because architectures genuinely require different training or synthesis methods. Report each baseline independently:
- training samples and source population;
- split receipt;
- offline compute and peak memory;
- hyperparameter/validation trial count;
- engineering hours when available;
- online P95 CPU latency, memory and communication bytes per decision;
- instrument and lifecycle costs at the separate sensor-placement/resource layer.

Do not hide these differences. Do not give an algorithm future demand, extra sensor channels, or a different actuator envelope and then describe it as a fair paired run.

## Baseline families

| Task | Required comparators |
|---|---|
| Pump control | Tuned rule/PI, calibrated hydraulic MPC, DeePC/data-driven MPC, 2026 ENC, qualified estimator + controller |
| Leak diagnostics | Water-balance/minimum-night-flow, fixed threshold, RF/XGBoost/LightGBM, LSTM/Transformer/GNN, event-level alarm policies |
| Community service | Weighted mean as diagnostic negative control, independent hard floors, max-min/lexicographic constrained policy, supplementary CVaR |
| Observability | Structural placement, continuous sigmoid residual score, probabilistic/DRM placement, current installed layout |
| Restoration | Operator/work-order assertions, alarm-clearance logic, telemetry-only candidate recovery, independent evidence-gated restoration |

Commercial products should be evaluated against documented capabilities first. A product feature list is not a measured performance result. Numeric comparison requires a rights-authorized, equivalent data/demo environment with identical pair inputs and declared integrations.

## Scoring protocol

Do not collapse results into one scalar. Report separate scorecard lanes:

1. **Service:** delivered volume, pressure/storage floor degree-time, worst required area, duration below floor, recovery time.
2. **Event:** event recall/misses, false alarms per nominal exposure, onset delay, localization error/Top-k, sizing calibration.
3. **Uncertainty:** interval coverage/width, calibration, OOD detection and abstention.
4. **Resilience:** performance under failed sensors, common-cause domains, topology changes, power/communications loss, recurrence.
5. **Resources:** energy per delivered service, P50/P95/worst-case latency, data/compute/memory budgets, communication and lifecycle cost.
6. **Evidence integrity:** rights/source lineage, calibration, geometry/time coverage, independent verification and known unknowns.

Protected service-floor breaches, false restoration and unsafe action proposals are non-compensatory gates. A lower energy or runtime score cannot erase them.

## Statistical discipline

- Use identical paired scenario seeds and disturbance realizations.
- Use network-, event-lineage- and time-held-out populations.
- Fit thresholds and calibrators on train/validation only; freeze before final holdout.
- Consume #5053/#4979 split receipts for leak populations; this contract does not replace their split-integrity controls.
- Report paired deltas with uncertainty intervals clustered by network/incident. Do not treat adjacent time samples as independent replicates.
- Show per-network and cross-network transfer separately.
- Publish coverage and abstention next to risk/accuracy; never omit hard cases post hoc.
- Keep synthetic, laboratory/HIL, retrospective utility and prospective field results in separate populations.

## Proposed targets

Targets, not achieved:
- 100% exact parity of the shared deployment-input object for every admitted pair.
- Zero accepted pairs with mismatched sensor sets, demand/fault traces, topology, time grids, service floors, actuator limits or scoring horizons.
- 100% configuration, implementation, runtime and development-budget disclosure.
- 100% paired reporting of abstention and eligible coverage alongside risk/accuracy.
- Zero numeric commercial superiority claims derived only from vendor feature descriptions.
- Any superiority claim must have a paired uncertainty interval on independent network/event holdouts and no hard-gate regressions.

## Frozen T0 challenge

The companion corpus contains 10 synthetic cases:
- one exact deployment-information match;
- sensor-set, sampling, disturbance, service-floor, actuator-envelope and horizon mismatches;
- future-covariate leakage;
- a single-baseline non-pair;
- missing baseline configuration provenance.

The schema/fixture/oracle validate **pair admission only**. They do not contain algorithm performance values or rank a vendor.

## Claim ceiling

This contract can qualify only paired-input equivalence, baseline identity/configuration and resource disclosure. It does not measure controller quality, leak accuracy, community service, restoration, safety, commercial competitiveness or physical performance.
