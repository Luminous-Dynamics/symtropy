# Personal and Family Vehicle Engineering Substrate — v0.1

**Status:** Research/benchmark contract, not a validated vehicle model or implementation.  
**Date:** 2026-10-17  
**Scope:** Affordable, serviceable passenger-vehicle design exploration; initial target is a small family road vehicle, with separate models for runabouts and rough-road utility vehicles.

## Existing work to compose, not duplicate

- Symtropy's [Vehicle and Mobility Design Bible](./Symtropy%20Vehicle%20%26%20Mobility%20Design.md) treats mobility, maintenance, access and provenance as system properties.
- [Vehicle and Spacecraft Physics and Operations Runtime](./VEHICLE_SPACECRAFT_PHYSICS_AND_OPERATIONS_RUNTIME_V0_1.md) already defines the shared runtime shape and distinct ground/water/air/rail/orbital profiles.
- [TRANSPORT-OPT-001 #1601](https://github.com/Luminous-Dynamics/symtropy/issues/1601) owns broad whole-life social-cost transport optimization.
- [TRANSPORT-SAFETY-001 #1636](https://github.com/Luminous-Dynamics/symtropy/issues/1636) owns conservative qualification of physical cargo loading/securing before dispatch.
- Symthaea's [road/off-road vehicle dynamics profile #6247](https://github.com/Luminous-Dynamics/symthaea/issues/6247) owns road-vehicle tire/contact, braking, handling, stability and evidence boundaries.

This specification does **not** propose another universal solver or duplicate the multimodal network optimizer. It adds an engineering-focused benchmark contract connecting a physical-vehicle model, repairability metadata, lifecycle economics and evidence-qualified simulation.



## Current artifact and independent-verifier status (2026-10-17)

The draft now contains:
- 21-case synthetic vehicle scenario corpus (`fixtures/personal_vehicle/scenario_corpus_v0_1.json`);
- 15-entry rights-aware benchmark/source inventory (`fixtures/personal_vehicle/benchmark_source_registry_v0_1.json`);
- a five-model South African advertised-price screening shortlist (`fixtures/personal_vehicle/south_africa_baseline_shortlist_v0_1.json`);
- common cross-project engineering evidence envelope schema plus 15 fixtures (`schemas/transport/engineering_evidence_envelope_v0_1.schema.json`, `fixtures/personal_vehicle/engineering_evidence_envelope_fixtures_v0_1.json`);
- repairability task-graph schema plus 16 fixtures (`schemas/transport/repair_task_graph_v0_1.schema.json`, `fixtures/personal_vehicle/repair_task_graph_fixtures_v0_1.json`);
- a preregistered workshop study protocol (`docs/tech/REPAIRABILITY_BENCHMARK_PROTOCOL_V0_1.md`);
- an 18-case South African localization/supply matrix across 3 production-route hypotheses × 3 volume tiers × 2 supply/FX conditions, plus 5 negative controls (`fixtures/personal_vehicle/localization_scenario_matrix_v0_1.json`, `schemas/transport/localization_scenario_matrix_v0_1.schema.json`).

The separate household-continuity workstream adds a 24-case synthetic 3-architecture × 8-stressor matrix including three combined-shock profiles and five adversarial controls (fixtures/personal_vehicle/mobility_continuity_scenarios_v0_1.json), an explicit model-config contract (schemas/transport/mobility_continuity_model_config_v0_1.schema.json), the protocol docs/tech/MOBILITY_CONTINUITY_BENCHMARK_PROTOCOL_V0_1.md, and a deterministic Rust proxy simulator in tools/mobility-continuity-simulator/. The simulator replays two fixed hypothetical outages over a 364-day/52-week horizon, applies synthetic per-week budget constraints, keeps primary and fallback trip service distinct, and emits input/report digests. It reports only synthetic variable mobility spend; whole-life cost remains null while required cost inputs are absent. It is not calibrated, does not model vehicle physics, does not verify real-world fallback availability, and does not establish reliability, road safety or regulatory approval. Calibration/held-out-validation work is tracked in https://github.com/Luminous-Dynamics/symtropy/issues/1791.

The standalone Rust contract tool at `tools/personal-vehicle-corpus-audit/` validates the vehicle-scenario corpus, source inventory, dated baseline shortlist, engineering-evidence envelope and repairability task graphs. The localization scenario package at `tools/vehicle-localization-scenario-audit/` separately validates the 18-case route/volume/supply matrix, source bindings, fail-closed policy route, null cost fields and five negative controls. A focused companion crate at `tools/repair-task-graph-audit/` validates the graph schema and its positive/negative fixtures separately. The report includes corpus/source/evidence/baseline digests and graph-fixture census. The path-scoped workflow `.github/workflows/personal-vehicle-corpus-audit.yml` uses Rust 1.96.0 and includes the schemas, fixtures and all three validator paths (corpus/evidence, repair graph, and localization matrix) in its triggers and test steps. The code is committed on the draft branch but **has not been compiled or executed in this environment**; exact-head hosted runs remain pending. Queued, skipped, cancelled, absent or failed-before-test workflow results are not PASS.

The evidence envelope binds artifact and subject/configuration identity, producer repository/revision/tool version, input/output digests, evidence class, conditions/scope, uncertainty, source/rights status, privacy class, limitations and claim ceiling. Semantic checks reject promotions such as a simulation run asserted as an instrumented-vehicle test or an independent review asserted as a regulatory receipt. It remains distinct from Mycelix's signed lifecycle-event/consent record; signature or cross-reference alone does not promote evidence.

The repairability graph expresses versioned service steps, exact component revisions, dependency edges, tools/availability, safety preconditions, estimate-vs-observation timing, procedure reuse status, lifecycle-cost inputs and a named post-repair check. Its semantic validator rejects duplicate IDs, dangling/duplicate edges, cycles, missing/unreachable post-checks and access/removal/repair/reassembly tasks that skip required isolation. The “observed-shaped” example uses synthetic values to validate record shape only; it is not workshop evidence.

The 15-entry source registry differentiates standard input schedules from measured validation data, records sources for repairability/autonomy/safety planning, and adds official South African manufacturer pages with clear source and quote limitations. No outside dataset or normative standards text is vendored. Neither the registry nor a public manufacturer page is a completed legal/data-rights review.

Even a successful contract verifier would establish only schema/corpus/metadata conformance. It does not compute vehicle dynamics, verify Mycelix cryptographic signatures, adjudicate rights, establish legal title, qualify a physical vehicle or repair procedure, measure workshop performance, or establish regulatory compliance.

## Repairability compiler: service procedures become explicit task graphs

The next workstream adds `schemas/transport/repair_task_graph_v0_1.schema.json`, 16 synthetic fixtures at `fixtures/personal_vehicle/repair_task_graph_fixtures_v0_1.json`, and an independent Rust package at `tools/repair-task-graph-audit/`.

The graph separates task nodes from dependency edges. Nodes bind exact vehicle configuration and component revision, step kind, tool availability, safety preconditions and duration provenance. The initial validator requires:
- unique task IDs, known edge endpoints, no duplicate edges and a directed acyclic graph;
- every task to reach the declared post-repair verification node, which must be terminal;
- safe isolation upstream of every task that explicitly requires it and of access/removal/repair/reassembly tasks;
- hard-stop preconditions to be marked as required before the task;
- estimated duration for a `procedure_hypothesis`, versus repeated, source-bound observation-shaped durations for an `observed_workshop_trial`;
- explicit units, procedure reuse status and a claim ceiling.

The 16 fixtures contain two accepted synthetic graph shapes, six semantic-negative controls (cycle, duplicate node ID, dangling edge, missing post-check, missing isolation, duplicate edge), and eight schema-negative controls. Even the “observed” fixture uses synthetic numbers—it only tests the *shape* required of a measurement record, not any actual workshop outcome. The standalone verifier has been added to the existing path-scoped Rust workflow. Source assets were committed and JSON/semantic fixture expectations were inspected in-session, but this new Rust verifier has not been compiled or executed here; the exact-head workflow must establish that evidence.

This is a useful paradigm shift because “repairability” is no longer one score or a vague CAD attribute: the same versioned graph can support access analysis, tool/parts dependencies, measured technician timing, downtime and rework accounting, and whole-life cost tradeoffs. However, graph validity does not prove a repair sequence is mechanically safe or complete. Only an approved procedure and controlled physical workshop trials can establish those claims.


## Confirmatory repairability measurement protocol

The preregistration template [Repairability Benchmark Protocol v0.1](./REPAIRABILITY_BENCHMARK_PROTOCOL_V0_1.md) now separates a small pilot from confirmatory evidence. It specifies event clocks for diagnosis, hands-on labor, bay elapsed time, parts/tool waiting, verification, rework and out-of-service time; recommends 5 qualified technicians × 6 repetitions per baseline/candidate/task cell as a planning default; and requires power analysis to refine that sample size when feasible. A 3-technician × 3-repetition pilot is for method calibration, not a defensible P90 victory claim.

Its hard gates prevent faster-but-incomplete work from counting as success: approved procedure, required isolation, compatible parts/tools and post-repair verification are mandatory. It calls for task-level medians/quantiles, confidence intervals, censoring/deviation ledger, technician effects, and priced local lifecycle inputs. The target of 25% lower median/P90 time on preregistered high-impact tasks remains a hypothesis until a matched physical study and independent review support it.


## South African benchmark shortlist (screening only)

To stop optimizing against a fictional competitor, the draft now includes `fixtures/personal_vehicle/south_africa_baseline_shortlist_v0_1.json`. The initial set is deliberately heterogeneous; it establishes price/utility boundaries, not a ranking of vehicles or a donor-platform decision.

| Comparator | Advertised starting price captured on 2026-10-17 | Role in the benchmark |
|---|---:|---|
| Toyota Vitz | R182,400 | Entry-price / small-hatch boundary |
| Suzuki Swift 1.2 GL MT | R228,900 | Small-hatch efficiency and serviceability |
| Toyota Starlet | R273,700 | Mainstream hatchback and family-utility comparison |
| Toyota Starlet Cross | R311,000 | Raised-hatch and rougher-road-use comparison |
| Toyota Rumion | R314,100 | Higher-capacity family packaging comparison |

Price snapshots come from the [Toyota South Africa vehicle range](https://www.toyota.co.za/vehicles) and [Suzuki Auto South Africa Swift model page](https://www.suzukiauto.co.za/cars/swift); Swift's published geometry/powertrain screen is linked to its [official specification page](https://www.suzukiauto.co.za/specs/swift). The manufacturer pages advertise *from* prices, not written dealer quotations or total on-road cost. The Swift page lists 3,860 mm length, 265 L boot volume, 145 mm ground clearance and a fuel-consumption figure from 4.4 L/100 km; treat all values as trim/source-specific manufacturer claims, not independently measured operating results.

The baseline file stores source IDs, exact unknowns and fields still needed. Before any trade study, capture exact model year/grade, safety evidence, written dealer quotes, warranty/service-plan terms, a common parts basket and repair times from the actual target market. Add used vehicles only with documented condition, odometer, service history and inspection.

**Comparison rule:** the cheapest advertised vehicle is not automatically a valid family-car baseline. Enforce a minimum passenger/cargo/child-seat/payload/handling utility gate first, then compare acquisition, energy, service, repair labour/parts, waiting, downtime and end-of-life over a matched duty cycle. The initial five-price list does not prove safety, reliability, repairability or ten-year cost.


## South African manufacturing route: localization-ready, not subsidy-dependent

The source registry now records the dtic's **15 October 2026** announcement that South Africa plans to phase out current semi-knocked-down (SKD) operations and will no longer support new SKD facilities, plus the SARS APDP2 programme reference. This is a relevant manufacturing-policy risk for any local-build business case, but the announcement does not by itself resolve every facility's transition, eligibility or legal obligations. Confirm current instruments, implementation dates and project eligibility directly with dtic, ITAC and SARS before financial commitments.

The new [MOBILITY-LOCALIZATION-001 #1780](https://github.com/Luminous-Dynamics/symtropy/issues/1780) workstream models this as a robust manufacturing/parts/service scenario problem. For design-space optimization, compare production scenarios instead of hard-coding a single tariff/incentive story:
- imported complete vehicle / imported subassembly dependence;
- locally made service parts and consumables;
- local component sourcing with qualified supplier/quality evidence;
- tooling, repair workforce and regional service capacity;
- meaningful local value-add / eligible CKD-style manufacturing only where the current written policy confirms the route;
- localization capex, skills, supplier qualification, homologation and warranty costs.

The objective is not “maximize local content regardless of cost.” It is to find robust designs that remain serviceable and economically plausible when policy, supplier access or foreign-exchange conditions change. Each manufacturing scenario must include dated primary sources, uncertainty and explicit non-eligibility assumptions; the public source inventory is not legal advice or an incentive approval.


## South African localization and supply-resilience matrix

The branch now includes a Draft 2020-12 schema and frozen scenario matrix at `fixtures/personal_vehicle/localization_scenario_matrix_v0_1.json`. Its 18 cases form a complete (3 × 3 × 2) grid:

- **Routes:** import complete vehicles as a market baseline; import qualified safety-critical modules while developing a local service-parts/network strategy; local value-add/CKD-style manufacture only after the applicable route is confirmed in writing.
- **Volume sensitivities:** 250, 2,500 and 25,000 vehicles/year, all labeled synthetic tiers rather than forecasts.
- **Supply conditions:** stable reference and stress inputs (FX multiplier 1.2, imported-part lead-time multiplier 2.0, assumed supplier availability 0.85, demand multiplier 0.7), expressly labeled sensitivity assumptions rather than observations.

All route cost fields remain `null`: no BOM, landed quote, local labor/training, capex, homologation, warranty/rework, parts inventory, incentive or 10-year TCO has been invented. The validator fails closed if a cost is filled before the model's evidence inputs exist, if a policy source is missing, if a route/volume/condition cell is missing, if a route gate is weakened, or if any of the five negative controls is relabeled.

This design avoids two common optimization failures: **missing data silently becoming free**, and **policy uncertainty silently becoming a manufacturing assumption**. The matrix is prepared for later priced/qualified inputs, but is not a factory-cost estimate, an incentive calculation, legal advice, or proof that any manufacturing route qualifies. The policy references are the [dtic SKD announcement dated 15 October 2026](https://www.thedtic.gov.za/minister-tau-announces-end-of-semi-knock-down-operations-in-sas-automotive-sector/) and the [SARS APDP2 programme page](https://www.sars.gov.za/customs-and-excise/registration-licensing-and-accreditation/automotive-production-and-development-programme-apdp/); each has a different evidentiary role, and direct project-specific confirmation remains necessary.

## Core paradigm shift: design a vehicle and its maintenance ecosystem jointly

Optimize across:
1. **Physical performance:** safe stopping/handling capability, realistic energy use, thermal margins, payload and ride response under declared conditions.
2. **Repair performance:** diagnostic access, service-task steps, replacement-part scope, required tools, expected downtime and parts lead time.
3. **Ownership economics:** acquisition, energy, maintenance, parts, labour, downtime, depreciation and end-of-life assumptions over a declared duty cycle.
4. **Availability and resilience:** local parts/repair support, energy infrastructure, repair workforce and bounded response to disruptions.
5. **Human utility:** safe usable seating, child-seat installation, cargo, visibility, climate protection, accessibility, rural/intercity reach and driver fatigue.
6. **Evidence quality:** model fidelity, calibrated parameters, uncertainty, physical validation, dataset rights and claim ceiling.

These measures must not collapse into a single opaque “vehicle score.” Generate Pareto fronts and expose safety, legality and minimum utility as hard constraints.

## Model partition

| Model domain | Required profile | Do not infer |
|---|---|---|
| Vehicle dynamics | Tire/road friction and slip, contact, suspension, steering, braking, grade, camber, payload, center of gravity, yaw/lateral response | Crashworthiness, roadworthiness, or certified handling |
| Energy/powertrain | Exact drivetrain class, efficiency maps/source, battery/engine thermal limits, duty cycle and environment | Universal powertrain superiority from a single cycle |
| Durability | Declared road roughness, pothole/impact inputs, climate, dust/water exposure, wear and maintenance intervals | Lifetime durability from accelerated or game-time simulation alone |
| Serviceability | Versioned bill of materials, common repair task graph, tools, access/removal steps, part availability, labor and downtime inputs | Repair-time advantage without timed, repeatable physical trials |
| Ownership economics | Region-specific quotes/prices, currency/date, mileage, energy cost, finance, repair-rate assumptions and sensitivity distributions | Market price, savings or resale estimates beyond their sources/scope |
| Mobility network | Routes, charging/fueling, workshop/parts coverage, transfer cost, closure/disruption scenarios | Vehicle quality from shortest-path or average-network scores |
| Autonomy (future) | Explicit operational design domain (ODD), actor/scenario set, sensor/actuator/configuration and fallback assumptions | General self-driving competence from this model or a synthetic scenario score |

The physical solver may be Symtropy's rigid-body engine or an explicitly identified specialized engineering solver. Each profile must identify its solver; “Symtropy simulation” alone is not a fidelity statement. Do not use a gameplay approximation for safety-critical predictions without fit-for-purpose validation.

## Versioned run manifest

Each run should record at least:

- `schema_version`, `vehicle_model_id`, `variant_id`, `configuration_digest`;
- geometry/mass/inertia/CG and exact component revisions;
- `solver_id`, `solver_version`, commit/build hash, fidelity profile and license;
- initial/boundary conditions, units, environment, road-surface/friction profile and payload;
- timestep/integrator/solver tolerances, random seed, deterministic settings and platform;
- input dataset IDs, rights/provenance and calibration/validation split;
- parameter distributions, uncertainty method and out-of-domain detection;
- output artifact digests, observables, confidence intervals, failed convergence and warnings;
- `evidence_class`, source refs, limitations and a claim ceiling.

No golden-result replacement without a versioned change note and an independent oracle. Preserve failing runs and negative controls.

## Scenario corpus v0.1

Start with a compact passenger/cargo model with declared dimensions, mass, wheelbase, CG, tire assumptions and drivetrain. Keep it synthetic until each parameter is sourced or measured.

Freeze deterministic scenario families:
1. nominal cruise, acceleration and low-speed manoeuvre;
2. defined dry/wet/low-friction surfaces and abrupt friction transition;
3. split-friction braking left-to-right;
4. brake temperature/fade and regenerative-to-friction brake handoff;
5. payload and CG shifts, including boundary/overload rejection;
6. grade, camber, cornering and emergency obstacle manoeuvre;
7. uneven road/pothole excitation, suspension travel and wheel unloading;
8. tire type, pressure and wear-generation change;
9. steering actuator degradation, sensor bias and stale calibration;
10. powertrain derating, battery/engine heat, low state of energy;
11. sensor dropout and communications loss for future assisted/autonomous functions;
12. component replacement with incompatible or unqualified revision;
13. scheduled maintenance withheld or overdue wear item;
14. local parts shortage, repair shop unavailability and service recovery;
15. negative control: model is outside calibrated profile and must return `insufficient_evidence` rather than a confident answer.

For every case bind exact input/configuration digests and explicit expected invariant or tolerance. Don't seed outcomes from case IDs; independent validators recompute expected predicates from case facts.

## Benchmark targets

All numeric targets below are provisional development gates; first establish the baseline and use independent reference data. Passing them is not type approval or a declaration that the vehicle is safe.

### Gate A — determinism and evidence integrity

- 100% of frozen corpus cases produce the documented status class (pass/fail/insufficient evidence) from an independent validator.
- Same binary, platform, inputs and deterministic settings: bit-identical outputs for deterministic kernels across 30 clean reruns; if a solver cannot guarantee this, define a justified numerical tolerance and report it explicitly.
- 100% rejection of frozen manifest tampering, missing model identity, changed config digest, mismatched units, missing source or solver version, and invalid/expired calibration fixtures.
- 100% of out-of-domain and unconverged cases fail closed; no hidden default parameters.

### Gate B — physics agreement (screening, not certification)

Validate on at least one held-out dataset or instrumented test campaign that was not used to tune the same parameters. Publish exact source/dataset licensing and all exclusions.

Provisional initial targets for a low-cost design-screening model:
- longitudinal speed and acceleration traces: mean absolute error within 10% of a declared normalized reference metric;
- energy consumption over the same declared cycle: within 10% of the measured/reference value;
- braking distance within 10% on predeclared surface, tire, speed, loading and temperature conditions;
- yaw rate, lateral acceleration and suspension-travel errors reported against declared measurement uncertainty; set numeric acceptance only after reference quality and sensor calibration are known.

Do not use percentage error without defining the denominator or behavior near zero. Report absolute error, relative error, peak error, bias, confidence interval and excluded/out-of-domain cases. A model that fails one profile may remain useful for another if the limitation is explicit.

### Gate C — lifecycle and repairability trade study

Use 10-year / 150,000 km as the initial comparison horizon; also run low-mileage, high-mileage and different ownership-horizon sensitivities. Compare at least three actually available baseline vehicles or architectures for the target market.

- Provisional ambition: at least 15% lower modeled total cost of ownership versus a matched baseline while meeting the same minimum safety, passenger, cargo, range and weather requirements.
- Test at least ten common service tasks on hardware or a physical serviceable mock-up, then target at least 25% lower median and 90th-percentile completion time versus baseline without increasing unsafe states or follow-on rework.
- For the 20 parts with highest expected maintenance/collision/downtime impact, attach region/date-specific price, source, lead time, compatibility restrictions and an independent repair path where one exists.
- Keep energy, purchase, maintenance, financing, resale and downtime as separate ledgers. Avoid double-counting operator expenses and customer payments as separate social costs.
- Publish Pareto alternatives for TCO, energy, repair time, downtime, mass, useful space and environmental impacts; sensitivity-test any scalar weighting.

These are hypotheses until backed by measurements, not promised savings.

### Gate C.5 — multi-fidelity modeling and value-of-information

Do not maximize simulation count. Select the next analysis or physical test by asking which uncertainty could change an engineering decision and what evidence would resolve it.

- Maintain an uncertainty ledger for each parameter/claim: source, distribution or bounded interval, correlation assumptions, model discrepancy, confidence class, and which decisions it can change.
- Use low-cost analytical estimates and broad parameter sweeps to screen the design space; run higher-fidelity component models only where sensitivity, constraint proximity or ranking ambiguity justifies the expense; use physical tests to calibrate discrepancies and falsify model assumptions.
- Keep calibration and held-out validation data separate. Selecting a new test because it agrees with the current model must not leak that same result into the holdout set.
- Pre-register each expensive test's decision relevance, expected observables, acceptance tolerance, instrument calibration, failure interpretation and evidence class before seeing its result.
- Estimate decision value using expected reduction in decision regret / uncertainty of the Pareto ranking, divided by the test's cost, elapsed time and resource burden. Never use this economic score to defer a legally required or safety-critical test.
- If small changes to energy price, annual distance, repair rate, part lead time or model discrepancy reverse the winning architecture, report a fragile/unresolved recommendation and prioritize the measurements that discriminate between those alternatives.
- Calibrate prediction intervals as well as point estimates; report interval coverage, bias and error by condition. Out-of-domain or non-converged predictions abstain rather than extrapolate silently.

**Provisional decision-quality target:** for *cost-only* recommendations, aim for the same leading architecture in at least 90% of the preregistered, plausible uncertainty scenarios before calling the choice robust. If the target is not met, present the Pareto set and identify the highest-value resolving tests. This is not a safety metric: hard safety/legal constraints remain non-negotiable.

### Gate D — resilience and service continuity

Evaluate bounded disruption profiles: unavailable parts, workshop outage, fuel/charging interruption, key subsystem failure, heat wave, flood/pothole damage and loss of a route. Measure essential trips preserved, time to restore service, dependence on single suppliers and the repair/safety trade-off. A vehicle-level result must not be treated as a network-availability result.

### Gate E — future automated driving

Autonomy is a separate research stage after the physical vehicle and its safety baseline are mature. Require:
- an explicit and narrow ODD with environmental, road, speed, visibility and system-health bounds;
- a scenario corpus with nominal, edge, adversarial and negative cases, including vulnerable road users and failures;
- independent safety invariants and a fallback that does not depend on Symthaea's cognitive layer remaining healthy;
- metrics for collision/contact, minimum separation, rule violations, intervention/fallback success, false positives/negatives, ODD exit detection, degraded sensors and software/configuration mismatch;
- simulation, software-in-loop, hardware-in-loop, closed-track and independent review stages, each tied to exact hardware/software revisions.

Never treat “zero collisions in the simulation corpus” as proof of safety. ISO 21448 addresses hazards from functional insufficiencies and performance limitations; ISO 26262 functional safety is a separate necessary workstream where applicable. Neither standard is satisfied merely by mentioning it here.

## Integration with the cross-project evidence chain

- **Symthaea:** generates engineering alternatives and consumes the road dynamics, braking/handling and qualification owner #6247; its design outputs are proposals until independently verified.
- **Symtropy:** orchestrates simulation cases and design-space trade studies; every result is tied to exact model, solver, parameter and dataset revisions.
- **Mycelix:** records owner-consented provenance, maintenance events, part identity, claims, disclosures and evidence references. It must not rewrite a physics result or infer title/safety from a signed record.
- **Physical test provider / engineer:** issues separate measurements and scoped review receipts. A simulation receipt never changes itself into a physical-test receipt.
- **Lifecycle optimizer:** composes these sources with Symtropy #1601's whole-life social-cost model while keeping hard safety/legal constraints, private cost and social cost distinct.

Use an artifact-reference envelope rather than synchronous dependency across repos. Proposed fields:
`schema_version`, `artifact_kind`, `artifact_id`, `subject_ref`, `producer_ref`, `producer_revision`, `inputs_digest`, `outputs_digest`, `evidence_class`, `scope`, `uncertainty`, `limitations`, `source_refs`.

## Physical qualification ladder

```text
requirements + sourced parameter set
    -> reproducible synthetic model
    -> calibrated model with held-out validation
    -> component/rig tests
    -> instrumented prototype tests
    -> independent engineering + regulatory assessment
    -> controlled field evaluation
    -> production/maintenance feedback loop
```

Promotion is scoped and reviewable. No stage allows simulation to stand in for crashworthiness, braking qualification, public-road approval or applicable type approval. For any actual design, identify the target market and legal route before building a road-going prototype.

## Immediate next actions

1. Freeze the personal-family mission and comparator selection.
2. Reuse #6247's contact/braking/handling contract rather than creating another vehicle dynamics owner.
3. Build a rights-aware data manifest for a small, public, independently reusable reference dataset.
4. Define a deterministic synthetic fixture corpus with mutation and out-of-domain controls.
5. Compare candidate powertrain/platform choices from real local prices and workshop access.
6. Publish the first Pareto report with all missing data and uncertainty exposed.
7. Advance from simulation only when hardware-level evidence and independent review justify it.

## References

- [Symtropy whole-life transport optimization #1601](https://github.com/Luminous-Dynamics/symtropy/issues/1601).
- [Symtropy physical load qualification #1636](https://github.com/Luminous-Dynamics/symtropy/issues/1636).
- [Symthaea road/off-road vehicle profile #6247](https://github.com/Luminous-Dynamics/symthaea/issues/6247).
- [SAE J1555: vehicle damageability and repairability](https://saemobilus.sae.org/standards/j1555_201910-recommended-practice-optimizing-automobile-damageability-repairability).
- [NHTSA automated driving safety reports and testable-scenario work](https://www.nhtsa.gov/automated-vehicles-safety/published-reports-and-documents).
- [ISO 21448:2022 SOTIF](https://www.iso.org/standard/77490.html).
