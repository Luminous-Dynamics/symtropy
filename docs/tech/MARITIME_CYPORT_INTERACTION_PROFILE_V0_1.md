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
| Zero impact + all three recovery fields sentinel | 1,650 rows |
| Zero impact + all three recovery fields present | 4 rows |
| Nonzero impact + all three recovery fields present | 273 rows |
| Mixed recovery-field sentinel triplets | 0 rows |

The broader CyPort release is described as covering 145 principal U.S. ports and 90 tropical cyclones. The interaction table itself contains 122 unique port IDs; these figures refer to different scopes and should not be conflated.

The observed start-year row counts are 2015: 105; 2016: 117; 2017: 287; 2018: 194; 2019: 256; 2020: 512; 2021: 308; 2022: 70; and 2023: 78. Event rows are unevenly distributed across time and cyclone IDs, so row-level random splits would create leakage and an overly optimistic uncertainty picture.

## 3. Three important scientific implications

### A. Treat this as an excess-zero / severity problem, not a raw-accuracy problem

The `total_impact` column is zero for 85.83% of rows and nonzero for 14.17%. The paper defines total impact as equivalent days of vessel activity lost and explains that originally non-integral impact values are rounded to the nearest integer for count-model analysis. The paper also describes cases with no detected impact window as ports whose vessel traffic remained within the normal range. These semantics matter: an all-zero predictor could obtain 85.83% accuracy on the *rounded* `total_impact > 0` classification while missing every nonzero value. Yet zero is not a guaranteed no-impact label, as the row cross-tab below demonstrates.

For any explicitly defined binary experiment, report the confusion matrix, positive-class precision/recall, PR-AUC, specificity, and episode-clustered uncertainty; include calibration only when probabilities are emitted and evaluated. Preserve the ordinal/count severity information in `total_impact` and continuous proxy fields such as `total_impact_value` and `max_impact`. Compare at least a simple baseline and a zero-aware/hurdle-style model, but do not treat the published metrics as independent ground truth if derived from the same observations. Source paper definitions: [impact-window method and normal-traffic cases](https://www.researchgate.net/publication/395969987_US_Port_Disruptions_under_Tropical_Cyclones_Resilience_Analysis_by_Harnessing_Multiple-Source_Dataset) and [rounding/excess-zero discussion](https://www.researchgate.net/publication/395969987_US_Port_Disruptions_under_Tropical_Cyclones_Resilience_Analysis_by_Harnessing_Multiple-Source_Dataset).

The executable source profile now keeps three distinct states separate: (1) no impact window recorded (all markers sentinel-coded with rounded impact zero), (2) impact-window recovery markers present (all markers present, regardless of rounded impact), and (3) malformed/mixed marker triplet. It also records any nonzero rounded impact with all markers missing as a separate contradiction state; the pinned source has zero of those cases. This prevents “missing recovery markers” from being used as a catch-all for every uncertainty.

The inspected row cross-tab is:
- 1,650 records have `total_impact=0` plus all three recovery fields set to zero/NA.
- 4 records have `total_impact=0` but all three recovery fields present; they also have nonzero `total_impact_value`, `max_impact` and `day_of_recover`.
- 273 records have nonzero `total_impact` and all three recovery fields present.
- No mixed sentinel triplets occur in the pinned table. There are also no nonzero-rounded-impact rows with all three recovery markers missing; the profiler tracks that condition separately from a partial/mixed triplet.

That 4-row exception is why zero cannot be blindly translated to “no disruption”. Given the paper's rounding rule and nonzero impact proxies, these rows may represent small positive unrounded impact that rounded down to zero. That is a plausible interpretation, not a row-level label to silently assume.

### B. Interpret recovery markers as a coherent triplet

The paper describes `start_recovery_date` and `end_recovery_date` as markers from the constructed resilience curve and `day_of_recover` as recovery duration in days. It describes no detected impact window as normal vessel traffic during the cyclone. These definitions are more informative than an untyped missing-value policy, but exact row-level coding should still be validated against the source's construction procedure.

Do **not** rewrite sentinels to a recovery time of zero and do **not** automatically label all absent recovery markers as right-censored. The pinned data has a consistent pattern of all three fields present or all three sentinel-coded. Preserve that as a triplet state:
- `recovery_markers_present`: all three markers are present and individually valid;
- `no_recovery_markers`: all three are sentinel-coded, with semantics to be resolved from the source pipeline and impact fields;
- `mixed_recovery_marker_state`: a data-quality exception; reject or isolate instead of imputing.

Only call an outcome right-censored where the source's observation window truly ends before recovery can be observed, and retain the actual observation-window end. The data's exposure start/end are not themselves evidence of the recovery observation cutoff.

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

### Frozen cyclone-grouped temporal split

The pinned source profile now accompanies [a frozen episode-split manifest](./CYPORT_EPISODE_SPLIT_V0_1.json), built by grouping every interaction row by cyclone `SID`, deriving that group's earliest `start_date`, and assigning the entire storm to one temporal partition:

| Split | Storms | Interaction rows | Unique ports represented |
|---|---:|---:|---:|
| Train (through 2020) | 59 | 1,471 | 122 |
| Calibration (2021) | 13 | 308 | 92 |
| Test (2022 onward) | 18 | 148 | 61 |

The split manifests all 90 cyclone IDs, not row-level samples. The fixed policy is `cyport-sid-temporal-v0.1:train<=2020;calibration=2021;test>=2022`. The validation code recomputes the manifest's internal counts and date boundaries. A second command compares a local CSV to the frozen manifest and requires the exact pinned source identity before accepting the comparison:

```sh
cargo run --locked -p maritime-data-receipt -- \
  --validate-cyport-episode-split-manifest \
  docs/tech/CYPORT_EPISODE_SPLIT_V0_1.json

cargo run --locked -p maritime-data-receipt -- \
  --verify-cyport-episode-split \
  /path/to/interaction_data.csv \
  docs/tech/CYPORT_EPISODE_SPLIT_V0_1.json
```

The first command checks manifest shape and internal consistency only; it does not re-read the source. The second re-profiles local source bytes, checks the Git blob identity, and compares every ordered cyclone group and split with the manifest. Because the dataset is public, this is a reproducible temporal split—not a secret blind holdout if the candidate method has already seen this dataset or its published derivatives.

## 5. Next steps and acceptance gates

1. Run the profiler on the locally obtained resource, confirm the exact Git blob identity, save its SHA-256 receipt and record the retrieval timestamp outside the source file.
2. Examine the accompanying data-description PDF and paper methodology to resolve `total_impact`, recovery-sentinel, event-window and baseline definitions. Pin these interpretations in a versioned mapping document.
3. Build canonical event records grouped by `SID`, preserving all interactions and source row IDs.
4. Freeze a cyclone-grouped temporal holdout before training or tuning.
5. Benchmark transparent baselines first; use imbalance-aware metrics and report positive-class failures explicitly.
6. Validate whether vessel-count reduction and recovery are fit-for-purpose proxies for the specific service claim. Do not label them cargo delivered, safety, berth feasibility or operational recovery without evidence.
7. Broaden to other hazard classes and geographies only after the U.S. tropical-cyclone slice has honest held-out results.

**Completion ceiling:** verified source schema and a source-specific profile contract. No raw source artifact is committed, no historical model score has been produced, and no maritime safety or operational claim is established.
