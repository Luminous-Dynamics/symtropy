# WaterLoop modular household non-potable architecture — design basis v0.1

**Status:** pre-prototype engineering design basis; not a construction drawing, treatment specification, product approval or water-quality claim.  
**Owner:** Symtropy WATERLOOP-MODULAR-ARCH-001 [#5822](https://github.com/Luminous-Dynamics/symtropy/issues/5822).  
**Composes with:** WaterLoop service continuity #5364 / PR #5368; economics #5425 / PR #5429; sanitation chain #4902; water-quality evidence Mycelix #6100; process closure #2152; leak diagnostics #4963.

## 1. Design objective

Deliver a defined non-potable service (initially toilet flushing and restricted subsurface irrigation where locally permitted) from qualified alternative sources, while preserving potable-water integrity, safe fallback, maintainability and auditable whole-system performance.

The unit of design is **useful, quality-qualified water delivered at the point of use**, not raw litres captured, tank capacity, filter throughput or percentage removal in one treatment stage.

A source is not safe because it is called greywater or rainwater. Every source has a feed envelope; every use has an exposure profile; every barrier has a measurable claim and failure response.

## 2. Evidence and engineering implications

1. Ninsiima et al. (published 23 Sep 2026) reviewed 131 empirical studies and found large source/time/storage variability: COD 100–2,000 mg/L, TSS 30–1,000 mg/L, laundry surfactants often above 100 mg/L, and frequent microbial indicators above reuse recommendations. These are literature ranges, not design values for a particular household. The consequence is source segregation, feed characterization and modular pretreatment—not sizing a universal filter from a mean value. [DOI 10.1007/s42452-026-09559-0](https://doi.org/10.1007/s42452-026-09559-0).
2. García-Agudelo et al. (29 Oct 2026) compared two residential bench trains on synthetic light-greywater feed. UF+UV and ozone+GAC+UV showed different organic/turbidity and microbiological outcomes. This is evidence that treatment selection is multi-objective; it is not a transferable guarantee for real household influent. [DOI 10.1111/1752-1688.70162](https://doi.org/10.1111/1752-1688.70162).
3. Morandi & Steinmetz (Aug 2026) review constructed wetlands and emphasize stream-specific pretreatment, clogging control, hybrid stages and post-disinfection where stringent reuse requires it. Nature-based treatment is a candidate for land/operation profiles that can support it, not a compact household drop-in by default. [DOI 10.1016/j.coesh.2026.100737](https://doi.org/10.1016/j.coesh.2026.100737).
4. South African GN 6292 (6 Jun 2025), Regulation 12, requires Water Services Institutions planning greywater use beyond a household boundary to identify and manage risks in a wastewater risk abatement plan and use the WRC 2018 greywater guidelines to assist. Obtain current legal interpretation and approvals for the actual site/use. [Gazette 52814](https://www.gov.za/sites/default/files/gcis_document/202506/52814gon6292.pdf).
5. City of Cape Town's public guidance advises against storing untreated greywater for more than 24 hours and excludes kitchen sinks/dishwashers, toilet water and pool/spa water from its household greywater advice. Treat this as a conservative local reference, not a nationwide legal threshold. [City guidance](https://www.capetown.gov.za/family-and-home/residential-utility-services/residential-water-and-sanitation-services/alternative-water-sources/).

## 3. Typed source and use boundaries

| Stream | Baseline admission | Main concern / control |
|---|---|---|
| Bathroom shower/bath | Candidate baseline greywater after source survey | Body oils, hair, skin particles, soaps, microbes; screened and characterized |
| Hand basin | Conditional; characterize cleaning products and use | Cosmetic residues, microbial contamination, variable loads |
| Laundry | Separate branch only after detergent/feed characterization | Surfactants, sodium, boron, lint, high episodic load; no automatic blending |
| Kitchen sink / dishwasher | Excluded from baseline | Fats, oils, grease, food solids, high organic and microbial load |
| Toilet / blackwater | Strictly excluded | Pathogen and solids load; separate sanitation system |
| Pool/spa | Excluded | Disinfectants and treatment chemicals |
| Roof rainwater | Separate candidate source; separate first-flush and debris control | Roof material, bird/animal faeces, metals, debris, microbial risk |
| Potable makeup | Physically isolated backup only | Cross-connection prevention; approved air gap/backflow design |
| Treated non-potable | Released only for named approved use | Quality profile, storage age, stagnation, residual disinfectant if applicable |
| Rejects, backwash, sludge, spent media | Captured and routed to approved disposal | Residual mass/quality and disposal route |

**Default v0.1:** bathroom-only greywater. Laundry and rainwater are optional, isolated modules, not blended into the baseline until their own feed envelopes and treatment claims are qualified. No potable use is in scope.

## 4. Reference system architecture

    BATHROOM SOURCE(S) -> source selector / isolation -> hair & lint screen
                                                     |
                                                     v
                                        short-residence equalization
                                                     |
                                                     v
                                     feed characterization / permissive
                                                     |
                                                     v
                                replaceable treatment module interface
                                [train selected for feed + intended use]
                                                     |
                                        barrier verification / release
                                                     |
                                                     v
                                        treated-water buffer tank
                                                     |
                                quality/age/level permissive + isolation
                                                     |
                                    dedicated non-potable distribution
                                                     |
                              toilet cistern / restricted subsurface use

    LAUNDRY (optional) -> separate screen / sampling / isolation ----+
    RAINWATER (optional) -> first flush / debris / separate storage -+

    POTABLE BACKUP -> approved physical air gap / break tank -> service fallback
                           (no direct cross-connection)

    ALL REJECTS / OVERFLOW / BACKWASH / SLUDGE -> approved sewer or residual route
                                                 (never assumed stormwater discharge)

The diagram is functional only. Pipe routes, air-gap geometry, backflow device selection, electrical zones, tank venting, overflow, drainage and structural supports require site-specific competent design to applicable current standards and municipal requirements.

### Design-for-service principles

- Make each module isolatable without shutting down the whole dwelling.
- Put screens, valves, sample taps, drains, unions and service clearances where a technician can reach them without confined-space entry.
- Use keyed/colour-coded and physically incompatible potable/non-potable interfaces where practicable; durable labels at every non-potable point.
- Avoid dead legs and stagnant branches; use short, drainable pipe runs and a documented flush/recommission procedure.
- Provide full-bore cleanout access at likely solids/fouling locations.
- Make media, UV lamps, membranes, seals and sensors replaceable without cutting pipework.
- Include a manual safe bypass to sewer/approved route; bypass must not connect untreated water to reuse outlets.
- Do not place the only controller, shutoff or alarm dependency in a cloud service.
- Design for common tools, locally available fittings, standard pipe sizes and documented spares; vendor lock-in is a lifecycle risk.

## 5. Modular treatment strategy — selection is evidence-gated

The treatment module is a replaceable process cartridge/skid interface, not a promise that every process is needed or safe for every feed.

| Candidate train | When to investigate | Key failure / maintenance burden | Required independent evidence |
|---|---|---|---|
| Screen + settling + media filtration | Low-solids source, low exposure restricted reuse | Clogging, biofilm, breakthrough, variable microbes | Turbidity/TSS trend, microbial profile, headloss, cleaning interval, post-clean performance |
| Biological filter / biofilter + polishing | Biodegradable organic load with stable operator support | Startup, temperature sensitivity, sloughing, odour, recovery after shock load | COD/BOD, microbial indicators, dissolved oxygen where relevant, stability after load pulses |
| UF + validated disinfection | Where fine solids/microbial barrier is required and energy/spares are supportable | Membrane fouling/integrity failure; UV dose failure or lamp ageing | Integrity challenge, transmembrane pressure/flux, UV intensity/dose validation, independent microbial sampling |
| Ozone + GAC + validated disinfection | Where oxidation/adsorption is justified by measured contaminants | Ozone off-gas/worker hazard, by-products, GAC exhaustion, biofilm release | Ozone contact/dose and off-gas control, contaminant-specific by-products, GAC breakthrough, microbial samples |
| Constructed wetland / hybrid nature-based train | Space and land management available; hydraulic loading and seasonal operation manageable | Clogging, vector/odour, plant/media maintenance, seasonal performance | Hydraulic loading, short-circuiting, clogging, seasonal water quality, microbial risk and land/exposure controls |
| Electrocoagulation hybrid | Only after a defined contaminant/feed problem and comparative lifecycle case | Electrode consumption, sludge, passivation, power and residual management | Electrode mass, energy, sludge composition/disposal, water quality and scale-up repeatability |

**No universal treatment train is selected in v0.1.** A feed profile and use-specific hazard assessment must select the minimum effective multi-barrier train. TDS alone is not a treatment-release metric. Turbidity alone does not demonstrate microbiological safety. UV is not a residual disinfectant and cannot compensate for downstream recontamination.

## 6. Sizing and mass/energy accounting

No dimensions are specified until occupancy, source profile, demand, site, hydraulic layout and quality requirements are measured.

### Source and capture

For each source s, measure flow at the source boundary. Daily captured volume is:

    V_cap,s = sum_i(q_s,i * delta_t_i * eta_capture,s)

where q is measured source flow, delta_t is the interval, and eta_capture is measured capture fraction. Do not substitute literature averages for site measurements.

### Treatment capacity

Select design throughput from the measured flow-duration curve and target service demand, including peak-day and shock-load cases. Report both average and percentile flows; do not size only to the mean. Treatment capacity must also account for backwash/cleaning downtime and reject fraction.

    V_qualified = V_feed - V_reject - V_backwash - V_overflow - V_offspec

All terms must use the same declared boundary and time window.

### Storage

For a discrete time step:

    S_(t+1) = min(S_max, max(0, S_t + Q_in,t - Q_use,t - Q_reject,t))

Record overflow separately. Storage is not treatment. Untreated greywater storage should be minimized; the conservative local 24-hour reference must not be exceeded without a separately reviewed preservation/treatment strategy. Treated storage needs its own validated residence-time, microbial regrowth, temperature, venting and turnover envelope.

### Service demand and resilience

    V_essential,day = sum_j(n_j * v_j)

where n_j is the measured number of uses/day for fixture/use j, and v_j is the use-specific volume. The system should size to a declared fraction of the non-potable demand, not claim full household independence. Essential service reserve and discretionary irrigation are separate allocations.

### Hydraulic and electrical design

- Total dynamic head: static elevation + friction + minor losses + required terminal pressure.
- Hydraulic power: P_h = rho * g * Q * H.
- Electrical energy: integrate true measured input power over operating time; report kWh per **verified useful m3**, not per nominal pump capacity.
- Measure pump duty point, starts/hour, dry-run protection, NPSH margin and noise/vibration with the pump vendor and competent engineer.
- Include standby/UV/controller energy, backwash and cleaning energy, and backup-water energy where in the declared boundary.

### Lifecycle and residuals

Track equipment, media, membrane, UV lamp, electrodes (if any), consumables, labour, water reject, sludge, spent carbon/media, transport, packaging and end-of-life. Never count recovered water or nutrients until quality and intended-use requirements are met.

## 7. Control-state machine and safe operation

| State | Entry condition | Allowed action | Exit / recovery |
|---|---|---|---|
| ISOLATED | Startup, service, unknown source | No reuse delivery | Operator verifies configuration and test |
| CAPTURE | Source identity and selector valid | Capture into short-residence feed buffer | High level, invalid source or timeout -> BYPASS |
| PRETREAT | Screen/settling/pretreatment healthy | Feed treatment at bounded rate | High differential pressure or fault -> BYPASS |
| TREAT | Feed envelope and module permissives valid | Run qualified train | Any barrier/energy/process fault -> QUARANTINE |
| VERIFY | Required release evidence current and valid | Hold treated batch/volume | Pass -> RELEASE; fail/expired -> QUARANTINE |
| RELEASE | All required barriers and storage conditions admitted | Deliver to approved non-potable endpoints | Fault/age/quality invalid -> ISOLATE |
| BYPASS | Off-spec feed, maintenance or power fault | Route feed/reject to approved safe path | Manual inspection/restart checklist |
| QUARANTINE | Suspected contamination or invalid quality evidence | Lock reuse outlet; identify and hold/dispose affected volume | Independent sampling + authorized release |
| MAINTENANCE | Service lockout and zero-pressure/drained condition | Replace/clean/inspect module | Leak/pressure/electrical checks + flush + revalidation |
| RECOVERY | Power restored or fault cleared | Controlled restart, no automatic release | Fresh permissives and required verification |

A sensor reading cannot by itself release a batch unless its measurand, method, calibration, location, response time and uncertainty have been qualified for the specific hazard. Where online measurement cannot verify a hazard (for example, microbiological quality), the control contract must require validated process conditions plus scheduled independent laboratory sampling and hold/release rules.

### Hard interlocks

- Potable and non-potable pipework are physically separated. Use an approved air gap/break tank and backflow protection as determined by the competent plumbing designer and authority.
- Reuse valves fail closed on loss of power, invalid sensor, expired calibration, communication loss or process fault.
- A high turbidity/pressure-drop/UV fault can trigger hold/bypass but cannot prove water is safe when the alarm clears.
- No automatic recovery from QUARANTINE to RELEASE.
- No cloud/AI command may override local safety interlocks.
- No automatic connection to a potable fixture or potable tank.

## 8. Instrumentation and evidence contract

Minimum T1 channels, each with instrument ID, calibration, method, location, uncertainty and timebase:

- source and treated/reject flow (independent totalizers or gravimetric reference);
- tank level/volume and overflow;
- pressure before/after each hydraulic stage and pump suction/discharge;
- electrical true power and energy;
- temperature;
- turbidity and pH as process indicators only;
- conductivity where source/ionic-load questions require it;
- UV intensity/dose status or validated disinfection process parameter if applicable;
- membrane TMP/flux/integrity indicators if UF is selected;
- laboratory samples selected by the hazard assessment: TSS, COD/BOD, surfactants/sodium/boron for laundry feed where relevant, E. coli/indicator organisms and any contaminant specific to the source/use.

Raw observations, derived assessments, operator reports and lab results remain separate evidence classes. Water-quality interpretation and standards mapping must be consumed from Mycelix #6100; no duplicate compliance engine is created here.

## 9. Bill-of-materials classes (no vendor lock-in)

| Assembly | Required capability | Service design |
|---|---|---|
| Source selector manifold | Isolate each source; prevent accidental blending | Lockable valves, labels, sample port, keyed connectors |
| Coarse screen / lint basket | Retain hair/lint/large solids | Tool-light removal, drainable, visual inspection |
| Equalization vessel | Short-term flow buffering only | Opaque, covered, screened vent, drain, level sensor, cleanout |
| Feed pump / gravity transfer | Deliver bounded flow at duty point | Dry-run/overcurrent protection, isolation unions, accessible strainer |
| Treatment module frame | Accept one qualified train at a time | Standardized hydraulic/electrical/data interfaces, drip tray, leak detection |
| Cartridge/media housings | Replaceable process media | Differential-pressure taps, bypass locked closed, no hidden dead volume |
| Treated-water buffer | Hold only released water within validated residence conditions | Opaque, cleanable, screened vent, drain, level/temperature, overflow routed safely |
| Non-potable distribution | Deliver to named fixtures | Dedicated pipework, durable purple/approved local marking, no cross-connections |
| Potable break tank/backup | Physical separation | Approved air gap/backflow and independent supply controls |
| Controller/interlocks | Local fail-safe sequence | Manual isolation, watchdog, event log, offline operation |
| Residual handling | Safe reject/backwash/sludge routing | Sampling, containment, service access, disposal documentation |

Materials must be selected for chemical compatibility, pressure/temperature ratings, UV exposure, drinking-water proximity requirements and applicable standards. No specific polymer, seal, coating or pump is approved by this design basis.

## 10. T1 bench qualification plan

### Stage A — baseline characterization

- Collect at least three temporally separated composite/representative samples for each proposed source, including morning/evening and cleaning/laundry events where relevant.
- Record source-use diary, detergent/cleaning products, occupancy, temperature, storage time and flow.
- Obtain laboratory characterization selected by the intended-use risk assessment.
- Do not blend sources until each source is characterized and compatibility is reviewed.

### Stage B — treatment train screening

- Run at least two materially different trains under matched feed batches and equivalent service target.
- Randomize test order where practicable; include blanks, spikes/known controls and independent reference methods.
- Measure quality before and after each barrier, energy, headloss, flow, reject, maintenance time and media consumption.
- Include clean startup, shock load, low-flow stagnation, peak flow, power interruption, sensor failure and end-of-life/breakthrough conditions.

### Stage C — endurance and recovery

- Minimum 30-day continuous bench campaign as an initial engineering target (not a standard or final reliability proof), with documented daily operation and weekly independent sampling; extend to cover seasonal/operational variation before product claims.
- Seed screen blockage, media exhaustion, pump failure, UV lamp fault, membrane integrity fault (if used), level sensor stuck-high/low, flowmeter drift, power loss and incorrect source selection.
- Demonstrate reuse outlet isolation, safe bypass, no potable cross-connection, no unrecorded overflow and safe restart after maintenance.
- Measure technician time, tools, replacement cost, local spares availability and restart/revalidation time.

### Stage D — independent verification

- Blind or independently coded laboratory samples.
- Calibrated reference flow and energy instruments independent of controller telemetry.
- Reconcile input, product, reject, backwash, storage change and unexplained residual with uncertainty bounds.
- Preserve all failed runs, missing data, non-detect qualifiers and invalid measurements.

## 11. Benchmark scorecard — report a Pareto vector

| Dimension | Metric | Comparison discipline |
|---|---|---|
| Service | Useful qualified m3/day; essential-demand coverage; outage hours | Same household/use profile and source availability |
| Quality | Each hazard-specific result and uncertainty | Same methods, sample points and use-specific requirements |
| Barrier reliability | Failure probability, time to detect, hold/isolation time | Seeded faults plus endurance |
| Water closure | Input = useful + reject + backwash + overflow + storage delta + residual | Uncertainty-aware independent measurements |
| Energy | kWh/verified useful m3; peak W; standby kWh/day | Same duty and quality outcome |
| Resource | Media, chemicals, sludge, spent carbon/membrane per useful m3 | Full treatment and residual boundary |
| Reliability | Availability, MTBF/MTTR, fault recovery | Same operating hours and maintenance policy |
| Maintainability | Minutes per service, tool count, skills, replacement interval | Timed task study by independent technicians |
| Economics | Capex annualization + energy + consumables + labour + residuals per useful m3 | Local prices, tariff period matched, service equivalent |
| Equity | Essential service continuity and burden distribution | Do not trade minimum service for lower average cost |
| Environmental | Reject quality/volume, residual route, embodied material and lifecycle | Site-specific permitted boundary |

**Do not combine these into a single score.** A barrier failure or cross-connection is a hard fail; lower energy or cost cannot compensate.

### Provisional engineering targets (not achieved and not universal standards)

- zero observed potable/non-potable cross-connections in all inspection and seeded fault tests;
- 100% of seeded invalid-quality, sensor-fault, power-loss and treatment-fault cases isolate the reuse outlet before off-spec delivery;
- water balance residual consistent with combined measurement uncertainty for every valid test window;
- no untreated-greywater storage beyond the conservative local 24-hour reference;
- report service availability, quality compliance and maintenance burden independently; set numeric availability/energy targets only after a matched baseline and intended-use profile are frozen;
- every released volume traceable to source, treatment configuration, process permissives, storage age and destination-use class.

These are design acceptance goals for T1, not a claim of certification.

## 12. Prototype sequence

1. **P0 transparent hydraulic mock-up:** source selector, screen, test vessels, pump, metering, bypass and isolated non-potable loop. No reuse on occupants or food crops.
2. **P1 treatment module skid:** one source at a time, selected train, sample taps before/after each barrier, independent power/flow/pressure measurement, reject containment.
3. **P2 endurance/fault bench:** 30-day initial campaign and seeded failure/recovery protocol.
4. **P3 supervised site pilot:** only after competent design review, applicable approvals, independent laboratory plan, operator training, user consent and safe discharge arrangements.
5. **P4 replicated deployment:** only after multiple sites/seasons and lifecycle data; retain site-specific limitations.

## 13. South African review gates

Before any installation or operation, a competent professional must establish which current editions and provisions apply, including the National Building Regulations, relevant SANS 10252/10254/1732 provisions, municipal water/sewer by-laws, cross-connection/backflow requirements, electrical and pressure-equipment requirements, environmental/discharge rules and any water-use authorization. This document does not reproduce or replace copyrighted standards and does not decide whether a particular use requires permission.

For any reuse beyond a household boundary, assess GN 6292 Regulation 12 and the WRC 2018 Greywater Use and Management Guidelines; engage the responsible Water Services Institution and environmental-health authority as appropriate. Confirm the latest municipal rules for the actual site.

## 14. Open design decisions before a physical prototype

- exact first use case: toilet flushing only, or toilet + restricted subsurface irrigation;
- exact source: shower/bath only, with hand basin and laundry separately gated;
- measured household flow and peak demand profile;
- local source-water quality and cleaning-product profile;
- gravity vs pumped transfer and actual head;
- treatment candidate shortlist after lab feed characterization;
- available indoor/outdoor service area, drainage and residual route;
- operator/maintenance model and spare-parts sourcing;
- municipal and professional approval path.

**Current disposition:** design basis drafted; no physical hardware, hydraulic sizing, treatment train, water-quality result, regulatory approval or field performance has been qualified.


## 15. Luminous platform integration contract

| System | Owns | Must not do |
|---|---|---|
| **Symtropy** | Water balance, process/hydraulic models, treatment-train comparisons, design envelopes, physical T1/T2 qualification and benchmark receipts | Claim field performance from synthetic/modelled evidence; self-authorize water-quality release |
| **Mycelix** | Source/evidence provenance, instrument identity and calibration, measurand/timebase/uncertainty, raw water observations and intended-use assessment under #6100, signed release/maintenance evidence where authorized | Infer microbial safety from proxy sensors; convert an operator assertion or model output into a laboratory result |
| **Sol-Atlas** | Site/catchment/parcel context, flood/groundwater/soil constraints, land rights, household/service context and post-occupancy outcomes | Treat a site suitability layer as a permit, water-quality result or installation approval |
| **Symthaea** | Forecasting, anomaly detection, causal diagnosis, treatment optimization hypotheses and uncertainty-aware maintenance suggestions | Override local safety interlocks, release quarantined water, change a qualified treatment profile or actuate potable/non-potable cross-connections |
| **Luminous platform** | Versioned design packages, traceable requirements, evidence graph, lifecycle cost/service comparisons, accessible operator/resident interfaces and reproducible decision records | Collapse separate evidence classes or allow a dashboard score to replace competent engineering or regulatory authority |

The execution boundary is: Symthaea may propose; Symtropy may simulate and benchmark; Mycelix may attest evidence within its authority; Sol-Atlas may supply site/rights context; a competent responsible person and applicable authority decide whether a design may be installed or operated. Local hard interlocks remain independent of cloud connectivity and model outputs.
