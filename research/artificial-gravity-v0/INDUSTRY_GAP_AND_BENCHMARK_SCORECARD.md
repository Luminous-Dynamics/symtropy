# Artificial-gravity industry gap and benchmark scorecard

**Snapshot date:** 2026-11-15  
**Status:** research baseline and proposed targets. No Luminous habitat, human exposure system, structure or rotor is qualified.

## Executive finding

The field is not yet at a stage where commercial roadmap claims can be compared with independently measured orbital human performance. Ground centrifuge research, historical engineering concepts, commercial station roadmaps, and unvalidated architecture proposals answer different questions.

Luminous should not claim to be ahead of NASA, Texas A&M, Vast or other aerospace organizations on hardware maturity. The defensible competitive opportunity is to build a more complete, reproducible and openly auditable co-design/qualification process, then earn better results through matched simulation, physical correlation and independent review.

## Industry and research landscape

| Reference | Evidence class | Publicly described capability or target | What it does not establish |
|---|---|---|---|
| NASA CR-181617 / 1988 rotating Advanced-Technology Space Station | Historical engineering study | 229 m diameter rotating torus, 2.8 RPM for approximately 1 g, 15.3 m ring cross-section; counter-rotating storage concepts examined | Modern qualified hardware, current materials/cost, orbital human tolerance |
| Texas A&M Anthony Wood Artificial Gravity Lab (2026) | Ground research capability | NASA-developed centrifuge transferred to Texas A&M; current studies use short sessions, with longer sessions a research goal | Sustained orbital exposure or long-duration habitat efficacy |
| Vast Artificial Gravity Station roadmap | Commercial roadmap target | 2035 target; 3.5 RPM; 950 m³ habitable volume, 2,160 m³ pressurized volume, 40-person crew | Built station, measured gravity field, in-orbit human tolerance |
| Aegis Station Gradient One Rev C | Public architecture proposal | 1.6 RPM; approximately 350 m 1 g band, 132 m Mars band, 58 m lunar band, rotating zero-g hub; author-described 10-person and 180-day campaign | Independently verified mass, structure, operations, long-duration human exposure |
| Tensentric habitat architecture (2026) | Published concept | Tension-dominated structural concept with habitat modules distinct from tensile framework | Qualified structure, manufacturing yield, mass advantage or flight demonstration |

Primary references:
- NASA CR-181617: https://ntrs.nasa.gov/api/citations/19880017013/downloads/19880017013.pdf
- Texas A&M (20 April 2026): https://stories.tamu.edu/news/2026/04/20/human-space-research-gets-a-boost-from-retired-nasa-centrifuge/
- Vast roadmap: https://www.vastspace.com/roadmap
- Gradient One Rev C: https://aegisstation.com/gradientone.html
- AGRE: https://aegisstation.com/artificial-gravity-requirements-evaluation.html
- Tensentric paper: https://doi.org/10.1016/j.actaastro.2026.04.071
- 2026 artificial-gravity research review: https://doi.org/10.3389/frspt.2026.1967384

Treat the Vast and Gradient One entries as their authors' published roadmap/proposal values, not independent results. Texas A&M is a terrestrial analogue research capability. NASA CR-181617 is a historical concept study. Tensentric is a published design concept.

## Matched kinematic comparison

The table below compares one simple, common condition: ideal 1 g at the occupied reference radius, 2 m radially aligned body length, and 1 m/s movement perpendicular to the spin axis. All values are independently derived from the same closed-form equations, not measured performance or physiological limits.

| RPM | Derived 1 g radius | Ideal 2 m head-to-feet gradient | 1 m/s Coriolis magnitude | Coriolis / 1 g |
|---:|---:|---:|---:|---:|
| 1.6 | 349.320 m | 0.573% | 0.3351 m/s² | 3.417% |
| 2.0 | 223.565 m | 0.895% | 0.4189 m/s² | 4.271% |
| 2.8 | 114.064 m | 1.753% | 0.5864 m/s² | 5.980% |
| 3.5 | 73.001 m | 2.740% | 0.7330 m/s² | 7.475% |
| 4.0 | 55.891 m | 3.578% | 0.8378 m/s² | 8.543% |

