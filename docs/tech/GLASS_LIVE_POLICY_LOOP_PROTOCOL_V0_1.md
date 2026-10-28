# GLASS-LIVE-POLICY-LOOP-001 — observation-conditioned search-space benchmark v0.1

Status: executable synthetic event-loop contract. Not an empirical materials result, a model benchmark result, or authorization to run equipment.

Parent: [adaptive search-space protocol](./GLASS_ADAPTIVE_SEARCH_SPACE_BENCHMARK_V0_1.md), [issue #2788](https://github.com/Luminous-Dynamics/symtropy/issues/2788), and [closed-loop runtime-isolation issue #2785](https://github.com/Luminous-Dynamics/symtropy/issues/2785).

## Why this is a separate benchmark layer

A static list of pre-authored actions can test budget arithmetic and hidden-outcome scoring, but it cannot show whether a policy actually changes its next action when it receives new evidence. This layer runs a deterministic policy loop. The policy gets only the public catalog, currently visible candidate descriptors, budget remaining, and observations actually returned by earlier actions. The evaluator charges a chosen action before looking up and revealing its result. The next policy decision is then computed from the updated public view.

The sealed oracle includes environment identity, candidate feasibility, utilities and evaluator-owned tariffs. None of these fields are copied into the policy view. The policy chooses an action from typed public state; the evaluator separately executes that action and supplies a result. Tests mutate unobserved oracle outcomes and require action choices to remain unchanged until the corresponding observation is revealed.

**Trust boundary status:** the default CLI now launches a fresh child process for each policy action. It sends a single JSON request over stdin containing only the registered policy class, reconstructed public view, and expansion-decision state; it supplies no catalog/oracle file path, clears environment variables, uses a temporary working directory, and accepts one strict JSON action over stdout. The parent evaluator retains the sealed oracle and charges/reveals outcomes after the child action returns. Independent replay reconstructs each view and checks that the chosen action matches the registered policy for that view.

This is a stronger, executable process/API boundary, but **not yet a security sandbox**: the child still inherits the host OS user's filesystem and network permissions and could access known absolute paths if arbitrary untrusted code were substituted. A production hostile-code lane still needs an OS-enforced filesystem/network boundary (or an equivalently constrained WASM runtime), resource limits, strict protocol validation, and replay from a separately trusted evaluator.

## Executable artifacts

- CLI: `tools/glass-silicon-contract-audit/src/bin/run-search-space-loop.rs`
- Public catalog: `tools/glass-silicon-contract-audit/fixtures/search-space-live-policy-catalog.synthetic.json`
- Sealed oracle: `tools/glass-silicon-contract-audit/fixtures/search-space-live-outcome-oracle.synthetic.json`
- Public schema: `docs/tech/glass-search-space-live-catalog-v0.1.schema.json`
- Sealed-oracle schema: `docs/tech/glass-search-space-live-oracle-v0.1.schema.json`

Run from repository root:

    cargo test --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml
    cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin run-search-space-loop -- tools/glass-silicon-contract-audit/fixtures/search-space-live-policy-catalog.synthetic.json tools/glass-silicon-contract-audit/fixtures/search-space-live-outcome-oracle.synthetic.json

    cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin run-search-space-loop -- --replay search-space-live-loop-report.json tools/glass-silicon-contract-audit/fixtures/search-space-live-policy-catalog.synthetic.json tools/glass-silicon-contract-audit/fixtures/search-space-live-outcome-oracle.synthetic.json

The normal run command can be redirected to `search-space-live-loop-report.json` before replay. The `--replay` mode independently rebuilds each public decision view and the registered deterministic policy's expected next action from that view, then validates action ordering/visibility and proposal digests, recomputes evaluator charges, verifies observations against the sealed oracle, and recomputes each episode summary. A legal-but-policy-inconsistent action fails replay. It fails closed on forged observations, under-reported costs, stale view hashes and mismatched input/oracle byte digests. The replay code path is separate from policy execution and does not trust the policy's reported totals; it is still in the same evaluator executable and does not replace the still-required OS-enforced sandbox.


## Policy arms are software controls, not claimed SOTA implementations

The runner executes six deterministic policy controls:

1. `fixed_space`: never expands; chooses candidates using declared public selector scores.
2. `gp_ei_fixed_space`: never expands; fits an RBF Gaussian process to revealed utility observations over public 2D candidate features and ranks visible candidates by expected improvement. Hyperparameters are fixed in source for this synthetic finite-pool baseline: lengthscale 0.85, observation-noise variance 0.05, utility normalized by 10, and exploration offset xi 0.01.
3. `random_expansion`: after the common first screening observation, expands randomly only when the revealed result crosses a fixed synthetic low-value threshold.
4. `rule_based_expansion`: applies a separately frozen synthetic trigger and then uses the predeclared rule-based expansion.
5. `prior_guided_expansion`: applies a more permissive trigger and uses a public prior-score proxy to select an expansion. It is **not an actual LLM call**.
6. `static_prior_expansion`: always expands before seeing any measurement; this is the non-adaptive expansion control.

The GP-EI arm is a real implementation of the finite-pool RBF Gaussian-process expected-improvement acquisition rule, but only for the declared 2D synthetic candidates and frozen hyperparameters. It is not yet a full GP-EI/PI implementation for real continuous/mixed glass-process variables, and it is not independently tuned against production materials datasets. The proxy-score control and expansion triggers are also fixed synthetic policy inputs, not results from trained glass models. This event-loop benchmark verifies action semantics and accounting before expanding the baseline suite. For real empirical comparisons, keep GP-EI/PI, random/DOE/space-filling, random-forest or TPE for mixed/categorical spaces, cost-aware/multi-fidelity VOI, the incumbent engineering workflow, and reproducible LLM-AL/GOLLuM implementations where licensing and input semantics permit.

### Strong literature baselines to add to the live adapter lane

Do not stop at heuristic and conventional-BO comparisons. Include the 2026 LLM active-learning literature as reproducible, model-specific policy arms once code, licenses, prompts/configuration, and candidate mappings can be verified:

- **LLM-AL** (npj Computational Materials, 20 May 2026) reports reducing experiments required to reach top-performing candidates by more than 70% across four diverse materials datasets versus the paper's evaluated baselines. That is a cross-task result, not evidence of a comparable win for glass; task, budget, scoring and policy costs must be aligned before a direct comparison. https://www.nature.com/articles/s41524-026-02136-4
- **GOLLuM** (Nature Machine Intelligence, 2026) trains LLM representations using Bayesian optimization objectives and reports results across 23 organic-synthesis, materials-science, process-chemistry and molecular-design tasks, including claims of matching traditional BO with more than 40% fewer experiments on its benchmark suite. Treat it as a high-priority comparator, not a result we can transfer to glass without reproduction. https://doi.org/10.1038/s42256-026-01283-z

For each, report exact model/checkpoint, training and retrieval data, prompt/configuration digest where applicable, candidate-enumeration/normalization costs, number of LLM calls and tokens, retries, human review, and invalid-proposal handling. If either method cannot use the exact same legal candidate pool or observation interface, mark it not comparable rather than quietly weakening the baseline.

## Budget and visibility rules

All six arms share a cost cap of 5 synthetic resource units and a duration cap of 25 synthetic seconds. Evaluation costs 1 unit / 5 seconds. Random and rule-based expansion cost 2 units / 4 seconds; prior-guided expansion uses a synthetic 3-unit / 8-second generation tariff. Expansion charges are applied before the generated candidate becomes visible. Evaluation charges are applied before the outcome is revealed. The evaluator, not the policy event, supplies the tariffs.

All observation-conditioned arms begin with the same public-priority screening candidate in every environment. Their expansion decision is made only after that observation is returned. The static-prior-expansion control intentionally differs: it spends its expansion budget before any measurement, and its first evaluation is then selected from the expanded space. This is a deliberate test of the cost of expanding without feedback, not a claim that all arms follow the same action order. After expansion, a generated candidate cannot be evaluated until the expansion event makes it visible. The policy cannot evaluate a candidate twice. Missing selected outcomes result in UNRESOLVED rather than a zero utility or a false negative. Every episode records the decision-view SHA-256 before each action, action, evaluator charge, and any subsequently revealed outcome.

## Required interpretation of current known answers

The four hidden synthetic regimes are helpful, neutral, harmful, and misleading prior. The runner emits 24 episodes: six controls in each regime. Its current known-answer assertions are intended to show:

- Observation-conditioned policies can expand in the helpful regime.
- The same policy can refrain from expansion when the initial measurement is already strong.
- Always-expand can lose to a policy that waits for evidence in the harmful regime.
- The prior-guided policy can encounter a false-feasible candidate in the misleading-prior regime.
- All policy arms remain within the same full cost/time caps and preserve distinct observations.

These outcomes validate the event loop only. They do not show that the prior heuristic, an actual LLM or an optimizer performs better on glass or silicon. Any result table from this fixture must be marked synthetic.

The evaluator scorecard now also reports terminal simple regret: the sealed environment's best feasible utility minus the best feasible utility reached by the campaign. The oracle optimum and regret are kept out of the policy view. The independent replay recomputes both from the sealed outcome oracle and rejects a forged regret field. In the helpful synthetic known answer, fixed-space and GP-EI have regret 6, while prior-guided expansion reaches regret 0. This validates metric/replay behavior only; it is not empirical evidence that expansion is superior.


## Repeated-seed random baseline

The live loop now registers 20 preregistered non-negative integer seeds for the auxiliary `uniform_random_fixed_space` control. Each seed/environment pair is evaluated separately, with the seed included in the public decision view and in the episode receipt. The random ranking is deterministic for a fixed seed and public state, but varies across seeds; changing an unobserved oracle outcome cannot change the next action until the outcome is revealed.

The runner emits 80 auxiliary episodes (20 seeds × four environments) and an aggregate per-environment summary: mean best feasible utility, mean terminal simple regret, sample standard deviation of regret, mean cost to first feasible result, false-feasible counts, and resource usage. Independent replay re-runs each seed/environment pair and recalculates the aggregate from the individual receipts; a summary that does not match the individual runs is rejected.

This provides an uncertainty/variance view for the synthetic random control only. It is not a 20-seed trial of the actual GP-EI or prior-guided policy, and the results remain synthetic. Empirical promotion still needs paired repeated runs of every real policy on rights-cleared, family/source-held-out tasks with model/prompt/compute costs fully charged.


## Budget-pressure sensitivity lane

The original five-unit cap is retained as an upper-budget smoke test, but it is not sufficient on its own to compare final best-feasible utility: a fixed-space policy can measure all five initially visible families, making terminal regret insensitive to candidate ordering. A second, preregistered three-unit catalog is therefore part of the v0.1 synthetic suite.

- Fixture: `tools/glass-silicon-contract-audit/fixtures/search-space-live-policy-catalog-tight-budget.synthetic.json`
- Oracle: the same evaluator-only outcome fixture as the five-unit lane. The public catalog and sealed outcome artifact remain byte-distinct and are independently SHA-256 bound.
- Budget: 3 cost units and at most 25 synthetic seconds for every primary policy episode. At the frozen evaluation tariff, fixed-space GP-EI can evaluate only three of the five initially visible candidates; the repeated-seed random control faces the same cap.
- Action surface: each policy receives the declared action tariffs as budget metadata, plus only expansion methods whose full evaluator-owned cost **and duration** fit the remaining budget. An expansion decision that is unaffordable is not offered as an available action; policies continue with affordable candidate evaluations instead.
- Accounting and replay: the evaluator still charges from its own tariff catalog before revealing an outcome. The replay reconstructs the tariff view and affordability set from the sealed oracle, then checks the action/view digest and cumulative charges. The policy cannot lower the tariff by editing its response.

Run the pressure lane with:

```sh
cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin run-search-space-loop -- \
  tools/glass-silicon-contract-audit/fixtures/search-space-live-policy-catalog-tight-budget.synthetic.json \
  tools/glass-silicon-contract-audit/fixtures/search-space-live-outcome-oracle.synthetic.json
```

The three-unit lane is a **diagnostic budget-frontier point**, not an assertion that three units is the correct industrial budget, and not a policy-performance result. Compare matched policy/seed pairs at multiple budget levels before selecting any deployment budget. The protocol known answer requires complete 24-arm episode coverage, exactly three charged units per primary episode, three GP-EI evaluations without expansion, three evaluations per random-seed control, zero unaffordable prior-guided expansions, and deterministic independent replay. If a policy has to abstain or cannot afford a useful action, record that as a result instead of waiving cost or hard constraints.


## Follow-on promotion gates

1. **Runtime isolation:** launch a policy as a sandboxed subprocess/WASM module, send only the current public view, reject all undeclared fields, and replay every action/event against the sealed evaluator independently.
2. **Real algorithm adapters:** add GP-EI/PI for compatible continuous/mixed domains, tree/random-forest or TPE baselines for categorical/mixed spaces, DOE/random, calibrated uncertainty, novelty/disagreement, cost-aware multi-fidelity VOI, and rule/expert expansion. Record all training inputs, seeds, model/version/config/prompt digests and candidate-matching costs.
3. **Matched task and splits:** pin rights-cleared source artifacts; separate the published baseline's original split from system/family/source/campaign/time-held-out tests; freeze holdouts before tuning policies.
4. **Repeated benchmark episodes:** use at least 20 paired task-seed blocks per environment for the initial promotion gate, followed by broader datasets/sites. Publish primary effect size and paired 95% interval, cost/time to first independently qualified feasible candidate, accepted feasible output per fully charged budget, calibration, false-feasible risk and regressions by regime.
5. **Physical/industrial evidence:** a better retrospective score or synthetic campaign is not a plant savings claim. Prospective shadow recommendations, independent safety review, preregistered physical trials and independently metered accepted-grade output are distinct advancement gates.

The next paradigm shift is to optimize **the choice of what to search, what to measure, when to stop and when to abstain** as one closed-loop decision problem, while making every action and hidden-information boundary auditable. It only constitutes SOTA improvement once it beats strong compatible baselines on the same admitted tasks and budgets.
