# Space Systems Reference v0.1

Dependency-free Rust arithmetic references for two bounded research questions:

1. **Planetary shade:** equivalent target-plane attenuation area and scenario mass proxy; not a physical L1 shade geometry or installed-mass estimate.
2. **Q-drive:** first-order dynamic-pressure power harvesting, conversion, electric-jet thrust, parasitic drag and net axial acceleration.

These are screening calculators with explicit assumptions. They are not physical simulators or qualification tools.

## Run

\`\`\`sh
cargo fmt --manifest-path tools/space-systems-reference/Cargo.toml -- --check
cargo test --manifest-path tools/space-systems-reference/Cargo.toml
cargo run --manifest-path tools/space-systems-reference/Cargo.toml
\`\`\`

The executable emits a machine-readable JSON report for the frozen synthetic arithmetic cases.

## What the v0.1 shade calculation does

It computes:

\`\`\`
A_earth = pi * R_earth^2
A_eq,target = target_flux_reduction * A_earth / attenuation_effectiveness
M_proxy = A_eq,target * scenario_areal_mass_density
M_local = M_installed * local_material_fraction
M_earth = M_installed - M_local
P_removed = solar_irradiance * target_flux_reduction * A_earth
\`\`\`

The area is an **abstract target-plane attenuation equivalent**, not a physical aperture, L1 membrane footprint, installed system area or guaranteed lower bound. Multiplying it by a scenario areal density produces only an equivalent-area mass proxy. A physical L1 mass estimate requires a finite-Sun optical transfer model, shade-to-target geometry, diffraction/scattering treatment, module arrangement and a complete installed-system bill of materials. The power term is intercepted-flux bookkeeping, not climate forcing or a surface-temperature prediction.

### Frozen illustrative shade case

| Input | Value |
|---|---:|
| Mean Earth radius | 6,371,000 m |
| Solar irradiance | 1,361 W/m² |
| Target flux reduction | 1% |
| Effective aperture factor | 0.90 |
| Installed areal density | 0.01 kg/m² |
| Locally sourced material fraction | 50% |

The 50% local-material fraction and 10 g/m² installed areal density are scenario inputs, not demonstrated lunar manufacturing capability or a selected design.

### Mass sensitivity — why the membrane system boundary matters

For the same 1% target and abstract 0.90 aperture-effectiveness factor, the equivalent aperture remains about **1.4168 million km²**. The equivalent-area mass proxy scales linearly with the assumed *scenario* areal mass density:

| Assumed installed areal density | Implied installed mass | Metric tonnes |
|---:|---:|---:|
| 1 g/m² | 1.4168 × 10⁹ kg | 1.42 million t |
| 10 g/m² | 1.4168 × 10¹⁰ kg | 14.17 million t |
| 100 g/m² | 1.4168 × 10¹¹ kg | 141.68 million t |
| 1 kg/m² | 1.4168 × 10¹² kg | 1.42 billion t |

These are sensitivity calculations, not candidate designs or lower bounds on physical system mass. The target-plane attenuation equivalent is not the L1 membrane area, so supports, joins, power, attitude control, navigation, communications, spares, transfer and repair cannot yet be consistently multiplied or added to this proxy. NASA ACS3's 80 m² deployed sail is a genuine component-demonstration baseline, but its physical sail area is not directly comparable to the target-plane equivalent. The comparison is only a scale illustration; the finite-distance optical transfer is the missing bridge.

## What the v0.1 q-drive calculation does

It evaluates a one-dimensional Newtonian screening chain:

\`\`\`
q = 0.5 * rho * v_rel^2
D_h = q * C_D * A_eff
P_h = D_h * v_rel
P_bus = eta_conversion * P_h
P_conversion_loss = P_h - P_bus
P_jet = eta_thruster * P_bus
P_thruster_loss = P_bus - P_jet
P_h = P_conversion_loss + P_thruster_loss + P_jet
mdot = 2 * P_jet / u_exhaust^2
T = mdot * u_exhaust
F_net = T - D_h * (1 + parasitic_drag_ratio)
a_net = F_net / vehicle_mass
\`\`\`

The model rejects beta > 0.05 for both vehicle/medium relative speed and exhaust speed. This does **not** cover the 0.05c–0.2c relativistic regime discussed in interstellar q-drive literature. Effective area and drag coefficient are abstract fit parameters; they do not derive from a coil geometry or solve collisionless plasma / electromagnetic coupling. The model also omits field-generation and superconducting mass, thermal rejection, power conditioning, thrust-vector misalignment, relativistic transformations, variable medium profiles, orbital dynamics, trajectory optimisation and braking.

## Frozen illustrative q-drive case

| Input | Value |
|---|---:|
| Medium mass density | 1.0 × 10⁻²⁰ kg/m³ |
| Relative speed | 0.05c |
| Effective interaction area | 1.0 × 10⁸ m² |
| Effective drag coefficient | 1.0 |
| Power conversion efficiency | 80% |
| Thruster efficiency | 60% |
| Exhaust speed | 0.043c |
| Vehicle mass | 1.0 × 10⁶ kg |
| Parasitic drag / harvesting drag | 10% |

Every value above is a synthetic input chosen to exercise the arithmetic. It is not a source-backed estimate of the interstellar medium, plasma-magnet interaction area, achievable efficiency, vehicle mass or real q-drive acceleration. In particular, the model's small positive net acceleration is **not a performance prediction**.

### Net-force margin — the important q-drive output

In this synthetic case, the model computes about **1.347 GW** of bus power and **0.808 GW** of jet power, producing **125.41 N** of ideal exhaust thrust. Approximately **123.58 N** is consumed by harvesting drag and the assumed parasitic drag. The residual is only **1.83 N**, or about **1.48%** of the total opposing force. The chosen exhaust speed (0.043c) is close to the model's break-even value (about 0.04364c). Small changes in conversion, thrust-vector alignment, drag coupling, medium density or parasitic loads can erase the positive result.

This is a sensitivity warning, not an estimate of achievable Q-drive performance. A confirmatory model must propagate uncertainty and demonstrate a robust positive lower bound on net acceleration, not merely a positive nominal value.

## Literature and current industry comparators

- Greason (2019), [A Reaction Drive Powered by External Dynamic Pressure](https://tauzero.aero/a-reaction-drive/), *Journal of the British Interplanetary Society* 72(5), pp. 146–152. The paper explicitly says practical assessment still needs conversion-efficiency and parasitic-drag terms, credible subsystem mass estimates and off-axis thrust analysis.
- Larrouturou, Higgins & Greason (2022), [Dynamic soaring as a means to exceed the solar wind speed](https://doi.org/10.3389/frspt.2022.1017442), *Frontiers in Space Technologies* 3. The proposed staged architecture requires pre-accelerating significant reaction mass.
- Matonti et al. (2026), [Roadmap toward a planetary sunshade for space-based solar geoengineering](https://doi.org/10.1016/j.asr.2026.04.007), *Advances in Space Research*. This is a phased technology roadmap, not a flight demonstration of a planetary shade.
- NASA, [ACS3 mission results and lessons learned](https://ntrs.nasa.gov/citations/20260004973). NASA reports deployment of an 80 m² solar sail from a 12U CubeSat in August 2024. This is a useful deployed-sail baseline, not a large-area membrane manufacturing baseline.
- ESA, [Proba-3 achieves precise formation flying](https://www.esa.int/Enabling_Support/Space_Engineering_Technology/Proba-3_achieves_precise_formation_flying). ESA reports millimetre-class alignment for two spacecraft approximately 150 m apart for several hours. It does not demonstrate autonomous control of thousands or millions of units near L1.
- NASA, [Lunar Surface Technology](https://www.nasa.gov/lunar-surface-technology/) and [NASA TechPort ISRUPower-TP](https://techport.nasa.gov/projects/146991). These describe lunar-regolith-simulant processing and maturation of ISRU process technologies; they do not establish lunar field production of flight-ready shade membranes.
- NASA, [2026 Small Spacecraft State-of-the-Art — In-Space Propulsion](https://www.nasa.gov/smallsat-institute/sst-soa/in-space_propulsion/). Its Hall-thruster table includes JPL MaSMi (55 mN, 1,920 s, 1,000 W, 3.4 kg) and Orbion Aurora (15.6 mN, 1,280 s, 300 W, 1.5 kg). NASA notes that small-spacecraft survey ranges do not cover all capabilities and that public performance/maturity data can be incomplete or conflicting.

### Why the NASA Hall-thruster figures are not a direct Q-drive comparison

For this synthetic q-drive case, the ideal thruster stage produces about 125.4 N from 808 MW of jet power (about 0.155 N/MW) at an assumed exhaust velocity of 0.043c. JPL MaSMi's listed 55 mN at 1 kW is about 55 N/MW, but its 1,920 s specific impulse corresponds to a radically lower exhaust speed. The Q-drive arithmetic case trades very high specific energy / low mass flow for extraordinarily high power and includes an external-medium drag penalty. These figures cannot establish relative superiority: a fair benchmark must match exhaust velocity, propellant mass, power source, complete system mass, environment, vehicle state, braking and mission time.

## Benchmark strategy

Do not claim better-than-SOTA from a toy equation. Compare with matched boundaries and preserve distinct evidence classes.

### Shade

- Effective radiative service per **installed-system** kilogram, including membranes, seams, booms, power, controls, relay, spares and repair equipment.
- Complete installed kg per m² of physical L1 membrane area, after the target-plane optical transfer and lifecycle bill of materials are qualified.
- Accepted membrane area per manufacturing kWh and per year, including feedstock preparation, rejected batches, inspection, assembly and replacement.
- Local-material fraction after complete process-yield, consumables, equipment and reject accounting—not input regolith fraction.
- Deployment success, module fault tolerance, collision risk, station-keeping and autonomous repair service.

### Q-drive

- Net acceleration after harvested drag, parasitic drag, conversion loss and thruster loss.
- Full mass and power boundary: interaction field generator, superconductors/cryogenics, tether, storage, power electronics, thruster, propellant and thermal rejection.
- Energy and momentum closure in the medium, vehicle and expelled-mass reference frames.
- Explicit power-conversion loss, thruster loss and energy-chain residual; require the harvested-power ledger to close within declared numeric tolerance.
- Useful mission service at matched departure state, payload, destination, arrival velocity, braking, time and reliability.
- Medium-density and field-coupling uncertainty; do not fit the effective area to the same data used for validation.

### Assurance targets

- 100% agreement with independently hand-calculated finite reference cases.
- Zero false passes on frozen sign, zero-input, invalid-efficiency, non-finite, over-domain and energy-identity negative controls.
- Deterministic output for identical input bytes and implementation version.
- Every output linked to exact input, source/evidence class, assumptions, units and claim ceiling.
- Unknown / out-of-domain physics remains unknown; no imputed neutral value or automatic promotion.

## Paradigm shifts to test next

1. **Service-centric engineering:** optimize reliable, accepted radiative or propulsion service, not a peak metric.
2. **Mass closure before performance claims:** account for complete installed hardware, maintenance, spares and replacement.
3. **ISRU as a process chain:** resource identification → extraction → refining → product qualification → fabrication → assembly → inspection → repair.
4. **Reversible modular infrastructure:** graceful degradation, inspectability, autonomous fault isolation and end-of-life disposal.
5. **Model hierarchy:** analytical identities → synthetic tests → independently validated specialist models → controlled component experiments → subsystem demonstrations → flight evidence.
6. **Regional public-interest accounting:** global solar forcing must never be equated directly with local wet-bulb/WBGT benefit.

## Evidence ceiling

This crate establishes only that the implemented arithmetic agrees with the stated formulas and frozen synthetic references. It does not establish optical performance, climate benefit, lunar ISRU capability, orbital stability, plasma coupling, achievable q-drive acceleration, hardware readiness, flight qualification or authorization for planetary intervention.
