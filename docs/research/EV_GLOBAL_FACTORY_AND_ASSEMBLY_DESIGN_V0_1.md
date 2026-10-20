# EV-FACTORY-DESIGN-001 — Globally deployable factory and assembly system v0.1

**Research freeze:** 20 October 2026  
**Implementation:** `tools/ev-passenger-cost-frontier/src/bin/ev_factory_frontier.rs`  
**Scenario data:** `tools/ev-passenger-cost-frontier/fixtures/ev_factory_design_v0_1.json`  
**Evidence rule:** every numeric regional and factory input in the first fixture is synthetic, including FX normalization. No supplier, integrator, landlord, government agency or other organization was contacted; no quotation was requested.

## Executive design principle

**Start with a production system, not a greenfield factory.** The cheapest *credible* first plant is usually the smallest capex boundary that still produces an independently testable, traceable and repairable vehicle. The model must compare reuse/contract assembly, a modular CKD line and an integrated body/paint plant under the same annual good-unit demand. The integrated plant must not win by hiding capex, and the contract-assembly option must not win by omitting imported bodies, logistics, tariffs or supplier dependence.

This is a design exploration, not an assertion that any archetype is the cheapest in the real world. IEA reports that global EV production is geographically concentrated and that some existing assembly capacity is severely underutilized; its 2026 outlook describes capacity utilization below 15% for BEV production in Indonesia and around 20% in Thailand in 2025. IEA also reports that new battery facilities can take over five years to approach nominal output. That is a warning against financing nameplate capacity before demand, supply-chain and yield evidence justify it—not a direct estimate for our plant. Sources: https://www.iea.org/reports/global-ev-outlook-2026/manufacturing-and-trade and https://www.iea.org/reports/global-ev-outlook-2026/electric-vehicle-batteries

## The three factory archetypes

### A. Contract assembly / flexible low-fixed-tooling line

**Default concept to test first for a low-volume launch.**

- Lease or reuse an existing industrial building; avoid buying land or constructing a new shell before utilization is demonstrated.
- Receive a certified painted body, traction battery pack and major drive assemblies as purchased modules. This avoids prematurely funding body stamping, welding and paint infrastructure. Cost and tariff inputs must include the real landed boundary, not assume supplied modules arrive free of logistics or duty.
- Use mobile kitting carts, adjustable fixtures, manual or semi-automatic lift assists, standard torque tools, serialized digital work instructions and modular workcells.
- Establish incoming inspection and component traceability; a quarantine lane; chassis/suspension; battery and e-drive installation; cabin closeout; software/fluid calibration; HV isolation and functional end-of-line (EOL) testing; final audit and segregated rework.
- Automate or duplicate the *measured constraint* only after cycle-time studies prove that doing so improves cost, quality or ergonomic risk.

The model intentionally prices purchased painted bodies separately from in-house body-material input; in-house stamping/welding/paint is included only in the integrated archetype. Actual body price, kit logistics, customs valuation, duties, minimum order quantities and warranty obligations remain unknown in this synthetic fixture.

### B. Modular CKD assembly

- Retain a common core of station interfaces, datums, torque records, electrical connectors, diagnostic identifiers, digital work instructions and EOL test schemas.
- Put repeatable operations into bolt-together workcells that can be rearranged or duplicated without replacing a long, fixed conveyor.
- Use simple fixtures first, then add automation as a documented cycle-time or quality bottleneck justifies capital expense.
- Build line balancing into the model by calculating the number of parallel cells needed at each process for the target annual good output, using net shift minutes, availability, performance and saleable yield.
- Standardize replenishment and small buffers while retaining a visible abnormality/stop-and-correct path.

Toyota's public description of its production system emphasizes Just-in-Time flow and jidoka—stopping a process when an abnormality appears, rather than producing defects faster. That is a useful comparator for this design, not a claim that the prototype reproduces Toyota's operating system: https://global.toyota/en/company/vision-and-philosophy/production-system/

Peer-reviewed research on reconfigurable automotive assembly describes managing module choice and line reconfiguration as demand, product variants and batch sizes change; a separate open-access paper discusses workstation planning in modular automotive assembly. That supports treating parallel cells, modular interfaces and reconfiguration rules as first-class parameters rather than fixing one line layout for every global site: https://www.sciencedirect.com/science/article/pii/S0007850616301238 and https://www.sciencedirect.com/science/article/pii/S2212827116312112 .

### C. Integrated flexible plant (body shop and paint)

