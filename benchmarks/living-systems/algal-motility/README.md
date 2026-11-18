# Algal movement benchmark — diatom gliding and Chlamydomonas phototaxis

This portfolio contains two different movement systems and four source anchors. It is not a generic algae locomotion model. The comparison envelope is shared; the mechanics and sensory processes are not.

## 1. Benthic diatoms: rigid-cell substrate gliding

### 2026: raphe-contact switching

Golfier et al., PNAS (published online 1 April / issue 7 April 2026), DOI [10.1073/pnas.2506122123](https://doi.org/10.1073/pnas.2506122123), study *Craspedostauros australis*. This raphid pennate diatom moves over submerged surfaces through an intracellular actomyosin/adhesion complex and secreted extracellular-polymer strands at slit-like raphes in a rigid silica cell wall.

The central result is that cell–substrate contact locations move along the curved raphe and switch between one- and two-raphe-branch contact modes. The local raphe geometry predicts trajectory curvature: one-branch contact gliding gives higher curvature; two-branch contact gives lower curvature. The study combines high-precision tracking, SEM, interference-reflection microscopy (IRM) and mathematical modelling.

Zenodo [10.5281/zenodo.17791511](https://doi.org/10.5281/zenodo.17791511), v1, contains single-cell tracks, SEM/raphe images, segmented raphes, calculated raphe curvature, IRM contact tracking and analysis/model notebooks. The retrieved record lists a file bundle but its visible license field is blank. The paper is CC BY 4.0; the dataset remains NOT_ADMITTED until the exact Zenodo record license and file-level metadata are verified.

### 2025: cross-species functional morphology

Bondoc-Naumovitz et al., PNAS (2025), DOI [10.1073/pnas.2426910122](https://doi.org/10.1073/pnas.2426910122), compare five benthic diatom taxa and use 2D and 3D tracking. Species include *Cylindrotheca*, *Halamphora*, *Nitzschia*, *Pleurosigma* and *Seminavis*. The paper reports four stereotyped movement states with species-dependent stochastic switching and links motility diversity to raphe morphology.

Zenodo [10.5281/zenodo.14751417](https://doi.org/10.5281/zenodo.14751417), v1, includes 3D trajectories, annotated tracks, TIFF microscopy, raphe curvature and raphe-point geometry. The record lists per-file MD5s; its visible license field is blank, so the dataset is NOT_ADMITTED pending exact rights verification. The PNAS article's CC BY 4.0 licence does not automatically apply to the separately deposited dataset.

### Model shift

Treat steering as a **contact-location and raphe-geometry selection process**, not as a generic turn command:

- state: one-raphe contact, two-raphe contact, no contact / unresolved;
- geometry: local signed curvature along the raphe and cell-size/shape descriptor;
- observation: active contact location(s) from IRM and raphe map from SEM;
- outcome: signed path curvature, angular velocity and curvature-switch time.

A future reduced model may learn a conditional law from contact state and local raphe geometry to observed trajectory curvature. Do not hard-code a universal proportionality or transfer one species' parameters to another before source-fit and held-out species tests.

## 2. Green algae: *Chlamydomonas reinhardtii* biflagellar swimming

### 2026: single-cell phototaxis under a light gradient

Bosc et al., *Frontiers in Photobiology* (28 May 2026), DOI [10.3389/fphbi.2026.1800988](https://doi.org/10.3389/fphbi.2026.1800988), provide a reproducible single-cell tracking workflow. A representative experiment contains 60 seconds of fluorescence imaging at 10 fps: 10 seconds of baseline swimming, followed by a 470 nm LED that imposes a horizontal light gradient. TrackMate returned 607 trajectories; 497 passed the reconstruction consistency checks. Three independent biological replicates are available.

The key insight is **behavioural heterogeneity**: stationary/local, confined/circling and directional cells coexist; directional cells include positive and negative phototaxis subgroups. Population averaging can cancel opposing responses and conceal organised subgroups. The workflow uses circular statistics and time-windowed MSD. Dataset and analysis-code records are Zenodo [10.5281/zenodo.19354253](https://doi.org/10.5281/zenodo.19354253) and [10.5281/zenodo.19364857](https://doi.org/10.5281/zenodo.19364857). The article is CC BY; verify each Zenodo record's own licence, version, file list and checksums before admission.

### 2024: population response and recovery kinetics

Choudhary et al., Dryad [10.5061/dryad.sj3tx96dg](https://doi.org/10.5061/dryad.sj3tx96dg), provide response/recovery polarization, speed and time-constant series across light intensity and cell density. The source reports that dark-to-light response kinetics depend on both light intensity and cell number density, while light-to-dark recovery kinetics are independent of both. Dryad's published reuse guidance states its datasets are CC0; retain the record, README, source citation and checksums in the admission receipt.

### 2025: competing light sources

Raikwar et al., *Physical Review Letters* 135, 228401 (26 November 2025), DOI [10.1103/3fry-7tsw](https://doi.org/10.1103/3fry-7tsw), examine phototaxis under two light beams. The reported preferred swimming direction follows an intensity-weighted average of the two light-propagation vectors. At large angular separations, subpopulations respond to one beam or the other or follow the vector-average direction, with stochastic switching. This is a mechanistic comparator, not the same experimental data as the 2026 single-cell tracking paper.

### Model shift

Represent phototactic populations as **mixtures of track-level behaviours**, not as a single average heading:

- classify tracks as local/stationary, confined/circling or directional;
- estimate positive/negative phototaxis fractions among directional tracks;
- carry track assignment confidence and rejected/missing tracks;
- fit the dark-to-light response separately from light-to-dark recovery;
- separate density effects on population response kinetics from per-cell speed and flagellar behaviour;
- under competing beams, compare the intensity-weighted-vector rule with single-source and switching-mixture alternatives.

Do not treat track rejection as a non-phototactic label. Report how many tracks fail quality filters and why.

## 3. Shared benchmark protocol

### Matched comparisons

| Lane | Strong baselines | Candidate improvement |
|---|---|---|
| Diatom | Empirical tracks; source 2025 four-state model; source 2026 raphe-contact model; TrackMate/Fiji measurement pipeline | Geometry-conditioned contact-site switching with calibrated uncertainty |
| Chlamydomonas | Empirical track/subgroup workflow; 2024 population response/recovery kinetics; 2025 intensity-weighted two-beam vector rule; 2026 single-cell subgroup workflow | Track-level mixture + adaptation/recovery dynamics + competing-light switching |

Use the same source splits, coordinate/time calibration, observation support and compute budgets. Hold out whole biological replicates; for cross-species diatom generalization, hold out a species or cell-size stratum when sample support allows.

### Metrics

**Diatom gliding:** raphe segmentation/contact localization error; one-/two-raphe contact-state precision/recall and abstention; signed path-curvature distribution; curvature-switch timing; angular velocity; speed, straightness and MSD; size-conditioned predictions.

**Chlamydomonas phototaxis:** trajectory reconstruction error; stationary/confined/directional classification; positive/negative subgroup prevalence; circular resultant length and polar order; response latency and time constant; recovery time constant; speed distribution; time-windowed MSD; competing-beam direction residual and subgroup switching rate.

**Shared:** prediction-interval coverage, worst-replicate performance, source/version completeness, tracking rejection rate, compute/memory, image-analysis latency and human review burden.

Never compare diatom gliding speed and Chlamydomonas swimming speed as a single biological leaderboard. Their propulsion, substrate contact and hydrodynamic conditions differ.

## 4. Source admission and reproducibility

- Pin each species to a provider/dataset/release-scoped taxon concept; preserve the original taxon name and any explicit crosswalk.
- Record exact source record version, per-file names/sizes/checksums, licence, attribution, coordinate and time units, imaging calibration and independent replicate hierarchy.
- Article licence and dataset licence are separate. The 2026/2025 diatom Zenodo record pages and 2026 Chlamydomonas Zenodo records do not expose a licence field in the retrieved metadata; do not infer CC BY from their papers.
- Dryad's published CC0 data policy covers the 2024 Chlamydomonas response/recovery dataset; preserve its README and source citation.
- No external dataset is ingested by this benchmark definition.

## 5. Success criteria (prospective)

1. Reproduce source-defined observations and reported statistics in their original measurement windows.
2. Zero false confidence: ambiguous raphe-contact states and uncertain phototactic track assignments remain unresolved.
3. Improve held-out predictive scores against the strongest admissible source-authored model, not only against a simple point-agent baseline.
4. Calibrate intervals across independent replicates and held-out species/stimuli.
5. Show value from extra mechanism complexity on error–cost Pareto frontiers; more parameters alone do not count as progress.
6. Publish negative results and failed/unsupported strata.

## Claim ceiling

This benchmark covers specific diatom and green-algal movement mechanisms. It does not create a universal algal model, infer ecosystem productivity from cell tracks, establish laboratory cultivation performance, or authorize wet-lab/environmental interventions.