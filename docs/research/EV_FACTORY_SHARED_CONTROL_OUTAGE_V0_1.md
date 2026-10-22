# EV Factory Shared Control-Plane Outage v0.1

## What was implemented

The Rust discrete-event engine now accepts an optional, strict `common_cause_control_outage` catalog entry. The configuration names a shared domain, its exact member stations, first start time, recurrence interval and duration. Unknown station IDs, duplicate members, missing identifiers, nonpositive time values and overlapping recurrence windows fail closed.

During a declared event, the affected stations:
- do not start new processing cycles;
- do not transfer completed output to a downstream buffer or finished-goods sink;
- allow cycles already underway to complete, with completed jobs held at the output boundary until the domain recovers.

The event is scheduled on deterministic simulation time. The report carries the exact outage configuration, number of outage starts after warm-up, measured outage seconds and per-station outage fractions. Existing catalogs omit this optional field and keep the prior independent-failure behavior.

## Why this is a deliberately narrow first mechanism

This is a *shared line-control / production-permission outage*, not an instantaneous electrical failure and not a general physical common-cause machine breakdown. Existing cycles are assumed able to finish; this represents graceful completion on local control/energy or a stop-new-work barrier. A physical power loss that interrupts active operations, loses work-in-process or requires restart/requalification needs a richer state machine and separate verified semantics. Do not relabel this v0.1 as a model of those behaviors.

The purpose is to make one structural assumption executable and independently testable before expanding the family set. The scenario is synthetic and not calibrated to a real EV line.

## Frozen fixture and oracle checks

Fixture:
`tools/ev-passenger-cost-frontier/fixtures/ev_factory_simulation_shared_control_outage_v0_1.json`

- Horizon: 4,000 s; warm-up: 0 s; one fixed replication seed.
- Shared domain: `shared-line-control`, containing `battery-module-install` and `final-assembly`.
- Outages start at 600 s and 2,400 s; each lasts 300 s. Two complete events yield 600 measured wall-clock outage seconds.
- Common random streams and the same product/supply config are used for the paired baseline comparison.
- Existing finite-buffer, input/output conservation, station time-accounting and no-release-authority invariants remain required.

Expected controls:
- Configured stations expose a nonzero outage fraction; nonmembers expose zero.
- No-outage catalog behavior reports null config and zero outage metrics.
- Same catalog/seed/build is deterministic.
- The test pairs no-outage and outage baseline outputs by the same 20 seed IDs and reports finite per-seed deltas. It does not require a monotonic throughput result: queueing, dispatch and finite-horizon interactions can change observed outputs, so the sign must be measured rather than assumed.
- The causal event trace carries all four expected transition times: 600 s start, 900 s end, 2,400 s start, 2,700 s end, with the exact same configured station group on each event.
- Invalid group membership and overlapping outage windows fail closed.

These are software/model-contract claims. The fixture is short and synthetic; they are not estimates of factory downtime or throughput loss.

## Next tests before treating this as credible model behavior

1. Add a causal event trace that proves every domain member is gated on exactly the same intervals, including at equal-time events.
2. Add replay for interruptions of already-started cycles (remaining-time pause/resume, scrap/restart and qualification behavior as explicit alternatives); do not hide these policy choices under a generic outage flag.
3. Add common-cause machine faults distinct from shared-control outages, with explicit cause IDs, affected servers, recovery ordering and overlapping-independent-fault semantics.
4. Compare nominal vs structural family outcomes on matched per-family seeds, then calculate ranking reversals, maximin choice and minimax regret. Never treat model families as IID seeds or infer probabilities from family counts.
5. Validate on rights-cleared real event traces before calibration, ROI or investment claims.

## Research context and relevant comparison points

A small benchmark should be compared with matching analytic or simulation tasks, not presented as a universal industrial simulator leaderboard.

- NIST AMS 100-85 (October 2026) recommends risk-proportionate computational-model credibility and uncertainty quantification tied to model influence and consequence: https://doi.org/10.6028/NIST.AMS.100-85
- Shi & Gershwin (September 2026) derive lead-time distributions for a three-machine/two-buffer unreliable production line and validate against DES. This is an appropriate future analytical cross-check for simplified finite-buffer submodels, not direct validation of this shared-control mechanism: https://doi.org/10.1016/j.ijpe.2026.110047
- NIST's Simantha is a DES comparator for asynchronous manufacturing lines with finite buffers and machine degradation/maintenance: https://www.nist.gov/services-resources/software/simantha-simulation-manufacturing
- Keshvarinia, MacKenzie & Zhao (2025) discuss simulation-based digital-twin decision optimization under supply-chain disruptions: https://doi.org/10.1016/j.dajour.2025.100646

A defensible comparative protocol is:
- first validate event semantics with hand-calculated traces and invariants;
- then run matching submodels against an analytical result where available;
- then compare intervention decisions on identical inputs/seeds and report tail/regret metrics;
- only then compare against industrial tools on a shared model, boundary, input data, constraint set and evaluation budget.

No SOTA, plant-performance, safety, purchasing or production-release claim follows from this change.