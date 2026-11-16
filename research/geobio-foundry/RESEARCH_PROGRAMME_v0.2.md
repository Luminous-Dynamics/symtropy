# GeoBio Foundry Research Programme v0.2

**Date:** 2026-11-16  
**Home:** Luminous-Dynamics/symtropy  
**Status:** Research programme and isolated reference prototype; no physical qualification claimed

## 1. Mission

Develop a reusable, evidence-carrying engineering workflow that can characterize a geological environment, determine which resources and spaces are actually usable, compile candidate infrastructure designs, compare biological and abiotic process routes, and qualify claims against independent observations.

The ambition is not a generic autonomous lunar base. It is an engineering framework for turning geology into productive infrastructure while retaining uncertainty, reversibility, provenance and independent verification.

## 2. Strategic insight from repository inspection

Symtropy already includes volumetric subterranean terrain and excavation/weathering in `symtropy-terrain`, ecological/environmental state in `symtropy-basin`, general physical simulation, and robotics bridge foundations. GeoBio should add evidence and process accounting around these layers rather than duplicate them.

First code stays under `research/geobio-foundry/` with an isolated Cargo workspace. No production workspace membership or runtime dependency is introduced in this increment.

## 3. The seven paradigm shifts

### 3.1 From habitat-first to geology-to-capability compilation

A site is not a habitat because it has a large cavity. Site characterization, resource accessibility, rock-mass stability, access, power, thermal rejection, pressure boundaries, contamination constraints and logistics must be compiled into a set of candidate capabilities. The output can be infeasible or require measurement.

### 3.2 From point estimates to evidence-preserving spatial fields

Geology should not be represented as one categorical label or one best-estimate mesh. Preserve observation source, calibration, time, modality, spatial resolution, coverage, uncertainty and model discrepancy. Masked regions remain unknown.

### 3.3 From optimizing collection to optimizing information value

Select the next scan, assay, sample, or borehole according to the expected value of information relative to its mass, energy, time, cost and mission risk. The best measurement is the one that can change an important decision, not necessarily the one that adds the most data.

### 3.4 From biology-versus-chemistry to adaptive process portfolios

Different mineralogies and products may favour abiotic, biological or hybrid pathways. Route selection must include nutrient and medium burden, conditioning, separation, purity, heat, water, energy, waste and maintenance. Biology is a candidate unit operation—not a default winner.

### 3.5 From energy efficiency to exergy-aware industrial ecology

Track gross energy, recovered energy, thermal level, conversion, storage, heat rejection and exergy destruction. Low-grade heat is not interchangeable with high-temperature process energy. Reuse of byproduct heat must be supported by timing and temperature compatibility.

### 3.6 From steady-state optimization to history-aware operation

Rock masses, pressure boundaries, bioreactors and robots are history-dependent. Include fatigue, thermal cycling, gradients, residence time, contamination events, transient contact, sensor drift and recovery trajectories.

### 3.7 From maximizing autonomy to maximizing safe recoverability

Prefer reversible excavation, contact and sampling, modular pressure isolation, safe shutdown, retreat options and explicit abstention. Autonomous operation is only valuable inside a validated domain with authority boundaries.

## 4. Research architecture

### GeoBioAtlas — spatial evidence

A versioned site representation combining terrain geometry, subsurface observations, assays, environmental measurements, access and preservation constraints. Source observations remain immutable; derived interpretations link back to their source evidence.

### GeoBio Resource and Constraint Graph

A typed graph of geological occurrences, deposits, measured inventories, pressure boundaries, utilities, energy/thermal flows, biological processes, batches, products, waste, robots and evidence. Unknown quantities are not zero.

### GeoBio Infrastructure Compiler

Pipeline:
`observations → site model → resource/constraint graph → candidate designs → model evaluation → value-of-information plan → independent tests → capability record`

Possible results include `insufficient_evidence`, `out_of_domain`, `requires_measurement`, `infeasible`, `candidate_only`, and `qualified_for_declared_envelope`.

### ResourceForge

Process and resource accounting on explicit functional units. Compare biological, abiotic and hybrid processing with matched feedstock, product purity, environment and downstream boundary.

### PressureForge

Model independent pressure compartments, boundary interfaces, seals, penetrations, leakage, isolation, pressure cycling, thermal transients and fault propagation. Natural rock is not assumed to be a pressure vessel.

### Symthaea

Propose hypotheses, choose measurements, diagnose anomalies and plan bounded recovery actions. It cannot self-certify its model or authorize high-consequence actuation.

### Symtropy

Supply physical simulation, terrain evolution, contact mechanics and deterministic replay. New geomechanical and pressure models require independent references and physical calibration.

### Mycelix

Coordinate qualified resource claims, production batches, capacity, contracts and attestations. A reported inventory is not a verified physical inventory.

### Sol-Atlas

Maintain spatial records, observation lineage, scientific preservation and mission-specific constraints. A generated interpretation cannot overwrite original observations.

## 5. Work packages and measurable deliverables

| Work package | First deliverable | Validation class |
|---|---|---|
| WP1 Evidence site model | Synthetic basalt cavern with masked regions, fractures, conflicting sensors and explicit observation provenance | S0/S1 |
| WP2 ResourceForge | Dependency-free elemental and process-intensity kernel with negative controls | S0/S1 |
| WP3 GeoSurveyBench | Blind reconstruction and uncertainty-calibration harness | S0/S1, then S2 |
| WP4 PressureForge | Compartment mass/leak/isolation reference model and analytical checks | S0/S1, then S2 |
| WP5 BioAsteroidRepBench | Protocol reconstruction, matched controls and resource-burden ledger | Literature reproduction, then S2/S4 |
| WP6 ContactSequenceBench | Approach/contact/stabilize/sample/retreat scenario and fault-injection corpus | S0/S1, then hardware-in-loop |
| WP7 Cross-site compiler | Candidate design generation with infeasible/unknown outcomes | S0/S1 |
| WP8 Assurance integration | Evidence manifests, negative controls, independent replay and qualification transitions | S0–S3 |

## 6. First demonstration

The first demonstrator is an uncrewed terrestrial analogue, not a human habitat:
- surveyed rock/void or basaltic cavern analogue;
- independent metrology and calibration records;
- two independently sealed test compartments;
- safe, instrumented leak/isolation loop designed by qualified engineers;
- sensor streams for pressure, temperature, flow and deformation;
- non-biological or non-pathogenic process surrogate at first;
- robotic inspection or recorded survey dataset;
- frozen model/configuration, blind prediction, independent test, residual analysis and qualification report.

Human occupancy, biological release, extraterrestrial deployment and flight claims are explicitly out of scope.

## 7. Commercial and scientific path

Near-term value should be pursued in the technology that can be tested on Earth: geological site characterization, process/resource accounting, digital-twin assurance, contained bioindustrial monitoring, autonomy safety cases, and industrial lifecycle optimization.

A space business case must compare delivered product against imported product using launch mass, energy, feedstock, nutrients, purification, downtime, spares, power system, thermal rejection, waste handling and capital recovery. Asteroid PGE mining is not a commercial thesis until target-specific assays and a full extraction/separation/logistics model support it.

## 8. Definition of success

GeoBio v0.2 succeeds when another engineer can use frozen site/process inputs to reproduce the resource balance, evaluate candidate constraints, regenerate benchmark scores, trace each output to its original observations and identify unresolved unknowns.

The first breakthrough is a small falsifiable compiler and evidence model that refuses to invent a safe, available or profitable answer. The next breakthroughs should be earned through blind tests and physical analogues.
