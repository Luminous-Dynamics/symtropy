# Symtropy locomotion — 2026 competitive map and research strategy

**Snapshot:** 5 November 2026  
**Status:** research positioning and benchmark design; no SOTA claim.

## Executive position

Symtropy should not try to replace FlyGym as a physics simulator, COPEx as an amortized constrained Bayesian experimental-design method, or improv as an adaptive neuroscience orchestration platform. Those projects solve valuable, different problems.

The opportunity is to make the **scientific claim connecting neural mechanism, intervention, observed trajectory and physical outcome independently checkable**. Symtropy's differentiator should be a source-qualified causal model and decision/evidence layer that can be attached to existing high-quality simulators and experiment-control systems.

The system should have four cooperating layers:

1. **Embodied physical twin:** pinned FlyGym/NeuroMechFly physics, measured joint/contact/actuator streams and standard locomotion controllers.
2. **Causal sensorimotor model:** per-leg mechanics and presynaptic/sensory state, persistent physical and acquisition confounds, competing mechanisms, and intervention-specific transition likelihoods.
3. **Experiment policy:** matched one-step and multi-step BED policies, exact finite-state oracle on enumerable cases, and approximate online state-space inference for larger models.
4. **Evidence and execution contract:** source/materialization pins, randomized assignment versus delivered intervention, protocol fidelity, sample and replicate identity, pre-outcome history, independent replay, and explicit abstention/model-inadequacy status.

The innovation is the verified connection between these layers, not claiming ownership of each layer in isolation.

## What the field already does well

### Embodied physics and locomotion control — FlyGym / NeuroMechFly

FlyGym 2.1.1 is the baseline to pin, not a capability to reimplement. Its current interface exposes joint angles, actuator forces and contact forces; supports mechanosensory feedback; and its 2.1.x examples include CPG, rule-based and hybrid controllers. The 2.1.1 changelog adds separate trochanter/femur bodies, GPU state APIs, trajectory recording and other API improvements. Its official documentation reports substantial GPU acceleration; benchmark timing must be taken from the pinned release and measured with synchronization, not inherited from old version claims.

**What Symtropy should do better:** predict the *effect of changing a specified biological mechanism* on held-out movement/contact/force trajectories, and quantify whether that prediction remains valid under per-leg, context, sensor and physical shifts. Do not claim novelty for mechanosensory feedback, trajectory recording, CPGs or hybrid CPG-plus-feedback control.

### Constrained adaptive experiment design — COPEx (ICML 2026)

COPEx combines offline amortization of a design policy/posterior network with online multi-step scenario-tree lookahead under changing costs, budgets and physical constraints. This is a serious comparator for sequential experiment design; a finite exact oracle on small kernels is not a like-for-like efficiency winner.

**What Symtropy should do better:** attach action semantics and evidence contracts to the experiment graph; distinguish the state being measured from the instrument; model serial dependence and carryover; make unsupported/non-identifiable claims lead to abstention; and use the exact oracle as a correctness/regret reference rather than claiming it is the best large-scale planner.

### Partially observed dynamical systems — online BED with state-space models (ICML 2026)

Perez-Vieites et al. address online Bayesian experimental design for partially observed dynamical systems, where latent-state integration makes likelihoods/EIG difficult and approximate online inference is required. This is the relevant comparator for trajectory-aware dynamic inference.

**What Symtropy should do better:** use the same frozen tasks/priors/costs to expose whether approximation error, particle degeneracy, conditional calibration and history leakage change the selected intervention or scientific conclusion. Preserve exact small-case checks and report approximation error rather than comparing only mean EIG.

### Adaptive neuroscience orchestration — improv (Nature Communications 2025)

improv integrates flexible data streams, analysis pipelines, real-time model updates and hardware execution; it demonstrates adaptive workflows in several neuroscience settings, including in-silico and in-vivo experiments. It establishes that orchestration, concurrency and live control are essential engineering problems.

**What Symtropy should do better:** remain a domain-specific causal qualification layer that can be orchestrated by a platform like improv. The model should state why a protocol is valid, what was actually delivered, which historical information conditioned the prediction, and what evidence tier supports each claim.

### Connectome-derived rhythm generation — Pugliese et al. (Nature, 28 October 2026)

Pugliese et al. report a DNg100-driven three-neuron motif for motor rhythm generation, with necessity/sufficiency results across four connectome datasets and a separate optogenetically tested DNb08 pathway. This is now a strong source-grounded rhythm baseline, but its claim must be kept at the neural-motor-rhythm layer. A rhythm result alone does not establish complete tripod coordination, contact adaptation, closed-loop locomotion or an embodied fly.

