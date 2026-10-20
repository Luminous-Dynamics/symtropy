# Wildfire Decision-Value Sensing Benchmark v0.1

Issue: [WILDFIRE-VOI-001](https://github.com/Luminous-Dynamics/symtropy/issues/2256)  
Fixture: [WILDFIRE_VOI_V0_1.json](../../benchmarks/fixtures/WILDFIRE_VOI_V0_1.json)  
Status: synthetic decision-policy microbenchmark, not operational sensor tasking.

## Research grounding

Li, Song and Li, “Bayesian active verification of lifeline roads for risk aware emergency logistics with trucks and drones,” published 29 August 2026, treats road inspection as a resource-coupled information-purchase decision. Their route-aware and CVaR-aware value-of-information measures rank an inspection by downstream service-cost consequences, including the opportunity cost of using scarce UAV capacity for information rather than delivery. Their reported results are regime-dependent: route-aware sensing helps where probes can reach route-critical bottlenecks, while broad uncertainty probing can be stronger when sensing budget is under pressure. This microbenchmark draws on that question but is **not a reproduction** of their model or experiments: https://link.springer.com/article/10.1007/s44163-026-02033-1.

A recent preprint also investigates decision-sensitive sensor tasking for real-time bushfire evacuation; its results should be treated as preprint claims rather than established universal benchmarks: https://ssrn.com/abstract=7560932.

## Paradigm shift

Do not ask “which measurement reduces uncertainty most?” Ask “which answer can change the downstream action soon enough to matter?” A high-entropy road that is absent from all viable routes has no decision value in this model. A less-uncertain bridge can be worth checking when its state changes whether a costly, time-critical route should be used.

## Frozen cases and policies

The fixture has four synthetic cases:
- An off-route road has higher entropy than the route-critical road, but probing the critical road reduces expected downstream cost enough to justify the inspection.
- A highly uncertain road is irrelevant to the best route; despite high entropy, the decision-aware policy should not spend the probe budget.
- A bridge with high consequence if blocked competes with an off-route, higher-entropy road.
- A potentially useful observation arrives after the decision deadline; it cannot be recommended for the earlier decision.

Each world enumerates independent Bernoulli road availability states. Inspections are assumed perfect and resolve one road at the stated availability step. Costs are abstract, hand-authored loss units. These idealized assumptions make the tiny oracle auditable; they are not sensor-calibration claims. Exact enumeration is capped at 16 roads (2^16 worlds) in this version.

- **B0 — exact enumeration oracle:** fixture-labelled optimal query/abstention used only for evaluation.
- **B1 — entropy-only:** selects the eligible observation with largest binary entropy, ignoring its downstream route consequence and inspection cost.
- **B2 — decision-aware VoI:** compute minimum expected route cost before observation; for each possible perfect probe, enumerate conditional route costs for the open and closed outcomes; compute gross expected cost reduction minus inspection cost; probe only when net value is positive and the answer arrives by the decision deadline. Otherwise abstain.

The policies receive the scenario roads, priors, actions, inspection costs/availability and deadline. They do not receive oracle labels.

## Metrics and hard gates

- Gross expected downstream cost reduction, inspection cost and net value of information, in separate fields.
- Selected-query match/mismatch against the tiny fixture oracle, abstention rate, late-observation rejection and query cost.
- Zero post-deadline observations selected.
- Zero selected probes with non-positive net decision value.
- 100% expected choices/abstentions on the frozen cases; deterministic output under input ordering changes.
- Exact source commit, fixture SHA-256, case inventory, model version and claim boundary in the receipt.
- No aggregate metric should hide deadline, accessibility, or catastrophic-loss failure. A future CVaR/tail-risk benchmark should report severe-loss rate separately from mean cost, and should compare against entropy, coverage, random and route-criticality baselines under the same probe budget.

## Scope and next stage

This is a perfect-observation, independent-road, single-probe toy model. It does not model noisy/censored sensors, correlated road failures, drone flight time, multi-probe posterior updates, real traffic flow, fire spread, human response, or operational loss calibration. The next serious comparison should add noisy inspection, common-cause priors, fixed-budget multiple observations and tail-regret metrics, then match assumptions and denominators to published code/data or a documented agency tabletop. It must never autonomously issue evacuation orders or dispatch actions.
