# AERO Design-for-Inspection and Sensing Integrity Benchmark v0.1

**Owner:** Symtropy AERO inspection / observability track #4165  
**Composes:** AERO material evidence #7319 / #7342, physical pilot #7396, AP242/QIF interop #7395, Mycelix AeroCommons #6347, Symthaea uncertainty #7412 and evidence-action selector #13112  
**Status:** benchmark protocol only; no physical NDE/POD, SHM, or airworthiness result  
**Claim ceiling:** synthetic challenge semantics and future physically correlated design/inspection evidence

## 1. Research objective

Move inspection from a late maintenance check to a design-time property of the aircraft, manufactured detail and evidence plan.

NASA Langley's Design-for-Inspection (DFI) work proposes an automation-friendly inspectability metric using simulation tests to interrogate defect findability and sensitivity to variability while covering parameter space efficiently. It explicitly frames inspectability as a metric that can enter multi-objective design optimization alongside mass and manufacturability. This is a useful external methodology comparator, not a result that Luminous has reproduced.

The second problem is sensing integrity. An anomalous structural-health signal can be caused by structural damage, sensor degradation, environment/operating-state variation, acquisition faults or unknown conditions. A system that "corrects" a sensor fault without preserving the original signal can accidentally erase a real damage signature.

The benchmark therefore asks:

1. Can a design be assessed for the inspectability of declared defect families before design freeze?
2. Can inspection methods be matched to flaw size/orientation/depth, access, geometry and material/process context?
3. Can SHM distinguish structural damage from sensor faults and environmental shifts while retaining raw observations?
4. Can inspection evidence be connected to the exact physical unit, configuration, NDE method, equipment, calibration, acquisition time and uncertainty?
5. Can evidence-directed design/testing reduce total verification burden without lowering a hard detection or engineering requirement?

## 2. Boundaries and ownership

- Symtropy #4165 owns aerospace inspection, NDE/SHM observability semantics, DFI/inspection design interfaces and its benchmark.
- Mycelix owns serialized physical-unit/build/inspect/test lineage and artifact digest/byte-length integrity.
- Symthaea uncertainty #7412 owns observation-level uncertainty and sample/dependence provenance.
- Symthaea BASALT/ASSURE own generic evidence freshness, observer coverage and qualification lifecycle.
- Symthaea #13112 owns bounded evidence-action selection.
- Competent laboratories, engineering authorities and aviation authorities own physical procedure approval, acceptance and certification decisions.

Do not build a second NDE database, artifact ledger, evidence graph, invalidation engine, qualification state machine, aircraft optimizer or certification authority.

## 3. DFI design contract

### 3.1 Defect and inspection context

Each inspectability claim must identify:
- physical component/detail and exact product configuration;
- material system, lot, process batch, manufacturing route, geometry and joint/layup context;
- defect family and physically meaningful descriptors (type, size, orientation, depth, location, morphology);
- inspection method, procedure revision, equipment/instrument, access and surface/curvature constraints;
- operator/automation mode and acquisition settings where they affect the method;
- calibration evidence, interval and measurement acquisition time;
- coverage, detection threshold, false-call behaviour, sizing error and uncertainty;
- repairability, allowable repair, inspection interval and maintenance access where relevant.

A single DFI score must not hide defect families or inspection conditions. A design may be inspectable for one flaw class and poorly inspectable for another.

### 3.2 Design variables to co-optimize

Where the physics and requirements support it, expose:
- access envelope, probe/tool approach and removal/install clearance;
- skin/stringer/joint geometry, local thickness, curvature and layup;
- critical-region visibility, scan paths and coverage;
- sensor number/location/orientation, attachment and wiring/communication route;
- sensor redundancy, observability and maintainability;
- repair access, repair boundary and replacement strategy;
- manufacturing tolerances and process variability that change detectability.

Keep structural mass, strength, stiffness, buckling, fatigue/damage tolerance, aeroelastic response, manufacturability, energy/thermal effects and lifecycle maintenance as separate metrics/constraints. Hard structural or detection requirements are not tradable against a lighter design.

### 3.3 DFI simulation versus physical detection

