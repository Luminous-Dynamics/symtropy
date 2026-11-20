# J3 Finger Kinematics Reconstruction v0.1

**Status:** M1 executable serial-chain reconstruction with geometry-derived fixtures. It is not an exact transcription of the PACT paper's Jacobians.

**Tracking issue:** [J3-KINEMATICS-RECON-001 #6395](https://github.com/Luminous-Dynamics/symtropy/issues/6395)  
**Related J3 force reference:** [PR #6139](https://github.com/Luminous-Dynamics/symtropy/pull/6139)

## What the source publishes

Kumar et al., *PACT: Pose-Robust Actuation Co-Design for Tendon-Driven Robotic Hands*, arXiv:2611.05302 (3 Nov 2026), publishes:

- Joint order MCP A/A, MCP F/E, PIP F/E, DIP F/E.
- Eight task-pose angle tuples in Table I.
- Abduction/adduction link length \(a=10\) mm.
- MCP, PIP and DIP link lengths \(\ell=(45,25,20)\) mm.
- A \(3\times4\) fingertip Jacobian used in its equilibrium equations.

The accessible article gives the Jacobian's role, but does not provide a complete homogeneous-transform chain, explicit joint-axis vectors, zero-pose frame convention or a machine-readable pose Jacobian table. The mechanism figure gives joint ordering and visual layout but does not uniquely specify all transforms.

## Declared transform convention

This crate makes the following reconstruction assumption, which must not be mistaken for a source-published convention:

- Right-handed base frame.
- Local +Y points along the extended finger.
- Local +Z points in the positive flexion direction.
- MCP F/E is about local +X at zero abduction.
- MCP A/A rotates the downstream chain about base +Z.
- The 10 mm A/A link is interpreted as a local +Y offset from the A/A origin to the MCP F/E origin.
- MCP, PIP and DIP flexion axes remain parallel to local +X; phalanx orientations use cumulative flexion angles.
- The fingertip is the distal endpoint of the 20 mm DIP phalanx.

For \(q=[q_{AA},q_{MCP},q_{PIP},q_{DIP}]\), define:

\[
\theta_1=q_{MCP},\quad
\theta_2=q_{MCP}+q_{PIP},\quad
\theta_3=q_{MCP}+q_{PIP}+q_{DIP}.
\]

The local fingertip point is:

\[
p_{local} =
\begin{bmatrix}
0\\
a+\sum_{i=1}^3\ell_i\cos\theta_i\\
\sum_{i=1}^3\ell_i\sin\theta_i
\end{bmatrix},
\qquad
p=R_z(q_{AA})p_{local}.
\]

All lengths are in metres and all joint-angle API values are radians. Source pose-table degrees are converted explicitly.

The Jacobian columns are analytic derivatives of this transform chain. The A/A column is the cross product of the base +Z axis with the fingertip point. The flexion columns are derivatives of the downstream phalanx vectors, rotated into the base frame.

## Executable checks

- Analytic FK and \(3\times4\) Jacobian.
- Independent central-finite-difference derivative oracle.
- All eight source pose angles, including 15-degree abduction for pose 8.
- Virtual-work consistency \(f^T\Delta p \approx (J^Tf)^T\Delta q\).
- Invalid dimensions, non-finite angles and invalid finite-difference steps rejected.

The JSON fixture records geometry-derived fingertip positions under the declared convention. It deliberately does not call them PACT-measured fingertip coordinates.

## Promotion path

1. **M0:** source dimensions and pose angles transcribed, transform assumptions explicit.
2. **M1:** analytic FK/Jacobian agrees with independent central differences and virtual-work checks.
3. **M2:** cross-check the assumed frames, base offset and fingertip definition against author source code, CAD, supplemental material or author confirmation. A visual plot fit alone is not sufficient to establish exact source equivalence.
4. **M3:** only after independent frame review, feed the Jacobian into the J3 force-polytope oracle and calculate source-bounded metrics.
5. **M4:** validate link geometry and encoder-derived pose against a measured physical finger.

## Claim ceiling

This is a self-consistent, geometry-bounded reconstruction, not proof of the exact PACT kinematics. It does not reproduce the paper's 0.88 nominal or 0.75 loss-aware coverage, and it does not qualify a physical finger, tendon transmission, actuator, whole-hand grasp or human-contact safety.
