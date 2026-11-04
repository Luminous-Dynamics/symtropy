# Neural Sensorimotor Adapter V0

**Status:** architecture / benchmark contract only. No executable neural-to-body integration is claimed.

**Parent work:** [ANIMA-NEURO-001 / Symtropy #3313](https://github.com/Luminous-Dynamics/symtropy/issues/3313)

**Related contracts and implementation:**
- [ANIMA / embodied agent architecture #1176](https://github.com/Luminous-Dynamics/symtropy/issues/1176)
- [Causal fauna contracts PR #244](https://github.com/Luminous-Dynamics/symtropy/pull/244)
- [Living World Observatory PR #245](https://github.com/Luminous-Dynamics/symtropy/pull/245)
- [Non-omniscient fauna perception oracle #249](https://github.com/Luminous-Dynamics/symtropy/issues/249)
- [Animal observation/replay programme #2128](https://github.com/Luminous-Dynamics/symtropy/issues/2128)
- [Animal observation and replay implementation candidate PR #2145](https://github.com/Luminous-Dynamics/symtropy/pull/2145)
- [Symthaea perturbation-grounded connectome dynamics #9522](https://github.com/Luminous-Dynamics/symthaea/issues/9522)
- [Symthaea task/state-conditioned connectome reduction #9549](https://github.com/Luminous-Dynamics/symthaea/issues/9549)
- [Symthaea synthetic C. elegans graph correction #9902](https://github.com/Luminous-Dynamics/symthaea/issues/9902)

---

## 1. Purpose and claim ceiling

Define a narrow, provider-neutral interface through which a versioned neural circuit model can participate in an embodied animal's sensorimotor loop.

The intended change is not "add a neural network to animation." The intended causal system is:

`physical/ecological event
→ signal emission
→ propagation / attenuation / noise
→ receptor
→ bounded percept
→ neural input encoding
→ circuit dynamics
→ motor-unit / muscle activation proposal
→ action arbitration
→ body and physics response
→ proprioceptive / interoceptive feedback
→ next neural step`

A neural model may be biologically grounded, connectome-constrained, phenomenological, learned, or synthetic. These categories must be explicit. A biologically named circuit is not automatically source-verified, functionally calibrated or behaviourally validated.

This document is an architecture hypothesis and test contract. It does not report a biological, causal, gameplay or performance result.

## 2. Existing authority boundaries

This adapter composes existing Symtropy systems; it must not become another animal simulator.

| Concern | Existing authority |
|---|---|
| Environmental truth, physical events and contact | Living World / physics |
| Signal emission and propagation | Basin / environmental signal systems |
| Non-omniscient sensing and percept provenance | Fauna perception |
| Persistent individual identity, bounded memory and beliefs | ANIMA |
| Homeostatic state and ecological consequences | Existing life-simulation authorities |
| Body topology, joints, muscles and contact | Animal body-plan / locomotion / physics |
| Neural circuit model and its state | Qualified Symthaea/provider adapter |
| Presentation and animation | One-way presentation layer |
| Empirical dataset scope, rights and splits | Animal observation / replay contracts |

A neural controller must not:
- query arbitrary world truth or hidden entities;
- infer a percept from renderer visibility, animation, decals or camera state;
- mutate canonical position, velocity, terrain, injury, physiology, relationships or ecology;
- bypass body affordances, action arbitration, collision/contact or physical constraints;
- write directly to animation transforms;
- erase future-bearing state when fidelity changes.

ANIMA is a persistent agency and world-simulation abstraction, not a claim that one software subsystem maps one-to-one onto an anatomical brain. A circuit model may own a declared local control scope; ANIMA retains its existing world-state, memory, relationship and action-authority contracts.

## 3. Causal controller architecture

### 3.1 Inputs

A controller receives two classes of input:

1. **Externally sensed signals:** only qualified receptor/percept outputs derived from the existing signal-emission and propagation chain.
2. **Self-sensed body state:** proprioceptive and interoceptive channels such as joint angle, muscle stretch, load, contact, balance error, fatigue, thermal state and energy availability.

Each input must declare:
- semantic channel and receptor identity;
- unit, coordinate frame, scale and valid range;
- source/tick/freshness and provenance;
- whether it is observed, derived, estimated or synthetic;
- uncertainty or confidence semantics;
- the exact mapping/version digest.

No hidden world-state field is an admissible neural input.

### 3.2 Typed records

The product interface should eventually provide equivalents of:

**NeuralModelDescriptor**
- organism/taxon, sex and life-stage scope, where known;
- model family and claim ceiling (synthetic, phenomenological, connectome-constrained, perturbation-calibrated, behaviourally validated);
- source structural-connectome identity;
- effective-coupling / perturbation dataset identity;
- dynamics and solver versions;
- sensor, proprioceptor, motor-neuron, muscle and body mapping digests;
- supported tasks, states and known unsupported conditions.

**NeuralObservationFrame**
- fixed-tick, ordered input channels;
- explicit units, ranges and frames;
- source provenance/freshness for external signals;
- sensor self-observation identity for body-derived channels;
- no untyped "world snapshot" or omniscient blackboard.

**ModulatoryContext**
- bounded internal pressures supplied by ANIMA and physiological authorities (energy, hydration, fatigue, thermal stress, injury-related limitations, arousal or learned expectation);
- explicit semantic distinction between a drive, a belief, an observed signal and an instruction;
- no direct joint transform, hidden predator identity or unobserved resource location.

**CircuitState**
- exact model/version binding;
- deterministic initialization and reset semantics;
- state-before/state-after digest;
- checkpoint/replay policy;
- explicit solver/timestep and numerical precision.

**MotorProposal**
- finite, normalized motor-unit or muscle activation proposals, reflex requests or declared effector-space signals;
- known body-region / effector mapping;
- bounded activation, rate of change and validity horizon;
- model provenance and confidence/abstention;
- no direct canonical world transform or unrestricted torque/force authority.

**NeuralStepReceipt**
- exact model, code, config, solver, input frame, circuit state, output and scenario digests;
- deterministic run identity and tick;
- explicit status: valid, abstained, unsupported, stale, invalid or failed;
- enough detail to reproduce and audit a controller decision.

These are semantic requirements, not frozen Rust type names. Keep the first implementation small and dependency-light.

### 3.3 Outputs and execution

The neural model emits a bounded proposal. Existing deterministic arbitration checks:
- whether the proposal is valid and fresh;
- whether the requested effector exists on this body plan;
- whether the action is supported by current affordances and physiology;
- whether it conflicts with other current obligations or safety constraints;
- whether the physics/body solver can realize it.

Only the physics/body authority commits resulting motion. A motor proposal is not proof that movement occurred.

### 3.4 Feedback

After physical integration, the body produces new receptor signals:
- joint position/velocity;
- contact force and slip;
- limb loading and support state;
- muscle/tendon stretch where represented;
- balance and orientation error;
- fatigue/energy/thermal changes.

These become the next neural observation frame. Do not replace this feedback with the previous requested movement; use the actual realized body state.

## 4. Distributed local control and supervisory cognition

A useful architectural hypothesis comes from the 2026 adult female Drosophila brain-and-nerve-cord connectome. The reported organization includes local sensory-to-effector feedback loops linked by ascending and descending behaviour-centric modules, with learning/navigation regions supervising them.

That supports investigating a layered controller:
- fast local feedback for posture, limb loading and reflexes;
- mid-level coordination for gait phase, turning, feeding, grooming or escape;
- slower supervisory processes for navigation, learning, goals and context;
- ANIMA's persistent memory, needs, uncertainty and relationships as world-level agent state that can modulate and interpret action without bypassing physical authority.

This is an architectural inspiration derived from fly-specific evidence, not a universal brain blueprint. Worm, fly, deer, horse, dog and robot controllers require separate evidence and mappings. Do not transplant fly circuitry into another taxon by analogy alone.

Reference: Bates et al. (2026), *Distributed control circuits across a brain-and-cord connectome*, https://www.nature.com/articles/s41586-026-10735-w

## 5. Model evidence layers must remain separate

A model descriptor and its receipts must distinguish:

1. **Structural connectome:** observed neuron identities and anatomical connections.
2. **Effective coupling:** measured response to perturbation, including multi-hop, neuromodulatory or extrasynaptic effects.
3. **Neural dynamics:** equations, parameter fitting, time constants, state and solver.
4. **Sensorimotor mapping:** how receptor channels and motor outputs map to the body.
5. **Body/environment model:** morphology, muscle mechanics, contact and substrate.
6. **Behavioural evidence:** held-out trajectories, tasks, perturbations and individual variation.

Do not write an effective coupling as an anatomical synapse. Do not treat a fitted model weight as a measured synaptic efficacy. Do not infer a physical body response from a neural output alone.

Symthaea's current legacy C. elegans module is a seeded synthetic 279-entry topology fixture, not a source-verified connectome. It is not an admissible biological input to this adapter. See Symthaea #9902 and its documentation-correction candidate PR #9903.

## 6. Validation ladder and staged vertical slices

### Stage A — interface/oracle tests (can proceed before the perception gate)

Use synthetic fixtures only to qualify:
- unit/range validation and overflow handling;
- stable input ordering and canonical digests;
- no-world-truth input boundary;
- deterministic fixed-step replay;
- circuit-state reset/checkpoint behavior;
- invalid/stale/unsupported/abstention behavior;
- motor proposal bounds and body-effector mismatch rejection;
- physics authority cannot be mutated through adapter outputs.

Synthetic qualification does not establish animal biology.

### Stage B — C. elegans scientific control

Use C. elegans first because it offers tractable neural and whole-body modeling references. Pin an exact, rights-cleared 302-neuron dataset/release and keep structural anatomy, perturbation/effective coupling, dynamics, motor layer, body model and sensory mapping separate.

Target a small chemotaxis arena with:
- odor concentration and gradients;
- receptor sampling and temporal sensing;
- locomotion, reversals and turns;
- body/substrate contact and proprioception;
- held-out field geometry, initial states and perturbation targets.

Compare to:
- BAAIWorm's brain/body/environment model;
- the exact versioned 2026 Worm-whisperer preprint/code configuration;
- a simple scripted/utility controller;
- a connectome-constrained dynamics baseline;
- a hybrid controller.

Retain critical negative controls, especially sensory-only input, shuffled topology, no odor/no valid receptor, disabled proprioception, motor/muscle stimulation leakage and out-of-scope field geometry. Do not optimize a published failure away without recording that the evaluated model has changed.

Reference implementations:
- BAAIWorm (2024): https://www.nature.com/articles/s43588-024-00738-w
- C. elegans signal propagation atlas (2023): https://www.nature.com/articles/s41586-023-06683-4
- Worm-whisperer (2026 preprint/code): https://doi.org/10.64898/2026.09.06.749731 and https://github.com/kairess/worm-whisperer

### Stage C — Drosophila sensorimotor circuit

Use NeuroMechFly v2 / FlyGym as the neuromechanical comparator. The 2026 FlyGym 2.x rewrite materially changes its API and performance, so exact version, solver and hardware must be pinned for each comparison.

Start with a single source-supported circuit and task (for example walking feedback, head stabilization, visual following or odor navigation). Use the whole connectome only where required by the declared question; do not run a full 100k+ neuron model for every game animal and every frame.

Relevant references:
- NeuroMechFly v2 paper: https://www.nature.com/articles/s41592-024-02497-y
- FlyGym project / version history: https://github.com/NeLy-EPFL/flygym
- Adult female whole-brain connectome: https://www.nature.com/articles/s41586-024-07558-y
- Brain-and-nerve-cord connectome: https://www.nature.com/articles/s41586-026-10735-w
- FlyLite 2026 preprint on task/state-specific compression: https://doi.org/10.64898/2026.09.19.752860

### Newly published walking-CGP reference (28 October 2026)

Pugliese et al., *Connectome simulations identify a central pattern generator for fly walking*, Nature (version of record 2026-10-28), DOI: https://doi.org/10.1038/s41586-026-11195-y, is now the most concrete peer-reviewed neural-locomotion target for this programme.

The paper reports:
- DNg100 as the strongest descending driver of rhythmic leg motor activity under its declared model and screening procedure;
- a minimal rhythm-generating core with one inhibitory and two excitatory interneurons;
- reported necessity and sufficiency of that three-interneuron circuit for motor rhythms across six legs and four connectome datasets;
- a distinct DNb08 descending pathway predicted by simulation and confirmed with optogenetic experiments in behaving flies.

This gives us a source-grounded insect motor-circuit case, but not a universal CPG, whole-animal behavioural model or ready-made controller for other taxa. A local circuit producing rhythmic motor output is not by itself evidence of stable walking: the downstream muscle/limb mapping, contact mechanics, body dynamics and sensory feedback must still be tested.

The paper's exact input identities must be version-pinned:
- MANC public release v1.2.1;
- FANC CAVE materialization v1178 (2025-01-07 timestamp);
- male CNS public release v0.9;
- BANC CAVE materialization v829 (2026-02-13 timestamp).

The BANC resource is a dense reconstruction spanning the adult female brain and ventral nerve cord. Do not call it a universally complete Drosophila CNS connectome: the paper itself describes fly reconstructions separately from the taxa whose complete connectomes are available. Preserve the distinction between a dense reconstruction, a region-complete reconstruction and a complete organismal connectome.

The authors' code is https://github.com/smpuglie/Pugliese_2026. The paper cites Zenodo record 22260923 for simulation outputs, while the repository README (still describing the earlier preprint) points to 22260924. Resolve the exact record/version and checksums before treating either artifact as the canonical reproduction source. Symthaea owns the exact scientific reproduction in [DROSOPHILA-WALKING-001 / #9934](https://github.com/Luminous-Dynamics/symthaea/issues/9934); Symtropy owns the body/contact qualification in [ANIMA-NEURO-002 / #3345](https://github.com/Luminous-Dynamics/symtropy/issues/3345).

### Critical limitation: motor rhythm is not coordinated walking

The Pugliese et al. Nature paper (2026-10-28) reports that its DNg100-driven rate-model simulations generate rhythmic leg motor-neuron activity but **do not generate the tripod inter-leg coordination pattern** characteristic of walking flies. The authors explicitly point to proprioceptive feedback, biomechanical coupling and/or other neural pathways as likely missing mechanisms. This is a direct warning against treating an oscillating connectome circuit as a completed gait controller.

The Symtropy benchmark must report three separate outcomes:

1. **Neural rhythm:** per-leg/per-module rhythmic motor output and perturbation response.
2. **Coordination:** correct inter-leg phase relationships, including tripod pattern where the reference species/task requires it.
3. **Embodied locomotion:** stable body translation, foot contact, stance/swing, slip resistance and recovery.

The controller must not receive a tripod-phase target as hidden world truth. Coordination should emerge from qualified circuitry, actual proprioceptive input, and/or mechanical coupling. Include the CPG-only case without proprioception/contact feedback as a negative control, then compare:
- CPG only;
- CPG + proprioceptive/mechanosensory feedback;
- CPG + articulated-body/contact coupling;
- CPG + both feedback and biomechanical coupling;
- published NeuroMechFly/FlyGym controller modes (exact version);
- a conventional hand-designed tripod oscillator / rule controller.

Report per-leg phase error, tripod phase error, duty factor, support/contact sequence, slip, body velocity and stability. A reference oscillator that matches average frequency but not phase coordination must fail the coordination task.

Do not force one universal three-neuron core across every dataset. The published full text describes MANC pruning convergence to E1/E2/I1; FANC often identifies a four-interneuron circuit that includes E1/E2 plus a different inhibitory and an additional excitatory cell; full CNS datasets include E1/E2 with either I1 or I2. The shared recurring motif is more robust than an assertion that all per-dataset minimal circuits are identical. Exact per-dataset node identities and retained edge sets must be pinned.

The reproduction should also retain the paper's separate evidence classes: DNg100 model screening and intensity prediction, DNg100 stimulation experiments in headless flies, and DNb08 optogenetic confirmation are different observations and must not be pooled.

### Central/peripheral coordination comparator: preserve the preprint boundary

Sapkal et al., *Central versus peripheral neural control of a coordinated walking pattern in Drosophila*, bioRxiv 2026.04.29.721658 v2 (preprint), DOI https://doi.org/10.64898/2026.04.29.721658, reports evidence for local CPG modules in each leg, proprioceptive/load feedback shaping intra-leg stepping, and central coupling plus proprioceptive and descending inputs modulating inter-leg coordination. In tested conditions, substrate contact and proprioceptive feedback help restore stronger coordination; co-stimulation of DNg100 with other descending neurons changes stepping frequency.

This is a complementary preprint comparator, not a peer-reviewed result and not a replication of Pugliese et al. Its proposed five-neuron kinematics-guided motif must not be assumed to be identical to, nested within, or an updated name for Pugliese's three-neuron E1/E2/I1 rhythm core. Verify exact source cell IDs and edges before making any correspondence claim.

This study suggests an additional controlled matrix:
- reduced proprioceptive feedback versus intact feedback;
- no substrate/contact versus substrate/contact;
- DNg100 alone versus separately declared DNg97/DNg75/MDN or co-stimulation conditions;
- per-leg local phase/frequency versus inter-leg coordination (phase-locking value and tripod coordination strength);
- contact/load feedback contribution to intra-leg step-cycle shape.

Preserve each study's exact genotype, sensory-deprivation protocol, contact condition, cell identity, connectome snapshot, sample size, stimulus and analysis. No direct code reproduction is assumed; the preprint states code will be made available after publication.

### Existing industry baseline: CPG + rules + sensory feedback already exists

NeuroMechFly v2 is already a strong baseline and prevents a false novelty claim. Its 2024 paper and documentation expose:
- a coupled-oscillator CPG controller that can synchronize to a tripod gait;
- a decentralized, local rule-based controller;
- a hybrid CPG plus sensory-feedback controller that corrects stumbling/overstretch and is more robust on complex terrain;
- mechanosensory access to joint angles, actuator forces and contact forces;
- higher-level vision/olfaction/RL navigation and connectome-constrained visual processing.

Therefore, "CPG plus proprioception" is not the differentiator by itself. Symtropy's research claim should be tested as the incremental value of **source-grounded connectome-constrained circuit dynamics + persistent ANIMA individual state + real body/contact feedback**, compared against NeuroMechFly's hand-designed CPG/rule/hybrid, learned recurrent and utility/PID baselines under matched observations, bodies, physics, tasks, hardware and compute.

FlyGym 2.0 was rewritten in March/April 2026 and reports roughly 10× CPU and 250–300× GPU simulation speedups depending on the official page/release. Pin the exact v2 API/release; it is not backward compatible with v1. Treat its throughput as a measured reference, not a promise Symtropy will exceed it.

### Gait coordination metrics are species-, speed- and task-conditioned

Do not collapse gait quality into one scalar or force a single gait pattern across taxa.

For Drosophila specifically, one published eLife study reports tripod-like coordination across its tested speeds. Keep speed strata because step timing, variability and coordination strength still matter, but do not assume low-speed non-tripod gait in this species without a matching measured reference. Other insects can show wave, tetrapod or tripod-like coordination depending on species, speed and terrain.

For each Drosophila trial, report separately:
- per-leg neural/motor rhythm frequency and variability;
- realized step frequency, stance/swing duration and duty factor;
- pairwise phase differences and PLV with the exact phase-extraction pipeline recorded;
- TCS for both tripod groups (L1-R2-L3 and R1-L2-R3);
- speed, turning, substrate/contact, slip, body stability and individual repeatability.

**TCS definitions are study-versioned and must not be conflated:**
- `TCS_SAPKAL_2026_V2`: Sapkal et al. bioRxiv v2 defines instantaneous TCS as overlapping swing duration divided by (onset of last swing minus onset of first swing), reports the per-trial median and assigns zero to steps not in a tripod event.
- `TCS_NATVAR_2024`: the 2024 *Natural variability and individuality of walking behavior in Drosophila* paper defines TCS as duration when all three tripod legs are simultaneously in swing divided by the interval from earliest swing onset to latest swing termination.

The denominators differ. These metrics are not interchangeable. Reproduce each paper under its own metric ID, event segmentation, sampling interval and edge-case rules; recompute from the same raw events before cross-paper comparisons.

Sapkal et al.'s methods describe PLV using AEP–PEP joint-angle signals band-pass filtered at 4–15 Hz with a fifth-order Butterworth filter, instantaneous phase from a Hilbert transform, then pairwise inter-leg phase coupling. The text describes mean absolute phase offset; do not substitute a different conventional PLV formula without a different metric ID and an explicit method audit.

The Sapkal preprint's results are also condition-dependent:
- DNg100 alone showed tripod-like coordination in its tested conditions, with its highest reported TCS in air.
- Under selected descending-neuron co-stimulation conditions, different leg frequencies in air could reduce coordination; with substrate contact, mechanical and proprioceptive/load feedback could align stepping frequencies and improve coordination.
- Other co-stimulation conditions changed stepping frequency and coordination in different directions.

Therefore benchmark central stimulation, proprioceptive feedback and substrate/contact as separate factors. Do not claim that adding contact always increases TCS.

Target distributions should be stratified by speed, turning, terrain/material, contact, descending inputs, sensory feedback, body condition, individual and study. A high tripod score is not a universal objective; match the qualified species/task reference distribution.

### Current FlyGym baseline: pin 2.1.1 and distrust stale throughput numbers

The official FlyGym release page currently lists v2.1.1 (released 24 October 2026; release commit `a63dc91`). This is the baseline version for the comparative programme, not an unspecified "FlyGym 2.x".

Release 2.1.1 contains several benchmark-critical changes:
- GPU benchmark/tutorial timing boundaries were fixed because prior measurements could overstate throughput by up to 50%.
- MuJoCo and MuJoCo Warp are v3.16.
- GPU constraint solver default is `1e-5`; `1e-6` could trigger float32 convergence-check stalls in a small fraction of worlds. With 1,000 worlds, the slowest world can hold up the entire batch and produced about 25% slowdown.
- Iteration-limit warnings are suppressed in GPU batch mode, but contact and constraint buffer-overflow warnings remain visible. Record these as explicit invalid-run signals; absence of an iteration warning does not imply no stall.
- `GPUSimulation.reset()` does not recompute all derived kinematic quantities; call `forward()` before consuming those derived states.
- Trajectory recording stores `qpos` and mocap poses at a declared video rate and allows post-hoc rendering, separating rendering cost from physics stepping.
- Release 2.1.1 changes body API details and fixes GPU timing; don't compare older v2.0 timings as current throughput.

Comparative protocol:
1. Pin exact FlyGym release, dependency lock, Python/runtime, MuJoCo/Warp, GPU/driver and scenario assets.
2. Keep warm-up/compilation outside the measured region.
3. Synchronize GPU work before and after the timed region.
4. Measure physics-only throughput with rendering disabled; use trajectory recording and post-hoc rendering for visual comparison.
5. Pin world count, simulation timestep, solver tolerance and control cadence; measure CPU and GPU separately.
6. Reproduce the official 2.1.1 `1e-5` tolerance as its primary GPU configuration, then perform a tolerance sensitivity/convergence run. Symtropy must declare and converge its own physics solver; tolerances across different solvers are not interchangeable.
7. Preserve solver/contact/constraint overflow warnings, per-world failures and slowest-world stalls in receipts.

The matched comparison protocol is tracked in [LOCO-BENCH-001 / #3357](https://github.com/Luminous-Dynamics/symtropy/issues/3357). It compares CPG-only, rule-based, hybrid, RL, phenomenological and connectome-constrained controllers on identical bodies, terrain, observations, compute and seeds.

Source: https://github.com/NeLy-EPFL/flygym/releases/tag/v2.1.1

### Calibration and identifiability: do not fit behaviour alone

Two 2026 preprints reinforce an important methodological constraint:

- PGOB reports parameter generation for whole-organism worm/fly simulators from tracking video, using behavioural observables and Jacobian sensitivity information: https://doi.org/10.5281/zenodo.20691878
- *How much of a nervous-system model does behaviour identify?* reports that behavioural data can leave most parameters in connectome-scale models weakly constrained, and that matching behaviour does not necessarily recover true parameters: https://doi.org/10.5281/zenodo.22120856

Treat both as version-pinned preprint comparators, not established universal laws. Their joint lesson for our evaluation is still important: separate **behavioural equivalence**, **parameter recovery**, and **mechanistic identification**. Use multiple observables (pose, kinematics, contact/forces, physiological or neural activity where available) and targeted circuit perturbations. Report sensitivity and unidentifiable parameter directions; do not claim that a trajectory fit recovered the true connectome or physiology.

### Stage D — deer / visible game wildlife

A deer is a useful gameplay vertical slice, but no validated deer connectome controller is claimed here.

Use the existing evidence-aware hybrid:
- sourced body and locomotor profile;
- non-omniscient sensing, occlusion and uncertainty;
- ANIMA homeostatic drives and persistent individual history;
- physical balance, contact, slip and fatigue;
- social information transferred only through observable signals;
- scripted/utility mechanisms clearly labelled where biological circuit evidence is unavailable.

A deer should orient, freeze, investigate, resume feeding, flee or rejoin a group based on available evidence, body condition, memory and affordances. A fixed player-distance threshold or hidden `herd_alert` flag is not an acceptable causal explanation.

## 7. Baselines and comparison discipline

For each task, use the same body, physics settings, initial conditions, sensor access, observations, split, time horizon, hardware and declared compute budget.

Compare:
1. scripted state machine / behaviour tree;
2. utility arbitration + classical reflex/PID control;
3. learned recurrent policy / RL controller;
4. connectome-constrained dynamical controller;
5. hybrid ANIMA + connectome/local-reflex controller;
6. published task-specific reference, where a meaningful reproduction is possible.

A comparator is not interchangeable merely because it produces a similar animation. Record all differences in body, sensors, model capacity, training budget, solver and hardware. Do not claim SOTA from visual demos or non-matched results.

## 8. Benchmark metrics and proposed targets

These are proposed targets. They are not achieved results or universal biological thresholds.

### Structural and provenance integrity
- 100% of source-backed nodes/edges resolve to the pinned artifact or remain explicitly unknown.
- Zero fabricated anatomical edges in source-backed models.
- Zero synthetic-to-biological promotion.
- Exact source, parser, model, code, solver, scenario and split digests.

### Neural response
- Held-out response sign, amplitude, latency and trajectory error with uncertainty.
- Report direct-edge, multi-hop, perturbation/effective and extrasynaptic components separately.
- Split by specimen, intervention target, stimulus, state and study wherever data permit.
- Reproduce a pinned published baseline before claiming improvement.

### Locomotion and embodiment
- Trajectory-distribution distance, path efficiency, speed/acceleration, turning curvature, gait phase/frequency, posture, contact timing and recovery.
- Task success and failure rates with confidence intervals.
- Zero non-finite forces/activations and zero invalid joint states; collision/contact tolerances must be pinned to the solver.
- Test slip, uneven terrain, fatigue, sensory noise, occlusion and body-parameter variation.

### Causal tests
- Preregistered circuit ablation/perturbation predicts the direction and timescale of behavioural change on held-out cases.
- Include shuffled-wiring, polarity/weight permutation, sensor disconnection and motor-access leakage controls.
- Zero hard-gate false acceptance on the frozen synthetic oracle.

### Generalization and individual variation
- Hold out environment geometry, field conditions, start states, body parameters, individuals and (only where justified) species/life stages.
- Same species and stimulus may yield a distribution of plausible outcomes; report individual-history sensitivity.
- Unsupported cross-taxon transfer remains `INDETERMINATE`, not a pooled pass.

### Runtime and fidelity
- Report p50/p95/p99 controller, body/physics and perception cost separately, plus memory and energy.
- Pin target hardware and scene load before comparison.
- Gameplay objective: keep total frame time within 16.67 ms for a declared 60 Hz scene at the reference hardware; report measured near/mid/far animal counts rather than promising a headcount in advance.
- Stretch research target: preserve at least 95% of a preregistered held-out behaviour score with a 10× reduction in controller compute for the mid-field tier, without violating any hard gate. This target must be tested, not assumed attainable.

### Human-visible believability
Keep:
- Q0 static contract/type evidence;
- Q1 exact deterministic execution;
- Q2 differential/invariance tests;
- Q3 empirical/causal behaviour;
- Q4 presentation legibility;
- Q5 blind human/player evaluation

as different claims. Good animation is not evidence of correct neural dynamics; a validated neural model is not automatically readable or enjoyable.

## 9. Multi-fidelity controller strategy

- **Near / research:** detailed neural dynamics for selected animals, full body/contact mechanics and high-rate feedback.
- **Mid-field:** distilled/reduced circuit controllers qualified against full-model behaviour on held-out tasks, network states and perturbations.
- **Far-field:** ecological/population aggregates for distant animals, preserving future-bearing identity/history/physiology and deterministic re-entry.

Compression must be task- and state-conditioned. FlyLite demonstrates why average-task preservation or weak-edge pruning alone is insufficient: circuit functions can collapse differently by task and activity state. A compressed model is a task-bounded surrogate, not a more correct biological connectome.

Fidelity changes may not erase learned fear, memory-relevant history, injuries, relationships, territory or any state that changes future behaviour.

## 10. Promotion gates and dependencies

- Keep this document, synthetic fixtures and interface design moving now.
- Do not begin production perception/memory integration before #249's existing exact-head oracle prerequisite is satisfied by a reviewed executable-green successor.
- Do not treat the draft #2145 animal profile/observation/replay implementation as qualified until exact-head format, compile, tests and Clippy evidence all pass.
- Do not use Symthaea's current synthetic 279-node graph as biological controller input.
- Python research implementations may be comparators, but no production Python runtime dependency is introduced.
- Queued, skipped, stale, cancelled or wrong-head checks are not PASS.
- No live-animal experiment, intervention or data collection is authorized by this contract.

## 11. Non-goals

- A universal neural controller for all taxa.
- A full detailed connectome for every in-scene animal.
- A deer connectome claim.
- Direct neural output to renderer/animation transforms.
- Hidden-world omniscience or second canonical world state.
- Autonomous veterinary, wildlife-management or conservation actions.
- Bulk dataset scraping or redistribution outside exact licensed scope.
- A SOTA claim before same-task, same-budget, held-out comparative evidence.
