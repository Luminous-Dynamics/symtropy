# WATER-CONTROL-ROBUSTNESS-001 — Sparse-Sensor and Safe-Abstention Benchmark v0.1

**Tracking:** Symtropy [#5018](https://github.com/Luminous-Dynamics/symtropy/issues/5018), child of [#4959 pump-system service](https://github.com/Luminous-Dynamics/symtropy/issues/4959)  
**Parent programme:** [#4962](https://github.com/Luminous-Dynamics/symtropy/pull/4962)  
**Status:** synthetic scenario contract; no control algorithm or physical equipment qualified

## 1. Why this benchmark exists

Villacres, Putri & Moazeni, *Data-driven, model-free control for reliable operation of water distribution systems: Implementation, benchmarking and validation*, *Water Research* 304, 126295 (1 October 2026), DOI [10.1016/j.watres.2026.126295](https://doi.org/10.1016/j.watres.2026.126295), report a model-free Exact Nonlinearity Cancellation (ENC) approach using 200 measured samples. The article reports over 99% less data and 90% less computation than its data-driven MPC comparator, with the operational small-WDS case showing demand satisfaction and pressure/storage bounds.

The most important qualification detail is also explicit in the paper: **all regulated tank levels and junction pressures were directly measured**. The authors identify partial measurements and sparse sensing as the most important next step toward field deployment. The paper's data availability statement says data will be made available on request. Exact reproduction therefore remains blocked until authorized data, methods and executable artifacts are obtained and pinned.

Other relevant comparators:
- Peralez et al., *Data Enabled Predictive Control for Water Distribution Systems Optimization*, Water Resources Research (first published 9 April 2025), DOI [10.1029/2024WR039059](https://doi.org/10.1029/2024WR039059).
- Constrained hydraulic state estimation for water-network digital twins, PubMed record [42800315](https://pubmed.ncbi.nlm.nih.gov/42800315/), which reports constrained state estimation on two real WDS cases (one synthetic-sensor case and one with real sensor measurements) and a reduced-model runtime improvement. Exact paper artifacts and applicability must be reviewed before reproduction.
- Rule-based / tuned PI supervision and calibrated EPANET/WNTR-based MPC remain required low-complexity and physics-model baselines.

The benchmark's research question is therefore: **which control architecture remains service-qualified when direct observability is reduced, sensors degrade, and plant assumptions change—and when should it abstain?**

## 2. Paradigm shift: control is a four-part contract

Separate four capabilities that are often collapsed into "the controller":

1. **Observe / estimate:** what state is directly measured, reconstructed, stale, contradictory or unobservable?
2. **Propose control:** what action does the policy recommend under its own declared model and data envelope?
3. **Independently constrain:** does a separate safety envelope accept the action against physical, equipment and service bounds?
4. **Verify outcome:** did measured flow, pressure, storage and quality actually satisfy the service contract after the action?

A controller must not certify its own state observability, safety, or success. A policy output is a proposal, not an actuator command. This is a research architecture; the present benchmark performs no actuation.

## 3. Baseline families and fair comparison

Freeze five baseline families:

| Baseline | Role | Required disclosure |
|---|---|---|
| Tuned schedule / rule / PI | Practical low-complexity floor | Tuning data, hysteresis, state inputs, fallback |
| Hydraulic-model MPC | Physics-model reference | EPANET/WNTR version, model calibration, uncertainty, solver budget |
| DeePC / data-driven MPC | Data-driven optimization baseline | Hankel/history construction, regularization, sample budget, online solve |
| 2026 ENC / model-free direct-data controller | Recent data-efficient research baseline | Exact method, measured-state assumptions, persistency of excitation, synthesis data and stability conditions |
| Qualified estimator + controller | Sparse-sensor hybrid | Estimator calibration, observability map, interval coverage and model discrepancy |

The 2026 ENC article is a published research comparator, not a Luminous implementation. The reported 200 samples and 90% compute reduction are not local measurements. Do not infer that its controller accepts sparse, conflicting or stale measurements.

### Fairness rules

Every paired run receives the same:
- physical network and initial state;
- demand / disturbance / fault time series;
- sensor locations, modality, sampling, missingness and noise;
- available historical data and training budget;
- actuator availability, command limits, ramp rate and protection envelope;
- service floors, scoring horizon and recovery definition.

Separate a method's required state estimator from the controller. If a method requires all regulated state variables directly measured, that assumption must be visible in its coverage score. Do not compare it with a model-based baseline that receives fewer or noisier measurements.

## 4. Synthetic scenario corpus

The companion schema and JSON pack define 15 cases, from fully observed nominal operation through demand ramps, sparse sensors, dropout, bias, common-mode drift, sensor disagreement, stale timebases, topology mismatch, pump trip, source/suction limitation, power interruption, communications loss and compound common-cause failure.

All pressure/storage bounds are expressly synthetic and non-normative. They are not Johannesburg Water setpoints, South African drinking-water standards or engineering design criteria.

The companion oracle records only the expected **admission disposition**:
- run nominally;
- run only with a separately qualified estimator;
- abstain / invoke a qualified local fallback;
- local safety system only;
- stop and reconcile.

It does not encode controller trajectories or predict hydraulic values.

## 5. Metrics: do not optimize a scalar score

### Service and physical constraints

- delivered volume / required volume at each declared boundary;
- minimum-pressure shortfall degree-minutes and maximum continuous shortfall;
- storage reserve shortfall degree-minutes and minimum reserve;
- maximum pressure / overflow excursions;
- time to recover service after each disturbance;
- pump duty-point, starts/stops, ramp and command chatter.

### Resource efficiency

- electrical kWh at the complete pump-system input boundary;
- delivered m³ at the declared service endpoint;
- wire-to-water energy per delivered volume, with pressure/head and flow included;
- operating hours, maintenance and unplanned downtime;
- lifecycle costs, component replacement, consumables and water losses.

### State estimation / observability

- direct observability coverage by regulated variable and critical network node;
- interval coverage and width for unmeasured states;
- residual calibration and model discrepancy under held-out regimes;
- false-safe rate when sensor values are biased, missing or stale;
- abstention and service-qualified coverage.

### Computational and data efficiency

- training / synthesis sample count and excitation conditions;
- offline synthesis compute and memory;
- online median, P95 and worst-case execution time;
- memory, communication volume, telemetry dependency and restart behavior.

Report all results by scenario and population. MAPE alone is not an acceptable measure for hard pressure or storage bounds. Never average a community/service-floor violation into a positive city-wide score.

## 6. Proposed research gates

These are proposed targets, not achieved results or normative standards.

**Hard gates**
- Zero unqualified supervisory proposals in scenarios with stale, conflicting or unobservable critical state.
- Zero silent continuation after topology/actuator identity mismatch.
- Zero relaxation of declared service/equipment limits to improve energy or runtime.
- A qualified local protection path must be independently assessed; if unavailable, the supervisory controller remains out of the authority path.
- Deterministic replay for frozen data, controller version and scenario seed.
- Report both coverage and abstention; never count unobserved states as correct predictions.

**Comparative targets**
- Reproduce the ENC article before superiority claims; for exact reproduced metrics, aim for ≤5% relative error only where the published metric definition and source artifacts permit a like-for-like comparison.
- On a held-out sparse-sensing population, target ≥10 percentage-point higher *service-qualified scenario coverage* than the strongest matched baseline, with no increase in hard service-floor violations.
- Under nominal matched duty traces, target either ≥10% lower energy per delivered volume or ≥20% lower worst-case execution time, only with non-inferior service, reliability, equipment and uncertainty outcomes.
- Zero false-safe dispositions on the frozen critical negative controls.
- For nominal 90% uncertainty intervals, target empirical coverage within ±5 percentage points on sufficiently large independent holdouts; publish sample size and uncertainty around coverage.

No target constitutes a claim until held-out population and external evaluation criteria are satisfied.

## 7. Benchmark progression

- **T0:** JSON contract, scenario admission, expected-disposition oracle and deterministic receipt.
- **T1:** reference simulation using qualified EPANET/WNTR, including sensor placement / fault injection and independent hydraulic checks.
- **T2:** hardware-in-loop with pump/valve hardware, independent pressure/flow measurements and external local interlocks.
- **T3:** utility-approved shadow mode; recommendations only, no commands. Compare against operator decisions and measured service.
- **T4:** controlled pilot with explicit engineering approval and independent service-outcome verification.
- **T5:** replicated network/utility/season transfer with a public, rights-reviewed performance and failure report.

This benchmark is not a shortcut around engineering review or operational authorization.

## 8. SOTA and industry positioning

Commercial platforms and research digital twins already combine SCADA, GIS, hydraulics, telemetry, alarms and analytics. Luminous should reuse those capabilities. Its differentiation hypothesis is to qualify cross-vendor evidence and safe control admission at the boundaries between telemetry, estimator, controller, safety envelope and measured service.

A 2026 WDT benchmark/review warns that sample-level recall can rise while alarm persistence increases and event-onset detection worsens. It recommends assessing event performance and nominal alarm burden together. This supports reporting false alarms per operating period and time-to-detection, not only sample-level F1. See Pérez-Pérez (15 October 2026), [Eng 7(10), 564](https://doi.org/10.3390/eng7100564).

A 2026 human-governed agentic-AI / SCADA cybersecurity review similarly recommends staged decision-support deployment, segmentation, least privilege, monitored sessions, drift detection, rollback and operator review. It is not safe to give an experimental controller direct utility authority. See [10.1007/s10791-026-10697-7](https://doi.org/10.1007/s10791-026-10697-7).

### Competitive claim we can credibly pursue

Not "the best water controller" before a field trial. Pursue: **the most reproducible cross-controller qualification harness for service, observability, safe abstention and failure recovery across heterogeneous water-network populations.**

That claim itself must be earned through public case packs, exact-head receipts, independent review and measured integration/replication cost.

## 9. Next engineering work

1. Implement a Rust parser and contract evaluator for the 15 scenario records and disposition oracle in the existing `water-service-t0` tool family.
2. Add sensor placement / observability sweeps, starting with deterministic graph/topology fixtures and using #4398's qualified hydraulic solver boundary.
3. Add event-onset and false-alarm burden to the leak-diagnostics tracks, preserving their separate dataset populations.
4. Add a controller adapter trait only after baseline data-access and versioning semantics are fixed.
5. Request the 2026 ENC paper's raw data, code, parameter definitions and small-WDS files; if not received, record exact reproduction as blocked.
6. Only after T1/T2 gates pass, prepare a utility shadow-mode protocol.

## 10. Evidence and claim ceiling

- The scenario pack is synthetic and designed for future evaluator implementation.
- The cited ENC article is a publication-level comparator; its source data are not acquired here.
- The DeePC paper and constrained state-estimation paper are comparator candidates, not reproduced baselines.
- No controller, state estimator, interlock, hydraulic network, pump, sensor, service outcome or field saving is qualified by these documents.
- No SCADA credentials, live network connection, pump/valve actuation or utility control authority are included.
