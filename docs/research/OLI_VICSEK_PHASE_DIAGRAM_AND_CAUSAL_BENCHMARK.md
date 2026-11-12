# OLI Vicsek finite-size, structural, and causal benchmark v0

**Status:** pre-registered sweep contract; simulation and source reproduction not yet executed  
**Owner:** Symtropy / `symtropy-lifesim-core::vicsek`  
**Machine-readable design:** `benchmarks/oli/vicsek-phase-diagram-v0.json`  
**Related:** [OLI collective-motion baseline matrix](benchmarks/oli/collective-motion-baseline-matrix-v0.json)

## Why this benchmark is necessary

The Vicsek model is a controlled mechanism for emergent collective motion, not a model of universal intelligence. Global polar order alone is insufficient: locally aligned clusters can face opposite directions and cancel at system level; single-particle neighbourhoods have local order 1 by construction; a shared environmental driver can correlate particles without direct interaction; and a one-way visual interaction does not imply reciprocal influence.

The 2026 literature makes structural measurements especially relevant:
- Jin and Dong (20 October 2026), *Communications Physics*, DOI [10.1038/s42005-026-02933-9](https://doi.org/10.1038/s42005-026-02933-9), report a relationship between polar order and average interaction number, and compare cluster-level with system-level interaction-number distributions.
- Rahman et al. (10 July 2026), *Physical Review Research*, DOI [10.1103/hkrb-wpdr](https://doi.org/10.1103/hkrb-wpdr), study cluster-network and geometric scaling.
- Crucially, the Rahman paper received an erratum on 27 October 2026, DOI [10.1103/b9yr-8wmv](https://doi.org/10.1103/b9yr-8wmv). The corrected heading equation uses the neighbour set at predicted positions t+dt, while using the neighbours' headings at t. The erratum says the corrected result is indistinguishable from their forward-update method; the backward update printed in the original paper produces different degree/path-length power-law scaling. OLI must therefore record update convention as part of model identity.

## Two tracks, never mixed

### A. Source-faithful Rahman reproduction

The manifest records the exact published article, correction, repository and pending artifact admission. The exact source grid, scripts, supplementary data, file hashes and licence have not yet been admitted. They must be recovered before claiming reproduction.

Use NextPositionAlignment for the corrected article profile. Keep the backward-update arm only as a distinct sensitivity comparison. Do not pool scaling exponents across update rules.

### B. Independent OLI finite-size extension

A separate, pre-registered OLI grid uses:
- N in {128, 256, 512};
- densities rho in {0.5, 1.0}, with L = sqrt(N/rho);
- interaction radius R=1;
- speed v0=0.03, dt=1;
- 17 angular-noise widths from 0 to 2π, where the implementation parameter is the full width of the interval [-eta/2,+eta/2];
- 30 independent seeds per design point, 1,000 burn-in steps and 2,000 measurement steps.

This is an OLI extension, not a reproduction of the exact 2026 article parameter set. It gives 102 design points and 3,060 trajectories. All per-seed outputs are preserved; frames are repeated measures, not independent cases.

## Measurements

Global order is reported beside local order and its coverage fraction; cluster size and nontrivial-cluster order; degree/interaction-number distributions; clustering coefficient; shortest-path length; connected-pair fraction; and time-dependent observables.

The system must preserve null for local-order estimates when no particle has another neighbour. Isolated particles must not make a disordered gas appear to have perfect collective local order.

Future geometry extensions should reproduce cluster area/perimeter scaling only after the exact geometric estimator used by the source paper is established. Do not substitute a convex hull or pixel-grid approximation and call it the same observable without declaring it as a separate estimator.

## Statistical protocol

- Each independent seed/configuration is the statistical unit.
- Aggregate temporal samples into per-run summaries, then quantify between-seed uncertainty.
- Estimate chi = N × Var_seed(P_bar), where P_bar is a per-run mean over the registered stationary window.
- Examine order-parameter histograms for bimodality, transient relaxation, finite-size behaviour and hysteresis under up/down noise sweeps.
- Report the entire grid, candidate transition interval, fit ranges, fit diagnostics and alternative scaling laws.
- Bootstrap over independent seed summaries, never over frames, particles or filaments.

A transition/noise threshold is not hard-coded. A phase-transition claim requires finite-size evidence, seed-level uncertainty, stationary-window validation and sensitivity to update rule. A scaling law requires independent source reproduction, declared fit range, confidence interval and alternative-fit diagnostics.

## Causal and observational separation

Use paired treatment/sham replays with common random numbers to measure the downstream effect of a single-particle heading intervention. Separately apply a common exogenous heading field to isolated particles. The two tests distinguish:
- direct model-defined interaction;
- interventional downstream influence under the declared dynamics;
- common-driver correlation;
- statistical information transfer.

These are not interchangeable. Transfer entropy or a learned attention weight alone is not a causal edge. A shared response to a common field is not direct communication.

## Performance and implementation integrity

The periodic cell index is intended to reduce candidate checks at bounded density. It must never omit a true radius neighbour. The new property-style test checks candidate-set completeness across one-, two-, three- and many-cell periodic grids plus fallback cases. Exact indexed-vs-all-pairs replay parity remains a gate.

Performance results must include index construction, per-step update, fast metrics, periodic full-topology checkpoints and dense-neighbour fallbacks. No speedup claim is made until pinned-hardware runs complete.

## Claim ceiling

This is a reproducible numerical study of known interaction rules. It does not show biological life, intelligence, agency or consciousness. It does not establish a universal critical noise, universal phase boundary, or natural-animal model.


## Angle-mean convention and global-phase branch-cut challenge

A separate source challenge is registered in `benchmarks/oli/vicsek-angle-branchcut-source-profile-v0.json` and summarized in the phase-diagram manifest.

Takahashi et al. (2026), *Physical Review E* 114, 014134, DOI [10.1103/9gcy-ky3z](https://doi.org/10.1103/9gcy-ky3z), show that the principal-value `atan(sum_sin/sum_cos)` rule can depend on the global phase reference because its output is constrained to a principal branch. The source's adaptive operation is `phi(t)=phi_bar - arithmetic_mean(theta_i(t))`, applied to the heading variables before velocities and the next update are computed.

OLI therefore records three separate conventions:
- **PrincipalValueArctanRatio:** exact branch-sensitive ratio rule in the paper.
- **QuadrantCorrectAtan2:** OLI's default circular mean, with full quadrant handling.
- **ArithmeticScalarMean:** the distinct scalar-angle average discussed by the paper; it is not a synonym for atan2 and requires its own implementation before source comparison.

The Rust model now carries the selected convention in its metrics and cluster-geometry report. Its adaptive phase operation preserves unwrapped shifted headings until the next update. The minimal self-only control begins with headings [-2.5, -1, 1, 2.5] radians and shifts to pi: atan2 preserves the polar order (about 0.1304), while the ratio rule folds the angles and raises the order to about 0.6707 without any inter-particle interactions.

The separate metamorphic rotation test rotates a local particle cluster (kept away from periodic boundaries), uses identical noise seeds, and tests both current- and next-position neighbour conventions. This is not a proof of continuous rotational symmetry of the square torus, and the four-particle branch-cut control is not the source paper's N=1600 phase-transition reproduction.

No phase-transition claim is made until the source's exact seed law, initial-state generation, averaging window, and full N=1600 / 10,000-step adaptive phase sweep are reproduced. Never pool results across the three angle conventions or across neighbour-time update rules.
