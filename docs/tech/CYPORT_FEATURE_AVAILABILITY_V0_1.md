# CyPort Feature Availability Audit v0.1

**Status:** 56/56 CSV fields have a conservative policy classification. **No predictor is approved.** The register is a provenance backlog and leak-prevention gate, not proof that the upstream field definitions are all understood.  
**Machine-readable register:** [CYPORT_FEATURE_AVAILABILITY_V0_1.json](./CYPORT_FEATURE_AVAILABILITY_V0_1.json)  
**Verifier:** `cargo run --locked -p maritime-data-receipt -- --validate-cyport-feature-availability docs/tech/CYPORT_FEATURE_AVAILABILITY_V0_1.json`  
**Pinned source:** [CyPort interaction CSV at commit 5cb1d8778d3968c5e50d0b8cb8213f2414d692d3](https://github.com/ChenchenMobility/Maritime-Data-CyPort/blob/5cb1d8778d3968c5e50d0b8cb8213f2414d692d3/Tropical_cyclone_interaction/interaction_data.csv)  
**Source description:** [README at the same immutable commit](https://github.com/ChenchenMobility/Maritime-Data-CyPort/blob/5cb1d8778d3968c5e50d0b8cb8213f2414d692d3/README.md)

## Why this audit is necessary

The upstream README describes multiple blocks: 2015–2023 baseline port and freight data; cyclone-period exposure; observed weather/tide data; port network centralities on recent weekly freight networks; resilience metrics; and static port characteristics joined from external sources. The fact that a variable is useful for retrospective explanation does not show that it was available when a real port operator had to make a decision.

The v0.1 rule is therefore intentionally conservative: the register accounts for all 56 field names in CSV header order, marks likely roles and the evidence needed to clear them, but leaves the predictor whitelist empty. This prevents the benchmark contract from being mistaken for a verified point-in-time dataset.

## Field eligibility summary

| Field family | Source columns | Initial disposition | Main evidence needed |
|---|---|---|---|
| Row/storm/port identifiers | `ID`, `SID`, `PID` | Join/group keys only | Deduplication, joins, and SID-level split/uncertainty; no raw ID features |
| Port labels / geography | `port`, `PORT_LON`, `PORT_LAT` | Blocked | Exact port crosswalk, coordinate reference/datum, and unseen-port evaluation policy |
| Time windows | `start_date`, `end_date` | Blocked | Prove values and their semantics were known at a forecast issue time; an observed interaction interval is not an advance forecast |
| Outcomes / post-event fields | `start_recovery_date`, `end_recovery_date`, `total_impact`, `total_impact_value`, `max_impact`, `day_of_recover` | Forbidden as predictors | Use as task-appropriate labels or descriptive fields; preserve rounding and marker-state distinctions |
| Unresolved codes | `DD`, `CCD`, `BCD`, `SSHS`, `SSHS_1`–`SSHS_5`, `WF`, `PCT_TA`, `PCT_ACEW`, `PCT_TI`, `PCT_SV`, `SL` | Blocked pending dictionary | Exact upstream encoding, units, category order, sentinel values and timestamp |
| Event/exposure variables | `IF_LANDFALL_CLOSE2PORT`, `IF_LANDFALL`, `WIND`, `PRESSURE`, `DISTANCE`, `Wind_speed`, `Surge_height`, `Rainfall` | Blocked pending point-in-time evidence | Source product, valid time, issue/ingest time, spatial aggregation, units, vintage and missingness; observed conditions must not substitute for forecast vintages |
| Freight-network variables | `Port_ranking`, `D_normal`, `D_cyclone`, `C_normal`, `C_cyclone`, `B_normal`, `B_cyclone`, `E_normal`, `E_cyclone` | Blocked pending graph-window audit | Graph/edge definitions, vessel or cargo scope, weekly window timestamps, forecast cutoff, transformation lineage and per-fold recomputation |
| Static/context candidates | `Seaport`, `Pop_C`, `Pop_Tract`, `PCT_Pov`, `Railway_Length`, `Highway_Length`, `Dock_Count`, `Coast_Gulf_of_Mexico`, `Coast_Atlantic`, `Coast_Pacific` | Blocked pending source-vintage and join review | Exact column meaning, source vintage/as-of time, geography crosswalk, units, missingness, licensing/attribution and transfer to unseen ports |

The classifications above are cautious, initial dispositions. In particular, the names `DD`, `CCD`, `BCD`, `WF`, and several percentage fields are not interpreted from abbreviations alone. Their precise definitions must be recovered from the paper's data dictionary or upstream build code.

## Evidence required to clear a feature

For each candidate predictor, require a field-level receipt containing:
1. Exact definition, upstream file/revision, source license and attribution.
2. Valid time plus issue/ingest time; whether it is a forecast, an observation, a derived metric or a later revised product.
3. Measurement units, spatial/port aggregation, coordinate/datum where applicable, and missing/sentinel policy.
4. Transformation formula and version; imputation/scaling/category encodings fit on train only.
5. A reproducible join key and effective timestamp; for network features, the exact graph snapshot must end before the decision/forecast cutoff.
6. Explicit test coverage for leakage controls, missingness shifts and unseen-port transfer.

Only then may an independently reviewed whitelist be created as a new versioned artifact. Do not mutate v0.1 to silently approve fields mid-experiment.

## Research implications

A credible benchmark should compare a train-only prevalence/climatology baseline, transparent regularized GLM/count models, a hurdle model, and then stronger tabular/graph challengers. Network features should earn their complexity through ablation against node-only models. For operational preparedness, keep forecast vintages and replay decisions at actual issue times; the current retrospective interaction table does not by itself support a valid 24/48/72-hour forecast benchmark.

**Claim ceiling:** complete column accounting and an enforceable conservative policy. No feature timing is certified, no model is fit, and no prediction or operational decision is authorized.
