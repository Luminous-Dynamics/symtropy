# HAPS operations evidence ledger v1

This is a HAPS-specific evidence projection over the general Mycelix AeroCommons lifecycle model. It is intended to supply real operational records to fleet availability and Semi-Markov reliability analysis without inventing maintenance rates or conferring release authority.

Artifacts:
- [Schema](schemas/haps_ops_evidence_ledger_v1.schema.json)
- [Synthetic maintenance lifecycle](examples/synthetic_haps_maintenance_lifecycle_v1.json)

The fixture contains a post-flight anomaly discovery, an interval-censored onset window, work-order and parts events, repair, inspection and functional-test evidence. It ends in awaiting_authorized_release. The aircraft is not returned to service.

A second asset contributes five days of operating exposure with no failure observed; it is right-censored. It must not be interpreted as proof of a five-day failure-free lifetime or mixed naively with the interval-censored defect to produce a point MTBF.

The Rust validator checks:
- event sequence and previous-event linkage;
- asset and as-flown configuration references;
- lifecycle state transitions;
- occurred/observed/recorded timestamp ordering;
- interval-censored and right-censored exposure semantics;
- exposure duration against its time bounds;
- no return-to-service status without an explicit external authority receipt.

This is structural/provenance validation only. It does not verify the content or cryptographic authenticity of source evidence, determine whether a repair is technically adequate, or replace an applicable inspection, airworthiness or return-to-service authority.
