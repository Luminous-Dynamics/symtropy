# LOCO-OPTION-LOSS-LEDGER-001 — deterministic finite option-loss accounting

Status: standalone Rust reference / synthetic semantics. No physical controller, viability solver, thermal model, actuator model, recovery-topology authority, or actuation authority is added.

Parent: [Symtropy #4908](https://github.com/Luminous-Dynamics/symtropy/issues/4908). It consumes qualified finite option-set snapshots from [LOCO-FUTURE-OPTIONS-001 #4588](https://github.com/Luminous-Dynamics/symtropy/issues/4588), physical-state identities from [LOCO-JOINT-METROLOGY-001 #4783](https://github.com/Luminous-Dynamics/symtropy/issues/4783), and the interop work in [#4889 / PR #4907](https://github.com/Luminous-Dynamics/symtropy/pull/4907). Symthaea's retrospective mission replay remains [#11664](https://github.com/Luminous-Dynamics/symthaea/pull/11664).

## Research question

Given two sequential, evidence-qualified finite option snapshots, can the system produce a deterministic account of which canonical continuations disappeared, why they disappeared, whether they can be restored, and what independently bounded resources a qualified restoration consumed?

The purpose is not to invent a universal future-capability scalar. It is to make changes in optionality auditable over time.

## Implemented reference

A dependency-free Rust 2024 crate is isolated at `reference/` with its own workspace boundary and lockfile. Run with pinned Rust 1.96:

```sh
cargo fmt --manifest-path research/loco-option-loss-ledger-001/reference/Cargo.toml -- --check
cargo test --locked --manifest-path research/loco-option-loss-ledger-001/reference/Cargo.toml
cargo clippy --locked --manifest-path research/loco-option-loss-ledger-001/reference/Cargo.toml --all-targets -- -D warnings
```

The reference consumes two option snapshots and one transition-evidence record. It verifies:
- identical mission, robot, scenario, physical-branch and action-domain scope;
- monotonic event order and exact predecessor/result physical-state continuity;
- unique canonical action identifiers and finite action-domain membership;
- branch and action-domain completeness before a positive result;
- one reason-coded evidence item for every removed or newly available option;
- exact predecessor/result binding for restoration receipts;
- explicit qualified evidence for observation-partition refinement;
- separate resource costs for time, electrical energy, mechanical work, compute, observations and actuator-authority change.

Missing change receipts remain Unknown. Contradictory identity or state continuity is Rejected. Unknown or incomplete preconditions never produce a qualified restored option. The ledger never authorizes actuation.

## Evidence semantics

The reference distinguishes Qualified, Unknown, Inconclusive and Rejected. It does not turn missing evidence into zero cost, no loss, no restoration or successful mission completion.

A changed observation partition may legitimately change the deployable continuation set, but only when the exact predecessor and result partition digests are bound by an explicit receipt. A newly appearing option likewise requires a receipt that binds the exact physical-state predecessor and result. This keeps an observation update distinct from a physical repair or cooldown.

## Option-loss vector

Each removal retains its canonical action ID, cause, evidence receipt and reversibility declaration. Conditional or temporary loss must include named restoration preconditions. Irreversible loss cannot smuggle in a restoration path.

Each newly available option retains its cause, preconditions and resource costs as separate dimensions:
- time (ns);
- electrical energy (microjoules);
- mechanical work (microjoules);
- compute time (microseconds);
- observation count;
- signed actuator-authority delta in caller-declared fixed-point units.

No cross-unit arithmetic is performed. Incomparable outcomes remain incomparable.

## Current deterministic tests

The source contains 23 authored unit tests covering qualified and missing loss evidence, Unknown preconditions, duplicate evidence, state-reset rejection, branch/action-domain mismatch, incomplete manifests, event-order constraints, restoration binding, changed observation partitions, permutation invariance, irreversible and conditional loss, rejected preconditions, and separate resource accounting.

These are **authored tests**. Do not describe them as executed until the exact-head dedicated GitHub Actions workflow reports success. This environment does not have a Rust toolchain, so local Rust formatting, compilation, tests and Clippy are not claimed.

## Claim boundary

A passing finite reference proves only deterministic set-difference and receipt-binding semantics over supplied finite option snapshots. It does not prove branch completeness, physical recovery, thermal dynamics, actuator authority, route independence, restoration-cost accuracy, continuous viability, controller safety or hardware performance. Those require the upstream qualified evidence owners and held-out sequential physical experiments.

## Next composition step

The next adapter should consume the common-continuation set digest from #4588 and bind the ledger transition to the normalized joint-receipt packet in #4889. It should then appear in the same Symthaea #11664 causal replay as an append-only mission event. Do not add a second physical model or permit the ledger to override local safety, projection, RTA, HAL or independent safe-stop authority.
