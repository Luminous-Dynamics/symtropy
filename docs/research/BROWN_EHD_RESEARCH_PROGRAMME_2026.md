# Biefeld–Brown / Townsend Brown: Evidence-Gated Research Programme

**Status:** Research protocol v0.1  
**Date:** 20 November 2026  
**Companion tracking:** [Symtropy #6462](https://github.com/Luminous-Dynamics/symtropy/issues/6462) · [Symthaea #13026](https://github.com/Luminous-Dynamics/symthaea/issues/13026)

## Executive position

The Biefeld–Brown effect is not one hypothesis. It is a family of claims that must be separated:

1. **Atmospheric EHD propulsion:** high-voltage asymmetric electrodes can produce thrust through corona ions transferring momentum to neutral gas. This is established engineering physics.
2. **Residual thrust at reduced pressure or vacuum:** a distinct claim requiring complete control-volume momentum accounting and direct measurement of residual gas, outgassing, charge transport, radiation, feedthroughs and apparatus interactions.
3. **Electromagnetic–gravitational coupling:** a distinct fundamental-physics claim requiring an explicit equation set and quantitatively testable prediction.
4. **Time-varying-field residuals:** a bounded experimental domain not settled by steady-field tests; not evidence that an anomaly exists.

The programme is designed to reproduce conventional explanations first, then give any residual claim a fair, stringent, falsifiable test. It does not assume Brown was right or wrong in advance.

## What the current evidence supports

| Evidence | What it supports | What it does not establish |
|---|---|---|
| NASA/CR-2004-213312 (2004), *Asymmetrical Capacitors for Propulsion* | Tested asymmetric-capacitor observations consistent with ion drift and momentum transfer to surrounding air | Vacuum propulsion or a new physical principle |
| Talley, USAF report ADA197537 (1988) | A reported residual in one apparatus, with a direction opposing both electrical-wind force and Brown's proposed vacuum force | Antigravity or Brown's claimed mechanism |
| Tajmar et al., Scientific Reports 14, 19427 (2024) | High-vacuum steady-field tests with null force/torque results down to the nN / nN·m scale for the tested configurations | A universal exclusion of all possible time-varying interactions |
| Applied Sciences 15(16), 9190 (2025) | Modern atmospheric electrode-geometry, current and flow measurements | Direct evidence of gravity coupling |
| Actuators 15(6), 326 (2026) | Closed-loop control and electrode placement comparisons for an EHD actuation system | Untethered flight or anomalous propulsion |

Sources:
- NASA report: <https://ntrs.nasa.gov/citations/20040171929>
- Talley report: <https://apps.dtic.mil/sti/citations/ADA197537>
- Tajmar et al. (2024): <https://doi.org/10.1038/s41598-024-70286-w>
- 2025 geometry study: <https://doi.org/10.3390/app15169190>
- 2026 control study: <https://doi.org/10.3390/act15060326>
- Thrust metrology recommended practice (2017): <https://doi.org/10.2514/1.B35459>

A critical nuance: the 2024 steady-field experiment motivates careful questions about dynamic regimes because it does not cover every time-varying configuration. This is a boundary on the experiment, not positive evidence for a time-varying anomaly.

## Paradigm shifts

### 1. From “does it levitate?” to a closed momentum ledger

Treat the whole apparatus, its electrical feed, surrounding gas, chamber and emitted fields as a control volume. Track mechanical force, gas momentum flux, electromagnetic stress / momentum flux, radiation, charge transport, cable and support interactions. A balance reading alone is not a mechanism.

The model should predict the sign, magnitude and scaling of all ordinary effects before residuals are interpreted.

### 2. From one “Biefeld effect” to regime-qualified cohorts

Atmospheric corona, glow / streamer regimes, reduced-pressure discharges, high vacuum, steady fields and pulsed RF are different physical cohorts. Do not interpolate between them or compare scores across them without an explicit transformation and validity argument.

### 3. From a favourite theory to a prediction-feasibility gate

Every candidate theory must predict an observable for a specified apparatus. If its maximum permitted signal is below the instrument floor, it is not an actionable experiment candidate. If it lacks equations or parameter bounds, it is NotEvaluable—not a reason to invent a model.

### 4. From solver output to independently qualified evidence

A completed simulation is not a measurement. A matching hash proves artifact identity, not physical accuracy. Model provenance, numerical convergence, applicability envelope, uncertainty and independent measurement correspondence remain separate qualifications.

### 5. From parameter hunting to decision-focused experiment design

Choose interventions for their ability to distinguish surviving models, including correlated systematic errors, independent-lab replication cost and the possibility that every current model is wrong. Reuse Symthaea's existing OED/oracle infrastructure; do not create another optimizer.

### 6. From peak thrust to a propulsion-system Pareto frontier

A meaningful EHD improvement must consider thrust-to-power, thrust density, system mass, payload fraction, endurance, noise, ozone / NOx, thermal load, reliability and manufacturing—not thrust alone. Compare only at matched pressure, gas, geometry class and full electrical-power boundary.

## Candidate model hierarchy

| Tier | Model | Role and admission condition |
|---|---|---|
| M0 | Maxwell + charge transport + EHD / neutral-gas flow | Mandatory baseline for atmospheric devices |
| M1 | Electrostatic / magnetostatic / MQS / full-wave EM formulations | Select by regime through Symtropy SPECTRA-SOLVER; no universal “Maxwell solver” |
| M2 | Fluid discharge / drift-diffusion / species chemistry | Only where assumptions and data support the discharge regime |
| M3 | Kinetic PIC / Vlasov methods | Use for kinetic regimes and where fluid assumptions fail; preserve Gauss / charge structure |
| M4 | Born–Infeld / Bopp–Podolsky / Euler–Heisenberg / axion electrodynamics | Consider individually, with equations, parameter bounds and device-scale predictions |
| M5 | Explicit nonminimal EM–gravity coupling | Require a covariant action, parameter constraints, predicted force/torque and consistency with precision tests |
| M6 | Historical qualitative “electrogravitic” statements | NotEvaluable until a quantitative model is supplied |

Do not infer a propulsive force from the name of a theory. A nonlinear electromagnetic correction, an axion coupling and a curvature coupling are different theories with different observables and applicability limits.

The EHD baseline should include, as required by the chosen regime, electrostatic / electromagnetic fields, charge continuity, ion mobility and collisions, ion-neutral momentum transfer, neutral-fluid response, thermal state and measured input power. The familiar scaling estimate relating current, gap and ion mobility is a useful sanity check, not a substitute for a full solution or a universal law.

Symtropy's existing rigid-body / N-dimensional engine is not a plasma solver. It should own scenario identity, replay, geometry/run binding, model receipts and qualified result projection. External specialist solvers own the equations and numerical solution. Symthaea owns literature synthesis, model discrimination and experiment selection. Do not map Φ / consciousness metric couplings into plasma or gravity laws.

## Experimental lanes

| Lane | Question | Essential controls |
|---|---|---|
| A. Atmospheric baseline | Can conventional EHD thrust and flow be reproduced? | Corona/discharge state, gas species, humidity, pressure, polarity, geometry, current, thermal effects |
| B. Pressure transition | How does the effect scale as neutral-gas density and residual species change? | Pressure calibration, residual-gas composition, outgassing, leakage, discharge transition |
| C. Steady-field vacuum comparison | Can a published null configuration be reconstructed closely enough to be comparable? | Battery / isolated supply, magnetic shielding, cable/feedthrough force, thermal drift, balance floor |
| D. Time-varying residual | Does a bounded dynamic hypothesis predict a force not explained by induction, radiation, dielectric loss or mechanics? | Frozen frequency/waveform bands, phase-resolved measurements, sham loads, orientation and polarity reversals, vibration and thermal monitoring |

This is a research protocol, not a high-voltage apparatus build guide. Physical work requires a qualified high-voltage / vacuum laboratory, interlocks, remote operation and independent metrology review.

## Measurement and metrology

Use independent channels where possible:

- calibrated force and torque;
- voltage and current waveforms measured at the apparatus boundary;
- temperature and thermal gradients;
- absolute pressure and residual-gas composition;
- magnetic field and field orientation;
- vibration / acoustic coupling;
- gas flow and optical discharge diagnostics.

Preserve native timestamps, calibration identity, sample lineage and instrument uncertainty. Do not resample or interpolate away fast transients. Treat below-noise-floor results as bounded observations, not exact zeros.

The force-measurement protocol should explicitly address balance calibration at operating conditions, zero drift, thermal effects, lead stiffness, parasitic electromagnetic forces, vibrations, bandwidth, repeatability and combined uncertainty.

## Benchmark ladder and proposed targets

These are **prospective targets**, not achieved results.

| Gate | Benchmark | Proposed first target |
|---|---|---|
| B0 Source fidelity | Reconstruct each source operating point | 100% of material fields present or explicitly unknown; no cross-row metric mixing |
| B1 Numerical verification | Analytical references, mesh/timestep convergence, charge and momentum closure | Independent evaluator reproduces all reference observables within declared numerical bounds |
| B2 Atmospheric EHD | Held-out geometry / operating-point force and flow prediction | ≤10% prediction error where source uncertainty supports that target; otherwise use the wider honest uncertainty |
| B3 Vacuum metrology | Force floor, drift and null controls | ≤1 nN detectable-force target under a declared bandwidth and averaging policy; no claim until demonstrated |
| B4 Dynamic residual | Preregistered, theory-derived bands and phase-sensitive force data | ≥5σ after systematic and multiple-comparison treatment, ≥3 independent sessions, then independent-lab replication before a new-physics interpretation |
| B5 Engineering | Matched-condition EHD Pareto frontier | ≥15% gain on a preselected Pareto dimension over a matched reference, without violating other declared constraints |
| B6 Scientific transfer | New apparatus / gas / geometry / session | Held-out cohort generalisation with calibrated uncertainty and abstention outside the supported envelope |

A null result is useful: report an upper bound on the coupling / force parameter over the tested domain. A significant exploratory frequency peak is not a discovery until it survives pre-registration, multiplicity correction, systematic-error analysis, negative controls and independent replication.

## How to beat the state of the art responsibly

Modern EHD studies report gains from electrode geometry and guide-electrode changes, while 2026 work is adding closed-loop control and integrated sensing. Those are the appropriate engineering comparators—not historical claims of antigravity.

To improve on them:

1. Reproduce a source point before optimising it.
2. Compare at the same gas, pressure, geometry family, voltage/current regime and wall-plug power boundary.
3. Report thrust, thrust-to-power, thrust density and system mass separately.
4. Optimise the full actuator/control loop, not only static electrode force.
5. Add emissions, thermal, acoustic and endurance constraints.
6. Use robust optimisation across held-out environmental conditions.
7. Publish failed and non-converged designs, not just the Pareto winners.

For new physics, the “state of the art” is experimental sensitivity and systematic-error control. The programme should aim to be unusually transparent and reproducible, not claim superior sensitivity until it has independently demonstrated it.

## Adversarial test set

The minimum frozen negative-control suite includes:

- powered supply with matched dummy load;
- symmetric electrode control;
- asymmetric geometry with discharge suppressed;
- ordinary corona with gas present;
- polarity and apparatus-orientation reversals;
- cable / feedthrough force and current-loop magnetic coupling;
- chamber interaction and residual-gas / outgassing checks;
- thermal transient / convection control;
- vibration and acoustic coupling;
- below-floor injected signal and blank-drift cases;
- incorrect solver-regime selection;
- unsupported theory parameter values;
- stale / missing calibration and non-causal time alignment;
- simulation mislabeled as physical measurement;
- cross-condition leaderboard mixing;
- multiple-comparison / researcher-degrees-of-freedom challenge.

Every control needs an expected disposition. Unknown or non-identifiable cases must return NotEvaluable / Abstain, not PASS.

## Software ownership and implementation boundary

Compose the existing public work:

- Symtropy [PLASMA-TWIN-001 #1685](https://github.com/Luminous-Dynamics/symtropy/issues/1685) and #1703 for solver-neutral runs and receipts.
- Symtropy [SPECTRA-SOLVER-001 #3396](https://github.com/Luminous-Dynamics/symtropy/issues/3396) and #3374 for regime-aware EM and field-observation qualification.
- Symtropy #6188 and #6208 for native time alignment and profile-qualified symmetry transforms.
- Symtropy #3119 / #3132 for structure-preserving momentum checks where the mechanics assumptions apply.
- Symthaea [PLASMA-LOCAL-GAUSS-PIC-001 #12962](https://github.com/Luminous-Dynamics/symthaea/issues/12962), [solver tournament #12976](https://github.com/Luminous-Dynamics/symthaea/issues/12976) and [cohort comparability #12979](https://github.com/Luminous-Dynamics/symthaea/issues/12979) for classical plasma baselines and comparison eligibility.
- Symthaea [BROWN-EHD-DISCOVERY-001 #13026](https://github.com/Luminous-Dynamics/symthaea/issues/13026) for model prediction and decision-focused experimental selection.

Do not create another field solver, unit system, OED engine, generic evidence ledger or measurement ontology. Do not put PDEs in the rigid-body contact solver.

## Phase plan

**P0 — literature and claim ledger:** source table, exact claims, model families, operating-point comparability.

**P1 — benchmark contract:** frozen synthetic reference suite, units / frame / provenance checks, negative controls and refusal dispositions.

**P2 — conventional EHD reproduction:** choose one fully documented published case and the correct specialist solver; compare to analytical limits and independent measurements.

**P3 — pressure and force-floor qualification:** only after P2; establish the actual sensitivity / background of the apparatus.

**P4 — dynamic hypothesis feasibility:** compute theory-specific predictions and upper bounds; admit a physical test only if the predicted effect is distinguishable from noise and confounds.

**P5 — independent replication and engineering optimisation:** separate scientific anomaly claims from practical EHD actuator improvements.

## Claim ceiling

Completion of this document or software contracts does not establish a new physical interaction, gravity coupling, vacuum propulsion, antigravity, experimental validation, safety certification or state-of-the-art performance. A simulation receipt qualifies provenance and declared checks—not physical truth. Every numerical result must be tagged as source-reported, independently reproduced, simulated, synthetic, predicted or physically measured.


## State-of-the-art snapshot (20 November 2026)

These are source-specific anchors for reconstruction, not one comparable leaderboard:

| Source / configuration | Reported result | Comparison caveat |
|---|---|---|
| Čurma et al., Applied Sciences 15(16), 9190 (2025), Table 2, (0.08)E0K2, 15 mm | 13.57 kV, 0.62 mA, 8.4134 W, 2.02 m/s flow, 7.64 mN calculated thrust, 0.183427% reported efficiency | Atmospheric conditions: 98,540 Pa, 46.7% RH, 20.5 °C. The paper's `F_real` is derived from flow velocity and cross-section, not a direct balance-thrust reading. |
| Čurma et al. (2025), Table 5, (0.08)E1K1, 45 mm | 27.3 kV, 0.03 mA, 0.819 W, 1.49 m/s, 4.157 mN calculated thrust, 0.756232% reported efficiency | A different geometry and operating point from the 7.64 mN result. Do not combine peak thrust with peak efficiency. |
| Nonaka et al., IEEJ Journal of Industry Applications 15(4), 675–682 (2026), guide electrode | Maximum thrust improvement reported as 36.6%; authors also report an improved thrust-to-power ratio | Relative improvement is configuration-specific; rebuild the exact baseline and power boundary before comparison. |
| Jiang et al., Scientific Reports, published online 7 November 2026, DOI 10.1038/s41598-026-78941-0, sub-unit / sub-unit-plus electrodes | At 22 kV: conventional 50.23 mN; sub-unit 64.09 mN (+27.59%); sub-unit-plus 68.70 mN (+36.77% vs conventional) | The paper explicitly notes a trade-off between absolute thrust and thrust-to-power ratio. It is not directly comparable with the 2025 study. |
| Tao et al., Nature Communications 17, 9530 (8 August 2026), 36.7 mg ion-propelled microrobot | 5:1 thrust-to-weight; 1 h tethered hover; reported ~0.45 W at 4.5 kV / 0.1 mA and ~2.8 mN/W; IMU closed-loop attitude control | Tethered power / support remains part of the system boundary; it is not a fully self-powered free-flight demonstration. |
| Control-oriented EHD actuation study, Actuators 15(6), 326 (2026) | Three independently driven HV channels and closed-loop electrode-placement comparison | A laboratory control-system benchmark, not a universal flight-performance baseline. |

Detailed source URLs:
- 2025 operating-point tables and measurement conditions: <https://www.mdpi.com/2076-3417/15/16/9190>
- 2026 guide-electrode study: <https://doi.org/10.1541/ieejjia.20250124>
- 2026 sub-unit electrode study: <https://doi.org/10.1038/s41598-026-78941-0>
- 2026 integrated-sensing microrobot: <https://doi.org/10.1038/s41467-026-76462-y>
- 2026 control-oriented actuation study: <https://doi.org/10.3390/act15060326>

### What a credible SOTA comparison must do

1. Reconstruct the exact source geometry and operating point, not just the headline maximum.
2. Distinguish direct balance measurements from thrust inferred through a flow model.
3. Measure power at the same declared boundary. HV output power, driver input and complete wall-plug power are not interchangeable.
4. Report thrust and thrust-to-power together; the largest thrust configuration may not be the most efficient.
5. Include mass, payload, control electronics, cooling, tether / power source, emissions and endurance for system-level claims.
6. Run the incumbent and candidate in the same facility, on the same calibration chain and under matched environmental conditions.
7. State the full uncertainty and replicate across separate builds / sessions.

A meaningful next engineering target is not an unqualified “more thrust.” It is a measured Pareto improvement in thrust-to-power or payload-specific impulse while preserving low noise, acceptable emissions, thermal limits and repeatability. For flight systems, the stronger target is integrated closed-loop control with a progressively reduced external support burden—clearly reporting what remains tethered.
