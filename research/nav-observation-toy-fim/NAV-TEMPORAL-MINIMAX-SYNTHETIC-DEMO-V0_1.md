# Temporal mission minimax synthetic demo v0.1

**Purpose:** reproduce one exact finite counterexample where capture-time accuracy and decision-time value rank sensors differently.

**Claim boundary:** all probabilities and losses below are synthetic mathematical inputs. This is not a UWB sensor measurement, physical transition calibration, dataset result, safety claim, or SOTA comparison.

## Scenario

Two mission-persistent models share a 50/50 prior over inside/outside state and equal prior mass over nominal/shared-cause regimes.

- Action 1 is perfectly informative at capture, but has 100 ns evidence age. Under model 0, the inside-to-outside probability before use is 0.5. Under model 1, that transition is zero.
- Action 2 has symmetric 10% observation error and zero evidence age under both models.
- Both actions cost 100 ns, 1 microjoule and 1 byte. The latency budget is 100 ns, so only one acquisition can be used.
- False-safe and false-degraded losses are both 1.

## Exact expected result

The shared mission-global minimax policy chooses action 2.

| Quantity | Exact numerator / denominator | Value |
|---|---:|---:|
| Model 0 loss under selected policy | 10^29 / 10^30 | 0.1 |
| Model 1 loss under selected policy | 10^29 / 10^30 | 0.1 |
| Worst-case mission risk | 10^29 / 10^30 | 0.1 |
| Model 0 model-aware oracle | 10^29 / 10^30 | 0.1 |
| Model 1 model-aware oracle | 0 / 10^30 | 0 |
| Worst-case minimax regret of selected risk policy | 10^29 / 10^30 | 0.1 |
| Enumerated observable policy trees | — | 18 |

Action 1 has model-0 use-epoch risk 0.25 and model-1 risk 0. Action 2 has 0.1 risk in both models. The static capture-only intuition prefers action 1; robust use-epoch decision loss prefers action 2.

## Reproduce

From the repository root:

```sh
cargo run --manifest-path research/nav-observation-toy-fim/Cargo.toml --bin navigation_temporal_minimax_demo
```

The binary asserts the selected action and exact common denominator before printing the per-model losses, robust risk, oracle risks, regret, and policy-tree count.

## Next challenge

This single-acquisition example does not yet demonstrate a benefit from a second-stage contingent action or a gap between mission-persistent and branch-switching objectives. Those require a multi-action fixture with a latency budget that permits a second acquisition and model-specific outcome branches. They are separate benchmark cases, not implied by this demo.
