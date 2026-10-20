# AtomFab Workcell Fault Corpus v1

**Status:** Frozen software regression suite; not physical qualification.  
**Scope:** Synthetic software-contract and replay checks shared by Symthaea AtomFab and Symtropy.

- Canonical machine-readable cases: [ATOMFAB_WORKCELL_FAULT_CORPUS_V1.json](ATOMFAB_WORKCELL_FAULT_CORPUS_V1.json)
- JSON Schema: [ATOMFAB_WORKCELL_FAULT_CORPUS_V1.schema.json](ATOMFAB_WORKCELL_FAULT_CORPUS_V1.schema.json)

The 14 cases cover stale and non-finite observations, sensor drift outside calibration, actuator saturation, collision-margin breach, actuator-count mismatch, bridge timeout, malformed planner output, out-of-domain models, replay divergence, simulation-to-measurement provenance confusion, unqualified physical-actuation requests, and two in-domain controls.

The oracle is intentionally conservative: reject, abstain, or reevaluate never authorizes a simulation action; a declared hard-constraint violation can never be accepted as feasible; and valid controls must pass so that reject-all does not score as safe. Exact case identifiers and reason codes are part of v1 and should be changed only by versioning the corpus.

This corpus tests software dispositions and provenance boundaries. It does **not** certify a safety function, validate a physical model, authorize hardware, prove process yield, or demonstrate atomistic fabrication. Symtropy's adapter and process-twin runtime still need to consume these cases in executable tests before cross-repository conformance can be claimed.
