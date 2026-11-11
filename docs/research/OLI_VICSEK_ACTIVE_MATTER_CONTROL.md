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
- Jin & Dong (2026), *Communications Physics*, [Quantitative relationship between coupled structural dynamics and collective order in the original Vicsek model](https://www.nature.com/articles/s42005-026-02933-9).

## Scientific boundary

This is a synthetic, known-rule active-particle model. It contains:
- no metabolism or self-maintaining biological boundary;
- no reproduction, inheritance or evolution;
- no memory or learning;
- no task objective or environmental problem-solving;
- no evidence of sentience or consciousness.

A high polar-order score demonstrates collective directional alignment under local interaction rules. It does not establish biological life, agency, cognition, intelligence or consciousness. It is not a validated model of a particular animal flock, bacterial swarm, plasma, or extraterrestrial system.

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
- polar order parameter;
- mean neighbour count excluding self;
- neighbour-count variance;
- mean and variance of per-particle local polar order;
- connected-component count in the metric-neighbour graph;
- largest connected-component fraction.

## Exact positive and negative controls

1. **All-to-all, zero-noise alignment:** six particles with different initial headings, interaction radius spanning the entire torus and zero noise must produce polar order 1 after one update.
2. **Self-only, zero-noise control:** radius zero means each particle only aligns with itself; polar order must remain unchanged, mean neighbour count excluding self must be zero, and each particle remains its own graph component.
3. **Seeded low-noise exploration:** 256 particles, (L=32), (R=1), (v=0.03), (eta=0.2), seed 20261111, 2,000 steps.
4. **Seeded high-noise control:** same finite system with (eta=4.0), seed 20261112, 2,000 steps.

The low/high noise runs are exploratory finite-size cases. No universal critical noise threshold is hard-coded or asserted. A complete phase diagram requires a pre-registered parameter grid, multiple independent seeds per point, finite-size scaling and uncertainty analysis.

## Benchmark design

- Compare local alignment against self-only and independent-particle controls.
- Sweep interaction radius, speed, noise and particle density.
- Report global polar order, local polar-order distribution, neighbour counts and spatial cluster statistics jointly. A globally disordered system may contain highly aligned local clusters whose directions cancel at system scale.
- Measure transient behaviour and stationary windows separately.
- Test time-window and sensor-sampling reductions to expose temporal aliasing.
- Hold out entire parameter regimes, particle counts and, separately, interaction-rule variants.
- Keep all frames from one trajectory in one split; frames are not independent systems.
- Run morphology/adversarial prompts that attempt to promote high polar order into claims of cognition, agency or life.

## Qualification targets (proposed, not achieved)

- Exact replay of the same seed/configuration on the same supported platform.
- Positive-control polar order within (10^{-12}) of 1 for the all-to-all zero-noise finite fixture.
- Negative-control invariance of polar order in the self-only zero-noise fixture.
- No non-finite positions, headings or metrics across a frozen parameter grid.
- Every claimed regime result backed by at least 30 independent seeds for exploratory statistics; sample-size and finite-size analysis required before inferential claims.
- Zero ontology promotions from polar order alone to learning, agency, intelligence, life or consciousness.

## Claim ceiling

This contribution adds a reproducible synthetic collective-order control to OLI's abiotic/active-matter tournament. It does not demonstrate natural flocking, biological intelligence, living plasma, or extraterrestrial collective cognition.
