# WaterLoop modular greywater treatment architecture v0.1

**Date:** 17 November 2026  
**Parent:** [#5713 WATERLOOP-MODULAR-TRAIN-001](https://github.com/Luminous-Dynamics/symtropy/issues/5713)  
**Evidence class:** conceptual design + synthetic challenge profiles  
**Claim ceiling:** design architecture and test planning only; no selected treatment train, process guarantee, water-quality qualification or build authorisation

## 1. Design decision

Build a **serviceable pilot skid with a common hydraulic/safety chassis and replaceable process cassettes**. Do not build a single fixed household appliance first.

The first stream under consideration is separately collected shower/bath and selected hand-basin greywater. Exclude kitchen/dishwasher, toilet/urinal/bidet, faecal/urine-contaminated laundry, paint/solvent/vehicle/machinery wash, and unknown mixed drains from the first demonstrator.

Reason: greywater is not one stable feed. The 2026 review of 131 empirical studies found COD spanning 100–2,000 mg/L, TSS 30–1,000 mg/L, and laundry anionic surfactant concentrations often above 100 mg/L. Source, user behaviour, mixing, temporal variability and storage change treatment feasibility and maintenance. A universal treatment recipe would be an unjustified design assumption.

## 2. Process architecture

```text
                  SOURCE SEGREGATION AT FIXTURE
       shower / bath / specifically admitted basin only
                              │
                              ▼
                  SOURCE-DIVERSION MANIFOLD
             unknown / excluded source → approved drain
                              │
                              ▼
                     REMOVABLE SCREEN
                 hair / lint / coarse solids
                              │
                              ▼
              EQUALISATION + BATCH QUARANTINE
       level • temperature • age • source identity • sample
                              │
                              ▼
                   COMMON PRETREATMENT BAY
               settling / drainable solids capture
                              │
                ┌─────────────┼──────────────┬─────────────┐
                ▼             ▼              ▼             ▼
             TRAIN A        TRAIN B         TRAIN C       TRAIN D
            media + UF    bio-media +     ozone contact   passive /
             + UV*          polishing       + GAC + UV*    nature-based
                │             │              │             │
                └─────────────┴──────────────┴─────────────┘
                              │
                              ▼
                 POST-PROCESS SAMPLE / HOLD
               no automatic quality assumption
                              │
                 independent use-specific decision
                     ┌────────┴────────┐
                     ▼                 ▼
                ADMITTED          NOT ADMITTED /
                     │             FAULT / STALE
                     ▼                 │
            DEDICATED NON-POTABLE      ▼
               REUSE STORAGE       APPROVED DRAIN
                     │
                     ▼
          named permitted non-potable end use

* UV is only a validated barrier when its dose, flow, transmittance,
  fouling, lamp state, sensor capability and system conditions have
  been independently established for the relevant process/use.
```

Train A, B, C and D are **candidate comparative modules**. No train is selected in v0.1. The bench-scale 2026 comparison of UF+UV against ozone+GAC+UV used a synthetic contaminant mix; the reported removal values are not household field-performance or pathogen-safety guarantees.

## 3. Why this should beat the fixed-train product paradigm

The defensible advantage is not a novel filter material claim. It is a testable system architecture that:
1. preserves source identity rather than blending incompatible drains;
2. screens feed properties before a treatment module is selected;
3. has a shared chassis but interchangeable process trains;
4. measures actual useful water at a declared fixture, not nominal treatment throughput;
5. quarantines water until the intended-use quality authority admits it;
6. diverts safely on uncertain evidence, failed barriers and power loss;
7. treats cleaning, reject, solids, consumables, labour and downtime as part of performance;
8. can compare a passive/nature-based route against membrane, biological and oxidation routes under the same feed and end-use conditions.

This is a modular product-development strategy, not yet a performance advantage. The 2026 full-scale nature-based study's reported 0.05–0.17 kWh/m³ is an important comparator, but its household/school wastewater treatment boundary differs from a shower-greywater appliance. It cannot be ranked directly until feed, reuse, monitoring, pumping, land, residual and service boundaries are matched.

## 4. Critical safety boundary

- No potable/reclaimed-water cross-connection, common pressure header, hose bridge or software changeover valve in the demonstrator.
- Any municipal backup arrangement must be designed by a qualified local plumbing professional using the applicable approved physical air-gap/backflow arrangement. Do not implement a direct cross-tie.
- Use separate, conspicuously labelled non-potable pipework and outlets.
- Raw untreated greywater and treated-but-unreleased water are separate states and separate storage/pipe identifiers.
- Sensors such as turbidity, conductivity, pH, ORP, UV intensity or membrane pressure are process indicators, not proof of pathogen or chemical safety.
- The controller may hold, isolate or divert; it may not independently certify quality.
- Missing/stale/conflicted sample, calibration, process or use evidence means no reuse release.
- Loss of power, failed treatment barrier, unknown source, maintenance mode, failed overflow route or invalid storage-age state means hold/divert.
- Reuse only for a specifically admitted non-potable end use. No drinking, food preparation, bathing, personal washing or other potable-like use is in scope.

The South African WRC TT 746/17 guidance summary cautions against kitchen water, faecal/urine-contaminated laundry, chemical cleaning/paint/vehicle wash, direct human contact and untreated storage beyond 24 hours; it identifies low-risk shower/bath/rinse water as preferable. Treat this as an initial conservative screen pending review of the full report, local bylaws and professional advice.

## 5. Module interface control

Each process cassette has one immutable module identity and revision, with:
- raw-feed inlet and named inlet sample boundary;
- treated-product outlet and named outlet sample boundary;
- separate reject/backwash/drain outlet(s);
- mechanical connection, flow/pressure envelope and installation orientation;
- electrical supply, maximum load, de-energised state and lockout method;
- process input envelope and its evidence source;
- process performance claim with declared method, feed and test boundary;
- sensors, measurands, calibration, uncertainty, timebase and failure status;
- cleaning method, chemical handling, waste stream and safe service procedure;
- dimensions, dry/wet mass, material compatibility, supports and lifting points;
- consumables, replacement basis, local supply, and service time.

Dimensions, pipe sizes, pressure ratings, tank volumes, process dose, contact time and membrane flux remain **TBD** until flow profiles, use demands, module vendor data, process trials, professional review and the P&ID are complete. Do not turn illustrative capacity into a construction dimension.

The machine-readable interface contract is in `WATERLOOP_MODULE_INTERFACE_CONTROL_V0_1.json`.

## 6. First pilot skid — manufacturability concept

A first build should be a contained, serviceable skid, not a wall-mounted consumer product:
- common bolted/welded frame with bund/drip tray;
- opaque, drainable, inspectable equalisation/quarantine vessel;
- removable screen basket and sediment cleanout;
- standardised but not yet dimensioned keyed couplings between process bays;
- sample valves before and after each process barrier and at final hold;
- flow, pressure, temperature, level and electricity measurement boundaries;
- physically separate reject/backwash/drain line to an approved contained drain;
- electrical/control enclosure outside wet zones, with isolation and lockout;
- transparent module passports, BOM revision, material/pressure compatibility, lot and maintenance history;
- one-person access for routine screen/cartridge work where safe, with engineered lift aids for heavy wet modules.

Material selection must be compatibility-reviewed for greywater, cleaning chemicals, ozone/UV exposure, temperature, pressure, corrosion, UV embrittlement and local availability. Generic material family names are not a sufficient specification.

## 7. FMEA starter — no automated release on failure

| Failure | Consequence | Detection / evidence | Safe state |
|---|---|---|---|
| Unknown or excluded source | Unexpected chemistry/pathogens | Source identifier or diversion-state mismatch | Divert to approved drain; quarantine affected batch |
| Screen blockage | Overflow, bypass, loss of flow | Differential level/pressure or flow reduction | Stop feed to process; divert and service |
| Equalisation over-age / temperature excursion | Influent transformation and microbial risk | Time/temperature history | Quarantine; no release pending quality decision |
| UF integrity loss / excessive pressure drop | Barrier failure or unavailable flow | Integrity test, pressure/flow and service evidence | Isolate cassette; hold product |
| UV barrier invalid | Inadequate validated disinfection | Qualified UV system state / dose parameters invalid | Hold/divert; no quality credit |
| Ozone leak / off-gas failure | Worker exposure | Qualified gas monitor / ventilation interlock | Isolate ozone generation; evacuate/service per hazard plan |
| GAC exhaustion | Breakthrough of adsorbed contaminants | Validated replacement/change criterion and sampling | Hold/divert; replace media |
| Biological instability | Unstable treatment / poor effluent | Process-state history + lab evidence | Hold/divert; restart protocol |
| Power interruption | Incomplete treatment / no monitoring | Power-state/event receipt | Fail closed; hold batch |
| Sensor stale/missing/conflicted | False-safe release | Calibration/timebase/applicability check | Indeterminate; hold/divert |
| Reject line blocked | Residual accumulation/overflow | Level and flow closure | Stop process; maintenance lockout |
| Cross-connection / backflow hazard | Potable contamination | Physical design inspection and pressure/backflow verification | Never connect; isolate and remediate physically |

This FMEA is a starter hazard list, not a completed engineering risk assessment or SIL/PL determination.

## 8. T0 and physical test sequence

T0 challenge cases are catalogued in `WATERLOOP_GREYWATER_CHALLENGE_PROFILES_V0_1.json`. Their values are synthetic and the profile IDs do not represent field households.

T1 sequence:
1. Characterise each raw source separately before mixing; include time-of-day and use-event variability.
2. Test screen and equalisation behaviour under normal and shock loading.
3. Run each cassette on the same frozen feed batches and the same hydraulic/use profile.
4. Sample feed, intermediate points, treated hold, reject and residuals with Mycelix-admitted identity/method/calibration/uncertainty.
5. Measure real electrical energy, pressure/flow, treated volume, useful delivered volume, reject/backwash, cleaning materials and operator labour.
6. Test power loss, stale sensor, barrier fault, source shock, over-age batch, cartridge end-of-life and restart.
7. Reconcile water/residual/energy/cost within a preregistered uncertainty budget.
8. Keep all failed/rejected batches in the results.

No T0 result qualifies any water for reuse. T1 should begin in a contained unoccupied lab skid after professional hazard review and approval of an appropriate waste route. Only after stable pilot evidence and relevant professional/authority approvals should any site-specific deployment be considered.

## 9. Benchmark comparison matrix

At the same feed batch, source boundary, intended use, measurement period and safety gate compare:
- direct sewer/no reuse baseline;
- simple source separation/collection (only where lawful and risk-assessed);
- commercial packaged greywater appliance;
- UF + UV;
- ozone + GAC + UV;
- biological + polishing + validated disinfection;
- passive/nature-based process where the land and feed conditions fit.

Report:
- use-qualified useful m³ and service availability;
- individual effluent parameters and microbial outcomes chosen by the quality authority;
- energy per actual treated and useful delivered m³;
- reject/backwash/purge/solids per useful m³;
- maintenance minutes, cleaning frequency, consumables, downtime and recovery;
- capital + installed + lifecycle cost per useful m³;
- water mass balance and uncertainty;
- false release, false hold, missed barrier failure and safe-diversion performance.

Do not collapse removal percentage, nominal treatment capacity, kWh/m³, litres saved or cost into one single score. Publish a Pareto frontier for quality evidence, useful service, energy, cost, land, operator effort and residual burden.

## 10. Acceptance gates before hardware release

- approved source/use matrix and excluded stream list;
- P&ID and sampling/measurand diagram;
- module interface contract and preliminary BOM;
- FMEA review by appropriately qualified professionals;
- cross-connection/backflow and electrical hazard review;
- site-safe reject/backwash/waste disposal route;
- test challenge profile and preregistered data analysis;
- source/sample/method/standard/use evidence bound to Mycelix;
- no unknown mechanical/process dimensions carried into fabrication as assumed values.

## 11. External evidence references

- Ninsiima et al. (23 Sep 2026), *A systematic review of greywater properties and variability that govern decentralised off-grid treatment feasibility*, DOI 10.1007/s42452-026-09559-0.
- García-Agudelo et al. (first published 29 Oct 2026), *Comparative Analysis of Integrated Treatment Trains for Residential Light Graywater Reclamation*, DOI 10.1111/1752-1688.70162.
- Halicki et al. (15 Jul 2026), *Decentralized wastewater treatment and water renewal via natural systems: Performance evaluation from household to school-based scale*, DOI 10.1016/j.jenvman.2026.130241.
- WRC Report TT 746/17, *Guidelines for greywater use and management in South Africa*; public summary: https://www.wrc.org.za/wp-content/uploads/mdocs/WW%20Sept-Oct%202018_Water%20reuse.pdf.

## 12. Nonclaims

This v0.1 design does not select a best process, establish validated treatment performance, qualify pathogen/chemical removal, produce potable water, demonstrate product cost or reliability, approve a plumbing/electrical layout, or authorise fabrication/installation. The next release must use actual source characterisation and qualified design review rather than treating the current concept as a construction drawing.
