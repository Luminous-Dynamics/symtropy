# Textile Attachment Removal Contract v0.1

A dependency-free Rust contract validator for garment attachment-removal
attempts. It is the first executable P0 slice for
[TEXTILE-ATTACHMENT-REMOVAL-001 #4941](https://github.com/Luminous-Dynamics/symtropy/issues/4941).

## Paradigm

Garment disassembly is a reverse-manufacturing task. The benchmark must prove
more than attachment detection: it must preserve the distinction between a
planned action, a simulated result, a physically observed action, and an
independently inspected physical removal.

## Contract checks

- Stable garment revision, specimen, attachment, task, attempt and tool IDs.
- Declared join method and requested disposition.
- Unknown join construction cannot be executed; it must abstain or enter manual
  review.
- Evidence IDs and SHA-256 digests are required for supplied evidence refs.
- Synthetic fixture evidence cannot prove an executed removal.
- Simulated removal remains simulation-only and cannot be promoted by attaching
  an independent-inspection label.
- Physical execution requires calibrated peak-force observation.
- Physical Removed/Partial outcomes require independent post-inspection and a
  separate textile-damage observation.
- Removed/Partial outcomes require a closed mass-balance ledger and positive
  recovered attachment mass.
- Planned and abstained attempts cannot claim positive recovered output.
- Force-limit exceedance remains a hard finding; it is never hidden in a
  composite success score.
- Invalid records receive the conservative ContractOnly evidence ceiling.

Mass closure is explicit:

`input = textile output + attachment output + residual + unexplained`

All masses use integer milligrams. The unexplained bucket is retained rather
than silently redistributed. Damage dimensions are recorded separately from
removal completion.

## Run locally

~~~sh
cargo test --manifest-path tools/textile-attachment-removal-contract/Cargo.toml
cargo fmt --manifest-path tools/textile-attachment-removal-contract/Cargo.toml --all -- --check
cargo clippy --manifest-path tools/textile-attachment-removal-contract/Cargo.toml --all-targets -- -D warnings
~~~

## Deliberate non-goals

This crate does not plan or execute robot motion, choose tools, estimate
physical material composition, authenticate external evidence, establish
recycling eligibility, certify worker safety, or calculate lifecycle impact.
It is a semantic/evidence gate for a named attempt record.

## Claim ceiling

A passing simulated attempt remains simulation-only. A passing physical
attempt requires the named specimen, calibrated force observation, independent
post-inspection, damage record and closed mass balance. Neither outcome proves
industrial disassembly throughput, downstream fibre quality, environmental
benefit or commercial readiness.
