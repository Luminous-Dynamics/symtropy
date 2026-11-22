# EHD second-law energy conversion, flow exergy and irreversibility benchmark (2026)

**Status:** research specification v0.1  
**Tracking:** [EHD-EXERGY-001 #6712](https://github.com/Luminous-Dynamics/symtropy/issues/6712)  
**Related:** [collision energy #6640](https://github.com/Luminous-Dynamics/symtropy/issues/6640), [thrust observability #6641](https://github.com/Luminous-Dynamics/symtropy/issues/6641), [control-volume reducer #6650](https://github.com/Luminous-Dynamics/symtropy/pull/6650)  
**Claim boundary:** specification only. No EHD exergy calculation or physical efficiency claim is implemented.

## 1. Why this is a new layer

The first law asks whether energy is conserved and where it transfers. The second law asks how much work potential is retained and where irreversibility destroys it.

A design may close its first-law energy ledger yet waste much of its electrical input as low-grade heat, mixing, viscous dissipation, electrode losses or radiation. Exergy accounting should identify those channels without replacing the first-law ledger.

The objective is an evidence-carrying, regime-aware exergy ledger that supports robust engineering optimisation.

## 2. Research grounding

- ASHRAE Handbook, Chapter 2, [Thermodynamics and Refrigeration Cycles](https://handbook.ashrae.org/Handbooks/F21/SI/F21_Ch02/F21_Ch02_si.aspx): exergy as useful work potential relative to a specified environment and the physical flow-exergy expression.
- Çengel & Kanoğlu (2025), [Exergy Efficiency of Closed and Unsteady-Flow Systems](https://doi.org/10.3390/e27090943), *Entropy* 27(9), 943. Addresses exergy-efficiency definitions for unsteady systems.
- [A Systematic Approach to Exergy Efficiency of Steady-Flow Systems](https://doi.org/10.3390/e27111108) (2025), describing exergy input/output and destruction.
- [NISTIR 8298](https://doi.org/10.6028/NIST.IR.8298) for CFD V&V/UQ discipline.

These are general thermodynamic foundations. They are not EHD-specific validation results, and no claim is made that exergy analysis is absent from all EHD literature.

## 3. Static thrust is not flight propulsive power

A stationary thrust stand has zero vehicle speed. Therefore T*V_infinity is zero and cannot be used as a static thruster's measured useful propulsion power.

Report distinct quantities:

1. **Static thrust:** calibrated net force, with declared force boundary and drag corrections.
2. **Flow energy/exergy rate:** reconstructed from complete measured mass, velocity and thermodynamic fields over all relevant control surfaces.
3. **Electrical-to-flow conversion:** flow-energy or flow-exergy output divided by the explicitly named electrical input boundary. Do not call this flight propulsive efficiency.
4. **Flight propulsive efficiency:** evaluate only in a qualified moving-platform test, using useful propulsive power and complete onboard energy/power accounting.
5. **Integrated-system performance:** report payload, endurance, total mass, wall-plug/onboard energy, thermal constraints and operating envelope.

Never multiply a static thrust value by an assumed velocity and report the result as measured propulsion power.

## 4. Flow exergy and control-volume balance

For a neutral-gas stream relative to a declared environmental dead state (h0, s0, T0, P0, composition), the standard physical flow-exergy form is:

\[
b_{flow}=(h-h_0)-T_0(s-s_0)+\frac{|\mathbf u|^2}{2}+gz
\]

Add chemical exergy when composition changes and a qualified chemical reference model exists. For plasma EHD this is not a complete exergy ledger: charged species, electric-field energy, surface charge, chemical/excitation states and radiation may require separately defined storage and transport terms.

A generic control-volume balance, with explicit signs, is:

\[
\frac{dB_{CV}}{dt}
=\dot W_{ex,in}
+\sum_j\left(1-\frac{T_0}{T_{b,j}}\right)\dot Q_{in,j}
+\sum_{in}\dot m b
-\sum_{out}\dot m b
-\dot B_{other,out}
-T_0\dot S_{gen}
\]

This is a schema-level form, not a complete plasma-EHD constitutive equation. The actual implementation must pin the control-volume boundary, field-energy storage/flux, electrical and mechanical work, species/chemical terms and radiation terms. No contribution may be counted both as electrical input and as stored field/internal exergy.

Electrical work crosses a boundary as exergy at its work value. Heat-exergy transfer uses the declared boundary temperature and direction. Exergy destruction is:

\[
\dot B_{destroyed}=T_0\dot S_{gen}\geq0
\]

within scale-aware numerical tolerance for a physically admissible irreversible model.

## 5. Work packages

### X0 — Reference-state and system-boundary ontology
- Freeze dead-state temperature, pressure, composition, velocity and elevation.
- Separate electrode-terminal power, converter output, wall-plug input, stored field energy and other work.
- Identify full gas inlet/outlet surfaces, entrainment, leakage, recirculation and wall/electrode heat paths.
- Keep neutral, charged, chemical, field and radiation stores/fluxes distinct.
- Keep bare thruster, converter, cooling system and integrated vehicle boundaries separate.

### X1 — Manufactured exergy oracle
- Reversible reference case with zero destruction.
- Pure kinetic-flow exergy.
- Pressure/thermal stream cases with known dead states.
- Heat transfer at boundary temperature Tb.
- Positive friction/viscous dissipation and non-negative entropy generation.
- Chemical and field-storage cases only after constitutive definitions are pinned.
- Dead-state changes must consistently change physical exergy without changing the first-law energy balance.

### X2 — Plasma and species irreversibility
Connect #6640 first-law channels to:
- ion-neutral drift/frictional dissipation;
- viscous dissipation;
- Joule/electrical losses;
- thermal conduction;
- species diffusion;
- ionization/recombination and reaction affinity;
- charge exchange and excitation/relaxation;
- wall/electrode heat transfer;
- radiation.

Each entropy-generation contribution must derive from the selected constitutive model and its validity envelope. Do not assume one simple formula applies to all non-equilibrium plasma closures. Unsupported terms remain unquantified, not fitted.

### X3 — Measurement-to-exergy reduction
- Reconstruct mass flux, velocity, pressure, temperature and composition across every relevant surface.
- Integrate total flow exergy, not a single exit-plane estimate.
- Use synchronized voltage/current records for real electrical work.
- Propagate calibration, reconstruction, timing and covariance uncertainty.
- Separate measured, simulated, interpolated and derived terms.

### X4 — Exergy loss map and robust optimization
- Attribute destruction to physical mechanisms and components.
- Distinguish destroyed exergy from exported heat/radiation.
- Optimize thrust, flow exergy, T/P, system mass, thermal constraints, noise and operating envelope together.
- Select mitigations by recoverable exergy and measurement confidence, not thrust gain alone.
- Compare single-stage, guide-electrode, multistage, array and ducted concepts only within matched cohorts.

### X5 — Flight and integrated-system reconciliation
- Reconcile force balance, momentum and flow exergy without double counting.
- Measure onboard power, converter losses, energy-source boundary, airframe drag, vehicle speed and mission duty cycle.
- Keep static jet-exergy conversion separate from in-flight propulsive efficiency.
- Include power electronics, supports and thermal-management mass in integrated-system comparisons.

## 6. Adversarial controls

- Static thrust multiplied by an arbitrary speed and labelled useful propulsive power.
- Terminal-electrode power compared with wall-plug power.
- RMS voltage times RMS current substituted for cycle-average real electrical power.
- Incomplete plume boundary omits side entrainment or recirculation.
- Field energy is counted both as electrical input and as stored exergy.
- Friction heat is counted both as useful flow exergy and destruction.
- Exported radiation/heat is labelled destroyed exergy without boundary measurement.
- Dead-state T0/P0/composition changes silently between cases.
- Chemical exergy is omitted when composition changes.
- Negative exergy destruction is accepted by hiding missing flux terms.
- RSS uncertainty is used without validated covariance.
- Different system boundaries or mass bases share one leaderboard.

## 7. Prospective benchmark targets

Targets, not results:
- First-law energy closure remains an independent mandatory gate.
- Exergy-balance residual closes within a predeclared combined uncertainty bound.
- Entropy generation and exergy destruction are non-negative within scale-aware tolerance.
- Component destruction/export terms reconcile with the aggregate exergy balance.
- No static-thrust result is promoted to flight propulsive efficiency.
- Nominal 95% exergy-output intervals achieve 90–98% empirical coverage on preregistered held-out cohorts, reported per cohort.
- Exergy-loss attribution is validated on held-out conditions and independent diagnostics before optimization uses it.
- Unquantified terms remain visible as uncertainty or abstention, never zero.

No universal efficiency percentage is proposed before the physical system boundary and reference cohort are frozen.

## 8. Evidence receipt

A future exergy receipt includes:
- system boundary and dead-state identity;
- solver/model and species/chemistry hashes;
- synchronized electrical waveform and power boundary;
- all mass/energy/exergy surface flux identities;
- thermal boundary temperatures and heat-flux evidence;
- field-energy storage and charge/species ledger identities;
- local/aggregate entropy-generation and exergy-destruction contributions;
- first-law and second-law closure residuals with covariance/bounds;
- calibration, sensor, session and data digests;
- measured/simulated/derived classifications and claim ceiling.

An exergy receipt qualifies the accounting performed; it does not prove physical model completeness.

## 9. Integration and ownership

- #6640: first-law species-resolved collisional energy.
- #6641/#6642: independent thrust observability and electrical/force evidence.
- #6650: control-volume momentum reducer.
- #6660/#6680/#6710: experiment identifiability and design selection.
- #6681/#6682: model discrepancy, uncertainty and predictive abstention.
- Symtropy: system boundary, ledger, provenance and replay.
- Symthaea: identify dominant irreversibilities, propose experiments and challenge attribution.
- Specialist solvers and measurement pipelines: authoritative equations and raw reductions.

## 10. Exit criteria

1. X0 freezes reference-state and system-boundary schemas.
2. X1 passes manufactured reversible, thermal, pressure and kinetic-flow exergy tests.
3. X2 derives entropy-generation terms from source-pinned constitutive models.
4. X3 reconstructs exergy from complete experimental flow/electrical evidence with uncertainty propagation.
5. X4 identifies a held-out dominant loss mechanism and tests a mitigation at matched conditions.
6. X5 separates static-thrust, flow-exergy and flight-propulsion metrics without boundary mixing.
7. No exergy-efficiency or superiority claim is made before independent validation.

**Current state:** research specification only. No EHD exergy model, entropy-generation pipeline, physical exergy measurement or efficiency result is claimed.
