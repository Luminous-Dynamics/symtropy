# Artificial-gravity architecture comparison protocol

**Status:** research protocol; no architecture is qualified or ranked yet.

## Purpose

Compare rotating-habitat architectures on equal mission requirements. Do not select a winner from pretty renders, a single peak stress, or a nominal 1 g point. The comparator must include useful occupied volume, the spatial acceleration field, dynamics, structure, interfaces, manufacturing, maintenance, failure response, and lifecycle cost.

## Reference scenarios

Use at least three declared scenarios to avoid optimizing one attractive point:

| Scenario | Target at reference radius | RPM | Ideal radius |
|---|---:|---:|---:|
| Earth-like, low-spin | 1.00 g | 2.0 | 223.565 m |
| Earth-like, intermediate | 1.00 g | 3.5 | 73.001 m |
| Mars-like, low-spin | 0.38 g | 2.0 | 84.955 m |

The ideal radius uses $r = g_{target} × 9.80665 / (2π RPM/60)^2$. These are comparison anchors, not chosen habitat dimensions. The occupied radius varies with architecture and floor/deck placement.

## Candidate families

1. **Monolithic ring/cylinder:** continuous primary rotating pressure structure. Baseline for simple load paths, but inspect joints, large-diameter fabrication, local damage, and repair/replacement logistics.
2. **Segmented ring with hub/spokes:** modular pressure/occupied modules carried by radial spokes or trusses; compare joint count, spoke buckling, hub loads, local load redistribution and assembly.
3. **Counter-rotating twin rings:** compare net angular momentum and attitude-control demand against added bearings, transfer interfaces, mass, failure coupling and maintenance. Counter-rotation can reduce net momentum; it does not make local structural loads or gyroscopic effects disappear.
4. **Cable/tether-supported habitat ring:** compare tensile load path, pretension, dynamic modes, slack/taut transitions, deployment, micrometeoroid damage, inspection, and redundant tether paths.
5. **Tension-dominated/tensentric topology:** model the proposed separation of tensile skeleton and habitat members as a candidate topology. Do not assume its conceptual advantages survive realistic mass, damping, connection, fatigue and fault constraints.
6. **End-over-end multi-arm architecture:** include only where useful to compare an alternate geometry; account for occupied-zone acceleration direction changes, end loads, and transfer architecture.

Do not compare a cable ring that lacks a pressure vessel against a complete pressurized cylinder as if they are equivalent. Each concept must include the complete functional structure and pressure boundary.

## Matched design contract

Every candidate must share the same versioned mission envelope:

- Target gravity, RPM range, reference radius, occupied-zone radial and axial bounds.
- Usable pressurized volume, deck area, occupancy, access/egress and privacy requirements.
- Pressure, atmosphere, radiation shielding, thermal control, micrometeoroid protection and fire compartmentation.
- Payload, furniture, equipment, crew and cargo mass distributions; declared mass growth and worst-case imbalance.
- Mission life, spin-up/down cycles, duty cycle, inspection intervals and repair assumptions.
- Docking/transfer interfaces, stationary hub requirements, rotary joints/seals, power/data/fluid transfer and emergency egress.
- Reliability, redundancy, fault-containment and safe-degraded-mode requirements.
- Launch/assembly constraints, robotic/human assembly capability, material/process availability and orbital servicing assumptions.

A candidate that cannot meet the same envelope is infeasible—not a low-cost alternative.

## Physics and analytical checks

Use the closed-form thin rotating-ring hoop-stress result as a limiting reference only:

$$σ = ρ ω^2 r^2$$

It assumes a uniform thin ring, homogeneous material and ideal axisymmetric rotation. It excludes pressure loads, joints, local attachments, stress concentrations, dynamic response, manufacturing defects, fatigue, thermal gradients, shielding, and safety factors. It is not a design allowable or a substitute for structural analysis.

Require:
- dimensional analysis and independent hand checks for force, torque, stress and energy;
- global force/momentum balance and angular-momentum accounting;
- static and dynamic load cases, spin-up/down and transient disturbance cases;
- modal analysis with operating harmonics and control bandwidth declared;
- buckling, fatigue, fracture/damage tolerance and connection/joint qualification;
- sensitivity to material scatter, geometry tolerances, mass growth, damping and boundary conditions;
- mesh/time-step convergence and independent solver or benchmark comparisons;
- physical correlation using representative coupons/joints/subassemblies before model promotion.

A digital twin does not inherit validation from a solver name. Each model is qualified only within the tested configuration and envelope.

## Multi-objective scorecard

Publish each measure independently; never collapse everything into one opaque score.

| Domain | Primary metrics |
|---|---|
| Gravity field | COM and occupied-zone g error; head/feet and floor/ceiling gradients; Coriolis/Euler terms under declared motion |
| Structure | structural mass / useful pressurized m³; stress and buckling margins; deflection; fatigue life; damage tolerance |
| Dynamics | modal spectrum; separation from 1/rev and higher harmonics; vibration transmissibility; imbalance sensitivity; nutation/attitude coupling |
| Interfaces | hub/rotary joint count; leakage/failure rate; power/data/fluid transfer losses; docking and egress time |
| Manufacturing | unique part count; material/process qualification; assembly hours; tolerance stack; robotic access; inspection coverage |
| Operations | availability; maintenance hours; replacement time; spares mass; safe-degraded operation; fault recovery time |
| Energy | spin-up energy; steady parasitic power; losses; thermal rejection; attitude-control energy; regenerative energy actually returned |
| Economics | lifecycle cost per usable pressurized m³-year and per crew-year; cost of launch/assembly/maintenance/replacement |
| Evidence | input/source provenance; uncertainty budget; independent replication; model-to-test residual; unclosed deviations |

