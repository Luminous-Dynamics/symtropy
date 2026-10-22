# Household Appliance Engineering Substrate v0.1

**Status:** research proposal and synthetic contract only  
**Issue:** [APPLIANCE-QUAL-001](https://github.com/Luminous-Dynamics/symtropy/issues/1805)  
**Physical status:** no appliance prototype or measured improvement is claimed

## 1. Objective

Make household appliances easier to maintain, reproduce, repair, upgrade and improve by designing the product, repair process, parts ecosystem, measurements and evidence model together.

The first target is a guarded, low-voltage DC fan powered by an appropriate external certified power supply. It is a deliberately narrow test vehicle for modularity, diagnostics, energy/performance measurement, repeatable manufacturing and repair workflows. It is not evidence that the same design or safety assumptions apply to ovens, washing machines, refrigeration, mains power electronics or other higher-risk appliances.

Do not start another universal appliance standard before reviewing existing efforts such as the Open Arsenal Specs [Universal Appliance Standard](https://github.com/openarsenalspecs/Civic-Tech/blob/main/Universal%20Appliance%20Standard.md). Our first contribution should be a reproducible engineering/evidence profile that composes with existing interfaces and standards rather than competing with them.

## 2. Paradigm shifts

### 2.1 Product price -> cost of sustained capability

A product succeeds when it delivers its useful function over time, including after faults—not merely when it leaves the factory. Compare acquisition, energy, consumables, planned maintenance, successful and unsuccessful repair attempts, waiting for parts, rework, downtime and end-of-life recovery. State the use profile, price date, location/tariff, discount rate and missing inputs. Never map an unknown cost to zero.

### 2.2 Modularity -> measured repair advantage

A module is not justified because it looks clean in CAD. A boundary must demonstrate lower repair effort, less discarded material, less downtime, a safer isolation/verification path, or worthwhile upgrade value without creating disproportionate cost, parasitic load, connector failure or performance loss. Retain a non-modular control where practical.

### 2.3 Repair -> verified capability restoration

Keep these facts distinct:

- failure/degradation observed;
- diagnostic hypothesis proposed;
- work/procedure performed;
- component planned versus actually consumed or replaced;
- post-repair functional verification;
- residual defect/limitation;
- recurrence during a defined follow-up window.

A closed work order is not proof of restored function. A successful one-time test is not proof of recurrence-free operation.

### 2.4 Open files -> independent rebuildability

Publish design files in their preferred editable source formats—not only exported manufacturing outputs—including CAD, drawings, schematics, PCB sources if any, firmware sources, build instructions, test fixtures, BOM, compatible-part specifications, service procedures, tool requirements and licence/provenance records. Mark which files are custom and which are third-party. Respect third-party licence terms and do not copy other projects' implementation without permission.

### 2.5 AI -> calibrated advisory assistance

Symthaea may eventually rank diagnostic hypotheses, explain evidence and identify the next measurement with highest expected information value. Begin with rules and synthetic known-fault tests before using a learned or cognitive model. Report per-class results, uncertainty, unknown/abstention rate and false reassurance. No AI output may bypass hardware protection, grant actuation authority or replace a physical verification step.

### 2.6 One factory -> qualified local variants

Permit repair shops and local workshops to fabricate permitted parts or use alternative suppliers. A compatible connector or matching nominal dimension does not qualify a substitute by itself. Each substitution must bind the exact part/revision, relevant electrical/mechanical/thermal ratings, compatibility checks, installation context and post-installation test.

### 2.7 Cloud intelligence -> local sovereignty

The fan's essential function must not depend on an account, network or cloud service. Diagnostics and service documentation should work offline. Connected features remain optional; the initial prototype has no remote actuation.

## 3. System responsibilities

| System | Responsibility | Explicit non-claim |
|---|---|---|
| **Symtropy** | Design-space exploration, engineering/task models, multi-objective trade-offs, lifecycle economics and repeatable benchmark protocols. Reuse the repair-task graph work in [#1773](https://github.com/Luminous-Dynamics/symtropy/issues/1773) and its draft parent [PR #1762](https://github.com/Luminous-Dynamics/symtropy/pull/1762) after the contract converges. | A synthetic or simulated result is not physical performance, product safety or legal conformity. |
| **Symthaea** | Optional local, advisory analysis of sensor/repair evidence; candidate fault ranking and next-test suggestions with uncertainty. | Prediction is not observation, a fault score is not a safe-to-operate verdict, and a model does not control safety-critical hardware. |
| **Mycelix** | Reuse existing manufacturing/repair/provenance owners for design revision, BOM, manufactured unit, work order, replacement-part history and qualification evidence. See [RepairProductiveLoopV1 #3313](https://github.com/Luminous-Dynamics/mycelix/issues/3313), [validator #3318](https://github.com/Luminous-Dynamics/mycelix/issues/3318), [physical-product evidence #3084](https://github.com/Luminous-Dynamics/mycelix/issues/3084) and [hardware release profile #2392](https://github.com/Luminous-Dynamics/mycelix/issues/2392). | A record, signature or release manifest does not prove the physical specimen was built, tested or found safe. |
| **Appliance hardware** | Perform its essential function with bounded, deterministic protection, local controls and documented safe isolation. | The AI, DHT, internet and user interface are not safety interlocks. |

Do not implement a second generic repair database, work-order lifecycle, artifact-provenance store, universal optimizer or safety authority model.

## 4. Reference product and first test questions

**Reference:** guarded low-voltage DC fan, external certified supply, replaceable controller and harness, serviceable motor/bearing assembly where feasible, accessible fasteners, replaceable feet/housing and clear airflow path.

Before design freeze, answer:

1. Which component failures can be detected without falsely treating a command as proof of physical behavior?
2. Which repair boundaries actually decrease task time and total restore cost?
3. How much airflow, sound, power and vibration variation arises across units and test setups?
4. Which interfaces permit sourcing substitutions while preserving fit, rating and verification?
5. Does modularity add connector/standby losses or new failure points?
6. Can a second builder reproduce the design and test the same functional requirements?
7. What remains unrepairable due to sealed components, unavailable parts, skills, tools, calibration or unsafe procedures?

Freeze the test setup before comparing results. Record appliance/model revision, unit ID, configuration, test fixture, instrument model/calibration status, environment, procedure revision, raw readings, uncertainty, failed attempts and exclusions.

## 5. Benchmark protocol

All numeric thresholds below are **prospective targets or hypotheses**, not observed results.

### 5.1 Openness/rebuildability

- Every custom artifact needed for permitted reproduction and modification is available in editable source form with an explicit licence.
- Every BOM line has a stable identifier and specification or is explicitly `unknown`.
- Build at least three candidate units; seek at least two independent builders besides the author where resources permit.
- Bind each result to the exact design revision, BOM digest, component variants, procedure and unit identity.
- Compare repeated measurements using preregistered tolerances based on instrument uncertainty and observed unit variation.

### 5.2 Repair time and outcomes

For each task, separately record setup/isolation, diagnosis, hands-on work, parts/tool wait, verification, rework, and total out-of-service time. Preserve unsuccessful and unresolved attempts in the denominator. Report median, P90, sample size, uncertainty and exclusions by task and repairer.

Pilot with at least 3 qualified repairers × 3 repetitions per task where practical. After instrument/task definitions stabilize, preregister a confirmatory design—initial planning default 5 repairers × 6 repetitions per candidate/control/task cell, revised only by a documented power/feasibility analysis.

Hypothesis: reduce **both median and P90 total restore time by at least 25%** against a frozen commercial control on a preregistered common-fault set, without worse post-repair function, safety exceptions or rework. Do not claim a P90 improvement from a tiny pilot.

### 5.3 Function, energy and noise

For a fan, collect airflow, power draw, sound pressure, vibration, speed setting, ambient conditions and measurement uncertainty. Compare energy per useful delivered airflow at matched airflow and a preregistered sound tolerance.

Initial hypothesis: at matched airflow and within the frozen sound tolerance, at least 10% lower energy per delivered airflow unit than the selected commercial control. If energy, sound, airflow, price or durability conflict, publish the Pareto trade-off instead of hiding it inside one scalar score.

### 5.4 Repair economics and parts availability

For each seeded fault, compare repair and replacement costs, including actual labour, parts, consumables, tools, test/calibration, shipping, waiting, rework and discarded components. Initial target: successful, verified repair costs less than 50% of replacement cost in at least 80% of eligible seeded cases. Report all ineligible cases and why.

Aim to publish a 10-year availability commitment for declared wear/replaceable modules after final production, including how compatible substitutions and interface revisions will be qualified. This is a voluntary design target for the reference appliance, not a claim that the same legal obligation applies to every product category.

### 5.5 Durability and service life

Before a limited field pilot, target a documented 1,000-hour operating screen and a separately specified start/stop-cycle test, recording every fault and intervention. These screens cannot establish a 10-year lifetime. Long-life claims require an explicit accelerated-test model or longitudinal field evidence and validation of the model's applicability.

### 5.6 Diagnostics

Begin with synthetic seeded faults and an independent oracle. Later use consented, instrumented physical cases, keeping training/tuning cases separated from held-out evaluation.

If Symthaea is introduced, report top-1/top-3 diagnostic accuracy, class coverage, per-class recall, calibration, abstentions, false reassurance, latency and failure under missing/stale/conflicting sensor data. Candidate target: ≥90% top-3 recall on the declared held-out seeded-fault set. It remains advisory: zero critical fault may be called safe solely on model confidence.

### 5.7 Materials and lifecycle

Publish total mass and known material classes by replaceable assembly, repair-induced waste, material recovery route, energy and consumables. Embodied carbon, recyclability and lifecycle cost must remain unknown/bounded when supplier/material/process data are missing. Avoid using a single weighted score to conceal trade-offs.

## 6. Minimum semantic invariants

The accompanying synthetic corpus is a **contract test plan**, not a data set of physical measurements. It must retain these boundaries:

- command ≠ physical function;
- diagnosis ≠ observed cause;
- simulation ≠ physical evidence;
- work-order completion ≠ capability restoration;
- replacement part planned ≠ part consumed;
- compatible form factor ≠ qualified substitute;
- a single nominal reading ≠ recurrence-free repair;
- one unit ≠ production-batch qualification;
- document/signature/release manifest ≠ safe product;
- unknown ≠ zero, failure-free or free of cost;
- repair succeeded once ≠ long-term service life proven;
- diagnostic confidence ≠ operational authority.

The original failure and all rejected/unsuccessful attempts remain preserved, even after a later successful repair.

## 7. Research inputs and data limits

- The [Open Repair Alliance dataset](https://openrepair.org/open-data/downloads/) was updated in October 2026, includes 416,284 electrical/electronic and 203,591 non-electrical/electronic records, uses ORDS v0.3, and is licensed CC BY-SA 4.0. The [published overview](https://openrepair.org/announcements/over-half-a-million-records-of-open-data-on-repair/) reports 64% event repair success overall. Community-event observations are selected samples; they are not the population's appliance repair probability or an unbiased field-failure distribution. Use them for taxonomy/failure/barrier discovery, retain source and license, and document selection/missingness bias.
- [ORDS v0.3](https://openrepair.org/open-data/open-standard/) describes product, repair and session fields for aggregation. Do not assume its current fields encode every engineering task, component revision, duration or proof of post-repair function needed here.
- [OSHWA's definition](https://oshwa.org/definition/) requires preferred editable design-source formats and clear documentation scope.
- [IEC 60335-2-80:2024](https://webstore.iec.ch/en/publication/70375) specifies particular safety requirements for household and similar electric fans; the published scope includes DC-supplied and battery-operated fans. The low-voltage DC prototype and external certified supply narrow some hazards but **do not waive finished-product safety review or establish conformity**. Confirm the applicable IEC 60335-1 edition, national adoption, intended use, and test programme with qualified product-safety specialists before a physical pilot or sale.
- [EU Right to Repair Directive 2024/1799](https://eur-lex.europa.eu/eli/dir/2024/1799/oj/eng) applies from 31 July 2026 through relevant national implementation; its repair duty is linked to product-specific repairability rules. [EU washing-machine ecodesign requirements](https://energy-efficient-products.ec.europa.eu/product-list/washing-machines_en) provide a category-specific spare-parts reference, not blanket rules for fans.

Third-party datasets and standards must keep their licence, attribution, version, retrieval date, limitations and permitted-use requirements. Avoid embedding the full third-party repair dataset in source-controlled fixtures; use synthetic fixtures for deterministic tests and rights-cleared aggregates for exploratory analysis.

## 8. Implementation gates

- **G0 — contract:** review existing schema owners, freeze test definitions, define the safety boundary and source/licence manifest.
- **G1 — synthetic corpus:** validate schema, unique case IDs, positive controls and negative controls; deterministic outputs from identical inputs; fail closed on malformed and unknown states.
- **G2 — baseline:** measure three retail control units with one published protocol, calibrated instruments and uncertainty.
- **G3 — prototype:** publish editable CAD/firmware, BOM, assembly/service instructions, test fixture and hazard review; keep essential operation local.
- **G4 — replication:** at least three candidate units, independent builders, preregistered repair trials and performance replication.
- **G5 — field pilot:** consented operation/repair data, bounded recurrence windows and real parts lead times. Expand to higher-risk appliances only after category-specific design and safety reviews.

At every gate, evidence is exact-head and exact-configuration. Queued, pending, skipped, cancelled, missing, synthetic-only or stale-head checks are not PASS. Results from a simulation cannot be promoted to physical, manufacturing or safety qualification.

## 9. First benchmark corpus

`schemas/appliances/benchmark_scenario_corpus_v0_1.schema.json` defines the corpus envelope. `fixtures/household_appliance/benchmark_scenarios_v0_1.json` provides positive controls plus adversarial cases for openness, repairability, function, sourcing, diagnostics, provenance, durability, security and external repair-data interpretation.

The first executable audit should be small and deterministic: reject duplicate IDs, unknown disposition values, missing assertions, malformed schema, unsafe claim promotion and all registered negative controls. Keep the validator's claim ceiling at synthetic-contract integrity until the physical gates above are independently completed.
