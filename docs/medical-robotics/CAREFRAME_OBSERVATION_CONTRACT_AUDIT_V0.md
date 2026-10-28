# Careframe Observation-Contract Audit v0

**Status:** synthetic adapter-contract audit. It does not test live sensor code, perception accuracy, control quality, a physical robot or clinical workflow.

## Why this is a benchmark validity gate

A peer-reviewed study published on 20 October 2026, “Observation-contract auditing for valid modality comparison in robot manipulation benchmarks,” reports that changing observation pathways can silently drop fields or reorder state keys. In its audit of ManiSkill3, RLBench and a Diffusion Policy/robomimic integration, it reports 31–53% of observable state missing from the evaluated paths. Under one matched setup, completeness of the state contract mattered more than adding point clouds.

That finding changes Careframe's evaluation order: **audit what the policy actually receives before comparing whether the policy succeeded**. A score comparison across state, image or point-cloud paths is invalid if one path silently receives less semantic state, uses different units/order, or is given simulator ground truth unavailable to the others.

Reference:
- He, Wang & Xu, “Observation-contract auditing for valid modality comparison in robot manipulation benchmarks,” *Scientific Reports*, published 20 October 2026. https://www.nature.com/articles/s41598-026-76821-1
- Jiang et al., “What Are We Actually Benchmarking in Robot Manipulation?” (2 June 2026), on shortcut solvability, statistical significance, overfitting and data-source dependence. https://arxiv.org/abs/2606.04233
- NIST, “Mobility Performance of Robotic Systems,” a measurement-science program for mobile robots and mobile manipulators. https://www.nist.gov/programs-projects/mobility-performance-robotic-systems

## Contract in this change

- `careframe-observation-contract-v0.json`: a catalog of 18 required policy-visible semantic fields, including robot pose/velocity, joint state, tool identity and revision, task context, safety state, target/object/obstacle estimates, confidence and visibility.
- `careframe-observation-adapters-v0.1.json`: synthetic declarations for state-vector estimates, RGB-derived estimates and point-cloud-derived estimates.
- `tools/careframe-observation-audit`: a standalone Rust auditor with a pinned dependency lock. It validates exact field coverage and ordering, unit/type identity, source-family matching, modality/raw-input mapping, and fail-closed treatment of undeclared fields.
- Two simulator-oracle-only values (object mass and surface friction) are explicitly listed as scoring-only and must never appear as policy-visible inputs.
- The receipt binds the contract bytes, adapter-suite bytes, auditor source and tested revision.

## Acceptance criteria

For the matched semantic-state profile:

1. All three adapters declare the same ordered 18-field policy contract.
2. Every field's type, unit and source family match the catalog.
3. Each adapter uses the raw-input family declared for its modality.
4. No simulator-truth field is exposed to policy input; oracle-only values remain scoring-only.
5. Missing fields, reordered fields, unit/type drift, source drift, unknown fields, duplicate IDs, incompatible modality declarations or weakened comparison guards fail closed.
6. Results are bound to exact input-file digests and a tested revision.

The frozen fixture is currently valid under an author-time semantic reimplementation, and mutation checks for dropped/reordered fields, unit/type changes, oracle leakage, source mismatch, unknown fields, wrong modality inputs and duplicate adapters are rejected. This is not yet proof of Rust test success or runtime integration.

## Correct interpretation

This audit checks declarations, not perception outputs. It cannot prove that an RGB detector estimates object pose correctly, that a point cloud is calibrated, or that sensor timestamps are synchronized. Real adapters must emit contract-validated observations from their actual code path, and the test harness must sample those exact outputs—not a hand-authored mirror of what the developer hoped the adapter would output.

If an adapter cannot provide a required field, the benchmark should label the comparison **not matched** or run a separately declared modality-specific profile. It must not silently insert simulator truth or compare scores as though the information were equal.

## Next validation stages

1. Run Rust schema/contract tests and exact-head CI.
2. Instrument the real Symtropy-to-Symthaea bridge at the policy boundary and record emitted field IDs/order/units and source provenance. A separate bridge PR has started this work by naming and validating the existing 13-value base-kinematics HDC projection and clearing invalid/stale observations: https://github.com/Luminous-Dynamics/symtropy/pull/2786. That projection remains incomplete for Careframe: the HDC path omits joints, tools, task context, scene estimates and proximity, and its angular-velocity values are bivector-plane components rather than the catalog's Cartesian xyz vector. The full adapter must explicitly convert and instrument those semantics before it can claim the matched profile.
3. Add runtime contract probes for each enabled observation pathway and verify perception estimates against independent ground truth.
4. Add matched held-out tasks and nuisance shifts; report field coverage alongside task success, interventions, safe aborts and recovery.
5. Only after the bridge and simulator semantics are verified, use this protocol to compare model/policy variants. No performance or safety superiority claim is made by this document.
