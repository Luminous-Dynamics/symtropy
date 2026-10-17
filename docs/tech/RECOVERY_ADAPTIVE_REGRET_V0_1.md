---
title: Adaptive Recovery Policies Under Scenario Uncertainty
version: 0.1
status: research-design
date: 2026-10-17
issue: https://github.com/Luminous-Dynamics/symtropy/issues/1894
---

# Adaptive Recovery Policies Under Scenario Uncertainty v0.1

## Claim ceiling

This is a synthetic, effects-disabled policy evaluator. It demonstrates scenario arithmetic, decision timing, and information constraints. It does not authorize timber recovery or establish real cost, weather, ecology, probability, or field performance.

## Paradigm shift: plan a policy, not one action

A static plan answers: "Which operation should we choose now?"

An adaptive policy answers:
- what can we do with the evidence visible now?
- which observation or inspection could separate otherwise indistinguishable futures?
- what action becomes admissible after that observation?
- which fallback remains safe when a condition fails?
- what is the worst regret of the resulting policy over a declared scenario set?

The product is a **policy** whose branches are tied to declared observations and tests—not a hidden future-state oracle that selects the best action after seeing a scenario label unavailable to the operator.

## V0.1 finite scenario semantics

The forestry corpus has five synthetic conditions and six policy candidates. Two scenarios share the same initial observation class `access_clear`, but their true handling costs differ. Before inspection, a policy must choose the same action for both. A qualified test can return `handling_low` or `handling_high`, after which a distinct branch is permitted. A second held-out corpus applies the same evaluator to photovoltaic second-life routing, with reuse blocked by an insulation-safety failure and test outcomes separating reuse from recycling.

Each scenario includes candidate operation costs and mandatory gate statuses:
- `PASS`: action is eligible in this synthetic case;
- `BLOCK`: action is unavailable under a mandatory constraint;
- `UNKNOWN`: action is not eligible until the gate is resolved.

A policy that selects a BLOCK or UNKNOWN operation in any scenario is ineligible across the complete scenario set. A high cost advantage never compensates for a failed gate.

Inspection incurs its explicit cost exactly once per inspected scenario. It reveals only `post_inspection_observation`; it does not grant permission or clear unrelated gates.

## Regret definition

For scenario (s), define the declared reference lower bound:

```text
best_cost(s) = min action_cost(a, s)
               over actions whose mandatory gate is PASS
```

For a feasible policy (pi):

```text
policy_cost(pi, s) =
    chosen_action_cost(pi, s) + inspection_cost(pi, s)

regret(pi, s) = policy_cost(pi, s) - best_cost(s)

total_cost(pi) = sum_s policy_cost(pi, s)
worst_regret(pi) = max_s regret(pi, s)
```

The scenario best cost is a **clairvoyant lower bound**: it assumes the scenario is known before acting and can be unattainable without an inspection. Regret includes the inspection burden, so sensing is not treated as free.

No defensible scenario probabilities are supplied. Therefore `total_cost` and `total_regret` are explicitly **unweighted scenario sums**, not expected values or probability forecasts. The frontier uses two declared minimize metrics: unweighted total scenario cost and worst-case regret. It does not collapse them into a weighted score.

## Frozen synthetic cases

Costs are synthetic euro cents per scenario. The fixture compares:
- fixed ground recovery;
- fixed cable recovery;
- fixed aerial lifting;
- retaining in every case;
- a policy adapted to initial observable context;
- inspection followed by an adapted action for the two otherwise indistinguishable access-clear scenarios.

The expected result is:
- three policies are ineligible somewhere in the scenario set because a selected action has a BLOCK gate;
- `retain_all` remains feasible but has total scenario cost 1500 cents and worst regret 200 cents;
- `visible_context_adaptive` has total scenario cost 900 and worst regret 70;
- `inspect_then_adapt` spends 40 cents on inspection in each of two scenarios, has total scenario cost 910 and worst regret 40;
- the Pareto frontier preserves both adaptive policies, because the inspect policy reduces worst regret but increases total scenario cost by 10 cents.

