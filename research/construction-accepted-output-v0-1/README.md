# Construction Robotics Accepted Output and Productivity Contract V0.1

**Issue:** [#3605 — functional units, independent acceptance and total-workflow productivity](https://github.com/Luminous-Dynamics/symtropy/issues/3605)  
**Layout adapter:** `profile/layout-accepted-output-v0.1.json`  
**Atomic ledger schema:** `schema/accepted-output-ledger-v0.1.schema.json`  
**Derived report schema:** `schema/accepted-output-report-v0.1.schema.json`  
**Status:** proposed research contract; fixtures are invented; no physical work has been benchmarked.

## Purpose

A construction robot must not receive productivity credit for work that is merely attempted, printed, moved, drilled or counted by its own controller. The benchmark's useful output is a task-family-specific functional unit that has passed a frozen, independently evidenced acceptance process and is usable by the downstream work package.

This contract adds robot/run-level accepted-output semantics beneath project/work-package/activity productivity frameworks. It deliberately aligns with the [Cambridge Framework for Construction Productivity](https://www.construction.cam.ac.uk/research-and-innovation/performance-productivity) rather than creating a competing project productivity ontology. Cambridge's framework, published on 30 September 2026, draws on more than 25 projects and structures production rate, labour productivity, production flow, variability and blockers across project, work-package and activity levels. This contribution binds those ideas to exact robot runs, individual work units, independent acceptance evidence, rework attempts and resource-time accounting.

## The central paradigm shift: output is a contract, not a counter

A functional unit must be declared before execution, and must include:
- an explicit task family and unit class;
- its measurand and unit;
- the design/model/export revision, coordinate frame and relevant site epoch;
- a versioned acceptance predicate and policy digest;
- criticality and required inspection coverage;
- a downstream-use definition;
- the independent measurement and adjudication method.

A robot's job-complete flag, print counter, waypoint count, odometry, self-reported accuracy or internal QA is not independent acceptance evidence.

Do not pool unlike units. A layout point, a printed line metre, a drilled hole, an installed pile and a square metre of accepted wall are different functional units. They require task-specific adapters and criteria, even when all are ultimately expressed as production per hour.

## Industry comparison and what we should improve

### Construction productivity frameworks

The Cambridge Framework for Construction Productivity is the strongest current project-productivity comparator identified in this tranche. It integrates production, labour, programme and contextual data; supports planned-versus-actual flowlines; and uses blockers and variability to diagnose delivery. Symtropy should align with its project/work-package/activity levels and add a verifiable robot/run and output-unit layer. Do not duplicate its project-management framework or claim that one task-level metric replaces construction productivity.

### Construction robot evaluation research

Brosque and Fischer's 2022 Robot Evaluation Framework assessed ten construction robots across 12 projects and 11 contractors, comparing safety, quality, schedule and cost. It reported improvements in many cases, but four of ten cases had increased cost. This is a valuable multi-task comparison precedent; the spread in outcomes makes matched scope, cost assumptions, baseline method and uncertainty non-negotiable.

### Construction layout case study

Kam et al. (2025) reported substantial benefits for a studied Dusty/manual multi-trade layout project, including 68% fewer total labour hours and rework decreasing from 6.42% to 0.25%. The paper also describes approximately one hour of wrongly placed printing after an incorrect coordinate-system CAD input. This demonstrates why accepted output must be conditional on valid model/frame admission and independent QA. These are project-specific case-study results, not universal SOTA or matched multi-site performance.

### Human–robot collaboration indicators

Recent HRC frameworks already consider completion time, capacity, human/robot ratios, labour-time ratios, collaboration efficiency, first-pass yield, rework and workload. We should retain useful operational measures but separate observed task outcomes from subjective trust, satisfaction, perceived stress and self-reported workload. Neither a positive survey nor a high throughput result establishes physical safety.

### Forthcoming infrastructure productivity specification

BSI's project page currently schedules PAS 4010:2026 for publication on 5 November 2026. As this protocol is dated 4 November 2026, PAS 4010 is treated as forthcoming; review the published text before making a mapping or conformity claim.

References:
- Cambridge Framework: https://www.construction.cam.ac.uk/research-and-innovation/performance-productivity
- Brosque & Fischer (2022): https://link.springer.com/article/10.1007/s41693-022-00072-5
- Kam et al. (2025): https://link.springer.com/article/10.1007/s41693-025-00163-z
- HRC indicators (2025): https://www.mdpi.com/2075-5309/15/15/2734
- BSI PAS 4010 project: https://standardsdevelopment.bsigroup.com/projects/2024-03460

## 1. Atomic output ledger

The ledger stores facts about planned units, attempts, inspections, defects, time entries and interruptions. It must not accept producer-authored aggregates such as "accepted units", "first-pass yield", "rework rate", "productivity score" or "overall rank".

### Work-unit states

A work unit is tracked through its attempts and independent inspections:
- planned, but not attempted;
- attempted, but no output recorded;
- output produced, awaiting inspection;
- first attempt accepted;
- first attempt rejected;
- reworked and resubmitted;
- finally accepted;
- finally rejected;
- unresolved, cancelled or superseded.

The current status is derived from the ordered attempt/inspection evidence, not authored as a favourable summary. If an attempted unit has no inspection, it is not accepted. Reprints and retries remain attached to the original unit; they do not create new production quantity.

The first inspection of attempt 1 determines first-pass acceptance. Later accepted rework remains final acceptance, not first-pass acceptance.

### Inspection and defects

Every acceptance record binds the exact acceptance-profile digest, inspection authority, method, reference artifact and evidence references. The validator can check referential consistency; it cannot authenticate the authority or prove that the reference instrument is physically accurate.

Defects are typed and severity-coded. A critical omission, wrong revision, wrong coordinate frame, duplicate feature or downstream-incompatible result cannot be averaged away by good measurements elsewhere. The task adapter defines which defects are disqualifying and whether partial acceptance is permitted.

Critical features require complete inspection or a separately justified, preregistered sampling method. Noncritical samples cannot support a claim that all critical features are present.

## 2. Productivity denominators

Keep the following distinct:

- **Robot operating hours:** machine time for the exact platform/serial unit and declared task scope.
- **Direct crew-hours:** direct labour assigned to execute and supervise the task.
- **Total workflow crew-hours:** all labour required to prepare, coordinate, survey, execute, inspect, correct, recover and close out the accepted output.
- **Elapsed time:** wall-clock duration for the declared work package, including the effects of waiting and interruptions.
- **Accepted functional units:** units with a current, final independent acceptance result.
- **First-pass accepted units:** units accepted on their first inspected attempt.

Time categories include, where applicable:
- model/BIM/VDC preparation and federation;
- design revision and trade coordination;
- survey/control establishment, calibration, instrument setup and station moves;
- robot setup, configuration, localization and tool/payload change;
- operator, spotter, supervisor and trade support;
- execution, charging, material handling and waiting;
- inspection, QA, rework/reprint and recovery;
- site change notices, coordination, briefing and requalification;
- evidence processing and closeout required for acceptance.

Record time by resource and interval. Do not double-count one person as two simultaneous workers. Record robot time separately from human crew-hours. If time cannot be attributed, retain an explicit unknown allocation; do not impute zero.

## 3. Derived metrics — no hidden composite

The report schema defines separate metrics. Examples:

- First-pass yield = first-pass accepted units / first-pass inspected units.
- Final acceptance fraction = finally accepted units / attempted units.
- Accepted production rate = finally accepted units / declared resource-hours.
- Rework burden = rework labour-hours / finally accepted units.
- Critical defect rate = critical defects / predeclared critical inspection opportunities.
- Elapsed time per accepted unit = work-package elapsed time / finally accepted units.

Each rate carries its numerator, denominator, unit, task/feature stratum, exclusions and uncertainty/confidence interval. Zero denominators produce `not_estimable`, never zero productivity or 100% yield.

The report must not collapse these into one score. A robot may have high print throughput and poor first-pass acceptance; excellent geometry and excessive upstream coordination; low rework and poor availability. Those trade-offs should remain visible.

The derived count table also separates `not_attempted_units`, `uninspected_units`, `cancelled_units`, `superseded_units`, `unresolved_units` and `active_units`. A never-started or superseded unit is not mislabeled as an uninspected physical output; it remains visible in its own disposition count. Cancelled/superseded work never earns accepted-output credit.

The v0.1 report is a **single-work-package descriptive report**. It deliberately publishes raw numerators and denominators but does not manufacture a confidence interval from one run. Cross-run, cross-robot or cross-site inference belongs to the round-robin/independent replication design: resample or model at the true independent unit (run, site, laboratory or robot serial unit as applicable), not at the level of correlated feature vertices or sensor frames. For first-pass acceptance, use exact binomial intervals only when the independent-unit assumptions are justified; for crew-hour rates and tail residuals, preregister appropriate clustered or hierarchical analysis and report variance components, strata and missing/aborted runs. Never present a per-feature sample count as the number of independent robot trials.

## 4. Baselines and causal attribution

Use a matched manual/current-practice baseline with the same functional unit, design revision, scope, acceptance criteria, site conditions, work sequence and coordination assumptions wherever possible. Prefer randomized/crossover or within-site matched designs. Otherwise state the counterfactual method and identify confounders.

Keep raw comparisons separate from adjusted estimates. Do not claim causal improvement from unmatched projects or compare vendor "up to" claims with independent measurements.

Blocker categories can align with the Cambridge Framework: design/input, site readiness, materials, upstream trade, downstream trade, labour/operator, equipment/instrument, environment, safety hold and unknown. Attribute causes only when evidence supports them; preserve unknown.

## 5. Safety separation

Safety qualification remains a separate hard gate under #3505/#3506 and the safety-specific workstreams. This productivity contract must not:
- convert safety into a productivity score;
- treat safety holds as automatically wasteful;
- incentivize bypassing stops, sensors, spotters or exclusion zones;
- infer safety from geometry, throughput, worker sentiment or low defect rates.

Safety holds and interventions are recorded as operational facts, but their safety verdict and applicability remain independent and non-compensable.

## 6. Layout adapter V0.1

Start with construction layout/printing. The unit class is fixed per scored result: point, line segment, arc, text/annotation group or explicitly defined feature group. Do not pool across classes unless the protocol supplies a justified equivalence and separate class-level results remain available.

The layout acceptance profile binds:
- exact source design, export and revision set;
- target frame, vertical reference, units and transform lineage;
- task footprint and survey-control coverage;
- required feature inventory and criticality;
- robot configuration and worksite epoch;
- independent geometric referee and uncertainty;
- omissions, duplicates, wrong-layer/type and downstream-fit checks.

Reuse #1607 for survey control and metrology, #3561/#3564 for input integrity, #3569 for support-domain coverage and #3523/#3524 for the layout robot benchmark. No second geometry or survey engine is introduced.

## 7. Contract targets

These are reporting/contract targets, not universal performance thresholds:
- 100% of scored units bind to a predeclared functional unit and acceptance predicate.
- 100% of attempted units have an explicit disposition or unresolved status.
- Zero productivity credit for uninspected, duplicate, wrong-revision, wrong-frame or finally rejected units.
- 100% of critical features are inspected or covered by a separately justified predeclared sampling plan.
- 100% of labour-time entries reconcile to the declared scope and resource; unknown time remains explicit.
- 100% of first-pass/final acceptance reports publish exact numerators, denominators and exclusions.
- No cross-task leaderboard without equivalent functional units and acceptance criteria.
- No safety-performance composite.

Do not impose a universal mm tolerance, first-pass-yield threshold or crew-hour improvement target. Set those only against the task risk, downstream requirements, matched baseline, uncertainty and pre-registered decision margin.

## 8. Executable Rust reference implementation

The package at this path is a standalone Rust crate. It:
- validates the closed Draft 2020-12 atomic ledger schema;
- checks contiguous attempt ordering, latest-inspection/terminal-disposition consistency, inspection authority/method/evidence presence, critical open defects, duplicate identities and non-overlapping resource-time intervals;
- binds the ledger's profile ID and functional-unit class to the declared layout adapter;
- derives counts and endpoint-specific rates from unit/attempt/time records rather than accepting producer-authored aggregate metrics;
- emits a SHA-256 digest of the deterministic serialized ledger in the derived report;
- marks zero-denominator measures as `not_estimable`, never as zero productivity or perfect yield;
- exercises a valid synthetic example and a negative control in which an uninspected unit is labelled accepted.

The CLI command is:

```sh
cargo run --manifest-path research/construction-accepted-output-v0-1/Cargo.toml
```

The crate's validator checks structural consistency and profile binding only. It does not fetch or authenticate referenced artifacts, verify signatures/actors, validate instrument calibration or independence, adjudicate physical quality, or qualify safety. The fixture values are invented and must not be cited as observed performance.

Direct crew-hours in v0.1 include on-site survey/control, robot setup/calibration, operator supervision, spotting, execution, charging/changeover, waiting, QA inspection, rework/reprint, interruption recovery and site coordination/requalification. Total workflow crew-hours include every human entry, adding upstream VDC/model preparation, design/trade coordination and evidence closeout. This is an explicit initial convention; freeze the mapping as a signed study policy before confirmatory use and version any later change.

Human resource IDs should be pseudonymous and task-scoped. Do not collect worker names, badge numbers, biometrics, inferred intent or affect. Retain only the role/time evidence needed for the declared productivity claim.

The derived report keeps critical-feature inspection coverage (any inspection) separate from critical first-pass inspection coverage, and reports first-pass defect counts by severity. It also rejects a work package labelled completed while any unit remains active or unresolved. The partially measured example is correctly marked unresolved. Metrics from an incomplete/unresolved work package are marked `provisional`; only a completed and fully reconciled package can produce `estimable` metrics. Attempt intervals must be chronological and non-overlapping per work unit, and all resource-time entries must use the package's declared clock domain. Zero denominators remain `not_estimable`.

## 9. Ownership

- #3605 owns this accepted-output accounting contract.
- #3523/#3524 own the layout robot common-reference performance profile and will consume its layout adapter.
- Cambridge Framework remains the project/work-package/activity productivity reference.
- #3561/#3564 and #3569 own pre-print input and survey support-domain admission.
- #1607 owns survey/control/frame/uncertainty semantics.
- #3521/#3522 own physical trial receipts and raw evidence references.
- #3531 owns claim-specific evidence admissibility.
- #3505/#3506, #3317/#3499/#3542/#3558/#3566 own separate physical, temporal, observation, worksite and chain safety contracts.
- Symthaea #4918 owns claim-relative dependency lineage/common-mode independence.

This is an output and productivity measurement contract, not a safety evaluator, survey engine, project production system, evidence ledger or event store.
