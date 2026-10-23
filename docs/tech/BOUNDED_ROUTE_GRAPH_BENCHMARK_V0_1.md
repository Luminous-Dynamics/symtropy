# Bounded-Uncertainty Route Graph Benchmark v0.1

**Status:** synthetic graph enumeration and conservative feasibility screening. This is not a live route planner, dispatcher, route clearance, safety case, clinical protocol, or validation of real vehicle performance.

## Paradigm shift: let the network design the service

The resilience audit in the companion work stress-tests a fixed set of candidate plans. This tool takes the next step: given a directed network of transfer hubs and transport legs, **enumerate simple candidate paths**, propagate conservative time/resource bounds, reject paths that violate mission requirements, and retain a Pareto frontier among candidates that pass every modeled gate.

It evaluates a route chain, not a single vehicle. One route may combine watercraft and a ground carrier; another may use an aerial leg or a direct carrier. Hub transfer time, transfer energy, payload handling, condition margin, route authorization, hazard evidence and recovery constraints are carried through the chain. A fast path cannot buy its way around a missing restraint, unsupported payload, depleted reserve, or unresolved route authorization.

This version remains deliberately bounded and explainable. It enumerates simple paths with a maximum of eight hops, up to 10,000 routes and 100,000 edge-expansion operations per mission. It fails closed if the enumeration bound is exceeded rather than silently returning an incomplete frontier. A next version should replace full path enumeration with a label-setting/multiobjective search and maintain auditable dominance proofs.

## What the contract models

The synthetic graph includes five environments (Amazon Basin, Congo Basin, Antarctica, Sahara and Afghanistan), 17 nodes, 19 directed edges and six missions. The missions cover cold-chain medicine, a fragile instrument, an accessible passenger journey, civilian aid with unresolved route/hazard status, and a disconnected sensor destination.

Each edge declares:
- Supported environments and payload classes.
- A duration interval \([t_{\min},t_{\max}]\), maximum energy consumption, and conservative condition margin.
- Required capabilities and six evidence gates: route authorization, environmental protection, hazard status, recovery status, operator status and condition control.
- Availability as a hard route constraint.

Each node declares payload/environment support, transfer duration interval, maximum transfer energy, condition margin, capabilities and the same evidence gates. Intermediate hubs contribute their upper-bound transfer duration and energy to a route. Origin and destination are not charged intermediate-handoff time. The route’s condition margin is the minimum declared node/edge margin encountered; the remaining energy reserve is the initial reserve minus the sum of worst-case edge and intermediate-hub energy budgets.

The planner reports both the topological route count and eligible route count. They are different quantities and must never be conflated.

## Fail-closed dispositions