These figures are invented to test logic; they are not timber prices or recommended operating economics.

The second, photovoltaic-specific corpus has three conditions and three policies. It distinguishes performance/visual screening from the separate insulation-safety outcome. The expected frontier retains `recycle_only` (total 530 synthetic cents; worst regret 100) and `inspect_then_route` (total 550; worst regret 60). This is a constructed trade-off only. The evaluator reuses the protocol but retains a separate PV profile; the forestry objective values and thresholds are not imported into PV decisions.

## Required hostile tests

- **Feasibility:** zero BLOCK/UNKNOWN action choices accepted as a fully feasible policy.
- **Nonanticipativity:** a policy cannot branch by hidden scenario ID when the observable information state is the same.
- **Cross-profile transfer:** the same finite evaluator and independent oracle pass a separate PV second-life corpus without reusing the forestry profile ID or thresholds.
- **Inspection accounting:** an inspection cost is charged exactly once, and post-test action branches require a declared test outcome.
- **Uncertainty / regret:** exact agreement with an independent finite scenario oracle for every policy's feasibility, total cost, worst regret, and frontier.
- **Action coverage:** every observation class has one declared decision; missing or extra observation rules fail closed.
- **Reproducibility:** plan/policy/scenario ordering does not change semantic metrics or the frontier.
- **No fictitious probabilities:** receipt explicitly says unweighted scenario sum and no probability distribution.
- **Claim ceiling:** every receipt carries `NO_ACTUATION_AUTHORIZED`.

The next extension should add contingent scenario trees with more than one inspection/replanning step, correlated disturbance/access/quality changes, and robust-regret comparisons against fixed-policy baselines. The same-policy semantics now have a second synthetic application: [forest corpus](recovery-adaptive-regret-corpus-v0.1.json) and [PV second-life corpus](recovery-adaptive-regret-pv-corpus-v0.1.json). Do not model transition probabilities until a defensible domain source supports them.

## Research grounding

- Chen et al., *Cost-effective and reliable power system transitions under deep uncertainty: A multi-scenario multi-objective robust decision making framework* (April 2026): distinguishes constraint and objective uncertainty; compares scenario-based, stochastic, robust, and min-max regret planning across 240 power-system scenarios. Results are specific to that domain: https://doi.org/10.1016/j.ijepes.2026.111766
- Li et al., *How can urban wastewater and reclaimed water systems tackle an uncertain future without regret? — Solutions based on adaptive pathway planning* (15 June 2026): adapts pathways to monitored conditions; its reported cost/resilience improvements are case-specific: https://doi.org/10.1016/j.jenvman.2026.130062
- Marinna et al., PV second-life qualification study (2025), uses visual inspection, insulation resistance, I–V curves and electroluminescence on 76 modules; the sample-specific 68% reuse result is not a target for this synthetic case: https://doi.org/10.1016/j.jclepro.2025.144941
- Thakur & Mukherjee, *Managing demand uncertainty and supply disruption risks: a multi-stage stochastic optimization framework for efficient sourcing strategies* (July 2026): hybrid scenario tree and rolling-horizon replanning for joint demand/disruption uncertainty: https://doi.org/10.1016/j.cie.2026.112047

These papers motivate methods, not transferable timber thresholds.

## Related work

- [RDO-002 / PR #1876](https://github.com/Luminous-Dynamics/symtropy/pull/1876): interval-robust Pareto frontier.
- [RECOVERY-ORACLE-001 / PR #1851](https://github.com/Luminous-Dynamics/symtropy/pull/1851): evidence provenance, gate contracts and receipt.
- [RDO-003 issue #1894](https://github.com/Luminous-Dynamics/symtropy/issues/1894): this work.

## Non-goals

- no real scenario probabilities or statistical expectations;
- no automatic operation selection or physical actuation;
- no generic transfer of safety thresholds across domains;
- no claim that scenario coverage is complete;
- no claim of actual economic or environmental improvement from synthetic results.
