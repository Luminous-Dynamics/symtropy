# Toolhead mass-property composition

**Status:** experimental exact-mass-property merge path plus a legacy approximation.
**Date:** 2026-10-31
**Implementation branch:** research/toolhead-mass-properties-2026-10-31.

## Problem

The previous toolhead merge accepted an arbitrary collider and a scalar mass, then modeled its rotational mass distribution as a uniform solid D-ball of radius 0.1. That can be a useful placeholder for a small spherical toolhead, but it is not an honest mass model for arbitrary CAD geometry, long tools, asymmetric end-effectors or composite attachments.

The merge also maintained a legacy isotropic inertia approximation using a parallel-axis increment m*|d|^2. That coefficient is not the mean of the D-dimensional bivector-plane inertias except in D=2.

## New API boundary

- `try_merge_toolhead_with_inertia(transform, collider, tool_inertia)` is the explicit path. It obtains the tool mass from `tool_inertia.total_mass()`, rejects raw second-moment matrices with unknown mass metadata, validates the current body inertia against the body mass, and combines both operators at the new common center of mass before changing any body/collider state.
- `try_merge_toolhead(transform, collider, tool_mass)` remains as a compatibility convenience. It explicitly constructs a uniform solid D-ball of radius 0.1 and delegates to the mass-aware API. This remains an approximation and its radius is not inferred from the collider.
- The old void `merge_toolhead` is still a compatibility wrapper. Invalid inputs cause no merge; callers that need error visibility should use a `try_` method.

The supplied tool inertia must be centered at the tool's own center of mass and expressed in its local frame. The local transform supplies that COM's offset from the parent body's COM frame.

## Exact parallel-axis composition

Let masses be m_a,m_b, component centered second-moment matrices S_a,S_b, component COM offsets d_a,d_b from the combined COM, and require:

\[
m_a d_a + m_b d_b=0.
\]

Then the combined centered second moment is:

\[
S=S_a+m_a d_a d_a^T+S_b+m_b d_b d_b^T.
\]

The full bivector rotation-plane inertia follows from S and is retained in the combined operator. The merge preserves world geometry by shifting every collider child by the negative local COM offset and moving the parent world transform by the rotated positive COM offset.

## Correct N-dimensional legacy mean

For eigenvalues s_i of S, the inertia in plane (i,j) is s_i+s_j. Across all independent planes:

\[
\overline I_{\mathrm{plane}}
=\frac{1}{D(D-1)/2}\sum_{i<j}(s_i+s_j)
=\frac{2}{D}\operatorname{tr}(S).
\]

Therefore a point-mass COM shift contributes:

\[
\Delta\overline I_{\mathrm{plane}}
=\frac{2m}{D}\|d\|^2.
\]

The former m*|d|^2 coefficient is only correct for D=2. When both components have authoritative operators, the compatibility scalar is now derived from the exact combined operator's trace. When the parent has only legacy scalar inertia, the fallback uses each component's mean-plane value and the dimension-correct 2/D parallel-axis increment.

A non-finite or non-positive resulting mean-plane inertia rejects the merge before mutation.

## Regression coverage

- Existing world-geometry preservation under a rotated parent.
- Existing ball/ball exact second-moment parallel-axis test.
- Compatibility scalar mean-plane inertia agrees with the authoritative combined operator.
- Asymmetric ellipsoidal tool mass properties are preserved in the exact merge.
- A raw second-moment operator with unknown mass is rejected with UnknownTotalMass and leaves body mass, transform and world support unchanged.
- Existing body/tool mass mismatch and failed-merge transactional tests remain.

These tests are source-level until exact-head CI completes formatting, Clippy, compilation and the physics test suite.

## Remaining engineering work

- Extract mass properties directly from CAD solids or tetrahedralized meshes, including density fields and COM.
- Record geometry/density hashes, units, source frame and integration accuracy as mass-property provenance.
- Support compound tools with multiple materials and per-part mass distributions.
- Add articulated-link mass-property merge and Jacobian/inertia consistency tests.
- Avoid silently assigning the radius-0.1 approximation in new CAD/robot ingestion paths; require an explicit model choice or supplied operator.
