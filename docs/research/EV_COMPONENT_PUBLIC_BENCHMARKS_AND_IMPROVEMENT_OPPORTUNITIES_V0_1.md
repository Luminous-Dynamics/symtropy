# Public component benchmarks and engineering improvement plan — v0.1

Benchmark: [EV-PUBLIC-COMPONENT-BENCHMARKS-001](../../tools/ev-passenger-cost-frontier/fixtures/ev_component_public_benchmarks_v0_1.json)  
As of: **20 October 2026**  
Constraint: **public information only**. No suppliers, manufacturers, distributors or service providers were contacted. No quote requests were submitted.

## How to read this evidence

Do not equate these four types of inputs:
1. **Public retail listing** — a price visible to consumers, not the factory's landed component cost.
2. **Industry market aggregate** — a cross-company volume-weighted average, not a quote to our project.
3. **Manufacturer technical specification** — useful for sizing and capability envelopes; a price may not be publicly available.
4. **Policy/tariff source** — an important scenario boundary, subject to the exact legal classification, origin and implementation rules.

The machine-readable ledger records source URL, date, evidence class, unit, metrics, derived arithmetic, and the claims each input cannot support. Price gaps are left unknown instead of filled with guessed supplier figures.

## Strongest public anchors found

### Battery pack: best initial market anchor, still not procurement evidence

BloombergNEF's 9 December 2025 survey reports:
- $108/kWh average for all lithium-ion packs in its sample;
- $99/kWh for battery-electric-vehicle packs;
- $81/kWh for LFP packs averaged across all sectors;
- $84/kWh for packs in China, compared with material regional premiums in North America and Europe.

Source: https://about.bnef.com/insights/clean-transport/lithium-ion-battery-pack-prices-fall-to-108-per-kilowatt-hour-despite-rising-metal-prices-bloombergnef/

