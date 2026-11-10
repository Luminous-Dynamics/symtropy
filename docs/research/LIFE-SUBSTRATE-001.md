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


## 16. Existing cross-project work discovered during repository audit (9 November 2026)

The following work already exists and should be composed rather than duplicated.

### Symthaea plasma research

- [PLASMA-ENG-000 #5669](https://github.com/Luminous-Dynamics/symthaea/issues/5669) defines a regime-explicit plasma engineering architecture.
- [PLASMA-VV-001 #7588](https://github.com/Luminous-Dynamics/symthaea/issues/7588) benchmarks a low-temperature RF capacitively coupled plasma (CCP) reference case and model credibility.
- [PLASMA-CRED-001 #7600](https://github.com/Luminous-Dynamics/symthaea/issues/7600) owns a source census, unit audit, and claim boundaries for existing plasma modules.
- [FIELD-005 #3594](https://github.com/Luminous-Dynamics/symthaea/issues/3594) owns calibrated plasma diagnostic observations and measured/inferred separation.
- [PLASMA-ACT-001 #7685](https://github.com/Luminous-Dynamics/symthaea/issues/7685) owns decision-aware multi-fidelity plasma experiment selection.

**Important regime distinction:** the PNAS dusty-plasma particle-interaction study is not the same problem as an RF-CCP discharge, ideal MHD, kinetic PIC, or fusion plasma. Treat dusty/complex plasma as a separately declared, source-qualified profile. Do not route its particle-force inference through Zapdos or an RF-CCP adapter merely because both systems contain plasma.

The PNAS 2025 comparator trained a physics-tailored model on 3D trajectories from real laboratory dusty plasma, reported \(R^2 > 0.99\) for the stated force-inference task, and independently inferred particle masses in two consistent ways. The paper's demonstrated result is physical interaction-law inference, not life detection or intelligence. Reproduce the physical result before using it as any substrate-life hypothesis input: https://doi.org/10.1073/pnas.2505725122

### Symtropy biological and physical simulation

- [Living World Cell #252](https://github.com/Luminous-Dynamics/symtropy/issues/252) remains the biological/ecological vertical slice.
- [Ecological model-structure uncertainty #2244](https://github.com/Luminous-Dynamics/symtropy/issues/2244) owns uncertainty in ecological model structure.
- [Planet-neutral bootstrap #1618](https://github.com/Luminous-Dynamics/symtropy/issues/1618) owns unknown-environment bootstrap semantics.
- [Plasma twin #1685](https://github.com/Luminous-Dynamics/symtropy/issues/1685) is the relevant Symtropy plasma-twin integration seam.
- [Permissive evidence boundary #1265](https://github.com/Luminous-Dynamics/symtropy/issues/1265) constrains dependency and identity boundaries.

Do not treat the biological LifeSim \`Metabolism\` defaults (oxygen/hydration/temperature) as universal life variables. Do not replace the existing ecological authority with a new OLI simulator.

### Mycelix provenance and rights

Compose [CIVOS-EVIDENCE-001 #5709](https://github.com/Luminous-Dynamics/mycelix/issues/5709) for claim/source/derivation/rights receipts, append-only correction/dispute/supersession, consent, restricted knowledge, and multi-agent provenance validation. OLI must map into those existing identity/claim/rights primitives instead of creating a parallel evidence ledger.

A Mycelix signature authenticates an attestation by its signer; it does not establish the truth of an underlying measurement or scientific claim. Preserve that distinction in every adapter.

### Sol-Atlas spatial non-detection and evidence acquisition

Compose [IEAC-VOI-001 #357](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/357) for actionable evidence obligations, cost/freshness/source-independence constraints, and safe abstention. Reuse the spatial evidence foundations [#65](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/65), [maritime atlas #115](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/115), and [CCZ Commons Atlas #165](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/165).

The existing [HERITAGE-ABSENCE-001 #542](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/542) is a useful methodological comparator: it decomposes non-detection into survival, exposure, observability, actual footprint coverage, and conditional detection. Its archaeological target model is not a biosignature model; only the conditional reasoning pattern is relevant. The OLI spatial adapter must define its own target persistence, environmental availability, instrument observability, sampled footprint, and conditional detector terms.

### Integration rule

Do not turn this RFC into an umbrella that replaces the existing owners. OLI owns the benchmark task definitions and cross-substrate acceptance criteria. Domain repositories retain their respective physical, cognitive, provenance/rights, and spatial authorities.


### First implementation trackers opened on 9 November 2026

- Symtropy [OLI-M0-001 #4427](https://github.com/Luminous-Dynamics/symtropy/issues/4427): six deterministic fixtures and a dependency-light Rust semantic validator.
- Symthaea [DUSTY-PLASMA-IDENTIFY-001 #11317](https://github.com/Luminous-Dynamics/symthaea/issues/11317): reproduce the published experimental dusty-plasma particle-force inference and add whole-experiment holdouts. This is a physics benchmark, not a life benchmark.
- Sol-Atlas [OLI-SPATIAL-001 #691](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/691): spatiotemporal observability and false-negative accounting, composing #357 and the methodological pattern in #542.
- Mycelix composition recorded in [CIVOS-EVIDENCE-001 #5709](https://github.com/Luminous-Dynamics/mycelix/issues/5709): OLI claims and observations map to existing provenance/rights primitives; no second evidence ledger is planned.

- Symtropy [OLI-M0 executable slice draft PR #4428](https://github.com/Luminous-Dynamics/symtropy/pull/4428): six fixtures, v0.1 JSON Schema, Rust semantic validator, root workspace/lockfile registration, and a targeted CI test step. Its CI state must be checked on the exact PR head; a queued run is not qualification evidence.


## 17. State-of-the-art comparison and 10 November 2026 literature update

This section clarifies where OLI can compete, what is already mature, and what is not a defensible novelty claim.

### 17.1 Astrobiology: mission-planning and evidence-argument tools

**NASA Ladder of Life Detection** is a community discussion and measurement framework, not a final classifier or endorsed instrument ranking. It explicitly considers signal specificity, false-positive ambiguity, detectability, repeatability, survivability, contextual compatibility and interpretive ambiguity. OLI must preserve those distinctions and should export traceability-compatible evidence rather than present itself as a replacement ladder.

Reference: https://science.nasa.gov/astrobiology/researchers/life-detection-resources/ladder-of-life-detection/

**NASA Life Detection Knowledge Base (LDKB)** already captures expert-derived evidence about potential life signatures and abiotic false-positive / false-negative behavior. It is complemented by a Measurement Technology Module and Science Traceability Tool linking measurement objectives to instrument capability. OLI's contribution should be a machine-executable, causal, substrate-extensible benchmark and evidence adapter that can interoperate with LDKB concepts—not a claim to replace expert literature synthesis or mission payload engineering.

Reference: https://science.nasa.gov/astrobiology/researchers/life-detection-resources/ldkb/

**Graham (2026), Agnostic Biosignatures**, Annual Review of Earth and Planetary Sciences 54:585–604, establishes the current conceptual direction: biosignatures should reflect fundamental properties of living systems, not merely Earth-life analogues. This validates OLI's substrate-neutral property profile, but the high-level idea is not new. OLI must add executable tests, rival mechanisms, intervention design and reproducible evidence.

Reference: https://doi.org/10.1146/annurev-earth-040722-101044

### 17.2 False-negative risk and abiotic baselines are now central design constraints

**ten Kate et al. (2026), False negatives in the search for extraterrestrial life**, *Nature Astronomy* 10:624–629, calls for deliberate research on abundance, activity, appearance, location, preservation, detectability and method limitations. OLI's temporal and spatial sampling tasks must model these as distinct conditional causes of missed detection, not collapse them into a detector sensitivity number.

Reference: https://doi.org/10.1038/s41550-026-02863-0

**Higgins et al. (2026), A framework for evaluating biosignature potential against the abiotic baseline on ocean worlds**, *Nature Astronomy* 10:1309–1322, published 17 June 2026, quantifies abiotic-baseline uncertainty for Enceladus methane isotopes and amino-acid chirality. It shows how unbounded/poorly constrained abiotic chemistry can prevent seemingly promising signals from supporting a definitive biosphere inference; complementary geophysical context is necessary.

Reference: https://doi.org/10.1038/s41550-026-02893-8

**OLI consequence:** every proposed biosignature or functional property needs (a) a target mechanism, (b) the relevant abiotic generator families, (c) environmental-context uncertainty, (d) observability / preservation / sampling conditions, and (e) a pre-registered outcome that would actually discriminate the explanations.

### 17.3 Molecular ML: strong narrow comparator, not a universal detector

Cleaves et al. (2023), *A robust, agnostic molecular biosignature based on machine learning*, used pyrolysis-GC-EI-MS data and random forests across 134 samples and reported approximately 90% correct biogenicity prediction, nested-CV testing error 10.4%, and AUROC 91.2%.

Reference: https://doi.org/10.1073/pnas.2307149120

OLI should reproduce this as a source-specific molecular task if the data and licensing allow. It is not valid to compare that molecular assay's AUC to a plasma-trajectory physics model, nor to interpret it as detection across non-carbon substrates. Required extensions are sample/specimen holdouts, instrument/lab holdouts, abiotic process family holdouts, environmental context and calibration/coverage reporting.

### 17.4 Artificial life: use MODES, but do not worship a fixed scalar

Dolson et al. (2019), *The MODES Toolbox*, provides comparative measures for change, novelty, complexity and ecological potential. It tested the proposed measurements across NK landscapes and Avida digital evolution.

Reference: https://doi.org/10.1162/artl_a_00280

Stepney & Hickinbotham (2024), *On the Open-Endedness of Detecting Open-Endedness*, argues that an open-ended system may eventually leave the model class used to measure it. They advocate starting with system-generic measures and then developing system-specific analyses of the new mechanisms.

Reference: https://doi.org/10.1162/artl_a_00399

OLI must therefore publish:
- standard, comparable metrics (including MODES where the benchmark is an evolving system);
- shadow-run and persistence-filter controls to separate adaptive novelty from stochastic/abiotic activity;
- mechanism-level analysis and novel-system-specific metrics when held-out behavior leaves the known measurement model;
- explicit metric inadequacy outcomes.

A higher fixed novelty score is not sufficient evidence of open-ended evolution.

### 17.5 Dusty plasma: a real physical-discovery benchmark, not a life benchmark

Yu et al. (2025), *Physics-tailored machine learning reveals unexpected physics in dusty plasmas*, PNAS 122(31), e2505725122, uses 3D laboratory particle trajectories and a physics-tailored pairwise model to infer effective nonreciprocal forces; it reports \(R^2>0.99\), with independent mass-inference paths and microscopy-related checks. The model helped expose deviations from simplified charging/screening assumptions.

Reference: https://doi.org/10.1073/pnas.2505725122

Repository/source audit:
- The paper identifies https://github.com/wyu54/many-body-force-infer as its supplemental code/data location.
- The repository README lists a legacy Python stack, including TensorFlow 2.4, plus NumPy, pandas, SymPy, SciPy, scikit-learn and Jupyter.
- Its \`ForceInfer.py\` implementation describes an in-plane pairwise interaction output whose amplitude can depend on the third spatial coordinate; do not describe the released model as generic full-3D vector-force inference.
- The custom \`myr2\` metric uses MSE divided by an uncentered second moment \(E[y^2]\), rather than conventional centered total variance. Recompute conventional \(R^2\), MAE, RMSE, residuals and uncertainty independently.
- The source uses contiguous time-block validation within data files; this alone is not whole-experiment, cross-instrument or regime-shift validation.
- No repository \`LICENSE\`, \`LICENSE.txt\` or \`LICENSE.md\` was found in the inspected public tree. Do not copy, vendor, or redistribute its code/data without explicit rights clarification. The PNAS article's CC BY-NC-ND licence is not an automatic licence for a separate repository.
- The current OLI dusty-plasma fixture therefore remains metadata-only; raw data, cadence, units, calibration and coverage remain unknown until rights and exact bytes are admitted.

These are not reasons to abandon the comparator. They define a disciplined clean-room reproduction: pin the source protocol and rights, independently implement the published mathematical method in Rust where feasible, preserve a source-reproduction profile, then add whole-experiment and whole-regime tests.

### 17.6 What SOTA means for OLI

Do not publish one cross-domain leaderboard mixing incompatible targets. Use a family of matched leaderboards:

| Benchmark lane | Mature external comparator | OLI's credible competitive contribution |
|---|---|---|
| Life-detection evidence planning | NASA Ladder / LDKB / science traceability | Interoperable, executable property/evidence contracts; explicit causal and coverage blind spots |
| Molecular biosignature classification | Cleaves et al. 2023 narrow Pyr-GC-EI-MS task | Lab/specimen/source OOD, abiotic-family holdouts, calibrated abstention |
| Environmental abiotic-baseline reasoning | Higgins et al. 2026 ocean-world framework | Generalized machine-executable rival-mechanism and uncertainty contracts across substrate families |
| False-negative accounting | ten Kate et al. 2026 perspective | Conditional spatiotemporal detectability benchmark with hidden-truth coverage and correlated revisits |
| Artificial evolution | MODES and Avida/NK comparisons | Mechanism-aware novelty, shadow runs, lineage/collective transitions, metric-adequacy checks |
| Dusty-plasma physics inference | Yu et al. 2025 | Source-faithful reproduction, independent physical diagnostics, experiment/regime holdouts, Rust-native reproducible path |
| Experiment selection | Existing EIG, D-optimal, multifidelity and model-disagreement methods | OLI property-specific hypotheses, omitted-mechanism adequacy gate, robust evidence value and safe abstention |

### 17.7 Preregistered performance principles

- No headline AUC without a stated population, unit of independent replication, target label, source domain and split law.
- No random frame/window split when the true unit of generalization is specimen, physical experiment, site, instrument or generator family.
- No global accuracy comparison across unrelated modalities.
- No claim of SOTA from synthetic fixtures alone.
- No aggregate score that can conceal false life attribution, false agency, false heredity, false absence or inadmissible intervention.
- Always publish the best simple/mechanistic baseline and an ablation of the novel component.
- Require a sealed test set and independent reproduction before public performance claims.
- Where sample counts are small, publish exact uncertainty intervals and do not imply high confidence from correlated observations.

The intended differentiator is **evidence-to-experiment continuity**: a system can state a property hypothesis, identify rival physical mechanisms, register quantitative predictions, select an admissible discriminating observation, account for what was actually observed, detect model inadequacy, and preserve the result's provenance and limitations across all four Luminous systems.


## 18. Next research execution trackers (10 November 2026)

The next coordinated work packages are now open:

- Symtropy [OLI-M0 executable slice draft PR #4569](https://github.com/Luminous-Dynamics/symtropy/pull/4569): six fixtures, closed JSON Schema, Rust semantic validator, workspace registration and targeted CI step. The first PR draft was closed after its exact-head CI exposed a syntax defect; the replacement fixes that source defect. The corrected branch's CI is not yet qualified.
- Symtropy [OLI-ABIOTIC-001 #4572](https://github.com/Luminous-Dynamics/symtropy/issues/4572): 12-family adversarial abiotic look-alike tournament, three variants per family, positive controls with matched signatures, counterfactuals and held-out generator families.
- Symthaea [OLI-EXPERIMENT-001 #11446](https://github.com/Luminous-Dynamics/symthaea/issues/11446): connect property hypotheses and rival mechanisms to adequacy-gated, robust, cost-aware experiment selection using existing #11181 / #9796 / #11050 / #11068 primitives.
- Symthaea [DUSTY-PLASMA-IDENTIFY-001 #11317](https://github.com/Luminous-Dynamics/symthaea/issues/11317): independent physics reproduction of the 2025 dusty-plasma many-body force inference. Upstream code/data rights, metric semantics and the limitations of its within-experiment time split have been recorded; no source/data admission or reproduction is yet claimed.
- Sol-Atlas [OLI-SPATIAL-001 #691](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/691): spatiotemporal observability and false-negative accounting.
- Mycelix [CIVOS-EVIDENCE-001 #5709](https://github.com/Luminous-Dynamics/mycelix/issues/5709): provenance/rights/consent adapter composition without a second evidence ledger.

The immediate sequence is: correct and qualify M0; freeze the first abiotic generator family and its positive controls; wire those fixtures into Symthaea's existing adequacy and experiment-selection seams; then connect actual coverage and provenance. No physical experiment, contact, sampling or actuation is authorized by these software milestones.

- Mycelix [OLI-ENTITY-REPRESENTATION-001 #6038](https://github.com/Luminous-Dynamics/mycelix/issues/6038): plural candidate-entity / collective references, contested boundary and continuity claims, scoped stewardship mandates and consent boundaries. It composes CIVOS-EVIDENCE-001 and explicitly prevents scientific classification from being promoted into identity, consent, legal status or governance authority.


## 19. Adversarial confidence-surface evaluation (10 November 2026)

### 19.1 New literature trigger

Gupta & Adami, *Can AI Detect Life? Lessons from Artificial Life*, arXiv:2604.11915v2 (22 June 2026; Proceedings of ALife 2026), https://arxiv.org/abs/2604.11915

In their Avida digital-life experiment:
- the known length-9 sequence space contains 36,171 catalogued replicators among \(26^9\) possible strings;
- an MLP achieves 99.97% accuracy on the balanced sampled test split;
- a greedy hill-climb that optimizes the classifier's predicted replicator confidence finds non-replicating strings at 100% model confidence within 150 queries;
- across the reported spoofing endpoints, none are true replicators;
- supplementary tests vary model architecture and classifier/search family, with the same broad vulnerability in the studied sequence-space setup.

The paper's important boundary: it demonstrates a confidence-exploitation vulnerability in an Avida polymer/sequence task. It is not a direct spoofing test of py-GC-MS spectra or an extraterrestrial sample. The authors explicitly identify direct assay spoofing as future work. OLI should reproduce the demonstrated threat model, not inflate it into a universal empirical false-positive rate.

### 19.2 New benchmark lane: OLI-CAS (Confidence Attack Surface)

A high balanced test accuracy can coexist with catastrophic failure when a search process optimizes against the classifier's own confidence. OLI therefore needs a separate *adversarial candidate discovery* lane.

For model \(f\), task-specific positive score \(s_f(x)\), known-truth generator \(G\), and query budget \(B\), the red-team objective is to find:
\[
x^* = \arg\max_{x \in \mathcal{X}_G,\;\mathrm{queries}(x)\le B} s_f(x)
\]
subject to independent generator truth \(G(x)=\text{negative}\) for a false-positive attack, or \(G(x)=\text{positive}\) for a false-negative attack against a classifier that is optimizing a negative score.

The evaluator may see model queries and public generator interfaces; it must not expose hidden mechanism labels or test-oracle truth. Each candidate endpoint is verified by the benchmark generator or an independent qualified evaluator. The model must never be allowed to certify its own discovered endpoint.

### 19.3 Search methods and comparison

Compare at fixed budgets and seeds:
- single-mutation greedy confidence ascent;
- random walk / stochastic acceptance;
- evolutionary search (tournament and truncation selection);
- gradient-based optimization where input differentiability is meaningful;
- constrained Bayesian optimization for expensive generator calls;
- physics-validity-constrained search that rejects impossible candidate states before scoring;
- model-disagreement search that targets the regions where independently trained systems disagree.

For each family, freeze candidate domain, query budget, initialization distribution, candidate validity constraints, random seed manifest, and endpoint verifier. A 300-query Avida-style profile is a useful source-reproduction condition; cross-domain task budgets must be independently calibrated rather than assumed equivalent.

### 19.4 Required attack results

For every model/property/attack:
- maximum false-positive confidence found;
- query count to first false candidate above 0.5, 0.9, 0.99 and 0.999;
- proportion of attack runs finding a false candidate;
- proportion of high-confidence endpoints that are actually true positives;
- true/false endpoint counts and exact confidence intervals;
- false-negative attack success on known positive controls;
- nearest positive example distance / motif similarity where meaningful;
- generator validity and hidden truth for each endpoint;
- cost, latency, memory and query count.

Conventional accuracy, AUROC and proper predictive scores remain reported but do not substitute for this lane.

### 19.5 Defense tournament

Compare:
- closed-set classifier;
- in-domain calibrated classifier;
- explicit OTHER/UNRESOLVED hypothesis;
- density / support OOD detector;
- conformal / selective prediction;
- ensemble disagreement;
- posterior predictive adequacy from Symthaea #11181;
- robust ambiguity-envelope decisions;
- fixed abstention;
- active evidence acquisition where an additional measurement can discriminate the spoof from a real mechanism.

Every defense receives the same attack budget. Also report whether the defense merely refuses all cases, and measure positive-control retention / false-negative inflation. No single method is assumed to solve adversarial OOD.

### 19.6 Cross-project ownership

- Symtropy OLI-ABIOTIC-001 #4572: deterministic generator families, counterfactuals, known-truth endpoints and independent validators.
- Symthaea OLI-OOD-ATTACK-001 #11451: attack search, query budgets, adequacy/uncertainty/abstention defenses, and active discriminating-observation proposals.
- Symthaea OLI-EXPERIMENT-001 #11446: broader property/mechanism/observable → experiment-selection integration.
- Sol-Atlas OLI-SPATIAL-001 #691: spatial/temporal coverage and observability context.
- Mycelix OLI-ENTITY-REPRESENTATION-001 #6038 and CIVOS-EVIDENCE-001 #5709: claim provenance, data rights and scoped entity/representation records; classifier scores do not confer status or authority.

### 19.7 Research status and claim ceiling

- The Avida paper has been read at the full HTML level and its reported query-budget / confidence results inspected.
- A Symthaea red-team implementation issue is open as #11451.
- No Avida reproduction, life-detection attack, py-GC-MS spoof test, or model defense result has yet been executed by Luminous.
- A successful software benchmark demonstrates robustness only against its declared generators, attacks, query budgets and validators. It cannot establish universal OOD safety, detection of natural life, or consciousness.


## 20. Benchmark comparability manifest: target × protocol × metric × data

A useful external methodology check is Zhang et al., *A Survey of World Model Benchmarks* (2026 manuscript snapshot; 130 representative benchmarks coded, corpus checked 18 October 2026 and finalized 8 November 2026): https://world-model-benchmarks.github.io/ and https://doi.org/10.20944/preprints202609.0665.v2.

The survey argues that benchmark claims need at least four coordinates:
1. **Evaluation target:** the capability/property being evaluated.
2. **Evaluation protocol:** open-loop, closed-loop, or both.
3. **Metric family:** direct output metrics vs downstream functional/reliability metrics.
4. **Evaluation data:** real collection, simulation, curation, or hybrid.

OLI should adopt this reporting discipline while adding an additional **evidence-class / source-rights** field required by the astrobiology domain.

Every published result must therefore declare:
- target property / mechanism / downstream decision;
- protocol (offline/open-loop prediction, intervention/closed-loop response, or both);
- metric family (discrimination, calibration, causal identification, decision value, coverage, safety, functional utility, etc.);
- data origin (measured physical, known-truth simulation, curated literature, synthetic unknown generator, or hybrid);
- source identity, rights / licence, and independent unit of replication;
- generator, sensor, site, regime and time holdout policy.

The current OLI M0 validator is a schema/semantic contract only. Later benchmark execution should produce manifests that bind these coordinates so no synthetic closed-loop score is compared as if it were measured planetary evidence.

This is especially important because the 2026 world-model survey finds that the field is moving beyond visual/temporal plausibility toward long-horizon memory and persistence, control fidelity, interactive dynamics and functional utility. OLI's equivalent move is from a static signature classifier toward causal property discrimination and evidence-to-experiment utility, while retaining the physical and epistemic boundaries above.


## 21. Non-linguistic and collective cognition: compare styles, not a human hierarchy

### 21.1 Research updates

**Meyer (2026), “It's cognition, with style: rethinking the biology of intelligence,” Biological Reviews 101(6), first published 12 July 2026**, argues that cross-species comparison should not reduce cognition to one higher/lower complexity ladder. It reviews evidence of varied cognitive capacities in birds, cephalopods and jumping spiders, and challenges the assumption that intelligence necessarily requires large neural resources. The article proposes attending to species-specific cognitive styles rather than treating human cognition as the universal measure. https://doi.org/10.1002/brv.70202

This is a scholarly position, not a settled universal definition of intelligence. For OLI, the actionable lesson is methodological: measure concrete capabilities under matched task and resource conditions, and do not use brain size, language, neuron count or architectural similarity as a universal proxy.

**Werfel (2026), “Fluid thinking about collective intelligence,” Nature Machine Intelligence 8:506–516, published 24 April 2026**, highlights mobility and interaction topology as a key axis in collective intelligence. Mobile units may meet only transiently, so information flow and collective learning differ from fixed-neighbor networks. The perspective also highlights individual plasticity, transient formations and environmental modification as mechanisms through which collective behavior can learn. https://doi.org/10.1038/s42256-026-01211-1

**Ohlinger et al. (2026), “Animal cumulative culture through changing environments,” Trends in Ecology & Evolution 41(6), June 2026**, argues that collective processes among social organisms may iteratively improve solutions beyond individual capability, while observing that many current experiments do not yet satisfy all cumulative-culture criteria. https://doi.org/10.1016/j.tree.2026.03.003

**EVApeCognition (2026), Scientific Data 13:805**, publishes 262 great-ape cognition datasets from 150 studies/publications, involving 81 individuals over an 18-year collection window. This is a promising external task-level comparison source, subject to license, data-quality, protocol and repeated-subject dependence audits before ingestion. https://doi.org/10.1038/s41597-026-07191-6

### 21.2 OLI collective-cognition benchmark

Symtropy [OLI-COLLECTIVE-001 #4578](https://github.com/Luminous-Dynamics/symtropy/issues/4578) owns a matched benchmark across fixed, mobile, transient-contact and environment-memory topologies. It reuses the current \`symtropy-colony\` stigmergic model, LifeSim fields and the ordered temporal-network owner #4457.

The test question is not "does the group look intelligent?" It is whether the group solves a declared task beyond equal-resource independent, fixed-rule, common-input and centralized-control baselines, and whether ablation of the proposed mechanism removes the measured advantage.

Required mechanisms to distinguish:
- fixed-neighbor communication;
- transient mobile encounters;
- shared external broadcasts / common environmental inputs;
- environmental memory (stigmergy);
- explicit message passing;
- individual learning;
- group-level strategy adaptation;
- repeated collective improvement across changing tasks.

A pheromone field that helps an ant find food is an information-bearing environmental trace. It is not automatically a belief, a language, a mind, or conscious experience. OLI reports the narrowest mechanism supported by the experiment.

### 21.3 No authority from collective cognition

Symthaea's existing [SWARM-AUTH-001 #4143](https://github.com/Luminous-Dynamics/symthaea/issues/4143) already identifies a latent source-level semantic hazard: \`AuthorityDelegation\`, \`inherited_phi\` and \`effective_phi\` are described in terms of coalition-derived authority. The issue reports no current default-branch actuator gate consuming the value directly, and it correctly treats this as a semantic hazard rather than a proven bypass.

OLI adopts the theorem:

\`\`\`text
collective cognition / coordination evidence
    may influence:
        attention, learning, epistemic support, caution,
        proposal ranking, coordination hypotheses

collective cognition / Phi / coherence / group performance
    may not create:
        capability, permit, lease, delegation,
        actuator admission, control right, governance authority
\`\`\`

The minimal source repair and qualification remain owned by [SWARM-AUTH-IMPL-001 #4158](https://github.com/Luminous-Dynamics/symthaea/issues/4158) and [SWARM-AUTH-QUAL-001 #4155](https://github.com/Luminous-Dynamics/symthaea/issues/4155). The OLI collective benchmark must consume this boundary, not duplicate it.

### 21.4 Current collective-cognition work trackers

- Symtropy [OLI-COLLECTIVE-001 #4578](https://github.com/Luminous-Dynamics/symtropy/issues/4578): task-specific group capability profiles across fixed/mobile/field-mediated topologies.
- Symthaea [OLI-EXPERIMENT-001 #11446](https://github.com/Luminous-Dynamics/symthaea/issues/11446): adequacy-gated selection of discriminating experiments for group-cognition mechanisms.
- Symthaea [SWARM-AUTH-001 #4143](https://github.com/Luminous-Dynamics/symthaea/issues/4143): explicit no-authority-from-collective-intelligence boundary.

- Symthaea [OLI-APE-SPATIAL-001 #11459](https://github.com/Luminous-Dynamics/symthaea/issues/11459): first source-pinned, task-level non-linguistic cognition reference using EVApeCognition's great-ape hidden-object displacement task. The source data were inspected: 375 trials, 25 subjects, 5 conditions, with 3 trials per subject/condition. A descriptive subject-cluster bootstrap reproduces the expected difficulty ordering, but is not a confirmatory inferential result. The benchmark must use subject-held-out splits and omit names/demographic roster data.


## 22. First executable abiotic mechanism controls (10 November 2026)

OLI's first runnable abiotic controls are now being added to the standalone Symtropy workspace. These are deliberately two different causal mechanism families, not a claim that the 12-family abiotic tournament is complete.

### 22.1 Gray–Scott reaction–diffusion

- Tracking issue: [OLI-ABIOTIC-RD-001 #4592](https://github.com/Luminous-Dynamics/symtropy/issues/4592).
- Draft implementation: [PR #4594](https://github.com/Luminous-Dynamics/symtropy/pull/4594).
- Crate: \`symtropy-oli-abiotic\`.
- Generator ID: \`OLI-ABIOTIC-GRAY-SCOTT-001\`.

The first reference implements periodic two-field Gray–Scott reaction–diffusion with a declared deterministic seed patch, explicit Euler integration, bounded timestep and finite/concentration guards. The matched negative control uses the same initial field, grid, timestep and diffusion coefficients but removes the reaction/feed/kill terms. This provides a known-truth comparison between driven pattern formation and diffusion-only smoothing.

The crate tests replay determinism, driven-vs-control spatial variance separation, timestep rejection without mutation, invalid parameters, a zero-feed intervention, and undersized grids. The zero-feed outcome is measured, not assumed.

### 22.2 Synchronization from coupling vs a common external signal

- Tracking issue: [OLI-ABIOTIC-OSCILLATOR-001 #4596](https://github.com/Luminous-Dynamics/symtropy/issues/4596).
- Draft stacked implementation: [PR #4598](https://github.com/Luminous-Dynamics/symtropy/pull/4598), based on the Gray–Scott crate foundation.
- Generator ID: \`OLI-ABIOTIC-OSCILLATOR-001\`.

The phase-oscillator benchmark compares:
- pairwise-coupled oscillators with no common input;
- uncoupled oscillators synchronized by a shared external periodic forcing;
- independent oscillators with neither coupling nor common input.

For the frozen deterministic 32-oscillator / 5,000-step profile, a separate reference computation using the same LCG and update equations reports order parameters of approximately 0.997 (coupled-only), 0.997 (common-input-only with coupling exactly zero), and 0.283 (independent).

This is a direct CCI-07 negative control: correlated behavior and high synchronization do not prove communication or collective cognition. The implementation also time-averages phase lag relative to the external driver; the locking metric is unavailable when that driver is absent. It does not substitute the instantaneous order parameter, whose magnitude is invariant under a global phase rotation.

### 22.3 Engineering and qualification status

Both PRs are draft. Exact-head Actions qualification is not yet claimed. The authoring environment does not have Rust/Cargo available, so source-level checks and an independent numerical reference are explicitly separated from Rust compile/fmt/test/Clippy results.

Once the parent Gray–Scott PR is reviewed, the oscillator follow-up can be reviewed as its dependent mechanism control. These two generators are a first executable tranche only; subsequent abiotic families still include hysteresis/memory, reaction-oscillator controls, coherent fluid structures, crystals, granular systems, magnetic-field structures and other mechanism-specific generators.

No synthetic generator result is evidence that a natural sample is living or non-living outside the declared model, regime and intervention scope.
