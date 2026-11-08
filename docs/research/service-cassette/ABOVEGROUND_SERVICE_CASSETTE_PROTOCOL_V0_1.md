# Above-ground service cassette benchmark protocol v0.1

**Issue:** [ABOVEGROUND-SERVICE-CASSETTE-001 #4181](https://github.com/Luminous-Dynamics/symtropy/issues/4181)  
**Classification:** engineering research protocol; synthetic-first; physical work separately gated.  
**Scope:** above-ground building-services modules for homes, schools, clinics, workshops and other facilities. No underground construction.

## 1. Research hypothesis

A service zone deliberately designed as an accessible, replaceable, factory-tested assembly may reduce site coordination, installation burden, fault isolation time and service interruption compared with a site-built baseline.

That hypothesis is conditional. Prefabrication may also increase interfaces, transport, lifting, common-mode failure, early design lock-in, supplier concentration and replacement complexity. The experiment must detect both effects.

This protocol does not certify MEP products or buildings. It defines a fair experiment for a low-energy mock article and a future owner-receipt path.

## 2. Reference article and strict physical boundary

The v0.1 article is a non-habitable, non-clinical mock services spine. Use segregated circuits:

- low-voltage DC demonstrator only;
- isolated data/network cabling and a test endpoint;
- contained non-potable, low-pressure water loop;
- low-energy ventilation fan and removable filter;
- instrumentation, low-voltage controller/telemetry and metrology ports;
- mechanical support frame with removable access covers and labelled service interfaces.

No connection to occupied-building mains electricity, potable water, gas, refrigerant, fire/life-safety circuits, medical equipment, pressurized production systems or public utility service is part of v0.1. No person may use the mock result as evidence of building-code compliance.

The physical risk assessment, electrical protection, water containment, guarding, work instructions, emergency arrangements and approval to conduct a lab test are prerequisites for E3; the benchmark does not supply or waive them.

## 3. Functional units and comparison boundary

Use owner-native accepted-output semantics from Symtropy #3605.

| ID | Unit | Acceptance boundary |
|---|---|---|
| FU-CASSETTE-01 | One cassette | Exact revision built, uniquely identified and passing its complete factory acceptance protocol |
| FU-INSTALLED-01 | One installed cassette | Factory-accepted article plus accepted mechanical, water, low-voltage and data interfaces to the exact mock-facility revision |
| FU-SERVICE-01 | One mock-facility service-year | Declared low-energy water-loop, DC, data and ventilation duties, reported separately by service |

Do not combine the individual service outputs into one synthetic “service quality” score. Do not equate an accepted cassette with an accepted installation or continuously available service.

The comparison boundary includes design preparation, coordination, procurement/kitting, manufacture, factory test, packaging, handling, transport, receiving, installation, connection, inspection, commissioning, fault recovery, maintenance, replacement and end-of-life handling to the extent actually measured. Every exclusion must be written down.

## 4. Routes and fair comparison

- **S0 conventional site-built:** conventional procurement and installation, with the same accepted function and test criteria.
- **S1 prefabricated spine:** one accessible, factory-assembled lineal service frame.
- **S2 distributed local modules:** separate local cassettes close to service endpoints.
- **S3 hybrid riser and local branches:** centralized primary distribution with replaceable local branches.
- **S4 maintain-existing:** preserve a verified existing mock configuration and replace only a declared defective component.

All routes use the same frozen facility scope, requirements, test instruments, environmental conditions, demand profile, design horizon and independent acceptance predicates. Differences in equipment and staffing are allowed but must be disclosed and costed/measured.

A route must not win by omitting a test, reducing service function, shifting work outside the measurement boundary, excluding rejected output, or assuming better source evidence.

## 5. Interface and assembly contract

Each module revision specifies, without inventing technical ratings:

- stable cassette, component, port and configuration identities;
- datum/reference frame and geometric envelope;
- interface type, units, direction, mating condition and tolerance source;
- service-zone segmentation and segregation assumptions;
- maintenance access and replacement clearances;
- isolation points, safe state and responsible lockout authority;
- connection sequence, inspection access and required test points;
- service demand and permitted operating envelope;
- material/product identity, lot and supplier references;
- installation, commissioning, maintenance and recommissioning responsibilities.

Unknown specialist properties stay Unknown. A schematic, BIM object or product catalogue field is not physical evidence.

## 5A. Tolerance architecture is a first-class design input

A prefab MEP module is only useful if its factory geometry, building as-built geometry, installation sequence and interfaces fit within an explicitly engineered tolerance budget. Do not assume that a nominal BIM coordinate is an as-built datum.

Bind a tolerance chain for each interface:
- design datum and design revision;
- host structure / support-frame as-built observation, instrument and calibration;
- module manufacturing measurement and calibration;
- support, hanger, opening and access geometry;
- component and interface manufacturing/install tolerance;
- declared buffer/adjustment mechanism and allowed range;
- assembly sequence and the point at which each adjustment can still be made;
- final measured fit and independent acceptance;
- remaining service clearance and maintainability after adjustment.

Record the named tolerance method (e.g. worst-case stack, statistical method or an owner-approved hybrid), its assumptions, source revision and responsible engineering owner. Do not mix statistical distributions with guaranteed physical envelopes without an explicit rule. A buffer may absorb only the variation it is designed and qualified to absorb. No force-fit, field cutting, slot extension, drilling, shimming or other deviation may be silently accepted; each needs its own approved disposition and evidence.

A useful 2026 comparator is the five-project MEP prefabrication study by Li et al. It identifies communication, detailed design, quality control and buffer assembly as prescriptive tolerance-management principles, and reports that none of the cases applied every principle. This supports testing tolerance closure as part of the workflow, not simply geometric accuracy at the factory: https://doi.org/10.1080/01446193.2026.2626735.

Singapore BCA describes prefab MEP as off-site integrated subassemblies and reports project-specific time savings up to 60%, while highlighting the need to identify suitable components and coordinate stakeholders. Treat this as an industry example, not as a performance target transferable to our mock article: https://www1.bca.gov.sg/growth-and-transformation/productivity/design-for-manufacturing-and-assembly-dfma/prefabricated-mechanical-electrical-and-plumbing-mep-system/.

## 6. Test sequence

### Factory

1. Verify released design/BOM/configuration identity.
2. Verify physical component and lot identity against the released requirements.
3. Inspect frame geometry and interface positions with a calibrated measurement system.
4. Test only the contained mock water loop, low-voltage DC, data link and fan/filter functions specified for the article.
5. Record all first-pass failures, corrections, replacements and retests.
6. Emit a factory acceptance record from the owner responsible for that measurement.

### Receiving and installation

1. Confirm delivered identity, custody and condition.
2. Verify the building/mock-facility revision is still compatible with the cassette revision.
3. Verify site datum, access envelope and service-zone separation.
4. Record lifting/handling and installation labour by role.
5. Inspect mechanical support and each connection separately.
6. Record every missing, mismatched, damaged or unresolved interface.

### Integrated commissioning

1. Test water, DC, data and ventilation independently.
2. Test their declared combined operating sequence, without assuming that independent component tests prove system integration.
3. Verify alarms, isolation and the declared safe/degraded state in the mock context.
4. Record commissioning failures and corrective work.
5. Only the commissioning owner may issue the commissioning outcome.

### Fault, repair and recommissioning

1. Inject only predeclared benign faults within the approved laboratory procedure.
2. Measure time to detect, isolate, diagnose, access and replace the affected component.
3. Verify that unaffected circuits remain in their declared state; otherwise record the common-mode interruption.
4. Re-test the replacement component and all interfaces affected by the repair.
5. Record final commissioning/recommissioning and service restoration separately.

## 7. Metrics and denominators

Report every metric independently.

**Accepted production**
- accepted cassettes / scheduled hour;
- accepted cassettes / total workflow labour-hour;
- first-pass and final acceptance rates;
- p50/p90 factory cycle and total lead time;
- defect/rework/scrap by interface and reason.

**Total human work**
- design coordination, supplier/kitting, setup, manufacture, inspection, packaging, receiving, handling, installation, commissioning, supervision, maintenance and recovery;
- hours by role and task;
- training time, interruptions, manual recovery and handoffs;
- ergonomic/manual-handling exposure where an approved measurement protocol exists.

**Installation / service**
- installation and commissioning duration;
- interface mismatch frequency;
- diagnosis, isolation, replacement and recommissioning time;
- fault-induced interruption per individual service;
- common-mode service interruption and unaffected service share.

**Lifecycle / supply**
- parts and connector variety;
- qualified spare availability and lead time;
- transport/packaging burden;
- energy, water, materials, scrap and waste per accepted functional unit;
- actual maintenance and replacement evidence; no invented service-life extension.

**Human factors**
- training, usability, perceived control, role clarity and acceptance;
- objective exposure removed versus new exposure introduced;
- human oversight and intervention burden;
- self-report remains supplementary and is not safety certification.

The ICI five pillars (standardization, prefabrication, modularization, automation, digitalization) are reported separately as maturity descriptors. They are not substituted for outcome metrics.

## 8. Synthetic conformance cases

The 24 frozen cases in #4181 cover factory/installed/commissioned distinction, stale calibration, wet/dry segregation, revision mismatch, custody/revocation, wrong spare, inaccessible filter replacement, common-mode riser failures, total workflow labour and service interruption.

The independent evaluator must recompute conformance from the fixture inputs. It must not trust a supplied outcome label. Synthetic evidence must never be promoted to measured or qualified physical evidence.

## 9. Baselines and research comparison

Use:
- Symtropy #3605 for task-specific accepted-output, full workflow time/labour, first-pass/final acceptance and defect/rework semantics;
- NIST's task-based assembly performance framework for relevant subskill test design (perception, mobility, dexterity and safety), without pretending a small-parts test board proves building-services installation;
- ICI for industrialization maturity crosswalk, not performance ranking;
- Symtropy #3773 panel cell as a related factory workflow comparator, not a same-functional-unit control;
- Symthaea #6040 for exact asset/design/as-built/commissioning digital-thread integration;
- Mycelix #4814/#5149 for supply dependencies, material/lot identity and custody;
- Sol-Atlas #602/#603/#615 for product-family coverage, lifecycle chain and district integration.

Current research warns against overclaiming construction HRC maturity: a 2026 systematic review analysed 375 construction-robotics studies and found operator-led workflows common and planning/maintenance/demolition relatively underexplored; a separate review of 163 construction HRC studies emphasizes cognitive, affective, physical and usability dimensions; NIOSH's 7 October 2026 bulletin highlights both exposure reduction and new interaction hazards. Sources: https://doi.org/10.1016/j.rineng.2026.110406 ; https://doi.org/10.1016/j.jsr.2026.08.009 ; https://www.cdc.gov/niosh/bulletin/2026/safety-in-construction-robotics.html.

## 10. Evidence ladder and proposed targets

- **E1 synthetic software:** 100% known-answer agreement; zero false promotion across factory/installed/commissioned/service states; zero wrong-revision or stale-calibration acceptance.
- **E2 method reproduction:** reproduce applicable test methods and publish deviations.
- **E3 lab mock article:** calibrated instruments; preregistered repeated trials; independent acceptance; all failures and rework retained.
- **E4 facility pilot:** only after professional design, statutory review, site-specific risk assessment and accountable commissioning.
- **E5 sustained replication:** maintenance, replacement, uptime and repeated deployment measured over a declared period.

Do not invent a target productivity improvement before the S0 baseline has trustworthy measurements. Proposed promotion requires a measured improvement in a predeclared primary outcome without violating any safety, technical, quality, service-continuity, workforce or lifecycle guardrail.

## 11. Owner boundaries

- Symtropy evaluates physical/resource/workcell scenario consequences and accepted output.
- Symthaea owns design/capability meaning, building realization, operational-service projection and digital-thread lineage.
- Mycelix owns supplier/inventory/lot/custody/approval facts.
- Sol-Atlas owns read-only spatial and district projection.
- Qualified engineering, testing, safety, regulatory and commissioning authorities own their respective decisions.

No benchmark PASS activates machinery, authorizes a building connection, issues a regulatory approval or proves real city service readiness.