- Explicitly include body stamping, body framing/welding and paint processes as capital-intensive cells with their own floor area, energy, labor, maintenance, utilization and validation cost.
- Treat paint emissions controls, water and chemical handling, fire design, ventilation, lifting, noise, worker exposure, maintenance and environmental permits as required work packages.
- Compare buying finished painted bodies with making them internally at the same annual demand, duty/transport assumption, warranty boundary and quality target.
- Do not recommend this route just because it creates a higher nominal local-spend figure. It becomes a candidate only when stable volume, delivered input cost, supply risk, local process competence and capital-cost evidence make its total economics defensible.

## Reference block layout

The first concept uses a one-way material flow with short, visible replenishment loops and no uncontrolled mixing of damaged batteries with normal work-in-process. The sketch is conceptual; exact aisle widths, fire compartments, hazardous-area classification, egress, sprinkler/suppression system, ventilation and separation distances must be designed and approved for the local jurisdiction and chemistry.

1. **Inbound and receiving:** dock, serialized receipt, supplier lot/COC capture, packaging inspection, sampling area and MRB hold.
2. **Separate battery receipt/quarantine:** controlled inspection, damaged or suspect pack isolation, clear status identity, physical separation and an emergency procedure. This is not a substitute for a code-compliant battery room or fire-engineering design.
3. **Kitting supermarket:** sequenced small lots, barcode/RFID part verification, visible replenishment and no mixing of variant-specific components.
4. **Non-HV subassembly:** chassis, suspension, brakes, cabin/closures and any body-prep module.
5. **HV integration zone:** authorized staff, interlocked tools, isolation verification, controlled pack installation, HV interlock and torque-record capture. Keep failed isolation, damaged-pack and rework flows segregated.
6. **Calibration and EOL:** HV insulation/isolation checks, diagnostic scan, firmware/software version, fluids and leak checks, brake/steering/alignment, lighting, charge interface, functional test and VIN-linked digital record.
7. **Quality hold and rework island:** independent disposition, defect root-cause logging and no bypass of safety-critical retests.
8. **Finished-goods release:** release only after all required manufacturing, safety and local compliance gates are passed with evidence. Digital completion is not a substitute for a required physical test.

OSHA's public lithium-ion battery safety guidance highlights hazard communication, emergency response, worker training and the potential physical/chemical hazards of failed batteries, including hazardous decomposition products. The guidance is US-specific, so it is a hazard reference—not the legal code for South Africa, India, Brazil or another deployment country: https://www.osha.gov/lithium-ion-battery-safety/control

## How the cost/capacity model works

The Rust CLI enumerates 3 illustrative site profiles × 3 factory archetypes × 4 demand cases (1,000 / 10,000 / 50,000 / 100,000 good vehicles per year), yielding 36 deterministic scenarios. Example currencies/FX and all costs are synthetic knobs to demonstrate the mathematics only.

For a target of (Q_g) good vehicles/year and saleable yield (y):

- Planned attempts: (Q_a = \lceil Q_g / y \rceil)
- Productive minutes/cell/year = operating days × shifts/day × (paid hours − unpaid breaks) × 60 × availability × performance
- Takt per attempted unit = productive minutes/cell/year ÷ (Q_a)
- Required parallel cells for process (i) = ceiling(process cycle minutes × (Q_a) ÷ productive minutes/cell/year)
- Bottleneck capacity = minimum attempted-unit capacity across required processes; good capacity = bottleneck attempts × yield.
- Building area = sum of parallel cell footprints × the layout/aisle/WIP multiplier + the dedicated battery quarantine/separation allowance.
- Installed capital = parallel cell equipment capital + building fit-out + launch/validation capital. Annual fixed cost includes labor, metered energy, lease, site and factory overhead, equipment maintenance and an explicitly assumed straight-line amortization life.
- Cost per good unit = annual factory fixed cost ÷ planned good output + declared variable kit/parts/body/consumables, yield loss/rework and warranty reserve.

Station count, operator positions, takt, bottleneck capacity, capacity headroom, total installed capex and cost per good vehicle are reported separately. The model distinguishes **factory conversion cost** from **declared vehicle-input cost**. It is not a complete vehicle COGS or import-landed model: do not make an MSRP claim from this output. The import-kit inputs do not automatically include a verified tariff, customs valuation or all transport/port costs; those should be supplied from the route model once a real component/price boundary is known.

The energy model now separates metered electrical use from useful thermal-energy demand, because an in-house paint shop is not just an electricity bill. The scenario fixture's integrated paint shop uses 75 kWh of electricity plus 550 kWh-equivalent of useful thermal energy per body (625 kWh-equivalent total); the split and energy prices are synthetic sensitivity values, not a matched estimate for a compact EV plant. As an external scale reference only, a US Department of Energy Better Buildings case reports traditional painting technology at approximately 860 kWh per truck/SUV and a newer paint shop expected to finish bodies at about 625 kWh, a projected reduction of about 27%. Vehicle class, paint quality, technology boundary and electricity/fuel mix differ, so these figures must not be directly transferred to our vehicle. They do show why body/paint deserves its own make-versus-buy and metered-energy study. Source: https://betterbuildingssolutioncenter.energy.gov/node/5475/pdf . ENERGY STAR's auto-assembly guidance identifies paint booths, HVAC, lighting and compressed air among major plant energy uses, supporting process-level submetering rather than one blended kWh/vehicle target: https://www.energystar.gov/sites/default/files/tools/Industry_Insights_Auto_Assembly_2015.pdf

