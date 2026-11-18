# Lunar granular-flow and material-transfer benchmark — v0

This benchmark qualifies material flow and transfer after excavation: rotating drums, hoppers, chutes, conveyors, collection, screening and beneficiation. It is deliberately separate from the excavation-force model benchmark in [#6012](https://github.com/Luminous-Dynamics/symtropy/issues/6012). A model can predict cutting force accurately and still fail to discharge or deliver useful feedstock.

**Functional output:** accepted target-specification feedstock delivered to a named downstream process, with measured loss and energy boundaries. Not nominal bucket payload, cut volume, or machine-reported mass.

## 2026 evidence that changes the engineering approach

### 1. The wall is an active part of the granular system

Wylęgała and Uhl, *Influence of wall material and vacuum conditions on lunar regolith flow stability in a rotating drum*, Acta Astronautica 249 (2026), DOI [10.1016/j.actaastro.2026.07.006](https://doi.org/10.1016/j.actaastro.2026.07.006).

The study tested smooth PETG, roughened PETG, PTFE, aluminium and copper at 2, 6 and 12 rpm in ambient air and high vacuum (about 6×10^-4 torr, approximately 0.08 Pa). It reports:
- vacuum increased mean dynamic surface angles by 8–22% for the PETG surfaces;
- low-speed angle stability improved by 26–47% under vacuum across most tested materials;
- wall material explained more than 80% of the variance in the reported geometry and angle metrics;
- copper showed a distinct response, which the authors relate to rapid charge redistribution.

The raw research dataset is listed openly at [Zenodo DOI 10.5281/zenodo.18749190](https://doi.org/10.5281/zenodo.18749190).

**Interpretation:** the hopper liner, drum wall, chute surface, conductivity and grounding are not passive hardware. They may materially alter the force network and flow state. The reported effect sizes are specific to that simulant, apparatus and test design; they are not universal material rankings.

### 2. Returned Chang'e-6 material exhibits cohesive agglomerate flow

Bao et al., *Sustained Agglomerate Flow and Low-Gravity Prediction of Chang'e-6 Lunar Regolith*, Acta Astronautica (online 3 November 2026), DOI [10.1016/j.actaastro.2026.10.071](https://doi.org/10.1016/j.actaastro.2026.10.071).

The CE60100 returned sample mass available for the rotating-drum study was 1.85 g; its original sealed bottle was used as the rotating chamber to limit sample disturbance and loss. The reported slumping regime has periodic avalanches and persistent cohesive clusters. Dynamic angle of repose decreases from 50.4° ± 3.1° at Fr = 3.3×10^-4 to 41.1° ± 1.8° at Fr = 3.3×10^-3. A force-balance model combines gravity, rotational inertia and cohesion and reproduces the Fr trend without fitting the reported CE-6 DAoR values.

This is a source-specific **micro-sample rotating-drum** benchmark. The 1.85-g sample is not bulk test material for hopper or conveyor campaigns, and the result is not a universal regolith discharge law.

### 3. Excavation is not delivery

ESA's MoLES3 integrated analogue campaign, *Integrated robotic feedstock preparation for lunar oxygen production*, DOI [10.1038/s44453-026-00051-y](https://doi.org/10.1038/s44453-026-00051-y), decomposes a physically separated rover excavation / stationary vibratory beneficiation architecture. In the tested configuration, transfer loss was the dominant recoverable feedstock inefficiency, while beneficiation consumed a smaller fraction of total system energy. Its 2.5-hour window, 60-kg mass limit and 300-Wh energy target are challenge-specific, not universal machine specifications.

MoLES3 defines a useful decomposition:
- transfer efficiency, η_trans;
- separation / beneficiation yield, η_sep;
- composite feedstock preparation efficiency, η_feed = η_trans × η_sep.

Retain each component separately. A product of two individually high efficiencies can still expose a costly weakest link.

### 4. Do not collapse materials into one "lunar soil" record

Gasteiner, Murdoch and D'Angelo's 2026 open lunar-regolith database includes more than 6,000 records and preserves material/source distinctions across historic in-situ measurements, inferred values and returned-sample/laboratory characterizations. See [DOI 10.1002/nag.70432](https://doi.org/10.1002/nag.70432) and the [open data release](https://doi.org/10.57745/NTSZ8G).

Every run must identify material batch, PSD, preparation/compaction history, morphology and evidence class. A simulant's name alone is insufficient.

## Paradigm shifts

1. **Co-design material and contact surfaces.** Optimize wall material, roughness, conductivity, grounding, coatings and wear state with the granular material. Do not optimize only hopper shape or bulk material parameters.
2. **Use a multidimensional similarity envelope.** Froude similarity alone is insufficient where cohesion and wall adhesion compete with gravity. Preserve at least:
   - rotational Froude number: Fr = ω²R/g;
   - particle cohesion-to-weight ratio: Cw = F_cohesive/(m_particle g);
   - wall-adhesion-to-weight ratio: Aw = F_wall_adhesion/(m_particle g);
   - particle-to-drum ratio: d50/R;
   - wall roughness ratio: Ra/d50;
   - fill fraction, pressure, temperature, grounding and electrostatic state.
   
   Cw and Aw are proposed measurable similarity coordinates, not claims that a single closed-form constitutive law exists. Measure or independently bound them; do not infer them from angle of repose alone. Matching Fr while mismatching Cw/Aw does not establish lunar-gravity similitude.
3. **Benchmark the complete material path.** Excavated mass → collected mass → transfer input/output → processed size fractions → accepted target feedstock → stored/delivered feedstock. Every interface has a separate mass and energy boundary.
4. **Treat flow as a dynamic state, not a static material property.** Avalanches, agglomerate persistence, intermittent discharge, arching, restart after dwell, segregation and wear matter alongside static/dynamic angle of repose.
5. **Make uncertainty and abstention first-class.** A model should identify an unseen batch, wall treatment, pressure or flow regime as out-of-envelope and abstain instead of confidently extrapolating.
6. **Use active experiment selection.** Symthaea can choose the next most informative wall/pressure/Fr/fill/batch experiment; the choice is preregistered and cannot be scored as its own independent validation.

## Industry and current capability comparison

The right comparison is by **mission boundary and accepted output**, not by raw rpm, kilograms excavated, or a single angle-of-repose number.

- **NASA IPEx:** NASA's current Lunar Surface Technology page says IPEx is designed to deliver 10 metric tonnes over 100 m and 11 days in its quick facts, while its detailed project description says "on the order of 20 metric tons" over a single mission. Preserve these distinct descriptions and their boundary/status; neither is a measured lunar operating record. The same NASA page describes surface excavation/site preparation, logistics and processing capabilities, not a flight-qualified subsurface tunnel-boring system.
- **ESA MoLES3:** the 2026 LUNA analogue uses a 2.5-hour field window, 60-kg system mass limit and 300-Wh target. It provides an integrated transfer/beneficiation comparator and decomposes transfer and separation efficiencies, but remains an Earth-gravity analogue with its own challenge constraints.
- **Terrestrial TBM:** USACE EM 1110-2-2901 reports a historical project database average utilization of 38% (range 5–69%) and average advance of 375 m/month (range 5–2,084) for its cited tunnel-project population. This is an old rock-tunnelling dataset, not a lunar regolith-flow target. Its enduring lesson is that penetration while cutting, utilization, calendar advance, mucking and support are different metrics; compare the complete time boundary and downtime taxonomy.
- **Granular-flow SOTA:** the Wylęgała–Uhl wall/vacuum study and Bao et al. CE60100 study are physical references for specific simulant / returned-sample regimes. They do not qualify full-scale hopper discharge or long-duration mining. The benchmark should aim to reproduce those source conditions, then generalize across batches, wall interfaces, environments and transfer geometries.

The competitive claim we should pursue is an independently replayable, provenance-preserving **accepted-feedstock service**: accepted target-size kg delivered per mission-hour and per kWh, with mass closure, uncertainty, jam/recovery, wear and contamination visible. We should not claim that this already exceeds NASA, ESA or terrestrial industry.

## Benchmark protocol

### Track A — reproduce the published wall/vacuum experiment

The first matrix has 30 factor combinations: five source wall materials × two pressure classes × three source rotation speeds. This is a **factor-level reproduction design**, not a claim that a reproduction has occurred.

Before executing it:
- recover exact source sample/batch, drum geometry, fill ratio, wall finish, speed-control and imaging/segmentation definitions from the publication and Zenodo raw data;
- record all differences from the source protocol;
- preserve raw video/images and the complete processed time series;
- reproduce source statistical summaries on the matched protocol before tuning any model.

The 30 cells are listed in `design-of-experiments.json`.

### Track B — extend by Froude number and fill fraction

The extension has 90 proposed cells: five wall materials × two pressure classes × three target Fr levels (3.3×10^-4, 1.0×10^-3, 3.3×10^-3) × three fill fractions (0.15, 0.25, 0.35). The 36-cell batch-transfer sentinel matrix is a separate experiment and must not be pooled into the source-reproduction score.

The fill levels and middle Fr are analyst-selected experimental levels, not values claimed from the source. Derive actual angular speed at execution from measured drum radius and gravity. Do not silently assume that the source apparatus has the same radius.

Use a screening stage followed by preregistered confirmatory repeats. Establish reset/reconditioning and sample-independence rules before calling repeated runs independent. Returned CE60100 data remain in a separate source-matched micro-sample track.

### Track C — batch-transfer sentinel experiment

A third, smaller matrix contains 36 planned cells: three independently characterized material batches (calibration, validation and blind-test roles) × three representative wall materials (smooth PETG, PTFE, copper) × two pressure classes × two Fr levels (3.3×10^-4 and 3.3×10^-3), at a fixed analyst-selected fill fraction of 0.25.

This is the explicit test of whether a model trained on one lot transfers to another. The batch roles must be assigned to real independently characterized lots at execution; the names in the design file are role labels, not material records. Keep the blind lot and raw outcomes under an independent custodian until the executable, source revision, configuration, parameters, seed and predictions are frozen. Never distribute repeats from one physical lot across calibration and blind partitions.

### Track C — transfer and discharge

For hoppers, chutes and conveyors, vary outlet geometry, wall angle/material/roughness, fill level, conditioning/dwell time, vibration, speed and material batch. Measure:
- mass flow rate and variability;
- bridging/jam incidence, duration and autonomous recovery;
- start/stop and restart after dwell;
- retained/residual mass and mass-balance residual;
- size segregation, fines/dust generation and contamination;
- energy per accepted delivered mass.

### Track D — beneficiation and accepted feedstock

Measure input, target-size product, oversize, undersize, retained material, dust/fines, contamination and unknown streams. Keep transfer efficiency and beneficiation yield separate; report their product only as a clearly named composite.

## Model tournament

Compare on identical, preregistered splits:
- analytical / force-balance granular-flow baselines;
- DEM with cohesion and wall-contact models;
- continuum models where their assumptions apply;
- physics-informed residual models;
- ML surrogates with out-of-distribution detection and abstention.

Separate:
- interpolation within represented batches, walls, geometries and environments;
- leave-one-material-batch-out transfer;
- leave-one-wall-surface/material-out transfer;
- leave-one-apparatus/laboratory-out transfer;
- pressure/temperature/grounding and Fr/fill regime transfer.

Report model-form disagreement and non-convergence; do not average disagreeing solvers into an untraceable consensus. Keep excavation-force scores in #6012 separate from granular-flow scores here.

## Required scorecard

| Category | Metrics |
|---|---|
| Flow state | static and dynamic AoR (method-labelled), angle stability CV, flow-regime classification |
| Dynamics | avalanche frequency/size, agglomerate fraction/lifetime, discharge-rate time series |
| Transfer | η_trans, retained mass, jam probability/duration, mass closure at each interface |
| Product | PSD shift, segregation, fines/dust, contamination, η_sep, accepted target-size mass |
| Resources | kWh per accepted kg, delivered kg per operating hour, sample mass consumed, calibration/operator hours |
| Model quality | point error by group, worst-group error, 50/80/90/95% interval coverage and sharpness, OOD detection/abstention, runtime |
| Reliability | restart recovery, wear state, sensor dropout response, repeatability and instrument drift |

Do not reduce this to one leaderboard scalar. Publish Pareto fronts for accepted feedstock, energy, reliability, time, wear and sample consumption.

## Proposed improvement targets (hypotheses, not achieved results)

1. Reproduce Wylęgała–Uhl's source-condition factor effects within source-data uncertainty before claiming replication.
2. Reproduce Bao et al.'s CE60100 Fr-dependent slumping trend on source-matched micro-sample conditions without fitting the withheld DAoR values.
3. Beat the strongest predeclared analytical/DEM baseline on group-disjoint discharge-rate and retained-mass predictions by at least 20%, while retaining calibrated 90–95% intervals.
4. Use at least 25% fewer calibration runs or sample mass than the strongest matched baseline at equal blind-set error and interval coverage.
5. Close measured mass balance at every transfer boundary within combined instrument uncertainty; otherwise return `indeterminate / reconciliation required`.
6. Improve accepted target-size kg per kWh and per mission-hour without increasing jams, dust loss, contamination, or wear.
7. Demonstrate OOD abstention on unseen material/wall/environment states rather than unsupported extrapolation.

These are falsifiable research targets. Absolute tolerances require a standardized analogue rig, a frozen test profile and declared instrument uncertainty.

## Files and checks

- `granular-flow-run.schema.json`: run-level Draft 2020-12 record schema.
- `dataset-manifest.schema.json`: group-disjoint calibration / validation / blind-test manifest.
- `design-of-experiments.json`: source-reproduction and proposed extension matrices; planned tests only.
- `examples/`: synthetic fixtures; no physical data.
- `scripts/validate-lunar-granular-flow-dataset.sh`: reusable semantic/provenance admission checks.
- `scripts/check-lunar-granular-flow-fixture.sh`: Linux CI checks and negative controls.

Schema conformance alone does not validate a physical experiment. The reusable validator is not a general JSON Schema engine. Blind-set custody requires an independent custodian and access control outside the run parser.

## Authority and integration boundaries

- Excavation-force model comparison: [#6012](https://github.com/Luminous-Dynamics/symtropy/issues/6012).
- Planned-to-as-built and disposition closure: [#6006](https://github.com/Luminous-Dynamics/symtropy/issues/6006).
- Granular flow / handling: [#6041](https://github.com/Luminous-Dynamics/symtropy/issues/6041).
- Subsurface observability: [#5983](https://github.com/Luminous-Dynamics/symtropy/issues/5983).
- Earth analogue campaign: [#5988](https://github.com/Luminous-Dynamics/symtropy/issues/5988).
- Material identity / source-bounded properties: [#6004](https://github.com/Luminous-Dynamics/symtropy/issues/6004).

Symtropy owns immutable input records, dataset manifests, replay and receipts. Specialist granular-flow/DEM solvers retain physical-model authority. Symthaea may propose experiments and identify OOD conditions, but cannot certify its own prediction. Mycelix may preserve provenance and custody but cannot establish physical accuracy.

## Claim ceiling

The checked-in run examples and design matrix are synthetic/planned. No rotating-drum, hopper, vacuum or returned-sample experiment has been executed by this PR. Earth-g simulant results do not automatically qualify lunar gravity, vacuum electrostatics, full-scale hopper flow, excavation throughput or flight hardware.

## New 2026 hopper/feeder benchmark and active-flow control

Stepanova, Dreyer & Garvey, *Vibration-assisted granular flow under simulated lunar conditions: parabolic flight validation of lunar regolith feed system* (9 November 2026), DOI [10.1007/s44461-026-00021-y](https://doi.org/10.1007/s44461-026-00021-y), provides a useful operational baseline:

- Eight parallel hopper–feeder assemblies represented four conical outlet geometries: 11, 17.5, 30 and 45 mm, with 46.5°, 55°, 72° and 90° hopper wall angles, each duplicated.
- Across 60 lunar-gravity parabolas (480 assembly/parabola observations), sustained hopper supply to the rotary pocket feeders occurred in all 330 vibration-on observations with functioning feeders, with measured Γ = 3.4–23.8. Flow arrested within seconds in 80 of 84 vibration-off observations.
- The test used ICN-LHT-1, an 800-g initial fill with a roughly 300-mm material column, no replenishment during flight, 0.16 g, and 27–60 Pa. The hoppers retained about 20% of initial fill after the 30-parabola sequence.
- The vibration system operated at measured 37–53 Hz and 0.16–0.78 mm peak-to-peak displacement. Load cells sampled at 5 Hz, accelerometers at 500 Hz, and video at 30 fps.
- The delivered 0.3–1.0 g/s rate was controlled primarily by pocket-feeder speed. Rate was about 21% lower at 0.16 g than in 1-g ground tests with identical commands.
- Important boundary: this was **hopper supply to a rotary pocket feeder**, not free gravity discharge. The study also did not test vibration levels between Γ = 0 and 3.4, so it only bounds Γ_cr ≤ 3.4 for ICN-LHT-1 and that apparatus; it does not identify the exact threshold.
- Supplementary agglutinate tests observed vibration-induced upward size segregation and fragmentation during repeated handling. Thus anti-jam operation must be scored together with composition drift and particle damage.

The new advanced-doe-v0.json contains:
- 480 source-record slots mirroring the paper's two flights × six high/low/off-vibration and high/low feeder-speed conditions × eight assemblies × five per-flight repeats. These are source-record placeholders, not reproduced measurements; exact condition schedules and numeric settings must be imported from the publication/source data before a matched-reproduction claim.
- 800 proposed threshold-screen cells spanning four outlet geometries, two gravity levels, two pressure levels, nine Γ settings (including values below 3.4), three frequencies for vibration-on tests, and two feeder-speed fractions. Every cell is planned/not run and each derived actuator stroke must be checked against hardware limits.
- 90 proposed annular vacuum shear-cell cells spanning a 3-decade granular Bond-number range, three inertial-number levels and three analyst-selected confining-pressure levels at fixed size ratio. These values are screening choices, not measured lunar thresholds.

### Supervisory control paradigm

The Rust crate at tools/lunar-regolith-feeder-supervisor implements an advisory stateful policy:
1. Use accelerometer-measured Γ rather than commanded motor settings; the paper reports resonance/isolator coupling that made command settings an unreliable proxy for actual displacement.
2. Regulate **hopper continuity** with vibration, subject to per-material/per-apparatus commissioned Γ bounds.
3. Regulate **delivered mass rate** with the rotary feeder when pocket filling is adequate. Do not use vibration as a substitute for feeder metering.
4. Pause the feeder during no-flow/intermittent-flow recovery, and require an observed return to continuous flow before resuming.
5. Stop on stale/failed critical sensors, unobserved material-quality metrics required by the feed specification, excessive segregation, excessive agglutinate fragmentation, or actuator values outside the commissioned envelope.
6. Reduce vibration only after repeated stable windows and only when vibration power is actually measured.

This is a reference supervisory policy and test target, not actuator firmware or safety-certified control. The published Γ ≤ 3.4 result is not a default for another simulant, wall, fill, vacuum, gravity or hopper geometry.

## Segregation is a separate physics and product-quality problem

Nakagawa & Kioka, *Segregation in Granular Flow Under Lunar Conditions*, Fluids 11(11), 279 (4 November 2026), DOI [10.3390/fluids11110279](https://doi.org/10.3390/fluids11110279), argues that granular Bond number Bo_m = F_adh/(m g) and inertial number I = γdot d/sqrt(P/rho) organize the lunar regime better than gravity alone. Its review explicitly states that the internal composition field of dense, sheared, cohesive, polydisperse flow has not yet been resolved under reduced gravity. It proposes a vacuum annular shear-cell experiment varying Bo_m over at least three decades at fixed size ratio, while varying confining pressure independently and measuring composition by tomography or registered sectioning.

That is tracked separately in [#6070](https://github.com/Luminous-Dynamics/symtropy/issues/6070). The run record now has fields for size-fraction/composition profile artifacts, out-of-spec mass and duration, agglomerate fragmentation, and flow continuity. A flow system can be jam-free and still produce unacceptable reactor feed. Preserve the time- and position-dependent composition field; a single bulk average can hide segregation.

## Rust tools

- tools/lunar-granular-flow-similarity: fail-closed similarity-envelope comparison across Fr, cohesion/weight, wall-adhesion/weight, particle/vessel ratio, roughness/particle ratio, fill, gravity, pressure and categorical boundaries. Missing coordinates return indeterminate. This screens claimed similitude; it does not establish physical equivalence.
- tools/lunar-regolith-feeder-supervisor: advisory, stateful hopper/rotary-feeder supervisory policy with sensor-health checks, flow recovery, metering control, quality holds and power-aware vibration trimming.
- scripts/validate-lunar-granular-flow-dataset.sh: dataset/run semantic and provenance admission checks.
- scripts/check-lunar-granular-flow-fixture.sh: synthetic fixture tests, DOE count checks and partition/digest negative controls.

The Linux CI explicitly formats, tests, Clippy-checks and executes both standalone Rust tools. These crates are not implicitly compiled by the root workspace.

## Additional published 1-g flow-regime comparator: LHS-1

Lappa, Watson & Bonnieu, *Vibrationally aided flow of lunar regolith simulants in a hopper-based conveying device*, Journal of Rheology 70, 557–579 (21 April 2026), DOI [10.1122/8.0001055](https://doi.org/10.1122/8.0001055), provides a complementary terrestrial-gravity comparator using LHS-1:

- Identifies four spatial flow configurations: **funnel, mass, asymmetric, and ratholing**.
- Reports vibration made flow more regular and spatially uniform and reduced arching, but average mass rate remained below purely gravitational flow in the tested configurations.
- Low frequencies promoted arching; at higher frequency, clogging could re-emerge at thresholds, particularly with low acceleration amplitude.
- Increasing vibration amplitude generally mitigated blockage, but its effectiveness weakened as sample mass increased.
- The paper identifies the four regimes using the spatial/temporal flow pattern, not simply average mass rate.

This is a 1-g LHS-1 hopper benchmark, distinct from the 0.16-g ICN-LHT-1 hopper-to-rotary-pocket-feeder parabolic-flight study. It should be used as a separate source-reproduction and cross-material comparison, not pooled into the parabolic-flight success ratio. Exact source factor values and test geometry must be extracted from the full experimental tables before calling a replication matched.

The new 63-cell LHS-1 extension in advanced-doe-v0.json is analyst-selected: seven Gamma values × three apparatus-commissioned frequency bands × three hopper charge-mass fractions. It explicitly holds frequency and acceleration as independent controls and includes spatial-flow regime classification, stagnant-zone/rathole area, asymmetry, jam probability, and mass-rate variability. All cells are planned only.

**Why this matters:** a mass-flow average can conceal ratholing or a large stagnant region. Regime classification, occupancy/velocity fields and outlet-rate time series must accompany throughput, energy and composition measurements. The run schema now includes charge mass and hopper geometry fields, an observed regime classification and confidence, and an artifact reference to the spatial flow-regime evidence.
