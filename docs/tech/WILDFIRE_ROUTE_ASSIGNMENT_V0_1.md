# Joint Route Assignment Benchmark v0.1

**Issue:** [WILDFIRE-ROUTE-ASSIGNMENT-001](https://github.com/Luminous-Dynamics/symtropy/issues/2300)  
**Fixture:** WILDFIRE_ROUTE_ASSIGNMENT_V0_1.json  
**Scope:** exact enumeration on tiny synthetic fixed-route queue networks.

## Paradigm shift: optimize the bundle, not the individual path

The independent shortest-path baseline chooses the lowest free-flow travel-time route for each vehicle without accounting for what the other vehicles will choose. This can concentrate trips on a common bottleneck even when parallel paths have spare capacity. SUMO's Dynamic User Assignment documentation explicitly discusses the need to address congestion produced when vehicles follow fastest empty-network paths: https://sumo.dlr.de/docs/Demand/Dynamic_User_Assignment.html.

B2 enumerates route assignments for each synthetic vehicle and evaluates them with the existing directed-network queue simulator. It does not simulate a different network per candidate; every candidate assignment is run against the same link capacities, queues, closures and sink rules. This isolates the benefit of joint assignment in the stated model.

## Policies and objective

- **B1 independent shortest path:** for every vehicle, select the route with the smallest sum of configured per-edge travel-time steps; ties are broken by route ID.
- **B2 exact joint assignment:** enumerate each route combination up to a hard budget of 100,000 assignments and 16 vehicle units. Simulate each assignment and minimize the lexicographic objective:
  1. unserved or stranded vehicles,
  2. p95 arrival step (missing p95 sorts last),
  3. queue vehicle-steps,
  4. sum of free-flow travel-time steps,
  5. lexical route-ID vector.
- The assignment enumerator never reads hidden expected-result labels; the fixture contains no labels used by the policy.
- Each vehicle gets an explicit demand row so the existing simulator can assign a different fixed route per vehicle. Accessible vehicles retain their classification, and sink total/accessibility constraints remain enforced by the network simulator.

## Frozen cases

1. Shared short bottleneck with longer parallel routes.
2. A closure after the start of a departure wave, where late vehicles on the nominally fastest path can become stranded.
3. A nearby but small shelter competing with a slower shelter that has capacity; total and accessible capacities remain hard constraints.
4. Adversarial control where the free-flow shortest route should remain optimal.

## Metrics and test gates

The receipt reports the full objective tuple, route assignment by demand, per-sink service, p50/p90/p95 arrivals, queue vehicle-steps, served/unserved demand and assignment combinations evaluated.

Frozen acceptance checks require the closure and shelter capacity cases to improve served demand relative to baseline, at least one congestion/capacity case to improve p95 or queue burden, and the adversarial control to return the same objective and allocation as B1. All baseline and oracle runs must preserve mass balance and satisfy the simulator's capacity checks. A search budget overflow is a hard error, not a partial result.

## Claim boundary

Exact enumeration is intentionally limited to tiny synthetic instances and is not a scalable traffic assignment algorithm. Because the oracle uses the same queue model as the candidate evaluator, this shows an optimization benefit *inside that model*, not superiority to SUMO, a full Cell Transmission Model, or real-world traffic. The next external comparison must freeze SUMO version, DUA configuration, demand, network, closure semantics and resource budget, and keep model mismatches explicit.

## Generated perturbation gate

A deterministic parameter-sweep test now runs 32 variations on each of the four frozen topologies (128 total). It varies per-edge capacity, storage and travel time, plus demand departure steps, while keeping route candidates and vehicle inventory bounded. Every variant reruns both B1 and exhaustive B2 and requires:

- B2's lexicographic objective is never worse than B1, since B1 is itself a feasible candidate in the declared search space.
- Both simulations preserve mass balance and pass their declared capacity checks.
- The enumerator stays within its explicit 100,000-assignment budget.

This is a property-style correctness stress test on small synthetic scenarios; it is not a randomized real-world benchmark, a scalability result, or a SOTA comparison. Seeds are derived deterministically from case index and sample index, so failures are reproducible.
