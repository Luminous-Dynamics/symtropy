# LIFE-SUBSTRATE-001: Open-world life and intelligence benchmark

**Status:** Research contract / first integration plan. This document does not claim that any non-biological or plasma-based life has been discovered. No life classifier, intelligence classifier, consciousness test, or field instrument is qualified by this document.

**Owner:** Symtropy research substrate, composed with existing Symthaea, Mycelix, and Sol-Atlas capabilities.

**Branch:** \`research/life-substrate-001-v0\`

## 1. Objective

Create a reproducible, substrate-agnostic research benchmark for distinguishing:

1. physical self-organization;
2. persistence and self-maintenance;
3. information retention and transfer;
4. adaptation and learning;
5. reproduction, heredity, and evolution;
6. agency and intelligence;
7. consciousness or sentience, which remain separate and much harder questions.

The benchmark must work across biological, collective, chemical, mineral, plasma, electromagnetic, and synthetic systems without presuming that all such systems are alive or intelligent.

The primary research question is:

> Can a system infer which organizational properties are supported by observations and controlled interventions, while detecting when its measurement model, substrate assumptions, or candidate explanations are inadequate?

This is a benchmark and research-infrastructure effort—not a universal definition of life, a claim of extraterrestrial discovery, or an autonomous experimentation authority.

## 2. Paradigm shifts

### 2.1 Replace a single life label with a property profile

Do not begin with a binary \`alive / not alive\` classifier. Represent separately observed or inferred properties, each with its own evidence, scope, uncertainty, and counter-explanations.

A system can self-organize without being autonomous; persist without self-maintenance; reproduce a pattern without heredity; adapt without learning; coordinate without intelligence. The benchmark must make these distinctions observable.

### 2.2 Replace passive pattern matching with intervention-first discrimination

A static image or time series can make abiotic systems look life-like. Prefer controlled perturbations that distinguish competing explanations:

- alter available energy while holding other boundary conditions fixed;
- perturb a candidate boundary or information channel;
- remove or mask a suspected signal source;
- change the environment and test recovery;
- introduce a controlled disturbance and measure whether the system compensates;
- test whether a putative memory changes future responses;
- compare the observed response to matched passive and non-adaptive controls.

Intervention proposals are advisory research plans. Physical experiments require independent safety, containment, ethical, and facility approval.

### 2.3 Treat model inadequacy as a first-class result

A candidate set that omits the true mechanism can still produce a confident winner. Reuse Symthaea's existing open-world model-adequacy work and preserve an explicit \`OTHER_OR_UNRESOLVED\` path. Do not renormalize away zero-support observations or silently force a new substrate into a familiar category.

### 2.4 Measure false negatives as deliberately as false positives

A detector must be tested under dormancy, low abundance, sparse sampling, long characteristic timescales, sensor mismatch, spatial aliasing, weak signals, occlusion, and environmental mismatch. "Not detected" must never be silently converted into "not present."

### 2.5 Separate physical laws, functional descriptions, and consciousness hypotheses

Do not encode a consciousness score, integrated-information estimate, coherence metric, or other contested proxy as a physical force or universal life criterion. Such quantities may be research measurements with explicit operational definitions and validity limitations. They are not proof of consciousness and must not authorize actuation, governance rights, or safety-critical behavior.

## 3. Scope and non-goals

### In scope

- a versioned observation / intervention / claim / evidence contract;
- a reusable property profile for candidate systems;
- adversarial abiotic controls and positive biological references;
- simulation and laboratory-data benchmark adapters;
- out-of-distribution (OOD) detection and calibrated abstention;
- causal and counterfactual tests;
- multi-timescale and spatial-coverage evaluation;
- reproducible baselines, split manifests, and independent verification;
- cross-project provenance and spatial/environmental context.

### Explicitly out of scope

- asserting that plasma, crystals, storms, fields, or anomalies are alive;
- claiming discovery of extraterrestrial life or non-human consciousness;
- a single scalar "life score" or "intelligence score";
- treating language, neurons, oxygen, water, DNA, or human-like behavior as universal requirements;
- inferring moral or legal status solely from a classifier;
- autonomous physical experiments, sample collection, contact, or intervention;
- using Φ, coherence, or other consciousness proxies as physical control variables;
- treating synthetic benchmark success as evidence that an unknown natural entity exists.

## 4. Shared conceptual contract

The first shared artifact should distinguish five records. Keep them independently versioned; an inference must never overwrite an observation.

### 4.1 Observation

An observation records what an instrument or source actually measured:

- stable observation ID and schema version;
- subject or sampled-region ID, if known; unknown identity is permitted;
- time interval, clock basis, and temporal resolution;
- spatial frame, footprint, resolution, and coverage;
- modality and measured variable;
- numeric or categorical value, unit, uncertainty, and detection limit;
- instrument / sensor identity, calibration record, and processing version;
- acquisition conditions and sample handling;
- raw-data URI, content digest, licence / access terms, and custody chain;
- contamination, confounder, and missingness flags.

A model-generated estimate is a derived result, not a raw observation.

### 4.2 Candidate system

A candidate system is a provisional grouping of observations that may share a physical or functional identity. It must allow:

- a provisional boundary with uncertainty;
- multiple competing boundary hypotheses;
- fragmented or distributed identity;
- nested components and collectives;
- changing boundaries over time;
- a system to be unresolvable at current sensor resolution.

Do not assume that "one detected object" equals "one organism."

### 4.3 Property assessment

Assess each property independently with one of:

- \`SUPPORTED_WITHIN_SCOPE\`
- \`CONTRADICTED_WITHIN_SCOPE\`
- \`INSUFFICIENT_EVIDENCE\`
- \`NOT_TESTED\`
- \`NOT_OBSERVABLE_WITH_CURRENT_INSTRUMENT\`
- \`MODEL_INADEQUATE\)

Every result binds the supporting observation and intervention IDs, the tested scope, alternative explanations, relevant negative controls, uncertainty, evaluator version, and exact data/model digests.

These labels are not a ranking. A missing observation is not a contradiction; a failed instrument is not evidence of absence.

### 4.4 Intervention and response

An intervention record states:

- the hypothesized mechanism being discriminated;
- controlled variable(s) and intended perturbation;
- variables held fixed and possible spillover effects;
- treatment, sham, and passive-control groups;
- pre-registered outcomes and stopping rules;
- safety / containment / ethics approval references;
- actual intervention delivered and deviations;
- observed response, latency, recovery, and uncertainty.

A proposal is not an experiment. A simulation is not a physical intervention. A response is not proof of agency unless passive physical explanations are addressed.

### 4.5 Evidence claim

A claim binds a property assessment to its complete evidence chain:

- claim text and scope;
- evidence IDs and source lineage;
- inference method and version;
- assumptions and model family;
- calibration and uncertainty;
- confounders and plausible counter-explanations;
- independent replication status;
- known limitations and explicit falsifiers;
- reviewer / attester identity and conflicts;
- immutable digest and supersession relation.

Do not promote a scientific claim merely because it has accumulated a fixed number of endorsements. Reviewer count, institutional reputation, cryptographic integrity, and empirical support are different dimensions.

## 5. Property dimensions

The initial ontology is deliberately extensible. These dimensions are questions, not mandatory checkboxes for all possible life.

| Dimension | Operational question | Required caution |
|---|---|---|
| Organization | Are there persistent, non-random relationships among components? | Complexity alone is not life |
| Boundary | Is there a measurable distinction between system and environment? | Boundaries may be fuzzy, nested, or distributed |
| Energy / matter throughput | Does the system use or transform resources while maintaining organization? | A dissipative structure is not automatically alive |
| Persistence | Does organization persist beyond transient relaxation? | Persistence alone is insufficient |
| Homeostasis / regulation | Does the system compensate for perturbations? | Passive feedback must be controlled for |
| Repair / self-maintenance | Does it restore damaged organization using available resources? | Distinguish repair from simple relaxation |
| Information retention | Can state encode information that affects later behavior? | Correlation is not a memory mechanism |
| Information transfer | Can information pass between components or generations? | Signal propagation is not necessarily communication |
| Reproduction | Can it create successor systems or structures? | Replication without heredity is not evolution |
| Heredity / variation | Are consequential traits transmitted with variation? | Require lineage and transmission evidence |
| Adaptation | Does response improve under changing conditions? | Separate adaptation from acclimation or selection artifacts |
| Learning | Does experience change future responses in a transferable way? | Compare against fixed and passive baselines |
| Agency | Are outcomes better explained by internally organized, context-sensitive action selection than by passive dynamics? | Requires intervention and rival-model comparison |
| Intelligence | Does it learn, predict, generalize, plan, or solve novel tasks? | Report task family and substrate-specific limits |
| Collective intelligence | Does a group solve tasks beyond matched independent-agent baselines? | Coordination is not automatically cognition |
| Sentience / consciousness | Is there evidence relevant to subjective experience? | No current benchmark result here establishes experience |

The assessment vector must be multi-dimensional and partially populated. There is no required global life classification.

## 6. Substrate-neutral environmental representation

A substrate profile should describe a physical model, not encode assumptions from terrestrial organisms.

Candidate environmental variables include:

- temperature, pressure, radiation, gravity, and time-varying boundary conditions;
- electromagnetic field components and gradients;
- charge density, plasma parameters, particle trajectories, and collision / interaction observations;
- chemical species, reaction rates, pH, redox potential, and solvent properties where measurable;
- energy and matter fluxes, reservoirs, and dissipation;
- mechanical stress, fluid flow, diffusion, and transport;
- spatial scale, temporal scale, sampling cadence, and instrument resolution.

Use explicit units and physically meaningful uncertainty. Unsupported variables remain unknown, not zero. Do not use oxygen, water, carbon, or a conventional biological metabolic model as mandatory fields.

For Symtropy, retain fast typed biological reference paths. Add substrate-neutral extension records or namespaced channels rather than breaking the existing \`FieldLayer\` / \`Metabolism\` contracts or forcing every candidate through oxygen / hydration assumptions.

## 7. Benchmark architecture: OLI-0.1

OLI means **Open Life & Intelligence**. The first version is a task suite, not a single leaderboard.

### Task families

| ID | Task | What it tests |
|---|---|---|
| OLI-01 | Abiotic look-alikes | Reject self-organization, crystals, convection, oscillations, vortices, and field structures when life properties are unsupported |
| OLI-02 | Known biological references | Detect supported properties across microbes, archaea, extremophiles, plants, animals, and collectives |
| OLI-03 | Source / instrument holdout | Generalize to unseen instruments, sites, acquisition pipelines, and sample populations |
| OLI-04 | Substrate shift | Avoid assuming that known chemistry, morphology, or field structure exhausts possible substrates |
| OLI-05 | Temporal blind spots | Test dormancy, rare events, slow dynamics, aliasing, and inadequate observation windows |
| OLI-06 | Spatial blind spots | Test sparse sampling, fragmented boundaries, occlusion, and incomplete coverage |
| OLI-07 | Intervention discrimination | Separate adaptive compensation, repair, memory, and agency from passive responses |
| OLI-08 | Information persistence | Test whether perturbation-resistant state carries information that changes later response |
| OLI-09 | Reproduction and heredity | Distinguish copied patterns from lineage, heritable variation, and selection |
| OLI-10 | Open-ended evolution | Extend MODES with lineage persistence, ecological interaction, and major transitions in individuality |
| OLI-11 | Collective cognition | Compare group performance to matched independent, randomized, and fixed-rule baselines |
| OLI-12 | Intelligence transfer | Test prediction, learning, planning, and generalization on held-out task families |
| OLI-13 | Calibration and abstention | Reward uncertainty, \`OTHER_OR_UNRESOLVED\`, and explicit model-adequacy failures |
| OLI-14 | Active observation | Select the next measurement or intervention by decision-relevant information gain, value, cost, and safety |

### Required dataset strata

1. **Positive reference set:** documented terrestrial biological systems with specimen/source/condition holdouts.
2. **Abiotic hard-negative set:** physical and chemical systems that self-organize or retain structure without the target property being present.
3. **Synthetic known-truth set:** simulations and digital artificial-life systems with complete state and causal ground truth.
4. **Experimental non-biological set:** physical systems such as dusty plasma and active matter; labels apply to measured physics, not assumed life.
5. **OOD / hidden-generator set:** unseen generative mechanisms, environmental regimes, sensors, and temporal scales.
6. **Blind challenge set:** sealed until evaluation; includes both false-positive traps and false-negative stress cases.

Synthetic positives must never be described as evidence of natural life. A system with unknown truth remains unknown; it must not be relabeled to make the benchmark convenient.

### Split law

- Split by specimen / physical system / experiment, not by image crop, time window, particle, or row.
- Keep all repeated observations from one specimen/system in one fold.
- Hold out entire source institutions, instruments, operating regimes, and generator families.
- Fit preprocessing, feature selection, calibration, thresholds, and model selection on training data only.
- Use nested model selection and a genuinely untouched final challenge set.
- Publish exact split manifests, IDs, seeds, source digests, and data-rights records.
- Do not pool heterogeneous targets or measurement units without a documented equivalence map.
- Treat independent experimental systems—not correlated windows—as the inferential sample size.

## 8. Metrics and proposed targets

No aggregate score may compensate for an unsafe false-positive rate or hide a failed substrate family.

### Detection and discrimination

Report per-task and per-family:

- sensitivity / recall and specificity;
- false-positive rate at fixed sensitivity and sensitivity at fixed false-positive rate;
- AUROC and AUPRC where class balance makes them meaningful;
- balanced accuracy and macro-F1;
- likelihood ratios and uncertainty intervals;
- calibration (Brier score, log loss, reliability curves, ECE);
- subgroup and source-held-out performance.

**Proposed challenge target:** on a sufficiently large, independently sampled abiotic blind set, aim for a false-positive point estimate below 1%, with a one-sided 95% exact-binomial upper confidence bound below 1%. This requires at least 299 independent negative cases with zero false positives; correlated windows do not count as independent cases. Treat this as a demanding research target, not an achieved result or universal release criterion.

For known biological references, target at least 90% sensitivity while maintaining calibrated uncertainty, but report this only for the documented organisms and measurement domains actually tested.

### Open-world behavior

- OOD detection AUROC / AUPRC by held-out substrate and generator family;
- selective risk versus coverage;
- abstention precision and recall;
- rate of unjustified closed-set classifications;
- calibration under domain shift;
- recovery time after a regime change;
- error localization by sensor, spatial coverage, and timescale.

**Proposed target:** at least 95% abstention on pre-registered unsupported-regime cases, while preserving declared minimum coverage on supported in-domain cases. Publish the full risk-coverage curve; do not optimize abstention by refusing everything.

### Causal / functional properties

- intervention effect recovery and confidence-interval coverage;
- treatment-versus-sham discrimination;
- model comparison against matched passive dynamics;
- false agency attribution on passive controls;
- memory retrieval after controlled perturbation;
- repair success versus spontaneous relaxation;
- lineage and heritability error rates;
- adaptation and learning transfer to novel contexts.

### Evolution and collective intelligence

Reuse MODES concepts (change, novelty, complexity, ecological potential) and report the raw time series, not only a final scalar. Add lineage persistence, extinction, diversity, ecological feedback, parasite / exploitability resistance, and major transitions in individuality. Include shadow runs in which adaptive dynamics are disabled or reset, so novelty generated by the environment or measurement process is not mistaken for evolutionary activity.

For collective systems compare:
- group performance;
- matched independent components;
- randomized communication;
- fixed-rule coordination;
- communication ablation;
- resource / energy / latency budgets.

### Reproducibility and engineering

- 100% exact source / model / protocol / split / result digest agreement;
- 100% rejection of malformed and semantically inconsistent fixtures;
- deterministic replay where model and numerical assumptions permit it;
- independent reference implementation for small benchmark cases;
- full data licensing / attribution manifest;
- compute, memory, latency, energy, and instrument-cost reporting;
- explicit failed, inconclusive, and not-run outcomes.

## 9. Baselines and state-of-the-art comparison

Compare against multiple families, not a single convenient model:

- simple physical / mechanistic models and preregistered thresholds;
- logistic regression, random forest, and gradient-boosted trees;
- one-class and anomaly-detection baselines;
- Bayesian hierarchical models with explicit source / instrument effects;
- calibrated open-set classifiers and conformal / selective prediction methods;
- physics-informed and symmetry-constrained models;
- graph neural and temporal models for interacting many-body systems;
- causal / system-identification models;
- MODES and established artificial-life measurements;
- Symthaea HDC / CfC candidates, with ablations for HDC, liquid dynamics, context, and active observation.

The baseline must be allowed to win. HDC, CfC, active inference, and any consciousness-related metric receive no presumed advantage. Compare matched data, splits, hardware, tuning budgets, and inference costs; publish confidence intervals and all negative results.

### Existing published comparators

**Molecular biosignatures:** Cleaves et al. (2023), PNAS, reported approximately 90% biotic / abiotic discrimination and ROC AUC 91.2% on 134 carbon-bearing samples using pyrolysis-GC-MS and random forests. Reproduce this as a narrow molecular comparator. It is not a universal life detector and does not establish detection of plasma, mineral, or non-carbon life. Source: https://doi.org/10.1073/pnas.2307149120

**Agnostic biosignatures:** Graham (2026), *Annual Review of Earth and Planetary Sciences*, provides the current conceptual foundation for generalizing biosignatures beyond Earth-like molecular analogues. Source: https://doi.org/10.1146/annurev-earth-040722-101044

**Life-detection knowledge:** NASA's Life Detection Knowledge Base organizes candidate signatures and evaluates biological / abiotic prevalence and signal strength, including false-positive and false-negative reasoning. Use it as a reference taxonomy and expert argument source, not as a machine-learning leaderboard. Source: https://science.nasa.gov/astrobiology/researchers/life-detection-resources/ldkb/

**Open-ended evolution:** The MODES Toolbox provides established measures for change, novelty, complexity, and ecological potential. The literature also cautions that any fixed metric may fail to represent genuinely novel behavior; use system-generic measures first, then inspect mechanisms and system-specific novelty. Source: https://doi.org/10.1162/artl_a_00280 and https://doi.org/10.1162/artl_a_00399

**Dusty-plasma system identification:** Yu et al. (2025), PNAS, used 3D particle trajectories and physics-tailored machine learning to infer effective interactions in laboratory dusty plasma; they reported trajectory-fit R² above 0.99 across five experiments and checked inferred particle properties by independent methods. This is a strong physics-discovery comparator, not evidence of plasma life or intelligence. Source: https://doi.org/10.1073/pnas.2505725122

### How Luminous can exceed current practice

The goal is not to claim better planetary instruments, better analytical chemistry, or a discovery record we do not have. The differentiator should be an end-to-end, open, reproducible bridge across:

1. multi-modal physical observations;
2. substrate-neutral property hypotheses;
3. intervention design and causal discrimination;
4. explicit model-adequacy / OOD detection;
5. spatial and temporal sampling coverage;
6. evidence provenance and independent replication;
7. responsible entity representation and stewardship.

A result beats SOTA only when it outperforms the strongest relevant comparator on the same admitted data and held-out split, with uncertainty and independent reproduction. "Broader architecture" is not itself a performance result.

## 10. Plasma research track: physics first, life claims last

Start with the reproducible physics of dusty plasma, not the assertion that plasma is alive.

**Stage P0 — dataset and rights admission**
- Fetch the PNAS 2025 paper's released data / code repository.
- Verify its licence, file provenance, exact experiment boundaries, calibration / units, and data quality.
- Store source metadata and checksums; do not redistribute files unless permitted.

**Stage P1 — reproduce physical inference**
- Reproduce the published many-body force inference with its stated evaluation protocol.
- Compare with analytical force laws, simpler regression, and an unconstrained predictor.
- Hold out entire experiments and operating regimes, not random trajectory rows.
- Check parameter recovery, invariance, uncertainty, and physical residuals—not R² alone.

**Stage P2 — characterize candidate organization**
- Measure persistence, structural transitions, response to confinement changes, information retention, and disturbance recovery.
- Use matched passive and abiotic controls.
- Keep plasma state, dust-particle dynamics, and any proposed information-bearing structure as separate objects.

**Stage P3 — test life-like hypotheses**
Only formulate a life hypothesis if the previous stages show repeatable, independently verified evidence relevant to self-maintenance, bounded organization, information persistence, or successor generation. Replication of a shape, a vortex, or a helix is not reproduction of an organism.

**Stage P4 — intelligence hypotheses**
Require evidence of learning, prediction, generalization, or goal-directed adaptive behavior that survives comparison against passive physics, engineered control, and environmental feedback explanations. A complex trajectory or unexplained anomaly is not intelligence.

## 11. Cross-project ownership and reuse

Do not create four competing ontologies, evidence ledgers, spatial systems, or model-adequacy engines.

### Symtropy — physical reference models and repeatable experiments

Owns the physical scenario, deterministic reference implementations, simulation controls, experiment replay, conservation checks, and benchmark execution.

Reuse:
- \`symtropy-lifesim-core\` as the current biological/ecological reference substrate;
- existing \`symtropy-colony\` and \`symtropy-mycelium\` domain work;
- [#252 Living World Cell](https://github.com/Luminous-Dynamics/symtropy/issues/252);
- [#2244 ecological model-structure uncertainty](https://github.com/Luminous-Dynamics/symtropy/issues/2244);
- [#1618 planet-neutral bootstrap / unknown-environment semantics](https://github.com/Luminous-Dynamics/symtropy/issues/1618);
- [#1265 permissive evidence boundary](https://github.com/Luminous-Dynamics/symtropy/issues/1265).

Do not overload the Living World Cell with exotic plasma physics. It remains the qualified biological/ecological vertical slice; OLI composes it as one reference family.

### Symthaea — hypotheses, cognition, active observation, calibration

Owns candidate models, property inference, predictive adequacy, causal discrimination, active observation proposals, calibration, and abstention.

Reuse rather than duplicate:
- [#11181 open-world model adequacy and robust decision](https://github.com/Luminous-Dynamics/symthaea/issues/11181);
- [#9796 open-world adequacy and robust EIG](https://github.com/Luminous-Dynamics/symthaea/issues/9796);
- [#11050 optimal experiment selection](https://github.com/Luminous-Dynamics/symthaea/issues/11050);
- [#11068 prediction ledger / theory discrimination](https://github.com/Luminous-Dynamics/symthaea/issues/11068);
- existing evidence-governed substrate-adaptation research and its independent-verifier boundary.

HDC / CfC is a candidate representation, not a privileged or pre-validated solution. Every claimed improvement needs ablation and matched baselines.

### Mycelix — identity, evidence provenance, consent, governance, stewardship

Owns stable provenance, authorship, scoped attestations, relationships, permissions, dispute / supersession, and plural representation.

It must not convert reputation, reviewer count, consensus, or cryptographic proof of record integrity into scientific truth. A signature proves who attested to a record; it does not prove the underlying observation is correct. A candidate entity's possible moral status must not be mechanically determined by the classifier.

### Sol-Atlas — spatial context and observation coverage

Owns spatial reference frames, candidate habitats, environmental layers, instrument footprints, coverage gaps, temporal-spatial provenance, and map-based uncertainty.

Reuse existing geospatial / evidence owners; do not add a second generic evidence ledger. Compose:
- [#65 geographic evidence foundations](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/65);
- [#115 maritime spatial evidence](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/115);
- [#165 CCZ Commons Atlas](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/165);
- [#357 evidence acquisition / value-of-information protocol](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/357).

### Shared flow

\`\`\`text
Sol-Atlas: where / when / coverage / environment
             |
             v
Symtropy: physical state / intervention / replay / conservation
             |
             v
Symthaea: hypotheses / prediction / adequacy / abstention
             |
             v
Mycelix: provenance / scoped attestations / consent / stewardship
             |
             +---- signed evidence and decisions back to all systems
\`\`\`

No project may upgrade another project's hypothesis into an established fact merely by importing it.

## 12. First executable milestones

### M0 — freeze a small common contract
Produce versioned JSON fixtures for:
- one known biological reference;
- one abiotic self-organizing hard negative;
- one dusty-plasma physics-only case;
- one unknown / insufficient-observation case;
- one false-negative case caused by temporal undersampling;
- one out-of-domain sensor / environment case.

Include positive, negative, and malformed fixtures. All values must have explicit units, source identity, scope, and uncertainty. No external data is redistributed until its rights are verified.

### M1 — reuse the existing Symtropy Living World Cell
Map one existing biological/ecological case to the contract. Do not replace its canonical state or add a parallel plant / fauna / ecology authority.

### M2 — build the abiotic negative-control tournament
Add deterministic synthetic cases for convection, reaction-diffusion, crystal growth, coupled oscillators, fluid vortices, field structures, and simple engineered feedback. The expected result is property-specific, not a single life label.

### M3 — reproduce one physical many-body dataset
Use the PNAS dusty-plasma study as a physics system-identification exercise. Keep physical inference and life hypotheses in separate benchmark reports.

### M4 — connect Symthaea's open-world adequacy and active-observation primitives
Ensure unsupported regimes yield explicit abstention and evidence obligations. Do not generate an experiment from an unqualified model or execute an experiment.

### M5 — Sol-Atlas sampling / coverage adapter
Represent the observation footprint, instrument resolution, temporal window, and unobserved regions. Generate false-negative stress cases from known synthetic ground truth.

### M6 — independent evidence and stewardship adapter
Bind evidence manifests and scoped attestations. No trust score, automated vote, or consciousness proxy may authorize contact, experimentation, or physical effect.

## 13. Qualification gates

- **G0 — contract:** schemas and examples validate; semantic negative controls reject; unknown and missing remain distinct.
- **G1 — source admission:** exact source, data rights, licence, instrument, units, and calibration recorded.
- **G2 — reference correctness:** deterministic fixtures and independent small-case oracle agree.
- **G3 — baseline parity:** published comparators reproduced under their stated conditions; discrepancies documented.
- **G4 — generalization:** whole-specimen, whole-experiment, source, instrument, regime, and generator-family holdouts.
- **G5 — open-world:** calibrated abstention, false-positive / false-negative reporting, and no silent closed-set fallback.
- **G6 — causal:** intervention / sham / passive-control comparisons validate the claimed property.
- **G7 — independent replication:** a separate implementation or team reproduces the result from the source manifest.
- **G8 — operational / ethical review:** required before physical experiments, entity contact, sampling, or consequential action.

A queued or skipped CI workflow is not a pass. A simulation is not a physical experiment. A benchmark pass is not evidence of extraterrestrial life. An unrun gate remains pending.

## 14. Acceptance and claim ceiling

The initial deliverable is a documented and executable benchmark protocol with fixtures and independent validators. It may establish that the software correctly distinguishes declared synthetic cases and reproduces admitted physical-analysis baselines.

It cannot establish that plasma life exists, that any unknown system is conscious, or that an automated entity classification is morally or legally authoritative.

The strongest successful outcome is not a high score on one classifier. It is a system that can:

- detect known properties across varied physical substrates;
- reject convincing abiotic look-alikes;
- identify when the current model family is inadequate;
- explain which observations and interventions support each property;
- expose sampling blind spots and false-negative risk;
- reproduce results independently;
- remain appropriately uncertain when no valid conclusion is available.

## 15. References

1. Neveu et al., NASA Ladder of Life Detection: https://science.nasa.gov/astrobiology/researchers/life-detection-resources/ladder-of-life-detection/
2. NASA, Life Detection Knowledge Base: https://science.nasa.gov/astrobiology/researchers/life-detection-resources/ldkb/
3. Davila et al. (2025), *Life Detection Knowledge Base: Taxonomy of Potential Biosignatures*, Astrobiology 25(7): https://doi.org/10.1089/ast.2022.0156
4. Graham (2026), *Agnostic Biosignatures: Expanding the Search for Life in the Solar System*, Annual Review of Earth and Planetary Sciences 54: https://doi.org/10.1146/annurev-earth-040722-101044
5. *False negatives in the search for extraterrestrial life*, Nature Astronomy, 21 May 2026: https://www.nature.com/articles/s41550-026-02863-0
6. Cleaves et al. (2023), *A robust, agnostic molecular biosignature based on machine learning*, PNAS 120(41): https://doi.org/10.1073/pnas.2307149120
7. Dolson et al. (2019), *The MODES Toolbox: Measurements of Open-Ended Dynamics in Evolving Systems*, Artificial Life 25(1): https://doi.org/10.1162/artl_a_00280
8. *On the Open-Endedness of Detecting Open-Endedness*, Artificial Life: https://doi.org/10.1162/artl_a_00399
9. Yu et al. (2025), *Physics-tailored machine learning reveals unexpected physics in dusty plasmas*, PNAS 122(31): https://doi.org/10.1073/pnas.2505725122
