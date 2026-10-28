# Glass–Silicon Constrained Pareto and Multi-Fidelity Benchmark v0.1

**Status:** synthetic executable benchmark; exact-head runtime qualification pending  
**Benchmark ID:** \`glass-pareto-fidelity-synthetic-v0-1\`  
**Claim ceiling:** accounting, hard-constraint, Pareto-scoring, and deterministic replay mechanics only. The fixture does not establish glass/silicon process performance or superiority over published methods.

## Why this lane exists

The scalar adaptive-search benchmark is useful for checking decision accounting, but scalar utility can hide trade-offs. A real process may need to maximize product quality while minimizing energy, throughput loss, rework, or cost. Hard qualification is different: no energy or cost improvement may compensate for a product-specification or safety failure.

Fidelity also matters. A cheap screening result can rank candidates, but must not certify a hard quality constraint unless that measurement method is explicitly qualified to do so. A screening policy should earn credit for a feasible Pareto candidate only after confirmatory evidence establishes feasibility.

This first lane is deliberately synthetic and small enough for known-answer tests. It validates the contract and scoring mechanics before any licensed physical dataset is admitted.

## Literature and industry comparators

1. **Glass-specific multi-objective Bayesian optimization.** Mende et al., *Multi-Objective Lookahead Bayesian Optimization for Process Parameter Optimization in Non-Isothermal Glass Molding*, Procedia CIRP (2025), DOI [10.1016/j.procir.2025.02.137](https://doi.org/10.1016/j.procir.2025.02.137). This is a particularly relevant comparator for optical glass molding. Its specific task and objectives are not automatically representative of container glass or metallurgical-grade silicon.
2. **Cost-aware multi-objective batch BO.** Alvi et al., *Deep Gaussian process-based cost-aware batch Bayesian optimization for complex materials design campaigns*, npj Computational Materials (2026), [article 105](https://www.nature.com/articles/s41524-026-01981-7). The paper combines correlated multi-output surrogates, q-expected hypervolume improvement, and fidelity/query cost.
3. **Multi-fidelity method selection.** Sabanza-Gil et al., *Best practices for multi-fidelity Bayesian optimization in materials and molecular research*, Nature Computational Science (2025), [DOI 10.1038/s43588-025-00822-9](https://doi.org/10.1038/s43588-025-00822-9). The benchmark must test when a cheap source is weak or misleading as well as when it is useful.
4. **Adaptive constrained acquisition policy.** Sinha et al., *Portfolio-Based Constrained Multi-Objective Bayesian Optimization for Materials Design*, arXiv (17 September 2026), [arXiv:2609.19550](https://arxiv.org/abs/2609.19550). This recent preprint compares fixed acquisitions with a bandit-style acquisition controller and an LLM-driven controller across synthetic and materials tasks. Treat it as a high-priority replication target, not a result transferable to glass without task-matched reproduction.
5. **Capability-compiled agentic BO.** Zeng et al., *Agentic-MatriBO: Capability-Compiled Agentic Bayesian Optimization for Materials and Chemical Design*, arXiv (13 October 2026), [arXiv:2610.17832](https://arxiv.org/abs/2610.17832). Its proposed separation between a language policy, compatibility/capability checking, and deterministic execution is directionally relevant: policy proposals should not bypass the evaluator or hard gates.

The practical industry baseline is not only a named BO paper. Where a rights-cleared task permits it, compare against the incumbent engineering/DOE workflow, the best reproducible compatible BO stack, fixed-fidelity and random-fidelity controls, and the cost of all model/experiment work. Mark baselines NOT_COMPARABLE where the candidate pool, target, or observation interface differs materially.

## Frozen synthetic task contract

Public input:
- \`tools/glass-silicon-contract-audit/fixtures/glass-pareto-fidelity-task.synthetic.json\`
- \`docs/tech/glass-pareto-fidelity-task-v0.1.schema.json\`

Evaluator-only input:
- \`tools/glass-silicon-contract-audit/fixtures/glass-pareto-fidelity-oracle.synthetic.json\`
- \`docs/tech/glass-pareto-fidelity-oracle-v0.1.schema.json\`

There are five synthetic candidate families and three regimes: \`informative\`, \`neutral\`, and \`misleading_low_fidelity\`. The public task freezes:
- two objectives, quality (maximize) and synthetic energy (minimize);
- a Pareto reference point of quality 0 and energy 10;
- the qualification hard gate;
- a maximum campaign cost of 5.5 synthetic units and duration of 9 synthetic seconds;
- five public candidate descriptors and a deterministic random seed.

The oracle owns the fidelity tariff table and all outcomes. A screen costs 0.5 units / 1 second; confirmatory evaluation costs 2 units / 3 seconds. Each campaign is charged against the same evaluator-owned tariffs. The task's units are synthetic and must not be converted to real money, energy or elapsed plant time.

## Policies actually executed

- \`confirmatory_priority\`: spends its budget on confirmatory candidates, chosen from public priority.
- \`screen_then_confirm\`: screens three candidates, then confirms screened candidates by the revealed screening objectives.
- \`screen_only\`: uses screening alone and must never report a qualified candidate or a feasible Pareto front.
- \`random_confirmatory\`: selects confirmatory candidates in a deterministic seed-derived order.

These are executable policy controls, not state-of-the-art implementations. No arm calls an LLM or fits a Gaussian process. This initial stage tests whether a richer policy may be evaluated safely—not whether any policy wins on real materials.

## Non-negotiable evaluator rules

1. **Hard constraints are lexicographically prior to reward.** Only confirmatory observations can establish \`qualification=true\`. A screen can guide where to spend next, but cannot certify feasibility.
2. **Costs and duration come from the sealed tariff table.** Action cost is charged before the selected outcome is exposed; policy output cannot set its own price.
3. **Public and evaluator-only data are separate byte artifacts.** Reports bind both by SHA-256.
4. **Outcomes must be complete.** Missing candidate/fidelity outcomes yield UNRESOLVED; they are not converted to zero, failed feasibility, or free retries.
5. **Pareto membership is based only on confirmed feasible candidates.** An infeasible candidate with excellent objectives is still excluded from the accepted front.
6. **Hypervolume definitions are fixed.** Objective directions and the reference point are validated rather than tuned after observing outcomes.
7. **Replay is exact.** Re-running the deterministic policy loop from the exact public/oracle bytes must reproduce actions, decision-view digests, observations, costs, feasibility, Pareto membership, hypervolume, and regret.

## Metrics

Primary synthetic mechanics:
- feasible Pareto candidate IDs and observed hypervolume;
- oracle Pareto hypervolume and hypervolume regret;
- Pareto recall against the sealed feasible front;
- confirmed infeasible selections and cost/duration totals.

The oracle front and regret are evaluator-only report fields: they are not included in policy observations. They are scoring references, not legitimate online policy inputs.

For future admitted, real tasks, the target scorecard should include:
- hypervolume improvement per fully charged experimental/compute cost and elapsed time;
- cost/time to first independently qualified Pareto candidate;
- constraint false-positive rate with a predeclared confidence bound;
- fidelity-choice regret / value of information per unit cost;
- invalid, censored, failed, repeat and abstention rates;
- robustness stratified by helpful, neutral and misleading low-fidelity regimes;
- paired task/seed effects and uncertainty intervals.

Hypervolume cannot be compared across tasks with different units, objective scaling, or reference points unless a preregistered normalization contract makes them comparable. Do not infer statistical significance from deterministic synthetic fixture rows.

## Reproduce locally

\`\`\`sh
cargo test --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-pareto-fidelity

cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-pareto-fidelity -- \\
  tools/glass-silicon-contract-audit/fixtures/glass-pareto-fidelity-task.synthetic.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-pareto-fidelity-oracle.synthetic.json > glass-pareto-fidelity-report.json

cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-pareto-fidelity -- \\
  --replay glass-pareto-fidelity-report.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-pareto-fidelity-task.synthetic.json \\
  tools/glass-silicon-contract-audit/fixtures/glass-pareto-fidelity-oracle.synthetic.json
\`\`\`

## Promotion gates

1. Pass the exact-head Rust suite, schema checks, known-answer output checks and transcript replay.
2. Add true model-based baselines: constrained multi-objective lookahead BO, qEHVI/qNEHVI where compatible, cost-aware multi-fidelity BO, random/DOE, fixed-fidelity, and the incumbent engineering workflow.
3. Add paired tasks where low fidelity helps, ties, and harms; include noisy, censored, invalid, delayed, correlated and distribution-shifted measurements. Do not tune the hidden oracle after observing policy results.
4. Admit rights-cleared real task data with raw-file digests, reproducible preprocessing, family/source/campaign-held-out splits, and reproduction of the literature comparator when compatible.
5. Claim a state-of-the-art improvement only after the strongest compatible baseline is beaten under equal fully charged budgets with preregistered endpoints and uncertainty intervals. Physical/industrial impact requires prospective confirmation and independently measured accepted-grade output.

**Current conclusion:** this benchmark is a test harness for hard constraints, multi-objective scoring, fidelity charges, and replay. Its synthetic outcomes intentionally make screening helpful in one regime and misleading in another. That validates the intended test conditions only; it does not show that Symtropy beats current materials-optimization methods.
