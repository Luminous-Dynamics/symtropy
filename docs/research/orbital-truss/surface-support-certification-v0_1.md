# Orbital Truss Surface-Support Certification v0.1

**Status:** research contract / not yet implemented
**Parent:** #6845
**Implementation owner:** #7136
**Related paper reproduction:** #7002
**Claim ceiling:** synthetic geometric support certification only

## 1. Motivation

The 2026 GSD-constrained k-coverage paper evaluates sensing constraints and visibility at voxel centres. It explicitly describes this as a discrete approximation, not a guarantee over the entire voxel surface (Bin Khalid and Yeum, arXiv:2610.36450, §III-G and §VII). A centre ray may pass while a boundary point violates range, FOV, incidence or GSD, or while another part of the support is occluded.

This increment must not modify the source-faithful reproduction in #7002. It will run a stricter support-certified model beside the published centre-sampled model on the same input geometry and report both outcomes.

## 2. Paradigm shift

Replace point-sampled coverage with support-certified coverage.

A feature's support is a first-class object with stable identity and explicit representation: triangle patch, planar polygon, calibrated datum patch, or a declared surface subset of a voxel. A candidate viewpoint may count as a full-support cover only when every point in that declared support is certified to satisfy the sensing envelope and the conservative occlusion contract.

Do not conflate centre sample with entire support; clear centre ray with unoccluded support; nominal visibility with robustness; view-count redundancy with failure-domain diversity; or finite-scenario robustness with a probability statement.

## 3. Conservative geometric bound

Let a support S be contained in a ball B(c,r), where c is its declared centre and r is a proven support radius. For an axis-aligned voxel of side s, the circumscribed radius is r = s·sqrt(3)/2. Let sensor position be p, unit camera optical axis toward the support be a, and nominal outward surface normal be n0.

Define d0 = ||p-c||. When d0 > r, the minimum and maximum sensor-to-support distances satisfy:

- d_min >= d0-r;
- d_max <= d0+r.

The maximum angular deviation between the centre ray and any ray from p to a point in B(c,r) is bounded by δ = asin(r/d0). If d0 <= r, this angular bound is undefined and the support cannot be certified by this construction.

Define centre optical-axis/FOV angle η0 = acos(clamp(a·(c-p)/d0)) and centre incidence angle α0 = acos(clamp(n0·(p-c)/d0)). If the support normal lies in a declared cone of half-angle ε_n around n0, conservative angular bounds are:

- η_up = η0 + δ;
- α_up = α0 + δ + ε_n.

For horizontal FOV θ_FOV, image width w pixels, and maximum admissible obliquity α_max, the entire support can only pass the conservative sensing envelope if:

- d_min >= minimum range;
- d_max <= maximum range;
- η_up <= θ_FOV/2;
- α_up <= α_max < 90°;
- GSD_up <= GSD_max, with GSD_up = [2·tan(θ_FOV/2)/w]·d_max/cos(α_up).

This uses the paper's GSD relation but bounds it over the support rather than evaluating at one voxel centre. It is deliberately conservative and may abstain even when dense sampling finds every sampled point acceptable.

### Validity conditions

The bound is valid only when:

1. support containment B(c,r) is itself proven for the declared geometry;
2. p, c, r, θ_FOV, w, α_max, minimum/maximum range and ε_n use pinned units and coordinate frames;
3. the sensor optical-axis convention and normal orientation are explicit;
4. α_up is below 90° so cos(α_up) is positive and bounded away from zero;
5. the normal cone includes all normal variation over the declared support.

Unknown support, normal, calibration or frame information must produce NOT_CERTIFIED / INDETERMINATE, never a favourable default.

## 4. Occlusion contract

A single centre ray cannot certify a whole patch. The reference implementation must reason about the set of rays from p to S. For voxel occupancy, a conservative first implementation may construct an enclosing ray cone / visibility volume that encloses every segment from p to every point in S.

An occupied non-target cell intersecting this conservative volume is a potential obstruction. If the intersection may be an artefact of an over-approximation rather than an actual support ray, the result is INDETERMINATE, not blocked. If an actual subset of support rays is proven blocked while another subset is proven visible, report PARTIAL. CERTIFIED_NOT_FULLY_VISIBLE requires a concrete support point or non-zero support subset proven to fail; an upper bound that merely exceeds a threshold is not proof of failure. CERTIFIED_BLOCKED is reserved for proof that the entire required support is blocked or outside the envelope. Define target-cell exclusion, edge tangency and boundary-contact semantics explicitly.

