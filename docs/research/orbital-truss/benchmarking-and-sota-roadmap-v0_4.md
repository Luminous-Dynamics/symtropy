# Orbital Truss Inspectability — Benchmark and SOTA Roadmap v0.4

**Research date:** 23 November 2026  
**Implementation scope:** synthetic 2D geometry/planning, conditional 3D support-envelope bounds, and planar-triangle support derivation helper.  
**Claim boundary:** no physical inspection qualification, calibrated measurement accuracy, NDE/POD, structural acceptance or flight authorization.

## 1. The paradigm shift: from surface coverage to decision-useful measurability

The correct optimization target is not simply the number of visible points or the percentage of a mesh that a camera can see. An industrial inspection plan must establish a chain of increasingly strong propositions:

1. **Geometric observability:** the candidate ray/support is in range, in FOV and not blocked.
2. **Sampling adequacy:** the measurement rays/points are dense and distributed sufficiently to resolve the required spatial feature.
3. **Feature measurability:** the acquired data constrain the characteristic of interest (hole diameter, axis, slot width, datum plane, truss-member centreline) with adequate geometry.
4. **Photometric / sensor usability:** the surface response and sensor configuration produce valid data rather than shadows, saturation, glare, dropout or poor depth.
5. **Metrological adequacy:** calibrated uncertainty and decision guard bands support the intended dimensional decision.
6. **Execution feasibility:** the view can be physically reached and acquired within robot, energy, collision and recovery constraints.
7. **Independent disposition:** the resulting evidence is identity-bound, reviewed and consumed by an authorized decision process.

These are not interchangeable and should not be collapsed into an opaque scalar score. Report each axis separately; let engineering decisions consume the right evidence level.

## 2. Direct 2026 k-coverage comparator

Bin Khalid and Yeum, *Ground Sample Distance Constrained k-Coverage Viewpoint Planning via Set Multi-Cover Formulation*, arXiv:2610.36450, 27 October 2026:

https://arxiv.org/abs/2610.36450

The method performs voxel-centre visibility under distance, obliquity, FOV and occlusion, adds candidate views to under-covered surface voxels, and uses a set multi-cover objective. Its dynamic greedy weights coverage deficit and the scarcity of remaining candidate views. On its two test structures and configured sweeps, it reports 3.00–16.39% more viewpoints than ILP where an optimum is certified.

The paper expressly describes the voxel-centre geometry as a discrete approximation, not a whole-voxel-surface guarantee. It uses an attainable target k*(v)=min(k, SBW(v)), where SBW is the number of available candidate views for voxel v, and discloses original-k shortfalls. Our #7002 reproduction must preserve both requested-k and attainable-target outcomes rather than misrepresenting this relaxation.

Published experiment settings to match include voxel sizes 0.04/0.06/0.08/0.10m, GSDmax 0.5mm/pixel, GSD setpoint 0.25mm/pixel, k values 1/3/5, overlap thresholds 50%/70%, maximum obliquity 45°, slant range 0.3394m, perpendicular distance 0.2400m, azimuth 0–360° in 30° increments, tilt augmentation 1–90° in 1° increments, and ILP time limit 3600s. Candidate-pool generation, visibility matrix, objective and proof status must match before comparing optimality gaps.

## 3. Direct support-patch comparator

The voxel-centre method is a selection comparator, not a conservative patch certificate. The separate #7136 implementation adds enclosing-ball-based range/FOV/incidence/GSD bounds. At its current increment:
- support-ball and normal-cone flags are explicitly caller-asserted;
- each receipt reports precondition_proofs_verified_independently=false;
- the evaluator does not independently prove that those assertions describe the physical mesh;
- the outputs are named conditional envelope outcomes;
- occlusion remains NOT_ASSESSED;
- no result can be counted as full-support visibility or coverage.

The triangle helper derives a planar triangle's centroid, candidate enclosing radius and geometric face normal from vertices, with a stated floating-point coordinate-scale margin. Outward winding remains caller-asserted. This is not formal interval arithmetic or proof of physical mesh identity.

The next qualification step is to expose provenance for each bound: mesh/face IDs, source digest, vertex coordinates, winding and orientation evidence, derivation algorithm/version, numeric error method, and independent replay. A support proof must not be encoded by a bare true boolean.

