# Port Asset Inspection-to-Maintenance Benchmark v0.1

**Status:** synthetic evidence/decision contract seed. Not a robot controller, NDT procedure, structural assessment, asset-management system, work-order authority, berth closure decision or permission to reopen an asset.

## Ownership

Parent: [PORT-ASSET-INSPECTION-001 #4476](https://github.com/Luminous-Dynamics/symtropy/issues/4476).

- Symthaea [PERSISTENT-PLATFORMS-001 #7343](https://github.com/Luminous-Dynamics/symthaea/issues/7343) owns aerial/marine platforms, sensing, embodiments and physical qualification.
- Symtropy [COASTAL-SEDIMENT-CHANNEL-001 #3279](https://github.com/Luminous-Dynamics/symtropy/issues/3279) owns sediment accounting, channel/bathymetry change and scour/morphology coupling.
- Symtropy [COMPONENT-DIGITAL-THREAD-001 #3331](https://github.com/Luminous-Dynamics/symtropy/issues/3331) owns component identity, metrology/NDT, repair and as-built qualification.
- Symtropy [MARITIME-ASSET-TWIN-COMPOSITION-001 #4406](https://github.com/Luminous-Dynamics/symtropy/issues/4406) owns federated asset identity, state/time semantics and twin commissioning.
- Sol-Atlas [MARITIME-COMMONS-EVIDENCE-PROJECTION-001 #654](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/654) owns spatial/temporal projection.
- Mycelix [MARITIME-CROSS-STACK-GOLDEN-TRACE-001 #5974](https://github.com/Luminous-Dynamics/mycelix/issues/5974) owns issuer authority, custody, approval/revocation and accountable action receipts.

This issue owns inspection-evidence quality, condition disposition and maintenance-decision comparison. It must not duplicate the above systems.

## Research and industry baseline

**Lima et al., Journal of Marine Science and Engineering 14(21), 2066 (3 November 2026)** reviewed 1,030 records and retained 86 studies on unmanned/autonomous maritime-port systems. Their coding reports 15 conceptual/review studies, 14 simulations, 29 controlled experiments, 27 field/port validations and one documented operational deployment. The review found no coded record explicitly reporting a measured improvement in decision outcomes; it recommends repeated multi-port validation, operational-system integration and comparative decision-outcome evaluation.

Source: https://www.mdpi.com/2077-1312/14/21/2066
DOI: https://doi.org/10.3390/jmse14212066

**Liu & Heikkilä, European Transport Research Review 18, 66 (12 August 2026)** review equipment-level automation and identify port inspection/maintenance applications for quays, storage areas and crane structures using UAVs, ROVs and AI/SLAM, while noting scarce comparative large-scale field studies.

Source: https://link.springer.com/article/10.1186/s12544-026-00816-2

The IHO S-100 Phase 1 product specifications entered into force in January 2026; S-102 Edition 3.0.0 is listed as in force from 1 January 2026. S-102 is bathymetric context, not structural condition or qualified NDT. Survey-order/coverage/vertical-datum/uncertainty evidence must remain source-bound.

Sources:
- https://iho.int/en/the-s-100-framework-is-now-operational
- https://iho.int/en/enc-ecdis
- IHO S-44: https://iho.int/en/standards-and-specifications

## Paradigm: measure the quality of the maintenance decision

UAV, ROV/AUV, wall crawler, survey vessel, diver/manual inspection and targeted NDT are complementary methods selected by access, environment and inspection task. A platform leaderboard would hide this complementarity.

Evaluate the complete chain:

`asset identity -> inspection mission -> sensor observations -> calibrated coverage -> indication -> independent reference -> condition disposition -> maintenance decision -> authorized intervention -> post-action verification`

Keep each stage distinct:
- an observation is not a defect;
- an indication is not a confirmed condition;
- model confidence is not calibration;
- a condition assessment is not a structural-capacity calculation;
- a maintenance recommendation is not an approved work order;
- work completion is not post-repair acceptance;
- a recent retrieval timestamp is not recent observation time.

A no-indication report is valid only for the verified inspected coverage; an obscured, inaccessible, shadowed or unresolved region is Unknown, not clear.

## v0.1 synthetic scope

Three representative port asset families:
1. reinforced-concrete quay wall / quay face;
2. steel pile / sheet pile / dolphin and submerged connections;
3. fender / bollard / crane-rail support and visible steelwork.

Platform and sensor families include optical UAV/ROV, imaging or multibeam sonar, AUV, structure-relative crawler, manual/diver survey and targeted NDT/metrology. The fixture uses only symbolic synthetic asset/sensor/method identities—no actual port drawings, sensitive infrastructure vulnerability data, real defect measurements or real underwater imagery.

Each case binds:
- asset/component/feature identity, configuration and geometry revision;
- mission/platform identity and status;
- sensor identity, modality, calibration/health/validity, resolution/range and source digest;
- observation time, retrieval time, position reference, relative localization/pose error, sensor coverage, occlusion and uncertainty;
- water clarity/current/environment envelope;
- CRS, vertical datum and bathymetric survey/order/coverage/uncertainty where used;
- observation/indication/assessment/reference/decision/action states;
- reviewer, source authority, rights and independent-reference lineage;
- projection/twin latency/fidelity and read-only write boundary.

## Benchmark design

### Detection and measurement
- sensitivity/recall and false-negative rates per indication/severity class;
- precision/false-positive and unnecessary follow-up rate;
- localization/registration error and uncertainty;
- calibrated thickness/deformation measurement error against a qualified reference;
- uncertainty interval coverage and confidence calibration;
- repeatability across campaigns, platforms and operators;
- defect/change detection after exact geometry/time registration.

Report by material, asset type, submerged/surface zone, sensor, water clarity, illumination, range, incidence angle, access, defect family and ODD. Never pool away difficult classes.

### Mission and coverage quality
- verified surface/volume/feature coverage and explicit blind/inaccessible regions;
- minimum detectable feature size only after physical calibration;
- relative SLAM/pose drift, loop closure and asset-relative localization;
- mission interruptions and recovery;
- comms/telemetry continuity, battery, tether and operator interventions;
- p50/p95/p99 observation-to-decision-ready latency and stale-state duration.

### Decision outcome (the under-measured frontier)
Run a preregistered matched or stepped-wedge comparison of baseline human inspection, robot-assisted human review and multi-platform evidence:
- time to validated condition disposition;
- reviewer disagreement/adjudication rate;
- critical missed-indication rate against blind independent reference;
- unnecessary repeat inspection/NDT/work order rate;
- priority changes and independent support for them;
- time to intervention, berth/asset downtime and post-repair recurrence;
- cost/inspection burden per accepted condition decision.

Record confounders: weather, tide, current, turbidity, access windows, asset age, crew and prior maintenance. A faster dashboard or more detections is not automatically a better decision.

## Frozen synthetic corpus

The accompanying v0.1 corpus contains 36 unique cases: one valid synthetic control and 35 adversarial cases covering identity, revision/effectivity, sensor calibration/range, localization, visibility, acoustic shadow, incomplete coverage, temporal skew, vertical datum, S-102 coverage, sparse scour observations, thickness evidence, indication/assessment escalation, calibration, independent references, false negatives/positives, multi-sensor conflict, stale state, out-of-order geometry, unqualified NDT claims, authority bypass, repair verification, mission interruption, rights/terms, geometry registration, duplicated blind spots, latency and simulated writeback.

The corpus is a fixture/specification seed, not an executed physical inspection or decision benchmark. The expected codes are contract-oracle labels; the follow-on Rust verifier must independently recompute them.

## Promotion targets (not achieved results)

- 100% correct asset/feature/mission/sensor identity on frozen cases.
- Zero promotion from occluded/uninspected to clear.
- 100% separation of observation, indication, interpreted condition, independent reference, decision and authorized action.
- Zero simulated/counterfactual writeback into canonical as-operated asset records.
- Zero misses for planted critical synthetic indications in the contract oracle; physical detection sensitivity must be measured with confidence intervals against qualified reference, not claimed as universal 100%.
- All geometry/time transformations retain parent digests, CRS/datum and uncertainty.
- Decision-outcome superiority only after preregistered, repeated, multi-campaign/multi-port comparison with independent adjudication and measured effect size/uncertainty.

## Claim ceiling

A passing synthetic corpus establishes only the declared identity, coverage, provenance, timing and decision-boundary behavior. It does not establish defect-detection accuracy, structural capacity, NDT qualification, asset safety, maintenance urgency, repair acceptance, berth availability, permission to close/reopen or authority to operate.