The first-order trade is visible: higher RPM reduces required radius but increases head-to-feet gradient and movement-induced Coriolis acceleration for the same body length and movement speed. Actual human response depends on vector direction, head motion, adaptation, exposure schedule and individual variability. Do not interpret these percentages as comfort thresholds.

The 1.6 RPM / 350 m proposal and 3.5 RPM roadmap target are therefore useful *different-regime* comparators, not evidence that one is inherently better.

## Luminous paradigm shifts and measurable differentiators

### 1. Gravity-field contract, not a nominal g-value

Specify the vector acceleration field over the entire occupied volume and over time. A contract includes radial/axial bounds, min/max/percentile local g, head/heart/centre-of-mass/feet values, body gradient, movement velocity, angular acceleration and the coordinate-frame convention.

**Differentiator:** an auditable gravity-field envelope with a configuration-bound uncertainty budget.

### 2. Multi-gravity infrastructure, not one-purpose centrifuges

Treat gravity levels as spatially and operationally assigned services: lunar/Mars/Earth-like occupied zones, rotating microgravity hub, stationary docking and service interfaces, and explicit transition procedures. Each band requires separate access, fire/pressure, egress and operational analysis.

**Differentiator:** demonstrate several gravity environments without hiding the extra structural, transfer and maintenance burden.

### 3. Human movement as an input to design

Build a tolerance-and-benefit surface over angular velocity, gravity level, derived radius, body geometry, movement direction/speed, head movement, spin transients, exposure schedule and individual variation. Use Symthaea's robust experiment design to identify informative experiments, but never infer unmeasured tolerance from synthetic data.

**Differentiator:** preregistered and powered human-factors research with held-out participants/geometries and explicit unmeasured regions.

### 4. Co-design structure and spacecraft attitude

Minimize net angular momentum only subject to gyroscopic stability, attitude-control authority, response to external torques, stored spin energy, bearings, interfaces and fault recovery. Counter-rotation is not free: it can reduce net momentum while changing passive stability and increasing stored energy and interface count.

**Differentiator:** a whole-vehicle Pareto frontier for momentum, stability, control cost, energy and fault tolerance.

### 5. Assurance-native manufacturing

Make inspection, repair, replacement, module isolation, tolerance management, as-built survey and evidence capture first-class topology variables. Compare mass and cost per useful pressurized m³-year, not mass per bare ring or one-off render.

**Differentiator:** independently traceable digital thread from material lot and part identity through manufacturing, assembly, test, repair and requalification.

### 6. Experiment-in-the-loop digital twin

Symthaea proposes tests/design candidates; Symtropy computes physics and uncertainty; physical instruments test predictions; independent analysis decides whether models remain valid; Mycelix preserves provenance and disagreements; Sol-Atlas integrates human-centred layout and build sequence.

**Differentiator:** closed-loop improvement where each prediction has a falsifiable test and every claim has a maturity/evidence class.

### 7. Separate direct-gravity research from artificial-gravity engineering

Rotation and linear acceleration are established kinematic mechanisms. Direct gravitational-field generation/manipulation remains a separate physics research question. Metric-shift simulations or exotic metric ansatzes must pass independent mathematical constraints, source feasibility, stability, energy-condition and physical-realization gates before any hardware claim is made.

**Differentiator:** research ambition without epistemic contamination between known physics and speculative hypotheses.

## Benchmark ladder and target scorecard

All numerical targets below are proposed programme objectives, not achieved performance or universal certification limits.

