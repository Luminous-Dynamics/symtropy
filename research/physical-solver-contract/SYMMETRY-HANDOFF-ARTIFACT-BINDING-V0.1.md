# Symmetry handoff artifact byte verifier v0.1

This implementation slice extends the existing physical-solver artifact verifier with a receiver-owned byte-binding check for the symmetry handoff defined by [Symtropy #4781 / PR #4784](https://github.com/Luminous-Dynamics/symtropy/pull/4784).

## Contract

The input is the exact handoff JSON plus a separate artifact manifest:

- The manifest binds the exact handoff bytes with `handoff_sha256`.
- It carries exactly 17 named artifact roles, each with an ID, normalized relative POSIX path, SHA-256, and byte length.
- Every manifest role must correspond to a digest-bearing field in the handoff envelope.
- The verifier rejects missing, extra, duplicate, stale, or role-mismatched entries.
- Files are resolved only beneath a canonical local artifact root. Parent traversal, absolute paths, malformed paths, absent files, non-regular files, and symlinks escaping the root are not admitted.
- File hashing is streamed in fixed 64 KiB chunks using the SHA-256 primitive already used by the physical solver contract.

The role inventory binds the source mathematical receipt and generator set; physical fixture, coordinate transform, transformed and original mass metrics, force covectors, constraint map and initial state; the complete scientific task semantics (task, domain, objective, feasibility constraints, initial/boundary conditions, observable mapping and answer interpretation); and the receiver-owned receipt.

## CLI

```sh
cargo run --locked --manifest-path research/physical-solver-contract/Cargo.toml \
  --bin symmetry-handoff-verify -- \
  --handoff handoff.json \
  --manifest artifact-manifest.json \
  --artifact-root ./bundle
```

The report distinguishes `PASS`, `FAIL`, and `UNRESOLVED`. A verifier execution that correctly reports FAIL or UNRESOLVED still exits successfully and prints the disposition; invalid arguments, unreadable inputs, or malformed contracts exit non-zero.

## Authority boundary

This is a **byte-identity** verifier only. It does not parse the source mathematics into an independently re-proved theorem, recompute mass congruence or force duality, validate the numerical error envelope, re-run the physical step map, or establish scientific task equivalence.

The report copies the source qualification, receiver physical status, step-equivariance status, and physical-equivalence authority exactly as declared in the handoff. It sets `physical_status_promoted=false` and `physical_claim_authority_created=false`. It never creates hard-pruning authority. A matching digest proves identity of bytes, not correctness of their contents.

The handoff schema and receiver-side physical verifier remain separate obligations. A complete physical PASS still requires the receiving verifier to independently recompute each of the nine physical obligations in the handoff contract.
