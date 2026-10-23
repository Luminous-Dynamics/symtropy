# EV Battery Circularity Screen v0.1

**Programme:** EV-BATTERY-CIRCULARITY-001 (#2155)  
**Implementation:** standalone Rust reference evaluator using Serde/JSON  
**Input:** `fixtures/ev_battery_circularity_v0_1.json`  
**Schema:** `schemas/ev_battery_circularity_v0_1.schema.json`

## Paradigm shift

Compare a battery pack by the functions it performs across its life—not only nominal kWh, gravimetric energy density, manufacturing cost or cell-to-pack integration. Removing module boundaries may lower mass and packaging overhead; the deleted boundary's structural, thermal, electrical, diagnostic and service functions must still exist elsewhere. Those consequences affect repair granularity, part wait, labor, downtime, material recovery and lifecycle value.

This v0.1 screen is an explicitly declared-factor accounting tool. It does not invent impact factors, recycling credits, service-event probabilities or lifetime throughput. Each candidate declares the system boundary, functional unit, configuration identity and evidence. All fixture examples are synthetic and **must not be interpreted as ranking real architectures**.

## Computation

For each candidate, calculate:

- Usable pack capacity = nominal capacity × declared usable fraction.
- Usable pack specific energy = usable kWh × 1,000 / pack mass (kg).
- Module/unit repair time = isolation + diagnosis + enclosure access + unit replacement + reassembly + post-service validation.
- Pack replacement time = isolation + diagnosis + pack removal + installation + post-pack validation.
- Lifecycle service labor and declared downtime from the operation durations, event counts and stated parts waits.
- Lifecycle cost = initial pack cost + replacement-part/pack costs + declared service labor cost + declared downtime cost + electricity tariff × lifetime delivered energy.
- Declared lifecycle GWP = initial pack manufacture + replacement pack impact + repair part impact + delivered-energy use-phase impact + end-of-life processing impact.
- Recoverable material = mass per pack × (initial + whole-pack replacement count) × collection fraction × material recovery fraction.
- Unrecovered mass = total material entering the declared end-of-life route − recovered mass.

Material inventory must sum to the declared pack mass within 1e-6 kg. For each material family, the model reports collection and recovery separately. Unrecovered includes both uncollected material and collected material not recovered by the declared process. It is not synonymous with landfill mass.

## What is—and is not—being ranked

The evaluator deliberately emits **no aggregate score or automatic winner**. It reports independent physical, service, cost, material-recovery and climate-impact quantities, alongside hard qualification gates. The gate set requires structural integrity, thermal containment, electrical isolation, diagnostic access, repair validation and end-of-life handling.

A gate can only be marked `verified_pass` when its evidence is not synthetic/unknown and a reference is supplied. Synthetic fixtures leave all hard gates not assessed. A candidate is excluded from Pareto eligibility if safety gates or evidence remain incomplete. This means a better mass or impact arithmetic result cannot compensate for an unverified safety requirement.

## Benchmark targets

| Contract | Target |
|---|---:|
| Material inventory | Absolute mass residual below 1e-6 kg per pack |
| Per-material end-of-life balance | Total mass = recovered + unrecovered within 1e-6 kg |
| Input validity | Zero negative/non-finite inputs, duplicate IDs or unknown schema fields accepted |
| Evidence | Preserve evidence class, reference and note for each design and material stream |
| Safety | Zero candidates Pareto-ranked while required gates are unverified |
| Reproducibility | Canonical output independent of design/material input order |
| Lifecycle normalization | Report total declared impact and kgCO2e per declared lifetime delivered kWh |
| Scope clarity | All excluded processes and credits disclosed alongside each report |

These are software-contract targets. Once measured data exist, expand the comparison to service units (e.g. lifetime productive trips, tonne-km, passenger-km), measured repair durations, battery survival/aging curves, warranty claims, real recycling yields, and independently sourced LCA inventories. Report uncertainty ranges and sensitivity rankings; do not report a universal improvement percentage from one assumed configuration.

## Claim ceiling

This tool is not a full ISO-conformant LCA, process inventory, cell degradation or second-life model, structural/thermal/crash simulation, thermal-runaway test, repair SOP, hazardous-material handling procedure or battery qualification. It does not currently model upstream electricity, charger/vehicle glider manufacturing, battery aging, failure distributions, reuse allocation, avoided-primary-material credits, logistics, safety risk, module-part material mass, taxes/financing, grid storage or charger infrastructure.

Declared lifetime throughput, replacement counts, impact factors and costs are assumptions unless backed by traceable measured/manufacturer/literature evidence. Comparisons are valid only within the stated functional unit, boundary, technology configuration and evidence quality.

## Research anchors

- Liang et al. (16 October 2026), *Battery integration as function reassignment*, Nature Reviews Electrical Engineering: https://doi.org/10.1038/s44287-026-00342-2
- Zhao et al. (16 June 2026), *Trade-offs in battery integration for transportation*, Nature Reviews Clean Technology: https://doi.org/10.1038/s44359-026-00185-y
- *Eco-design of EV battery packs: A parametric LCA model considering integration efficiency, compression system, and disassembly*, Resources, Conservation & Recycling 231 (30 May 2026): https://doi.org/10.1016/j.resconrec.2026.108930
- *Design matters: The influence of EV battery pack design for disassembly on environmental and circularity impact*, Sustainable Production and Consumption 65 (June 2026): https://doi.org/10.1016/j.spc.2026.04.003
- *Operationalisation of eco-design through performance-based life cycle assessment of a cobalt-free NMA lithium-ion battery*, Journal of Power Sources 678 (30 June 2026): https://doi.org/10.1016/j.jpowsour.2026.240068
