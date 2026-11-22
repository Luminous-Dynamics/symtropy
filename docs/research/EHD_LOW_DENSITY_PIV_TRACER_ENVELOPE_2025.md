# EHD low-density PIV tracer envelope — Noelle et al. 2025

**Source:** Noelle, Schubert, Hildebrand & Pfifer, *Particle Image Velocimetry for Ion Wind Thruster Measurement in Low Density Atmosphere*, AIAA SciTech Forum 2025, paper 2025-0473, DOI 10.2514/6.2025-0473.  
**Atlas record:** NOELLE_2025_LOW_DENSITY_IWT_PIV_TRACERS  
**Status:** source-transcribed method cohort; no raw PIV images/vectors imported and no EHD thrust result reproduced.

## Why this cohort matters

This is a direct measurement-method reference for ion-wind propulsion in low-density air. It combines a vacuum wind tunnel, an operating IWT, tracer-material screening, corona on/off comparisons and collector-surface optics. The paper addresses the important gap between conventional atmospheric EHD-PIV and a high-altitude IWT operating in reduced-density air.

The test article uses:
- two gold-covered tungsten emitter wires, 120 μm diameter and 6 mm vertical separation;
- a 40 mm horizontal emitter-to-collector gap;
- an NACA0010 collector with 25 mm chord;
- 100 mm electrically active span;
- positive DC emitter excitation, with 8 kV used during tracer/corona interaction tests.

The pressure range is 1.0 to 0.3 atm. This is a low-density-air wind-tunnel study; it is not a true-vacuum test, a Mars-CO2 discharge experiment or a flight validation.

## Pressure-by-tracer resolution matrix

The paper reports the achievable PIV interrogation-area sizes at a 5 m/s inlet speed and 108 μs pulse separation. Smaller windows give higher spatial resolution. Parentheses denote edge cases where some quantitative data could be extracted but the complete flow field was not available.

| Pressure (atm) | DEHS | Vaporized fog | Incense sticks | Incense cones | SiO2 |
|---:|---:|---:|---:|---:|---:|
| 1.00 | (64)-128 | 32 | (32)-64 | (32)-64 | (32)-64 |
| 0.90 | 128 | 32 | (32)-64 | (32)-64 | 64 |
| 0.80 | 128 | (64)-128 | (32)-64 | 64 | 64 |
| 0.70 | 128 | (128) | 64 | 64 | 64 |
| 0.60 | (128) | unavailable | (64)-128 | 64 | 64 |
| 0.50 | (128) | unavailable | (64)-128 | (64)-128 | 64 |
| 0.45 | unavailable | unavailable | (64)-128 | 128 | no data |
| 0.40 | unavailable | unavailable | (64)-128 | (128) | (64)-128 |
| 0.35 | unavailable | unavailable | (64)-128 | (128) | 128 |
| 0.30 | unavailable | unavailable | (64)-128 | (128) | 128 |

The table must be treated as a source-specific image-resolution result, not a universal tracer ranking. It also reports only three images per condition, with the best image selected for analysis; it is not a large repeated-measures uncertainty study.

The paper's conclusion is that incense sticks and atomized SiO2 were the most useful choices in this facility and pressure range. SiO2 was preferred when flexible seeding was needed during chamber evacuation; incense sticks gave a stable, low-cost source where their emission remained available.

## Particle physics and pressure-specific limitations

### DEHS

The DEHS droplet range is reported as 0.08–4 μm, with most particles around 0.2–0.3 μm. The facility could sustain seeding down to 0.3 atm, but particle density was too low for fine PIV windows in many reduced-pressure conditions. This is a different failure mode from the 2026 atmospheric needle-to-ring study, where DEHS altered the discharge and deposited a dielectric layer. A tracer can be optically/dynamically useful in one regime yet physically intrusive or simply too sparse in another.

### Vaporized fog

Fog gave the smallest interrogation windows at 1 atm, but its particle density deteriorated rapidly during evacuation and its emission stopped around 0.5 atm. A good atmospheric image therefore does not imply a useful low-pressure diagnostic.

### Incense

Incense sticks continued to burn during evacuation, but their smoke emission stopped around 0.5 atm. Sticks and cones had different burn duration and interrogation-window behaviour. Do not combine them into one tracer family without recording the source geometry and burn condition.

### Atomized SiO2

The tested SiO2 particles were 1 μm diameter with density 2.65 g/cm³, compared with 4.23 g/cm³ for TiO2. The authors selected SiO2 because its lower density improved suspension in low-density air and its lower polarizability reduced electric-field interaction relative to TiO2.

## Corona-on/off interaction test

