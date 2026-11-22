# EHD-DPM-001 — diagnostic observer perturbation and passivity qualification

**Status:** manufactured Rust reference and nine tests authored; Rust execution and exact-head CI pending  
**Primary source:** Zwolińska-Glądys, Cardona Murillo & Jaszczur, *Experiments in Fluids* 67, 168 (2026), DOI 10.1007/s00348-026-04322-y  
**Atlas cohort:** ZWOLINSKA_2026_PIV_SEEDING_HUMIDITY

## A paradigm shift: a measurement can change the thing being measured

Conventional uncertainty budgets often treat an instrument as a passive observer: its output has noise, bias and calibration uncertainty, but the underlying system is unchanged.

That assumption fails for some EHD diagnostics. The 2026 needle-to-ring fan study compared DEHS droplets and incense smoke as PIV seeding. At 14 kV, the reported peak axial velocity was 3.8 m/s under DEHS versus 5.9 m/s under incense smoke (36% lower); the integrated flow estimate was 0.89 versus 1.07 L/s (about 17% lower). The authors attribute this to DEHS reducing effective gas-gap conductivity/ionization and forming a dielectric deposit on the collector. They explicitly conclude that seeding particles alter the actual EHD state, not merely the measurement. The dataset is available from the corresponding author on reasonable request.

This is not a normal calibration offset. The diagnostic intervention changes the physical process. A flow field acquired with DEHS is not necessarily an unbiased observation of the unseeded EHD flow.

## What the new reference qualifies

The Rust reference compares three phases under a matched operating-condition identity:

1. **Independent baseline:** a separately qualified reference measurement made without the diagnostic intervention under test.
2. **During diagnostic:** the same operating condition with the diagnostic intervention present.
3. **After removal:** a follow-up after removing the diagnostic intervention, to detect persistent deposition, contamination, hysteresis or other state memory.

For baseline value b, diagnostic value d, and conservative absolute uncertainty bounds Ub and Ud, the absolute observer-effect interval is

\[
E_{\min}=\max(0,|d-b|-(U_b+U_d)),
\qquad
E_{\max}=|d-b|+(U_b+U_d).
\]

The uncertainty bounds are added conservatively. There is no silent root-sum-square assumption.

Given a predeclared maximum allowed absolute effect τ:

- **PerturbationDetected:** E_min > τ.
- **WithinTolerance:** E_max ≤ τ.
- **Unresolved:** the interval overlaps the tolerance boundary.

The post-removal comparison applies the same rule to baseline versus post-removal observation. A missing post-removal measurement never proves recovery. Without a separately qualified baseline, the result is IndependentBaselineRequired.

Independent physical conservation and validation-envelope gates take precedence. A diagnostic perturbation receipt cannot repair a failed charge, mass, momentum or energy ledger, and a good passivity comparison cannot authorize an out-of-envelope prediction.

## Published challenge case versus manufactured reference

The paper reports at 14 kV:
- DEHS peak axial velocity: 3.8 m/s;
- incense-smoke peak axial velocity: 5.9 m/s;
- DEHS flow: 0.89 L/s;
- incense-smoke flow: 1.07 L/s;
- dielectric deposition and a flattened axial velocity profile under DEHS.

The Rust self-check reuses the two peak-velocity magnitudes only to create an inspectable manufactured example. Its uncertainty bounds (±0.05 m/s), allowed effect (0.2 m/s), matched evidence identities and post-removal value (4.0 m/s) are synthetic. They are **not** claimed to be the paper's uncertainty intervals, raw measurements or measured recovery trajectory.

The source reports a 15-minute 8 kV current-stability test (180 readings at 5 s intervals) to study seeding-related discharge stability. The atlas retains this protocol, but the raw series are not available in the committed record.

## Why this changes the engineering workflow

An EHD diagnostic manifest must now carry more than sensor accuracy and calibration:
- diagnostic intervention identity and material;
- particle size, density, conductivity/permittivity and concentration where relevant;
- applied voltage, polarity, waveform, gas, pressure, temperature and humidity;
- baseline and diagnostic acquisition windows and timing;
- electrode condition and contamination/deposition state before and after intervention;
- synchronous current/voltage traces to reveal discharge changes;
- independent reference method and its own uncertainty;
- recovery measurement after removing the diagnostic intervention.

For PIV specifically, report both the optical tracer fidelity and its electrical/physical compatibility with the discharge. A tracer that follows neutral flow well in a conventional wind tunnel can still alter an EHD discharge.

## Benchmark programme

1. Reproduce the DEHS/incense comparison using the source's 11 mm needle-to-ring gap, positive polarity and 4–14 kV sweep; first request the underlying raw PIV vectors and current series.
2. Randomize or counterbalance the diagnostic sequence, with cleaning and electrode inspection between conditions; retain pre/during/post current traces to quantify persistent effects.
3. Compare at least one independent unseeded flow method (hot wire, Pitot or another independently qualified method) with seeded PIV. The source uses hot-wire readings as an additional flow reference; a full-field unseeded baseline remains a desired new test.
4. Expand to diagnostic materials and concentrations, with humidity and voltage as separate experimental factors.
5. Use robust experiment design to select the smallest safe set of interventions that distinguishes particle charging, electrical shielding, deposition, optical image quality and actual fluid-flow changes.
6. Train no flow surrogate on a mixed tracer corpus until each measurement is classified as passive, physically perturbed or unresolved.

## Prospective qualification gates

These are proposed targets, not achieved results:

| Gate | Target |
|---|---|
| Passivity | Upper bound on diagnostic-induced effect is below a preregistered tolerance for every qualified observable |
| Detection | Lower effect bound exceeds tolerance for known intervention positive controls |
| Abstention | Every uncertainty interval overlapping the tolerance yields Unresolved |
| Persistence | No diagnostic intervention is called recovered without a post-removal measurement and independent evidence |
| Reproducibility | Matched voltage/humidity/geometry runs report tracer material, dose, order, cleaning and electrode condition |
| Physical integrity | No metrology or discrepancy result can override a failed independent physical ledger |
| Transfer | Hold out tracer material, electrode geometry and humidity/voltage cohorts, rather than random PIV vectors |

## Competitive implication

This is complementary to standard V&V/UQ and to multivariate validation metrics: the issue is not only whether the measured vector field differs from the model, but whether the measurement process itself changes the physical state being validated.

The research opportunity is an **observer-aware engineering assurance architecture**: diagnostics become versioned interventions with their own state-change and recovery receipts. The system learns what can be measured passively, what can only be measured with a qualified physical correction, and what remains unobservable with the chosen instrument.
