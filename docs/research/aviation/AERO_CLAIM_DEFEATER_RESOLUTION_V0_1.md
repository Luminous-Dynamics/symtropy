# AERO Claim Defeater Resolution v0.1

**Status:** synthetic advisory source-binding adapter; exact-head build, tests and independent validation pending  
**Authority ceiling:** owner-reviewed recording of assumption/exclusion dispositions only; no complete claim decision or engineering/physical authority  
**Relationship:** additive to the closed claim-obligation profile and the evidence/model/authority/observer/temporal adapters

## 1. Purpose

A claim specification declares assumptions and exclusions. They are not ordinary evidence-class obligations: they identify conditions on which the argument depends and boundaries outside which the claim is not asserted.

They must not disappear during compilation or source binding. They also must not be converted into caller-controlled hard gates or silently treated as resolved.

This adapter derives the exact defeater universe from the registered profile and requires an explicit, source-scoped, owner-reviewed disposition for each declared assumption and exclusion.

## 2. Supported review states

| State | Required record | Interpretation |
|---|---|---|
| `OPEN` | Owner-reviewed receipt and exact scope/chronology | The assumption/exclusion remains unresolved and visible. |
| `RESOLVED_WITH_EVIDENCE` | At least one supporting evidence reference and its canonical manifest digest | The reviewer records evidence as resolving the stated defeater; physical truth is not established by this adapter. |
| `ACCEPTED_AS_LIMITATION` | Rationale digest and exact authority role | The reviewer accepts the condition as a declared limitation, not as proof that the condition is harmless. |
| `NOT_APPLICABLE_WITH_JUSTIFICATION` | Rationale digest and exact authority role | The reviewer records why the defeater does not apply within this claim/configuration. |

Missing receipts are represented as open, not assumed resolved. Duplicate receipts, undeclared obligation IDs, wrong requirement-source references, changed statement digests, invalid evidence manifests, wrong authority roles and failed verification are rejected. Late or inactive receipts remain unverified at the selected historical cutoff.

## 3. Exact binding

Each receipt binds:
- claim and configuration;
- exact decision-context digest;
- claim-specification semantic digest;
- registered obligation-profile and obligation-manifest digests;
- the derived defeater obligation ID and requirement-source reference;
- kind (assumption or exclusion) and exact statement digest;
- review state, supporting evidence manifest and rationale digest;
- reviewer/authority holder/role;
- receiver-first-seen time and half-open effectivity.

The derived obligation set is recomputed from the typed claim specification. A caller cannot omit an assumption/exclusion or introduce a new defeater that is not in the registered profile.

## 4. Non-compensatory and authority boundary

The current registered v0.1 requirement profile explicitly classifies assumption/exclusion defeaters as advisory/nonblocking. This adapter preserves that policy; it does not upgrade their severity, downgrade a different hard gate, mutate the assurance state, or replace ClaimGate.

It reports whether all defeaters have owner-reviewed dispositions and which remain open/unverified/rejected. Even an `ALL_REVIEWED` result always has:
- `complete_claim_assurance_state = false`;
- `assurance_state_mutated = false`;
- `engineering_acceptance_granted = false`;
- `physical_work_authorized = false`.

A different requirement profile may classify a particular assumption or exclusion as a hard gate, but that requires a separately versioned, registered and independently reviewed profile. A caller-supplied receipt cannot make that policy change.

## 5. Adversarial tests

Rust tests cover:
- all derived assumptions and exclusions receiving explicit dispositions;
- an open item remaining open;
- missing receipt never implying resolution;
- undeclared defeater rejection;
- resolved-without-evidence rejection;
- accepted limitation / not-applicable without rationale rejection;
- late receipt cannot backfill historical review;
- authority-role substitution rejection;
- receipt-order permutation invariance.

The source code also exposes canonical support-evidence and defeater-manifest digest functions. Future CI oracle work should independently reconstruct these commitments and add a fixture for each review state.

## 6. Non-claims

This module does not verify signatures or trust roots, authenticate the reviewer, prove source/evidence truth, establish whether an assumption is physically valid, decide whether an exclusion is safe, modify the registered hard-gate policy, approve an engineering change, or establish certification credit. The existing ClaimGate remains authoritative.
