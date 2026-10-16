# CyPort Interaction Table Profile v0.1

**Status:** pinned source bytes were inspected in a transient fetch and their Git blob identity was independently recomputed. A source artifact has **not** been persisted in this repository, and no predictive model has been validated.  
**Machine-readable profile:** [CYPORT_INTERACTION_SOURCE_PROFILE_V0_1.json](./CYPORT_INTERACTION_SOURCE_PROFILE_V0_1.json)  
**Executable local profiler:** `--profile-cyport-interactions <path-to-interaction_data.csv>`

## 1. The exact resource

- Source repository: [Maritime-Data-CyPort](https://github.com/ChenchenMobility/Maritime-Data-CyPort)
- Immutable repository revision: `5cb1d8778d3968c5e50d0b8cb8213f2414d692d3`
- Resource path: `Tropical_cyclone_interaction/interaction_data.csv`
- Exact Git blob SHA-1: `21beb5b7673918143a0ab108b03acc3cccaeb330`
- Declared byte length: `643047`
- Independent Git-blob preimage check: **matched** the above GitHub object ID.
- CSV header: 56 columns; records: 1,927.

The source bytes were fetched and profiled for this increment, but the CSV is not bundled or committed. The code is designed so a local copy can be checked against the same pinned Git blob identity and have its SHA-256 emitted in a deterministic profile report. Git object identity verifies the bytes against that repository revision; it does not, by itself, prove scientific validity, upstream rights, or the authenticity of every field.

The repository README declares [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) and requests citation of Kuai et al. (2026), *U.S. port disruptions under tropical cyclones: Resilience analysis by harnessing multiple-source dataset*, *Transportation Research Part D: Transport and Environment*, 157, 105420, [doi:10.1016/j.trd.2026.105420](https://doi.org/10.1016/j.trd.2026.105420). Component-level upstream terms and attribution requirements still need review before further source material or derivatives are redistributed.

## 2. What the pinned table actually contains

| Observed property | Profile result |
|---|---:|
| Columns | 56 |
| Interaction rows | 1,927 |
| Unique interaction IDs | 1,927 |
| Unique cyclone IDs (`SID`) | 90 |
| Unique port IDs (`PID`) | 122 |
| Unique `SID`–`PID` pairs | 1,927 |
| Duplicate interaction IDs | 0 |
| Duplicate cyclone-port pairs | 0 |
| Event-start coverage | 6 May 2015 – 22 October 2023 |
| Event-end coverage | 6 May 2015 – 23 October 2023 |
| `total_impact == 0` | 1,654 rows (85.83%) |
| `total_impact != 0` | 273 rows (14.17%) |
| Recovery-start zero/NA sentinels | 1,650 rows |
| Recovery-end zero/NA sentinels | 1,650 rows |
| `day_of_recover` zero/NA sentinels | 1,650 rows |

The broader CyPort release is described as covering 145 principal U.S. ports and 90 tropical cyclones. The interaction table itself contains 122 unique port IDs; these figures refer to different scopes and should not be conflated.

The observed start-year row counts are 2015: 105; 2016: 117; 2017: 287; 2018: 194; 2019: 256; 2020: 512; 2021: 308; 2022: 70; and 2023: 78. Event rows are unevenly distributed across time and cyclone IDs, so row-level random splits would create leakage and an overly optimistic uncertainty picture.

## 3. Three important scientific implications

### A. Class imbalance is an immediate benchmark issue

The `total_impact` column is zero for 85.83% of rows and nonzero for 14.17%. A model that always predicts zero would look accurate under raw accuracy alone while failing to identify most disruption-positive examples.

For any binary experiment built from an explicitly defined target, report the confusion matrix, positive-class precision/recall, PR-AUC, specificity, calibration if probabilities are actually emitted, and episode-clustered uncertainty. Preserve the full target semantics: the observed nonzero `total_impact` values range from 1 to 10, so collapsing them into a binary label loses severity information. A positive/nonpositive classification should be a declared secondary task, not a silent reinterpretation of the source field.

### B. Recovery sentinel values are not self-interpreting labels

1,650 rows carry a zero-or-NA sentinel in the recovery-start, recovery-end and day-of-recovery fields. The source profile shows what is present, but does not establish why each value is absent. It may be associated with no detected impact, an unavailable recovery label, or the source's event-window/coding rules.

Do **not** rewrite these sentinels as a recovery time of zero. Do **not** automatically label all of them as right-censored either. Inspect the [interaction data description](https://github.com/ChenchenMobility/Maritime-Data-CyPort/blob/5cb1d8778d3968c5e50d0b8cb8213f2414d692d3/Tropical_cyclone_interaction/Interaction%20Data%20Descrptions.pdf), paper definitions, and source-generation logic; then encode the defensible meanings as distinct states. When observation truly ends before recovery is observed, use an explicit censoring record with its observation-window end.

### C. Storm-level dependence determines the validation split

The 1,927 interaction rows are not 1,927 independent disruption episodes. They are grouped under 90 cyclone IDs and include related ports from the same storm. Every record tied to one cyclone—affected ports, control candidates, observations, revised labels and derived metrics—must remain in a single split.

For the first evaluation:
1. define a target strictly from documented columns and freeze it;
2. compare a predeclared majority/persistence baseline and a simple static baseline;
3. group by cyclone ID before temporal/held-out partitioning;
4. report results separately by event and port;
5. use leave-one-port-out transfer only where the number and composition of ports support it;
6. perform error and calibration analysis with event-grouped uncertainty, not naive row-wise standard errors.

Published derived resilience metrics should not be treated as independent ground truth when they were computed from the same underlying vessel activity used by the candidate model. Recompute outcomes from the rawest permitted observations where possible, and maintain raw/derived feature lineage.

## 4. What the Rust profiler adds

The new `cyport_interaction_profile.rs` module and CLI are designed to:
- parse quoted CSV safely, including commas in quoted port names;
- require the pinned 56-column schema and consistent row widths;
- reject invalid event dates, missing identity fields and invalid impact values;
- calculate a Git blob SHA-1 over the exact Git object preimage and SHA-256 over the supplied bytes;
- verify that an exact-size local file is also the pinned Git blob, failing closed on a same-size mutation;
- enforce the inspected profile totals when the exact pinned source is supplied;
- count sentinel values without converting them into recovery outcomes;
- label a synthetic/schema-only profile as not pinned and never as model validation.

CI exercises only a small synthetic fixture with the same header; it does **not** bundle the dataset or prove the external file has been captured. The pinned-source counts above were computed from the actual fetched file in this research pass; the Rust command has not yet been executed locally or verified by exact-head CI.

## 5. Next steps and acceptance gates

1. Run the profiler on the locally obtained resource, confirm the exact Git blob identity, save its SHA-256 receipt and record the retrieval timestamp outside the source file.
2. Examine the accompanying data-description PDF and paper methodology to resolve `total_impact`, recovery-sentinel, event-window and baseline definitions. Pin these interpretations in a versioned mapping document.
3. Build canonical event records grouped by `SID`, preserving all interactions and source row IDs.
4. Freeze a cyclone-grouped temporal holdout before training or tuning.
5. Benchmark transparent baselines first; use imbalance-aware metrics and report positive-class failures explicitly.
6. Validate whether vessel-count reduction and recovery are fit-for-purpose proxies for the specific service claim. Do not label them cargo delivered, safety, berth feasibility or operational recovery without evidence.
7. Broaden to other hazard classes and geographies only after the U.S. tropical-cyclone slice has honest held-out results.

**Completion ceiling:** verified source schema and a source-specific profile contract. No raw source artifact is committed, no historical model score has been produced, and no maritime safety or operational claim is established.
