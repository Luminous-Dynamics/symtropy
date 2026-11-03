# Locomotion Mechanism Identifiability Contract V1

**Status:** Research and benchmark contract only. No completed biological reproduction, prospective animal intervention, mechanism identification, or SOTA result is claimed.

**Parent work**
- [ANIMA-NEURO-002 / #3345](https://github.com/Luminous-Dynamics/symtropy/issues/3345) — CPG-to-contact locomotion and model-family discrimination.
- [LOCO-BENCH-001 / #3357](https://github.com/Luminous-Dynamics/symtropy/issues/3357) — matched FlyGym 2.1.1 and controller baselines.
- [ANIMAL-GAIT-METRICS-001 / #3353](https://github.com/Luminous-Dynamics/symtropy/issues/3353) — versioned raw gait metrics.
- [Symthaea DROSOPHILA-WALKING-001 / #9934](https://github.com/Luminous-Dynamics/symthaea/issues/9934) — reproduction of Pugliese et al.
- [Symthaea blind-intervention benchmark / #9587](https://github.com/Luminous-Dynamics/symthaea/issues/9587), [optimal perturbation design / #9592](https://github.com/Luminous-Dynamics/symthaea/issues/9592), and the discrete-hypothesis EIG work in [#9589](https://github.com/Luminous-Dynamics/symthaea/issues/9589).

This document adds a missing layer: a pre-unblinding contract for model predictions, an explicit test of whether candidate mechanisms are distinguishable under available interventions, and a claim ladder that prevents a rhythm result from being promoted into a locomotion or mechanism claim.

## 1. Scientific motivation: do not collapse explanatory levels

Pugliese et al., Nature (28 October 2026), report connectome simulations in which descending drive, including DNg100, recruits rhythmic motor-neuron activity, and network pruning identifies a compact rhythm-generating interneuron motif. The reported minimal motif and exact pruning outcome are dataset-dependent; the paper includes a four-interneuron FANC motif as well as three-interneuron examples. Critically, the paper explicitly reports that DNg100 simulations did not generate tripod inter-leg coordination despite producing rhythmic leg motor output. The authors point to proprioceptive feedback, biomechanical coupling and/or additional pathways as possible missing ingredients. Treat this as a central negative result and the concrete Symtropy bridge to explain—not as an incidental limitation. The reduced motif is not, by itself, evidence of stable, coordinated, physically embodied walking. Full-network and optogenetic results are separate evidence.

Sapkal et al., Central versus peripheral neural control of a coordinated walking pattern in Drosophila (bioRxiv version published 3 May 2026; preprint), report a complementary sensory-deprivation result: local per-leg rhythm, contact/load and descending contributions to within-leg step structure, and central coupling plus proprioceptive/descending modulation of inter-leg coordination. This is a preprint and must retain that evidence status.

References:
- [Pugliese et al., Nature (2026), DOI 10.1038/s41586-026-11195-y](https://doi.org/10.1038/s41586-026-11195-y)
- [Sapkal et al., bioRxiv 2026.04.29.721658, v2](https://www.biorxiv.org/content/10.1101/2026.04.29.721658v2)
- [FlyGym releases](https://github.com/NeLy-EPFL/flygym/releases), pinned for external neuromechanical comparison at v2.1.1 / commit a63dc91.

Preserve six outcome levels:

1. **Circuit activity:** membrane/rate/spike dynamics and local rhythm generation.
2. **Motor output:** motor-neuron or motor-unit activity and muscle activation.
3. **Within-limb coordination:** joint sequence, phase relations, stance/swing and load-linked step microstructure.
4. **Between-limb coordination:** relative phase, tripod-group overlap and condition-dependent coordination.
5. **Embodied locomotion:** actual contact, body displacement, slip, stability, turning and disturbance recovery.
6. **Biological validity:** agreement with rights-cleared, source-grounded observations and interventions in the relevant species, individual and condition.

A pass at level N does not imply a pass at level N+1. A spectral peak in motor output is not a gait; a gait metric is not proof of causal circuit identity; a visually plausible trajectory is not biological validation.

## 2. Candidate model families

Keep models as explicit competing hypotheses. Do not collapse them into one hybrid before testing their distinct predictions.

| ID | Candidate family | Minimum declared structure |
|---|---|---|
| M1 | Pugliese full connectome model | Dataset/release-bound VNC network, declared descending input and source-grounded neural dynamics |
| M2 | Pugliese reduced rhythm motif | Exact dataset-specific pruned circuit, motor-neuron projection and stimulus scope |
| M3 | Sapkal distributed local-CPG hypothesis | Per-leg local rhythm modules, central coupling pathways, load/proprioceptive feedback and descending context |
| M4 | Independent local oscillators | Separate limb oscillators without central coupling; mechanical coupling must not be smuggled in as neural coupling |
| M5 | Phenomenological coupled oscillators | Explicit phase/amplitude equations and coupling topology, without a connectome claim |
| M6 | Reflex/PID or rule-based controller | Sensor-driven control with no inferred central rhythm circuit |
| M7 | Learned recurrent/RL controller | Pinned policy, training data, observation channels, training budget and random seeds |
| M8 | Connectome-constrained sensorimotor hybrid | Source-grounded local dynamics plus explicit central coupling, sensory feedback, descending modulation and embodied mechanics |

Every model descriptor must state species/taxon, life stage, connectome/release, included and omitted cells/pathways, synapse/sign assumptions, dynamics family, parameter priors, sensor/effector mapping, solver and evidence tier. A model with omitted sensory pathways must not receive those signals through an unrecorded side channel.

Negative controls are first-class candidates:
- degree- and cell-type-preserving rewired connectomes;
- sign-preserving versus sign-permuted edge controls where biologically meaningful;
- shuffled neuron/annotation identity controls;
- sham perturbation and random-target controls;
- sensory-channel permutation or delay controls;
- mechanical-only coupling controls.

Null models receive the same calibration, optimization and data budget as positive models.

## 3. Prediction manifest: commit before unblinding

For every candidate model and every registered intervention, produce an immutable prediction record before exposing held-out outcomes.

The record must bind:
- experiment and intervention IDs, protocol version and evidence tier;
- organism/taxon, life stage, individual/session scope and initial-state distribution;
- model family, code, parameters, connectome, sensor map, effector map and configuration digests;
- the exact observed channels available to that model;
- prediction time grid, units, missing-data rules and measurement/observation model;
- predictive mean or median, predictive interval/distribution, and uncertainty decomposition;
- expected direction and latency of each preregistered effect, where scientifically justified;
- stopping, failure and abstention criteria;
- random seeds and replicate plan;
- timestamp and commitment hash created before outcome unblinding.

Predictions must include raw time-resolved outputs for primary channels, not just a final score. Aggregate features are declared in advance and derived from those outputs. No post-hoc feature selection, threshold tuning, phase-filter tuning or metric substitution is allowed on the held-out set.

Uncertainty must remain decomposed, at minimum, into parameter uncertainty within a model, individual/session variation, measurement and observation-kernel uncertainty, model-discrepancy uncertainty, and numerical/solver uncertainty. Do not combine these into one opaque confidence band. If a component cannot yet be estimated, mark it unknown and widen or withhold the corresponding claim.

## 4. Intervention matrix and discriminative intent

Select interventions because candidate models predict meaningfully different outcomes—not because they create dramatic-looking failures.

| Intervention family | Primary contrast | Required observations |
|---|---|---|
| Descending drive increase/decrease or withdrawal | Local rhythm autonomy versus descending dependence/modulation | Local rhythm frequency/amplitude, motor recruitment, within- and between-leg timing |
| Candidate rhythm-core silencing/ablation | Necessity of specific source-grounded circuit elements | Neural activity, motor outputs, recovery/compensation, dataset-specific identity |
| Local inter-leg coupling perturbation | Independent local oscillators versus centrally coupled modules | Pairwise phase, tripod overlap, per-leg rhythm and variability |
| Proprioceptive/mechanosensory reduction | Central rhythm versus peripheral feedback contribution | Local period, within-leg sequence, coordination, sensory response |
| Contact/load alteration | Load-dependent step microstructure and feedback loops | Contact impulses, stance duration, joint sequence, motor response |
| Mechanical disturbance / terrain change | Neural robustness versus mechanics-only compensation | Body speed, slip, stability, recovery latency, neural and motor traces |
| Sensory latency/noise perturbation | Feedback timing sensitivity and stability margin | Oscillation stability, phase drift, recovery and controller failure rate |

Each row must be tagged as one of:
- **Published observation:** a measured intervention already reported in a cited source.
- **Source-grounded in-silico intervention:** a perturbation executed only in a model.
- **Prospective biological proposal:** an experiment not yet performed and requiring its own scientific, ethical and practical review.

A simulated ablation is never described as an observed biological ablation. A published sensory-deprivation result is not silently treated as a connectome-specific perturbation if the source did not identify the targeted neurons. Preserve unsuccessful/unstable screens as their own outcome class; do not recode them as non-rhythmic controls. The Pugliese source audit reports 74 of 933 descending-neuron screens unstable in all replicates and not scored.

For every intervention, specify target and exact identity (stable neuron ID, pathway, sensory channel, load/contact parameter or mechanical condition); intervention waveform, onset, duration, amplitude, washout and sham condition; permitted observation channels and acquisition/processing configuration; matched control condition and pairing unit; feasibility/safety constraints and evidence tier; and predicted response for every candidate model, including uncertainty and explicit no-prediction states.

## 5. Model identifiability audit: allow “not distinguishable”

Before running a held-out intervention, estimate whether it can separate the model set.

Let candidate model m under intervention a and initial condition x predict an observable feature vector with mean μ_m(a,x) and covariance Σ_m(a,x). Let Σ_obs represent separately estimated measurement/observation uncertainty. One useful screening statistic for a pair of models is:

    D²(i,j | a,x) =
      (μ_i - μ_j)^T [Σ_i + Σ_j + Σ_obs + εI]^{-1} (μ_i - μ_j)

The regularization ε, feature scaling, covariance estimator and any dimensionality reduction must be fixed using training/calibration data only. This statistic is not exact expected information gain and must not be labelled as such. It is a noise-aware separation heuristic.

For each candidate intervention, report:
- minimum pairwise separation across model pairs;
- average and distribution of pairwise separation;
- which pairs remain observationally equivalent;
- separation by outcome level (circuit, motor, limb, whole-body);
- sensitivity to parameter draws, individual variation and observation-model uncertainty;
- feasibility, cost and safety/rights constraints.

An intervention that separates only one easy pair but leaves the scientifically important alternatives indistinguishable must not be presented as a general mechanism-discrimination experiment.

Use the discrete-hypothesis expected information gain implementation in Symthaea #9589 when predictive distributions and priors are available. Keep it distinct from parameter-identification objectives such as Fisher information and D-optimality in #9592. Do not relabel a pairwise-distance heuristic as EIG or Fisher information.

The decision should be a Pareto view, not a single opaque weighted score:
- expected model discrimination;
- worst-pair discrimination;
- parameter identifiability;
- expected predictive improvement;
- intervention cost and duration;
- feasibility and safety;
- robustness to prior/model misspecification.

If no feasible intervention separates the remaining candidates above a preregistered resolution threshold, report an **empirical equivalence class for this benchmark domain** and abstain from claiming unique mechanism identification. This is a scientifically valuable result, not a failed benchmark.

## 6. Leakage-resistant calibration and blind evaluation

The split unit must be the biological source unit, not a randomly selected time point.

Minimum requirements:
- split by individual and session before preprocessing, feature extraction, normalization or parameter fitting;
- reserve entire intervention targets or intervention families for held-out evaluation where sample size permits;
- preserve a prospective untouched test set;
- keep repeated trials from one individual/session in the same partition unless a specific within-individual adaptation experiment is declared;
- fit observation filters, phase extraction, scaling, imputation and feature selection only on training/calibration partitions;
- keep the test-set intervention identity blind to model developers until prediction records are committed;
- disclose exclusions, missingness, failed recordings and failed runs.

Random time-window splits may be used for exploratory debugging only. They do not support claims of individual or intervention generalization.

For individual-specific modelling, report both population-level performance on unseen individuals and within-individual adaptation performance with a declared adaptation budget and no leakage from the test intervention.

## 7. Evaluation: vector scorecard and proper scoring rules

Do not optimize one composite locomotion score. Preserve the following dimensions.

**Circuit and motor dynamics**
- oscillation frequency, amplitude, spectral stability and recruitment;
- event timing and phase relationships;
- motor-unit/muscle activation and neural-to-motor prediction error.

**Limb and whole-body coordination**
- within-leg sequence and joint phase;
- pairwise phase metrics with the exact extraction/filter method ID;
- tripod coordination using explicit metric IDs (TCS_NATVAR_2024, TCS_SAPKAL_2026_V2, or another precisely declared definition);
- contact/load correlation, stance/swing, duty factor and per-leg asymmetry;
- center-of-mass velocity, slip, stability, disturbance recovery and energy/cost proxies where observable.

**Predictive quality**
- held-out trajectory error on declared channels;
- negative log predictive density or log score where the distribution is well specified;
- continuous ranked probability score (CRPS) or another preregistered proper scoring rule;
- interval coverage and interval width/sharpness;
- calibration stratified by individual, intervention family and condition;
- worst-decile performance and failure/abstention rate.

**Scientific and computational cost**
- parameter count and effective degrees of freedom;
- data and calibration budget;
- accepted/rejected solver steps, derivative evaluations and numerical failures;
- wall time, memory and energy where measurable;
- simulator fidelity, hardware, drivers and batch/world count;
- model and source-data rights/provenance.

Confidence intervals and paired comparisons must respect individual/session clustering. Preserve raw per-trial values. Use the existing animal replay scorecard (#2145) for paired aggregation when qualified; this document does not create a second scorecard.

## 8. Fair SOTA comparison and scope boundaries

The external embodied baseline is FlyGym 2.1.1 / commit a63dc91 and its NeuroMechFly controller set—not an older v2.0 or v1 measurement. The release notes document changes that directly affect benchmarking: GPU timing synchronization was corrected because earlier timing could overstate throughput by up to about 50%; the GPU solver default is 1e-5; the release uses MuJoCo and MuJoCo Warp 3.16; and GPU batch execution is affected by the slowest world. Pin all of these in the benchmark receipt.

The fair matched-controller matrix remains owned by #3357: coupled-oscillator CPG; rule-based/decentralized controller; hybrid CPG plus sensory-feedback rules; recurrent/RL controller with pinned training budget; PID/reflex baseline; Symtropy phenomenological controller; and connectome-constrained and hybrid candidates.

Use identical morphology, physics/contact settings, timestep, sensor access, observation latency/noise, task horizon, initial-state distribution, training/calibration budget and hardware. Separate inference-only and training-inclusive cost. Synchronize GPU timing and include reset/forward semantics. Report both per-world throughput and slowest-world/batch latency; never hide failures behind aggregate FPS.

Do not compare the published Pugliese neural network runtime directly to FlyGym rigid-body throughput as if they were equivalent workloads. They answer different performance questions. Report neural integration cost, body simulation cost and full closed-loop cost separately.

## 9. Multi-fidelity promotion gates

| Tier | Model/evidence | Permitted claim |
|---|---|---|
| 0 | Analytic and synthetic known-answer systems | Numerical/API correctness only |
| 1 | Phenomenological circuit plus synthetic body/contact | Contract and controlled-mechanism qualification |
| 2 | Source-grounded connectome model with explicit neural/motor mapping | Connectome-constrained circuit prediction in declared scope |
| 3 | Closed-loop neural + sensory + body/physics simulation | In-silico embodied intervention prediction |
| 4 | Held-out measured biological observations/interventions | Perturbation-grounded prediction in measured species/condition |
| 5 | Prospective replication across individuals/labs/conditions | Replicated biological mechanism evidence within stated scope |

A promotion requires exact source/version digests, numerical qualification, calibration evidence, leakage checks, negative controls and an independent reproduction receipt. A result at one tier cannot be silently promoted to the next.

Cheap models may screen intervention candidates and parameter regions. They cannot substitute for the higher-fidelity simulation or biological evaluation that supports the final claim. Measure and report discrepancy between fidelity levels on shared cases before using low-fidelity rankings to allocate expensive runs.

## 10. Proposed targets — aspirational, not achieved

After a baseline corpus and measurement variance are established, preregister:
- at least 10% relative improvement in held-out predictive score over the strongest non-connectome baseline on designated intervention strata;
- 90–95% predictive interval coverage on held-out conditions, with interval sharpness reported;
- no more than 5% relative degradation on baseline locomotion when adding biological constraints;
- at least one intervention that separates a biologically meaningful model pair above a predeclared resolution threshold;
- zero unreported numerical, physics, data-integrity or provenance failures;
- no unique mechanism claim when the candidate set remains observationally equivalent.

The first four are hypotheses to test, not universal biological constants. Freeze thresholds only after sample size, baseline variance, measurement quality and the primary outcome are documented. Hard integrity violations are fail conditions regardless of average score.

## 11. Execution sequence

1. **Freeze artifacts:** pin the Pugliese paper/code/data version, Sapkal preprint version, connectome releases, rights and exact FlyGym 2.1.1 commit.
2. **Reproduce the neural result:** reproduce rhythm-generation claims per dataset, retaining dataset-specific circuit identity and source mapping. Do not yet call this locomotion.
3. **Build prediction records:** for every model/intervention pair, generate blinded time-series predictions and uncertainty using training/calibration data only.
4. **Run identifiability screening:** expose which model pairs candidate interventions can and cannot separate; select a small Pareto set.
5. **Verify numerics:** compare the production DOPRI path against independent stiff and non-stiff methods on shared known-answer and biological-rate fixtures; numerical failure is a failed run, not a missing datum. The Pugliese source implementation uses Diffrax with throw disabled and then sanitizes non-finite result entries to zero and clips rates. An independent reproduction must preserve raw solver status and invalid-output counts; it must never turn numerical failure into an ordinary zero-rate trace.
6. **Close the sensorimotor loop:** admit only qualified receptor/body observations, emit bounded muscle proposals, and let the authoritative body/physics stack determine contact and motion.
7. **Run matched controller benchmarks:** use #3357 scenarios and the same body, physics, observations, compute and training budget.
8. **Unblind once:** evaluate frozen prediction records on the untouched intervention set; preserve all strata and failed runs.
9. **Promote cautiously:** move from synthetic to source-grounded, in-silico embodied, and measured biological claims only when the corresponding gate passes.

## 12. Non-goals

- No universal fly CPG claim and no transfer of insect circuitry to mammals.
- No claim that a rhythm-generating subnetwork alone explains tripod coordination or locomotion.
- No animal intervention or biological efficacy claim from synthetic/in-silico results.
- No SOTA, throughput or energy win without matched measurements.
- No post-hoc change to metrics, filters, interventions, splits, thresholds or candidate models after held-out unblinding.
- No single scalar score that erases a failed outcome dimension.
- No merge or promotion of a result solely because a workflow was queued or a documentation contract passed.
