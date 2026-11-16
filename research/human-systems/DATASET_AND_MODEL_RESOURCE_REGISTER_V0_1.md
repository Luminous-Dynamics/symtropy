# Dataset and model-resource register v0.1

**Purpose:** identify publicly accessible resources for bounded human-systems research without controlled-cohort access.  
**Parent:** [HUMAN-SYSTEMS-SIM-001 / #5625](https://github.com/Luminous-Dynamics/symtropy/issues/5625).  
**Status:** discovery register; re-check terms at download and before each reuse.

## Public datasets

| Resource | Evidence content | Access / licence | Good first use | Important limit |
|---|---|---|---|---|
| [MMASH / PhysioNet](https://physionet.org/content/mmash/1.0.0/) | 22 healthy participants; 24-hour beat-to-beat heart data, accelerometry, sleep/activity, psychological characteristics, cortisol and melatonin | Open access, ODbL 1.0 | Daily physiological and activity time-series; exploratory subject-held-out association checks | Small, healthy, observational cohort. Does not validate causal interventions or a full population model. ODbL terms must be followed for derived database products. |
| [Sleep-EDF Expanded / PhysioNet](https://physionet.org/content/sleep-edfx/1.0.0/) | 197 whole-night PSG recordings with EEG, EOG, chin EMG, event markers, expert hypnograms; some files include respiration and body temperature | Open access, ODbL 1.0 | Sleep-stage measurement and selected sleep-model evaluation | Primarily overnight recordings; cannot validate daytime mood, social behavior or whole-body simulation. |
| [OpenNeuro ds003478 — Depression Rest](https://openneuro.org/datasets/ds003478/versions/1.1.0) | 122 participants, one resting EEG session | Public, CC0 | Resting EEG feature extraction/quality pipeline, if this modality is in scope | Cross-sectional resting EEG; not an MDD trajectory, treatment-response or intervention dataset. |
| [WESAD](https://ubi29.informatik.uni-siegen.de/usi/data_wesad.html) | 15 participants in laboratory baseline, stress and amusement conditions; wrist/chest BVP/ECG/EDA/EMG/respiration/temperature/acceleration and self-reports | Public download; provider states scientific **non-commercial** use and attribution | Narrow lab stress/affect task evaluation | Small controlled lab task; not free-living stress. Do not use for commercial development unless rights are separately resolved. |

### Dataset handling rules

1. Download only from the source of record and pin an exact version.
2. Verify the current licence file and provider terms at the time of use. Dataset rights may differ from software rights.
3. Do not commit raw participant-level data, identifiers, timestamps, free text, embeddings or participant-specific model outputs to this public repository.
4. Do not redistribute derivatives unless the licence clearly permits it. ODbL may impose share-alike obligations on derivative databases.
5. Store data in a local controlled directory; add only synthetic fixtures and aggregate, disclosure-reviewed receipts.
6. Split by participant and preserve the unit of analysis. A 24-hour participant record is not 1,440 independent samples.
7. Keep public-data evaluation descriptive/observational unless the source design actually supports causal inference.

## External model and software resources

| Resource | Potential role | Why it is useful | Qualification warning |
|---|---|---|---|
| [OpenSim](https://opensim.stanford.edu/) | Musculoskeletal movement, forces and joint mechanics | Open-source musculoskeletal modeling and simulation; validated models are available | Model choice, geometry, inverse dynamics, contact and task boundary conditions need task-specific validation |
| [BioGears](https://www.biogearsengine.com/) | Whole-body reference physiology | Open-source physiology engine with cardiovascular, respiratory, endocrine, renal, energy, nervous, tissue and drug models | Its own FAQ states the development validation does not include rigorous sensitivity analysis and it is not aware of independent third-party validation. Treat as a reference implementation, not ground truth |
| [Physiome / CellML repository](https://models.cellml.org/) | Selected organ/system mechanisms | Public repository of published model equations with metadata and citations | Model-specific licence, curation, equations, parameter units and validity must be reviewed individually; there is no blanket licence for all contributed models |

## Reuse policy

- External reference models must remain separate adapters with exact source/version and licence metadata.
- Do not port all of BioGears or OpenSim into Symtropy just to claim a native human solver.
- Start with one subsystem and compare it against its source reference before coupling it to Symthaea.
- Any translated model must carry equation-level provenance and numerical comparison tests against the source implementation.
- Never treat a model's self-reported "validation" statement as independent validation.

