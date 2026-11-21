# EHD Thrust Observability and Control-Volume Qualification (2026)

**Benchmark:** EHD-THRUST-OBSERVABILITY-001  
**Status:** measurement protocol and machine-readable contract only; no Luminous measurement  
**As of:** 21 November 2026  
**Machine-readable form:** [ehd_thrust_observability_v0_1.json](../../data/research/ehd_thrust_observability_v0_1.json)  
**Tracking issue:** [#6641](https://github.com/Luminous-Dynamics/symtropy/issues/6641)

## 1. Paradigm shift: validate thrust through independent observables

A force balance, a measured velocity field and a coupled solver do not measure exactly the same thing. The strongest test is not to choose one as truth, but to reconcile their physically distinct observables through a frozen control volume and an explicit uncertainty budget.

The target evidence chain is:

1. **Direct force balance:** a calibrated measurement of the thruster/fixture reaction.
2. **Velocity-field reconstruction:** PIV/LDV or another qualified field measurement integrated across a complete control volume.
3. **Control-volume momentum balance:** momentum accumulation, advective flux, pressure traction, viscous traction and known body forces, with the sign convention frozen before data reduction.
4. **Solver prediction:** an independently versioned discharge/neutral-flow prediction over the same boundaries.
5. **Reconciliation:** compare the three estimates within their combined uncertainty. Do not average incompatible measurements into a plausible-looking answer.

## 2. Research baselines

### Poeppe et al., EUCASS 2025

*Experimental Investigation of Electroaerodynamic Propulsion: Velocity-Information-Based Thrust and Self-Induced Drag Estimation* (DOI 10.13009/EUCASS2025-301) studies a wire-to-droplet airfoil EAD configuration using PIV, a load cell and electrical characterization. The abstract reports agreement between velocity-informed thrust estimates and direct load-cell measurements, and compares methods for self-induced drag estimation.

This is a particularly relevant method comparator. It is a specific test configuration, not a universal proof that any single-plane velocity measurement equals thrust.

### Terenzi et al., Scientific Reports 2025

*Atmospheric electroaerodynamic thrusters with grid collectors* (DOI 10.1038/s41598-025-25308-6) reports a direct high-precision balance measurement, with thruster-on/off differencing, 0.1 mN stated accuracy, 20 acquisitions and combined uncertainty reporting. It also demonstrates why electrode geometry has coupled electric and aerodynamic effects.

The correction published 3 April 2026 must be included in any source-data reproduction.

### Polk et al., Journal of Propulsion and Power 2017

*Recommended Practice for Thrust Measurement in Electric Propulsion Testing* (DOI 10.2514/1.B35564) describes thrust stand calibration, repeatability, error sources, systematic bias and uncertainty budgets. Its recommendations are principally for pendulum electric-propulsion stands, but the error-budget discipline generalizes to EHD measurement.

### Integrated microrobot measurement, Nature Communications 2026

*Controlled flight of high-thrust ultralight ion-propelled microrobot with integrated sensing* (DOI 10.1038/s41467-026-76462-y) provides a useful integrated-platform comparator, including shielded balance thrust measurements and calibrated downstream velocity sensing. Tethered/system-powered boundaries must remain distinct from bare-thruster measurements.

These source families have different geometry, flow, atmosphere, power and mass boundaries. Absolute thrust values from them must not be combined into a single leaderboard without a matched test definition.

## 3. Direct force-balance protocol

Required records:
- force-balance type, sensor model, range, resolution, bandwidth and cross-axis sensitivity;
- traceable calibration identity and calibration curve;
- before/after calibration residuals, hysteresis, repeatability and zero drift;
- on/off or modulated differential runs with randomized order;
- high-voltage cable routing, tension and force/torque characterization;
- electrostatic shielding/guarding and grounding design;
- temperature at balance, frame, cable and thruster;
- vibration, acoustic, airflow, buoyancy/convection and electromagnetic interference records;
- raw sensor time series and tare intervals, not only the averaged reading.

Potential biases must be either physically eliminated, measured/corrected with propagated uncertainty, or explicitly left as unresolved systematic uncertainty.

## 4. Velocity-field and control-volume protocol

A point velocity probe is not a control-volume momentum measurement. The benchmark requires a spatially resolved, coordinate-registered field and all relevant control-volume surfaces.

Record:
- PIV/LDV/hot-wire method and calibration;
- image/velocity processing configuration, synchronization, frame rate and uncertainty;
- coordinates, measured planes, interrogation/measurement volume and spatial resolution;
- missing-vector masks, outlier rejection and interpolation policy;
- density, temperature, pressure, humidity and gas composition;
- inflow/outflow and side-boundary treatment;
- recirculation, entrainment, walls, ducts and ambient return flow.

The momentum equation and sign convention are frozen before unblinding:

\[
\frac{d}{dt}\int_{CV}\rho\mathbf u\,dV
+\oint_{CS}\rho\mathbf u(\mathbf u\cdot\mathbf n)\,dA
=\oint_{CS}(-p\mathbf I+\boldsymbol\tau)\cdot\mathbf n\,dA
+\int_{CV}\mathbf f_{body}\,dV
+\mathbf F_{solid\to fluid}.
\]

The sign of the inferred fluid-on-solid interaction is the opposite of the solid-on-fluid term. The implementation must preserve the exact sign convention in the receipt.

Do not report only \(\int_A\rho u^2dA\) at one exit plane as net thrust unless momentum storage, inflow, side flux, pressure traction, viscous traction and body forces are measured or bounded.

Report self-induced drag on electrodes, collector, duct, frame and support separately from the free-jet momentum flux.

## 5. Electrical and system boundary

- Acquire synchronous electrode-terminal voltage/current waveforms.
- Compute real terminal power as \(\overline{P}=T^{-1}\int_0^T v(t)i(t)dt\).
- Do not substitute RMS voltage times RMS current when phase and waveform distortion make it invalid.
- Separate electrode terminal power, high-voltage supply input, converter losses, switching losses, wiring losses and auxiliary loads.
- Record complete mass: electrodes, frame, guide/collector, duct, insulation, converter, cable/support hardware and controls.
- Bare-thruster and integrated-platform metrics are separate cohorts.
- Tethered or externally powered hover is not self-powered free flight.

## 6. Holdout and statistical design

- Hold out entire experimental sessions, geometry families and waveform families.
- Do not use random time samples from the same run as independent holdouts.
- Randomize operating point and configuration order where safety permits.
- Retain repeated baseline/no-voltage and mechanically/electrically matched sham conditions.
- Freeze calibration, sensor processing, PIV masking, control-volume boundaries and analysis code before holdout unblinding.
- Preserve raw files, analysis configuration hashes and run manifest.

## 7. Reconciliation and acceptance gates

The primary gate is uncertainty-aware cross-observable agreement:

\[
|T_{balance}-T_{CV}|\le U_{combined},
\]

where \(U_{combined}\) is derived from the declared covariance and systematic uncertainty budget, not selected after observing the discrepancy.

Required reporting:
- direct force-balance result and expanded uncertainty;
- field-based control-volume result and expanded uncertainty;
- individual momentum, pressure, viscous, storage and body-force terms;
- solver prediction and held-out error;
- difference between independent observables and combined uncertainty;
- unresolved bias/failure disposition;
- terminal real power and complete system-mass boundary.

Suggested prospective targets:
- Force-balance expanded uncertainty below 2% at adequately resolved signal levels.
- Field-integral uncertainty below 5% for a fully covered, adequately resolved benchmark case.
- Cross-observable disagreement within the combined expanded uncertainty.
- Held-out thrust prediction error below 10% only where the measurement/model uncertainty makes that target meaningful.

These are research targets, not current Luminous achievements. An uncertainty target must never override signal-to-noise limits or incomplete boundary coverage.

## 8. Luminous differentiator

The proposed output is an evidence-carrying thrust certificate binding:
- raw force-balance and field data hashes;
- calibration and sensor identities;
- geometry and control-volume definition;
- gas/environment state;
- synchronized electrical waveforms and real power;
- all momentum/traction/body-force terms;
- system mass boundary;
- uncertainty model and covariance;
- solver/model revisions and held-out disposition.

The value is not another thrust stand. It is the independent reconciliation of force, flow and model evidence under an auditable boundary.

## 9. Claim ceiling

This document defines a benchmark protocol. It does not implement a PIV processing pipeline or force estimator, perform a physical measurement, demonstrate force-balance/control-volume agreement, validate a coupled solver, establish a SOTA improvement or qualify an EHD propulsion system.
