# MNB External-Loop Experiment Protocol and Qualification Contract v0.1

**Status:** protocol design only; no equipment procurement, physical runs, cleaning results, energy savings or product claims.  
**Tracker:** [LAUNDRY-MNB-001 #5489](https://github.com/Luminous-Dynamics/symtropy/issues/5489)  
**Parent:** [LAUNDRY-SERVICE-001 #5471](https://github.com/Luminous-Dynamics/symtropy/issues/5471)  
**Research context:** [Laundry cleaning-intensification screen](./LAUNDRY_CLEANING_INTENSIFICATION_SCREEN_V0_1.md)

## 1. Decision and research question

Prioritize micro-/nanobubble (MNB) wash-liquor conditioning for a bounded external-loop experiment. Do not integrate an MNB generator into a domestic washer before a net-benefit and safety case exists. Conventional washing chemistry and mechanical action remain the control.

The primary question is:

> At matched accepted cleaning, rinse quality and textile gentleness, does MNB conditioning reduce total electrical energy or another declared resource after generator, pump, pressure-drop, heating and control loads are counted?

Separate three questions that must not be conflated:

1. **Mechanism effect:** at identical textile, water, detergent, temperature, time and mechanical action, does MNB change cleaning by stain/fabric class?
2. **Resource substitution:** at the same accepted cleaning/rinse/fabric-care outcome, can MNB permit less detergent, lower temperature, lower mechanical action or shorter time?
3. **Net system effect:** does the complete process use fewer resources per kilogram of accepted dry-equivalent textile after all auxiliary loads, rejected runs, rinse burden and maintenance are counted?

A stain-specific improvement is useful only for a bounded treatment path. Do not force a universal MNB stage if oily stains respond but other soil/fabric classes do not.

## 2. Evidence boundary

The 7 May 2026 textile study reported up to a 5.6-fold improvement in oil-removal efficiency under its tested conditions, across two substrates and five stains. This is a hypothesis-generating literature result, not a household product result. The external-loop protocol must independently reproduce or reject the relevant effect before any integration claim.

The 2024 microbubble flotation study concerns downstream wastewater separation, not wash-bath cleaning. Keep it as a separate later work package. Ultrasound remains a bounded pretreatment hypothesis, not part of the first MNB experiment.

## 3. Stage-gated experiment sequence

### Stage A — generator and fluid-loop characterization (no textile efficacy claim)

Characterize the selected generator and loop before using stained textiles:

- Generator identity, model, configuration, operating settings, gas source, wetted materials, service requirements and manufacturer documentation.
- Electrical input to generator, recirculation pump, heaters and controls/instrumentation, each separately metered and synchronized.
- Gas flow and liquid flow; differential pressure across the generator and full loop; inlet/outlet temperature; water chemistry.
- Bubble-size distribution and number concentration using a documented method, including detection limits and uncertainty. Use an orthogonal characterization method where practical; nanoparticle-tracking analysis alone is not proof that observed scatterers are gas bubbles.
- Dissolved-gas concentration or a declared proxy, with calibration and temperature compensation.
- Start-up, steady-state, shutdown and post-generation time evolution. Record at least 0, 5, 15, 30 and 60 minute characterization points where method capability permits.
- Repeatability across independent water preparations and days; fouling, foam, clogging, air entrainment, gas carryover and pump cavitation observations.

A vendor's "microbubble" or "nanobubble" label is not a measured particle-size distribution. Do not label a run "nano" unless the measurement method, uncertainty, concentration and operational definition support it.

### Stage B — controlled textile/stain screening

Use a non-product external test loop. Keep textile swatches, fluid volume, detergent identity/dose, hardness, temperature, exposure time and mechanical mixing specified. Randomize run order and block by textile/stain batch, water batch and test day. Blind the cleaning measurement analyst to treatment where possible.

The following arms isolate the main effects. The six arms are a minimum design skeleton, not a claim of adequate statistical power. Pilot variance first and preregister confirmatory sample size and analysis before candidate runs.

| Arm | Purpose | Configuration |
|---|---|---|
| C0 | Conventional reference | Declared detergent and conventional mixing; no MNB generator or recirculation module |
| C1 | MNB candidate | MNB-conditioned liquor; all generator and recirculation loads metered |
| C2 | Hydraulic sham | Generator off; same recirculation path, pump, pressure drop and operating time as C1 |
| C3 | Detergent dose control | Conventional reference with a predeclared reduced detergent dose |
| C4 | MNB + detergent reduction | MNB candidate with the same reduced dose as C3 |
| C5 | MNB + lower mechanical action | MNB candidate with a predeclared reduction in mechanical mixing |

C1 versus C0 estimates whole-system performance, not just generator effect. C1 versus C2 helps isolate bubble generation from recirculation/hydraulic effects. C4 versus C3 tests whether MNB changes the detergent dose-response. C5 is only interpretable when mechanical action is measured and the accepted cleaning/rinse/fabric-care floors are preserved.

Do not compare arms with different textile loads, water hardness, stain conditioning, temperature histories or measurement methods and then attribute the difference to bubbles.

### Stage C — matched-outcome resource frontier

After Stage B identifies a repeatable stain/fabric response, test one resource lever at a time. For each candidate point, tune only one of: detergent dose, temperature, time or mechanical action. Keep all other variables fixed and report the resulting Pareto frontier.

The candidate is accepted only if it meets the same predeclared cleaning, rinse and textile-gentleness requirements as the reference. Do not average away a failed stain class or fabric family with strong results elsewhere.

## 4. Measurement contract

### Cleaning and textile outcome

- Stain/fabric identity, supplier/lot, conditioning history, initial reflectance or other validated reference measure.
- Blinded post-treatment measurement and a declared stain-specific cleaning calculation.
- Results by stain class and textile family, with repeat count and uncertainty.
- Textile mass, dimensions, colour, tensile/tear strength and relevant pilling/abrasion measures for repeated-cycle follow-up.
- Rinse residue (including the declared surfactant method), water carryover and any redeposition observation.
- Microfibre release by mass where feasible; distinguish captured material from released material and identify polymer/fibre class before calling all recovered mass plastic.

### Process and resource inputs

- Water mass/volume, water hardness, pH, conductivity and temperature time series.
- Detergent chemistry, exact mass, lot and dosing method.
- Generator electrical energy and power trace.
- Pump electrical energy and power trace.
- Heater energy, control/instrumentation energy and any other auxiliary load.
- Liquid flow, gas flow, differential pressure, run duration and pump operating state.
- Bubble-size distribution, number concentration, method, sampled location and elapsed time since generation.
- Foam, blockage, fouling, leakage, cavitation, noise and maintenance observations.

### Primary and secondary metrics

Primary net-energy metric:

**Complete-loop electrical Wh per kg dry-equivalent textile accepted.**

The denominator includes only textile output that passes frozen cleaning, rinse and fabric-care acceptance. Rejected, aborted and repeated runs remain in the resource/cost ledger and are reported separately; they may not disappear from the denominator without an explicit predeclared exclusion reason.

Report generator Wh, pump Wh, heater Wh and controls/instrumentation Wh separately as diagnostic components. Report the sum as the complete-loop boundary. Do not subtract a component because it is "only an accessory".

Secondary metrics include detergent g/kg accepted textile, water L/kg, elapsed min/kg, stain-specific cleaning, rinse residue, microfibre mg/kg, fabric change, and maintenance burden. Report each independently; do not compress them into one weighted score.

## 5. Calibration, uncertainty and statistical design

Before confirmatory runs:

1. Publish the protocol version, instrument list, calibration status, data dictionary, exclusions, analysis code/version and acceptance margins.
2. Establish meter accuracy and traceability for electrical energy, flow, pressure, temperature, mass and analytical cleaning measurements.
3. Estimate variance using pilot runs that are excluded from confirmatory inference unless explicitly designed and preregistered as a blinded internal pilot.
4. Calculate sample size for the smallest practically meaningful effect and the chosen non-inferiority/superiority claim; disclose assumptions and power.
5. Randomize order within blocks; use independent preparations rather than treating repeated readings of one sample as independent replicates.
6. Preserve raw time series and original instrument exports. Store derived values separately with method/version and input digests.
7. Report effect sizes, confidence intervals, measurement uncertainty, missingness, all exclusions, and per-stain/per-fabric results. Use robust or non-parametric methods if assumptions do not hold.

Non-inferiority margins are not guessed from product targets. Establish them from method repeatability, reference variability, textile significance and the intended claim, then freeze them before unblinding candidate outcomes.

## 6. Safety and containment boundary

This protocol is for a qualified laboratory and a reviewed external test rig, not an instruction to build or modify a mains-powered washer. No person or garment is exposed to an uncharacterized gas, pressure, acoustic, electrical or thermal process.

- No ozone, plasma, solvent, ultrasound or oxidant dosing in the first MNB loop.
- Wetted materials, pressure ratings, relief provisions, electrical protection, isolation, ingress protection, leak detection and safe gas exhaust require competent engineering review before energization.
- No sealed improvised pressure vessels or uncharacterized gas accumulation.
- Keep generator and pump protection independent of Symthaea and ordinary application software.
- Emergency stop, overcurrent, overtemperature, leakage and pressure protection must be deterministic and independently tested.
- Stop immediately on leak, unsafe gas accumulation, abnormal heating, pressure excursion, uncontrolled foam, pump cavitation, unstable instrumentation or material damage.
- Do not reuse treated water on people, food, animals or domestic hygiene applications. Greywater reuse requires its own water-quality, microbiological and local regulatory qualification.

## 7. Decision rules

A candidate advances only when:

1. The measurement system and bubble characterization are repeatable within the preregistered uncertainty budget.
2. Cleaning is non-inferior or better for the declared stain/fabric scope.
3. Rinse and textile-gentleness gates pass.
4. Complete-loop energy is measured and the candidate's net effect is positive within uncertainty for the claim being made.
5. No unacceptable fibre release, foam, fouling, blockage, leakage, cavitation or service burden appears.
6. The effect replicates with independent preparations and, before a product claim, an independent laboratory.

A go at one stage authorizes only the next stage. It does not authorize a domestic product integration or performance claim.

## 8. Research targets (hypotheses, not acceptance defaults)

The first objective is a statistically defensible positive net effect, not a marketing percentage. After pilot variance and control repeatability are known, explore:

- At least 10% lower complete-loop electrical energy per kg accepted textile at matched cleaning/rinse/gentleness.
- Alternatively, at least 20% lower detergent mass at the same accepted outcome without higher energy, water, rinse residue or fabric damage.
- Alternatively, at least 10% shorter elapsed treatment time at the same accepted outcome and without increased total energy.

These are independent exploration hypotheses. They are not additive, guaranteed, or frozen statistical thresholds. A result that improves stain removal while increasing total resources may still be scientifically interesting, but it is not a resource-efficiency win.

## 9. Symtropy, Symthaea, Mycelix and Sol-Atlas

- **Symtropy:** maintain the process boundary, energy/mass/pressure-drop accounting, design-space exploration and evidence-qualified reduced-order models. Do not label a model CFD, cavitation or bubble-population simulation unless it explicitly models and validates those mechanisms.
- **Symthaea:** after a frozen dataset exists, use advisory experiment selection/expected information value to select the next stain, fabric, hardness or operating point. Benchmark it against random selection, fixed rules and conventional design-of-experiments. No actuation authority.
- **Mycelix:** reuse component identity, calibration, supplier, maintenance and repair evidence. Do not create a parallel parts/provenance ledger.
- **Sol-Atlas:** consume qualified measurements for lifecycle cost, water, material and environmental projections. Unknown burdens remain unknown, not zero.

## 10. References and standards watch

- Ding et al. (2026), textile MNB study: https://doi.org/10.1080/00405000.2026.2668156
- MNB cleaning review (2025): https://doi.org/10.3390/nano15070480
- Kimmel et al. (2023), household ultrasound study: https://doi.org/10.1007/s12053-023-10115-7
- ACS ES&T Water (2024), microbubble flotation: https://doi.org/10.1021/acsestwater.3c00802
- IEC 60456:2024 is the current sixth-edition household washer performance-method reference; it adds cleaning, rinse and gentleness methods. It is not itself a safety standard.
- IEC 60704-1 / IEC 60704-2-4 cover household appliance acoustic-noise measurement; keep noise outside the IEC 60456 cleaning result.
- IEC 60335-2-7:2024 is the current IEC washer-safety edition found in this desk review; IEC 60335-2-11:2024 covers tumble-dryer safety, including heat-pump dryers and flammable-refrigerant provisions. Confirm the current SANS adoption and conformity route before design freeze.
- IEC 61121:2012 remains the IEC household electric tumble-dryer performance publication identified in this desk review; verify its stability/revision status and national adoption at protocol freeze.
- South Africa VC 9008 was amended by Notice 7859 / Government Gazette 55257 on 28 August 2026. The notice specifies Phase 1 effective 12 months after publication and Phase 2 effective 24 months after publication. The resulting dates are 28 August 2027 and 28 August 2028 respectively; verify the official schedule, appliance-specific transition and current consolidated requirements with NRCS before market entry.

Contract status remains protocol_proposed_no_physical_runs; this document records a test plan, not a result.
