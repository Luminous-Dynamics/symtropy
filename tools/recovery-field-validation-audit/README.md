# Rights-Aware Recovery Field Validation Audit

Standalone Rust auditor for the synthetic field-validation packet. It keeps legal/authority gates, study-data rights, evidence timing, measurement state, decision-manifest integrity, and post-decision outcomes distinct.

## Run

Use Cargo to format/test the crate and execute the audit with the frozen packet plus the exact source commit. The tool emits deterministic JSON and returns nonzero when the packet fails structural, temporal, evidence, manifest, lifecycle or claim-ceiling checks.

## Contracts

- Decision inputs must be observable and available by the decision cutoff and must be tagged as decision-time evidence.
- Outcome observations are separate records with strictly later timestamps; they cannot be decision inputs.
- `MEASURED` zero is a numeric result; `NOT_MEASURED`, `INACCESSIBLE`, `CENSORED`, `CONTRADICTORY` and `NOT_APPLICABLE` cannot contain numeric zero or any other numeric value.
- Required metrics must have a measured value and evidence tied to a decision-time input.
- Data-use rights, legal authority, access/consent, ecological review, safety, qualified operator, and transport/disposal are separate non-compensatory gates.
- Decision manifests have canonical compact JSON payloads and SHA-256 digests; matching digest is integrity, not issuer authenticity.
- No real field observation, legal permission, professional qualification or operation authority is asserted; every source remains `UNVERIFIED_SYNTHETIC`.
- Referrals, no-action episodes and censored follow-ups stay in the denominator.

See `docs/tech/RIGHTS_AWARE_RECOVERY_FIELD_VALIDATION_V0_1.md`, `schemas/recovery/field-validation-packet-v0.1.schema.json`, and issue #1929.
