# Construction Option + Evidence Co-design Core — v0.1

**Status:** synthetic bounded composition of robust production optioneering and exact observation-campaign planning. It does not design buildings, qualify sensors, authorize site access or accept physical construction work.

Parent: [CONSTRUCTION-OPTION-EVIDENCE-CODESIGN-001 #5250](https://github.com/Luminous-Dynamics/symtropy/issues/5250).

This crate composes:
- [production optioneering #5238](https://github.com/Luminous-Dynamics/symtropy/pull/5238);
- [observation campaign planner #5240](https://github.com/Luminous-Dynamics/symtropy/pull/5240);
- the resource, occupancy, task and acceptance owners in their existing dependency chain.

## Core decision

A robust-feasible option is not discarded simply because its construction-only objective vector is dominated. The option's own evidence campaign may change the joint ranking.

Each robust-feasible option requires one option-specific campaign package, bound to:
- the same functional contract and project context;
- the exact option design digest;
- one floor, site epoch, frame and clock domain;
- the functional contract's currency;
- an explicit observation-cost boundary treatment.

The evaluator calls the existing exact bounded campaign solver for each option. It does not duplicate set-cover, spatial conflict or capacity semantics.

A valid Avoid alternative can have zero work needs and zero observation actions. The campaign solver must prove this empty campaign complete, with zero observation burden; it must not invent a scan merely because a campaign input exists.

## Cost boundary

- `AdditionalToOptionLifecycleCost`: add campaign cost once to construction lifecycle cost.
- `AlreadyIncludedInOptionLifecycleCost`: report campaign cost separately but do not double-count it in combined cost.

The boundary reference is a caller-supplied accountable statement; the code cannot inspect an external cost ledger to prove it.

## Search and optimality

A complete campaign found before the node budget expires is feasible but not globally minimum-burden. Only exhaustive, complete campaign results enter the proven joint Pareto frontier. If any robust-feasible option has unresolved campaign search, `joint_frontier_proven` is false. Exhaustively proven campaign infeasibility is a resolved exclusion.

Construction elapsed minutes, observation duration minutes and observation scheduled span milliseconds are reported as separate dimensions. They are not silently summed across different time semantics.

## Local commands

```sh
cargo fmt --manifest-path research/construction-option-evidence-codesign-v0-1/Cargo.toml -- --check
cargo test --manifest-path research/construction-option-evidence-codesign-v0-1/Cargo.toml --locked
cargo clippy --manifest-path research/construction-option-evidence-codesign-v0-1/Cargo.toml --locked --all-targets -- -D warnings
cargo bench --manifest-path research/construction-option-evidence-codesign-v0-1/Cargo.toml --locked --bench joint_scale
```

The current synthetic Rust corpus includes fourteen tests, including construction-only dominated option reversal after campaign burden, explicit included/additional cost treatment, robust-infeasible short-circuit, node-budget exhaustion, exact small Pareto oracle, mismatched base and action design bindings, action clock-domain mismatch, missing campaign input, zero-work Avoid campaign, proven campaign infeasibility, empty open-stage frontier, and read-only preservation of both planning snapshots during joint evaluation.

The hosted benchmark measures composition p50/p95/p99 across 2/4/8/16 alternatives with 25 repetitions each. It reports campaign search nodes and joint frontier size separately. These are runner-specific measurements, not a construction productivity claim or field SLO.

## Claim ceiling

V0.1 establishes synthetic, bounded option/campaign composition only. It does not establish structural adequacy, inspection validity, site safety, real survey coverage, construction productivity, cost/carbon savings or standards conformity.
