# ANIMA Sequential Intervention Design v0

Standalone, dependency-free Rust policy primitives for the sequential experiment-design layer tracked by Symtropy [#3588](https://github.com/Luminous-Dynamics/symtropy/issues/3588).

This crate does **not** implement a biological circuit, execute experiments, control hardware, or claim a biological/SOTA result. It consumes model-conditional likelihood tables supplied by separately versioned model and inference components. The finite categorical outcome model is an explicit V0 fixture/interface, not a claim that locomotor trajectories are categorical or conditionally independent.

## Paradigm shift

The one-shot selector asks which intervention is informative under today's posterior. This crate asks which *permitted* intervention should come next after sequential evidence, subject to a frozen policy, finite budget, causal-validity preconditions, and a stop/abstain/model-revision contract.

The policy does not control wet-lab hardware. The interface is an offline decision oracle until protocol review, delivery validation, ethics approval where applicable, and a separately qualified execution system exist.

## Implemented surface

- Exact finite-outcome EIG: H(Model) - E[H(Model | outcome)]; unlike a Monte Carlo estimator, this categorical calculation has no integration error.
- Sequential Bayes update after an observed outcome, with assignment identity, delivered-protocol identity, replication-unit/session identity and washout checks.
- Greedy EIG, EIG-per-cost, robust maximin EIG-per-cost over an explicit posterior ambiguity set, deterministic random-feasible, and fixed-manifest-order baselines.
- Hard exclusion for safety-rejected, infeasible, invalid-design, low-fidelity, unverified-delivery, interference-uncontrolled, carryover-uncleared and over-budget candidates.
- Four explicit outcomes: select, stop at a preregistered posterior/budget threshold, abstain when no valid/informative option exists, or stop for model revision after a posterior-predictive failure.
- Candidate and state reproducibility fingerprints; candidate ordering and model maps are canonicalized, and rejected candidates are retained in receipts.
- Forty deterministic parameterized synthetic known-answer fixtures (eight regimes × five parameterizations), plus focused negative tests.

## Important integrity limits

The V0 receipt fingerprint is FNV-1a 64-bit and is explicitly **non-cryptographic**. It detects accidental input drift but is not a cryptographic content address, signature, or tamper-evidence mechanism. Before any biological data, external protocol exchange or high-consequence use, replace it with an audited SHA-256/BLAKE3 implementation and a versioned canonical serialization. No privacy, security or provenance-integrity claim follows from the current fingerprint.

The caller must keep randomized assignment distinct from delivered stimulation. A mismatch causes the outcome to be rejected from the causal posterior update; it must still be retained as a delivery-failure record. Every experiment record carries an explicit evidence tier (synthetic fixture, in-silico generated, preprint measured, or peer-reviewed measured), and that tier is included in the decision-state fingerprint. The V0 single-event Bayesian updater rejects a replication unit or session already present in the prior history. This is deliberately conservative: repeated measurements from the same animal/preparation or session require a joint cluster/trajectory likelihood, which is not yet implemented. Do not feed repeated time samples through this updater as independent trials.

The categorical likelihood interface also does not yet represent serially correlated trajectories, continuous multivariate outcomes, parameter-posterior mixtures, delayed effects, within-session interference or time-varying carryover. Those remain owned by the separate full-covariance EIG, history-adjusted state-space and trajectory scorecard work. Integration must not duplicate those engines.

## Benchmark protocol

Compare the policies on identical frozen model sets, priors, candidate manifests, outcome schemas, seeds and compute budgets:

1. one-shot/fixed manifest ranking;
2. greedy sequential EIG;
3. cost-normalized greedy EIG;
4. robust maximin EIG-per-cost over prior/model ambiguity;
5. random feasible;
6. balanced fixed/factorial design when a schedule-aware runner is added;
7. synthetic oracle upper bound (never a deployable policy).

Report cumulative information gain, held-out intervention predictive log score, trials and cost to decision, regret versus the synthetic oracle, calibration, false confident selection, false stopping/discovery, abstention quality, rejected candidates, runtime and model-evaluation count. Report paired-seed confidence intervals and stratify by regime; do not collapse failures into mean EIG.

## Initial acceptance gates

- Zero unsafe/infeasible selections.
- Zero posterior updates after assignment/delivery mismatch or uncleared washout.
- Zero confident model choice when every candidate has identical predictive distributions.
- Correct stop on posterior-predictive model inadequacy.
- Candidate/outcome ordering does not alter deterministic adaptive decisions.
- No false precise posterior when the observed outcome has zero likelihood under all surviving models; request model revision instead.
- Random and fixed baselines remain selectable even when their chosen candidate has low EIG; the information threshold is an adaptive-policy gate, not a way to bias the baselines.

These are software acceptance properties, not biological performance targets.

## Commands

```bash
cargo fmt --manifest-path research/anima-sequential-design-v0/Cargo.toml -- --check
cargo test --manifest-path research/anima-sequential-design-v0/Cargo.toml --locked
cargo clippy --manifest-path research/anima-sequential-design-v0/Cargo.toml --locked --all-targets -- -D warnings
```
