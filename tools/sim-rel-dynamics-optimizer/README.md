# SIM-REL-012 exact stateful risk–cost–abstention optimizer

This dependency-light Rust crate implements an exact finite-horizon dynamic-programming reference for deterministic stateful experiments. It depends only on the independent SIM-REL-011 replay evaluator through a local path dependency.

## Model semantics

At a public observation history, the optimizer maintains the surviving semantic hypotheses, their prior masses, and their current physical states. A candidate action is looked up in each hypothesis' current state. A safe transition emits a binary public observation and advances that hypothesis' physical state; a hazard terminates that hypothesis' campaign path. The action cost is paid whenever it is attempted, including on hazard paths.

Repeated action IDs are legal. Their next outcome depends on the current physical state; they are not treated as independent IID samples. Reset and washout are represented as explicit actions in the world transition tables.

## Objective

The solver enumerates the exact nondominated frontier over:

- expected cumulative experiment cost;
- probability of at least one campaign hazard;
- probability of explicit abstention;
- probability of correct exact identification.

For the deterministic finite model these satisfy:

`R_hazard + P_abstain + P_identify = 1`.

No weighted scalar silently trades safety against uncertainty. A caller chooses an operating point with explicit maximum hazard and abstention probabilities. If no frontier point satisfies both, selection returns infeasible.

A semantic hypothesis can be claimed only when it is the sole surviving hypothesis at that public history. All unresolved terminals are explicit abstentions. Arithmetic overflow and state/frontier/combination caps fail closed.

## Algorithm

The memoized Bellman state is the canonical vector of (hypothesis identity, current physical state, prior mass) plus remaining horizon. Each action partitions surviving prior mass into:
- immediate hazard mass;
- safe outcome-zero mass and updated physical states;
- safe outcome-one mass and updated physical states.

For every pair of continuation frontier points, the solver forms the exact expected cost, campaign risk and abstention values. It prunes only componentwise dominated points. All probabilities and frontier comparisons use checked exact rational arithmetic.

The output is a policy tree using the evaluator's public-observation policy format. Every frontier point can be independently replayed through `sim-rel-dynamics-evaluator`.

## Golden fixtures and tests

Nine authored Rust tests currently cover:
- exact rational arithmetic and overflow-safe comparison;
- non-commutative condition-then-measure sequence discovery;
- exact three-point (cost, hazard, abstention) frontier;
- policy selection under explicit risk and abstention budgets;
- repeated action availability;
- memoryless control;
- action/hypothesis input-order invariance;
- malformed worlds and fail-closed solver caps;
- horizon-zero abstention.

The tests are definitions, not a hosted passing result. Qualification is performed by the dedicated workflow.

## Run

```sh
cargo fmt --manifest-path tools/sim-rel-dynamics-optimizer/Cargo.toml -- --check
cargo test --locked --manifest-path tools/sim-rel-dynamics-optimizer/Cargo.toml --lib
cargo clippy --locked --manifest-path tools/sim-rel-dynamics-optimizer/Cargo.toml --lib -- -D warnings
```

## Deliberate limitations

This is an exact optimizer for one supplied deterministic finite stateful world and a finite horizon. It does not yet implement stochastic observation/state likelihoods, simultaneous multi-world robust optimization, continuous action spaces, or physical safety certification. The solver's exactness does not establish that the supplied transition model is correct or adequate.