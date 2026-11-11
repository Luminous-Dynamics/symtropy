# Shipyard twin composition and commissioning oracle v0.1

Tracking: [SHIPYARD-DT-COMPOSITION-COMMISSIONING-001 #4718](https://github.com/Luminous-Dynamics/symtropy/issues/4718)

## Standards context

- ISO/TR 23247-101:2026 is a published manufacturing digital-twin use case for robotic multilayer/multipass GMAW.
- ISO 23247-5:2026 specifies the manufacturing digital thread for digital twins.
- ISO 23247-6:2026 specifies composition principles for integrated, unified and federated digital twins.
- ISO/AWI 23247-7 (commissioning) and ISO/AWI 23247-8 (VVUQ) are under development. This v0.1 aligns its research concerns with their published scopes; it does not claim conformance.

## Executable v0.1

A dependency-free Rust library accepts synthetic-only twin members and interface contracts. It checks:
- exact workpiece identity and member revisions;
- source SHA-256 binding and member freshness;
- schemas, units and coordinate-frame mappings;
- calibration revision binding for transforms;
- clock-domain mapping and offset budget;
- update period, latency, jitter and fidelity separately;
- common capability intersection and authority preservation;
- integrated/unified/federated topology evidence;
- end-to-end aggregate latency;
- unknown update outcomes and live-operation denial.

Commissioning produces a time-bounded receipt bound to member IDs, revisions and source digests. A member revision or source digest change, expiry, composition revision change or authority change requires requalification. An unknown update outcome suspends the existing composition receipt until reconciliation.

The crate is isolated in a nested Cargo workspace with its own lockfile; root workspace files are untouched.

## Synthetic test coverage

The 23 tests cover all three composition topologies, receipt binding and expiry, member revision/source drift, workpiece mismatch, stale members, physical evidence rejection, missing interfaces, schema/unit/frame/clock mismatch, frequency/latency/jitter/fidelity budgets, end-to-end latency, authority capability escalation, unknown outcomes, lifecycle suspension/requalification and retirement.

All metrics are synthetic ticks and permille units. There is no connection to live equipment, no physical digital twin, and no operational commissioning authority.

## Claim ceiling

A PASS proves only deterministic synthetic composition/commissioning contract behavior. It is not an ISO conformity assessment, production commissioning, a physical VVUQ result, WPS qualification, NDT acceptance or robot-execution permission.