Simulation-derived detectability and physical probability-of-detection evidence are different evidence classes. A simulation can identify difficult viewing angles, inaccessible surfaces or sensitivity to geometry/defect variation; it cannot on its own establish physical POD, procedure qualification or in-service detection performance.

Every DFI model must report:
- exact geometry/material/defect/inspection domain;
- solver/simulation version and input configuration;
- mesh/resolution and numerical convergence evidence;
- boundary/visibility/acoustic/thermal assumptions as applicable;
- uncertainty and model-form limitations;
- the physical tests still required to validate the prediction.

Out-of-domain geometry, defect morphology or inspection configuration must produce NOT_COVERED / UNKNOWN, not extrapolated confidence.

## 4. POD and NDE experiment contract

### 4.1 Conditional probability of detection

Treat POD as conditional on a declared examination configuration:

P(detect | flaw family, size, orientation, depth, location, geometry, material/process, procedure, equipment, calibration, operator, environment)

Do not report a universal POD number independent of its defect population and examination parameters.

For hit/miss data, ASTM E2862-23 provides a statistical analysis practice for hit/miss POD for a specific set of examination parameters. Select the method only when its assumptions and data type fit. Use the applicable governing standard, program requirement and authority-approved procedure; do not treat E2862 as a universal standard for every NDE modality or certification basis.

### 4.2 Experimental design

- Preregister the flaw families, sizes, orientations, depths, locations and blind truth labels.
- Include representative manufacturing/process defects and service damage only where their definitions and physical introduction are controlled.
- Split data by independent physical article, panel, lot, process batch, procedure, instrument and operator as applicable. Randomly splitting repeated indications from one panel is leakage.
- Preserve misses, false calls, ambiguous indications, censored measurements, excluded samples and invalid trials with reasons.
- Use blinded inspectors and independent flaw truth establishment where feasible.
- Select sample counts through the applicable method, precision/power and confidence requirements; do not invent a universal minimum N.
- Report POD curves/intervals only for the method and data type that support them; state the flaw size parameter, confidence method, excluded data and experimental-unit hierarchy.

### 4.3 Required metrics

Report separately:
- hit/miss counts and denominators by defect class;
- POD estimate and confidence bounds over the supported flaw-size range;
- false-call rate and sizing error;
- detection threshold and threshold uncertainty;
- coverage / inaccessible-surface fraction;
- operator/equipment/procedure variation;
- calibration failures and out-of-window observations;
- independent article/lot/batch/operator counts;
- inspection time, repeat scans, setup and access burden;
- repairability and maintenance downtime implications;
- explicit abstention and NOT_COVERED cases.

An a90/95 statement may be reported only when the applicable POD method and data justify it. It is not a universal acceptance threshold to apply to every aircraft, material or NDE method.

## 5. SHM sensing integrity benchmark

### 5.1 Competing hypotheses

For every changed signal, the evaluator must preserve and distinguish:
1. structural damage or degradation;
2. sensor / acquisition-chain fault;
3. environment or operating-state change;
4. unknown / out-of-domain condition.

Examples of sensor faults include bias, drift, gain change, saturation, dropout, stuck-at output, timing error, synchronization loss, cross-talk and intermittent connection. Challenge them both independently and in combination with real/injected structural damage.

### 5.2 Immutable observation lineage

- Raw sample bytes and raw acquisition metadata are immutable.
- Calibration, sensor identity, timebase, clock quality and acquisition configuration are pinned.
- A corrected/reconstructed signal is a new derived artifact with a parent reference, algorithm/model version, parameters, uncertainty and reason.
- Sensor reliability is a separate claim from the reconstructed signal.
- If the system cannot separate damage from sensor fault or environmental shift, it must abstain and request independent inspection/NDE.
- A correction must never overwrite raw data, clear a damage indication, alter the original detection label or promote a claim without an independent verification step.

### 5.3 SHM metrics

Report:
- structural-damage sensitivity/recall by damage family and severity;
- false alarms per operating hour or mission, with exposure denominator;
- sensor-fault detection/diagnosis precision and recall;
- damage-versus-sensor-fault confusion matrix;
- missed-damage rate when a sensor fault is simultaneous;
- probability calibration / reliability of sensor-health estimates;
- signal reconstruction error and uncertainty versus immutable raw signal;
- OOD/abstention and unresolved-cause rate;
- robustness to environment/temperature/load/aging and acquisition-clock drift;
- latency, power, compute, memory, sensor mass and maintenance burden.

