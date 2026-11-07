# CivDyn Normative Premise Audit v0.1

A dependency-free evaluator for checking explicitly declared normative premises against a frozen outcome snapshot.

## The methodological boundary

This crate does **not**:
- derive morality from thermodynamics or resource efficiency;
- select one authoritative moral framework;
- infer human preferences or consent from free text;
- simulate agents or calculate physical resource flows;
- assign a scalar "morality", "legitimacy" or universal welfare score.

It consumes a frozen outcome snapshot and asks whether the snapshot's supplied evidence supports, contradicts or leaves undetermined each explicit premise. The resource ledger remains the sole owner of stock-flow accounting. A ledger receipt's `source_verified` flag and digest are source-supplied assertions; this crate does not cryptographically authenticate the source or verify the digest.

## Premise families

- **Aggregate consequence:** mean water/energy service floors.
- **Equal consideration:** per-group service floors and maximum service gaps.
- **Distribution:** maximum group burden share.
- **Rights constraints:** recorded recipient consent for transfers and a recorded exit route for every group.
- **Intergenerational stewardship:** resource closing-stock floors and no-net-depletion rules.
- **Physical-accounting qualification:** source-verified, unit-correct resource-ledger receipts with bounded conservation residuals.

These are example premises, not an exhaustive taxonomy and not claims that any one profile is objectively correct.

## Tri-state semantics

- `supported`: all required observations are present and satisfy the declared rule.
- `contradicted`: at least one observed fact violates the declared rule.
- `undetermined`: no known violation is present, but required evidence is missing or unverified.

Within a profile, any contradicted premise makes the profile contradicted; otherwise any undetermined premise makes it undetermined; only all-supported premises yield supported. A known contradiction is not hidden by unrelated missing data.

## Synthetic control

The executable evaluates the same two-community snapshot under four separate profiles:
- aggregate consequence;
- equal consideration;
- rights constrained;
- intergenerational stewardship.

The fixture is intentionally constructed so aggregate service means pass while a group-level service gap, a non-consensual transfer, and food-stock depletion contradict other premises. The receipts and their digests are synthetic labels; they are not real source attestations.

The point is not to declare which profile should govern. The point is to show that different declared premises can reach different, inspectable conclusions about the same frozen outcome evidence.

## Run

```sh
cargo fmt --manifest-path research/civdyn-normative-audit/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-normative-audit/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-normative-audit/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-normative-audit/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-normative-audit/Cargo.toml --bin normative-audit-sweep
```

## Claim ceiling

This validates tri-state rule execution and evidence-missing semantics on synthetic software fixtures only. It does not establish an empirical moral conclusion, prove that consent records are accurate, validate the physical ledger itself, or derive normative conclusions from physical laws. Real evaluation requires source rights, authority and provenance checks, explicit normative review, contested-premise handling and independent evaluation.