The current prototype assumes ten-year equipment/fit-out amortization and 5-year launch-validation amortization, plus 4% annual equipment maintenance. Those are transparent modelling constants for this first iteration, not industry benchmarks. They should become versioned inputs with lower/upper bounds and asset-specific service lives before investment use.

## Make it cheap without making the product or plant unsafe

### Capital allocation order

1. **Avoid premature process scope.** Compare buying painted bodies and qualified packs against in-house stamping/paint or battery-cell production. Buying a module moves cost to the supply chain; it does not remove the cost.
2. **Lease/reuse shell and utilities where feasible.** Reuse can avoid greenfield land and building costs, but require structural, electrical, fire, environmental, egress, loading, ventilation, worker-safety and insurance review.
3. **Use general-purpose fixtures and modular workcells.** Keep datums and fixture interfaces standardized; use replaceable locators, modular carts, common fasteners and programmable torque tools. Do not assume a flexible fixture meets dimensional or crash-critical requirements without validation.
4. **Keep automation payback explicit.** For each station, compare manual, assisted and automated options at expected volume. A first-order break-even unit count is incremental automation capital divided by per-unit labor, scrap and quality savings net of added maintenance, energy, integration and downtime. Do not automate simply because a robot is available.
5. **Design maintenance and changeover up front.** Standard interfaces, diagnostic access, lockout/isolation, spare-part commonality and a safe rework loop reduce unplanned downtime and variant-change penalty.
6. **Measure before scaling.** Start with a pilot line and collect cycle-time distributions, defect/rework causes, actual yield, repair hours, downtime, energy, WIP age and supplier delivery variance. Add duplicate cells or shifts before adding high fixed-capital process areas where mathematically cheaper.
7. **Model working capital and supplier minima next.** The current factory tool excludes working capital, imported-part lead times, MOQ stock, foreign exchange exposure, financing, inventory carrying cost, VAT/tax and import duty. These can reverse the apparent factory winner and are required before an investment decision.
8. **Pool purchasing power carefully.** Design the core platform to accept qualified equivalent components, but only where fit, thermal/electrical limits, safety case, diagnostics, warranty and homologation remain valid. A second source has a real qualification cost, but reduces single-source fragility.

## Global deployment architecture: one core, local packs

Treat every factory as a versioned, reproducible configuration—not a unique consulting document. Keep a standard core schema with regional evidence packs.

### Global core, reused at every site

- process/station IDs, cycle-time distributions, work content, equipment class, staffing model and cell footprints;
- BOM and unit boundaries, work instructions, traceability/VIN record, torque and tool calibration requirements;
- yield/rework accounting, capacity model, maintenance and spares data, metered energy, quality gates and SHA-256 evidence manifest;
- battery/HV hazard register, quarantine workflow, lockout/isolation procedures, EOL checklist, failure handling and incident-learning interface;
- digital twin inputs and validation protocol; every model has recorded assumptions, a calibration set, a validation set, error/uncertainty and version.

### Regional pack, never assumed universal

- currency and dated FX methodology, wage/shift norms, power tariffs, grid reliability and backup requirements;
- customs classifications/origin, freight/port costs, local supplier capabilities, minimum order quantities and service-parts lead times;
- building/fire/electrical/environmental/workplace/labor law and permit evidence; emergency-service capabilities and language-appropriate training;
- local vehicle homologation, charger connector/grid requirements, climate/altitude/dust/humidity, water constraints and right-hand/left-hand-drive variants;
- localization/incentive accounting method, with legal source, effective date, claim ceiling and independent review.

A site without a verified local-code pack and battery process-safety signoff stays **not release-ready**, even if its spreadsheet economics look attractive. The sample profiles intentionally leave these gates unknown.

NIST's digital-twin work emphasizes the need for common vocabularies/interfaces and verification, validation and uncertainty quantification. That is the right approach for a global factory model: reusable data and interfaces, but quantified local uncertainty and local authorization—not one unvalidated simulation copied worldwide. Sources: https://www.nist.gov/programs-projects/digital-twins-advanced-manufacturing and https://www.nist.gov/publications/digital-twins-advanced-manufacturing-standardized-approach

## Benchmark programme