| Benchmark | Required score | Initial target |
|---|---|---|
| AG-KIN-0 analytic reference | Forward/inverse radius, RPM, g, angular velocity, vector sign, invalid values | Relative residual below 1e-10 over declared f64 analytic corpus |
| AG-MOTION-0 rotating-frame field | Centrifugal, Coriolis, Euler and combined vector across directions/speeds/transients | Independent reference parity below 1e-10 for exact analytic cases |
| AG-SIM-1 dynamics | Analytic trajectory, energy/angular momentum, timestep convergence, solver stability | Publish convergence curves and conservative error envelopes before any pass threshold |
| AG-STRUCT-2 architecture | Same pressure volume, load cases, crew/cargo, shielding, mission life, service/repair constraints | No winner until all candidates satisfy the matched contract; physical coupon/joint correlation before model promotion |
| AG-ATTITUDE-2 whole-vehicle | Vector angular momentum, gyroscopic stability, control effort, external torque and fault recovery | SO(3) covariance and momentum/port-ledger gates before architecture performance claims |
| AG-ROTOR-3 uncrewed hardware | Calibrated acceleration/speed/radius, vibration, thermal, repeatability, protective chain | Less than 1% model-to-measurement residual inside a calibrated unmanned envelope as a stretch target, after uncertainty is established |
| AG-HUMAN-4 analogue/physiology | Preregistered, powered, independently reviewed task and physiological endpoints | No universal RPM/g threshold before adequate data; report intervals, lower-tail response and adverse events |
| AG-FLIGHT-5 orbital | Qualified vehicle, transfer/utilities, long-duration human/operations evidence | Separate flight qualification; no ground-simulation promotion |
| Lifecycle Pareto | Useful volume, crew-year, structural mass, energy, availability, assembly, repair and cost | At least 20% lower lifecycle cost or structural/maintenance burden per usable m³-year versus best matched baseline, with no hard safety/performance regression; stretch target only |

## Fair comparison rules

Every design comparison freezes:
- mission phase/duration, crew, destination and occupied pressurized volume;
- gravity-field envelope, RPM, radius, movement and exposure requirements;
- pressure boundary, atmosphere, radiation shielding, thermal rejection, fire compartmentation and life support;
- mass distribution, cargo, crew movement and mass growth;
- docking, rotating/stationary transfer, power/data/fluid service and egress;
- lifetime, spin cycles, inspection, replacement, repairs and spares;
- launch/assembly assumptions and technology readiness;
- numerical solver/version, mesh/time step, model calibration and measurement uncertainty.

Report infeasible candidates and missing evidence. Do not rank a naked ring against a full habitat. Do not compare nominal RPM or simulated throughput across unmatched boundaries. Do not collapse hard safety constraints into a weighted score.

## Existing Luminous ownership

- Symtropy analytic and rotating-frame model: PR #5279.
- Symtropy structure and architecture comparison: #5280.
- Symtropy occupied-field and motion benchmark: #5289.
- Symtropy counter-rotation and attitude benchmark: #5291.
- Symtropy inert-payload demonstrator: #5292.
- Symthaea human-factors experiment design: https://github.com/Luminous-Dynamics/symthaea/issues/12222.
- Symthaea settlement profile: https://github.com/Luminous-Dynamics/symthaea/issues/7687.
- Mycelix evidence integration: https://github.com/Luminous-Dynamics/mycelix/issues/6204.

Do not create a second rigid-body solver, generic experiment-design engine or scientific evidence ledger. Reuse Symtropy #3946/#3949/#3132/#3139/#3975 and shared physics validation owners; reuse Symthaea CTGP prediction adequacy/robust experiment design; reuse the canonical Mycelix DeSci event kernel.

## Sources and interpretation limits

- Briard & Baatout (2026), artificial-gravity physiology/evidence-gap review: https://doi.org/10.3389/frspt.2026.1967384
- NASA-CR-181617 (1988): https://ntrs.nasa.gov/api/citations/19880017013/downloads/19880017013.pdf
- Texas A&M Engineering (2026-04-20): https://stories.tamu.edu/news/2026/04/20/human-space-research-gets-a-boost-from-retired-nasa-centrifuge/
- Vast roadmap: https://www.vastspace.com/roadmap
- Gradient One Rev C: https://aegisstation.com/gradientone.html
- AGRE: https://aegisstation.com/artificial-gravity-requirements-evaluation.html
- Tensentric, Acta Astronautica (2026): https://doi.org/10.1016/j.actaastro.2026.04.071

Review claims and design concepts at their actual evidence level. An author-reported design value is not an independent measurement; an analytic formula is not a human tolerance result; a digital twin is not a physical qualification.