Accuracy alone is insufficient. Report calibration and false-negative risk separately from reconstruction quality and sensor-health classification.

## 6. Frozen benchmark arms

For the same design and evidence corpus, compare:

1. Inspection-naive structural design baseline.
2. Conventional design-for-manufacture / maintenance review without DFI metric.
3. NASA-style simulation-based inspectability metric where its published scope and input assumptions match.
4. Fixed/manual sensor-layout baseline.
5. Physics-only SHM detector with no sensing-layer diagnosis.
6. Damage + sensor-fault competing-hypothesis model.
7. Full co-design and evidence layer with DFI, POD evidence, sensor reliability and owner-native provenance/freshness.

For test/inspection selection, compare:
- random / conventional inspection plan;
- space-filling design;
- highest model uncertainty;
- sensitivity-only;
- prediction disagreement;
- calibrated EVSI / expected decision-loss reduction;
- distribution-free minimax regret;
- finite-horizon policy from the existing evidence-action selector;
- qualified human engineering/NDE plan.

All arms use the same safety obligations, defect truth, data budget, inspection resources, hardware configuration and scoring code.

## 7. Prospective gates

These are research targets, not achieved results, industry standards or certification criteria:

- zero false acceptance of known wrong-unit, wrong-configuration, stale-calibration and missing-POD-domain controls on the bounded challenge corpus;
- 100% preservation of raw observation identity and provenance through the pipeline;
- zero silent deletion of missed, censored, excluded, false-call or invalid inspection records;
- no positive POD/SHM promotion outside the validated defect/material/geometry/procedure/environment domain;
- 100% deterministic semantic reports for deterministic inputs;
- explicit damage / sensor fault / environment / unknown disposition or abstention;
- any verification-effort reduction demonstrated only at matched defect coverage, detection confidence, false-negative constraints and hard engineering requirements.

Physical claims require blind, rights-cleared, independently reviewed evidence from representative articles and qualified procedures. Synthetic simulation and injected sensor faults can validate software/control logic, not physical POD or aircraft SHM acceptance.

## 8. Reference comparators

- NASA Langley Design for Inspection: NTRS 20240003953 proposes a simulation-based, automation-friendly inspectability metric evaluating defect findability and sensitivity to variability.
- NASA Langley / ASNT Inspectability Metric: NTRS 20240007593 describes integration of inspectability into multi-objective design optimization alongside mass/manufacturability.
- ASTM E2862-23: hit/miss POD analysis for a specific examination parameter set; not a universal NDE modality standard.
- MIL-HDBK-1823A: active DoD handbook for NDE system reliability assessment; DLA identifies it as guidance only, not a standalone requirement.
- Di Fiore et al. (8 June 2026), “Real-time sensing digital twin for aerospace structural health monitoring,” Structural and Multidisciplinary Optimization 69, article 159. It separates structural and sensing degradation and reconstructs sensor-fault-affected signals; its demonstration is a simulated composite wing panel and must not be represented as a physical flight/aircraft qualification.

- Yan et al. (2026), “Physics-informed digital twin for spacecraft structures under sensor faults: Virtual sensing via a frequency attention U-Net and closed-loop model updating,” *Aerospace Science and Technology* 177, 112794, DOI: 10.1016/j.ast.2026.112794. This is a stronger adjacent experimental comparator: the authors report tests on a scaled reconfigurable spacecraft across multiple geometric configurations and 10 Hz / 800 Hz excitation, combining reduced-order FEM priors, frequency-domain neural reconstruction and constrained model updating. Their paper reports a 76.7 dB average reduction in a specific log-amplitude main-peak error metric at 800 Hz and 8–10 dB masked-channel errors versus up to 48 dB for spatial interpolation under its stated protocol. These are study-specific results, not directly transferable AERO targets, not aircraft qualification, and not evidence of general superiority. Replicate the exact metric/protocol before comparison.

### 8.1 Replication and extension benchmark

