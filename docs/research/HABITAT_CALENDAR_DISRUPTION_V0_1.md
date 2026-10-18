# Habitat Calendar and Disruption Benchmark v0.1

**Status:** research contract, frozen synthetic fixture and bounded Rust micro-oracle implementation; Rust compilation/tests and performance remain unverified in this authoring environment.
**Tracking issue:** [HABITAT-CALENDAR-001 #1938](https://github.com/Luminous-Dynamics/symtropy/issues/1938)
**Fixture:** [HABITAT_CALENDAR_DISRUPTION_V0_1.json](fixtures/HABITAT_CALENDAR_DISRUPTION_V0_1.json)

## Paradigm shift: from duration sums to work that exists in time

The current scheduling micro-oracle reasons in abstract integer time. That is useful for resource contention and precedence but cannot, by itself, establish that a timeline respects work shifts, lunch breaks, holidays, machine outages or worker absence. A task with four work-hours does not necessarily occupy four consecutive calendar-hours.

Model three separate things:

1. **Work required:** task duration in work-hours/units.
2. **When each required resource and task is available:** explicit half-open intervals in a finite calendar horizon.
3. **Execution semantics:** whether a task must run uninterrupted or may pause outside working intervals and resume later.

These semantics are not interchangeable. A three-work-hour task released at time 2 with resource windows [0,4) and [6,10) can finish at time 7 if it may pause (segments [2,4) and [6,7)); if it must be uninterrupted, its earliest feasible interval is [6,9). An elapsed-time duration model would answer a different question.

## Frozen test contract

The fixture contains nine hand-calculated cases covering parallel work, exclusive resource contention, pause/resume across lunch, uninterrupted work across lunch, inspection windows, outage re-planning, a holiday/shutdown that removes all feasible windows, intersection of multiple required resource calendars, and a deadline invalidated by an outage.

Conventions:
- Time is integer and finite for this tiny oracle; all intervals are half-open [start, finish).
- Task durations are active work units, not elapsed calendar time.
- Resource capacity is consumed only during task work segments; a pause consumes zero task work. Whether a paused physical operation can safely be interrupted is a domain fact and must be explicitly declared per task.
- Every segment must fit inside every required resource's effective working windows after scenario blackouts. A continuous/uninterrupted task must occupy one contiguous segment; a pause/resume task may have several segments whose lengths sum exactly to required work.
- Precedence uses completion of the predecessor before the first work segment of the successor. A successor may not begin merely because the predecessor began.
- A disruption is a counterfactual scenario, not an inferred probability. Baseline and disrupted results remain separate.
- `PROVEN_INFEASIBLE` is allowed only for the bounded exact micro-oracle and exact finite horizon. A production heuristic that finds no candidate must report a search miss/incomplete result instead.
- Every proposed schedule requires independent recomputation of active work, calendars, precedence, resource occupancy, deadlines and makespan.
- Planning is not execution, inspection, commissioning or professional approval.

The JSON records reference outcomes and canonical witnesses. They have not yet been consumed by a Rust test harness; do not describe them as an executed or validated software result.

## External benchmark due diligence

A relevant public dataset was published in *Data in Brief* in September 2026: Uddin et al., “An expert-informed multi-scale benchmark dataset for calendar-aware resource-constrained project scheduling (CA-RCPSP),” DOI [10.1016/j.dib.2026.113256](https://doi.org/10.1016/j.dib.2026.113256), data DOI [10.5281/zenodo.20553817](https://doi.org/10.5281/zenodo.20553817). The article describes 12 synthetic instances with 5–1,000 tasks, resource calendars, shifts, lunch breaks, holidays and disruption records; its source repository declares CC BY 4.0 for the dataset. This is a useful external target because PSPLIB is a widely used project-scheduling benchmark library, but PSPLIB's standard instances do not provide the same structured work-calendar object. See [PSPLIB](https://www.om-db.wi.tum.de/psplib/library.php) and the [CA-RCPSP article](https://pmc.ncbi.nlm.nih.gov/articles/PMC13599651/).

**Audit finding — do not accept the published solver comparison table as a ranking yet.** In the dataset repository's `data/scenarios/tiny/tiny_manufacturing_demo.json`, task T006 has duration 6.93 and the field is documented in hours; in `data/results/per_scenario_results.csv`, the same scenario's Max-Plus makespan is reported as 1.84 hours. For a schedule that must execute T006, makespan cannot be lower than that task's required work duration if both values use those units. The separately released [calrcpsp package](https://pypi.org/project/calrcpsp/) documents a 6.93 work-hour result for T006 and explicitly says its solver comparisons are not reproducible from the dataset's deposited code; it does not infer a solver ranking from that table. This is a **reproducibility/unit-consistency flag**, not proof that the entire dataset or scheduling method is invalid.

Before comparison, preserve the exact source release and confirm duration/time-unit semantics, then independently solve and validate the source instance. Record the source file SHA-256, DOI/version, parser version, all transformations, solver/toolchain, platform, resource and calendar semantics, and raw result bytes. Do not silently replace inconsistent CSV values or report them as authoritative.

## Benchmark ladder

| Stage | Target | Required evidence |
|---|---|---|
| Micro-oracle | All nine frozen cases | Exact expected makespan/feasibility; every canonical witness passes a separately implemented validator |
| Calendar correctness | 100% test-segment compliance | Zero work during breaks, non-working intervals or outages; exact active-work sum per task |
| Negative controls | Zero false-safe candidates | Reject resource overlap, precedence violations, deadline breaches, missing work segments and malformed/ambiguous calendars |
| Re-planning | Baseline and each disruption independently | Report baseline makespan, revised makespan, delta, changed tasks/segments and any infeasible hard constraints |
| External data | 12 CA-RCPSP instances after source audit | Independent validation and reported semantic discrepancies; no inherited solver ranking |
| Scale and anytime behavior | 25/40, 120/180, 500/750/1,000 tasks | p50/p95 runtime, peak RSS, candidate coverage, objective gap vs exact/best-known references, budget exhaustion and disruption recovery cost |

Targets are a test plan, not achieved measurements. On small exact cases, require 100% feasible-instance coverage and zero false-infeasibility claims. On larger cases, prioritize a valid, independently checked candidate quickly; publish anytime curves and explicitly label incomplete search. Report makespan, cost, resource utilization, calendar compliance, disruption recovery and uncertainty separately instead of collapsing them into an opaque “best habitat” score.

## Longer-term opportunity

Create a site-to-as-built-to-operations digital twin in which every planned intervention has (a) a dependency graph, (b) a calendar/resource-feasible candidate, (c) explicit uncertainty and disruption scenarios, (d) evidence that becomes stale when its inputs change, and (e) a separately recorded completion/inspection event. Sol-Atlas can supply geospatial and access constraints, Symtropy the scheduling/digital-twin substrate, Symthaea the model comparison and uncertainty analysis, and Mycelix an attributable approval/provenance trail. Integration must preserve each system's authority boundary: a simulation never proves geotechnical, structural, fire/life-safety or code compliance.

## References

- Uddin, M. A. et al. (2026). [Calendar-aware RCPSP dataset article](https://pmc.ncbi.nlm.nih.gov/articles/PMC13599651/), DOI [10.1016/j.dib.2026.113256](https://doi.org/10.1016/j.dib.2026.113256); data DOI [10.5281/zenodo.20553817](https://doi.org/10.5281/zenodo.20553817).
- [Dataset source repository and deposited files](https://github.com/aftab-fr/carm-plus-dataset).
- [calrcpsp package notes and test contract](https://pypi.org/project/calrcpsp/).
- Kolisch, R. and Sprecher, A. (1997). [PSPLIB: A project scheduling problem library](https://www.om-db.wi.tum.de/psplib/library.php), *European Journal of Operational Research*, 96(1), 205–216.
