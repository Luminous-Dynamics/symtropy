# Cislunar Industrialization Gate v0.1

**Status:** Research design basis / benchmark contract.  
**Owner:** Symtropy cross-domain integration.  
**Tracking:** [CISLUNAR-INDUSTRIALIZATION-GATE-001](https://github.com/Luminous-Dynamics/symtropy/issues/5341).  
**Claim ceiling:** Architecture and synthetic benchmark specification only. No cost forecast, resource reserve, flight qualification, or human-safety claim.

## 1. Decision thesis

Large rotating habitats and industrial-scale orbital shipyards should be treated as **downstream consumers of a mature cislunar industrial economy**, not as the initial business case for creating one.

Earth-supplied research stations, robotic demonstrators, small logistics nodes, and early orbital servicing facilities remain valid early steps. Their purpose is to create demand, operational experience, data, and reusable infrastructure while the off-Earth supply chain is immature.

A large habitat or shipyard becomes a serious candidate only when the products it needs can be:

1. made from a measured and accessible feedstock;
2. processed using dependable energy and maintainable equipment;
3. accepted to an explicit product specification;
4. transported and stored at the intended assembly node;
5. supplied over the intended lifecycle despite declared failures and resupply delays; and
6. delivered at a competitive risk-adjusted lifecycle cost per matched unit of service, or justified by an explicitly stated strategic/scientific premium.

**Local mass fraction is a diagnostic, not a viability test.** A system can claim high local content while depending on one imported controller, seal, bearing, catalyst, sensor, or machine-tool component that can stop the whole production chain.

## 2. System boundary: source to accepted service

The canonical flow to be evaluated is:

`resource observation → site characterization → access/excavation → beneficiation → conversion/refining → product qualification → storage/handling → transport/transfer → assembly-node acceptance → operation/repair/recycling`

Every edge carries declared units, coordinate frame/site, epoch, capacity, energy and thermal demand, product quality, yield uncertainty, residual streams, maintenance burden, and evidence status.

A material is not credited as usable supply merely because:
- a remote-sensing layer indicates possible occurrence;
- a returned sample contains the element;
- an analog process has worked on a simulant;
- an integrated prototype completed a short test;
- a candidate planner reports a production number; or
- a supplier or provenance record asserts that an item exists.

The benchmark must keep resource evidence, process capability, product qualification, and dependable delivery as separate states.

## 3. Cislunar and Solar-System network

The first reference topology includes these nodes, without assuming every route is economically useful:

| Node | Role | Typical products/services |
|---|---|---|
| Earth industrial base | Advanced manufacturing, electronics, machine tools, pressure systems, initial launch supply | High-spec components, controls, sensors, specialty materials |
| LEO integration/service node | Assembly, inspection, servicing, storage and orbital transfer | Accepted modules, repaired spacecraft, depots |
| Lunar surface industrial site | Prospecting, site preparation, power, excavation and feedstock processing | Aggregate, berms, shielding, possible water/oxygen and qualified material streams |
| Lunar orbit / cislunar depot | Aggregation, storage, transfer and assembly logistics | Delivered cargo, qualified stock, propellant only when qualified |
| Cislunar orbital foundry/shipyard | Fabrication, structural assembly, repair, recycling and integration | Accepted truss/panel/module assemblies |
| Near-Earth asteroid node (future) | Prospecting and possible resource recovery | Candidate metal/volatile feedstocks after access and transfer are proved |
| Mars-local industry (separate branch) | Surface resource use and manufacturing for Mars missions/settlements | Planet-specific products; not a cislunar feedstock source |

The map is a planning topology, not a claim that all nodes exist or that a direct route between them is currently available. Sol-Atlas is the spatial projection and evidence-context owner; the physical route, timing, energy and economics must come from qualified domain models.

## 4. Product hierarchy and substitution order

Do not use one generic `local_material` category. Separate products by function and qualification burden.

| Product class | Candidate off-Earth pathway | Qualification boundary |
|---|---|---|
| Bulk site material | Graded regolith, aggregate, berms, landing pads, shielding | Grain distribution, bearing/compaction behavior, dust, thermal stability, placement and site geotechnics |
| Glass and ceramics | Regolith-derived sintered/melted tiles, blocks, glass and ceramic stock | Composition, porosity, strength, fracture, thermal cycling, radiation/UV exposure and repeatability |
| Industrial feedstocks | Oxygen, water, silicon-, iron-, aluminum- or titanium-bearing streams | Site grade, recovery, purity, energy use, residuals, downstream process and accepted product specification |
| Non-pressure structure | Truss elements, panels, frames, brackets and equipment supports | Strength, stiffness, fatigue, fracture, joints, dimensional tolerance, NDE and as-built acceptance |
| Pressure/safety-critical structure | Pressure vessels, pressure boundaries, seals, valves and crew-system interfaces | Separate pressure, fracture, leak, fatigue, QA, proof-test and human-rating requirements |
| Precision/critical imports initially | Sensors, electronics, control modules, bearings, catalysts, optics, high-grade alloys and machine tools | Imported recurring mass and service dependency remain explicit until local production and repair are proven |

A local bulk-shielding supply chain can make a habitat more attractive without making its pressure shell local. A local truss factory can make a shipyard more attractive while electronics remain imported. Partial substitution is valuable, but must not be mislabeled as industrial independence.

## 5. Reference architecture families

Compare matched demand and product requirements across the following families:

- **E0 — Earth-supplied:** all target products manufactured on Earth and delivered to the assembly node.
- **L1 — Lunar bulk substitution:** locally produced site material, aggregate, landing pads, berms and shielding; critical pressure systems and precision equipment imported.
- **L2 — Lunar construction products:** L1 plus repeatable, accepted glass/ceramic products and selected non-pressure structures.
- **L3 — Cislunar industrial network:** qualified surface feedstock processing plus orbital handling, fabrication, repair, recycling, storage and logistics; remaining critical imports are explicit.
- **A1 — Asteroid extension:** a future, independently gated source pathway that includes prospecting, target access, capture/transfer, processing and delivery. Do not assume asteroid material is free or easy to move.
- **M1 — Mars-local branch:** separate planetary system and market; excluded from cislunar supply credits.

Every family must include its bootstrap sequence. A mature-state diagram without the machinery, power, spares, survey, metrology, transport and maintenance required to reach it is not a complete architecture.

## 6. Demand ladder: build the economy before the megastructure

| Stage | Main demand anchor | What it teaches / enables | Large-habitat gate |
|---|---|---|---|
| D0: demonstrations | Small robotic payloads and Earth-supplied research facilities | Inspection, docking, power, communications, assembly and metrology | Not applicable |
| D1: cislunar service | Cargo handling, repair, storage, site preparation and power | Logistics cadence, operational maintenance, demand and service reliability | Not applicable |
| D2: local bulk utility | Landing infrastructure, berms, radiation mass, shielding and aggregate | Repeated extraction, movement, placement, bulk QA and local power | Bulk-material supply may be credited only for accepted uses |
| D3: local industrial products | Glass/ceramics, selected metals, oxygen/water products where justified | Process uptime, purity, product QA, machine maintenance and recycling | Non-pressure structure trade study may advance |
| D4: orbital manufacturing | Reusable truss/panel/module fabrication and repair | Scale, interface standardization, mass flow, transfer and lifecycle economics | Modular shipyard trade study may advance |
| D5: mass-intensive habitation | Large rotating habitat and long-duration transit settlements | Large continuous demand for structure, shielding, spares, life-support equipment and people-support services | Commitment only after product, supply and economic gates |

A habitat is not a bootstrap prerequisite for a lunar mine, and a mine is not automatically a profitable business. Initial infrastructure must have its own mission demand and value. Habitats and shipyards become later demand anchors when the wider network can support them.

## 7. Economic accounting and break-even surface

For each candidate, calculate the lifecycle cost of the *same accepted product and useful service*, at the *same destination*, over the *same service horizon*.

The accounting boundary includes:
- bootstrap plant and infrastructure capital;
- survey, prospecting, site preparation and commissioning;
- launch and transfer mass for imported and locally produced equipment;
- excavation, beneficiation, process energy, thermal management and consumables;
- storage, packaging, handling, transport, landing and orbital transfer;
- plant availability, utilization and capacity shortfall;
- maintenance, spare production/imports, repair crews/robots and replacement;
- recycling, quality degradation, waste/residual handling and end-of-life;
- finance/discount assumptions, schedule delay and uncertainty.

Core functional units:
- accepted `kg` of a defined product grade delivered to a named assembly node;
- `habitable m³-year` and `crew-slot-year` for habitation;
- accepted structural `tonnes/year` of shipyard throughput;
- critical service availability over a declared mission horizon.

Report a vector rather than a hidden weighted score:

`(lifecycle cost, Earth-launched mass, accepted output, process energy, firm power, availability, repair time, recurring import burden, uncertainty, residual/environmental burden)`

The break-even surface varies with destination, product grade, energy price/availability, recovery yield, plant utilization, bootstrap mass, launch/transfer cost, maintenance imports, product lifetime, demand, and discount rate. The result should be an uncertainty interval or frontier—not a single false-precision number.

A local pathway advances only if:
- its product gate passes;
- its service/supply gate passes under the declared stress cases; and
- its risk-adjusted lifecycle outcome is competitive with E0 under the same functional unit, or the premium has a documented strategic/scientific justification.

Do not set a universal 70%, 80% or 90% local-mass threshold.

## 8. Reliability and closure

The industrial system must be evaluated as a time-dependent network, not a static bill of materials.

Required stress families:
- loss or degradation of a critical power/thermal service;
- delayed resupply and depletion of a critical imported spare;
- shared control, power, communications or transport failure;
- unverified local-repair claim;
- failed product batch and rework/recycling;
- reduced process yield or higher process-energy demand;
- one production line or transfer window unavailable;
- demand ramp that arrives before capacity or repair capability;
- quality degradation across repeated recycle/remanufacture cycles.

Count recurring imported dependencies as part of the service system. If an imported part has a replacement interval, include the runway, inventory and arrival distribution. No availability credit for an unverified repair capability.

## 9. Benchmark design

V0.1 is a deterministic, headless integration campaign. It consumes the receipts and declared outputs of the existing process, power, resource, logistics, closure and resilience owners. It does not duplicate their physical solvers or decision engines.

Minimum scenario families:
1. E0 Earth-supplied baseline with low initial demand.
2. L1 local aggregate/shielding with imported critical systems.
3. L1 under a delayed material-delivery event.
4. L2 with a product batch rejected at acceptance.
5. L2 with process-energy capacity constrained.
6. L3 with verified local repair and recycling.
7. High-local-mass L3 with one unverified recurring imported controller (must fail closure).
8. L3 with common-cause power/control failure.
9. L3 with reduced yield and elevated maintenance burden.
10. Demand ramp for a modular foundry before full utilization.
11. Habitat demand case with imported pressure boundary and local shielding/structure split.
12. Future asteroid-feedstock case marked `NOT_COMPARABLE` until target access and delivery inputs are qualified.

Include a Mars-local case only as a separate boundary test: it must never be credited as cislunar supply.

The harness should:
- bind each case to exact input bytes, model versions, source commit, evidence classes and toolchain;
- validate units, site/frame/time semantics and product grades;
- independently recompute mass, energy, inventory and lifecycle totals;
- preserve infeasible and indeterminate alternatives rather than hiding them;
- produce deterministic receipts and explicit reason codes;
- test negative controls before any candidate ranking;
- report the Pareto set and the assumptions that determine each crossover.

## 10. Comparison to current state of the art

Use NASA's 2026 ISAM State of Play as a structured reference for on-orbit servicing, assembly and manufacturing capabilities. It records 621 capability-area entries, 61 facilities and 169 developer profiles. This is a capability survey, not evidence that any entry is a sustained lunar production line or an orbital megastructure supply chain.

Use NASA's 2026 lunar surface technology programme to anchor the current capability gaps: power, excavation/construction, ISRU, robotics, communications/navigation, thermal/dust/environmental systems and advanced manufacturing. Its September 2026 solicitation explicitly groups surface power, regolith oxygen production, radioisotope power, in-space advanced manufacturing and nanomaterials.

Evidence distinctions that must stay visible:
- NASA's February 2026 CaRD update describes integrated prototype testing with **simulated lunar regolith** and reports carbon-monoxide production in the demonstrated test. This is a valuable process milestone, not a continuously operated lunar oxygen plant.
- NASA's ISRU overview explicitly distinguishes resource occurrence and accessibility from recoverable products, and notes that many demonstrations have been short-duration analog tests under Earth conditions.
- NASA's fission surface power programme describes a 40-kWe-class lunar system targeted for the early 2030s; this is a development programme, not an installed power source available to this design.
- The recent artificial-gravity literature continues to identify major gaps in durable, multisystem human-outcome evidence. No habitat can pass on the basis of radius/RPM calculations alone.

The competitive opportunity is to outperform by integrating product qualification, physical supply, bootstrap cost, recursive closure, reliability, lifecycle economics and human-service demand in one auditable decision contract—not by claiming a superior single solver or a larger speculative design.

## 11. Stretch objectives (targets, not results)

- Zero unexplained mass residual in each valid product path.
- Energy and thermal balances close within declared numeric tolerances; energy grade and temperature are explicit.
- 100% of frozen negative controls fail closed for the expected reason.
- Zero credit for unqualified coproducts, inferred reserves, unverified repairs or missing recurring-import dependencies.
- Byte-stable normalized receipts for identical exact inputs, code, toolchain and model versions.
- Report 2× and 0.5× sensitivities for uncertain process energy, yield, maintenance and transport.
- Demonstrate at least one domain where E0 wins, one where a hybrid wins, and one where qualified local supply wins—under matched requirements, not forced outcomes.
- Quantify the break-even ranges and their dominant uncertainties.
- Preserve service floors and product quality as hard constraints; optimize cost only among feasible candidates.

## 12. Integration ownership

| System | Responsibility |
|---|---|
| Symtropy | Physical/resource/energy/transport accounting, lifecycle and robustness evaluation, independent deterministic verification |
| Symthaea | Candidate architecture generation, causal diagnosis, design-space exploration and value-of-information proposals; no authority to weaken hard gates |
| Mycelix | Scoped provenance, rights, custody, approvals and inter-organizational evidence records; no claim that signatures prove measurement truth |
| Sol-Atlas | Spatial/temporal resource and logistics projection with explicit uncertainty, provenance, site and epoch; no canonical physical prediction |

Safety-critical structural, pressure, radiation, life-support, artificial-gravity and flight qualifications remain separate specialist gates. This integration benchmark cannot qualify those domains.

## 13. Sources

- NASA, [2026 ISAM State of Play](https://www.nasa.gov/isam/) (2026).
- NASA, [Lunar Surface Technology](https://www.nasa.gov/lunar-surface-technology/) (current programme overview).
- NASA, [Lunar Enabling Infrastructure Accelerator solicitation](https://www.nasa.gov/news-release/nasa-calls-for-proposals-to-accelerate-lunar-surface-technologies/) (8 September 2026).
- NASA, [CaRD integrated prototype update](https://www.nasa.gov/centers-and-facilities/johnson/sunlight-extracts-oxygen-from-regolith-using-solar-chemistry/) (13 February 2026).
- NASA, [In-Situ Resource Utilization overview](https://www.nasa.gov/overview-in-situ-resource-utilization/).
- NASA, [Fission Surface Power](https://www.nasa.gov/exploration-systems-development-mission-directorate/fission-surface-power/).
- [Lunar surface energy infrastructure review](https://www.nature.com/articles/s44453-026-00053-w) (2026).
- [Lunar base construction materials and technologies review](https://www.sciencedirect.com/science/article/pii/S0094576526004376) (2026).
- [Artificial gravity for long-duration human spaceflight: evidence gaps and research priorities](https://www.frontiersin.org/journals/space-technologies/articles/10.3389/frspt.2026.1967384/full) (2026).

---

**Next implementation slice:** a small machine-readable synthetic case manifest plus a thin Rust integration verifier that consumes existing space-economy receipts. Do not build another mining, power, logistics, physics or evidence engine.