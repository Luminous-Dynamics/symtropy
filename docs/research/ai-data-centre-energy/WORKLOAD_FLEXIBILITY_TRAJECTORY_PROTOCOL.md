# Service-Equivalent Workload Flexibility Trajectory Protocol

**Protocol:** GRID-WORKLOAD-FLEX-001  
**Version:** 0.1  
**Status:** paired deterministic schedule verifier adapter; workload-only

## 1. Objective

Translate two independently verified workload schedules into a signed, slot-by-slot PCC grid-import trajectory for the chronological adequacy evaluator. The two schedules must deliver the same useful-work obligations under identical service and facility contracts.

This is a bridge, not a scheduler. It reuses the existing workload schedule verifier rather than creating a second implementation of job/deadline/power-budget accounting.

## 2. Counterfactual construction

Let:
- (I_b(t)) be baseline grid import in kW for slot (t);
- (I_f(t)) be flexible-schedule grid import in kW;
- (Delta t) be the exact common slot duration in hours.

The emitted signed reduction is:

    R(t) = I_b(t) - I_f(t)

Positive (R(t)) is a reduction relative to baseline. Negative (R(t)) is increased import, including deferred-work rebound. Signed energy in the slot is:

    E_R(t) = R(t) × Δt  [kWh]

The report separately aggregates:
- gross reduction energy: sum of max(E_R(t), 0);
- rebound energy: sum of max(-E_R(t), 0);
- net grid-import energy reduction: sum of E_R(t).

The three values must not be collapsed. A pure temporal shift may produce high gross reduction and equal rebound with exactly zero net energy saving.

## 3. Paired-service invariants

The adapter rejects the comparison unless:
- service-contract ID and work-unit ID match;
- ordered job-contract manifests are exactly equal, including release slots, deadlines, required work, per-slot work ceiling, preemption policy, power assumptions and checkpoint overhead;
- ordered slot contracts are exactly equal, including start time, duration, background load, base cooling, committed non-grid supply, facility/grid/cooling limits and compute capacity;
- schedule IDs differ;
- both schedules independently pass the existing workload-schedule verifier;
- both schedule reports complete the same useful work within the declared absolute work tolerance.

Since the same job contracts are verified in both schedules, each job must complete within its frozen release/deadline window. Per-slot work may differ; the output retains per-slot baseline and flexible work receipts.

Caller-supplied schedule digests, site ID and PCC boundary ID are pinned into the report but are not authenticated or recomputed by this module. The adapter does not independently prove that the supplied PCC boundary maps into a power-system model's load node; that reconciliation remains a separate evidence obligation.

## 4. Known-truth example

The frozen fixture moves one 10-unit job from slot 0 to slot 1. The job requires 0.5 kW IT and 0.1 kW incremental cooling per work unit; fixed facility load is 100 kW; each slot is one hour.

| Slot | Baseline import | Flexible import | Signed reduction |
|---:|---:|---:|---:|
| 0 | 106 kW | 100 kW | +6 kW |
| 1 | 100 kW | 106 kW | -6 kW |

Both schedules complete 10 work units within the same contract. Gross reduction is 6 kWh, rebound is 6 kWh and net energy reduction is 0 kWh. The fixture proves the arithmetic and the need to preserve the recovery trajectory; it does not prove a physical site's ability to execute the shift.

## 5. Relationship to existing Symtropy modules

- `workload_schedule.rs` verifies job allocations, preemption, deadlines, compute capacity, cooling budget, facility input, grid-import limit and checkpoint overhead. It is the schedule feasibility authority for V0.1.
- `workload_flexibility_trajectory.rs` verifies the paired comparison and emits the signed per-slot import delta.
- `flexibility_evaluation.rs` remains the event-level matched-service check for completed work, energy, thermal limits and rebound over its event horizon.
- `grid_response.rs` verifies dispatch latency, sustained delivery, overshoot and ramp at the PCC boundary.
- `grid_firm_capacity.rs` aggregates repeated response trials within a declared stratum.
- `chronological_adequacy.rs` consumes a scenario trajectory and calculates single-zone adequacy metrics and an ELCC-like added-load counterfactual.

Avoid double-counting: the paired workload trajectory must represent the change at one declared boundary. Do not add IT, cooling, facility and PCC meter deltas together if they are nested measurements.

## 6. Paradigm shifts

1. **Work is the conserved commodity.** Flexibility is not the right to disappear jobs from the denominator. Preserve job identity, release, deadline, useful work and preemption policy.
2. **A flexibility product is a trajectory, not a peak.** Keep the slot-by-slot sign and complete rebound. A dispatch-window kW number cannot represent a schedule that moves load into another scarcity interval.
3. **Use independent verification, not scheduler self-certification.** Search/optimization policy is free to be EDF, MILP, MPC, heuristic or learned, but a separate verifier reconstructs both schedules and the paired delta.
4. **Match service before matching energy.** An apparent kWh reduction is invalid if jobs are dropped, work semantics change or deadlines/SLOs are weakened.
5. **Separate temporal shifting from physical efficiency.** The module reports gross reduction, rebound and net energy separately. Energy savings require an actual mechanism (e.g. reduced conversion/cooling losses) and should be demonstrated at a reconciled boundary.
6. **Build a composable evidence chain.** A workload schedule produces a signed PCC trajectory; response qualification verifies physical delivery; chronological adequacy estimates system value. No layer inherits claims the previous layer did not establish.

## 7. Limits and next layers

This V0.1 adapter does not yet model:
- queue state transitions and stochastic arrivals outside the supplied job manifest;
- latency-sensitive inference and interactive workloads beyond release/deadline contracts;
- online scheduling, work migration, network bandwidth or geographic failover;
- storage state-of-charge, charge/discharge efficiency, degradation or reserve;
- thermal inertia, temperature/humidity limits, cooling COP curves or water consumption;
- backup generation fuel, ramp, start time, minimum run and emissions;
- control/communications failure and correlated response uncertainty;
- nonlinear power curves or facility control interactions.

The next issue (#3669) defines a full stateful model. Implementation should add one mechanism at a time and emit the same signed trajectory receipt contract. The highest-value next increments are battery SoC conservation, thermal-state constraints/recovery, queue arrival replay and correlated response availability. Maintain paired matched-service reference runs throughout.

## 8. Benchmark targets

These are proposed engineering targets, not claimed results or regulatory requirements.

- 100% exact agreement with independent known-truth fixtures for all slot deltas and signed energies.
- 100% fail-closed results for mismatched job/slot contracts, missing work, invalid schedule windows and verifier errors.
- Zero work loss and zero deadline violations in all credited schedules.
- Zero unreported rebound; report all negative slot deltas.
- Independent replay must reproduce all slot-level PCC deltas from frozen schedule manifests and the same verifier tolerances.
- Compare multiple scheduling policies under identical job arrivals and facility conditions; use paired scenario bootstrap or equivalent uncertainty estimation once stochastic traces are introduced.
- Benchmark throughput and memory against identical inputs and hardware; record compiler version, build flags, CPU, core count and input digests.
- No claim of SOTA or industry superiority until matched tests against production workload traces and independently validated facility/power models exist.

## 9. Validation status

Eight unit tests are authored in V0.1 for known-truth shift/rebound, job-manifest mismatch, slot-contract mismatch, invalid baseline schedule, schedule-ID mismatch, identity/digest checks and service-contract mismatch. They have not been run in this environment. Rust build, unit test, rustfmt and Clippy results must come from a local Rust toolchain or exact-head CI and must be reported separately.
