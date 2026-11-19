# Resumable Grid Asset Recovery v0.1

**Issue:** [GRID-ASSET-RECOVERY-002 #6213](https://github.com/Luminous-Dynamics/symtropy/issues/6213)  
**Parent:** [GRID-ASSET-RECOVERY-001 #6119](https://github.com/Luminous-Dynamics/symtropy/pull/6119)  
**Snapshot:** 2026-11-19

## Purpose

This is an execution-ledger verifier and candidate-schedule consistency gate. It is not a new optimal scheduler and it does not authorize physical work.

The v0.1 slice composes with the existing non-preemptive calendar-aware recovery scheduler. It checks:
- completed effort, live in-progress effort and proposed future effort separately;
- exact task effort conservation and no over-delivery;
- task-specific non-preemptible versus checkpointed execution;
- minimum segment effort, interruption limit and legal cumulative-work checkpoints;
- immutable event identity and idempotent replay of exact duplicate event deliveries;
- current/superseded versus stale/conflicted/unknown evidence;
- in-progress commitment freeze and explicit reconciliation for state/version changes;
- resource retention or release during pauses;
- time-varying resource capacity windows and overlap;
- task precedence and candidate completion estimates only when all remaining effort is scheduled.

## Three clocks and three quantities

A task has **required effort**, **productive effort delivered**, and **elapsed wall-clock time**. They are not interchangeable.

A segment records start/end time and productive effort. Gaps between segments are explicit interruption time; they do not count as labor. A live progress snapshot is distinct from a closed segment. A planned future segment is distinct from observed work.

For task i:
`observed_i + planned_i + remaining_i = required_i`

The validator rejects any plan that makes the sum exceed the requirement. It does not infer progress from elapsed calendar time.

## Preemption semantics

- `non_preemptible`: one segment only; it must fit its resource/capacity calendar.
- `checkpointed`: multiple segments are permitted only at declared cumulative-effort checkpoints.
- Every boundary before a later segment must have an interruption cause. A future planned boundary may name its intended cause; an observed boundary must be backed by a current execution event.
- An active in-progress segment cannot be moved or followed by a new planned segment in the same proposal. Replanning must wait until a pause/checkpoint is observed and reconciled.
- `pausedResourcePolicy=release` frees task resources between segments. `retain` reserves them continuously across the pause and, while a task remains paused without a planned resume, through the declared evaluation horizon.
- Minimum segment effort and maximum interruptions are task-specific.

## Event and commitment semantics

Execution events bind:
- event and idempotency identities;
- task/segment/sequence;
- effective time and observed time;
- work interval, delivered effort and evidence status/reference.

Exact duplicate deliveries with the same event identity and content are deduplicated. Reuse of an event ID or idempotency key with conflicting content is rejected. Superseded progress snapshots remain in the ledger but are reconciled against a later completed segment; stale, conflicted or unknown records block the evaluation.

The proposal retains all baseline commitment identities and frozen event IDs. A new progress snapshot must be appended to the proposed event ledger and bound by an explicit version increment and reconciliation action. Supported actions are `progress_recorded` (in-progress to in-progress), `pause_confirmed` (in-progress to paused), `resume_authorized` (paused to in-progress), and `completion_confirmed` (paused/in-progress to completed). Changing commitment state, version, active segment, frozen event set or source evidence reference requires an explicit current reconciliation event binding the old/new versions, action, authority and evidence. Completed commitments cannot be changed. The schema and validator check consistency, not cryptographic signatures, legal authority or field truth.

## Benchmark and industry comparison

### Calendar-aware baseline: CARM-Plus (August 2026)

Uddin et al., *An expert-informed multi-scale synthetic benchmark dataset for calendar-aware resource-constrained project scheduling (CA-RCPSP)*, Data in Brief 69 (2026), 113256, DOI [10.1016/j.dib.2026.113256](https://doi.org/10.1016/j.dib.2026.113256).

The public corpus contains 12 reproducible instances across four scales: 5–8, 25–40, 120–180 and 500–1,000 tasks; heterogeneous machines and skilled workers; explicit shifts, breaks and holidays; and disruption scenarios drawn from a 14-type catalogue. The authors include descriptive Max-Plus, OR-Tools CP-SAT and Gurobi MIP baselines. The dataset is released on Zenodo under CC BY 4.0: [10.5281/zenodo.20553817](https://doi.org/10.5281/zenodo.20553817); generator and fixtures: [aftab-fr/carm-plus-dataset](https://github.com/aftab-fr/carm-plus-dataset).

This is the best first external calendar/disruption transfer corpus for #6119/#6213. It must be imported with its exact version, seed, license, source hash and baseline semantics. This PR does not claim that the CARM-Plus corpus has been imported or executed.

### Preemption trade-off comparator

Salvadori and Agnetis, *The impact of the number of preemptions in resource-constrained project scheduling problems with time-varying resources*, Annals of Operations Research (2025), DOI [10.1007/s10479-025-06892-2](https://doi.org/10.1007/s10479-025-06892-2). The study computes makespan/preemption Pareto fronts for 30-job instances and reports that a small number of preemptions often captures much of the makespan benefit, especially with time-varying resources. The benchmark implication is to measure makespan and interruption/coordination burden together, not maximize preemption.

### Stochastic reactive scheduling

*Constraint programming approaches for stochastic RCPSP subject to disruptions*, Computers & Operations Research 191 (July 2026), 107431, DOI [10.1016/j.cor.2026.107431](https://doi.org/10.1016/j.cor.2026.107431), compares proactive-reactive CP strategies with a right-shift control policy under random job durations. It reports improved robustness/stability against its own classical baseline but no public dataset. Use it as a policy comparator, not an imported corpus.

### Multi-skilled reliability under disruptions

Ghasemi et al., *A multi-skilled RCPSP with reliability constraints*, Computers & Industrial Engineering 219 (September 2026), 112154, DOI [10.1016/j.cie.2026.112154](https://doi.org/10.1016/j.cie.2026.112154). It uses M/M/1 disruption/retrieval rates, an exact AUGMECON2-VIKOR small-instance approach and a hybrid NSGA-II large-instance comparator on PSPLIB/iMOPSE. This informs future multi-skill/reliability benchmarking; it is not equivalent to the current fixed-resource recovery case.

## Benchmark protocol

Compare the existing non-preemptive scheduler with:
1. non-preemptive calendar-aware baseline;
2. bounded checkpointed preemption (one interruption, then two, then a declared maximum);
3. preemption plus resource release;
4. preemption plus resource retention;
5. right-shift repair after a disruption;
6. rolling-horizon repair with frozen commitments;
7. bounded exhaustive oracle on tiny instances.

All arms receive the same calendar, task graph, resources, event prefix, horizon and compute budget. Import CARM-Plus first, then add PSPLIB/MMLIB/MSLIB/iMOPSE only where problem semantics match.

Report separately:
- makespan and time to first accepted restoration milestone;
- schedule churn and changed/frozen commitments;
- number of interruptions, interruption duration and restart/hand-over effort;
- lost work and remaining work;
- resource utilization and capacity conflicts;
- service-weighted unserved energy from the existing recovery report;
- feasibility, optimality gap, p50/p95 runtime and timeout/INCONCLUSIVE count.

Do not combine interruption count, restoration time, service loss and resource cost into an opaque single score.

## Qualification targets (not achieved)

- 100% work-effort conservation in all frozen cases.
- Zero duplicate progress from event replay.
- Zero completed/in-progress commitment mutation without explicit reconciliation.
- Zero unauthorized segment split for non-preemptible tasks.
- 100% agreement with hand-calculated tiny execution fixtures.
- 100% agreement with bounded exhaustive calendar/resource oracle on supported tiny cases.
- Zero resource-capacity violations.
- Zero downstream task starts before the full required work of predecessors is completed or scheduled.
- Measurable Pareto trade-off between interruption burden and makespan on matched cases.

## Qualification boundary

This v0.1 tool validates a supplied execution ledger and candidate segments. It does not optimize a full preemptive RCPSP, model human fatigue or safe work-rest rules, prove a field observation true, authorize task interruption/resumption, or actuate equipment. HSE/permit/lockout/tagout and asset-owner procedures remain external hard prerequisites. A schema-valid event is not proof that work physically occurred.
