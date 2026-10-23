# Non-anticipative shipyard recovery-policy benchmark (V0)

**Status:** benchmark proposal and small conformance harness. No industrial superiority claim or optimizer improvement is established.

## Why this benchmark layer exists

Published shipyard recovery work often evaluates a proposed schedule/recovery method over a finite set of disruptions. That is valuable but does not by itself rule out a method that implicitly uses future scenario identity when choosing earlier actions. Recent reviews also identify a need to connect disruption propagation, integrated recovery, schedule stability, and operational outcomes rather than focusing narrowly on cost or delay.

V0 adds one falsifiable requirement before policy-quality comparisons: scenarios with the same information available at a decision epoch must use the same action. It is a **causality gate**, not a substitute for exact geometry, feasibility verification, or outcome simulation.

## Frozen benchmark strata

- **R0 — replay integrity:** unique scenario and event IDs, ordered decision ticks, one action per scenario per epoch, no empty suite, no missing/extra decisions.
- **R1 — non-anticipativity:** identical observed histories at the same tick imply identical canonical action keys, including negative controls that inject future-scenario leakage.
- **R2 — independent feasibility:** each chosen action/schedule is audited by the independent geometry and resource/precedence verifier. This crate does not supply this result.
- **R3 — disruption outcomes:** delayed workstation clearance, vehicle/crane outage, compound outage, delayed arrivals, and clearance-time uncertainty, with every scenario and decision epoch accounted for.
- **R4 — recourse burden:** changed operations and placements, number of rehandles, affected-operation tardiness, unaffected-plan churn, and time to first independently feasible recovery.
- **R5 — uncertainty and tail behaviour:** report worst-case results and empirical quantiles separately. Do not call a finite scenario list a probability distribution without sourced probabilities.
- **R6 — scaling and runtime:** scenario count, event count, decisions per scenario, action-comparison count, wall time, peak memory, and time to first valid recovery, with parsing/verifying/reporting time separated from optimizer time.
- **R7 — industrial replay:** permissioned execution traces, shadow-mode first, predeclared evaluation horizon, baseline frozen before replay, independent reviewer, and documented operational constraints.

## Comparators

Compare under identical event streams, decisions epochs, action encoding, compute budget and independent validator:

1. static nominal schedule followed by a documented fixed recovery rule;
2. deterministic greedy recovery with fixed-order and MRV/exact-small-instance baselines where applicable;
3. rolling-horizon optimization with only observations available at each epoch;
4. robust / stochastic method only when its uncertainty and information assumptions are explicit;
5. candidate learned or Symthaea-generated policy, evaluated without privileged future-scenario identifiers;
6. oracle hindsight schedule as an **upper-bound diagnostic only**, never a fair online-policy competitor.

Keep the same input visibility for all online methods. Any deliberate clairvoyant policy must be retained as a negative control and rejected by R1.

## Proposed gates and reporting

| Measure | Proposed target | Boundary |
|---|---|---|
| Replay integrity | 100% declared scenarios × epochs represented exactly once | Structural completeness only |
| Non-anticipativity | 0 action disagreements for identical observable history; injected clairvoyance always rejected | Causality of supplied replay, not event truth |
| Independent hard constraints | 0 violations in all declared scenarios for a feasible claim | Supplied finite model only |
| Mandatory work and precedence | 0 dropped commitments or shortened required durations | Source model must define the commitments |
| Recovery quality | report makespan, worst-case tardiness, time-to-recovery, churn, rehandles, travel and utilization separately | No weighted score may offset a hard-gate failure |
| Search efficiency | anytime feasible-solution quality at matched time/evaluation budgets | Requires exact matched runs and hardware |
| Generalization | pass on held-out disturbance combinations and unseen event timing, with coverage reported | Synthetic holdouts are not industrial evidence |
| Operational value | predeclared improvement over permissioned baseline traces without additional hard-constraint violations | Requires controlled real execution evidence |

These are proposed target gates, not measured results. For stochastic methods, use paired seeds and report medians, dispersion and tail cases; count timeouts, invalid outputs and unrun cases explicitly. Reproduce published baselines on their original instances/objectives before comparing headline numbers. If data are unavailable, identify the result as a non-reproduction and avoid direct superiority claims.

## Falsification plan

The benchmark is considered defective if it accepts: a suite whose scenarios differ in action before any differing event is observable; missing decisions masked as no-ops; duplicate event IDs with ambiguous meaning; empty scenario coverage; or a clairvoyant candidate whose action keys depend on an unobserved future branch. Add each such defect as a permanent negative control.

## Research references

- Wang, Yin & Khan (2026), “Spatial-Scheduling Strategy for Irregularly Shaped Blocks Under Disruption in Shipbuilding,” *IET Collaborative Intelligent Manufacturing*, https://doi.org/10.1049/cim2.70066. Considers workstation and vehicle disruptions and a recovery strategy; original profile data are needed for exact reproduction.
- “Digitalisation of Shipyard Production Planning: A Review of Simulation, Optimisation, AI, and Digital Twin Methods (2010–2025),” *Journal of Marine Science and Engineering* (2026), https://www.mdpi.com/2077-1312/14/4/396. Discusses diversity of methods and uneven maturity of yard-wide operational validation.
- “Recovery strategies for disrupted seaside planning in container terminals: a systematic review” (2026), https://doi.org/10.1080/00207543.2026.2719841. Reviews disruption, recovery and schedule-stability trade-offs in the adjacent terminal-planning domain.

## Implementation boundary

This benchmark document plus `shipyard-policy-replay` tests whether declared observable histories constrain action selection. It does not implement rolling-horizon optimization. That should be a separate policy engine whose outputs are replayed through this independent gate and the existing independent feasibility audit.
