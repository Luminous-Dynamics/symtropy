# AeroWeft Service Resilience Oracle

Synthetic-first Rust evaluator for aviation service capability states, failure-domain diversity, cross-flight impact and contingency-coordination evidence under degraded shared U-space services.

It does not run a UTM service, issue operational decisions, authorize flights, select emergency routes, command aircraft, create contingency plans, authenticate provider receipts, or prove physical service availability. Provider/trust/coordination evidence are owner-supplied inputs; this crate checks their declared structure only.

It composes Tracks M/N/O/P/Q/S/T/U and Mycelix common-cause/trust owners. It does not create a second trust registry, event store, identity system, lease algebra, surveillance fusion or flight-control system.
