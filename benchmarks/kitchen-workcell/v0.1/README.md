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

Use [BEHAVIOR-1K](https://behavior.stanford.edu/) and the [2026 BEHAVIOR Challenge](https://behavior.stanford.edu/) as a complementary long-horizon household-service benchmark. The official project describes 1,000 everyday activities across 50 interactive scenes and 10,000+ objects; it is broader than a kitchen benchmark.

Use:
- **RoboCasa365** for kitchen-specific mobile manipulation and its reported Atomic-Seen, Composite-Seen and Composite-Unseen splits.
- **BEHAVIOR-1K** for longer-horizon household work and task generalization outside the kitchen-only setting.
- **Kitchen-R** for the separate planning-only, low-level-control-only and integrated kitchen evaluation modes.

Only compare matched tasks, embodiments, sensors, simulator versions and success rubrics. Do not place the three benchmarks on one leaderboard or compare their aggregate success rates as if they were interchangeable. For each external evaluation, record the exact 2026 challenge release, split, task subset, assets/license, robot embodiment, onboard sensing, partial-credit rule, recovery, collision and human-takeover behavior.


## Pre-trial metrology gate

The manifest now names the minimum measurement roles without implying that any instrument has been acquired or calibrated. All instrument slots currently have `status=not_selected` and `calibration_ref=null`. The shortlist covers true electrical real-power/transient capture, water, precision and platform mass, temperature, ambient conditions, particle-size distribution, sound and cleaning residue. See [KITCHEN-METROLOGY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5894).

Do not mark a metric as measured until the exact instrument/model/serial, range, resolution, uncertainty, calibration evidence, sampling cadence, placement and raw-data digest are recorded. Invalid or missing measurement terms remain unknown; no instrument is implied to exist by its manifest slot.
