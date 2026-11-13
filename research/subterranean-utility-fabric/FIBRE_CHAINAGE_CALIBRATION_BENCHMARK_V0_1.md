# Fibre Chainage and Route Calibration Benchmark v0.1

**Owner:** Symtropy — new qualification tranche under #4972  
**Spatial owner:** Sol-Atlas #755  
**Status:** proposed benchmark protocol and synthetic scenario plan; no calibration algorithm or survey result is claimed

## 1. Problem statement

A distributed acoustic sensing system reports measurements along an optical path. A useful infrastructure decision needs to know which physical fibre, which route segment and potentially which utility asset is implicated. These are not the same coordinate.

The common failure is to treat an interrogator distance bin as if it were a surveyed geographic coordinate. This silently ignores patch leads, optical-path offsets, fibre slack, coils, splice/repair changes, conduit routing, path branching, multiplexing, endpoint reversal, survey datum and route uncertainty.

## 2. Coordinate model

Use distinct, typed coordinates:

| Symbol / field | Meaning | Unit / reference | What it is not |
|---|---|---|---|
| optical_range_m | Interrogator-reported position along the configured optical path | metres on an exact interrogator/configuration revision | Not automatically physical cable/fibre length or ground distance |
| physical_fibre_chainage_m | Cumulative measured fibre length along a named fibre path, including accounted slack/loops/patch segments | metres from a named endpoint and direction | Not utility-route station |
| route_station_m | Curvilinear station along a versioned surveyed route geometry | metres along that geometry, with explicit route/segment ID | Not optical range or straight-line distance |
| geographic_position | Spatial position derived from a route geometry and CRS/datum/epoch | declared CRS and axis order; vertical reference where available | Not warranted by a chainage value alone |
| asset_attribution | Evidence that a fibre/route position relates to an owner-issued asset identity | owner namespace and exact asset revision | Not inferred merely because two geometries are nearby |

The system may use a graph of fibre spans, patch leads, splices, splitters, multiplexed fibres, loops and endpoint identities. A single scalar offset/scale is permissible only when the exact configuration and validation evidence demonstrate that a simpler mapping is adequate.

## 3. Transform chain and receipts

Every mapping step is a separately versioned transformation:

1. optical range → physical fibre chainage;
2. physical fibre chainage → fibre span / route segment and orientation;
3. fibre span / route station → surveyed route geometry;
4. mapped geometry → geographic coordinates in the declared CRS and vertical reference;
5. geographic position → owner asset attribution, if supported.

Each receipt records source and target frame IDs, source and target revision, method/version, configuration digest, control-point IDs, independent truth source, residual distribution, uncertainty, validity interval, operator/agent, rights scope and acceptance state.

A receipt becomes invalid when any required input revision changes, including the interrogator configuration, optical path, fibre endpoint/orientation, splice/patch topology, as-built route, survey control or coordinate reference. Superseded transforms remain immutable for replay; do not overwrite history.

## 4. Benchmark populations

Keep these populations separate:

- paper reproduction: Cohen et al.'s traffic-induced route-geolocation conditions;
- direct-buried communications fibre;
- fibre in conduit;
- shared trench with an offset target utility;
- fibre with known slack coils, patch leads or multiple splice events;
- branched, multiplexed or multi-span optical paths;
- route geometry with sparse surveyed anchors;
- repaired/rerouted fibre with a new as-built revision;
- unknown endpoint, orientation or datum;
- non-linear route with turns, loops, self-near approaches or ambiguous road/fibre alignment.

Do not transfer a result from one profile to another without a held-out test.

## 5. Required baselines

Compare against matched truth and identical inputs:

- uncalibrated optical-range-as-ground-distance baseline (negative/control baseline, never accepted as a qualified method);
- fixed offset/scale calibration;
- piecewise linear interpolation between known exposed/surveyed tie points;
- manual tap-test or equivalent field calibration where permitted;
- matched-filter initialization from the Cohen et al. method;
- physics-informed strain-map inversion;
- complete trajectory-optimization method;
- topology-aware factor-graph or joint route/fibre calibration method as a new research candidate.

