# Mobility dependency-component quarantine v0.1

Parent: Symtropy issue #6698. Native authority boundary: #6669.
Status: research-only read-only reference; it does not authorize, cancel, release or dispatch actions.

## Objective

When an action may have applied but its outcome is Unknown, PartialEffect or AppliedUnreconciled, hold the exact transitive dependency closure only when every required owner supplies a complete inventory for the same exact action-attempt scope. Otherwise hold the entire scope.

The graph is a typed hypergraph, not merely a pairwise conflict list. A shared capacity, fixture, observer, state invariant or common-cause dependency may connect three or more actions. The evaluator indexes action-to-domain incidence and expands every domain at most once.

## Fail-closed semantics

- Every in-scope action has exactly one outcome classification. Missing, duplicate or foreign outcomes force GlobalHold.
- Every profile-required owner supplies exactly one inventory for the same scope and exact action references, including attempt and ticket digest.
- An inventory is usable only when it explicitly reports Complete and binds the expected action/domain counts and nonzero commitments.
- Missing, unknown, stale-scope, malformed or structurally incomplete inventories force GlobalHold.
- Component domains require at least two exact members. Global domains must name the entire scope. Unknown impact forces GlobalHold.
- Multiple unresolved seeds produce the union of their closures.
- An action outside the result closure is not released; it still requires fresh authority, reservations, local runtime assurance and native #6669 application checks.

Unknown is never interpreted as an empty dependency graph. A complete empty owner inventory is meaningful only as an explicit owner assertion for the exact scope.

## Ownership

Compose existing owners; do not recreate them:
- M1/M2 identity, authority and native application: #6669.
- Space-time occupancy/motion envelopes: #5233/#5401 and their upstream geometry owners.
- Resource reservations and aggregate capacity: #3644/#4992/#5009.
- Domain state, precondition and effect owners.
- Observer/evidence owners and Mycelix AGENT-009 #803 / AGENT-010 #830.

This crate validates structure and computes closure only. It does not authenticate signatures, owner truth, calibration, inventory completeness, physical outcomes or digests.

## Complexity and qualification

The ordered-map reference is approximately O((A + I) log A), where A is action count and I is action/domain membership incidence. Benchmark long chains and giant hyperedges at 10, 100, 1,000 and 10,000 actions; no wall-clock result is claimed.

Run:
- cargo fmt --manifest-path research/mobility-dependency-quarantine/Cargo.toml -- --check
- cargo test --manifest-path research/mobility-dependency-quarantine/Cargo.toml --locked
- cargo clippy --manifest-path research/mobility-dependency-quarantine/Cargo.toml --all-targets --locked -- -D warnings

The Rust toolchain is not present in the local execution environment. Only exact-head hosted results may qualify this crate. Queued, cancelled or stale-head runs are not passes.

## Non-claims

No scheduling, dispatch, cancellation, rollback, automatic retry, effect reconciliation, durable restart semantics, physical independence, runtime safety, simulator fidelity, HIL, field productivity or regulatory qualification.
