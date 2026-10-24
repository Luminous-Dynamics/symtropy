# Autonomous Space Industrial Cell: Reference Mission and Verification Plan v0.1

**Status:** Research architecture and qualification proposal; not a flight design, operational authorization, or claim of a self-sufficient factory.  
**Date:** 2026-10-14  
**Parent workstream:** Space Bootstrap and Industrial Closure Runtime (Symtropy PR #1543).

## Executive decision

Yes: robots can build useful robots, factories, and structures using in-situ resources, but only through a staged industrialization programme. We should not begin by promising a self-replicating factory. Start with imported, maintainable equipment that can prospect, move regolith, establish power and communications, and build low-risk infrastructure. Add resource processing only after local resources are measured. Add parts manufacturing only after feedstock quality, process yield, energy demand, inspection, and repair are demonstrated. Promote a factory to “locally reproducible” only when its critical dependencies and evidence are explicit.

**Recommended first target:** a robotic lunar-south-polar operations cell producing and maintaining defined civil-infrastructure products—graded routes, berms, pads, shielding blocks, cable/rail supports, and selected noncritical brackets—before attempting pressure vessels, high-performance electronics, precision actuators, or a factory capable of reproducing itself.

**Governing principle:** automate the work system, not just individual robots. Power, thermal control, navigation, communications, maintenance, inventory, tooling, process quality, and recovery must be modelled as one resource-bounded system.

This document extends the existing [Symtropy space-bootstrap and industrial-closure work](https://github.com/Luminous-Dynamics/symtropy/pull/1543), [Symthaea candidate-to-qualified robotics loop](https://github.com/Luminous-Dynamics/symthaea/pull/7423), and [Sol Atlas evidence-first lunar observatory](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/89).

## 1. What external evidence does—and does not—show

NASA's current lunar-surface technology programme covers regolith excavation and site preparation, autonomous structural assembly, power/communications integration, dust mitigation, and integrated surface systems. NASA's public descriptions of IPEx are not fully consistent on the headline throughput: one passage on its lunar-surface technology page describes 10 metric tons delivered across 100 meters in 11 days, while another describes on the order of 20 metric tons during a mission. Treat these as design/programme figures whose scope and revision must be reconciled—not demonstrated lunar throughput or a value our simulator may assume. Our source record should preserve the exact cited passage, access date, scenario and metric definition before adopting a benchmark.

NASA's ARMADAS programme has demonstrated autonomous assembly of metre-scale structures in Earth gravity from repeatable modular elements. Its architectural lesson is valuable: design the structural modules and interfaces so robots can be simpler than the structure, then scale through repeated, inspectable operations. This does not establish that a lunar habitat or industrial plant is flight-qualified.

NASA's current ISRU portfolio includes molten-regolith electrolysis work that completed vacuum testing with oxygen extracted from regolith simulant and a metal-rich slag by-product. NASA also describes Blue Alchemist as an integrated development approach using regolith simulants to demonstrate production paths for oxygen, iron, aluminium wire and silicon solar-cell feedstock. These are meaningful subsystem milestones, not proof that an unattended lunar factory can meet long-duration production, reliability, maintenance, or economic targets.

NASA's ISAM history offers a caution: OSAM-2 concluded in 2023 before its planned flight demonstration, and NASA proceeded toward shutdown/cancellation of OSAM-1 after technical, cost, schedule and partnership challenges. Space robotics and manufacturing have real promise, but programme integration and mission fit matter as much as a successful isolated mechanism.

Primary references:

- NASA, [Lunar Surface Technology](https://www.nasa.gov/lunar-surface-technology/) — excavation, construction, IPEx, regolith processing.
- NASA, [ARMADAS](https://www.nasa.gov/automated-reconfigurable-mission-adaptive-digital-assembly-systems-armadas/) — robotically assembled modular structures.
- NASA Technical Reports Server, [ARMADAS lunar infrastructure concept](https://ntrs.nasa.gov/citations/20230001316).
- NASA TechPort, [Molten Regolith Electrolysis Tech Maturation](https://techport.nasa.gov/projects/116413).
- NASA, [ISRU overview](https://www.nasa.gov/overview-in-situ-resource-utilization/).
- NASA, [OSAM-1 status](https://www.nasa.gov/mission/on-orbit-servicing-assembly-and-manufacturing-1/) and [OSAM-2 programme history](https://www.nasa.gov/mission/on-orbit-servicing-assembly-and-manufacturing-2-osam-2/).
- NASA, [Moon to Mars architecture components](https://www.nasa.gov/moontomarsarchitecture-components/).
- NASA, [September 2026 lunar-surface technology call](https://www.nasa.gov/news-release/nasa-calls-for-proposals-to-accelerate-lunar-surface-technologies/).

**Evidence boundary:** these sources support a staged technology roadmap. They do not establish economically closed factories, years of unattended end-to-end operation, full local electronics production, or readiness to construct crew-critical structures from untreated regolith.

## 2. What to build first

Do not start with a humanoid workforce or general-purpose robot bodies. Most early space-industrial tasks are specialized and repetitive. Favor simple, repairable machines with common power, data, cargo and tool interfaces.

| Role | Early function | Design priorities | Do not assume |
|---|---|---|---|
| Survey/scout | Terrain, boulder, resource, illumination, thermal and route mapping | Sensor calibration, localization, uncertainty, health telemetry, safe recovery | Remote sensing proves a mineable reserve |
| Cargo/haulage | Move containers, feedstock, spares and tools | Standard cargo interface, traction margin, low centre of mass, tow/recovery points | Nominal mobility proves access to all terrain |
| Excavator/regolith handler | Dig, collect, screen and place bulk material | Dust-tolerant joints, wear monitoring, force limits, replaceable cutting elements | Earth-gravity tests translate directly to lunar performance |
| Manipulator/assembler | Mate modules, handle cables, exchange tools, assemble frames | Force/torque sensing, compliant insertion, replaceable end effectors, metrology | A commanded grasp proves secure capture |
| Processing cell | Screen/beneficiate, heat, reduce or sinter a chosen feedstock | Contained feed paths, mass-flow metering, power/thermal instrumentation, maintainable seals | Bulk extraction rate equals product yield |
| Inspection/maintenance | Survey wear, connectors, joints, leakage, geometry and repair | Repeatable inspection, calibrated gauges, tool changers, fault isolation | “Looks correct” is dimensional or structural QA |

A small fleet may initially share a few chassis across roles. Optimize for useful capability per imported mass, energy, maintenance hour and critical dependency—not robot count.

## 3. Industrialization ladder

Progress through independently testable stages. Each stage supplies capabilities needed by the next.

### Stage A — Survive, observe and communicate

Deliver a node with thermal-safe modes, power monitoring, local storage, time/frame consistency, terrain data, local navigation, Earth/relay link management, and an emergency safe state. Establish how it survives lost communications and recovers from interrupted tasks.

**Exit evidence:** time-stamped sensor-quality records; known landing/terrain envelope; measured duty cycle; successful fault-injected lost-link recovery; no unsafe authority escalation when perception becomes stale.

### Stage B — Move bulk material before refining it

Demonstrate repeated excavation, loading, hauling, dumping and grading. First products should be routes, berms, equipment pads or shielding mass where validated material properties suffice. Bulk construction can offer an earlier local-mass benefit than advanced metallurgy because it may not require purification to aerospace-alloy standards.

Measure feed and delivered mass, distance, slope, excavation resistance, energy, wear, dust-related failures, blocked-route recovery, and accepted placement tolerance. Report the distribution across trials, not just the best run.

**Exit evidence:** repeated cycles across a frozen terrain corpus; stuck/recovery cases; measured mass/energy balance; a quality criterion for the resulting terrain or berm; independent inspection.

### Stage C — Build with local bulk material

Start with non-pressure-bearing, replaceable and inspectable products: sintered/paved coupons, shielding blocks, surface anchors, cable/rail supports, landing-area surface treatments, or simple structural modules. Select a product only after its load path, environment and material acceptance requirements are defined.

For each product specify feedstock class/preparation, process settings, geometry/tolerance, porosity and contamination limits, mechanical tests, thermal-cycle and vacuum exposure, dust/radiation applicability, inspection plan, rejection/rework rule and acceptable failure consequence.

**Exit evidence:** multiple batches meet predefined acceptance criteria, including worst-case process conditions. A rendered structure or one successful coupon is not qualification.

### Stage D — Process a measured resource into a usable stream

A valid ISRU chain is not “mine → resource”. It is:

~~~text
survey and estimate
  -> in-situ confirmation
  -> excavate / collect
  -> screen and characterize
  -> prepare feedstock
  -> extract / reduce / sinter
  -> separate co-products and waste
  -> purify where required
  -> assay and certify stock
  -> store and track
~~~

On the Moon, near-term candidates include regolith-based oxygen/metal processing and local bulk construction. Water-ice extraction remains site-specific and gated by local measurements of concentration, depth, temperature, recoverability and energy burden. A remotely detected hydrogen signature is not proof of accessible water inventory.

For each run, record dry feed mass, volatile assumptions, composition, product mass by stream, waste, contaminants, power, energy, peak power, process duration, heat rejected, consumables, tool wear, uncertainty and sample/assay evidence. Maintain strict mass balance and report unreconciled residuals.

Core metrics:

- accepted product kg per kWh and per operating hour;
- accepted product kg per kg of plant mass;
- feedstock rejection and usable yield by stream;
- availability, mean repair time and consumable use;
- product quality and independent assay agreement;
- uncertainty in resource reserve, extraction yield and equipment lifetime.

### Stage E — Repair and manufacture selected parts

Prioritize parts that are useful, low-consequence if imperfect, easy to inspect, and recurring in imported-spares inventories: guards, brackets, cable trays, noncritical housings, fixtures, jigs, and selected mechanical pieces after process qualification.

The first factory need not reproduce every robot. It must reliably reproduce a useful subset of parts, repair specific failure modes, and demonstrate reduced resupply risk after accounting for feedstock processing, energy, rejects, quality assurance and maintenance.

Continue importing or keeping certified inventory for components whose manufacture is not qualified: high-performance motors, bearings, lubricants, precision gears, seals, sensors, electronics, semiconductor devices, power electronics, and crew-critical life-support components. Locally printed geometry is not automatically a qualified replacement.

**Exit evidence:** a provenance-bound work order from design revision to manufactured part, dimensional/material inspection, fit/function test, installation record and observed reliability. Track the failures that can truly be repaired from on-site inventory; do not infer reproducibility from part material alone.

### Stage F — Factory-of-factories, only after evidence

A factory may claim local reproduction of a tooling class only when it also produces or has accounted for the tooling, materials, calibration, controls, maintenance parts and quality evidence required for that claim. If one imported controller, bearing, sensor, seal, calibration instrument or consumable can halt production, that dependency remains explicit.

Do not reduce progress to one “percent local” score. A small imported component can block the whole operation. Report both local mass fraction and weighted critical-dependency closure using the parent bootstrap runtime.

## 4. Site strategy: the Moon first for a demonstration, not as the final answer

Separate three lunar-south-polar site functions:

1. **Operations/power site:** terrain suitable for landing and setup, with characterized illumination and communications.
2. **Resource prospecting zone:** reachable candidate resources after hazard/route analysis, with sample/assay evidence and uncertainty.
3. **Processing/construction zone:** selected for power, thermal rejection, material handling, regolith properties, safe standoff from landers/crew assets, and waste/by-product handling.

These need not be one location. Do not place the first operations base deep inside a permanently shadowed region just because water ice may be nearby. Prefer an operations node with supportable power/comms plus robotic routes toward higher-risk resource areas.

Sol Atlas's [Lunar Site Observatory PR #89](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/89) is the right presentation foundation for keeping landing suitability, operations support, energy, communications, science access and resource suitability separate. Each map layer must carry source, coordinate reference, resolution, time window, uncertainty and load state. Resource measurements must not be silently promoted from remote estimate to inventory.

For Mars, do not port lunar process recipes unchanged. Mars offers atmospheric CO₂ as a feedstock for oxygen production; NASA’s MOXIE demonstration produced 122 g of oxygen over its mission, reaching up to 12 g/h at 98% purity or better. That is a valuable in-situ chemistry demonstration—not an industrial plant throughput, duty-cycle, storage or propellant-economy result. A Mars process tree must include atmospheric intake/compression, contaminant handling, high-temperature electrochemical processing, heat rejection, product purity, storage and seasonal/environmental operating envelopes. Lunar molten-regolith electrolysis uses a different feedstock and process chain, with oxygen and metal-rich outputs from regolith; the two should share inventory/evidence interfaces, not the same process recipe. Reuse software/evidence contracts but keep environment parameters, process models, qualification corpora and uncertainty separate. See [NASA’s MOXIE mission results](https://www.nasa.gov/missions/mars-2020-perseverance/perseverance-rover/nasas-oxygen-generating-experiment-moxie-completes-mars-mission/) and [NASA’s MRE technology record](https://techport.nasa.gov/projects/116413). For LEO, prioritize assembly and servicing of structures from imported or previously qualified feedstock; surface ISRU is not the right operational analogue.

## 5. Safe automation architecture

### Layer 1 — Physical plant and independent protection

Hardware-level limits govern torque, current, thermal state, speed, braking, pressure, power isolation and process containment. Each robot and process cell needs a physically enforceable safe state and an emergency-stop path appropriate to its hazards. Higher-level planners cannot override these limits.

### Layer 2 — Local robot autonomy

Each robot completes bounded local skills (dock, grasp, lift, drive, excavate, align, fasten, inspect, park, self-test), detects skill failure and enters a conservative state without waiting for Earth. Each skill declares preconditions, operating envelope, expected result, maximum duration/energy and recovery action.

### Layer 3 — Workcell orchestration

A scheduler matches work to robot/tool capability, resource and energy budgets, material quality, location, safety zones, tooling, maintenance needs and recovery cost. The planner proposes schedules; a separate checker verifies hard constraints and budgets before execution.

### Layer 4 — Plant digital thread

Each work order binds design revision, recipe version, source batch, machine/tool identities, calibration state, control-software digest, environment assumptions, resource/energy reservations and inspection results. Part provenance continues through storage, installation, repair and recycling.

### Layer 5 — Fleet/site coordination

Coordinate traffic, shared infrastructure, energy peaks, tools, maintenance windows, charging/thermal survival, hazard zones and relays. The fleet operates locally during link outages; remote updates and new recipes wait for authenticated, versioned acceptance.

### Layer 6 — Long-horizon planning

Symthaea proposes repair plans, schedules, fault diagnoses, resource-allocation options and process improvements with uncertainty and abstention when sensors/world models are inadequate. The independent runtime checks constraints; Mycelix records evidence and authorization; the physical control system executes only within approved limits.

Do not put every action behind a network vote or grant a cognitive model unrestricted actuation authority. Time-critical collision, power and thermal protections must be local. Governance can authorize policies and resource priorities but cannot replace real-time control.

## 6. Cross-project responsibilities

### Symtropy — physical simulation and industrial closure

Use [space bootstrap and industrial closure PR #1543](https://github.com/Luminous-Dynamics/symtropy/pull/1543) for dependency graphs, resource/energy ledgers, process/co-product accounting, faulted factory populations and deterministic replay. Use rigid-body/contact simulation for robot and construction tasks only within a validated scope. Treat terrain, thermal, regolith and process-physics models as distinct modules with explicit fidelity limits.

Keep consciousness/Φ or other agent-state metrics separate from physical constitutive laws and hard safety constraints unless an independently validated physical coupling exists.

### Symthaea — planning, estimation and anomaly detection

Use the [candidate-to-qualified robotics loop](https://github.com/Luminous-Dynamics/symthaea/pull/7423) to stage immutable robot candidates, run frozen scenario corpora, measure recovery and verify evidence manifests. Extend it to construction/manipulation after task models and reference oracles exist. Benchmark factory scheduling against deterministic heuristics and a simple optimization baseline. Use time-ordered holdouts and adversarial faults; do not rank robot designs from undertrained controllers or one favorable run.

### Mycelix — coordination, work records and evidence

Use typed records for resource observations, material batches, calibration, work orders, energy reservations, capability claims, maintenance/failures, inspection certificates, authorization, handoffs and bootstrap milestones. A received record is not automatically true: evidence class and independent qualification remain explicit. Distributed ledgers coordinate and preserve provenance; they do not run physical equations or issue hard-real-time actuation.

### Sol Atlas — site and plant observability

Use the [lunar site observatory](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/89) to show site layers, machine/factory locations, work-order state, energy/resource flows, route hazards and evidence. Distinguish live sensor values, source reports, simulation outputs, inferred estimates and stale/unavailable data. A green UI state means only the named check passed, not general safety or physical correctness.

### Cross-stack record minimum

Every work unit should bind:

- stable mission, robot, workcell and work-order identifiers;
- source commit and software/toolchain identities;
- coordinate frame, epoch/time scale and units where relevant;
- exact input/scenario digests, model version, settings and random seeds;
- source batch and evidence/uncertainty class;
- reserved and actual energy, mass, duration, consumables and waste/by-products;
- expected/measured output, quality tolerances, failure/rework state;
- independent verifier identity and bounded claim supported by the evidence.

Do not substitute one opaque confidence or “truth” score for these fields.

## 7. End-to-end reference simulation campaign

Keep the first campaign small enough to understand and rich enough to falsify the industrial story.

**Reference product:** choose one noncritical product whose geometry, feedstock and acceptance tests are specified—e.g. a berm segment, sintered block, cable/rail support or mounting bracket. Start with one product; every additional product multiplies unvalidated process assumptions.

**Reference chain:**

1. Sol Atlas provides a frozen site/terrain package with source identity and uncertainty.
2. A scout agent records observations and labels unobserved areas as unknown.
3. The planner identifies a low-risk excavation patch and reachable route.
4. The scheduler checks robot capability, tool state, traffic zone, energy reserve and thermal survival margin.
5. Excavation/handling emits a named feedstock batch with measured mass and material evidence.
6. A versioned process recipe emits product, co-products and waste as separate ledger events.
7. An independent checker evaluates mass/energy balance and product-quality predicates.
8. An inspector verifies geometry and applicable mechanical/environmental tests.
9. Accepted product may be installed; rejected product remains rejected, quarantined or scheduled for rework.
10. A fault/maintenance event is recorded and recovery or degraded mode is demonstrated.
11. Mycelix receives the typed evidence envelope; Sol Atlas renders it read-only.

**Mandatory fault cases:** lost communications; power reduction/peak conflict; stale or conflicting terrain/resource observations; blocked route or failed tool; tool wear; contamination or composition mismatch; under-yield/off-spec product; failed inspection and rework; loss of a critical factory/robot role; duplicate/missing/reordered/replayed inventory events; overcommitted energy reservation; damaged sensor/non-finite actuator observation; failed heat rejection; lost remote link during a non-immediately-interruptible process; malformed frame/unit; unknown recipe version.

Freeze scenario IDs and independent expected outcomes before planner runs. Compare the same corpus against a deterministic baseline scheduler and the Symthaea-assisted planner. A planner cannot pass by skipping failures, shrinking the corpus or treating “not evaluated” as success.

## 8. Qualification gates

| Gate | Required evidence | Does not authorize |
|---|---|---|
| G0 — Contract | Typed inputs, units/frames, provenance and valid dependency graph; missing data fails closed | Physical work |
| G1 — Safe local operation | Lost-link, sensor-fault, power-loss and stop/recovery tests in simulation and hardware | Unsupervised production |
| G2 — Bulk handling | Repeated excavation/haul/place trials, mass/energy records, bounded wear and recovery | Mining a proven reserve |
| G3 — Qualified local product | Frozen design/recipe, repeatable accepted batches, independent inspection and environment tests | Crew-critical structures |
| G4 — Measured ISRU process | Source/feedstock evidence, assays, product/waste mass balance, power/thermal records and uncertainty | Resource-economy claims |
| G5 — Repair capability | On-site repair of a real failure mode, measured repair time and post-repair qualification | Full machine independence |
| G6 — Local manufacturing | Tooling, calibration, controls and critical dependencies declared and tested | Factory independence |
| G7 — Resilience | Loss of robot, process cell, relay or power source causes bounded degradation and recovery | Industrial expansion |
| G8 — Reproduction claim | Second instance assembled/produced from accounted inputs, independent QA and no hidden dependency substitution | General self-replication |

Promotion is per capability, environment, product class and operating envelope. A qualified berm does not qualify a pressure vessel; a factory that reproduces a bracket has not qualified its electronics. Simulation, Earth-gravity tests, relevant-environment tests and operational evidence remain distinct evidence classes.

## 9. Metrics that tell us whether automation works

Report values with denominators and uncertainty; avoid one headline “autonomy percentage”.

- **Industrial performance:** accepted product per operating hour; kg accepted output/kWh; kg accepted output/kg imported plant; energy per repaired failure; tooling wear; yield/reject/rework fraction; downtime; maintenance hours per 100 operating hours; recovery time by failure class.
- **Robustness:** successes/failures by scenario stratum; confidence bounds where sample size warrants; worst cases; recovery time; unsafe commands blocked; reservation conflicts; maximum energy deficit; minimum survival-power/thermal margin.
- **Closure:** locally satisfiable critical dependencies weighted by consequence; mass closure; essential consumables and control-system dependencies; lost capabilities when a machine/factory class fails; recovery horizon and spare coverage. High local-mass fraction must not hide a single imported component that stops all production.
- **Product quality:** dimensional tolerance, material properties, defect rate, destructive-test results, calibration provenance and accepted batch count. Inspection must be independent of the process controller when failure consequences warrant it.
- **Evidence quality:** exact source/build/input identity, full sample/scenario counts, independent-check identity, replay result and every exclusion. Blocked, NotRun, Rejected, partial or stale results remain visible.

## 10. Economics: count the whole loop

A local process is not automatically cheaper than importing a finished part. Compare alternatives within a lifecycle boundary including initial plant/deployment mass; power generation/storage/conditioning and heat rejection; site preparation, mobility, shielding and dust mitigation; resource characterization, beneficiation, purification and rejects; tooling, calibration and consumables; inspection, failure and rework; maintenance robots and their dependencies; logistics, delay, downtime, end-of-life and recycling.

Evaluate value as risk-adjusted mission capability gained per imported mass, available power and recovery time—not only tonnes extracted or fraction of mass sourced locally. Include uncertainty and a baseline that imports the part. Stop or redesign local production when plant mass/energy, rejects, maintenance or quality burden exceeds avoided resupply burden for the decision horizon.

## 11. First implementation package

1. **Typed Rust mission manifest:** environment, geometry/frame/epoch, robot roles, product spec, scenario corpus, resource evidence, budgets, recipe identity, acceptance policy and required oracles. Reject unknown fields, missing evidence, invalid units and non-finite numbers.
2. **Product/process ledger:** material batch, transformation, co-products, waste, energy, peak power, heat, quality, tool state and failure/rework events. Do not mint usable inventory until output and mass/energy accounting pass the selected gate.
3. **Scheduler baseline:** deterministic priority/constraint scheduler with readable scorecard and explicit failed constraints. Compare learned/active-inference proposals against it.
4. **Faulted reference corpus:** frozen scenario IDs, seeds, negative controls, expected invariants and manifest digest; include every mandatory fault class.
5. **Independent checker:** recompute mass/energy balance, resource-reservation availability, product identity, quality predicates and event lineage rather than trusting solver summaries.
6. **Sol Atlas view:** read-only site/scenario/work-order/model/recipe records, accepted/rejected output, uncertainty and exact qualification state.
7. **Measured hardware upgrade:** after simulation and ground tests, select a vacuum/dust analog demonstration with instrumentation for energy, wear, output quality and repair time. Lunar deployment is a later independent gate.

Use the existing Symtropy industrial-closure track and reuse other projects' contracts where they fit. Do not create duplicate evidence registries or competing receipt schemas without an explicit gap analysis.

## 12. Stop conditions and anti-patterns

Reject claims that:
- call excavation throughput “usable production” without product quality and waste accounting;
- declare industrial independence because most mass is local;
- assume all regolith is chemically or mechanically interchangeable;
- promote remote resource estimates directly into stock inventory;
- rely on continuous ground contact for every robot decision;
- allow a planner/model to override local actuator, power, thermal or emergency constraints;
- define a general self-replication boolean;
- use model confidence, Φ, similarity or an aggregate score as a safety certificate;
- remove failed/missing tests from qualification denominators;
- replace a qualified component solely because a local part fits;
- render unqualified structures as if their structural, material and thermal gates had passed;
- describe deterministic replay or a code test as flight qualification.

## Final recommendation

The highest-value path is **autonomous civil infrastructure first, resource extraction second, useful local manufacturing third, factory reproduction last**. Use modular robots and standardized interfaces; build with local bulk material only where product requirements permit it; keep critical parts imported until local production is independently qualified; and maintain explicit power, mass, thermal, maintenance and evidence ledgers throughout.

This is a programme we can simulate and progressively validate. It is not yet a claim that lunar or Martian industry can build every part of itself.


## Runnable synthetic reference example

The companion Rust example is the file at:

https://github.com/Luminous-Dynamics/symtropy/blob/feat/space-bootstrap-industrial-closure/crates/domains/symtropy-bootstrap/examples/industrial_cell_reference.rs

Run it from the Symtropy workspace with:

~~~~sh
cargo run -p symtropy-bootstrap --example industrial_cell_reference
~~~~

It composes existing kernel primitives into a small, deterministic end-to-end reference:

- a dependency graph where high local mass closure (850,000 ppm) coexists with unresolved imported metrology/control electronics and only 800,000 ppm critical closure;
- one synthetic multi-output process that must conserve all feed mass across products and waste;
- negative controls that must reject a 1 g mass imbalance and an underfunded energy request;
- a causal energy-ledger consumption event with a conservation replay;
- a Pareto frontier that excludes a dominated import-heavy option without collapsing all trade-offs into one score.

All example quantities are synthetic fixture units, **not measured lunar material yields, kWh, factory performance or physical qualification evidence**. The reference validates bounded software contracts only; even a passing run is not a robot, process or physical qualification result. The paragraph above reflects the state when the example was first added; later exact-head execution history is tracked on PR #1543.

## Fault-injected ledger controls — 2026-10-14

The executable reference now includes three explicit fault cases in addition to the process-mass and energy-budget controls:

1. **Duplicate energy event identity:** a second event reusing an accepted event ID must be rejected with no state or history mutation.
2. **Non-monotonic energy sequence:** a stale sequence with a fresh event ID must be rejected with no state or history mutation.
3. **Inventory overdraw:** a consumption event larger than available synthetic inventory must be rejected without changing the balance or appending history.

Each control tests the failure path and atomicity, not just the returned error. The fixture uses in-memory synthetic quantities. Lost communications, sensor faults, route obstruction, tool wear, material contamination and hardware stop/recovery remain campaign-spec requirements, not yet executed robot-simulation scenarios. Do not count them as completed coverage until a frozen corpus and independent expected-outcome verifier exist.

The three current software fault cases are listed in the frozen [industrial-cell fault corpus](industrial-cell-fault-corpus-v0.1.json). The focused workflow invokes `scripts/verify-industrial-cell-reference.sh`, which pins the corpus SHA-256 and separately checks the reference transcript for each exact scenario marker and the expected scenario count. This verifier checks the bounded executable contract/output; it is not an independent physics model or hardware safety assessor. Broader physical faults remain explicitly not covered in this corpus.

## Standards crosswalk

The existing G0–G8 gates now have a [standards crosswalk](AUTONOMOUS_SPACE_INDUSTRIAL_CELL_STANDARDS_CROSSWALK_V0_1.md) covering requirement/verification matrices, software assurance and independent verification, metrology/calibration, space-system verification/testing, robotics-cell safety analogs, machinery risk assessment, and technology-readiness terminology. It records official status as checked on 2026-10-14 and distinguishes reference guidance from requirements formally adopted by a mission or project. The crosswalk does not declare Symtropy or a future robot compliant with any cited standard.

## Machine-readable software evidence receipt

After the focused workflow successfully executes the reference and verifies the frozen corpus, it emits and uploads `industrial-cell-reference-receipt.json` with a companion SHA-256 file. The receipt distinguishes the PR source head (`source_commit_sha`) from the exact checkout tree tested (`tested_checkout_commit_sha`) and event SHA (`event_commit_sha`), then binds the workflow run ID/URL and attempt, pinned Rust toolchain, corpus digest, verifier digest, reference-program digest, synthetic fixture class, and explicit claim ceiling. For pull requests, the checkout may be GitHub's synthetic merge commit; consumers must not confuse that with the PR source head. The workflow only reaches receipt generation after the format check, required transcript markers and exact three-scenario count pass, then validates required receipt fields before upload. A failed, canceled, queued, or NotRun job is not a receipt and must not be reinterpreted as success.

The receipt is software-contract evidence only. It does not authenticate external physical observations, prove a calibration certificate is current, validate a lunar/Mars process, or qualify robot hardware. A downstream `BootstrapMilestone` may reference it as a `WorkflowRun` or `TestReport` artifact after independently checking the artifact checksum, run conclusion, exact source commit and manifest digest.
