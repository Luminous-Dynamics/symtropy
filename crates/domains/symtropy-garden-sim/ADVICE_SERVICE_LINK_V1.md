# Farmer Advice-to-Service Link — V1

**Status:** Draft reference contract and synthetic tests. Not a production integration or field-validation claim.  
**Tracking:** [FARMER-LINK-001 #1982](https://github.com/Luminous-Dynamics/symtropy/issues/1982)  
**Implementation:** `src/advice_service_link.rs`  
**Reference schema:** `advice_service_link.v1.schema.json`  
**Synthetic corpus:** `fixtures/advice_service_link_cases.v1.json` + `fixtures/advice_service_link_cases.v1.schema.json`

## Paradigm shift

**Trace a decision across systems without creating another source of truth—and without pretending the link proves causation.**

A farmer may ask for an irrigation repair with no AI advice at all. A recommendation may precede a request without causing it. A provider can claim completion while the farmer has not accepted the work. An accepted repair does not automatically establish that crop output, water use, income or welfare improved.

This V1 contract links opaque references only. It does not copy numeric outcomes or the lifecycle objects owned by other systems.

## Ownership and authority

| Capability | Canonical owner | Link module's role |
|---|---|---|
| Advice episode, decision and premise dependencies | Symthaea / WCARE | Carry opaque advice/decision references; do not reproduce premises |
| Predicted and observed consequential outcomes, corrections and repairs | Symthaea / WCARE evidence and accountability ledgers | Carry optional references to observed outcome claims; do not store outcome values |
| Request, quote, booking, completion and farmer decision | Symtropy `service_access` | Check request reference and scope; never change service state |
| Consent, relayed authority and sharing receipts | Mycelix | Require an authorization reference for a `linked` state; do not authenticate or authorize disclosure |
| Place, field/asset and route evidence | Sol Atlas | Use existing opaque scope references; do not invent coordinates or route time |

A reference digest is not source authentication. A structurally valid authorization reference is not proof of a current permission. Downstream sharing must resolve permission in the owning Mycelix consent system.

## Link fields

```json
{
  "schema_version": 1,
  "link_id": "opaque-link-reference",
  "service_request_ref": "canonical-symtropy-request-id",
  "advice_episode_ref": "canonical-wcare-advice-episode-id",
  "decision_ref": "canonical-decision-id-or-null",
  "advice_scope_ref": "opaque-site-or-asset-reference",
  "advice_period_ref": "declared-season-or-period-reference",
  "advice_task_code": "irrigation-pump-repair",
  "reason": "farmer_declared",
  "initiated_by_role": "farmer",
  "state": "linked",
  "authorization_ref": "owning-system-consent-receipt-reference",
  "observed_outcome_claim_refs": ["canonical-observed-claim-reference"],
  "source_ref": "source-or-fixture-reference",
  "recorded_at_epoch_seconds": 1790000000
}
```

The example is illustrative. The implementation validates reference shape and scope consistency only; it does not resolve the references or prove the referenced facts.

## Core invariants

1. **An unlinked service request is valid.** Advice participation is optional; no model recommendation is required to receive service.
2. **No advice-to-action authority.** A linked advice record never creates a request, quote, booking, dispatch, payment or physical action.
3. **No causality by association.** A shared episode/request timeline cannot establish that advice caused a service request or downstream outcome.
4. **Scope must match.** Service request ID, site/asset scope, period and task must agree. Mismatch blocks the link but does not invalidate the service request.
5. **Consent stays with its owner.** A missing permission reference blocks `linked` state; even a present reference does not grant disclosure in this module.
6. **Revocation has honest semantics.** `revoked_for_future_sharing` prevents new disclosures through this path but does not claim prior external copies are recalled.
7. **Observed outcomes are references, not copies.** No crop/yield/water/income value is duplicated here; predictions remain distinct from separately sourced observations.
8. **Unresolved states stay unresolved.** Pending, stale, disputed and unavailable link states require resolution; they are not treated as missing value=0 or as successful linkage.
9. **Replay stays idempotent.** The current function is pure reference assessment; any future event-storage adapter must separately prove duplicate link-event replay is idempotent.
10. **All failure paths preserve the request.** Every assessment returns `service_request_remains_valid = true`; link issues may block the link, not erase or downgrade the service request.

## Frozen synthetic scenarios

The JSON fixture contains 20 cases spanning:
- valid and absent optional link;
- site/period/task and service-request reference mismatches;
- missing authorization reference;
- duplicate/empty observed-claim references;
- stale, disputed, pending, unavailable and revoked states;
- unsupported schema version and empty fields;
- digest-shaped source and authorization references that do not raise the claim ceiling.

The unit test applies the frozen mutations, asserts exact disposition and blocker codes, and checks for every case that the service request remains valid, disclosure is not authorized by this module, and causal claims are not permitted.

The companion binary `verify-advice-service-link-manifest` checks fixture/schema inventory, the supported schema dialect, required mutation coverage, and mapping to the Rust mutation arms. It prints `STRUCTURE_VALID_ONLY`. This is deliberately not described as schema-engine validation, semantic test execution, source authentication or field evidence. The separate `cargo test --all-targets` workflow step is responsible for actually running unit tests.

## Proposed benchmarks

### Contract gates
- 100% exact match to frozen synthetic expected dispositions and blockers.
- 100% mutation fixture coverage against supported mutation arms.
- Zero invalid/duplicate/cross-scope links treated as successfully linked.
- Zero service-request invalidation caused by missing, stale, revoked or malformed optional link.
- Zero disclosure authorization or causal claim emitted by this module.
- Zero duplicate outcome references accepted in one link.
- Unknown versions fail closed.

### System / process evaluation
- When later integrated with online/offline persistence, prove duplicate/out-of-order link event replay is idempotent, without changing the canonical service lifecycle.
- Test permissions by calling the owning Mycelix consent interface; a reference-only module cannot pass an authorization test alone.
- In field evaluation, report linked and unlinked requests, declared linkage reason, channel, permission currentness, farmer acceptance, no-provider/late/disputed/rework outcomes and missing follow-up separately.
- Do not quantify advice-caused yield, income, water-saving or welfare improvements without a task/season-specific comparison design and independently measured targets.

## Release maturity

The current artifact is a draft contract plus source tests. Structural-manifest verification, unit-test execution, linked-system integration, usability testing and field evaluation are separate maturity stages. Queued workflows are not a pass, and synthetic results do not establish operational outcomes.
