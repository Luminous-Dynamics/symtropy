# Preservation Watch Reference

This zero-dependency Rust crate evaluates externally produced, time-bounded preservation observations and returns a deterministic watch state plus non-executing action proposals.

It does not collect or authenticate physical observations, verify signatures/hashes, operate readers, predict media lifetime, execute remediation, write/erase media, release keys or actuate hardware.

## Model

- Required observations come from a frozen WatchPolicy.
- Each observation carries a unique ID, target, signal, monotonic sequence, observation/expiry timestamps, evidence SHA-256, observer and provenance status.
- Expired or unverified evidence becomes Unknown; invalid provenance is a blocker.
- Missing required observations become Unknown, never Stable.
- A prior Alert is cleared only by a fresh, verified Healthy observation explicitly superseding that alert.
- Independent-copy thresholds are enforced only when the IndependentCopies signal is explicitly required. The count is supplied by the independent dependency/failure-domain authority; replica count alone does not establish independence.
- Disposition precedence is Blocked > InterventionRequired > Unknown > Watch > Stable.
- Every proposal includes required approvals/evidence and `auto_execute=false`.

## Run locally

~~~bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
~~~

The authoring environment has no Cargo, rustc or rustfmt; tests are authored but not executed here.

## Corpus

- `../fixtures/preservation_watch_cases_v0_1.json`: 15 synthetic known-answer/adversarial cases.
- `../preservation_watch_cases_v0_1.schema.json`: JSON Schema 2020-12 structure.
- `../PRESERVATION_WATCH_INTERVENTION_001.md`: protocol, OAIS mapping, policy and evidence boundary.

The fixture is not a physical observation dataset, failure-probability model or lifetime study.