Tracer interactions with the corona discharge were examined at 8 kV and inlet velocities 2.5 and 5 m/s. The 8 kV operating point was deliberately selected to keep ion-wind interference with the imposed free stream low. The larger flow maps near the emitter were nearly identical with the discharge off and on, with a small downstream velocity increase and a secondary emitter-wake feature in selected conditions.

This is a useful control design, but do not treat it as proof of universal tracer passivity:
- the applied voltage was a low-thrust test condition;
- the observed wake depended on the inlet/particle-residence condition;
- a collector shadow blocked part of the illumination plane;
- the source notes that wake oscillations limit reconstruction by simple symmetry reflection.

## Collector optical/electrical co-design

The paper separately tested three collector surface configurations:

| Surface | PIV observation | Electrical boundary |
|---|---|---|
| Shiny aluminum tape | Diffuse scattering and overexposed regions near boundary layer | Conventional reflective collector surface |
| Aluminum tape with black paint | Increased scattering; not suitable for PIV | No electrical-property effect measured at 8 kV, but higher voltage still requires investigation |
| Black aluminum tape | Matte finish and least diffuse scattering; best boundary-layer image clarity in this set | Retains aluminum electrical function without an added coating |

This is a useful engineering insight: collector surface is simultaneously an electrostatic boundary, a flow boundary and an optical boundary. Optimizing one role independently can degrade another. The black-paint electrical check was only at 8 kV and should not be generalized to the higher-voltage operating envelope.

## Measurement-resolution limitation

The smallest tested interrogation window was 32×32 px. The authors found this still insufficient to represent the boundary layer satisfactorily. They estimated a trailing-edge boundary-layer thickness of about 60 px at Reynolds number 2500 and recommended higher focal length/zoom for resolving it.

Thus, “quantitative velocity map obtained” and “boundary layer adequately resolved” are different qualification outcomes. The benchmark must report:
- spatial resolution relative to local gradient and boundary-layer thickness;
- invalid-vector and optical-shadow masks;
- particle density and image correlation quality;
- surface-reflection/overexposure masks;
- tracer/material and corona state;
- pressure, temperature, gas and inlet speed;
- independent calibration and uncertainty.

## Proposed Luminous benchmark: PIV-TRACER-002

These are prospective targets, not achieved Luminous results.

1. Reproduce the source's ten pressure points using the same tracer families and record the minimum interrogation window plus whether full-field extraction is achieved.
2. Add a separate tracer-following test against Pitot or another independent calibrated reference at each pressure; do not treat the source's image-window table as a velocity-accuracy table.
3. Add a corona-off/on comparison at matched inlet velocity, pressure, gas and geometry, with simultaneous current/voltage recording.
4. Add a residence-time series near the emitter to determine whether the observed emitter wake is a tracer-interaction artifact, real flow redistribution, or both.
5. Repeat collector surface comparisons with optical masks and electrical I–V / discharge stability checks at more than one voltage.
6. Hold out a complete pressure regime and a tracer material when evaluating a PIV image-quality classifier or flow surrogate.
7. Do not claim boundary-layer resolution until the interrogation footprint, calibration, shadow mask and measured boundary-layer thickness satisfy a frozen spatial-resolution criterion.

### Qualification scorecard

| Dimension | Required evidence |
|---|---|
| Particle following | Stokes/response evidence and independent Pitot comparison |
| Electrical compatibility | Current–voltage and corona on/off control at matched conditions |
| Pressure stability | Seeding availability and interrogation-window quality across pressure |
| Optical adequacy | Correlation quality, vector mask, shadow/reflection mask and dynamic range |
| Boundary-layer resolution | Interrogation size relative to boundary-layer thickness and local gradients |
| Surface co-design | Optical quality plus electrical-property / I–V stability evidence |
| Transfer | Held-out pressure, tracer and collector-surface cohorts |
| Physical state | Observer-perturbation and post-removal receipts where seeding can alter the discharge |

## Integration with the observer-perturbation layer

This cohort complements:
- Hamdi et al. 2014 for Pitot-referenced tracer selection, Stokes/Archimedes/electrical-mobility considerations and current evolution during seeding;
- Zwolińska-Glądys et al. 2026 for strong DEHS-induced discharge/flow perturbation and electrode deposition in atmospheric needle-to-ring EHD;
- the DPM receipt, which distinguishes an active observer effect from persistent state change and requires an independent baseline plus a post-removal observation.

Do not merge the atmospheric DEHS result and the reduced-pressure IWT table into one universal tracer correction. Their gas density, geometry, pressure, seeding hardware and discharge operating points differ.
