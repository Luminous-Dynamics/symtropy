# Construction Contingent Policy Core — v0.1

**Status:** synthetic finite-policy evaluator. Not a construction scheduler, dispatch system, safety controller, permit authority or physical recovery qualification.

Parent: [CONSTRUCTION-CONTINGENT-POLICY-001 #5058](https://github.com/Luminous-Dynamics/symtropy/issues/5058).

Stack: interop #3644 → accepted-floor #4960 → space-time occupancy #4976 → observation planner #4989 → this contingent-policy core.

## Implemented v0.1

- Reuses the existing interop `ContextBinding` and observation planner's typed outcome.
- Declares a finite `TriggerKey` → `RecoveryAction` catalogue.
- Binds policy and trigger evidence to exact site epoch, design revision, target frame and robot configuration.
- Rejects absent, stale, future, context-mismatched and unmatched trigger evidence into an explicit `Hold`.
- Treats incomplete, invalidated and unavailable observations as unknown and forces hold-and-reconcile; site epoch changes always invalidate the old policy.
- Requires every non-hold proposal to carry qualification references and external approval roles.
- Rejects duplicate branch IDs, ambiguous trigger mappings, duplicate approval roles and malformed actions.
- Emits either `Hold` or `Proposal`. There is no `Execute`, `Grant`, `Permit` or `Accept` disposition.
- Requires current resource, occupancy, capability and authority revalidation before any external execution; this crate performs none of those grants.
- Thirteen Rust tests cover positive branches, unknown/no evidence, stale context, time freshness, approvals, qualification references and ambiguity rejection.

## Local commands

```sh
cargo fmt --manifest-path research/construction-contingent-policy-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-contingent-policy-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-contingent-policy-v0-1/Cargo.toml --locked --all-targets -- -D warnings
```

## Claim ceiling

This is synthetic decision/authority-boundary semantics only. It does not optimize a recovery schedule, authenticate approvals, inspect qualification records, make resource reservations, validate physical geometry, dispatch robots or demonstrate construction productivity. Hidden truth is not available to the policy evaluator.
