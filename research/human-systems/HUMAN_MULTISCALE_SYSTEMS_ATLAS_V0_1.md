# Human multi-scale systems atlas and Symtropy–Symthaea coupling v0.1

**Status:** research architecture and source-audited gap map. This is not an implemented whole-body model, a human digital twin, or a clinical system.  
**Programme:** [HUMAN-SYSTEMS-SIM-001 / #5625](https://github.com/Luminous-Dynamics/symtropy/issues/5625)  
**Implementation issue:** [HUMAN-MULTISCALE-COUPLING-001 / #5772](https://github.com/Luminous-Dynamics/symtropy/issues/5772)  
**Companion work:** [evidence-boundary runner #5730](https://github.com/Luminous-Dynamics/symtropy/issues/5730), [identifiability coverage #5740](https://github.com/Luminous-Dynamics/symtropy/issues/5740), [Symthaea observation events #12407](https://github.com/Luminous-Dynamics/symthaea/issues/12407)

## 1. Decision: yes to more biology, no to a monolithic virtual human

We should model more biological systems, but only through a **modular, multi-scale, context-of-use architecture**. We should not attempt to simulate every cell, molecule, organ and thought simultaneously. Biology spans different mathematical regimes and characteristic scales; a recent review describes biological organization over roughly ten orders of spatial scale and eighteen orders of temporal scale. A single solver, state vector or update rate is not an adequate abstraction for that hierarchy.

A useful human model is not a collection of independent organ dashboards. It is a graph of interacting dynamical systems:

- local biochemical reactions and cellular regulation;
- cell populations, extracellular matrix, tissue mechanics and transport;
- organ-level function;
- cross-organ material, energy, gas and control loops;
- whole-organism homeostasis and adaptation;
- perception, interoception, cognition, action and behaviour;
- environmental and social context.

Emergence is a property of coupled dynamics, not a feature we get merely by using the words fractal, agent-based, hyperdimensional or self-organizing. The model must identify which local rules and couplings produce a particular macro-level behaviour, and validate that behaviour at the scale where the claim is made.

**Design rule:** implement the smallest set of mechanisms that can answer the declared question. Add a coupling only when removing it changes the answer in a falsifiable way.

## 2. Nested organization: the modelling hierarchy

| Level | What is represented | Useful mathematical families | What can emerge |
|---|---|---|---|
| L0 — molecular / biochemical | Molecules, binding, reaction rates, metabolites, ions, hormones, cytokines, energy carriers | Reaction networks, ODEs, stochastic kinetics, SBML | Feedback, switching, oscillation, pathway adaptation |
| L1 — subcellular / cellular | Gene regulation, signalling, membrane potentials, calcium, cell metabolism, cell states | ODE/PDE, stochastic networks, logical models | Excitability, differentiation, secretion, cell-state transitions |
| L2 — cellular population / tissue | Cell populations, extracellular matrix, perfusion, diffusion, oxygen, local mechanics and immune trafficking | PDE, agent-based models, cellular automata, finite elements, hybrid systems | Pattern formation, repair, inflammation, remodeling, collective mechanics |
| L3 — organ | Heart, lung, kidney, liver, gut, pancreas, brain, skeletal muscle, adipose, skin, bone, marrow | Lumped-parameter models, ODE/PDE, continuum mechanics, organ-specific solvers | Organ function and regulation |
| L4 — organ-system / whole-body | Circulation, gas transport, renal-fluid, endocrine, immune, metabolic, thermoregulatory and autonomic feedback | Coupled ODE/network models, hybrid discrete-event models, transport networks | Homeostasis, compensation, cross-organ trade-offs |
| L5 — organism / embodied person | Integrated physiology, sensorimotor function, sleep-wake, energy, adaptation and recovery | Multi-rate hybrid dynamical system, reduced-order models | Whole-person response to workload, environment and perturbation |
| L6 — cognitive / behavioural agent | Beliefs, goals, attention, memory, interoceptive inference, policy and action | Active inference, state estimation, decision processes, temporal models | Behaviour conditioned on estimated internal and external state |
| L7 — environment / social context | Lighting, temperature, air, food/water, workload, built environment, social schedules and resource availability | Spatial physics, event systems, networks, population models | Exposure, interaction, collective outcomes |

These levels are not a rigid one-way pipeline. Higher levels can regulate lower levels: autonomic and endocrine signals change cellular states; behaviour changes physical activity, intake, sleep and environmental exposure; environmental conditions perturb immune and thermoregulatory systems. Lower-level processes generate higher-level behaviour, but reverse influences must be represented as explicit causal inputs, not hidden corrections.

### 2.1 Different regimes need different solvers

Use the model class that matches the underlying process:

- **Reaction / pathway:** biochemical rates and conservation of species.
- **Cell:** stochastic state transitions, signalling and electrophysiology.
- **Tissue:** spatial transport, mechanics, growth and agent interactions.
- **Organ:** reduced-order ODE / lumped-parameter models where spatial detail is not decision-relevant.
- **Whole body:** coupled subsystem networks with explicit feedback and conserved flows.
- **Behaviour:** event-driven policies over observations and beliefs, not a second hidden physiology solver.
- **Environment:** spatial and rigid-body physics, fluid/thermal fields and event scheduling as needed.

Use model reduction and homogenization when many microstates can be represented by an effective macro-variable without losing the phenomenon under study. Preserve the omitted detail as a declared approximation and test whether it changes the result. Do not couple a cell-scale PDE to every whole-body update merely because both are available.

## 3. Subsystem inventory and current gap assessment

Status labels:
- **Implemented (bounded):** code exists for the named limited mechanism.
- **Reference / research:** equations, data or external software are identified, but no qualified Symtropy subsystem is claimed.
- **Absent / unqualified:** no validated implementation is currently claimed for that system.

| Subsystem | Key state / outputs | Critical coupling partners | Symtropy status | Symthaea status |
|---|---|---|---|---|
| Cardiovascular / vascular / microcirculation | Pressure, flow, volume, resistance, perfusion | Respiratory, renal, endocrine, thermoregulation, tissue metabolism | Absent / unqualified | No in-repository cardiovascular plant model; JOS-3 is an external reference candidate with internal blood-flow/cardiac-output outputs, not a Symthaea implementation |
| Respiratory / gas exchange | Ventilation, lung volumes, O₂/CO₂ transfer, blood gases | Cardiovascular, acid-base, metabolism, brainstem control | Absent / unqualified | No integrated ventilation, lung mechanics or gas-exchange physiology identified |
| Renal / fluid / electrolyte / acid-base | Filtration, reabsorption, water, sodium/potassium, pH, clearance | Cardiovascular, endocrine, blood chemistry, GI | Absent / unqualified | No renal filtration, tubular transport, electrolyte or fluid-regulation model identified; do not infer hydration from a sweat proxy |
| Hepatic / GI / pancreatic | Digestion, absorption, hepatic processing, bile, glucose regulation | Microbiome, metabolic, endocrine, immune, renal | Absent / unqualified | Chemistry substrate exists, but no integrated digestive/hepatic physiology |
| Energy / glucose / insulin / adipose | Glucose, glycogen, substrate flux, insulin/glucagon, adipose stores | Liver, pancreas, muscle, endocrine, brain, activity | Absent / unqualified | BMR/TDEE/macronutrient arithmetic exists; no dynamic glucose–insulin, glycogen, substrate-flux or adipose-control model |
| Endocrine / neuroendocrine | Hormone states, secretion, feedback, receptor response | Brain, kidney, liver, immune, metabolism, circadian | Absent / unqualified | Neuromodulator bath includes a cortisol variable and coarse stress/PNI links; no validated gland, plasma hormone, receptor or endocrine feedback kinetics |
| Nervous / autonomic / hypothalamic | Neural control signals, autonomic tone, reflex loops | Cardiovascular, respiratory, GI, endocrine, immune, thermoregulation | Absent / unqualified | FEP is a candidate cognitive architecture; Soma maps phone sensors to neuromodulator nudges, but no validated autonomic, vagal, hypothalamic or organ-reflex plant model is identified |
| Immune / inflammatory / hematopoietic / lymphatic | Cell populations, cytokine signals, immune activation, blood/lymph transport | Tissue, liver, gut, endocrine, vascular, nervous | Absent / unqualified | Neuromodulators has a PNI cytokine-state coupling (normalized [0,1], cognitive-cycle dynamics); epidemiology has population SIR. Neither is a cellular, hematopoietic, lymphatic or concentration-calibrated immune plant |
| Thermoregulation / integument / sweating | Core/surface heat, skin blood flow, sweating, heat storage | Cardiovascular, renal-fluid, muscle, environment | Absent / unqualified | Generic Fourier conduction/Newton cooling primitives and device thermal throttling exist; no regional human skin/core, vasomotor, sweat or thermoregulatory controller model |
| Musculoskeletal / neuromuscular / connective tissue | Joint state, muscle activation/force, tendon strain, bone loading | Nervous, cardiovascular, respiratory, metabolism, skin | Generic rigid-body physics only; no qualified muscle/tendon/soft-tissue physiology | Humanoid/robotics are not a human muscle physiology model |
| Sleep / circadian / recovery | Homeostatic sleep pressure, circadian phase, light response, sleep events | Nervous, endocrine, metabolism, immune, behaviour, environment | Bounded Process S + fixed harmonic Process C; light and observation research tracks | Nurture schedule logic is not a validated adult physiological sleep engine |
| Reproductive | Reproductive hormones, cycle or developmental state, pregnancy physiology where relevant | Endocrine, cardiovascular, renal, immune, metabolism | Absent / unqualified | No integrated reproductive physiology identified |
| Microbiome / host-microbe | Community state, metabolites, barrier signalling | GI, liver, immune, endocrine, nutrition | Absent / unqualified | Organic chemistry is structural chemistry, not a host-microbiome model |
| Sensory / measurement | Sensor state, calibration, capture, uncertainty, missingness, delivery | Every latent subsystem; Symthaea observation update | HOBS synthetic sensor-world and sampling-rate fixtures | FEP observation semantics need #12407; Soma has phone motion/light/barometer/GPS/proximity inputs and touch events, but these are not calibrated internal-organ/interoceptive measurements |
| Cognition / behaviour | Beliefs, policy, attention, action, task switching | Physiology, sensed evidence, goals, environment | Behaviour is not Symtropy's authority | Symthaea is the candidate agent/controller, not a validated human cognition model |
| Social / built environment | Social contact, task schedules, food/water/light access, environmental exposure | Behaviour, immune exposure, circadian, resources | Spatial/environment substrate exists; human exposure semantics incomplete | Social reasoning is not empirical population physiology |

### 3.1 Existing Symtropy living-system substrate (reuse, do not overclaim)

Symtropy already has `symtropy-lifesim-core`, a CPU-first deterministic 2D field substrate. Its current field channels include moisture, nutrient, toxin, biomass, heat, light, oxygen and disease, alongside ecology-specific pheromone, obstacle and signal-noise channels. It provides explicit finite-difference diffusion/decay with source terms, obstacle boundaries, an explicit 2D stability guard (`diffusion × dt ≤ 0.25`), Arrow snapshots and an optional WGPU backend. A generic `Metabolism` structure tracks normalized energy, hydration, oxygen need, heat tolerance, toxin tolerance, hunger and recovery parameters.

This is an unusually useful starting point for **spatial environmental and tissue-microenvironment scaffolding**, particularly when a future tissue model needs fields and local transport. It is not yet a human tissue model:
- field values are generic `f32` concentrations/normalized channels, not typed SI quantities or calibrated molar concentrations;
- oxygen is a scalar field, not oxygen partial pressure, blood oxygen content, saturation, or perfusion;
- moisture/hydration are generic ecological variables, not plasma/interstitial volume or total body water;
- heat is a channel, not a validated multi-node human thermal state or a thermoregulation controller;
- generic `Metabolism::stress_from_fields` is a normalized stress heuristic, not organ energy balance;
- the current field is 2D and has no human tissue geometry, vascular network, cell identities, extracellular matrix constitutive law or organ-specific boundary conditions.

**Recommended reuse path:** keep LifeSim as a deterministic spatial-field kernel. Add a human/tissue-specific adapter only after its field units, geometry, conservation properties, boundary conditions, source/sink terms and solver convergence are defined. The same channel name must not imply the same physical quantity across ecological and human domains.

This table is deliberately conservative. A crate name or equation does not qualify the system named in the crate's README. Current Symthaea building blocks include: (1) BMI, BMR, TDEE and macronutrient arithmetic plus one-compartment first-order pharmacokinetics in symthaea-physiology; (2) generic engineering heat-transfer and fluid equations in symthaea-thermofluids; (3) an SIR population infectious-disease model in symthaea-epidemiology; (4) a normalized, cognitive-cycle PNI/cytokine-to-neuromodulator coupling in symthaea-neuromodulators; and (5) mobile device sensor bridges and a compute wake-state machine in symthaea-soma. These are useful primitives with different contexts of use, not an integrated human physiological plant. Symtropy's current human-systems crate has a bounded Process S / harmonic Process C sleep reference plus synthetic observation and intervention-event fixtures. Neither codebase currently contains a validated coupled whole-body model.

## 4. High-value coupling graph

The first-order graph should contain explicit directed ports, not an assumption that every system talks to every other system.

| Coupling edge | Physical / biological exchange | Why it matters | Initial validation idea |
|---|---|---|---|
| Cardiovascular ↔ respiratory | Perfusion, ventilation, oxygen and carbon-dioxide transport | Oxygen delivery depends on both circulation and gas exchange | Coupled O₂/CO₂ mass balance and workload response |
| Cardiovascular ↔ renal | Pressure, perfusion, volume, sodium/water balance | Fluid regulation changes circulation; perfusion affects kidney function | Volume and pressure perturbation; balance residuals |
| Renal ↔ endocrine | Renin–angiotensin–aldosterone and water/electrolyte control | Hormonal regulation alters volume, pressure and electrolytes | Closed-loop response and parameter sensitivity |
| GI ↔ liver ↔ pancreas ↔ metabolism | Nutrient absorption, portal transport, glucose/insulin and substrate flows | Food intake and hepatic/pancreatic responses shape energy balance | Meal-response mass/energy and glucose known-answer cases |
| Liver ↔ kidney ↔ blood | Metabolite/drug clearance, plasma transport | Substance exposure and clearance are distributed processes | One- and multi-compartment transport consistency |
| Skeletal muscle ↔ cardiovascular / respiratory | Mechanical work, O₂ demand, CO₂ production, heat | Activity changes circulation, ventilation and metabolism | Workload ramp and recovery against a declared reference |
| Muscle ↔ endocrine / metabolic | Glucose uptake, substrate use, insulin response | Exercise and energy balance are inseparable | Matched workload/substrate perturbation |
| Immune ↔ vascular / endocrine / metabolism | Cytokines, cell trafficking, acute-phase and metabolic changes | Inflammation has systemic effects, not only local tissue effects | Synthetic perturbation with explicit assumptions; later external data |
| Thermoregulation ↔ skin ↔ cardiovascular ↔ renal | Heat storage, skin perfusion, sweat and fluid loss | Heat dissipation changes circulation and hydration | Heat/water/energy balance under controlled environment |
| Circadian ↔ sleep ↔ endocrine / metabolic / behaviour | Phase, sleep pressure, light and time-dependent regulation | Biological timing modulates multiple subsystem responses | Light schedule and phase-response reference |
| Gut microbiome ↔ gut / liver / immune | Microbial metabolites and barrier/immune signalling | Host-microbe effects can propagate beyond digestion | Bounded metabolite-port model, not a full simulated microbiome |
| Nervous / autonomic ↔ organ systems | Neural drive, reflexes and autonomic regulation | Fast coordination across organ systems | Event/loop stability and reflex-response fixtures |
| Physiology ↔ behaviour ↔ environment | Interoceptive observation, task choice, activity, food, fluid, sleep, light and heat exposure | Behaviour changes the inputs the body experiences | Assignment → delivery → exposure → observation → response chain |

### 4.1 The first coupled demonstrator

Start with **workload + thermoregulation + cardiovascular response + respiratory demand + hydration**.

Why this slice:
- it exercises mechanics/workload, environment, several organ systems and behaviour;
- it contains clear transport and conservation relationships;
- it has fast and slow dynamics;
- it is relevant to operator ergonomics, assistive robotics and human-centred work environments;
- it can initially be tested with synthetic known-truth data without clinical claims.

Do not begin with immune + microbiome + endocrine + brain + every cell type as one giant integrated model. That creates a parameter-identification and validation problem before we have qualified the coupling contract.

## 5. How self-organization should be represented

A nested model needs to express local rules, transport and feedback explicitly.

### 5.1 Spatially local, globally coupled

Each component has:
- a local state and local dynamics;
- local neighbours or transport connections;
- boundary conditions supplied by a higher-scale model;
- outputs passed upward as aggregate fluxes, signals or mechanical responses;
- uncertainty about state and parameters.

For example, a tissue module may contain cell populations and a spatial extracellular field. Local cellular uptake alters substrate concentration; diffusion/perfusion replenishes it; tissue oxygen demand influences local metabolism; the tissue sends net flow/demand back to organ-level circulation. The organ-level model returns perfusion and boundary concentration. This is a bidirectional coupling, not one scalar "tissue health" value.

### 5.2 Use typed exchange mechanisms

1. **Conserved flux ports:** mass, volume, energy, charge, gas, nutrient and fluid flows.
2. **Signal ports:** neural, hormonal, cytokine and other control signals, each with units/meaning and kinetics.
3. **Mechanical ports:** forces, moments, displacement, strain and work.
4. **Boundary-condition ports:** pressure, concentration, temperature, perfusion and field boundaries.
5. **Observation ports:** measurements with capture time, receive time, calibration, uncertainty, mask and source.
6. **Action ports:** request, delivery, physical exposure and response are separate events.

Only ports whose mechanisms and units are justified may be connected. A hormone signal should not become an instantaneous dimensionless multiplier on every downstream organ; it needs a defined transport, delay, receptor/response and validity envelope if those details matter to the question.

### 5.3 Multi-rate integration is essential

Biochemical reactions, membrane dynamics, tissue transport, organ regulation, circadian rhythms and behaviour do not share one time step. The composed system needs:
- physical event time distinct from solver substeps and agent-cycle count;
- declared per-model integration method and allowed step range;
- explicit subcycling, interpolation/hold, event boundaries and synchronization;
- bounded delayed/out-of-order input handling;
- conservation checks across subsystem boundaries;
- no repeated physical update from a duplicated sensor packet.

An update-order or coupling-interval change is a benchmark perturbation, not an implementation detail that can be ignored.

## 6. Responsibility between Symtropy and Symthaea

### Symtropy: authoritative world and plant

Symtropy should own the simulated physical world and admitted physiological mechanisms:
- spatial environment, geometry, contact and mechanical work;
- admitted physiology submodels and deterministic state evolution;
- physical fluxes, conserved quantities, solver schedule and event replay;
- action assignment/delivery/exposure and simulator latent truth;
- scorer-side truth in synthetic qualification.

Symtropy physics should not be mistaken for anatomy merely because an avatar has joints and collisions. Human muscle, tendon, ligament, soft-tissue, vascular or organ mechanics need dedicated models/backends with declared validity.

### Symthaea: inference, cognition and candidate controller

Symthaea should own:
- interpretation of authorized observations through the existing FEP/cognitive architecture;
- state estimation and uncertainty;
- parameter/system identification where observable;
- active sensing and value-of-information;
- goals, candidate policies, memory, action selection and behavioural hypotheses;
- explicit abstention when the world is not identifiable.

Symthaea must not rewrite simulator ground truth, use scorer-only labels, or be scored against truth it has received directly.

### Neutral exchange contract

Use a versioned HumanSubsystemPort / HumanSystemEvent contract at the boundary. Keep two logically distinct streams:
- **plant coupling stream:** state/flux/signal ports between physiological subsystems and the physical environment;
- **observation stream:** sensor observations actually available to Symthaea.

The second stream should adapt to Symthaea #12407's observation-envelope work. It needs values and units, per-dimension observed masks, calibrated precision/uncertainty, modality/source, event/capture time, clock domain, receive/availability time, sensor health and late/duplicate semantics. A missing dimension contributes no likelihood; it is not a zero or prior-mean measurement.

Mycelix may later bind immutable model, dataset, configuration, rights and verification receipts. Provenance integrity does not make a biological assertion true and does not grant clinical authority.

## 7. Standards and reusable model ecosystem

Do not rewrite a hundred published models in Rust before testing the model question.

| Need | Candidate standard / resource | Use |
|---|---|---|
| Equation-based mathematical physiology | CellML 2.0 | Declare model components, variables, units, equations, imports and encapsulation |
| Biochemical reaction and regulatory networks | SBML Level 3 | Encode reactions and model types through supported packages |
| Modular model composition | SBML comp package; CellML imports/encapsulation | Reuse and compose model fragments without losing boundaries |
| Spatial / constraint-based / qualitative biochemical models | SBML spatial, FBC, qual and related packages | Select only when the model uses those semantics |
| Published physiological equations | Physiome / CellML Model Repository | Source-audited models across cardiovascular, electrophysiology, circadian, endocrine, metabolism, immunology, hepatology and more |
| Whole-body physiological comparison | BioGears | Candidate reference engine and subsystem methodology corpus; audit exact beta/release and validation limitations |
| Musculoskeletal mechanics | OpenSim | Candidate model backend for skeleton, joints, muscle-tendon forces and movement |

**Important comparator limitations:**
- BioGears documents many whole-body systems and modular lumped-parameter models, but its documentation states that a beta deployment does not include all intended functions and that the project is not aware of third-party validation; it also notes limits in its general sensitivity-analysis resources. Use it as a reference candidate, not an automatic truth oracle.
- OpenSim is a musculoskeletal modelling platform. It is not a whole-body biochemical or organ physiology engine.
- CellML/SBML encode models; they do not certify that the underlying biology is correct.
- Every imported model needs a pinned version, exact parameter set, equation/source provenance, licence decision, solver assumptions and independent test corpus.

## 8. Benchmarks and quality gates

Do not define a single human realism or whole-human SOTA score.

| Layer | Benchmark | Report |
|---|---|---|
| Module verification | Analytic cases, dimensional analysis, positivity/bounds, conservation, solver convergence | Maximum error in physical units; balance residuals; convergence order |
| Parameter identification | Synthetic truth, profile likelihood / posterior recovery, sensitivity and equivalence classes | Bias, interval coverage, identified-set width, non-identifiability rate |
| Coupling integrity | Boundary flux closure, update-order invariance, coupling-step sweep, delayed events | Mass/energy/volume residuals; instability; double-counting/leakage |
| Dynamic response | Controlled perturbation and recovery | Peak, time-to-peak, delay, overshoot, settling, recovery trajectory and uncertainty |
| Observation integrity | MCAR/MAR/MNAR, sensor drift, dropout, clocks, latency, sampling-policy shift | Capture coverage, error, calibration, selective risk and false certainty |
| Cross-domain transfer | Participant/device/task/site-held-out domains where licensed and relevant | Per-domain and lower-tail performance; no pooled concealment |
| Decision quality | Compare policies under a declared outcome/loss and defensible DGP | Value/regret, safety constraints and uncertainty; causal claims only with suitable evidence |
| Systems efficiency | Matched hardware/compute for full coupled workload | p50/p95/p99 latency, memory, energy per simulated second/event, adaptation cost |

### Minimum verification gates

- Dimensional analysis and unit conversion tests for every port.
- Conservation and balance residuals at every connected boundary.
- Positivity and physical-range constraints where applicable.
- Single-module analytic solutions and solver-step convergence.
- Multi-rate coupling-step convergence; results should not materially change merely because the update partition changes within the declared range.
- Sensitivity and parameter identifiability, including cases where the correct answer is "not identifiable".
- Deterministic replay for fixed versions, seeds, initial conditions and event streams.
- External held-out evaluation on data not used for fitting or model selection.
- Negative controls for hidden truth, scenario labels, missingness mechanisms, future data, duplicates and delayed observations.
- Explicit no-go output when the request requires a subsystem or validity range that is absent.

### Proposed engineering targets (not achieved results)

- Zero unexplained mass/energy/volume balance residual beyond declared solver and model tolerances in closed synthetic systems.
- No hidden truth or future-event access in candidate processes.
- No state mutation on rejected, unsupported or stale inputs.
- Verified convergence under pre-registered step-size and coupling-interval sweeps.
- Honest identification-region coverage and calibrated uncertainty on known-truth synthetic fixtures.
- Matched predictive and deployment comparisons before asserting any SOTA improvement.

Numerical tolerances must be derived from each model's formulation and solver, not copied as a universal percentage across incompatible physical quantities.

## 9. Staged implementation roadmap

### Stage 0 — atlas and contract
- Keep the subsystem inventory and coupling register versioned.
- Audit source availability, data rights, model licences and validation receipts.
- Specify port units, event time, flux direction and model validity for one demonstrator.

### Stage 1 — minimal coupled demonstrator
- Workload / movement input with declared mechanical work.
- A bounded energy-demand reference.
- Thermoregulation / heat exchange.
- Cardiovascular and respiratory response references.
- Hydration / fluid-balance state.
- Synthetic sensor and observation process.
- Symthaea policy that can continue, pause, rest or request another measurement.
- Evaluator-side known truth and identifiability controls.

No medical alert, safe-to-work or treatment recommendation.

### Stage 2 — metabolic and renal extensions

Add GI/nutrient absorption, liver/pancreatic glucose handling, renal water/electrolyte and clearance only with explicit interfaces and appropriate reference equations. Verify that the new coupling improves a declared prediction or decision rather than merely adding model state.

### Stage 3 — immune, endocrine and tissue dynamics

Add slow feedback, inflammation, hematopoiesis, local tissue repair and cell-population models. Use multi-scale spatial models only for questions that depend on local geometry or transport.

### Stage 4 — microbiome, reproductive and population heterogeneity

Introduce only with fit-for-purpose data, donor/population variation, privacy and rights review, and independent validation. Do not extrapolate one generic virtual person into a population.

### Stage 5 — personalized twin research

Only after repeated person-specific observation, parameter/state identifiability, prospective prediction and context-of-use validation. Clinical decisions require appropriate institutional, regulatory and prospective validation beyond this architecture.

## 10. Primary sources and references

- Chörsi et al., *Model systems and integration strategies to capture human biology*, Nature Materials, 5 November 2026: https://www.nature.com/articles/s41563-026-02784-3
- Mohsen & Trayanova, *Digital System Physiology: Integrating Mechanistic Models, Artificial Intelligence, and Digital Twins*, Annual Review of Physiology, 28 October 2026: https://doi.org/10.1146/annurev-physiol-040225-113230
- Hunter et al., *The Physiome Project and Digital Twins*, IEEE Reviews in Biomedical Engineering, 2025: https://doi.org/10.1109/RBME.2024.3490455
- CellML 2.0 specification: https://www.cellml.org/specifications/cellml_2.0
- SBML Level 3 specifications: https://sbml.org/documents/specifications/
- Physiome / CellML Model Repository: https://models.physiomeproject.org/
- BioGears model documentation and limitations: https://www.biogearsengine.com/documentation/index.html
- OpenSim modelling documentation: https://opensimconfluence.atlassian.net/wiki/spaces/OpenSim/pages/53088700/Tutorial%2B1%2B-%2BIntro%2Bto%2BMusculoskeletal%2BModeling
- Multiscale tissue modelling challenges: https://doi.org/10.1002/wsbm.1527

## 11. Claim boundary

This atlas is a design and gap-analysis artefact. It does not implement the listed systems. A synthetic module can be numerically verified without being biologically valid; a mechanistic model can explain a mechanism without predicting a person; a predictive model can perform well without identifying the mechanism; and none of these alone establishes clinical utility. The system must keep these claims separate.
