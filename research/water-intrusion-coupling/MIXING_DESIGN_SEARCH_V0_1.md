# Finite Experiment and Sensor Subset Selection v0.1

**Programme:** WATER-MIXING-DESIGN-SEARCH-001 / [#5259](https://github.com/Luminous-Dynamics/symtropy/issues/5259)  
**Parent design qualification:** [#5226](https://github.com/Luminous-Dynamics/symtropy/issues/5226)  
**Implementation:** synthetic Rust evaluator mixing-design-search-t0.rs  
**Evidence ceiling:** finite synthetic subset search only

## 1. The question is selection, not another scorecard

The first design evaluator scores a small set of complete candidate experiment plans. This new stage chooses a set of experiment blocks from a finite candidate pool.

Each block is a self-contained planned measurement bundle with a precomputed Fisher-information matrix after declared sensitivity scaling and measurement-covariance whitening. A block also carries pairwise model-separation squared distances, resource use, sensor identities, an explicit cross-block noise-independence contract and a digest of the block receipt content.

The search combines blocks, checks nuisance identifiability after aggregation, counts shared sensors once, applies resource budgets, and compares exact, greedy and random selection.

**Boundary:** the search trusts the supplied per-block information and model-separation sufficient statistics after verifying receipt digests and structural contracts. It does not recompute their underlying sensitivity matrices or covariance whitening. The upstream design evaluator and physical-data qualification remain separate.

## 2. Aggregate information before eliminating nuisance parameters

For block b, let the declared full information matrix be partitioned into target and nuisance parameters:

\[
F_b =
\begin{bmatrix}
F_{tt,b} & F_{tn,b}\\
F_{nt,b} & F_{nn,b}
\end{bmatrix}
\]

For a selected set S of independent blocks:

\[
F_S = \sum_{b\in S}F_b
\]

Then form the target information after nuisance adjustment:

\[
F_{t\mid n,S} = F_{tt,S} -
F_{tn,S}F_{nn,S}^{-1}F_{nt,S}
\]

The order is mandatory. Do not calculate a Schur complement for each block separately and sum the adjusted outputs. A block may have singular nuisance information by itself, while another independent experiment identifies a different nuisance direction. Their combined nuisance block can be full-rank and support an estimable target.

If the aggregate nuisance block is not positive definite at the declared tolerance, return NUISANCE_UNRESOLVED. Do not add an undeclared ridge or prior.

## 3. Cross-block covariance contract

The v0.1 exact search combines Fisher matrices by addition and pairwise Mahalanobis separation squared by addition:

\[
\delta^2_{S,ab} = \sum_{b\in S}\delta^2_{b,ab}
\]

This is valid only under an explicit independence assumption for the measurement errors across selected blocks, with each block's own covariance already represented in its sufficient statistics.

Every block must declare cross_block_noise_contract = independent. Unknown or correlated cross-block errors are rejected by v0.1. A future version may accept a full joint covariance across blocks and recompute the joint whitened information, but it must not silently assume independence.

Shared sensor IDs are counted once for instrumentation cost. Shared sensor identity does not itself make observation errors independent; the declared covariance contract remains a separate assumption requiring physical evidence.

## 4. Resource constraints

Every subset is tested against separate hard budgets:

- total test cost;
- unique sensor count;
- total experiment duration;
- calibration burden.

Sensor IDs are unioned across selected blocks. Cost, duration and calibration units are additive in v0.1; real campaigns may later need reusable-calibration or setup-cost models. Those are not inferred by the T0 evaluator.

A subset violating any resource cap is ineligible even if its Fisher information is excellent.

## 5. Exact, greedy and random algorithms

### Exact enumeration

For N candidate blocks, enumerate all 2^N - 1 non-empty subsets. V0.1 is deliberately capped at **12 blocks** (4,095 non-empty subsets). Above that cap the evaluator refuses exact mode rather than silently truncating or switching algorithms.

For each subset, compute aggregate information, aggregate nuisance adjustment, target singular values/rank/condition, D-optimal log determinant, pairwise model separation, unique sensors and resource totals. The full eligible set and its Pareto frontier are compared against the frozen oracle.

### Greedy D-optimal baseline

The greedy baseline selects the next block with the largest incremental regularized joint-information log determinant per cost unit. A tiny numerical ridge is used solely to rank intermediate singular subsets; it is not included in the final qualification metrics or target covariance.

This baseline is intentionally not model-discrimination-aware. A case where greedy spends the full budget on a high-information but non-discriminating experiment is a required negative control.

### Seeded random baseline

The random baseline runs a declared number of deterministic seeded block-order permutations, greedily adding a block when all hard budgets remain satisfied. It reports the best eligible result by target D-optimality. The seed, trial count and result must be replayable.

The exact search is the reference for these small pools. Greedy/random are baselines, not evidence of global optimality.

## 6. Frozen counterexample

The five-block corpus is intentionally constructed to separate raw information from experiment usefulness:

- two inlet-excitation blocks each confound a target direction with a different nuisance direction;
- two calibration blocks identify the separate nuisance directions;
- a fifth block has a much larger joint information determinant but insufficient separation between the declared models.

The exact subset selects the first four blocks. The nuisance information of the selected set becomes estimable only after aggregation; its adjusted target information has two equal non-zero directions. The selected set uses four experiment blocks but only **three unique sensors**, because the two excitation blocks share one sensor identity.

The D-optimal-only greedy baseline selects the high-information fifth block and spends the budget, but its model-separation score remains below threshold. It is therefore rejected. This is the intended counterexample: maximizing information without model discrimination can choose the wrong physical experiment.

The independent random baseline is compared under the same cost, sensor, duration and calibration budgets.

## 7. Frozen T0 controls

- Exact enumeration of every non-empty subset in the five-block pool.
- Exact best eligible subset and complete eligible-subset identity.
- Pareto frontier membership.
- Greedy high-information / low-discrimination trap.
- Seeded random baseline replay.
- Nuisance directions unidentifiable per block but estimable after aggregation.
- Shared sensor IDs counted once.
- Correlated/unknown cross-block noise contract refusal.
- Invalid, duplicate and changed block receipt digests.
- Non-positive-semidefinite block information rejection.
- Invalid model-pair keys and non-finite separation scores.
- Hard resource-budget enforcement.
- Exact-search cap and fail-closed overflow behavior.

## 8. Benchmark targets

Research targets, not achieved:

| Dimension | Target |
|---|---|
| Exact small-pool correctness | 100% frozen subset/oracle agreement |
| Subset enumeration | Exact results for all pools up to declared limit |
| Nuisance handling | Aggregate information before Schur complement; zero false-identifiable decisions |
| Shared instrumentation | No double-counting shared sensor identities |
| Greedy comparison | Publish optimality gap and cases where greedy violates model-discrimination requirements |
| Random comparison | Fixed seeds, repeated trials and cost-matched distributions |
| Pareto | Publish information, model separation, cost, sensors, duration and calibration trade-offs |
| Scaling | Benchmark search runtime and memory as candidate count approaches the exact cap |
| Evidence | Every information block digest-verified; no physical authority from synthetic output |

## 9. Next scaling step

After T0 correctness, build a branch-and-bound or admissible-pruning exact solver and compare it with the exhaustive oracle on small pools. For larger pools, add:

- lazy greedy and submodular-greedy baselines only after checking whether their assumptions hold for nuisance-adjusted information and model discrimination;
- multi-objective evolutionary or local-search baselines with frozen seeds;
- robust optimization across geometry/flow strata;
- joint covariance across experiment blocks;
- explicit setup/reuse cost and calibration reuse;
- sequential design where each new measurement changes the posterior and the next candidate pool.

Do not claim a greedy approximation guarantee unless the actual combined objective is proven to have the required monotonicity/submodularity properties. Nuisance Schur complements and multi-model discrimination can invalidate naive assumptions.

## 10. Evidence ceiling

T0 demonstrates synthetic combinatorial and information arithmetic only. It does not validate local sensitivity matrices, instruments, sensor independence, physical model hypotheses, tracer experiments, a junction or tank, potability, contamination, toxicology, SANS 241 compliance or release authority.

T1 requires a controlled non-potable test loop, qualified instrumentation, complete provenance, pre-registration, competent oversight and approved tracer compatibility/handling/disposal.
