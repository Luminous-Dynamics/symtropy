# AERB Scope-Closure Verifier Adapter v0.1

**Status: fail-closed integration scaffold.** This crate composes the existing RRVR Evidence Capsule temporal-closure evaluator with an injected attestation-trust provider. It contains no production signature algorithm, keyring or permissive verifier. Until the cross-language trust provider from #7563 is wired in, callers cannot obtain a closed-scope disposition.

## Reuses rather than duplicates

- `mobility-rrvr-evidence-capsule` owns typed Evidence Capsules, capsule digests, bundle validation and `TemporalClosure` claim qualification.
- The `AttestationVerifier` trait is the integration seam for #7563. A production implementation must verify the signed preimage and raw signature envelope against a pinned trust policy and revocation state, then return the actually verified signer identity.
- This crate verifies raw artifact bytes, execution-scope/profile/frontier matching, completeness of effect-producing components, command holders and forwarding edges, signer independence from the coordinator, then constructs an Evidence Bundle and delegates temporal-closure assessment to Evidence Capsules.

## Critical trust rule

A `verification_status = verified` JSON field is not evidence. The only path to `ClosedWithinDeclaredScope` is a successful `AttestationVerifier::verify` result plus all raw-artifact and Evidence Capsule checks. No production verifier is included in v0.1. A mock verifier exists only in unit tests. The CLI/integration boundary must inject the real provider from #7563; absence or rejection means fail closed.

## Scope boundary

Closure applies only to the declared execution boundary and validity interval. It does not prove that the entire physical universe has no other effect-producing path. It does not establish Applied/NoEffect, authorize retry, or certify robot safety.

## Run

```bash
cargo fmt --manifest-path research/mobility-rrvr-aerb-scope-closure-v0.1/Cargo.toml -- --check
cargo test --manifest-path research/mobility-rrvr-aerb-scope-closure-v0.1/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-rrvr-aerb-scope-closure-v0.1/Cargo.toml --locked --all-targets -- -D warnings
```

## Claim ceiling

Current v0.1 is an integration scaffold and structural verifier. It cannot authenticate any receipt without a real #7563 provider. No physical completeness, simulator/HIL/physical qualification, universal absence of effects, robot safety certification or SOTA claim is made.