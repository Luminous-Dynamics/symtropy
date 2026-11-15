# Body Armor Research Protocol v0.1

**Programme:** BODY-ARMOR-001 (#5377), under PROTECTIVE-SYSTEMS-001 (#5246)  
**Repository:** Luminous-Dynamics/symtropy  
**Snapshot date:** 2026-11-15  
**Status:** research protocol and benchmark proposal; synthetic-first  
**Claim ceiling:** no armour design, ballistic performance, injury reduction, product readiness, certification or field-protection result is claimed.

## 1. Objective

Create a reproducible, evidence-gated research path for **human-worn torso protection** that treats armour as a complete protective wearable system rather than a material coupon or a single rating.

The core question is:

> Can a design process improve verified protection while reducing burden on the wearer, preserving fit and mobility, managing heat and moisture, remaining inspectable and maintainable, and keeping every claim attached to the exact tested configuration?

The first target is a benchmark protocol and evidence chain. It is not a ballistic prototype, an ammunition/weapon study, or an alternative certification programme.

## 2. Scope and non-goals

### In scope

- Human-worn torso ballistic protection as the initial standards-anchored profile.
- Exact configuration identity: carrier, panels/inserts, covers, seams, closures, size, fit, material lot, process history and condition.
- Wearer-centred outcomes: coverage, mass, bulk, fit, range of motion, task performance, thermal/moisture burden and equipment interaction.
- Durability, conditioning, inspection, damage, repair/replacement and evidence invalidation.
- Process–structure–property–performance research under existing Symthaea material evidence owners.
- System-level wearer geometry, movement and scenario modelling in Symtropy where the physics model is applicable.
- Evidence and lifecycle traceability through Mycelix.
- South African standards and manufacturability as a jurisdiction-specific profile, with current status verified before claims.

### Out of scope for v0.1

- Designing or optimizing for a named weapon, projectile, ammunition type or specific attack.
- Publishing recipe-level ballistic constructions, threat-specific construction parameters or instructions intended to defeat a particular weapon.
- Autonomous material synthesis, purchasing, live-fire testing, destructive testing or wearer exposure.
- Claiming compliance, certification or protection from simulation, synthetic data or AI-generated predictions.
- Extending NIJ torso qualification to limbs, helmets, shields, backpacks, blankets, animals or other articles.
- Building a new finite-element solver, material ontology, optimizer, evidence ledger or certification authority.

## 3. Paradigm shift: from material rating to wearer protection

A ballistic rating is necessary for a declared ballistic claim, but it is not a complete measure of the protective wearable system. The design and qualification object must include at least these linked layers:

1. **Threat/protection scope:** exact standard, edition, addenda, protection category and intended article.
2. **Article configuration:** panel/insert, carrier, cover, closures, seams, size, orientation and assembly.
3. **Material/process lineage:** precursor/lot, architecture, executed process and condition, plus the resulting structure state where evidenced.
4. **Test evidence:** laboratory scope, exact test method, specimen/article identity, conditioning, calibration and uncertainty.
5. **Wearer interaction:** fit, coverage during movement, mobility, load distribution, thermal burden and equipment interference.
6. **Service lifecycle:** inspection, aging, cleaning, damage, repair, replacement and requalification.

A high-performing coupon is not automatically a high-performing article. A passing ballistic test is not proof of comfort, inclusive fit, thermal usability, long-term durability, or performance after unqualified modification.

## 4. Current standards and scope

### 4.1 NIJ torso-ballistic reference

The initial internationally comparable profile should use the current NIJ framework as a **reproducible reference**, not as a claim that NIJ is the only legitimate national standard.

- **NIJ 0101.07** is the ballistic-resistance test standard for body armour intended to protect the human torso. NIJ's page dated 1 December 2025 states that three addenda have been published and must be incorporated.
- **NIJ 0123.00** defines ballistic protection levels and associated test threats. NIJ's page dated 8 July 2025 states that two addenda have been published and must be incorporated. It explicitly says 0123.00 does not itself define product performance requirements or the method for testing a product; it is used with product test standards such as 0101.07.
- NIJ uses separate standards for stab-resistant body armour. Do not merge stab and ballistic results into one generic rating.
- A threat-level label is not a test result. A test result is not a certificate unless the applicable compliance programme requirements are satisfied.

The new NIJ naming separates handgun and rifle protection categories (HG and RF families). The research database should store the exact standard identifiers and declared protection category, not translate legacy labels into a current claim without a versioned mapping.

Official sources:
- NIJ 0101.07: https://nij.ojp.gov/topics/equipment-and-technology/ballistic-resistance-body-armor-nij-standard-010107
- NIJ 0123.00: https://nij.ojp.gov/topics/equipment-and-technology/specification-nij-ballistic-protection-levels-and-associated-test-threats-nij-standard-012300
- NIJ body armour performance standards and compliance testing: https://nij.ojp.gov/topics/equipment-and-technology/body-armor/performance-standards-and-compliance-testing
- NIJ compliant products list: https://nij.ojp.gov/topics/equipment-and-technology/body-armor/ballistic-resistant-armor

### 4.2 NIJ compliance is more than a laboratory test

NIJ's current compliance page (published 14 August 2026) states that:
- the compliance programme covers law-enforcement body armour within its declared scope;
- tested products must meet workmanship and labelling requirements in addition to performance tests;
- manufacturers are subject to six follow-up inspections and tests over five years;
- NIJ compliance testing must occur at an NIJ-approved independent laboratory, with NIST NVLAP accreditation and the specified technical scope; the page states compliance testing is conducted within the United States.

Therefore:
- a laboratory report is not automatically NIJ certification;
- an ISO/IEC 17025 or SANAS accreditation does not, by itself, make a laboratory an NIJ-approved CTP laboratory;
- a product must match the exact model/configuration on the official list and the applicable follow-up programme;
- a changed carrier, panel, closure, cover, size, material lot or construction must not inherit an earlier claim without a documented applicability decision and any required requalification.

### 4.3 South African SANS 1658 status

The local standards record is intentionally conservative.

The South African Government Gazette, Notice 3339 of 2025 (27 June 2025), lists **SANS 1658:20XX Edition 3** as a draft standard for public comment, with a closing date of 27 August 2025. It describes the scope as minimum ballistic-resistance performance requirements for body armour intended to protect the wearer's torso and says it is limited to ballistic resistance.

A local testing provider currently advertises **SANS 1658:2025** on its service webpage. That is useful evidence of market practice, but a provider webpage is not the SABS catalogue or the full normative standard. The available public evidence is not sufficient to establish the exact current published SABS edition, amendments, adoption status or applicability on 15 November 2026.

**Crosswalk disposition: VERIFY_LOCALLY.** Before relying on SANS 1658 for any design, procurement, tender, product or compliance decision:
1. obtain the current SABS catalogue entry and exact normative document;
2. verify edition, amendments, corrigenda and withdrawal/supersession status;
3. confirm the exact article and protection scope;
4. verify the testing laboratory's current SANAS certificate and schedule for the exact method and scope;
5. verify any procurement, regulator or local-authority acceptance requirement.

Sources:
- Government Gazette Notice 3339 of 2025: https://www.gov.za/sites/default/files/gcis_document/202506/52914gen3339.pdf
- SABS standards catalogue: https://www.sabs.co.za/standards
- SANAS accredited facilities directory: https://www.sanas.co.za/Pages/website/accreditation/load/latest-accredited-facilities.aspx
- Local provider's stated test services (vendor claim; not normative authority): https://www.bpt.co.za/services/ballistic-testing/

The local provider's current webpage lists a 2025 edition and claims SANAS approval. Neither the advertised standard nor the provider's statement should be treated as independent confirmation of current accreditation scope. The exact SANAS schedule must be retrieved and checked.

### 4.4 Separate protection propositions

Ballistic resistance, stab resistance, slash resistance, fragmentation, thermal/fire protection and blunt-impact injury outcomes are different propositions. They require distinct standards, test methods, conditioning and evidence. Do not:
- infer stab resistance from ballistic compliance;
- infer slash performance from a stab test without checking the method's actual scope;
- infer broad medical injury reduction from a single backface-deformation or non-perforation outcome;
- infer protection for a non-torso geometry from NIJ's torso test;
- infer multi-hit or post-aging performance beyond what the applicable test and conditioning actually establish.

## 5. Research frontier and falsifiable hypotheses

### 5.1 Mechanism-aware, process-aware material design

A 30 May 2026 mechanistic review of aramid-fibre composites surveys unidirectional, 2D-woven, stitched and 3D-woven reinforcement; hybridization; coatings; matrix effects; and shear-thickening-fluid treatments. It emphasizes that different design choices activate different damage and energy-dissipation mechanisms, and that inappropriate hybridization can reduce ballistic performance rather than improve it.

A September 2026 systematic review of 3D-woven fabric composites identifies a particularly important research gap: it reports that no multiscale modelling framework has yet demonstrated broad, independently validated predictive capability across architectures and impact regimes. It also highlights insufficient dynamic interfacial fracture-toughness data and a near-absence of validated multi-hit models. Reported model accuracy can be calibration-dependent rather than genuinely predictive.

This suggests a better research strategy:
- treat process and architecture as first-class variables;
- preserve interface and damage-state uncertainty;
- distinguish calibrated fit from independent predictive validation;
- use held-out architectures, lots and impact regimes;
- treat multi-hit and lifecycle behavior as unresolved until independently demonstrated.

Candidate research families may include textile architecture, hybrid fibre systems, interface/matrix design, coatings, adaptive treatments and graded/segmented article layouts. They are hypotheses for literature screening and controlled comparison—not proven design recommendations. The first v0.1 deliverable is the evidence plan, not a construction recipe.

Sources:
- Rana et al. (30 May 2026), *Aramid Fibre Composites for Ballistic Impact Protection: A Mechanistic Review of Recent Advances*, https://doi.org/10.1007/s10443-026-10485-y
- Wei et al. (first published 19 September 2026), *Three-dimensional woven fabric composites under ballistic impact: A systematic review of multiscale modeling strategies from fiber to structural scale*, https://doi.org/10.1177/00219983261480233

### 5.2 Wearer ergonomics is a protection variable

A January 2026 *Applied Ergonomics* study of military body armour reports that female soldiers experienced more fit, pain and mobility restrictions than male soldiers; excessive armour system length affected both sexes, and front length was the strongest predictor in the study. This is not a universal population estimate, but it is a reason to measure fit and movement rather than assume one nominal geometry suits everyone.

The research programme should explicitly stratify fit and wearer outcomes by relevant anthropometry, body shape, posture, carried equipment and movement. It must not equate a size label with fit.

Source:
- *Soldiers experience fit issues, musculoskeletal pain, and equipment and mobility interference with military body armour*, Applied Ergonomics 130 (January 2026), https://doi.org/10.1016/j.apergo.2025.104645

### 5.3 Paradigm hypotheses

The programme should test, not assume, the following:

1. **Wearer-centred protection:** improvements in the complete protective task may matter more than maximizing a coupon metric.
2. **Architecture/process co-design:** fabric architecture, interfaces and process history may be as decision-relevant as nominal fibre selection.
3. **Injury-informed evaluation:** penetration resistance and blunt-trauma outcomes must remain distinct; medical interpretation requires competent evidence and review.
4. **Fit-aware geometry:** coverage must be measured during movement, not only on a static torso form.
5. **Damage-aware lifecycle:** service events should invalidate only the claims they affect, with unknown dependencies moving to review rather than silently remaining valid.
6. **Robustness under variation:** performance across lots, conditioning, fit and environmental variation matters more than a single best-case sample.
7. **Inspectable by design:** panels, seams, interfaces and carrier assemblies should be designed so degradation can be found and claims can be withdrawn safely.

## 6. Symthaea / Symtropy / Mycelix architecture

### Symthaea — discovery and engineering reasoning

Compose existing work:
- MAT-ENG-001 #6185: engineering demand ↔ materials discovery ↔ engineering qualification;
- MAT-PSPP-001 #6224: process → structure → property → performance;
- MAT-UQ-001 #6225: calibration, cross-fidelity discrepancy, OOD and uncertainty;
- MAT-014 #4345: active discovery and value of information;
- MAT-016 #4991: advisory-model/evidence-authority firewall.

Symthaea may generate candidate hypotheses, estimate uncertainty, propose discriminating computations and prioritize measurements. It must not promote a candidate, model prediction or optimization score into physical evidence. Physical tests and material-process operations remain human-authorized and must be tied to an exact protocol and facility.

### Symtropy — wearer/system dynamics and scenario coordination

Use the existing rigid-body engine for the physics it actually represents:
- wearer and equipment geometry;
- load distribution and movement scenarios;
- range-of-motion and task-interference proxies;
- coverage changes with posture and motion;
- balance, contact and environmental interaction;
- exact replay and scenario-level comparisons where the model is validated.

**Do not use Symtropy's current rigid-body core as a ballistic-impact or soft-composite solver.** Its documented scope excludes soft-body simulation. High-rate textile/laminate deformation, fibre/yarn response, delamination, fracture and projectile–material interaction require a separately qualified continuum/explicit-dynamics model or competent external laboratory measurements.

A candidate external solver for investigation is OpenCourant, the AGPL-3.0 community continuation of OpenRadioss, which publishes an explicit finite-element solver for crash, impact and highly nonlinear dynamic simulation. It is only a solver candidate: no ballistic armour validation is claimed here. Before integration, compare licensing, exact release and regression status, material law coverage, boundary-condition support, convergence, calibration and independent validation. Prefer an out-of-process adapter and preserve license boundaries; do not add it as a linked dependency to Symtropy's permissively licensed core.

Sources:
- Symtropy architecture and scope: https://github.com/Luminous-Dynamics/symtropy
- OpenCourant project and license: https://opencourant.org/
- OpenCourant repository: https://github.com/OpenCourant/OpenCourant

### Mycelix — exact product and service lineage

Compose existing Mycelix textile/product and manufacturing evidence work. Every result must bind to:
- product model, size and configuration revision;
- carrier, cover, panel and insert identities;
- material supplier, lot/batch and process history;
- test report, lab scope, standard edition and conditioning;
- inspection/damage/repair/replacement events;
- exact requalification evidence and current claim state.

A damage or unknown service event must not silently inherit a prior claim. Directly affected claims should be invalidated; unresolved dependency scope should move to review. Repair alone must not restore a protection claim.

## 7. Benchmark ladder and comparison to industry

### B0 — configuration and evidence semantics

Software-only; reuse the parent Protective Systems verifier.

Required adversarial cases:
- wrong product or configuration revision;
- missing carrier/panel/size identity;
- stale or superseded standard;
- NIJ 0101.07 without its applicable addenda;
- NIJ 0123.00 without its applicable addenda;
- threat specification misrepresented as product performance;
- torso result applied to non-torso geometry;
- supplier declaration or model prediction promoted to physical test;
- unknown lot, missing units, conditioning or calibration;
- damaged or repaired article with stale evidence;
- signature/hash accepted as proof of physical truth.

Proposed target: reject every seeded hard-gate violation and accept every valid positive control. Reject-all must fail. All outputs must be deterministic for frozen inputs.

### B1 — model verification and predictive credibility

For every material/impact model:
- verify solver implementation against analytic or published canonical cases where available;
- check mesh/time-step convergence and energy balance;
- expose constitutive, interface, failure and boundary-condition assumptions;
- distinguish calibration, validation, verification and applicability;
- report uncertainty and model discrepancy;
- hold out whole material lots, architectures, test campaigns and conditions;
- report OOD and abstention;
- disclose whether a result is calibrated to the same test series it is evaluated against.

The September 2026 systematic review is a direct warning against reporting calibrated fit as independent predictive accuracy.

### B2 — matched-budget design and discovery

Compare, on frozen candidate spaces and equal budgets:
1. expert/rule-based selection;
2. simple DOE/grid or stratified search;
3. random search as a control;
4. an appropriate Pareto optimizer such as NSGA-II;
5. matched-budget Bayesian optimization and uncertainty sampling;
6. Symthaea diversity-aware, hypothesis-discriminating and uncertainty-aware acquisition.

Reuse the parent programme's NASA materials-BO crosswalk where its benchmark tasks apply; do not imply that a generic materials benchmark is an armour benchmark. Freeze the armour-specific corpus and held-out split before a superiority test.

Report:
- feasibility and hard-constraint false-PASS;
- exact-oracle regret on small finite synthetic problems;
- hypervolume with a declared reference point, IGD+ and epsilon where appropriate;
- model-predicted versus independently measured rank reversals;
- cost to first independently confirmed feasible candidate;
- computation, material, measurement and confirmation budgets;
- failed runs, retries and exclusions;
- performance by fit/size/lot/conditioning stratum;
- uncertainty calibration, coverage and abstention.

A better synthetic optimizer score is a software result, not better armour.

### B3 — independent physical qualification

Only after competent professional review, method applicability, appropriate safety controls and explicit authorization:
- test the exact complete article and declared configuration;
- use a laboratory competent and accredited for the exact method and scope;
- pre-register conditioning, specimen numbers, uncertainty and analysis;
- use independent/blinded replication and inter-laboratory comparisons where justified;
- keep coupon, component, full article, wearer evaluation and field evidence distinct;
- bind test results to the exact model, size, lot and construction.

No design, physical test, wearer exposure, synthesis or purchase is authorized by this protocol.

## 8. Separate scorecard — no single compensatory protection score

Report each outcome separately:

| Dimension | What to report |
|---|---|
| Standard-specific protection | Exact test, edition/addenda, protection category, article scope and disposition |
| Coverage | Protected area and coverage under declared static and movement conditions |
| Mass and bulk | Complete ensemble mass, areal density, thickness/bulk and distribution |
| Blunt-trauma/injury evidence | Test-specific blunt-trauma outcome; any injury-risk interpretation separately scoped and professionally reviewed |
| Fit | Fit by body shape/size, movement and equipment configuration |
| Mobility | Range of motion and task completion under a preregistered task set |
| Thermal/moisture | Heat burden, moisture management, ventilation and wear-duration usability |
| Durability | Conditioning, cleaning, flex, aging and repeated-use evidence within the declared method |
| Damage observability | Detection sensitivity, false alarms, inspection time and inaccessible damage limitations |
| Lifecycle | Repair, replacement, lot traceability, cost, maintainability and embodied/lifecycle impacts |

A hard protection floor cannot be exchanged for lower mass, greater mobility or reduced cost. Conversely, a system that passes a ballistic test but cannot be worn safely or used for its declared task is not a successful wearer-centred outcome.

## 9. Immediate runner-independent work

1. Publish the current NIJ and South African standards/applicability crosswalk (see companion JSON).
2. Extend the existing B0 verifier with exact body-armour configuration and claim-lifecycle cases; do not build a second verifier.
3. Add a body-armour claim-state crosswalk to the existing damage/repair oracle.
4. Prepare literature reproduction cases and identify rights-cleared open data; do not download, ingest or redistribute data before rights/version/hash checks.
5. Define wearer fit/mobility/thermal measurement protocols with competent ergonomics/occupational-health partners before any wearer study.
6. Define solver-selection criteria and an explicit model validation plan before any material impact simulation.
7. Only after B0/B1 review, design a benign coupon/inspection demonstrator under qualified lab supervision. No live ballistic testing is a v0.1 deliverable.

## 10. Claim ceiling

The initial deliverables may establish standards semantics, configuration identity, evidence lineage, synthetic verifier behaviour, literature mapping, solver selection criteria and software decision quality. They do not establish ballistic resistance, injury reduction, product readiness, certification, regulatory compliance, manufacturing readiness, field protection or superiority over commercial armour.

All future physical work requires explicit human authorization, competent engineering/laboratory oversight, exact test protocols and appropriate safety controls.