## 4. Stronger recent comparator: feature measurability and sampling density

Vanessa Staderini, *Integrated Planning and Optimization Framework for Robotic Visual Inspection*, TU Wien doctoral dissertation (submitted 1 December 2025):

https://repositum.tuwien.at/bitstream/20.500.12708/226537/1/Staderini%20Vanessa%20-%202026%20-%20Integrated%20Planning%20and%20Optimization%20Framework%20for...pdf

This work is particularly relevant because it separates **spatial resolution** from **sampling resolution**. Its chapter 4 introduces a sampling-density matrix for non-uniform mesh elements. For surface element i and candidate viewpoint j, its density is based on the number of camera rays intersecting the element divided by the element's surface area; total density accumulates contributions across selected viewpoints. The resulting set-coverage optimization targets both coverage and a minimum sampling density without remeshing the object.

This is a stronger comparator than binary k-coverage for defect-scale sampling: one ray intersecting a large mesh face is not treated as equivalent to many rays across a small face. The method explicitly reports a resolution target and experiments with reconstructed point-cloud density. It was tested in simulation and physical measurements; the stated sensor setup for one simulation comparison used 2448×2050 pixels, 12.93mm focal length, 140mm focusing distance, 135–155mm depth of field and 75° maximum incidence. A surface-sampling approach used 13 viewpoints for the Hirth coupling versus 15 for a patch-based comparator, and 23 for the crankshaft versus 23 for the patch comparator, with complete-coverage results in nearly all evaluated scenarios. These are thesis-specific results, not transferable claims.

Chapter 5 adds **photometric visibility** using Blinn–Phong and Cook–Torrance BRDF models. In one 100-candidate crankshaft example, geometry-only visibility suggested nine viewpoints for complete coverage, while usable photometric coverage varied materially by surface: aluminum 50.78% (Blinn–Phong) and 37.54% (Cook–Torrance), gold 79.26% and 71.12%, and plastic 91.60% and 99.44%. This demonstrates that a geometrically visible reflective surface may still yield unusable acquisition. Those values are specific to the thesis sensor/material/lighting model.

Chapters 6–7 move from viewpoint selection toward robot configuration, set-coverage generalized TSP, collision/workspace constraints and physical point-cloud acquisition. The thesis uses an UR10e, linear/rotary stages and a Zivid structured-light sensor in its physical transfer tests. It states that algorithmic baseline comparisons were performed in simulation while real-world experiments validated transfer to the physical system; keep these evidence classes separate.

## 5. Feature-specific measurement quality

A second 2026 comparator is Zhang et al., *Feature-oriented automatic viewpoint planning for structured-light 3D measurement*, Optics & Laser Technology, 203, article 116282, DOI 10.1016/j.optlastec.2026.116282:

https://doi.org/10.1016/j.optlastec.2026.116282

The stated contribution is feature measurement quality (FMQ): sufficient depth information along the feature normal and sufficient tangential constraints in the measurement plane. This addresses holes/slots where total visible surface area is not a reliable proxy for the accuracy or conditioning of a fitted diameter, centre or axis.

The relevant Symtropy owner is #7043 / PR #7049, which already has a synthetic circle-measurability contract and an adversarial matched-count example. Do not duplicate it. The next matched comparison should place #6925's k-coverage, #7136's conditional support geometry and #7043's feature-measurability outputs on the same candidate-view pool, with #6846 owning the subsequent metrological uncertainty model.

## 6. Full 3D multi-robot pipeline comparator

Pšenička and Edelkamp, *A GPU-Resident Pipeline for Multi-Robot 3D Inspection with Limited Perception*, arXiv:2610.26106:

https://arxiv.org/abs/2610.26106  
https://github.com/krystofpsenicka/3D-Inspection

This pipeline combines candidate viewpoint generation, GPU visibility, set cover, min-max routing MIP, continuous-time multi-agent path finding, trajectory smoothing and post-smoothing collision/waypoint audits. It reports 14 meshes up to 7.2 million faces. It notes the decomposition sacrifices global optimality and that no shared multi-robot 3D-inspection benchmark exists. Its code environment is GPU/OptiX/RAPIDS-specific and the inspected repository revision had no visible licence file; use it as a benchmark/reproducibility reference, not a source to copy or redistribute.

