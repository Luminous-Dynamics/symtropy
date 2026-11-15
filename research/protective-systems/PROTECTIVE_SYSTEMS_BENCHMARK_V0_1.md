# Protective Systems Benchmark v0.1

**Programme:** PROTECTIVE-SYSTEMS-001  
**Repository:** Luminous-Dynamics/symtropy  
**Status:** research protocol proposal; synthetic-first  
**Version:** 0.1  
**Frozen date:** 2026-11-14  
**Claim ceiling:** software contract and benchmark design only; no physical product, safety, certification, or performance claim.

## 1. Objective

This benchmark asks whether an evidence-gated design workflow can find and qualify better protective-system trade-offs across three distinct application domains:

1. Human-worn protection and PPE.
2. Vehicle occupant protection.
3. Protective housing and habitat envelopes.

The benchmark is not a universal armour design or a new multiphysics solver. It is the decision and evidence boundary joining existing domain owners. It should expose when a proposed design is promising, when the evidence is sufficient for a specific claim, when the source is out of scope, and when the system must abstain.

The governing hypothesis is that protection should be optimized as a **complete-system and lifecycle outcome**, not as a single material property or a single score.

## 2. Non-negotiable invariants

- A candidate is not a test result.
- A simulation is not a physical measurement.
- A coupon result is not an article-level qualification.
- A standard name without edition, amendments and scope is not a complete standards reference.
- A valid signature or digest proves record integrity, not physical truth.
- An unknown, stale, revoked, mismatched or out-of-domain input is never a PASS.
- No scalar benefit may compensate for a failed life-safety, egress, occupant-health, crash-partner or certification hard gate.
- A proposed physical experiment is not authorization to buy, synthesize, expose, destroy, install or operate anything.
- All three domains retain separate safety endpoints and scorecards.

## 3. Composed architecture

| Owner | Responsibility in this programme | Existing work to consume |
|---|---|---|
| Symthaea | Candidate reasoning, engineering demand, multi-fidelity discovery, acquisition strategy and calibrated uncertainty | MAT-014 #4345; MAT-ENG #6185; MAT-PSPP #6224; MAT-UQ #6225; MAT-016 #4991 |
| Symtropy | Exact configuration replay, domain simulation adapters, scenario stress, model applicability and Pareto comparison | VECTOR-HABITAT #4095; SPATIAL-RESILIENCE #1848; AERO-INSPECTION-OBSERVABILITY #4165; GLASS-SILICON #2715 |
| Mycelix | Product/configuration identity, lot/process lineage, evidence references, lifecycle, repair and disclosure controls | CONSTRUCTION-TEXTILE-EVIDENCE #6097; CIRC-MFG #3095; TEXTILE-DPP-PROJECTION #6092 |
| Sol-Atlas | Resident/site context, local components and suppliers, climate exposure, accessibility, buildability and safe substitution | HOME-MATERIAL-INTELLIGENCE #811; CITY-HEAT-RETROFIT-ATLAS #593; CITY-MORPH-SAFE #591 |

Do not fork canonical types merely to make a new adapter convenient. Before adding any type or data store, demonstrate the exact semantic loss that existing owners cannot represent.

## 4. Evidence record and claim boundary

Every scored result must be traceable to an exact subject and configuration. At minimum the crosswalk must preserve:

- subject/product/system identity and configuration revision;
- material/article/assembly identity and applicable lot or batch references;
- geometry and interface revision where relevant;
- declared hazard, exposure and environmental conditioning;
- test or model method, version, standard edition, addenda and applicability;
- specimen or complete-article scope;
- raw observation / report references and calibration identity;
- result, units, uncertainty and analysis revision;
- laboratory / reviewer identity, competency and scope where applicable;
- model version, input digest, solver settings and domain-of-validity statement for simulated results;
- repair, aging, inspection and service history;
- source rights, disclosure class, currentness and revocation/supersession state.

Evidence classes must remain explicit: synthetic, literature-reported, supplier-declared, model-predicted, calculated, physically measured, independently tested, professionally reviewed, field-observed and clinically observed. These classes are not a universal linear ladder; their sufficiency depends on the proposition being claimed.

## 5. Application-specific scorecards

### 5.1 Human-worn protection

Required scorecard dimensions:

- exact standard-specific protection result and configuration scope;
- coverage and mass, reported separately;
- range of motion, mobility and task completion;
- heat/moisture burden, ventilation and wear duration;
- fit across relevant body sizes and movement states;
- conditioning, cleaning, aging, seam/closure and repair effects;
- user acceptance, maintainability and lifecycle cost.

For ballistic torso protection, use the current NIJ 0101.07 text and applicable addenda together with NIJ 0123.00 and its applicable addenda. Do not infer protection for limbs, accessories, animals or other geometries from a torso test. Stab resistance, thermal PPE and other hazards require their own applicable test methods; their outcomes must not be combined into a generic protection rating.

A focused human-worn torso-ballistic research child is now tracked in [BODY-ARMOR-001 #5377](https://github.com/Luminous-Dynamics/symtropy/issues/5377). Its protocol, versioned standards crosswalk and separate synthetic B0 fixture corpus are linked here:
- [Body armor research protocol v0.1](./human-worn/BODY_ARMOR_RESEARCH_V0_1.md)
- [NIJ / South African standards and solver crosswalk (2026-11-15)](./human-worn/BODY_ARMOR_STANDARDS_CROSSWALK_2026_11_15.json)
- [Synthetic body armor evidence-boundary cases](./fixtures/body-armor-cases-v0.1.json)

This child composes the existing materials, wearer/system, product-lineage and evidence owners. It does not create another material ontology, physics solver, certification authority or physical test programme.

### 5.2 Vehicle occupant protection

Required scorecard dimensions:

- occupant injury measures under the selected protocol;
- occupant-compartment intrusion and survival-space preservation;
- restraint interaction and occupant kinematics;
- crash pulse and energy-management behaviour;
- vehicle mass, centre-of-gravity, handling and braking consequences;
- compatibility with crash partners and vulnerable road users;
- post-event fire/electrical hazards, door function, egress and rescue access;
- inspection, repairability, downtime and lifecycle cost.

Any added protective mass or local reinforcement must be evaluated on the **complete vehicle configuration**. Do not assume that a stiffer local structure or added mass improves real crash outcomes.

### 5.3 Housing and habitat envelopes

Required scorecard dimensions:

- structural/wind performance and load-path continuity;
- fire/smoke behaviour, compartmentation and safe egress;
- water ingress, drainage, moisture and mould-risk conditions;
- overheating and thermal-comfort hours;
- ventilation and indoor-air-quality outcomes;
- vector exclusion where locally relevant and supported by species-specific evidence;
- essential water/energy service continuity under outages;
- accessibility, affordability, repairability and resident acceptance.

Use the existing Symtropy VECTOR-HABITAT and SPATIAL-RESILIENCE work. In South Africa, site context should use the CSIR GreenBook and local authority/SABS applicability evidence; climate context does not substitute for product or assembly certification.

## 6. Standards applicability register

The benchmark stores each standard as a versioned reference with: identifier, edition, amendments/addenda, publication/currentness date, jurisdiction, product scope, test method, intended proposition and applicability decision.

| Domain | Reference snapshot checked 2026-11-14 | Interpretation guardrail |
|---|---|---|
| Body armour | NIJ 0101.07 (three published addenda); NIJ 0123.00 (two published addenda); NIJ 0115.00 for stab resistance | Do not use the base document without addenda. NIJ 0101.07 torso geometry is not a qualification for limb/accessory or animal products. |
| Thermal PPE | ISO 13506-1:2024; NFPA 1970:2025; NFPA 1850:2026 | Complete-garment thermal tests have stated exposure limits. NFPA 1970 consolidates firefighter ensemble/equipment requirements; NFPA 1850 addresses selection, care and maintenance. |
| Composite coupons | ASTM D7136/D7136M-25 plus the relevant compression-after-impact method | Coupon performance depends on specimen, layup, impactor, energy and boundary conditions; do not scale it directly to a full article. |
| Vehicle | NHTSA NCAP procedures/data; IIHS 2026 test protocols and rating guidance | Compare like vehicle classes and protocol versions. Verify the applicable approval basis for the sales jurisdiction; NCAP/IIHS ratings are not equivalent to a complete modification approval. |
| Storm shelters | ICC 500-2023; FEMA P-361 fifth edition (November 2024) | Shelter-specific requirements; not a generic building certificate. Include egress, ventilation, sanitation, lighting and maintenance requirements. |
| Building fire | ISO 11925-2:2026; ASTM E84-26a; applicable local fire test/classification methods | ISO 11925-2 is a small-flame ignitability test. ASTM E84 is a comparative surface-burning test. Neither alone establishes complete-assembly fire resistance. |
| Indoor environment | ASHRAE 55-2023; ASHRAE 62.2-2025 | Thermal comfort and residential ventilation are separate required outcomes; do not trade ventilation away to improve another envelope metric. |
| South Africa | Applicable SANS 10400 parts, referenced SANS product standards and local-authority rules | Public references identify SANS 10400-T:2024 Edition 5 as a fire-protection baseline. Confirm the active SABS catalogue, amendments and local approval basis before any compliance claim. Use CSIR GreenBook as climate-risk context, not certification. |

Primary source URLs:

- NIJ 0101.07: https://nij.ojp.gov/topics/equipment-and-technology/ballistic-resistance-body-armor-nij-standard-010107
- NIJ 0123.00: https://nij.ojp.gov/topics/equipment-and-technology/specification-nij-ballistic-protection-levels-and-associated-test-threats-nij-standard-012300
- NIJ compliance programme: https://nij.ojp.gov/topics/equipment-and-technology/body-armor/performance-standards-and-compliance-testing
- ISO 13506-1:2024: https://www.iso.org/standard/75863.html
- NFPA 1970: https://www.nfpa.org/codes-and-standards/nfpa-1970-standard-development/1970
- NFPA 1850: https://www.nfpa.org/codes-and-standards/nfpa-1850-standard-development/1850
- NFPA 1850 TIA 26-4: https://docinfofiles.nfpa.org/files/AboutTheCodes/1850/TIA_1850_26_4.pdf
- ASTM D7136/D7136M: https://store.astm.org/d7136_d7136m-25.html
- IIHS protocols: https://www.iihs.org/ratings/about-our-tests/test-protocols-and-technical-information
- NHTSA NCAP: https://www.nhtsa.gov/ratings/resources-related-nhtsas-new-car-assessment-program
- ICC 500-2023: https://shop.iccsafe.org/ansi-icc-500-2023-icc-nssa-standard-for-the-design-and-construction-of-storm-shelters.html
- FEMA P-361 fifth edition: https://www.fema.gov/sites/default/files/documents/fema_p-361_safe-rooms-for-tornadoes-and-hurricanes_122024.pdf
- ISO 11925-2:2026: https://www.iso.org/standard/89109.html
- ASTM E84-26a: https://store.astm.org/standards/e84/
- ASHRAE 55: https://www.ashrae.org/technical-resources/bookstore/standard-55-thermal-environmental-conditions-for-human-occupancy
- ASHRAE 62.2-2025 scope: https://www.ashrae.org/technical-resources/standards-and-guidelines/titles-purposes-and-scopes
- ASHRAE standards scope: https://www.ashrae.org/technical-resources/standards-and-guidelines/titles-purposes-and-scopes
- CSIR GreenBook: https://greenbook.co.za/

## 7. Benchmark ladder

### B0 — record integrity and semantic qualification

Software-only first tranche. A valid result requires exact identity, scope, units, version, provenance and evidence-state semantics.

Hard gates:
- reject wrong product revision, stale standard, revoked evidence and out-of-scope test methods;
- reject dimensionally invalid or unitless physical values;
- reject missing test report, calibration, conditioning or uncertainty where required;
- reject any supplier/model/signed claim silently promoted to a physical measurement;
- reject unknown, missing or not-applicable values represented as zero or PASS;
- accept valid positive controls, so reject-all cannot pass;
- reproduce identical outputs for identical frozen inputs and tool versions.

### B1 — source reproduction and model credibility

- Reproduce public/source baselines only within their documented boundary and uncertainty.
- Distinguish verification, validation, calibration, sensitivity, applicability and OOD detection.
- Hold out complete material lots, product revisions, test campaigns, vehicle families, sites or climate periods where dependencies make random row splits unsafe.
- Report residuals and interval coverage by relevant subgroup.
- Abstain when the candidate, measurement, solver or environment is outside the qualified domain.

### B2 — matched-budget decision quality

Compare at least:
1. conventional expert/rule-based selection;
2. simple deterministic search, grid or DOE where tractable;
3. NSGA-II or another suitable Pareto optimizer;
4. matched-budget Bayesian optimization / uncertainty sampling;
5. Symthaea diversity-aware, uncertainty-aware and hypothesis-discriminating acquisition.

All policies receive the same frozen candidate space, objectives, constraints, training data, tool access, wall-time/compute/measurement budget and confirmation budget. Charge failed runs, retries, candidate expansion and high-fidelity confirmation. Keep confirmation cases sealed until policy freeze.

Report:
- feasible-front quality, feasibility and hard-constraint false-PASS;
- exact-oracle regret on small finite cases;
- hypervolume with a declared reference point, IGD+ and epsilon where valid;
- candidate rank reversals after high-fidelity evaluation;
- cost per accepted feasible candidate and cost-to-decision;
- solver/evaluation calls, wall time and energy/compute cost;
- worst-stratum and OOD performance, not only aggregate means.

### B3 — independent physical qualification

Only after B0–B2 and competent professional review:
- exact complete article/assembly configuration is tested at a lab competent and accredited for the method and scope;
- pre-register specimen, conditioning, replication, uncertainty and analysis;
- use blinded replicates and inter-laboratory round robins where justified;
- keep coupon, component, subsystem, complete-product, field and clinical evidence separate;
- a repository CI PASS, synthetic benchmark PASS or simulation PASS never substitutes for a physical test, professional sign-off or regulator decision.

## 8. Pareto objectives and industry comparison

Do not produce a single weighted protection score. First enforce domain-specific safety floors. Then compare the remaining feasible candidates on declared Pareto axes.

| Domain | Protection axis | Burden / counter-objectives |
|---|---|---|
| Human-worn | Standard-specific test result, coverage, durability | Mass, heat/moisture, mobility/task burden, repair, cost |
| Vehicle | Occupant injury, intrusion, survival space, post-event egress | Mass, handling, braking, crash-partner risk, repair cost |
| Housing | Structural/fire/flood performance, safe indoor hours and essential service continuity | Installed/lifecycle cost, energy/water use, maintenance, embodied impacts, accessibility |

An industry-superiority claim requires:
- a named, current, relevant industry baseline with a source;
- equivalent function, threat/exposure, product class, geography and test method;
- identical configuration and boundary accounting;
- independent evaluation, uncertainty intervals and adequate replication;
- no regression against any hard safety floor;
- full lifecycle, manufacturing and service costs, not material cost alone.

A result that beats an optimizer on synthetic data is a software decision-quality result, not better physical protection. A result that beats one commercial product is not proof of industry-wide superiority.

## 9. First experimental tranche

Proceed without waiting for hosted CI or hardware by freezing this research artefact:

1. Standards/applicability registry with currentness states: CURRENT, DRAFT, SUPERSEDED, OUT_OF_SCOPE, UNKNOWN.
2. Synthetic known-answer corpus covering all domains, with valid positives and adversarial negatives.
3. Crosswalk to existing material/process/UQ, simulation/applicability, product/lot/lifecycle and resident/site/component owners.
4. Matched-baseline protocol, holdout design and metric definitions.
5. A future physical demonstrator chosen only after a domain expert confirms testability, safety controls and a competent independent test route.

The first physical choice should be selected by value-of-information and safety/accessibility, not by visual appeal or by assuming the most ambitious application is best.

## 10. Acceptance criteria for v0.1

- Protocol covers all three domains without merging their safety endpoints.
- Every listed standard has edition/currentness/scope fields and source links.
- The synthetic corpus includes at least one positive and one negative control per application track.
- Every negative case identifies the exact reason for reject/abstain.
- No synthetic fixture is represented as physical evidence.
- No new canonical ontology or solver is created.
- A reviewer can replay and understand why each case is accepted, rejected or held for review.

## 11. Known limitations

This version does not contain proprietary full test methods, ballistic test procedures, a physical design recipe, a certified material selection, an accredited test report, or complete local regulatory advice. Public standard abstracts are used to identify applicability boundaries; implementation must consult the authoritative standard text and competent professionals. Currentness is a snapshot and must be rechecked before use.
