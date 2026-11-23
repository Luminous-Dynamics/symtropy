# Draft technical comments for NIST SP 800-82 Rev. 4 IPD

**Document:** NIST SP 800-82 Rev. 4 Initial Public Draft, published 21 September 2026  
**Comments deadline:** 30 November 2026  
**Status:** Luminous research draft for internal review; **not submitted**  
**Reference:** https://csrc.nist.gov/pubs/sp/800/82/r4/ipd

## Overall position

The draft's expansion of OT guidance to cloud/IIoT convergence, maritime and freight sectors, its CSF 2.0 structure, asset management, monitoring and zero-trust discussion are useful. We recommend making AI/ML and digital-twin assurance more explicit as a cross-cutting OT governance topic, while preserving safety-system independence and local control authority.

## Suggested comment 1 — Define AI authority classes for OT

**Recommendation:** Distinguish at least:
1. offline analytics / model development;
2. read-only monitoring and advisory;
3. supervisory recommendation requiring human approval;
4. bounded automatic supervisory action; and
5. direct control logic.

For each class specify allowed data paths, trust boundaries, approval, fail-safe behaviour, monitoring, rollback and assurance obligations. State that AI advisory systems are read-only by default and must not bypass independent BPCS/SIS protection layers. Any move toward automatic control requires a separate hazard analysis, lifecycle validation and regulatory/sector approval.

**Rationale:** "AI-enabled" is not one authority level. A forecasting model, a maintenance prioritiser and a controller have radically different consequence profiles.

## Suggested comment 2 — Treat model artefacts as controlled OT software

**Recommendation:** Extend change-management guidance to include model weights, model code, prompts/system instructions, feature schemas, training/validation data snapshots, calibration records, inference runtimes, containers, plug-ins and remote model endpoints. Require signed artefacts, provenance, version pinning, approved registries, rollback, drift monitoring and periodic requalification.

**Rationale:** A model update can change operational recommendations even when PLC/DCS configuration remains unchanged. Model/data lineage must therefore be governed as a production change.

## Suggested comment 3 — Protect telemetry-to-cloud and external data paths

**Recommendation:** For cloud/IIoT analytics, require explicit one-way or narrowly scoped egress where possible, data minimisation, authenticated source identity, message integrity, freshness/time bounds, anti-replay, schema validation and independently monitored control-plane separation. Degraded/lost cloud or external connectivity must not remove local safe operation.

**Rationale:** Remote analytics creates value but also new attack and common-cause failure paths. Signed telemetry can prove the message is unchanged since signing; separate metrology and sensor assurance are still required.

## Suggested comment 4 — Separate proposal, approval and execution authority

**Recommendation:** For consequential supervisory actions, enforce separate identities for model/service proposer, human approver and execution principal. Require dual control for high-consequence changes, short-lived scoped authorisation, a bounded action envelope, audit receipts and independent post-action observation.

**Rationale:** No single compromised model, agent or account should both originate and execute an unreviewed high-consequence action.

## Suggested comment 5 — Add qualification tests, not only design controls

**Recommendation:** Include or reference repeatable negative tests for:
- stale, replayed, duplicated, reordered or forged telemetry;
- model drift and out-of-distribution inputs;
- mismatched units and product grades;
- loss of time synchronisation and cloud connectivity;
- compromised model registry or inference image;
- unauthorised control writes and privilege escalation;
- failure of monitoring/management plane;
- common-cause failures across supposedly independent sources.

Measure false accepts, false rejects, recovery and safe-state behaviour. Report tests against a named system boundary and operating envelope.

## Proposed wording principle

"AI- and digital-twin-derived recommendations should be treated as untrusted decision inputs until validated against the authorised system state, the applicable engineering constraints and the organisation's approved change and safety processes. The use of AI does not transfer operational authority from designated personnel or certified control and safety functions."

## Review notes

- These comments should be checked against the full draft and sector-specific standards before submission.
- Do not claim that digital signatures establish measurement truth.
- Do not prescribe a particular blockchain or vendor.
- Preserve IEC 61511 / ISA-84 safety lifecycle requirements and existing SIS independence principles.
- NIST's draft remains a draft; do not cite it as a final requirement.
