# CivDyn Opponent-Roster Meta-Game v0.1

A dependency-free Rust reference for evaluating strategies against explicit opponent rosters. It does not assign context-free agent rankings.

## Reference RPS game

Strategies are rock, paper and scissors; row-player payoff is +1 for a win, 0 for a tie and -1 for a loss.

The four fixed opponent rosters are:
- balanced: 10 rock, 10 paper, 10 scissors;
- rock-heavy: 24 rock, 3 paper, 3 scissors;
- paper-heavy: 3 rock, 24 paper, 3 scissors;
- scissors-heavy: 3 rock, 3 paper, 24 scissors.

The roster is the exact distribution of opponents faced by every focal strategy. It is not a population count from which a focal agent is subtracted.

## Metrics

- Roster-conditional expected payoff and one-based average competition rank. Equal payoffs share rank (the balanced roster gives all three strategies rank 2.0); deterministic name order is serialization only.
- Best-response payoff and per-strategy exploitability against that same roster.
- Directed pairwise dominance edges and oriented three-cycles.
- Spearman rank correlation between rosters, with average ranks for ties; all-tied rankings return no correlation value.
- Prediction-error regret certificate:
  actual regret is bounded by (maximum payoff - minimum payoff) times the L1 distance between actual and predicted opponent distributions. For RPS payoffs in [-1,1], this is 2 times the L1 distance. Actual regret is reported separately from the bound.

## Claim ceiling

The RPS benchmark is an exact software control. It does not reproduce the 54-agent empirical study or establish any model as state of the art.

## Run

```sh
cargo fmt --manifest-path research/civdyn-opponent-roster/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-opponent-roster/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-opponent-roster/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-opponent-roster/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-opponent-roster/Cargo.toml --bin opponent-roster-sweep
```
