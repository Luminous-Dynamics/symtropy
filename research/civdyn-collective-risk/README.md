# CivDyn Collective Risk v0.1

A dependency-free Rust reference task for the six-seat collective-risk dilemma described in Huynh et al. (2026), *Cooperation Without Adaptation: LLM Agents in a Collective-Risk Dilemma*.

## Source-aligned task

- 6 seats, 10 simultaneous rounds.
- Each seat starts with 40 units.
- Allowed contribution per round: 0, 2 or 4.
- Shared target: 120 units.
- Reaching the target preserves each seat's unspent endowment.
- Missing the target destroys unspent savings with probability (p).
- Scoring uses expected payoffs analytically; the catastrophe lottery is not sampled.

The paper's welfare-optimal expected payoff has a pivot at (p^*=T/(6e)=0.5). Below the pivot, zero contribution is welfare-optimal; above the pivot, reaching the target is welfare-optimal; both modes are optimal at the pivot. This is a welfare benchmark, not a unique prediction of individual play.

## Implemented

- Deterministic simultaneous-round simulation with public previous-round contribution history.
- Transparent strategies: always-zero, fixed fair-share, fixed maximum, risk-pivot, last-round defection and copy-previous-group-mean.
- Expected payoff vector, group/per-seat welfare, target status, overshoot, shortfall and payoff Gini.
- Exhaustive best response against five scripted partners; the focal seat's maximum search space is (3^{10}=59,049) legal action-sequence candidates before endowment filtering.
- A risk-grid CSV sweep from (p=0.0) to (p=1.0), with fixed-partner best-response rows at low, pivot and high risk.

## Run

```sh
cargo fmt --manifest-path research/civdyn-collective-risk/Cargo.toml --all -- --check
cargo check --locked --manifest-path research/civdyn-collective-risk/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-collective-risk/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-collective-risk/Cargo.toml --bin collective-risk-sweep
```

## Scientific claim ceiling

This crate reconstructs the public game contract and its analytical proposition. It does **not** reproduce the paper's LLM prompts, model outputs, human/LLM sample, mixed-population selection process, or exact model-specific results. The scripted policies are inspectable baselines, not claims about real agents.

The next extension is a finite-population empirical game-theoretic analysis (N=6, 30 and larger) with hypergeometric table composition and payoff-biased imitation. Mechanisms from CoopEval (repetition, reputation, mediation and outcome-contingent contracts) must be explicit interventions, not inferred from a policy name.


## Finite-population selection extension

The population module adds an exact source-aligned model of the selection procedure described by Huynh et al. (2026):

- N=6 and N=30 populations, with six-person interaction groups.
- Exact multivariate-hypergeometric enumeration of the five tablemates sampled without replacement.
- Composition-weighted expected payoff for each strategy type.
- Pairwise Fermi imitation with explicit beta (default 1).
- Exact two-type rare-mutation fixation probability, evaluated in the log domain.
- Embedded Markov chain among homogeneous strategy populations and a bounded stationary-distribution solver.

The source paper compares N=6 (the whole population is one table) with N=30 (agents occupy overlapping randomly mixed tables). This crate evaluates the six inspectable scripted policies, not the paper's five production LLMs.

Run:

```sh
cargo run --locked --manifest-path research/civdyn-collective-risk/Cargo.toml --bin collective-risk-population-sweep
```

The sweep varies risk p in {0.1, 0.5, 0.9}, population size N in {6, 30}, and selection intensity beta in {0, 0.1, 1, 5}. The fitness layer evaluates six frozen population profiles: one-of-each at N=6, plus balanced, zero-heavy, fair-share-heavy, maximum-heavy and copy-heavy at N=30. This is a controlled roster-shift stress test; the alternative N=30 profiles are synthetic fixtures, not estimates of real-world population frequencies.

The output separates roster-conditioned expected fitness from the rare-mutation pairwise fixation/stationary model. It emits 432 roster-conditioned fitness records, 720 fixation records, 144 stationary-weight records and 24 invasion-graph summaries, plus individual cycle records where present.

It also derives a directed invasion graph: an edge from resident i to mutant j is emitted when the mutant's single-invasion fixation probability exceeds the neutral baseline 1/N. Directed three-cycles are enumerated with orientation preserved. This prevents pairwise results from being compressed into a misleading total strategy order. The criterion is a finite-population selection reference, not a claim of universal dominance.

Scientific limits: rare-mutation/Fermi dynamics is a declared selection model, not a validated description of actual institutions or social learning. A population-relative strategy ranking is conditional on N, risk, group size, strategy roster, payoff model, beta and mutation assumptions. The binary emits no universal winner.
