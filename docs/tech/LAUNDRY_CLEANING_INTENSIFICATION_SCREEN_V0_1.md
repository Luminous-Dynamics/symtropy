# Laundry Cleaning-Intensification Research Screen v0.1

**Status:** literature-informed hypothesis ranking and experimental plan. No household-appliance improvement is demonstrated.  
**Tracker:** [LAUNDRY-SERVICE-001 #5471](https://github.com/Luminous-Dynamics/symtropy/issues/5471)  
**Parent benchmark:** [Laundry service system benchmark v0.1](./LAUNDRY_SERVICE_SYSTEM_BENCHMARK_V0_1.md)

## 1. Decision

Prioritize **micro-/nanobubble-assisted wash-liquor conditioning** as the first cleaning-intensification research candidate. Keep conventional drum mechanics and detergent as the baseline. Do not yet add ultrasound, ozone, plasma, or a novel solvent system to the first physical architecture.

This is not a product selection. It is a research-order decision based on recent textile-specific evidence, prototype tractability, and the ability to test it on an external low-risk loop before changing an appliance.

## 2. Evidence screen

### MNB-assisted washing — priority 1, exploratory

A 2026 study evaluated micro-/nanobubbles across soaking, machine washing and spraying, using polyester mesh and cotton duck with five representative stains. It reported up to a 5.6-fold increase in oil-removal efficiency on hydrophobic fabric under its tested conditions. The authors describe an advantage that was strongest early in treatment and at moderate mechanical action. This makes MNBs worth testing, but does not establish broad household wash performance, energy savings, reduced detergent dose, or performance across cotton, wool, elastane, mixed loads and aged stains.

Reference: Ding et al., *Micro and nano bubbles technology for textile cleaning: mechanisms and the impact of multi-dimensional factors on cleaning efficiency*, published online 7 May 2026, DOI [10.1080/00405000.2026.2668156](https://doi.org/10.1080/00405000.2026.2668156).

A 2025 review of micro-/nanobubble technology describes plausible cleaning mechanisms and application constraints, but is a review rather than a matched household-appliance performance demonstration: [Micro-Nano Bubbles: A New Field of Eco-Friendly Cleaning](https://doi.org/10.3390/nano15070480).

**Hypotheses:** improve oily-soil removal at lower wash temperature, lower detergent dose, lower mechanical action, or shorter time. Each must be tested separately and only compared at matched accepted cleaning and rinse quality.

**Main risks:** bubble-generator parasitic energy; pump/head loss; bubble-size and concentration drift; water-hardness and detergent dependence; foaming; gas dissolution/degassing; filter blockage; air entrainment in the pump; sensor fouling; effects on rinsing and wastewater treatment. A vendor's “nanobubble” label is not a measurement. Characterize generator power, gas flow, bubble size distribution, dissolved-gas concentration and operating stability.

### Ultrasonic assistance — priority 2, bounded adjunct

A 2023 study of two household ultrasonic devices found better cleaning than soaking alone but lower performance than conventional washing programmes; it concluded there may be a use case in low-temperature programmes with small textile loads. The paper also describes why ultrasound is difficult to couple into a large laundry bath: cavitation fields, textile-bound air and acoustic attenuation matter.

Reference: Kimmel et al., *Potential of ultrasonics for energy saving in the household washing process*, *Energy Efficiency* 16, 33 (2023), [DOI 10.1007/s12053-023-10115-7](https://doi.org/10.1007/s12053-023-10115-7).

A companion 2023 study developed an experimental design to separate ultrasound, friction, soaking and temperature effects for handheld pretreatment devices. It is useful methodologically, not evidence that a whole-drum ultrasonic appliance is superior: [DOI 10.1177/15589250231217086](https://doi.org/10.1177/15589250231217086).

**Hypothesis:** a localized, removable pretreatment station or small treatment zone may deliver useful stain-specific benefit without attempting to acoustically energize the entire drum.

**Risks:** acoustic attenuation, local heating, cavitation hotspots, material damage, transducer coupling loss, added electrical power, audible/ultrasonic exposure, water sealing and poor performance at full drum scale. Do not infer efficacy from a claimed ultrasound frequency alone; measure acoustic field and delivered intensity.

### Microbubble flotation — priority 3, wastewater-side research

A 2024 ACS ES&T Water bench/pilot study reported over 98 wt% microplastic/microfibre and over 95 wt% surfactant removal under optimized microbubble flotation conditions. That is a promising downstream treatment concept, not a washing-machine filter result. It does not by itself establish energy, sludge, chemical, maintenance or household packaging performance.

Reference: *Removal of Microplastics/Microfibers and Detergents from Laundry Wastewater by Microbubble Flotation*, ACS ES&T Water (2024), [DOI 10.1021/acsestwater.3c00802](https://doi.org/10.1021/acsestwater.3c00802).

**Hypothesis:** a separate, modular greywater polishing cartridge could recover fibres and surfactants where a household water-reuse system is justified. Compare it with a passive filter, settling/filtration and no-treatment control. Measure captured mass, residual effluent, energy, pressure drop, cleaning burden, concentrate/sludge disposal and the safety of any reuse water.

### Ozone, plasma, solvent and waterless methods — defer from first product

Ozone and plasma may be useful in specialist textile-processing contexts, but domestic use introduces oxidation, inhalation, material compatibility, by-product, seal, interlock and service risks. Supercritical CO2 and other solvent-based cleaning systems introduce pressure-vessel, solvent-recovery and installation complexity. Do not put these into the first household washer architecture without a separate safety/regulatory and lifecycle case.

Do not claim “detergent-free”, “waterless”, “sterilizing”, “disinfecting”, or “self-cleaning” from mechanism plausibility. Each claim requires the relevant chemical, microbiological, materials and performance methods.

## 3. First experiment: external recirculation loop

Before modifying a washer, build an external, low-voltage, open test loop with no garment or user exposure to uncontrolled acoustic, ozone, plasma, pressure or thermal hazards.

### Arms

- C0: tap water / declared detergent / conventional mixing baseline.
- C1: MNB-conditioned water, same detergent, temperature, time and mechanical mixing.
- C2: conventional process with reduced detergent, no MNB (dose-response control).
- C3: MNB + reduced detergent, at the same cleaning acceptance target.
- C4: MNB + lower mechanical action, at the same cleaning acceptance target.
- C5: generator-off sham with the same pump and loop pressure drop, to isolate parasitic energy and hydraulic effects.

Do not treat these six arms as a statistically sufficient sample by themselves. Pilot variance first; randomize and block stains, textile swatches, water batches and run order. Use independent preparations and repeated measurements.

### Measurements

- Standardized stain removal, with the same textile/stain conditioning and blinded image/spectrophotometric analysis.
- Cleaning results by stain class and fabric, not a single averaged score.
- Wash-water temperature and time history.
- Detergent mass and chemistry; water volume and hardness.
- Total electrical energy for generator, pump, circulation and controls.
- Flow, pressure drop, dissolved gas, bubble-size distribution and concentration/stability.
- Rinse residue and any detergent carryover.
- Fabric strength, colour, dimensions, pilling and mass loss after repeated cycles.
- Filter/pipe blockage, foam, leakage, pump cavitation and cleaning/maintenance load.
- Microfibre release and downstream treatment impacts.

### Acceptance and stop rules

1. Freeze control, workload, instruments, analysis, sample-size approach and non-inferiority margins before candidate data.
2. A candidate only enters the next stage if cleaning is non-inferior or better and rinse/fabric-care guardrails pass.
3. Count all generator, pump, heating and circulation electricity. No auxiliary load is free.
4. No claim of lower energy or detergent until it has been measured at matched accepted function.
5. Stop for fabric damage, uncontrolled foam, unsafe gas release, leakage, pump cavitation, unstable measurements or unacceptable wastewater effects.
6. If MNB only improves oily stains, use a bounded stain-specific treatment strategy rather than forcing it into every cycle.

## 4. Symtropy / Symthaea integration

- **Symtropy:** begin with a mass/energy/pressure-drop model of the external loop; fit only to measured flow, pressure, power, bubble-characterization and temperature data. Do not represent a reduced-order model as CFD or bubble-population physics unless the model has those mechanisms and is validated.
- **Symthaea:** after a frozen dataset exists, advisory test selection can rank which stain/fabric/water conditions have highest expected information value. It must not control gas-generating equipment or bypass deterministic protections.
- **Mycelix:** record exact generator, pump, sensor and fluid-path identities and calibration/maintenance evidence by reuse of existing component-provenance and service records.
- **Sol-Atlas:** compare measured auxiliary power, detergent, water, fibre discharge, materials and maintenance burden in lifecycle projections. Do not treat desk estimates as physical results.

## 5. Go/no-go evidence ladder

`literature signal → external-loop characterization → controlled textile-swatches → matched household washer pilot → repeated-cycle textile care → independent replication → field deployment`

A pass at one stage only supports that stage's claims. A five-stain benchtop result is not a product-cycle claim. A bubble generator running successfully is not evidence of cleaning superiority. A lower detergent dose is not beneficial if rinse residue, soil redeposition, fabric damage or water treatment burden increases.

## 6. References

- 2026 MNB textile cleaning study: https://doi.org/10.1080/00405000.2026.2668156
- 2025 MNB green-cleaning review: https://doi.org/10.3390/nano15070480
- 2023 ultrasonic household laundry study: https://doi.org/10.1007/s12053-023-10115-7
- 2023 ultrasonic pretreatment method study: https://doi.org/10.1177/15589250231217086
- 2024 microbubble flotation of laundry wastewater: https://doi.org/10.1021/acsestwater.3c00802
