# High-rise reference-floor schedule v0.1

**Status:** frozen synthetic scenario proposal; not a real project schedule, construction estimate, method statement, safety case or engineering approval.

- Parent: [HIGHRISE-CLIMBING-WORKCELL-001 #4914](https://github.com/Luminous-Dynamics/symtropy/issues/4914)
- Adapter issue: [HIGHRISE-REFERENCE-FLOOR-ADAPTER-001 #4937](https://github.com/Luminous-Dynamics/symtropy/issues/4937)
- Fixture: [HIGHRISE_REFERENCE_FLOOR_SCHEDULE_V0_1.json](./HIGHRISE_REFERENCE_FLOOR_SCHEDULE_V0_1.json)
- Companion schema: [HIGHRISE_REFERENCE_FLOOR_SCHEDULE_SCHEMA_V0_1.json](./HIGHRISE_REFERENCE_FLOOR_SCHEDULE_SCHEMA_V0_1.json)

## What this fixture is

One abstract notional floor with nine generic work packages: survey/control, temporary-works readiness, reinforcement, MEP embeds, formwork close, pre-pour inspection, concrete placement, strength/inspection release, and formwork strip/reposition.

All durations are abstract integer planning ticks. They are not hours, days, measured crew rates, construction instructions, or project estimates. Resource capacities are abstract scheduling capacities. No geometry, floor area, structural system, jurisdiction, actual lift plan, or actual equipment is supplied.

The fixture deliberately reuses the task, resource, precedence, duration, hard-gate and expected-witness fields from the generic Habitat Scheduling Micro-Oracle (#1902). It does not define a second resource-constrained scheduler. Its construction-specific contribution is the task-to-functional-output mapping and the reference-floor scenario family.

## Four frozen cases

| Case | Nominal makespan | Upper-duration makespan | Result |
|---|---:|---:|---|
| HF01 — one abstract hoist | 17 ticks | 26 ticks | Nominal feasible; not robust to 17-tick deadline |
| HF02 — two abstract hoist-capacity units | 15 ticks | 23 ticks | Nominal feasible; not robust to 17-tick deadline |
| HF03 — missing pre-pour hold-point gate | — | — | Infeasible; required gate unavailable |
| HF04 — inspection rejection, corrective work and reinspection | 20 ticks | 31 ticks | Nominal feasible; not robust to 20-tick deadline |

The feasible fixtures have explicit witnesses. The nine-task cases have two precedence-valid topological orders; the rework case also has two. An exhaustive topological-order / earliest-resource-feasible placement check produced the stated small-instance makespans. This local reference check is not a substitute for running the separate #1902 oracle/planner implementation on the exact fixture bytes; that integration remains a follow-on.

### What the hoist comparison demonstrates

In HF01, REBAR_INSTALL and MEP_EMBEDS both become eligible after TEMP_WORKS_READY, but they contend for one exclusive abstract HOIST unit. They therefore run sequentially, yielding a 17-tick nominal makespan.

HF02 raises the synthetic HOIST resource capacity from one to two, permitting the two tasks to overlap. The nominal makespan becomes 15 ticks, a 2-tick / 11.7647% schedule reduction for this fixture.

This is **not** a productivity or investment recommendation. The model omits the purchase/lease cost, support structure, access envelope, physical layout, safety functions, maintenance, operator requirements, energy and lifecycle cost of the additional capacity. It does not establish that two actual hoists can serve the workface safely or concurrently.

### What the rework case demonstrates

HF04 preserves the initial pre-pour inspection as a rejected disposition. Corrective work and a second inspection become explicit predecessor tasks before concrete placement can be scheduled. The failed first inspection contributes no accepted-output credit. This demonstrates the output contract, not a real defect-remediation method.

## Why the model is useful

It makes several hidden sources of floor-cycle loss visible:
- parallel work can still be serialized by an exclusive resource;
- additional capacity may change the schedule, but has an unmodelled economic/physical cost;
- required approval/inspection gates are hard prerequisites;
- inspection rejection adds real downstream delay and rework;
- nominal duration feasibility does not imply upper-duration robustness;
- a schedule is not an accepted physical output.

The next extension should add bounded disruptions from #1843: hoist outage, late/damaged material, robot/tool failure, stale design/site epoch, blocked workface and incomplete observation. The scenario must describe what is known at decision time and permit safe hold/replan rather than perfect-information hindsight.

## Strategy comparison is intentionally unpopulated

B0 conventional/manual, B1 assisted/digitally guided and B2 guarded robotic workcell route profiles are not parameterized. Do not fill route durations or labour assumptions from intuition. Populate them only from matched task scopes, full-workflow time studies, independent acceptance, quality, cost and safety evidence.

## Measurement alignment

The reporting hierarchy should align with the Cambridge Framework for Construction Productivity at project, work-package and activity levels. Do not create a second generic construction productivity taxonomy.

Use NIST's task-driven robotic performance decomposition for perception, mobility, dexterity and safety components where applicable, while retaining whole-work-package accepted output as the delivery metric.

- Cambridge Framework: https://www.construction.cam.ac.uk/research-and-innovation/performance-productivity
- NIST Performance Assessment Framework for Robotic Systems: https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems

## Generic scheduler adapter

The companion [HIGHRISE_REFERENCE_FLOOR_MICRO_ORACLE_V0_1.json](../fixtures/HIGHRISE_REFERENCE_FLOOR_MICRO_ORACLE_V0_1.json) projects the same four cases into the exact generic fixture envelope consumed by the #1902 Habitat Scheduling Micro-Oracle. It changes only suite identity/status/claim ceiling and keeps the task, resource, precedence, gate, duration and expected-result fields in the existing shape.

This adapter is intentionally a duplicate serialization of the cases, not a second scheduler schema or solver. The construction-specific task-to-output contracts and floor-level interpretation remain in the high-rise profile. Once #1902's tool branch is available on the integration branch, run its existing oracle and hybrid planner against this fixture unchanged. Preserve the exact fixture bytes, toolchain and output receipts.

## Validation boundary

The JSON is intended to be checked by the companion closed JSON Schema and then translated into the exact #1902 generic schedule fixture shape for the existing exhaustive oracle and hybrid planner. A schema pass establishes document shape only. A local witness check establishes only the frozen abstract schedule semantics. Neither establishes field productivity, physical safety, code compliance, lift/temporary-works adequacy or an optimal real-world floor programme.

No task execution, procurement, motion authority, lifting, site access or construction authorization is represented by this profile.
