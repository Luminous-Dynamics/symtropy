# Symtropy — Complexity-Aware Physics, Causal Simulation and HHLF Research Programme

**Status:** research programme and source audit, 2026-11-16. This proposal builds on Symtropy's existing research-validation protocol. It does not claim that Φ is a physical force, that the current coupled energy ledger proves thermodynamic closure, or that the simulator is a qualified digital twin of any real platform.

## Executive decision

Symtropy should become the **physical and causal experiment substrate** in the HHLF / complexity-science programme.

The division of responsibility should be:

- **Symthaea:** form hypotheses, infer candidate mechanisms, estimate regimes, propose interventions and uncertainty.
- **Symtropy:** execute controlled, parameterized physical simulations, emit trajectories/events/invariants and expose modeled consequences of interventions.
- **SAM / independent qualification:** assess whether evidence is sufficient for a particular claim, model validity domain or physical decision.
- **Mycelix:** preserve provenance, identity and authorized sharing where appropriate; it does not turn a simulation receipt into physical truth.
- **Sovereign Boot / Ouroboros:** may visualize bounded, non-sensitive observations; never relies on Symtropy or HHLF to boot or establish trust.

The strongest paradigm shift is from a physics engine that happens to have complexity/Φ coupling to an **evidence-qualified experimental substrate**:

hypothesis -> model + assumptions -> controlled intervention -> simulation -> invariant and uncertainty audit -> independent reference -> physical validation (where needed) -> scoped claim

Symtropy should not become a second knowledge store, a universal capability authority, a robot safety controller or a duplicate benchmark registry.

## Existing strengths worth building on

The public Symtropy repository already contains:

- A const-generic N-dimensional mathematics and rigid-body substrate.
- GJK/EPA collision, CCD, contact manifolds, joints, raycasting and replay.
- Deterministic replay within a declared fixed-build/single-thread contract; cross-platform bitwise equality is explicitly not guaranteed.
- A reusable PhysicsCallback<D> coupling point.
- A distinct symtropy-consciousness-physics research layer for metric coupling, energy budgets, harmony fields, FEP/active inference and Φ-oriented experiments.
- A symtropy-hdc-physics semantic shadow that derives deterministic, versioned hypervectors from exact physics state without changing the authoritative physics world.
- A Symtropy Physics Research Validation Protocol with analytical, independent-implementation and reproducible-campaign validation tiers.
- Domain experiments and work in world simulation, robotics, fluid, energy and life-simulation directions.

