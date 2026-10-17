# CIV-INFRA Decision-Impact Measurement Planning v0.1

**Status:** draft / synthetic software-evaluation contract  
**Workstream:** CIV-INFRA-DECISION-001 / R2  
**Implementation:** `tools/civ-infra-decision-impact-planner/`  
**Related:** [CIV-INFRA-ORACLE-002 #1777](https://github.com/Luminous-Dynamics/symtropy/issues/1777), [decision-planning tracker #1789](https://github.com/Luminous-Dynamics/symtropy/issues/1789), [primitive-derived evaluator PR #1784](https://github.com/Luminous-Dynamics/symtropy/pull/1784), [robust oracle PR #1772](https://github.com/Luminous-Dynamics/symtropy/pull/1772), [solver-backed drainage evidence #1764](https://github.com/Luminous-Dynamics/symtropy/issues/1764).

## 1. Paradigm shift

Conventional Pareto analysis asks which candidates are non-dominated under a chosen set of objectives. Conventional value-of-information analysis asks how valuable an observation is under a decision model, usually with probability assumptions. Both are useful, but neither automatically answers the operational question:

> Given what we do not know, which measurement can most reliably change the next engineering decision, in the worst case, and what uncertainty remains afterward?

CIV-INFRA R2 starts with exact finite-state *decision signatures* from the upstream evaluator and candidate measurements with declared outcome partitions. It ranks actions by residual decision ambiguity and state reduction without inventing probabilities or collapsing safety, performance, time and cost into an opaque weighted score.

This is a capability layer around Pareto/robust analysis, not a claim that Pareto analysis is obsolete. Robust Pareto results describe candidate trade-offs; decision-impact planning chooses what evidence to obtain next.

## 2. Scope and claim ceiling

Inputs contain:

- A versioned set of candidate IDs.
- A finite list of named states. Every state has a candidate-by-candidate partition into feasible, infeasible or unknown, plus a frontier subset of feasible candidates.
- A reference to the upstream case/report that produced the decision signature.
- Candidate survey/measurement actions. Each action supplies a required outcome-model reference, an optional SHA-256 digest, and non-empty outcome buckets that must partition the complete state set exactly once.
- Optional cost and duration estimates, with explicit units and a provenance basis.

The v0.1 planner verifies the internal consistency and decision consequences of these supplied signatures and partitions. It **does not** independently derive engineering constraints, verify an upstream report blob or outcome-model artifact, predict the real outcome of a proposed survey, or establish that a measurement is feasible or safe. Optional upstream-report and outcome-model digests are syntax-checked and preserved; they are not looked up in an artifact store. Those are explicit future integration requirements, not silently assumed capabilities.

The frozen input is synthetic. Cost/time numbers are intentionally invented solely to test retention and reporting. No hydrology, utility, terrain or construction solver is invoked.

## 3. Metrics and ordering

For \(n\) states, let \(n_j\) be the count sharing decision signature \(j\). Define the number of unordered state pairs that disagree on the supplied decision signature:

\[
D = {n \choose 2} - \sum_j {n_j \choose 2}
\]

For each outcome bucket \(o\), calculate \(D_o\) the same way, using only states in that bucket. Those discordant pairs cannot yet be distinguished after that outcome. The planner reports:

- Worst- and best-case remaining state counts.
- Worst-case distinct decision-signature classes remaining.
- Worst-case decision-discordant pairs remaining in an outcome.
- Homogeneous outcomes/states.
- Total state pairs with decision disagreement before observation.
- The exact count of decision-discordant pairs separated into different outcome buckets.

These are combinatorial counts on an explicitly supplied finite set. They are **not probabilities**, expected value, calibrated confidence, causal effects or claims about likelihood.

The default ordering is lexicographic and printed into every report:

1. Minimize the worst-case number of distinct decision signatures after an outcome.
2. Minimize the worst-case number of decision-discordant state pairs remaining.
3. Minimize worst-case remaining state count.
4. Maximize the count of decision-discordant state pairs separated.
5. Break exact ties by stable action ID.

Estimated cost and duration are emitted side-by-side, with units and the basis of each estimate. They are not smuggled into a total utility score. A later policy could add an explicit budget constraint or report a multi-objective Pareto set over impact, cost and elapsed time; that policy should be separately versioned and tested rather than introduced through hidden weights.

## 4. Fail-closed requirements

The evaluator rejects:

- Unsupported schema versions and unknown JSON fields.
- Empty or duplicate candidate, state, action or outcome identifiers; a missing outcome-model reference.
- A candidate missing from the feasible/infeasible/unknown classification, or assigned to more than one class.
- A frontier candidate not in that state's feasible class.
- An outcome with no states.
- Unknown state references, duplicate state membership, or a partition that omits any declared state.
- A malformed supplied upstream-report or outcome-model SHA-256 digest, invalid estimates, and oversized evaluation workloads. Model digests are references only until a future adapter retrieves and verifies the exact bytes.

It does not fill an absent state with a zero-probability guess, implicitly infer independence, normalize an incomplete partition, or treat unknown candidates as infeasible. If a state signature contains unknown candidate status, that uncertainty remains visible in the signature and therefore in the decision ambiguity metrics.

The planner caps input at 100,000 states, 10,000 actions, 100,000 outcome buckets, and 10,000,000 state-action evaluations.

## 5. Benchmark plan and acceptance targets

| Gate | Target | Evidence |
|---|---|---|
| Frozen fixture | Exact known ordering and metrics | Small, reviewed four-state corpus |
| Partition soundness | 100% rejection of missing, duplicate, empty and unknown-state partitions | Mutation/negative-control tests |
| Signature integrity | 100% rejection of class overlap, missing candidates and infeasible frontier members | Negative-control tests |
| Independent oracle | Exact equality on all generated worlds | Pairwise, deliberately simpler reference implementation |
| Initial scale | 100 synthetic worlds × 20 states × 5 actions = 10,000 state-action evaluations | Complete input snapshots, hashes, oracle metrics and reports in CI artifact |
| Determinism | Same exact input bytes yield identical serialized report; state/action/outcome permutations preserve metrics and ranking | Replay/permutation tests |
| Probability discipline | Zero probabilities or expected-VoI fields emitted in v0.1 | Schema and output assertions |
| Work bound | Inputs above 10,000,000 state-action evaluations reject before action evaluation | Boundary/negative-control test |
| Performance disclosure | Report wall time, corpus size and artifact SHA; later record p50/p95/p99 and peak memory at 10k/100k/1M evaluations | CI receipt; no performance claim before measurement |

The independent oracle uses pairwise comparisons on the finite small worlds, rather than reproducing the production metric's group-count calculation. Generated-corpus tests compare worst-case remaining states, worst-case decision classes, worst-case unresolved discordant pairs, and pairs separated. CI preserves input snapshots and digests so a future regression can be replayed rather than merely summarized as a pass/fail count.

### Later gates (not implemented in v0.1)

1. **Sequential planning:** evaluate post-observation states, remaining budget and stop/continue decisions; compare the chosen policy with exhaustive search on tiny worlds.
2. **Observation-model calibration:** compare predicted outcome sets with observed survey results across held-out cases. Report out-of-support rates and stratified performance; never treat a synthetic partition as field validation.
3. **Real engineering evidence:** admit SWMM or other solver-backed outcomes only with exact model, input, solver-build, raw-output and diagnostic digests and declared applicability limits (#1764).
4. **Probability-based VoI:** only after prior distributions and observation likelihoods are justified, independently calibrated, versioned and reviewed. Report assumptions and sensitivity instead of hiding them.
5. **Decision utility and constraints:** encode hard safety constraints before any cost trade-off. If multiple objectives matter, publish the Pareto set or a user-approved lexicographic policy; avoid magic scalar weights.

## 6. Research basis

- NIST, [*On Methods of Uncertainty Quantification in Computational Modeling and Simulation: Harmonizing Modeling and Metrology Practices*](https://www.nist.gov/publications/methods-uncertainty-quantification-computational-modeling-and-simulation-harmonizing), published 5 October 2026. The risk-based credibility framing scales verification, validation and uncertainty quantification with model influence and decision consequence; this motivates explicit claim ceilings and evidence-specific qualification.
- Mishra et al., [*Beyond uncertainty in modern active learning for trustworthy AI*](https://doi.org/10.3389/frai.2026.1844765), published 19 June 2026. The review highlights cost, redundancy, distribution shift, human oversight and workflow-grounded evaluation—reasons not to rank measurements by a single generic uncertainty number.
- Borgonovo et al., [*Fifty years of decision analysis in operational research: A review*](https://doi.org/10.1016/j.ejor.2025.05.023), published in 2026. Its coverage of information acquisition, value-focused thinking and uncertainty analysis provides the decision-analysis context; this prototype intentionally does not claim expected VoI without a probability model.

The sources motivate the evaluation questions; they do not validate this implementation, and the synthetic benchmark does not reproduce or establish field performance from those papers.

## 7. Current evidence state

This branch contains an implementation, a frozen synthetic fixture, tests, and a focused workflow. Until the workflow completes successfully on the exact PR head, the Rust code is **authored but not verified by CI**. Queued, absent or skipped checks are not passes. No engineering decision is authorized by this tool's output alone.
