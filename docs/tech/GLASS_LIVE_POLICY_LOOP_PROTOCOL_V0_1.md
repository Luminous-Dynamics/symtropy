# GLASS-LIVE-POLICY-LOOP-001 — observation-conditioned search-space benchmark v0.1

Status: executable synthetic event-loop contract. Not an empirical materials result, a model benchmark result, or authorization to run equipment.

Parent: [adaptive search-space protocol](./GLASS_ADAPTIVE_SEARCH_SPACE_BENCHMARK_V0_1.md), [issue #2788](https://github.com/Luminous-Dynamics/symtropy/issues/2788), and [closed-loop runtime-isolation issue #2785](https://github.com/Luminous-Dynamics/symtropy/issues/2785).

## Why this is a separate benchmark layer

A static list of pre-authored actions can test budget arithmetic and hidden-outcome scoring, but it cannot show whether a policy actually changes its next action when it receives new evidence. This layer runs a deterministic policy loop. The policy gets only the public catalog, currently visible candidate descriptors, budget remaining, and observations actually returned by earlier actions. The evaluator charges a chosen action before looking up and revealing its result. The next policy decision is then computed from the updated public view.

The sealed oracle includes environment identity, candidate feasibility, utilities and evaluator-owned tariffs. None of these fields are copied into the policy view. The policy chooses an action from typed public state; the evaluator separately executes that action and supplies a result. Tests mutate unobserved oracle outcomes and require action choices to remain unchanged until the corresponding observation is revealed.

**Trust boundary warning:** the current runner and policy implementations execute in one Rust process that reads both input files. Its types and mutation tests demonstrate the intended information flow, but they are not a security sandbox against malicious policy code. The next production-grade step remains an isolated subprocess/WASM policy host that receives only serialized public state, with independent transcript replay and restricted filesystem/network capabilities.

## Executable artifacts

- CLI: \`tools/glass-silicon-contract-audit/src/bin/run-search-space-loop.rs\`
- Public catalog: \`tools/glass-silicon-contract-audit/fixtures/search-space-live-policy-catalog.synthetic.json\`
- Sealed oracle: \`tools/glass-silicon-contract-audit/fixtures/search-space-live-outcome-oracle.synthetic.json\`
- Public schema: \`docs/tech/glass-search-space-live-catalog-v0.1.schema.json\`
- Sealed-oracle schema: \`docs/tech/glass-search-space-live-oracle-v0.1.schema.json\`

Run from repository root:

    cargo test --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml
    cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin run-search-space-loop -- tools/glass-silicon-contract-audit/fixtures/search-space-live-policy-catalog.synthetic.json tools/glass-silicon-contract-audit/fixtures/search-space-live-outcome-oracle.synthetic.json

    cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin run-search-space-loop -- --replay search-space-live-loop-report.json tools/glass-silicon-contract-audit/fixtures/search-space-live-policy-catalog.synthetic.json tools/glass-silicon-contract-audit/fixtures/search-space-live-outcome-oracle.synthetic.json

The normal run command can be redirected to \`search-space-live-loop-report.json\` before replay. The \`--replay\` mode independently rebuilds each public decision view and the registered deterministic policy's expected next action from that view, then validates action ordering/visibility and proposal digests, recomputes evaluator charges, verifies observations against the sealed oracle, and recomputes each episode summary. A legal-but-policy-inconsistent action fails replay. It fails closed on forged observations, under-reported costs, stale view hashes and mismatched input/oracle byte digests. This replay mode is a separate code path in the same executable; it is not a separate OS process and does not replace the still-required subprocess/WASM isolation gate.


## Policy arms are software controls, not claimed SOTA implementations

The runner executes six deterministic policy controls:

1. \`fixed_space\`: never expands; chooses candidates using its declared public selector scores.
2. \`random_expansion\`: after the common first screening observation, expands randomly only when the revealed result crosses a fixed synthetic low-value threshold.
3. \`rule_based_expansion\`: applies a separately frozen synthetic trigger and then uses the predeclared rule-based expansion.
4. \`prior_guided_expansion\`: applies a more permissive trigger and uses a public prior-score proxy to select an expansion. It is **not an actual LLM call**.
5. \`static_prior_expansion\`: always expands before seeing any measurement; this is the non-adaptive expansion control.

These thresholds and selection scores are controlled test policy inputs. They are not trained or calibrated on glass data and they do not represent GP expected improvement, a real LLM, or an incumbent industrial optimizer. Keep the conventional fixed-space GP-EI, random/DOE, cost-aware BO and the incumbent workflow as mandatory real-policy comparators when their input semantics match. This synthetic runner tests the event loop and observation timing before those live adapters are built.

### Strong literature baselines to add to the live adapter lane

Do not stop at heuristic and conventional-BO comparisons. Include the 2026 LLM active-learning literature as reproducible, model-specific policy arms once code, licenses, prompts/configuration, and candidate mappings can be verified:

- **LLM-AL** (npj Computational Materials, 20 May 2026) reports reducing experiments required to reach top-performing candidates by more than 70% across four diverse materials datasets versus the paper's evaluated baselines. That is a cross-task result, not evidence of a comparable win for glass; task, budget, scoring and policy costs must be aligned before a direct comparison. https://www.nature.com/articles/s41524-026-02136-4
- **GOLLuM** (Nature Machine Intelligence, 2026) trains LLM representations using Bayesian optimization objectives and reports results across 23 organic-synthesis, materials-science, process-chemistry and molecular-design tasks, including claims of matching traditional BO with more than 40% fewer experiments on its benchmark suite. Treat it as a high-priority comparator, not a result we can transfer to glass without reproduction. https://doi.org/10.1038/s42256-026-01283-z

For each, report exact model/checkpoint, training and retrieval data, prompt/configuration digest where applicable, candidate-enumeration/normalization costs, number of LLM calls and tokens, retries, human review, and invalid-proposal handling. If either method cannot use the exact same legal candidate pool or observation interface, mark it not comparable rather than quietly weakening the baseline.

## Budget and visibility rules

All five arms share a cost cap of 5 synthetic resource units and a duration cap of 25 synthetic seconds. Evaluation costs 1 unit / 5 seconds. Random and rule-based expansion cost 2 units / 4 seconds; prior-guided expansion uses a synthetic 3-unit / 8-second generation tariff. Expansion charges are applied before the generated candidate becomes visible. Evaluation charges are applied before the outcome is revealed. The evaluator, not the policy event, supplies the tariffs.

All observation-conditioned arms begin with the same public-priority screening candidate in every environment. Their expansion decision is made only after that observation is returned. The static-prior-expansion control intentionally differs: it spends its expansion budget before any measurement, and its first evaluation is then selected from the expanded space. This is a deliberate test of the cost of expanding without feedback, not a claim that all arms follow the same action order. After expansion, a generated candidate cannot be evaluated until the expansion event makes it visible. The policy cannot evaluate a candidate twice. Missing selected outcomes result in UNRESOLVED rather than a zero utility or a false negative. Every episode records the decision-view SHA-256 before each action, action, evaluator charge, and any subsequently revealed outcome.

## Required interpretation of current known answers

The four hidden synthetic regimes are helpful, neutral, harmful, and misleading prior. The runner emits 20 episodes: five controls in each regime. Its current known-answer assertions are intended to show:

- Observation-conditioned policies can expand in the helpful regime.
- The same policy can refrain from expansion when the initial measurement is already strong.
- Always-expand can lose to a policy that waits for evidence in the harmful regime.
- The prior-guided policy can encounter a false-feasible candidate in the misleading-prior regime.
- All policy arms remain within the same full cost/time caps and preserve distinct observations.

These outcomes validate the event loop only. They do not show that the prior heuristic, an actual LLM or an optimizer performs better on glass or silicon. Any result table from this fixture must be marked synthetic.

## Follow-on promotion gates

1. **Runtime isolation:** launch a policy as a sandboxed subprocess/WASM module, send only the current public view, reject all undeclared fields, and replay every action/event against the sealed evaluator independently.
2. **Real algorithm adapters:** add GP-EI/PI for compatible continuous/mixed domains, tree/random-forest or TPE baselines for categorical/mixed spaces, DOE/random, calibrated uncertainty, novelty/disagreement, cost-aware multi-fidelity VOI, and rule/expert expansion. Record all training inputs, seeds, model/version/config/prompt digests and candidate-matching costs.
3. **Matched task and splits:** pin rights-cleared source artifacts; separate the published baseline's original split from system/family/source/campaign/time-held-out tests; freeze holdouts before tuning policies.
4. **Repeated benchmark episodes:** use at least 20 paired task-seed blocks per environment for the initial promotion gate, followed by broader datasets/sites. Publish primary effect size and paired 95% interval, cost/time to first independently qualified feasible candidate, accepted feasible output per fully charged budget, calibration, false-feasible risk and regressions by regime.
5. **Physical/industrial evidence:** a better retrospective score or synthetic campaign is not a plant savings claim. Prospective shadow recommendations, independent safety review, preregistered physical trials and independently metered accepted-grade output are distinct advancement gates.

The next paradigm shift is to optimize **the choice of what to search, what to measure, when to stop and when to abstain** as one closed-loop decision problem, while making every action and hidden-information boundary auditable. It only constitutes SOTA improvement once it beats strong compatible baselines on the same admitted tasks and budgets.