These are **proposed internal acceptance targets**, not public claims about current industry best performance. Establish a measured baseline and compare equivalent vehicle/product scope before claiming state-of-the-art superiority.

| Metric | Measurement rule | Target / gate |
|---|---|---|
| Planned capacity | Use net productive minutes and station-cycle distributions, not theoretical shift minutes alone | Bottleneck good capacity ≥ 115% of target good volume at the design case; then test sensitivity to downtime and demand uncertainty |
| Cost robustness | Report low/base/high inputs, and separately show conversion vs purchased inputs | High-cost case must fit approved factory-conversion ceiling before saying robustly cost-feasible; total vehicle COGS still needs the separate BOM/route model |
| Utilization | Report annual good output ÷ capacity and idle installed assets | Prefer the least-capital configuration that safely meets demand; do not maximize utilization at the expense of maintenance or resilience |
| Quality/yield | Measure first-pass yield, final-good yield, scrap, rework labor, defect Pareto and field return by VIN/lot | Provisional pilot exit target: ≥98% final-good yield for the established process after a defined stabilization period, with zero bypass of safety-critical tests; revise only with evidence and product risk review |
| Labor/productivity | Direct and indirect hours per good unit; split launch from steady state | Show hours/unit and cost/unit at every demand tier; improvement claims need time-study evidence |
| Changeover | Measure planned downtime and first-good-unit time when changing a variant | Set per-operation target after a representative changeover study; do not use a single fleet-wide number without product mix |
| Energy | Separate factory kWh/good vehicle by process; exclude traction pack energy and distinguish paint/body processes | Set a measured baseline by archetype, then reduce kWh/good unit without harming quality, safety or environmental controls |
| Delivery/flow | On-time start/finish, WIP age, supplier service level and throughput variation | Keep visible queues, age limits and a defined abnormality response; avoid opaque buffer accumulation |
| Maintainability | MTTR, planned maintenance hours, critical spares availability, downtime cause | Use replaceable standard modules and proven spare/repair plans; targets depend on actual equipment and operating context |
| Safety and compliance | Audited hazards, protective systems, isolation tests, drills, permits, environmental and workplace requirements | Zero unresolved critical safety findings at release; no economic metric can compensate for a failed safety gate |
| Global reproducibility | Run identical core test cases against site-specific regional packs | Same versioned outputs and formulas, explicit local differences, all assumptions/date/source recorded; unknown local law or costs remain blocked |

Toyota's TPS supplies a useful reference for built-in quality and flow; NIST's manufacturing performance programme highlights measuring asset utilization, agility and sustainability within a well-defined system boundary. Our target is not “more automation” but lower risk-adjusted cost per good, traceable vehicle under realistic demand: https://global.toyota/en/company/vision-and-philosophy/production-system/ and https://www.nist.gov/programs-projects/operations-driven-performance-measurement-smart-manufacturing-systems

## Digital twin and optimization roadmap

- **v0.1 (current):** deterministic capacity, cell count, footprint, staffing, annual capex/overhead and cost sensitivities; 36 synthetic combinations.
- **v0.2:** import cost/reliability and working-capital model; facility space constraints, shift and staffing rules, demand distribution and bottleneck headroom; support real measured inputs with typed evidence status.
- **v0.3:** line balancing and work-content distributions; discrete-event simulation for queues/WIP, line stoppages, rework and maintenance; compare multiple fixture layouts and staffing plans.
- **v0.4:** site utilities/renewables/outages, HVAC, environmental permits and battery-safety envelope; region-specific conformance packs and versioned local-law sources.
- **v0.5:** connect Symtropy digital twin to measured pilot line telemetry; compare predicted versus observed throughput, energy, yield, downtime and cost. Record error bounds and prevent unvalidated simulation from upgrading a decision gate.

A useful optimization objective is multi-objective rather than one magic score: minimize annualized fixed capital plus risk-adjusted conversion cost per good unit, while constraining safety, quality, local compliance, expected service performance, bottleneck headroom, and supply resilience. Publish the Pareto frontier; let decision-makers see which route trades capital for labor, local capability, volume or supplier dependence.

## Current status and limits

The Rust code exercises station-parallelization calculations, takt and capacity headroom, labor/energy cost, facility area including a dedicated battery-quarantine allowance, fixed-cost amortization and evidence-gated release readiness. Eight unit tests cover capacity, takt changes, yield effects, monotonic labor/energy cost, unknown safety gates, deterministic ordering and invalid inputs.

All regional wage, utility, lease, fit-out, component, station-cost, cycle-time and FX values in the fixture are invented. The output is a comparative *model demonstration*, not a capital-cost estimate. The immediate next improvement is to replace assumed values with traceable public evidence or measured pilot data—without contacting vendors or inventing quotations—and extend the line model into stochastic throughput/working capital and local compliance packs.