The SHM tournament should include two distinct benchmark tiers instead of collapsing different evidence types:

1. **Numerical replication tier:** reproduce the Di Fiore composite-panel study's published geometry/material/model assumptions, damage parameterization, virtual sensor layout, six sensor-fault families (bias, drift, gain, saturation, dropout and stuck), sampling protocol and reported task metrics where the authors' artifacts permit. Preserve the fact that its structural damage is modelled by local element-stiffness weakening and its sensor data are synthetic; this is a controlled software/model baseline, not physical defect validation.
2. **Experimental replication tier:** reproduce the Yan et al. scaled-spacecraft virtual-sensing/model-updating protocol on its stated configurations and 10 Hz / 800 Hz conditions, with the exact signal-processing and peak-search definitions. Compare against its spatial-interpolation baseline and disclosed ROM/FEM baselines. Report absolute and normalized errors, not only the paper's headline dB difference.

**AERO extension challenges** should add:
- held-out physical geometry/configuration and excitation conditions, rather than random time-window splits;
- simultaneous structural changes plus sensor faults, with independent and common-cause sensor-fault groups;
- correlated thermal/load/environment shifts, clock offset/drift and synchronization faults;
- sensor relocation, partial channel loss and sensor-layout perturbation;
- joint/contact nonlinearity and model-form discrepancy, with scope explicitly declared;
- blind physical defect/NDE holdouts whose damage mechanism and detectability domain are known independently;
- an unknown/OOD arm where a model must abstain instead of reconstructing a plausible but unsupported healthy response.

**Required ablations and baselines:** spatial interpolation; ROM-only virtual sensing; data-driven reconstruction without physics prior; physics-informed reconstruction; sensor-health classifier without reconstruction; full coupled reconstruction + sensor-health + model update; and human analyst adjudication. Keep each study's native baselines separate where the testbeds are not interchangeable.

**Score separately:** complex frequency-response error (magnitude and phase); peak amplitude and frequency error; modal-parameter and model-update residuals; damage-detection sensitivity and false-alarm exposure; sensor-fault detection/diagnosis; simultaneous-fault missed-damage rate; uncertainty calibration; OOD/abstention; latency and compute; and raw-to-derived evidence lineage. A lower reconstruction error must never be allowed to compensate for a missed damage indication or false structural-health clearance.

The cross-study stretch goal is not to beat either published headline number on a different testbed. It is to demonstrate a better risk–resource Pareto frontier on a preregistered, matched testbed: no increase in missed-damage risk or unsupported healthy clearances, lower reconstruction/model-update error, calibrated uncertainty, and lower compute/inspection burden.

## 8.2 Public physical-data comparators: add a separate replication tier

The two 2026 comparators above should remain separate: Di Fiore et al. is a simulated composite-panel benchmark, while Yan et al. is an experimental scaled-spacecraft virtual-sensing/model-updating study. Do not pool their metrics or describe either as an aircraft qualification result.

A stronger physical-data programme can use three additional public sources. They are different structures and materials, so each must retain its own domain and test protocol.

### A. NASA PCoE CFRP composites dataset — fatigue and internal damage

NASA's Prognostics Center of Excellence repository provides run-to-failure experiments on CFRP panels under tension–tension fatigue. The published description identifies three layups, a 16-channel PZT sensor network, triaxial strain gauges and periodic X-ray images used to characterize internal damage.

Use this as a composite damage-progression baseline:
- preserve whole-panel / layup / fatigue-run grouping;
- use X-ray characterization only according to its documented timing and ground-truth scope;
- distinguish damage progression from sensor-path or acquisition degradation;
- report temporal prediction and detection tasks separately;
- keep any sensor-fault injection clearly labeled as synthetic unless independently present in the released records.

Source: NASA Prognostics Center of Excellence Data Set Repository, “Carbon Fiber-Reinforced Polymer (CFRP) Composites.” The dataset is linked from the NASA repository.

### B. Full-scale composite outer-wing demonstrator — physical aerospace geometry

