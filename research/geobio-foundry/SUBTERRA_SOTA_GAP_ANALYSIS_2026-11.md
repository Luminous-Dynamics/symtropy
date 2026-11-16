# Subterra — Planetary Subterranean SOTA and Competitive Gap Analysis

**Cut-off:** 2026-11-16  
**Status:** Focused literature/industry scan, not a systematic review  
**Purpose:** ground Subterra in current planetary cave science and subterranean robotics; define defensible differentiation.

## 1. Executive conclusion

Planetary subterranean exploration is an active research field. Existing work already covers orbital cave-candidate catalogues, lunar pit radar interpretation, Mars multi-sensor candidate characterization, Venus radar-based conduit inference, terrestrial cave morphology, autonomous robot platforms, multirobot communications, LiDAR/IMU/SLAM and line-of-sight planning.

Subterra must not claim to invent planetary cave detection, cave robotics, autonomous mapping, multirobot relay networks, lava-tube habitat concepts or digital twins.

The strongest opportunity is a **planetary subterranean evidence-to-operation framework** that maintains separate confidence in:
- visible morphology;
- geological origin;
- subsurface connectivity;
- robotic accessibility;
- surveyed interior extent;
- structural/environmental engineering suitability;
- use-specific suitability and planetary-protection permissions.

It should compile this evidence into exploration plans, robot task chains, resource/engineering constraints and independently reproducible qualification records. It should explicitly abstain where evidence is missing.

## 2. Scientific baselines by planetary body

### 2.1 Moon — pit access and conduit evidence

Carrer et al. (Nature Astronomy, published 15 July 2024) analysed Lunar Reconnaissance Orbiter Mini-RF data for the Mare Tranquillitatis Pit. Their radar interpretation attributes part of the reflected signal to an accessible subsurface cave conduit extending tens of metres from the pit. This is an important advance beyond morphology-only cave candidates, but it is not a full survey of the connected network or a structural/habitation qualification.

Source: https://www.nature.com/articles/s41550-024-02302-y

The 2026 LunarLeaper concept proposes a roughly 15 kg-class legged robot to investigate lunar volcanic pits using geophysical instruments and high-resolution imaging. It is a mission concept, not a flown capability.

Sources:
- https://www.sciencedirect.com/science/article/pii/S0094576525008082
- https://arxiv.org/abs/2609.11453

A November 2026 Acta Astronautica study examines line-of-sight as a mission design variable for lunar pit/cave exploration, using viewshed analysis to optimize landing/observation locations and pit-wall visibility at Marius Hills. This indicates that communications and observation geometry are already active baselines.

Source: https://doi.org/10.1016/j.actaastro.2026.05.054

**Subterra extension:** jointly optimize rim/base placement, descent, robot geometry, relay deployment, retreat, scientific coverage and uncertainty reduction. Do not treat viewshed or surface mapping alone as proof of cave connectivity.

### 2.2 Mars — multi-sensor candidate interpretation

Sharma et al. (npj Space Exploration, 12 June 2026) investigate a potential cave candidate in the Elysium Mons region using CTX/HiRISE morphology, illumination, THEMIS nighttime thermal observations, MOLA topography and regional compositional context. The paper proposes legged robotic mobility, LiDAR mapping and tethered descent as an exploration architecture.

Source: https://www.nature.com/articles/s44453-026-00039-8

The important methodological point is the use of complementary instruments. The observations support a potential cave interpretation; the paper does not demonstrate a robot entering and mapping that cave.

**Subterra extension:** preserve each sensor's native resolution, observation geometry and uncertainty; avoid treating multiple products derived from the same source as independent confirmation. Separately score morphology, inferred origin, access probability and scientific value.

### 2.3 Venus — radar-inferred large pyroduct

Carrer, Diana and Bruzzone (Nature Communications, 9 February 2026) apply SAR analysis to Magellan radar imagery around a localized collapse in the Nyx Mons region. They hypothesize a subsurface pyroduct with an estimated diameter of about 1 km, roof thickness of at least 150 m, void height of at least 375 m, and lateral continuation of at least 300 m.

Source: https://www.nature.com/articles/s41467-026-68643-6

These are radar-derived inferred dimensions, not direct interior measurements or a confirmed engineering site. Venus's surface pressure, temperature and atmospheric chemistry make its infrastructure envelope radically different from the Moon or Mars.

**Subterra extension:** use Venus as a remote-sensing and geological-model transfer case first; no near-term habitation or biological-industry claim.

### 2.4 Earth analogues — non-uniform geometry

A 7 November 2026 Journal of Geophysical Research: Planets article, “Quantitative Morphometry of the Ape Cave Lava Tube”, reports a 3.4 km terrestrial lava-tube model and more than 1,400 cross-sectional slices, with five recurring profile classes. The authors argue for empirical morphology over a uniform-cylinder assumption.

Source: https://doi.org/10.1029/2026JE010115

The study supports a morphology benchmark using longitudinal profiles, cross-section classes, branching, chamber transitions and bottlenecks. Earth cave geometry can inform model variety, but Earth rock strength, gravity, weathering and hydrology do not directly transfer to lunar, Martian or Venusian conditions.

### 2.5 Icy moons, Mercury and small bodies

NASA's planetary-protection guidance identifies Mars, Europa, Ganymede, Callisto, Titan and Enceladus as bodies with stringent or special protection concerns due to their potential for astrobiological investigation.

Source: https://sma.nasa.gov/sma-disciplines/planetary-protection/

Subterra should not treat an icy moon's subsurface ocean as an accessible cave. Fractured ice, crevasses, basal conduits and ocean access are distinct scientific/engineering problems. Europa and Enceladus should be preservation-first exploration targets, not industrial biofactory targets.

