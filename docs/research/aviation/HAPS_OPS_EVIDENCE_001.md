# HAPS operations evidence ledger v1

This is a HAPS-specific evidence projection over the general Mycelix AeroCommons lifecycle model. It is intended to supply real operational records to fleet availability and Semi-Markov reliability analysis without inventing maintenance rates or conferring release authority.

Artifacts:
- [Schema](schemas/haps_ops_evidence_ledger_v1.schema.json)
- [Synthetic maintenance lifecycle](examples/synthetic_haps_maintenance_lifecycle_v1.json)

The fixture contains a post-flight anomaly discovery, an interval-censored onset window, work-order and parts events, repair, inspection and functional-test evidence. It ends in awaiting_authorized_release. The aircraft is not returned to service.

A second asset contributes five days of operating exposure with no failure observed; it is right-censored. It must not be interpreted as proof of a five-day failure-free lifetime or mixed naively with the interval-censored defect to produce a point MTBF.

The return-to-service boundary is deliberately stricter than a boolean status. A valid release requires an external authority record that is bound consistently in three places: the ledger boundary, a `return_to_service_authorized` event, and the exact asset/configuration active at that event sequence. The event must also include a source evidence item of kind `release_authority_record` with the same authority reference. A release for an earlier configuration does not authorize a later configuration.

Four new negative/positive controls exercise this rule: exact scoped authorization passes; a mismatched authority reference, mismatched asset/configuration, or non-authority evidence kind fails closed. The synthetic example remains unreleased and ends in `awaiting_authorized_release`. The active-configuration guard also means that a later `configuration_changed` event makes an earlier release receipt stale; a new configuration must complete its own verification and receive its own exact external release.

The Rust validator checks:
- event sequence and previous-event linkage;
- asset and as-flown configuration references;
- lifecycle state transitions;
- occurred/observed/recorded timestamp ordering;
- interval-censored and right-censored exposure semantics;
- exposure duration against its time bounds;
- no return-to-service status without an explicit external authority receipt.

This is structural/provenance validation only. It does not verify the content or cryptographic authenticity of source evidence, determine whether a repair is technically adequate, or replace an applicable inspection, airworthiness or return-to-service authority. The exact-reference check proves only that the ledger is internally bound to the declared external receipt; it does not prove that the issuer is competent, that the signature is authentic, or that the release scope is legally sufficient.
