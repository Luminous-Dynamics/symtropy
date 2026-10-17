# Forest Sensor Footprint and Visibility Coverage v0.1

**Status:** offline synthetic software benchmark. Not field-performance evidence, a physical sensor model, or a safety certification.

## Why this tranche matters

A robot that drives through a forest has not necessarily observed the space around it. A radius/disk metric can claim coverage of a point behind a trunk or log pile merely because the point is near the robot. This tranche changes the measured output from route proximity to conservative, pose-conditioned visibility.

Recent robotics research describes the same failure mode in stop-and-scan LiDAR planning: a disk rule can credit cells behind occluders, while ray-cast line-of-sight coverage changes viewpoint choices and observed coverage. A 2026 study reports higher line-of-sight coverage than its disk baseline in both controlled simulation and an indoor hardware experiment. Its results motivate an ablation for this project; its indoor numbers are not forestry performance claims. Source: https://www.mdpi.com/1424-8220/26/17/5430.

Active perception research also studies selecting viewpoints based on unknown-space reduction and target-specific confidence, reinforcing that viewpoint value should be part of mission optimization rather than an afterthought: https://arxiv.org/abs/2603.23112.

## v0.1 model

- **Map:** deterministic 9×9, square 1 m synthetic cells; map seed and exact serialized map are bound to the receipt digest.
- **Sensor:** three-cell Euclidean range, fixed 180° horizontal field of view.
- **Pose heading:** derived from the incoming grid step; continuous rotation and turning cost are not modelled.
- **Visibility:** integer grid line-of-sight with conservative diagonal corner checks. Known obstacles and unknown cells block rays beyond them. A directly visible obstacle/unknown cell may count as sensor-exposed, but does not imply traversability or confirmed free space.
- **Coverage:** the union of cells within each traversed pose's field of view and line of sight. Receipt reports planned_area_cells, observed_area_cells, unobserved_area_cells, and observed_coverage_basis_points, separately from survey target planned/traversed/verified counters.
- **Failure accounting:** a fault at simulation step k prevents pose k from contributing coverage; only prior traversed poses count.
- **Determinism:** fixed sensor config is part of the configuration digest. Faults, maps, source SHA, corpus SHA, and outcomes are included in the receipt.

## Frozen regression tests

In addition to the 15 mission/fault scenarios, 16 Rust tests cover baseline state/reason codes, deterministic digests, energy reserve, fault timing, and the coverage extension. New visibility-specific negatives check:
1. a baseline reports partial coverage of an 81-cell area;
2. cells behind the forward-facing sensor are not counted;
3. an occluder stops the ray even where a disk baseline would credit the cell;
4. an unknown intermediate cell blocks visibility farther along the ray, while the unknown cell itself can be directly targeted;
5. fault interruption reduces observed coverage and does not let the plan claim target verification.

## Proposed benchmark gates

These are engineering targets, not standards or achieved results.

| Metric | Target | Protocol |
|---|---|---|
| Occlusion false-credit rate | **0** | Every cell behind a blocking obstacle/unknown cell receives no line-of-sight coverage from that pose |
| Determinism | **100% exact repeatability** | Same map/config/route/corpus/source yields byte-identical semantic receipt fields and digests |
| Coverage integrity | **0 planned-only credit** | Fault-injected routes receive coverage only from poses traversed before the fault |
| Visibility recall | Report median, p90 and per-scene distribution | Compare predicted visible cells with independent ray-traced ground truth across map/pose combinations |
| Baseline ablation | Report disk vs FOV-only vs ray-cast visibility | Same maps, poses, range and seeds; measure false-positive coverage and number of additional viewpoints |
| Mission efficiency | Verified coverage gain per meter, joule and intervention | Never compare against a model that is allowed to count occluded or unknown space as verified |
| Tree/object inventory | Object detection precision/recall; DBH bias, median absolute error, p90/p95 | Use independent plot ground truth; stratify by diameter, species, occlusion, season, sensor, and terrain |
| Ecological impact | Ground disturbance and missed-hazard rate | Matched field plots and human-baseline runs, after safe supervised hardware validation |

## Important limits and next steps

The simulator presently uses a 2D cell map and perfect deterministic ray sensing. It cannot model layered canopy occlusion, branch geometry, volumetric LiDAR returns, RGB appearance, fog/rain, leaf-on/leaf-off changes, trunk shape, pose uncertainty, or probabilistic classification. It is an explicit test harness for a coverage contract, not a predictive virtual forest.

Next steps:
1. add pose-uncertainty envelopes so coverage is credited only under a stated conservative bound;
2. add per-sensor modalities and detection/no-detection confusion matrices measured from real sensor trials;
3. add tree/log target geometry and object completeness rather than using cells as a proxy for objects;
4. compare deterministic next-best-view strategies on held-out synthetic scenes with identical starting poses and a fixed energy budget;
5. validate against independent point clouds and field plots before setting production accuracy thresholds.

Physical forestry machinery safety remains a separate engineering gate. ISO lists ISO 11850:2011 with amendments as published/current after review in 2023, while ISO/CD 11850 Edition 4 is under development and will replace it. Neither this simulator nor that status statement claims project compliance: https://www.iso.org/standard/51182.html and https://www.iso.org/standard/87180.html.
