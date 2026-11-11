# Behavioral-set diagnosability benchmark

A read-only reference evaluator for uncertainty-bounded **sets of fault-response trajectories**. This is a finite interval-tube conformance method, not a general nonlinear reachability solver.

## Why it exists

Pointwise residual or marginal interval comparisons can discard temporal correlation. In the frozen first case, both fault hypotheses have identical pooled marginal ranges at every sample time, but their finite sets of admissible two-sample trajectories are disjoint. A marginal-envelope baseline cannot distinguish them; the joint behavioral-set comparison can.

The method is grounded in established geometric/behavioral-set fault diagnosis research, especially Sheikhi et al. (2026), *Annual Review of Control, Robotics, and Autonomous Systems* 9:147–179, DOI 10.1146/annurev-control-030123-015422. This benchmark is an executable engineering application, not a claim to invent behavioral-set diagnosis.

## v0.1 math

Each trajectory tube has value intervals [lᵢ,uᵢ], timestamp uncertainty δᵢ, and a declared maximum absolute slope bound sᵢ. The evaluator conservatively expands:

- lowerᵢ = lᵢ − sᵢ δᵢ
- upperᵢ = uᵢ + sᵢ δᵢ

Two trajectory tubes (axis-aligned boxes in the fixed time-coordinate space) are disjoint when at least one sample-coordinate interval is strictly disjoint. A measurement principle separates a fault pair only if **every** admissible tube from fault A is disjoint from **every** admissible tube from fault B. If one cross-hypothesis tube pair intersects, unique isolation is not established by that principle.

All tubes must use the same qualified timebase, exact event-relative sample grid, units and independence group for a given measurement principle. No dynamic time warping, post-hoc phase shift or sample reordering is accepted. Shared dependency groups cannot be counted as independent principles.

The evaluator also computes a naive pooled marginal-envelope baseline at every time coordinate, enabling direct comparison with joint trajectory-set separation.

## Frozen suite

Seven cases cover: identical marginals but disjoint joint trajectories; a single overlapping tube pair; timestamp uncertainty removing apparent separation; unqualified timebase; mismatched sample grid; shared common-cause dependency collapsing two apparent principles; and prohibited post-hoc phase alignment.

## Run

```bash
cargo test --locked -p symtropy-behavioral-set-diagnosability-bench
cargo clippy --locked -p symtropy-behavioral-set-diagnosability-bench --all-targets -- -D warnings
cargo run --locked -p symtropy-behavioral-set-diagnosability-bench -- benchmarks/maritime-behavioral-set/trajectory-corpus-v0.1.json --source-commit <40-hex> --corpus-sha256 <64-hex>
```

## Claim ceiling

The fixtures are synthetic. A pass demonstrates only the interval-box arithmetic and contract handling for these frozen cases. It does not validate an engine model, measured trajectory tubes, vessel clocks, physical diagnosability or shipboard safety. Future measured use requires independently qualified sensors/timebase, full-run fault truth and held-out asset/campaign groups. Nonlinear reachability, zonotopes and distributionally robust behavioural sets remain future comparators.
