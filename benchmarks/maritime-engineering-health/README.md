# Maritime engineering-health evaluator

Standalone offline Rust evaluator for the synthetic corpus at
benchmarks/maritime/engineering-health-corpus-v0.1.json.

## Run

From the repository root:

~~~sh
cargo fmt --manifest-path benchmarks/maritime-engineering-health/Cargo.toml -- --check
cargo test --locked --manifest-path benchmarks/maritime-engineering-health/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/maritime-engineering-health/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path benchmarks/maritime-engineering-health/Cargo.toml -- benchmarks/maritime/engineering-health-corpus-v0.1.json
~~~

The CLI emits the SHA-256 of the exact input bytes, an expected-versus-actual
result for each case, and an asset/configuration-bound capability-impact projection.
Evidence admission recomputes freshness from age_ms/max_age_ms and requires a
monotonic-and-UTC-bound clock. A source cannot be admitted merely because it
carries a quality label of "current".
The projection is read-only: it cannot authorize a transition or clear a restriction.
"No restriction indicated" applies only to the tested engineering-health evidence;
it is not a vessel-wide ODD pass. The CLI exits non-zero for any mismatch.

## Claim ceiling

A PASS proves only that this reference evaluator reproduces the frozen synthetic
contract outcomes. It does not establish machine diagnosis accuracy, machinery
health, safety, physical behavior, restoration authorization, class/statutory
compliance, MASS Code compliance or permission to operate a vessel. The crate has
no network, OT, controller, protection-system, actuator or equipment effects.
