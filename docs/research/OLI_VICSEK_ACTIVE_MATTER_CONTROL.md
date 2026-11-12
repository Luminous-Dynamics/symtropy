# OLI synthetic collective-order control: Vicsek active matter v0

**Status:** deterministic CPU reference added; Rust validation pending  
**Owner:** Symtropy / `symtropy-lifesim-core`  
**Related:** [OLI abiotic look-alike tournament #4572](https://github.com/Luminous-Dynamics/symtropy/issues/4572)  
**Machine-readable cases:** `benchmarks/oli/vicsek-active-matter-v0.json`

## Research rationale

The Vicsek model is a canonical minimal model of collective motion. Self-driven particles move at constant speed in a periodic two-dimensional domain; each particle aligns its heading with the circular mean of neighbours inside a metric radius and receives bounded angular noise. Vicsek et al. (1995) demonstrated a transition from disordered motion to collective polar order as interaction/noise parameters change.

A 2026 analysis revisits the original Vicsek model and relates polar order to interaction-number statistics and cluster/system-level structure. OLI therefore records polar order, mean neighbour count, neighbour-count variance and connected-component structure together rather than relying on a single global scalar.

Sources:
- Vicsek et al. (1995), *Physical Review Letters*, DOI [10.1103/PhysRevLett.75.1226](https://doi.org/10.1103/PhysRevLett.75.1226).
- Rahman et al. (2026), *Physical Review Research*, [Topology and geometry of clusters in the Vicsek model across scales](https://doi.org/10.1103/hkrb-wpdr).
- Rahman et al. (2026), [Erratum correcting the neighbor-set time index](https://doi.org/10.1103/b9yr-8wmv), published 27 October 2026.
- Jin & Dong (2026), *Communications Physics*, [Quantitative relationship between coupled structural dynamics and collective order in the original Vicsek model](https://www.nature.com/articles/s42005-026-02933-9).
- Pang & Wang (2026), *Physical Review E*, [Quantifying influence and information transfer in a modified Vicsek model with nonreciprocal interactions](https://doi.org/10.1103/rf2v-sy5d).

## Scientific boundary

This is a synthetic, known-rule active-particle model. It contains:
- no metabolism or self-maintaining biological boundary;
- no reproduction, inheritance or evolution;
- no memory or learning;
- no task objective or environmental problem-solving;
- no evidence of sentience or consciousness.

A high polar-order score demonstrates collective directional alignment under local interaction rules. It does not establish biological life, agency, cognition, intelligence or consciousness. It is not a validated model of a particular animal flock, bacterial swarm, plasma, or extraterrestrial system.

## 2026 topology benchmark comparator and erratum

Rahman et al., *Topology and geometry of clusters in the Vicsek model across scales*, Physical Review Research 8, 033039 (10 July 2026), DOI [10.1103/hkrb-wpdr](https://doi.org/10.1103/hkrb-wpdr), analyze connected-component clusters as interaction-network subgraphs. They report scale-dependent relationships in average degree and path length, convergence of average clustering coefficient, and geometric scaling of cluster area and perimeter. This motivates reporting local and cluster-level structure rather than only global polar order.

**Important correction:** the paper has an erratum published 27 October 2026, DOI [10.1103/b9yr-8wmv](https://doi.org/10.1103/b9yr-8wmv). Equation (2)'s neighbour-set time index was corrected: the heading at (t+\Delta t) is aligned using the neighbour set at (t+\Delta t), with velocities/headings at (t). The erratum says the physical interpretation is unchanged and the corrected result is indistinguishable from a forward-update method; a backward-update model yields different power-law scaling of mean degree and path length with cluster mass. OLI must record the exact update rule with every result.

The implementation exposes two explicit rules:
- `CurrentPositionAlignment`: select neighbours at positions (x(t)), compute all new headings from the previous heading state, then move along the new headings.
- `NextPositionAlignment`: predict positions using headings at (t), select neighbours at predicted positions (x(t+\Delta t)), compute new headings from headings at (t), and retain the predicted positions.

A deterministic fixture has two particles initially outside the interaction radius but inside it after moving along their old headings. The classic rule keeps them separate; the next-position rule couples them. This tests the semantic difference directly.

## Implementation

Module: `crates/domains/symtropy-lifesim-core/src/vicsek.rs`

- Two-dimensional periodic square (torus).
- Metric neighbour search using minimum-image distance.
- Self is included as a neighbour, matching the standard alignment rule.
- Circular-mean heading update using `atan2(sum(sin θ), sum(cos θ))`; arithmetic averaging of angles is avoided.
- Synchronous updates from the previous particle configuration.
- Constant-speed movement after heading update.
- Uniform angular noise of declared full width `η`, sampled over `[-η/2,+η/2]`.
- Seeded xorshift64 random initialization and continuous, replayable angular-noise stream.
- CPU-first (O(N^2)) reference implementation.

The module exports:
- global polar order parameter;
- mean local polar order, calculated only over particles with at least one other neighbour;
- local-order coverage fraction, preventing isolated particles' trivial self-alignment from appearing as collective local order;
- mean neighbour count excluding self;
- neighbour-count variance;
- mean and variance of per-particle local polar order (each local neighbourhood includes its focal particle);
- mean and variance of per-particle local polar order;
- connected-component count in the metric-neighbour graph;
- largest connected-component fraction;
- mean local clustering coefficient;
- mean shortest-path length over connected unordered pairs;
- fraction of all unordered pairs that are connected.

## Exact positive and negative controls

1. **All-to-all, zero-noise alignment:** six particles with different initial headings, interaction radius spanning the entire torus and zero noise must produce polar order 1 after one update.
2. **Self-only, zero-noise control:** radius zero means each particle only aligns with itself; polar order must remain unchanged, mean neighbour count excluding self must be zero, local-order coverage is zero and local polar order is undefined (null). This prevents a single isolated particle from being counted as perfectly locally ordered.
3. **Opposed local flocks:** two spatially separated three-particle groups have headings 0 and π. Global polar order is approximately zero, but local-order coverage is 1, mean local polar order is 1, and two clusters are present. This is an exact counterexample to interpreting global order alone.
4. **Seeded low-noise exploration:** 256 particles, (L=32), (R=1), (v=0.03), (eta=0.2), seed 20261111, 2,000 steps.
5. **Seeded high-noise control:** same finite system with (eta=4.0), seed 20261112, 2,000 steps.

The low/high noise runs are exploratory finite-size cases. No universal critical noise threshold is hard-coded or asserted. A complete phase diagram requires a pre-registered parameter grid, multiple independent seeds per point, finite-size scaling and uncertainty analysis.

## Update-rule correctness and the 2026 erratum

The 2026 Rahman et al. paper on cluster topology was corrected by a published erratum on 27 October 2026 (DOI [10.1103/b9yr-8wmv](https://doi.org/10.1103/b9yr-8wmv)). The erratum states that the focal particle's next heading must use the neighbor set at (t+\Delta t), while summing neighbors' headings/velocities at (t). The authors report that the corrected results are indistinguishable from their forward-update method; the backward-update rule as originally printed produces different network scaling.

The implementation exposes two explicitly tagged conventions:
- `CurrentPositionAlignment`: determine the neighbor graph at (t), compute headings, then move.
- `NextPositionAlignment`: predict positions at (t+\Delta t) using headings at (t), determine the neighbor graph there, compute new headings using neighbor headings at (t), and retain the predicted positions.

Do not pool benchmark runs across these modes. The frozen `UPDATE_RULE_NEIGHBOUR_TIME_COUNTEREXAMPLE` fixture demonstrates a pair outside the interaction radius at (t) but inside it after the predicted move.

## From statistical transfer to causal influence

Pang & Wang (2026), *Physical Review E*, DOI [10.1103/rf2v-sy5d](https://doi.org/10.1103/rf2v-sy5d), explicitly distinguish statistical information transfer from causal influence. Transfer entropy or predictive dependence can be useful observables, but neither alone establishes that changing one agent would change another.

The implementation now exposes `intervene_heading`, which returns a `HeadingIntervention` receipt (tick, particle, previous heading, imposed heading) without consuming or resetting the PRNG stream. A matched sham is made by cloning the same pre-intervention state; both are then advanced with identical random draws.

Two exact cases are frozen:
1. All-to-all influence: rotate one of six initially aligned particles by 90°. After one noiseless update, every particle adopts `atan2(1,5)`; the matched sham remains at heading 0.
2. Self-only null: rotate one particle when the interaction radius is zero. The other particles must match their sham trajectory exactly.

This establishes causation only inside the known simulation model. It does not establish agency, communication, intention, or intelligence in a natural swarm.

### Network topology observables

The current branch implements:
- mean local clustering coefficient (degree 0 or 1 contributes zero);
- mean unweighted shortest-path length across connected unordered pairs only;
- connected-pair fraction across all unordered particle pairs;
- mean and variance of component size;
- mean and variance of component polar order.

Disconnected pairs are not assigned an arbitrary infinite or sentinel path length. The path-length mean is `None` when the graph has no connected pairs, and the connected-pair fraction communicates fragmentation separately. Cluster area/perimeter and finite-size scaling are still future extensions.

### Common-driver confounding control

The reference model now exposes `apply_common_heading_impulse`, which applies the same exogenous angular input to every particle and returns a separate `CommonFieldIntervention` receipt. It does not create interaction edges, move positions, or consume random numbers.

The frozen three-particle control has interaction radius zero. A shared (+pi/3) environmental impulse changes all three headings identically, while the instantaneous interaction graph remains three isolated vertices. This produces coordinated co-change with no direct agent-to-agent influence. It complements the single-particle intervention/sham controls and gives the Symthaea causal benchmark a known common-driver negative case.

### Controlled influence interventions

`VicsekModel::intervene_heading` applies an explicit heading intervention and returns a receipt with the tick, particle index, previous heading and imposed heading. It does not advance or reset the random stream. The benchmark clones the exact pre-intervention state to produce a sham; the intervention and sham then consume identical future random draws.

An all-to-all positive control changes every particle's next heading from 0 to `atan2(1,5)` when one of six headings is rotated by 90°. A self-only negative control changes only the intervened particle and leaves the other particles identical to the sham. The reusable paired runner records these effects across a full response trajectory using common random numbers. These are causal demonstrations inside the declared simulation model—not evidence of natural agency or intelligence.


### Paired counterfactual replay and nontrivial-cluster support

The module now exposes `run_paired_heading_intervention`. It clones the same pre-intervention model twice, imposes the heading only on the treatment copy, and advances sham and treatment with the same future PRNG draws. For each tick it records:
- sham and treatment metric snapshots, including the explicit update rule;
- mean absolute untreated-particle heading effect and responder fraction;
- mean minimum-image untreated-particle displacement effect and responder fraction;
- the original intervention receipt and the numerical response threshold.

The direct target particle is excluded from downstream-response summaries. The 1e-12 response threshold is a deterministic numerical criterion for this synthetic double-precision model, not an instrument detection limit. A physical or observational adapter must supply its own uncertainty and detection threshold.

Two paired tests are frozen:
1. All-to-all positive control: the first response matches the analytic `atan2(1,5)` heading change across untreated particles, and the downstream position response is nonzero.
2. Self-only null: with angular noise present, 20 treatment/sham steps use common random numbers; all untreated-particle heading and position effects remain exactly zero.

A repeated invocation from the same baseline must produce an identical full result, including the response trajectory.

The cluster metrics also now expose `nontrivial_cluster_coverage_fraction` and `mean_nontrivial_cluster_polar_order`. This avoids a subtle statistical trap: an isolated one-particle component has polar order 1 by construction. The traditional mean across all components remains available for reference, but singleton components do not contribute to nontrivial-cluster coverage or coherence. If every component is a singleton, the nontrivial-cluster mean and variance are `None`, not 1 or 0.

### Asymmetric perception: intervention direction must follow visibility

The new one-way visibility fixture uses two particles one unit apart on the x-axis, both initially facing forward. With a forward visual sector of half-aperture π/4, particle 0 sees particle 1, but particle 1 does not see particle 0.

The paired treatment/sham test runs both directions:
- Intervene on particle 0 (the unseen source from particle 1's perspective): particle 1's heading and position remain identical to sham.
- Intervene on particle 1 (the visible source to particle 0): particle 0 changes by exactly π/4 after one noiseless update, matching the circular mean of the original and imposed headings.

This is a stronger directional causal check than an edge-count test alone: the intervention response must follow the focal-agent visibility relation. The result is still causal evidence only inside the specified synthetic update model.

## Performance and measurement cadence

The implementation separates trajectory-scale and graph-topology analysis:
- `step()` and `run_steps()` use `fast_metrics()`, computing polar/local order, degree distributions and connected components in (O(N^2)) per step.
- `metrics()` computes the complete network topology snapshot, including local clustering and all connected-pair shortest paths. This is more expensive and should be called at pre-registered checkpoints rather than every frame.

The benchmark runner should record its checkpoint schedule in the receipt. The initial proposed schedule is tick 0, after warm-up, every 50 ticks and the final tick; this is a proposed operational default, not yet a qualified optimum. Full topology metrics must never be silently omitted: each snapshot declares whether topology analysis was computed, and unset values are not interpreted as zero.

## Deterministic periodic spatial index

The update path and `fast_metrics` now use a sparse periodic cell index to prune impossible distant candidates:
- cell width is at least the configured interaction radius;
- a periodic 3×3 cell stencil contains every possible range neighbour;
- exact minimum-image distance and visual-sector visibility predicates are still applied after pruning;
- candidate particle IDs are sorted/deduplicated before floating-point summation, preserving the brute-force reference accumulation order;
- for zero/tiny radii or grid requirements above one million cells per axis, the implementation falls back to the all-pairs candidate set.

Expected complexity is near (O(N + 	ext{local candidate work})) at bounded density and local interaction range; worst-case remains (O(N^2)). The full graph shortest-path/clustering calculation remains an explicit checkpoint-only cost. No wall-clock speedup is claimed yet.

Two parity tests are included:
1. **Candidate completeness:** for periodic random fixtures spanning one-cell, two-cell, normal and fallback radii, every brute-force radius neighbour must appear in the indexed candidate set; tests assert that zero, extremely small and larger-than-box radii actually select the fallback.
2. **Simulation and metric parity:** metric-radius and flipped visual-sector one-step updates must exactly match a test-only all-pairs reference, including the PRNG state; a separate all-pairs metric oracle checks directed degree, reciprocal counts, local-order coverage and values.

The next qualification step is a pinned-hardware benchmark over population size, density, interaction radius, visual-sector aperture and occupancy skew. Report candidate counts, index build cost, update cost, metrics cost and worst-case regressions separately; do not publish a speedup from a microbenchmark that excludes the fast-metric work performed each tick.


## Finite-size, source-reproduction, and structure–order programme

The pre-registered grid is in `benchmarks/oli/vicsek-phase-diagram-v0.json`. It deliberately separates two evidence tracks:

1. **Source reproduction:** recover the Rahman et al. 2026 code/data/parameter grid and reproduce the corrected 27 October erratum convention (`NextPositionAlignment`). The source artifact admission is still pending.
2. **Independent OLI extension:** a fixed-radius, fixed-speed finite-size/noise grid across N=128/256/512 and densities 0.5/1.0, with 30 independent seeds per point. This is not the paper's original grid.

The OLI sweep reports global and local order, local-order coverage, cluster and interaction structure, and seed-level susceptibility. It does not hard-code a universal critical noise. A transition claim requires stationary-window diagnostics, finite-size behaviour, seed-level confidence intervals, and update-rule sensitivity. A scaling claim requires a reproduced source protocol, disclosed fit ranges and uncertainty.

The 2026 erratum is a model-identity correction, not a cosmetic detail: the corrected focal heading uses the neighbour set at predicted positions t+dt, but headings at t. The published erratum explicitly notes that the corrected and forward-update results are indistinguishable, while the backward update yields different cluster degree/path-length scaling. Therefore, OLI never pools results across the two conventions.

## Spatial-index correctness gate

The periodic cell index is used in both the simulation step and fast trajectory metrics. At bounded density it is intended to reduce candidate checks; in dense-neighbour and fallback cases the worst case remains O(N²). A new property-style test checks that every brute-force radius neighbour is present in the candidate set across:
- one-, two-, three- and many-cell periodic grids;
- particles close to periodic wrap boundaries;
- several radii including radius comparable to or larger than the box;
- deterministic pseudo-random particle clouds;
- the all-pairs fallback for pathological grid geometries.

Candidate IDs must be sorted and unique before trigonometric accumulation, preserving the reference summation order. This is a correctness gate, not a measured speedup. The separate pinned-hardware scaling benchmark includes index construction, update, fast metrics and full-topology checkpoint costs.

## Benchmark design

- Compare local alignment against self-only and independent-particle controls.
- Compare current-position and next-position neighbour-time rules; never pool results without recording the update convention.
- Measure local and cluster polar order for systems whose opposing clusters cancel at global scale.
- Future parity work against the 2026 topology paper should add clustering coefficient, mean path length, and cluster area/perimeter scaling. These are not yet implemented in the reference module.
- Sweep interaction radius, speed, noise and particle density.
- Report global polar order, local polar-order distribution, neighbour counts and spatial cluster statistics jointly. A globally disordered system may contain highly aligned local clusters whose directions cancel at system scale.
- Measure transient behaviour and stationary windows separately.
- Test time-window and sensor-sampling reductions to expose temporal aliasing.
- Hold out entire parameter regimes, particle counts and, separately, interaction-rule variants.
- Keep all frames from one trajectory in one split; frames are not independent systems.
- Run morphology/adversarial prompts that attempt to promote high polar order into claims of cognition, agency or life.

## Qualification targets (proposed, not achieved)

- Exact replay of the same seed/configuration on the same supported platform.
- Spatial-index candidate sets and indexed metric/visual trajectories must match the all-pairs reference exactly on frozen controls.
- Performance claims require separate, pinned-hardware scaling runs that include index construction, per-step metrics and the full simulation loop.
- The opposed-local-flocks control must show near-zero global polar order alongside local polar order 1 and two disconnected groups.
- The self-only control must produce null local polar order and zero local-order coverage.
- The self-only control must also produce zero nontrivial-cluster coverage and null nontrivial-cluster polar order.
- Opposed multi-particle flocks must produce nontrivial-cluster coverage 1 and nontrivial-cluster polar order 1 despite global order near zero.
- Positive-control polar order within (10^{-12}) of 1 for the all-to-all zero-noise finite fixture.
- Negative-control invariance of polar order in the self-only zero-noise fixture.
- No non-finite positions, headings or metrics across a frozen parameter grid.
- Every claimed regime result backed by at least 30 independent seeds for exploratory statistics; sample-size and finite-size analysis required before inferential claims.
- Zero ontology promotions from polar order alone to learning, agency, intelligence, life or consciousness.
- Exact sham parity: zero downstream effect in the self-only intervention negative control across the full paired trajectory, with common random numbers.
- The paired intervention helper must replay bit-for-bit for identical input state, update rule, intervention and step count.
- Reproduce the analytic all-to-all intervention response `atan2(1,5)` for the treated system and zero response for the sham.
- Keep transfer entropy / predictive dependence and interventional causal effect as separate report fields.
- Report directed visibility edges, reciprocal pair counts and reciprocity fraction for visual-sector runs; never treat weak connectivity as mutual perception.

## Directional visual fields and nonreciprocal visibility

Ramlall, Butail and Roy (2026), *Physical Review E* 113, 064409, DOI [10.1103/bbl5-1yjb](https://doi.org/10.1103/bbl5-1yjb), study a restricted visual field offset from the particle's direction of motion. They report near-perfect alignment and centre-of-mass clustering for a flipped/rear-facing field even under high noise. Their companion dataset is DOI [10.17632/y8fx3shsvx.1](https://doi.org/10.17632/y8fx3shsvx.1), with MATLAB scripts and analysed data, and a dataset-level CC BY 4.0 declaration. The package file inventory, per-file rights and hashes have not yet been acquired; the exact aperture, offset, noise and density grids remain pending extraction from the source.

The reference now supports `VicsekPerception::VisualSector { half_aperture_rad, offset_rad }`:
- offset 0 radians: forward-facing field;
- offset π radians: rear-facing/flipped field;
- neighbour visibility is focal-agent-specific and may be asymmetric;
- local degree and local polar order use each focal particle's directed visible-neighbours;
- graph topology uses an explicitly labelled weakly connected projection (an undirected edge is present if either endpoint sees the other).

Every `VicsekMetrics` snapshot now records perception geometry and graph semantics. It also reports:
- `directed_interaction_edge_count`;
- `reciprocal_interaction_pair_count`;
- `interaction_reciprocity_fraction`, undefined when there are no directed edges.

The exact controls distinguish (1) reciprocal all-to-all metric visibility, (2) a forward visual sector where A sees B but B does not see A, and (3) forward versus flipped angular sectors that see opposite targets. The default `MetricRadius` behavior remains unchanged for existing callers.

This matters because a weakly connected cluster is not a mutually aware collective. A component can be connected by one-way visibility while its members have different information and influence opportunities. OLI must preserve the directed edge data instead of equating graph connectivity with reciprocal interaction.

**Source boundary:** this implementation is an OLI visual-sector reference control, not yet a source-faithful reproduction of the PRE paper. The source's MATLAB scripts and exact parameter grid must be admitted and reproduced before performance comparisons are made.


## Claim ceiling

This contribution adds a reproducible synthetic collective-order control to OLI's abiotic/active-matter tournament. It does not demonstrate natural flocking, biological intelligence, living plasma, or extraterrestrial collective cognition.


## External source reproduction audit (12 November 2026)

The 2026 cluster-topology study has a published erratum (Phys. Rev. Research 8, 049002(E), DOI [10.1103/b9yr-8wmv](https://doi.org/10.1103/b9yr-8wmv)). It corrects the neighbour-set time index: the heading at (t+dt) uses the neighbours at (t+dt), while the neighbour headings/velocities used in the average are at (t). The authors state that the corrected result is indistinguishable from their forward-update implementation, while the backward-update implementation produces different cluster-scaling results.

I inspected the public author repository at commit `68dfc94192d3fb7324eb980ef095da373ddf5ec1`, file `scripts_1_simulation_upto_steady_state/sml_template.cpp` (Git blob `bb483123de3351f75712877ffcf51c21d3b724ec`). Its forward update:
1. predicts positions using headings at (t);
2. rebuilds the periodic cell list using those predicted positions;
3. chooses neighbours at the predicted positions;
4. averages the neighbours' headings at (t);
5. adds angular noise and retains the predicted positions.

This is algorithmically equivalent to OLI's `NextPositionAlignment` and matches the erratum-corrected neighbour-time interpretation.

### Exact interaction boundary

The inspected C++ code uses `dx*dx + dy*dy < r*r`, a strict open-radius disk. OLI now uses the same strict predicate. Exact-radius pairs are excluded; the focal particle is included separately for heading alignment. A regression test covers exact-radius pairs and coincident particles when radius is zero.

### Reproduction differences deliberately retained

The source template uses C++ `float` and `std::mt19937` seeded from `std::random_device`, with no exposed seed. OLI uses `f64` and an explicit xorshift64 stream. The source template's scalar noise `nu` maps to full angular interval (2pi
u); OLI stores the interval directly in radians. Therefore OLI's contribution offers deterministic replay and an algorithmically pinned update rule, but does not claim bitwise trajectory parity with the upstream program.

The inspected template sets (N=2^9=512), (r=1), (L=sqrt{N/ho}), (10^4) time steps and five realizations per noise value. The exact density/speed/noise grids and sibling BW/PRR code files still need source pinning before claiming a complete reproduction of the paper's parameter study.

### Qualification boundary

The 2026 study reports cluster degree, path length, clustering coefficient, area and perimeter scaling. OLI currently implements global/local order, interaction degree/reciprocity, connected components, cluster-size statistics, mean clustering coefficient and mean shortest-path length. It does not yet reproduce the complete cluster-mass scaling analysis or geometric area/perimeter observables. Those are explicit next-step benchmarks, not achieved results.


## Multiscale cluster persistence: H0 interaction-radius filtration

The new `VicsekModel::h0_persistence(max_radius)` method treats the interaction radius as a filtration parameter rather than assuming one fixed threshold is the only meaningful cluster definition.

Every particle begins as a singleton at radius zero. An unordered pair becomes an edge when at least one focal endpoint can perceive the other at a minimum-image distance strictly smaller than the tested radius. For directional visual-sector perception, the H0 graph uses the explicitly declared weakly connected projection: one-way visibility can connect components, but it is not treated as reciprocal influence.

The output contains:
- finite component death/merge radii;
- deterministic elder-rule representative (smallest particle index survives equal-birth merges);
- component size immediately before a merge and merged size after it;
- right-censored components still disconnected at the selected maximum radius;
- the perception geometry and graph semantics used.

This yields a persistence profile: components that merge only at larger scales are structurally more robust to the selected interaction radius than components that disappear at very small scales. It does not prove agency, intelligence or causality. The current checkpoint implementation is (O(N^2log N)); it is not a per-frame metric for large populations.

Regression tests cover analytic pair-merge radii, strict boundary exclusion, one-way visual projection and invariance under global translation across the periodic box. The tests are committed but Rust validation remains pending.


## Periodic-aware convex-envelope baseline

The new `VicsekModel::cluster_geometry()` method reports convex-envelope geometry for each connected component at a registered snapshot.

It:
1. Builds components using the same declared metric or weakly connected visual graph semantics as the current model.
2. Unwraps particle positions through a deterministic breadth-first spanning tree using minimum-image edge displacements.
3. Detects inconsistent cycle closure that implies non-zero winding around the torus.
4. Withholds planar hull area/perimeter if a winding is detected or if unwrapped extent exceeds half the periodic box in either dimension.
5. Otherwise computes a monotone-chain convex hull and reports hull area, perimeter, vertex count and unwrapped spans.

Statuses are explicit: `Valid`, `DegenerateHull`, `PeriodicWindingDetected`, and `PeriodicExtentAmbiguous`. When the periodic image is ambiguous, hull area/perimeter are `None`, not a fabricated value.

This is deliberately a **convex hull baseline**, not the authors' MATLAB alpha shape and not the radius-1 disk-union area. Convex hulls overestimate concave geometry. The source-faithful methods remain separately documented in [cluster geometry profile PR #4843](https://github.com/Luminous-Dynamics/symtropy/pull/4843).

Tests added:
- exact area and perimeter of an analytic right triangle;
- correct unwrapping of a small component crossing the periodic seam;
- rejection of a component whose cycle winds around the torus.

The method is checkpoint-only and currently uses O(N²) pair construction. Rust formatting, compilation and test results remain pending exact-head CI.