Read first:
- [Architecture](https://github.com/Luminous-Dynamics/symtropy/blob/main/ARCHITECTURE.md)
- [Formal specification](https://github.com/Luminous-Dynamics/symtropy/blob/main/FORMAL_SPECIFICATION.md)
- [Physics research validation](https://github.com/Luminous-Dynamics/symtropy/blob/main/crates/core/symtropy-physics/RESEARCH_VALIDATION.md)
- [Oriented collision validation](https://github.com/Luminous-Dynamics/symtropy/blob/main/crates/core/symtropy-physics/ORIENTED_COLLISION_VALIDATION.md)
- [HDC physics semantic shadow](https://github.com/Luminous-Dynamics/symtropy/tree/main/crates/domains/symtropy-hdc-physics)
- [Current paired mission challenge work #5547](https://github.com/Luminous-Dynamics/symtropy/issues/5547) and [challenge generator #5590](https://github.com/Luminous-Dynamics/symtropy/issues/5590).

Do not duplicate the existing physics validation protocol, #5547 challenge corpus, #5488 evidence applicability, #4947 clock transforms, or #3522/#3531 physical-trial owners. Compose with them.

## Priority 0 — qualify the thermodynamic and physical semantics

A source review of the current public main branch found several places where language and measurement contracts should be tightened before results are used as physical evidence.

### 1. Energy ledger is not yet a closed-system conservation proof

The current ThermodynamicLedger uses record_action to add entity energy consumption to energy_in, and record_dissipation to add dissipation to energy_out; tick_balance compares those counters. Those are not, by themselves, the complete energy balance of a simulated world.

A physically meaningful control volume needs explicit accounting for all relevant stores and transfers, as applicable:

- kinetic and potential energy;
- entity/internal energy reservoirs;
- energy wells and external supply;
- regeneration and resource transfer between entities;
- actuator/external work;
- contact, friction, damping and collision heat;
- energy carried across the tick boundary;
- any modeled field/potential energy and numerical/integrator residual.

Consumption is not automatically energy entering a system, and dissipation is not automatically energy leaving the control volume. Heat remains energy in the world if the modeled boundary includes the thermal reservoir.

**Deliverable:** define closed-world and open-world balance contracts separately. Use a typed energy-flow event with source store, destination store, quantity, unit, sign, frame/time, causal event ID and boundary disposition. A balance may be called conserved only when all modeled stores and boundary transfers are accounted for.

### 2. Collision impulse must not be relabelled as joules

In the current coupling path, process_collisions records a fraction of collision impulse as dissipation, while the sanctuary path estimates absorbed energy as a constant multiple of impulse. Impulse has units of momentum (N·s); it is not energy (J). Converting impulse to energy requires the relevant mass/effective mass and pre/post relative velocity or an independently derived work calculation. A constant multiplier does not supply those missing quantities.

**Deliverable:** keep impulse, momentum transfer, work and dissipated energy as distinct dimensional quantities. Derive energy change from pre/post kinetic energy or the contact solver's work/energy calculation and include uncertainty/residual.

### 3. Landauer bound needs a narrower interpretation

The code comment for above_landauer_floor says a computation below the bound is thermodynamically impossible. Landauer's principle concerns the minimum heat/work cost associated with logically irreversible information erasure under specified thermodynamic conditions. It is not a universal minimum joule cost for every operation, bit read, reversible computation or arbitrary number of “bits processed.”

**Deliverable:** rename inputs/claims to represent irreversible bit erasures under declared temperature/environment assumptions. Do not apply the floor to generic operations or all neural state updates unless the number of irreversible erasures is defined and supported.

### 4. Helmholtz free energy requires a thermodynamically consistent state

The current available_work expression is max(available - temperature * entropy, 0). The field available is a bounded energy reservoir; the implementation does not establish that it is the thermodynamic internal energy U of a defined system, and the entropy reference/state assumptions are not explicit.

**Deliverable:** either model a consistent internal energy, entropy, heat bath and work boundary with units and update laws, or rename this to an application-specific energy heuristic. Do not call it Helmholtz free energy until the thermodynamic state variables meet the required definitions.

For a constant heat-capacity thermal reservoir, temperature/entropy integration must be consistent with the chosen model; document approximations and numerical error.

### 5. Public energy APIs need adversarial input contracts

In the current EnergyBudget implementation, consume, regenerate and dissipate_heat accept floating-point inputs without a shared typed validation boundary. Negative or non-finite quantities can create invalid accounting or non-finite thermal state.

**Deliverable:** reject negative, NaN and infinite values; validate initial capacities, temperatures and heat capacities; use explicit error types rather than silently mutating counters. Add negative-control tests for all public energy-flow entry points.

### 6. Φ and “safety” must remain separate from certified actuator authority

The code itself documents that the four-tier SafetyTier::from_phi cut points do not match measured values in the platform demos, and that demos instead use a platform-calibrated two-level signal mapping. This is an important qualification note.

A cognitive/integration scalar is not a safety-rated sensor, certified risk estimator, protective function or independent actuator-authority boundary.

**Deliverable:** preserve Φ or any other metric as an experimental policy input inside simulation. For hardware-facing robotics, an independently specified and validated safety chain must impose authority limits, collision prevention, E-stop and safe-state behavior. No model metric may override it, grant authority or relax the ODD. Run degraded, missing, stale, spoofed, saturated and out-of-range metric tests.

### 7. Harmony fields and Φ-to-physics coupling need empirical model labels

The radial field falloff, friction/impulse modulation, temperature-to-Φ coupling and metric-gated force rules are useful hypotheses for simulation. They are not established physical laws merely because implemented as equations. CEMI-inspired field analogies do not by themselves validate a real force field in the simulated world.

**Deliverable:** label these as explicit phenomenological coupling models, state their dimensioned variables/units, compare against uncoupled and alternative models, and state what observations could falsify them.

These source observations define audit priorities; they do not imply that every Symtropy experiment or generic physics primitive is invalid.

## Paradigm shifts

1. **From metric-coupled physics to causal mechanisms.** Every coupling channel must declare intervention semantics, dimensional units, energy/momentum consequence and falsifiable prediction. The engine should support arbitrary metrics without implying every metric has a physical law behind it.
2. **From a single Φ narrative to model ensembles.** Compare uncoupled, generic scalar-coupled, Φ-derived, random-matched and task-optimized control inputs. A specific metric earns explanatory status only when it beats controls out of sample.
3. **From one conservation percentage to a complete boundary ledger.** Track stores, transfers, external work, heat and numerical residuals. Report a conservation residual with the control volume, units and uncertainty.
4. **From deterministic replay to deterministic replay plus uncertainty.** Keep replay as a reproducibility tool, but also propagate parameter uncertainty, initial-condition uncertainty, numerical error and model discrepancy. A perfectly repeatable wrong model is still wrong.
5. **From trajectory matching to intervention validity.** Match not only observed traces but the effects of controlled changes. A model must predict what happens when mass, friction, delay, control policy or coupling is changed.
6. **From a fixed “edge of chaos” setting to task-conditioned phase maps.** Map stability, exploration, energy, prediction horizon, collision risk and cooperation as parameters vary. Do not assume a universal criticality optimum.
7. **From point estimates to viability and robust control envelopes.** Characterize the region of state/parameter space in which the model remains bounded and constraints remain satisfied under perturbations.
8. **From HDC as an extra output to a semantic observer.** Use the deterministic HDC shadow for compact episode retrieval/anomaly hypotheses; require each prediction to link back to exact physical state, encoder fingerprint and source digest. HDC must not mutate the authoritative physics world.
9. **From demos to reproducible challenge distributions.** Build seeded, paired, stage-aware scenarios that preserve coupled physical factors, unknown/out-of-envelope cases and independent reconstructions. Extend existing #5547/#5590 rather than creating a competing benchmark suite.
10. **From “artificial life” as an aesthetic to operational self-maintenance.** Use Symtropy's world/life-simulation domains to test boundary maintenance, resource flow, endogenous repair, reproduction, lineage and evolution. Complexity or survival time alone does not pass.

## Symtropy's role in HHLF

Use Symtropy for the environment/interaction and intervention layer, not to rebuild Symthaea's HDC or Ouroboros's renderer.

A useful integrated experiment:

1. Symthaea proposes a bounded multiscale HDC/CfC state model and a falsifiable intervention hypothesis.
2. Symtropy creates a versioned world with explicit mechanics, units, environment, boundaries and parameter distributions.
3. The physics engine executes fixed or sampled interventions.
4. The HDC semantic shadow encodes exact state/trajectory windows with encoder fingerprint and digest.
5. Independent analysis estimates stability, Lyapunov behavior, dimension, recurrence, information transfer and uncertainty where applicable.
6. The verifier checks source/config/seed/trajectory digests, invariants, balance residuals and model validity scope.
7. The resulting visual projection may be rendered in Ouroboros, but is never interpreted as boot integrity or consciousness evidence.

The first HHLF experiment should be a minimal local dynamical graph coupled to rigid bodies or a simple world field, not an all-in-one “living physics” rewrite.

## Research and benchmark programme

### A. Physical validity

Build on RESEARCH_VALIDATION.md; retain its validation tiers.

- Analytical reference cases: free fall, constant force, elastic collision, harmonic oscillator, pendulum, rotational invariants and other supported regimes.
- Independent implementation comparisons: Rapier/Jolt/PhysX for game rigid-body cases; MuJoCo for robotics/constrained dynamics; analytical/high-precision references where possible.
- At least four timestep values and convergence-order reporting.
- Momentum, angular momentum (where modeled), energy, contact penetration, constraint residual, rotation orthogonality and non-finite-state counts.
- Publish all failures and non-convergent cases; no survivor-only aggregates.

### B. Thermodynamic closure

Create two separate test suites:

**Closed model:** declare all stores and transfers. After accounting for boundary work, energy residual should remain within the predeclared analytical/numerical tolerance over the full trace.

**Open model:** declare every boundary transfer (external actuator work, resource input/output, heat transfer, regeneration, source/sink). The residual after transfers must be bounded and reported; do not force a zero residual by definition.

Test collision, friction, damping, impulse modulation, regeneration, energy wells, rescue transfer, high-energy impact, zero-energy collapse and restart. Include malformed/negative/non-finite input as hostile controls.

### C. Dynamical-systems and complexity benchmarks

Use qualified Symthaea analysis routines or independent reference analysis; avoid maintaining duplicate estimators in Symtropy.

- Stable linear oscillator and damped oscillator.
- Van der Pol limit cycle.
- Logistic map in fixed/periodic/chaotic regimes.
- Hénon, Lorenz-63 and Rössler.
- Quasiperiodic and stochastic negative controls.
- Coupled oscillators / Kuramoto and coupled-map lattices.
- Parameter sweep, noise, forcing, delay and component-loss regimes.

Report Lyapunov exponents with convergence and independent estimator agreement; distinguish correlation dimension from Kaplan–Yorke dimension; use recurrence statistics and surrogate tests to avoid confusing stochasticity/nonstationarity with deterministic chaos.

### D. Coupling and emergent cooperation

For each existing coupling channel, run controlled paired experiments:
- no coupling;
- generic scalar coupling with matched distribution;
- Φ-derived signal;
- shuffled/random signal with matched marginal and autocorrelation where practical;
- an oracle/upper-bound policy where it is legitimate;
- individual-channel ablations;
- sign-reversed or deliberately misspecified coupling.

Measure:
- survival/viability and accepted task output;
- resource use and complete ledger balance;
- collision/contact and recovery;
- network clustering and interaction topology;
- distributional stability, sensitivity and seed variance;
- fairness/inequality and robustness to agent heterogeneity.

A result such as “solo agents collapse, cooperative agents persist” is a model result only. To support a general cooperation claim, show it survives parameter sweeps, alternative resource policies, matched controls and model uncertainty. Preserve complete trial distributions, not just one illustrative seed.

### E. HDC semantic-shadow evaluation

Compare HDC episode retrieval/anomaly detection with:
- exact state/trajectory nearest-neighbor baselines;
- engineered feature vectors;
- time-series similarity baselines;
- a learned embedding where the task merits it.

Measure retrieval recall@k, precision@k, calibration, false-positive rate, robustness to missing/noisy state, memory footprint and latency. Keep train/test split by scenario family, parameter family and seed lineage to prevent near-duplicate leakage. HDC retrieval is a hypothesis generator, not a ground-truth physics authority.

### F. HHLF and artificial-life evaluation

Use ablations: rigid-body/world dynamics only; multiscale graph only; HDC only; liquid/continuous-time only; paired combinations; full proposed system. Evaluate:
- measured scale-dependent correlations and finite-size behavior;
- causal influence across scales under intervention;
- local/global stability and viability;
- repair after controlled node/component/agent loss;
- resource-bounded homeostasis;
- lineage and reproduction fidelity;
- variation and adaptation under changed environments;
- persistence of functional diversity.

Compare against simple controllers, non-autopoietic repair scripts, cellular automata and other appropriate artificial-life nulls. A reset script or a persistent animation is not endogenous self-repair.

## Benchmarking against the industry

Comparison must be workload-specific and matched. The existing physics research protocol already identifies Jolt, Box2D, Rapier, PhysX and MuJoCo as candidate references depending on the domain. Add Chrono or domain-specific validated simulators only when the problem requires them.

Compare:
- error at matched tolerance, not frame rate alone;
- supported shapes/dimensions and explicit limitations;
- solver convergence, contact stability and CCD coverage;
- replay contract and cross-platform variance;
- memory, allocations, latency and throughput;
- constraint and energy residual;
- robustness across adversarial geometries and parameter shifts.

Do not claim a universal physics-engine victory from a narrow microbenchmark. A useful advantage could be N-dimensional composition, deterministic replay inside a clear contract, metric-agnostic physical coupling, or an HDC-linked semantic shadow—but each must be compared on the problem it actually solves.

## Shared evidence contract across the three projects

Define a portable experiment receipt (candidate schema, not yet an implemented API) with:

- experiment and scenario IDs/revisions;
- model/engine build digest and dependency/toolchain digest;
- equations/model assumptions, units and validity domain;
- exact initial-state, parameter, random seed and intervention digests;
- integrator, timestep, solver settings and floating-point contract;
- raw event/trajectory artifact digest;
- invariant and thermodynamic-balance summaries, including all failures;
- analysis code/version, estimator settings and uncertainty;
- independent reference identity and comparison result;
- explicit claim scope, evidence level, exclusions and unresolved discrepancies.

Do not duplicate canonical identity, capability, safety or evidence-applicability authorities from existing domain owners. This receipt links their records.

## Work plan that starts without CI runners

### S0 — Research protocol and source audit
- Add this document.
- Open an issue that composes with existing physical validation and robot challenge work.
- Freeze a small canonical energy-flow contract and dimensional-analysis table.
- Add hostile-input cases for all energy operations.

### S1 — Ledger repair
- Model stores and transfers explicitly.
- Replace impulse-to-energy shortcuts with work/kinetic-energy accounting.
- Clarify Landauer and free-energy semantics.
- Add deterministic closed/open system fixtures with negative controls.

### S2 — Coupling qualification
- Define each coupling channel's units and causal mechanism.
- Add matched null controls and ablation campaigns.
- Separate simulation motor policy from hardware safety authority.

### S3 — Dynamical/complexity bridge
- Generate trajectories from fixed Symtropy worlds.
- Run independent stability/chaos/complexity estimators using Symthaea's reference programme.
- Add HDC semantic-shadow comparisons and provenance.

### S4 — HHLF substrate experiments
- Couple a small multiscale lattice to a Symtropy world.
- Test cross-scale causal intervention, viability, recovery and resource costs.
- Keep the renderer and boot integration separate from the experiment authority.

### S5 — External validation
- Compare matched workloads with independent engines/researchers.
- Publish exact source, raw data, environment and analysis digests.
- Keep claims below the strongest reproduced evidence.

## Initial acceptance targets (proposed; not achieved results)

- Zero invalid or non-finite energy-flow values admitted by validated public boundaries.
- 100% of modeled energy transfers carry explicit units, source/destination and boundary classification.
- Zero unaccounted impulses mislabeled as energy.
- Closed-system residual within a predeclared reference tolerance across all frozen fixtures and timestep refinements.
- 100% of hardware-facing authority paths remain bounded by independent safety controls regardless of Φ/metric input.
- Identical same-build replay for frozen scenario/config/seed; cross-platform differences reported, not hidden.
- Every chaos/dimension/criticality claim includes estimator convergence, uncertainty and negative controls.
- Every HDC result includes encoder fingerprint, source-state digest and held-out scenario-family evaluation.
- Every alleged emergence/repair result beats its null model under pre-registered metrics and multiple seeds.
- No SOTA claim without matched independent comparison and full failed-run accounting.

## Non-claims

This programme does not establish that Φ is a physical force, that energy use proves consciousness, that Landauer's bound applies to every operation, that a simulator is a physically accurate digital twin outside its validity domain, that HHLF is alive, or that complexity/chaos provides cryptographic hardness. It does not qualify a real robot or authorize actuator control.

## References

- Symtropy Physics Research Validation: https://github.com/Luminous-Dynamics/symtropy/blob/main/crates/core/symtropy-physics/RESEARCH_VALIDATION.md
- Symtropy Formal Specification: https://github.com/Luminous-Dynamics/symtropy/blob/main/FORMAL_SPECIFICATION.md
- Symtropy HDC Physics: https://github.com/Luminous-Dynamics/symtropy/tree/main/crates/domains/symtropy-hdc-physics
- Symthaea complexity/chaos programme: https://github.com/Luminous-Dynamics/symthaea/blob/main/docs/COMPLEXITY_CHAOS_KNOWLEDGE_PROGRAM.md
- Information theory for complex systems scientists (2025): https://doi.org/10.1016/j.physrep.2025.09.007
- Reservoir computing opportunities/challenges (2024): https://doi.org/10.1038/s41467-024-45187-1
- Landauer (1961), “Irreversibility and Heat Generation in the Computing Process,” IBM Journal of Research and Development 5(3), 183–191.
- NIST IR 8547, PQC transition guidance (final 2026-10-27): https://csrc.nist.gov/pubs/ir/8547/final

**First executable proof:** a two-body closed collision with independently computed pre/post kinetic energy, explicit heat transfer and a ledger that balances under timestep refinement; then repeat with a negative impulse/NaN input and require typed rejection. This is a more valuable first result than adding another speculative coupling channel.