- \`blocked_unavailable\`: at least one selected edge is marked unavailable.
- \`blocked_hard_gate\`: a required gate is explicitly failed.
- \`blocked_constraints\`: payload/environment/capability fit, worst-case deadline, residual reserve or condition margin fails.
- \`needs_gate_evidence\`: no known failed gate or known constraint violation, but one or more required gates remain unknown.
- \`eligible_for_human_review\`: all modeled constraints and evidence gates pass; this is not a route permit or deployment approval.

An overall mission is classified as:
- \`no_topological_route\`: no simple path exists from origin to destination within the declared hop limit.
- \`no_admissible_route\`: one or more topological paths exist, but none pass the modeled hard constraints/evidence.
- \`admissible_routes_available\`: at least one synthetic path passes for expert review.

The Afghanistan fixture has topological paths but no admissible route because route/hazard evidence is unknown. The disconnected Sahara control has zero topological paths. Those outcomes are intentionally different.

## Synthetic baseline oracle

| Measure | Expected result |
|---|---:|
| Environments | 5 |
| Missions | 6 |
| Nodes / directed edges | 17 / 19 |
| Simple topological paths | 14 |
| Eligible paths for human review | 9 |
| Mission topology/feasibility expectations matched | 6/6 (100%) |
| Weighted universal score produced | No |
| Route/deployment authorization emitted | No |

By mission, expected topological/eligible path counts are Amazon 3/3, Congo 3/2, Antarctica 3/2, Sahara cold-chain 3/2, Afghanistan civilian aid 2/0, and disconnected Sahara sensor 0/0. The faster unsecure Congo path must be blocked. Antarctica's aerial candidate remains non-admissible while route/hazard evidence is unknown. Pareto frontiers preserve the trade-offs between worst-case duration, remaining reserve and minimum condition margin.

These figures are regression oracles for fictional values. They do not represent observed journeys, terrain, vehicle specifications, probabilities, product performance or route clearances.

## Benchmark targets

### Contract and safety properties
- **100% exact-oracle agreement** on small generated graphs, including parallel edges, cycles, disconnected components, and varying hop limits.
- **Zero false-feasible routes** when any hard gate is failed, required evidence is unknown, a capability is missing, or any conservative bound violates a constraint.
- **100% worst-case timing/resource accounting**: all intermediate transfer durations and energy budgets counted exactly once; zero routes admitted if the worst-case deadline or energy reserve fails.
- **100% Pareto-frontier completeness** against an independent exact oracle on graph instances within a declared size bound.
- **100% deterministic output** after reordering independent node, edge, mission and expectation collections.
- **100% route accounting**: report topological paths separately from admissible paths; distinguish disconnected from blocked.
- **Zero universal winners**: expose non-dominated alternatives instead of hiding mission-relevant trade-offs in one weighted score.

### Scalability and quality targets for future versions
- Measure expansion count, memory, median and p95 computation time over fixed graph sizes and topological densities, with hardware and exact input SHAs published.
- For the exact enumerator, publish the route/expansion cap hit rate; never report a partial set as the complete Pareto frontier.
- For a future label-setting solver, target 100% exact-frontier agreement on small instances, then report frontier recall and objective regret against exact solutions on held-out medium instances.
- Add interval travel time and energy consumption, disturbance sets, correlated multi-edge failures, telemetry freshness, hub capacity/queueing and recovery reachability.
- Calibrate travel/energy/environmental models against instrumented data before calling any confidence interval or reliability estimate empirical.
- Add ecological disturbance, cost, service equity, maintainability and repairability as explicit objectives once evidence supports them; maintain accessibility and human safety as constraints.

The route counts and 100% oracle agreement are synthetic software gates, not a field-performance guarantee. The long-term goal for false-feasible safety-gate outcomes is zero; any unresolved mismatch blocks qualification.

## Research rationale

A systematic review published in September 2026 surveyed time-dependent vehicle-routing research through 2026 and identified uncertainty, energy limits, sustainability integration, operational realism and benchmark comparability as active gaps. This supports measuring robust time/resource budgets, multiple objectives and repeatable instances rather than reporting a single score. See [Time-Dependent Vehicle Routing Problem with Time Windows: A Systematic Review](https://www.mdpi.com/2071-1050/18/18/9557).

Multiobjective route trade-offs are well established in operations research; this benchmark adopts that vocabulary but adds hard feasibility filters and explicit non-authorizing evidence semantics. See the review [Multi-objective vehicle routing problems](https://www.sciencedirect.com/science/article/pii/S0377221707005498).

NIST's modeling-and-simulation guidance emphasizes verification, validation, model confidence, reproducibility and software reliability. This first benchmark is only a verification artifact over synthetic inputs; real claims will require validation against instrumented trials and an independently reviewed operational domain. See [NISTIR 7655: Best Practices for Modeling, Simulation and Analysis](https://www.nist.gov/publications/best-practices-modeling-simulation-and-analysis-msa-homeland-security-applications).

## Reproduction

From the repository root:

    cargo fmt --manifest-path tools/bounded-route-graph-audit/Cargo.toml -- --check
    cargo test --manifest-path tools/bounded-route-graph-audit/Cargo.toml --all-targets
    cargo run --manifest-path tools/bounded-route-graph-audit/Cargo.toml -- fixtures/transport/bounded_route_graph_scenarios_v0_1.json > bounded-route-graph-report.json
    jq -e '.benchmark_id == "bounded-route-graph-v0.1" and .mission_count == 6 and .environment_count == 5 and .node_count == 17 and .edge_count == 19 and .topological_route_count == 14 and .eligible_route_count == 9 and .expected_oracle_match_count == 6 and .expected_oracle_match_percent == 100 and .weighted_utility_score_produced == false and .universal_winner_produced == false and .route_or_deployment_authorized == false and .real_world_validated == false' bounded-route-graph-report.json
    sha256sum bounded-route-graph-report.json

The tool rejects malformed schemas/identifiers and exits non-zero when fixture expectations do not match. Passing this check means only that the code matches this synthetic contract.

