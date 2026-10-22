# AtomFab Workcell Fault Corpus v1

**Status:** Frozen software regression suite; not physical qualification.  
**Scope:** Synthetic software-contract and replay checks shared by Symthaea AtomFab and Symtropy.

- Canonical machine-readable cases: [ATOMFAB_WORKCELL_FAULT_CORPUS_V1.json](ATOMFAB_WORKCELL_FAULT_CORPUS_V1.json)
- JSON Schema: [ATOMFAB_WORKCELL_FAULT_CORPUS_V1.schema.json](ATOMFAB_WORKCELL_FAULT_CORPUS_V1.schema.json)

The frozen v1 suite has 15 uniquely identified cases: stale and non-finite observations, sensor drift outside calibration, actuator saturation and invalid planner gain, collision-margin breach, actuator-count mismatch, bridge timeout, malformed planner output, model out-of-domain, replay divergence, simulation-to-measurement provenance confusion, unqualified physical-actuation requests, and two valid controls.

Expected outcomes include stable reason codes, typed adapter-fault mappings where applicable, simulation-action authorization, feasibility status, and hard-constraint disposition. The AtomFab crate enforces corpus invariants. Symtropy's typed robotics adapter has a conformance test for the six cases mapped to its public fault enum; process-level cases remain a specification for future workcell-runtime integration.

The oracle is intentionally conservative: reject, abstain, or reevaluate never authorizes a simulation action; a declared hard-constraint violation can never be accepted as feasible; and valid controls must pass so that reject-all does not score as safe.

This corpus tests software dispositions and provenance boundaries. It does **not** certify a safety function, validate a physical model, authorize hardware, prove process yield, or demonstrate atomistic fabrication. The fault cases are synthetic test inputs, not measured apparatus behavior.
