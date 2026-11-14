# Research Note: Standards-First Executable Experiments and Tested Visual Mathematics

- **Date:** 2026-10-19
- **Status:** Research synthesis and implementation recommendations; no interoperability adapter or human-factors study is claimed complete.
- **Scope:** Symtropy experiment-math core, Bevy visualization, domain-specific solvers, Symthaea-assisted modelling, and evidence-linked historical reconstructions.

## Executive recommendation

Do not attempt to invent one universal physics language or one universal solver. Build a small, typed experiment graph for stable IDs, units/dimensions, parameter values, equations, provenance, run manifests and visual projections. Then connect domain-specific model formats and solvers through explicit adapters with machine-readable capability and loss reports.

This recommendation is supported by existing standards that deliberately solve different pieces of the problem:
- **SBML** is a computable, tool-neutral interchange representation aimed at systems biology—not a universal model language.
- **FMI 3.0** standardizes interfaces to exchange and execute models, including model exchange and co-simulation. In co-simulation, the coupling algorithm remains an importer/simulator responsibility, and FMUs may advance their own internal solver between communication points.
- **MA-HS Experiments Format** describes experiment setups, including start/stop times, step size, tolerance, parameter sets, stimuli and reference results. It is a useful example of separating a model from experiment configuration and test data.

The Symtropy arithmetic graph should become an **integration seam**, not a competitor to every mature scientific tool.

## 1. Current prototype boundary

The new Rust **symtropy-experiment-core** crate is a narrow, renderer-independent prototype with three related capabilities:

- An acyclic arithmetic graph validates typed dimensions and references, generates formula labels from operator/variable IDs, and supports scoped input/parameter overrides and one-variable sweeps.
- A separate one-dimensional harmonic-oscillator module implements explicit Euler and symplectic Euler, compares the numerical trajectory against the known analytic solution, and reports integration error and energy drift.
- A `visual_math_projection` produces a renderer-neutral graph from canonical IDs; its formula labels are generated from the executable operator graph, and the `inspect` CLI can still show an incomplete model if baseline execution fails.

The oscillator is the first time-dependent numerical regression case, **not** a universal physics engine:
- no general ODE/DAE solver, contact solver, constraint solver, or stochastic process runtime;
- no nonlinear/damped/forced dynamics, torque train, backlash, elasticity, friction, heat transfer, or instrument model;
- no full unit catalogue or conversion engine;
- no SBML/FMI importer/exporter;
- no formal theorem proving, historical validation or empirical replication.

The two numerical modules test different properties and should not be fused into an implicit universal abstraction prematurely.

The eighth dimension exponent is explicitly a semantic angle axis. SI treats plane angle as dimensionless; the prototype retains a separate angle type to catch semantically invalid operations such as adding a tooth count to a radian value. This convention must be documented in every adapter and never silently presented as strict SI dimensional analysis.

## 2. Paradigm shifts to prioritize

### 2.1 From scenes to a compiled model

A physical scene should be a projection of a canonical model, never a separate place to store the scientific law. Stable IDs connect an apparatus component, variable, equation node, parameter, source, run, result plot and validation record. Formula text is generated from the executable operators to avoid a hand-written equation drifting away from code.

**Test:** 100% of rendered equation labels resolve back to the typed operation and variable IDs that are actually executed. A label is explanatory output, never the source of truth.

### 2.2 From one solver to a federation of solvers

Rigid-body mechanics, circuit simulation, orbital integration, heat transfer, reaction kinetics and probabilistic inference need different numerical methods and validity checks. Keep a shared envelope for inputs, parameters, outputs, events, units, provenance and reproducibility; keep domain semantics and solvers separate. Where systems interact, require explicit coupling and time/event semantics.

**Test:** every adapter declares supported constructs, solver capabilities, time semantics, parameter/unit mappings and unsupported features. Fail closed on semantic losses that change results; otherwise emit a loss report.

### 2.3 From file conversion to reproducible experiment packages

Treat each reported result like a reproducible build: model spec, exact code/model revision, environment, units, input data hashes, parameter set, random seeds, solver settings, command, outputs, residuals and comparison rule. Distinguish source-reported target values from our output. A citation alone is never an executed run.

**Test:** a reviewer can reconstruct the inputs and environment; reruns either match the declared deterministic target or satisfy the preregistered numerical tolerance, and discrepancies remain archived.

### 2.4 From a single best reconstruction to model ensembles

Missing geometry, uncertain material properties and disputed procedures should be represented as alternative models or parameter distributions. Explore the range of outcomes, identify which assumptions determine the observable result, and select new measurements that discriminate between alternatives. Do not average incompatible historical hypotheses into a fictional consensus.

