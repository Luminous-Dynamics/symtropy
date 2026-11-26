# Maritime Plan Bundle v0.1

**Status:** synthetic composition prototype; dedicated exact-head workflow passed.
**Tracking:** [#7544 — MARITIME-PLAN-BUNDLE-001](https://github.com/Luminous-Dynamics/symtropy/issues/7544).
**Related:** #3683, #7488, #7504, #7512, #7519, #7528.

## What it answers

A physical resource plan can be feasible while the network still lacks consent from the parties that control required resources. This crate asks:

> For every reservation in an upstream resource-plan receipt, is there exactly one accepted, unexpired, correctly scoped commitment projection for the same resource, slot, quantity, unit, controller, accepting participant, revision and terms digest at the same decision-time cutoff?

The result is a **commitment-coverage receipt**, not an execution authorization.

## Ownership boundaries

- #7519 / PR #7521 owns physical resource feasibility and Pareto enumeration. This crate consumes a ResourcePlanReceipt; it does not recalculate resource capacities.
- #7528 / PR #7532 owns offer/counteroffer/acceptance/rejection/revocation event reduction and unknown-outcome mutation guard. This crate consumes CommitmentProjectionReceipt via a provider trait; it does not reimplement the state machine.
- #7505 / #7512 own source history and decision/audit temporal projections.
- Mycelix owns participant permission, revocation and custody authority.
- #7488 consumes typed owner receipts and applies cross-owner fail-closed composition.
- #3683 owns end-to-end service and receiver outcomes.

The current fixture provider is a static adapter over frozen receipts. It is not a production connector and does not authenticate the origin of those receipts.

## Fail-closed contract

For a complete bundle, every physical reservation must have exactly one commitment requirement and exactly one provider projection. All exact terms must match, and the commitment must be Accepted at the same knowledge cutoff with expiry strictly after that cutoff.

Missing, unavailable, invalid, stale-cutoff, offered, rejected, revoked, expired or terms-mismatched commitments keep the bundle incomplete. Orphan requirements and duplicate identities are surfaced rather than silently ignored. A physically infeasible or unknown plan cannot be promoted by good commitment state.

Quantity and unit are both compared exactly. `12 container` is not silently treated as `12 TEU`.

## Current frozen history corpus

The grouped TSV fixtures cover:
- complete two-party berth + rail commitment;
- partial acceptance (berth accepted, rail only offered);
- container/TEU mismatch;
- revoked commitment;
- expiry exactly at the cutoff;
- missing commitment projection;
- stale commitment knowledge cutoff;
- unknown physical plan;
- wrong resource-controller party.

There are 11 fixture rows across 9 case groups. The deterministic Rust runner emits a bundle-level result and each reservation-level result. A separately implemented standard-library Python reference reconstructs the expected report directly from the TSV and compares the complete receipt.

The Rust unit suite additionally tests missing requirements, orphan commitments, duplicate requirements and duplicate commitment identity reuse.

## Run locally

```sh
cargo fmt --manifest-path research/maritime-plan-bundle/Cargo.toml -- --check
cargo test --locked --manifest-path research/maritime-plan-bundle/Cargo.toml
cargo clippy --locked --manifest-path research/maritime-plan-bundle/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path research/maritime-plan-bundle/Cargo.toml --bin plan-bundle-audit > plan-bundle-receipt.tsv
python3 research/maritime-plan-bundle/scripts/independent_reference.py --receipt plan-bundle-receipt.tsv
```

## Limitations and claim ceiling

- The ResourcePlanReceipt is a typed upstream input; this crate does not authenticate its signature, issuer or digest.
- The static provider is a fixture adapter, not the live #7532 reducer.
- The reducer receipt's authenticity, participant identity and permissions are not cryptographically verified here.
- This crate does not validate DCSA/N4/Portchain payloads, external contracts or legal enforceability.
- No bookings, dispatch, TOS mutations, custody transfers or payments are issued.
- Bundle completeness is not execution authorization, proof of actual resource availability or proof of realized cargo delivery.
- The synthetic corpus does not demonstrate a real port's service improvement or a SOTA win.

The next adapter must consume actual versioned receipts from the physical and commitment owners, preserve their provenance/cutoffs, and hand their typed outputs to #7488. It must not create duplicate ownership of capacity, participant permissions, source history or custody.

## Exact-head qualification record

The dedicated `maritime-plan-bundle` workflow passed on commit `ae924f193f3b0745a31b5ee5f836e3625f761071`:
- Formatting: pass
- Rust tests: 16 passed, 0 failed
- Strict Clippy: pass
- Deterministic bundle receipt: pass
- Independent multi-party history reference: 9/9
- Receipt artifact uploaded

This qualifies only the frozen synthetic commitment-coverage contract. The static provider remains a fixture adapter; no production receipt authentication or external mutation is connected.
