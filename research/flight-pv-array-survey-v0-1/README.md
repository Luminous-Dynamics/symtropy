# Flight PV Array Survey v0.1

Parent: Symtropy #5803  
Portfolio: #5700 / #5702  
Starting app: crates/apps/symtropy-flight-demo  
Status: standalone synthetic-first Rust reference; exact-head runner qualification pending.

## Purpose

Qualify **survey acquisition evidence** for a synthetic PV array using:
- revision-bound array/module geometry;
- RGB image coverage;
- radiometric IR pairing;
- ground sample distance (GSD);
- exposure/integration-time motion-smear upper bounds;
- camera incidence and georeferencing/footprint uncertainty;
- consecutive along-track image overlap;
- environmental and radiometric QA/QC metadata;
- independent route-pass/view-angle coverage;
- a separate model-based energy-return envelope.

This is not an image processor, photogrammetry implementation, radiometric camera calibration, PV fault classifier, thermographer, module health assessor, SCADA diagnosis or maintenance authority.

## Published-standard boundary

IEC TS 62446-3:2017, Edition 1, remains the published IEC specification at the date of this v0.1 research branch. IEC's catalogue shows Edition 2 is a separate in-development project; re-check the official catalogue and applicable national adoption before any real campaign.

- IEC catalogue: https://webstore.iec.ch/en/publication/28628
- IEC TS 62446-3 Edition 2 project: https://iss.rs/en/project/show/iec%3Aproj%3A134729

The published 2017 specification treats aerial-drone whole-array thermography as simplified inspection; it can help find obvious anomalies but may not resolve fine artifacts or identify specific failure modes. It also requires camera speed to be related to detector response time to avoid IR motion smear. Reported common bolometer smear can appear around 3 m/s; this is not a universal safe speed threshold.

## Two independent evidence outcomes

- **Geometric survey coverage**: RGB view geometry, footprint, uncertainty, GSD, sharpness, blur, incidence and trackline overlap support module-level mapping.
- **Radiometric survey eligibility**: the paired IR frame passes camera/calibration/NUC, GSD/blur, irradiation/weather, emissivity, reflected-apparent-temperature or reflection-mitigation, saturation, glare and QC gates.

A module may be geometrically covered but radiometrically ineligible. A module with eligible radiometry but uncertain array registration is not qualified. The report never labels a module healthy, faulty, safe, accepted or in need of maintenance.

## P2 camera-constrained flight envelope

The camera envelope helper takes RGB and radiometric-IR horizontal/vertical field of view, pixel dimensions, maximum GSD and uncertainty, exposure/detector integration time, altitude bounds, maximum blur and speed uncertainty. It computes:
- the maximum altitude that satisfies both cameras' horizontal and vertical GSD bounds;
- the common RGB/IR footprint at the selected altitude;
- the blur-limited speed for each modality;
- the common commanded speed ceiling after subtracting speed uncertainty.

The conservative GSD bound is nominal GSD plus its uncertainty. The motion-smear proxy uses the smaller axis GSD after subtracting GSD uncertainty and multiplies the speed upper bound by exposure/integration time. This is a first-order translation-only envelope; it omits rotation, gimbal motion, rolling shutter, vibration, propwash, terrain/array-plane relief and camera-specific image processing. It is not a flight controller or operational flight authorization.

## P1 raster route baseline

The library includes a deterministic boustrophedon/lawnmower route generator. It derives the array-plane bounding rectangle from the exact module roster, computes along-lane and cross-lane waypoint spacing from the conservative camera footprint and requested overlap fractions, alternates lane direction, and reports which complete modules fall inside each planned footprint.

It also reports a straight-line route-length lower bound over the generated waypoint centres. It deliberately omits turn radius, acceleration/deceleration, wind compensation, obstacles, rotor dynamics, controller tracking and flight-safety constraints; it is not a flyable trajectory. P2 should later select altitude and speed jointly from camera FOV, detector integration/exposure, GSD and motion-blur limits, rather than treating a fixed footprint as universally valid.

## Motion smear model

The v0.1 upper-bound proxy is:

    blur_px = (ground_speed + speed_uncertainty) * integration_time
              / (GSD - GSD_uncertainty)

RGB uses exposure time; IR uses detector integration time. The estimate is meaningful only if the supplied speed, GSD and integration-time fields are valid and refer to the same acquisition geometry. It omits rotational/gimbal motion, shutter readout, vibration and image-processing effects; real sensors require measured motion-blur validation.

## Coverage semantics

- Module polygons and frame footprints are axis-aligned rectangles in a frozen array-plane frame.
- The admitted footprint is conservatively shrunk by declared pose and footprint uncertainty.
- A module is covered only when its **entire** bounds lie inside the conservative footprint.
- Frames are grouped into ordered survey passes. Consecutive along-track overlap is computed from adjacent uncertainty-shrunk footprints, not copied from a robot-reported percentage.
- RGB image time must agree with the pose/event time within the profile's pairing tolerance; RGB/IR pairing has the same explicit skew gate.
- Multiple frames in one pass are not multiple independent survey passes.
- Distinct pass IDs and minimum view-angle separation are required per module.
- Duplicate frame/trigger IDs and stale array/frame/camera/calibration revisions are rejected.
- Invalid radiometric calibration/NUC evidence remains InvalidEvidence at frame, module and (when IR is required) survey level; it is not silently downgraded to Unknown or NotEligible.
- Results are sorted deterministically; no module omissions are hidden inside an aggregate percentage.

## Environmental and radiometric gates

The example profile's 600 W/m² irradiance, wind, cloud stability, emissivity, glare, saturation, GSD and image-quality limits are illustrative profile parameters. They are not asserted to be the complete IEC requirements or a universal PV survey recipe. The applicable published standard and site's measurement plan must determine actual thresholds and operating conditions.

## Run

From this directory:

    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test --all-targets
    cargo run

No third-party dependencies are required.

## Synthetic tests

Tests cover full module coverage, partial footprints, GSD, motion smear, irradiance/radiometry separation, NUC, glare/saturation, missing or skewed RGB/IR pairs, stale identities, duplicate/replayed triggers, pass overlap, independent views, wind, deterministic ordering and return-reserve sufficiency/unknown/overflow.

All tests are synthetic contract checks. They do not compile or validate the existing Bevy app, renderer, flight-control model, camera model, radiometric camera, operational site or a real drone.

## Claim ceiling and owners

- #5803 owns the flight-demo PV survey task and module-level acquisition-evidence evaluator.
- #5702 owns demo maturity/build/launch qualification.
- #3522/#3531 own physical trial receipts/evidence admissibility.
- #5488 owns configuration-change evidence applicability.
- #5657 owns evidence-carrying mission progression.
- Existing PV/SCADA/electrical asset owners retain diagnosis and maintenance decisions.

The result is only survey acquisition evidence. It does not establish physical flight readiness, safe flight near energized infrastructure, aviation compliance, camera calibration, IEC conformity, module health, fault diagnosis, absolute temperature accuracy, SCADA diagnosis, maintenance action or energy-production improvement.
