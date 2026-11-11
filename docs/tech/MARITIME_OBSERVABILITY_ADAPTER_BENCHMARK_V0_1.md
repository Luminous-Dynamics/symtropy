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

Five installed measurement principles, six state hypotheses, five required pairs, thirty response cells and two common-cause/outage scenarios. The fixture contains two separable fault pairs, one deliberately non-isolable cooling pair, one non-isolable pressure-sensor-fault pair (the faulty sensor cannot validate itself), and one OOD pair that must remain insufficient-evidence.

A complete shared gateway loss withdraws both previously identifiable pairs. The fuel-pressure-transducer loss does not withdraw those two pairs because independent shaft-speed, exhaust-temperature, rack-position and vibration evidence remains available. This is a synthetic software conformance outcome, not a physical resilience result.

## Claim ceiling

This demonstrates only adapter mapping and deterministic software treatment of fabricated response intervals and declared outage scenarios. It does not establish real transducer independence, shipboard fault coverage, diagnostic accuracy, maintenance authority, ODD clearance or permission to operate.
