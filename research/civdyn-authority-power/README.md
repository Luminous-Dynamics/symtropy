# CivDyn Authority Power v0.1

Exact small-council weighted-voting power and configurable role/separation-of-duties audits.

## Why this exists

Nominal voting weights do not fully describe coalition power. A role can also be formally held by many actors while agenda, evidence, disbursement, record-editing or appeal functions are concentrated in a small subset.

This crate deliberately emits separate vectors:
- nominal weight and normalized Banzhaf power;
- Shapley–Shubik pivotality;
- veto/dictator flags and removal robustness;
- minimal winning coalitions;
- HHI/Gini of nominal weights and effective Banzhaf power;
- role coverage, single-holder roles and configured role-combination conflicts;
- proposal-level self-approval, approval/disbursement, evidence self-verification and appeal-independence controls.

It does not emit a universal democracy/decentralization score.

## Exact voting scope

- Exact coalition enumeration is capped at 20 voters (1,048,576 coalitions).
- Banzhaf swings count coalitions where a voter is critical.
- Shapley–Shubik is calculated from the factorial-weighted probability that each voter is pivotal in a random ordering.
- Minimal winning coalitions are explicitly enumerated.
- Veto means every winning coalition contains the voter.
- Dictator means that voter alone meets quota and all other voters together do not.
- Removal robustness means all remaining members together can still meet quota; it does not assert that they will agree to do so.

The cap is intentional. Larger councils need a separately validated dynamic-programming/BDD or approximation path, not a silent exponential blow-up.

## Authority roles and policy

The default example policy marks Approver+Disburser, EvidenceIssuer+EvidenceVerifier and OriginalAdjudicator+AppealAdjudicator as forbidden combinations. AgendaController+Summarizer, AgendaController+PublicRecordEditor and RuleAmender+Auditor are advisory concentrations. These are configurable audit rules, not universal laws.

A deployment must publish the policy, its rationale and any exceptions. The code reports nominal role coverage and concentration; it cannot discover informal or coercive power from a permission graph.

## Run

```sh
cargo fmt --manifest-path research/civdyn-authority-power/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-authority-power/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-authority-power/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-authority-power/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-authority-power/Cargo.toml --bin authority-power-sweep
```

## Claim ceiling and next gates

This benchmark measures the declared formal rules. It does not determine democratic legitimacy, social justice, real-world influence or whether a voting rule is normatively appropriate. Those require empirical and constitutional context. The next research stage is to compare exact indices against independently maintained implementations, add BDD/dynamic programming for large games, time-varying delegation, veto/emergency powers, appeals, removal attacks and empirical influence/agenda evidence.