Mercury and Titan are included as hypothesis/technology-transfer domains until stronger site-specific evidence supports a concrete accessible-void programme. Asteroid porosity and rubble-pile voids are separate microgravity contact/anchoring problems rather than cave equivalents.

## 3. Robotics and sensing state of the art

### 3.1 Planetary cave robotics review (2026)

Swinton et al., “The Evolution of Autonomous Systems for Planetary Cave Exploration: A Review”, Journal of Field Robotics, first published 10 May 2026.

Source: https://doi.org/10.1002/rob.70233

Key findings relevant to Subterra:
- low-light environments weaken conventional visual odometry and stereo-only mapping;
- LiDAR, IMU and sensor-fusion SLAM are important navigation candidates;
- robot teams can provide complementary sensing and reduce single-robot mission risk;
- communication strategies include breadcrumb relays, robot mesh, tethered/fiber backhaul, store-and-forward and communication-aware droppable nodes;
- existing terrestrial cave deployments include legged/aerial teams and quadruped pairs in Martian lava-tube analogues;
- the review identifies the gap between surface exploration capability and direct subterranean exploration experience.

The review describes an underground mine test with a quadruped/aerial pair, where the ground robot operated 37 minutes and the aerial robot 8 minutes in a 45-minute exploration. It also reviews paired quadruped cave experiments and other multirobot deployments.

**Competitive consequence:** Subterra cannot claim LiDAR-SLAM, tethered descent, robot mesh, breadcrumbs or heterogeneous teams as new. The integration challenge is their evidence-aware orchestration across different geological and planetary envelopes.

### 3.2 DARPA Subterranean Challenge

The 2018–2021 DARPA SubT programme remains a major terrestrial benchmark for robot operation in complex underground environments. Its analysis shows that high-coverage maps do not necessarily translate into complete detection, localization and reporting of mission-relevant artifacts.

Source: https://doi.org/10.1146/annurev-control-062722-100728

**Subterra extension:** score semantic discovery, localization, hazard interpretation, scientific evidence quality, data exfiltration, retreat and recovery—not only point-cloud coverage or distance traversed.

### 3.3 SCOUT and field analogues

CSIRO and DLR have used the SCOUT rover in a Lanzarote lava tube as an analogue for autonomous cave exploration and mapping. This is a useful terrestrial hardware-validation baseline.

Source: https://www.csiro.au/en/news/All/Articles/2025/January/Exploring-lava-tubes

## 4. Competitive comparison

| Existing frontier | Demonstrated/active baseline | Subterra's testable extension |
|---|---|---|
| Orbital cave candidate search | Multi-instrument morphology, thermal, topography and radar interpretation | Evidence graph with native-scale uncertainty, alternative origin hypotheses, correlated-source detection and abstention |
| Lunar pit exploration | LunarLeaper concept, rim viewshed/line-of-sight analysis, Mini-RF conduit interpretation | Joint scientific value, landing/access, tether/anchor, relay, retreat and evidence-collection planning |
| Mars cave reconnaissance | Multi-sensor candidate study, legged/tethered robot concept | Candidate-to-direct-connectivity evidence progression and blind cross-site transfer |
| Cave geometry | High-resolution 3D surveys and 1,400+ Ape Cave sections | Profile-aware data model, coverage-aware morphometry, bottleneck and branch characterization with unknown intervals explicit |
| Robot navigation | LiDAR/IMU/SLAM, legged, tracked, aerial and tethered robots | Multi-robot task/energy/communication/retreat co-planning with independent localization checks |
| Cave communications | Breadcrumb, mesh, tether/fiber and store-and-forward strategies | Communication-aware exploration with mission-evidence delivery guarantees and relay-resource accounting |
| Subterranean autonomy | DARPA SubT, terrestrial mine/cave deployments | Semantic situational awareness and recoverability, not just mapped area |
| Habitat concepts | Natural shielding and engineered habitat envelopes | Separate geotechnical, pressure-boundary, thermal, fire and contamination qualifications |
| Planetary science | Targeted caves as geological/astrobiological archives | Preservation-first science sampling and explicit planetary-protection gate |

## 5. Principal research gaps

1. Evidence fusion that preserves source resolution, calibration, correlation and alternative interpretations.
2. Direct mapping of entrance-connected extent and explicit limits on extrapolation beyond surveyed regions.
3. Realistic non-uniform geometry and morphology transfer across planetary bodies.
4. Communication and localization co-design for deep cave networks.
5. Heterogeneous robot teams that can recover, relay, retreat and return interpretable data after partial failures.
6. Semantic site understanding beyond occupancy maps.
7. Joint scientific value-of-information and operational risk planning.
8. Geomechanical and pressure engineering qualified independently from geometry reconstruction.
9. Planetary-protection constraints included in site planning and sampling authorization.
10. A common data/evidence protocol to compare robotic deployments and site interpretations across missions.

## 6. Defensible differentiator

Subterra's research hypothesis is a **subsurface evidence-to-operation compiler**:

`orbital observations → candidate taxonomy → uncertainty-preserving site model → targeted geophysics → access/communication plan → in-situ mapping → science/engineering assessments → independently qualified use-specific capability`

The system should produce a vector of independent claims, not one “site readiness” score. It must be able to conclude that a feature is scientifically valuable but inaccessible, that its morphology is well mapped but origin is uncertain, or that a cave is accessible but not qualified for habitation.

This is a research differentiation hypothesis, not proof of novelty or freedom to operate. A formal prior-art and IP review remains necessary before publication claims.
