# Independent Rust SCM reference v0

**Status:** code added; source review and Rust validation pending  
**Owner:** Symtropy / `symtropy-lifesim-core`  
**Source profile:** `benchmarks/oli/scm-collective-optimization-source-profile-v0.json`  
**Machine-readable tests:** `benchmarks/oli/scm-reference-cases-v0.json`

## Purpose

This is an independently authored CPU reference for the consensus-coupled adaptive-noise search model described by Nitti et al. (2025). It is deliberately not a port of the MATLAB source. The upstream repository lacks a visible software licence and the Zenodo rights metadata did not expose a licence, so no MATLAB source files are copied or redistributed.

The source audit resolves the social-kernel equation: the published discrete update equation and the pinned MATLAB `SCM/tstep.m` both use `sin((π/2) * delta_x)`. The earlier apparent `2/π` alternative was a misreading and has been removed. The Rust reference now has one `ScmInteractionKernel::PublishedEquationAndMatlabSource` variant. This resolves the formula ambiguity only; source reproduction, RNG parity, supplementary-data reproduction and benchmark validation remain pending.

## Model structure

For each agent, the independent reference combines:
- a sinusoidal all-to-all social interaction term;
- a scalar fitness gradient;
- an agent-specific stochastic term scaled by the distance from the fitness-weighted centroid;
- a global stochastic amplitude adapted from consensus change over a sliding time window.

The update is synchronous and uses an Euler-style drift plus Gaussian increment. Landscape values are dimensionless fitness values in [0,1] to maximize. Out-of-domain points use the declared conical barrier continuation with (H=-0.5), (20°) angle, and zero assigned fitness.

## Numerical and evidence safeguards

1. **Published and MATLAB social kernels agree.** Both use sine scale (π/2); a regression test pins this exact value. No alternate `2/π` kernel is retained.
2. **Zero gradient:** the source gain (\xi=1/\operatorname{mean}(\|\nabla V\|)\) is singular. The reference uses finite zero gradient gain and records the degeneracy.
3. **Zero swarm spread:** if (D_{max}=0), the per-agent noise weights are set to zero and the degeneracy is reported.
4. **Equal fitness:** a zero shifted-weight sum falls back to the unweighted centroid.
5. **Flat consensus trend:** if the two consensus windows are equal, sigma is not updated; the implementation does not evaluate 0/0.
6. **Transactional movement update:** proposed positions, RNG state, consensus history and successor observation cache commit only if all successor observations are valid. Successfully acquired initial observations and their evaluation cost are retained even if a later successor observation fails. A partial successor batch also counts every attempted agent observation through the failing call; agent positions, RNG, and tick remain unchanged. This avoids both partial movement and underreported sensor cost.
7. **Consensus-window semantics:** \`PublishedEqualHalfIntervals\` uses two disjoint equal halves over \`tau_steps\` samples; \`RepositoryMatlabInclusiveEndpoints\` uses \`tau_steps + 1\` samples, an early half of \`tau_steps/2 + 1\`, a late half of \`tau_steps/2\`, and first updates at tick \`tau_steps + 1\`.
7. **Measurement cost:** the first update counts initial observations plus successor observations; later updates reuse the cached observation and count only newly observed positions.

The independent Rust PRNG (xorshift64 + Box-Muller) is not MATLAB's RNG. It supports deterministic Rust replay and independent statistical comparisons, not bitwise trajectory parity with MATLAB.

## Tests added

The source includes seventeen tests for:
- the published and pinned MATLAB social-kernel scale being exactly π/2;
- paper-style and MATLAB-inclusive consensus-window conventions updating at their separately declared first ticks;
- trajectory divergence under the two kernels;
- deterministic Rust replay;
- coincident agents / (D_{max}=0);
- zero-gradient gain;
- zero consensus-window delta;
- out-of-domain barrier observations;
- function-evaluation accounting;
- fail-closed transactional updates;
- dimensionless fitness bounds;
- failed successor observation preserves agent/RNG/tick state while retaining the cost and cache for already acquired initial observations;
- minimum agent count and consensus-window configuration.

These are source-level tests. Rust formatting, compilation, unit execution and exact-head CI remain pending. The seventeen tests are source-level additions only.

## What has not yet been reproduced

- The 100-replication static landscapes from the paper.
- Supplementary Datasets 1–3.
- Matched PSO and MIPA runs.
- The moving contaminant field and AUV dynamic simulation.
- The source MATLAB RNG law and bitwise trajectory values.
- The source-matched consensus-window indexing and benchmark reproduction.

## Competitive plan

The first defensible comparison is an exact matched 2D/3D static task with the same fixed circle-packing start, 100 independent Wiener/noise processes, centroid-to-optimum tolerance of (0.05L), and source stopping rule. Then add dynamic peaks, advective transport, network latency/dropout, sensor calibration drift, agent failure, and energy/range limits.

Measure success probability and resource costs separately: samples/evaluations per success, energy, distance, time, communication, and recovery lag. The research target is a lower cost at matched success or higher success at matched total cost. No improvement is claimed yet.

The full benchmark must report a Pareto frontier rather than hiding SCM's published success-versus-function-evaluation trade-off in one scalar.



## Task fitness is now reported separately from consensus

The per-step report exposes:
- best current agent fitness and its stable index;
- mean current-agent fitness and population variance;
- best fitness ever observed across the initial and successor measurements.

Best-seen fitness is an audit statistic, not an elitist agent reintroduced into the swarm. It remains monotonic even if the current population loses its best candidate. These metrics make it possible to evaluate actual landscape progress without substituting consensus or cohesion.

A topology-budget control test now compares CompleteGraph, Ring and explicit NoCommunication using identical initial positions, seed, steps and objective-evaluation budget. For N agents and T complete updates the exact count is N(T+1). The test does not assume that communication improves objective performance.

The associated OLI benchmark contract is in:
- `benchmarks/oli/collective-competence-scm-v0.json`
- `docs/research/OLI_COLLECTIVE_COMPETENCE_BENCHMARK.md`

It adds a unimodal analytic quadratic and a multimodal periodic landscape; matched communication topologies; random-search and centralized controls to be implemented as separate baseline adapters; success/regret/anytime metrics; a high-consensus/low-fitness adversarial case; and 100 matched seeds for qualification. No performance sweep or improvement claim has been produced yet.

The explicit `NoCommunication` topology always returns empty adjacency, even if two agents occupy identical positions. This is the valid independent-agent/no-message arm; a zero-radius metric graph is not a substitute because coincident positions satisfy the metric distance test.
