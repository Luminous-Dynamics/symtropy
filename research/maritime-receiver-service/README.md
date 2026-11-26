# Maritime Receiver Service v0.1

**Status:** synthetic receiver-outcome evaluator; exact-head qualification pending.
**Tracking:** [#7554 — receiver outcomes and protected service floors](https://github.com/Luminous-Dynamics/symtropy/issues/7554).
**Related:** #6662, #6578, #7436, #7488, #7505, #7519, #7528, #7544, #3683.

## What this measures

The service endpoint is independently verified receiver acceptance. Vessel departure and terminal exit are not delivery. The evaluator reports one summary per (cohort, explicit unit) and does not aggregate containers, TEU, tonnes or packages together.

## Outcome semantics

- Independently verified receiver acceptance enters the on-time or late numerator according to the declared deadline.
- Verified non-acceptance by the measurement horizon is a confirmed non-success.
- Verified loss/damage is a confirmed non-success.
- Terminal-exit-only, censored, disputed and unverified outcomes remain unresolved.
- Unit reconciliation failure invalidates a configured protected floor.

## Conservative service bounds

For scheduled quantity N, verified on-time quantity A and unresolved quantity C:
- observed rate among adjudicated outcomes = A / adjudicated_quantity;
- lower bound = A / N;
- best-case upper bound = (A + C) / N.

A protected floor F passes only when the lower bound reaches F; it fails only when the upper bound is below F; otherwise it is UNKNOWN. This avoids calling missing/censored cargo a failure or silently counting it as success.

## Frozen corpus

The fixture has ten scenario cases: all-on-time, terminal-exit-only, late acceptance, censored bounds, confirmed non-acceptance, verified loss/damage, unverified acceptance, explicit separation of container and TEU groups, unit reconciliation failure, and acceptance after the measurement horizon.

The dependency-free Rust reducer has 14 unit tests. A deterministic TSV receipt is compared with a separately implemented standard-library Python reference.

## Run locally

```sh
cargo fmt --manifest-path research/maritime-receiver-service/Cargo.toml -- --check
cargo test --locked --manifest-path research/maritime-receiver-service/Cargo.toml
cargo clippy --locked --manifest-path research/maritime-receiver-service/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path research/maritime-receiver-service/Cargo.toml --bin receiver-service-audit > receiver-service-receipt.tsv
python3 research/maritime-receiver-service/scripts/independent_reference.py --receipt receiver-service-receipt.tsv
```

## Limitations and claim ceiling

- Synthetic arithmetic/state contracts only; no live custody or receiver connector is included.
- The evaluator does not authenticate source signatures, participant identity or rights; it consumes an already-qualified outcome flag.
- It does not estimate causal treatment effects or confidence intervals; those require preregistered field design and cluster-aware statistical analysis.
- It does not replace Mycelix permissions/custody or #6662 receiver closure ownership.
- The fixture is not evidence of actual DGT or any other port's performance.

Keep three distinct scorecards: CPPI-compatible vessel time; terminal resource productivity; and unit-level receiver acceptance / protected service. A gain on one cannot be used as a proxy for another.