For small triangle fixtures, compare the conservative volume method against independent deterministic pointwise ray casting over a dense surface sample. Dense sampling is a reference test, not a mathematical proof of full-support visibility.

## 5. Required result model

- CERTIFIED_VISIBLE: all declared support passes the range, FOV, incidence, GSD and occlusion bounds.
- CERTIFIED_NOT_FULLY_VISIBLE: at least one concrete support point or non-zero support subset is proven to fail the declared envelope or visibility requirement; report the failing support region and reason.
- PARTIAL: a visible support subset and a failed support subset are both proven; report covered measure and total measure.
- CERTIFIED_BLOCKED is a stronger optional disposition, reserved for proof that the entire required support is blocked or outside the envelope.
- INDETERMINATE: the support or interval/occlusion bounds are insufficient to decide.
- INVALID_INPUT: malformed support identity, frame, units, calibration, normal cone or camera model.

Only CERTIFIED_VISIBLE contributes to whole-support coverage. PARTIAL and INDETERMINATE do not count as fully covered.

## 6. Frozen adversarial benchmark

Begin with small synthetic 3D fixtures whose exact support geometry and occluders are known:

1. centre passes but one support extremum exceeds maximum range;
2. centre passes FOV but a support corner crosses the FOV boundary;
3. centre incidence passes but normal-cone uncertainty violates maximum obliquity;
4. centre GSD passes but support GSD upper bound exceeds GSD_max;
5. centre ray is clear but a blocker intersects a subset of support rays;
6. blocker tangency and occupied-cell boundary contact;
7. thin wall / multi-normal voxel;
8. sealed cavity versus exterior surface;
9. wholly visible, partially visible, wholly outside-FOV and wholly blocked patches;
10. rigid-frame rotation/translation and unit scaling metamorphic tests;
11. support-radius enlargement and normal-cone widening monotonicity tests;
12. a negative control proving that removing a blocker cannot reduce the visible support under the same geometry.

Use independently authored Rust and standard-library Python reference implementations. The Python code is test-oracle only. Compare interval bounds against dense surface samples and a high-fidelity triangle/voxel intersection oracle; record every conservative abstention and false block.

## 7. Metrics

Report independently:

- false-CERTIFIED_VISIBLE count (target: zero on frozen controls);
- support-area / support-measure fully visible fraction;
- partial support, not-visible support and indeterminate fractions;
- conservatism of GSD_up versus dense-sample GSD maximum;
- FOV and incidence safety margins;
- occlusion false-clear and false-block counts;
- bound tightness versus support radius and normal-cone width;
- runtime and memory by triangle count, voxel count and support partitions;
- sensitivity to voxel resolution and support representation.

Do not combine these into a single score that can trade away false-visible errors.

## 8. Prospective acceptance targets

These are research targets, not achieved results:

- zero false-CERTIFIED_VISIBLE cases in the frozen adversarial corpus;
- zero false-clear occlusion cases in the conservative reference suite;
- exact invariance under rigid coordinate changes and explicitly declared unit conversions;
- monotonicity: enlarging the support or normal cone cannot improve its certificate without new evidence;
- every PARTIAL / INDETERMINATE result includes support identity, bounds and reason codes;
- dense-reference false blocks and conservative abstentions are reported rather than optimized away.

## 9. Integration boundaries

- #7002 owns source-faithful reproduction of the published centre-sampled GSD/voxel k-coverage method.
- #6925 owns candidate-view subset selection and orthogonal k-view / failure-domain contracts.
- #7136 owns full-support GSD, FOV, incidence and occlusion certification.
- #6846 owns datum registration and covariance-aware measurement decisions.
- #5920 / #5892 own route and fleet feasibility.
- #6752 / #4258 own actual observation processes, correlated sensing and common-cause dependencies.

## 10. Claim ceiling

A passing synthetic support certificate establishes only that the declared mathematical support is inside the declared idealized geometric sensing model. It does not establish camera calibration, physical image quality, defect findability, NDE/POD, instrument performance, route reachability, structural integrity, acceptance, airworthiness or flight authorization.

## Primary reference

W. Bin Khalid and C. M. Yeum, “Ground Sample Distance Constrained k-Coverage Viewpoint Planning via Set Multi-Cover Formulation,” arXiv:2610.36450, 27 Oct 2026: https://arxiv.org/abs/2610.36450
