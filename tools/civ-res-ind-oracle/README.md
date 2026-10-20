# CIV-RES-IND-002 — Rust reference oracle

This package independently recomputes the hand-checkable synthetic corpus for
[CIV-RES-IND-001](https://github.com/Luminous-Dynamics/symtropy/issues/1708).
It intentionally has no GPU, database, network-service, Holochain, or model-weight
dependency. It uses the repository's pinned Rust dependencies and root lockfile.

Run from the repository root:

~~~sh
cargo test --locked --manifest-path tools/civ-res-ind-oracle/Cargo.toml
cargo run --locked --manifest-path tools/civ-res-ind-oracle/Cargo.toml -- \
  --fixture docs/research/CIV_RESILIENCE_RESTART_FIXTURE_V0_1.json \
  --manifest docs/research/CIV_RESILIENCE_RESTART_MANIFEST_V0_1.json \
  --source tools/civ-res-ind-oracle/src/main.rs \
  --lock Cargo.lock
~~~

The verifier checks the manifest-pinned SHA-256 digests for the raw fixture bytes,
evaluator source, and `Cargo.lock` before evaluation. Any digest mismatch is a
non-zero exit. The output report sorts scenarios by ID, carries the exact
fixture/source/lock identities, and independently compares recomputed values
with every declared expected field.

The service allocator uses the fixture's explicit
`service_power_units_per_service_unit` ratio and its declared service-priority
order. Unknown preconditions block production. A completed reference operation
counts only after essential-service floors and every declared production
precondition are satisfied.

## Evidence and limitations

- The corpus is synthetic, not field data. It does not calibrate Symtropy or
  qualify real industrial capacity.
- The S06 result validates only the *shape* of the future-blindness contract.
  Without policy action traces, it does not establish action-level future
  blindness.
- A matching SHA-256 digest binds bytes; it does not authenticate an author,
  instrument, physical event, or truth claim.
- A passing oracle establishes only these frozen accounting semantics—not that a
  recovery policy beats a baseline, that any workshop is suitable, or that a
  physical plant may restart.
- Keep safety/quality, service floors, conforming output, resource consumption,
  and per-scenario failures as separate results. Do not create one aggregate
  “industrial efficiency” or “resilience” score.
