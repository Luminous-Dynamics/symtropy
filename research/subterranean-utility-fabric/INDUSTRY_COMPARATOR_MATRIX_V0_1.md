# SUF Industry Comparator Matrix v0.1

**Snapshot:** 2026-11-12  
**Owner:** Symtropy #4910  
**Purpose:** Profile-matched commercial and standards baselines for the Subterranean Utility Fabric programme.

## 1. Positioning rule

The market already contains mature and emerging capabilities in autonomous sewer inspection, multi-sensor pipe condition assessment, power-tunnel patrol, fibre cable installation, pipeline asset management and survey standards.

Luminous must not claim invention of those categories. The research opportunity is to integrate existing machine/tool families into a verified, vendor-neutral infrastructure lifecycle: survey evidence → method eligibility → authorized work → inspection/construction → as-built evidence → independent acceptance → maintenance economics.

The comparator matrix separates:

- **manufacturer claims** from independently measured results;
- **commercial deployment** from research prototypes/pilots;
- **sewer, water pipe, cable tunnel, utility tunnel and microduct** populations;
- **inspection** from installation, localization, condition classification and actual repair;
- **software/workflow capability** from physical qualification.

Public pricing was not found for the referenced industrial products; no price comparison is invented.

## 2. Commercial and institutional baselines

### C1 — RedZone SOLO small-diameter sewer crawler

Manufacturer page: https://redzone.com/our-technology/solo/

Published manufacturer description:
- tracked autonomous inspection robot for 8–12 inch (200–300 mm) sewer pipe;
- front/rear panoramic imaging stitched into 360° footage;
- autonomous traversal with return to the launch point;
- 10–15 minute typical deployment;
- vendor-reported 4,500 ft/day per crew when operating multiple units;
- NASSCO PACP/MACP, WRc and WSSA deliverables.

**Fair SUF comparison:** use only on a matching physical pipe profile and inspection task. Compare route reachability, visible-surface coverage, defect recall, chainage accuracy, setup/cleaning/retrieval time, operator effort, accepted inspection reports and cost per accepted metre. Do not compare SOLO directly with small cable microduct tools or assume its sewer defect coding transfers to cable ducts.

The 4,500 ft/day and throughput claims are vendor-stated crew-level results; they are not independently measured Luminous benchmarks.

### C2 — RedZone multi-sensor pipe inspection platforms

Manufacturer portfolio: https://redzone.com/our-technology/

Published portfolio includes platforms using combinations of laser profiling, sonar and CCTV; the MDProfiler family is listed for approximately 20–36 inch (500–915 mm) pipes, with long-distance inspection configurations. The product family is a useful commercial baseline for quantitative multi-sensor pipe condition assessment.

**Fair SUF comparison:** match bore diameter, flow state, sensor modality, deployment length and defect class. Compare dimensional metrology, sensor coverage, localization, class-specific precision/recall, calibration, retrieval and report acceptance. No vendor marketing description is a substitute for a matched independent test.

### C3 — commercial power-tunnel quadruped inspection

DEEP Robotics reports that its X30 quadruped, adapted as “SPock” for SP Group in Singapore, has been used in sections of a 40 km underground transmission-tunnel network. The reported configuration uses visual and thermal sensing to inspect for conditions such as cracks and water seepage.

Source: https://www.deeprobotics.us/news/deep-robotics-x30-robot-dog-leads-smart-inspection-in-singapores-power-cable-tunnels/

**Fair SUF comparison:** compare route mission completion, thermal/visual detection by class, false alarms, signal-denied localization, navigation through access constraints, mission uptime, retrieval, safety-stop behavior, time-to-review and whether findings become owner-accepted work orders. Do not claim that a general quadruped can traverse small ducts.

### C4 — rail-mounted cable/utility-tunnel inspection

Coolight advertises the CL803D dual-rail inspection platform for cable and utility tunnel corridors. Its product page lists IP66 protection, modular sensor payloads, IR temperature, gas and water-leak monitoring, obstacle avoidance and offline autonomous patrol.

Manufacturer page: https://sdkulai.com/en/tdlasjiguangguangpushiyanpingtaixilie/322.html

**Fair SUF comparison:** include rail installation/maintenance cost, coverage, thermal calibration, gas/water alarm detection, availability, obstacle handling, independent recovery, false-alarm rates, integration with owner systems and whether the rail constrains or enables future maintenance. All listed performance properties remain manufacturer claims until independently tested.

### C5 — fibre cable blowing / jetting machinery

Air-assisted fibre installation is an established commercial process. It is a critical comparator because a robot should not replace a specialized installation machine merely to satisfy a “robotic” product narrative.

References:
- FOA underground installation: https://www.thefoa.org/tech/ref/OSP_Construction/Underground_Installation.html
- OFS microcable installation practice IP-055: https://www.ofsoptics.com/wp-content/uploads/IP055-MicroCable-Installation-Oct-2019.pdf

**Fair SUF comparison:** the robot/toolchain should prove and inspect the duct, locate obstruction, record route evidence and recover tools. The qualified jetting/blowing machine may perform cable installation. Measure cable-specific push/pull force, duct/cable condition, bend compliance, installation acceptance, damage, downtime and complete setup/cleaning/operator burden. Manufacturer force limits are product-specific.

### C6 — pipeline asset-management and reporting software

RedZone Integrity is a vendor example of an inspection data platform using GIS-based mapping, condition reports and sewer asset assessment workflows.

Source: https://redzone.com/our-technology/

