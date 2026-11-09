# LOCO-MORPH-FAULT-CODESIGN-001 — Structural redundancy and graceful-degradation morphology

**Status:** candidate-space protocol and synthetic design fixtures. No morphology or robot-level performance result is claimed.

Issue: [#4360](https://github.com/Luminous-Dynamics/symtropy/issues/4360).

## Objective

Co-design morphology, actuation, transmission, sensing, passive compliance and control so that common component faults leave more useful mobility and recovery options—without paying an unacceptable penalty in nominal capability, mass, cost, energy, complexity or service burden.

This is a design-space/evidence-composition workstream. It connects:
- #4358 actuator-fault injection and matched diagnosis/recovery campaign;
- #4342/#4345 complete common-cause fault possibilities;
- #4332/#4334 evidence-qualified multimodal failover;
- #4338/#4340 exact-cell qualified operating envelope;
- #4206 actuator dynamics; #4179/#4044 actuator loss maps and energy;
- #4061/#4083 robust viability/recovery and sound abstraction;
- #4326 adverse-outcome vector; #4329 empirical capability;
- QWM morphology-conditioned world model (CoRL 2026);
- #3357 matched locomotion baselines.

It does not implement another controller, fault classifier, fault enumerator, viability solver, scorecard or generic weighted objective. The ownership is the *embodiment design variables, candidate families, nominal/degraded Pareto design matrix, and validation boundary*.

## Paradigm shift: from fault response to fault-prepared embodiment

The conventional approach optimizes nominal mobility, then asks the controller to compensate after a leg, actuator, drive, sensor or bus fails. That assumes the remaining body still has adequate support geometry, actuator authority, sensing, mechanical load paths and recoverability.

We instead ask: **can the robot's body be designed so that credible failures leave a larger physically recoverable and useful operating set?**

The target is not maximum hardware redundancy. Additional limbs, actuators, sensors and clutches can increase mass, distal inertia, wiring, thermal load, maintenance and common-cause failure exposure. The target is a better **nominal-versus-degraded Pareto surface**, conditioned on an independently qualified recovery/safety gate.

Three decisions remain separate:
1. **Design:** which structure and resource allocation provides useful redundancy?
2. **Diagnosis/reconfiguration:** what fault condition is present and which new mode can be entered?
3. **Physical assurance:** is the transition and resulting action set viable/recoverable for every currently plausible fault signature?

A clever morphology or learned controller cannot waive the physical viability/assurance evidence.

## Literature / industry baselines

### Passive overload protection and modularity: INSECTRON

Romano & Manduca, *Insect-inspired hexapod robot with magnetically coupled compliant and modular drive system*, npj Robotics 4, 54 (published **5 November 2026**), DOI 10.1038/s44182-026-00106-y. [Paper](https://www.nature.com/articles/s44182-026-00106-y).

INSECTRON is a centimeter-scale hexapod (8.7 cm body length, 33.9 g). Two DC motors drive two magnetically coupled three-leg modules. Magnetic micro-clutches transmit torque in nominal conditions and slip under overload; additional magnetic couplings accommodate load imbalance between modules. Hall sensors read output gear phase without a conventional mechanical encoder. The magnet interfaces also provide self-alignment, tool-free leg replacement and rapid mechanical reconfiguration.

The paper reports 360 controlled vertical perturbation trials: two body locations and three indenter hold durations. Reported recovery probabilities are 0.91, 0.95 and 0.94 for the 0.1 s, 0.5 s and 1 s hold conditions. It also demonstrates slope/substrate locomotion and differential turning.

**Correct use:** a passive mechanical resilience, overload decoupling and serviceability comparator. It is not a whole-quadruped limb-loss result, nor a controlled actuator fault campaign. The study reports no force/torque sensing, and a minimum CoT of 30 at this 33.9 g scale; do not directly compare that CoT with a full-size quadruped.

Research question to extract: can overload-slip coupling preserve the drive train, prevent motor stalls and allow safe re-engagement after asymmetric or transient loads? The missing next tests are controlled force/torque envelopes, slip threshold dispersion, re-engagement probability/time, missed/false re-engagement, repeated-cycle wear, and behavior during actual actuator or limb failure.

### Functional appendage redundancy

Guo et al., *Fault-tolerant control of a mole cricket-inspired quadruped robot with functional arms under single-leg loss* (Advanced Bionics, Mar 2026), DOI 10.1016/j.abs.2026.02.001. The robot uses functional arms as compensatory support/mobility appendages after a single-leg loss; paper-reported physical mobility is 85.5–90.9% of the nominal reference depending on failure and configuration. This is a morphology concept demonstrator with different sensing/control hardware, not a directly comparable Go2 result.

### Self-sufficient modular sub-machines and amputation-agnostic control

Yu et al., *Agile legged locomotion in reconfigurable modular robots*, PNAS 123(10), e2519129123 (published 10 March 2026), [paper](https://pmc.ncbi.nlm.nih.gov/articles/PMC12974426/), [project](https://modularlegs.github.io/).

Each module is itself a small legged machine: one-DoF actuation with its own battery, motor, sensing, processing and communications. The modules can be joined into bodies with modules serving as legs, arms, tails or a spine. The authors estimate about (10^{11}) possible designs for five modules and search using an eight-dimensional latent representation and asynchronous Bayesian optimization. They explicitly report that the learned latent decoder does not always reconstruct a hand-designed topology exactly, a warning that every proposed latent design must be decoded and validated against a physical topology/geometry manifest.

A single amputation-agnostic sensorimotor policy was distilled from expert trajectories for intact, one-module-loss, two-module-loss and four-module-loss configurations. The physical five-module quadruped was tested at three previously unseen cut locations: one hindlimb removed, both hindlimbs removed and all but one module remaining. The authors report retained forward locomotion in all three cases and 105.3% of the original controller's speed in the undamaged configuration.

**How we use it:** this is the strongest current comparator for *self-sufficient modular sub-machines and structural damage tolerance*. It is distinct from (a) functional arms that are repurposed only after failure, (b) detachable limbs dependent on a central torso for power/control, and (c) passive overload clutches that protect a transmission but do not supply autonomous locomotion after module loss.

**Limits:** only three previously unseen amputation cases were tested on physical hardware; broader damage coverage was simulated. The modules are not autonomously reabsorbed/reconfigured after damage; the paper explicitly leaves self-repair and in-situ morphology changes as future work. Treat module disconnection, broken dock, power loss, sensing loss and control/network isolation as separate fault modes. A controller that continues sending commands to a disconnected module is not evidence that the module/whole robot has passed a viability gate.

### Morphology-conditioned dynamics transfer

Danesh et al., *Morphology-Conditioned World Model for Cross-Embodiment Quadrupedal Locomotion* (CoRL 2026; arXiv:2604.08780; code https://github.com/modanesh/QWM). One world model is conditioned on a compact morphology vector and trained across eight quadruped embodiments. The project reports 20 held-out hardware trials across Go1 and ANYmal-D with zero falls on flat-ground velocity tracking, and reports 75–84% normal episode length under 20% torque loss or added payload. The authors describe a fixed 12-DoF quadruped template and flat-ground scope; this does not establish transfer to structural topology changes, limb loss or complex contact terrain. Use as a model-transfer baseline, not a fault diagnosis/reconfiguration or safety certificate.

### Reconfigurable/differentially coupled mechanisms

*A Morphology-Reconfigurable Quadruped Robot with Differentially Coupled Actuation* (YAC 2026), DOI 10.1109/YAC71005.2026.11616163. The paper describes a single-leg physical prototype with two configurations and 5 kg load-bearing tests. This is a mechanism-level baseline, not whole-robot locomotion/fault qualification.

### Fault-tolerant control baselines

- Adaptive gait timing, Gravina et al., arXiv:2608.07328: adaptive gait-frequency policy with a 68 kg Kyon flat-ground hardware stratum and uneven-terrain simulation.
- Diagnosis-conditioned MoE, Turrisi et al., arXiv:2606.25965.
- FT-WBC, Zhong et al., arXiv:2606.24466: Go2+arm fault estimation and posture adaptation, to be compared in a separate manipulation morphology stratum.

These papers answer different questions. Keep their morphology, task, evidence tier and fault taxonomy separate.

## Candidate embodiment design variables

### Structure and support topology
- limb count, location, reach, workspace and contact geometry;
- primary/auxiliary support appendages and multifunctional arms;
- hip spacing, stance polygon, limb length ratios and joint topology;
- passive compliance, damping, springs, hard stops and mechanical fuses;
- support-load paths under one or more disabled actuators;
- ability to shift support, brace or reposition a failed limb.

### Transmission and overload tolerance
- independent vs coupled/differential actuation;
- proximal vs distal motors and transmission ratios;
- passive differentials, clutches, slip interfaces and disconnects;
- torque-slip threshold, hysteresis, re-engagement and wear;
- backlash, stiction, friction, backdrivability and passive joint dynamics;
- fail-safe brakes and mechanical state after power loss.

### Actuation and power architecture
- torque/power density, continuous vs peak authority and thermal headroom;
- per-leg vs shared drive electronics and common-bus failure domains;
- independent power isolation and reserve distribution;
- actuator modularity, replacement time and connector count;
- derating and safe torque envelope under thermal/voltage faults.

### Sensing and diagnosability
- independent position, velocity, current, temperature and bus measurements;
- foot force/pressure/shear, wrench and contact event channels;
- Hall/encoder placement and observability of clutch slip, joint lock and transmission faults;
- common-cause groups and power/clock/compute isolation;
- sensor redundancy cost in mass, wiring, compute and maintenance.

### World-model/controller interface
- topology-aware morphology schema distinct from continuous parameter vector;
- morphology-conditioned dynamics and actuator response;
- reuse of control primitives without pretending an unseen topology is interpolable;
- uncertainty/fault-set interface, failover and recovery policy;
- independent viability and assurance admission.

**Representation rule:** continuous parameters (geometry, mass distribution, stiffness, torque limits) and categorical topology (leg count, joint graph, clutch/differential architecture, appendage presence) must be distinct. A compact morphology vector for a fixed 12-DoF family cannot silently represent a different support graph as a smooth interpolation.

## Candidate design families (V0)

1. Conventional 12-DoF quadruped reference.
2. Quadruped plus two multifunctional support/manipulation arms.
3. Proximally actuated limb with differential or mechanically coupled DOFs.
4. Reconfigurable limb geometry with two independently verified configuration states.
5. Passive-compliant leg/transmission with overload-slip clutch, self-alignment and verified re-engagement.
6. Distributed per-limb power/drive layout with common-cause isolation.
7. Sensor-redundant/diagnosable actuator module with independent position, velocity, current and thermal evidence.
8. Hybrid candidates combining no more than two or three selected mechanisms, each justified by incremental Pareto improvement.

A candidate needs a concrete CAD/URDF/USD/actuator map and an explicit physical configuration digest before simulation comparison.

## Fault-aware objectives and hard constraints

Report independent objectives/constraints, not one universal weighted score.

**Nominal capability:** speed, terrain clearance, turning, payload, manipulation workspace, task success, dynamic support margins, nominal energy and CoT.

**Degraded capability:** retained task/route/workspace after each fault family and severity; continued locomotion vs controlled stop; robust recovery horizon and post-fault action set.

**Physical safety:** all #4326 adverse outcomes; #4061/#4083 viability/recovery witnesses for the exact morphology and fault signature; no extrapolation beyond the physical abstraction.

**Energy and thermal:** battery/source and actuator electrical energy; mechanical work; peak/continuous torque; temperature rise; voltage/thermal derating; energy reserved for worst-case recovery.

**Serviceability:** module replacement, inspection access, tool count, repair time, connector/wiring complexity and repeated-cycle reliability.

**Diagnosability:** fault set coverage, isolation ambiguity, detection time and ability to distinguish actual actuator faults from encoder/current sensor faults.

A heavier or less efficient design may remain on the Pareto frontier if it significantly enlarges the recoverable fault envelope; it is not automatically superior. A design that only survives by losing nearly all nominal task capability must also remain visible as such.

## Matched campaign

Use #4358 as the fault-injection owner and #4345 as the conservative possibility-space authority. Apply the same frozen fault × terrain × payload × initial gait phase matrix to every morphology that can physically realize that condition.

### Faults
- torque loss, weakening, thermal derating, lock, limited joint range and free swing;
- intermittent drive dropout, slew/saturation and supply/bus failure;
- clutch overload, slip, disconnect and re-engagement;
- encoder/velocity bias, dropout and common-cause sensing faults;
- single-leg loss, multiple joints, two-limb faults and actuator + sensor combinations.

### Disturbances
- impact and body perturbations at defined contact points and durations;
- low friction, slope, step/gap, uneven substrates and obstacle entanglement;
- payload/CoM shifts;
- fault onset during stance, swing, touchdown, turn and transition.

### INSECTRON-inspired mechanism subcampaign
For magnetic or mechanical overload couplings:
- static and dynamic torque-slip threshold distribution;
- slip vs damage protection, motor stall/current reduction and gear stress;
- re-engagement probability and latency;
- phase error and gait recovery after re-engagement;
- nuisance slip in nominal terrain;
- contamination, wear, temperature and repeated-cycle drift;
- one-leg entanglement and asymmetric module load.
Do not infer leg-loss capability from body-perturbation recovery.

### Functional-appendage subcampaign
- nominal manipulation and locomotion with arms;
- support/bracing role during fault;
- object retention/drop and unintended human/environment contact;
- added mass/CoM/inertia and energy;
- workspace sacrificed or retained after reconfiguration;
- fault in the compensatory appendage itself.

## Baseline and ablation matrix

- conventional body + conventional controller;
- conventional body + adaptive gait timing;
- conventional body + diagnosis-conditioned MoE;
- conventional body + FT-WBC in matching arm stratum;
- QWM morphology-conditioned dynamics model vs morphology-unconditioned model;
- passive mechanical overload protection vs rigid transmission;
- functional appendage body vs same body with appendages disabled (matched mass/CoM control);
- independent vs shared/differential drive and power;
- Symtropy full fault-qualified co-design;
- ablate auxiliary appendage, clutch/compliance, sensor redundancy, per-limb power isolation, morphology-conditioned model, active diagnosis, viability gate and recovery reserve individually.

All comparisons share platform class, task, command, environment, interaction budget and test exposure when possible. If the body cannot be made equivalent, use a separately identified design stratum and report mismatch; do not force a false direct comparison.

## Evidence and scorecard

Preserve:
- nominal task success/capability and held-out families;
- fault-specific task success and remaining useful workspace;
- robust viable/recovery classification and counterexample witnesses;
- full adverse-event vector, measurement coverage and severity;
- actuator electrical/mechanical energy, torque/thermal/voltage margin;
- passive clutch slip/re-engagement statistics and cycle wear;
- body mass, payload, inertia, turning and footprint;
- diagnosis coverage and latency;
- service/replacement/inspection time;
- compute and command-to-motion latency;
- simulator, test-rig, hardware and field evidence tiers.

The design-level output is a **nominal-vs-degraded Pareto surface**. A headline count of directly tested, qualified fault-condition cells is useful, but is not a safety proof. Exact cell qualification remains the #4340 conjunction, not a newly invented score.

## Frozen design fixtures

The machine-readable corpus is [fixtures/design_candidates_v0_1.json](fixtures/design_candidates_v0_1.json). It defines:
- baseline and eight candidate design families;
- explicit continuous vs categorical design variables;
- fault and serviceability dimensions;
- matching rules and evidence-tier boundaries;
- adversarial cases for topology interpolation, extra-mass bias, unverified clutch re-engagement, sensor common causes, and a design that improves degraded capability while regressing a required adverse outcome.

## Proposed qualification targets (not achieved)

- 100% morphology/configuration and actuator-map identity binding across CAD/URDF/USD, simulation and hardware.
- Zero structural-topology interpolation without an explicit validated mapping.
- Zero support/reconfiguration actions without current robust viability/recovery admission.
- Zero false-safe qualification from a clutch or appendage whose re-engagement/failure behavior is untested.
- 100% preservation of nominal and per-fault capability/resource/safety results separately.
- Stretch research target: maintain at least 85% of nominal useful mobility on a preregistered single-leg/actuator fault set, without worsening any adverse category or recovery-reserve criterion and within explicit nominal mass/energy/maintenance limits. The 85% value is literature-motivated (not a universal threshold or achieved Symtropy result).

## Claim ceiling

This is a design and validation protocol, not a morphology performance result. A microrobot perturbation recovery rate, simulation optimization, single-leg mechanism prototype or short hardware demo does not establish whole-quadruped arbitrary-fault tolerance. Only directly measured, exact hardware cells supported by #4358 and the #4340 conjunction may be called qualified.