Report each baseline's assumptions and resource inputs. Do not compare a manually surveyed dense route against a sparse-anchor algorithm as though the inputs were equivalent.

## 6. Metrics

- optical-to-fibre residual P50/P95/max;
- fibre-to-route station residual P50/P95/max;
- horizontal cross-track, along-track and vertical errors separately;
- end-to-end geographic error P50/P95/max;
- 90% and 95% interval empirical coverage and interval width;
- route-length completeness and length with valid calibration;
- ambiguous/one-to-many mapping rate and correct abstention rate;
- false high-confidence geography on invalid, stale, reversed or ambiguous inputs;
- number and spacing of independent tie points;
- field calibration time and cost per accepted mapped kilometre;
- sensitivity to endpoint, splice, slack, gauge-length and interrogator configuration changes;
- recalibration latency after repair and proportion of dependent observations correctly invalidated.

The error metric used for the stretch target must match the published paper's exact definition before a direct numerical SOTA comparison is made.

## 7. Qualification gates (proposed research targets)

- 100% rejection or indeterminate disposition for seeded endpoint reversal, missing CRS/datum, stale route revision, unbound optical configuration and ambiguous topology.
- Zero high-confidence geographic coordinates when any required transform is missing or invalid.
- 95% uncertainty interval coverage on held-out, in-domain route profiles after calibration.
- Preserve all alternatives where two or more route hypotheses remain plausible.
- Keep route geometry, asset identity and sensor observation identity separate.
- A P95 cross-track error below 0.5 m is a stretch target for a held-out profile only after reproducing the Cohen et al. baseline and confirming that the paper metric, truth metrology and profile are comparable. It is not a current result or a universal requirement.
- Publish calibration cost and survey effort alongside error. A centimetre-scale result obtained with dense manual control may be less valuable than a metre-scale result at a fraction of the cost for a different decision.

## 8. Adversarial scenarios and oracle policy

The accompanying machine-readable manifest freezes planned scenarios only. The independent oracle must verify exact input revisions, transform dependencies, expected invalidation and whether a geographic claim is warranted. A scenario result is Accepted, Rejected, Indeterminate or Unsupported.

The oracle must not reuse the candidate algorithm's route prediction as its truth. Independent truth comes from surveyed control, physically exposed tie points, known laboratory path construction, or a declared source whose uncertainty is itself recorded.

## 9. Cross-project integration

- Symtropy #4972: DAS event chainage and instrument/fibre calibration.
- This tranche: transform composition, path topology, uncertainty and stale-revision tests.
- Sol-Atlas #755: spatial-frame, geometry, CRS, route revision, provenance, MUDDI mapping and disclosure.
- Symthaea: propose the next calibration observation or physical check using expected decision value.
- Mycelix: preserve permission, custody and accepted survey/maintenance receipt; never grant field access.
- Surveyor/utility owner: provide authoritative endpoints, tie points, route/as-built evidence and asset attribution.

## 10. References

- Cohen, K. et al. (2026), *Buried Fiber-Optic Geolocalization With Distributed Acoustic Sensing*, IEEE Transactions on Geoscience and Remote Sensing 64, DOI: https://doi.org/10.1109/TGRS.2026.3729504
- Kamaruzaman et al. (2026), field DAS on operational SCADA communication fibre, DOI: https://doi.org/10.1016/j.rio.2026.101093
- OGC MUDDI Part 1 v1.0: https://docs.ogc.org/is/23-024/23-024.html
- OGC SensorThings API 2.0: https://docs.ogc.org/is/23-019/23-019.html

## 11. Claim ceiling

This artifact defines a benchmark and synthetic plan. It does not implement route calibration, establish survey accuracy, reproduce the Cohen paper, certify a route, validate a CRS transformation, identify an asset or provide engineering/survey approval.
