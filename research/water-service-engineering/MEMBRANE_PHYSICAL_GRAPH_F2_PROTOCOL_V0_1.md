# F2 — Signed, lag-aware membrane process graph qualification v0.1

**Issue:** [#5063 MEMBRANE-PHYSICAL-GRAPH-F2](https://github.com/Luminous-Dynamics/symtropy/issues/5063)  
**Parent:** [#5056 observability and sensor faults](https://github.com/Luminous-Dynamics/symtropy/issues/5056)  
**Temporal benchmark:** [#5059](https://github.com/Luminous-Dynamics/symtropy/issues/5059)  
**Status:** synthetic graph contract and ablation plan only. No real plant topology is asserted.

## Research thesis

A treatment plant is not merely a set of correlated time series. It is a physical network of unit operations, hydraulic pathways, chemical/transport dependencies, instrumentation and delayed propagation.

The retrieved 2026 PI-STGAT SWRO study uses a hybrid graph with physical/domain and mutual-information edges, lag-aware temporal edges, and sensor-time-step attribution. It reports average F1 0.953 on a specific 1,000 m³/day SWRO anomaly task (DOI 10.1016/j.watres.2026.126904). That is a useful comparator, not a comparable membrane-fouling score.

The programme should go beyond architecture reproduction by evaluating graph provenance, uncertainty, incorrect-edge sensitivity, sensor faults, unknown topology and the stability of explanations under graph ablation.

## Graph contract

Each node is a process unit, sensor or virtual/latent state. Each edge states:
- source and destination IDs;
- physical relation;
- signed/monotonic direction;
- expected minimum and maximum propagation lag, with unit;
- evidence class and provenance;
- confidence/uncertainty;
- operating-regime applicability;
- dependent sensors and measurement-health requirements.

Evidence classes must distinguish measured, derived, engineering-design, literature-supported, synthetic and unknown. A statistical association is not automatically a physical edge. A domain edge must not be silently inferred from the evaluation labels.

## Example topology

The accompanying synthetic graph contains a simplified pretreatment → high-pressure pump → membrane feed/concentrate/permeate pathway plus conductivity/osmotic-pressure and normalized-resistance states. The graph is illustrative only. It is not an actual P&ID and should not be applied to a plant without its verified P&ID/tag dictionary and qualified process-engineer review.

## Matched baseline ladder

1. Univariate statistical alarms.
2. Correlation/MI-only graph.
3. Domain/engineering graph only.
4. Hybrid domain + MI graph.
5. Hybrid graph with learned lag-aware edges.
6. Signed physical graph with interval-valued lag and uncertain topology.
7. Hybrid physical graph + mass/pressure/flow residual observer.

All methods must use identical sensor availability, lag windows, graph budgets, feature cutoffs and temporal/site split. Learned graph edges may use training data only. Test labels must never construct topology.

## Required ablations and adversarial controls

- remove physical edges;
- remove MI edges;
- remove lag information;
- reverse selected edge signs;
- perturb or widen lag intervals;
- delete sensors and edges;
- inject sensor drift, clock skew and missingness;
- change feed salinity, temperature and pump operating regime;
- introduce a deliberately incorrect domain edge;
- provide incomplete graph provenance;
- evaluate interacting causes.

A useful graph should degrade in predictable ways under these challenges. It should not confidently localize a fault using a broken or unverified process path.

## Metrics

- source-node and upstream-origin localization accuracy;
- macro-F1 across sensor/process/multiple-cause classes;
- edge/path attribution precision and recall against independently specified propagation;
- lag estimation error and propagation-order accuracy;
- graph edge recovery under missing/noisy measurements;
- stability under edge removal and sign/lag perturbation;
- calibration, selective risk and abstention when topology is incomplete;
- false fouling localization under sensor-only and regime-only negative controls.

## Initial stretch targets (not achieved)

- at least 0.90 source-node localization macro-F1 on a held-out synthetic graph challenge;
- at least 0.90 correct upstream/downstream propagation ordering on unseen lag patterns;
- under 10% relative performance degradation when up to 10% of noncritical edges are removed, under a pre-registered perturbation design;
- explicit abstention when a critical physical path is unknown or outside the validated envelope;
- no increase in false fouling localization on sensor-only and regime-only negative controls.

Targets are research goals, not current metrics.

## Physical admission gate

A real plant graph requires:
- current P&ID and process-flow diagram revision;
- equipment and sensor tag dictionary;
- sensor units, calibration, locations and sample times;
- pump curves, valve/control logic and membrane train/module boundaries;
- operating modes and documented abnormal-state paths;
- engineer-reviewed edge sign, lag range and applicable regimes;
- provenance and approval receipt.

Until then, use the synthetic graph as a schema/algorithm fixture only.

## Nonclaims

No real P&ID was ingested. No plant graph, causal pathway, fault origin, sensor localization, membrane mechanism or control policy is qualified by this contract.
