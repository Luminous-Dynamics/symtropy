# Kitchen Workcell Benchmark V0.1

Status: draft / not run. Parent execution issue: https://github.com/Luminous-Dynamics/symtropy/issues/5618.

- `kitchen-workcell-benchmark.schema.json`: structural contract for comparator/task/metric/run manifests.
- `manifest.json`: initial comparator and task basket. All comparators are candidates, not acquired or measured. `runs` is intentionally empty.

## Use

1. Validate `manifest.json` against the local schema with a Draft 2020-12 validator.
2. Freeze the protocol, exact SKU/revision, task input, output assays, meter calibration and randomization before trials.
3. Append a run record only after retaining raw instrument records and input digests.
4. Keep failed, aborted, incomplete and unknown runs in the manifest.
5. Never change `evidence_class` from simulated/modelled to measured because a prediction matched an expected value.


## Experimental design and analysis plan

The V0.1 manifest now carries an explicit `analysis_plan`. It is proposed, not preregistered. The following are design commitments for a future confirmatory study, not evidence that a trial has been conducted.

- **Matched randomized crossover:** use the same meal/task graph, input lots and masses, serving count, accessories, electrical envelope, water boundary, environmental stratum and instrumentation across comparator conditions. Randomize condition order within blocks and balance operators.
- **Freeze exact identity:** comparator is the exact type plate, model/revision, firmware, accessory kit and market SKU—not a retailer product name.
- **Keep endpoints separate:** accepted servings, Wh/accepted serving, water/accepted serving, actual cost/accepted serving, user-present time and blinded sensory liking remain separate estimates. There is no composite score.
- **Calculate power before confirmatory trials:** pilot variance informs the predeclared power calculation and any sensory non-inferiority margin. Required repetitions and panel size are intentionally null until the power analysis is reviewed.
- **Paired inference and uncertainty:** report per-block paired differences/ratios, raw distributions, two-sided 95% intervals, clustered effects by meal/operator/day/revision and propagation of meter/assay uncertainty. The exact inferential model is to be selected and frozen by the reviewer before confirmatory data collection.
- **Retain all attempts:** failed, aborted, indeterminate and withdrawn observations are counted and explained. Unknown measurements are not zero-filled; exclusions are predeclared and sensitivity results retained.
- **Hard gates do not trade:** physical safety, food-contact hygiene, allergen cross-contact, independent conformity and user consent are eligibility gates. Energy, water, speed, throughput, sensory or cost cannot compensate for a gate failure.
- **External benchmark parity:** RoboCasa365, BEHAVIOR-1K and Kitchen-R remain different benchmark families. Compare only matched task splits, evaluator versions, embodiment, sensors, initial states, seeds and data restrictions; never rank a physical appliance trial on a simulator leaderboard.

The first trial basket remains a pilot. Confirmatory sample counts, power analysis, exact sensory method, exclusions, statistical model and all acceptance margins must be frozen before the first confirmatory run. The manifest remains `draft_not_run`, `frozen_before_trials=false`, with an empty `runs` array.

## Comparator sourcing note

Consumer Reports' 2026 public test descriptions are a task-basket source, not a standard or proof of local model availability:
- Multi-cookers: https://www.consumerreports.org/appliances/multi-cookers/best-multi-cookers-of-the-year-a7136971834/
- Food processors: https://www.consumerreports.org/appliances/food-processors-choppers/best-food-processors-of-the-year-a7878155198/

The Breville Paradice 9 has a regional capacity discrepancy in public technical material (Consumer Reports describes a seven-cup model; an Australian technical sheet lists nine cups). Fix the market-specific SKU, capacity, supply voltage and accessory kit before comparison. A comparator name alone is not an adequate test identity.

## Hard boundaries

- Commercial, certified appliances only during first metrology phase.
- No pressure-vessel modification, interlock bypass or unsafe blocked-vent trial.
- Pressure, thermal cut-outs and blade guarding remain local deterministic safety functions.
- Food safety, microbiological lethality and conformity are external qualification gates.
- The current manifest contains hypotheses only; no physical results or SOTA claims.


## Household power-envelope scheduling

