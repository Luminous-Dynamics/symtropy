# SIM-REL-013 joint stochastic experimental-design oracle

This dependency-free Rust crate is the bounded exact reference for SIM-REL-013.

## Research shift: one joint latent state, not three convenient marginals

The hidden state is the tuple (mechanism hypothesis, physical/process state, sensor/noise state). The kernel is supplied directly as a joint probability over next physical state, next sensor state, observation, and hazard conditioned on the current joint state and action.

The implementation does not factor this kernel into physical and sensor transitions, and it does not marginalize the belief over mechanism, specimen, or sensor state before an action. This preserves correlations that can determine whether a later experiment is informative.

Every kernel row is a complete finite distribution with exact rational mass one. The initial belief is a joint distribution over the full tuple and also sums exactly to one. A safe transition may change both physical and sensor state. A hazard ends that path. Reset and calibration semantics are ordinary actions whose distinct effects on specimen and sensor state are declared in the kernel and whose cost is charged.

## Exact decision objective

The finite-horizon dynamic program returns the exact Pareto frontier over expected cumulative experiment cost, campaign hazard probability, false-identification probability, and abstention probability.

Correct-identification probability is reported separately; the four terminal probabilities (hazard, false claim, abstention, correct claim) must sum exactly to one. A terminal claim is not assumed correct: the evaluator scores it against the latent mechanism. The caller chooses explicit maximum budgets for hazard, false identification, and abstention; the solver minimizes expected cost under those constraints. It does not hide the trade-off in a weighted score.

The returned policy branches only on public observations. It never receives the true hypothesis, physical state, sensor state, or model identity. Repeated actions are permitted and are evaluated under the current joint state distribution.

## Independent replay and qualification controls

The evaluator independently replays a policy from every initial joint-state possibility and the full kernel. The tests compare the memoized DP frontier against non-memoized exhaustive policy-tree enumeration followed by independent trajectory replay on a small stochastic domain.

Authored tests cover correlation preservation; calibration followed by measurement to resolve persistent sensor/physical confounding; repeated measurement that must not treat persistent sensor bias as IID evidence; exact stochastic probability mass and rational arithmetic; hazard cost accounting; malformed or incomplete kernels and resource caps; and agreement between the DP and an exhaustive tiny-domain policy oracle.

## Qualification

Run formatting, locked unit tests, and strict Clippy using the dedicated workflow. It pins Rust 1.96.0, verifies the exact checked-out SHA, and requires a clean tree.

## Scope limits

This oracle is intentionally small and exact. It does not implement particle filters, SMC-squared, approximate posterior inference, continuous states, model-family discovery, or physical safety certification. Its purpose is to make stochastic policy correctness auditable on finite cases before comparing scalable methods. Exactness is conditional on the supplied kernel, initial belief, costs, horizon, and policy class; it does not establish that these describe a real specimen or sensor.


## Myopic EIG comparator

The baseline module ranks actions by one-step expected information gain about the mechanism, divided by action cost. It integrates the current joint physical/sensor belief through the same transition kernel, and treats a hazard event as an observed terminal outcome. Its score is floating-point and is deliberately separate from the exact rational optimizer.

The calibration fixture is a useful counterexample: neither the measurement nor calibration action has immediate mechanism information, but calibration changes sensor state so that a subsequent measurement becomes identifying. A one-step EIG policy cannot value that enabling sequence; the finite-horizon joint optimizer can.


## Model-indexed robust replay (SIM-REL-015)

The robust module evaluates one identical public-observation policy tree against a finite family of complete joint stochastic kernels. A kernel identity remains fixed for an entire campaign; the policy cannot inspect a model-world ID. Each world is replayed independently, with exact per-world expected cost, hazard, false-identification, abstention, and correct-identification probabilities.

The report exposes exact worst-case cost and risk summaries and checks explicit worst-case hazard, false-claim, and abstention budgets. The worlds must share hypothesis, action, observation, physical-state and sensor-state semantics, plus the same action-cost contract; each world may carry its own validated initial joint belief and transition kernel.

This is a robust evaluator, not a robust policy optimizer. It does not choose a minimax policy. Robustness is only over the supplied finite world family and does not establish that the true data-generating process is covered.


## Shared-policy robust optimizer (SIM-REL-016)

The robust optimizer performs finite-horizon dynamic programming over a vector of per-world joint beliefs. Each public observation updates every world under its own complete kernel. Worlds assigning zero probability to a branch are inactive on that branch, while their model identity remains fixed; no model switching or rectangularization occurs.

