# Deterministic shipyard production DES oracle v0.1

Tracking: [SHIPYARD-PRODUCTION-DES-001 #4817](https://github.com/Luminous-Dynamics/symtropy/issues/4817); implemented on a child branch of [verification recovery #4813](https://github.com/Luminous-Dynamics/symtropy/issues/4813).

## Purpose and research context

This Rust library provides a deterministic non-preemptive discrete-event replay for finite task sets with finite resource capacities, multi-resource demands, precedence dependencies and minimum lags. It produces task start/finish ticks, a completion/start event trace and makespan.

Bordbar et al. (2026), *Digitalisation of Shipyard Production Planning: A Review of Simulation, Optimisation, AI, and Digital Twin Methods (2010–2025)*, JMSE 14(4), 396, DOI 10.3390/jmse14040396, identifies hybrid DES–optimisation as the most operationally credible class in bounded shipyard planning contexts. The review evaluates maturity across four evidence levels: simulation-only, historical-data calibration/replay, partial shop-floor/APM/MES integration, and sustained live operation with demonstrable production KPIs. It also finds scarce open industrial data and weak cross-study numerical comparability.

This supports a precise division of labour:
- optimization proposes sequence / fixture / release alternatives (#4633);
- DES checks whether the proposed sequence can run under declared concurrent resource and precedence constraints;
- actual MES/yard integration and calibration are separate later evidence tiers.

## Dispatch semantics

- Non-preemptive tasks; a task holds all declared resource quantities for its full duration.
- A task acquires every required resource atomically. Partial resource reservation is forbidden.
- At a tick, all completions are processed in lexical task-ID order and resources are released before new starts.
- Ready tasks are dispatched by ascending explicit priority, then lexical task ID.
- A blocked task does not block a lower-priority independent task that can run with currently free resources.
- Predecessor completion plus minimum lag defines earliest start eligibility.
- The event queue advances to the next running-task completion or next dependency release.
- The simulator rejects cycles, missing identities, missing resources, impossible demand, duplicate task/resource/dependency identity and checked arithmetic overflow.
- All times and capacities in v0.1 are symbolic integers. There are no shifts, calendars, setup transitions, transportation distances, stochastic breakdowns or physical travel envelopes.

## Known-answer cases

The 18 unit/mutation tests include:
- shared robot tasks serialized while an independent crane task overlaps (makespan 8 rather than sum 12);
- minimum predecessor lag;
- atomic two-resource allocation;
- capacity-two concurrent resource use;
- lexical tie-break and deterministic event ordering;
- same schedule under reordered input when priority and identity are unchanged;
- cycle, unknown dependency, duplicate dependency, self-dependency and missing-task rejection;
- demand greater than capacity, zero demand, empty demand and duplicate resource/task rejection;
- time and lag overflow.

## Benchmark programme

- 100% exact start/finish/makespan agreement on frozen 2–6 task hand-enumerated cases.
- Zero resource-capacity and precedence/lag violations.
- Zero duplicate task start/completion or resource double release.
- Exact replay determinism across supported targets.
- Compare FCFS, priority dispatch, small-instance MILP/CP optimal schedules, and optimizer→DES hybrid schedules under the same resource map and event rules.
- Scale to 10–40 tasks with robots, fixtures, cranes, transporters and operators; then add setups, travel, cooling, shift calendars and stochastic downtime as separately versioned model extensions.
- Evaluation tiers: synthetic oracle; historical schedule replay; yard/APM/MES integration; sustained live KPI validation.

A DES PASS proves only that the symbolic event/resource model is simulated correctly. A real-yard claim requires calibrated task durations, resource capacities, setup and travel times, calendars, spatial/access constraints, downtime and observed execution history.

## Claim ceiling

Not a real shipyard schedule, not physical weld feasibility, not production-optimality beyond the declared model, not WPS/PQR/WPQT qualification and not authorization to execute robots or release work.
