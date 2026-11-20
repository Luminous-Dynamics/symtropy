# Marine Runtime Action Shield: state of practice and benchmark position

**Date:** 20 November 2026  
**Track:** Symtropy #6269; stacked on inter-sample envelope PR #6293.

## What is already SOTA / industry practice

The frontier is not "add a CBF" or "use MPC":

- Sang et al., *Intelligent trajectory tracking control for autonomous sailboats based on event-triggered NMPC and adaptive reference trajectory*, Ocean Engineering 356 (30 May 2026), DOI https://doi.org/10.1016/j.oceaneng.2026.10747. The paper reports an event-triggered NMPC, adaptive reference trajectory and barrier-function event trigger, including >30% lower actuator energy in its evaluated cases. That result is paper-reported and has not been reproduced by this repository.
- Huang et al., *FFRLS-MPC: An adaptive model predictive control method for robust ship course keeping*, Scientific Reports 16 (8 June 2026), DOI https://doi.org/10.1038/s41598-026-56603-5. It combines online recursive least-squares dynamics identification with MPC under time-varying vessel dynamics and persistent environmental disturbance.
- Tan et al., *Distributed safety-critical formation control for networked marine surface vehicles with observer-enhanced robustness*, Ocean Engineering 368 (15 November 2026), DOI https://doi.org/10.1016/j.oceaneng.2026.128430. It builds disturbance-observer-enhanced control-barrier constraints into MPC and keeps safety/actuator limits hard while softening performance tracking.
- Jin et al., *Passivity-preserving safety-critical path following cascade control of underactuated surface vehicles with disturbance rejection*, Ocean Engineering 364 (30 August 2026), DOI https://doi.org/10.1016/j.oceaneng.2026.126980. It combines port-Hamiltonian/passivity structure, barrier filtering and disturbance rejection.
- Pagnini & Malis, *Segment-safe control barrier functions for model predictive control*, European Journal of Control (23 July 2026), DOI https://doi.org/10.1016/j.ejcon.2026.101582. It explicitly addresses safety between sampled states; this is stronger than endpoint-only constraints but its published validation is on a double integrator and quadrotor, not a liveaboard resource system.
- The October 2026 ship weather-routing work already combines adaptive H3 routing, TSS topology, edge-level IMO SGIS pruning and member-coherent ensemble CVaR. Failed deterministic-forecast routing can be worse than weather-blind routing. DOI https://doi.org/10.1016/j.oceaneng.2026.127661.

Commercial weather routers and autopilots already provide multi-model routing, personalized polars, comfort and course/path following. Do not claim novelty for those individual capabilities. The runtime shield is not a replacement route planner, autopilot or COLREG interpretation engine.

## Proposed differentiator

The testable differentiator is an **evidence-qualified, resource-aware, independent next-action gate** that composes:
- the forecast information set and member-coherent disturbance evidence;
- live state freshness, monotonic sequence and telemetry qualification;
- conservative inter-sample and next-state resource/service envelopes;
- hard-gate and recoverability/intervention-time receipts;
- predeclared fallback ordering with explicit abstention.

The planner cannot self-certify; it cannot alter the shield's contract or ODD; and a permit receipt is not an actuator command. Unknown executor outcomes require reconciliation before another state mutation.

## Benchmark ladder

| Dimension | Proposed target |
|---|---|
| Known-answer correctness | 100% deterministic agreement with independently calculated outcomes |
| False permits | Zero false permits in the complete preregistered hard-gate, interval, disturbance, staleness and recovery negative-control corpus |
| Fallback correctness | 100% selection of the first admissible predeclared fallback; no dynamic/hidden fallback invention |
| Fail-closed behavior | 100% stale, expired, replayed, mismatched, incomplete and unknown evidence cases abstain or invalidate |
| Safety/performance transparency | Report false-reject rate, abstention rate, feasible-set shrinkage and minimum robust margins separately from false-permit rate |
| Latency | p50/p95/p99 and worst-case execution time; research target p99 below 10% of the named onboard control-cycle budget |
| Re-evaluation | Deterministic maximum reevaluation interval; earlier trigger on state drift, model residual, disturbance change, forecast/member change or hard-gate activation |
| Physical progression | SIL negative controls → PIL → HIL/PHIL fault injection → independently reviewed sheltered-water tests |

The proposed latency and false-permit targets are research goals, not measured results. Statistical comparisons of energy, arrival and service outcomes require paired whole-voyage/weather-event holdouts and a prospective power analysis.

## First executable increment

This PR implements the read-only one-step decision receipt over precomputed successor intervals. It does not implement a dynamic model, reachable-set computation, CBF synthesis, route planner or executor. The next research increment should independently validate the supplied transition/reachable envelopes and their operating-domain scope, then compose telemetry requalification and atomic single-use consumption in a separately authorized executor.
