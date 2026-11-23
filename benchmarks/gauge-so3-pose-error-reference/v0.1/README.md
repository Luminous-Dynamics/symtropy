# GAUGE rigid-pose SE(3) error reference v0.1

**Parent:** [BENCH-GAUGE-RIGID-001 #6446](https://github.com/Luminous-Dynamics/symtropy/issues/6446)  
**Related scorecard:** [SIM2REAL-ASSURANCE-001 #6527 / PR #6529](https://github.com/Luminous-Dynamics/symtropy/pull/6529)  
**Scope:** analytical metric reference only; no GAUGE data is imported.

## Motivation

GAUGE describes rigid-body measurements as six-degree-of-freedom pose trajectories. The M1 real-to-sim scorecard currently models orientation as a scalar `object_orientation_rad` channel. That representation is useful for scalar components but does not, by itself, specify a coordinate frame or an intrinsic distance between two 3D rotations.

Rotations belong to SO(3), not ordinary Euclidean 3-space. Component-wise Euler differences can be discontinuous at angle-wrap boundaries. Quaternion storage also has a double-cover property: `q` and `-q` encode the same rotation.

This reference therefore adds a supplementary pose diagnostic. It does not replace GAUGE's published trajectory metrics, whose released rigid-body generalized trajectory is position-based.

## Contract

A pose is declared as:
- translation of the body origin expressed in a named world frame, in metres;
- a rotation mapping body coordinates into that world frame;
- an explicitly declared right-handed frame, Hamilton quaternion algebra and quaternion component storage order;
- stable world-frame and body-frame identifiers.

The reference accepts WXYZ or XYZW storage and converts it to canonical WXYZ. It rejects left-handed frames, body-from-world transforms and JPL algebra rather than guessing a conversion. Real and simulated poses must use the same world/body frame identities. Any required calibration transform must be applied explicitly by an adapter before comparison.

The output preserves two distinct quantities:
- translation error: `||p_sim - p_real||_2`, in metres;
- geodesic rotation error: `2 atan2(||vec(conj(q_real) * q_sim)||, |scalar(conj(q_real) * q_sim)|)`, in radians and bounded by `[0, pi]`.

No position/rotation weighting or composite score is introduced.

## Known-answer and negative controls

The Rust reference includes tests for:
- identical poses and quaternion sign equivalence;
- WXYZ/XYZW representation equivalence;
- a 90-degree rotation and an exact half-turn;
- +179 / -179 degree wrap-boundary comparison returning 2 degrees;
- independent translation and rotation outputs;
- frame mismatch rejection;
- non-unit and non-finite quaternion rejection;
- unsupported handedness and algebra rejection;
- invariance under a common rigid change of world frame.

These are analytical controls for the reference implementation. They are not measurements from a physical robot or a result on the GAUGE dataset.

## Limits and integration rule

The GAUGE raw-trial rotation convention, handedness, frame IDs and exact orientation availability still require M0 dataset inspection under #6446. The future adapter must convert the source representation only after those semantics are verified. Do not infer Euler order, quaternion order, frame direction or axis conventions from file names or visualization.

Keep this rotation error as an additional diagnostic channel. Preserve GAUGE's official position-based scores and all task-specific metrics. Report translation and rotation separately; never allow one to compensate for the other.
