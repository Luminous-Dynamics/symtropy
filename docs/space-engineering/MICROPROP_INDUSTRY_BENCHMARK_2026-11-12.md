# MICROPROP Industry and Mission Benchmark V0.1

**Benchmark:** MICROPROP-INDUSTRY-001  
**Date:** 2026-11-12  
**Evidence boundary:** published-source comparison only; no Luminous hardware result or qualification

## 1. Why the competitive baseline must change

A laboratory micro-thruster result cannot be compared directly with a spacecraft propulsion product. The complete mission service depends on power processing, capacitors, propellant storage/feed, thermal behavior, spacecraft integration, plume interaction, lifetime and flight evidence.

NASA's May 2026 *State-of-the-Art: Small Spacecraft Technology*, Chapter 4, explicitly notes that the public small-spacecraft propulsion literature contains confusing, conflicting, unverified or incomplete claims. NASA introduces **Progress toward Mission Infusion (PMI)** as a complementary evidence classification:
- C — Concept
- D — In-Development
- E — Engineering-to-Flight
- F — Flight-Demonstrated

NASA says PMI is not a replacement for TRL. A flown device is not automatically Flight-Demonstrated if the published record does not show its key capabilities working in the target environment. NASA's Table 4-11 groups pulsed-plasma and vacuum-arc products and states nominal values are transcribed from cited references and should not be assumed independently verified unless published.

References:
- NASA, *State-of-the-Art: Small Spacecraft Technology* (May 2026), Chapter 4, especially §§4.1, 4.4.2, 4.6.2 and Table 4-11: https://www.nasa.gov/smallsat-institute/sst-soa/in-space_propulsion/
- Full report: https://www.nasa.gov/wp-content/uploads/2026/05/soa-2026.pdf

## 2. Published small-spacecraft product baseline

The following values are transcribed from NASA Table 4-11. They are **published source values**, not a matched independent test campaign. Wet mass, nominal power, mission context and technology class are retained. Vacuum-arc thrusters (VAT) and pulsed plasma thrusters (PPT) remain separate categories even though NASA lists them in one table.

| Device | Thrust | Impulse bit | Isp | Total impulse | Wet mass | Envelope | Power | NASA PMI | Notes |
|---|---:|---:|---:|---:|---:|---:|---:|---|---|
| AIS-VAT1-PQ (VAT, not PPT) | 26 µN max BOL at 5 W; 13 µN at 2.5 W; 4.2 µN minimum <1 W | not reported | 87 s | 0.13 N·s | 0.056 kg | 37 cm³ (42×42×21 mm) | 2.5 / 5 W options | D | 40,000 shots reported; vendor index says EOL thrust <1 µN (condition unspecified) |
| AIS-VAT1-DUO (2 parallel VAT modules) | 52 µN max BOL at 10 W; 26 µN at 5 W; 8.4 µN minimum at 1 W | not reported | 87 s | 0.26 N·s | 0.084 kg | 74 cm³ (84×42×21 mm) | 5 / 10 W options | D | 40,000 shots reported; vendor index says EOL thrust <1 µN (condition unspecified) |
| AIS-VAT1-QUAD (4 parallel VAT modules) | 104 µN max BOL at 20 W; 52 µN at 10 W; 16.8 µN minimum at 2 W | not reported | 87 s | 0.52 N·s | 0.177 kg | 148 cm³ (84×84×21 mm) | 10 / 20 W options | D | 40,000 shots reported; vendor index says EOL thrust <1 µN (condition unspecified) |
| Xantus Metal Plasma Thruster | 400 µN | 1 µN·s | 1,774 s | 5,000 N·s | 1.2 kg | 0.53 U | 40 W | E | NASA lists USSF RROCI (not deployed) and USSF EWS |
| Comat Plasma Jet Pack | 150 µN | 20 µN·s | 2,000 s | 120 N·s | 1.2 kg | 0.66 U | 30 W | E | NASA notes launch on ∑yndeo-2; commissioning details not yet reported in the survey |
| CU Aerospace FPPT-1.7 | 500 µN | 165 µN·s | 3,200 s | 24,000 N·s | 3.0 kg | 1.7 U | 96 W | E | NASA lists DUPLEX (2025) |
| Hypernova NanoThruster A XS | 80 µN | not reported | ≥500 s | not reported | 0.8 kg | 0.5 U | 10 W | D | South African product; values remain NASA-cited nominal values |
| PPTCUP | 40 µN | 40 µN·s | 600 s | 44 N·s | 0.28 kg | 0.33 U | 2 W | D | NASA cites qualification campaign literature |
| Neumann ND-15 | **hold out** | 45 µN·s | 2,000 s | 880 N·s | 1.9 kg | 1.5 U | 15 W | F | NASA Table 4-11 thrust column is µN but lists 3.75; suspected unit mismatch, not corrected here |
| Neumann ND-50 | 100 µN | 150 µN·s | 2,000 s | 1,800 N·s | 1.3 kg | 1 U | 50 W | F | NASA lists EDISON and CarbSAR missions |