Existing Symtropy route owners remain #5920 (exact small joint allocation-routing-charging oracle) and #5892 (source-faithful reproduction of a 2026 charging-aware inspection baseline).

## 7. Matched benchmark ladder

| Stage | Question | Comparator | Key metrics |
|---|---|---|---|
| P0 | Does the contract implement its stated rules? | Frozen controls and independent oracle | expected outcomes, false-accepts, deterministic receipts |
| P1 | Are candidate views sufficient? | Paper source-faithful GSD candidate generation | candidate count, per-target available views, requested-k shortfall |
| P2 | How efficient is selection? | Paper greedy, overlap pruning, certified ILP, exact ≤20 oracle | views, lower/upper bound, optimality gap, runtime |
| P3 | Does the whole support fit the sensor envelope? | #7136 conditional ball/normal-cone bounds | certified-condition fraction, indeterminate fraction, bound slack |
| P4 | Is the support actually unoccluded? | Conservative ray/triangle oracle | false-clear, false-block, partial/unknown support |
| P5 | Is the mesh sampled densely enough? | Staderini sampling-density matrix | rays/area, per-element target attainment, local gaps, point-cloud density |
| P6 | Is the acquisition usable? | BRDF/photometric sensor model | saturated, shadowed, under-exposed, depth-invalid fraction |
| P7 | Is the characteristic identifiable? | #7043 FMQ analytic/feature tests | rank, singular spectrum, condition, contour-sector coverage |
| P8 | Is the measurement adequate? | #6846 calibrated uncertainty/guard-band model | covariance, uncertainty, conformance risk |
| P9 | Is it physically executable? | #5920/#5892 route/fleet baselines | path, makespan, collisions, energy, charging, return reserve |
| P10 | Does the observation support a decision? | #6752/#4258 + #6956/#4997 | observation validity, decision regret, repeat measurement value |
| P11 | Is the evidence interoperable and authorized? | AP242/QIF, Mycelix #6390/#6519 | identity loss, provenance, authority chain |
| P12 | Does the method detect actual defects? | Procedure-specific blinded NDE | hit/miss, POD, false-clear, confidence bounds |

## 8. Proposed research targets (not achieved)

- Zero false feasible results on frozen support-envelope controls; all asserted preconditions surfaced in the receipt.
- Zero false-clear support occlusion before full-support visibility can be claimed.
- 100% reproducible requested-k, attainable-target and candidate-pool-shortfall reporting.
- 100% separation of view count, failure-domain diversity, rays/area, photometric usability, feature conditioning and metrological uncertainty.
- Exact solver/oracle agreement on all tractable candidate pools; for larger matched pools report certified ILP gaps and timeout/uncertified cases separately.
- Feature-measurability controls: no rank/conditioning false-accepts in sealed degeneracy cases and no covariance without a declared noise model.
- Physical targets are procedure/instrument/feature-specific; establish baselines before proposing numeric percentage improvements.

## 9. Integration graph

- #7002: centre-sampled GSD k-coverage paper reproduction.
- #6925: geometric candidate views, exact small-instance selection, robust and nominal k-coverage semantics.
- #7136: conditional support-envelope geometry, support derivation and conservative occlusion.
- #7043/#7049: feature-measurement-quality / geometric identifiability.
- #6846: datum, pose, covariance, GUM-compatible uncertainty and decision guard bands.
- #4258/#6752: observation models, sensor dependence and stateful measurement processes.
- #5920/#5892: robot/fleet reachability, routing, charging and execution.
- #4997/#2239/#1892/#6956: active measurement and non-anticipative post-observation decision policies.
- Mycelix #6390/#6519: AP242/QIF identity, evidence custody and authority evidence.

The integration rule is: candidate generation -> conditional support geometry -> k-coverage -> sampling density / photometric usability -> feature measurability -> calibrated uncertainty -> route-feasible acquisition -> observed measurement -> independent decision. No downstream stage may inherit a claim that upstream evidence did not establish.
