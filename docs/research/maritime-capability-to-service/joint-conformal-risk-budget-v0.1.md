# Joint conformal risk budget and robust port-action gate v0.1

Companion to [MCSC-ROBUST-ACTION-ENVELOPE-001 #7858](https://github.com/Luminous-Dynamics/symtropy/issues/7858), following the decision-time conformal calibration contract in [#7855](https://github.com/Luminous-Dynamics/symtropy/issues/7855).

## Why a joint risk budget is required

Five marginal 90% intervals do not imply that all five values are simultaneously inside their intervals with 90% probability. Without relying on independence, a union-bound guarantee requires marginal miscoverage probabilities whose sum is no greater than the desired joint miscoverage budget.

v0.1 requires a predeclared alpha allocation for arrival, handling duration, crane minutes, shore power and yard slots. Each interval must carry exactly its allocated alpha, and the allocation sum must not exceed the target joint alpha.

## Implemented API

The Rust module src/robust_action.rs provides JointRiskBudget, JointPredictionIntervals, ResourceCapacities, per-constraint assessments and an overall Feasible/Infeasible/Indeterminate result. It verifies interval alpha allocation, exact finite-sample rank from sample count and alpha, same-decision-cutoff semantics, and the adverse berth start/finish using latest arrival and upper handling duration.

The interval type carries alpha_ppm so the adapter can verify each calibrated interval against the predeclared resource risk allocation.

## Joint-coverage reasoning

For resource intervals with marginal miscoverage allowances alpha_i, the union bound gives joint miscoverage <= sum(alpha_i), without requiring independence between the resource errors. If the sum exceeds the declared joint alpha target, the adapter returns Indeterminate. Five 2% marginal intervals fit a 10% joint budget; five 10% marginal intervals do not.

This is a lower-bound guarantee only under the declared marginal split-conformal assumptions for each interval. It does not repair distribution shift, temporal dependence or unsupported exchangeability. Any shifted/unknown interval, missing/unbounded interval, alpha mismatch, invalid rank or decision-cutoff mismatch prevents a Feasible result.

## Resource gate

The adapter checks:
- berth window and receiver deadline: max(berth_slot_start, latest_arrival) + upper_handling_duration must fit before both berth_slot_end and receiver_deadline;
- quay-crane minutes: upper demand must not exceed declared available crane minutes;
- shore power: upper kW demand must not exceed available kW;
- yard: upper slot demand must not exceed available yard slots.

Capacity inputs must refer to the same planning window and compatible units. A resource overrun produces Infeasible. Missing/invalid risk evidence produces Indeterminate. Infeasible is a result under the supplied action and capacity model, not proof that a real vessel cannot be served.

## Deliberate limits

v0.1 does not model berth overlap with other vessels, crane interference, tidal windows, channel capacity, yard topology, energy duration, labor shifts, common-cause failures, or cumulative multi-stage state. It does not choose an action or optimize a schedule. It is a fail-closed feasibility filter for one candidate action under a bounded resource set.

## Required benchmark reporting

- predeclared per-resource alpha allocations and total joint alpha;
- interval sample count, exact finite-sample rank, model/schema/data digests, calibration windows and decision cutoff;
- exchangeability applicability for each marginal interval;
- interval width and empirical marginal coverage per resource;
- union-bound joint coverage lower bound implied by the alpha allocation;
- each adverse resource upper bound, capacity and individual gate;
- overall Feasible/Infeasible/Indeterminate rates;
- runtime/latency and unbounded/shifted evidence fallback rates.

## Claim ceiling

A passing synthetic test establishes alpha-budget arithmetic and bounded resource-gate semantics. It does not establish real Durban calibration, actual simultaneous coverage, multi-vessel scheduling optimality, physical resource accuracy, receiver acceptance, or authorization to dispatch.