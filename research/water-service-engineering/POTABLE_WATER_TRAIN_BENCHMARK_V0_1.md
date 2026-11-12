# POTABLE-WATER-TRAIN-BENCH-001 — Benchmark Protocol v0.1

**Tracking issue:** [Symtropy #4958](https://github.com/Luminous-Dynamics/symtropy/issues/4958)  
**Status:** proposed research contract; no product, process or water-quality result qualified  
**Scope:** household/point-of-use, community/small supply and municipal/utility drinking-water treatment.

## 1. Research question

Can a treatment train deliver more **profile-conforming water at the point of use per lifecycle unit of cost, energy, consumables and operator effort**, while maintaining all required contaminant/pathogen barriers under realistic source-water variation, ageing, interruption and maintenance?

This is not a universal "best filter" search. The correct treatment train depends on the raw-water hazard profile, intended use, service level, operator capability, reject/residual route and regulatory context.

## 2. Paradigm shift — from removal percentage to service yield

Replace a product claim such as "removes 99% of X" with a traceable service claim:

`V_service = integral of delivered flow only during intervals that satisfy the exact, versioned quality and operating profile`

Keep distinct:
- measured raw-water observations;
- treatment configuration and exact component revisions;
- contaminant-specific concentration/load and pathogen-specific log reduction;
- hydraulic throughput, pressure loss and downtime;
- water recovery, reject, backwash, sludge and consumables;
- outlet, storage outlet and point-of-use results;
- profile-bound derived assessment;
- certification, regulatory determination and authority to distribute/consume.

A single removal percentage or "potability score" cannot substitute for the complete analyte panel. Missing critical evidence is Indeterminate/Unsupported, never an implied pass.

## 3. Scales and candidate treatment families

### Household / point-of-use
Compare source-appropriate combinations of ceramic/depth filtration, biosand, activated carbon/adsorptive media, ion exchange, microfiltration, ultrafiltration, reverse osmosis, UV, chemical disinfection, coagulation/flocculation and protected storage/dispensing.

### Community / small supply
Compare modular multi-barrier trains: raw-water protection, screening/settling, coagulation/flocculation where appropriate, media filtration, carbon/adsorption, membranes, disinfection, treated storage, pressure distribution, backwash/reject management and maintenance logistics.

### Municipal / utility
Compare source-specific conventional and advanced processes, including coagulation/flocculation, clarification, media filtration, GAC, membrane processes, ion exchange and disinfection. Include source-to-treatment-to-storage-to-network-to-consumer boundaries.

Do not assume RO is preferable for low dissolved-solids sources; it adds energy, reject stream and membrane maintenance. Do not assume carbon removes every dissolved contaminant. Do not assume UV provides a residual disinfectant barrier. Each process stage has a declared contaminant scope and failure envelope.

## 4. External comparison baselines

### WHO / health-based reference
WHO's *Guidelines for Drinking-water Quality*, fourth edition with first, second and third addenda (2026), is the current international risk-management reference. It supports health-based targets, water safety plans and independent surveillance from catchment to consumer: <https://www.who.int/publications/i/item/9789240121225>.

WHO's International Scheme for HWT products reports 51 evaluated products across four rounds, with the Rounds III–IV report published 18 March 2026: <https://www.who.int/publications/b/80163>. The WHO microbial performance target ladder includes:
- 3-star: >=4-log bacteria, >=5-log viruses, >=4-log protozoa;
- 2-star: >=2-log bacteria, >=3-log viruses, >=2-log protozoa;
- 1-star: targeted protection under the scheme's conditions.

Use these as **microbial performance comparators for eligible household treatment products**. They do not establish removal of metals, salts, organics, PFAS or every chemical contaminant, nor do they replace local drinking-water requirements.

### Product-claim standards
For household devices, identify the exact current edition, model, configuration, claim and certified reduction for the relevant NSF/ANSI standards (e.g. 42, 53, 55, 58, 401). A standard number or marketing statement alone is not evidence that a specific model has a particular claim. Freeze live listings and edition status at benchmark intake.

### Local requirements
For South African demonstrators, use the exact applicable SANS 241 edition, national water-service rules, source/discharge/reuse permits and approved methods. For other countries, use their jurisdiction-specific health-based and regulatory requirements. WHO guidance is not a substitute for local legal limits.

## 5. Matched source-water profiles

The first corpus should include at least:
1. Low-turbidity microbial challenge.
2. High-turbidity / particle-associated microbial challenge.
3. Dissolved ionic / salinity challenge.
4. Named metals or inorganic chemical challenge.
5. Named organic chemical challenge only where validated reference methods/materials exist.
6. Mixed challenge with interactions (e.g. turbidity, organic load and disinfectant demand).
7. Temperature, pH and conductivity excursions.
8. Intermittent supply, stagnation, restart and storage.
9. Point-of-use handling / post-treatment recontamination.
10. End-of-life and analyte-specific breakthrough.
11. Fouling, scaling, biofilm, cleaning and backwash.
12. Power outage, UV fault, exhausted chemical, failed pump or bypass.
13. Missing, stale, contradictory or unqualified sensor/sample evidence.
14. Wrong cartridge, installation error, bypass, reverse flow or unapproved replacement.
15. Unit, analyte, method and detection-limit misbinding.
16. Raw-water shift outside the validated process envelope.

Use synthetic data for T0 semantic/arithmetic fixtures. Physical microbial or chemical challenges must only be carried out by appropriately equipped, approved laboratories using validated protocols and safe surrogates/challenge preparations.

## 6. Required measurements

### Treatment and health-related performance
- Pathogen-specific log-reduction value, challenge organism/surrogate, method, detection limit, confidence/uncertainty and sample custody.
- Analyte-specific influent/effluent concentration and mass load where paired flow is available.
- Treatment outlet, storage outlet and point-of-use results.
- Full required-analyte coverage; missing/censored/nondetect results retained with method-specific qualifiers.
- Disinfectant residual and contact-time evidence where relevant.
- Turbidity, pH, conductivity and temperature as process indicators, not generic proxies for safety.
- Breakthrough curves and cumulative profile-conforming treated volume.

### Hydraulic / operational performance
- Flow, pressure, differential pressure, pressure drop by stage.
- Rated versus measured throughput, peak and sustained delivery.
- Product-water recovery; reject/brine, backwash, sludge and residual flows.
- Energy per m³ of delivered profile-conforming water.
- Media, membrane, reagent and consumable replacement; cleaning and regeneration.
- Availability, service interruptions, maintenance hours, safe restart and spares.
- Alarm, isolate, hold and bypass outcomes under fault.

### Lifecycle / equity
- Lifecycle cost per 1,000 L of point-of-use water that meets the exact profile.
- Cost per household/person served at a declared daily service level.
- Capital, installation, power, chemicals, media, laboratory testing, labour, reject/residual management and end-of-life.
- Repairability, local spare availability, materials risk and user time burden.
- Distribution of outcomes by worst-served household/community; averages cannot hide non-service.

## 7. Fair comparison rules

Freeze:
- raw-water lot, contaminant panel, concentration range and challenge source;
- intended use, jurisdiction, exact profile revision and target analytes;
- product model, cartridge/media batch, firmware and assembly revision;
- flow, pressure, temperature, pH, turbidity, conductivity and operating pattern;
- start-up/run-in, stagnation, cleaning, replacement and end-of-life protocol;
- analytical methods, lab scope, instrument calibration, uncertainty and chain of custody;
- sample points, storage and distribution conditions;
- energy, water-recovery and lifecycle accounting boundary.

Use paired source-water lots, randomized run order, blinded lab samples where practicable and multiple units/batches. Split calibration and holdout by raw-water profile, product batch and site, not adjacent samples. Publish failures, no-results and nonconverged runs. Do not compare a household POU product against a municipal plant on an unadjusted kWh/m³ score.

## 8. Benchmark tiers

| Tier | Work | Allowed conclusion |
|---|---|---|
| T0 | Typed schema, synthetic fixtures, mass/volume arithmetic, evidence negative controls | Contract/arithmetic only |
| T1 | Clean-water hydraulic bench, component pressure loss, capacity and non-biological ageing | Bench performance within tested envelope |
| T2 | Independent qualified laboratory microbial/chemical challenge, breakthrough and endurance | Method-/specimen-specific result |
| T3 | Approved site pilot with source-season variation, point-of-use sampling, operators and independent surveillance | Site-/time-/profile-specific service |
| T4 | Replicated independent sites, users, seasons, operators and batches; lifecycle and equity follow-up | Broad transfer claims only within replicated scope |

No T0/T1 pass implies potable water. No synthetic or modelled outcome may be labelled a laboratory or field result.

## 9. Candidate targets — research goals, not achieved results

### Hard gates
- Zero false-safe results in the frozen critical corpus.
- Every quality claim is analyte/pathogen-specific and binds method, revision, sample identity, limits, uncertainty and evidence class.
- Missing critical analytes, stale method/calibration, invalid sample custody and out-of-envelope raw water produce Indeterminate/Unsupported/no-release.
- Zero safety clearance from turbidity, conductivity, pH, sensor health or AI score alone.
- 100% of service-volume/cost/energy values disclose the boundary and denominators.
- No composite score can compensate for failed safety, availability, access or residual-handling floors.

### Improvement objectives
- At least three treatment families and two commercial/reference products in the first matched household profile, and at least two distinct trains per admitted community/municipal profile when comparable data are available.
- >=20% lower lifecycle cost per 1,000 L profile-conforming water in at least one matched scenario without any quality or service regression.
- Investigate >=20% additional qualified service volume per consumable set against the exact matched commercial baseline, with no weakened contaminant claim.
- Report full Pareto fronts across quality, availability, water yield, energy, chemicals, cost, maintenance and equity; do not claim one universal winner.

## 10. First execution increments

1. Freeze the machine-readable profile/case manifest.
2. Implement T0 known-answer water-volume and contaminant-load arithmetic, including reject/backwash/residual streams.
3. Add missing-analyte, bypass, breakthrough, method-staleness and source-water-out-of-envelope negative controls.
4. Crosswalk assessments to Mycelix #6100 / #3233 and instrument evidence to #6034 / #6057 / #6025 / #6029.
5. Build a current comparator census from WHO Scheme results and exact NSF/ANSI model claims.
6. Define a safe T1 clean-water rig and measurement plan before procurement.
7. Select T2 challenge protocols and laboratories only after method, safety and regulatory review.

## 11. Explicit nonclaims

No product, cartridge, membrane, UV device, treatment train or water result has been qualified by this protocol. It does not establish potability, WHO scheme stars, NSF certification, SANS 241 compliance, lab accreditation, public-health safety, reuse permission or regulatory release.
