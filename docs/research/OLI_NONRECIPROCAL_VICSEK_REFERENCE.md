# OLI nonreciprocal Vicsek reference v0

**Status:** independent Rust reference implemented; exact source reproduction pending  
**Source:** Pang & Wang (2026), *Physical Review E* 114, 024111, DOI [10.1103/rf2v-sy5d](https://doi.org/10.1103/rf2v-sy5d)  
**Module:** symtropy_lifesim_core::nonreciprocal_vicsek  
**Machine-readable profile:** benchmarks/oli/nonreciprocal-influence-source-profile-v0.json  
**Related causal benchmark:** [Symthaea #11614](https://github.com/Luminous-Dynamics/symthaea/issues/11614)

## Why this is a useful OLI control

Pang & Wang explicitly distinguish model-defined influence from statistical information transfer. Their abstract reports that transfer entropy changes nonlinearly with interaction weights because noise applied to influencers can increase information transfer while noise applied to followers can suppress it. The paper presents a physical-model testbed for keeping causal and information-theoretic concepts distinct.

OLI should therefore report at least four distinct outputs:
1. the interaction weights and visibility graph declared by the simulator;
2. the exact per-update model contribution of each source to each receiver;
3. statistical predictive information measures (for example transfer entropy);
4. effects measured by controlled intervention against a matched sham.

None of these alone establishes biological life or intelligence.

## Rust reference semantics

The interaction matrix is row-major:
W[receiver, source] — rows are the receiving particle, columns are the source particle.

For each receiver i, the update is:
[
\theta_i(t+\Delta t)=\operatorname{wrap}\left(
\theta_i(t)+
\frac{\sum_j A_{ij}(t)|W_{ij}|\,\operatorname{wrap}(\theta_j(t)-\theta_i(t))}
{\sum_j A_{ij}(t)|W_{ij}|}
+\epsilon_i(t)
\right).
]

The model-defined influence component is:
[
I_{i\leftarrow j}(t)=
\frac{A_{ij}(t)|W_{ij}|\,\operatorname{wrap}(\theta_j(t)-\theta_i(t))}
{\sum_k A_{ik}(t)|W_{ik}|}.
]

The Rust API preserves:
- periodic minimum-image sensing geometry;
- row-receiver / column-source matrix orientation;
- absolute interaction weights as in the source equation;
- independent per-receiver uniform angular noise;
- explicit seeded randomness for replay;
- per-frame directed model contributions;
- source-style aggregate summaries: mean absolute contribution (A_at) and absolute mean signed contribution (A_ta).

The reset_influence_accumulator() method permits the source's declared 1,000-step burn-in to be excluded before calculating A_at/A_ta.

The model uses scaled row-wise weight normalization to avoid overflow with very large but finite weights while preserving the exact normalized equation. This changes numerical stability, not the mathematical model.

## Frozen analytic controls

1. **Two-agent leader/follower:** with headings 0 and π/2 and row-major weights [1,1,4,1], the analytically expected first-step changes are π/4 for particle 0 and −2π/5 for particle 1.
2. **Zero direct weight:** [1,0,0,1] leaves no inter-agent edge; with zero noise and zero speed, neither particle's heading changes.
3. **A_at versus A_ta:** reverse a directed heading contribution between analysis frames. The mean absolute contribution must exceed the absolute mean signed contribution.
4. **Matched intervention/sham:** clone state and PRNG before changing the leader heading; the follower's next trajectory must differ from sham while random draws remain paired.

## Source reproduction caveat

The inspected source driver creates unseeded NumPy generators for initial state and update noise. Exact trajectory parity is therefore impossible from the published driver alone. OLI must use registered Rust seed sets, compare aggregate curves and confidence intervals, and label this as an independent seeded reproduction—not bitwise source trajectory reproduction.

The source pairwise study uses two particles on a 5×5 periodic square with sensing radius 5, speed 2, dt 0.05, 131,072 steps, a leader/follower weight sweep and an angular-noise sweep. It discards the first 1,000 steps for its information-theoretic analyses. OLI preserves those settings in the source profile and must reproduce the A_at, A_ta, transfer-entropy and noise-asymmetry trends separately.

The source's float16 trajectory storage should be treated as a file-format/analysis detail. The Rust model retains f64 internally; any source-format comparison must apply explicit, documented quantization rather than silently reducing internal precision.

## Claim boundary

The Rust reference is not yet a reproduction of the complete paper analysis: transfer entropy, mutual information, kernel/kNN estimators and partial-information decomposition have not been independently implemented or benchmarked here. The current module is the deterministic dynamics and model-defined influence foundation. Statistical estimators remain separate adapters and must not be called causal proof.

CI, formatting, strict Clippy, Rust tests and source aggregate reproduction remain required before qualification.


## Source-faithful scalar noise versus OLI heterogeneous-noise extension

The inspected pairwise source driver receives one scalar `eta` and samples an independent draw for every particle from the same interval. The new source-faithful constructors retain that exact noise-profile convention.

An additional constructor, `new_with_noise_profile`, accepts one full-width angular-noise interval per particle. This is an OLI extension—not part of the source-faithful pairwise reproduction—and permits controlled influencer-noise versus follower-noise ablations. Every width must be finite and in [0, 2π]. The seeded RNG stream consumes one draw per receiver per update even when that receiver's noise width is zero, preserving draw-count consistency across profile variants.

The frozen tests cover a zero-noise agent paired with a high-noise agent, plus profile length and invalid-value rejection. Results from this mode must be labelled separately from the paper's shared-scalar-eta grid.
