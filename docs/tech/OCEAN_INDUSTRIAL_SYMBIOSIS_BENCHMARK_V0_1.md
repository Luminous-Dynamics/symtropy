# OCEAN-INDUSTRIAL-SYMBIOSIS-001 — synthetic benchmark v0.1

**Status:** an executable synthetic contract for screening candidate flows. No real coastal/offshore site or project is scored.

## Research motivation

Current ocean-economy strategy increasingly frames long-term value as dependent on ecological limits, resilient coastal communities and better governance rather than gross output alone. The European Commission's *Next wave of blue growth* (13 February 2026) emphasizes planetary-boundary risks ([EU Blue Economy Observatory](https://blue-economy-observatory.ec.europa.eu/publications/next-wave-blue-growth)). The World Economic Forum's 8 June 2026 *Regenerative Blue Economy* report groups governance, equitable finance, human capacity and technology as coupled levers ([WEF](https://www.weforum.org/publications/the-regenerative-blue-economy-pathways-to-prosperity)).

The EU/EIB-backed report on seaweed and bivalve industries identifies biorefineries, infrastructure, integrated multi-trophic aquaculture and climate adaptation as investment paths, while also noting financing, technology, regulation and climate risks ([EU report, 13 March 2025](https://blue-economy-observatory.ec.europa.eu/publications/unlocking-potential-europes-seaweed-and-bivalve-industries_en)). These are opportunities to test rather than automatic recommendations.

Two tempting pathways need stronger gates. A 2025 review finds marine carbon-dioxide-removal approaches still face major gaps in efficacy, permanence, additionality, MRV, cost/resource/energy inputs and environmental/social effects ([Annual Review of Marine Science](https://doi.org/10.1146/annurev-marine-040523-014702)). A 2025 review of ocean thermal energy conversion highlights multi-energy and by-product integration, but also practical efficiency and high-capital-cost constraints ([Energy Conversion and Management: X](https://doi.org/10.1016/j.ecmx.2025.101188)). Offshore hydrogen reviews likewise identify challenges in cost, desalination/water treatment, brine and environmental effects ([Ramakrishnan et al.](https://doi.org/10.1016/j.rser.2024.114320)).

## Industry portfolio hypotheses

Start with service and circularity opportunities before new extractive frontiers:

1. **Ocean observation as a utility:** calibrated passive sensing, environmental monitoring/MRV, coastal hazard warning, port/cable/renewable inspection and shared public-interest data products.
2. **Renewable multi-service hubs:** offshore wind/wave/tidal where site-appropriate, storage, flexible loads, desalination/refrigeration, then hydrogen/ammonia/e-fuels only with verified buyers, logistics and lifecycle economics.
3. **Heat, water and cold-chain integration:** recover low-grade heat for a compatible process; desalination and refrigeration cogeneration; OTEC only after pumping parasitics, corrosion/fouling, intake/outfall, thermal effects and capex are fully included.
4. **Regenerative aquaculture and biomanufacturing:** seaweed/bivalve cultivation, carefully evaluated integrated multi-trophic aquaculture, residues to biopolymers/bioproducts, enzymes and biomaterials; include contaminants, disease, invasives, nutrients and benefit-sharing.
5. **Circular residuals and repair:** marine debris/ghost gear capture, port waste sorting, ship repair/remanufacture, and selective recovery from already-concentrated industrial residuals or brines only when chemical fate and full-life-cycle accounting support it.
6. **Restoration and resilience services:** location-appropriate mangrove, seagrass, saltmarsh, reef or other restoration linked to measured ecosystem outcomes and long-term monitoring—not area planted or credits alone.
7. **Robotics and field-service fleets:** non-contact monitoring, maintenance of renewable/aquaculture/port assets, authorized selective debris recovery and inspection with cumulative fleet disturbance budgets.
8. **Carbon/alkalinity research and MRV:** monitoring, controlled trials and independent measurement before any tonne-based product claim; do not treat seaweed production or alkalinity addition as automatic durable removal.

## Paradigm shift

Move from maximizing each industry's standalone ROI to **co-designing a portfolio under typed resource flows, real capacity, lifecycle counterfactuals, permission, ecological ceilings and uncertainty**. Waste heat, brine, freshwater, electricity or biomass is not a “resource” merely because a spreadsheet assigns it a price. Its quantity, quality, timing, transport, treatment burden, alternative use and receiving process must all fit.

The evaluator gives each stream a type and canonical unit; matches producer and consumer ports; checks declared temperature/purity/salinity range where applicable; and requires matching time windows. Candidate links need explicit rights review and permitted use, environmental review, known chemical fate, monitoring coverage, route bounds, a counterfactual and a distinct benefit-claim ID.

Each candidate portfolio must respect aggregate source availability, sink demand, and separate sound, light, sediment, thermal, nutrient and chemical-pressure caps. A duplicated avoided-emissions or waste-diversion claim blocks a combination. The empty portfolio is enumerated as a no-build baseline.

## Frozen acceptance goals

- Zero accepted flows with a wrong stream type or canonical unit.
- Zero accepted links that violate declared quality or time-window compatibility.
- Zero portfolio above producer capacity, consumer demand, transfer-distance bounds, or any independent ecological-pressure cap.
- Zero use while rights are restricted/unresolved, environmental review is incomplete, chemical fate is unknown, or monitoring is below the preregistered threshold.
- Zero duplicated benefit-claim/counterfactual IDs within a portfolio.
- Every portfolio output preserves net value, lifecycle GHG delta, water saved, local value, transfer energy and every pressure channel separately.
- Exact exhaustive subset enumeration for the tiny frozen cases; deterministic receipt bound to corpus bytes and source commit.
- Pareto frontier rather than opaque weighted sustainability score; no-build can remain on the frontier.
- All values remain explicitly synthetic. A passing benchmark is contract evidence only, not proof of commercial feasibility or safety.

## V0 limitations and next gates

This version uses predeclared synthetic quality values and lifecycle deltas; it is not a process simulator or LCA engine. It does not compute thermodynamics, plume dispersion, hydrology, species impacts, corrosion rates, cost curves, financing, market demand or regulatory approvals. Port capacities are static and time windows are exact identity matches, not a scheduling model. The lifecycle/GHG fields must not be used as credit issuance.

Next increments should add dimensionally typed physical models with uncertainty ranges, time-expanded capacity, reliability and storm/heat stress, transparent functional-unit/counterfactual definitions, uncertainty propagation, a multi-scenario exact oracle, and community/local-benefit reporting. Any real site evaluation must use source-pinned, rights-reviewed data and independent process/environmental review before moving beyond research status.
