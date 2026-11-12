# High-rise climbing workcell benchmark profile v0.1

**Status:** research template; not a project design, safety case, structural approval, certification or physical performance result.

- Parent research issue: https://github.com/Luminous-Dynamics/symtropy/issues/4914
- Machine-readable profile: [HIGHRISE_CLIMBING_WORKCELL_PROFILE_V0_1.json](./HIGHRISE_CLIMBING_WORKCELL_PROFILE_V0_1.json)
- Branch: `research/highrise-climbing-workcell-v0.1`

## Purpose

This profile frames a vendor-neutral comparison of modular self-climbing workcells and complete high-rise floor-production workflows. It treats the platform as a reusable, sensorized production environment that advances with the building—not as an isolated lifting device.

The measured outcome is finally accepted, task-specific work delivered through the complete workflow. A robot action, lift command, route completion, platform-position flag, simulation, or operator-entered counter is not independent acceptance.

## What v0.1 contains

- A source register separating peer-reviewed study results from project-reported context.
- An explicitly unbound evaluation scope. Null values and empty arrays mean unknown, unbound or not supplied; never interpret them as zero or as acceptance.
- System boundaries, external interfaces and lifecycle phases.
- Distinct task families for temporary works, synchronized lifting, reinforcement, formwork, logistics and facade/MEP work.
- Metric contracts with required evidence and task-appropriate denominators.
- Preregistration, matching, measurement-independence and reporting rules.
- P0–P5 staged validation with progressively higher evidence classes.
- Non-compensable engineering, lifting, ODD, safety and independent-acceptance gates.
- Prospective target ladder, existing owner references and unresolved inputs.

## Research reference handling

The 20 August 2026 *Scientific Reports* paper is a peer-reviewed, study-specific reference for a lightweight intelligent aerial construction machine. Its reported mass, material, installation and reuse values are recorded as source-reported comparisons. The profile does not assert that Luminous has reproduced them or that their system boundary maps directly to another building.

The September 2026 Jeddah Tower progress report is contextual project reporting. Its approximate height, floor and core-cycle figures are not a matched baseline or a generic industry target.

Before comparative use, each source must be reconciled against building type, geometry, height, floor scope, loads, wind/ODD, platform configuration, work content, crew, logistics, inspection boundary and measurement method.

## Metric discipline

Do not collapse the metrics into a weighted score.

- Floor-cycle time includes all declared starts, stops, holds, rework and accepted-output disposition.
- Workflow productivity includes design preparation, survey, setup, supervision, kitting, material handling, lifting, inspection, rework, maintenance and recovery inside the declared boundary.
- Robot-hour and machine cycle metrics remain separate from total crew-hour and elapsed project-time measures.
- First-pass and final acceptance publish exact numerators, denominators, inspection methods and exclusions.
- Structural, lifting, safety, quality, environmental and economic outcomes remain separate. No cost or productivity gain offsets a failed safety or engineering gate.

## Safety and authority

A qualified engineer and the accountable lifting / temporary-works process own project-specific structural and lifting decisions. Safety-rated controls, physical restraints/locks and emergency systems remain independent of AI planning. Symthaea may propose and replan within explicitly authorized constraints; it does not approve structural adequacy, lifting, occupancy, anchorage changes, work-zone entry or return to service.

No early physical trial should expose people to test hazards. Physical work proceeds only after the applicable design reviews, hazard analysis, instrumentation, permits, competent supervision and site approvals exist.

## Relationship to existing Luminous owners

This profile intentionally composes rather than duplicates existing contracts:

- Symtropy #1834 capability envelopes; #3505 physical ODD and safety qualification; #3521 physical trial receipts; #3591 round-robin reproducibility; #3605 accepted-output accounting; #3628 endurance; #3703/#3706 VDA 5050 and mixed-fleet interoperability; #4043 inspection/repair/re-verification.
- Symthaea #6033 robot specialization and #6040 benign workcell digital-thread pilot.
- Mycelix #5149 construction provenance and approvals.
- Sol-Atlas #622 design-to-panel-to-maintained-asset thread.

## Immediate next work

1. Review the source boundaries and reconcile the 2026 lightweight-platform paper's reported values.
2. Define a reference floor/work-package scope and freeze the conventional baseline boundary.
3. Complete the system functional decomposition, interface map, hazard log, load-case register and ODD with competent engineering owners.
4. Add a closed JSON Schema and deterministic fixture tests; do not call this file schema-validated until those checks run against the exact commit.
5. Keep P0/P1 desk and software work separate from any physical lift or site execution.

## Claim ceiling

A valid JSON document or a passing schema/fixture suite proves only document/contract consistency. A desk study or model does not establish physical strength, structural adequacy, safe lifting, site safety, code compliance, productivity superiority, reliability or transferability.