At the 20 October 2026 USD/ZAR mid-market reference of R16.4417 per USD (https://wise.com/us/currency-converter/usd-to-zar-rate/history/20-10-2026), the $99/kWh global BEV pack reference is about R1,628/kWh:
- 42 kWh: about **R68,365**;
- 45 kWh: about **R73,248**.

These are arithmetic benchmarks—not South African landed costs. They omit purchase volume, cell supplier terms, pack design, freight, insurance, local handling, FX spreads/hedging, warranty, tax treatment and early-manufacturing overhead. We should use $99/kWh as a high-volume plausibility anchor and test sensitivities above it, not put it in the BOM as though it were a supplier offer.

The current synthetic cost-first pack assembly in the cost-frontier fixture is:
- cells: 42 kWh × R900–R1,150/kWh;
- pack enclosure and thermal system: R15,000–R20,000 per vehicle set;
- BMS and high-voltage protection: R8,000–R12,000 per set.

That totals R60,800–R80,300 (about $88–$116/kWh at the displayed reference FX rate). It brackets the global high-volume BEV benchmark. That is a useful calibration observation—but **both our inputs and the output are still synthetic**, and the comparison is not evidence that a locally produced 42 kWh pack can be bought at these prices.

**Engineering leverage:** compare pack structures at *equal usable energy and safety function*, with explicit cell, enclosure, thermal, BMS, contactor, fuse, sensing and high-voltage distribution line items. Optimize mass, cooling pressure-drop, service access, joining method, sealing, fastener count, crush protection and material recovery. Do not simply minimize pack mass or fasteners: isolation, thermal propagation, crash-load transfer, sealing, repair and end-of-life separation are hard gates.

Research has found large design-dependent spread in traction-pack disassembly costs; one peer-reviewed open-access assessment reported up to 75% variation across its studied packs, driven partly by pack architecture, module and fastener count. This is a reason to measure teardown time and recovery—not a universal saving claim. https://doi.org/10.1016/j.apenergy.2022.120437

### Traction powertrain: reject boutique conversion-kit prices as OEM BOM

EV West publicly lists a Tesla small-drive-unit conversion kit at **US$26,222.84** for up to 200 kW. The page includes the motor/drive unit, controller and adaptation pieces; it still leaves custom mounts, custom-length axles and application-specific cooling. Source: https://evwest.com/universal-tesla-small-drive-unit-ev-conversion-package-sdu-motor-controller-throttle-stub-axles

At the reference FX that is roughly **R431,148**, already above the whole target retail ceiling. This does not mean a production OEM drive unit costs that much; it means a bespoke aftermarket conversion kit is the wrong sourcing architecture and must not be used as the low-cost-car BOM.

EMRAX publishes a 188-size axial-flux motor envelope of up to 60 kW peak, up to 33 kW continuous, 7.1–7.9 kg depending on cooling configuration, 100 Nm peak torque, and up to 96% efficiency—but does not publish a price on its product page. Source: https://emrax.com/e-motors/emrax-188/

This is a technical comparison point, **not a recommended purchase**. Its published maximum efficiency is not cycle-average efficiency and does not establish an integrated motor-inverter price or mass-production economics.

**Engineering leverage:**
- Start with one front integrated e-axle for the compact vehicle, unless the vehicle dynamics and duty cycle justify a second motor.
- Compare permanent-magnet synchronous, induction and magnet-reduced synchronous-reluctance candidates using whole-cycle motor + inverter + gearbox efficiency maps; don't optimize for peak kW or a single peak-efficiency number.
- Select continuous torque from a frozen urban/highway/gradeability duty cycle, including cabin HVAC loads, low-speed hill climbs, thermal soak and hot-day repeats.
- Treat the inverter, reduction gear, coolant circuit, mounts, cables, seals and manufacturing calibration as part of the same subsystem boundary.
- For SiC versus IGBT, require a costed crossover: purchase premium versus recovered battery capacity/range, cooling mass, switching losses and actual drive-cycle duty. The technology label itself is not the decision.

Open literature explicitly studies the coupled efficiency maps of motors and inverters and shows why they should be optimized as a system, rather than independent peak specifications: https://www.sciencedirect.com/science/article/pii/S0360544225042586 and https://www.mdpi.com/1996-1073/13/6/1698 (use the current cited study/version relevant to any specific claim).

### Tyres: real South African retail pricing, not OEM purchase price

The BYD South African spec sheet lists 185/55 R16 tyres for both Dolphin Surf variants. A South African price-comparison page showed consumer prices from **R865 to R3,627 per tyre**, updated 13 October 2026, excluding fitment. It estimates fitment and balancing around R180–R300 per tyre. Source: https://tyrecompare.co.za/tyre-size/185-55-r16

That gives a broad consumer replacement envelope of R3,460–R14,508 for four tyres before fitment. The illustrative fitment-included envelope is R4,180–R15,708; these are not equivalent tyres or OEM costs, so don't interpret either as our component budget.

**Engineering leverage:** use a common, size-efficient tyre/rim package only after load rating, speed rating, braking, aquaplaning, durability, noise and rolling resistance are jointly qualified. A tyre that slightly reduces range but worsens wet braking or load margin is not an improvement. Evaluate rolling resistance against real cycle-energy change, while keeping wet grip and tyre life as hard constraints. Standardized wheel sizes and minimizing variant proliferation can reduce spares complexity without choosing the lowest-grade tyre by sticker price.

### Charging and high-voltage electronics: publishable technical evidence, weak open pricing

BYD's public spec sheet states 6.6 kW AC charging for both trims and 30 kW DC for Comfort / 40 kW DC for Dynamic. That provides a useful segment-specific charging boundary. It is not a complete charging curve; thermal limits and taper matter. https://www.byd.com/material/byd-site/za/product/dolphin-surf/Dolphin%20Surf%20Spec.pdf

The initial online pass did **not** uncover a credible public production unit price for an automotive-grade 6.6–11 kW on-board charger, high-voltage junction box, BMS, contactors, pyrofuse, CCS2 inlet, inverter or production harness. Retail electronics components and hobby/aftermarket hardware are not suitable proxies for qualified automotive assemblies. Those price fields should remain unknown until a reliable public catalog, tender, audited BOM or market study is found; no one will be contacted to fill them.

**Engineering leverage:** reduce variant count, connector count, unnecessary HV transitions, harness mass and redundant boxes; place HV protection as close as required to fault-energy sources; optimize thermal and EMI design as a complete unit. Keep safety-related isolation, fault detection, high-voltage interlock, crash-disconnect, diagnostics, validation and repair access explicit. For charging, benchmark full time from 10–80%, cold/hot curves, cell temperature, station compatibility and total energy lost—not advertised peak kW.

### Body, chassis, cabin and electronics: public part prices are not enough

The public evidence located in this pass is too sparse to produce defensible OEM unit-cost anchors for the body-in-white, closures, suspension, brakes, airbags, HVAC, glass, seats, harness and control electronics. Retail parts, dismantler parts and consumer replacement parts have different quality, warranty, production yield and traceability boundaries.

**Engineering leverage to validate rather than assume:**
- high-strength-steel body and stamped parts versus aluminum/complex castings, using total formed-part mass, press investment, scrap/rework, corrosion, joining, crash performance and local supply;
- fewer unique brackets/fasteners and a deliberately designed repair sequence, while preserving crash-load paths and part replacement;
- a compact cabin with physical controls for critical operations, standard display electronics, simple wiring topology and software support life; no high-cost infotainment feature should be allowed to consume the cost budget by default;
- an HVAC/thermal strategy optimized for the actual South African ambient profile and windshield/cabin loads. Compare cabin heating/cooling energy and pack conditioning, not just component price;
- common modules only when they do not force a payload, safety, packaging, cooling or maintenance compromise across incompatible vehicle classes.

Each candidate design needs a frozen engineering envelope and source evidence. “Fewer parts”, “AI-designed”, “structural pack” or “local” are not in themselves cost or quality proof.

## New South African manufacturing constraint

Reuters reported on 15 October 2026 that South Africa intends to phase out policy support for existing semi-knocked-down (SKD) operations and withhold support from new SKD facilities, while tightening the definition of complete-knock-down (CKD) manufacture. The report describes an ambition to raise average local content from 38% to 60%. This is a policy announcement reported in the press, not a complete legal rulebook; we must not code an incentive as guaranteed before the published legal instruments say so. Source: https://www.reuters.com/world/africa/south-africa-stop-subsidizing-imported-vehicle-kits-pushing-automakers-build-2026-10-15/

The current SARS tariff schedule shows:
- finished all-electric passenger vehicles under tariff line 8703.80.90 at 25% general duty, with preferential entries varying by origin/trade bloc;
- lithium-ion accumulators under 8507.60 with a zero duty shown in the published columns.

Source: https://www.sars.gov.za/wp-content/uploads/Legal/SCEA1964/Legal-LPrim-CE-Sch1P1Chpt1-to-99-Schedule-No-1-Part-1-Chapters-1-to-99.pdf

These are scenario inputs only. Tariff classification, origin rules, preference eligibility, customs value, VAT, port and compliance costs must be checked for the actual product and route. A zero battery duty is not zero landed cost.

**Strategy implication:** compare at least three industrial routes in the cost model:
1. imported complete vehicle (benchmark);
2. CKD final assembly with a deliberately staged localization plan and true parts-origin accounting;
3. deeper local manufacture where volume justifies stamping, body, seats/trim, glass, harnesses, tyres/wheels, pack-enclosure fabrication and selected power-electronics or drivetrain operations.

For each route, calculate fixed tooling/certification, yield and scrap, labor, imported-content exposure, FX risk, logistics, local supplier capability and annual volume. Do not assume SKD kit assembly will retain policy advantages.

## Cost-volume sensitivity: tooling can dominate the first 1,000 units

This section derives from the current **synthetic fixture**, not a public tooling quote. The fixture assumes fixed program investments of R150–250 million for the cost-first design, R175–300 million for the serviceability-first design and R200–350 million for the structural-pack design. Amortized per vehicle, the cost-first design alone carries:

| Assumed units over which program cost is amortized | Fixed-cost contribution per vehicle |
|---:|---:|
| 1,000 | R150,000–R250,000 |
| 10,000 | R15,000–R25,000 |
| 50,000 | R3,000–R5,000 |
| 100,000 | R1,500–R2,500 |

The structural-pack variant's assumed range is R200,000–R350,000 per unit at 1,000 vehicles and R2,000–R3,500 at 100,000. This does **not** establish what actual tooling will cost. It shows that the result is extremely sensitive to volume and program boundaries: a first low-volume batch cannot finance a clean-sheet platform, dedicated dies, engineering, validation and homologation under the same unit economics as an established platform.

At the current synthetic 50,000-unit amortization, the high-cost case would need a cost reduction of approximately **18.5%** (cost-first), **21.8%** (serviceability-first), **24.2%** (structural pack), or **20.3%** (45 kWh range-margin derivative) to reach the assumed COGS ceiling. Those are model-derived reductions from unverified cost assumptions, not forecasts. A reduction in BOM costs also cannot be allowed to weaken safety, payload, repairability or warranty coverage.

**Design implication:** treat platform reuse/carryover, shared tooling, contract manufacturing, module commonality and a staged localization route as first-class alternatives—not as afterthoughts. Any shared investment must be allocated once across the products that actually use it; do not charge all tooling to every vehicle or silently spread it across unrealistic lifetime volumes.

## What we should implement next in the model

1. Preserve every public source and evidence class in the machine-readable ledger; never replace unknown production prices with public consumer proxies.
2. Extend the cost frontier with **localization and import-route sensitivity**: currency/date, shipping/insurance, tariff code/rate/origin scenario, non-recoverable tax/fees, local content definition and fixed tooling at 1k/10k/50k/100k units.
3. Add an explicit **price-boundary field** to every benchmark: ex-works, landed assembly gate, OEM COGS, wholesale, retail ex-VAT or retail inc-VAT.
4. Derive pack cost from separate cells per kWh + enclosure/thermal + BMS/HV protection + pack labor/yield/validation, then compare against the global BEV pack anchor as an external check—not as a quote.
5. Run drive-cycle design sweeps for pack size, motor/inverter efficiency, aero/rolling losses, tyre choice, mass and auxiliary loads. Couple costs to mission service and use uncertainty bounds rather than declaring a universal winner.
6. Keep the candidate **unqualified** until measured test data, release-grade cost evidence, crashworthiness, electrical safety and legal homologation exist.

The immediate engineering thesis is to combine a production-cost powertrain and a simple, repairable compact structure with a correctly sized LFP pack, low energy demand, low variant count and a locally viable manufacturing route. That is a hypothesis to test against the evidence ledger, not a claim that we already beat BYD.
