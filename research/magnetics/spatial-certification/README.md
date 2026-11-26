# Magnetic spatial certification (v0.1)

A zero-dependency Rust reference for certifying a scalar magnetic-system functional margin across a continuous, axis-aligned 3D region.

## Why this is separate from active metrology

The active-metrology planner (#7540 / #7539) ranks a finite set of candidate probe locations. This crate addresses a different question: can a named functional requirement be certified throughout the unsampled volume between measurements?

A finite grid can miss a narrow region where encoder phase margin, usable field, Hall threshold reserve or another scalar response changes sign. Spatial certification encloses the response over a whole box using independently qualified component-wise derivative supremum bounds.

## Mathematical contract

For a box centred at x_c with half-widths h_x, h_y, h_z, a centre functional-margin interval [m_lo, m_hi], and component-wise derivative absolute supremum bounds L_x, L_y, L_z valid over the **entire box**, compute:

    delta_max = L_x*h_x + L_y*h_y + L_z*h_z

    margin(box) is enclosed by:
      [m_lo - delta_max, m_hi + delta_max]

The L_i values are not derivatives sampled at the centre. They must be conservative suprema over the complete cell, for the exact model, condition and named requirement. A centre gradient is not a substitute.

For a convention where positive margin means compliant:
- lower enclosure strictly above zero: certified safe throughout the box;
- upper enclosure strictly below zero: certified failing throughout the box;
- otherwise: unresolved.

A cell touching zero is unresolved. The computed lower and upper enclosures use outward-rounded f64 operations: derivative products/sums round upward, the lower margin rounds downward and the upper margin rounds upward. Overflow or non-finite results fail closed. Missing or invalid identities, a centre sample away from the exact box centre, or derivative evidence bound to another cell also fails closed.

## Deterministic refinement

For an unresolved adaptive-exploration cell:
- select the axis with the largest contribution L_i*h_i;
- exact ties preserve X, then Y, then Z;
- divide at a numerically stable midpoint;
- emit two child boxes and their centres as suggested next measurement locations;
- enforce maximum depth, maximum leaf count and minimum child width.

A split is accepted only when its two children exactly partition the parent: matching outer bounds on all axes, one shared interior split plane, no overlap and no gap. Repeated validated splits preserve the root volume by construction. For a supplied leaf set, domain coverage is verified using a coordinate-plane sweep: each atomic X/Y slab must have Z intervals that tile the root depth with exact shared boundaries. This rejects even small representable gaps instead of relying on approximate volume-sum equality. The domain-certification entry point also checks that every leaf shares the same domain, frame, condition, probe calibration, model/solver, geometry/material, requirement, units and evidence role; it reports safe, failing and unresolved cell counts and marks the domain safe only when every leaf is certified safe. The domain-certification entry point also checks that every leaf shares the same domain, frame, condition, probe calibration, model/solver, geometry/material, requirement, units and evidence role; it reports safe, failing and unresolved cell counts and marks the domain safe only when every leaf is certified safe.

If all spatial contributions are zero, further subdivision cannot tighten the supplied interval. The result remains unresolved with a measurement/model-uncertainty disposition rather than inventing a spatial split. Budget exhaustion also leaves the cell unresolved.

Blind-holdout, calibration and post-selection-qualification evidence may be classified, but cannot be sent to the adaptive refinement planner. The planner accepts only the AdaptiveExploration role.

## Evidence contract

Each cell record binds:
- cell/domain ID and exact 3D bounds;
- exact centre sample ID, digest and position;
- centre margin interval and units;
- component-wise derivative supremum values, units, source record IDs and the cell over which they are valid;
- coordinate frame, operating condition, probe calibration, model set, solver run, geometry/material digests and functional requirement;
- evidence role.

This crate validates arithmetic, identity consistency and split geometry. IDs and digests are provenance links, not cryptographic authentication. It cannot prove that an external derivative bound is physically valid.

## Benchmark protocol

Compare adaptive spatial certification against uniform grids and the finite-candidate planner using the same domain, centre evidence, qualified derivative bounds, probe calibration/cost, operating conditions and frozen blind holdout.

Report physical observations and acquisition time to certify the entire domain; safe/failing/unresolved volume; false-safe and missed-risk regions; subdivisions; interval-bound evaluation cost; and sensitivity to bound conservatism.

A target of 2x fewer physical measurements than a uniform grid at the same blind-holdout false-safe rate is aspirational. The primary qualification criterion is zero false-safe classifications on the preregistered challenge set. Do not infer a probability or manufacturing yield from finite challenge cases.

## Claim ceiling

This is a deterministic enclosure of the supplied scalar margin, conditional on the supplied derivative supremum bounds being valid throughout each exact cell. It does not execute a field solver, authenticate measurements, validate calibration, establish the correctness of a magnetic model, estimate yield, replace physical metrology or authorize component release.
