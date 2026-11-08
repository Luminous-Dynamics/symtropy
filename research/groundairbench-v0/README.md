# GroundAirBench v0 — episode composition manifest

**Status:** synthetic manifest-validator seed. This is not a physical model, scheduler, controller, safety case, flight result or energy-transfer result.

This first executable slice composes upstream evidence by immutable receipt reference. It does not copy the physical semantics owned by #3448/#3451, #3472, #3980/#3983, #4080/#4082, #4115/#4121 or #4141.

## Required receipt owners

Every FIXTURE references exactly one receipt from each owner:
1. SOURCE_CATALOGUE
2. PHYSICAL_SUPPORT
3. OBSERVABILITY
4. SESSION_LIFECYCLE
5. SITE_CAPACITY
6. ENERGY_METROLOGY
7. STANDARDS_APPLICABILITY

References carry opaque receipt ID, SHA-256, evidence class, disposition and claim ceiling. Upstream workstreams remain authoritative for their own receipt meaning.

## Key invariant

A high signal-strength proxy cannot override AMBIGUOUS, UNKNOWN, stale or out-of-domain observability. Any critical receipt whose disposition is not PASS requires expected_session_state=NOT_ADMITTED. Even when every synthetic receipt says PASS, v0 can only emit SYNTHETIC_ONLY_NO_POWER. There is no operational admission state.

## Claim ceiling

CONFORMS means only that the manifest has complete, internally consistent synthetic receipt references and typed metric definitions. It does not validate the referenced upstream artefacts, prove digest availability, physical feasibility, energy transfer, safety, compliance, aircraft authorization or flight.

## Files and commands

- episode.schema.json — Draft 2020-12 structural schema.
- ambiguous-pose.synthetic.json — high-signal / ambiguous-pose negative control.
- episode.NOT_RUN.template.json — empty template; NOT_RUN is not a pass.
- tools/groundairbench-manifest-validator — dependency-pinned Rust CLI and independent semantic tests.

```sh
cargo test --locked --manifest-path tools/groundairbench-manifest-validator/Cargo.toml
cargo run --locked --manifest-path tools/groundairbench-manifest-validator/Cargo.toml -- research/groundairbench-v0/ambiguous-pose.synthetic.json
```
