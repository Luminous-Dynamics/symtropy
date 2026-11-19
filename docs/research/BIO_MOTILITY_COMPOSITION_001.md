---
title: BIO-MOTILITY-COMPOSITION-001 — Cross-taxon biological movement, growth and nested coupling
status: research-contract
version: 0.1.0
owner: Symtropy life systems
tracking_issue: https://github.com/Luminous-Dynamics/symtropy/issues/5981
---

# BIO-MOTILITY-COMPOSITION-001

## Purpose

This programme qualifies how independently grounded biological models can be composed across mechanisms and scales. It focuses on a specific missing integration surface: **motility, morphogenesis, organism physiology and environmental coupling**.

It is not a proposal for one universal organism equation or a new all-purpose simulator. Bacterial swimming, archaeal archaellum motion, amoeboid crawling, ciliary motion, animal gait, fungal tip growth, plant root growth and passive dispersal arise from different mechanisms. They should share exchange and evidence contracts where useful, while retaining their own physics, state, units and validity domains.

The research question is:

> Can Symtropy compose mechanism-specific models of biological movement and growth across nested scales, while preserving causal observability, physical boundaries, uncertainty and prospective predictive validity?

The programme is governed by [issue #5981](https://github.com/Luminous-Dynamics/symtropy/issues/5981).

## Design thesis

Five design shifts guide this work:

1. **Mechanism before taxonomy.** Classify movement by the physical process, not only by organism label.
2. **Property profiles before life labels.** Reuse LIFE-SUBSTRATE-001 and OLI-M0; do not introduce another binary life/intelligence classifier.
3. **Open systems before closed toy worlds.** Declare reservoirs, inputs, outputs, omitted processes and uncertainty.
4. **Overlapping composition before a strict tree.** Containment is hierarchical, but symbiosis, transport, predation, signalling and resource exchange form a directed multigraph or hypergraph.
5. **Prospective prediction before visual plausibility.** A rendered pattern is not evidence that its mechanism is correct.

## Existing owners and integration boundaries

This programme composes the following work; it does not replace it.

| Existing owner | Responsibility retained |
|---|---|
| `symtropy-lifesim-core` | CPU-first field substrate, ecological channels, deterministic reference stepping, telemetry and optional GPU parity |
| Symtropy [Living World Cell #252](https://github.com/Luminous-Dynamics/symtropy/issues/252) | Causal ecology vertical slice and world-level integration |
| Symtropy [LIFE-SUBSTRATE-001 #4426](https://github.com/Luminous-Dynamics/symtropy/pull/4426) / [OLI-M0 #4427](https://github.com/Luminous-Dynamics/symtropy/issues/4427) | Open-world life/intelligence property assessments, abiotic controls, OTHER/UNRESOLVED and benchmark fixtures |
| Symtropy [MYCO-BIO #5379](https://github.com/Luminous-Dynamics/symtropy/issues/5379) / [MYCO-MULTISCALE #5413](https://github.com/Luminous-Dynamics/symtropy/issues/5413) | Hyphal morphology, transport, plant-fungus exchange and graph-to-wave reduction |
| Symtropy [MYCO-HIGHWAY #5562](https://github.com/Luminous-Dynamics/symtropy/issues/5562) | Hypha/liquid-film-associated bacterial dispersal; this is not bacterial self-propelled motility |
| Symtropy [PLANT-HYDROMECH #5780](https://github.com/Luminous-Dynamics/symtropy/issues/5780) | Plant hydraulics, stomatal regulation and hydromechanical stress propagation |
| Symtropy [ANIMA-NEURO #3313](https://github.com/Luminous-Dynamics/symtropy/issues/3313) | Animal neural sensorimotor integration with body physics and perception authority |
| Symtropy [SIM-BACKEND-QUAL #5727](https://github.com/Luminous-Dynamics/symtropy/issues/5727) | Matched simulator/backend comparison |
| Symthaea [BIO-MILLENNIUM #6271](https://github.com/Luminous-Dynamics/symthaea/issues/6271) | Evidence-bounded biology discovery, prospective validation and specialist-model composition |
| Symthaea [AOBENCH #11980](https://github.com/Luminous-Dynamics/symthaea/issues/11980) | Organizational autonomy, self-maintenance and intervention benchmarks |
| Symthaea [ROB-LOCO #11475](https://github.com/Luminous-Dynamics/symthaea/issues/11475) | Robot mission-level belief, planning and recoverability; no new robot controller is created here |

The first inventory pass located clear issue-level ownership for fungi, plants, animal sensorimotor systems and hypha-associated bacterial dispersal. It did not establish equivalent benchmark coverage for autonomous bacterial chemotaxis, archaeal archaellum movement or protist amoeboid/ciliary motility. This is a **public issue-inventory gap**, not proof that no related source code exists. M0 must check code and data before any implementation is proposed.

### Current foundation limitations to preserve

- `symtropy-lifesim-core` is CPU-first and field-oriented. Its common grid channels and generic metabolism are useful scaffolding, not validated species physiology.
- `symthaea-alife` provides an existing artificial-life domain based on shared FEP/Markov-blanket mechanisms. Its own documentation labels some physiology-like modulators as modelling analogies, not taxon-specific empirical mechanisms.
- The Symtropy physics core is not a soft-body biology solver. Do not smuggle compliant tissue or fluid mechanics into rigid-body primitives.
- HDC/CfC summaries may assist prediction and retrieval, but may not replace explicit physical state, biochemical quantities, units, or conservation ledgers.

## Mechanism atlas

The registry is mechanism-first. One organism may participate in multiple classes; the same movement category may appear in different lineages.

| Class | Example processes | Required distinction |
|---|---|---|
| Active microbial motility | Bacterial run–tumble, flagellar/archaellar swimming, gliding | Active propulsion vs externally advected motion |
| Amoeboid movement | Pseudopod extension, adhesion, contraction and migration | Shape change and substrate interaction |
| Ciliary/flagellar eukaryotic motion | Ciliary beat, flagellar waveform, fluid coupling | Propulsive kinematics and hydrodynamic regime |
| Whole-organism locomotion | Undulatory, appendage-based, hydrostatic, jet, adhesion-based | Body mechanics, contact, fluid and sensorimotor loop |
| Growth-mediated displacement | Hyphal tip extension, root growth, tissue expansion | Morphogenesis, growth fronts and resource-limited development |
| Collective relocation | Aggregation, swarming, colony movement | Individual rules, interaction graph and collective statistics |
| Passive transport/dispersal | Advection, sedimentation, liquid films, vectors | External transport must not be relabelled active motility |

A common benchmark envelope may expose trajectories, events, resource use, environment and uncertainty. It must not force all classes into a shared equation or a single speed/efficiency ranking.

## Taxonomic coverage register

Use a source-pinned taxon-concept registry. Kingdom is a reporting attribute, not a permanent enum or simulator dispatch key. The initial seven-kingdom view is a coverage convenience; taxonomy changes and alternative classifications must be supported.

Candidate reference cases are **prospective**, subject to source/data rights and reproducibility:

| Coverage target | Candidate | First mechanism to qualify |
|---|---|---|
| Bacteria | *Escherichia coli* or a data-admitted strain | Run–tumble chemotaxis and flagellar motion |
| Archaea | *Halobacterium salinarum* | Archaellum swimming / phototactic response |
| Amoebozoa | *Dictyostelium discoideum* | Amoeboid migration and aggregation transition |
| Ciliates | *Paramecium* spp. | Ciliary beat and fluid-coupled motion |
| Stramenopile lineages | Data-admitted diatom or oomycete | Only the movement/life-cycle mechanism supported by the selected source |
| Fungi | *Neurospora crassa* or admitted mycorrhizal reference | Hyphal extension, branching, fusion and network transport |
| Plants | *Arabidopsis thaliana* or admitted root dataset | Root growth, tropism and soil–root hydraulics |
| Animals | *Caenorhabditis elegans* | Closed-loop sensorimotor locomotion |

Viruses, viroids and prions are tracked separately as acellular replicators or host-dependent systems; they are not extra cellular kingdoms. Lichens, microbiomes, mycorrhizae, coral holobionts and biofilms are overlapping multi-organism associations, not kingdoms.

No taxon is admitted because it is familiar or visually compelling. Admission requires an explicit taxon concept, strain/species identity where relevant, source provenance, experimental unit, data licence, measurement support and mechanism justification.

## Compositional exchange contract

Containment is hierarchical; interaction is a directed multigraph or hypergraph. A root, fungal network, bacterial population, soil-water volume and plant host may overlap in space and exchange multiple resource species. A single parent pointer cannot express this.

Every coupled model must declare:

- stable model and revision identity;
- taxon concept, strain, life stage, phenotype and source version where applicable;
- coordinate frame, spatial support, units and temporal scale;
- internal state, observable state, hidden state and measurement model;
- ports for matter, named chemical species, energy, momentum, mechanical work and information;
- declared control volume, environmental reservoirs, boundary inputs and outputs;
- coupling law, numerical solver, step size, latency, synchronization and interpolation rules;
- conservation quantities, numerical error budget and residual policy;
- parameter source, calibration data, uncertainty and operating envelope;
- intervention semantics, initial-state digest, seed and deterministic replay identity.

Information flow, belief entropy and thermodynamic entropy must remain distinct. A mathematical Markov blanket is a model construct; it does not establish a specific organism's membrane physiology or prove life.

## Benchmark tracks

### M1 — Movement and growth anchors

Begin with two executable, rights-cleared cases, not a giant synthetic biodiversity catalogue:

- one autonomous microbial motility case (chemotaxis/flagellar motion if a suitable data and source package is admitted);
- one existing, data-admitted animal or fungal mechanism already owned by #3313 or #5379.

A third case should add an explicit environment coupling (e.g. fluid field, substrate mechanics or resource gradient). Do not choose a case until experimental units, measurement cadence, boundaries and licensing are known.

Metrics are mechanism-specific:
- trajectories: speed, displacement, persistence, turning, drift and directional bias;
- chemotaxis: response latency, chemotaxis index, gradient/flow dependence and run/tumble statistics;
- cilia/flagella: beat frequency, waveform and body/fluid kinematics where measured;
- growth fronts: tip velocity, elongation, branching, curvature, fusion and network topology;
- collectives: aggregation time, cohesion, dispersion and spatial statistics;
- resource cost: only when a measurable boundary and compatible physical units exist.

### M2 — Nested coupling and conservation

Frozen tests must cover:
1. parent/child boundary exchange against an independently computed reference;
2. per-species matter balance and energy balance over a declared control volume;
3. coupling stability under time-step and solver changes;
4. synchronized versus deliberately delayed cross-scale signals;
5. fine-to-coarse and coarse-to-fine projection error;
6. perturbation propagation and recovery across at least three declared scales;
7. topology changes, component loss, dormancy, death and decomposition without silent creation/deletion of state.

Unmeasured reservoirs remain explicit unknowns or residuals. A closed balance is not claimed if the control volume omits relevant flows.

### M3 — Causal intervention and generalization

Use positive controls, abiotic hard negatives, sensor dropouts, temporal/spatial undersampling, parameter shifts, out-of-domain environments and held-out organisms/strains where sample support permits. Split by biological replicate and experimental unit, not by adjacent video frames.

Symthaea proposes hypotheses and informative observations. Symtropy executes deterministic model scenarios and replay. Independent evaluators score outcomes. Sol-Atlas preserves taxon/geospatial knowledge and source lineage. Mycelix handles rights, provenance and collaboration receipts. No component is allowed to certify its own scientific claim.

### M4 — Coarse-graining tournament

For each high-fidelity and reduced model, run identical initial conditions, perturbations and observables. Measure:
- predictive error and calibration;
- which events and statistics survive reduction;
- boundary/conservation discrepancy;
- loss of topology, rare events or causal paths;
- decision sensitivity to omitted information;
- compute and memory cost.

Promotion to finer fidelity must be triggered by process sufficiency, residuals, uncertainty, topology change or decision sensitivity—not render distance alone.

## Industry and research baseline map

The programme should use the strongest appropriate specialist baseline for each task. There is no honest universal leaderboard across cell migration, worm locomotion, fungal growth and ecosystem fields.

| Comparator | Strong use case | What a fair comparison must control |
|---|---|---|
| PhysiCell + BioFVM | Lattice-free multicellular agents coupled to microenvironment PDEs | Same cells, microenvironment, initial conditions, solver tolerances and observable outputs |
| CompuCell3D | Cellular Potts / virtual-tissue morphology and cell sorting | Same lattice resolution, energy terms, update schedule and boundary conditions |
| Morpheus | Declarative multiscale ODE/PDE/Cellular-Potts models and parameter scans | Same process equations, parameter priors, scheduling and numerical tolerance |
| BioDynaMo | High-throughput agent-based biological modelling and scaling | Same agent rules, hardware, thread count, workload and output fidelity |
| OpenWorm / BAAIWorm | *C. elegans* body/neural/environment coupling | Same data, sensory inputs, biomechanics, behavior task and held-out trajectories |
| The Simsulator | Artificial-creature evolution and population throughput | Artificial-life/evolution tasks only; do not equate with biological model fidelity |
| SBML / CellML / SED-ML / NeuroML / MultiCellDS | Model/data interoperability and reproducible experiment descriptions | Test semantic preservation, not merely successful parsing |

BioDynaMo's published speedups are workload- and hardware-specific. They are a reason to include it in a matched scaling tournament, not a transferable performance target. PhysiCell/BioFVM, CompuCell3D and Morpheus represent different biological modelling regimes; choose the comparator by process, not brand. OpenWorm and BAAIWorm are relevant to closed-loop worm locomotion, while The Simsulator is relevant to artificial-creature population evolution.

## Scorecard and prospective acceptance criteria

Report independent dimensions; do not hide scientific validity inside one weighted score.

| Dimension | Measures |
|---|---|
| Mechanism fidelity | Held-out trajectory/field error, event timing, morphology and statistical discrepancy |
| Calibration | Predictive interval coverage, sharpness, residual structure and abstention |
| Causal validity | Intervention-response agreement, positive/negative controls, false positives/negatives |
| Composition | Boundary exchange residuals, conservation, coupling stability and scale-transfer error |
| Generalization | Leave-one-environment / strain / organism out where sample support permits |
| Performance | Updates/s, memory, CPU/GPU energy, parallel scaling, determinism and replay cost |
| Evidence | Rights, source version, experimental unit, split, parameter provenance and independent replication |

Prospective gates (not achieved results):

1. Every result binds a source-pinned model, dataset, solver, configuration, split, seed and hardware manifest.
2. The CPU reference path is reproducible without requiring a GPU or external commercial simulator.
3. Numerical tolerances are derived from model order, measurement uncertainty and independent reference error; no arbitrary universal epsilon.
4. Held-out predictive intervals satisfy preregistered calibration criteria, or the model widens uncertainty/abstains outside its supported envelope.
5. Reduced models expose their validity envelope and lost observables.
6. At least one experiment composes three independently owned process models and passes boundary, conservation and perturbation tests.
7. An SOTA improvement is claimed only against the strongest admissible comparator on matched data, information, compute and physical boundaries, with paired uncertainty and failures/Unknown outcomes reported.

No single raw speed, number of agents, visual quality, test count or synthetic score can establish SOTA.

## Execution phases

| Phase | Deliverable | Exit evidence |
|---|---|---|
| M0 — inventory and rights | Locate code, datasets, licences and taxon/mechanism coverage | Source inventory; rights and experimental-unit decisions |
| M1 — exchange contract | Minimal typed manifest and boundary semantics | Schema review; positive, negative and malformed fixtures |
| M2 — two anchors | One microbial motility case + one existing animal/fungal case | Deterministic CPU replay; held-out comparison to admitted source data |
| M3 — nested composition | Three-scale coupling to environment/ecology | Independent balance checks, coupling and perturbation reports |
| M4 — broader lineage coverage | Additional bacteria, archaea, protist, chromist, plant, fungal and animal cases | Case-specific data admission and validation |
| M5 — external tournament | Matched specialist comparison | Published reproducibility pack, limitations and negative results |

The initial implementation order should be data-first. Do not add new Rust types until M0 demonstrates what existing interfaces fail to represent. Keep the first reference implementation CPU-first and dependency-light; GPU paths are optional optimizations that must be checked against the CPU reference.

## Claim ceiling and safeguards

This programme can qualify a declared model for a declared biological mechanism and data envelope. It cannot establish a universal theory of life, prove a simulated system is alive, prove consciousness, or infer general biological superiority from an internal benchmark.

It does not authorize wet-lab work, organism release, ecological intervention, physical actuation or robot motor authority. Research proposals are not experimental approvals. Physical biological work requires independent biosafety, ethical, containment and facility review.

## References and comparator entry points

- [PhysiCell](https://physicell.org/) and [BioFVM](https://github.com/MathCancer/PhysiCell)
- [CompuCell3D](https://compucell3d.org/)
- [Morpheus](https://morpheus.gitlab.io/)
- [BioDynaMo](https://www.biodynamo.org/)
- [BioDynaMo performance comparison](https://academic.oup.com/bioinformatics/article/38/2/453/6371176)
- [OpenWorm modelling documentation](https://docs.openworm.org/modeling/)
- [BAAIWorm, Nature Computational Science (2024)](https://www.nature.com/articles/s43588-024-00738-w)
- [The Simsulator, Artificial Life (2026)](https://doi.org/10.1162/ARTL.a.457)
- [SBML specifications](https://sbml.org/documents/specifications/)
