# Wildfire Capacity-Aware Traffic Microbenchmark v0.1

**Issue:** [WILDFIRE-TRAFFIC-001](https://github.com/Luminous-Dynamics/symtropy/issues/2249)  
**Fixture:** [WILDFIRE_TRAFFIC_CTM_V0_1.json](../../benchmarks/fixtures/WILDFIRE_TRAFFIC_CTM_V0_1.json)  
**Status:** synthetic route-path microbenchmark; not an operational evacuation planner.

## Why this next

Moreno et al. (Transportation Research Part E, September 2026) use a Cell Transmission Model (CTM) and an exact Branch-and-Benders-Cut method to jointly plan evacuation routes, shelter network and flows under congestion, time-varying link characteristics, capacities and multiple time periods. The paper reports that conventional models ignoring traffic dynamics underestimated evacuation time by as much as 60% on its benchmark instances. That is a result reported by the paper, not by Symtropy; its detailed benchmark data are listed as available on request, so this repository does not claim a direct replication: https://doi.org/10.1016/j.tre.2026.104965.

A separate 2026 modular agent-based framework couples fire-driven road closures, pedestrian and vehicle movement, rescue traffic and dynamic rerouting; that is another useful target for later matched comparisons, not something this microbenchmark reproduces: https://doi.org/10.1016/j.ssci.2026.107200.

## Model boundary

This first increment is an intentionally small **sending/receiving-limited, discrete-time cell model** over independent candidate corridors. It has source demand, cells, a per-step link capacity, jam storage, a downstream shelter receiving capacity, a closure timestep, and total/accessibility shelter capacities. At each step it computes sending flow from current occupancy, receiving room in the downstream cell, and sink receiving limits before applying simultaneous cell updates.

It is not yet a general directed road-network CTM, does not model merges/diverges, lane changes, car-following, route switching during a run, multimodal behavior, spread of fire, smoke, human compliance, contraflow, or real incident data. A later implementation must add explicit network-node flow allocation and validate those mechanics before using CTM terminology for the broader solver.

## Baselines and frozen cases

- **B1 — static free-flow ranking:** select the route with the fewest cells and ignore the capacity queue, closures and shelter receipt constraints. This is a negative control, not a summary of all commercial software.
- **B2 — capacity-aware cell simulation:** simulate the same total demand on each candidate independently, reject choices that fail to deliver all demand or violate either shelter capacity, then choose the feasible path with the smallest observed clearance step; abstain if none qualifies.
- **B0 — synthetic oracle label:** expected route ID for scoring only. It is never used by B1/B2 to select a path.

The frozen fixture includes three cases: low-free-flow-time but low-throughput corridor; closure during a departure wave; and a shelter receiving bottleneck. The model reports arrival-step percentiles, queue vehicle-steps, completion/stranding, maximum occupancy, maximum observed flow, closure enforcement and mass conservation separately.

## Invariants and limits

- Per-step conservation: source queue + all cells + accumulated arrivals equals original demand.
- Cell occupancy remains within configured jam storage; each internal flow respects both sender capacity and downstream receiving room; sink flow respects sink receiving capacity.
- A closure starting at time step t prevents all transitions at and after t; vehicles and demand remaining are explicitly counted as unserved/stranded, not silently dropped.
- Total and accessible shelter capacity are separate hard gates.
- Input path order must not affect deterministic winners.
- Fixture labels, case IDs, model version, timestep units, raw fixture digest, and exact source commit are emitted in the receipt.

## Benchmarks to pursue next

For this microbenchmark, the pass criteria are invariant satisfaction and exact expected choices, not a numeric SOTA score. For a broader model, measure total clearance time, p50/p90/p95 arrival time, queue vehicle-minutes, throughput by timestep, unserved/stranded demand, shelter utilization, closure reroutes, runtime, peak memory, and regret against a small enumerated oracle. Keep all demand/capacity units and denominators explicit.

The next serious comparison should align the network, demand, departure waves, link capacity and jam storage, time discretization, closure timing, shelter and assisted-demand constraints, and compute budget. Use fixed generated topology families first; then seek published or agency data access for a held-out evaluation. No claim of SOTA superiority, field safety, predictive fire accuracy, or reduced harm follows from passing synthetic tests.
