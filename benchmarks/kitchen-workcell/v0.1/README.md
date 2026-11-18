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

## Interoperability and appliance qualification boundaries — 18 November 2026

- **Matter 1.6** (CSA release 17 Jun 2026) adds standardized capability/limit communication, improved status information and Joint Fabric multi-controller sharing. This is useful for transport and device-management interoperability; it does not define meal semantics, verify food lots, prove device safety, or grant a controller permission to actuate. Keep the Symtropy capability contract and appliance-local safety authority above any Matter adapter. See https://csa-iot.org/newsroom/matter-1-6-enables-more-intuitive-setup-multi-ecosystem-experiences-and-context-driven-control/.
- **Dishwasher performance:** IEC 60436:2025 (corrected version April 2026) defines household-dishwasher performance measurement methods and explicitly is not a safety standard or minimum-performance requirement. Cleaning, sanitation, food-contact hygiene and allergen cross-contact remain independent tests. See https://webstore.iec.ch/en/publication/80459.
- **Sensory methods:** ISO 11136:2014 provides controlled-area consumer hedonic testing; ISO 13299:2016 provides sensory profiling. A liking score is not an instrumental texture/temperature assay and neither is food-safety evidence. See https://www.iso.org/standard/50125.html and https://www.iso.org/standard/58042.html.
- **South African test evidence:** SABS identifies its Appliances laboratory as SANAS ISO/IEC 17025 accredited and describes electrical-safety, performance and energy-efficiency tests for household appliances. Comparator selection should therefore include local type-plate identity and the relevant test report/standard scope, not a retailer page alone. The NRCS 2026–27 non-compliant-goods schedule still lists VC 8055 product classes; the applicability and current Letter of Authority requirements must be checked with NRCS for the exact SKU before import or sale. See https://www.sabs.co.za/appliances and https://www.etenders.gov.za/Home/Download?blobName=07d09afb-b05b-4313-b272-7568e2066b08.pdf&downloadedFileName=NRCS+002+2026-27+LIST+OF+NON+-COMPLAINT+GOODS+updated+06+July+2026.pdf.

## Pre-trial metrology gate

The manifest now names the minimum measurement roles without implying that any instrument has been acquired or calibrated. All instrument slots currently have `status=not_selected` and `calibration_ref=null`. The shortlist covers true electrical real-power/transient capture, water, precision and platform mass, temperature, ambient conditions, particle-size distribution, sound and cleaning residue. See [KITCHEN-METROLOGY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5894).

Do not mark a metric as measured until the exact instrument/model/serial, range, resolution, uncertainty, calibration evidence, sampling cadence, placement and raw-data digest are recorded. Invalid or missing measurement terms remain unknown; no instrument is implied to exist by its manifest slot.


## Safety is a separate eligibility gate, not a success bonus

The benchmark includes a task-aware simulated safety audit and [SafeVLA-Bench](https://safevla.org/leaderboard) as an external baseline. Its public leaderboard (updated 26 Sep 2026) separates native task success (SR), episode-level safety compliance, successful-but-unsafe (SBU), and worst-violation severity (VSI). On RoboCasa-365 Atomic-Seen, the safety-ranked leader RLDX-1-FT-RC365 reports 53.1% safety, 58.4% SR, 24.0% SBU and VSI 0.222. The highest success-only row, Xiaomi-Robotics-1, reports 81.9% SR but only 50.1% safety and 37.7% SBU. These are exact rows from the same split, not a universal score. Preserve the full specification set, raw trajectories and evaluator version; a success-only ranking misses unsafe completions.

For our kitchen workcell:
- Report native task success separately from the fraction satisfying every applicable safety spec.
- Count successful-but-unsafe episodes explicitly; a successful task with a safety violation is not an accepted success.
- Report worst violation severity and signal coverage; rare severe violations cannot disappear in a mean.
- Use task-aware applicability, but an absent signal is unknown, not safe or automatically N/A.
- Freeze Signal Temporal Logic (or equivalent) limits and required telemetry before evaluation.
- Simulation-only safety results do not qualify physical robots, pressure equipment, blades, heaters, electrical systems or human interaction. Appliance-local safety protections remain independent and authoritative.

The current safety targets are unverified hypotheses; the manifest remains draft, unpreregistered and has zero runs.

## External robotics benchmark extensions — 18 November 2026

- **RoboCasa365:** the official public leaderboard updated 16 Nov 2026 reports a top agentic result of 0.856 overall, 0.911 Atomic-Seen (18 tasks), 0.850 Composite-Seen (16 tasks) and 0.800 Composite-Unseen (16 tasks). The current research hypotheses are 0.907 overall, 0.955 Atomic-Seen, 0.900 Composite-Seen and 0.860 Composite-Unseen, each on its exact official split. The official 50-task multi-task leaderboard evaluates a policy-observation-only benchmark; reproduce the release/evaluator, horizons, training-data restrictions, task split and base-policy attribution before claiming a comparison. See https://robocasa.ai/leaderboard.
- **BEHAVIOR-1K 2026 hidden test:** the official challenge used 100 full-length household tasks in seven scenes, RGB + depth + proprioception and BDDL partial-credit scoring. The verified winner, HiPhys-RSI / The North Star, reports 0.683 Q and 0.482 full-task success on organizer-held hidden instances (1,000 rollouts: 100 tasks × 10 instances). Our manifest therefore sets a separate stretch hypothesis of at least 0.720 hidden-test Q and 0.520 full-task success, with strict task/embodiment/sensor/time-budget parity and no access to hidden instances or derivative labels during development. See https://behavior.stanford.edu/challenge/index.html and https://behavior1k-champion.com/.
- **BEHAVIOR-1K 2026 public submission:** a published ninth-place paper reports 0.446 Q and 0.273 full-task success on its public submission. Those numbers are not the winning hidden-test scores and are retained only as a separate public-set comparison. Do not compare 0.446/0.273 against 0.683/0.482 as if they shared an evaluation split. See https://arxiv.org/abs/2611.14432.
- **SafeVLA-Bench:** its RoboCasa Atomic-Seen table makes the success/safety gap concrete: the highest success-only row reports 81.9% SR but 50.1% safety and 37.7% SBU; the safety-ranked leader reports 53.1% safety, 58.4% SR and 24.0% SBU. This is why our scorecard keeps success, safety compliance, successful-but-unsafe, and worst violation severity separate. See https://safevla.org/leaderboard.
- These remain separate simulated robotics tracks. RoboCasa365 is kitchen-task manipulation; BEHAVIOR is broader household mobile manipulation; SafeVLA is a post-hoc safety evaluation layer on matched benchmark trajectories. Do not aggregate them or compare their scores to physical appliance outcomes. The manifest still has zero physical runs and remains unpreregistered.