NASA's footnote marks mass values with † as wet mass and notes values are nominal. The table states data are documented as provided in references and, unless otherwise published, must not be assumed independently verified. Preserve those qualifications in every downstream dataset.

### AIS vacuum-arc unit and lifecycle caveats

NASA Table 4-11's HTML chapter labels the AIS VAT thrust column `mN`, while the same report's PDF text layer labels it `µN`. The current exact-model Applied Ion Systems product pages explicitly give 13/26 µN for the PQ at 2.5/5 W, 26/52 µN for the DUO at 5/10 W, and 52/104 µN for the QUAD at 10/20 W; the manufacturer's listing supports µN as the source-reported thrust unit. The discrepancy remains recorded as a NASA HTML transcription issue rather than erased. The PDF page screenshot could not be retrieved because NASA's PDF endpoint rejected the oversized file; the PDF text extraction itself reports µN.

Primary manufacturer sources: [VAT1-PQ](https://appliedionsystems.com/portfolio/ais-vat1-pq-micro-vacuum-arc-thruster/), [VAT1-DUO](https://appliedionsystems.com/portfolio/ais-vat1-dual-micro-vacuum-arc-thruster/), [VAT1-QUAD](https://appliedionsystems.com/portfolio/ais-vat1-quad-quad-micro-vacuum-arc-thruster/), [AIS product index](https://appliedionsystems.com/products/).

These pages are important for a different reason too: they report a **40,000-shot life**, 87 s Isp, 0.13/0.26/0.52 N·s total impulse and a vendor-index EOL thrust below 1 µN. The EOL measurement condition is not described on the index. The research benchmark must therefore measure per-unit thrust/impulse-bit drift versus shot count, failed shots, total impulse accrued before the service floor, and actual end-of-life definition—not merely quote 40,000 shots as an endurance pass. The manufacturer index and detailed pages also disagree on displayed web prices / lead times; use a dated formal quote, including PPU/integration scope, for cost comparisons.


This is not one homogeneous leaderboard:
- Xantus, Comat, FPPT-1.7, Hypernova and PPTCUP do not necessarily share the same pulse physics, propellant, power boundary or qualification context.
- VAT is predominantly electromagnetic and differs from conventional PPT.
- Missing fields are unknown, not zero.
- Do not compare wet mass with dry mass or infer that a unit fits a spacecraft solely from a U envelope.
- ND-15 thrust remains ineligible for numeric ranking until its original reference is checked.

## 3. Laboratory research frontier — separate source cases

These studies are useful micro-propulsion architecture references, but do not outrank product baselines until measurement, full-subsystem mass/power, duty-cycle, endurance and mission boundaries are aligned.

### 3.1 Micro-PPT flown/used reference

Ou et al. (2024), *Advances in Space Research* 74(4), DOI [10.1016/j.asr.2024.05.039](https://doi.org/10.1016/j.asr.2024.05.039), report a micro-PPT of about 450 g and 90 × 90 × 35 mm, with approximately 1,600 s Isp. The article's abstract and highlights report efficiency differently (>9% versus >10%); preserve the source discrepancy rather than selecting the more favorable number. The paper reports use aboard Hangsheng-1 for attitude control / drag compensation; independent raw mission telemetry and orbit-determination details are not supplied in the present comparison.

### 3.2 Low-power micro Z-pinch frontier

- Zhao et al. (February 2026), *Acta Astronautica* 239, DOI [10.1016/j.actaastro.2025.11.034](https://doi.org/10.1016/j.actaastro.2025.11.034): 1-W-class micro Z-pinch PPT; reported 25.5% propulsion efficiency and 84.9 µN/W thrust-to-power. The paper also reports 95.1% single-cycle energy deposition and >97% electrothermal impulse contribution under its stated configuration.
- Zhao et al. (September 2026), *Aerospace Science and Technology* 176 Part B, DOI [10.1016/j.ast.2026.112161](https://doi.org/10.1016/j.ast.2026.112161): sub-2-J micro Z-pinch PPT; reported 19.0% propulsion efficiency, 72.8 µN/W, 531.9 s Isp at the stated optimum, 92.7% energy deposition and >95% electrothermal impulse contribution.

These are separate laboratory configurations and operating points. They must not be pooled into one device record or interpreted as satellite flight qualification.

### 3.3 Coaxial PPT endurance and correction

Huang et al. (April 2026), *Vacuum* 247, DOI [10.1016/j.vacuum.2026.115131](https://doi.org/10.1016/j.vacuum.2026.115131), report over 100,000 discharges at 18.4 J, 38,000 ignition cycles, and a 37% maximum efficiency headline. After accounting for electrode redeposition, the reported corrected average is 25.6%; impulse-bit estimates vary with discharge waveform and have substantial uncertainty. Preserve maximum, corrected mean, sample count, and mass-loss accounting as separate fields.

### 3.4 RF gridded-ion reference

Chai et al. (2025), *Journal of Applied Physics*, DOI [10.1063/5.0293389](https://doi.org/10.1063/5.0293389), report a 13.56 MHz ICP RF ion thruster with two-grid optics: 1.2 mN thrust, 1,250 s Isp, 100 W RF input and xenon at 1 sccm. This is a distinct acceleration architecture and laboratory configuration; do not compare RF input directly to spacecraft DC bus input without the PPU conversion boundary.

## 4. Paradigm shift: benchmark mission service

Freeze three independent mission profiles and retain separate Pareto frontiers:

1. **Precision pointing / attitude trim:** impulse-bit distribution, torque authority, repeatability, plume disturbance and restart precision.
2. **Drag compensation / orbit maintenance:** average thrust over duty cycle, available bus energy, propellant throughput, lifetime total impulse and orbital maintenance service.
3. **Controlled orbit change:** required Δv for declared spacecraft mass, mission schedule, power/thermal constraints, installed subsystem mass and propellant.

For each architecture, compare:
- mission Δv and total impulse over the mission, under the same spacecraft and environment;
- complete installed mass, separating dry and wet mass, including PPU, capacitors, tank/feed, propellant, harness, thermal structure and any neutralizer;
- bus-side average and peak power, conversion efficiency, energy-storage mass and duty cycle;
- propellant mass consumed and exact Isp denominator;
- per-unit pulse/discharge count, operating hours, ignition success, misfires, restarts and failures;
- impulse-bit mean, distribution and uncertainty rather than only a best value;
- electrode erosion, redeposition, dielectric/feed degradation, drift and service interval;
- plume angle/species/energy, spacecraft torque, contamination of optics/solar arrays/radiators/sensors;
- qualification environment, radiation, vibration, thermal cycling, vacuum, storage and restart after environmental exposure;
- lifecycle cost, production repeatability, replacement burden and ground operations when evidence exists.

## 5. Proposed targets — goals, not achieved performance

The first performance objective is data integrity, not an arbitrary record number. The target ladder is:

- Reproduce at least one published μ-ZPPT and one CPPT point from raw waveforms/calibrated diagnostics before ranking their performance.
- Reconcile thruster-terminal power with bus-side power and separately report PPU + capacitor losses.
- Establish per-unit endurance with all failed pulses and restarts retained; 100,000 pulses is an initial research gate, 1,000,000 a stretch gate after the first is independently reproduced and failure modes characterized.
- Initial impulse-bit repeatability goal: CV ≤10% over a frozen operating point, subject to diagnostic uncertainty.
- Demonstrate stable plume and mass-loss/erosion accounting across lifetime, not only at beginning-of-life.
- For each mission profile, improve mission service per installed subsystem mass and bus energy against the best feasible incumbent without violating pointing, thermal, contamination or lifetime constraints.
- Only claim superiority when a blinded/independent matched comparison has confidence intervals excluding the incumbent under the same system boundary.

Do not target a single universal “highest Isp” or “highest efficiency” ranking. A thruster can win in Isp and lose in impulse bit, power processing, thermal availability, reliability or mission value.

## 6. Evidence classes and admission

Every row must classify evidence:
- author-reported model / analysis;
- laboratory experiment;
- independently reproduced bench result;
- environmental / qualification test;
- flight opportunity / hardware flown;
- flight-demonstrated key capability;
- operational record / repeat production.

Use NASA PMI C/D/E/F as a source field when applicable, but never equate it to an Luminous qualification decision. Flight status without public commissioning and measured key-capability evidence does not become Flight-Demonstrated.

## 7. Adversarial controls

- Merge distinct 1-W and sub-2-J Z-pinch papers into one optimum.
- Compare a maximum CPPT efficiency with a corrected lifetime average.
- Infer per-unit endurance from a programme-wide discharge count.
- Ignore electrode redeposition and thereby misstate propellant use.
- Drop misfires, resets or failed restarts from denominators.
- Compare source RF input with DC-bus input.
- use Isp with inconsistent consumed-mass boundaries.
- compare a lab prototype against a commercial device without PPU and installed-mass normalization.
- assume launch means successful thruster commissioning.
- silently repair NASA ND-15's possible thrust-unit inconsistency.
- rank a device with unknown total impulse as though its value were zero.
- interpret Symtropy replay or Symthaea reasoning as thrust measurement.

## 8. Work status and boundaries

This document is a literature-derived benchmark baseline. It does not provide a thruster design, construction recipe, energized test procedure, spacecraft integration authorization, flight qualification or hardware performance result. The next executable work should reuse the existing analytic propulsion verifier [PR #4220](https://github.com/Luminous-Dynamics/symtropy/pull/4220), not duplicate its rocket-equation implementation.

Related:
- Symtropy [MICROPROP-001 #4912](https://github.com/Luminous-Dynamics/symtropy/issues/4912)
- Symtropy [MICROPROP-INDUSTRY-001 #4930](https://github.com/Luminous-Dynamics/symtropy/issues/4930)
- Symtropy [PROPULSION-QUALIFICATION-001 #4201](https://github.com/Luminous-Dynamics/symtropy/issues/4201)
- Symtropy [PLASMA-TWIN-001 #1685](https://github.com/Luminous-Dynamics/symtropy/issues/1685)
- Symthaea [MICROPLASMA-001 #11780](https://github.com/Luminous-Dynamics/symthaea/issues/11780)
