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

## Evidence gate and handoff

- Symtropy owns physical simulation, numerical qualification and model-to-test comparisons.
- Symthaea may propose candidate topologies and multi-objective search directions, but may not relax hard constraints or self-certify.
- Mycelix records immutable artifact hashes, provenance, attestations, and correction/disagreement history. Hash validity proves identity, not physical truth.
- Sol-Atlas owns habitat arrangement, build/assembly logic, access, maintainability and lifecycle integration.

Unknown structural properties remain explicit unknowns. No ranking is publishable until inputs, assumptions, uncertainty, failure cases and independent review are attached.
