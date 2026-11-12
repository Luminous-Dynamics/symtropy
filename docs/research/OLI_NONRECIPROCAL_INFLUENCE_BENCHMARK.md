# OLI nonreciprocal influence and information-transfer benchmark v0

**Status:** upstream source code pinned; Rust implementation and aggregate reproduction pending  
**Source paper:** Pang & Wang, *Physical Review E* 114, 024111 (7 August 2026), DOI [10.1103/rf2v-sy5d](https://doi.org/10.1103/rf2v-sy5d)  
**Public implementation:** [Pang-Jiah/Influence_and_information](https://github.com/Pang-Jiah/Influence_and_information), MIT, commit `4055debd9711ba46289738c340ee83d50d3797e9`  
**Machine-readable source profile:** `benchmarks/oli/nonreciprocal-influence-source-profile-v0.json`

## Why this is a significant OLI baseline

Pang and Wang explicitly distinguish physical influence from statistical information transfer. Their 2026 paper reports that pairwise influence and transfer entropy can have different relationships depending on noise and interaction weights; noise on influencers and noise on followers have different effects on information transfer. It also reports collective-level two-branch relations and phase-transition information not captured by conventional order parameters alone.

This is directly relevant to OLI's core distinction:
- a directed interaction mechanism is a causal model property;
- a paired intervention measures the model's counterfactual response;
- transfer entropy / mutual information measure predictive information in observed trajectories;
- none of these should be substituted for the others.

## What the upstream code actually implements

Pinned source files include:
- `Vicsek.py`: `non_reciprocal_alignment` and `non_reciprocal_update`;
- `Vicsek_pairwise/Vicsek_pairwise.py`: pairwise trajectory generation;
- `Vicsek_pairwise/parallel_master_Vicsek_pairwise.py`: exact parameter sweep;
- `Vicsek_pairwise/inf_quantities.py`: 8-bin information analysis;
- `information_processor.py`: transfer-entropy and influence estimators;
- `Vicsek_pairwise/Entropy_estimator/information_quantities.py`: kernel and kNN/metric estimator alternatives.

The source uses a directed interaction matrix with **receiver rows and source columns**. For the pairwise one-influencer/one-follower case:
- all matrix weights start at 1;
- the follower's row, leader's column is set to `wLF`;
- the leader's row, follower's column remains 1.

The model updates angles by a row-normalized weighted sum of shortest wrapped angular differences:
[
	heta_i(t+Delta t)=operatorname{wrap}left[
	heta_i(t)+
rac{sum_j A_{ij}(t)|W_{ij}|operatorname{wrap}(	heta_j(t)-	heta_i(t))}
{sum_j A_{ij}(t)|W_{ij}|}
+epsilon_i(t)
ight].
]

The per-edge physical influence component is:
[
I_{ileftarrow j}(t)=
rac{A_{ij}(t)|W_{ij}|operatorname{wrap}(	heta_j(t)-	heta_i(t))}
{sum_k A_{ik}(t)|W_{ik}|}.
]

This is *not* the same as the circular-mean Vicsek rule already implemented in Symtropy. It must be a separate tagged alignment rule to avoid silently changing existing model semantics.

## Source parameter sweep

The pairwise runner uses:
- N=2, one influencer (agent 0), one follower (agent 1);
- square torus L=5, sensing radius R=5;
- speed=2, FPS=20, (Delta t=0.05);
- 131,072 simulation steps, one run per parameter configuration;
- (w_{LF}) in `[1.0,1.1,1.2,1.3,1.4,1.5,1.7,1.9,2,3,4,5,7,9,10,15,20,25,30,40,50,60,70,80,90,100]`;
- noise (eta/pi) in `[0,0.2,0.4,0.6,0.8,1.0,1.2,1.4,1.6,1.8,2.0]`.

For this exact two-agent setup, the maximum minimum-image torus separation is (sqrt{2}L/2approx3.536), below R=5, so the pair is always mutually in range. The interaction is nonreciprocal through W, not through visibility.

## Important reproduction limitation: the source is unseeded

The upstream pairwise program uses `numpy.random.default_rng()` without a supplied seed for the initial state. Its `non_reciprocal_update` function constructs a fresh unseeded generator for per-particle noise on every timestep. Therefore exact stochastic trajectory replay is impossible from the published driver alone.

OLI should:
1. reproduce the deterministic update equations and parameter grid;
2. use a published, fixed seed list in Rust;
3. compare aggregate distributions and trends, not claim exact trajectory parity;
4. report seed-level uncertainty and source-vs-Rust differences explicitly.

## Source analysis and caveats

The source starts information analysis at step 1000, uses lag (	au=1), and bins each angular variable into 8 equal intervals over ((-pi,pi]). It computes TDMI, conditional-MI transfer entropy, mutual information, IMI/PID and SURD quantities. A separate pipeline uses a box-kernel estimator (bandwidth (0.05	imes2pi)) and a kNN/metric estimator with k=4.

The source's pairwise influence indicators are:
- `A_at`: time mean of the absolute per-step influence contribution;
- `A_ta`: absolute value of the time-mean signed influence contribution.

The pairwise script passes an all-ones adjacency tensor into these two statistics. This is compatible with its pairwise source configuration because R=5 and L=5 guarantee all-to-all visibility, but is not a valid general definition for local-neighbour cases.

The upstream trajectory writer stores orientation, noise, influence and velocity as float16; position is float32. The Rust model should keep f64 internally and quantize only when explicitly comparing to source formats.

## OLI's paradigm shift: influence must survive intervention

The source's pairwise influence component is a mechanism-defined contribution. OLI should additionally estimate:
- direct intervention effect of perturbing the influencer on the follower;
- reverse intervention effect of perturbing the follower on the influencer;
- matched sham trajectories with common random numbers;
- noise-on-influencer versus noise-on-follower sweeps;
- transfer entropy and PID/SURD side-by-side, clearly labelled as statistical information measures.

The key red-team test is whether an estimator reports high transfer entropy when the physical coupling weight is zero, or misses a nonzero intervention effect because noise, sampling or discretization obscures it.

## Claim ceiling

This is a reproducible two-agent mathematical model. It does not establish social intelligence, biological agency, life, or consciousness. Influence weights, counterfactual responses and transfer entropy remain distinct measurements with distinct assumptions.
