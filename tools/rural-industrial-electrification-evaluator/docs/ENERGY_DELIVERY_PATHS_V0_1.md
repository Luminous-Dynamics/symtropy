# Energy Delivery Paths v0.1 — research note

## Research question

How should energy from a qualified industrial source bus reach fixed process loads and mobile platforms, while minimizing losses, capital, charging downtime and oversized onboard storage?

This work starts **downstream of the generation plant boundary**. It does not design an SMR or take authority over its nuclear island.

## Two fundamentally different industrial pathways

### Electrical transfer

A qualified generation bus feeds transformers, switchgear, distribution feeders, converters and equipment. The comparison needs a site load profile, peak and continuous power, feeder / converter ratings, outage requirements, and exact losses. Storage or demand management may buffer coincident peaks, but neither creates energy: losses, reserve and cycle life must be counted.

### Thermal transfer

A qualified thermal interface may feed a separated intermediate heat-transfer circuit, insulated pipe network, process heat exchanger and thermal storage. Heat power alone is not enough: source temperature must be high enough for the process after heat-exchanger approach temperatures and distribution losses. A primary reactor circuit is never modeled as a generic industrial pipe loop. Nuclear/process isolation and plant qualification remain under the dedicated nuclear programme.

High-temperature SMR cogeneration and thermal storage are active research areas; economics depend on site load, grid access and operating goals. See:
- IAEA nuclear process-heat / intermediate-circuit discussion: https://www-pub.iaea.org/MTCD/Publications/PDF/P1749_web.pdf
- 2025 nuclear CHP and industrial energy-park study: https://doi.org/10.1016/j.ecmx.2025.101012
- IEA, *The Path to a New Era for Nuclear Energy*: https://www.iea.org/reports/the-path-to-a-new-era-for-nuclear-energy

A lower-grade heat load may be served more effectively by heat pumps, thermal storage or recovered heat than by delivering high-grade nuclear heat directly. Those paths require coupled heat-source, electricity-input and COP accounting; v0.1 only screens direct heat transfer and does not model heat pumps.

## Transferring energy to mobile platforms

- **Conductive charge:** practical baseline for most vehicles and mobile machines. High-power charging is limited by the vehicle's receiver, battery charge curve, connector, cooling, charger and site grid, not the supply source alone. CharIN's Megawatt Charging System is one standards-based heavy-duty reference: https://www.charin.global/technology/mcs/
- **Trolley / cable-electric:** attractive to evaluate for fixed corridors and repeated high-power segments, including mine haulage. It can deliver power while moving and potentially permit a smaller battery, but adds route infrastructure and limits off-corridor operation. A 2025 case analysis studied trolley-assisted battery haul trucks with battery degradation included: https://doi.org/10.1016/j.jclepro.2025.146452
- **Battery swapping:** compare pack charging, spare inventory, standardized interfaces, safe handling, queues and degradation. A 2026 simulation compared swaps and fast charging for an eight-truck underground mine fleet: https://doi.org/10.1080/19236026.2025.2607763
- **Wireless inductive transfer:** valuable where automated contactless coupling, contamination, connector wear or continuous movement is worth the added equipment. ORNL reported an integrated 100 kW light-duty EV demonstration with 96% coupling-system efficiency across a five-inch air gap: https://www.ornl.gov/news/charging-commute . That figure is for the demonstrated wireless coupling system, not end-to-end SMR-to-wheel efficiency. Static and dynamic wireless systems require separate qualification for coil alignment, gap variation, field exposure, foreign-object detection, shielding, weather/contamination, power electronics and infrastructure utilization.
- **Hybrid direct-supply + battery:** for fixed but not completely constrained routes, trolley power can serve peak-demand sections and a battery can provide off-wire autonomy. Compare route length, connection availability, transitions and failure recovery rather than assume either architecture wins.
- **Fuel / hydrogen pathways:** include only with the upstream electricity or heat required for production, compression / storage / conversion losses, supply logistics, safety and utilization explicitly counted.

**Wireless power is not long-range energy transmission from the SMR.** The efficient system boundary usually involves conventional electrical distribution to the charging element, then a short-gap inductive coupler at the receiver.

## v0.1 accounting method and limits

For each stage:

`converted_power = input_power × efficiency_fraction`

`output_power = min(converted_power, declared_stage_limit)`

The evaluator performs a forward capacity pass and a backward required-power pass: it reports maximum deliverable power separately from the source power required to supply the requested load. That avoids counting unused route capability as energy actually delivered. The report exposes conversion losses and capacity curtailment separately at each stage. Thermal routes also report the estimated load-side temperature using the synthetic first-order sum of declared stage temperature drops. It also reports source power/energy not delivered at the endpoint; this aggregate includes both conversion losses and capacity-limited output, so it is not pure thermodynamic loss. The path is feasible only if its final output meets the same requested receiver power. Thermal routes require per-stage temperature-drop inputs and have an additional hard temperature compatibility gate: the source temperature minus the declared stage drops and terminal approach must still meet the load requirement.

The synthetic fixtures illustrate the method only. The conductive and static-wireless 90 kW / 30-minute cases use the same source budget and load requirement, so their required-source estimates can be compared arithmetically; their efficiencies and stage limits are synthetic assumptions, not measured topology performance. Stage values are not equipment quotes or field-validated performance. The evaluator has not yet been compiled or executed locally, and its calculations do not model transient power flow, reactive power, harmonics, fault protection, thermohydraulics, heat-pump COP, charging curves, dynamic wireless alignment, route utilization, cost, life-cycle emissions or safety compliance.

## Next research milestones

1. Independently verify all stage products and energy integrals.
2. Add explicit per-stage evidence/uncertainty fields and reject malformed or mismatched sources.
3. Extend to time-series load profiles, concurrency, grid import limits, storage state, charger/trolley availability and route coverage.
4. Add separate heat-pump/COP and thermal-storage models with consistent multi-source energy balances.
5. Compare architecture Pareto fronts on useful work, energy loss, power capacity, infrastructure capital, storage mass, downtime, degradation and uncertainty.
6. Hand off the nuclear interface to the existing NUCL-SMR programme and obtain specialist review before any real site-specific or regulated claim.