The optimizer preserves per-world objective vectors during Pareto pruning. This is required because observation probabilities differ across worlds: pruning only by current worst-case aggregates inside a subtree can discard a globally optimal shared policy. At the root, callers can minimize worst-case expected cost or nominal expected cost subject to worst-case hazard, false-identification and abstention budgets.

The implementation is bounded and exact for the supplied finite model family, prior beliefs, action costs, horizon, and policy class. It is not evidence that the real process belongs to that family.


## Deterministic adversarial benchmark factory (SIM-REL-017)

The benchmark module generates seeded finite joint-stochastic challenges in five families: specimen/sensor confounding, persistent sensor drift, physical carryover, calibration-induced specimen damage, and observation-correlated hazard. Each case contains a nominal kernel and a plausible alternative kernel sharing the same hypothesis/action/observation vocabularies, state dimensions and costs.

Generation is deterministic for a fixed seed and generator version and has explicit per-family caps. Every instance includes a stable 64-bit FNV-1a reproducibility fingerprint over its manifest, priors, costs and full world kernels; this is a reproducibility digest, not a cryptographic commitment. Every emitted kernel is validated for complete transition tables and exact rational probability normalization before it is returned. Family labels are construction metadata, not inferred from an agent's self-report.

This is a development fixture factory, not a sealed leaderboard by itself. Keep holdout seeds and mutation compositions out of agent development, publish manifests after evaluation, and do not interpret generated coverage as proof of real-world failure-mode completeness.


## Finite-lookahead joint-state EIG baseline (SIM-REL-018)

The baseline module includes a finite-horizon observation-tree planner that minimizes expected terminal mechanism entropy under a hard per-path experiment-cost budget and exact campaign hazard budget. Every outcome updates the full joint belief over mechanism, physical state and sensor state. Hazards are terminal observable outcomes, and costs are charged even on hazardous paths.

Unlike the one-step EIG ranker, the lookahead planner can value an action that is useful only because it changes the specimen or sensor state for a later measurement. Its posterior entropy objective is evaluated in floating point, while kernel probabilities, posterior masses, expected costs and hazard constraints remain exact rationals.

This is a finite discrete scenario-tree comparator, not a reproduction of COPEx's amortized policy/posterior training, continuous design optimization or gradient-based planner.


## Joint-state particle-filter baseline (SIM-REL-019)

The particle-filter module supplies a seeded, fully adapted conditional filter over mechanism, physical state and sensor state. Each filter instance borrows one immutable kernel for its lifetime; alternative model worlds require separate filter instances. At each observation update, particles are weighted by the event likelihood from the full joint kernel and the next physical/sensor tuple is sampled conditional on the observed event.

The filter normalizes weights, measures ESS, and performs deterministic systematic resampling only when the configured threshold is crossed. Diagnostics distinguish particles that lost support for an observed event from posterior degeneracy (ESS near one). A rare event absent from the approximate particle support is reported as a particle-support failure, not silently treated as an impossible model event. A hazard outcome is terminal and prevents further actions. The filter retains the successfully incorporated observation count and cumulative log predictive-evidence estimate across a campaign.

The exact rational one-step posterior function is the reference. Tests compare particle posterior with that exact binary-channel posterior, verify deterministic resampling and ESS reporting, check that impossible / unsupported observations fail atomically, enforce terminal hazard behavior, and prove that calibration resets sensor state without destroying mechanism/specimen-state correlation.

The particle module also exposes a paired particle-count sweep against the exact one-step joint posterior. Each record reports predictive-probability error, joint-posterior total-variation and squared error, mechanism-posterior squared error, ESS before/after resampling, support-loss fraction, resampling occurrence, and elapsed time. Runtime is descriptive; posterior metrics are seeded and deterministic for a fixed configuration.

This is a finite-state fully adapted particle-filter comparator. It is not nested SMC-squared, does not infer continuous parameters, and does not reproduce BAD-PODS' theoretical convergence guarantees.


## Running the particle-count comparison

Run the deterministic synthetic sweep with the pinned toolchain:

\`\`\`sh
cargo run --release --locked \\
  --manifest-path tools/sim-rel-joint-stochastic-oracle/Cargo.toml \\
  --example particle_filter_sweep > particle-filter-sweep.tsv
\`\`\`

The example generates eight instances per challenge family, sweeps 32–4096 particles, and emits TSV records with the per-instance manifest fingerprint. It uses the nominal kernel and the same first measurement action for every particle count. Timing includes initialization and one update and is descriptive; exact posterior errors and ESS diagnostics are the primary comparison.