**Fair SUF comparison:** compare data lineage, source-quality distinctions, confidence/unknown states, geospatial interoperability, asset revision handling, cross-vendor ingestion, independent acceptance evidence, exportability and ownership boundaries. Do not claim that commercial systems lack GIS or condition reporting.

### C7 — industry process standards

| Reference | Scope / benchmark role | Limitation |
|---|---|---|
| NASSCO PACP/MACP | Established sewer CCTV condition coding and reporting workflow | Sewer-specific; human-certified/adjudicated scope; not cable-duct qualification |
| PAS 128:2022 | Underground utility detection, verification and location quality framework | UK PAS; not South African law; does not replace local statutory/owner requirements |
| IEC 63439-1-1:2025 | Electric-power robot terminology | Vocabulary/definitions, not proof of a specific robot's performance |
| IEC TR 63439-1-2:2026 | State-of-the-art and standardization roadmap for power-system robots, including validation and reliability | Technical report/roadmap, not a universal product certification |
| IEC 63439-3-31 work item | Land cable-tunnel inspection robot requirements in development | Not a published compliance standard in this snapshot; do not assume microduct scope |
| OGC MUDDI Part 1 v1.0 | Conceptual model for underground asset, space, event and observation interoperability | Conceptual model; a logical profile and encoding/conformance evidence are still needed |

Sources:
- NASSCO: https://www.nassco.org/resource/cctv-inspection-quality-in-sewer-cleaning
- PAS 128:2022: https://accord-checkout.bsigroup.com/products/pas-128-underground-utility-detection-verification-and-location-specification
- IEC TR 63439-1-2:2026: https://webstore.iec.ch/en/publication/109098
- IEC 63439-3-31 work item: https://standardsdevelopment.bsigroup.com/projects/9025-12660
- OGC MUDDI: https://docs.ogc.org/is/23-024/23-024.html

## 3. What the industry already does well

- Small-pipe sewer inspection with fast deployments and autonomous return.
- Multi-sensor quantitative pipeline condition assessment (CCTV, laser, sonar).
- Visual/thermal patrol in large, walkable power tunnels using quadruped robots.
- Fixed rail-based inspection in utility corridors.
- Specialized fibre blowing/jetting rather than general-purpose robot installation.
- Mature inspection coding and GIS-linked asset reporting in sewer workflows.
- Emerging power-robot terminology and performance/reliability standardization.

## 4. Where Luminous should compete

Do **not** set the goal as “build a better crawler than every vendor.” Compete on cross-workflow and lifecycle performance:

1. **Vendor-neutral orchestration:** choose the simplest eligible tool for a declared task and profile; use existing crawlers, quadrupeds, rail robots, inspection heads, mandrels, winches and jetting machines where appropriate.
2. **Evidence-qualified spatial context:** every observation binds route, geometry, coordinate frame, uncertainty, source, calibration and time. A legacy drawing never silently becomes a verified as-built.
3. **Safe unknown handling:** stale, conflicting, unobserved, out-of-envelope and unsupported evidence cannot be promoted to clear.
4. **Independent acceptance:** robot dispatch, alarm generation, footage capture and contractor invoice are not “accepted service.” Acceptance needs a separate owner-qualified observation/receipt.
5. **Robot-first maintainability:** access nodes, duct geometry, draw paths, retrieval and replaceable modules are designed and tested for future maintenance.
6. **Lifecycle service economics:** evaluate accepted service, downtime, avoided excavation, local operator hours and repeat work. Do not compare robot travel speed against manual labor while excluding setup, retrieval, cleaning and review.

## 5. Fair competitor benchmark protocol

For every commercial comparison:
- freeze exact vendor model, firmware, payload, calibration, declared envelope and documentation revision;
- match duct/pipe/tunnel geometry, profile, occupancy, water/debris state, distance and access conditions;
- define identical seeded defects/obstructions and independently measured ground truth;
- include setup, travel, inspection, cleaning, charging, retrieval, maintenance, human review and failed/aborted runs;
- preserve vendor-stated claims separately from independent measurements;
- compare complete accepted workflows, not headline robot speed;
- record data export format, proprietary dependencies, owner acceptance process, service contract, spare-parts availability and switching cost.

Report separate measures: coverage, detection, dimensional error, localization P50/P95/max, false alarm, false negative, safe retrieval, mission availability, accepted metres/hour, operator minutes/accepted metre, energy/accepted metre, damage, first-pass acceptance, rework, service loss and whole-life cost. No single score should conceal a profile-specific failure.

## 6. Proposed Luminous competitive targets

These are research goals, not achieved results:
- Integrate at least three distinct vendor/tool classes through versioned, vendor-neutral evidence interfaces without making any one manufacturer mandatory.
- Zero false clearances on the frozen synthetic hostile corpus.
- 100% traceability for counted pilot assets and independently accepted repairs.
- No unauthorized disclosure of restricted synthetic asset geometry in the finite access-control corpus.
- At least 25% lower active-sensing dwell time at non-inferior localization task performance versus a fixed-cadence reference, once matched tests exist.
- Demonstrate lower total accepted-work cost or better service availability than the best eligible local workflow in at least one real site stratum before claiming economic superiority.
- Build profile-specific competitor reports for microduct, 200–300 mm sewer-like pipe, 500–915 mm multi-sensor pipe and walkable utility/cable tunnel. No scores transfer between them.

## 7. Claim ceiling

This matrix is a research inventory. Product specifications and performance statements are vendor- or project-reported unless explicitly marked otherwise. It contains no independent Luminous product test, pricing, field pilot or verified superiority result. Product specifications can change; recheck exact revisions and procure current datasheets before procurement or formal claims.
