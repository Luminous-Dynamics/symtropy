# CivDyn Authority Audit v0.1

A dependency-free Rust reference for small weighted councils and formal role/appeal topology.

## Exact weighted voting

For councils of 2–20 actors, the kernel enumerates all coalitions exactly and reports:
- nominal voting weights and shares;
- raw and normalized Banzhaf swing counts;
- Shapley–Shubik pivotality indices;
- minimal winning coalitions;
- veto-player and dictator flags;
- HHI, Gini and Banzhaf-to-nominal power ratios.

The exact coalition limit is deliberate. Larger councils must use an independently validated dynamic-programming/BDD implementation; this crate fails closed rather than approximating silently.

## Institutional roles and contestability

Formal capabilities are separate: propose, agenda control, summarize, edit public record, issue evidence, verify evidence, approve, disburse, audit, initial adjudication, appeal adjudication and rule change.

A caller declares separation-of-duties rules. The audit reports each actor who violates each declared rule, role holder counts/single points of control, appeal-channel availability and whether the final appeal adjudicator is independent of the initial adjudicators.

A report that the declared separation policy is satisfied is not a claim that a governance system is democratic, just or legitimate. Informal influence, wealth, coercion, expertise and social norms remain outside this v0.1 formal model.

## Synthetic controls

The executable covers:
- equal three-member majority;
- asymmetric [2,1,1] weights at quota 3, where effective power differs from nominal shares;
- [51,49] at quota 51, with a dictator/veto player;
- a deliberately concentrated role topology with independent appeal adjudication.

The fixtures are mathematical software controls, not empirical claims about a real institution.

## Run

```sh
cargo fmt --manifest-path research/civdyn-authority-audit/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-authority-audit/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-authority-audit/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-authority-audit/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-authority-audit/Cargo.toml --bin authority-audit-sweep
```
