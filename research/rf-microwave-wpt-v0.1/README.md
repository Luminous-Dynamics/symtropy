# RF / microwave wireless power delivery benchmark v0.1

**Issue:** [RF-MICROWAVE-WPT-001 #5913](https://github.com/Luminous-Dynamics/symtropy/issues/5913)  
**Parent:** [Power Fabric transport primitives #5832](https://github.com/Luminous-Dynamics/symtropy/issues/5832)  
**Reuses:** [SPECTRA solver selection #3396](https://github.com/Luminous-Dynamics/symtropy/issues/3396), [WPT empirical envelope #3448](https://github.com/Luminous-Dynamics/symtropy/issues/3448), [WPT energy accounting #3472](https://github.com/Luminous-Dynamics/symtropy/issues/3472), [multi-receiver WPT #3473](https://github.com/Luminous-Dynamics/symtropy/issues/3473), [robust WPT dispatch #3559](https://github.com/Luminous-Dynamics/symtropy/issues/3559), and [interval-aware Pareto safety #3453](https://github.com/Luminous-Dynamics/symtropy/issues/3453).

**Status:** literature-grounded benchmark definition and reproduction plan; no physical RF system or SOTA result is claimed.

## 1. Programme thesis

RF/microwave wireless power is not one operating regime. At minimum, keep distinct:
- low-power ambient/dedicated RF harvesting for autonomous sensors;
- rectenna power transfer under directed RF illumination;
- near-field MIMO electromagnetic focusing;
- far-field microwave/mmWave beaming;
- simultaneous information-and-power or backscatter systems.

The proposed paradigm is **complete-chain co-design**:

```text
source DC
  -> power amplifier / RF source
  -> feed network and antenna
  -> propagation path
  -> receiving antenna
  -> rectifier
  -> regulation / storage
  -> useful load service
```

A good antenna gain, RF-to-DC rectifier PCE, or received RF level is not itself a useful energy-service result. Every auxiliary and conversion stage must be visible, and the source and load must be metered at declared planes.

## 2. Source-grounded 2026 cases

### R1 — 5.64 GHz MIMO end-to-end DC-to-DC optimization

Lee, Yoon, Oh and Nam, *An End-to-End DC-to-DC Efficiency Optimization and Design Analysis for MIMO Wireless Power Transfer Systems*, IEEE Transactions on Microwave Theory and Techniques 74(9), 8545–8562 (published 1 September 2026).

- DOI: https://doi.org/10.1109/TMTT.2026.3714844
- Measurement-based PA and rectifier nonlinear models are jointly included in a DC-to-DC optimization.
- A 12×12 transmit / 8×8 receive-array simulation at 0.5 m reports 31% and 48% higher simulated end-to-end efficiency than RF-to-DC and RF-to-RF optimization alternatives, respectively.
- The SCA rank-one recovery procedure retains more than 97% of the SDP bound across reported tested conditions.
- Hardware validation uses a 16×16 TX array and a 5×5 rectenna. The paper reports measured DC-to-DC efficiency tracking simulation over distance and azimuth sweeps.
- **Boundary caveat:** the published summary gives relative simulation improvements but not an absolute measured DC-to-DC percentage. Do not infer or fabricate an absolute measurement. The paper identifies near-field focusing as an important mechanism; do not label this a kilometer-range far-field system.

Sources: https://snu.elsevierpure.com/en/publications/an-end-to-end-dc-to-dc-efficiency-optimization-and-design-analysi/ and https://doi.org/10.1109/TMTT.2026.3714844.

### R2 — 28 GHz far-field mmWave using Cu/Co metaconductor

Lee, Jang and Yoon, *High efficiency far-field mmWave-based wireless power transfer system using Cu/Co metaconductor*, Scientific Reports 16, article 12340 (5 March 2026).

- DOI: https://doi.org/10.1038/s41598-026-42136-4
- A 4×4 patch-array transmitter and receiver at 28 GHz are tested at 20 cm. The article states that this distance meets its far-field criterion for the declared aperture.
- The paper reports 7.5% system efficiency for the Cu/Co metaconductor compared with 0.42% for an equal-thickness solid-Cu counterpart at 20 cm.
- Measured rectifier RF-to-DC efficiency at 10 dBm is 70.6% for Cu/Co and 63.7% for solid Cu.
- The paper reports 81% size/weight reduction under its stated comparison.
- **Critical boundary caveat:** the system-efficiency calculation assumes an 85% DC-to-RF stage. The authors explicitly state that the DC-to-RF converter is not included in the prototype. The reported 7.5% is therefore not measured wall-DC-to-useful-load efficiency. The PA / DC-RF stage must be measured before a complete source-to-load efficiency can be claimed.

Sources: https://www.nature.com/articles/s41598-026-42136-4 and https://doi.org/10.1038/s41598-026-42136-4.

### R3 — 3.5 GHz 5G-band rectenna for low-power IoT

Ben Hamadi et al., *New design of a high-efficiency rectenna for wireless power transfer in 5G applications*, Scientific Reports 16, article 12573 (8 March 2026).

- DOI: https://doi.org/10.1038/s41598-026-43603-8
- A compact FR-4 patch plus Schottky rectifier is designed for the 3.5 GHz 5G band and a 5 kΩ reference load.
- The prototype reports 0.91 V measured DC output at 0 dBm RF input; the paper compares this with a higher simulated output of 1.46 V.
- The work emphasizes dependence of RF-to-DC behavior on input power, load resistance and frequency; it targets low-power IoT rather than high-power long-range beaming.
- **Boundary caveat:** preserve the RF input power, measured voltage, load and figure/run identity separately. Do not convert the abstract values into a system efficiency unless the exact paired conditions and measurement planes are resolved from the primary data.

Sources: https://www.nature.com/articles/s41598-026-43603-8 and https://doi.org/10.1038/s41598-026-43603-8.

## 3. Regime classification

Every benchmark record must state:
- carrier frequency and waveform / occupied bandwidth;
- largest effective radiating aperture (D), wavelength (lambda), range (R), array geometry and element spacing;
- the criterion used to classify reactive near-field, radiating near-field / Fresnel, or far-field / Fraunhofer behavior;
- model validity and boundary conditions.

For a common far-field screening reference, report (R_F=2D^2/\lambda) when its assumptions are appropriate. Do not let this screening equation replace a field-model applicability review.

**R1** is a near-field-focused MIMO array example at 0.5 m.  
**R2** is reported as far-field at 28 GHz and 20 cm for its declared array.  
**R3** is a low-power 3.5 GHz rectenna / harvesting case.

No ranking is valid across these without matching the useful service, receiver aperture, incident power level, duty cycle and source/load boundaries.

## 4. Required measurement planes

Preserve the following as separate measured or explicitly unknown quantities:

1. DC source input to PA / RF source.
2. PA DC input and RF output.
3. RF power at TX antenna feed.
4. Feed-network insertion loss, antenna efficiency, gain, pattern and EIRP.
5. Beam shape, sidelobes, polarization and incident power density across the receiver plane.
6. RF power captured at RX antenna port.
7. Rectifier RF input and DC output under the same load and interval.
8. DC regulation / storage input-output and stored-energy change.
9. Useful load service (e.g. measured load Wh, sensor work/duty service).
10. All sensing, tracking, communication, cooling, control, conversion and standby power.

The whole source-to-useful-load result must integrate synchronized energy over the same time window. Do not substitute RF-to-DC efficiency for DC-source-to-load efficiency. Unknown PA efficiency remains unknown, not a default.

## 5. Benchmark protocol

### Hardware / scenario stratification

Freeze and report:
- frequency, waveform, modulation, occupied bandwidth and crest factor;
- TX / RX aperture, element count, pitch, feed topology, material stack and manufacturing revision;
- distance and field regime;
- receiver pose, azimuth/elevation, polarization and motion;
- incident RF power, EIRP, power density and exposure assessment;
- rectifier input power, load, impedance matching and temperature;
- environmental attenuation, blockage, reflections and path availability;
- source/load meter planes, instrument calibration, timebase and uncertainty;
- hardware/software/configuration digests and raw data locators.

### Baselines

Compare:
- fixed single beam and fixed phase/amplitude;
- measured-link-gain maximization;
- RF-to-RF optimization;
- RF-to-DC optimization;
- full DC-to-DC optimization including PA and rectifier nonlinearities;
- fixed equal split / round-robin for multiple receivers;
- greedy instantaneous efficiency;
- robust service-constrained multi-receiver dispatch using #3559 where the physical system matches;
- exact exhaustive small-instance oracle for algorithmic optimality-gap measurement.

Keep the best published hardware baseline distinct from an algorithm-only comparator. A new policy on a different aperture or rectifier is not a fair replacement benchmark.

### Required outcome vector

- total DC source Wh and useful-load Wh;
- PA DC-to-RF, propagation/capture, RF-to-DC, regulation and end-to-end efficiencies separately;
- receiver DC W and service completion over the declared duty cycle;
- EIRP, aperture, power density and distance;
- rectifier output vs incident power, load, frequency, waveform and temperature;
- P50/P95/P99 azimuth/elevation/polarization/mobility degradation;
- multi-receiver spatial uniformity, worst-receiver service, starvation and handover interruption;
- feed-network loss, thermal rise, material mass, size, cost and maintenance where source-grounded;
- model-to-measurement error, uncertainty, energy balance and repeatability;
- hard RF exposure, EMC, thermal and unauthorized-receiver outcomes.

No single weighted scalar can compensate for a hard exposure, thermal or authorization violation. Keep non-dominated candidates and unresolved candidates visible.

## 6. Proposed research targets

Targets are programme hypotheses, not standards or achieved results.

| Lane | Reproduction gate | Research target |
|---|---|---|
| R1 MIMO 5.64 GHz | Reproduce source-reported 16×16 / 5×5 distance and azimuth response; obtain absolute paired measurement records | Improve measured source-DC-to-load energy by ≥10% relative to fixed-beam and RF-to-DC baselines on the same hardware, without worse worst-receiver service or constraints |
| R2 28 GHz far-field | Reproduce the reported 20 cm Cu/Co vs solid-Cu transfer and rectifier results; measure the DC-RF stage rather than assume 85% | Achieve ≥10% measured DC-source-to-useful-load efficiency at the same distance and declared aperture, with all PA and auxiliary consumption included |
| R3 3.5 GHz IoT | Reproduce measured 0.91 V / 0 dBm / 5 kΩ reference with full load and frequency curves | Improve usable net harvested energy per incident RF joule at low input powers while retaining low-cost FR-4 manufacturing and broader angular coverage |
| Multi-receiver RF | Reproduce fixed-beam, equal split and greedy baseline under the same array and receiver set | Zero unauthorized activations; reduce worst-receiver unmet service and improve spatial uniformity under receiver movement and channel uncertainty |

The 10% target on R2 is a proposed stretch target; the existing 7.5% figure is not a complete measured DC-source-to-load baseline. It must not be used as if it were one.

## 7. Safety and system boundaries

- RF exposure limits and assessment method must be selected for the exact frequency, waveform, duty cycle, geometry and jurisdiction. Do not hard-code universal exposure values.
- RF exposure and unauthorized receivers are hard gates, not objectives to trade against efficiency.
- A software beamforming recommendation cannot energize or steer a physical transmitter.
- High-power RF hardware testing requires a separately approved, competent-laboratory safety protocol, interlocks and independent stop authority. This document authorizes no physical RF experiment.

## 8. Integration with Power Fabric

- #5832 owns the carrier/mechanism/medium/useful-service taxonomy.
- #5900 owns metadata-level matched-service comparison; it does not compute efficiency.
- #3396 owns regime-aware EM solver selection and cross-solver verification.
- #3448 owns the empirical WPT source register and stratified experimental envelope.
- #3472 owns source-to-load energy accounting and meter planes.
- #3473/#3559 own multi-receiver WPT interfaces and robust dispatch.
- #3453 owns interval-aware feasibility and Pareto evaluation.
- Symthaea may propose candidates and policies over qualified input; it does not self-certify physics or override independent safety authority.

No second EM solver, energy ledger, service graph or WPT optimizer is introduced.

## Claim ceiling

This artifact supports literature normalization, source reproduction planning and benchmark design only. It establishes no new RF transmitter, long-range power network, efficiency superiority, safety certification or SOTA result.