The `workflow-power-envelope` task composes with the existing Symtropy [KITCHEN-OPS-001 planner](https://github.com/Luminous-Dynamics/symtropy/issues/2227). Its implementation branch adds independent power aggregation, explicit time-varying task profiles, background load and a conservative serial fallback.

- [PowerConstrainedPlanV1 schema](https://github.com/Luminous-Dynamics/symtropy/blob/feat/kitchen-ops-planner-v1/crates/domains/symtropy-kitchen-planner/schemas/PowerConstrainedPlanV1.schema.json)
- [Synthetic power-envelope fixtures](https://github.com/Luminous-Dynamics/symtropy/blob/feat/kitchen-ops-planner-v1/crates/domains/symtropy-kitchen-planner/schemas/fixtures/power-envelope-v1.json)
- [Planner implementation](https://github.com/Luminous-Dynamics/symtropy/blob/feat/kitchen-ops-planner-v1/crates/domains/symtropy-kitchen-planner/src/power.rs)

No local household limit is assumed. Limits are real watts, not VA/kVA; inverter compatibility also needs power factor, startup waveform and quality evidence. The benchmark's power work remains unrun until physical profiles and calibrated metering exist.

 
## Additional qualification tracks — resilience and lifecycle

The manifest also defines separate scenarios for:

- **Offline/manual resilience:** eligible workflows must remain operable with WAN/cloud unavailable; manual takeover success and latency are measured separately. Camera and microphone are not prerequisites.
- **Repairability:** time and functional verification for declared user-serviceable, non-safety module replacement. Pressure vessels, high-voltage assemblies and safety-critical components are excluded from consumer repair trials.
- **Whole-process accounting:** water per accepted serving and unaccounted mass fraction, with cooking, preparation and cleaning boundaries visible. Unknown water or mass flows remain unknown.
- **Disturbance replanning:** deterministic seeded simulation of task delays, resource outages, reduced power envelopes and stale observations. No disturbance is injected into live hazardous equipment.

These targets are hypotheses, not current results. The manifest still has zero physical runs and is not preregistered. Do not combine these dimensions into one aggregate score or trade them against safety, manual control or evidence completeness.


## Additional external robotics comparator: BEHAVIOR-1K

Use [BEHAVIOR-1K](https://behavior.stanford.edu/) and the released [2026 BEHAVIOR Challenge](https://behavior.stanford.edu/challenge/index.html) task set as a complementary long-horizon household-service benchmark. The base benchmark describes 1,000 everyday activities across 50 interactive scenes and 10,000+ objects; the 2026 challenge round itself used 100 full-length tasks in 7 scenes. The 2026 submission deadline was 16 October 2026 and winners were announced 4 November 2026, so this is a replay/comparison target, not an open submission opportunity.

Use:
- **RoboCasa365** for kitchen-specific mobile manipulation and its reported Atomic-Seen, Composite-Seen and Composite-Unseen splits.
- **BEHAVIOR-1K** for longer-horizon household work and task generalization outside the kitchen-only setting.
- **Kitchen-R** for the separate planning-only, low-level-control-only and integrated kitchen evaluation modes.

Only compare matched tasks, embodiments, sensors, simulator versions and success rubrics. Do not place the three benchmarks on one leaderboard or compare their aggregate success rates as if they were interchangeable. For each external evaluation, record the exact 2026 challenge release, split, task subset, assets/license, robot embodiment, RGB/depth/proprioception availability, partial-credit rule, recovery, collision and human-takeover behavior.


## Dual-graph meal compilation conformance

The integrated manifest now includes a software-only compiler conformance lane that composes with the Symthaea proposal handoff and the Symtropy compiler branch:

- [Symthaea MealProposalV1 handoff PR #12715](https://github.com/Luminous-Dynamics/symthaea/pull/12715)
- [Symtropy dual-graph meal compiler PR #5972](https://github.com/Luminous-Dynamics/symtropy/pull/5972)

The compiler emits two linked representations rather than forcing material transformations into a task DAG:

1. The existing `RecipeTaskGraphV1` for culinary actions, explicit ontology mapping, dependency and scheduling fields.
2. A typed `FoodFlowGraphV1` for raw ingredient allocations, task-output transformations, planned edible outputs and explicit planned losses.

The manifest adds a 28th task and seven software-conformance metrics covering false-compilation rate, unknown promotion, material lineage, evidence-reference coverage, exact acceptance binding, deterministic replay and p95 latency. Its target hypotheses are deliberately separate:
- **Hard gates:** zero false compiled results, zero unknown-to-known promotion, exact acceptance binding, complete material lineage and complete decision-evidence references.
- **Performance hypothesis:** p95 below 50 ms for 100 tasks / 500 material edges on a frozen reference host, only after correctness gates pass.

The conformance lane is synthetic software verification. It does not establish that household stock exists, that lots are physically present or fresh, that allergens or food safety are qualified, that the schedule is feasible, that a module is admitted, that the food is accepted, or that an appliance can be actuated. The manifest still has zero physical runs and remains draft/unpreregistered.

## Pre-trial metrology gate

The manifest now names the minimum measurement roles without implying that any instrument has been acquired or calibrated. All instrument slots currently have `status=not_selected` and `calibration_ref=null`. The shortlist covers true electrical real-power/transient capture, water, precision and platform mass, temperature, ambient conditions, particle-size distribution, sound and cleaning residue. See [KITCHEN-METROLOGY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5894).

Do not mark a metric as measured until the exact instrument/model/serial, range, resolution, uncertainty, calibration evidence, sampling cadence, placement and raw-data digest are recorded. Invalid or missing measurement terms remain unknown; no instrument is implied to exist by its manifest slot.


## Safety is a separate eligibility gate, not a success bonus

The benchmark now includes a task-aware simulated safety audit and [SafeVLA-Bench](https://safevla.org/leaderboard) as an external baseline. SafeVLA-Bench evaluates native task success alongside safety-spec compliance, success-but-unsafe (SBU), and violation severity. Its project paper reports that 38–56% of successful RoboCasa-365 rollouts in its evaluated policy set violated at least one active safety clause. The public leaderboard was last updated 2026-09-26; use its exact spec release and evaluation protocol, not a hand-copied subset.

For our kitchen workcell:
- Report native task success separately from the fraction satisfying every applicable safety spec.
- Count successful-but-unsafe episodes explicitly; a successful task with a safety violation is not an accepted success.
- Report worst violation severity and signal coverage; rare severe violations cannot disappear in a mean.
- Use task-aware applicability, but an absent signal is unknown, not safe or automatically N/A.
- Freeze Signal Temporal Logic (or equivalent) limits and required telemetry before evaluation.
- Simulation-only safety results do not qualify physical robots, pressure equipment, blades, heaters, electrical systems or human interaction. Appliance-local safety protections remain independent and authoritative.

The current safety targets are unverified hypotheses; the manifest remains draft, unpreregistered and has zero runs.

## External robotics benchmark extensions — 17 November 2026

- **RoboCasa365:** current public leaderboard (updated 16 Nov 2026) reports 0.856 overall, 0.911 Atomic-Seen (18 tasks), 0.850 Composite-Seen (16 tasks) and 0.800 Composite-Unseen (16 tasks). The target is 0.907 overall with split floors above the current leader, under the exact official 50-task protocol. The leaderboard allows agentic submissions but requires policy-observation-only operation, no privileged state/success signal, base-policy attribution and disclosure of memory/tooling. See https://robocasa.ai/leaderboard.
- **BEHAVIOR-1K 2026:** the official challenge evaluates 100 long-horizon household tasks across seven scenes with RGB, depth and proprioception; the ranking uses average BDDL task score with partial credit. A published ninth-place paper reports 0.446 Q and 0.273 full-task success for its public submission. These are not the winning scores. The research target is 0.500 Q and 0.320 full completion on the same official protocol. See https://behavior.stanford.edu/challenge/index.html and https://arxiv.org/abs/2611.14432.
- These are separate simulated robotics tracks. RoboCasa365 is kitchen-task manipulation; BEHAVIOR is broad household mobile manipulation. Do not aggregate them or compare either to physical appliance outcomes. Current manifest still has zero physical runs and remains unpreregistered.