# Laundry Service System Benchmark and Reference Architecture v0.1

**Status:** research programme, benchmark contract and design hypotheses. No CAD freeze, appliance build, physical test, safety qualification, product superiority or legal conformity is claimed.  
**Tracker:** [LAUNDRY-SERVICE-001 #5471](https://github.com/Luminous-Dynamics/symtropy/issues/5471)  
**Composes with:** [APPLIANCE-PORTFOLIO-001 #1825](https://github.com/Luminous-Dynamics/symtropy/issues/1825), [APPLIANCE-WASHER-001 #1828](https://github.com/Luminous-Dynamics/symtropy/issues/1828), [APPLIANCE-MORPHOLOGY-001 #1893](https://github.com/Luminous-Dynamics/symtropy/issues/1893), [APPLIANCE-DOE-001 #1910](https://github.com/Luminous-Dynamics/symtropy/issues/1910), and [repair decision engine #1862](https://github.com/Luminous-Dynamics/symtropy/issues/1862).

## 1. Thesis: design the laundry outcome, not two boxes

A household does not buy a wash cycle and a dry cycle as separate abstract products. It needs textiles returned clean, adequately rinsed, acceptably dry, and not prematurely degraded. The washer and dryer therefore form a **coupled service system** even when they remain separate, independently usable and independently repairable appliances.

The first system-level question is not “which motor or heat pump has the best nameplate efficiency?” It is:

> How much resource, time, cost, repair burden and textile wear are required to deliver one kilogram of accepted dry-equivalent laundry?

### Functional unit and denominators

Use **1 kg of dry-equivalent textile accepted at the end of a declared laundry process** as the paired-system functional unit. Acceptance must be specified by textile family and include cleaning, rinse-residue, textile-care and final-moisture floors before data collection.

Keep three views separate:

1. **Washer service:** kg of dry-equivalent input successfully washed/rinsed, with energy, water, detergent, cycle time, noise and post-spin residual moisture.
2. **Dryer service:** kg of moisture removed and kg of dry-equivalent textile delivered at the declared final-moisture endpoint, with energy, time, condensation, noise and textile-care measures.
3. **Paired laundry service:** kg of accepted dry-equivalent output that passes both washer and dryer acceptance rules, with the entire wash-to-dry energy, water, detergent, time, cost, textile-wear and emissions boundary.

Do not add label numbers from appliances with different capacities or test programmes and call the result a paired benchmark. A 10 kg washer and an 8 kg dryer are useful reference points, but are not a matched pair.

## 2. Current desk benchmark: what the frontier already looks like

The values below are **public desk references**, not measurements performed by this project. Freeze exact model code, PNC/revision, market, programme, label basis and date before selecting physical controls.

| Appliance reference | Publicly reported attributes | What it tells us | Caveat |
|---|---|---|---|
| AEG LR9WW80409, 10 kg front-load washer | 20 kWh/100 Eco 40–60 cycles; 50 L/wash; 3 h 55 min; spin noise 72 dB; spin class B; residual moisture 53.9% in the cited energy-label dataset | A high-efficiency washer is already at 20 kWh/100 cycles and 50 L/cycle; basic energy-only differentiation is difficult | EnergyLabelEU is a third-party display of EU label data and links the official EPREL entry. The 53.9% residual-moisture value is secondary-dataset metadata; verify against the exact label/product fiche |
| AEG TR948V4BC, PNC 916 900 129, 8 kg heat-pump dryer | 73 kWh/100 cycles (0.73 kWh/cycle); 94% condensation efficiency; 60 dB; 3 h 19 min full programme and 2 h 05 min half-load programme | Modern heat-pump drying already has a low labelled energy figure and strong condensation performance | Values are from the manufacturer’s regional product page; confirm label/programme and market when acquiring the control |

Sources:
- Washer: [AEG LR9WW80409 label data / EPREL cross-reference](https://energielabeleu.com/model/aeg-lr9ww80409-914505606).
- Dryer: [AEG Ireland TR948V4BC product page](https://www.aeg.ie/laundry/laundry/dryers/heat-pump-dryer/tr948v4bc/). The current page identifies PNC 916 900 129.
- The EU entry benchmark for 7 kg heat-pump dryers is an older regulatory reference, not the 2026 market frontier. It is useful as a regulatory floor/reference, not as a “best-in-market” comparator: [Commission Regulation (EU) 2023/2533](https://eur-lex.europa.eu/eli/reg/2023/2533/oj/eng).

**Critical comparison rule:** the washer and dryer figures above must not be combined into a synthetic pair score. The first acquisition task is to select matched controls, document their programme/load compatibility, then measure whole-process service on a common textile workload.

## 3. Standard methods and regulatory gate

### Performance standards

- **Washer:** [IEC 60456:2024, Edition 6](https://webstore.iec.ch/en/publication/70049). The latest edition replaces the 2010 edition plus Amendment 1:2022 and includes updated methods for cleaning performance, wool cleaning, LAS rinsing, gentleness of action, low-power modes, wash-load temperature, cold-water machines, alternative ambient conditions and mixed polyester/cotton loads. It also clarifies that noise is measured under the IEC 60704 series.
- **Tumble dryer:** [IEC 61121:2012, Edition 4](https://webstore.iec.ch/en/publication/4539), with current national adoption and any amendments to be confirmed at protocol freeze.
- **Combined washer-dryer:** [IEC 62512:2012](https://webstore.iec.ch/en/publication/7145) and the applicable market adoption/amendments. Combined appliance methods do not replace the separate washer and dryer measurements for a separate-appliance pair.
- **Washer noise:** IEC 60704-1, IEC 60704-2-4 and IEC 60704-3, as applicable.

IEC 60456:2024 is an especially useful opportunity: cleaning performance, LAS rinse performance and gentleness are now explicit measurement dimensions. We should not optimize an energy label while degrading any of those outcomes.

### Safety and conformity are non-compensatory

- [IEC 60335-2-7:2024](https://webstore.iec.ch/en/publication/80076) for household washing machine safety.
- [IEC 60335-2-11:2024](https://webstore.iec.ch/en/publication/75494) for household tumble-dryer safety, including heat-pump dryers and refrigerant-related provisions.
- [IEC TS 63576:2025](https://webstore.iec.ch/en/publication/91814) provides dryer-specific fire-risk test-method guidance. Treat lint accumulation, air-path restriction, ignition sources, over-temperature behaviour and service contamination as explicit design-review topics.
- **South Africa:** VC 9008 covers automatic washing machines, tumble dryers and washer-dryer combinations. Notice 7859 of 28 August 2026 amends VC 9008 with phased effective dates 12 and 24 months after publication. Before a product-market decision, obtain the current official Gazette schedule and applicable SANS/NRCS conformity requirements; do not infer that every amended MEPS date or obligation applies identically to each laundry category. See the [Notice 7859 of 2026 summary](https://www.acts.co.za/national-/n7859_notice_no__7859_of_2026) and the [VC 9008 policy record](https://www.iea.org/policies/8418-compulsory-specifications-for-energy-efficiency-and-labelling-of-electrical-and-electronic-apparatus-vc-9008).

No prototype should be powered or operated with guards removed. Mains power electronics, heater circuits, high-speed rotating assemblies, stored energy, water and leaks, pressure, sealed refrigerant circuits and flammable refrigerants require qualified design and test boundaries. A service panel is not permission for unqualified repair.

## 4. Paradigm shifts worth testing

### 4.1 Wash + dry -> whole laundry service

The washer's spin stage changes the dryer’s inlet moisture. Treat that as a measured coupling, not an assumption. More extraction can lower dryer energy and time, but may increase vibration, noise, creasing, imbalance, fabric damage and mechanical wear. The correct answer is a joint Pareto frontier, not “highest RPM wins.”

Test at least:
- low/nominal/high spin strategies at equivalent accepted cleaning and load;
- residual moisture after spin;
- dryer kWh and elapsed time to the same final moisture;
- textile gentleness and crease outcomes;
- vibration, noise, imbalance events and service-life proxies.

### 4.1.1 First-principles spin-to-dryer leverage (illustrative bound)

A useful early calculation gives this coupling a concrete experimental value. If residual moisture is expressed on a dry-mass basis, an 8 kg dry-equivalent load falling from 53.9% to 45.0% residual moisture carries about 0.712 kg less water into the dryer. Using approximately 2.3 MJ/kg (0.639 kWh/kg) as a first-order latent-heat value, that represents roughly 0.455 kWh of latent thermal duty. At an illustrative whole-system heat-pump COP of 2–3, the idealized electrical equivalent is approximately **0.15–0.23 kWh**.

That is an order-of-magnitude screening calculation, not a predicted product saving. It assumes the residual-moisture basis is confirmed and that the incremental water would otherwise be evaporated; it omits real dryer heat losses, sensible heating, fan/compressor controls, condensate and load-specific COP. The 10 kg washer reference and 8 kg dryer reference are not a matched pair. This calculation is only a reason to measure the coupling carefully.

The experiment must measure the washer's incremental spin energy and fabric/vibration penalties together with the dryer energy/time reduction at the same final-moisture endpoint. Optimize **net paired-system energy and textile outcome**, not dryer energy in isolation.

### 4.2 Commanded cycle -> measured accepted output

A completed programme is not evidence that textiles were cleaned or dried correctly. Use load, water, temperature, motor, moisture, vibration and airflow measurements to qualify the process result. Sensor estimates must be calibrated against gravimetric or other reference measurements.

The optimization objective becomes:

`minimise (energy, water, detergent, elapsed time, textile damage, emissions, lifecycle cost)`

subject to cleaning, rinse, textile-care, final-moisture, safety and accessibility floors.

### 4.3 Eco mode -> quality-constrained adaptive processing

Choose the least-resource process that satisfies the user-selected textile and cleanliness profile. Start with transparent deterministic baselines: fixed eco cycle, conventional sensor-adaptive cycle, and then any Symthaea-assisted policy. Run ablations to prove the cognitive component adds value beyond good sensors and good control logic.

Abstain or request a conservative cycle when load classification, sensor freshness, water quality, fabric mix or the operating envelope is uncertain. Never trade a hard quality or safety gate for a lower energy prediction.

### 4.4 Filter accessory -> quantified microfiber containment

A 2025 peer-reviewed study of three commercial point-of-use filtration systems reported 35–65% microfiber capture and found a trade-off between filtration efficiency and hydraulic performance; it also reported increased microfiber release during tumble drying in its studied conditions. This is a measurement problem, not a marketing claim. Source: [Journal of Hazardous Materials, 2025](https://doi.org/10.1016/j.jhazmat.2025.137646).

Measure captured and released mass, not only particle count. Include polymer/fibre identification, filter bypass, retained filter mass, cleaning loss, sludge/residual disposal, pressure drop, clogging, maintenance interval and user burden. The proposed ≥80% capture target below is a stretch target for a fully declared boundary, not an assumed outcome.

### 4.5 One appliance -> a repairable product-service platform

Retain the existing Symtropy service-spine and hazard-island work. The common platform should be **service contracts and evidence**, not a universal physical pump, motor, fan or heat exchanger. Publish exact-model drawings, wiring/hydraulic information where safe, part compatibility evidence, test scripts, repair roles and post-repair checks. A module boundary must beat its extra seals, connectors, fasteners, inventory, shipping, standby loss and failure modes.

### 4.6 App/cloud -> local sovereignty and grid resilience

Basic wash and dry operation, service instructions, diagnostics and essential settings must work offline. Optional connectivity can support firmware updates or energy scheduling, but an account, cloud or remote service must never be required for essential function.

For South African deployments, test safe pause/recovery after outages, power-quality tolerance, local tariffs, water pressure/quality, local repair availability and parts lead times. PV, battery and time-of-use scheduling may move flexible cycles when useful; they must never compromise safe states or household control.

## 5. Proposed performance goals

These are **research targets, not achieved results**. They are deliberately compared with the desk references above, but become valid superiority goals only after matched controls, measurement uncertainty, output floors and statistical analysis are frozen.

| Dimension | Desk reference | First differentiation hypothesis | Stretch hypothesis | Hard guardrail |
|---|---:|---:|---:|---|
| Washer energy, 10 kg class | 20 kWh/100 cycles | ≤18 kWh/100 matched cycles | ≤16 kWh/100 | No cleaning/rinse/gentleness regression |
| Washer water, 10 kg class | 50 L/cycle | ≤45 L/cycle | ≤42.5 L/cycle | No water-quality, hygiene or cleaning regression |
| Washer residual moisture after spin | 53.9% in cited secondary data | ≤50% | ≤45% | Textile damage, vibration and noise stay within frozen margins |
| Washer spin noise | 72 dB in cited label dataset | ≤68 dB aspiration | Better only if measured | Same IEC noise method; do not conflate wash and spin noise |
| Dryer energy, 8 kg class | 0.73 kWh/cycle | ≤0.62 kWh/cycle | ≤0.55 kWh/cycle | Same load, inlet moisture, final moisture and programme basis |
| Dryer duration | 199 min full / 125 min half-load on cited page | ≤180 min full-load | Better only without higher fabric stress | Equal final moisture and garment-care outcomes |
| Condensation efficiency | 94% | ≥96% | ≥97% | No unsafe condensate, corrosion or air-path fouling |
| Dryer noise | 60 dB | ≤58 dB | Lower where feasible | Like-for-like acoustic method and installation |
| Microfiber capture | 35–65% in one 2025 three-filter study | ≥70% mass capture | ≥80% | Full mass balance, low bypass, manageable pressure drop and maintenance |
| Paired-system resource use | To be established on matched controls | ≥10% lower total energy/kg accepted dry textile | ≥15% lower | No lower acceptance, poorer textile life or shifted burden |
| Repair restoration | Existing washer programme hypothesis | ≥25% lower median and P90 verified restoration time | Further improvement after baseline | Include diagnosis, parts delay, failed attempts, rework and post-check |
| User maintenance | Existing washer programme hypothesis | ≥90% completion of approved user tasks | Improve with usability trials | User tasks remain within explicit safe scope |
| Parts/service horizon | Existing EU washer/dryer rules commonly specify 10 years for selected parts | Design for ≥15 years of documented parts/service support | Longer where sourcing allows | Check exact jurisdiction/product obligations; no unsupported guarantee |
| Whole-life economics | No matched local baseline yet | Explore ≥20% lower cost per 1,000 kg accepted output | Better at same service quality | Include acquisition, electricity, water, consumables, repairs, parts, freight, downtime and end of life |

### Why these are not one score

A candidate can be lower energy but worse at stain removal, or more repairable but use more material and water, or catch more fibres while clogging and increasing wastewater overflow. Report each dimension with units, uncertainty and evidence class. Safety, accepted output, textile damage and unacceptable discharge are hard gates. Use interval-robust Pareto comparison; overlapping uncertainty means unresolved, not a winner.

## 6. Candidate physical architecture

This is a research topology, not frozen CAD or a bill of materials.

### Washer

- Front-load, cold-fill starting configuration; final capacity is chosen after local user research and control acquisition.
- Service spine for user filter/dispenser/seal maintenance and qualified access to inlet, drain, pump, heater, sensors/harness, drive, control and suspension.
- Separate structural tub from replaceable bearing/seal service unit only if leak, alignment, vibration, fatigue, manufacturing yield and lifecycle economics pass.
- Compare belt drive and direct drive on matched useful output and service task data; neither is assumed superior.
- Instrument water intake, drain, power, temperature, motor/load response and vibration using independent reference metrology.
- Evaluate a maintainable microfiber interception path without making the filter an unserviceable wet-core dependency.

### Heat-pump dryer

- Heat-pump, closed-loop air path as the reference direction; keep the refrigerant circuit as a qualified sealed hazard island.
- Make lint removal, airflow path inspection, condensate pump/drain and accessible air-side cleaning practical.
- Test filter and heat-exchanger fouling over repeated loads; monitor airflow/pressure drop, temperatures, humidity and moisture removed.
- Compare sensor arrangements and control strategies for endpoint accuracy, overdrying, underdrying, cycle time, energy and fabric wear.
- Treat refrigerant selection, charge, leakage, ignition sources, enclosure, service procedure and fire behaviour as product-specific safety decisions; do not assume R290 or any other refrigerant is a simple drop-in choice.

### Pair-level interface

The first version should not require a proprietary cloud or hard real-time link between machines. The dryer must measure or independently validate its own inlet load moisture. An optional local interface may pass load profile, estimated residual moisture, fabric-care request and a signed cycle receipt, but the dryer must safely fall back to its own measurements if the washer is absent, offline or wrong.

## 7. Luminous systems: clear ownership

| System | Laundry responsibility | Boundary |
|---|---|---|
| Symtropy | Reduced-order wash/thermal/air/moisture/resource models; architecture variants; design-of-experiments; service-task and lifecycle trade-offs; benchmark audit | Its current rigid-body physics engine is not a validated CFD or psychrometric heat-pump solver. Add reduced-order process models, then correlate against physical data before relying on them |
| Symthaea | Advisory process optimization, uncertainty/abstention, diagnostic hypothesis ranking and EVSI-based next-test selection | No actuator authority over interlocks, heater/refrigerant protection, door locks, motor safety, leak protection or fire protections |
| Mycelix | Reuse exact unit, BOM, part revision, supplier, work-order, repair and verification provenance owners | Signed provenance proves a record’s origin/integrity, not that the physical test was adequate or the appliance is safe |
| Sol-Atlas | Read-only household lifecycle cost, service accessibility, repair knowledge and material/lifecycle projections | Do not create a second source of truth for work orders, part identity or physical test evidence |
| Device safety controller | Deterministic local protective logic and bounded normal control | Independent from internet, AI, trust network and cloud availability |

### Symthaea value demonstration

Compare these policies on frozen workloads:
1. Fixed reference programme.
2. Conventional sensor-adaptive rule-based policy.
3. Symthaea advisory policy using the same sensors and resource/function constraints.

Report energy, water, cleaning/rinse/gentleness, final moisture, time, abstention, out-of-distribution response, failure rate and inference overhead. If Symthaea does not outperform the simpler adaptive baseline with statistical confidence and safe abstention, do not include it in the product’s critical path.

### Symtropy model-validation ladder

1. Close mass and energy balances with simple deterministic equations.
2. Implement Rust reduced-order washer and heat-pump dryer models with uncertainty ranges.
3. Calibrate against bench load, airflow, temperature, humidity, power, water and moisture measurements.
4. Use detailed CFD/FEA only where the reduced-order model identifies a decision-sensitive question and validated tools/data are available.
5. Maintain train/calibration and held-out validation datasets; simulation fit is not independent physical replication.

## 8. Experiment sequence: work that can start now

### T0 — desk baseline and protocol freeze

- Freeze product mission, target customer/load profile and market before selecting control products.
- Capture exact labels, product codes, manuals, service docs, spare-parts listings, warranty/parts commitments and dated local price/stock records.
- Reconcile the IEC 60456:2024 protocol with the exact label programme and record any incompatibility.
- Define textiles, stains, detergent, water hardness, conditioning, ambient temperature/humidity, load mass, transfer delay and final dryness endpoint.
- Preregister all metrics, calibration requirements, missing-data rules, exclusions, uncertainty budgets and analysis.

### T1 — instrumented control-only pilot

Acquire controls only after T0. Use separate reference meters and sensors:
- true-power energy analyser with appropriate safety category and calibrated uncertainty;
- water flow and temperature measurement;
- calibrated load scales and moisture reference method;
- ambient and process temperature/RH sensors;
- vibration and acoustic instruments;
- lint/microfiber collection, mass-balance and microscopy/polymer identification route;
- time-synchronised logging and immutable raw-data manifest.

The first pilot estimates repeatability, variance, task duration and sensor error. It does not establish product superiority. Set the sample-size calculation after observing pilot variance and deciding the minimum effect of interest.

### T2 — design-of-experiments and architecture trade-offs

Extend the existing 2×2×2 washer service architecture DOE rather than duplicating it. Add a separate washer-spin × dryer-control factorial for whole-service coupling. Candidate factors include spin strategy, dryer airflow policy, endpoint sensing and heat recovery/desiccant concepts. Use blocked/randomized runs and preserve interaction terms. Avoid a single-factor “demo” where interactions dominate.

### T3 — prototype and qualification

Only after design review, create instrumented subassemblies and guarded prototypes. First qualify containment, safe states, leakage, thermal/fire controls, interlocks, electrical safety, vibration and structural integrity. Then run performance and durability trials. A prototype passing a performance test is not thereby safety-certified or legally marketable.

### T4 — local field study

Evaluate representative South African water, power, ambient and service conditions, with consent and clear user burden. Measure actual tariff and parts availability, outages and maintenance behaviour. Field claims require a preregistered protocol and enough independent units/sites to support the claim.

## 9. Evidence and claim ladder

`design hypothesis → schema/contract validation → synthetic control tests → calibrated simulation → instrumented bench measurement → repeated physical trials → independent replication → field evidence → product conformity`

A step supports only claims that its evidence can establish. In particular:
- A JSON/Rust audit validates a benchmark contract, not a washer or dryer.
- A simulation supports design exploration, not energy, cleaning, safety or lifecycle claims.
- A supplier listing is not proof of exact-model fit or long-term availability.
- A signed test record is not a substitute for adequate test methodology.
- A short accelerated test is not a ten- or fifteen-year life claim without a validated acceleration model.
- A single unit is not a product-family qualification.

## 10. Immediate deliverables

1. Complete this service-level benchmark profile and freeze its machine-readable metrics/claim ceiling.
2. Add a strict schema/fixture and Rust semantic audit with negative controls.
3. Update the existing appliance workflow to qualify the exact new artifact and bind its digest.
4. Prepare a desk-only control matrix for South Africa and EU reference models, distinguishing sourcing leads from selected physical controls.
5. Start the reduced-order washer-spin / dryer-moisture coupling model before any hardware purchase.
6. Prepare a low-risk instrumented airflow, filter-fouling and condensate subassembly test plan; defer sealed refrigerant and mains modifications to qualified engineering review.

## References

- IEC 60456:2024: https://webstore.iec.ch/en/publication/70049
- IEC 61121:2012: https://webstore.iec.ch/en/publication/4539
- IEC 62512:2012: https://webstore.iec.ch/en/publication/7145
- IEC 60335-2-7:2024: https://webstore.iec.ch/en/publication/80076
- IEC 60335-2-11:2024: https://webstore.iec.ch/en/publication/75494
- IEC TS 63576:2025: https://webstore.iec.ch/en/publication/91814
- 2025 microfiber filtration study: https://doi.org/10.1016/j.jhazmat.2025.137646
- South African VC 9008 Notice 7859 (28 August 2026): https://www.acts.co.za/national-/n7859_notice_no__7859_of_2026
- AEG 10 kg washer label reference: https://energielabeleu.com/model/aeg-lr9ww80409-914505606
- AEG 8 kg heat-pump dryer reference: https://www.aeg.ie/laundry/laundry/dryers/heat-pump-dryer/tr948v4bc/