Report p50, p95, p99 and worst credible cases where distributions are relevant. Report infeasible candidates and why; don't quietly drop them.

## Paradigm-shift hypotheses to test

- **Field-contract design:** make the occupied-zone gravity field a first-class product specification, rather than optimizing structure around a single nominal floor acceleration.
- **Assurance-native topology:** choose structure and interfaces partly for inspectability, fault isolation, repair and evidence capture, not just minimum initial mass.
- **Lifecycle Pareto optimization:** optimize mass, useful volume, energy, assembly, maintenance, reliability and cost as a Pareto front under hard safety constraints; avoid arbitrary weighted scalar scores.
- **Graceful degradation:** qualify how much useful habitat can remain operational after a local member, seal, bearing, power, or sensor fault.
- **Digital thread:** bind each as-built member, joint, material batch, calibration, NDE record, repair and model update to a common configuration identity.

These are hypotheses, not demonstrated advantages.

## Benchmark targets

First release: exact geometry, units, load-case definitions, source data and baseline scorecards; zero untracked input substitutions; all candidates use the same functional-unit contract; all FEA has mesh/time convergence and an independent analytical or reference-solver check.

Later stretch target: at least 20% lower lifecycle cost or structural/maintenance burden per usable m³-year than the best matched baseline, with no regression in gravity-field quality, reliability, safety margins, repairability or service life. Freeze the metric and target only after baseline uncertainty is quantified. This is a programme target, not a result or industry SOTA claim.

## Historical counter-rotation reference and stability trade

NASA Contractor Report 181617 (June 1988), *Some Operational Aspects of a Rotating Advanced-Technology Space Station for the Year 2025*, is a useful historical engineering comparator:

- The reference design used a 229 m diameter rotating torus and reported 1 g at 2.8 RPM.
- Its torus/ring cross-section was 15.3 m and the report's mass table listed about 2.27 × 10^6 kg total torus mass.
- It considered counter-rotating water-storage tanks to reduce the station's net angular momentum and gyroscopic torquing requirements.
- The report also notes that reducing gyroscopic properties reduced inertial stability, making the station more responsive to gravity-gradient and aerodynamic torques.

NASA source: https://ntrs.nasa.gov/api/citations/19880017013/downloads/19880017013.pdf (NASA-CR-181617).

This is a 1988 concept study, not modern flight-qualified hardware or a current SOTA design. Its enduring lesson is methodological: net angular momentum, gyroscopic stability, attitude-control authority, response to environmental torques, and stored rotational energy are separate quantities. Counter-rotation can reduce net angular momentum while increasing stored energy, adding bearings/interfaces, and changing passive stability.

The axial scalar reference oracle in this programme covers only same-axis signed momentum and kinetic energy. It is not a substitute for 3D anisotropic inertia, SO(3) dynamics, environmental torque, nutation/precession, wobble damping or spacecraft attitude control. Those must reuse and pass the existing Symtropy rotational-dynamics owners before an architecture-level claim is made.

## Axial momentum reference fixture

The Rust reference functions and matrix executable implement:

\[
H_z = \sum_i I_i \omega_i,\qquad
E_{rot} = \frac{1}{2}\sum_i I_i\omega_i^2
\]

with signed omega for direction. For two coaxial modules, ideal zero-net-momentum cancellation requires:

\[
\omega_2 = -\frac{I_1}{I_2}\omega_1
\]

The fixture at fixtures/rotor_balance_matrix.csv varies counter-rotor inertia ratio and 90%, 100%, and 110% of the ideal cancellation speed. It intentionally shows that zero net momentum does not imply zero stored rotational energy. The values are independent analytic reference values, not output captured from a compiled Rust binary.

Run the generator with:

~~~bash
cargo run --release --manifest-path research/artificial-gravity-v0/Cargo.toml --bin rotor_balance_matrix
~~~

Follow-on owner: ARTGRAV-ATTITUDE-001 (#5291), which consumes the existing frame-aware inertia and momentum-map work instead of duplicating a generic rigid-body solver.

## Evidence gate and handoff

- Symtropy owns physical simulation, numerical qualification and model-to-test comparisons.
- Symthaea may propose candidate topologies and multi-objective search directions, but may not relax hard constraints or self-certify.
- Mycelix records immutable artifact hashes, provenance, attestations, and correction/disagreement history. Hash validity proves identity, not physical truth.
- Sol-Atlas owns habitat arrangement, build/assembly logic, access, maintainability and lifecycle integration.

Unknown structural properties remain explicit unknowns. No ranking is publishable until inputs, assumptions, uncertainty, failure cases and independent review are attached.
