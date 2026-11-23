# Surface-Support Certification Contract v0.1

**Owner:** Symtropy #7136.
**Implementation:** tools/orbital-surface-support-bench.
**Upstream geometry:** #7020 / orbital-gsd-kcoverage-bench.
**Evidence ceiling:** synthetic 3D geometric sensing-envelope and voxel-occupancy visibility only.

## Purpose

The 3D candidate pipeline evaluates sensing constraints at voxel centres. This extension evaluates the entire declared support AABB around each source surface feature. It consumes the upstream voxel, flood-fill, face and candidate outputs rather than introducing another geometry engine.

## Support representation

The upstream support exporter creates per-feature surface supports. Opposing thin-wall faces are separate planar supports. A non-cancelling union of exterior faces is represented by the full voxel AABB and a normal cone enclosing each constituent face normal. Extents are rounded outward; normal-cone bounds include normal quantization error.

## Sensing envelope

Let p be camera position, c the quantized support centre, h the outward-rounded support half-extents, and r = ||h|| + e_p, where e_p bounds camera-position quantization. d0 = ||c-p|| and delta = asin(r/d0) when d0 > r. If d0 <= r, the certificate is INDETERMINATE.

With camera axis a, support normal n, declared normal cone epsilon_n, horizontal FOV theta and pixel width w:

- eta_up = angle(a,c-p) + delta + epsilon_axis;
- alpha_up = angle(n,p-c) + delta + epsilon_n;
- d_max = d0 + r;
- GSD_up = d_max * 2*tan(theta/2) / (w*cos(alpha_up)).

Certification requires d_max not exceed the source-derived slant range, eta_up <= theta/2, alpha_up <= alpha_max < pi/2, and GSD_up <= GSD_max. It uses the upstream camera model, including its conical FOV convention and GSD equation. No missing intrinsics are inferred.

## Conservative occlusion

The AABB enclosing the camera position and entire support AABB encloses every line segment from the camera to every support point. If any non-target occupied voxel intersects this enclosure (including boundary contact), the result is INDETERMINATE. This broad phase can create false-positive overlaps; it must not create false clears. If no non-target occupied voxel intersects the enclosure, all support rays are clear under the declared voxel occupancy model.

## Outcomes

- CERTIFIED_VISIBLE: all geometric envelope upper bounds pass and the conservative ray enclosure is disjoint from all non-target occupied cells.
- INDETERMINATE: any bound is incomplete/violated without a constructive support counterexample, or a conservative occlusion enclosure intersects a possible blocker.
- CENTRE_NOT_VISIBLE: upstream centre visibility was false; this is reported separately and is not a support-level failure proof.

v0.1 deliberately does not emit PARTIAL, CERTIFIED_NOT_FULLY_VISIBLE or CERTIFIED_BLOCKED. Those require a constructive proof of a non-zero failing support subset or of the entire support being blocked. An upper bound overrun is inconclusive, not proof that the support fails.

## Frozen controls

- An isolated voxel with a synthetic camera model for which some full face supports fit the complete bound and have no non-target occupied blockers.
- The same voxel with reduced pixel count, where the upstream centre point remains visible but the sensor lies within the support bounding sphere; support certification must abstain.

Compare full receipts against a separate Python oracle that reuses the qualified source GSD visibility oracle but independently reconstructs surface supports and the support certificate.

## Claim boundary

Even CERTIFIED_VISIBLE is not a calibrated camera measurement, defect-detection result, NDE/POD, metrology certificate, route feasibility, structural acceptance, airworthiness or flight authorization. All outputs remain synthetic with authority false and physical_validation NOT_ASSESSED.
