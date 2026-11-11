# Maritime observability adapter benchmark P0.1

**Owner:** MARITIME-OBSERVABILITY-001 #4609  
**Parent engine:** Symtropy #4005 / `symtropy-sensor-observability-bench`  
**Status:** synthetic adapter conformance; no vessel instrumentation map validated

## Mapping contract

- Asset family = declared engine model/family.
- Asset configuration = exact configuration SHA-256.
- Operating domain = versioned engine regime.
- Installation profile = sensor-topology SHA-256.
- `measurement_principle_ref` maps to the generic independent-principle group.
- `dependency_groups` remain separate and are exercised through explicit outage scenarios.
- State IDs are case-local and bound to `state_taxonomy_ref`; marine fault IDs are not translated to excavation labels.
- Only installed sensors enter the generic layout.
- Non-current quality/calibration and derived/reconstructed source roles cannot supply independent measured intervals.

## Frozen synthetic corpus

Five installed measurement principles, six state hypotheses, eleven required pairs, thirty response cells and two common-cause/outage scenarios. The fixture contains two separable fault pairs, one deliberately non-isolable cooling pair, one non-isolable pressure-sensor-fault pair (the faulty sensor cannot validate itself), and one OOD pair that must remain insufficient-evidence.

A complete shared gateway loss withdraws both previously identifiable pairs. The fuel-pressure-transducer loss does not withdraw those two pairs because independent shaft-speed, exhaust-temperature, rack-position and vibration evidence remains available. This is a synthetic software conformance outcome, not a physical resilience result.

## Claim ceiling

This demonstrates only adapter mapping and deterministic software treatment of fabricated response intervals and declared outage scenarios. It does not establish real transducer independence, shipboard fault coverage, diagnostic accuracy, maintenance authority, ODD clearance or permission to operate.


## P0.2 — separate detectability from fault isolation

The frozen matrix now contains all unordered pairs among the declared in-domain states and the reference-vs-OOD control. This is necessary because normal-vs-fault comparisons only establish detectability; they do not establish that two different fault hypotheses can be isolated from one another. The synthetic fixture intentionally makes turbocharger degradation and cooling restriction observationally equivalent while keeping both separable from the normal reference. The report must therefore return a shared ambiguity class for those two faults, not two independent unique diagnoses.

Per-fault assessments are derived from the declared reference-state pair and all required fault-fault pairs. A fault indistinguishable from the reference is UNOBSERVABLE; one distinguishable from the reference but not from another fault is assigned an AMBIGUITY_CLASS; OOD is UNSUPPORTED_REGIME; and missing pair evidence remains NOT_EVALUATED. Response-level evidence provenance is checked jointly with sensor source role, calibration and quality; a measured sensor inventory cannot promote a MODEL_ESTIMATE or OPERATOR_REPORTED response into independent measured evidence.