Marzani et al. describe an open guided-wave database from a composite panel closing the bottom part of a full-scale outer-wing demonstrator approximately 4.5 m long and 1.2–2.3 m wide. The SHM system used 160 secondarily bonded PZT transducers. Measurements included electromechanical impedance checks at each transducer to assess reliability/bond strength, alongside active guided-wave screening. Measurements were made before loading, before fatigue, before impacts and after impacts.

This is the highest aerospace-structural relevance of the three public sources, but it is still a demonstrator/test article rather than a certified in-service aircraft fleet. Its paired impedance and guided-wave measurements make it especially useful for the benchmark's sensor-health versus damage distinction.

Use it to test:
- physical sensor-network identity and channel mapping;
- sensor bonding/reliability indicators versus structural damage indications;
- pre/post loading, fatigue and impact comparisons;
- localization and sizing only where the published truth and test configuration support them;
- whether model transfer survives the demonstrator's geometry and sensor layout.

Source: Marzani et al., “An open database for benchmarking guided waves structural health monitoring algorithms on a composite full-scale outer wing demonstrator,” *Structural Health Monitoring* (first published 28 November 2019), DOI: 10.1177/1475921719889029. The article links the database at shm.ing.unibo.it.

### C. University of Utah / SmartDATA-Lab long-term guided waves — environmental and sensor drift

Yang et al. (2025) publish a long-term outdoor guided-wave dataset from an aluminum plate monitored over more than 4.5 years, with approximately 6.4 million wave measurements and 13 introduced damage types. The records include environmental variables and document sensor drift and installation shifts. The released benchmark is approximately 100 GB, compared with roughly 9 TB for the original acquisition. The source paper describes sampling approximately one measurement from every 20 original measurements (about every 176 seconds in the released data rather than every 8.6 seconds) and retaining 2,000 of the original 10,000 samples per wave. These are material sampling/preprocessing changes, not incidental file-format details. Any benchmark card and reported result must pin the released data version and preserve the original-versus-published distinction.

This dataset is not an aerospace composite benchmark and must not be presented as one. It is valuable as an environmental and long-duration monitoring stress test:
- hold out entire time periods, not random adjacent windows;
- test temperature, humidity, weather, sensor drift and installation-shift robustness;
- assess false alarms per operating exposure and detection delay;
- distinguish baseline adaptation from damage absorption;
- test whether a model can abstain when the environmental domain changes;
- never let an adaptation routine normalize away a new damage signature.

Source: Yang et al., “Dataset on guided waves from long-term structural health monitoring under uncontrolled and dynamic conditions,” *Scientific Data* 12, 991 (12 June 2025), DOI: 10.1038/s41597-025-05300-5. Data repository DOI: 10.6084/m9.figshare.28112504; loading code: github.com/SmartDATA-Lab/Long_Term_Guided_Waves.

### Dataset eligibility and reproducibility gate

Before using any source in a public leaderboard or external claim, record:
- exact artifact URL, dataset version, retrieval date, license/rights and redistribution conditions (the public listing is not itself a blanket redistribution license);
- immutable downloaded file hashes, file sizes and retrieval date;
- data dictionary, channel map, units, calibration/timebase information and known missingness;
- article/structure identity, material, layup, geometry and test campaign;
- damage truth source, introduction time, location, type, severity and uncertainty;
- sensor placement, attachment/bond evidence and any known sensor faults;
- original versus downsampled/trimmed signals and all preprocessing;
- fixed train/development/calibration/confirmatory partitions grouped by panel, campaign and time as appropriate;
- benchmark code, dependency versions, random seeds, model checkpoints and exact evaluation manifest.

Do not use a random row/window split where adjacent measurements share the same structure, damage state, weather episode or acquisition campaign. A temporal split is necessary for drift claims; whole-panel/test-campaign holdouts are necessary for structural generalization claims.

### Recommended benchmark sequence

| Tier | Dataset | Primary claim tested | Explicit limitation |
|---|---|---|---|
| S0 | Di Fiore simulated composite panel | Reproducible synthetic damage/sensor-fault logic | Simulated structure and virtual sensors |
| S1 | Yan scaled reconfigurable spacecraft | Physics-informed virtual sensing under masked channels | Spacecraft test article; native metric/protocol only |
| P1 | NASA PCoE CFRP composites | Physical composite fatigue/damage progression | Panel/layup/run-specific; X-ray truth scope |
| P2 | Full-scale outer-wing demonstrator | Aerospace-scale composite guided-wave and sensor-health behavior | Demonstrator, not fleet/service qualification |
| P3 | Utah long-term guided waves | Environmental drift, installation drift and long-duration false-alarm behavior | Aluminum plate, non-aerospace domain |

