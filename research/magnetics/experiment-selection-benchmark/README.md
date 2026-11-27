# Magnetic experiment-selection benchmark

**Programme:** MAG-AM-EXPERIMENT-SELECTION-BENCH-001  
**Issue:** #7794  
**Status:** Rust 2024 trace evaluator; synthetic policy traces only

## Purpose

Compare experiment-selection policies on matched task definitions, seed sets, budgets and independent holdouts. This crate computes decision-quality metrics from recorded traces; it does not implement GP/BO/NSGA-II/active-learning policies or operate equipment.

## Why a richer baseline?

The 2026 active-learning literature warns that random search alone is not a sufficient baseline. Human/domain-prior-informed selection, DOE, space filling, diversity and representativeness need to be compared too. Active learning can chase noisy or physically irrelevant areas if uncertainty and feasibility are poorly modelled.

## Metrics

- confirmed feasibility, failures and unresolved outcomes;
- iteration and total cost to first qualified experiment;
- simple regret to known oracle on synthetic tasks;
- 2D Pareto hypervolume under a frozen reference point;
- false-feasible and unresolved-feasible selections;
- holdout interval coverage and mean interval width;
- holdout feasibility Brier score;
- selection diversity and unique-candidate fraction;
- strict lot/build/process-revision holdout isolation.

No universal weighted score is emitted. A method that produces high hypervolume but selects physically infeasible experiments is not declared the winner.

## Fair-comparison requirements

- identical task, frozen candidate space, target, acceptance policy and initial design;
- matched random seed IDs and experiment budgets;
- same holdout identities and actual outcomes across policies;
- no training/holdout group overlap under the declared split unit;
- every initiated experiment has one outcome row, including missing/fail/indeterminate;
- physical comparisons use lot/build/revision holdouts, not random row splits;
- all policy traces and seeds should be published.

## Files

- [Evaluator source](evaluator/src/lib.rs)
- [Trace schema](evaluator/experiment-selection-trace-v0.1.schema.json)
- [Synthetic known-answer fixture](evaluator/fixtures/synthetic-experiment-selection-v0.1.json)
- [Benchmark plan](benchmark-plan.toml)
- [Source register](fixtures/experiment-selection-source-register-v0.1.json)

## Claim ceiling

The synthetic known-answer validates trace and metric semantics only. It does not establish magnetic route performance, real experiment savings or SOTA superiority.
