# GLASS-SPACE-EXPANSION-001 — adaptive search-space benchmark v0.1

Status: proposed benchmark contract. No adaptive-space policy has been shown to outperform a state-of-the-art baseline on glass, metallurgical-grade silicon, or an industrial process.

Parent: [Glass–Silicon decision-value protocol](./GLASS_SILICON_DECISION_VALUE_BENCHMARK_V0_1.md), [issue #2788](https://github.com/Luminous-Dynamics/symtropy/issues/2788), and [PR #2716](https://github.com/Luminous-Dynamics/symtropy/pull/2716).

## Research premise and claim ceiling

Iwasaki et al., *Communications Materials* 7, article 202 (published 25 August 2026), couple an LLM outer loop that redefines the materials search space to an inner Bayesian-optimization loop. Their demonstration uses 16,908 ternary-alloy records derived from high-throughput DFT calculations. Gains are strongest for properties where the LLM has substantial relevant prior knowledge and more limited where that prior is weak. This motivates prior-strength and misleading-prior ablations; it does not establish that LLM expansion wins for glass or silicon. https://www.nature.com/articles/s43246-026-01304-9

MADE (ICML 2026) benchmarks closed-loop discovery campaigns under constrained oracle budgets and provides discovery-efficiency metrics such as AF, EF, AUDC and mSUN. Reuse these only where the candidate/oracle semantics match. Convex-hull stability is not glass formation, saleable product, or plant savings. https://proceedings.mlr.press/v306/malik26a.html · https://github.com/diffractivelabs/MADE

NASA Glenn's 2026 BO comparison is a practical baseline floor: it evaluates GP, random-forest, Monte-Carlo neural-network and TPE-based approaches with multiple acquisition functions, random controls and repeated trials. Expected improvement was a strong default in its tested settings; random forests were competitive on categorical/mixed spaces. These were not glass-process experiments. https://ntrs.nasa.gov/citations/20260001102

NIST's materials active-learning benchmark compares acquisition functions on a real Fe–Co–Ni thin-film library. It is additional evidence for shared-data, shared-budget baselines, not a glass result. https://www.nist.gov/publications/benchmarking-active-learning-strategies-materials-optimization-and-discovery

## Paradigm shift: version the domain, not only the next candidate

Conventional BO generally selects the next candidate from a fixed domain. An outer loop can decide that the domain itself is wrong: a useful family is absent, the instrument configuration limits available variables, or a new process route deserves a controlled branch. That decision is itself an intervention with cost, uncertainty, provenance and potential harm. Each expansion is therefore an immutable state transition—not a prompt, rationale, or silently replaced list.

A proposal must be bound to the exact raw-byte SHA-256 of its parent space; identify the versioned proposer/configuration; state applicability domains, additions/removals and supporting evidence; record its expected decision consequence; and declare generation cost and elapsed time. LLM proposals record model identity plus prompt/configuration digests. Hidden outcomes, oracle utilities and evaluator labels are forbidden in proposer-visible payloads. A policy must not observe outcomes before the matching observation event is revealed.

Hard product and safety constraints are immutable inside a space proposal. Changing one requires a separately versioned benchmark/process contract and independent review; an optimizer may not lower accepted-grade requirements or widen the claim ceiling to make a candidate pass. An unknown domain or unsupported proposal remains UNRESOLVED. Structural PASS means PROPOSED_NOT_ADMITTED: it is not material feasibility or authorization to run an experiment.

## Executable proposal audit v0.1

Artifacts:
- Parent schema: docs/tech/glass-search-space-base-v0.1.schema.json
- Proposal schema: docs/tech/glass-search-space-proposal-v0.1.schema.json
- Base fixture: tools/glass-silicon-contract-audit/fixtures/base-space.synthetic.json
- Positive fixture: tools/glass-silicon-contract-audit/fixtures/space-proposal-helpful.synthetic.json
- Executable binary: tools/glass-silicon-contract-audit/src/bin/audit-search-space.rs

Run from repository root:

    cargo test --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml
    cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-search-space -- tools/glass-silicon-contract-audit/fixtures/base-space.synthetic.json tools/glass-silicon-contract-audit/fixtures/space-proposal-helpful.synthetic.json

The binary returns PASS only for a structurally consistent proposal tied to the exact parent bytes. Its disposition remains PROPOSED_NOT_ADMITTED. It emits raw base/proposal SHA-256 values and a normalized resulting-space digest, candidate counts and declared generation cost/time. A stale parent or unsupported domain/evidence is UNRESOLVED; constraint mutation, hidden-outcome fields, ID collisions, invalid budget charges or an empty resulting domain is FAIL.

The checker validates the declared envelope, not evidence authenticity, actual proposal quality, proposer sandbox isolation, semantic equivalence of constraints, or material outcomes. Built-in fixtures use synthetic evidence only.

## Matched-budget experiment matrix

Every policy receives the same initial evidence, legal action surface, evaluation holds and total cost/time allowance. Charge outer-loop generation, candidate enumeration, prompt/retrieval/model inference, domain validation, filtering, rejected candidates, measurements, retries, failures and computation. Use evaluator-owned tariffs where possible; policy-supplied costs must never reduce charges.

Required policy arms, where compatible:

1. Fixed-space uniform random and DOE/space-filling.
2. Fixed-space GP expected improvement and probability of improvement where appropriate.
3. Random-forest or tree-structured Parzen estimator baselines for categorical/mixed spaces.
4. Fixed-space uncalibrated/calibrated uncertainty, novelty/disagreement and cost-aware value-of-information.
5. Random expansion and documented domain-rule/expert expansion.
6. LLM-guided expansion paired with exactly the same inner optimizer as the fixed-space arm.
7. Incumbent industrial workflow, if available and rights-cleared.

Pre-register why a standard baseline is incompatible before opening the final holdout. Do not let the LLM arm have extra oracle calls, private outcome access or a larger candidate-generation budget. Keep the inner optimizer fixed in the primary ablation so outer-loop expansion is isolated; test optimizer interactions separately.

The sealed evaluator must include:
- Expansion helps: the initial domain omits a productive, admissible family.
- Neutral expansion: new families duplicate or add little decision value.
- Expansion harms: expansion consumes budget or attracts the policy into unproductive families.
- Misleading prior: foundation-model priors systematically favor an unproductive family.
- Low prior knowledge: the target domain is sparse or unlike common model examples.
- Out-of-domain/unsupported: proposals enter a domain without evidence, instrument support, rights or approved feasibility gates.
- Constraint attack: a proposal tries to weaken a minimum product requirement or suppress a failure.

Seal family/source/campaign and time/site holdouts before tuning. Preserve all revisions and rejected proposals. A metamorphic test changes sealed outcomes while keeping observed history identical; no pre-observation proposal or selector action may change due to information the policy has not received.

## Endpoints and proposed targets

Primary endpoint: independently verified feasible accepted-output utility per fully charged resource budget. Where utility cannot be made commensurate, report accepted output per total cost and time separately rather than inventing a combined score.

Required secondary endpoints: cost/time to first independently qualified feasible candidate; cumulative best feasible utility versus cumulative cost/time; verified feasible discoveries; false-feasible rate and its upper confidence bound; quality failures and hard-constraint violations; unsupported/out-of-domain and UNRESOLVED rates; new-family/source/campaign coverage; expansion precision/recall against adjudicated outcomes; rejected-candidate and retest burden; runtime/compute/inference cost; space-revision reversals/cycles.

**Aspirational promotion gate, not a measured result:** target at least a 10% relative gain in the preregistered primary endpoint over the strongest compatible baseline on held-out tasks, with a paired 95% confidence interval excluding zero, across at least 20 independent task-seed blocks per environment. Publish all arms and all environments, including regressions. A win in one favorable environment cannot offset harm in another without a preregistered deployment-weighted estimand. Also require no increase in hard-constraint violations and a false-feasible upper bound within predeclared risk tolerance. If uncertainty is too large, the result is UNRESOLVED. The 10% threshold is a project target, not a published SOTA boundary.

Software integrity gate: zero accepted hard-constraint mutations, zero hidden-outcome leakage in metamorphic controls, deterministic receipt replay, no budget undercharging, and a non-empty positive-control search space. Synthetic fixtures validate verifier behavior only; they cannot establish empirical policy ranking.

## Executable matched-budget campaign evaluator v0.1

The first outcome-scoring slice is now present separately from the proposal-envelope checker:

- Public policy trace: \`tools/glass-silicon-contract-audit/fixtures/search-space-campaign-input.synthetic.json\`
- Sealed evaluator outcomes and action tariffs: \`tools/glass-silicon-contract-audit/fixtures/search-space-outcome-oracle.synthetic.json\`
- Public trace schema: \`docs/tech/glass-search-space-campaign-input-v0.1.schema.json\`
- Evaluator-only schema: \`docs/tech/glass-search-space-outcome-oracle-v0.1.schema.json\`
- CLI: \`audit-search-space-trial <public-campaign-input.json> <sealed-oracle.json>\`

Run:

    cargo test --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml
    cargo run --locked --manifest-path tools/glass-silicon-contract-audit/Cargo.toml --bin audit-search-space-trial -- tools/glass-silicon-contract-audit/fixtures/search-space-campaign-input.synthetic.json tools/glass-silicon-contract-audit/fixtures/search-space-outcome-oracle.synthetic.json

The synthetic known-answer panel contains 16 campaigns across four policy classes (fixed space, random expansion, rule-based expansion, LLM outer-loop expansion) and four deliberately distinct regimes (helpful, neutral, harmful, misleading prior). All campaigns share the same maximum resource cap. Evaluator-owned tariffs charge expansion generation and candidate evaluation before results; the policy cannot self-report a lower charge. A candidate can only be evaluated after its expansion action, one candidate cannot be evaluated twice in a campaign, and each policy arm must appear in each regime. Missing outcomes are UNRESOLVED, not zero. Predictions of feasibility are recorded before revealing evaluator outcomes, so false-feasible predictions are counted.

The fixture's known-answer pattern is intentionally non-universal: fixed-space best feasible utility is 4 while LLM expansion reaches 10 in the helpful regime; the arms tie at 6 in the neutral regime; fixed space reaches 8 and LLM expansion only 6 in the harmful regime; a misleading prior produces a false-feasible LLM prediction. This matrix demonstrates the scorer can represent improvement, neutrality and regression—it is not evidence that any live policy achieved those utilities.

The audit reports per-campaign utility, verified feasible hits, false-feasible predictions, first-feasible cost, charged cost/time and raw input/oracle SHA-256 digests. The hidden-outcome mutation test keeps the public trace byte-identical while showing that the score changes when sealed outcomes change. This verifies separation at the evaluator interface only; it does not prove the policy ran in a sandbox, that actual observed actions were generated without hidden information, or that a material label is authentic. Issue #2785 remains the isolation/replay follow-on.

## Advancement gates

1. Synthetic contract: schema and CLI known answers; mutation tests for stale parents, missing evidence, unknown domains, hidden outcomes, duplicate IDs, budget undercharging and constraint weakening.
2. Rights-cleared retrospective replay: exact source-byte digests, rights/license record, pinned preprocessing, relationship graph, sealed family/source/time holds, and baseline reproduction.
3. Prospective shadow mode: propose and score candidates without controlling equipment; compare recommendations to an authorized incumbent without exposing future outcomes.
4. Human-authorized experiment: independently review safety, feasibility, instruments, stopping rules and data custody before any controlled physical experiment. A software receipt is not an operating authorization.
5. Industrial claim: replicate across campaigns/sites where required; measure accepted-grade output, energy per accepted tonne, quality, downtime, yield, cost and uncertainty. Use a preregistered causal design for savings claims.

## Falsification and decision rule

Expansion is justified only if it adds independently verified feasible decision value over the strongest compatible fixed-space and expansion baselines under the same complete budget, without weakening product/safety constraints or increasing false-feasible risk. If GP-EI, random/DOE, a domain-rule policy or the incumbent ties or wins, publish that result and prefer the simpler/cheaper method. Candidate count, domain volume, novelty score, rationale quality, or a synthetic positive control is never success by itself.
