# OLI Vicsek angle convention and global-phase symmetry challenge v0

**Status:** source profile and minimal known-truth controls pinned; full source reproduction pending  
**Owner:** Symtropy / OLI collective-motion benchmarks  
**Machine-readable manifest:** `benchmarks/oli/vicsek-angle-branchcut-source-profile-v0.json`  
**Related:** [OLI Vicsek phase diagram and causal benchmark](OLI_VICSEK_PHASE_DIAGRAM_AND_CAUSAL_BENCHMARK.md)

## Why this changes the benchmark

Takahashi et al. (2026), *Physical Review E* 114, 014134, DOI [10.1103/9gcy-ky3z](https://doi.org/10.1103/9gcy-ky3z), report that the principal-value arctan-ratio update in the original angle-based Vicsek formulation is not invariant under a global phase shift at the level of angular increments. They study an adaptive phase reference
`phi(t) = phi_bar - mean_i(theta_i(t))`
and report that the order transition in their arctan-ratio model depends strongly on the selected phase, including a loss of clear flocking order for their `phi_bar=pi` condition.

The exact source equation is:

`theta_i(t+dt) = atan(mean_j sin(theta_j) / mean_j cos(theta_j)) + noise  (mod 2pi)`.

This must not be confused with:

- `atan2(sum_sin, sum_cos)`: quadrant-correct circular mean / complex argument;
- arithmetic scalar mean of angle representatives: a third convention discussed by the paper.

The expressions `atan(y/x)` and `atan2(y,x)` are **not equivalent** in quadrants II and III. The OLI default remains quadrant-correct `atan2`; the source-specific ratio model is an explicit sensitivity variant. The two model families must never be pooled in one phase diagram or scaling fit.

## OLI implementation

The Vicsek model now records `VicsekHeadingMean` in its trajectory metrics and cluster-geometry receipts:
- `QuadrantCorrectAtan2`: the default OLI model.
- `PrincipalValueArctanRatio`: the source-specific principal-value ratio variant.

`apply_adaptive_global_phase_shift(target_phase_rad)` reproduces the paper's phase operation: map headings to the principal interval, calculate the arithmetic mean (not the circular mean), and add the common phase shift without wrapping shifted headings before the next update.

The standard `apply_common_heading_impulse` remains a separate wrapped intervention API. Do not substitute it for the unwrapped adaptive phase procedure in the source reproduction.

## Minimal known-truth branch-cut test

A four-particle, zero-noise, self-only control begins with headings:
`[-2.5, -1.0, 1.0, 2.5]` radians.

The initial polar order is 0.13042065483939697.

After applying an adaptive global phase shift to target phase pi and advancing one step:
- quadrant-correct `atan2` preserves order at 0.13042065483939697;
- principal-value `atan(sum_sin/sum_cos)` folds angles separated by pi onto one tangent branch and raises order to 0.6707229607075367.

There are no inter-particle interactions in this fixture. The order change is therefore a mathematical branch-cut artifact, not collective coordination. This small control is not a reproduction of the paper's 1,600-particle phase-transition simulation.

## Full source profile to reproduce

The paper's numerical profile records:
- N=1600, L=8;
- speed 0.01, interaction radius 0.1, dt=1;
- angular-noise amplitude eta=0.75, uniform noise interval [-0.375, +0.375] at dt=1;
- uniformly sampled initial positions over [0,8)²;
- initial headings uniform over (-pi, pi];
- adaptive phase targets 0 and pi;
- plotted evolution through 10,000 time steps.

The exact random seeds, source code commit, averaging windows and any additional reproducibility details still need to be admitted. No full source reproduction is claimed.

## Required comparisons

1. Exact principal-value ratio source variant.
2. Quadrant-correct `atan2` OLI variant.
3. The paper's arithmetic scalar angle-mean variant, if implemented; it must be its own model ID.
4. Fixed phase reference versus adaptive targets 0 and pi.
5. Both neighbour-time conventions (current-position and next-position) as separately labelled controls.
6. Multiple seeds and finite-size sensitivity before any transition claim.

The paper's adaptive shift is applied to angular variables before calculating velocities and the next alignment update. A one-time rotation of positions and headings is not an equivalent experiment.

## Interpretation safeguards

- The metamorphic rotation test in the Rust suite rotates a local cluster away from periodic boundaries. It is not a proof that a square torus has continuous O(2) symmetry under arbitrary whole-domain rotations.
- The adaptive global phase experiment changes angle reference while holding the spatial coordinates fixed. It is a model-coordinate sensitivity challenge, not a physical applied torque.
- A branch-cut-dependent order parameter is not evidence of intelligence, agency, or biological life.
- Do not make a universal claim about the Vicsek model from one four-particle fixture or one finite-size run. Reproduce the paper profile, its exact convention and seed law first.

## Qualification

The exact-angle positive/negative unit tests, source profile manifest, and full finite-size phase sweep are separate deliverables. The full paper claim requires the N=1600 source-profile run, fixed/adaptive phase comparisons, multiple independent seeds, reproducible uncertainty estimates, and a comparison of the three angle-mean conventions.