**What Symtropy should do better:** reproduce the source outputs and graph identities first (#3511), then test whether distinct CPG, local FeCO feedback and distributed brain-and-cord pathways make different held-out predictions under matched interventions. Keep DNg100 and DNb08 separate; keep the four connectome materializations, sex/animal populations and evidence tiers separate; compare neural rhythm, inter-leg coordination and physical gait as distinct outcomes. The paper's reported Zenodo DOI 10.5281/zenodo.22260923 differs from the author-linked record 22260924; #3511 correctly leaves that discrepancy unresolved pending source verification.

### Whole-cycle autonomous science — Autonomous Science Bench and FIRE-Bench (2026)

Autonomous Science Bench 0.1 evaluates end-to-end laboratory agents on six pilot tasks and measures both campaign progress and delivered result quality. Its published first release has only three trials per task; small leaderboard gaps are therefore not strong evidence. FIRE-Bench tests evidence-backed rediscovery of established scientific insights and reports substantial limitations and run-to-run variance in current agents.

**What Symtropy should do better:** make the locomotion benchmark auditable at the level of physical interventions and source-locked observations, not just task resolution or narrative quality. Publish every failure, negative control, abstention, mismatch and model revision.

## Competitive comparison

| Capability | Strong external baseline | Symtropy target | What would count as real progress |
|---|---|---|---|
| Physics and morphology | FlyGym/NeuroMechFly 2.1.1 | Integrate and measure against the pinned simulator | Matched simulator receipts; no unsupported speed claims |
| Standard locomotion controllers | CPG, rule-based, hybrid; learned recurrent/RL | Same morphology, solver, sensor streams and budget | Higher held-out stability/recovery or lower cost without worse worst-decile performance |
| Adaptive design | COPEx; one-step joint EIG | Exact reference plus matched constrained multi-step policies | Lower cost-to-decision with no risk/calibration regression |
| Dynamic inference | Online state-space BED / nested particle methods | History-conditioned physical+sensor state estimation | Better held-out intervention log score and calibrated intervals |
| Experiment execution | improv-style streaming/orchestration | Interoperable evidence/receipt contract | Reproducible action-to-observation lineage under dropped/delayed/mismatched events |
| Scientific-agent evaluation | Autonomous Science Bench; FIRE-Bench | Sealed locomotion rediscovery and intervention tasks | Correct mechanism discrimination with evidence, not plausible prose |

## Paradigm shifts worth pursuing

### 1. From trajectory imitation to intervention-predictive twins

A model that reproduces a trajectory may still have the wrong causal mechanism. The core score should be held-out predictive performance under interventions: suppress a pathway, alter a sensor channel, change load on a particular leg, or move between active and passive contexts, then predict the measured response distribution.

### 2. From one global controller to distributed, context-conditioned mechanics

Represent each leg's mechanics and sensor state explicitly, but make coupling sparse and declared: ipsilateral, contralateral, tripod, intersegmental, descending and shared mechanical/environmental factors. Use the separate factorized per-leg work in PR #3677; do not expand every combination into an unbounded dense Cartesian kernel.

### 3. From information gain to robust decision value under model inadequacy

EIG is conditional on the supplied hypothesis set and likelihoods. A planner can be very informative about a wrong model family. Add model-criticism actions, posterior predictive checks, planted omitted-mechanism controls, robust/maximin policies and an explicit "model inadequate; revise hypotheses" outcome. Reward calibrated abstention over false confidence.

### 4. From aggregate scores to replicate-aware evidence

Treat animal/preparation as the biological replication unit, session as a clustered repeated-measure unit, and trial/sample as a nested observation. Preserve complete history and use conditional likelihoods for sequential updates. Do not allow a large number of frames or repeated trials from one animal to impersonate independent biological replication.

### 5. From source labels to source-bearing likelihoods

A paper, connectome release or simulator name attached to a mechanism is insufficient. Every transition/observation likelihood should retain source materialization, evidence tier, claim ceiling and provenance. Distinguish connectome-derived constraints, simulated observations and measured biology at every inference boundary.

## Benchmark targets (ambitions, not results)

Use the separate layers below; never collapse them into a single leaderboard score.

### A. Exact computational correctness

- 100% exact agreement with independent exhaustive policy-tree enumeration on enumerable small kernels.
- 100% exact factorized-to-dense equivalence on small enumerable cases in PR #3677.
- Zero accepted malformed probability tables, missing parents, invalid action semantics, invalid provenance, or declared-resource-budget violations.
- Report state count, factor width, intermediate factor sizes, memory, runtime and solver limits.

### B. Experimental-design value

- Compare seeded random, balanced factorial, fixed schedule, one-step joint EIG, EIG/cost, maximin, COPEx-style constrained multi-step policy, and online state-space BED where paper-matched.
- Target at least 10% lower median cost-to-decision on predeclared stateful-confounding/per-leg intervention families, with paired 95% intervals excluding zero and no worse false-identification, hazard or abstention rates.
- Report regret against exact finite-horizon oracle on tractable cases.
- No global SOTA claim from a handful of synthetic cases.

### C. Predictive and statistical validity

- Primary: held-out intervention predictive log score, stratified by leg, context, intervention, individual and session.
- Secondary: Brier score, interval coverage, calibration slope/intercept, posterior predictive residuals and mechanism recovery.
- Target calibrated 90% and 95% predictive intervals on the corresponding held-out data; report per-stratum coverage and uncertainty, not just aggregate ECE.
- At least 30 independent synthetic seeds per family where compute permits; use biological replicate counts based on power/precision planning, not a fixed convenience count.
- Zero false confident identifications on non-identifiable or omitted-mechanism controls under the preregistered abstention policy.

### D. Embodied locomotion quality

- Same pinned FlyGym version, morphology, physics solver/tolerance, timestep, terrains, sensors, actuation limits, controller rate, hardware and training/calibration budget.
- Report gait success, trajectory error, fall rate, disturbance recovery, contact timing/impulse, foot slip, joint-limit/actuator saturation, energy/work and worst-decile performance.
- Require improvement on at least one preregistered hard stratum (e.g. terrain transition, sensor dropout, per-leg load perturbation) without a material regression on flat ground or the remaining strata.
- Separate controller-only inference cost from full lifecycle cost (training/calibration plus inference).

### E. Evidence integrity

- Zero contact/load interventions accepted when they also alter target-leg joint position, joint velocity, gait phase or actuator force.
- Zero non-target leg mechanical changes under a single-leg action.
- Zero accepted source/claim-ceiling, assignment/delivery, low-fidelity, sample-identity or history-digest contract violations.
- Every run binds exact source commit, simulator build, physics settings, scenario asset digest, action manifest, seed and replayable trajectory/evidence receipt.

## Current work and next boundary

- PR #3656: source-aware locomotion adapter, per-leg sensory/physical states and intervention contracts.
- PR #3677: factorized per-leg dynamics intended to prevent dense Cartesian expansion.
- Issue #3357: pinned FlyGym 2.1.1 controller-baseline protocol.
- Issue #3021: SIM-REL-014 matched/sealed experiment-design evaluation protocol.
- Issue #3588: sequential intervention planning / evidence control track.
- #3547: history-adjusted dynamic interventional state-space inference.
- #3511/#3518/#3519: source-lock and biological circuit/data reproduction tracks.

The next integration should link factorized per-leg mechanics to pinned FlyGym trajectory receipts, then fit/qualify conditional transition and observation models on source-locked interventions. Do not call the six-leg bins a calibrated biomechanical digital twin until they are tied to measured units, coordinate frames, timebases and held-out trajectories.

## References

- FlyGym/NeuroMechFly docs: https://neuromechfly.org/
- FlyGym 2.1.1 changelog: https://neuromechfly.org/2.1.1/changelog/
- COPEx, Guo et al., ICML 2026: https://proceedings.mlr.press/v306/guo26g.html
- Online Bayesian Experimental Design for Partially Observed Dynamical Systems, Perez-Vieites et al., ICML 2026: https://proceedings.mlr.press/v306/perez-vieites26a.html
- improv, Draelos et al., Nature Communications 2025: https://www.nature.com/articles/s41467-025-64856-3
- Autonomous Science Bench 0.1: https://www.autonomous-science-bench.ai/
- FIRE-Bench, Wang et al., ICML 2026: https://proceedings.mlr.press/v306/wang26d.html
- FeCO presynaptic inhibition, Dallmann et al., Nature 2025: https://doi.org/10.1038/s41586-025-09554-2
- Pugliese et al., Nature 2026, DOI: https://doi.org/10.1038/s41586-026-11195-y
