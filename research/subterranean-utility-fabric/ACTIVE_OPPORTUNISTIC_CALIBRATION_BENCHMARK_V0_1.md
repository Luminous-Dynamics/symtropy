# Active and Opportunistic Fibre-Route Calibration Benchmark v0.1

**Workstream:** OPPORTUNISTIC-CALIBRATION-QUAL-001 (#5002)  
**Parent:** SUF #4910 / DAS #4972  
**Status:** research protocol and synthetic campaign plan; no new method or physical result claimed  
**Date:** 2026-11-13

## 1. Research thesis

Treat fibre-route calibration as an adaptive measurement problem. Ambient traffic and approved controlled excitations may reveal fibre geometry; sparse independent survey control provides the absolute reference needed to place the route in a geographic coordinate frame. Select the next measurement according to uncertainty reduction, decision value, acquisition cost and access constraints.

The central hypothesis is not that passive traffic can always map a route. It is that **the system can spend survey effort where it is most informative and economically consequential**, while explicitly abstaining where geometry is unidentifiable.

## 2. Five distinct claims

The benchmark separates:

1. **Relative path shape:** geometry up to translation, rotation, reflection, scale or other unresolved degrees of freedom.
2. **Absolute route registration:** alignment to an independently declared CRS/datum and geographic control.
3. **Fibre-span identity:** evidence that a measured path belongs to a named, versioned optical fibre span.
4. **Utility asset attribution:** owner-authorized evidence linking that span/route to a particular asset.
5. **Event/condition localization:** placing an event observation on a qualified fibre span and then, only through a valid transform, on the route.

A method may estimate relative geometry without supporting any absolute coordinate. A route can be geolocated while its utility asset identity remains unresolved. Neither claim proves event cause or physical damage.

## 3. Identifiability and datum rule

Absolute geography requires an independent tie to the spatial reference frame. Sufficient ties depend on the method and geometry, but may include:
- surveyed route control points with CRS, axis order, datum and epoch;
- independently located fibre endpoints and direction;
- vehicle trajectories with independently referenced GNSS/INS or surveyed camera controls;
- exposed/verified physical tie points with recorded uncertainty.

Without such a tie, absolute translation/orientation/scale may remain unobservable. Repeated or parallel routes may remain ambiguous even after one anchor. The estimator must expose rank deficiency, multimodal alternatives or weak geometry rather than return a single high-confidence path because an optimizer converged.

The benchmark must declare the minimum independent constraints required by each method. Never use the same candidate algorithm output as its own truth or calibration anchor.

## 4. Candidate methods

| ID | Method | Required evidence and baseline role |
|---|---|---|
| M0 | Uncalibrated optical range mapped directly to ground distance | Negative control only; must not qualify |
| M1 | Fixed offset/scale with sparse control | Simple calibration baseline |
| M2 | Piecewise interpolation between fixed-spacing survey/tap anchors | Conventional calibration baseline |
| M3 | Passive traffic excitation with independent trajectory observations | Reproduction/transfer family, grounded in Cohen et al. 2026 |
| M4 | Controlled trajectory / excitation diversity | Active measurement baseline |
| M5 | Hybrid traffic + tap-test + survey control | Multimodal calibration baseline |
| M6 | Adaptive value-of-information measurement selection | Candidate Luminous research method |
| M7 | Topology-aware joint route/fibre inference with multi-hypothesis output | Advanced candidate; must report ambiguity |

A fair comparison freezes the same route population, held-out segments, candidate action set, measurement budget and independent truth. If methods require different inputs, record their cost and acquisition effort rather than hiding them.

## 5. Active measurement policy

At each iteration:

1. Maintain a set/distribution of route-calibration hypotheses and explicit unsupported/ambiguous states.
2. Identify weakly constrained segments and the decisions affected by their uncertainty.
3. Generate candidate actions: collect passive traffic trajectories, request another independent vehicle path, perform a controlled tap sequence on an approved accessible segment, obtain a survey tie point, inspect a splice/endpoint, or deploy a robot/survey crew.
4. Estimate expected information gain and expected decision-loss reduction separately.
5. Apply hard access, privacy, safety, data-rights and cost constraints.
6. Select no action when value cannot be established or costs/losses are not comparable.
7. Acquire approved data and bind it to exact source identity, timebase, calibration and rights.
8. Update a versioned hypothesis; preserve previous candidates and rejected/indeterminate states.
9. Verify on held-out independent controls not used for fitting.
10. Invalidate downstream route transforms after a fibre, route, endpoint, interrogator or coordinate-frame revision.

A sensor action can improve entropy while leaving the optimal maintenance decision unchanged. In that case information gain may be positive but expected value of sample information can be zero. The policy must not conflate these.

## 6. Required adversarial conditions

- two parallel or nearby fibre routes with similar traffic signatures;
- road surface route diverges from buried duct route;
- traffic trajectories all occupy the same narrow corridor;
- vehicle GNSS multipath or bias;
- video occlusion, frame drops or incorrect camera-to-ground transform;
- trajectory clock offset and drift relative to DAS;
- unknown vehicle speed or unobserved excitation location;
- route self-near approaches and crossings;
- slack loops, patch leads, splice topology and endpoint reversal;
- fibre in conduit versus direct burial, changing coupling and moisture;
- unknown depth, soil anisotropy and changing compaction;
- conflicting legacy route geometry;
- privacy/access permission unavailable for video or traffic data;
- one measurement that reduces uncertainty but changes no decision;
- action costs and decision losses declared in incomparable units;
- adversarial or corrupted trajectory evidence;
- route/fibre revision change after calibration.

## 7. Benchmark metrics

**Spatial accuracy**
- cross-track and along-track P50/P95/max errors separately;
- route shape error after explicitly declared alignment;
- absolute geographic error after independent datum tie;
- error by route curvature, crossing and near-parallel profile.

**Uncertainty and identifiability**
- 90%/95% interval empirical coverage and interval width;
- calibration error by profile;
- route hypothesis count and ambiguity-preservation rate;
- high-confidence false-assertion rate on non-identifiable scenarios;
- numerical conditioning/rank-deficiency diagnostics where applicable;
- abstention quality and risk-coverage curve.

**Measurement efficiency**
- independent anchor count and spacing;
- measurement actions and field minutes per accepted mapped km;
- cost per accepted mapped km;
- error reduction per measurement/cost unit;
- expected information gain, expected value of sample information and realized decision regret.

**Operational integrity**
- time/route/fibre revision mismatch rejection;
- invalidation completeness following splice/reroute/configuration changes;
- percentage of source evidence with rights and lineage;
- privacy/access denial behavior;
- preservation of all failures and ambiguous outputs.

## 8. Proposed research targets (not achieved results)

- Zero high-confidence absolute geography without a valid datum/anchor/trajectory tie.
- 100% correct rejected/indeterminate treatment of the finite negative-control corpus for route identity, endpoint, CRS, timebase, revision and rights faults.
- Nominal 95% uncertainty intervals achieve statistically defensible empirical coverage on held-out in-domain routes; publish finite-sample intervals and width.
- At matched budget, M6 should improve P95 route error, cost per accepted mapped kilometre or decision regret relative to fixed-spacing anchors; it need not win every stratum.
- Retain multiple hypotheses for ambiguous parallel/nearby routes.
- Demonstrate the limit where passive traffic is insufficient and identify the smallest additional control measurement that resolves the ambiguity.
- Report failures to beat baselines. Do not tune on the held-out site or route.

## 9. Independent truth and leakage prevention

- Route/segment groups, acquisition days and vehicles used for fitting do not appear in held-out evaluation where generalization is claimed.
- Repeated frames, overlapping DAS windows and repeated trajectories from one physical event stay within one split.
- Independent survey control used for evaluation is withheld from fitting.
- If a control point is used to fit the route, it cannot also be reported as an independent validation point.
- Report the original paper split only for faithful reproduction, then report leakage-resistant grouped splits separately.
- Store exact data version, byte digests, licence/permission, preprocessing lineage, time synchronization, unit conventions and held-out grouping.

## 10. Cross-project integration

- Symtropy #5002: active/opportunistic calibration policies, measurement selection, replay and benchmark.
- Symtropy #4972: DAS event chainage, data ingestion and observability strata.
- Symtropy #4925: mobile robot localization and physical measurement pose; do not duplicate that estimator.
- Symtropy #4953: mobile in-pipe acoustic sensing; distinct from traffic/DAS route calibration.
- Sol-Atlas #755: route geometry revisions, CRS/datum, uncertainty, provenance and disclosure.
- Symthaea: bounded policy proposals and information/decision-value analysis.
- Mycelix: custody, authorization and accepted survey/maintenance evidence; no access authority.
- Utility owner/surveyor: route ownership, permits, independent controls and asset attribution.

## 11. References

- Cohen et al. (2026), *Buried Fiber-Optic Geolocalization With Distributed Acoustic Sensing*, DOI 10.1109/TGRS.2026.3729504: https://doi.org/10.1109/TGRS.2026.3729504
- Kamaruzaman et al. (2026), operational SCADA fibre event detection, DOI 10.1016/j.rio.2026.101093: https://doi.org/10.1016/j.rio.2026.101093
- OGC SensorThings API 2.0: https://docs.ogc.org/is/23-019/23-019.html
- OGC MUDDI Part 1 v1.0: https://docs.ogc.org/is/23-024/23-024.html

## 12. Claim ceiling

This is a proposed experiment programme. It is not a working adaptive calibration algorithm, a route survey, a DAS reproduction, a physical field result, proof of unique route identifiability, or evidence of reduced survey costs.
