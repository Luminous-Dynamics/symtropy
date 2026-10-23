# Shipyard robust-recovery benchmark contract

**Status:** benchmark protocol proposal. No industrial performance comparison is claimed until source inputs are available and exact-head tests pass.

## Paradigm shift: optimize a verified set of recoveries, not one nominal schedule

A conventional planner emits one schedule and then ranks it by one weighted objective. That can conceal a catastrophic disturbance case, a resource conflict, or the silent deletion of a future operation. This benchmark instead separates:

1. **Hard feasibility gate:** every plan is independently checked against the declared spatial, temporal, precedence and unary-resource model.
2. **Scenario coverage gate:** exactly the declared scenario set is evaluated. Missing, duplicated, changed or unexecuted cases are reported as incomplete, not as successes.
3. **Robustness gate:** every declared scenario passes. One counterexample disproves robust acceptance for that finite scenario set.
4. **Operational quality:** compare makespan, affected-operation tardiness, travel time, recovery horizon, unaffected-plan churn, rehandles and resource utilization separately.
5. **Computational quality:** record time to first valid recovery, wall time to termination, candidate/evaluation count, peak memory and valid optimality/bound gap where available.

Do not combine safety-critical feasibility and quality measures into one weighted score. First reject invalid plans; only then compare the quality vector and its Pareto frontier.

## Mandatory per-scenario invariants

Every scenario must specify an immutable source-of-truth definition and a candidate plan keyed by the same scenario ID. Required future work must not disappear. Each required reservation identity must remain present and satisfy its minimum occupancy duration; operation identity and modeled duration must be preserved; source-defined finish-to-start constraints and minimum lags cannot be dropped; required resource commitments cannot disappear or change to an unqualified resource; and source-declared outages cannot be shifted to make the candidate pass.

Geometry and resources must be checked together. Space-time reservations use half-open integer tick intervals. Two blocks conflict if their exact fixed-point footprints overlap or touch under the declared conservative policy while their occupancy windows overlap. A unary crane/workstation/tug/inspection resource may not be reserved by two operations at once, and a reservation may not intersect a declared outage. Transfer-path swept volumes, capacity greater than one, tandem lifts, crane kinematics, load charts, setup times, operator qualifications, stacking and floor bearing must be declared unsupported until modelled and independently verified.

## Frozen test strata

Keep distinct strata and do not collapse their metrics:

- **P0 — semantic controls:** exact tiny polygon layouts, deliberate conflicts, half-open endpoint reuse, invalid geometry, overflow, duplicate IDs and negative controls.
- **P1 — proof semantics:** feasible layout, proven infeasible, no solution, timeout, invalid output and not run remain distinct. Budget exhaustion can never count as an infeasibility proof.
- **P2 — nominal yard:** identical source shapes, yard boundary, aisles and operation set across solvers.
- **P3 — finite disruptions:** delayed clearance, workstation outage, vehicle/crane outage, and compound disturbances. Definitions are immutable and every candidate uses the same scenario IDs.
- **P4 — recourse burden:** measure how much of the unaffected schedule changes, how many placements/transfers change, and how many rehandles become necessary.
- **P5 — uncertainty envelope:** after deterministic finite cases pass, add predeclared clearance intervals or scenario ensembles; report worst case and empirical quantiles separately. A finite scenario set is not a probability distribution unless probabilities are explicitly sourced.

## Minimum comparison protocol

### Published-paper reproduction

For Wang, Yin and Khan (2026), preserve the paper's Case9_50_4 input and objective definitions, hardware/software assumptions, disturbance schedule and 15 repetitions. Report the Table 6 anchors—yard utilization 0.775, algorithm time 308.0 seconds and vehicle time 2,339 minutes for its proposed method—as publication values only until the original instance and method configuration are obtained. The article's data statement says experimental data and simulation results are available from the corresponding author upon request. See [PUBLISHED-SOURCE-AVAILABILITY.md](PUBLISHED-SOURCE-AVAILABILITY.md) and [PUBLISHED-REPRODUCTION-CHECKLIST.md](PUBLISHED-REPRODUCTION-CHECKLIST.md).

### Fair method comparison after reproduction

- Use the same source geometry, transformation/quantization rule, aisles, block set, processing durations, travel-time matrix, resource roster, outage schedule and objective definition.
- Run every baseline and candidate through the same independent verifier.
- Pin exact source/executable/config hashes, hardware class, toolchain, random seeds and time/evaluation budget.
- Run 15 repetitions when reproducing the study's published protocol. For a new stochastic comparison, preregister a larger matched seed set (target 30 or more where feasible), compare paired runs on identical cases, and report distributions and uncertainty rather than best-of-run only.
- Count timed-out, incomplete, invalid and not-run cases in the outcome table; never omit them or treat them as infeasible by default.
- Publish raw per-run records to the extent licensing permits, with the digest of any restricted data bundle and a clear data-access statement.
- Do not compare raw runtime across different hardware/software without a controlled environment or explicit calibration. Report solver runtime separately from input parsing, verification and report serialization.

## Targets and claim thresholds

| Gate | Target | What it does and does not establish |
|---|---|---|
| Exact tiny oracle | Zero false-feasible and zero false-infeasible classifications on the complete frozen model corpus | Validates the implementation against that model only |
| Scenario coverage | 100% of the exact declared scenario set present and audited | Prevents success by omission; does not cover unknown disturbances |
| Robust feasibility | Zero hard-constraint violations across all declared scenarios | A finite-scenario certificate, not a physical safety certification |
| Required work | Zero missing/shortened mandatory reservations, operation duration changes, or dropped precedence/resource commitments | Prevents recovery plans from gaming the score |
| Geometry fidelity | Zero simplification-induced area inflation beyond declared quantization tolerance, with clearance reported separately | Geometry representation target; not equivalent to higher yard utilization |
| Optimizer quality | Lower worst-case recovery horizon/tardiness or lower churn at non-inferior feasibility and makespan, measured separately | Requires matched inputs and repeat distributions |
| Performance | Better anytime quality per fixed evaluation/time budget, without weaker checking | Requires matched hardware/budgets and reproducible runs |
| Industrial result | Improvement on permissioned execution traces over a predeclared evaluation window | Needed for operational claims; synthetic cases cannot substitute |

## How to make a real SOTA claim

A credible result must distinguish four statements:

- **Correctness advance:** fewer incorrect feasibility classifications against a declared exact oracle.
- **Algorithmic advance:** better coverage, objective vectors or proof gap on the same instances and budget.
- **Modeling advance:** better representation of irregular geometry or explicit disturbances, with additional constraints independently tested.
- **Industrial advance:** better measured yard KPIs on permissioned real execution traces over a declared horizon.

Each is a different claim and needs different evidence. Do not compare our rectangular or synthetic polygon model directly to an irregular-block paper's yard utilization or a multi-crane paper's travel distances before reproducing their constraints and inputs. Do not say “better than SOTA” when only one benchmark, one seed, a smaller search space or a different objective has improved.

## Reference

Wang, Ningfei; Yin, Jingbo; Khan, Rafi Ullah. “Spatial-Scheduling Strategy for Irregularly Shaped Blocks Under Disruption in Shipbuilding.” *IET Collaborative Intelligent Manufacturing* (2026). DOI: https://doi.org/10.1049/cim2.70066.
