# WATER-OBSERVABILITY-QUALIFICATION-001 — Topology and Failure-Domain Qualification v0.1

**Tracking:** [#5045](https://github.com/Luminous-Dynamics/symtropy/issues/5045)  
**Existing sensor-layout owner:** [#4006 SENSOR-PLACEMENT-001](https://github.com/Luminous-Dynamics/symtropy/issues/4006)  
**Control consumer:** [#5018](https://github.com/Luminous-Dynamics/symtropy/issues/5018)

## SOTA and the new gap

Van Gemert et al., *Sensor placement for real-life water distribution networks, ensuring observability under uncertainties*, arXiv:2611.12414 (9 November 2026), demonstrate a structural-observability placement framework on real Brabant-Water networks. Its important property is independence from exact hydraulic parameter values **conditional on a correct topology and pressure-flow interconnection structure**. The authors also note that full observability can require many sensors on large/cyclic networks, creating a practical cost/maintenance tradeoff.

This benchmark does not reproduce or replace that placement algorithm. It asks what happens after the candidate sensor layout is chosen:
- Does the topology revision/digest still match the proof?
- Are apparent redundant sensors independent across calibration, physical interface, power, communications, clock and measurement-principle domains?
- Does one domain failure remove several channels at once?
- Does a structurally observable state actually have a numerically stable, calibrated estimator?
- Can a stale topology certificate invalidate model-derived states while leaving independently verified direct instrument values visible?

## Paradigm shift

Replace `sensor_count` and a single `fully_observable=true` bit with a **failure-domain-aware observability certificate** containing separate results for:
1. direct live measurements;
2. exact-topology structural observability;
3. numerical estimator qualification for a declared operating scope;
4. sensor/path survival under common-cause failure sets;
5. evidence freshness, calibration and time synchronization.

Two instruments are not independent just because their device IDs differ. They share a common failure domain if they depend on the same physical tap, calibration lineage, power source, gateway/communications route, clock, or measurement principle.

A structural proof is conditional on the exact graph. It does not make a stale sensor current or an ill-conditioned estimate accurate.

## Contract and dispositions

Per required state variable:
- `directly_observed_current`;
- `estimator_qualified_for_declared_scope`;
- `structurally_observable_only`;
- `unobservable_under_declared_faults`;
- `topology_invalidates_certificate`;
- `indeterminate_evidence`.

Overall:
- `all_direct_observed`;
- `estimator_qualified_all`;
- `structurally_observable_only`;
- `unobservable_under_declared_faults`;
- `topology_invalidates_certificate`;
- `indeterminate_evidence`.

Crucial separation: when the topology certificate is stale, independent direct instrument values remain visible as direct observations, but the structural proof is not reusable and no missing state may be reconstructed from it. The overall disposition reports `topology_invalidates_certificate`.

## Frozen synthetic corpus

The companion schema, 12-case scenario fixture and separate oracle cover:
1. independent direct channels;
2. single sensor dropout;
3. shared calibration lineage;
4. shared UPS/power domain;
5. common gateway/communication path;
6. common clock drift;
7. topology revision/digest mismatch;
8. full structural observability but ill-conditioned numeric estimator;
9. incomplete critical-state mapping;
10. alternate independent measurement principle surviving a common-cause fault;
11. estimator with synthetic scope/calibration/held-out/OOD contract;
12. no verified topology certificate.

Each sensor declares six dependency domains: physical interface, calibration lineage, power supply, communications path, clock domain and measurement principle. All sensors, IDs, failure groups, topology digests and bounds are synthetic.

## Qualification logic

A direct state is only credited when at least one channel is available, correctly bound, calibrated, synchronized and not dependent on an active failed domain.

A structural result may be used only when the certificate's topology revision and digest match the scenario exactly and the required state mapping is complete.

An estimator may be labelled `synthetic_qualified_contract` only with explicit model scope, calibration, holdout and OOD references, plus the v0.1 synthetic thresholds:
- empirical interval coverage at least 0.85 and no more than 0.95;
- at least 200 calibration/held-out samples in the fixture;
- maximum interval width no greater than 0.20;
- condition-number bound no greater than 1e6.

Those are challenge-set admission thresholds, not proof of real-estimator quality or engineering standards. `qualification_claimed` must remain false.

## Metrics and proposed targets

- exact per-state and overall known-answer agreement across all frozen cases;
- observability-survival curve versus sensor count and failed dependency domains;
- count of surviving *independent* measurement paths per state;
- false redundancy credit under shared physical/calibration/power/network/clock domains;
- rate of invalid structural proof reuse after topology change;
- interval coverage, interval width and condition-number distribution for estimators;
- sensor count, lifecycle cost and maintenance burden from candidate layouts in #4006.

Targets (not achieved):
- 100% exact result agreement on frozen challenge cases;
- 100% rejection of structural proof reuse after topology revision/digest mismatch;
- zero independent redundancy credit when all paths share an active critical failure domain;
- zero current direct-observation claims when all direct paths are stale, failed, calibration-invalid, clock-invalid or disconnected;
- zero estimator-qualified states without all required scope/calibration/holdout/OOD references and numeric gates;
- on future independent field holdouts, 90% interval coverage within ±5 percentage points, with sample size and interval width reported.

## Ownership and safety

#4006 remains the only owner of sensor layout optimisation and active observation selection. This challenge consumes its candidate layouts and qualifies their evidence; it does not add an optimizer.

The benchmark is offline/read-only. It does not connect to SCADA, operate pumps/valves, set utility service levels, allocate water, or qualify a real network.

## Claim ceiling

A synthetic contract and expected-disposition oracle only. The 9 November structural-observability result is a research comparator, not a Luminous result. No physical sensor layout, estimator, topology certificate, service outcome or utility operation is qualified.
