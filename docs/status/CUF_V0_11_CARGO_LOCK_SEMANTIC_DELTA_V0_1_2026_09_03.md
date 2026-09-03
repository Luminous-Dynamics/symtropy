# CUF v0.11 Cargo.lock Semantic Delta Review v0.1 — 2026-09-03

This review supplements the byte-for-byte lock repair evidence in PR #120.

The lock repair generator/verifier proves that Cargo generated exactly one reviewed `Cargo.lock` delta and that the repair commit contains exactly that delta. That is necessary but does not by itself explain the semantic shape of the dependency change.

For every Stage A or Stage B repair, reviewers should additionally classify the generated lock delta into:

- workspace package records added/removed/modified;
- registry/git package records added/removed;
- existing package version/source/checksum identity changes;
- dependency-list changes for existing package records.

The important policy is not to assume every one-file Cargo.lock change is harmless. Unexpected unrelated dependency churn must be explained before the repair is accepted, even when Cargo generated it.

Current static expectations are intentionally predictions, not verifier truth:

- Stage A is expected to introduce/reconcile the `symtropy-sim-contracts` workspace package entry and its already-present serde/sha2 dependencies;
- Stage B is expected to reconcile `symtropy-terrain` dependency metadata after v4.8 adds serde/sha2 and enables `bevy_pbr`, potentially activating additional already-version-constrained Bevy dependencies.

Cargo under the pinned Nix environment remains authoritative for the actual lock result. Do not hand-edit the lockfile to force it to match these expectations.

A semantic review should reject or escalate unexplained outcomes such as:

- unrelated package version upgrades/downgrades;
- registry or git source changes unrelated to the manifest delta;
- checksum identity changes for an unchanged package version/source;
- disappearance of unrelated workspace packages;
- broad resolver churn not justified by the recorded Cargo.toml inputs.

This semantic review is a human/review policy in v0.1. It is deliberately not encoded as a brittle automatic allowlist before we observe the first real Cargo-generated Stage A/Stage B deltas.

Once those real deltas are captured, the policy can be promoted into narrower machine-checkable expectations if the evidence supports doing so.