The sequence is a research prioritization, not a universal ranking of dataset quality. Dataset-specific results must remain separate. A model that performs well on the Utah outdoor plate has not thereby demonstrated composite-wing generalization; a model that reconstructs masked spacecraft channels has not thereby demonstrated reliable NDE probability of detection.

AERO should publish a benchmark card for every dataset with scope, rights, article identity, channels, time coverage, damage truth, independent grouping, preprocessing and claim ceiling. Cross-dataset transfer is a separate experiment with a held-out target domain and explicit abstention, not a pooled score.

## 9. Non-claims

This benchmark does not establish physical POD, structural health monitoring acceptance, fatigue/damage tolerance, maintenance interval, repair approval, structural substantiation, part acceptance, airworthiness, aircraft certification or authority to operate. It is an experiment and software-benchmark contract only.


## 8.3 Rights-aware dataset manifest and secure ingestion

A machine-readable discovery manifest is maintained at:

- `docs/research/aviation/examples/aero_shm_dataset_manifest_v0_1.json`
- `docs/research/aviation/schemas/aero_shm_dataset_manifest_v0_1.schema.json`

The manifest records the three candidate datasets, source and download URLs, verified published metadata, domain limits, security concerns, rights status and acquisition status. At this revision **all three datasets remain `UNVERIFIED` for reuse rights and `NOT_DOWNLOADED`**. Public access, a free download, a repository listing or the presence of a DOI is not a blanket redistribution license.

NASA's general science-data policy distinguishes NASA-led mission data from other provider data and advises users to verify source rights for the latter. The NASA PCoE CFRP listing identifies Stanford Structures and Composites Laboratory as the experimental partner; do not infer CC0 solely because the record is hosted in a NASA repository. See https://science.data.nasa.gov/about/license.

The outer-wing paper describes its database as freely available at the University of Bologna SHM site, but the dataset-specific reuse and redistribution conditions still need to be recorded from the current host or rights holder. The Utah record must likewise be checked against the item-specific Figshare license and version metadata; do not assume Figshare's platform-level defaults determine the license of this particular item.

### Unsafe serialization boundary

The long-term Utah dataset is distributed as Python `.pickle` files. Python's official documentation warns that unpickling malicious data can execute arbitrary code and says not to unpickle data from untrusted or tampered sources: https://docs.python.org/3/library/pickle.html.

Therefore:
- do not run the provided loader or unpickle the archive on a developer workstation, shared runner or production system;
- first record the exact source version, download URL, file size and SHA-256 after acquisition;
- inspect archive paths, file types, compressed/uncompressed sizes and metadata without deserializing;
- only after source integrity and rights review, use a disposable, network-isolated conversion environment with no credentials, no writable host mounts, least privilege, resource/time limits and an explicit allowlist of required object types;
- preserve the original bytes read-only and hash all converted outputs;
- prefer a documented, safe, chunkable interchange representation for analysis, with conversion code, environment, input digest and output digests recorded;
- never treat successful deserialization or conversion as evidence that labels, measurements or damage truth are physically valid.

A restricted unpickler may reduce the allowed object surface, but the benchmark should not assume it eliminates all parser, dependency or resource-exhaustion risk. Any exception to the sandbox boundary requires explicit security review.

### Manifest promotion states

`DISCOVERY_ONLY` means the source is identified but not cleared or ingested. A dataset may advance to `RIGHTS_REVIEWED` only with item-specific permission/license evidence; to `INGESTED_AND_HASHED` only after immutable source hashes, file inventory and data dictionary are recorded; and to `BENCHMARK_READY` only after independent group partitions, preprocessing, labels, truth limitations and evaluation manifests are reviewed.

No benchmark leaderboard or external performance claim may consume a dataset whose rights state is unresolved or whose provenance/hash inventory is incomplete.