**Test:** report sensitivity/identifiability and the assumptions most influential to each output; compare alternatives on declared axes without silently collapsing them into one truth score.

### 2.5 From “more visual” to experimentally proven visual encoding

Visual math should be optional, layered, accessible and evaluated against text/equation-only alternatives. More pictures are not guaranteed to help: a 2026 study with 81 nine-year-olds found that visual aids in its arithmetic-task conditions caused more errors and longer response times than text-only tasks. Conversely, a 2026 study of 131 eighth-grade students solving systems-of-linear-equations word problems reported improved performance and lower reported cognitive load from structural representation using text segmentation and tabular organization. These results concern different tasks and learners; together they argue for testing the exact representation, not for a universal verdict on visualization.

Design implications:
- default to task-relevant structure rather than decoration;
- provide physical, graph, symbolic and text views with fast switching;
- progressively disclose complexity; allow local collapse/expansion without hiding assumptions or variables;
- use text labels, shape/line patterns and uncertainty bands in addition to color;
- support keyboard navigation, screen readers, static alternatives and reduced motion;
- let expert users pin exact equations and units, while newcomers can enter through a physical or causal view.

**Proposed human-factors target (aspirational, not achieved):** on preregistered expert tasks, aim for at least 10% lower median completion time than an equation-only baseline while keeping accuracy non-inferior within 2 percentage points. Use a power-calculated study, confidence intervals, error-category analysis, and separate results by familiarity/access needs. If this target is not reached, revise the visual encoding rather than claiming success from preference ratings.

## 3. Interoperability strategy

### Layer 1 — Symtropy experiment envelope

Keep a small Rust-native package for stable IDs and evidence-linked metadata:
- experiment question, status and limitations;
- typed variables, dimensions, unit symbols and parameter values;
- equation/operator graph and expression rendering;
- initial/boundary conditions and stimuli;
- reference observations and uncertainty descriptions;
- run manifests, solver identity, tolerances, hashes and comparison rules;
- visualization mappings and linked semantic IDs.

This envelope must not pretend to encode every solver's internals.

### Layer 2 — Domain formats and adapters

| Domain/interface | Recommended role | Guardrail |
|---|---|---|
| **SBML** | Import/export supported biochemical and systems-biology models and reaction networks. | Record SBML Level, Version and package capabilities; do not treat the subset supported by our first adapter as full SBML conformance. |
| **FMI 3.0** | Integrate externally built models/simulators using Model Exchange, Co-Simulation or Scheduled Execution as supported. | Make communication steps, event handling, variable mapping and co-simulation algorithm explicit. Capability flags and solver semantics are not optional metadata. |
| **MA-HS Experiments Format** | Interchange experiments, parameter sets, stimuli and reference outputs for compatible FMI-oriented test suites. | Keep experiment configuration separate from model semantics; preserve original files and content digests. |
| **Native Rust backends** | High-performance, transparent solvers for domains where native support is justified. | Each solver owns a validation suite and declared validity envelope. |
| **Visual equation graph** | Explain and navigate structure across all domains. | It is a linked projection, not a replacement for the source model or mathematical definition. |

### Layer 3 — Capability and loss reports

For every import/export, produce a machine-readable report:
- source format/version and supported feature set;
- imported constructs and preserved identifiers;
- unit conversions and transformation rules;
- unsupported or approximated constructs;
- whether semantics round-trip, are partially preserved, or were rejected;
- all warnings with affected model IDs.

Do not claim “lossless conversion” based on parsing success. Compare semantics and outputs on reference cases.

## 4. From one-off runs to design of experiments

The executable core now supports parameter/input overrides scoped to a single run and deterministic one-variable sweeps, a renderer-neutral visual-math projection, and a domain-specific harmonic-oscillator integrator comparison against an analytical solution plus observed-order convergence report. The gear fixture sweeps driver tooth counts 47, 48 and 49 without mutating the canonical graph. The oscillator fixture uses a unit-mass, 4 N/m ideal spring, initial displacement 1 m, zero velocity, and a 10 s horizon at a 0.01 s timestep; it contrasts explicit-Euler energy growth with the bounded-energy behavior of symplectic Euler at this stable step size. This is a synthetic regression case only.

This is the beginning of an **experiment exploration loop**, not yet a full uncertainty quantification package:
1. Sweep selected inputs over declared values or ranges.
2. Plot outputs and finite differences against the varied inputs.
3. Flag regions of strong response, nonlinear change, discontinuities or failed runs.
4. Propagate declared uncertainty only after its distribution and dependence structure are specified.
5. For expensive models, later choose runs that best distinguish competing hypotheses or reduce uncertainty in a named quantity of interest.

