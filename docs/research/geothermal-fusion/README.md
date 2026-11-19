# Geothermal–Fusion Energy Systems

**Programme:** GEOFUSION-INTEGRATION-001  
**Status:** v0.1 research architecture and analytic benchmark contract  
**Owner:** Symtropy system integration; domain physics remains with its specialist owners  
**Tracking:** [Symtropy #6142](https://github.com/Luminous-Dynamics/symtropy/issues/6142)
**Industry baseline and matched comparison protocol:** [INDUSTRY_BASELINE_AND_BENCHMARKS.md](INDUSTRY_BASELINE_AND_BENCHMARKS.md)

## Thesis

Build an energy system that is useful with geothermal alone, becomes more resilient when paired with storage and the grid, and can accept future fusion electricity and heat streams without redesigning its evidence, dispatch or accounting foundations.

The design target is not a geothermal plant with a fusion reactor bolted on. It is a **time-aware, temperature-graded, multi-carrier energy network** whose sources, conversion stages, stores and useful services are explicitly identified and independently reconciled.

## The essential physical distinction

Geothermal heat cannot directly heat a conventional D–T plasma to fusion conditions. ITER describes plasma temperatures near 150 million °C and external heating systems including two 16.5 MW neutral beam injectors. Geothermal can contribute through a qualified power-conversion path to the facility electrical bus, and through storage can help serve pulsed/high-ramp electrical demand. It cannot be counted as direct plasma heating.

Fusion thermal output is a separate, future-facing opportunity. Blanket, divertor, vessel and balance-of-plant streams have different coolants, pressures, temperature grades and safety boundaries. They are not one generic heat source, and the values in plant-system studies are design analyses rather than operating-plant measurements.

## Architecture

The system is a typed graph with five physical layers and one evidence layer.

1. **Resource layer**
   - Natural geothermal reservoir / wellfield (hydrothermal, EGS, closed-loop or superhot).
   - Grid and other independently qualified electrical sources.
   - Future fusion electrical and thermal outputs, admitted only with source-owned plant receipts.
   - Engineered thermal stores are assets, not geothermal resources.

2. **Primary and isolation layer**
   - Geothermal production / injection and surface heat-exchange circuits.
   - Fusion primary circuits (blanket, divertor, vessel, tritium-breeding interfaces and other concept-specific loops).
   - Intermediate loops, pressure boundaries, leak detection and isolation.
   - Do not connect primary circuits merely because temperatures look compatible.

3. **Conversion layer**
   - Geothermal flash, binary/ORC, steam or candidate sCO₂ surface power blocks.
   - Fusion concept-specific Rankine, helium Brayton, sCO₂ Brayton or combined cycles.
   - Electrical transformers / converters, heat pumps and heat exchangers.
   - Temperature and fluid-property validity are hard constraints.

4. **Buffer and transport layer**
   - Electrical battery, capacitor and flywheel buffers, each with power/energy limits and losses.
   - Thermal storage by medium, temperature band and duration.
   - Mine/cavern storage, dedicated borehole thermal energy storage and natural geothermal reservoirs remain distinct.
   - Electrical and thermal networks retain their own losses, timebases and failure modes.

5. **Useful-service layer**
   - Fusion research / plant auxiliary loads: plasma heating and current drive, magnets, cryogenics, vacuum, tritium systems, cooling, controls and buildings.
   - Industrial process heat by required temperature and delivery schedule.
   - Water services: reverse osmosis powered by qualified electricity; thermal desalination only where source temperature, heat-transfer boundary, product quality and brine handling are qualified.
   - Cooling services: heat-driven absorption cooling or heat-pump integration where temperature grade and complete parasitics justify it. ASHRAE's handbook gives example single-effect absorption equipment ranges of 115–132 °C for steam-heated units (some waste-heat units lower) and 160–188 °C hot-fluid input for typical double-effect equipment; exact manufacturer maps and boundary conditions must be used. This is building/process cooling, not a replacement for fusion magnet cryogenics or cryoplant refrigeration.
   - Reverse-osmosis (RO) electrical loads and thermal desalination / multi-effect distillation (MED) only where source temperature, water chemistry, product quality, heat-exchanger approach and brine handling are qualified.
   - District heat and other site-specific services.
   - Grid export/import at an explicitly metered point of common coupling (PCC).

6. **Evidence and decision layer**
   - Source receipts, exact model/configuration digests, units, uncertainty, timebase and applicability envelope.
   - Independent mass/energy reconciliation and exergy calculation only where supported.
   - Pareto decisions across cost, service, reliability, environmental burden and resource sustainability.
   - Symthaea proposes; independently qualified models and validators evaluate; safety systems and operators retain physical authority.

## Synergy matrix

| Path | Plausible useful coupling | Must not be claimed |
|---|---|---|
| Geothermal → fusion facility | Net electricity to steady auxiliaries; recharge electrical buffers; suitable low/mid-grade heat to non-critical facility/process services | Geothermal heat directly ignites or heats D–T plasma |
| Geothermal → power cycle | ORC / flash / steam; evaluate sCO₂ when source temperature and component maps support it | Supercritical CO₂ is the geothermal reservoir fluid |
| Geothermal → industrial users | Direct heat, heat pump, desalination or process duty where temperature/chemistry match | MWth at an unusable temperature is useful process heat |
| Fusion → electrical bus | Future net export after fixed/variable recirculating power and site loads | Plasma Q is equivalent to net electric export |
| Fusion → thermal users | Concept-dependent blanket/divertor heat recovery via isolated loops and temperature-grade cascade | All fusion waste heat is high-temperature or interchangeable |
| Fusion → engineered thermal store | Charge a dedicated mine/borehole/solid-media thermal store with measured charge/recovery losses | Charged storage is new geothermal generation |
| Shared subsurface capability | Common modelling, monitoring, drilling/materials knowledge where separately qualified | One well design, fluid chemistry or safety case fits both systems |

## Multi-service energy: heat quality, not waste-heat optimism

Geothermal polygeneration is an established research direction, but it must be benchmarked as a complete system. Published studies propose geothermal combinations for electricity, cooling and freshwater. For example, a 2024 *Energy* study reports a simulated configuration combining an organic flash cycle, absorption refrigeration, Kalina cycle and humidification-dehumidification desalination, with reported model outputs of 342.3 kW electric, 301.7 kW cooling and 0.233 kg/s freshwater. These are outputs of a thermoeconomic model under its assumptions, not a demonstrated plant performance result ([DOI 10.1016/j.energy.2024.131667](https://doi.org/10.1016/j.energy.2024.131667)).

The 2026 review of desalination energy performance concludes that modern seawater reverse osmosis is already near the practical thermodynamic minimum; the larger remaining opportunities are system-level loss reduction, high-recovery process configuration, pretreatment/cleaning/post-treatment, energy recovery and brine management—not a membrane-permeability-only contest ([Wang et al., 2026](https://doi.org/10.1038/s44359-026-00221-x)).

Our resulting design principle is a **temperature-grade cascade**: first match each source stream to the highest-value service it can physically satisfy, then reuse the remaining heat at progressively lower grades. Every split is charged once against a source-interval allocation ledger. No heat may be re-used twice in the ledger simply because multiple destinations can claim the same source identifier.

## Why superhot geothermal is a distinct track

ARPA-E's SUPERHOT programme defines its target regime as greater than 375 °C and 22 MPa. At these conditions, constant-heat-capacity approximations are unsafe for system accounting. The fluid identity, composition, thermodynamic state, chemistry and property-model validity range must be pinned. The wellbore material/cement/connection lifetime and thermal-cycle degradation remain first-class uncertainty terms.

Supercritical CO₂ may be considered as an **indirect surface power-cycle working fluid**. It must be separated from the geothermal reservoir fluid by a qualified heat exchanger and pressure boundary.

A November 2026 *Geothermics* study adds a critical optimization result: under its calibrated reservoir–wellbore model, reservoir pressure and transmissivity can dominate temperature alone. Near-isenthalpic expansion produced a modeled 50–150 °C reservoir-to-wellhead temperature drop. The study found >30 MWe/well only in cases with very high transmissivity (approximately >1,000 md·m at 30 MPa); that transmissivity has not yet been deliberately engineered in deep superhot rock. Its modeled output maxima were around 450–500 °C for the higher-pressure cases, not at the largest temperature. These are calibrated model results, not demonstrated commercial well outputs ([Scott, 2026](https://doi.org/10.1016/j.geothermics.2026.103784)).

The benchmark must therefore track the whole state path: reservoir pressure / temperature / composition / transmissivity → wellbore pressure, enthalpy and phase path → wellhead mass flow and usable heat → surface conversion and parasitics → net useful service. Do not use reservoir temperature as a proxy for plant inlet temperature or power.

The same model identifies wellhead pressure and casing velocity as coupled limits, not afterthoughts. It reports that wellhead pressures above roughly 15 MPa can preserve single-phase production in some modeled regimes, reducing condensation/scaling risk. In its 9⅝-inch casing example, a 50 m/s erosional-velocity reference is exceeded above about 30 MPa reservoir pressure for 450–550 °C conditions. These are specific model conditions, not universal safe operating setpoints or design approvals. The model is steady-state and calibrated against IDDP-1 discharge data; it explicitly does not establish field-scale transient well integrity. The benchmark therefore adds shut-in, circulation interruption, restart, cold injection and thermal-cycle cases. The 2026 well-materials review reports that corrosion may peak in subcritical windows near 300–350 °C depending on chemistry and phase, reinforcing that qualification must track the full transient path rather than only maximum temperature. A second 2026 materials review finds corrosion can be most severe in the subcritical region around 300–350 °C, depending on chemistry and phase state; material qualification must include transients and phase crossings, not merely the maximum-temperature rating ([Meng et al., 2026](https://pangea.stanford.edu/ERE/db/IGAstandard/record_detail.php?id=38347)).

## Model ownership and integration

| Responsibility | Existing owner to compose |
|---|---|
| End-to-end energy-system graph and architecture search | [Energy Systems Studio #2721](https://github.com/Luminous-Dynamics/symtropy/issues/2721) |
| Dynamic pinch / temperature-grade dispatch | [THERMAL-DYNAMIC #1916](https://github.com/Luminous-Dynamics/symtropy/issues/1916) |
| Thermal-store state trajectory | [ENERGY-THERMAL-STATE #2683](https://github.com/Luminous-Dynamics/symtropy/issues/2683) |
| Mine/cavern store, water, flow and geochemistry | [MINE-THERMAL-STORE #4070](https://github.com/Luminous-Dynamics/symtropy/issues/4070) |
| Fusion solver receipts | [PLASMA-TWIN #1703](https://github.com/Luminous-Dynamics/symtropy/issues/1703) |
| Fusion heat-exhaust boundary | [Fusion plant bridge #5851](https://github.com/Luminous-Dynamics/symtropy/issues/5851) |
| Fusion net electric covariance / uncertainty | [FUSION-NET-POWER-UQ #5400](https://github.com/Luminous-Dynamics/symtropy/issues/5400) and [covariance bridge #5333](https://github.com/Luminous-Dynamics/symtropy/issues/5333) |
| Carrier-neutral energy transport | [POWER-FABRIC #5832](https://github.com/Luminous-Dynamics/symtropy/issues/5832) |
| Next-generation geothermal access | [Symthaea GEOTHERM-NEXT #6147](https://github.com/Luminous-Dynamics/symthaea/issues/6147) |
| Symbiotic campus profile | [Symthaea ENG-SYMBIOSIS #6146](https://github.com/Luminous-Dynamics/symthaea/issues/6146) |
| Lattice-confinement fusion anomaly research | [Spark Engine FUSION-E2E #43](https://github.com/Luminous-Dynamics/spark-engine/issues/43) |

Symtropy's existing energy-systems crate is a deterministic, 0D energy-conversion / evidence layer. It is not a reservoir simulator or thermal-hydraulic solver. The fluid crate is not presently a qualified high-pressure geothermal CFD model. Use external domain solvers and keep the adapter boundary explicit.

## First implementation slice

The first frozen fixture under [benchmarks/geothermal-fusion/v0.1](../../../benchmarks/geothermal-fusion/v0.1/) is intentionally small:

- A synthetic 200 °C geothermal heat source supplies a stated thermal duty.
- A defined share goes to a direct-heat service and a separate share to an ORC-like power conversion block.
- Gross electric output and all geothermal / power-cycle parasitics are separated.
- A synthetic facility base load plus a short pulse is supplied by geothermal net electricity and a bounded electrical buffer.
- Every joule is reconciled on the declared boundaries.
- The facility load is explicitly synthetic and is **not** a measured or representative tokamak/fusion-plant load.
- The fixture has no fusion plasma model, no fusion heat source and no South African site data.

This proves only arithmetic and interface semantics. It does not demonstrate geothermal feasibility, net fusion power, a viable sCO₂ cycle, or hardware performance.

## External baselines — use rather than replace

- [NREL SAM geothermal / GETEM](https://sam.nlr.gov/geothermal.html): utility geothermal plant output and lifetime economics for supported resource / conversion types.
- [GEOPHIRES-X](https://github.com/NatLabRockies/GEOPHIRES-X): geothermal reservoir, wellbore, surface plant, lifetime energy and economics.
- [OpenGeoSys TH2M benchmarks](https://www.opengeosys.org/stable/docs/benchmarks/th2m/): coupled process verification against analytical solutions, experiments and other codes.
- [UKAEA PROCESS](https://www.ukaea.org/service/process/): fusion systems code across machine physics, engineering, heat conversion and buildings.
- [NETL STEP sCO₂ pilot](https://www.netl.doe.gov/node/14437): real pilot evidence for sCO₂ equipment / cycle maturity boundaries.
- [ITER external heating systems](https://www.iter.org/machine/supporting-systems/external-heating-systems): reference for plasma heating distinctions and device-specific load claims.

Our differentiation must be demonstrated at the system-integration level: source-preserving data contracts, temperature-grade-aware dispatch, full-boundary ledgers, common-cause fault analysis, calibrated uncertainty and deterministic evidence replay. We should not claim to outperform these tools on their native reservoir, plasma, neutronics or process equations.

## Local South African posture

The Council for Geoscience currently reports preliminary **low-enthalpy** geothermal potential and ongoing magnetotelluric work near known hot springs. This is not a verified superhot geothermal resource. South African scenarios must retain unknown temperature, depth, flow, reservoir, chemistry and cost fields until measured site evidence exists.

## Initial decision rule

A hybrid design is promising only when it delivers more useful, reliable service at acceptable lifecycle cost and environmental burden than the best matched non-hybrid design. Integration complexity is a cost, not a free benefit.

No SOTA, safety, commercial-readiness or physical-plant claim is permitted from synthetic fixtures alone.
