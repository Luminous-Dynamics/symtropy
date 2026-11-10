# ANYTIME-DRIVING-IMAGINATION-001

**Status:** deterministic inference-budget allocator reference; no world model, simulator or training integration  
**Canonical issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4577  
**Parent benchmark:** https://github.com/Luminous-Dynamics/symtropy/issues/4550  
**Related selector:** https://github.com/Luminous-Dynamics/symtropy/issues/4568

## Research question

Can a driving planner use a variable amount of world-model imagination according to decision value, action-ranking uncertainty, and the time remaining before it must act—rather than spending the same inference budget on every frame?

The reference allocator consumes qualified, per-rollout marginal decision-regret reductions, measured compute cost and conservative rollout-latency bounds. It schedules additional rollouts across multiple simultaneous decision opportunities.

## Why this matters

World-model visual quality is not a sufficient objective. A world model must help the agent select better actions in a closed loop. More inference-time compute can improve decision quality, but spending too long imagining can make an otherwise good action arrive too late.

The research question is therefore not "how many rollouts can the model afford?" It is "which extra rollout has the greatest expected decision value per unit compute, while preserving the time required to execute and recover?"

## Algorithm

Each opportunity supplies a marginal expected-regret-reduction schedule:

`g[0], g[1], ... g[n]`

The schedule must be non-increasing (diminishing marginal value). An additional rollout is eligible only when:
- the model and SAM evidence are admitted and identity-bound to the same checkpoint, observation and context;
- the policy observation is visible, fresh and available at the planning tick;
- rollout compute cost and conservative latency bound are positive;
- the opportunity has not reached its maximum rollout count;
- the next rollout fits within the remaining decision window after the actuation/recovery reserve.

The allocator repeatedly chooses the next eligible rollout with the greatest exact ratio:

`marginal_expected_regret_reduction / compute_units_per_rollout`

The comparison uses 128-bit cross multiplication, not floating point. Stable opportunity ID tie-breaking makes the result deterministic. The allocator stops when the shared compute budget is exhausted or no opportunity has a positive marginal gain that fits its deadline.

The reference implementation is intentionally bounded to **64 simultaneous decision opportunities** and **128 marginal rollouts per opportunity** (8,192 maximum declared rollout slots) to keep allocation work deterministic and resource-bounded. It selects the next best candidate with a linear minimum scan rather than sorting all candidates at every allocation step.

This is a deterministic greedy priority scheduler, **not a globally optimal knapsack solver**. Its output is a reproducible baseline for comparison against fixed, uniform, uncertainty-only and other inference-budget policies.

## Deadline invariant

For each opportunity with k additional sequential rollouts:

`planning_tick + k * latency_bound_per_rollout + actuation_recovery_reserve <= decision_deadline`

The latency bound is a caller-qualified conservative per-rollout upper bound in the same tick domain. It must not be silently replaced with a mean or assumed wall-clock estimate. The sum is used as an explicit conservative schedule bound, not as an assertion that separately measured p95 values mathematically compose into a joint p95.

The allocator rejects overflow and never spends the reserved actuation/recovery window. If the next rollout cannot fit, it leaves the compute unused for that opportunity. Independent runtime assurance and fallback remain outside this component.

## Comparison policies for empirical evaluation

- Fixed rollout count per decision.
- Uniform compute allocation across decision opportunities.
- Entropy/disagreement-proportional compute.
- Prediction-error-proportional compute.
- Decision-margin / near-boundary compute.
- SAM expected-decision-value-per-compute.
- SAM recoverability/fragility-aware compute.

Compare compute-matched and latency-matched studies separately. Report regret-vs-compute curves, task success, deadline compliance, worst-family performance, option preservation, memory, energy and p50/p95/p99 latency. Do not merge them into one weighted score.

## Run

From this directory:

```sh
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

The repository workflow uses Rust 1.96.0.

## Claim ceiling

Passing tests establish deterministic scheduling over finite supplied estimates and timing budgets only. They do not prove:
- SAM or world-model estimates are calibrated;
- the declared latency bound is physically valid;
- a rollout improves the selected action in the simulator;
- adaptive inference beats fixed compute or improves closed-loop driving;
- a vehicle is safe, roadworthy or authorized for public-road operation.

Empirical usefulness requires matched compute, latency and task-outcome experiments under #4550.