Keep these outputs distinct:
- **Scenario sweep:** response to a declared list of input values.
- **Local sensitivity:** an explicitly defined derivative estimate around a point.
- **Global sensitivity:** a method such as Sobol variance indices, with its assumptions, sampling design and convergence checks.
- **Uncertainty propagation:** a distribution of possible outputs conditional on the specified input uncertainty.
- **Experimental design:** a proposed physical measurement selected by its expected discrimination/value and cost.

A sweep is not a probability distribution. A finite difference is not a global sensitivity index. A well-resolved sensitivity calculation does not prove a model is correct.

## 5. Verification, validation and uncertainty must be separate gates

NASA-STD-7009B defines practices for modeling and simulation, including acceptance criteria and credibility assessment across a model's life cycle. NASA's verification guidance distinguishes verifying implementation/numerical solution from validating a model against experimental evidence. Symtropy should encode these as distinct statuses and artifacts rather than one unqualified “validated” flag. (References: https://standards.nasa.gov/standard/NASA/NASA-STD-7009; https://www.grc.nasa.gov/www/wind/valid/tutorial/verassess.html; https://www.nasa.gov/reference/appendix-i-verification-and-validation-plan-outline/)

- **Software verification:** did the implementation match the declared model, and are numerical errors bounded?
- **Solution verification:** is this particular numerical result sufficiently converged for its intended use?
- **Validation:** does the model agree with independent data for a specified use and domain?
- **Uncertainty quantification:** how do input, measurement, numerical and model-form uncertainties affect the quantities of interest?
- **Credibility/acceptance:** is the evidence sufficient for the stated decision, at a declared risk and use context?

Every run should include an intended-use statement, acceptance metric/threshold, target evidence, data split or independence note, limitations, solver/environment identity and visible failed checks. No aggregate score may hide a failed safety-critical validation gate.

## 6. Benchmark suite to build

Benchmarks should be reported as a vector; a good render must not compensate for wrong science.

| Area | Initial release gate | Longer-term benchmark |
|---|---|---|
| Graph integrity | 100% detection of seeded invalid references, duplicate outputs, cycles and invalid operator arity; no false positive on known-good fixtures. | Mutation/property tests over generated equation graphs. |
| Dimensional typing | All negative tests for add/subtract mismatch, multiplication/division dimensions and exponent overflow are rejected. | Unit conversion, affine units, uncertainty propagation and cross-format dimension agreement. |
| Arithmetic execution | The gear known-answer case returns -1.9 rad within 10^-12 in the stated binary64 reference interpreter. | Numerical tolerance policies per solver/backend; exact arithmetic only where explicitly supported. |
| Stateful dynamics | Harmonic oscillator: compare each sample to the closed-form position/velocity, assert halving timestep reduces maximum position error, and assert symplectic Euler's energy deviation is below 2% for the fixed reference case while explicit Euler growth is detected. | Add damped/forced oscillator analytic cases, resonance, event/state handling and solver-independent reference comparisons before broad ODE claims. |
| Convergence order | For the synthetic undamped oscillator, evaluate time steps 0.04, 0.02, 0.01, 0.005 s; require decreasing position error and pairwise observed order in [0.8, 1.3] for this finite horizon. Preserve individual pair estimates. | Build work-precision curves across different methods: error against runtime/function evaluations/memory, including reference-error floors and stiff/non-stiff cases. |
| Visual semantic integrity | 100% of displayed equations generated from graph operators/IDs; all collapsed sections expose their hidden variable count and full expansion path. | Automated scene-to-model linkage checks and accessible keyboard navigation. |
| Reproducibility | Same fixture and binary revision produce the same declared result under the reference environment; record input and output hashes. | Cross-platform comparisons with explicit floating-point tolerances and solver/environment fingerprints. |
| Interoperability | Test representative imports/exports; every unsupported feature is rejected or named in a loss report. | Semantic round-trip equivalence on published conformance suites and cross-solver result comparisons. |
| Physical/numerical correctness | Validate a solver only against analytic or independently trusted reference solutions appropriate to its domain. | Conservation, stability, convergence, event timing, parameter identifiability and sim-to-real error per model family. |
| Scientific grounding | Source-reported data, our computed outputs and independent replication remain separately labelled. | Independent reproductions, adversarial sources, negative controls and peer-reviewed benchmark packages. |
| Visual usefulness | Pilot study against text/equation-only controls; record correctness, time, misconception types, accessibility and cognitive load. | The proposed 10% speed-up/non-inferiority target on powered, preregistered expert tasks. |
| Uncertainty handling | Missing inputs and unverified assumptions are surfaced; values are not silently filled in. | Calibrated predictive intervals, sensitivity analysis, alternatives and experiment-design utility. |

The 10^-12 rad goal is specific to the simple arithmetic fixture and reference interpreter; it is not a universal tolerance for real physical simulation. Numerical thresholds must be justified for each model family. The one-variable sweep has a known finite-difference check against the toy model's analytic slope, -theta_in/N_driven = -1/24 rad per added driver tooth. For the oscillator, an independent calculation mirroring the formulas gives maximum position errors of 0.04464, 0.02116, 0.01029 and 0.005072 m at 0.04, 0.02, 0.01 and 0.005 s; the corresponding pairwise observed orders are approximately 1.077, 1.040 and 1.020. These numbers are a static equation-level audit, not Rust execution results. The benchmark practice mirrors two well-established ideas: report observed convergence instead of inferring it from nominal method order, and compare error against computational work (for example, function evaluations and wall time), not accuracy alone.

## 7. Recommended implementation sequence

1. **Secure the baseline:** run the new crate tests and workspace checks on a toolchain; preserve logs/SHAs. The current authored tests are not claimed as executed until a real run is recorded.
2. **Harden model semantics:** add unit symbols/conversion policies, explicit variable roles and evidence mappings; property-test graph validation and canonical expression generation.
3. **Extend stateful dynamics deliberately:** use the analytical oscillator to harden run manifests, time-step controls, integrator comparisons, and failure statuses; then specify event/state semantics before adding a second ODE family.
4. **Build independent reference examples:** conservation balance, dimensional-analysis failures, simple reaction network, and gear kinematics. Each example needs an analytical or independently trusted target, convergence tests where numerical solving is involved, and negative controls.
5. **Add one adapter at a time:** prioritize a minimal SBML subset or an FMI wrapper based on the actual target domain, not on fashionable breadth. Publish capability matrices and loss reports.
6. **Ship the first evidence-linked historical model:** Antikythera's best-documented gear subsystem, with alternative reconstructions for missing parts. Do not present the generic synthetic gear fixture as a historical reproduction.
7. **Run usability experiments:** compare equation-only, structure-graph and physical-plus-equation views on concrete research tasks before optimizing visual style.
8. **Close the loop:** use model disagreement and uncertainty to propose a discriminating physical experiment, then preserve the observations and calibrated instrument record.

## References

7. NASA Handbook for Models and Simulations, NASA-HDBK-7009B (active, published 2026-02-03), companion implementation guidance for NASA-STD-7009B: https://standards.nasa.gov/standard/NASA/NASA-HDBK-7009
8. SciMLBenchmarks, solver benchmark suite, uses shootouts and work-precision diagrams to compare algorithm runtime and error on shared problems: https://docs.sciml.ai/DiffEqDevDocs/stable/alg_dev/benchmarks/
9. SciML convergence simulations, which estimate observed order from error sequences when a true/reference solution is provided: https://docs.sciml.ai/DiffEqDevDocs/stable/alg_dev/convergence/

1. SBML.org, “What is SBML?” — explains SBML as a computable interchange representation focused on systems biology and explicitly rejects a universal quantitative-model language: https://sbml.org/documents/what-is-sbml/
2. Modelica Association, “FMI for Co-Simulation,” FMI Standard documentation — describes communication points, solver encapsulation and the responsibility of the importer for co-simulation orchestration: https://github.com/modelica/fmi-standard/blob/main/docs/4___co-simulation.adoc
3. Modelica Association, “Experiments Format,” current version dated 2026-09-17 — describes experiment setups, parameters, stimuli, reference data and simulation timing/tolerance fields: https://modelica.github.io/ma-hs-experiments/main/
4. Skau, Karlsson & Åsberg Johnels (2026), “Visualizations during arithmetic tasks hamper performance and increase cognitive load in 9-year-olds: an fNIRS study,” *Frontiers in Psychology*: https://doi.org/10.3389/fpsyg.2026.1722948
5. Shang et al. (2026), “The effects of structural visualization methods on student performance in mathematics word problems: an eye-tracking analysis,” *International Journal of Educational Research* 140, 103075: https://doi.org/10.1016/j.ijer.2026.103075
6. NASA-STD-7009B, active Standard for Models and Simulations, on acceptance criteria and model credibility over the modelling/simulation lifecycle: https://standards.nasa.gov/standard/NASA/NASA-STD-7009
7. NASA Glenn, Verification Assessment, on code/calculation verification, consistency and convergence studies: https://www.grc.nasa.gov/www/wind/valid/tutorial/verassess.html
8. Efstathiou et al. (2012), “Determination of the gears geometrical parameters necessary for the construction of an operational model of the Antikythera Mechanism,” *Mechanism and Machine Theory* 52: https://doi.org/10.1016/j.mechmachtheory.2012.01.020
