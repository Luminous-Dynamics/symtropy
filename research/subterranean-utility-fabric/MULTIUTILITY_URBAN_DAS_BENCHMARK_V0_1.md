# Multi-Utility Urban DAS Benchmark v0.1

**Workstream:** MULTIUTILITY-URBAN-DAS-QUAL-001 (#5033)  
**Programme:** SUF #4910  
**Status:** research protocol and synthetic campaign plan only  
**Date:** 2026-11-13

## 1. Objective

Evaluate whether fibre already embedded in or routed near urban infrastructure can support several independent services without conflating their physical meaning:

1. hydraulically active sewer anomaly screening;
2. pipe structural-condition screening under declared excitation;
3. surface traffic trajectory/speed observation;
4. subsurface geophysical imaging;
5. cross-modal evidence fusion and follow-up inspection.

The research opportunity is **shared infrastructure, separated claims**. Source identity, timebase, fibre/route calibration, custody and evidence replay can be shared. The observable, model, truth source, metric and operational decision for each use case must remain distinct.

## 2. What the 2026 literature establishes—and does not

### 2.1 Triple-duty urban sewer deployment

Song et al. (2026), *Triple-duty distributed acoustic sensing in urban environments: Concurrent subsurface imaging, pipeline diagnostics, and traffic surveillance*, Journal of Applied Geophysics 246, 106117, DOI 10.1016/j.jappgeo.2026.106117.

The article describes fibre deployed within an urban sewer pipeline and reports:
- a sensing scheme intended to support subsurface imaging, pipeline blockage diagnosis and vehicle trajectory/speed estimation;
- Hilbert attributes and U-Net for traffic analysis;
- Ningbo Fenghua testing;
- inversion of subsurface anomalies up to 80 m in its study;
- CCTV verification for reported blockage candidates.

This is an important multi-use deployment concept. It is not universal proof that sewer fibre sees all pipe defects or that traffic, geophysical and sewer diagnostic accuracy are interchangeable. Coupling and route/as-built uncertainty remain material.

### 2.2 Sewer inflow and exfiltration proof-of-principle

*Proof-of-principle of Distributed Acoustic Sensing (DAS) for detection of sewer defects, extraneous water intrusion and sewage exfiltration*, Water Science & Technology 93(11):1674–1690 (2026), DOI 10.2166/wst.2026.291.

The study reports:
- a 5 m above-ground PVC laboratory sewer model with controlled intrusion and exfiltration scenarios;
- intrusion transients with localized responses under the studied flows;
- exfiltration localized using amplified 20–30 Hz PSD in the controlled study;
- an 800 m operational sewer field screen where recurrent mid-level PSD activity identified two candidate defective segments later confirmed by CCTV.

The authors explicitly limit the result: field evidence is preliminary, does not establish general precision/recall or broad applicability, and does not uniquely determine the field anomaly mechanism. A structural defect can be hydraulically silent; without an active water exchange, DAS may not observe it. Pipe material, burial, fluid–structure coupling, flow and urban noise remain open variables.

**Important:** the 20–30 Hz PSD band is a paper-specific feature under its acquisition/configuration and population. It must not become a universal threshold or fixed production rule.

### 2.3 Concrete pipe defects using active impact

*Non-intrusive detection of concrete pipe defects using active impact excitation and fiber-optic distributed acoustic sensing*, Automation in Construction (2026), DOI 10.1016/j.autcon.2025.106544.

The method uses a vehicle-mounted Falling Weight Deflectometer (VFWD) to apply external impact excitation and fibre installed inside buried pipes to measure resulting vibration. The paper reports field validation on 12 real drainage pipes, totaling 402 m, with 88.9% precision and 85.5% recall in that study.

This is an active-excitation structural screening method, not passive continuous sewer leak detection. Its small field population, impact geometry, pipe/fibre installation and data-access limits must be retained in any comparison. Data are reported as available on request.

### 2.4 Superlet traffic monitoring

Sun et al. (2026), *Superlet-enhanced distributed acoustic sensing for real-time traffic volume monitoring and vehicle classification*, Optics & Laser Technology 201, 115310, DOI 10.1016/j.optlastec.2026.115310.

The article reports, on its own real-world traffic dataset:
- phase-consistent superlet preprocessing to enhance 10–80 Hz vehicle signatures;
- trajectory segmentation IoU of 68.75%;
- vehicle classification accuracy of 93.31%;
- less than 50 ms inference latency per sample.

These are traffic trajectory/classification metrics. They do not measure sewer defect POD, pipe structural-condition accuracy, route geolocation error or utility asset attribution.

## 3. Separate evidence products

Maintain independent output products and claims.

| Product | Target observable | Independent truth | Primary metrics |
|---|---|---|---|
| H — Hydraulic anomaly | Flow/impact signatures associated with intrusion, exfiltration, blockage or surcharge | Flow/level sensors, controlled hydraulic rig, CCTV and owner adjudication | POD, false alarms per observable sewer-km-day, activation-conditioned recall, segment error |
| P — Pipe structural screening | Response to declared external/controlled excitation or structural condition | Known test-rig defects, pipe coupons/segments, inspection and physical verification | Precision/recall by defect/severity, localization error, repeatability |
| T — Traffic | Vehicle trajectories, speed, type | Independent video/GNSS/loop or approved reference | IoU/mIoU, Dice, track F1, speed/classification error, latency |
| G — Geophysics | Subsurface velocity/attenuation anomaly | Independent geophysical survey, borehole or known target | inversion error/resolution, depth sensitivity, uncertainty coverage |
| F — Fusion | Cross-modal consistency and investigation prioritization | Separate independently adjudicated evidence from each modality | added decision value, confirmed-action ratio, false-dispatch burden |

A common source artifact or fibre does not make the outputs physically equivalent. No product is allowed to use another product's score as its own evidence.

## 4. Benchmark families

### H. Hydraulic sewer anomaly screening

Stratify by:
- intrusion/infiltration, exfiltration, blockage/partial blockage, manhole inflow, surcharge/backwater and confounding hydraulic events;
- active versus inactive/silent defect state;
- pipe material, diameter, joint type, lining and installation;
- dry/wet groundwater and rainfall conditions;
- background flow rate, turbulence, pump state and diurnal time;
- depth, bedding, backfill, soil and fibre coupling;
- traffic/footstep/maintenance and other urban noise.

Metrics:
- event POD and 95% interval by event mechanism and minimum magnitude;
- false alarms per independently observable sewer-km-day;
- candidate-segment precision/recall and localization error;
- false-negative rate on structurally defective but hydraulically inactive periods;
- detection latency, blind/degraded duration and abstention;
- CCTV/hydraulic confirmation rate with denominator and adjudicator.

The primary negative control is a structural defect with no active water exchange. It must not be counted as an expected DAS-positive event; instead, it measures the modality's intrinsic observability boundary.

### P. Structural pipe screening under active excitation

Stratify by:
- concrete, reinforced concrete, PVC, clay and other material;
- crack/joint/void/deformation/wall-condition analogue and severity;
- pipe diameter, burial depth, backfill, groundwater and support condition;
- fibre inside pipe, attached to wall, in nearby duct or external coupling;
- impact energy, location, footprint, repeatability and surface-to-pipe transfer;
- water level and fluid loading.

Metrics:
- precision/recall and confidence intervals per defect class/severity;
- detection probability versus excitation energy and geometry;
- segment localization and uncertainty;
- repeatability across repeated impacts and different VFWD/impact paths;
- false positives from joints, supports, bends and ordinary heterogeneity;
- crew/road-closure time and energy per tested metre.

Active impact work is restricted to approved non-live testbeds or specifically authorized methods. Do not induce damage or impact operational live pipes without an explicit safe engineering method.

### T. Traffic observation

Stratify by:
- road layout, lanes, crossings and road/duct divergence;
- vehicle type, speed, direction and multiple overlapping trajectories;
- low SNR, wet road, rain, background activity, occlusion and timebase shift;
- sewer route alignment relative to road centreline;
- sample pitch, gauge length, interrogator and route calibration.

Metrics:
- segmentation IoU/mIoU and Dice;
- trajectory/track association F1 and false-connection rate;
- speed/vehicle classification only with independent truth;
- end-to-end latency, energy and resource use;
- track confidence/abstention and held-out road/site transfer.

A traffic trajectory along a road does not prove a sewer/pipe defect or exact underground utility geometry.

### G. Subsurface imaging

Stratify by:
- target depth and lateral size;
- soil, groundwater, fill, void and material contrasts;
- sewer/fibre alignment and sensor–target offset;
- traffic/ambient noise regime and coupling;
- independent geophysical control and known test targets.

Metrics:
- depth and lateral localization error;
- anomaly detection sensitivity/specificity by target size;
- resolution as a tested two-target separation property;
- uncertainty coverage and false anomaly rate;
- survey effort per accepted subsurface anomaly.

An inversion anomaly is not automatically a utility asset. Asset attribution requires owner-authorized independent identity evidence.

### F. Shared-fibre economics and fusion

Compare:
- single-purpose DAS deployment;
- shared sewer-mounted fibre with multiple independent products;
- spare/dark fibre reuse;
- point hydraulic/level/pressure sensors;
- CCTV-only and scheduled inspection;
- hybrid DAS + DTS + flow/level + CCTV/geophysics;
- no-added-sensing conventional practice.

Metrics:
- cost per qualified observable sewer-km-year by product;
- incremental cost and incremental decision value of each additional product;
- cost per independently accepted actionable defect;
- fibre/interrogator/storage/compute and maintenance capacity;
- common-cause failure, service outage and rights/access dependency;
- human review and false-dispatch burden.

A shared fibre is economically beneficial only if the marginal value of additional products exceeds added acquisition, processing, retention, calibration, rights, access and false-positive burden.

## 5. Fair comparison and leakage controls

- Keep each paper's task, population, acquisition configuration and metric definition intact.
- Reproduce a published method before changing the preprocessing, classifier, thresholds or split.
- Split by route/site, physical test specimen, acquisition day/session and physical event—not overlapping windows.
- Windows, crops, augmentations and repeated impact measurements of one physical defect/event remain grouped.
- Hold out entire sites/pipe materials or routes for transfer claims.
- Survey/control points used for inversion or route registration cannot also be reported as blind evaluation truth.
- Preserve raw source bytes and exact transform lineage.
- Report no-event exposure and observability separately from event trials.
- Report field-screened candidates and CCTV-confirmed defects separately; candidate count is not confirmed true positives.
- Do not compare H/P/T/G metrics on one aggregate leaderboard.

## 6. Safety and data rights

- No intentional leakage, blockage, flooding, exfiltration, structural damage or pipe failure on live infrastructure.
- Controlled hydraulic scenarios only on contained laboratory/mock or decommissioned facilities.
- Active impact/road excitation only on an approved non-live testbed or under an explicit owner/road-authority engineering plan.
- Confined-space access follows the applicable permit, qualified crew, gas monitoring, rescue and ventilation procedures. DAS does not make access safe.
- CCTV, vehicle video, GNSS and route data have purpose-bound data rights, retention and disclosure controls.
- Exact critical utility geometry stays restricted.
- Sensor intake and APIs remain read-only relative to SCADA/protection/utility control.
- Field claims require asset-owner acceptance and independent adjudication.

## 7. Proposed targets (not achieved results)

- No high-confidence sewer defect claim from an inactive/silent-defect period without an observable flow/acoustic mechanism.
- 100% of in-scope segment/time cells have observable, degraded, unobserved, unavailable or unknown states.
- Report hydraulic POD and structural-screening recall separately, with independent trial IDs and confidence intervals.
- Report traffic IoU/track F1 and geophysical inversion error on separate task leaderboards.
- Hold out at least one complete pipe material/site/route for transfer claims.
- A shared-fibre deployment must demonstrate incremental value for each product against a single-purpose/point-sensor/conventional baseline; otherwise drop the unprofitable product.
- No active-excitation result may be presented as passive continuous-monitoring performance.
- No candidate segment becomes a confirmed defect without independent post-screening verification.

## 8. References

- Song et al. (2026), DOI 10.1016/j.jappgeo.2026.106117: https://doi.org/10.1016/j.jappgeo.2026.106117
- Sewer intrusion/exfiltration proof-of-principle (2026), DOI 10.2166/wst.2026.291: https://doi.org/10.2166/wst.2026.291
- Concrete pipe active-impact DAS (2026), DOI 10.1016/j.autcon.2025.106544: https://doi.org/10.1016/j.autcon.2025.106544
- Superlet traffic DAS (2026), DOI 10.1016/j.optlastec.2026.115310: https://doi.org/10.1016/j.optlastec.2026.115310

## 9. Claim ceiling

This is a literature synthesis and benchmark protocol. It does not establish a working multi-utility detector, field-qualified service, universal threshold, sewer defect POD, pipe structural accuracy, subsurface imaging accuracy, cost saving or permission for physical testing.
