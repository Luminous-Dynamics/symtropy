# CivDyn Group Size v0.1

A deterministic-by-seed Rust reference for finite-memory local coordination and population-size experiments.

## Source anchor and boundary

Flint, Aiello, Pastor-Satorras & Baronchelli, PNAS 2026, "Group size effects and collective misalignment in LLM multi-agent systems":
- https://doi.org/10.1073/pnas.2531697123
- https://pmc.ncbi.nlm.nih.gov/articles/PMC13505991/
- https://github.com/Ariel-Flint-Ashery/LLM-group-size

The published model uses two conventions, random distinct-agent pairwise interactions, simultaneous choices, +100/-50 match/mismatch payoffs, finite memory of H=5 interactions and N pairwise interactions per population round.

The public repository is a useful protocol and policy-extraction reference, but its retrieved default-branch `simulation_module.py` ends inside `get_full_transition_matrix` after retrieving a transition matrix. It does not visibly complete that function's export path in the checked-in file. The repository also contains multiple population-size grids. The public repository main commit inspected for this audit was `ddbbca5b7fd0551ed409903c275f75505db7da9d`.

Therefore this v0.1 does **not** claim to reproduce the paper's extracted LLM policies or numerical results. It implements the explicit pairwise finite-memory mechanism with injected policy tables. The six policies in the executable are synthetic software controls, not LLMs or human behavioural estimates.

## Memory state space

A memory state stores an ordered sequence of up to H pairs:
- the agent's own prior convention;
- the partner's prior convention.

For H=5, this gives:
`1 + 4 + 4² + 4³ + 4⁴ + 4⁵ = 1,365` distinct states.

The empty history is distinct from a one-pair all-zero history and from a five-pair all-zero history. A fixed 10-bit encoding would incorrectly alias shorter states, so this reference uses a length-qualified canonical encoding.

Each policy table must explicitly supply one finite probability `P(option_0 | memory_state)` for every state. Missing rows, duplicate semantic state encodings, invalid probabilities and memory-capacity mismatches are rejected; there is no silent fallback policy.

## Population mechanics

- N >= 2 agents begin in the explicit empty-memory state.
- Each population round contains exactly N pairwise encounters.
- Each encounter samples an ordered pair of distinct agents uniformly.
- Both agents sample actions using their own current memory state and policy probability.
- Match payoff is +100 each; mismatch payoff is -50 each.
- Both agents append their own/partner action pair, dropping only the oldest pair when memory is full.
- A pinned SplitMix64 generator and rejection-based bounded sampling make seed/config replay deterministic.

The agreement convergence indicator is the mean pairwise match rate over three successive population rounds reaching 0.98. It is distinct from an all-actions-identical round and from three consecutive all-actions-identical rounds. The convergence-window convention label is assigned from the average option-0 action share in that window; ties are assigned to option 0. This is a benchmark bookkeeping rule, not a normative judgment.

## Synthetic policy portfolio

- `always_option_0`: constant probability 1 for convention 0.
- `always_option_1`: constant probability 0 for convention 0.
- `neutral_memory_independent`: constant probability 0.5.
- `weak_option_0_bias`: constant probability 0.55.
- `copy_partner_majority`: follows the majority of the partner's own recorded actions, with a 0.5 tie probability.
- `anti_copy_partner_majority`: selects the opposite of that partner-history majority, with a 0.5 tie probability.

The injected policies demonstrate mechanics and negative controls. They do not represent the paper's learned LLM policies.

## Evaluation smoke portfolio

The executable uses:
- N = 2, 3, 4, 6, 10, 24, 100;
- 30 deterministic seeds per policy and population size;
- H = 5;
- 100 maximum population rounds;
- three-round, 0.98 agreement convergence.

It emits one CSV row per run (7 population sizes × 6 policies × 30 seeds = 1,260 rows). This is a software smoke/variance portfolio, not the paper's full 1,000-replication source protocol or an inferential claim.

## Metrics

The CSV preserves per-run:
- initial individual option-0 bias;
- for constant memory-independent policies, exact analytical expected option-0 share and pairwise agreement plus finite-N per-round standard deviations; memory-dependent policies leave these oracle fields empty. CI checks replication means against the analytic expectations using their calculated sampling standard errors.
- population size, seed, memory capacity and exact state count;
- population rounds and total pairwise interactions;
- agreement convergence, convergence round and declared window convention;
- true action unanimity round;
- final option-0 action share and pairwise agreement rate;
- average, minimum and maximum cumulative payoff;
- unique memory-state count and memory-distribution entropy.

No single collective-bias or "alignment" scalar is emitted. Non-converged runs remain present in the denominator.

## Run

```sh
cargo fmt --manifest-path research/civdyn-group-size/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-group-size/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-group-size/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-group-size/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-group-size/Cargo.toml --bin group-size-sweep
```

## Next research gates

1. Source-pin and reconcile the original transition-table extraction and simulation path.
2. Import only licensed, source-exact policy tables with prompt/model/tokenizer/temperature/order provenance.
3. Reproduce the paper's precise convergence criteria, size grid, replication count and reported figures.
4. Add heterogeneous policies and partner/cohort distributions.
5. Compare finite-size dynamics with mean-field or pair approximation predictions and held-out seeds.
6. Add independent bootstrap/finite-size uncertainty rather than treating micro-interactions as independent population samples.

No claim is made about real human bias, LLM alignment, morality, consciousness, thermodynamics or civilizational behaviour.
